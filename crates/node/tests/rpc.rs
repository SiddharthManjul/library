//! The node's stdio protocol, with no database or keychain.
#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not

use std::io::Cursor;

use library_node::rpc::{MAX_LINE_BYTES, Node};
use library_protocol::{RpcResponse, SessionState};

fn run(input: &[u8]) -> Vec<RpcResponse> {
    let mut out = Vec::new();
    Node::new()
        .serve(Cursor::new(input.to_vec()), &mut out)
        .unwrap();
    String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn req(id: u64, method: &str) -> String {
    format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"{method}\",\"params\":{{}}}}\n")
}

#[test]
fn node_info_reports_version_and_protocol() {
    let r = &run(req(1, "node.info").as_bytes())[0];
    assert_eq!(r.id, 1);
    let v = r.result.as_ref().unwrap();
    assert_eq!(v["name"], "library-node");
    assert_eq!(v["protocol"], 1);
    assert!(
        v["platform"]
            .as_str()
            .unwrap()
            .contains(std::env::consts::OS)
    );
}

#[test]
fn a_fresh_node_is_signed_out() {
    let r = &run(req(2, "session.state").as_bytes())[0];
    let s: SessionState = serde_json::from_value(r.result.clone().unwrap()).unwrap();
    assert_eq!(s, SessionState::SignedOut);
}

#[test]
fn methods_that_need_a_session_fail_when_signed_out() {
    for m in ["local.ask", "local.sync_status", "local.permissions.decide"] {
        let r = &run(req(3, m).as_bytes())[0];
        let e = r
            .error
            .as_ref()
            .unwrap_or_else(|| panic!("{m} should fail"));
        assert_eq!(
            e.data.as_ref().unwrap().code,
            library_core::ErrorCode::Unauthenticated,
            "{m}"
        );
    }
}

#[test]
fn unknown_methods_and_gateway_methods_are_not_found() {
    for m in ["nope.nothing", "sync.subscribe", "dispatch.call", ""] {
        let r = &run(req(4, m).as_bytes())[0];
        assert_eq!(r.error.as_ref().unwrap().code, -32601, "{m:?}");
    }
}

#[test]
fn bad_input_gets_an_error_and_the_node_keeps_serving() {
    let input = format!(
        "not json\n{{\"jsonrpc\":\"1.0\",\"id\":5,\"method\":\"node.info\"}}\n[]\n{}",
        req(6, "node.info")
    );
    let rs = run(input.as_bytes());
    assert_eq!(rs.len(), 4);
    assert_eq!(rs[0].error.as_ref().unwrap().code, -32700);
    assert!(rs[1].error.is_some(), "wrong jsonrpc version");
    assert_eq!(rs[2].error.as_ref().unwrap().code, -32700);
    assert_eq!(rs[3].id, 6);
    assert!(rs[3].result.is_some());
}

#[test]
fn an_oversized_line_is_refused_and_does_not_poison_the_next() {
    let mut input = vec![b'x'; MAX_LINE_BYTES + 10];
    input.push(b'\n');
    input.extend_from_slice(req(7, "node.info").as_bytes());
    let rs = run(&input);
    assert_eq!(rs.len(), 2);
    assert_eq!(
        rs[0].error.as_ref().unwrap().data.as_ref().unwrap().code,
        library_core::ErrorCode::PayloadTooLarge
    );
    assert_eq!(rs[1].id, 7);
}

#[test]
fn responses_never_echo_params() {
    let input = "{\"jsonrpc\":\"2.0\",\"id\":8,\"method\":\"local.ask\",\"params\":{\"secret\":\"hunter2-token\"}}\n";
    let out = serde_json::to_string(&run(input.as_bytes())).unwrap();
    assert!(!out.contains("hunter2"));
}

#[test]
fn the_binary_speaks_the_protocol_over_real_pipes() {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};
    let mut child = Command::new(env!("CARGO_BIN_EXE_library-node"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    stdin.write_all(req(9, "node.info").as_bytes()).unwrap();
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    assert!(line.contains("\"id\":9") && line.contains("library-node"));
    drop(stdin);
    assert!(
        child.wait().unwrap().success(),
        "node exits cleanly when stdin closes"
    );
}
