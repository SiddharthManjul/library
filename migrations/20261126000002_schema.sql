-- Library IDE schema, part 2 of 2: tables, forced row-level security, policies and grants.
-- Roles app_rw, app_sequencer and app_vault exist before this runs (deploy/roles.sql).

CREATE TABLE tenants (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  name text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
ALTER TABLE tenants ENABLE ROW LEVEL SECURITY;
ALTER TABLE tenants FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON tenants USING (id = app_tenant()) WITH CHECK (id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON tenants TO app_rw;
GRANT SELECT ON tenants TO app_sequencer;

CREATE TABLE users (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  email text NOT NULL UNIQUE,
  display_name text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE users FORCE ROW LEVEL SECURITY;
GRANT SELECT, INSERT, UPDATE, DELETE ON users TO app_rw;
GRANT SELECT ON users TO app_sequencer;

CREATE TABLE plans (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  plan text NOT NULL CHECK (plan IN ('starter','team','business','enterprise')),
  features jsonb NOT NULL DEFAULT '{}',
  limits jsonb NOT NULL DEFAULT '{}',
  updated_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (tenant_id)
);
ALTER TABLE plans ENABLE ROW LEVEL SECURITY;
ALTER TABLE plans FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON plans USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON plans TO app_rw;
GRANT SELECT ON plans TO app_sequencer;

CREATE TABLE memberships (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role text NOT NULL CHECK (role IN ('member','department_lead','product_leader','c_suite','admin')),
  PRIMARY KEY (tenant_id, user_id)
);
ALTER TABLE memberships ENABLE ROW LEVEL SECURITY;
ALTER TABLE memberships FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON memberships USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON memberships TO app_rw;
GRANT SELECT ON memberships TO app_sequencer;

CREATE TABLE scopes (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  kind text NOT NULL CHECK (kind IN ('department','fold')),
  name text NOT NULL,
  fold_id uuid REFERENCES scopes(id),
  sealed boolean NOT NULL DEFAULT false,
  UNIQUE (tenant_id, name)
);
CREATE INDEX scopes_tenant_idx ON scopes (tenant_id);
ALTER TABLE scopes ENABLE ROW LEVEL SECURITY;
ALTER TABLE scopes FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON scopes USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON scopes TO app_rw;
GRANT SELECT ON scopes TO app_sequencer;

CREATE TABLE scope_members (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  scope_role text NOT NULL CHECK (scope_role IN ('member','lead')),
  PRIMARY KEY (scope_id, user_id)
);
CREATE INDEX scope_members_tenant_idx ON scope_members (tenant_id);
ALTER TABLE scope_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE scope_members FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON scope_members USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON scope_members TO app_rw;
GRANT SELECT ON scope_members TO app_sequencer;

CREATE TABLE devices (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  public_key bytea NOT NULL,
  signing_public_key bytea NOT NULL,
  last_seen timestamptz,
  revoked_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now()
);
ALTER TABLE devices ENABLE ROW LEVEL SECURITY;
ALTER TABLE devices FORCE ROW LEVEL SECURITY;
CREATE POLICY user_isolation ON devices USING (user_id = app_user()) WITH CHECK (user_id = app_user());
GRANT SELECT, INSERT, UPDATE, DELETE ON devices TO app_rw;
GRANT SELECT ON devices TO app_sequencer;

CREATE TABLE connections (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  provider text NOT NULL,
  server_url text NOT NULL,
  auth_mode text NOT NULL CHECK (auth_mode IN ('automatic','our_app','customer_app','api_key','open')),
  oauth_client_ref uuid,
  status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','connected','expired','revoked')),
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX connections_tenant_idx ON connections (tenant_id);
CREATE INDEX connections_scope_idx ON connections (tenant_id, scope_id);
ALTER TABLE connections ENABLE ROW LEVEL SECURITY;
ALTER TABLE connections FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON connections USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT, INSERT, UPDATE, DELETE ON connections TO app_rw;
GRANT SELECT ON connections TO app_sequencer;

CREATE TABLE tenant_oauth_clients (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  provider text NOT NULL,
  client_id text NOT NULL,
  secret_ref uuid NOT NULL,
  signing_secret_ref uuid,
  redirect_uri text NOT NULL,
  UNIQUE (tenant_id, provider)
);
CREATE INDEX tenant_oauth_clients_tenant_idx ON tenant_oauth_clients (tenant_id);
ALTER TABLE tenant_oauth_clients ENABLE ROW LEVEL SECURITY;
ALTER TABLE tenant_oauth_clients FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON tenant_oauth_clients USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON tenant_oauth_clients TO app_rw;
GRANT SELECT ON tenant_oauth_clients TO app_sequencer;

CREATE TABLE oauth_tokens (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  connection_id uuid NOT NULL REFERENCES connections(id) ON DELETE CASCADE,
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  secret_ref uuid NOT NULL,
  granted_scopes text[] NOT NULL DEFAULT '{}',
  expires_at timestamptz,
  PRIMARY KEY (connection_id, user_id)
);
CREATE INDEX oauth_tokens_tenant_idx ON oauth_tokens (tenant_id);
ALTER TABLE oauth_tokens ENABLE ROW LEVEL SECURITY;
ALTER TABLE oauth_tokens FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON oauth_tokens USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON oauth_tokens TO app_rw;
GRANT SELECT ON oauth_tokens TO app_sequencer;

CREATE TABLE events (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  connection_id uuid REFERENCES connections(id),
  source text NOT NULL,
  external_id text NOT NULL,
  kind text NOT NULL,
  occurred_at timestamptz NOT NULL,
  received_at timestamptz NOT NULL DEFAULT now(),
  payload jsonb NOT NULL,
  taint text NOT NULL DEFAULT 'external' CHECK (taint IN ('external','clean')),
  UNIQUE (tenant_id, source, external_id)
);
CREATE INDEX events_tenant_idx ON events (tenant_id);
CREATE INDEX events_scope_idx ON events (tenant_id, scope_id);
ALTER TABLE events ENABLE ROW LEVEL SECURITY;
ALTER TABLE events FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON events USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT, INSERT ON events TO app_rw;
GRANT SELECT ON events TO app_sequencer;

CREATE TABLE tool_registry (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  connection_id uuid NOT NULL REFERENCES connections(id) ON DELETE CASCADE,
  tool_name text NOT NULL,
  def_hash bytea NOT NULL,
  definition jsonb NOT NULL,
  tier text NOT NULL DEFAULT 'external_write' CHECK (tier IN ('read','sandbox_write','external_write')),
  status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','approved','blocked')),
  approved_by uuid REFERENCES users(id),
  approved_at timestamptz,
  UNIQUE (connection_id, tool_name)
);
CREATE INDEX tool_registry_tenant_idx ON tool_registry (tenant_id);
ALTER TABLE tool_registry ENABLE ROW LEVEL SECURITY;
ALTER TABLE tool_registry FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON tool_registry USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON tool_registry TO app_rw;
GRANT SELECT ON tool_registry TO app_sequencer;

CREATE TABLE skill_packs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid REFERENCES tenants(id) ON DELETE CASCADE,
  department text NOT NULL,
  version integer NOT NULL,
  hash bytea NOT NULL,
  manifest jsonb NOT NULL,
  UNIQUE NULLS NOT DISTINCT (tenant_id, department, version)
);
CREATE INDEX skill_packs_tenant_idx ON skill_packs (tenant_id);
ALTER TABLE skill_packs ENABLE ROW LEVEL SECURITY;
ALTER TABLE skill_packs FORCE ROW LEVEL SECURITY;
CREATE POLICY global_read ON skill_packs FOR SELECT USING (tenant_id IS NULL OR tenant_id = app_tenant());
CREATE POLICY tenant_write ON skill_packs FOR ALL USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON skill_packs TO app_rw;
GRANT SELECT ON skill_packs TO app_sequencer;

CREATE TABLE entities (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  kind text NOT NULL,
  name text NOT NULL,
  summary text,
  external_ids jsonb NOT NULL DEFAULT '{}',
  tsv tsvector GENERATED ALWAYS AS (to_tsvector('simple', coalesce(name,'') || ' ' || coalesce(summary,''))) STORED,
  seq bigint NOT NULL DEFAULT 0
);
CREATE INDEX entities_tenant_idx ON entities (tenant_id);
CREATE INDEX entities_scope_idx ON entities (tenant_id, scope_id);
CREATE INDEX entities_x0 ON entities USING gin (tsv);
ALTER TABLE entities ENABLE ROW LEVEL SECURITY;
ALTER TABLE entities FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON entities USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT ON entities TO app_rw;
GRANT SELECT, INSERT, UPDATE ON entities TO app_sequencer;
GRANT SELECT ON entities TO app_sequencer;

CREATE TABLE entity_aliases (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  entity_id uuid NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
  alias text NOT NULL,
  source text NOT NULL,
  PRIMARY KEY (entity_id, alias)
);
CREATE INDEX entity_aliases_tenant_idx ON entity_aliases (tenant_id);
CREATE INDEX entity_aliases_scope_idx ON entity_aliases (tenant_id, scope_id);
ALTER TABLE entity_aliases ENABLE ROW LEVEL SECURITY;
ALTER TABLE entity_aliases FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON entity_aliases USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT ON entity_aliases TO app_rw;
GRANT SELECT, INSERT, UPDATE ON entity_aliases TO app_sequencer;
GRANT SELECT ON entity_aliases TO app_sequencer;

CREATE TABLE merge_proposals (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  entity_a uuid NOT NULL REFERENCES entities(id),
  entity_b uuid NOT NULL REFERENCES entities(id),
  score real NOT NULL,
  state text NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','accepted','rejected')),
  decided_by uuid REFERENCES users(id)
);
CREATE INDEX merge_proposals_tenant_idx ON merge_proposals (tenant_id);
CREATE INDEX merge_proposals_scope_idx ON merge_proposals (tenant_id, scope_id);
ALTER TABLE merge_proposals ENABLE ROW LEVEL SECURITY;
ALTER TABLE merge_proposals FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON merge_proposals USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT, INSERT, UPDATE, DELETE ON merge_proposals TO app_rw;
GRANT SELECT ON merge_proposals TO app_sequencer;

CREATE TABLE facts (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  subject_id uuid NOT NULL REFERENCES entities(id),
  relation text NOT NULL,
  object_id uuid REFERENCES entities(id),
  statement text NOT NULL,
  valid_from timestamptz NOT NULL,
  valid_to timestamptz,
  recorded_at timestamptz NOT NULL DEFAULT now(),
  retracted_at timestamptz,
  source_event_id uuid NOT NULL REFERENCES events(id),
  source_ref jsonb NOT NULL DEFAULT '{}',
  extractor_version text NOT NULL,
  confidence real NOT NULL,
  taint text NOT NULL DEFAULT 'external' CHECK (taint IN ('external','clean')),
  tsv tsvector GENERATED ALWAYS AS (to_tsvector('simple', statement)) STORED,
  seq bigint NOT NULL DEFAULT 0
);
CREATE INDEX facts_tenant_idx ON facts (tenant_id);
CREATE INDEX facts_scope_idx ON facts (tenant_id, scope_id);
CREATE INDEX facts_x0 ON facts USING gin (tsv);
CREATE INDEX facts_x1 ON facts (subject_id);
ALTER TABLE facts ENABLE ROW LEVEL SECURITY;
ALTER TABLE facts FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON facts USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT ON facts TO app_rw;
GRANT SELECT, INSERT, UPDATE ON facts TO app_sequencer;
GRANT SELECT ON facts TO app_sequencer;

CREATE TABLE community_summaries (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  community_key text NOT NULL,
  member_ids uuid[] NOT NULL DEFAULT '{}',
  summary text NOT NULL,
  citations jsonb NOT NULL DEFAULT '[]',
  computed_seq bigint NOT NULL,
  UNIQUE (scope_id, community_key)
);
CREATE INDEX community_summaries_tenant_idx ON community_summaries (tenant_id);
CREATE INDEX community_summaries_scope_idx ON community_summaries (tenant_id, scope_id);
ALTER TABLE community_summaries ENABLE ROW LEVEL SECURITY;
ALTER TABLE community_summaries FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON community_summaries USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT ON community_summaries TO app_rw;
GRANT SELECT, INSERT, UPDATE ON community_summaries TO app_sequencer;
GRANT SELECT ON community_summaries TO app_sequencer;

CREATE TABLE briefs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  audience_kind text NOT NULL CHECK (audience_kind IN ('leader','department')),
  audience_id uuid NOT NULL,
  scope_ids uuid[] NOT NULL,
  content text NOT NULL,
  citations jsonb NOT NULL DEFAULT '[]',
  stale boolean NOT NULL DEFAULT false,
  computed_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (tenant_id, audience_kind, audience_id)
);
CREATE INDEX briefs_tenant_idx ON briefs (tenant_id);
ALTER TABLE briefs ENABLE ROW LEVEL SECURITY;
ALTER TABLE briefs FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON briefs USING (tenant_id = app_tenant() AND scope_ids <@ app_scope_ids()) WITH CHECK (tenant_id = app_tenant() AND scope_ids <@ app_scope_ids());
GRANT SELECT ON briefs TO app_rw;
GRANT SELECT, INSERT, UPDATE ON briefs TO app_sequencer;
GRANT SELECT ON briefs TO app_sequencer;

CREATE TABLE playbook_items (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  department text NOT NULL,
  content text NOT NULL,
  helpful integer NOT NULL DEFAULT 0,
  harmful integer NOT NULL DEFAULT 0,
  status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','active','deprecated')),
  source_run_id uuid,
  approved_by uuid REFERENCES users(id)
);
CREATE INDEX playbook_items_tenant_idx ON playbook_items (tenant_id);
CREATE INDEX playbook_items_scope_idx ON playbook_items (tenant_id, scope_id);
ALTER TABLE playbook_items ENABLE ROW LEVEL SECURITY;
ALTER TABLE playbook_items FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON playbook_items USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT ON playbook_items TO app_rw;
GRANT SELECT, INSERT, UPDATE ON playbook_items TO app_sequencer;
GRANT SELECT ON playbook_items TO app_sequencer;

CREATE TABLE deltas (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  seq bigint NOT NULL,
  ops jsonb NOT NULL,
  state_root bytea NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (scope_id, seq)
);
CREATE INDEX deltas_tenant_idx ON deltas (tenant_id);
ALTER TABLE deltas ENABLE ROW LEVEL SECURITY;
ALTER TABLE deltas FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON deltas USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT ON deltas TO app_rw;
GRANT SELECT, INSERT, UPDATE ON deltas TO app_sequencer;
GRANT SELECT ON deltas TO app_sequencer;

CREATE TABLE snapshots (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  seq bigint NOT NULL,
  r2_key text NOT NULL,
  state_root bytea NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (scope_id, seq)
);
CREATE INDEX snapshots_tenant_idx ON snapshots (tenant_id);
ALTER TABLE snapshots ENABLE ROW LEVEL SECURITY;
ALTER TABLE snapshots FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON snapshots USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT ON snapshots TO app_rw;
GRANT SELECT, INSERT, UPDATE ON snapshots TO app_sequencer;
GRANT SELECT ON snapshots TO app_sequencer;

CREATE TABLE replica_cursors (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  device_id uuid NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
  applied_seq bigint NOT NULL,
  state_root bytea NOT NULL,
  updated_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (device_id, scope_id)
);
CREATE INDEX replica_cursors_tenant_idx ON replica_cursors (tenant_id);
CREATE INDEX replica_cursors_scope_idx ON replica_cursors (tenant_id, scope_id);
ALTER TABLE replica_cursors ENABLE ROW LEVEL SECURITY;
ALTER TABLE replica_cursors FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON replica_cursors USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT, INSERT, UPDATE, DELETE ON replica_cursors TO app_rw;
GRANT SELECT ON replica_cursors TO app_sequencer;

CREATE TABLE scope_keys (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  key_version integer NOT NULL,
  wrapped_key bytea NOT NULL,
  valid_from timestamptz NOT NULL DEFAULT now(),
  valid_to timestamptz,
  PRIMARY KEY (scope_id, key_version)
);
CREATE INDEX scope_keys_tenant_idx ON scope_keys (tenant_id);
ALTER TABLE scope_keys ENABLE ROW LEVEL SECURITY;
ALTER TABLE scope_keys FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON scope_keys USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT, INSERT, UPDATE, DELETE ON scope_keys TO app_rw;
GRANT SELECT ON scope_keys TO app_sequencer;

CREATE TABLE tasks (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  title text NOT NULL,
  owner_id uuid NOT NULL REFERENCES users(id),
  state text NOT NULL DEFAULT 'draft' CHECK (state IN ('draft','awaiting_signoff','rejected','approved','running','awaiting_action_approval','evaluating','completed','failed')),
  version integer NOT NULL DEFAULT 0,
  budget jsonb NOT NULL DEFAULT '{}',
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX tasks_tenant_idx ON tasks (tenant_id);
ALTER TABLE tasks ENABLE ROW LEVEL SECURITY;
ALTER TABLE tasks FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON tasks USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON tasks TO app_rw;
GRANT SELECT ON tasks TO app_sequencer;

CREATE TABLE task_scopes (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  task_id uuid NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  decision text NOT NULL DEFAULT 'pending' CHECK (decision IN ('pending','approved','rejected')),
  decided_by uuid REFERENCES users(id),
  decided_at timestamptz,
  PRIMARY KEY (task_id, scope_id)
);
CREATE INDEX task_scopes_tenant_idx ON task_scopes (tenant_id);
CREATE INDEX task_scopes_scope_idx ON task_scopes (tenant_id, scope_id);
ALTER TABLE task_scopes ENABLE ROW LEVEL SECURITY;
ALTER TABLE task_scopes FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON task_scopes USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT, INSERT, UPDATE, DELETE ON task_scopes TO app_rw;
GRANT SELECT ON task_scopes TO app_sequencer;

CREATE TABLE task_items (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  task_id uuid NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  item_key text NOT NULL,
  description text NOT NULL,
  passes boolean NOT NULL DEFAULT false,
  evaluated_by_run uuid,
  PRIMARY KEY (task_id, item_key)
);
CREATE INDEX task_items_tenant_idx ON task_items (tenant_id);
ALTER TABLE task_items ENABLE ROW LEVEL SECURITY;
ALTER TABLE task_items FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON task_items USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON task_items TO app_rw;
GRANT SELECT ON task_items TO app_sequencer;

CREATE TABLE progress_log (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  task_id uuid NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  seq bigint NOT NULL,
  run_id uuid,
  entry text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (task_id, seq)
);
CREATE INDEX progress_log_tenant_idx ON progress_log (tenant_id);
ALTER TABLE progress_log ENABLE ROW LEVEL SECURITY;
ALTER TABLE progress_log FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON progress_log USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT ON progress_log TO app_rw;
GRANT SELECT ON progress_log TO app_sequencer;

CREATE TABLE task_artifacts (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  task_id uuid NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  kind text NOT NULL,
  ref text NOT NULL,
  version integer NOT NULL DEFAULT 0
);
CREATE INDEX task_artifacts_tenant_idx ON task_artifacts (tenant_id);
ALTER TABLE task_artifacts ENABLE ROW LEVEL SECURITY;
ALTER TABLE task_artifacts FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON task_artifacts USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON task_artifacts TO app_rw;
GRANT SELECT ON task_artifacts TO app_sequencer;

CREATE TABLE artifact_leases (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  artifact_id uuid NOT NULL REFERENCES task_artifacts(id) ON DELETE CASCADE,
  run_id uuid NOT NULL,
  expires_at timestamptz NOT NULL,
  PRIMARY KEY (artifact_id)
);
CREATE INDEX artifact_leases_tenant_idx ON artifact_leases (tenant_id);
ALTER TABLE artifact_leases ENABLE ROW LEVEL SECURITY;
ALTER TABLE artifact_leases FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON artifact_leases USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON artifact_leases TO app_rw;
GRANT SELECT ON artifact_leases TO app_sequencer;

CREATE TABLE runs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  task_id uuid REFERENCES tasks(id),
  skill_pack_hash bytea,
  placement text NOT NULL CHECK (placement IN ('node','server')),
  state text NOT NULL DEFAULT 'queued',
  taint text NOT NULL DEFAULT 'clean' CHECK (taint IN ('clean','external')),
  budget_tokens bigint,
  budget_steps integer,
  budget_ms bigint,
  budget_cost numeric,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX runs_tenant_idx ON runs (tenant_id);
ALTER TABLE runs ENABLE ROW LEVEL SECURITY;
ALTER TABLE runs FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON runs USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON runs TO app_rw;
GRANT SELECT ON runs TO app_sequencer;

CREATE TABLE approvals (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  task_id uuid REFERENCES tasks(id),
  proposal_id uuid,
  run_id uuid REFERENCES runs(id),
  tool_id uuid REFERENCES tool_registry(id),
  tier text NOT NULL,
  preview jsonb NOT NULL,
  decided_by uuid REFERENCES users(id),
  decision text CHECK (decision IN ('approved','denied')),
  created_at timestamptz NOT NULL DEFAULT now(),
  CHECK (task_id IS NOT NULL OR proposal_id IS NOT NULL)
);
CREATE INDEX approvals_tenant_idx ON approvals (tenant_id);
ALTER TABLE approvals ENABLE ROW LEVEL SECURITY;
ALTER TABLE approvals FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON approvals USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON approvals TO app_rw;
GRANT SELECT ON approvals TO app_sequencer;

CREATE TABLE triggers (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  kind text NOT NULL,
  condition jsonb NOT NULL,
  cooldown_seconds integer NOT NULL DEFAULT 0,
  level text NOT NULL CHECK (level IN ('notify','question','review'))
);
CREATE INDEX triggers_tenant_idx ON triggers (tenant_id);
CREATE INDEX triggers_scope_idx ON triggers (tenant_id, scope_id);
ALTER TABLE triggers ENABLE ROW LEVEL SECURITY;
ALTER TABLE triggers FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON triggers USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT, INSERT, UPDATE, DELETE ON triggers TO app_rw;
GRANT SELECT ON triggers TO app_sequencer;

CREATE TABLE proposals (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  trigger_id uuid REFERENCES triggers(id),
  level text NOT NULL CHECK (level IN ('notify','question','review')),
  suggestion text NOT NULL,
  preview jsonb,
  state text NOT NULL DEFAULT 'open' CHECK (state IN ('open','approved','dismissed','answered')),
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX proposals_tenant_idx ON proposals (tenant_id);
CREATE INDEX proposals_scope_idx ON proposals (tenant_id, scope_id);
ALTER TABLE proposals ENABLE ROW LEVEL SECURITY;
ALTER TABLE proposals FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON proposals USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT, INSERT, UPDATE, DELETE ON proposals TO app_rw;
GRANT SELECT ON proposals TO app_sequencer;

CREATE TABLE run_steps (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  run_id uuid NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
  seq integer NOT NULL,
  kind text NOT NULL,
  model text,
  tool_id uuid,
  tool_hash bytea,
  tokens_in bigint NOT NULL DEFAULT 0,
  tokens_out bigint NOT NULL DEFAULT 0,
  tokens_cached bigint NOT NULL DEFAULT 0,
  model_ms bigint NOT NULL DEFAULT 0,
  tool_ms bigint NOT NULL DEFAULT 0,
  cost numeric NOT NULL DEFAULT 0,
  checkpoint jsonb,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (run_id, seq)
);
CREATE INDEX run_steps_tenant_idx ON run_steps (tenant_id);
ALTER TABLE run_steps ENABLE ROW LEVEL SECURITY;
ALTER TABLE run_steps FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON run_steps USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT ON run_steps TO app_rw;
GRANT SELECT ON run_steps TO app_sequencer;

CREATE TABLE decision_log (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  run_id uuid,
  kind text NOT NULL,
  inputs jsonb NOT NULL,
  choice text NOT NULL,
  outcome text,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX decision_log_tenant_idx ON decision_log (tenant_id);
ALTER TABLE decision_log ENABLE ROW LEVEL SECURITY;
ALTER TABLE decision_log FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON decision_log USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT ON decision_log TO app_rw;
GRANT SELECT ON decision_log TO app_sequencer;

CREATE TABLE relay_jobs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  kind text NOT NULL,
  input_refs jsonb NOT NULL DEFAULT '[]',
  schema_version integer NOT NULL DEFAULT 1,
  cause text,
  state text NOT NULL DEFAULT 'queued' CHECK (state IN ('queued','offered','claimed','completed','failed')),
  claimed_by_device uuid REFERENCES devices(id),
  attempts integer NOT NULL DEFAULT 0,
  result_hash bytea,
  signature bytea,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX relay_jobs_tenant_idx ON relay_jobs (tenant_id);
CREATE INDEX relay_jobs_scope_idx ON relay_jobs (tenant_id, scope_id);
ALTER TABLE relay_jobs ENABLE ROW LEVEL SECURITY;
ALTER TABLE relay_jobs FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON relay_jobs USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT, INSERT, UPDATE, DELETE ON relay_jobs TO app_rw;
GRANT SELECT ON relay_jobs TO app_sequencer;

CREATE TABLE change_scores (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  scope_id uuid NOT NULL REFERENCES scopes(id) ON DELETE CASCADE,
  community_key text NOT NULL,
  score real NOT NULL DEFAULT 0,
  last_delta_at timestamptz,
  last_rewrite_seq bigint NOT NULL DEFAULT 0,
  PRIMARY KEY (scope_id, community_key)
);
CREATE INDEX change_scores_tenant_idx ON change_scores (tenant_id);
ALTER TABLE change_scores ENABLE ROW LEVEL SECURITY;
ALTER TABLE change_scores FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON change_scores USING (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids())) WITH CHECK (tenant_id = app_tenant() AND scope_id = ANY (app_scope_ids()));
GRANT SELECT, INSERT, UPDATE, DELETE ON change_scores TO app_rw;
GRANT SELECT ON change_scores TO app_sequencer;

CREATE TABLE usage_meter (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  user_id uuid NOT NULL REFERENCES users(id),
  provider text NOT NULL,
  job_kind text NOT NULL,
  day date NOT NULL,
  runs integer NOT NULL DEFAULT 0,
  seconds bigint NOT NULL DEFAULT 0,
  tokens_reported bigint,
  PRIMARY KEY (tenant_id, user_id, provider, job_kind, day)
);
ALTER TABLE usage_meter ENABLE ROW LEVEL SECURITY;
ALTER TABLE usage_meter FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON usage_meter USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON usage_meter TO app_rw;
GRANT SELECT ON usage_meter TO app_sequencer;

CREATE TABLE eval_runs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid REFERENCES tenants(id) ON DELETE CASCADE,
  suite text NOT NULL,
  suite_version text NOT NULL,
  build text NOT NULL,
  k integer NOT NULL,
  pass_k real NOT NULL,
  score real NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX eval_runs_tenant_idx ON eval_runs (tenant_id);
ALTER TABLE eval_runs ENABLE ROW LEVEL SECURITY;
ALTER TABLE eval_runs FORCE ROW LEVEL SECURITY;
CREATE POLICY global_read ON eval_runs FOR SELECT USING (tenant_id IS NULL OR tenant_id = app_tenant());
CREATE POLICY tenant_write ON eval_runs FOR ALL USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON eval_runs TO app_rw;
GRANT SELECT ON eval_runs TO app_sequencer;

CREATE TABLE secrets (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  ciphertext bytea NOT NULL,
  wrapped_dek bytea NOT NULL,
  key_version integer NOT NULL
);
CREATE INDEX secrets_tenant_idx ON secrets (tenant_id);
ALTER TABLE secrets ENABLE ROW LEVEL SECURITY;
ALTER TABLE secrets FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON secrets USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT, UPDATE ON secrets TO app_vault;

CREATE TABLE audit_log (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  seq bigint NOT NULL,
  actor text NOT NULL,
  action text NOT NULL,
  target text,
  tool_hash bytea,
  payload_hash bytea NOT NULL,
  prev_hash bytea NOT NULL,
  hash bytea NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (tenant_id, seq)
);
ALTER TABLE audit_log ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_log FORCE ROW LEVEL SECURITY;
CREATE POLICY isolation ON audit_log USING (tenant_id = app_tenant()) WITH CHECK (tenant_id = app_tenant());
GRANT SELECT, INSERT ON audit_log TO app_rw;
GRANT SELECT ON audit_log TO app_sequencer;

-- The audit log can never change, even if a grant is added by mistake.
CREATE FUNCTION audit_log_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'audit_log is insert-only';
END $$;
CREATE TRIGGER audit_log_no_update BEFORE UPDATE OR DELETE ON audit_log FOR EACH ROW EXECUTE FUNCTION audit_log_immutable();
CREATE TRIGGER audit_log_no_truncate BEFORE TRUNCATE ON audit_log FOR EACH STATEMENT EXECUTE FUNCTION audit_log_immutable();

-- users is global identity: visible to its owner and to members of the current tenant.
CREATE POLICY users_visible ON users FOR SELECT USING (id = app_user() OR EXISTS (SELECT 1 FROM memberships m WHERE m.user_id = users.id AND m.tenant_id = app_tenant()));
CREATE POLICY users_self_write ON users FOR UPDATE USING (id = app_user()) WITH CHECK (id = app_user());
