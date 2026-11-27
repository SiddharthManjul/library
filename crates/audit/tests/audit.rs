#![allow(clippy::unwrap_used, clippy::expect_used)] // integration tests may unwrap; production code may not
//! The audit chain against a real database and an attacker who can bypass the table's protections.

use ed25519_dalek::SigningKey;
use library_audit::{
    AuditEntry, AuditLog, FileHeadStore, HeadSigner, HeadStore, Problem, Row, genesis_hash,
    hash_payload, verify_chain,
};
use library_core::{Plan, Principal, RequestCtx, Role, Taint, TenantId, TraceId, UserId};
use library_testkit::{TestDb, seed_two_tenants};
use time::OffsetDateTime;

fn ctx(tenant: uuid::Uuid, user: uuid::Uuid) -> RequestCtx {
    RequestCtx {
        tenant_id: TenantId(tenant),
        principal: Principal::User {
            user_id: UserId(user),
        },
        roles: vec![Role::Admin],
        scopes: vec![],
        plan: Plan::Business,
        taint: Taint::Clean,
        trace_id: TraceId::new(),
        device_id: None,
    }
}

fn entry(n: u8) -> AuditEntry {
    AuditEntry {
        actor: format!("user:{n}"),
        action: "tool.call".into(),
        target: Some(format!("target-{n}")),
        tool_hash: Some([n; 32]),
        payload_hash: hash_payload(&[n]),
    }
}

struct Env {
    db: TestDb,
    log: AuditLog,
    ctx: RequestCtx,
    other: RequestCtx,
}

async fn env() -> Env {
    let db = TestDb::new().await.unwrap();
    let fx = seed_two_tenants(&db).await.unwrap();
    // The fixture seeds one arbitrary audit row per tenant; start from an empty log so chains are ours.
    sqlx::query("ALTER TABLE audit_log DISABLE TRIGGER audit_log_no_update")
        .execute(&db.admin)
        .await
        .unwrap();
    sqlx::query("DELETE FROM audit_log")
        .execute(&db.admin)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE audit_log ENABLE TRIGGER audit_log_no_update")
        .execute(&db.admin)
        .await
        .unwrap();
    let log = AuditLog::new(db.app.clone());
    Env {
        log,
        ctx: ctx(fx.a.id, fx.a.user),
        other: ctx(fx.b.id, fx.b.user),
        db,
    }
}

/// What a database superuser can do: switch the guard trigger off, change history, switch it back on.
async fn as_attacker(db: &TestDb, sql: &str) {
    sqlx::query("ALTER TABLE audit_log DISABLE TRIGGER USER")
        .execute(&db.admin)
        .await
        .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(sql.to_string()))
        .execute(&db.admin)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE audit_log ENABLE TRIGGER USER")
        .execute(&db.admin)
        .await
        .unwrap();
}

async fn fill(e: &Env, n: u8) {
    for i in 1..=n {
        e.log.append(&e.ctx, &entry(i)).await.unwrap();
    }
}

fn key() -> (HeadSigner, ed25519_dalek::VerifyingKey) {
    let s = HeadSigner::from_seed([5; 32]);
    let k = s.public_key();
    (s, k)
}

async fn verify(e: &Env, heads: &[library_audit::SignedHead]) -> library_audit::Report {
    let rows = e.log.read_all(&e.ctx).await.unwrap();
    verify_chain(e.ctx.tenant_id, &rows, heads, &key().1)
}

#[tokio::test]
async fn an_honest_log_verifies_and_numbers_rows_from_one_without_gaps() {
    let e = env().await;
    fill(&e, 6).await;
    let rows = e.log.read_all(&e.ctx).await.unwrap();
    assert_eq!(
        rows.iter().map(|r| r.seq).collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5, 6]
    );
    assert_eq!(rows[0].prev_hash, genesis_hash(e.ctx.tenant_id));
    assert_eq!(rows[1].prev_hash, rows[0].hash);
    let report = verify(&e, &[]).await;
    assert!(report.is_ok(), "{report:?}");
    assert_eq!(report.rows, 6);
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn an_empty_log_verifies() {
    let e = env().await;
    assert!(verify(&e, &[]).await.is_ok());
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn concurrent_appends_never_fork_or_skip_the_chain() {
    let e = env().await;
    let entries: Vec<AuditEntry> = (0..20u8).map(entry).collect();
    let futures = entries.iter().map(|en| e.log.append(&e.ctx, en));
    for r in futures::future::join_all(futures).await {
        r.unwrap();
    }
    let report = verify(&e, &[]).await;
    assert!(report.is_ok(), "{report:?}");
    assert_eq!(report.rows, 20);
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn editing_a_row_is_detected() {
    let e = env().await;
    fill(&e, 5).await;
    as_attacker(
        &e.db,
        &format!(
            "UPDATE audit_log SET action = 'nothing.happened' WHERE tenant_id = '{}' AND seq = 3",
            e.ctx.tenant_id
        ),
    )
    .await;
    assert_eq!(
        verify(&e, &[]).await.problems,
        vec![Problem::RowChanged { seq: 3 }]
    );
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn editing_a_row_and_recomputing_its_hash_breaks_the_next_link() {
    let e = env().await;
    fill(&e, 4).await;
    let rows = e.log.read_all(&e.ctx).await.unwrap();
    // The attacker rewrites row 2 consistently, hash included.
    let mut forged = rows[1].clone();
    forged.entry.actor = "user:someone-else".into();
    let report = {
        let mut all = rows.clone();
        all[1] = Row {
            hash: library_audit_row_hash(e.ctx.tenant_id, &forged),
            ..forged
        };
        verify_chain(e.ctx.tenant_id, &all, &[], &key().1)
    };
    assert_eq!(report.problems, vec![Problem::BrokenLink { seq: 3 }]);
    e.db.drop_db().await.unwrap();
}

// Recomputes a row hash the way an attacker with the source code would.
fn library_audit_row_hash(tenant: TenantId, r: &Row) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let lp = |b: &[u8]| [(b.len() as u32).to_be_bytes().as_slice(), b].concat();
    let opt = |b: Option<&[u8]>| b.map_or(vec![0], |b| [vec![1], lp(b)].concat());
    let canon = [
        b"library-audit-v1\0".as_slice(),
        tenant.0.as_bytes(),
        &r.seq.to_be_bytes(),
        &r.created_us.to_be_bytes(),
        &lp(r.entry.actor.as_bytes()),
        &lp(r.entry.action.as_bytes()),
        &opt(r.entry.target.as_deref().map(str::as_bytes)),
        &opt(r.entry.tool_hash.as_ref().map(<[u8; 32]>::as_slice)),
        &r.entry.payload_hash,
    ]
    .concat();
    Sha256::digest([r.prev_hash.as_slice(), &canon].concat()).into()
}

#[tokio::test]
async fn deleting_a_row_from_the_middle_is_detected() {
    let e = env().await;
    fill(&e, 5).await;
    as_attacker(
        &e.db,
        &format!(
            "DELETE FROM audit_log WHERE tenant_id = '{}' AND seq = 3",
            e.ctx.tenant_id
        ),
    )
    .await;
    let report = verify(&e, &[]).await;
    assert!(
        report.problems.contains(&Problem::Gap {
            expected: 3,
            found: 4
        }),
        "{report:?}"
    );
    assert!(
        report.problems.contains(&Problem::BrokenLink { seq: 4 }),
        "{report:?}"
    );
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn inserting_a_row_is_detected() {
    let e = env().await;
    fill(&e, 3).await;
    let rows = e.log.read_all(&e.ctx).await.unwrap();
    let fake = Row {
        seq: 4,
        prev_hash: [0; 32],
        ..rows[2].clone()
    };
    let mut all = rows.clone();
    all.push(Row {
        hash: library_audit_row_hash(e.ctx.tenant_id, &fake),
        ..fake
    });
    let report = verify_chain(e.ctx.tenant_id, &all, &[], &key().1);
    assert!(
        report.problems.contains(&Problem::BrokenLink { seq: 4 }),
        "{report:?}"
    );
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn cutting_off_the_end_is_caught_only_by_the_signed_head() {
    let e = env().await;
    fill(&e, 5).await;
    let rows = e.log.read_all(&e.ctx).await.unwrap();
    let (signer, _) = key();
    let head = signer.sign(e.ctx.tenant_id, "2026-11-26", 5, rows[4].hash);
    as_attacker(
        &e.db,
        &format!(
            "DELETE FROM audit_log WHERE tenant_id = '{}' AND seq > 3",
            e.ctx.tenant_id
        ),
    )
    .await;
    // The chain alone cannot see that anything is missing.
    assert!(verify(&e, &[]).await.is_ok());
    // The head signed before the cut can.
    assert_eq!(
        verify(&e, &[head]).await.problems,
        vec![Problem::TruncatedBeforeHead {
            head_seq: 5,
            chain_len: 3
        }]
    );
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn replacing_the_tail_with_a_different_history_is_caught() {
    let e = env().await;
    fill(&e, 4).await;
    let rows = e.log.read_all(&e.ctx).await.unwrap();
    let head = key().0.sign(e.ctx.tenant_id, "2026-11-26", 4, rows[3].hash);
    // Rewrite the last row, consistently, so the chain itself still verifies.
    let mut forged = rows[3].clone();
    forged.entry.action = "tool.call.hidden".into();
    forged.hash = library_audit_row_hash(e.ctx.tenant_id, &forged);
    let mut all = rows.clone();
    all[3] = forged;
    let report = verify_chain(e.ctx.tenant_id, &all, &[head], &key().1);
    assert_eq!(report.problems, vec![Problem::HeadNotOnChain { seq: 4 }]);
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn a_forged_head_is_reported_and_never_trusted() {
    let e = env().await;
    fill(&e, 3).await;
    let rows = e.log.read_all(&e.ctx).await.unwrap();
    let attacker =
        HeadSigner::from_seed([99; 32]).sign(e.ctx.tenant_id, "2026-11-26", 3, rows[2].hash);
    let report = verify(&e, std::slice::from_ref(&attacker)).await;
    assert_eq!(
        report.problems,
        vec![Problem::ForgedHead {
            day: "2026-11-26".into()
        }]
    );
    // A forged head claiming a longer log must not be believed either.
    let longer = HeadSigner::from_seed([99; 32]).sign(e.ctx.tenant_id, "2026-11-27", 50, [1; 32]);
    assert_eq!(
        verify(&e, &[longer]).await.problems,
        vec![Problem::ForgedHead {
            day: "2026-11-27".into()
        }]
    );
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn a_chain_cannot_be_moved_to_another_tenant() {
    let e = env().await;
    fill(&e, 3).await;
    let rows = e.log.read_all(&e.ctx).await.unwrap();
    let report = verify_chain(e.other.tenant_id, &rows, &[], &key().1);
    assert!(
        !report.is_ok(),
        "the tenant is part of the genesis hash and of every row"
    );
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn tenants_have_separate_chains_and_cannot_read_each_other() {
    let e = env().await;
    fill(&e, 3).await;
    e.log.append(&e.other, &entry(9)).await.unwrap();
    assert_eq!(e.log.read_all(&e.ctx).await.unwrap().len(), 3);
    assert_eq!(e.log.read_all(&e.other).await.unwrap().len(), 1);
    assert!(verify(&e, &[]).await.is_ok());
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn the_runtime_role_cannot_rewrite_history_even_by_mistake() {
    let e = env().await;
    fill(&e, 2).await;
    assert!(
        sqlx::query("UPDATE audit_log SET actor = 'x'")
            .execute(&e.db.app)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM audit_log")
            .execute(&e.db.app)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("TRUNCATE audit_log")
            .execute(&e.db.app)
            .await
            .is_err()
    );
    e.db.drop_db().await.unwrap();
}

#[tokio::test]
async fn the_row_hash_format_matches_an_independent_implementation() {
    // Expected values come from a separate Python implementation of the format in canonical.rs.
    let tenant = TenantId(uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap());
    assert_eq!(
        hex::encode(genesis_hash(tenant)),
        "ae75d61598970cbf61caed829a77a9db2bd4a2aadc2f4318cd17812ea6b424f1"
    );
    let r1 = Row {
        seq: 1,
        created_us: 1_764_000_000_000_000,
        prev_hash: genesis_hash(tenant),
        hash: [0; 32],
        entry: AuditEntry {
            actor: "user:u1".into(),
            action: "tool.call".into(),
            target: Some("github.pr.merge".into()),
            tool_hash: Some([7; 32]),
            payload_hash: [9; 32],
        },
    };
    let h1 = library_audit_row_hash(tenant, &r1);
    assert_eq!(
        hex::encode(h1),
        "5b97bbcbaad1709e30db7a08eec8ab98a0961c65f82eb58794e1a50016019aa4"
    );
    // The crate's own implementation agrees: a stored row with that hash verifies.
    let r1 = Row { hash: h1, ..r1 };
    let r2 = Row {
        seq: 2,
        created_us: 1_764_000_001_000_000,
        prev_hash: h1,
        hash: [0; 32],
        entry: AuditEntry {
            actor: "system".into(),
            action: "approval.decide".into(),
            target: None,
            tool_hash: None,
            payload_hash: [1; 32],
        },
    };
    let h2 = library_audit_row_hash(tenant, &r2);
    assert_eq!(
        hex::encode(h2),
        "5cd0c3d73ea584aa25b0a1109a6c18059fbd0df782b2b5b562f467f9e6c6c2e4"
    );
    let report = verify_chain(tenant, &[r1, Row { hash: h2, ..r2 }], &[], &key().1);
    assert!(report.is_ok(), "{report:?}");
}

#[tokio::test]
async fn heads_survive_in_a_file_outside_the_database() {
    let e = env().await;
    fill(&e, 3).await;
    let rows = e.log.read_all(&e.ctx).await.unwrap();
    let dir = std::env::temp_dir().join(format!("library-heads-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let store = FileHeadStore::new(dir.join("heads.jsonl"));
    assert!(
        store.heads(e.ctx.tenant_id).unwrap().is_empty(),
        "no file yet means no heads"
    );
    let (signer, vk) = key();
    store
        .append(&signer.sign(e.ctx.tenant_id, "2026-11-26", 3, rows[2].hash))
        .unwrap();
    store
        .append(&signer.sign(e.other.tenant_id, "2026-11-26", 9, [4; 32]))
        .unwrap();
    std::fs::OpenOptions::new()
        .append(true)
        .open(dir.join("heads.jsonl"))
        .and_then(|mut f| std::io::Write::write_all(&mut f, b"this is not json\n"))
        .unwrap();
    let mine = store.heads(e.ctx.tenant_id).unwrap();
    assert_eq!(
        mine.len(),
        1,
        "other tenants' heads and garbage lines are ignored"
    );
    assert!(mine[0].is_valid(&vk));
    assert!(verify_chain(e.ctx.tenant_id, &rows, &mine, &vk).is_ok());
    let _ = std::fs::remove_dir_all(dir);
    let _ = SigningKey::from_bytes(&[0; 32]);
    let _ = OffsetDateTime::now_utc();
    e.db.drop_db().await.unwrap();
}
