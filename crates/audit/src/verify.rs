use ed25519_dalek::VerifyingKey;
use library_core::TenantId;

use crate::{
    canonical::{Row, genesis_hash, row_hash},
    head::SignedHead,
};

/// What is wrong with a chain, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// A row's stored hash does not match its content: the row was edited.
    RowChanged { seq: i64 },
    /// A row does not point at its predecessor: a row was removed or inserted before it.
    BrokenLink { seq: i64 },
    /// Sequence numbers skip: rows are missing.
    Gap { expected: i64, found: i64 },
    /// The latest signed head is not on the chain: the end of the log was cut off, or rewritten.
    HeadNotOnChain { seq: i64 },
    /// The chain is shorter than the latest signed head.
    TruncatedBeforeHead { head_seq: i64, chain_len: i64 },
    /// A stored head carries a signature that does not verify.
    ForgedHead { day: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    pub rows: usize,
    pub problems: Vec<Problem>,
}

impl Report {
    pub fn is_ok(&self) -> bool {
        self.problems.is_empty()
    }
}

/// Checks the whole chain, then the signed heads against it. Reports every problem, not just the first.
pub fn verify_chain(
    tenant: TenantId,
    rows: &[Row],
    heads: &[SignedHead],
    key: &VerifyingKey,
) -> Report {
    let mut report = Report {
        rows: rows.len(),
        problems: vec![],
    };
    let mut prev = genesis_hash(tenant);
    let mut expected = 1;
    for r in rows {
        if r.seq != expected {
            report.problems.push(Problem::Gap {
                expected,
                found: r.seq,
            });
            expected = r.seq;
        }
        if r.prev_hash != prev {
            report.problems.push(Problem::BrokenLink { seq: r.seq });
        }
        if row_hash(tenant, &r.prev_hash, r.seq, r.created_us, &r.entry) != r.hash {
            report.problems.push(Problem::RowChanged { seq: r.seq });
        }
        // Continue from the stored hash so one bad row is reported once, not on every row after it.
        prev = r.hash;
        expected += 1;
    }

    let mut valid: Vec<&SignedHead> = vec![];
    for h in heads {
        if h.is_valid(key) {
            valid.push(h);
        } else {
            report
                .problems
                .push(Problem::ForgedHead { day: h.day.clone() });
        }
    }
    if let Some(latest) = valid.iter().max_by_key(|h| h.seq) {
        let chain_len = rows.last().map_or(0, |r| r.seq);
        if chain_len < latest.seq {
            report.problems.push(Problem::TruncatedBeforeHead {
                head_seq: latest.seq,
                chain_len,
            });
        } else if rows
            .iter()
            .find(|r| r.seq == latest.seq)
            .is_none_or(|r| r.hash != latest.hash)
        {
            report
                .problems
                .push(Problem::HeadNotOnChain { seq: latest.seq });
        }
    }
    report
}
