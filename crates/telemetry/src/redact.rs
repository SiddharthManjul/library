use std::{borrow::Cow, sync::LazyLock};

use regex::Regex;

pub const REDACTED: &str = "[redacted]";

/// Whole-match patterns: the secret is the entire match. Prefix formats carry no `\b` anchor, because
/// escape codes and punctuation next to a secret must not hide it.
fn compile_whole() -> Result<Vec<Regex>, regex::Error> {
    [
        // Order matters. The longest, most specific patterns run first, so a generic one (Bearer, a hex run)
        // cannot eat part of a structure and leave the rest unrecognisable. PEM keys come first.
        // PEM private keys.
        r"(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----",
        // Library's own tokens.
        r"v1\.[A-Za-z0-9_-]{16,}\.[A-Za-z0-9_-]{16,}",
        r"lrt1_[A-Za-z0-9_-]{8,}",
        // Standard bearer credentials and JWTs.
        r"(?i)\bbearer\s+[A-Za-z0-9._~+/=-]{8,}",
        r"eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}",
        // Vendor tokens in the catalog and the usual suspects.
        r"xox[abprs]-[A-Za-z0-9-]{10,}",
        r"gh[pousr]_[A-Za-z0-9]{20,}",
        r"github_pat_[A-Za-z0-9_]{20,}",
        r"\bsk-[A-Za-z0-9_-]{20,}",
        r"\b[sr]k_(?:live|test)_[A-Za-z0-9]{16,}",
        r"AKIA[0-9A-Z]{16}\b",
        r"AIza[0-9A-Za-z_-]{30,}",
        // Key material: 256-bit keys and longer written in hex.
        r"\b[0-9a-fA-F]{64,}\b",
    ]
    .iter()
    .map(|p| Regex::new(p))
    .collect()
}

static PATTERNS: LazyLock<Result<Patterns, regex::Error>> = LazyLock::new(|| {
    Ok(Patterns {
        whole: compile_whole()?,
        // scheme://user:password@host keeps the scheme and host and drops the credentials.
        url_credentials: Regex::new(r"(://)[^\s/@:]+:[^\s/@]+(@)")?,
        ansi: Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]")?,
        named: Regex::new(NAMED_PATTERN)?,
        query: Regex::new(QUERY_PATTERN)?,
    })
});

struct Patterns {
    whole: Vec<Regex>,
    url_credentials: Regex,
    ansi: Regex,
    named: Regex,
    query: Regex,
}

/// True when every redaction pattern compiled. Checked by a test, and by `scrub`, which withholds all
/// output when it is false rather than letting secrets through.
pub fn patterns_loaded() -> bool {
    PATTERNS.is_ok()
}

pub const WITHHELD: &str = "[log line withheld: redaction is unavailable]";

/// `name = value`, `name: value` and `"name": "value"` where the name says the value is secret.
/// Only the value is replaced, so the log still shows that the field existed.
const NAMED_PATTERN: &str = r#"(?i)(\b(?:password|passwd|secret|token|api[_-]?key|authorization|refresh|access[_-]?token|client[_-]?secret|signing[_-]?secret|private[_-]?key|master[_-]?keys?|credentials?|ciphertext|wrapped[_-]?dek|dek)\b["']?\s*[=:]\s*)("[^"]*"|'[^']*'|[^\s,;&}\]"']+)"#;

/// `?access_token=...` style query parameters.
const QUERY_PATTERN: &str = r"(?i)([?&](?:access_token|refresh_token|token|key|api_key|secret|password|code|signature|sig)=)[^&\s#]+";

/// Removes anything that looks like a credential. Cheap when there is nothing to remove.
pub fn scrub(text: &str) -> Cow<'_, str> {
    let Ok(p) = PATTERNS.as_ref() else {
        return Cow::Borrowed(WITHHELD);
    };
    let mut out = Cow::Borrowed(text);
    for re in &p.whole {
        if re.is_match(&out) {
            out = Cow::Owned(re.replace_all(&out, REDACTED).into_owned());
        }
    }
    if p.url_credentials.is_match(&out) {
        out = Cow::Owned(
            p.url_credentials
                .replace_all(&out, format!("${{1}}{REDACTED}${{2}}"))
                .into_owned(),
        );
    }
    if p.query.is_match(&out) {
        out = Cow::Owned(
            p.query
                .replace_all(&out, format!("${{1}}{REDACTED}"))
                .into_owned(),
        );
    }
    if p.named.is_match(&out) {
        out = Cow::Owned(
            p.named
                .replace_all(&out, format!("${{1}}{REDACTED}"))
                .into_owned(),
        );
    }
    out
}

/// Removes terminal colour codes. Log files are for machines and for redaction; escape codes sitting
/// between a field name and its value would otherwise hide the value from every pattern.
pub fn strip_ansi(text: &str) -> Cow<'_, str> {
    match PATTERNS.as_ref() {
        Ok(p) if p.ansi.is_match(text) => Cow::Owned(p.ansi.replace_all(text, "").into_owned()),
        Ok(_) => Cow::Borrowed(text),
        Err(_) => Cow::Borrowed(text),
    }
}

/// Scrubs every string in a JSON value, and replaces whole values whose key names a secret.
pub fn scrub_json(v: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    const SECRET_KEYS: [&str; 14] = [
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
        "apikey",
        "authorization",
        "refresh",
        "access_token",
        "client_secret",
        "signing_secret",
        "private_key",
        "ciphertext",
        "credentials",
    ];
    match v {
        Value::String(s) => Value::String(scrub(s).into_owned()),
        Value::Array(a) => Value::Array(a.iter().map(scrub_json).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, val)| {
                    let key = k.to_ascii_lowercase().replace('-', "_");
                    let secret = SECRET_KEYS
                        .iter()
                        .any(|s| key == *s || key.ends_with(&format!("_{s}")));
                    (
                        k.clone(),
                        if secret && !val.is_null() {
                            Value::String(REDACTED.into())
                        } else {
                            scrub_json(val)
                        },
                    )
                })
                .collect(),
        ),
        other => other.clone(),
    }
}
