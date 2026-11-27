use anyhow::{Context, Result};
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

const ROLES_SQL: &str = include_str!("../../../deploy/roles.sql");

/// A throwaway database with every migration applied, plus one pool per role.
/// Pools connect on first use and hold at most two connections each, so many
/// tests can run in parallel inside Postgres's connection limit.
pub struct TestDb {
    pub name: String,
    /// Superuser on the test database. Used only to seed and to inspect.
    pub admin: PgPool,
    /// Table owner; runs migrations.
    pub owner: PgPool,
    /// Runtime role: not owner, no BYPASSRLS.
    pub app: PgPool,
    pub sequencer: PgPool,
    pub vault: PgPool,
    /// Sign-in and session code only.
    pub auth: PgPool,
    server: PgPool,
}

fn admin_url() -> String {
    std::env::var("DATABASE_ADMIN_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5433/postgres".into())
}

/// `user:password@host:port` taken from the admin URL, so tests follow whatever server is configured.
fn host_part() -> String {
    let admin = admin_url();
    let rest = admin.split("://").nth(1).unwrap_or_default();
    let host = rest.rsplit('@').next().unwrap_or_default();
    host.split('/').next().unwrap_or_default().to_string()
}

fn url_for(user: &str, password: &str, db: &str) -> String {
    format!("postgres://{user}:{password}@{}/{db}", host_part())
}

fn admin_url_for(db: &str) -> String {
    let admin = admin_url();
    let base = admin.rsplit_once('/').map_or(admin.as_str(), |(b, _)| b);
    format!("{base}/{db}")
}

fn pool(url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(2)
        .connect_lazy(url)
        .context("pool url")
}

impl TestDb {
    pub async fn new() -> Result<Self> {
        let server = pool(&admin_url())?;
        let mut conn = server
            .acquire()
            .await
            .context("Postgres is not reachable; run `just dev-up`")?;

        // Roles are cluster-wide and tests run in parallel, so serialise their creation.
        sqlx::query("SELECT pg_advisory_lock(7001)")
            .execute(&mut *conn)
            .await?;
        let created = sqlx::raw_sql(ROLES_SQL).execute(&mut *conn).await;
        sqlx::query("SELECT pg_advisory_unlock(7001)")
            .execute(&mut *conn)
            .await?;
        created.context("create roles")?;
        drop(conn);

        let name = format!("test_{}", Uuid::new_v4().simple());
        sqlx::query(sql!("CREATE DATABASE \"{name}\" OWNER library_owner"))
            .execute(&server)
            .await?;

        let owner = pool(&url_for("library_owner", "owner_dev_pw", &name))?;
        sqlx::migrate!("../../migrations")
            .run(&owner)
            .await
            .context("migrate")?;

        Ok(Self {
            admin: pool(&admin_url_for(&name))?,
            app: pool(&url_for("app_rw", "app_dev_pw", &name))?,
            sequencer: pool(&url_for("app_sequencer", "sequencer_dev_pw", &name))?,
            vault: pool(&url_for("app_vault", "vault_dev_pw", &name))?,
            auth: pool(&url_for("app_auth", "auth_dev_pw", &name))?,
            owner,
            name,
            server,
        })
    }

    /// Closes every pool and drops the database.
    pub async fn drop_db(self) -> Result<()> {
        self.admin.close().await;
        self.owner.close().await;
        self.app.close().await;
        self.sequencer.close().await;
        self.vault.close().await;
        self.auth.close().await;
        sqlx::query(sql!(
            "DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)",
            self.name
        ))
        .execute(&self.server)
        .await?;
        Ok(())
    }
}
