// Private consequences proof for the persistence integration target.
async fn commit_kill(
    store: &PostgresStore,
    pool: &sqlx::PgPool,
    world_id: wire::FacetId,
    assessment: &tme_rules::PlayerKillAssessmentV1,
) -> Result<(), String> {
    let row =
        sqlx::query("SELECT facet_revision,last_server_sequence,checkpoint_bytes FROM tme.facets")
            .fetch_one(pool)
            .await
            .unwrap();
    let revision: i64 = row.get("facet_revision");
    let sequence: i64 = row.get("last_server_sequence");
    let bytes: Vec<u8> = row.get("checkpoint_bytes");
    let checkpoint = tme_rules::FacetCheckpointV5::from_bytes(bytes).unwrap();
    store
        .commit_system(tme_server::store::SystemCommit {
            facet_id: world_id,
            expected_server_sequence: u64::try_from(sequence).unwrap(),
            expected_revision: u64::try_from(revision).unwrap(),
            next_server_sequence: u64::try_from(sequence).unwrap() + 1,
            next_revision: u64::try_from(revision).unwrap() + 1,
            checkpoint: &checkpoint,
            action: "facet_deadlines",
            durable_effects: std::slice::from_ref(
                &tme_rules::DurableGameplayEffectV1::PlayerKillAssessed(assessment.clone()),
            ),
        })
        .await
}

async fn schedule_marks(pool: &sqlx::PgPool) {
    sqlx::raw_sql(
        "WITH ordered AS ( \
             SELECT mark_id, \
                    row_number() OVER (PARTITION BY killer_account_id \
                                       ORDER BY assessed_at, mark_id) AS position, \
                    count(*) OVER (PARTITION BY killer_account_id) AS total \
             FROM tme.player_kill_marks \
             WHERE forgiven_at IS NULL AND expired_at IS NULL) \
         UPDATE tme.player_kill_marks m \
         SET expires_at = CASE WHEN o.total >= 4 THEN NULL ELSE \
             statement_timestamp() + \
             ((((o.total - o.position) + 1) * 2) * interval '1 week') END \
         FROM ordered o WHERE m.mark_id = o.mark_id",
    )
    .execute(pool)
    .await
    .unwrap();
    PostgresStore::new(pool.clone())
        .verify_player_kill_marks()
        .await
        .expect("synthetic marks form a valid schedule");
}

async fn forgivable(pool: &sqlx::PgPool, sequence: i64) -> bool {
    sqlx::query_scalar(
        "SELECT linked_karma_added AND karma_forgiveness_eligible \
         FROM tme.player_kill_marks WHERE facet_kill_sequence=$1",
    )
    .bind(sequence)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn truncate_all(pool: &sqlx::PgPool) {
    sqlx::raw_sql(
        "TRUNCATE tme.audit_events,tme.command_receipts,tme.socket_tickets,\
         tme.sessions,tme.characters,tme.facets,tme.account_credentials,\
         tme.accounts RESTART IDENTITY CASCADE",
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn open_sessions(
    pool: &sqlx::PgPool,
    killer_account: wire::AccountId,
    killer_character: wire::CharacterId,
    victim_account: wire::AccountId,
    victim_character: wire::CharacterId,
) -> (Uuid, Uuid) {
    let mut sessions = Vec::new();
    for (account, character) in [
        (killer_account, killer_character),
        (victim_account, victim_character),
    ] {
        let session_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO tme.sessions \
             (session_id,account_id,token_digest,csrf_digest,selected_character_id, \
              idle_expires_at,absolute_expires_at) \
             VALUES ($1,$2,$3,$4,$5,statement_timestamp()+interval '1 day', \
                     statement_timestamp()+interval '2 days')",
        )
        .bind(session_id)
        .bind(account.as_uuid())
        .bind(Sha256::digest(session_id.as_bytes()).to_vec())
        .bind(Sha256::digest(account.as_uuid().as_bytes()).to_vec())
        .bind(character.as_uuid())
        .execute(pool)
        .await
        .unwrap();
        sessions.push(session_id);
    }
    (sessions[0], sessions[1])
}

async fn insert_pending_consequence(
    pool: &sqlx::PgPool,
    sequence: i64,
    killer_account: wire::AccountId,
    killer_character: wire::CharacterId,
    victim_character: wire::CharacterId,
) {
    sqlx::query(
        "INSERT INTO tme.pending_player_kill_consequences \
         (facet_kill_sequence,killer_account_id,killer_character_id,victim_character_id, \
          victim_alignment,victim_nature,assessed_logical_millis) \
         VALUES ($1,$2,$3,$4,'lawful','human',1)",
    )
    .bind(sequence)
    .bind(killer_account.as_uuid())
    .bind(killer_character.as_uuid())
    .bind(victim_character.as_uuid())
    .execute(pool)
    .await
    .unwrap();
}

async fn pending_count(pool: &sqlx::PgPool, killer_account: wire::AccountId) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM tme.pending_player_kill_consequences WHERE killer_account_id=$1",
    )
    .bind(killer_account.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn world_checkpoint_sha(pool: &sqlx::PgPool) -> Vec<u8> {
    sqlx::query_scalar("SELECT checkpoint_sha256 FROM tme.facets")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn admit_character(
    state: &PostgresState,
    pool: &sqlx::PgPool,
    account: wire::AccountId,
    character: wire::CharacterId,
) -> Result<wire::ObserverFrame, String> {
    let session_id: Uuid = sqlx::query_scalar(
        "SELECT session_id FROM tme.sessions WHERE account_id=$1 \
         AND selected_character_id=$2 AND revoked_at IS NULL ORDER BY session_id LIMIT 1",
    )
    .bind(account.as_uuid())
    .bind(character.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    // One ticket at a time: the digest is the primary key, and each admission
    // in this test issues a fresh one.
    sqlx::query("DELETE FROM tme.socket_tickets")
        .execute(pool)
        .await
        .unwrap();
    let ticket = wire::AdmissionTicket::new("A".repeat(43)).unwrap();
    let epoch: i64 =
        sqlx::query_scalar("SELECT control_epoch FROM tme.characters WHERE character_id=$1")
            .bind(character.as_uuid())
            .fetch_one(pool)
            .await
            .unwrap();
    let actor_id: String =
        sqlx::query_scalar("SELECT actor_id FROM tme.characters WHERE character_id=$1")
            .bind(character.as_uuid())
            .fetch_one(pool)
            .await
            .unwrap();
    sqlx::query(
        "INSERT INTO tme.socket_tickets \
         (ticket_digest,session_id,account_id,character_id,actor_id, \
          expected_control_epoch,origin,host,selected_major,expires_at) \
         VALUES ($1,$2,$3,$4,$5,$6,'https://localhost:3000','localhost:3000',1, \
                 statement_timestamp()+interval '1 hour')",
    )
    .bind(Sha256::digest(ticket.expose_for_admission().as_bytes()).to_vec())
    .bind(session_id)
    .bind(account.as_uuid())
    .bind(character.as_uuid())
    .bind(&actor_id)
    .bind(epoch)
    .execute(pool)
    .await
    .unwrap();
    let (outbound, _outbound_receive) = mpsc::channel(8);
    let (terminal, _terminal_receive) = watch::channel(None);
    match state
        .admit(
            &ticket,
            &[wire::PROTOCOL_MINOR],
            "https://localhost:3000",
            "localhost:3000",
            outbound,
            terminal,
        )
        .await
    {
        Ok((_grant, welcome)) => Ok(welcome.frame),
        Err(error) => Err(format!("{error:?}")),
    }
}

async fn admit_and_read_karma(
    state: &PostgresState,
    pool: &sqlx::PgPool,
    account: wire::AccountId,
    character: wire::CharacterId,
) -> i64 {
    let frame = admit_character(state, pool, account, character)
        .await
        .unwrap_or_else(|error| panic!("admission failed: {error}"));
    i64::from(frame.character.karma_points)
}
