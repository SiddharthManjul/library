//! The node's stdio JSON-RPC server. The desktop app spawns the node and speaks one JSON object per
//! line. stdout carries only protocol messages; diagnostics go to stderr and never include params.

use std::io::{BufRead, Write};

use library_core::{ErrorCode, LibraryError};
use library_protocol::{
    Access, Link, NodeInfo, PROTOCOL_VERSION, RpcRequest, RpcResponse, SessionState, find,
};

/// A single message larger than this is refused, so a broken or hostile peer cannot exhaust memory.
pub const MAX_LINE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Default)]
pub struct Node {
    session: Option<SessionState>,
}

impl Node {
    pub fn new() -> Self {
        Self::default()
    }

    fn session(&self) -> SessionState {
        self.session.clone().unwrap_or(SessionState::SignedOut)
    }

    /// Answers one request. Unknown methods, methods meant for another link and methods that need a
    /// session all fail with a typed error; nothing panics on bad input.
    pub fn handle(&self, req: RpcRequest) -> RpcResponse {
        let id = req.id;
        if req.jsonrpc != "2.0" {
            return RpcResponse::err(
                id,
                LibraryError::new(ErrorCode::InvalidParams, "Only JSON-RPC 2.0 is supported."),
            );
        }
        let Some(decl) = find(&req.method).filter(|m| m.link == Link::AppToNode) else {
            return RpcResponse::method_not_found(id);
        };
        if matches!(decl.access, Access::Member | Access::Roles(_))
            && matches!(self.session(), SessionState::SignedOut)
        {
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
            "session.state" => to_result(id, &self.session()),
            _ => RpcResponse::err(
                id,
                LibraryError::new(ErrorCode::Unavailable, "This is not available yet."),
            ),
        }
    }

    /// Reads requests line by line until the input closes. Returns the number handled.
    pub fn serve<R: BufRead, W: Write>(&self, mut input: R, mut output: W) -> std::io::Result<u64> {
        let (mut handled, mut line) = (0u64, Vec::new());
        loop {
            line.clear();
            // Read at most one byte past the limit so an oversized line is detected without buffering it all.
            let n = (&mut input)
                .take(MAX_LINE_BYTES as u64 + 1)
                .read_until(b'\n', &mut line)?;
            if n == 0 {
                return Ok(handled);
            }
            let response = if line.len() > MAX_LINE_BYTES {
                discard_rest_of_line(&mut input, line.last() == Some(&b'\n'))?;
                RpcResponse::err(
                    0,
                    LibraryError::new(ErrorCode::PayloadTooLarge, "That message is too large."),
                )
            } else {
                match serde_json::from_slice::<RpcRequest>(&line) {
                    Ok(req) => self.handle(req),
                    Err(_) => RpcResponse::parse_error(),
                }
            };
            serde_json::to_writer(&mut output, &response).map_err(std::io::Error::other)?;
            output.write_all(b"\n")?;
            output.flush()?;
            handled += 1;
        }
    }
}

fn to_result<T: serde::Serialize>(id: u64, v: &T) -> RpcResponse {
    match serde_json::to_value(v) {
        Ok(v) => RpcResponse::ok(id, v),
        Err(_) => RpcResponse::err(id, LibraryError::internal()),
    }
}

fn discard_rest_of_line<R: BufRead>(input: &mut R, already_ended: bool) -> std::io::Result<()> {
    if already_ended {
        return Ok(());
    }
    let mut sink = Vec::new();
    loop {
        sink.clear();
        let n = input.take(64 * 1024).read_until(b'\n', &mut sink)?;
        if n == 0 || sink.last() == Some(&b'\n') {
            return Ok(());
        }
    }
}

use std::io::Read;
