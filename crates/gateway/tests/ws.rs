#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not
//! The real transport: an actual server on a local port and a real WebSocket client.

mod common;

use std::time::Duration;

use common::{Env, env};
use futures_util::{SinkExt, StreamExt};
use library_gateway::router;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::{self, Message, client::IntoClientRequest, http::HeaderValue};

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn serve(e: &Env) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(e.gw.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    addr
}

fn request(
    addr: std::net::SocketAddr,
    token: Option<&str>,
    version: Option<&str>,
) -> tungstenite::http::Request<()> {
    let mut req = format!("ws://{addr}/v1/ws").into_client_request().unwrap();
    if let Some(t) = token {
        req.headers_mut().insert(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {t}")).unwrap(),
        );
    }
    if let Some(v) = version {
        req.headers_mut()
            .insert("x-library-protocol", HeaderValue::from_str(v).unwrap());
    }
    req
}

async fn open(addr: std::net::SocketAddr, token: &str) -> Socket {
    tokio_tungstenite::connect_async(request(addr, Some(token), Some("1")))
        .await
        .unwrap()
        .0
}

async fn rpc(ws: &mut Socket, id: u64, method: &str, params: Value) -> Value {
    ws.send(Message::Text(
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    next_json(ws).await
}

async fn next_json(ws: &mut Socket) -> Value {
    loop {
        match tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("timed out waiting for the server")
            .expect("socket closed")
            .unwrap()
        {
            Message::Text(t) => return serde_json::from_str(t.as_str()).unwrap(),
            Message::Ping(_) | Message::Pong(_) => {}
            other => panic!("unexpected frame {other:?}"),
        }
    }
}

/// The HTTP status and error code a refused upgrade returns.
async fn refused(
    addr: std::net::SocketAddr,
    token: Option<&str>,
    version: Option<&str>,
) -> (u16, String) {
    match tokio_tungstenite::connect_async(request(addr, token, version)).await {
        Err(tungstenite::Error::Http(resp)) => {
            let body: Value = serde_json::from_slice(resp.body().as_deref().unwrap_or(b"{}"))
                .unwrap_or(Value::Null);
            (
                resp.status().as_u16(),
                body["error"]["data"]["code"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
            )
        }
        other => panic!(
            "expected a refused upgrade, got {:?}",
            other.map(|_| "connected")
        ),
    }
}

#[tokio::test]
async fn a_member_connects_and_calls_methods_over_the_socket() {
    let e = env().await;
    let addr = serve(&e).await;
    let c = e.connect(false).await;
    let mut ws = open(addr, &c.pair.access).await;
    let r = rpc(&mut ws, 1, "session.whoami", json!({})).await;
    assert_eq!(r["id"], 1);
    assert_eq!(r["result"]["user_id"], c.user.0.to_string());
    let w = rpc(&mut ws, 2, "usage.report", json!({})).await;
    assert_eq!(
        w["error"]["data"]["code"], "invalid_params",
        "a write without a key is refused over the wire too"
    );
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn the_upgrade_is_refused_without_valid_credentials_or_a_matching_version() {
    let e = env().await;
    let addr = serve(&e).await;
    let c = e.connect(false).await;
    assert_eq!(
        refused(addr, Some(&c.pair.access), None).await,
        (426, "version_mismatch".into())
    );
    assert_eq!(
        refused(addr, Some(&c.pair.access), Some("2")).await,
        (426, "version_mismatch".into())
    );
    assert_eq!(
        refused(addr, Some(&c.pair.access), Some("one")).await,
        (426, "version_mismatch".into())
    );
    assert_eq!(
        refused(addr, None, Some("1")).await,
        (401, "unauthenticated".into())
    );
    assert_eq!(
        refused(addr, Some("v1.garbage.garbage"), Some("1")).await,
        (401, "unauthenticated".into())
    );
    assert_eq!(
        refused(
            addr,
            Some("lrt1_refresh_tokens_are_not_access_tokens"),
            Some("1")
        )
        .await,
        (401, "unauthenticated".into())
    );
    e.clock.advance_ms(16 * 60 * 1000);
    assert_eq!(
        refused(addr, Some(&c.pair.access), Some("1")).await,
        (401, "token_expired".into())
    );
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn a_revoked_device_cannot_connect() {
    let e = env().await;
    let addr = serve(&e).await;
    let c = e.connect(false).await;
    sqlx::query("UPDATE devices SET revoked_at = now() WHERE id = $1")
        .bind(c.device.0)
        .execute(&e.db.admin)
        .await
        .unwrap();
    assert_eq!(
        refused(addr, Some(&c.pair.access), Some("1")).await,
        (401, "unauthenticated".into())
    );
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn the_token_travels_in_a_header_never_in_the_url() {
    let e = env().await;
    let addr = serve(&e).await;
    let c = e.connect(false).await;
    // A token in the query string is ignored: the connection is refused as unauthenticated.
    let url = format!("ws://{addr}/v1/ws?access_token={}", c.pair.access);
    let mut req = url.into_client_request().unwrap();
    req.headers_mut()
        .insert("x-library-protocol", HeaderValue::from_static("1"));
    match tokio_tungstenite::connect_async(req).await {
        Err(tungstenite::Error::Http(r)) => assert_eq!(r.status().as_u16(), 401),
        _ => panic!("a token in the URL must not authenticate"),
    }
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn notifications_are_pushed_on_the_same_socket() {
    let e = env().await;
    let addr = serve(&e).await;
    let c = e.connect(false).await;
    let mut ws = open(addr, &c.pair.access).await;
    rpc(&mut ws, 1, "session.whoami", json!({})).await; // the connection is registered once the first message is served
    assert!(
        e.gw.hub()
            .notify(c.device, "sync.delta", json!({ "seq": 5 }))
    );
    let n = next_json(&mut ws).await;
    assert_eq!(n["method"], "sync.delta");
    assert_eq!(n["params"]["seq"], 5);
    assert!(n.get("id").is_none(), "a notification has no id");
    assert!(
        !e.gw
            .hub()
            .notify(library_core::DeviceId::new(), "sync.delta", json!({})),
        "an unknown device gets nothing"
    );
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn a_second_connection_from_the_same_device_replaces_the_first() {
    let e = env().await;
    let addr = serve(&e).await;
    let c = e.connect(false).await;
    let mut first = open(addr, &c.pair.access).await;
    rpc(&mut first, 1, "session.whoami", json!({})).await;
    let mut second = open(addr, &c.pair.access).await;
    rpc(&mut second, 1, "session.whoami", json!({})).await;
    // The first is told to go away.
    let end = tokio::time::timeout(Duration::from_secs(5), first.next())
        .await
        .unwrap();
    assert!(matches!(
        end,
        Some(Ok(Message::Close(_))) | None | Some(Err(_))
    ));
    // The second keeps working, and the first one's cleanup did not remove its registration.
    assert!(e.gw.hub().notify(c.device, "task.updated", json!({})));
    assert_eq!(next_json(&mut second).await["method"], "task.updated");
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn disconnecting_a_device_closes_its_socket() {
    let e = env().await;
    let addr = serve(&e).await;
    let c = e.connect(false).await;
    let mut ws = open(addr, &c.pair.access).await;
    rpc(&mut ws, 1, "session.whoami", json!({})).await;
    assert!(e.gw.hub().disconnect(c.device));
    let end = tokio::time::timeout(Duration::from_secs(5), ws.next())
        .await
        .unwrap();
    assert!(matches!(
        end,
        Some(Ok(Message::Close(_))) | None | Some(Err(_))
    ));
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn binary_frames_and_oversized_messages_close_the_connection() {
    let e = env().await;
    let addr = serve(&e).await;
    let c = e.connect(false).await;
    let mut ws = open(addr, &c.pair.access).await;
    ws.send(Message::Binary(vec![1, 2, 3].into()))
        .await
        .unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .unwrap(),
        Some(Ok(Message::Close(_))) | None | Some(Err(_))
    ));

    let mut big = open(addr, &c.pair.access).await;
    let huge = json!({ "jsonrpc": "2.0", "id": 1, "method": "plugins.catalog", "params": { "pad": "x".repeat(2 * 1024 * 1024) } }).to_string();
    let _ = big.send(Message::Text(huge.into())).await;
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(5), big.next())
            .await
            .unwrap(),
        Some(Ok(Message::Close(_))) | None | Some(Err(_))
    ));
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn one_tenants_connection_cannot_see_another_tenants_data() {
    let e = env().await;
    let addr = serve(&e).await;
    let (a, b) = (e.connect(false).await, e.connect(true).await);
    let (mut wa, mut wb) = (
        open(addr, &a.pair.access).await,
        open(addr, &b.pair.access).await,
    );
    let (ra, rb) = (
        rpc(&mut wa, 1, "session.whoami", json!({})).await,
        rpc(&mut wb, 1, "session.whoami", json!({})).await,
    );
    assert_ne!(ra["result"]["tenant_id"], rb["result"]["tenant_id"]);
    let own = e.fx.a.scopes[0].to_string();
    let foreign = rpc(&mut wb, 2, "sync.snapshot", json!({ "scope_id": own })).await;
    assert_eq!(foreign["error"]["data"]["code"], "scope_denied");
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn the_health_endpoint_answers_without_credentials() {
    let e = env().await;
    let addr = serve(&e).await;
    let body = reqwest_free_get(addr, "/healthz").await;
    assert!(body.starts_with("HTTP/1.1 200"), "{body}");
    e.db.drop_db().await.unwrap();
}

/// A bare HTTP GET over TCP, to avoid pulling in an HTTP client for one check.
async fn reqwest_free_get(addr: std::net::SocketAddr, path: &str) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut s = tokio::net::TcpStream::connect(addr).await.unwrap();
    s.write_all(format!("GET {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").as_bytes())
        .await
        .unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).await.unwrap();
    out
}
