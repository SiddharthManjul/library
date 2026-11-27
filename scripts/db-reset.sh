#!/usr/bin/env bash
# Recreate the dev database, then run migrations. Dev only.
set -euo pipefail
cd "$(dirname "$0")/.."
ADMIN="${DATABASE_ADMIN_URL:-postgres://postgres:postgres@127.0.0.1:5433/postgres}"
psql "$ADMIN" -v ON_ERROR_STOP=1 -q -f deploy/roles.sql
psql "$ADMIN" -v ON_ERROR_STOP=1 -q -c 'DROP DATABASE IF EXISTS library WITH (FORCE)' -c 'CREATE DATABASE library OWNER library_owner'
sqlx migrate run --source migrations --database-url "${DATABASE_OWNER_URL:-postgres://library_owner:owner_dev_pw@127.0.0.1:5433/library}"
echo "database reset and migrated"
