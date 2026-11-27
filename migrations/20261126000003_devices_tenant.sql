-- A device is registered into one tenant (the same keys may register into several).
-- Every table except users (global identity, members of many tenants) now carries tenant_id.
ALTER TABLE devices ADD COLUMN tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE;
ALTER TABLE devices ADD CONSTRAINT devices_tenant_key UNIQUE (tenant_id, public_key);
CREATE INDEX devices_tenant_idx ON devices (tenant_id);
DROP POLICY user_isolation ON devices;
CREATE POLICY isolation ON devices
  USING (tenant_id = app_tenant() AND user_id = app_user())
  WITH CHECK (tenant_id = app_tenant() AND user_id = app_user());
