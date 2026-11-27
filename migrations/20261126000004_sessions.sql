-- Sign-in and sessions (P0-10, P0-11).
-- identities: which external account (Google, Microsoft, GitHub) is which user. Global, like users.
-- refresh_tokens: rotating refresh tokens grouped in families; only a SHA-256 hash is stored.
-- app_auth is the one role that sees across tenants, and only for these sign-in tables.

CREATE TABLE identities (
  provider text NOT NULL CHECK (provider IN ('google','microsoft','github')),
  subject text NOT NULL,
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (provider, subject)
);
CREATE INDEX identities_user_idx ON identities (user_id);

CREATE TABLE refresh_tokens (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  device_id uuid NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
  family_id uuid NOT NULL,
  token_hash bytea NOT NULL UNIQUE,
  issued_at timestamptz NOT NULL,
  expires_at timestamptz NOT NULL,
  used_at timestamptz,
  revoked_at timestamptz
);
CREATE INDEX refresh_tokens_tenant_idx ON refresh_tokens (tenant_id);
CREATE INDEX refresh_tokens_family_idx ON refresh_tokens (family_id);

ALTER TABLE identities ENABLE ROW LEVEL SECURITY;
ALTER TABLE identities FORCE ROW LEVEL SECURITY;
ALTER TABLE refresh_tokens ENABLE ROW LEVEL SECURITY;
ALTER TABLE refresh_tokens FORCE ROW LEVEL SECURITY;

-- Members: see their own identities and sessions, nothing else.
CREATE POLICY own_identities ON identities FOR SELECT USING (user_id = app_user());
CREATE POLICY isolation ON refresh_tokens FOR SELECT
  USING (tenant_id = app_tenant() AND user_id = app_user());
GRANT SELECT ON identities, refresh_tokens TO app_rw;
GRANT SELECT ON identities, refresh_tokens TO app_sequencer;

-- Sign-in code: full access to its own tables, narrow access to the rest.
CREATE POLICY auth_all ON identities TO app_auth USING (true) WITH CHECK (true);
CREATE POLICY auth_all ON refresh_tokens TO app_auth USING (true) WITH CHECK (true);
CREATE POLICY auth_select ON users FOR SELECT TO app_auth USING (true);
CREATE POLICY auth_insert ON users FOR INSERT TO app_auth WITH CHECK (true);
CREATE POLICY auth_select ON memberships FOR SELECT TO app_auth USING (true);
CREATE POLICY auth_select ON tenants FOR SELECT TO app_auth USING (true);
CREATE POLICY auth_select ON devices FOR SELECT TO app_auth USING (true);
CREATE POLICY auth_insert ON devices FOR INSERT TO app_auth WITH CHECK (true);
CREATE POLICY auth_update ON devices FOR UPDATE TO app_auth USING (true) WITH CHECK (true);

GRANT SELECT, INSERT, UPDATE ON identities, refresh_tokens TO app_auth;
GRANT SELECT, INSERT ON users TO app_auth;
GRANT SELECT ON memberships, tenants TO app_auth;
GRANT SELECT, INSERT, UPDATE ON devices TO app_auth;
