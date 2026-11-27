use library_core::TenantId;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

const ROW_TAG: &[u8] = b"library-audit-v1\0";
const GENESIS_TAG: &[u8] = b"library-audit-genesis-v1\0";

/// What a caller records. `actor` names who acted ("user:<id>", "agent:<id>", "system"); `action` says
/// what happened ("tool.call", "approval.decide"); `payload_hash` commits to the details, which are not
/// stored here so the log never holds secrets or personal data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntry {
    pub actor: String,
    pub action: String,
    pub target: Option<String>,
    /// The pinned hash of the tool definition, for plugin calls.
    pub tool_hash: Option<[u8; 32]>,
    pub payload_hash: [u8; 32],
}

/// A stored row, as read back for verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub seq: i64,
    pub entry: AuditEntry,
    /// Microseconds since the Unix epoch, as Postgres stores them.
    pub created_us: i64,
    pub prev_hash: [u8; 32],
    pub hash: [u8; 32],
}

pub fn hash_payload(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// `prev_hash` of the first row: fixed per tenant, so a chain cannot be moved to another tenant.
pub fn genesis_hash(tenant: TenantId) -> [u8; 32] {
    Sha256::digest([GENESIS_TAG, tenant.0.as_bytes()].concat()).into()
}

pub(crate) fn micros(t: OffsetDateTime) -> i64 {
    i64::try_from(t.unix_timestamp_nanos() / 1_000).unwrap_or(0)
}

fn put_len(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&u32::try_from(bytes.len()).unwrap_or(u32::MAX).to_be_bytes());
    out.extend_from_slice(bytes);
}

fn put_opt(out: &mut Vec<u8>, v: Option<&[u8]>) {
    match v {
        Some(b) => {
            out.push(1);
            put_len(out, b);
        }
        None => out.push(0),
    }
}

/// Length-prefixed binary, never JSON: the same bytes on every machine, whatever the serializer.
fn canonical(tenant: TenantId, seq: i64, created_us: i64, e: &AuditEntry) -> Vec<u8> {
    let mut out = Vec::with_capacity(160);
    out.extend_from_slice(ROW_TAG);
    out.extend_from_slice(tenant.0.as_bytes());
    out.extend_from_slice(&seq.to_be_bytes());
    out.extend_from_slice(&created_us.to_be_bytes());
    put_len(&mut out, e.actor.as_bytes());
    put_len(&mut out, e.action.as_bytes());
    put_opt(&mut out, e.target.as_deref().map(str::as_bytes));
    put_opt(&mut out, e.tool_hash.as_ref().map(<[u8; 32]>::as_slice));
    out.extend_from_slice(&e.payload_hash);
    out
}

/// `SHA-256(prev_hash || canonical(row))`.
pub fn row_hash(
    tenant: TenantId,
    prev: &[u8; 32],
    seq: i64,
    created_us: i64,
    e: &AuditEntry,
) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(prev);
    h.update(canonical(tenant, seq, created_us, e));
    h.finalize().into()
}
