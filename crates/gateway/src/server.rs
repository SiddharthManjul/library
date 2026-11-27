//! WebSocket transport. Authentication is a bearer header on the upgrade request, so a token never
//! appears in a URL, a log line or a proxy's access log.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{
        ConnectInfo, DefaultBodyLimit, State, WebSocketUpgrade,
        ws::{CloseFrame, Message, WebSocket},
    },
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use library_core::{ErrorCode, LibraryError};
use library_protocol::{PROTOCOL_VERSION, rpc_error_code};
use serde_json::json;

use crate::{Gateway, Push, Session};

pub fn router(gw: Arc<Gateway>) -> Router {
    Router::new()
        .route("/v1/ws", get(upgrade))
        .route(
            "/v1/session/refresh",
            post(refresh).layer(DefaultBodyLimit::max(4 * 1024)),
        )
        .route("/healthz", get(|| async { "ok" }))
        .with_state(gw)
}

fn refuse(status: StatusCode, e: LibraryError) -> Response {
    let body =
        json!({ "error": { "code": rpc_error_code(e.code), "message": e.message, "data": e } });
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

async fn upgrade(
    State(gw): State<Arc<Gateway>>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    // A node and a gateway on different protocol versions refuse to talk, with a message that says what to do.
    let version = headers
        .get("x-library-protocol")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok());
    if version != Some(PROTOCOL_VERSION) {
        return refuse(
            StatusCode::UPGRADE_REQUIRED,
            LibraryError::new(
                ErrorCode::VersionMismatch,
                "This version of Library is out of date. Update the app to continue.",
            ),
        );
    }
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let Some(token) = token else {
        return refuse(
            StatusCode::UNAUTHORIZED,
            LibraryError::new(ErrorCode::Unauthenticated, "Sign in to continue."),
        );
    };
    let session = match gw.open(token).await {
        Ok(s) => s,
        Err(e) => {
            return refuse(
                if e.code == ErrorCode::Internal {
                    StatusCode::INTERNAL_SERVER_ERROR
                } else {
                    StatusCode::UNAUTHORIZED
                },
                e,
            );
        }
    };
    let max = gw.cfg.max_message_bytes;
    ws.max_message_size(max)
        .max_frame_size(max)
        .on_upgrade(move |socket| connection(gw, session, socket))
}

async fn connection(gw: Arc<Gateway>, mut session: Session, mut socket: WebSocket) {
    let Some(device) = session.ctx.device_id else {
        return;
    };
    let (mut inbox, generation) = gw.hub.register(device);
    loop {
        tokio::select! {
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    let response = gw.handle(&mut session, text.as_str()).await;
                    let Ok(out) = serde_json::to_string(&response) else { break };
                    if socket.send(Message::Text(out.into())).await.is_err() { break }
                }
                // JSON-RPC here is text only.
                Some(Ok(Message::Binary(_))) => {
                    let _ = socket.send(Message::Close(Some(CloseFrame { code: 1003, reason: "text only".into() }))).await;
                    break;
                }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                Some(Ok(_)) => {}
            },
            push = inbox.recv() => match push {
                Some(Push::Notify { name, payload }) => {
                    let msg = json!({ "jsonrpc": "2.0", "method": name, "params": payload }).to_string();
                    if socket.send(Message::Text(msg.into())).await.is_err() { break }
                }
                Some(Push::Disconnect) | None => {
                    let _ = socket.send(Message::Close(Some(CloseFrame { code: 1008, reason: "session ended".into() }))).await;
                    break;
                }
            },
        }
    }
    gw.hub.unregister(device, generation);
}

#[derive(serde::Deserialize)]
struct RefreshBody {
    refresh: String,
}

/// `POST /v1/session/refresh {"refresh": "..."}`. Answers with a new token pair, or a typed error.
async fn refresh(
    State(gw): State<Arc<Gateway>>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    body: Result<Json<RefreshBody>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Ok(Json(body)) = body else {
        return refuse(
            StatusCode::BAD_REQUEST,
            LibraryError::new(ErrorCode::InvalidParams, "A refresh token is required."),
        );
    };
    match gw.refresh_http(&body.refresh, &addr.ip().to_string()).await {
        Ok(pair) => (StatusCode::OK, Json(pair)).into_response(),
        Err(e) => {
            let status = match e.code {
                ErrorCode::RateLimited => StatusCode::TOO_MANY_REQUESTS,
                ErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
                _ => StatusCode::UNAUTHORIZED,
            };
            refuse(status, e)
        }
    }
}
