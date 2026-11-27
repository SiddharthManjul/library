-- Cluster-level roles. Idempotent. Run before migrations (dev and tests); in production the deploy script does this.
-- library_owner: owns tables, runs migrations.  app_rw: runtime, not owner, no BYPASSRLS.
-- app_sequencer: the only role that writes state tables.  app_vault: the only role that reads secrets.
-- app_auth: used only by the sign-in and session code, which runs before any tenant context exists.
DO $$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'library_owner') THEN
    CREATE ROLE library_owner LOGIN PASSWORD 'owner_dev_pw' CREATEDB;
  END IF;
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_rw') THEN
    CREATE ROLE app_rw LOGIN PASSWORD 'app_dev_pw' NOSUPERUSER NOBYPASSRLS;
  END IF;
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_sequencer') THEN
    CREATE ROLE app_sequencer LOGIN PASSWORD 'sequencer_dev_pw' NOSUPERUSER NOBYPASSRLS;
  END IF;
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_auth') THEN
    CREATE ROLE app_auth LOGIN PASSWORD 'auth_dev_pw' NOSUPERUSER NOBYPASSRLS;
  END IF;
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_vault') THEN
    CREATE ROLE app_vault LOGIN PASSWORD 'vault_dev_pw' NOSUPERUSER NOBYPASSRLS;
  END IF;
END
$$;
