//! Shared test helpers: a fresh migrated Postgres database per test, role pools,
//! request-context helpers and a two-tenant fixture seeded into every table.

/// Dynamic SQL built from test-controlled values only (table names, fixture ids).
macro_rules! sql {
    ($($t:tt)*) => { sqlx::AssertSqlSafe(format!($($t)*)) };
}

mod ctx;
mod db;
mod seed;

pub use ctx::{Ctx, count_as, exec_as};
pub use db::TestDb;
pub use seed::{Fixture, TenantFx, seed_two_tenants};
