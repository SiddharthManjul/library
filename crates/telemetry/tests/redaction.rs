#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not
//! Redaction: known secrets are injected everywhere a log line can come from, and none may come out.

use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use library_telemetry::{RedactingMakeWriter, patterns_loaded, scrub, scrub_json};
use serde_json::json;
use tracing_subscriber::fmt::MakeWriter;

/// Every kind of credential the product handles, as it might appear in the wild.
const SECRETS: &[(&str, &str)] = &[
    (
        "library access token",
        "v1.eyJzdWIiOiIxMjM0NTY3ODkwIn0.c2lnbmF0dXJlLXNpZ25hdHVyZS1zaWduYXR1cmUtc2lnbmF0dXJl",
    ),
    (
        "library refresh token",
        "lrt1_Zm9vYmFyYmF6cXV4Zm9vYmFyYmF6cXV4Zm9v",
    ),
    ("bearer header", "Bearer abcDEF123456789xyzXYZ"),
    (
        "jwt",
        "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1c2VyIn0.dGhpcy1pcy1hLXNpZ25hdHVyZQ",
    ),
    ("slack token", "xoxp-1234567890-0987654321-abcdefghijklmnop"),
    ("github token", "ghp_abcdefghijklmnopqrstuvwxyz0123456789"),
    (
        "github fine-grained",
        "github_pat_11ABCDEFG0abcdefghijklmnopqrstuvwxyz0123456789",
    ),
    (
        "openai style key",
        "sk-proj-abcdefghijklmnopqrstuvwxyz0123456789",
    ),
    ("stripe live key", "sk_live_abcdefghijklmnop1234"),
    ("aws access key", "AKIAIOSFODNN7EXAMPLE"),
    ("google api key", "AIzaSyA-abcdefghijklmnopqrstuvwxyz012345"),
    (
        "master key hex",
        "0101010101010101010101010101010101010101010101010101010101010101",
    ),
    (
        "pem private key",
        "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIPrivateKeyMaterialHere\n-----END PRIVATE KEY-----",
    ),
];

/// A clonable in-memory log sink.
#[derive(Clone, Default)]
struct Sink(Arc<Mutex<Vec<u8>>>);

impl Write for Sink {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Sink {
    type Writer = Sink;

    fn make_writer(&'a self) -> Sink {
        self.clone()
    }
}

impl Sink {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

fn with_logging<R>(f: impl FnOnce() -> R) -> (R, String) {
    let sink = Sink::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(RedactingMakeWriter(sink.clone()))
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let r = tracing::subscriber::with_default(subscriber, f);
    (r, sink.text())
}

fn assert_no_secret(log: &str, how: &str) {
    for (name, secret) in SECRETS {
        // Compare on a core fragment too, so a partly masked secret still counts as a leak.
        let core = secret.lines().nth(1).unwrap_or(secret);
        let fragment = &core[core.len().saturating_sub(24)..];
        assert!(
            !log.contains(secret) && !log.contains(fragment),
            "{name} leaked through {how}:\n{log}"
        );
    }
}

#[test]
fn every_secret_kind_is_removed_from_plain_text() {
    for (name, secret) in SECRETS {
        let out = scrub(&format!("calling plugin with {secret} now")).into_owned();
        assert!(!out.contains(secret), "{name}: {out}");
        assert!(
            out.contains("calling plugin with") && out.contains("now"),
            "context must survive: {out}"
        );
    }
}

#[test]
fn secrets_do_not_survive_any_way_of_logging_them() {
    #[derive(Debug)]
    #[allow(dead_code)]
    struct Request {
        url: String,
        headers: Vec<(String, String)>,
        body: serde_json::Value,
    }
    let (_, log) = with_logging(|| {
        for (_, secret) in SECRETS {
            // As part of a message.
            tracing::info!("token is {secret} for the call");
            // As a structured field, plain and as Debug.
            tracing::info!(value = %secret, "field");
            tracing::warn!(value = ?secret, "debug field");
            // Inside a Debug dump of a larger struct.
            let req = Request {
                url: "https://api.example.com/x".into(),
                headers: vec![
                    ("Authorization".into(), format!("Bearer {secret}")),
                    ("X-Other".into(), (*secret).into()),
                ],
                body: json!({ "nested": { "k": secret } }),
            };
            tracing::debug!(?req, "request");
            // Inside an error string.
            let err = std::io::Error::other(format!("failed to call with {secret}"));
            tracing::error!(error = %err, "call failed");
            tracing::error!("{err:?}");
            // As a span field.
            let span = tracing::info_span!("call", secret = %secret);
            span.in_scope(|| tracing::info!("inside the span"));
            // In a URL.
            tracing::info!(url = %format!("https://host/path?access_token={secret}&x=1"), "url");
        }
    });
    assert_no_secret(&log, "the logger");
    assert!(
        log.contains("request")
            && log.contains("inside the span")
            && log.contains("api.example.com"),
        "ordinary context must survive redaction:\n{log}"
    );
}

#[test]
fn json_logs_are_scrubbed_too() {
    let sink = Sink::default();
    let sub = tracing_subscriber::fmt()
        .json()
        .with_writer(RedactingMakeWriter(sink.clone()))
        .finish();
    tracing::subscriber::with_default(sub, || {
        for (_, s) in SECRETS {
            tracing::info!(secret = %s, "x {s}");
        }
    });
    assert_no_secret(&sink.text(), "the JSON logger");
}

#[test]
fn named_fields_are_redacted_even_when_the_value_has_no_telltale_shape() {
    let cases = [
        "password=hunter2",
        "password: hunter2",
        "\"password\": \"hunter2\"",
        "client_secret = 'hunter2'",
        "Authorization: hunter2",
        "api_key=hunter2&other=1",
        "https://user:hunter2@db.example.com/x",
        "refresh: hunter2",
        "?token=hunter2&a=b",
        "?sig=hunter2",
    ];
    for c in cases {
        let out = scrub(c);
        assert!(!out.contains("hunter2"), "{c:?} -> {out}");
    }
}

#[test]
fn ordinary_text_is_left_alone() {
    for text in [
        "the token expired, sign in again",
        "token budget exceeded for run 12",
        "approved by user 123e4567-e89b-42d3-a456-426614174000",
        "synced 3 scopes in 120 ms",
        "state_root a1b2c3d4",
        "hash 5b97bbcb",
        "TokenExpired",
        "refresh the page",
        "password reset requested",
        "GET /v1/ws 101",
        "tenant=acme user=ravi method=sync.subscribe",
    ] {
        assert_eq!(scrub(text), text, "over-redacted: {text}");
    }
}

#[test]
fn json_values_are_scrubbed_by_key_and_by_content() {
    let v = json!({
        "name": "slack",
        "token": "anything-at-all",
        "Client-Secret": "x",
        "nested": { "api_key": "y", "note": format!("use {}", SECRETS[4].1), "ok": 3 },
        "list": [{ "refresh": "z" }, SECRETS[1].1],
        "empty": null,
        "access_token": null
    });
    let out = scrub_json(&v).to_string();
    for leaked in [
        "anything-at-all",
        "\"x\"",
        "\"y\"",
        "\"z\"",
        SECRETS[4].1,
        SECRETS[1].1,
    ] {
        assert!(!out.contains(leaked), "{leaked} leaked: {out}");
    }
    assert!(
        out.contains("slack") && out.contains("\"ok\":3"),
        "non-secrets survive: {out}"
    );
    assert!(out.contains("\"access_token\":null"), "a null stays null");
}

#[test]
fn a_secret_split_across_writes_is_still_caught() {
    let sink = Sink::default();
    {
        let mw = RedactingMakeWriter(sink.clone());
        let mut w = mw.make_writer();
        w.write_all(b"before lrt1_Zm9vYmFy").unwrap();
        w.write_all(b"YmF6cXV4Zm9vYmFyYmF6cXV4Zm9v after").unwrap();
    }
    let out = sink.text();
    assert!(!out.contains("lrt1_"), "{out}");
    assert!(out.contains("before") && out.contains("after"));
}

#[test]
fn long_input_is_handled_without_blowing_up() {
    let big = format!("{} {} {}", "word ".repeat(200_000), SECRETS[1].1, "tail");
    let out = scrub(&big);
    assert!(!out.contains("lrt1_") && out.ends_with("tail"));
}

#[test]
fn every_redaction_pattern_compiles() {
    // A pattern that fails to compile used to vanish silently, taking its protection with it.
    assert!(patterns_loaded());
}

#[test]
fn terminal_colour_codes_next_to_a_secret_do_not_hide_it() {
    let secret = SECRETS[0].1;
    let coloured = format!("\x1b[3mvalue\x1b[0m\x1b[2m=\x1b[0m{secret}");
    let sink = Sink::default();
    {
        let mut w = RedactingMakeWriter(sink.clone()).make_writer();
        w.write_all(coloured.as_bytes()).unwrap();
    }
    assert!(!sink.text().contains(secret), "{}", sink.text());
    assert!(sink.text().contains("value"), "the field name survives");
    // And the scrubber alone does not rely on a word boundary either.
    assert!(!scrub(&format!("m{secret}")).contains(secret));
}

#[test]
fn credentials_in_urls_are_removed_but_the_host_stays() {
    let out = scrub("connect to postgres://app:hunter2@db.internal:5432/library please");
    assert!(!out.contains("hunter2") && !out.contains("app:"), "{out}");
    assert!(
        out.contains("postgres://") && out.contains("@db.internal:5432/library"),
        "{out}"
    );
}

#[test]
fn overlapping_formats_do_not_hide_each_other() {
    // A bearer header carrying a PEM key: the generic Bearer pattern must not eat the PEM header.
    let pem = SECRETS[12].1;
    let out = scrub(&format!("Authorization: Bearer {pem}")).into_owned();
    assert!(
        !out.contains("PrivateKeyMaterialHere") && !out.contains("MC4CAQ"),
        "{out}"
    );
    // The same, with newlines escaped as a Debug dump writes them.
    let escaped = pem.replace('\n', "\\n");
    let out = scrub(&format!(
        "headers: [(\"Authorization\", \"Bearer {escaped}\")]"
    ))
    .into_owned();
    assert!(
        !out.contains("PrivateKeyMaterialHere") && !out.contains("MC4CAQ"),
        "{out}"
    );
    // A token inside a hex run, and a hex key inside a named field.
    assert!(!scrub(&format!("key={}", SECRETS[11].1)).contains("0101010101"));
}
