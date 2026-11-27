set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]
set dotenv-load := true

default:
    @just --list

# Start development containers (Postgres)
dev-up:
    docker compose -f deploy/dev/compose.yml up -d
    bash scripts/wait-pg.sh

dev-down:
    docker compose -f deploy/dev/compose.yml down

# Drop and recreate the dev database, then migrate
db-reset: dev-up
    bash scripts/db-reset.sh

migrate:
    sqlx migrate run --source migrations --database-url "$DATABASE_OWNER_URL"

fmt:
    cargo fmt --all

check-fast:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo nextest run --workspace --no-tests=pass

# Everything that must be green before work is done
check: check-fast contracts-check spec-check

# Regenerate contracts and the TypeScript types for the app
types:
    cargo nextest run -p library-core -p library-protocol export_bindings --no-tests=pass
    cargo xtask contracts

contracts-check:
    cargo xtask contracts --check

spec-split:
    cargo xtask spec-split

spec-check:
    cargo xtask spec-check

schema-dump:
    cargo xtask schema-dump

# Row-level security isolation suite only
rls:
    cargo nextest run -p library-authz --test rls_isolation
