#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not
//! The link's session and reconnect logic against a scripted network, so every case is exact.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use library_core::{ErrorCode, LibraryError, TenantId, UserId};
use library_node::{
    keychain::{KeychainError, MemoryStore, SecretStore},
    link::{ConnectError, Connection, Link, LinkState, Transport, backoff},
    session::{KEYCHAIN_ITEM, StoredSession},
};
use library_protocol::TokenPairWire;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use zeroize::Zeroizing;

/// What the fake gateway does next.
#[derive(Default)]
struct Script {
    connects: VecDeque<Result<(), ConnectErrorKind>>,
    refreshes: VecDeque<Result<TokenPairWire, LibraryError>>,
    seen_access: Vec<String>,
    seen_refresh: Vec<String>,
}

#[derive(Clone, Copy)]
enum ConnectErrorKind {
    Unauthorized,
    Outdated,
    Offline,
}

struct Fake {
    script: Arc<Mutex<Script>>,
    /// Messages the fake gateway sends to the link; the test pushes here.
    to_link: Arc<Mutex<Option<mpsc::UnboundedSender<Option<String>>>>>,
    /// Messages the link sent; the fake answers them.
    from_link: Arc<Mutex<Vec<Value>>>,
    autorespond: bool,
}

struct FakeConn {
    inbox: mpsc::UnboundedReceiver<Option<String>>,
    from_link: Arc<Mutex<Vec<Value>>>,
    to_link: mpsc::UnboundedSender<Option<String>>,
    autorespond: bool,
}

impl Connection for FakeConn {
    async fn send(&mut self, text: String) -> bool {
        let v: Value = serde_json::from_str(&text).unwrap();
        self.from_link.lock().unwrap().push(v.clone());
        if self.autorespond {
            let resp = json!({ "jsonrpc": "2.0", "id": v["id"], "result": { "echo": v["method"], "key": v["idempotency_key"] }, "error": null });
            let _ = self.to_link.send(Some(resp.to_string()));
        }
        true
    }

    async fn recv(&mut self) -> Option<String> {
        // `None` on the wire means the gateway hung up.
        self.inbox.recv().await.flatten()
    }
}

impl Transport for Fake {
    type Conn = FakeConn;

    async fn connect(&self, access: &str) -> Result<FakeConn, ConnectError> {
        let next = {
            let mut s = self.script.lock().unwrap();
            s.seen_access.push(access.to_string());
            s.connects.pop_front().unwrap_or(Ok(()))
        };
        match next {
            Ok(()) => {
                let (tx, rx) = mpsc::unbounded_channel();
                *self.to_link.lock().unwrap() = Some(tx.clone());
                Ok(FakeConn {
                    inbox: rx,
                    from_link: self.from_link.clone(),
                    to_link: tx,
                    autorespond: self.autorespond,
                })
            }
            Err(ConnectErrorKind::Unauthorized) => Err(ConnectError::Unauthorized(
                LibraryError::new(ErrorCode::TokenExpired, "expired"),
            )),
            Err(ConnectErrorKind::Outdated) => Err(ConnectError::Outdated(LibraryError::new(
                ErrorCode::VersionMismatch,
                "update",
            ))),
            Err(ConnectErrorKind::Offline) => Err(ConnectError::Offline),
        }
    }

    async fn refresh(&self, refresh: &str) -> Result<TokenPairWire, LibraryError> {
        let mut s = self.script.lock().unwrap();
        s.seen_refresh.push(refresh.to_string());
        s.refreshes
            .pop_front()
            .unwrap_or_else(|| Err(LibraryError::new(ErrorCode::Unavailable, "down")))
    }
}

struct Harness {
    link: Link<Fake>,
    push_rx: mpsc::UnboundedReceiver<(String, Value)>,
    store: Arc<MemoryStore>,
    script: Arc<Mutex<Script>>,
    to_link: Arc<Mutex<Option<mpsc::UnboundedSender<Option<String>>>>>,
    from_link: Arc<Mutex<Vec<Value>>>,
}

fn harness(signed_in: Option<i64>, autorespond: bool) -> Harness {
    let store = Arc::new(MemoryStore::new());
    if let Some(exp) = signed_in {
        session("access-0", "refresh-0", exp).save(&*store).unwrap();
    }
    let (script, to_link, from_link) = (
        Arc::new(Mutex::new(Script::default())),
        Arc::new(Mutex::new(None)),
        Arc::new(Mutex::new(vec![])),
    );
    let fake = Fake {
        script: script.clone(),
        to_link: to_link.clone(),
        from_link: from_link.clone(),
        autorespond,
    };
    let (link, push_rx) = Link::new(fake, store.clone()).unwrap();
    Harness {
        link,
        push_rx,
        store,
        script,
        to_link,
        from_link,
    }
}

fn session(access: &str, refresh: &str, exp: i64) -> StoredSession {
    StoredSession {
        tenant_id: TenantId::new(),
        user_id: UserId::new(),
        access: access.into(),
        refresh: refresh.into(),
        access_expires_at: exp,
    }
}

fn pair(n: u32, exp: i64) -> TokenPairWire {
    TokenPairWire {
        access: format!("access-{n}"),
        refresh: format!("refresh-{n}"),
        access_expires_at: exp,
    }
}

const NOW: i64 = 1_800_000_000;
const SECOND: Duration = Duration::from_secs(5);

#[tokio::test]
async fn a_fresh_install_is_signed_out_and_cannot_call_or_connect() {
    let h = harness(None, true);
    assert_eq!(h.link.state().await, LinkState::SignedOut);
    assert_eq!(
        h.link.connect(NOW).await.unwrap_err().code,
        ErrorCode::Unauthenticated
    );
    assert_eq!(
        h.link
            .call("session.whoami", json!({}), None, SECOND)
            .await
            .unwrap_err()
            .code,
        ErrorCode::Unauthenticated
    );
}

#[tokio::test]
async fn a_valid_token_connects_and_calls_get_answers() {
    let h = harness(Some(NOW + 600), true);
    assert_eq!(
        h.link.state().await,
        LinkState::Offline,
        "a stored session is offline until it connects"
    );
    h.link.connect(NOW).await.unwrap();
    assert_eq!(h.link.state().await, LinkState::Online);
    let r = h
        .link
        .call(
            "usage.report",
            json!({ "a": 1 }),
            Some("key-0123456789".into()),
            SECOND,
        )
        .await
        .unwrap();
    assert_eq!(r["echo"], "usage.report");
    assert_eq!(
        r["key"], "key-0123456789",
        "the idempotency key goes to the gateway"
    );
    assert_eq!(h.script.lock().unwrap().seen_access, vec!["access-0"]);
    assert!(
        h.script.lock().unwrap().seen_refresh.is_empty(),
        "no refresh when the token is good"
    );
}

#[tokio::test]
async fn an_expired_token_is_refreshed_before_connecting_and_the_new_pair_is_stored() {
    let h = harness(Some(NOW - 7200), true);
    h.script
        .lock()
        .unwrap()
        .refreshes
        .push_back(Ok(pair(1, NOW + 900)));
    h.link.connect(NOW).await.unwrap();
    assert_eq!(h.script.lock().unwrap().seen_refresh, vec!["refresh-0"]);
    assert_eq!(
        h.script.lock().unwrap().seen_access,
        vec!["access-1"],
        "the socket opens with the fresh token, never the dead one"
    );
    let stored = StoredSession::load(&*h.store).unwrap().unwrap();
    assert_eq!(
        (stored.access.as_str(), stored.refresh.as_str()),
        ("access-1", "refresh-1")
    );
}

#[tokio::test]
async fn a_token_about_to_expire_is_refreshed_too() {
    let h = harness(Some(NOW + 10), true);
    h.script
        .lock()
        .unwrap()
        .refreshes
        .push_back(Ok(pair(1, NOW + 900)));
    h.link.connect(NOW).await.unwrap();
    assert_eq!(h.script.lock().unwrap().seen_access, vec!["access-1"]);
}

#[tokio::test]
async fn when_the_gateway_disagrees_about_expiry_the_node_refreshes_once_and_retries() {
    let h = harness(Some(NOW + 900), true);
    h.script
        .lock()
        .unwrap()
        .connects
        .push_back(Err(ConnectErrorKind::Unauthorized));
    h.script
        .lock()
        .unwrap()
        .refreshes
        .push_back(Ok(pair(1, NOW + 900)));
    h.link.connect(NOW).await.unwrap();
    assert_eq!(
        h.script.lock().unwrap().seen_access,
        vec!["access-0", "access-1"]
    );
    assert_eq!(h.link.state().await, LinkState::Online);
}

#[tokio::test]
async fn a_refresh_the_gateway_refuses_signs_the_device_out_and_wipes_the_keychain() {
    let h = harness(Some(NOW - 7200), true);
    h.script
        .lock()
        .unwrap()
        .refreshes
        .push_back(Err(LibraryError::new(
            ErrorCode::Unauthenticated,
            "revoked",
        )));
    let e = h.link.connect(NOW).await.unwrap_err();
    assert_eq!(e.code, ErrorCode::Unauthenticated);
    assert_eq!(h.link.state().await, LinkState::SignedOut);
    assert!(
        h.store.get(KEYCHAIN_ITEM).unwrap().is_none(),
        "a dead session must not linger in the keychain"
    );
    assert_eq!(
        h.link
            .call("session.whoami", json!({}), None, SECOND)
            .await
            .unwrap_err()
            .code,
        ErrorCode::Unauthenticated
    );
}

#[tokio::test]
async fn an_unreachable_gateway_keeps_the_session_so_the_node_can_try_again() {
    let h = harness(Some(NOW - 7200), true);
    h.script
        .lock()
        .unwrap()
        .refreshes
        .push_back(Err(LibraryError::new(ErrorCode::Unavailable, "down")));
    assert_eq!(
        h.link.connect(NOW).await.unwrap_err().code,
        ErrorCode::Unavailable
    );
    assert_eq!(h.link.state().await, LinkState::Offline);
    assert!(
        StoredSession::load(&*h.store).unwrap().is_some(),
        "being offline is not being signed out"
    );
    // And it works once the network is back.
    h.script
        .lock()
        .unwrap()
        .refreshes
        .push_back(Ok(pair(1, NOW + 900)));
    h.link.connect(NOW).await.unwrap();
    assert_eq!(h.link.state().await, LinkState::Online);
}

#[tokio::test]
async fn offline_connects_leave_the_session_and_calls_fail_fast_with_a_clear_error() {
    let h = harness(Some(NOW + 900), true);
    h.script
        .lock()
        .unwrap()
        .connects
        .push_back(Err(ConnectErrorKind::Offline));
    assert_eq!(
        h.link.connect(NOW).await.unwrap_err().code,
        ErrorCode::Unavailable
    );
    assert_eq!(h.link.state().await, LinkState::Offline);
    let e = h
        .link
        .call("session.whoami", json!({}), None, SECOND)
        .await
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::Unavailable);
    assert!(e.retryable);
}

#[tokio::test]
async fn an_outdated_node_is_told_to_update_and_keeps_its_session() {
    let h = harness(Some(NOW + 900), true);
    h.script
        .lock()
        .unwrap()
        .connects
        .push_back(Err(ConnectErrorKind::Outdated));
    let e = h.link.connect(NOW).await.unwrap_err();
    assert_eq!(e.code, ErrorCode::VersionMismatch);
    assert_eq!(h.link.state().await, LinkState::UpdateRequired);
    assert!(StoredSession::load(&*h.store).unwrap().is_some());
}

/// A keychain that reads fine and refuses to write, to prove a spent refresh token is never lost silently.
struct ReadOnlyStore(MemoryStore);
impl SecretStore for ReadOnlyStore {
    fn get(&self, n: &str) -> Result<Option<Zeroizing<Vec<u8>>>, KeychainError> {
        self.0.get(n)
    }
    fn set(&self, _n: &str, _s: &[u8]) -> Result<(), KeychainError> {
        Err(KeychainError::Unavailable("locked".into()))
    }
    fn delete(&self, n: &str) -> Result<(), KeychainError> {
        self.0.delete(n)
    }
}

#[tokio::test]
async fn if_the_new_tokens_cannot_be_saved_the_node_says_so_instead_of_carrying_on() {
    let inner = MemoryStore::new();
    session("access-0", "refresh-0", NOW - 7200)
        .save(&inner)
        .unwrap();
    let script = Arc::new(Mutex::new(Script::default()));
    script
        .lock()
        .unwrap()
        .refreshes
        .push_back(Ok(pair(1, NOW + 900)));
    let fake = Fake {
        script: script.clone(),
        to_link: Arc::default(),
        from_link: Arc::default(),
        autorespond: true,
    };
    let (link, _rx) = Link::new(fake, Arc::new(ReadOnlyStore(inner))).unwrap();
    let e = link.connect(NOW).await.unwrap_err();
    assert_eq!(e.code, ErrorCode::Unavailable);
    assert!(e.message.contains("secure storage"), "{}", e.message);
    assert!(
        script.lock().unwrap().seen_access.is_empty(),
        "it must not connect on tokens it could not keep"
    );
}

#[tokio::test]
async fn a_damaged_session_in_the_keychain_is_an_error_not_a_silent_sign_out() {
    let store = Arc::new(MemoryStore::new());
    store.set(KEYCHAIN_ITEM, b"{ not a session").unwrap();
    let fake = Fake {
        script: Arc::default(),
        to_link: Arc::default(),
        from_link: Arc::default(),
        autorespond: true,
    };
    assert!(Link::new(fake, store).is_err());
}

#[tokio::test]
async fn calls_are_matched_to_answers_by_id_even_when_answers_come_back_out_of_order() {
    let h = harness(Some(NOW + 900), false);
    h.link.connect(NOW).await.unwrap();
    let link = Arc::new(h.link);
    let (a, b) = (link.clone(), link.clone());
    let first = tokio::spawn(async move { a.call("tasks.list", json!({}), None, SECOND).await });
    let second = tokio::spawn(async move { b.call("runs.get", json!({}), None, SECOND).await });
    // Wait until both requests have been written, then answer them in reverse.
    for _ in 0..200 {
        if h.from_link.lock().unwrap().len() == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let sent = h.from_link.lock().unwrap().clone();
    assert_eq!(sent.len(), 2);
    let tx = h.to_link.lock().unwrap().clone().unwrap();
    for req in sent.iter().rev() {
        tx.send(Some(json!({ "jsonrpc": "2.0", "id": req["id"], "result": { "for": req["method"] }, "error": null }).to_string())).unwrap();
    }
    assert_eq!(first.await.unwrap().unwrap()["for"], "tasks.list");
    assert_eq!(second.await.unwrap().unwrap()["for"], "runs.get");
}

#[tokio::test]
async fn typed_gateway_errors_reach_the_caller_intact() {
    let h = harness(Some(NOW + 900), false);
    h.link.connect(NOW).await.unwrap();
    let link = Arc::new(h.link);
    let l = link.clone();
    let call = tokio::spawn(async move {
        l.call(
            "org.departments.upsert",
            json!({}),
            Some("key-0123456789".into()),
            SECOND,
        )
        .await
    });
    for _ in 0..200 {
        if !h.from_link.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let id = h.from_link.lock().unwrap()[0]["id"].clone();
    let err = serde_json::to_value(LibraryError::new(
        ErrorCode::PermissionDenied,
        "Your role cannot do this.",
    ))
    .unwrap();
    h.to_link.lock().unwrap().clone().unwrap().send(Some(json!({ "jsonrpc": "2.0", "id": id, "result": null, "error": { "code": -32003, "message": "Your role cannot do this.", "data": err } }).to_string())).unwrap();
    let e = call.await.unwrap().unwrap_err();
    assert_eq!((e.code, e.retryable), (ErrorCode::PermissionDenied, false));
}

#[tokio::test]
async fn a_call_nobody_answers_times_out_and_leaves_nothing_behind() {
    let h = harness(Some(NOW + 900), false);
    h.link.connect(NOW).await.unwrap();
    let e = h
        .link
        .call(
            "session.whoami",
            json!({}),
            None,
            Duration::from_millis(100),
        )
        .await
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::Unavailable);
    // A late answer to the abandoned call is ignored.
    let id = h.from_link.lock().unwrap()[0]["id"].clone();
    h.to_link
        .lock()
        .unwrap()
        .clone()
        .unwrap()
        .send(Some(
            json!({ "jsonrpc": "2.0", "id": id, "result": {}, "error": null }).to_string(),
        ))
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(h.link.state().await, LinkState::Online);
}

#[tokio::test]
async fn when_the_connection_drops_pending_calls_fail_at_once_and_the_link_goes_offline() {
    let h = harness(Some(NOW + 900), false);
    h.link.connect(NOW).await.unwrap();
    let link = Arc::new(h.link);
    let l = link.clone();
    let pending = tokio::spawn(async move {
        l.call("session.whoami", json!({}), None, Duration::from_secs(30))
            .await
    });
    for _ in 0..200 {
        if !h.from_link.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    h.to_link
        .lock()
        .unwrap()
        .clone()
        .unwrap()
        .send(None)
        .unwrap(); // the gateway hangs up
    let started = std::time::Instant::now();
    let e = tokio::time::timeout(Duration::from_secs(5), pending)
        .await
        .expect("a dropped connection must not leave a caller waiting")
        .unwrap()
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::Unavailable);
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(link.state().await, LinkState::Offline);
    assert!(
        StoredSession::load(&*h.store).unwrap().is_some(),
        "a dropped connection does not sign anyone out"
    );
}

#[tokio::test]
async fn notifications_pushed_by_the_gateway_reach_the_app_side() {
    let mut h = harness(Some(NOW + 900), true);
    h.link.connect(NOW).await.unwrap();
    h.to_link
        .lock()
        .unwrap()
        .clone()
        .unwrap()
        .send(Some(
            json!({ "jsonrpc": "2.0", "method": "sync.delta", "params": { "seq": 9 } }).to_string(),
        ))
        .unwrap();
    let (name, params) = tokio::time::timeout(SECOND, h.push_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (name.as_str(), params["seq"].as_i64()),
        ("sync.delta", Some(9))
    );
}

#[tokio::test]
async fn signing_out_clears_the_keychain_and_stops_calls() {
    let h = harness(Some(NOW + 900), true);
    h.link.connect(NOW).await.unwrap();
    h.link.sign_out().await;
    assert_eq!(h.link.state().await, LinkState::SignedOut);
    assert!(h.store.get(KEYCHAIN_ITEM).unwrap().is_none());
    assert_eq!(
        h.link
            .call("session.whoami", json!({}), None, SECOND)
            .await
            .unwrap_err()
            .code,
        ErrorCode::Unauthenticated
    );
}

#[tokio::test]
async fn signing_in_stores_the_session_and_connects() {
    let h = harness(None, true);
    h.link
        .sign_in(session(
            "access-9",
            "refresh-9",
            library_node::link::now_secs() + 900,
        ))
        .await
        .unwrap();
    assert_eq!(h.link.state().await, LinkState::Online);
    assert_eq!(
        StoredSession::load(&*h.store).unwrap().unwrap().refresh,
        "refresh-9"
    );
}

#[test]
fn reconnect_waits_grow_to_a_minute_and_are_spread_out() {
    let waits: Vec<f64> = (0..9).map(|n| backoff(n, 0.0).as_secs_f64()).collect();
    assert_eq!(
        waits,
        vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 60.0, 60.0, 60.0]
    );
    assert!(backoff(3, 1.0) > backoff(3, 0.0), "jitter adds time");
    assert!(backoff(3, 1.0).as_secs_f64() <= 8.0 * 1.25 + 0.001);
    assert!(
        backoff(u32::MAX, 5.0).as_secs_f64() <= 60.0 * 1.25 + 0.001,
        "huge attempt counts and wild jitter stay bounded"
    );
}

#[test]
fn a_stored_session_never_prints_its_tokens() {
    let s = session("access-SECRET-value", "refresh-SECRET-value", 1);
    assert!(!format!("{s:?}").contains("SECRET"));
}
