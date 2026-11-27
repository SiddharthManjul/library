-- Schema of the Library IDE database, dumped from the migrations by `cargo xtask schema-dump`.
-- Read-only reference for agents. The migrations in migrations/ are the source.

CREATE FUNCTION public.app_scope_ids() RETURNS uuid[]
    LANGUAGE sql STABLE
    AS $$ SELECT coalesce(string_to_array(nullif(current_setting('app.scope_ids', true), ''), ',')::uuid[], '{}'::uuid[]) $$;


CREATE FUNCTION public.app_tenant() RETURNS uuid
    LANGUAGE sql STABLE
    AS $$ SELECT nullif(current_setting('app.tenant_id', true), '')::uuid $$;


CREATE FUNCTION public.app_user() RETURNS uuid
    LANGUAGE sql STABLE
    AS $$ SELECT nullif(current_setting('app.user_id', true), '')::uuid $$;


CREATE FUNCTION public.audit_log_immutable() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
  RAISE EXCEPTION 'audit_log is insert-only';
END $$;



CREATE TABLE public.approvals (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    task_id uuid,
    proposal_id uuid,
    run_id uuid,
    tool_id uuid,
    tier text NOT NULL,
    preview jsonb NOT NULL,
    decided_by uuid,
    decision text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT approvals_check CHECK (((task_id IS NOT NULL) OR (proposal_id IS NOT NULL))),
    CONSTRAINT approvals_decision_check CHECK ((decision = ANY (ARRAY['approved'::text, 'denied'::text])))
);
ALTER TABLE ONLY public.approvals ALTER COLUMN id SET (pg_attver=1394910600);
ALTER TABLE ONLY public.approvals ALTER COLUMN tenant_id SET (pg_attver=1394910600);
ALTER TABLE ONLY public.approvals ALTER COLUMN task_id SET (pg_attver=1394910600);
ALTER TABLE ONLY public.approvals ALTER COLUMN proposal_id SET (pg_attver=1394910600);
ALTER TABLE ONLY public.approvals ALTER COLUMN run_id SET (pg_attver=1394910600);
ALTER TABLE ONLY public.approvals ALTER COLUMN tool_id SET (pg_attver=1394910600);
ALTER TABLE ONLY public.approvals ALTER COLUMN tier SET (pg_attver=1394910600);
ALTER TABLE ONLY public.approvals ALTER COLUMN preview SET (pg_attver=1394910600);
ALTER TABLE ONLY public.approvals ALTER COLUMN decided_by SET (pg_attver=1394910600);
ALTER TABLE ONLY public.approvals ALTER COLUMN decision SET (pg_attver=1394910600);
ALTER TABLE ONLY public.approvals ALTER COLUMN created_at SET (pg_attver=1394910600);

ALTER TABLE ONLY public.approvals FORCE ROW LEVEL SECURITY;


CREATE TABLE public.artifact_leases (
    tenant_id uuid NOT NULL,
    artifact_id uuid NOT NULL,
    run_id uuid NOT NULL,
    expires_at timestamp with time zone NOT NULL
);
ALTER TABLE ONLY public.artifact_leases ALTER COLUMN tenant_id SET (pg_attver=1394853136);
ALTER TABLE ONLY public.artifact_leases ALTER COLUMN artifact_id SET (pg_attver=1394853136);
ALTER TABLE ONLY public.artifact_leases ALTER COLUMN run_id SET (pg_attver=1394853136);
ALTER TABLE ONLY public.artifact_leases ALTER COLUMN expires_at SET (pg_attver=1394853136);

ALTER TABLE ONLY public.artifact_leases FORCE ROW LEVEL SECURITY;


CREATE TABLE public.audit_log (
    tenant_id uuid NOT NULL,
    seq bigint NOT NULL,
    actor text NOT NULL,
    action text NOT NULL,
    target text,
    tool_hash bytea,
    payload_hash bytea NOT NULL,
    prev_hash bytea NOT NULL,
    hash bytea NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);
ALTER TABLE ONLY public.audit_log ALTER COLUMN tenant_id SET (pg_attver=1395273576);
ALTER TABLE ONLY public.audit_log ALTER COLUMN seq SET (pg_attver=1395273576);
ALTER TABLE ONLY public.audit_log ALTER COLUMN actor SET (pg_attver=1395273576);
ALTER TABLE ONLY public.audit_log ALTER COLUMN action SET (pg_attver=1395273576);
ALTER TABLE ONLY public.audit_log ALTER COLUMN target SET (pg_attver=1395273576);
ALTER TABLE ONLY public.audit_log ALTER COLUMN tool_hash SET (pg_attver=1395273576);
ALTER TABLE ONLY public.audit_log ALTER COLUMN payload_hash SET (pg_attver=1395273576);
ALTER TABLE ONLY public.audit_log ALTER COLUMN prev_hash SET (pg_attver=1395273576);
ALTER TABLE ONLY public.audit_log ALTER COLUMN hash SET (pg_attver=1395273576);
ALTER TABLE ONLY public.audit_log ALTER COLUMN created_at SET (pg_attver=1395273576);

ALTER TABLE ONLY public.audit_log FORCE ROW LEVEL SECURITY;


CREATE TABLE public.briefs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    audience_kind text NOT NULL,
    audience_id uuid NOT NULL,
    scope_ids uuid[] NOT NULL,
    content text NOT NULL,
    citations jsonb DEFAULT '[]'::jsonb NOT NULL,
    stale boolean DEFAULT false NOT NULL,
    computed_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT briefs_audience_kind_check CHECK ((audience_kind = ANY (ARRAY['leader'::text, 'department'::text])))
);
ALTER TABLE ONLY public.briefs ALTER COLUMN id SET (pg_attver=1394454176);
ALTER TABLE ONLY public.briefs ALTER COLUMN tenant_id SET (pg_attver=1394454176);
ALTER TABLE ONLY public.briefs ALTER COLUMN audience_kind SET (pg_attver=1394454176);
ALTER TABLE ONLY public.briefs ALTER COLUMN audience_id SET (pg_attver=1394454176);
ALTER TABLE ONLY public.briefs ALTER COLUMN scope_ids SET (pg_attver=1394454176);
ALTER TABLE ONLY public.briefs ALTER COLUMN content SET (pg_attver=1394454176);
ALTER TABLE ONLY public.briefs ALTER COLUMN citations SET (pg_attver=1394454176);
ALTER TABLE ONLY public.briefs ALTER COLUMN stale SET (pg_attver=1394454176);
ALTER TABLE ONLY public.briefs ALTER COLUMN computed_at SET (pg_attver=1394454176);

ALTER TABLE ONLY public.briefs FORCE ROW LEVEL SECURITY;


CREATE TABLE public.change_scores (
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    community_key text NOT NULL,
    score real DEFAULT 0 NOT NULL,
    last_delta_at timestamp with time zone,
    last_rewrite_seq bigint DEFAULT 0 NOT NULL
);
ALTER TABLE ONLY public.change_scores ALTER COLUMN tenant_id SET (pg_attver=1395151272);
ALTER TABLE ONLY public.change_scores ALTER COLUMN scope_id SET (pg_attver=1395151272);
ALTER TABLE ONLY public.change_scores ALTER COLUMN community_key SET (pg_attver=1395151272);
ALTER TABLE ONLY public.change_scores ALTER COLUMN score SET (pg_attver=1395151272);
ALTER TABLE ONLY public.change_scores ALTER COLUMN last_delta_at SET (pg_attver=1395151272);
ALTER TABLE ONLY public.change_scores ALTER COLUMN last_rewrite_seq SET (pg_attver=1395151272);

ALTER TABLE ONLY public.change_scores FORCE ROW LEVEL SECURITY;


CREATE TABLE public.community_summaries (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    community_key text NOT NULL,
    member_ids uuid[] DEFAULT '{}'::uuid[] NOT NULL,
    summary text NOT NULL,
    citations jsonb DEFAULT '[]'::jsonb NOT NULL,
    computed_seq bigint NOT NULL
);
ALTER TABLE ONLY public.community_summaries ALTER COLUMN id SET (pg_attver=1394413496);
ALTER TABLE ONLY public.community_summaries ALTER COLUMN tenant_id SET (pg_attver=1394413496);
ALTER TABLE ONLY public.community_summaries ALTER COLUMN scope_id SET (pg_attver=1394413496);
ALTER TABLE ONLY public.community_summaries ALTER COLUMN community_key SET (pg_attver=1394413496);
ALTER TABLE ONLY public.community_summaries ALTER COLUMN member_ids SET (pg_attver=1394413496);
ALTER TABLE ONLY public.community_summaries ALTER COLUMN summary SET (pg_attver=1394413496);
ALTER TABLE ONLY public.community_summaries ALTER COLUMN citations SET (pg_attver=1394413496);
ALTER TABLE ONLY public.community_summaries ALTER COLUMN computed_seq SET (pg_attver=1394413496);

ALTER TABLE ONLY public.community_summaries FORCE ROW LEVEL SECURITY;


CREATE TABLE public.connections (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    provider text NOT NULL,
    server_url text NOT NULL,
    auth_mode text NOT NULL,
    oauth_client_ref uuid,
    status text DEFAULT 'pending'::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT connections_auth_mode_check CHECK ((auth_mode = ANY (ARRAY['automatic'::text, 'our_app'::text, 'customer_app'::text, 'api_key'::text, 'open'::text]))),
    CONSTRAINT connections_status_check CHECK ((status = ANY (ARRAY['pending'::text, 'connected'::text, 'expired'::text, 'revoked'::text])))
);
ALTER TABLE ONLY public.connections ALTER COLUMN id SET (pg_attver=1394006376);
ALTER TABLE ONLY public.connections ALTER COLUMN tenant_id SET (pg_attver=1394006376);
ALTER TABLE ONLY public.connections ALTER COLUMN scope_id SET (pg_attver=1394006376);
ALTER TABLE ONLY public.connections ALTER COLUMN provider SET (pg_attver=1394006376);
ALTER TABLE ONLY public.connections ALTER COLUMN server_url SET (pg_attver=1394006376);
ALTER TABLE ONLY public.connections ALTER COLUMN auth_mode SET (pg_attver=1394006376);
ALTER TABLE ONLY public.connections ALTER COLUMN oauth_client_ref SET (pg_attver=1394006376);
ALTER TABLE ONLY public.connections ALTER COLUMN status SET (pg_attver=1394006376);
ALTER TABLE ONLY public.connections ALTER COLUMN created_at SET (pg_attver=1394006376);

ALTER TABLE ONLY public.connections FORCE ROW LEVEL SECURITY;


CREATE TABLE public.decision_log (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    run_id uuid,
    kind text NOT NULL,
    inputs jsonb NOT NULL,
    choice text NOT NULL,
    outcome text,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);
ALTER TABLE ONLY public.decision_log ALTER COLUMN id SET (pg_attver=1395074784);
ALTER TABLE ONLY public.decision_log ALTER COLUMN tenant_id SET (pg_attver=1395074784);
ALTER TABLE ONLY public.decision_log ALTER COLUMN run_id SET (pg_attver=1395074784);
ALTER TABLE ONLY public.decision_log ALTER COLUMN kind SET (pg_attver=1395074784);
ALTER TABLE ONLY public.decision_log ALTER COLUMN inputs SET (pg_attver=1395074784);
ALTER TABLE ONLY public.decision_log ALTER COLUMN choice SET (pg_attver=1395074784);
ALTER TABLE ONLY public.decision_log ALTER COLUMN outcome SET (pg_attver=1395074784);
ALTER TABLE ONLY public.decision_log ALTER COLUMN created_at SET (pg_attver=1395074784);

ALTER TABLE ONLY public.decision_log FORCE ROW LEVEL SECURITY;


CREATE TABLE public.deltas (
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    seq bigint NOT NULL,
    ops jsonb NOT NULL,
    state_root bytea NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);
ALTER TABLE ONLY public.deltas ALTER COLUMN tenant_id SET (pg_attver=1394545104);
ALTER TABLE ONLY public.deltas ALTER COLUMN scope_id SET (pg_attver=1394545104);
ALTER TABLE ONLY public.deltas ALTER COLUMN seq SET (pg_attver=1394545104);
ALTER TABLE ONLY public.deltas ALTER COLUMN ops SET (pg_attver=1394545104);
ALTER TABLE ONLY public.deltas ALTER COLUMN state_root SET (pg_attver=1394545104);
ALTER TABLE ONLY public.deltas ALTER COLUMN created_at SET (pg_attver=1394545104);

ALTER TABLE ONLY public.deltas FORCE ROW LEVEL SECURITY;


CREATE TABLE public.devices (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    public_key bytea NOT NULL,
    signing_public_key bytea NOT NULL,
    last_seen timestamp with time zone,
    revoked_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    tenant_id uuid NOT NULL
);
ALTER TABLE ONLY public.devices ALTER COLUMN id SET (pg_attver=1393983608);
ALTER TABLE ONLY public.devices ALTER COLUMN user_id SET (pg_attver=1393983608);
ALTER TABLE ONLY public.devices ALTER COLUMN public_key SET (pg_attver=1393983608);
ALTER TABLE ONLY public.devices ALTER COLUMN signing_public_key SET (pg_attver=1393983608);
ALTER TABLE ONLY public.devices ALTER COLUMN last_seen SET (pg_attver=1393983608);
ALTER TABLE ONLY public.devices ALTER COLUMN revoked_at SET (pg_attver=1393983608);
ALTER TABLE ONLY public.devices ALTER COLUMN created_at SET (pg_attver=1393983608);
ALTER TABLE ONLY public.devices ALTER COLUMN tenant_id SET (pg_attver=1395412352);

ALTER TABLE ONLY public.devices FORCE ROW LEVEL SECURITY;


CREATE TABLE public.entities (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    kind text NOT NULL,
    name text NOT NULL,
    summary text,
    external_ids jsonb DEFAULT '{}'::jsonb NOT NULL,
    tsv tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, ((COALESCE(name, ''::text) || ' '::text) || COALESCE(summary, ''::text)))) STORED,
    seq bigint DEFAULT 0 NOT NULL
);
ALTER TABLE ONLY public.entities ALTER COLUMN id SET (pg_attver=1394226648);
ALTER TABLE ONLY public.entities ALTER COLUMN tenant_id SET (pg_attver=1394226648);
ALTER TABLE ONLY public.entities ALTER COLUMN scope_id SET (pg_attver=1394226648);
ALTER TABLE ONLY public.entities ALTER COLUMN kind SET (pg_attver=1394226648);
ALTER TABLE ONLY public.entities ALTER COLUMN name SET (pg_attver=1394226648);
ALTER TABLE ONLY public.entities ALTER COLUMN summary SET (pg_attver=1394226648);
ALTER TABLE ONLY public.entities ALTER COLUMN external_ids SET (pg_attver=1394226648);
ALTER TABLE ONLY public.entities ALTER COLUMN tsv SET (pg_attver=1394226648);
ALTER TABLE ONLY public.entities ALTER COLUMN seq SET (pg_attver=1394226648);

ALTER TABLE ONLY public.entities FORCE ROW LEVEL SECURITY;


CREATE TABLE public.entity_aliases (
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    entity_id uuid NOT NULL,
    alias text NOT NULL,
    source text NOT NULL
);
ALTER TABLE ONLY public.entity_aliases ALTER COLUMN tenant_id SET (pg_attver=1394264792);
ALTER TABLE ONLY public.entity_aliases ALTER COLUMN scope_id SET (pg_attver=1394264792);
ALTER TABLE ONLY public.entity_aliases ALTER COLUMN entity_id SET (pg_attver=1394264792);
ALTER TABLE ONLY public.entity_aliases ALTER COLUMN alias SET (pg_attver=1394264792);
ALTER TABLE ONLY public.entity_aliases ALTER COLUMN source SET (pg_attver=1394264792);

ALTER TABLE ONLY public.entity_aliases FORCE ROW LEVEL SECURITY;


CREATE TABLE public.eval_runs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid,
    suite text NOT NULL,
    suite_version text NOT NULL,
    build text NOT NULL,
    k integer NOT NULL,
    pass_k real NOT NULL,
    score real NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);
ALTER TABLE ONLY public.eval_runs ALTER COLUMN id SET (pg_attver=1395221672);
ALTER TABLE ONLY public.eval_runs ALTER COLUMN tenant_id SET (pg_attver=1395221672);
ALTER TABLE ONLY public.eval_runs ALTER COLUMN suite SET (pg_attver=1395221672);
ALTER TABLE ONLY public.eval_runs ALTER COLUMN suite_version SET (pg_attver=1395221672);
ALTER TABLE ONLY public.eval_runs ALTER COLUMN build SET (pg_attver=1395221672);
ALTER TABLE ONLY public.eval_runs ALTER COLUMN k SET (pg_attver=1395221672);
ALTER TABLE ONLY public.eval_runs ALTER COLUMN pass_k SET (pg_attver=1395221672);
ALTER TABLE ONLY public.eval_runs ALTER COLUMN score SET (pg_attver=1395221672);
ALTER TABLE ONLY public.eval_runs ALTER COLUMN created_at SET (pg_attver=1395221672);

ALTER TABLE ONLY public.eval_runs FORCE ROW LEVEL SECURITY;


CREATE TABLE public.events (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    connection_id uuid,
    source text NOT NULL,
    external_id text NOT NULL,
    kind text NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    received_at timestamp with time zone DEFAULT now() NOT NULL,
    payload jsonb NOT NULL,
    taint text DEFAULT 'external'::text NOT NULL,
    CONSTRAINT events_taint_check CHECK ((taint = ANY (ARRAY['external'::text, 'clean'::text])))
);
ALTER TABLE ONLY public.events ALTER COLUMN id SET (pg_attver=1394109608);
ALTER TABLE ONLY public.events ALTER COLUMN tenant_id SET (pg_attver=1394109608);
ALTER TABLE ONLY public.events ALTER COLUMN scope_id SET (pg_attver=1394109608);
ALTER TABLE ONLY public.events ALTER COLUMN connection_id SET (pg_attver=1394109608);
ALTER TABLE ONLY public.events ALTER COLUMN source SET (pg_attver=1394109608);
ALTER TABLE ONLY public.events ALTER COLUMN external_id SET (pg_attver=1394109608);
ALTER TABLE ONLY public.events ALTER COLUMN kind SET (pg_attver=1394109608);
ALTER TABLE ONLY public.events ALTER COLUMN occurred_at SET (pg_attver=1394109608);
ALTER TABLE ONLY public.events ALTER COLUMN received_at SET (pg_attver=1394109608);
ALTER TABLE ONLY public.events ALTER COLUMN payload SET (pg_attver=1394109608);
ALTER TABLE ONLY public.events ALTER COLUMN taint SET (pg_attver=1394109608);

ALTER TABLE ONLY public.events FORCE ROW LEVEL SECURITY;


CREATE TABLE public.facts (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    subject_id uuid NOT NULL,
    relation text NOT NULL,
    object_id uuid,
    statement text NOT NULL,
    valid_from timestamp with time zone NOT NULL,
    valid_to timestamp with time zone,
    recorded_at timestamp with time zone DEFAULT now() NOT NULL,
    retracted_at timestamp with time zone,
    source_event_id uuid NOT NULL,
    source_ref jsonb DEFAULT '{}'::jsonb NOT NULL,
    extractor_version text NOT NULL,
    confidence real NOT NULL,
    taint text DEFAULT 'external'::text NOT NULL,
    tsv tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, statement)) STORED,
    seq bigint DEFAULT 0 NOT NULL,
    CONSTRAINT facts_taint_check CHECK ((taint = ANY (ARRAY['external'::text, 'clean'::text])))
);
ALTER TABLE ONLY public.facts ALTER COLUMN id SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN tenant_id SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN scope_id SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN subject_id SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN relation SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN object_id SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN statement SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN valid_from SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN valid_to SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN recorded_at SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN retracted_at SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN source_event_id SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN source_ref SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN extractor_version SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN confidence SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN taint SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN tsv SET (pg_attver=1394355632);
ALTER TABLE ONLY public.facts ALTER COLUMN seq SET (pg_attver=1394355632);

ALTER TABLE ONLY public.facts FORCE ROW LEVEL SECURITY;


CREATE TABLE public.identities (
    provider text NOT NULL,
    subject text NOT NULL,
    user_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT identities_provider_check CHECK ((provider = ANY (ARRAY['google'::text, 'microsoft'::text, 'github'::text])))
);
ALTER TABLE ONLY public.identities ALTER COLUMN provider SET (pg_attver=1395428376);
ALTER TABLE ONLY public.identities ALTER COLUMN subject SET (pg_attver=1395428376);
ALTER TABLE ONLY public.identities ALTER COLUMN user_id SET (pg_attver=1395428376);
ALTER TABLE ONLY public.identities ALTER COLUMN created_at SET (pg_attver=1395428376);

ALTER TABLE ONLY public.identities FORCE ROW LEVEL SECURITY;


CREATE TABLE public.memberships (
    tenant_id uuid NOT NULL,
    user_id uuid NOT NULL,
    role text NOT NULL,
    CONSTRAINT memberships_role_check CHECK ((role = ANY (ARRAY['member'::text, 'department_lead'::text, 'product_leader'::text, 'c_suite'::text, 'admin'::text])))
);
ALTER TABLE ONLY public.memberships ALTER COLUMN tenant_id SET (pg_attver=1393885128);
ALTER TABLE ONLY public.memberships ALTER COLUMN user_id SET (pg_attver=1393885128);
ALTER TABLE ONLY public.memberships ALTER COLUMN role SET (pg_attver=1393885128);

ALTER TABLE ONLY public.memberships FORCE ROW LEVEL SECURITY;


CREATE TABLE public.merge_proposals (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    entity_a uuid NOT NULL,
    entity_b uuid NOT NULL,
    score real NOT NULL,
    state text DEFAULT 'pending'::text NOT NULL,
    decided_by uuid,
    CONSTRAINT merge_proposals_state_check CHECK ((state = ANY (ARRAY['pending'::text, 'accepted'::text, 'rejected'::text])))
);
ALTER TABLE ONLY public.merge_proposals ALTER COLUMN id SET (pg_attver=1394300224);
ALTER TABLE ONLY public.merge_proposals ALTER COLUMN tenant_id SET (pg_attver=1394300224);
ALTER TABLE ONLY public.merge_proposals ALTER COLUMN scope_id SET (pg_attver=1394300224);
ALTER TABLE ONLY public.merge_proposals ALTER COLUMN entity_a SET (pg_attver=1394300224);
ALTER TABLE ONLY public.merge_proposals ALTER COLUMN entity_b SET (pg_attver=1394300224);
ALTER TABLE ONLY public.merge_proposals ALTER COLUMN score SET (pg_attver=1394300224);
ALTER TABLE ONLY public.merge_proposals ALTER COLUMN state SET (pg_attver=1394300224);
ALTER TABLE ONLY public.merge_proposals ALTER COLUMN decided_by SET (pg_attver=1394300224);

ALTER TABLE ONLY public.merge_proposals FORCE ROW LEVEL SECURITY;


CREATE TABLE public.oauth_tokens (
    tenant_id uuid NOT NULL,
    connection_id uuid NOT NULL,
    user_id uuid NOT NULL,
    secret_ref uuid NOT NULL,
    granted_scopes text[] DEFAULT '{}'::text[] NOT NULL,
    expires_at timestamp with time zone
);
ALTER TABLE ONLY public.oauth_tokens ALTER COLUMN tenant_id SET (pg_attver=1394077304);
ALTER TABLE ONLY public.oauth_tokens ALTER COLUMN connection_id SET (pg_attver=1394077304);
ALTER TABLE ONLY public.oauth_tokens ALTER COLUMN user_id SET (pg_attver=1394077304);
ALTER TABLE ONLY public.oauth_tokens ALTER COLUMN secret_ref SET (pg_attver=1394077304);
ALTER TABLE ONLY public.oauth_tokens ALTER COLUMN granted_scopes SET (pg_attver=1394077304);
ALTER TABLE ONLY public.oauth_tokens ALTER COLUMN expires_at SET (pg_attver=1394077304);

ALTER TABLE ONLY public.oauth_tokens FORCE ROW LEVEL SECURITY;


CREATE TABLE public.plans (
    tenant_id uuid NOT NULL,
    plan text NOT NULL,
    features jsonb DEFAULT '{}'::jsonb NOT NULL,
    limits jsonb DEFAULT '{}'::jsonb NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT plans_plan_check CHECK ((plan = ANY (ARRAY['starter'::text, 'team'::text, 'business'::text, 'enterprise'::text])))
);
ALTER TABLE ONLY public.plans ALTER COLUMN tenant_id SET (pg_attver=1393855792);
ALTER TABLE ONLY public.plans ALTER COLUMN plan SET (pg_attver=1393855792);
ALTER TABLE ONLY public.plans ALTER COLUMN features SET (pg_attver=1393855792);
ALTER TABLE ONLY public.plans ALTER COLUMN limits SET (pg_attver=1393855792);
ALTER TABLE ONLY public.plans ALTER COLUMN updated_at SET (pg_attver=1393855792);

ALTER TABLE ONLY public.plans FORCE ROW LEVEL SECURITY;


CREATE TABLE public.playbook_items (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    department text NOT NULL,
    content text NOT NULL,
    helpful integer DEFAULT 0 NOT NULL,
    harmful integer DEFAULT 0 NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    source_run_id uuid,
    approved_by uuid,
    CONSTRAINT playbook_items_status_check CHECK ((status = ANY (ARRAY['pending'::text, 'active'::text, 'deprecated'::text])))
);
ALTER TABLE ONLY public.playbook_items ALTER COLUMN id SET (pg_attver=1394488576);
ALTER TABLE ONLY public.playbook_items ALTER COLUMN tenant_id SET (pg_attver=1394488576);
ALTER TABLE ONLY public.playbook_items ALTER COLUMN scope_id SET (pg_attver=1394488576);
ALTER TABLE ONLY public.playbook_items ALTER COLUMN department SET (pg_attver=1394488576);
ALTER TABLE ONLY public.playbook_items ALTER COLUMN content SET (pg_attver=1394488576);
ALTER TABLE ONLY public.playbook_items ALTER COLUMN helpful SET (pg_attver=1394488576);
ALTER TABLE ONLY public.playbook_items ALTER COLUMN harmful SET (pg_attver=1394488576);
ALTER TABLE ONLY public.playbook_items ALTER COLUMN status SET (pg_attver=1394488576);
ALTER TABLE ONLY public.playbook_items ALTER COLUMN source_run_id SET (pg_attver=1394488576);
ALTER TABLE ONLY public.playbook_items ALTER COLUMN approved_by SET (pg_attver=1394488576);

ALTER TABLE ONLY public.playbook_items FORCE ROW LEVEL SECURITY;


CREATE TABLE public.progress_log (
    tenant_id uuid NOT NULL,
    task_id uuid NOT NULL,
    seq bigint NOT NULL,
    run_id uuid,
    entry text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);
ALTER TABLE ONLY public.progress_log ALTER COLUMN tenant_id SET (pg_attver=1394792424);
ALTER TABLE ONLY public.progress_log ALTER COLUMN task_id SET (pg_attver=1394792424);
ALTER TABLE ONLY public.progress_log ALTER COLUMN seq SET (pg_attver=1394792424);
ALTER TABLE ONLY public.progress_log ALTER COLUMN run_id SET (pg_attver=1394792424);
ALTER TABLE ONLY public.progress_log ALTER COLUMN entry SET (pg_attver=1394792424);
ALTER TABLE ONLY public.progress_log ALTER COLUMN created_at SET (pg_attver=1394792424);

ALTER TABLE ONLY public.progress_log FORCE ROW LEVEL SECURITY;


CREATE TABLE public.proposals (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    trigger_id uuid,
    level text NOT NULL,
    suggestion text NOT NULL,
    preview jsonb,
    state text DEFAULT 'open'::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT proposals_level_check CHECK ((level = ANY (ARRAY['notify'::text, 'question'::text, 'review'::text]))),
    CONSTRAINT proposals_state_check CHECK ((state = ANY (ARRAY['open'::text, 'approved'::text, 'dismissed'::text, 'answered'::text])))
);
ALTER TABLE ONLY public.proposals ALTER COLUMN id SET (pg_attver=1394990584);
ALTER TABLE ONLY public.proposals ALTER COLUMN tenant_id SET (pg_attver=1394990584);
ALTER TABLE ONLY public.proposals ALTER COLUMN scope_id SET (pg_attver=1394990584);
ALTER TABLE ONLY public.proposals ALTER COLUMN trigger_id SET (pg_attver=1394990584);
ALTER TABLE ONLY public.proposals ALTER COLUMN level SET (pg_attver=1394990584);
ALTER TABLE ONLY public.proposals ALTER COLUMN suggestion SET (pg_attver=1394990584);
ALTER TABLE ONLY public.proposals ALTER COLUMN preview SET (pg_attver=1394990584);
ALTER TABLE ONLY public.proposals ALTER COLUMN state SET (pg_attver=1394990584);
ALTER TABLE ONLY public.proposals ALTER COLUMN created_at SET (pg_attver=1394990584);

ALTER TABLE ONLY public.proposals FORCE ROW LEVEL SECURITY;


CREATE TABLE public.refresh_tokens (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    user_id uuid NOT NULL,
    device_id uuid NOT NULL,
    family_id uuid NOT NULL,
    token_hash bytea NOT NULL,
    issued_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    used_at timestamp with time zone,
    revoked_at timestamp with time zone
);
ALTER TABLE ONLY public.refresh_tokens ALTER COLUMN id SET (pg_attver=1395454040);
ALTER TABLE ONLY public.refresh_tokens ALTER COLUMN tenant_id SET (pg_attver=1395454040);
ALTER TABLE ONLY public.refresh_tokens ALTER COLUMN user_id SET (pg_attver=1395454040);
ALTER TABLE ONLY public.refresh_tokens ALTER COLUMN device_id SET (pg_attver=1395454040);
ALTER TABLE ONLY public.refresh_tokens ALTER COLUMN family_id SET (pg_attver=1395454040);
ALTER TABLE ONLY public.refresh_tokens ALTER COLUMN token_hash SET (pg_attver=1395454040);
ALTER TABLE ONLY public.refresh_tokens ALTER COLUMN issued_at SET (pg_attver=1395454040);
ALTER TABLE ONLY public.refresh_tokens ALTER COLUMN expires_at SET (pg_attver=1395454040);
ALTER TABLE ONLY public.refresh_tokens ALTER COLUMN used_at SET (pg_attver=1395454040);
ALTER TABLE ONLY public.refresh_tokens ALTER COLUMN revoked_at SET (pg_attver=1395454040);

ALTER TABLE ONLY public.refresh_tokens FORCE ROW LEVEL SECURITY;


CREATE TABLE public.relay_jobs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    kind text NOT NULL,
    input_refs jsonb DEFAULT '[]'::jsonb NOT NULL,
    schema_version integer DEFAULT 1 NOT NULL,
    cause text,
    state text DEFAULT 'queued'::text NOT NULL,
    claimed_by_device uuid,
    attempts integer DEFAULT 0 NOT NULL,
    result_hash bytea,
    signature bytea,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT relay_jobs_state_check CHECK ((state = ANY (ARRAY['queued'::text, 'offered'::text, 'claimed'::text, 'completed'::text, 'failed'::text])))
);
ALTER TABLE ONLY public.relay_jobs ALTER COLUMN id SET (pg_attver=1395103888);
ALTER TABLE ONLY public.relay_jobs ALTER COLUMN tenant_id SET (pg_attver=1395103888);
ALTER TABLE ONLY public.relay_jobs ALTER COLUMN scope_id SET (pg_attver=1395103888);
ALTER TABLE ONLY public.relay_jobs ALTER COLUMN kind SET (pg_attver=1395103888);
ALTER TABLE ONLY public.relay_jobs ALTER COLUMN input_refs SET (pg_attver=1395103888);
ALTER TABLE ONLY public.relay_jobs ALTER COLUMN schema_version SET (pg_attver=1395103888);
ALTER TABLE ONLY public.relay_jobs ALTER COLUMN cause SET (pg_attver=1395103888);
ALTER TABLE ONLY public.relay_jobs ALTER COLUMN state SET (pg_attver=1395103888);
ALTER TABLE ONLY public.relay_jobs ALTER COLUMN claimed_by_device SET (pg_attver=1395103888);
ALTER TABLE ONLY public.relay_jobs ALTER COLUMN attempts SET (pg_attver=1395103888);
ALTER TABLE ONLY public.relay_jobs ALTER COLUMN result_hash SET (pg_attver=1395103888);
ALTER TABLE ONLY public.relay_jobs ALTER COLUMN signature SET (pg_attver=1395103888);
ALTER TABLE ONLY public.relay_jobs ALTER COLUMN created_at SET (pg_attver=1395103888);

ALTER TABLE ONLY public.relay_jobs FORCE ROW LEVEL SECURITY;


CREATE TABLE public.replica_cursors (
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    device_id uuid NOT NULL,
    applied_seq bigint NOT NULL,
    state_root bytea NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);
ALTER TABLE ONLY public.replica_cursors ALTER COLUMN tenant_id SET (pg_attver=1394608688);
ALTER TABLE ONLY public.replica_cursors ALTER COLUMN scope_id SET (pg_attver=1394608688);
ALTER TABLE ONLY public.replica_cursors ALTER COLUMN device_id SET (pg_attver=1394608688);
ALTER TABLE ONLY public.replica_cursors ALTER COLUMN applied_seq SET (pg_attver=1394608688);
ALTER TABLE ONLY public.replica_cursors ALTER COLUMN state_root SET (pg_attver=1394608688);
ALTER TABLE ONLY public.replica_cursors ALTER COLUMN updated_at SET (pg_attver=1394608688);

ALTER TABLE ONLY public.replica_cursors FORCE ROW LEVEL SECURITY;


CREATE TABLE public.run_steps (
    tenant_id uuid NOT NULL,
    run_id uuid NOT NULL,
    seq integer NOT NULL,
    kind text NOT NULL,
    model text,
    tool_id uuid,
    tool_hash bytea,
    tokens_in bigint DEFAULT 0 NOT NULL,
    tokens_out bigint DEFAULT 0 NOT NULL,
    tokens_cached bigint DEFAULT 0 NOT NULL,
    model_ms bigint DEFAULT 0 NOT NULL,
    tool_ms bigint DEFAULT 0 NOT NULL,
    cost numeric DEFAULT 0 NOT NULL,
    checkpoint jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);
ALTER TABLE ONLY public.run_steps ALTER COLUMN tenant_id SET (pg_attver=1395034272);
ALTER TABLE ONLY public.run_steps ALTER COLUMN run_id SET (pg_attver=1395034272);
ALTER TABLE ONLY public.run_steps ALTER COLUMN seq SET (pg_attver=1395034272);
ALTER TABLE ONLY public.run_steps ALTER COLUMN kind SET (pg_attver=1395034272);
ALTER TABLE ONLY public.run_steps ALTER COLUMN model SET (pg_attver=1395034272);
ALTER TABLE ONLY public.run_steps ALTER COLUMN tool_id SET (pg_attver=1395034272);
ALTER TABLE ONLY public.run_steps ALTER COLUMN tool_hash SET (pg_attver=1395034272);
ALTER TABLE ONLY public.run_steps ALTER COLUMN tokens_in SET (pg_attver=1395034272);
ALTER TABLE ONLY public.run_steps ALTER COLUMN tokens_out SET (pg_attver=1395034272);
ALTER TABLE ONLY public.run_steps ALTER COLUMN tokens_cached SET (pg_attver=1395034272);
ALTER TABLE ONLY public.run_steps ALTER COLUMN model_ms SET (pg_attver=1395034272);
ALTER TABLE ONLY public.run_steps ALTER COLUMN tool_ms SET (pg_attver=1395034272);
ALTER TABLE ONLY public.run_steps ALTER COLUMN cost SET (pg_attver=1395034272);
ALTER TABLE ONLY public.run_steps ALTER COLUMN checkpoint SET (pg_attver=1395034272);
ALTER TABLE ONLY public.run_steps ALTER COLUMN created_at SET (pg_attver=1395034272);

ALTER TABLE ONLY public.run_steps FORCE ROW LEVEL SECURITY;


CREATE TABLE public.runs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    task_id uuid,
    skill_pack_hash bytea,
    placement text NOT NULL,
    state text DEFAULT 'queued'::text NOT NULL,
    taint text DEFAULT 'clean'::text NOT NULL,
    budget_tokens bigint,
    budget_steps integer,
    budget_ms bigint,
    budget_cost numeric,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT runs_placement_check CHECK ((placement = ANY (ARRAY['node'::text, 'server'::text]))),
    CONSTRAINT runs_taint_check CHECK ((taint = ANY (ARRAY['clean'::text, 'external'::text])))
);
ALTER TABLE ONLY public.runs ALTER COLUMN id SET (pg_attver=1394874600);
ALTER TABLE ONLY public.runs ALTER COLUMN tenant_id SET (pg_attver=1394874600);
ALTER TABLE ONLY public.runs ALTER COLUMN task_id SET (pg_attver=1394874600);
ALTER TABLE ONLY public.runs ALTER COLUMN skill_pack_hash SET (pg_attver=1394874600);
ALTER TABLE ONLY public.runs ALTER COLUMN placement SET (pg_attver=1394874600);
ALTER TABLE ONLY public.runs ALTER COLUMN state SET (pg_attver=1394874600);
ALTER TABLE ONLY public.runs ALTER COLUMN taint SET (pg_attver=1394874600);
ALTER TABLE ONLY public.runs ALTER COLUMN budget_tokens SET (pg_attver=1394874600);
ALTER TABLE ONLY public.runs ALTER COLUMN budget_steps SET (pg_attver=1394874600);
ALTER TABLE ONLY public.runs ALTER COLUMN budget_ms SET (pg_attver=1394874600);
ALTER TABLE ONLY public.runs ALTER COLUMN budget_cost SET (pg_attver=1394874600);
ALTER TABLE ONLY public.runs ALTER COLUMN created_at SET (pg_attver=1394874600);

ALTER TABLE ONLY public.runs FORCE ROW LEVEL SECURITY;


CREATE TABLE public.scope_keys (
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    key_version integer NOT NULL,
    wrapped_key bytea NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone
);
ALTER TABLE ONLY public.scope_keys ALTER COLUMN tenant_id SET (pg_attver=1394645640);
ALTER TABLE ONLY public.scope_keys ALTER COLUMN scope_id SET (pg_attver=1394645640);
ALTER TABLE ONLY public.scope_keys ALTER COLUMN key_version SET (pg_attver=1394645640);
ALTER TABLE ONLY public.scope_keys ALTER COLUMN wrapped_key SET (pg_attver=1394645640);
ALTER TABLE ONLY public.scope_keys ALTER COLUMN valid_from SET (pg_attver=1394645640);
ALTER TABLE ONLY public.scope_keys ALTER COLUMN valid_to SET (pg_attver=1394645640);

ALTER TABLE ONLY public.scope_keys FORCE ROW LEVEL SECURITY;


CREATE TABLE public.scope_members (
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    user_id uuid NOT NULL,
    scope_role text NOT NULL,
    CONSTRAINT scope_members_scope_role_check CHECK ((scope_role = ANY (ARRAY['member'::text, 'lead'::text])))
);
ALTER TABLE ONLY public.scope_members ALTER COLUMN tenant_id SET (pg_attver=1393950528);
ALTER TABLE ONLY public.scope_members ALTER COLUMN scope_id SET (pg_attver=1393950528);
ALTER TABLE ONLY public.scope_members ALTER COLUMN user_id SET (pg_attver=1393950528);
ALTER TABLE ONLY public.scope_members ALTER COLUMN scope_role SET (pg_attver=1393950528);

ALTER TABLE ONLY public.scope_members FORCE ROW LEVEL SECURITY;


CREATE TABLE public.scopes (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    kind text NOT NULL,
    name text NOT NULL,
    fold_id uuid,
    sealed boolean DEFAULT false NOT NULL,
    CONSTRAINT scopes_kind_check CHECK ((kind = ANY (ARRAY['department'::text, 'fold'::text])))
);
ALTER TABLE ONLY public.scopes ALTER COLUMN id SET (pg_attver=1393917216);
ALTER TABLE ONLY public.scopes ALTER COLUMN tenant_id SET (pg_attver=1393917216);
ALTER TABLE ONLY public.scopes ALTER COLUMN kind SET (pg_attver=1393917216);
ALTER TABLE ONLY public.scopes ALTER COLUMN name SET (pg_attver=1393917216);
ALTER TABLE ONLY public.scopes ALTER COLUMN fold_id SET (pg_attver=1393917216);
ALTER TABLE ONLY public.scopes ALTER COLUMN sealed SET (pg_attver=1393917216);

ALTER TABLE ONLY public.scopes FORCE ROW LEVEL SECURITY;


CREATE TABLE public.secrets (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    ciphertext bytea NOT NULL,
    wrapped_dek bytea NOT NULL,
    key_version integer NOT NULL
);
ALTER TABLE ONLY public.secrets ALTER COLUMN id SET (pg_attver=1395249200);
ALTER TABLE ONLY public.secrets ALTER COLUMN tenant_id SET (pg_attver=1395249200);
ALTER TABLE ONLY public.secrets ALTER COLUMN ciphertext SET (pg_attver=1395249200);
ALTER TABLE ONLY public.secrets ALTER COLUMN wrapped_dek SET (pg_attver=1395249200);
ALTER TABLE ONLY public.secrets ALTER COLUMN key_version SET (pg_attver=1395249200);

ALTER TABLE ONLY public.secrets FORCE ROW LEVEL SECURITY;


CREATE TABLE public.skill_packs (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid,
    department text NOT NULL,
    version integer NOT NULL,
    hash bytea NOT NULL,
    manifest jsonb NOT NULL
);
ALTER TABLE ONLY public.skill_packs ALTER COLUMN id SET (pg_attver=1394196512);
ALTER TABLE ONLY public.skill_packs ALTER COLUMN tenant_id SET (pg_attver=1394196512);
ALTER TABLE ONLY public.skill_packs ALTER COLUMN department SET (pg_attver=1394196512);
ALTER TABLE ONLY public.skill_packs ALTER COLUMN version SET (pg_attver=1394196512);
ALTER TABLE ONLY public.skill_packs ALTER COLUMN hash SET (pg_attver=1394196512);
ALTER TABLE ONLY public.skill_packs ALTER COLUMN manifest SET (pg_attver=1394196512);

ALTER TABLE ONLY public.skill_packs FORCE ROW LEVEL SECURITY;


CREATE TABLE public.snapshots (
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    seq bigint NOT NULL,
    r2_key text NOT NULL,
    state_root bytea NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);
ALTER TABLE ONLY public.snapshots ALTER COLUMN tenant_id SET (pg_attver=1394575176);
ALTER TABLE ONLY public.snapshots ALTER COLUMN scope_id SET (pg_attver=1394575176);
ALTER TABLE ONLY public.snapshots ALTER COLUMN seq SET (pg_attver=1394575176);
ALTER TABLE ONLY public.snapshots ALTER COLUMN r2_key SET (pg_attver=1394575176);
ALTER TABLE ONLY public.snapshots ALTER COLUMN state_root SET (pg_attver=1394575176);
ALTER TABLE ONLY public.snapshots ALTER COLUMN created_at SET (pg_attver=1394575176);

ALTER TABLE ONLY public.snapshots FORCE ROW LEVEL SECURITY;


CREATE TABLE public.task_artifacts (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    task_id uuid NOT NULL,
    kind text NOT NULL,
    ref text NOT NULL,
    version integer DEFAULT 0 NOT NULL
);
ALTER TABLE ONLY public.task_artifacts ALTER COLUMN id SET (pg_attver=1394824184);
ALTER TABLE ONLY public.task_artifacts ALTER COLUMN tenant_id SET (pg_attver=1394824184);
ALTER TABLE ONLY public.task_artifacts ALTER COLUMN task_id SET (pg_attver=1394824184);
ALTER TABLE ONLY public.task_artifacts ALTER COLUMN kind SET (pg_attver=1394824184);
ALTER TABLE ONLY public.task_artifacts ALTER COLUMN ref SET (pg_attver=1394824184);
ALTER TABLE ONLY public.task_artifacts ALTER COLUMN version SET (pg_attver=1394824184);

ALTER TABLE ONLY public.task_artifacts FORCE ROW LEVEL SECURITY;


CREATE TABLE public.task_items (
    tenant_id uuid NOT NULL,
    task_id uuid NOT NULL,
    item_key text NOT NULL,
    description text NOT NULL,
    passes boolean DEFAULT false NOT NULL,
    evaluated_by_run uuid
);
ALTER TABLE ONLY public.task_items ALTER COLUMN tenant_id SET (pg_attver=1394764088);
ALTER TABLE ONLY public.task_items ALTER COLUMN task_id SET (pg_attver=1394764088);
ALTER TABLE ONLY public.task_items ALTER COLUMN item_key SET (pg_attver=1394764088);
ALTER TABLE ONLY public.task_items ALTER COLUMN description SET (pg_attver=1394764088);
ALTER TABLE ONLY public.task_items ALTER COLUMN passes SET (pg_attver=1394764088);
ALTER TABLE ONLY public.task_items ALTER COLUMN evaluated_by_run SET (pg_attver=1394764088);

ALTER TABLE ONLY public.task_items FORCE ROW LEVEL SECURITY;


CREATE TABLE public.task_scopes (
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    task_id uuid NOT NULL,
    decision text DEFAULT 'pending'::text NOT NULL,
    decided_by uuid,
    decided_at timestamp with time zone,
    CONSTRAINT task_scopes_decision_check CHECK ((decision = ANY (ARRAY['pending'::text, 'approved'::text, 'rejected'::text])))
);
ALTER TABLE ONLY public.task_scopes ALTER COLUMN tenant_id SET (pg_attver=1394718480);
ALTER TABLE ONLY public.task_scopes ALTER COLUMN scope_id SET (pg_attver=1394718480);
ALTER TABLE ONLY public.task_scopes ALTER COLUMN task_id SET (pg_attver=1394718480);
ALTER TABLE ONLY public.task_scopes ALTER COLUMN decision SET (pg_attver=1394718480);
ALTER TABLE ONLY public.task_scopes ALTER COLUMN decided_by SET (pg_attver=1394718480);
ALTER TABLE ONLY public.task_scopes ALTER COLUMN decided_at SET (pg_attver=1394718480);

ALTER TABLE ONLY public.task_scopes FORCE ROW LEVEL SECURITY;


CREATE TABLE public.tasks (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    title text NOT NULL,
    owner_id uuid NOT NULL,
    state text DEFAULT 'draft'::text NOT NULL,
    version integer DEFAULT 0 NOT NULL,
    budget jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT tasks_state_check CHECK ((state = ANY (ARRAY['draft'::text, 'awaiting_signoff'::text, 'rejected'::text, 'approved'::text, 'running'::text, 'awaiting_action_approval'::text, 'evaluating'::text, 'completed'::text, 'failed'::text])))
);
ALTER TABLE ONLY public.tasks ALTER COLUMN id SET (pg_attver=1394680296);
ALTER TABLE ONLY public.tasks ALTER COLUMN tenant_id SET (pg_attver=1394680296);
ALTER TABLE ONLY public.tasks ALTER COLUMN title SET (pg_attver=1394680296);
ALTER TABLE ONLY public.tasks ALTER COLUMN owner_id SET (pg_attver=1394680296);
ALTER TABLE ONLY public.tasks ALTER COLUMN state SET (pg_attver=1394680296);
ALTER TABLE ONLY public.tasks ALTER COLUMN version SET (pg_attver=1394680296);
ALTER TABLE ONLY public.tasks ALTER COLUMN budget SET (pg_attver=1394680296);
ALTER TABLE ONLY public.tasks ALTER COLUMN created_at SET (pg_attver=1394680296);

ALTER TABLE ONLY public.tasks FORCE ROW LEVEL SECURITY;


CREATE TABLE public.tenant_oauth_clients (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    provider text NOT NULL,
    client_id text NOT NULL,
    secret_ref uuid NOT NULL,
    signing_secret_ref uuid,
    redirect_uri text NOT NULL
);
ALTER TABLE ONLY public.tenant_oauth_clients ALTER COLUMN id SET (pg_attver=1394045488);
ALTER TABLE ONLY public.tenant_oauth_clients ALTER COLUMN tenant_id SET (pg_attver=1394045488);
ALTER TABLE ONLY public.tenant_oauth_clients ALTER COLUMN provider SET (pg_attver=1394045488);
ALTER TABLE ONLY public.tenant_oauth_clients ALTER COLUMN client_id SET (pg_attver=1394045488);
ALTER TABLE ONLY public.tenant_oauth_clients ALTER COLUMN secret_ref SET (pg_attver=1394045488);
ALTER TABLE ONLY public.tenant_oauth_clients ALTER COLUMN signing_secret_ref SET (pg_attver=1394045488);
ALTER TABLE ONLY public.tenant_oauth_clients ALTER COLUMN redirect_uri SET (pg_attver=1394045488);

ALTER TABLE ONLY public.tenant_oauth_clients FORCE ROW LEVEL SECURITY;


CREATE TABLE public.tenants (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);
ALTER TABLE ONLY public.tenants ALTER COLUMN id SET (pg_attver=1393803064);
ALTER TABLE ONLY public.tenants ALTER COLUMN name SET (pg_attver=1393803064);
ALTER TABLE ONLY public.tenants ALTER COLUMN created_at SET (pg_attver=1393803064);

ALTER TABLE ONLY public.tenants FORCE ROW LEVEL SECURITY;


CREATE TABLE public.tool_registry (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    connection_id uuid NOT NULL,
    tool_name text NOT NULL,
    def_hash bytea NOT NULL,
    definition jsonb NOT NULL,
    tier text DEFAULT 'external_write'::text NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    approved_by uuid,
    approved_at timestamp with time zone,
    CONSTRAINT tool_registry_status_check CHECK ((status = ANY (ARRAY['pending'::text, 'approved'::text, 'blocked'::text]))),
    CONSTRAINT tool_registry_tier_check CHECK ((tier = ANY (ARRAY['read'::text, 'sandbox_write'::text, 'external_write'::text])))
);
ALTER TABLE ONLY public.tool_registry ALTER COLUMN id SET (pg_attver=1394154296);
ALTER TABLE ONLY public.tool_registry ALTER COLUMN tenant_id SET (pg_attver=1394154296);
ALTER TABLE ONLY public.tool_registry ALTER COLUMN connection_id SET (pg_attver=1394154296);
ALTER TABLE ONLY public.tool_registry ALTER COLUMN tool_name SET (pg_attver=1394154296);
ALTER TABLE ONLY public.tool_registry ALTER COLUMN def_hash SET (pg_attver=1394154296);
ALTER TABLE ONLY public.tool_registry ALTER COLUMN definition SET (pg_attver=1394154296);
ALTER TABLE ONLY public.tool_registry ALTER COLUMN tier SET (pg_attver=1394154296);
ALTER TABLE ONLY public.tool_registry ALTER COLUMN status SET (pg_attver=1394154296);
ALTER TABLE ONLY public.tool_registry ALTER COLUMN approved_by SET (pg_attver=1394154296);
ALTER TABLE ONLY public.tool_registry ALTER COLUMN approved_at SET (pg_attver=1394154296);

ALTER TABLE ONLY public.tool_registry FORCE ROW LEVEL SECURITY;


CREATE TABLE public.triggers (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    kind text NOT NULL,
    condition jsonb NOT NULL,
    cooldown_seconds integer DEFAULT 0 NOT NULL,
    level text NOT NULL,
    CONSTRAINT triggers_level_check CHECK ((level = ANY (ARRAY['notify'::text, 'question'::text, 'review'::text])))
);
ALTER TABLE ONLY public.triggers ALTER COLUMN id SET (pg_attver=1394955912);
ALTER TABLE ONLY public.triggers ALTER COLUMN tenant_id SET (pg_attver=1394955912);
ALTER TABLE ONLY public.triggers ALTER COLUMN scope_id SET (pg_attver=1394955912);
ALTER TABLE ONLY public.triggers ALTER COLUMN kind SET (pg_attver=1394955912);
ALTER TABLE ONLY public.triggers ALTER COLUMN condition SET (pg_attver=1394955912);
ALTER TABLE ONLY public.triggers ALTER COLUMN cooldown_seconds SET (pg_attver=1394955912);
ALTER TABLE ONLY public.triggers ALTER COLUMN level SET (pg_attver=1394955912);

ALTER TABLE ONLY public.triggers FORCE ROW LEVEL SECURITY;


CREATE TABLE public.usage_meter (
    tenant_id uuid NOT NULL,
    user_id uuid NOT NULL,
    provider text NOT NULL,
    job_kind text NOT NULL,
    day date NOT NULL,
    runs integer DEFAULT 0 NOT NULL,
    seconds bigint DEFAULT 0 NOT NULL,
    tokens_reported bigint
);
ALTER TABLE ONLY public.usage_meter ALTER COLUMN tenant_id SET (pg_attver=1395190312);
ALTER TABLE ONLY public.usage_meter ALTER COLUMN user_id SET (pg_attver=1395190312);
ALTER TABLE ONLY public.usage_meter ALTER COLUMN provider SET (pg_attver=1395190312);
ALTER TABLE ONLY public.usage_meter ALTER COLUMN job_kind SET (pg_attver=1395190312);
ALTER TABLE ONLY public.usage_meter ALTER COLUMN day SET (pg_attver=1395190312);
ALTER TABLE ONLY public.usage_meter ALTER COLUMN runs SET (pg_attver=1395190312);
ALTER TABLE ONLY public.usage_meter ALTER COLUMN seconds SET (pg_attver=1395190312);
ALTER TABLE ONLY public.usage_meter ALTER COLUMN tokens_reported SET (pg_attver=1395190312);

ALTER TABLE ONLY public.usage_meter FORCE ROW LEVEL SECURITY;


CREATE TABLE public.users (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    email text NOT NULL,
    display_name text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);
ALTER TABLE ONLY public.users ALTER COLUMN id SET (pg_attver=1393833088);
ALTER TABLE ONLY public.users ALTER COLUMN email SET (pg_attver=1393833088);
ALTER TABLE ONLY public.users ALTER COLUMN display_name SET (pg_attver=1393833088);
ALTER TABLE ONLY public.users ALTER COLUMN created_at SET (pg_attver=1393833088);

ALTER TABLE ONLY public.users FORCE ROW LEVEL SECURITY;


ALTER TABLE ONLY public.approvals
    ADD CONSTRAINT approvals_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.artifact_leases
    ADD CONSTRAINT artifact_leases_pkey PRIMARY KEY (artifact_id);


ALTER TABLE ONLY public.audit_log
    ADD CONSTRAINT audit_log_pkey PRIMARY KEY (tenant_id, seq);


ALTER TABLE ONLY public.briefs
    ADD CONSTRAINT briefs_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.briefs
    ADD CONSTRAINT briefs_tenant_id_audience_kind_audience_id_key UNIQUE (tenant_id, audience_kind, audience_id);


ALTER TABLE ONLY public.change_scores
    ADD CONSTRAINT change_scores_pkey PRIMARY KEY (scope_id, community_key);


ALTER TABLE ONLY public.community_summaries
    ADD CONSTRAINT community_summaries_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.community_summaries
    ADD CONSTRAINT community_summaries_scope_id_community_key_key UNIQUE (scope_id, community_key);


ALTER TABLE ONLY public.connections
    ADD CONSTRAINT connections_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.decision_log
    ADD CONSTRAINT decision_log_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.deltas
    ADD CONSTRAINT deltas_pkey PRIMARY KEY (scope_id, seq);


ALTER TABLE ONLY public.devices
    ADD CONSTRAINT devices_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.devices
    ADD CONSTRAINT devices_tenant_key UNIQUE (tenant_id, public_key);


ALTER TABLE ONLY public.entities
    ADD CONSTRAINT entities_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.entity_aliases
    ADD CONSTRAINT entity_aliases_pkey PRIMARY KEY (entity_id, alias);


ALTER TABLE ONLY public.eval_runs
    ADD CONSTRAINT eval_runs_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.events
    ADD CONSTRAINT events_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.events
    ADD CONSTRAINT events_tenant_id_source_external_id_key UNIQUE (tenant_id, source, external_id);


ALTER TABLE ONLY public.facts
    ADD CONSTRAINT facts_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.identities
    ADD CONSTRAINT identities_pkey PRIMARY KEY (provider, subject);


ALTER TABLE ONLY public.memberships
    ADD CONSTRAINT memberships_pkey PRIMARY KEY (tenant_id, user_id);


ALTER TABLE ONLY public.merge_proposals
    ADD CONSTRAINT merge_proposals_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.oauth_tokens
    ADD CONSTRAINT oauth_tokens_pkey PRIMARY KEY (connection_id, user_id);


ALTER TABLE ONLY public.plans
    ADD CONSTRAINT plans_pkey PRIMARY KEY (tenant_id);


ALTER TABLE ONLY public.playbook_items
    ADD CONSTRAINT playbook_items_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.progress_log
    ADD CONSTRAINT progress_log_pkey PRIMARY KEY (task_id, seq);


ALTER TABLE ONLY public.proposals
    ADD CONSTRAINT proposals_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.refresh_tokens
    ADD CONSTRAINT refresh_tokens_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.refresh_tokens
    ADD CONSTRAINT refresh_tokens_token_hash_key UNIQUE (token_hash);


ALTER TABLE ONLY public.relay_jobs
    ADD CONSTRAINT relay_jobs_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.replica_cursors
    ADD CONSTRAINT replica_cursors_pkey PRIMARY KEY (device_id, scope_id);


ALTER TABLE ONLY public.run_steps
    ADD CONSTRAINT run_steps_pkey PRIMARY KEY (run_id, seq);


ALTER TABLE ONLY public.runs
    ADD CONSTRAINT runs_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.scope_keys
    ADD CONSTRAINT scope_keys_pkey PRIMARY KEY (scope_id, key_version);


ALTER TABLE ONLY public.scope_members
    ADD CONSTRAINT scope_members_pkey PRIMARY KEY (scope_id, user_id);


ALTER TABLE ONLY public.scopes
    ADD CONSTRAINT scopes_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.scopes
    ADD CONSTRAINT scopes_tenant_id_name_key UNIQUE (tenant_id, name);


ALTER TABLE ONLY public.secrets
    ADD CONSTRAINT secrets_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.skill_packs
    ADD CONSTRAINT skill_packs_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.skill_packs
    ADD CONSTRAINT skill_packs_tenant_id_department_version_key UNIQUE NULLS NOT DISTINCT (tenant_id, department, version);


ALTER TABLE ONLY public.snapshots
    ADD CONSTRAINT snapshots_pkey PRIMARY KEY (scope_id, seq);


ALTER TABLE ONLY public.task_artifacts
    ADD CONSTRAINT task_artifacts_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.task_items
    ADD CONSTRAINT task_items_pkey PRIMARY KEY (task_id, item_key);


ALTER TABLE ONLY public.task_scopes
    ADD CONSTRAINT task_scopes_pkey PRIMARY KEY (task_id, scope_id);


ALTER TABLE ONLY public.tasks
    ADD CONSTRAINT tasks_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.tenant_oauth_clients
    ADD CONSTRAINT tenant_oauth_clients_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.tenant_oauth_clients
    ADD CONSTRAINT tenant_oauth_clients_tenant_id_provider_key UNIQUE (tenant_id, provider);


ALTER TABLE ONLY public.tenants
    ADD CONSTRAINT tenants_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.tool_registry
    ADD CONSTRAINT tool_registry_connection_id_tool_name_key UNIQUE (connection_id, tool_name);


ALTER TABLE ONLY public.tool_registry
    ADD CONSTRAINT tool_registry_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.triggers
    ADD CONSTRAINT triggers_pkey PRIMARY KEY (id);


ALTER TABLE ONLY public.usage_meter
    ADD CONSTRAINT usage_meter_pkey PRIMARY KEY (tenant_id, user_id, provider, job_kind, day);


ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_email_key UNIQUE (email);


ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


CREATE INDEX approvals_tenant_idx ON public.approvals USING btree (tenant_id);


CREATE INDEX artifact_leases_tenant_idx ON public.artifact_leases USING btree (tenant_id);


CREATE INDEX briefs_tenant_idx ON public.briefs USING btree (tenant_id);


CREATE INDEX change_scores_tenant_idx ON public.change_scores USING btree (tenant_id);


CREATE INDEX community_summaries_scope_idx ON public.community_summaries USING btree (tenant_id, scope_id);


CREATE INDEX community_summaries_tenant_idx ON public.community_summaries USING btree (tenant_id);


CREATE INDEX connections_scope_idx ON public.connections USING btree (tenant_id, scope_id);


CREATE INDEX connections_tenant_idx ON public.connections USING btree (tenant_id);


CREATE INDEX decision_log_tenant_idx ON public.decision_log USING btree (tenant_id);


CREATE INDEX deltas_tenant_idx ON public.deltas USING btree (tenant_id);


CREATE INDEX devices_tenant_idx ON public.devices USING btree (tenant_id);


CREATE INDEX entities_scope_idx ON public.entities USING btree (tenant_id, scope_id);


CREATE INDEX entities_tenant_idx ON public.entities USING btree (tenant_id);


CREATE INDEX entities_x0 ON public.entities USING gin (tsv);


CREATE INDEX entity_aliases_scope_idx ON public.entity_aliases USING btree (tenant_id, scope_id);


CREATE INDEX entity_aliases_tenant_idx ON public.entity_aliases USING btree (tenant_id);


CREATE INDEX eval_runs_tenant_idx ON public.eval_runs USING btree (tenant_id);


CREATE INDEX events_scope_idx ON public.events USING btree (tenant_id, scope_id);


CREATE INDEX events_tenant_idx ON public.events USING btree (tenant_id);


CREATE INDEX facts_scope_idx ON public.facts USING btree (tenant_id, scope_id);


CREATE INDEX facts_tenant_idx ON public.facts USING btree (tenant_id);


CREATE INDEX facts_x0 ON public.facts USING gin (tsv);


CREATE INDEX facts_x1 ON public.facts USING btree (subject_id);


CREATE INDEX identities_user_idx ON public.identities USING btree (user_id);


CREATE INDEX merge_proposals_scope_idx ON public.merge_proposals USING btree (tenant_id, scope_id);


CREATE INDEX merge_proposals_tenant_idx ON public.merge_proposals USING btree (tenant_id);


CREATE INDEX oauth_tokens_tenant_idx ON public.oauth_tokens USING btree (tenant_id);


CREATE INDEX playbook_items_scope_idx ON public.playbook_items USING btree (tenant_id, scope_id);


CREATE INDEX playbook_items_tenant_idx ON public.playbook_items USING btree (tenant_id);


CREATE INDEX progress_log_tenant_idx ON public.progress_log USING btree (tenant_id);


CREATE INDEX proposals_scope_idx ON public.proposals USING btree (tenant_id, scope_id);


CREATE INDEX proposals_tenant_idx ON public.proposals USING btree (tenant_id);


CREATE INDEX refresh_tokens_family_idx ON public.refresh_tokens USING btree (family_id);


CREATE INDEX refresh_tokens_tenant_idx ON public.refresh_tokens USING btree (tenant_id);


CREATE INDEX relay_jobs_scope_idx ON public.relay_jobs USING btree (tenant_id, scope_id);


CREATE INDEX relay_jobs_tenant_idx ON public.relay_jobs USING btree (tenant_id);


CREATE INDEX replica_cursors_scope_idx ON public.replica_cursors USING btree (tenant_id, scope_id);


CREATE INDEX replica_cursors_tenant_idx ON public.replica_cursors USING btree (tenant_id);


CREATE INDEX run_steps_tenant_idx ON public.run_steps USING btree (tenant_id);


CREATE INDEX runs_tenant_idx ON public.runs USING btree (tenant_id);


CREATE INDEX scope_keys_tenant_idx ON public.scope_keys USING btree (tenant_id);


CREATE INDEX scope_members_tenant_idx ON public.scope_members USING btree (tenant_id);


CREATE INDEX scopes_tenant_idx ON public.scopes USING btree (tenant_id);


CREATE INDEX secrets_tenant_idx ON public.secrets USING btree (tenant_id);


CREATE INDEX skill_packs_tenant_idx ON public.skill_packs USING btree (tenant_id);


CREATE INDEX snapshots_tenant_idx ON public.snapshots USING btree (tenant_id);


CREATE INDEX task_artifacts_tenant_idx ON public.task_artifacts USING btree (tenant_id);


CREATE INDEX task_items_tenant_idx ON public.task_items USING btree (tenant_id);


CREATE INDEX task_scopes_scope_idx ON public.task_scopes USING btree (tenant_id, scope_id);


CREATE INDEX task_scopes_tenant_idx ON public.task_scopes USING btree (tenant_id);


CREATE INDEX tasks_tenant_idx ON public.tasks USING btree (tenant_id);


CREATE INDEX tenant_oauth_clients_tenant_idx ON public.tenant_oauth_clients USING btree (tenant_id);


CREATE INDEX tool_registry_tenant_idx ON public.tool_registry USING btree (tenant_id);


CREATE INDEX triggers_scope_idx ON public.triggers USING btree (tenant_id, scope_id);


CREATE INDEX triggers_tenant_idx ON public.triggers USING btree (tenant_id);


CREATE TRIGGER audit_log_no_truncate BEFORE TRUNCATE ON public.audit_log FOR EACH STATEMENT EXECUTE FUNCTION public.audit_log_immutable();


CREATE TRIGGER audit_log_no_update BEFORE DELETE OR UPDATE ON public.audit_log FOR EACH ROW EXECUTE FUNCTION public.audit_log_immutable();


ALTER TABLE ONLY public.approvals
    ADD CONSTRAINT approvals_decided_by_fkey FOREIGN KEY (decided_by) REFERENCES public.users(id);


ALTER TABLE ONLY public.approvals
    ADD CONSTRAINT approvals_run_id_fkey FOREIGN KEY (run_id) REFERENCES public.runs(id);


ALTER TABLE ONLY public.approvals
    ADD CONSTRAINT approvals_task_id_fkey FOREIGN KEY (task_id) REFERENCES public.tasks(id);


ALTER TABLE ONLY public.approvals
    ADD CONSTRAINT approvals_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.approvals
    ADD CONSTRAINT approvals_tool_id_fkey FOREIGN KEY (tool_id) REFERENCES public.tool_registry(id);


ALTER TABLE ONLY public.artifact_leases
    ADD CONSTRAINT artifact_leases_artifact_id_fkey FOREIGN KEY (artifact_id) REFERENCES public.task_artifacts(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.artifact_leases
    ADD CONSTRAINT artifact_leases_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.audit_log
    ADD CONSTRAINT audit_log_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.briefs
    ADD CONSTRAINT briefs_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.change_scores
    ADD CONSTRAINT change_scores_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.change_scores
    ADD CONSTRAINT change_scores_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.community_summaries
    ADD CONSTRAINT community_summaries_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.community_summaries
    ADD CONSTRAINT community_summaries_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.connections
    ADD CONSTRAINT connections_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.connections
    ADD CONSTRAINT connections_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.decision_log
    ADD CONSTRAINT decision_log_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.deltas
    ADD CONSTRAINT deltas_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.deltas
    ADD CONSTRAINT deltas_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.devices
    ADD CONSTRAINT devices_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.devices
    ADD CONSTRAINT devices_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.entities
    ADD CONSTRAINT entities_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.entities
    ADD CONSTRAINT entities_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.entity_aliases
    ADD CONSTRAINT entity_aliases_entity_id_fkey FOREIGN KEY (entity_id) REFERENCES public.entities(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.entity_aliases
    ADD CONSTRAINT entity_aliases_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.entity_aliases
    ADD CONSTRAINT entity_aliases_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.eval_runs
    ADD CONSTRAINT eval_runs_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.events
    ADD CONSTRAINT events_connection_id_fkey FOREIGN KEY (connection_id) REFERENCES public.connections(id);


ALTER TABLE ONLY public.events
    ADD CONSTRAINT events_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.events
    ADD CONSTRAINT events_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.facts
    ADD CONSTRAINT facts_object_id_fkey FOREIGN KEY (object_id) REFERENCES public.entities(id);


ALTER TABLE ONLY public.facts
    ADD CONSTRAINT facts_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.facts
    ADD CONSTRAINT facts_source_event_id_fkey FOREIGN KEY (source_event_id) REFERENCES public.events(id);


ALTER TABLE ONLY public.facts
    ADD CONSTRAINT facts_subject_id_fkey FOREIGN KEY (subject_id) REFERENCES public.entities(id);


ALTER TABLE ONLY public.facts
    ADD CONSTRAINT facts_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.identities
    ADD CONSTRAINT identities_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.memberships
    ADD CONSTRAINT memberships_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.memberships
    ADD CONSTRAINT memberships_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.merge_proposals
    ADD CONSTRAINT merge_proposals_decided_by_fkey FOREIGN KEY (decided_by) REFERENCES public.users(id);


ALTER TABLE ONLY public.merge_proposals
    ADD CONSTRAINT merge_proposals_entity_a_fkey FOREIGN KEY (entity_a) REFERENCES public.entities(id);


ALTER TABLE ONLY public.merge_proposals
    ADD CONSTRAINT merge_proposals_entity_b_fkey FOREIGN KEY (entity_b) REFERENCES public.entities(id);


ALTER TABLE ONLY public.merge_proposals
    ADD CONSTRAINT merge_proposals_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.merge_proposals
    ADD CONSTRAINT merge_proposals_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.oauth_tokens
    ADD CONSTRAINT oauth_tokens_connection_id_fkey FOREIGN KEY (connection_id) REFERENCES public.connections(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.oauth_tokens
    ADD CONSTRAINT oauth_tokens_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.oauth_tokens
    ADD CONSTRAINT oauth_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.plans
    ADD CONSTRAINT plans_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.playbook_items
    ADD CONSTRAINT playbook_items_approved_by_fkey FOREIGN KEY (approved_by) REFERENCES public.users(id);


ALTER TABLE ONLY public.playbook_items
    ADD CONSTRAINT playbook_items_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.playbook_items
    ADD CONSTRAINT playbook_items_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.progress_log
    ADD CONSTRAINT progress_log_task_id_fkey FOREIGN KEY (task_id) REFERENCES public.tasks(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.progress_log
    ADD CONSTRAINT progress_log_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.proposals
    ADD CONSTRAINT proposals_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.proposals
    ADD CONSTRAINT proposals_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.proposals
    ADD CONSTRAINT proposals_trigger_id_fkey FOREIGN KEY (trigger_id) REFERENCES public.triggers(id);


ALTER TABLE ONLY public.refresh_tokens
    ADD CONSTRAINT refresh_tokens_device_id_fkey FOREIGN KEY (device_id) REFERENCES public.devices(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.refresh_tokens
    ADD CONSTRAINT refresh_tokens_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.refresh_tokens
    ADD CONSTRAINT refresh_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.relay_jobs
    ADD CONSTRAINT relay_jobs_claimed_by_device_fkey FOREIGN KEY (claimed_by_device) REFERENCES public.devices(id);


ALTER TABLE ONLY public.relay_jobs
    ADD CONSTRAINT relay_jobs_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.relay_jobs
    ADD CONSTRAINT relay_jobs_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.replica_cursors
    ADD CONSTRAINT replica_cursors_device_id_fkey FOREIGN KEY (device_id) REFERENCES public.devices(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.replica_cursors
    ADD CONSTRAINT replica_cursors_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.replica_cursors
    ADD CONSTRAINT replica_cursors_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.run_steps
    ADD CONSTRAINT run_steps_run_id_fkey FOREIGN KEY (run_id) REFERENCES public.runs(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.run_steps
    ADD CONSTRAINT run_steps_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.runs
    ADD CONSTRAINT runs_task_id_fkey FOREIGN KEY (task_id) REFERENCES public.tasks(id);


ALTER TABLE ONLY public.runs
    ADD CONSTRAINT runs_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.scope_keys
    ADD CONSTRAINT scope_keys_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.scope_keys
    ADD CONSTRAINT scope_keys_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.scope_members
    ADD CONSTRAINT scope_members_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.scope_members
    ADD CONSTRAINT scope_members_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.scope_members
    ADD CONSTRAINT scope_members_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.scopes
    ADD CONSTRAINT scopes_fold_id_fkey FOREIGN KEY (fold_id) REFERENCES public.scopes(id);


ALTER TABLE ONLY public.scopes
    ADD CONSTRAINT scopes_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.secrets
    ADD CONSTRAINT secrets_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.skill_packs
    ADD CONSTRAINT skill_packs_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.snapshots
    ADD CONSTRAINT snapshots_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.snapshots
    ADD CONSTRAINT snapshots_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.task_artifacts
    ADD CONSTRAINT task_artifacts_task_id_fkey FOREIGN KEY (task_id) REFERENCES public.tasks(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.task_artifacts
    ADD CONSTRAINT task_artifacts_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.task_items
    ADD CONSTRAINT task_items_task_id_fkey FOREIGN KEY (task_id) REFERENCES public.tasks(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.task_items
    ADD CONSTRAINT task_items_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.task_scopes
    ADD CONSTRAINT task_scopes_decided_by_fkey FOREIGN KEY (decided_by) REFERENCES public.users(id);


ALTER TABLE ONLY public.task_scopes
    ADD CONSTRAINT task_scopes_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.task_scopes
    ADD CONSTRAINT task_scopes_task_id_fkey FOREIGN KEY (task_id) REFERENCES public.tasks(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.task_scopes
    ADD CONSTRAINT task_scopes_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.tasks
    ADD CONSTRAINT tasks_owner_id_fkey FOREIGN KEY (owner_id) REFERENCES public.users(id);


ALTER TABLE ONLY public.tasks
    ADD CONSTRAINT tasks_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.tenant_oauth_clients
    ADD CONSTRAINT tenant_oauth_clients_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.tool_registry
    ADD CONSTRAINT tool_registry_approved_by_fkey FOREIGN KEY (approved_by) REFERENCES public.users(id);


ALTER TABLE ONLY public.tool_registry
    ADD CONSTRAINT tool_registry_connection_id_fkey FOREIGN KEY (connection_id) REFERENCES public.connections(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.tool_registry
    ADD CONSTRAINT tool_registry_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.triggers
    ADD CONSTRAINT triggers_scope_id_fkey FOREIGN KEY (scope_id) REFERENCES public.scopes(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.triggers
    ADD CONSTRAINT triggers_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.usage_meter
    ADD CONSTRAINT usage_meter_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;


ALTER TABLE ONLY public.usage_meter
    ADD CONSTRAINT usage_meter_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


ALTER TABLE public.approvals ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.artifact_leases ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.audit_log ENABLE ROW LEVEL SECURITY;

CREATE POLICY auth_all ON public.identities TO app_auth USING (true) WITH CHECK (true);


CREATE POLICY auth_all ON public.refresh_tokens TO app_auth USING (true) WITH CHECK (true);


CREATE POLICY auth_insert ON public.devices FOR INSERT TO app_auth WITH CHECK (true);


CREATE POLICY auth_insert ON public.users FOR INSERT TO app_auth WITH CHECK (true);


CREATE POLICY auth_select ON public.devices FOR SELECT TO app_auth USING (true);


CREATE POLICY auth_select ON public.memberships FOR SELECT TO app_auth USING (true);


CREATE POLICY auth_select ON public.tenants FOR SELECT TO app_auth USING (true);


CREATE POLICY auth_select ON public.users FOR SELECT TO app_auth USING (true);


CREATE POLICY auth_update ON public.devices FOR UPDATE TO app_auth USING (true) WITH CHECK (true);


ALTER TABLE public.briefs ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.change_scores ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.community_summaries ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.connections ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.decision_log ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.deltas ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.devices ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.entities ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.entity_aliases ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.eval_runs ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.events ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.facts ENABLE ROW LEVEL SECURITY;

CREATE POLICY global_read ON public.eval_runs FOR SELECT USING (((tenant_id IS NULL) OR (tenant_id = public.app_tenant())));


CREATE POLICY global_read ON public.skill_packs FOR SELECT USING (((tenant_id IS NULL) OR (tenant_id = public.app_tenant())));


ALTER TABLE public.identities ENABLE ROW LEVEL SECURITY;

CREATE POLICY isolation ON public.approvals USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.artifact_leases USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.audit_log USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.briefs USING (((tenant_id = public.app_tenant()) AND (scope_ids <@ public.app_scope_ids()))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_ids <@ public.app_scope_ids())));


CREATE POLICY isolation ON public.change_scores USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.community_summaries USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.connections USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.decision_log USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.deltas USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.devices USING (((tenant_id = public.app_tenant()) AND (user_id = public.app_user()))) WITH CHECK (((tenant_id = public.app_tenant()) AND (user_id = public.app_user())));


CREATE POLICY isolation ON public.entities USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.entity_aliases USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.events USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.facts USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.memberships USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.merge_proposals USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.oauth_tokens USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.plans USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.playbook_items USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.progress_log USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.proposals USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.refresh_tokens FOR SELECT USING (((tenant_id = public.app_tenant()) AND (user_id = public.app_user())));


CREATE POLICY isolation ON public.relay_jobs USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.replica_cursors USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.run_steps USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.runs USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.scope_keys USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.scope_members USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.scopes USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.secrets USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.snapshots USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.task_artifacts USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.task_items USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.task_scopes USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.tasks USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.tenant_oauth_clients USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.tenants USING ((id = public.app_tenant())) WITH CHECK ((id = public.app_tenant()));


CREATE POLICY isolation ON public.tool_registry USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY isolation ON public.triggers USING (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids())))) WITH CHECK (((tenant_id = public.app_tenant()) AND (scope_id = ANY (public.app_scope_ids()))));


CREATE POLICY isolation ON public.usage_meter USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


ALTER TABLE public.memberships ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.merge_proposals ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.oauth_tokens ENABLE ROW LEVEL SECURITY;

CREATE POLICY own_identities ON public.identities FOR SELECT USING ((user_id = public.app_user()));


ALTER TABLE public.plans ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.playbook_items ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.progress_log ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.proposals ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.refresh_tokens ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.relay_jobs ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.replica_cursors ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.run_steps ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.runs ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.scope_keys ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.scope_members ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.scopes ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.secrets ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.skill_packs ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.snapshots ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.task_artifacts ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.task_items ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.task_scopes ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.tasks ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.tenant_oauth_clients ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_write ON public.eval_runs USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


CREATE POLICY tenant_write ON public.skill_packs USING ((tenant_id = public.app_tenant())) WITH CHECK ((tenant_id = public.app_tenant()));


ALTER TABLE public.tenants ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.tool_registry ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.triggers ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.usage_meter ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.users ENABLE ROW LEVEL SECURITY;

CREATE POLICY users_self_write ON public.users FOR UPDATE USING ((id = public.app_user())) WITH CHECK ((id = public.app_user()));


CREATE POLICY users_visible ON public.users FOR SELECT USING (((id = public.app_user()) OR (EXISTS ( SELECT 1
   FROM public.memberships m
  WHERE ((m.user_id = users.id) AND (m.tenant_id = public.app_tenant()))))));
