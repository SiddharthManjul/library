-- P1-01: idempotency for writes. A write carries a client-chosen key; replaying it returns the first result.
-- A key is scoped to one tenant and user, and remembers the method and a hash of the params, so reusing
-- a key for a different request is refused instead of returning someone else's answer.
CREATE TABLE idempotency_keys (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  key text NOT NULL CHECK (length(key) BETWEEN 8 AND 128),
  method text NOT NULL,
  params_hash bytea NOT NULL,
  state text NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','done')),
  response jsonb,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (tenant_id, user_id, key)
);
CREATE INDEX idempotency_keys_created_idx ON idempotency_keys (created_at);
ALTER TABLE idempotency_keys ENABLE ROW LEVEL SECURITY;
ALTER TABLE idempotency_keys FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON idempotency_keys
  USING (tenant_id = app_tenant() AND user_id = app_user())
  WITH CHECK (tenant_id = app_tenant() AND user_id = app_user());
GRANT SELECT, INSERT, UPDATE, DELETE ON idempotency_keys TO app_rw;
GRANT SELECT ON idempotency_keys TO app_sequencer;
