-- Library IDE schema, part 1 of 2: session-variable helpers used by every row-level security policy.
-- authz sets app.tenant_id, app.user_id and app.scope_ids (comma separated uuids) with SET LOCAL per transaction.
-- Unset or empty means no tenant, no user and no scopes, so policies return no rows.
CREATE FUNCTION app_tenant() RETURNS uuid LANGUAGE sql STABLE
  AS $$ SELECT nullif(current_setting('app.tenant_id', true), '')::uuid $$;
CREATE FUNCTION app_user() RETURNS uuid LANGUAGE sql STABLE
  AS $$ SELECT nullif(current_setting('app.user_id', true), '')::uuid $$;
CREATE FUNCTION app_scope_ids() RETURNS uuid[] LANGUAGE sql STABLE
  AS $$ SELECT coalesce(string_to_array(nullif(current_setting('app.scope_ids', true), ''), ',')::uuid[], '{}'::uuid[]) $$;
