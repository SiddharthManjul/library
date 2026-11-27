use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::TestDb;

#[derive(Debug, Clone)]
pub struct TenantFx {
    pub id: Uuid,
    pub user: Uuid,
    pub scopes: [Uuid; 2],
}

#[derive(Debug, Clone)]
pub struct Fixture {
    pub a: TenantFx,
    pub b: TenantFx,
}

struct Col {
    name: String,
    nullable: bool,
    udt: String,
    generated: bool,
    has_default: bool,
}

/// Tables seeded by hand because their ids matter to the fixture.
const MANUAL: [&str; 3] = ["tenants", "users", "scopes"];

/// Seeds two tenants, each with one user and two scopes, and one row per tenant
/// (or per scope) in every other table, so isolation can be checked everywhere.
/// Foreign keys are skipped while seeding; row-level security is what is under test.
pub async fn seed_two_tenants(db: &TestDb) -> Result<Fixture> {
    let fx = Fixture {
        a: tenant(),
        b: tenant(),
    };
    let mut conn = db.admin.acquire().await?;
    sqlx::query("SET session_replication_role = replica")
        .execute(&mut *conn)
        .await?;

    for (n, t) in [("a", &fx.a), ("b", &fx.b)] {
        sqlx::query(sql!(
            "INSERT INTO tenants (id, name) VALUES ('{}', 'tenant {n}')",
            t.id
        ))
        .execute(&mut *conn)
        .await?;
        sqlx::query(sql!("INSERT INTO users (id, email, display_name) VALUES ('{}', 'user-{n}@example.test', 'user {n}')", t.user)).execute(&mut *conn).await?;
        for (i, s) in t.scopes.iter().enumerate() {
            sqlx::query(sql!("INSERT INTO scopes (id, tenant_id, kind, name) VALUES ('{s}', '{}', 'department', 'scope {n}{i}')", t.id)).execute(&mut *conn).await?;
        }
    }

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE' AND table_name <> '_sqlx_migrations' ORDER BY 1",
    )
    .fetch_all(&db.admin)
    .await?;

    for table in tables.iter().filter(|t| !MANUAL.contains(&t.as_str())) {
        let cols = columns(&db.admin, table).await?;
        let scoped = cols.iter().any(|c| c.name == "scope_id");
        for t in [&fx.a, &fx.b] {
            if scoped {
                for s in t.scopes {
                    insert_row(&mut conn, &db.admin, table, &cols, t, Some(s)).await?;
                }
            } else {
                insert_row(&mut conn, &db.admin, table, &cols, t, None).await?;
            }
        }
    }

    // Shared rows with no tenant: built-in skill packs and release eval results.
    sqlx::query("INSERT INTO skill_packs (department, version, hash, manifest) VALUES ('shared', 1, '\\x00', '{}')").execute(&mut *conn).await?;
    sqlx::query("INSERT INTO eval_runs (suite, suite_version, build, k, pass_k, score) VALUES ('security', '1', 'dev', 5, 1, 1)").execute(&mut *conn).await?;
    sqlx::query("SET session_replication_role = DEFAULT")
        .execute(&mut *conn)
        .await?;
    Ok(fx)
}

fn tenant() -> TenantFx {
    TenantFx {
        id: Uuid::new_v4(),
        user: Uuid::new_v4(),
        scopes: [Uuid::new_v4(), Uuid::new_v4()],
    }
}

async fn columns(pool: &PgPool, table: &str) -> Result<Vec<Col>> {
    let rows = sqlx::query(
        "SELECT column_name, is_nullable, udt_name, is_generated, column_default FROM information_schema.columns WHERE table_schema='public' AND table_name=$1 ORDER BY ordinal_position",
    )
    .bind(table)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|r| Col {
            name: r.get("column_name"),
            nullable: r.get::<String, _>("is_nullable") == "YES",
            udt: r.get("udt_name"),
            generated: r.get::<String, _>("is_generated") == "ALWAYS",
            has_default: r.get::<Option<String>, _>("column_default").is_some(),
        })
        .collect())
}

/// First quoted value of a `CHECK (col = ANY (ARRAY['x'::text, ...]))` constraint.
async fn enum_value(pool: &PgPool, table: &str, col: &str) -> Result<Option<String>> {
    let defs: Vec<String> = sqlx::query_scalar(sql!(
        "SELECT pg_get_constraintdef(oid) FROM pg_constraint WHERE conrelid = 'public.\"{table}\"'::regclass AND contype = 'c'"
    ))
    .fetch_all(pool)
    .await?;
    for d in defs {
        if d.contains(&format!("({col} = ANY"))
            && let Some(start) = d.find('\'')
            && let Some(len) = d[start + 1..].find('\'')
        {
            return Ok(Some(d[start + 1..start + 1 + len].to_string()));
        }
    }
    Ok(None)
}

async fn insert_row(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    pool: &PgPool,
    table: &str,
    cols: &[Col],
    t: &TenantFx,
    scope: Option<Uuid>,
) -> Result<()> {
    let (mut names, mut vals) = (vec![], vec![]);
    for c in cols.iter().filter(|c| !c.generated) {
        let forced = matches!(c.name.as_str(), "tenant_id" | "scope_id" | "scope_ids")
            || (table == "approvals" && c.name == "task_id");
        if !forced && (c.nullable || c.has_default) {
            continue;
        }
        let v = match (c.name.as_str(), c.udt.as_str()) {
            ("tenant_id", _) => format!("'{}'", t.id),
            ("scope_id", _) => format!("'{}'", scope.unwrap_or(t.scopes[0])),
            ("scope_ids", _) => format!("ARRAY['{}'::uuid]", scope.unwrap_or(t.scopes[0])),
            ("user_id" | "owner_id" | "audience_id", _) => format!("'{}'", t.user),
            (_, "uuid") => "gen_random_uuid()".into(),
            (_, "_uuid" | "_text") => "'{}'".into(),
            (_, "text" | "varchar") => match enum_value(pool, table, &c.name).await? {
                Some(v) => format!("'{v}'"),
                None => "md5(random()::text)".into(),
            },
            (_, "int2" | "int4" | "int8") => "1".into(),
            (_, "float4" | "float8" | "numeric") => "0".into(),
            (_, "bool") => "false".into(),
            (_, "timestamptz") => "now()".into(),
            (_, "date") => "current_date".into(),
            (_, "jsonb" | "json") => "'{}'".into(),
            (_, "bytea") => "decode(md5(random()::text), 'hex')".into(),
            (n, u) => anyhow::bail!("seed: no value for {table}.{n} of type {u}"),
        };
        names.push(format!("\"{}\"", c.name));
        vals.push(v);
    }
    sqlx::query(sql!(
        "INSERT INTO \"{table}\" ({}) VALUES ({})",
        names.join(", "),
        vals.join(", ")
    ))
    .execute(&mut **conn)
    .await
    .map_err(|e| anyhow::anyhow!("seed {table}: {e}"))?;
    Ok(())
}
