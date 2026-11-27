#!/usr/bin/env bash
# Wait until the dev Postgres accepts connections.
ADMIN="${DATABASE_ADMIN_URL:-postgres://postgres:postgres@127.0.0.1:5433/postgres}"
for _ in $(seq 1 60); do
  psql "$ADMIN" -c 'select 1' >/dev/null 2>&1 && exit 0
  sleep 1
done
echo "postgres did not become ready" >&2; exit 1
