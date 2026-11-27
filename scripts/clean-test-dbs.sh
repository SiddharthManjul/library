#!/usr/bin/env bash
# Drop the throwaway `test_*` databases that interrupted test runs leave behind. Dev only.
set -euo pipefail
ADMIN="${DATABASE_ADMIN_URL:-postgres://postgres:postgres@127.0.0.1:5433/postgres}"
n=0
for d in $(psql "$ADMIN" -tAc "select datname from pg_database where datname like 'test\_%'"); do
  psql "$ADMIN" -q -c "DROP DATABASE IF EXISTS \"$d\" WITH (FORCE)"; n=$((n+1))
done
echo "dropped $n test databases"
