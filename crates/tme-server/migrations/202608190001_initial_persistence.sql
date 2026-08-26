-- The Mortal Estate founding persistence schema.
--
-- D4: this database holds exactly one world. Players never select among
-- divergent copies of it, so there is no copy family, no per-copy arrival gate,
-- and no world column on a character. Separate world instances — tests,
-- development, private staging, disaster recovery, and any future transparent
-- scaling — are separate databases served by separate processes, which is why
-- tme.facets carries a singleton index rather than a copy directory.

CREATE SCHEMA tme;

CREATE TABLE tme.accounts (
    account_id uuid PRIMARY KEY CHECK (account_id <> '00000000-0000-0000-0000-000000000000'),
    username text NOT NULL UNIQUE CHECK (username ~ '^[a-z0-9](?:[a-z0-9_]{1,30}[a-z0-9])?$'),
    display_name text NOT NULL CHECK (char_length(display_name) BETWEEN 1 AND 64),
    status text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'disabled')),
    created_at timestamptz NOT NULL DEFAULT statement_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT statement_timestamp(),
    CHECK (updated_at >= created_at)
);

CREATE TABLE tme.account_credentials (
    account_id uuid PRIMARY KEY REFERENCES tme.accounts(account_id) ON DELETE CASCADE,
    password_phc text NOT NULL CHECK (length(password_phc) BETWEEN 1 AND 1024),
    credential_updated_at timestamptz NOT NULL DEFAULT statement_timestamp()
);

CREATE TABLE tme.facets (
    facet_id uuid PRIMARY KEY CHECK (facet_id <> '00000000-0000-0000-0000-000000000000'),
    facet_key text NOT NULL UNIQUE CHECK (length(facet_key) BETWEEN 1 AND 64 AND facet_key ~ '^[ -~]+$'),
    catalog_id text NOT NULL CHECK (length(catalog_id) BETWEEN 1 AND 128),
    profile_id text NOT NULL CHECK (length(profile_id) BETWEEN 1 AND 128),
    template_id text NOT NULL CHECK (length(template_id) BETWEEN 1 AND 128),
    content_digest bytea NOT NULL CHECK (octet_length(content_digest) = 32),
    checkpoint_schema smallint NOT NULL CHECK (checkpoint_schema = 3),
    facet_revision bigint NOT NULL DEFAULT 0 CHECK (facet_revision >= 0),
    last_server_sequence bigint NOT NULL DEFAULT 0 CHECK (last_server_sequence >= 0),
    checkpoint_bytes bytea NOT NULL CHECK (octet_length(checkpoint_bytes) BETWEEN 1 AND 67108864),
    checkpoint_sha256 bytea NOT NULL CHECK (octet_length(checkpoint_sha256) = 32),
    updated_at timestamptz NOT NULL DEFAULT statement_timestamp()
);

-- D4 enforced in storage: a second world row cannot be created, so no code path
-- can reintroduce player-selectable divergent histories by accident.
CREATE UNIQUE INDEX facets_singleton_idx ON tme.facets ((true));

CREATE TABLE tme.characters (
    character_id uuid PRIMARY KEY CHECK (character_id <> '00000000-0000-0000-0000-000000000000'),
    account_id uuid NOT NULL REFERENCES tme.accounts(account_id) ON DELETE CASCADE,
    slot smallint NOT NULL CHECK (slot BETWEEN 1 AND 8),
    display_name text NOT NULL CHECK (char_length(display_name) BETWEEN 1 AND 64),
    actor_id text NOT NULL UNIQUE CHECK (length(actor_id) BETWEEN 1 AND 128),
    control_epoch bigint NOT NULL DEFAULT 0 CHECK (control_epoch >= 0),
    UNIQUE (account_id, slot),
    UNIQUE (account_id, display_name),
    UNIQUE (account_id, character_id)
);

CREATE TABLE tme.sessions (
    session_id uuid PRIMARY KEY CHECK (session_id <> '00000000-0000-0000-0000-000000000000'),
    account_id uuid NOT NULL REFERENCES tme.accounts(account_id) ON DELETE CASCADE,
    token_digest bytea NOT NULL UNIQUE CHECK (octet_length(token_digest) = 32),
    csrf_digest bytea NOT NULL CHECK (octet_length(csrf_digest) = 32),
    selected_character_id uuid,
    created_at timestamptz NOT NULL DEFAULT statement_timestamp(),
    last_seen_at timestamptz NOT NULL DEFAULT statement_timestamp(),
    idle_expires_at timestamptz NOT NULL,
    absolute_expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    FOREIGN KEY (account_id, selected_character_id)
        REFERENCES tme.characters(account_id, character_id),
    UNIQUE (session_id, account_id),
    CHECK (last_seen_at >= created_at),
    CHECK (idle_expires_at > created_at),
    CHECK (absolute_expires_at > created_at),
    CHECK (revoked_at IS NULL OR revoked_at >= created_at)
);

CREATE TABLE tme.socket_tickets (
    ticket_digest bytea PRIMARY KEY CHECK (octet_length(ticket_digest) = 32),
    session_id uuid NOT NULL,
    account_id uuid NOT NULL,
    character_id uuid NOT NULL,
    actor_id text NOT NULL CHECK (length(actor_id) BETWEEN 1 AND 128),
    expected_control_epoch bigint NOT NULL CHECK (expected_control_epoch >= 0),
    origin text NOT NULL CHECK (length(origin) BETWEEN 1 AND 512),
    host text NOT NULL CHECK (length(host) BETWEEN 1 AND 255),
    selected_major smallint NOT NULL CHECK (selected_major = 1),
    created_at timestamptz NOT NULL DEFAULT statement_timestamp(),
    expires_at timestamptz NOT NULL,
    consumed_at timestamptz,
    FOREIGN KEY (session_id, account_id) REFERENCES tme.sessions(session_id, account_id) ON DELETE CASCADE,
    FOREIGN KEY (account_id, character_id) REFERENCES tme.characters(account_id, character_id) ON DELETE CASCADE,
    CHECK (expires_at > created_at),
    CHECK (consumed_at IS NULL OR consumed_at >= created_at)
);

CREATE TABLE tme.command_receipts (
    account_id uuid NOT NULL REFERENCES tme.accounts(account_id) ON DELETE CASCADE,
    command_id uuid NOT NULL CHECK (command_id <> '00000000-0000-0000-0000-000000000000'),
    request_digest bytea NOT NULL CHECK (octet_length(request_digest) = 32),
    session_id uuid,
    actor_id text,
    control_epoch bigint CHECK (control_epoch >= 0),
    client_sequence bigint CHECK (client_sequence >= 0),
    server_sequence bigint CHECK (server_sequence >= 0),
    before_revision bigint CHECK (before_revision >= 0),
    after_revision bigint CHECK (after_revision >= 0),
    outcome_schema smallint NOT NULL CHECK (outcome_schema = 3),
    disposition text NOT NULL CHECK (disposition IN ('accepted', 'rejected', 'expired')),
    outcome_bytes bytea,
    created_at timestamptz NOT NULL DEFAULT statement_timestamp(),
    full_expires_at timestamptz NOT NULL,
    PRIMARY KEY (account_id, command_id),
    FOREIGN KEY (session_id, account_id) REFERENCES tme.sessions(session_id, account_id),
    CHECK (full_expires_at > created_at),
    CHECK ((disposition = 'expired' AND outcome_bytes IS NULL) OR
           (disposition <> 'expired' AND outcome_bytes IS NOT NULL))
);

CREATE TABLE tme.audit_events (
    audit_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    account_id uuid REFERENCES tme.accounts(account_id) ON DELETE SET NULL,
    session_id uuid REFERENCES tme.sessions(session_id) ON DELETE SET NULL,
    character_id uuid REFERENCES tme.characters(character_id) ON DELETE SET NULL,
    command_id uuid,
    actor text NOT NULL CHECK (actor IN ('runtime', 'operator', 'recovery')),
    action text NOT NULL CHECK (
        action IN (
            'account_create', 'account_set_password', 'login', 'logout',
            'admit', 'command', 'restore_fence', 'facet_tick',
            'facet_presence', 'mark_assess', 'mark_expire', 'mark_forgive',
            'session_expire'
        )
    ),
    result text NOT NULL CHECK (result IN ('success', 'rejected', 'failed')),
    correlation_id uuid NOT NULL CHECK (correlation_id <> '00000000-0000-0000-0000-000000000000'),
    occurred_at timestamptz NOT NULL DEFAULT statement_timestamp()
);

CREATE TABLE tme.store_state (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    cluster_system_identifier text NOT NULL CHECK (cluster_system_identifier ~ '^[0-9]+$'),
    database_oid text NOT NULL CHECK (database_oid ~ '^[0-9]+$'),
    last_restore_fenced_at timestamptz,
    restore_fence_epoch bigint NOT NULL DEFAULT 0 CHECK (restore_fence_epoch >= 0)
);

INSERT INTO tme.store_state (singleton, cluster_system_identifier, database_oid)
SELECT true, system_identifier::text,
       (SELECT oid::text FROM pg_database WHERE datname = current_database())
FROM pg_control_system();

CREATE FUNCTION tme.mark_now() RETURNS timestamptz
LANGUAGE sql STABLE
SET search_path = pg_catalog
AS $$
    SELECT COALESCE(
        NULLIF(current_setting('tme.test_now', true), '')::timestamptz,
        statement_timestamp()
    )
$$;

REVOKE ALL ON FUNCTION tme.mark_now() FROM PUBLIC;

CREATE TABLE tme.player_kill_marks (
    mark_id uuid PRIMARY KEY
        CHECK (mark_id <> '00000000-0000-0000-0000-000000000000'),
    facet_kill_sequence bigint NOT NULL CHECK (facet_kill_sequence > 0),
    killer_account_id uuid NOT NULL,
    killer_character_id uuid NOT NULL,
    victim_account_id uuid NOT NULL,
    victim_character_id uuid NOT NULL,
    killer_session_id uuid,
    victim_session_id uuid NOT NULL,
    assessed_at timestamptz NOT NULL,
    assessed_logical_time numeric(20, 0) NOT NULL
        CHECK (assessed_logical_time BETWEEN 0 AND 18446744073709551615),
    linked_karma_added boolean NOT NULL,
    karma_forgiveness_eligible boolean NOT NULL,
    expires_at timestamptz,
    forgiven_at timestamptz,
    forgiven_by_account_id uuid,
    expired_at timestamptz,
    FOREIGN KEY (killer_account_id, killer_character_id)
        REFERENCES tme.characters(account_id, character_id),
    FOREIGN KEY (victim_account_id, victim_character_id)
        REFERENCES tme.characters(account_id, character_id),
    FOREIGN KEY (killer_session_id, killer_account_id)
        REFERENCES tme.sessions(session_id, account_id),
    FOREIGN KEY (victim_session_id, victim_account_id)
        REFERENCES tme.sessions(session_id, account_id),
    FOREIGN KEY (forgiven_by_account_id)
        REFERENCES tme.accounts(account_id),
    CHECK ((forgiven_at IS NULL) = (forgiven_by_account_id IS NULL)),
    CHECK (forgiven_by_account_id IS NULL OR
           forgiven_by_account_id = victim_account_id),
    CHECK (linked_karma_added OR NOT karma_forgiveness_eligible),
    CHECK (NOT (forgiven_at IS NOT NULL AND expired_at IS NOT NULL)),
    UNIQUE (facet_kill_sequence)
);

-- Owner ruling 2026-08-20 (successor issue #3): logging off is not a karma
-- escape. When a delayed hostile effect kills and the credited killer's sheet
-- is not loaded, the karma/alignment consequence is recorded here in the same
-- transaction as the mark, and applied to the sheet at the killer's next
-- admission. Rows are deleted in the same transaction that makes the applied
-- sheet durable, which is what makes application exactly-once across a crash.
CREATE TABLE tme.pending_player_kill_consequences (
    facet_kill_sequence bigint PRIMARY KEY
        REFERENCES tme.player_kill_marks(facet_kill_sequence) ON DELETE CASCADE,
    killer_account_id uuid NOT NULL,
    killer_character_id uuid NOT NULL,
    victim_character_id uuid NOT NULL,
    victim_alignment text NOT NULL
        CHECK (victim_alignment IN ('lawful', 'neutral', 'chaotic', 'evil')),
    victim_nature text NOT NULL
        CHECK (victim_nature IN ('human', 'animal', 'other')),
    assessed_logical_time numeric(20, 0) NOT NULL
        CHECK (assessed_logical_time BETWEEN 0 AND 18446744073709551615),
    recorded_at timestamptz NOT NULL DEFAULT statement_timestamp(),
    FOREIGN KEY (killer_account_id, killer_character_id)
        REFERENCES tme.characters(account_id, character_id) ON DELETE CASCADE
);

CREATE INDEX pending_player_kill_consequences_killer_idx
    ON tme.pending_player_kill_consequences
       (killer_account_id, killer_character_id, facet_kill_sequence);

CREATE INDEX player_kill_marks_killer_active_idx
    ON tme.player_kill_marks (killer_account_id, assessed_at, mark_id)
    WHERE forgiven_at IS NULL AND expired_at IS NULL;

CREATE INDEX player_kill_marks_victim_active_idx
    ON tme.player_kill_marks (victim_account_id, assessed_at, mark_id)
    WHERE forgiven_at IS NULL AND expired_at IS NULL;

CREATE INDEX sessions_active_account_idx ON tme.sessions (account_id, absolute_expires_at)
    WHERE revoked_at IS NULL;
CREATE INDEX tickets_session_idx ON tme.socket_tickets (session_id, expires_at);
CREATE INDEX receipts_full_expiry_idx ON tme.command_receipts (full_expires_at)
    WHERE outcome_bytes IS NOT NULL;
