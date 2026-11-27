#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not
//! The node's stdio protocol with a scripted gateway: no database, no keychain, no network.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use library_core::{ErrorCode, LibraryError, TenantId, UserId};
use library_node::{
    keychain::MemoryStore,
    link::{ConnectError, Connection, Link, Transport, now_secs},
    rpc::{MAX_LINE_BYTES, Node},
    session::StoredSession,
};
use library_protocol::{RpcResponse, SessionState, TokenPairWire};
use serde_json::{Value, json};
use tokio::{io::AsyncWriteExt, sync::mpsc};

/// A gateway that answers every call with `{ "echo": method }`, or never answers `slow.*`-style methods.
struct Gateway {
    calls: Arc<Mutex<Vec<Value>>>,
}

struct Conn {
    rx: mpsc::UnboundedReceiver<String>,
    tx: mpsc::UnboundedSender<String>,
    calls: Arc<Mutex<Vec<Value>>>,
}

impl Connection for Conn {
    async fn send(&mut self, text: String) -> bool {
        let v: Value = serde_json::from_str(&text).unwrap();
        self.calls.lock().unwrap().push(v.clone());
        // `runs.get` is answered late, so tests can see that a slow call does not block other requests.
        let delay = if v["method"] == "runs.get" { 400 } else { 0 };
        let tx = self.tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(delay)).await;
            let _ = tx.send(json!({ "jsonrpc": "2.0", "id": v["id"], "result": { "echo": v["method"], "params": v["params"], "key": v["idempotency_key"] }, "error": null }).to_string());
        });
        true
    }

    async fn recv(&mut self) -> Option<String> {
        self.rx.recv().await
    }
}

impl Transport for Gateway {
    type Conn = Conn;

    async fn connect(&self, _a: &str) -> Result<Conn, ConnectError> {
        let (tx, rx) = mpsc::unbounded_channel();
        Ok(Conn {
            rx,
            tx,
            calls: self.calls.clone(),
        })
    }

    async fn refresh(&self, _r: &str) -> Result<TokenPairWire, LibraryError> {
        Err(LibraryError::new(ErrorCode::Unavailable, "down"))
    }
}

struct Rig {
    node: Arc<Node<Gateway>>,
    calls: Arc<Mutex<Vec<Value>>>,
}

async fn rig(signed_in: bool) -> Rig {
    let store = Arc::new(MemoryStore::new());
    if signed_in {
        StoredSession {
            tenant_id: TenantId::new(),
            user_id: UserId::new(),
            access: "a".into(),
            refresh: "r".into(),
            access_expires_at: now_secs() + 900,
        }
        .save(&*store)
        .unwrap();
    }
    let calls = Arc::new(Mutex::new(vec![]));
    let (link, _pushes) = Link::new(
        Gateway {
            calls: calls.clone(),
        },
        store,
    )
    .unwrap();
    let link = Arc::new(link);
    if signed_in {
        link.connect(now_secs()).await.unwrap();
    }
    Rig {
        node: Arc::new(Node::new(link)),
        calls,
    }
}

/// Runs the server on in-memory pipes and returns every response line it wrote.
async fn run(node: &Arc<Node<Gateway>>, input: Vec<u8>) -> Vec<RpcResponse> {
    let (mut client, server) = tokio::io::duplex(8 * 1024 * 1024);
    let (read, write) = tokio::io::split(server);
    let task = tokio::spawn(node.clone().serve(tokio::io::BufReader::new(read), write));
    client.write_all(&input).await.unwrap();
    client.shutdown().await.unwrap();
    task.await.unwrap().unwrap();
    let mut out = String::new();
    tokio::io::AsyncReadExt::read_to_string(&mut client, &mut out)
        .await
        .unwrap();
    out.lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn req(id: u64, method: &str, params: Value) -> String {
    format!(
        "{}\n",
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
    )
}

fn by_id(rs: &[RpcResponse], id: u64) -> &RpcResponse {
    rs.iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("no response for id {id}"))
}

fn error_code(r: &RpcResponse) -> Option<ErrorCode> {
    r.error
        .as_ref()
        .and_then(|e| e.data.as_ref())
        .map(|d| d.code)
}

#[tokio::test]
async fn node_info_reports_version_and_protocol() {
    let r = rig(false).await;
    let rs = run(&r.node, req(1, "node.info", json!({})).into_bytes()).await;
    let v = rs[0].result.as_ref().unwrap();
    assert_eq!(
        (v["name"].as_str(), v["protocol"].as_i64()),
        (Some("library-node"), Some(1))
    );
    assert!(
        v["platform"]
            .as_str()
            .unwrap()
            .contains(std::env::consts::OS)
    );
}

#[tokio::test]
async fn session_state_reflects_the_real_session_and_the_connection() {
    let out = rig(false).await;
    let s: SessionState = serde_json::from_value(
        run(&out.node, req(1, "session.state", json!({})).into_bytes()).await[0]
            .result
            .clone()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(s, SessionState::SignedOut);

    let r = rig(true).await;
    let v = run(&r.node, req(1, "session.state", json!({})).into_bytes()).await[0]
        .result
        .clone()
        .unwrap();
    assert_eq!(
        (v["state"].as_str(), v["link"].as_str()),
        (Some("signed_in"), Some("online"))
    );
    assert!(v["tenant_id"].is_string() && v["user_id"].is_string());
    assert!(
        !v.to_string().contains("access") && !v.to_string().contains("refresh"),
        "no token ever reaches the app: {v}"
    );
}

#[tokio::test]
async fn methods_that_need_a_session_fail_when_signed_out() {
    let r = rig(false).await;
    for m in [
        "local.ask",
        "local.sync_status",
        "local.permissions.decide",
        "remote.call",
    ] {
        let rs = run(
            &r.node,
            req(3, m, json!({ "method": "session.whoami" })).into_bytes(),
        )
        .await;
        assert_eq!(error_code(&rs[0]), Some(ErrorCode::Unauthenticated), "{m}");
    }
    assert!(r.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn unknown_methods_and_gateway_methods_are_not_found_on_the_app_link() {
    let r = rig(true).await;
    for m in [
        "nope.nothing",
        "sync.subscribe",
        "dispatch.call",
        "session.whoami",
        "",
    ] {
        let rs = run(&r.node, req(4, m, json!({})).into_bytes()).await;
        assert_eq!(rs[0].error.as_ref().unwrap().code, -32601, "{m:?}");
    }
    assert!(
        r.calls.lock().unwrap().is_empty(),
        "nothing leaked to the gateway"
    );
}

#[tokio::test]
async fn remote_call_forwards_a_gateway_method_with_its_params_and_idempotency_key() {
    let r = rig(true).await;
    let p = json!({ "method": "usage.report", "params": { "day": "2026-11-27" }, "idempotency_key": "key-0123456789" });
    let rs = run(&r.node, req(5, "remote.call", p).into_bytes()).await;
    let v = rs[0].result.as_ref().unwrap();
    assert_eq!(
        (
            v["echo"].as_str(),
            v["key"].as_str(),
            v["params"]["day"].as_str()
        ),
        (
            Some("usage.report"),
            Some("key-0123456789"),
            Some("2026-11-27")
        )
    );
    assert_eq!(r.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn remote_call_refuses_what_the_app_must_not_reach() {
    let r = rig(true).await;
    // Server-only methods, the node's own session management, app-link methods, unknown methods, bad params.
    for (params, want) in [
        (
            json!({ "method": "brain.context" }),
            Some(ErrorCode::PermissionDenied),
        ),
        (
            json!({ "method": "runs.lease" }),
            Some(ErrorCode::PermissionDenied),
        ),
        (
            json!({ "method": "session.refresh", "params": { "refresh": "x" } }),
            Some(ErrorCode::PermissionDenied),
        ),
        (json!({ "method": "local.ask" }), None),
        (json!({ "method": "node.info" }), None),
        (json!({ "method": "no.such.method" }), None),
        (json!({}), Some(ErrorCode::InvalidParams)),
        (json!({ "method": 7 }), Some(ErrorCode::InvalidParams)),
        (json!("just a string"), Some(ErrorCode::InvalidParams)),
    ] {
        let rs = run(&r.node, req(6, "remote.call", params.clone()).into_bytes()).await;
        match want {
            Some(code) => assert_eq!(error_code(&rs[0]), Some(code), "{params}"),
            None => assert_eq!(rs[0].error.as_ref().unwrap().code, -32601, "{params}"),
        }
    }
    assert!(
        r.calls.lock().unwrap().is_empty(),
        "no refused call reached the gateway"
    );
}

#[tokio::test]
async fn a_slow_gateway_call_does_not_hold_up_other_requests() {
    let r = rig(true).await;
    let mut input = req(10, "remote.call", json!({ "method": "runs.get" })); // answered after 400 ms
    input.push_str(&req(11, "node.info", json!({})));
    input.push_str(&req(12, "session.state", json!({})));
    let started = std::time::Instant::now();
    let (mut client, server) = tokio::io::duplex(1 << 20);
    let (read, write) = tokio::io::split(server);
    let task = tokio::spawn(r.node.clone().serve(tokio::io::BufReader::new(read), write));
    client.write_all(input.as_bytes()).await.unwrap();
    // The two quick answers must be readable long before the slow one is ready.
    let mut buf = vec![0u8; 4096];
    let mut seen = String::new();
    while seen.lines().count() < 2 {
        let n = tokio::io::AsyncReadExt::read(&mut client, &mut buf)
            .await
            .unwrap();
        seen.push_str(&String::from_utf8_lossy(&buf[..n]));
    }
    assert!(
        started.elapsed() < Duration::from_millis(300),
        "quick requests waited behind the slow one: {:?}",
        started.elapsed()
    );
    assert!(
        !seen.contains("runs.get"),
        "the slow answer is not here yet"
    );
    client.shutdown().await.unwrap();
    task.await.unwrap().unwrap();
    tokio::io::AsyncReadExt::read_to_string(&mut client, &mut seen)
        .await
        .unwrap();
    assert!(
        seen.contains("runs.get"),
        "and it still arrives: nothing is dropped when the input closes"
    );
}

#[tokio::test]
async fn bad_input_gets_an_error_and_the_node_keeps_serving() {
    let r = rig(false).await;
    let input = format!(
        "not json\n{{\"jsonrpc\":\"1.0\",\"id\":5,\"method\":\"node.info\"}}\n[]\n{}",
        req(6, "node.info", json!({}))
    );
    let rs = run(&r.node, input.into_bytes()).await;
    assert_eq!(rs.len(), 4);
    assert_eq!(
        rs.iter()
            .filter(|r| r.error.as_ref().is_some_and(|e| e.code == -32700))
            .count(),
        2,
        "two unparseable lines"
    );
    assert!(by_id(&rs, 5).error.is_some(), "wrong jsonrpc version");
    assert!(by_id(&rs, 6).result.is_some());
}

#[tokio::test]
async fn an_oversized_line_is_refused_and_does_not_poison_the_next() {
    let r = rig(false).await;
    let mut input = vec![b'x'; MAX_LINE_BYTES + 10];
    input.push(b'\n');
    input.extend_from_slice(req(7, "node.info", json!({})).as_bytes());
    let rs = run(&r.node, input).await;
    assert_eq!(rs.len(), 2);
    assert!(
        rs.iter()
            .any(|r| error_code(r) == Some(ErrorCode::PayloadTooLarge))
    );
    assert!(by_id(&rs, 7).result.is_some());
}

#[tokio::test]
async fn responses_never_echo_params() {
    let r = rig(false).await;
    let input = req(8, "local.ask", json!({ "secret": "hunter2-token" }));
    let out = serde_json::to_string(&run(&r.node, input.into_bytes()).await).unwrap();
    assert!(!out.contains("hunter2"));
}

#[tokio::test]
async fn many_requests_at_once_all_get_answers() {
    let r = rig(true).await;
    let input: String = (1..=300)
        .map(|i| {
            req(
                i,
                if i % 3 == 0 {
                    "node.info"
                } else {
                    "session.state"
                },
                json!({}),
            )
        })
        .collect();
    let rs = run(&r.node, input.into_bytes()).await;
    assert_eq!(rs.len(), 300);
    let mut ids: Vec<u64> = rs.iter().map(|r| r.id).collect();
    ids.sort_unstable();
    assert_eq!(
        ids,
        (1..=300).collect::<Vec<_>>(),
        "every request is answered exactly once"
    );
}

#[tokio::test]
async fn the_binary_speaks_the_protocol_over_real_pipes() {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};
    let mut child = Command::new(env!("CARGO_BIN_EXE_library-node"))
        .env("LIBRARY_GATEWAY_URL", "http://127.0.0.1:9")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    stdin
        .write_all(req(9, "node.info", json!({})).as_bytes())
        .unwrap();
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    assert!(
        line.contains("\"id\":9") && line.contains("library-node"),
        "{line}"
    );
    drop(stdin);
    assert!(
        child.wait().unwrap().success(),
        "node exits cleanly when stdin closes"
    );
}
