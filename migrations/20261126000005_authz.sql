-- P1-02: what authz reads to build a RequestCtx, and the sealed-scope rule enforced in the database.

-- Finance, legal and HR are sealed: they can never join a fold, and a fold itself is never sealed.
ALTER TABLE scopes ADD CONSTRAINT scopes_sealed_never_in_fold CHECK (NOT (sealed AND fold_id IS NOT NULL));
ALTER TABLE scopes ADD CONSTRAINT scopes_fold_never_sealed CHECK (NOT (kind = 'fold' AND sealed));
ALTER TABLE scopes ADD CONSTRAINT scopes_fold_points_at_fold_only CHECK (kind = 'department' OR fold_id IS NULL);

-- app_auth reads the membership facts it needs to build a context, nothing else.
CREATE POLICY auth_select ON scopes FOR SELECT TO app_auth USING (true);
CREATE POLICY auth_select ON scope_members FOR SELECT TO app_auth USING (true);
CREATE POLICY auth_select ON plans FOR SELECT TO app_auth USING (true);
GRANT SELECT ON scopes, scope_members, plans TO app_auth;
