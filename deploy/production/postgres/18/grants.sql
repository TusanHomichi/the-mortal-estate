\set ON_ERROR_STOP on

DO $grants$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'tme') THEN
        REVOKE ALL ON SCHEMA tme FROM PUBLIC;
        GRANT USAGE ON SCHEMA tme TO tme_runtime, tme_auth, tme_monitor;

        GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA tme TO tme_runtime;
        REVOKE ALL ON TABLE tme.account_credentials FROM tme_runtime;
        GRANT SELECT ON tme.accounts TO tme_auth;
        GRANT SELECT, UPDATE ON tme.account_credentials TO tme_auth;
        GRANT SELECT, INSERT ON tme.sessions TO tme_auth;
        GRANT INSERT ON tme.audit_events TO tme_auth;
        GRANT SELECT ON tme.store_state TO tme_monitor;
        GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA tme TO tme_runtime, tme_auth;
        IF to_regprocedure('tme.mark_now()') IS NOT NULL THEN
            GRANT EXECUTE ON FUNCTION tme.mark_now() TO tme_runtime;
        END IF;
        IF to_regclass('public._sqlx_migrations') IS NOT NULL THEN
            GRANT SELECT ON TABLE public._sqlx_migrations TO tme_runtime;
        END IF;
    END IF;
END
$grants$;

ALTER DEFAULT PRIVILEGES FOR ROLE tme_owner IN SCHEMA tme REVOKE ALL ON TABLES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES FOR ROLE tme_owner IN SCHEMA tme REVOKE ALL ON SEQUENCES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES FOR ROLE tme_owner IN SCHEMA tme GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO tme_runtime;
ALTER DEFAULT PRIVILEGES FOR ROLE tme_owner IN SCHEMA tme GRANT USAGE, SELECT ON SEQUENCES TO tme_runtime;
