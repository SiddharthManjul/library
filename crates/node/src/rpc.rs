//! The node's stdio JSON-RPC server. The desktop app spawns the node and speaks one JSON object per line.
//! stdout carries only protocol messages; diagnostics go to stderr and never include params.
//!
//! Requests are handled concurrently, so a slow gateway call never delays `node.info` or the next request.
//! Responses may therefore arrive in a different order; the id says which is which.

use std::{sync::Arc, time::Duration};

use library_core::{ErrorCode, LibraryError};
use library_protocol::{
    Access, Link as WireLink, LinkStatus, NodeInfo, PROTOCOL_VERSION, RpcRequest, RpcResponse,
    SessionState, find,
};
use serde_json::Value;
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::mpsc,
    task::JoinSet,
};

use crate::link::{Link, LinkState, Transport};

/// A single message larger than this is refused, so a broken or hostile peer cannot exhaust memory.
pub const MAX_LINE_BYTES: usize = 1024 * 1024;
/// How long the app waits for the gateway before the call fails with a retryable error.
pub const REMOTE_TIMEOUT: Duration = Duration::from_secs(20);
/// At most this many requests run at once; more wait their turn.
const MAX_IN_FLIGHT: usize = 64;

pub struct Node<T: Transport> {
    link: Arc<Link<T>>,
}

/// The wire shape of a `remote.call` request's params.
#[derive(serde::Deserialize)]
struct RemoteCall {
    method: String,
    #[serde(default)]
    params: Value,
    #[serde(default)]
    idempotency_key: Option<String>,
}

fn status(s: &LinkState) -> Option<LinkStatus> {
    match s {
        LinkState::SignedOut => None,
        LinkState::Online => Some(LinkStatus::Online),
        LinkState::Connecting => Some(LinkStatus::Connecting),
        LinkState::Offline => Some(LinkStatus::Offline),
        LinkState::UpdateRequired => Some(LinkStatus::UpdateRequired),
    }
}

fn to_result<V: serde::Serialize>(id: u64, v: &V) -> RpcResponse {
    match serde_json::to_value(v) {
        Ok(v) => RpcResponse::ok(id, v),
        Err(_) => RpcResponse::err(id, LibraryError::internal()),
    }
}

impl<T: Transport> Node<T> {
    pub fn new(link: Arc<Link<T>>) -> Self {
        Self { link }
    }

    /// Answers one request. Unknown methods, methods meant for another link and methods that need a
    /// session all fail with a typed error; nothing panics on bad input.
    pub async fn handle(&self, req: RpcRequest) -> RpcResponse {
        let id = req.id;
        if req.jsonrpc != "2.0" {
            return RpcResponse::err(
                id,
                LibraryError::new(ErrorCode::InvalidParams, "Only JSON-RPC 2.0 is supported."),
            );
        }
        let Some(decl) = find(&req.method).filter(|m| m.link == WireLink::AppToNode) else {
            return RpcResponse::method_not_found(id);
        };
        let signed_in = self.link.identity().await.is_some();
        if matches!(decl.access, Access::Member | Access::Roles(_)) && !signed_in {
            return RpcResponse::err(
                id,
                LibraryError::new(ErrorCode::Unauthenticated, "Sign in to continue."),
            );
        }
        match req.method.as_str() {
            "node.info" => to_result(
                id,
                &NodeInfo {
                    name: "library-node".into(),
                    version: env!("CARGO_PKG_VERSION").into(),
                    protocol: PROTOCOL_VERSION,
                    platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
                },
            ),
            "session.state" => to_result(id, &self.session_state().await),
            "remote.call" => self.remote(id, req.params).await,
            _ => RpcResponse::err(
                id,
                LibraryError::new(ErrorCode::Unavailable, "This is not available yet."),
            ),
        }
    }

    async fn session_state(&self) -> SessionState {
        match (self.link.identity().await, status(&self.link.state().await)) {
            (Some((tenant_id, user_id)), Some(link)) => SessionState::SignedIn {
                tenant_id,
                user_id,
                link,
            },
            _ => SessionState::SignedOut,
        }
    }

    async fn remote(&self, id: u64, params: Value) -> RpcResponse {
        let call: RemoteCall = match serde_json::from_value(params) {
            Ok(c) => c,
            Err(_) => {
                return RpcResponse::err(
                    id,
                    LibraryError::new(ErrorCode::InvalidParams, "A method is required."),
                );
            }
        };
        // Only methods that exist on the gateway link, and never the ones the node must keep to itself.
        let Some(decl) = find(&call.method).filter(|m| m.link == WireLink::NodeToGateway) else {
            return RpcResponse::method_not_found(id);
        };
        if matches!(decl.access, Access::Server) || decl.name == "session.refresh" {
            return RpcResponse::err(
                id,
                LibraryError::new(
                    ErrorCode::PermissionDenied,
                    "The app cannot call this method.",
                ),
            );
        }
        match self
            .link
            .call(
                &call.method,
                call.params,
                call.idempotency_key,
                REMOTE_TIMEOUT,
            )
            .await
        {
            Ok(v) => RpcResponse::ok(id, v),
            Err(e) => RpcResponse::err(id, e),
        }
    }

    /// Serves requests until the input closes. Returns how many were handled.
    pub async fn serve<R, W>(self: Arc<Self>, mut input: R, output: W) -> std::io::Result<u64>
    where
        R: AsyncBufRead + AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        // One writer task, so concurrent responses never interleave inside a line.
        let (tx, mut rx) = mpsc::channel::<RpcResponse>(MAX_IN_FLIGHT);
        let writer = tokio::spawn(async move {
            let mut out = output;
            while let Some(resp) = rx.recv().await {
                let Ok(mut line) = serde_json::to_vec(&resp) else {
                    continue;
                };
                line.push(b'\n');
                if out.write_all(&line).await.is_err() || out.flush().await.is_err() {
                    break;
                }
            }
        });
        let (mut handled, mut tasks, mut line) = (0u64, JoinSet::new(), Vec::new());
        loop {
            line.clear();
            // Read at most one byte past the limit so an oversized line is detected without buffering it all.
            let n = (&mut input)
                .take(MAX_LINE_BYTES as u64 + 1)
                .read_until(b'\n', &mut line)
                .await?;
            if n == 0 {
                break;
            }
            handled += 1;
            if line.len() > MAX_LINE_BYTES {
                discard_rest_of_line(&mut input, line.last() == Some(&b'\n')).await?;
                let _ = tx
                    .send(RpcResponse::err(
                        0,
                        LibraryError::new(ErrorCode::PayloadTooLarge, "That message is too large."),
                    ))
                    .await;
                continue;
            }
            match serde_json::from_slice::<RpcRequest>(&line) {
                Ok(req) => {
                    while tasks.len() >= MAX_IN_FLIGHT {
                        tasks.join_next().await;
                    }
                    let (node, tx) = (self.clone(), tx.clone());
                    tasks.spawn(async move {
                        let _ = tx.send(node.handle(req).await).await;
                    });
                }
                Err(_) => {
                    let _ = tx.send(RpcResponse::parse_error()).await;
                }
            }
        }
        // Input closed: let every request that is still running answer before the writer stops.
        while tasks.join_next().await.is_some() {}
        drop(tx);
        let _ = writer.await;
        Ok(handled)
    }
}

async fn discard_rest_of_line<R: AsyncBufRead + Unpin>(
    input: &mut R,
    already_ended: bool,
) -> std::io::Result<()> {
    if already_ended {
        return Ok(());
    }
    let mut sink = Vec::new();
    loop {
        sink.clear();
        let n = (&mut *input)
            .take(64 * 1024)
            .read_until(b'\n', &mut sink)
            .await?;
        if n == 0 || sink.last() == Some(&b'\n') {
            return Ok(());
        }
    }
}
