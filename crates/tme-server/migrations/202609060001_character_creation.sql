-- Account-scoped creation receipts make retries safe after a lost HTTP response.
CREATE TABLE tme.character_creations (
    account_id uuid NOT NULL,
    request_id uuid NOT NULL CHECK (request_id <> '00000000-0000-0000-0000-000000000000'),
    request_sha256 bytea NOT NULL CHECK (octet_length(request_sha256) = 32),
    character_id uuid NOT NULL UNIQUE,
    PRIMARY KEY (account_id, request_id),
    FOREIGN KEY (account_id, character_id)
        REFERENCES tme.characters(account_id, character_id) ON DELETE CASCADE
);
