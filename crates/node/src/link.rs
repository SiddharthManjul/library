//! The node's one connection to the gateway (README: Desktop app and local node).
//!
//! `Link` owns the session: it connects with the stored access token, refreshes over HTTPS when the token
//! is expired (a node offline for hours cannot open the socket to ask), rotates the stored pair, and
//! answers `remote.call` for the app. A refresh the gateway refuses signs the device out; a gateway that
//! cannot be reached leaves the session alone. One task owns the socket and multiplexes outgoing calls
//! with incoming messages, so a call never waits behind a read.

use std::{collections::HashMap, sync::Arc, time::Duration};

use library_core::{ErrorCode, LibraryError};
use library_protocol::{PROTOCOL_VERSION, RpcRequest, RpcResponse, TokenPairWire};
use serde_json::Value;
use tokio::sync::{Mutex, mpsc, oneshot};

use crate::{keychain::SecretStore, session::StoredSession};

/// What the link needs from the network.
pub trait Transport: Send + Sync + 'static {
    type Conn: Connection;
    /// Opens the WebSocket with `Authorization: Bearer <access>` and `X-Library-Protocol`.
    fn connect(
        &self,
        access: &str,
    ) -> impl Future<Output = Result<Self::Conn, ConnectError>> + Send;
    /// `POST /v1/session/refresh`.
    fn refresh(
        &self,
        refresh: &str,
    ) -> impl Future<Output = Result<TokenPairWire, LibraryError>> + Send;
}

#[derive(Debug)]
pub enum ConnectError {
    /// The gateway said the access token is expired or invalid.
    Unauthorized(LibraryError),
    /// This node is too old for the gateway.
    Outdated(LibraryError),
    /// Network trouble: worth retrying.
    Offline,
}

pub trait Connection: Send + 'static {
    fn send(&mut self, text: String) -> impl Future<Output = bool> + Send;
    /// The next text message, or `None` when the connection ended. Must be cancel safe.
    fn recv(&mut self) -> impl Future<Output = Option<String>> + Send;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkState {
    SignedOut,
    Connecting,
    Online,
    /// Signed in but the gateway is not reachable. Reads from the replica still work.
    Offline,
    /// This build is too old for the gateway.
    UpdateRequired,
}

type Waiters = Mutex<HashMap<u64, oneshot::Sender<RpcResponse>>>;

struct Inner {
    session: Option<StoredSession>,
    out: Option<mpsc::Sender<String>>,
    state: LinkState,
    next_id: u64,
    generation: u64,
}

struct Shared {
    inner: Mutex<Inner>,
    waiters: Waiters,
    push: mpsc::UnboundedSender<(String, Value)>,
}

pub struct Link<T: Transport> {
    transport: Arc<T>,
    store: Arc<dyn SecretStore>,
    shared: Arc<Shared>,
}

fn unauthenticated() -> LibraryError {
    LibraryError::new(ErrorCode::Unauthenticated, "Sign in to continue.")
}

fn offline() -> LibraryError {
    LibraryError::new(ErrorCode::Unavailable, "Cannot reach Library right now.")
}

fn storage() -> LibraryError {
    LibraryError::new(
        ErrorCode::Unavailable,
        "The secure storage on this computer could not be read or written.",
    )
}

/// Wait before reconnect attempt `n` (0 based): 1 s doubling to 60 s, with up to a quarter added as jitter
/// so a whole company reconnecting after a gateway restart does not arrive in the same second.
pub fn backoff(attempt: u32, jitter: f64) -> Duration {
    let base = 1u64 << attempt.min(6);
    Duration::from_secs_f64((base.min(60) as f64) * (1.0 + 0.25 * jitter.clamp(0.0, 1.0)))
}

impl<T: Transport> Link<T> {
    /// Loads the stored session, if any, and returns the receiver for notifications the gateway pushes.
    /// Nothing connects until `connect` is called.
    pub fn new(
        transport: T,
        store: Arc<dyn SecretStore>,
    ) -> Result<(Self, mpsc::UnboundedReceiver<(String, Value)>), LibraryError> {
        let session = StoredSession::load(&*store).map_err(|_| storage())?;
        let state = if session.is_some() {
            LinkState::Offline
        } else {
            LinkState::SignedOut
        };
        let (push, push_rx) = mpsc::unbounded_channel();
        let shared = Arc::new(Shared {
            inner: Mutex::new(Inner {
                session,
                out: None,
                state,
                next_id: 1,
                generation: 0,
            }),
            waiters: Mutex::new(HashMap::new()),
            push,
        });
        Ok((
            Self {
                transport: Arc::new(transport),
                store,
                shared,
            },
            push_rx,
        ))
    }

    pub async fn state(&self) -> LinkState {
        self.shared.inner.lock().await.state.clone()
    }

    /// Who is signed in, without any token.
    pub async fn identity(&self) -> Option<(library_core::TenantId, library_core::UserId)> {
        self.shared
            .inner
            .lock()
            .await
            .session
            .as_ref()
            .map(|s| (s.tenant_id, s.user_id))
    }

    /// Stores a freshly signed-in session (from the sign-in flow) and connects.
    pub async fn sign_in(&self, session: StoredSession) -> Result<(), LibraryError> {
        session.save(&*self.store).map_err(|_| storage())?;
        self.shared.inner.lock().await.session = Some(session);
        self.connect(now_secs()).await
    }

    /// Ends this device's session locally.
    pub async fn sign_out(&self) {
        let mut g = self.shared.inner.lock().await;
        let _ = StoredSession::clear(&*self.store);
        g.session = None;
        g.out = None;
        g.generation += 1;
        g.state = LinkState::SignedOut;
        drop(g);
        self.shared.waiters.lock().await.clear();
    }

    /// Connects, refreshing first when the stored access token has expired. Safe to call again after a drop.
    pub async fn connect(&self, now: i64) -> Result<(), LibraryError> {
        let mut g = self.shared.inner.lock().await;
        let Some(mut s) = g.session.clone() else {
            g.state = LinkState::SignedOut;
            return Err(unauthenticated());
        };
        g.state = LinkState::Connecting;
        // A token about to expire is refreshed now instead of failing the upgrade.
        if s.access_expires_at <= now + 30 {
            s = self.refresh_locked(&mut g, &s).await?;
        }
        let conn = match self.transport.connect(&s.access).await {
            Ok(c) => c,
            Err(ConnectError::Unauthorized(_)) => {
                // The gateway and this clock disagree about expiry, or the token was revoked: one refresh, one retry.
                let fresh = self.refresh_locked(&mut g, &s).await?;
                match self.transport.connect(&fresh.access).await {
                    Ok(c) => c,
                    Err(e) => return Err(Self::settle(&mut g, e)),
                }
            }
            Err(e) => return Err(Self::settle(&mut g, e)),
        };
        g.generation += 1;
        let generation = g.generation;
        let (tx, rx) = mpsc::channel(64);
        g.out = Some(tx);
        g.state = LinkState::Online;
        tokio::spawn(run_connection(conn, rx, self.shared.clone(), generation));
        Ok(())
    }

    fn settle(g: &mut Inner, e: ConnectError) -> LibraryError {
        match e {
            ConnectError::Outdated(e) => {
                g.state = LinkState::UpdateRequired;
                e
            }
            ConnectError::Unauthorized(e) => {
                g.state = LinkState::Offline;
                e
            }
            ConnectError::Offline => {
                g.state = LinkState::Offline;
                offline()
            }
        }
    }

    /// Exchanges the refresh token and stores the new pair before anything uses it. A refusal from the
    /// gateway (revoked, reused, expired) signs the device out; an unreachable gateway leaves the session
    /// in place so the next attempt can try again.
    async fn refresh_locked(
        &self,
        g: &mut Inner,
        s: &StoredSession,
    ) -> Result<StoredSession, LibraryError> {
        match self.transport.refresh(&s.refresh).await {
            Ok(pair) => {
                let fresh = StoredSession {
                    access: pair.access,
                    refresh: pair.refresh,
                    access_expires_at: pair.access_expires_at,
                    ..s.clone()
                };
                // The old refresh token is spent. If the new pair cannot be saved, carrying on would sign the
                // member out at the next restart with no explanation, so fail loudly now.
                fresh.save(&*self.store).map_err(|_| storage())?;
                g.session = Some(fresh.clone());
                Ok(fresh)
            }
            Err(e) if matches!(e.code, ErrorCode::Unauthenticated | ErrorCode::TokenExpired) => {
                let _ = StoredSession::clear(&*self.store);
                g.session = None;
                g.out = None;
                g.generation += 1;
                g.state = LinkState::SignedOut;
                Err(e)
            }
            Err(e) => {
                g.state = LinkState::Offline;
                Err(e)
            }
        }
    }

    /// `remote.<method>`: one call to the gateway, and its answer.
    pub async fn call(
        &self,
        method: &str,
        params: Value,
        idempotency_key: Option<String>,
        timeout: Duration,
    ) -> Result<Value, LibraryError> {
        let (id, out) = {
            let mut g = self.shared.inner.lock().await;
            if g.session.is_none() {
                return Err(unauthenticated());
            }
            let Some(out) = g.out.clone() else {
                return Err(offline());
            };
            let id = g.next_id;
            g.next_id += 1;
            (id, out)
        };
        let req = RpcRequest {
            jsonrpc: "2.0".into(),
            id,
            method: method.to_string(),
            params,
            idempotency_key,
        };
        let text = serde_json::to_string(&req).map_err(|_| LibraryError::internal())?;
        let (tx, rx) = oneshot::channel();
        self.shared.waiters.lock().await.insert(id, tx);
        if out.send(text).await.is_err() {
            self.shared.waiters.lock().await.remove(&id);
            return Err(offline());
        }
        let resp = match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(r)) => r,
            Ok(Err(_)) => return Err(offline()),
            Err(_) => {
                self.shared.waiters.lock().await.remove(&id);
                return Err(LibraryError::new(
                    ErrorCode::Unavailable,
                    "Library did not answer in time.",
                ));
            }
        };
        match (resp.result, resp.error) {
            (Some(v), None) => Ok(v),
            (_, Some(e)) => Err(e
                .data
                .unwrap_or_else(|| LibraryError::new(ErrorCode::Internal, e.message))),
            _ => Err(LibraryError::internal()),
        }
    }
}

/// Owns one connection until it ends: sends what the link queues, hands responses to their callers,
/// forwards notifications. On exit it fails every pending call and marks the link offline, unless a newer
/// connection has already taken over.
async fn run_connection<C: Connection>(
    mut conn: C,
    mut out: mpsc::Receiver<String>,
    shared: Arc<Shared>,
    generation: u64,
) {
    loop {
        tokio::select! {
            queued = out.recv() => match queued {
                Some(text) => { if !conn.send(text).await { break } }
                None => break,
            },
            incoming = conn.recv() => match incoming {
                Some(text) => dispatch(&shared, &text).await,
                None => break,
            },
        }
    }
    let mut g = shared.inner.lock().await;
    if g.generation == generation {
        g.out = None;
        g.state = if g.session.is_some() {
            LinkState::Offline
        } else {
            LinkState::SignedOut
        };
        drop(g);
        shared.waiters.lock().await.clear(); // dropping the senders fails every pending call
    }
}

async fn dispatch(shared: &Shared, text: &str) {
    let Ok(v) = serde_json::from_str::<Value>(text) else {
        return;
    };
    if v.get("id").is_some() {
        if let Ok(resp) = serde_json::from_value::<RpcResponse>(v)
            && let Some(tx) = shared.waiters.lock().await.remove(&resp.id)
        {
            let _ = tx.send(resp);
        }
    } else if let Some(name) = v.get("method").and_then(Value::as_str) {
        let _ = shared.push.send((
            name.to_string(),
            v.get("params").cloned().unwrap_or(Value::Null),
        ));
    }
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

/// The headers every connection sends.
pub fn upgrade_headers(access: &str) -> [(&'static str, String); 2] {
    [
        ("authorization", format!("Bearer {access}")),
        ("x-library-protocol", PROTOCOL_VERSION.to_string()),
    ]
}
