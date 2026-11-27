//! The real network: `wss://` through the operating system's certificate store (so it works behind a
//! company's TLS-inspecting proxy), and HTTPS for the refresh call.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use library_core::{ErrorCode, LibraryError};
use library_protocol::TokenPairWire;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{self, Message, client::IntoClientRequest, http::HeaderValue},
};

use crate::link::{ConnectError, Connection, Transport, upgrade_headers};

pub struct GatewayTransport {
    /// `https://api.example.com` (no trailing slash). The WebSocket uses the matching `wss://` URL.
    base: String,
}

impl GatewayTransport {
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_string(),
        }
    }

    fn ws_url(&self) -> String {
        if let Some(rest) = self.base.strip_prefix("https://") {
            format!("wss://{rest}/v1/ws")
        } else if let Some(rest) = self.base.strip_prefix("http://") {
            format!("ws://{rest}/v1/ws")
        } else {
            format!("wss://{}/v1/ws", self.base)
        }
    }
}

pub struct WsConnection(WebSocketStream<MaybeTlsStream<TcpStream>>);

impl Connection for WsConnection {
    async fn send(&mut self, text: String) -> bool {
        self.0.send(Message::Text(text.into())).await.is_ok()
    }

    async fn recv(&mut self) -> Option<String> {
        loop {
            match self.0.next().await? {
                Ok(Message::Text(t)) => return Some(t.as_str().to_string()),
                Ok(Message::Close(_)) | Err(_) => return None,
                // Pings are answered by the library; anything else is not part of the protocol.
                Ok(_) => {}
            }
        }
    }
}

/// The gateway's typed refusal inside an HTTP error body, when there is one.
fn typed_error(body: &str) -> Option<LibraryError> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get("error")?
        .get("data")
        .and_then(|d| serde_json::from_value(d.clone()).ok())
}

impl Transport for GatewayTransport {
    type Conn = WsConnection;

    async fn connect(&self, access: &str) -> Result<WsConnection, ConnectError> {
        let mut req = self
            .ws_url()
            .into_client_request()
            .map_err(|_| ConnectError::Offline)?;
        for (name, value) in upgrade_headers(access) {
            req.headers_mut().insert(
                name,
                HeaderValue::from_str(&value).map_err(|_| ConnectError::Offline)?,
            );
        }
        match tokio::time::timeout(
            Duration::from_secs(15),
            tokio_tungstenite::connect_async(req),
        )
        .await
        {
            Ok(Ok((ws, _))) => Ok(WsConnection(ws)),
            Ok(Err(tungstenite::Error::Http(resp))) => {
                let body = resp
                    .body()
                    .as_deref()
                    .map(|b| String::from_utf8_lossy(b).into_owned())
                    .unwrap_or_default();
                let err = typed_error(&body).unwrap_or_else(|| {
                    LibraryError::new(ErrorCode::Unauthenticated, "Sign in again to continue.")
                });
                match resp.status().as_u16() {
                    426 => Err(ConnectError::Outdated(err)),
                    401 => Err(ConnectError::Unauthorized(err)),
                    _ => Err(ConnectError::Offline),
                }
            }
            _ => Err(ConnectError::Offline),
        }
    }

    async fn refresh(&self, refresh: &str) -> Result<TokenPairWire, LibraryError> {
        let url = format!("{}/v1/session/refresh", self.base);
        let body = serde_json::json!({ "refresh": refresh });
        // ureq is blocking, so it runs on the blocking pool and never stalls the runtime.
        let result = tokio::task::spawn_blocking(move || {
            // The agent uses rustls with the platform verifier: Windows' and Linux's own certificate store.
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .tls_config(
                    ureq::tls::TlsConfig::builder()
                        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                        .build(),
                )
                .timeout_global(Some(Duration::from_secs(15)))
                .http_status_as_error(false)
                .build()
                .into();
            agent.post(&url).send_json(&body).map(|mut r| {
                (
                    r.status().as_u16(),
                    r.body_mut().read_to_string().unwrap_or_default(),
                )
            })
        })
        .await;
        match result {
            Ok(Ok((200, text))) => {
                serde_json::from_str(&text).map_err(|_| LibraryError::internal())
            }
            Ok(Ok((status, text))) => Err(typed_error(&text).unwrap_or_else(|| {
                if status == 429 {
                    LibraryError::new(
                        ErrorCode::RateLimited,
                        "Too many requests. Try again shortly.",
                    )
                } else {
                    LibraryError::new(ErrorCode::Unavailable, "Cannot reach Library right now.")
                }
            })),
            _ => Err(LibraryError::new(
                ErrorCode::Unavailable,
                "Cannot reach Library right now.",
            )),
        }
    }
}
