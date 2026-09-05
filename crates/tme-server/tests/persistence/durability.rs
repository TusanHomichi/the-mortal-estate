// Private durability proof for the persistence integration target.
#[test]
fn credential_and_ticket_debug_output_is_redacted() {
    let password = wire::Password::new(PASSWORD).unwrap();
    let csrf = wire::CsrfToken::new("A".repeat(43)).unwrap();
    let session = wire::SessionToken::new("A".repeat(43)).unwrap();
    assert_eq!(format!("{session:?}"), "SessionToken([REDACTED])");
    let ticket = wire::AdmissionTicket::new("E".repeat(43)).unwrap();
    assert_eq!(format!("{password:?}"), "Password([REDACTED])");
    assert_eq!(format!("{csrf:?}"), "CsrfToken([REDACTED])");
    assert_eq!(format!("{ticket:?}"), "AdmissionTicket([REDACTED])");
}

#[tokio::test]
#[ignore = "requires an EV runner-owned PostgreSQL database"]
async fn postgres_bootstrap_command_and_restart_are_durable() {
    let database_url = std::env::var("TME_TEST_DATABASE_URL")
        .expect("the exact PostgreSQL runner must provide TME_TEST_DATABASE_URL");
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    sqlx::raw_sql(
        "TRUNCATE tme.audit_events,tme.command_receipts,tme.socket_tickets,\
         tme.sessions,tme.characters,tme.facets,tme.account_credentials,\
         tme.accounts RESTART IDENTITY CASCADE",
    )
    .execute(&pool)
    .await
    .unwrap();

    two_client_social_socket_smoke(&database_url, &pool).await;
    sqlx::raw_sql(
        "TRUNCATE tme.audit_events,tme.command_receipts,tme.socket_tickets,\
         tme.sessions,tme.characters,tme.facets,tme.account_credentials,\
         tme.accounts RESTART IDENTITY CASCADE",
    )
    .execute(&pool)
    .await
    .unwrap();

    let account_id = wire::AccountId::new(Uuid::now_v7()).unwrap();
    let character_id = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let world_id = wire::FacetId::new(Uuid::now_v7()).unwrap();
    insert_account(&pool, account_id).await;

    let first = PostgresState::open(&database_url, bootstrap(account_id, character_id, world_id))
        .await
        .unwrap();
    assert!(first.gameplay_ready());
    let login = first
        .login(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            wire::LoginRequestV1 {
                username: wire::Username::new(USERNAME).unwrap(),
                password: wire::Password::new(PASSWORD).unwrap(),
            },
        )
        .await
        .unwrap();
    let token = login.session_token.expose().to_string();
    assert_eq!(
        login.bootstrap.control_api_version,
        wire::CONTROL_API_VERSION
    );
    let selection = first
        .select_character(
            &token,
            wire::CharacterSelectRequestV1 {
                csrf_token: login.bootstrap.csrf_token,
                character_id,
            },
        )
        .await
        .unwrap();
    assert_eq!(selection.control_api_version, wire::CONTROL_API_VERSION);
    let session_bootstrap = first.session_bootstrap(&token).await.unwrap();
    assert_eq!(
        session_bootstrap.control_api_version,
        wire::CONTROL_API_VERSION
    );
    let ticket = first
        .issue_ticket(
            &token,
            wire::SocketTicketRequestV1 {
                csrf_token: session_bootstrap.csrf_token,
            },
            "https://localhost:3000",
            "localhost:3000",
        )
        .await
        .unwrap();
    assert_eq!(ticket.protocol_major, 1);
    assert_eq!(ticket.supported_minors, vec![8]);
    let (outbound, _outbound_receive) = mpsc::channel(8);
    let (terminal, _terminal_receive) = watch::channel(None);
    let (grant, welcome) = first
        .admit(
            &ticket.ticket,
            &[wire::PROTOCOL_MINOR],
            "https://localhost:3000",
            "localhost:3000",
            outbound,
            terminal,
        )
        .await
        .unwrap();
    assert!(welcome.server_sequence > 0);
    let observation_center_before_restart = welcome.frame.observation_center.clone();
    let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let actor_id = wire::ActorId::new("player").unwrap();
    let command = wire::ClientCommandEnvelope::Command {
        command_id,
        control_epoch: wire::DecimalU64::new(grant.control.control_epoch),
        client_sequence: wire::DecimalU64::new(1),
        observed_world_revision: wire::DecimalU64::new(0),
        actor_id: actor_id.clone(),
        intent: wire::Intent::Wait,
    };
    let request_digest: [u8; 32] = Sha256::digest(serde_json::to_vec(&command).unwrap()).into();
    let reply = grant
        .facet
        .try_command(tme_server::facet::FacetCommand {
            connection_id: grant.control.connection_id,
            account_id,
            session_id: grant.control.session_id,
            character_id,
            command_id,
            control_epoch: grant.control.control_epoch,
            client_sequence: 1,
            observed_facet_revision: 0,
            actor_id,
            intent: wire::Intent::Wait,
            request_digest,
        })
        .unwrap()
        .await
        .unwrap();
    let (command_sequence, command_revision) = match reply.envelope {
        wire::ServerEnvelope::CommandResult {
            disposition: wire::CommandDisposition::Accepted,
            replay_status: wire::ReplayStatus::New,
            server_sequence: Some(server_sequence),
            after_revision: Some(after_revision),
            ..
        } => (server_sequence.get(), after_revision.get()),
        envelope => panic!("unexpected command reply: {envelope:?}"),
    };
    let row = sqlx::query(
        "SELECT facet_revision,last_server_sequence,checkpoint_bytes,checkpoint_sha256 \
         FROM tme.facets WHERE facet_id=$1",
    )
    .bind(world_id.as_uuid())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(row.get::<i64, _>("facet_revision") >= i64::try_from(command_revision).unwrap());
    assert!(row.get::<i64, _>("last_server_sequence") >= i64::try_from(command_sequence).unwrap());
    let checkpoint: Vec<u8> = row.get("checkpoint_bytes");
    let checkpoint_sha: Vec<u8> = row.get("checkpoint_sha256");
    assert_eq!(Sha256::digest(&checkpoint).as_slice(), checkpoint_sha);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM tme.command_receipts WHERE account_id=$1 AND command_id=$2",
        )
        .bind(account_id.as_uuid())
        .bind(command_id.as_uuid())
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );

    drop(grant);
    drop(first);
    tokio::time::sleep(Duration::from_millis(100)).await;
    let second = PostgresState::open(&database_url, bootstrap(account_id, character_id, world_id))
        .await
        .unwrap();
    let restarted = second.session_bootstrap(&token).await.unwrap();
    assert_eq!(restarted.selected_character_id, Some(character_id));
    assert_eq!(restarted.control_api_version, wire::CONTROL_API_VERSION);
    drop(second);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let third = PostgresState::open(&database_url, bootstrap(account_id, character_id, world_id))
        .await
        .unwrap();
    assert!(third.gameplay_ready());
    let restarted_session = third.session_bootstrap(&token).await.unwrap();
    let restarted_ticket = third
        .issue_ticket(
            &token,
            wire::SocketTicketRequestV1 {
                csrf_token: restarted_session.csrf_token,
            },
            "https://localhost:3000",
            "localhost:3000",
        )
        .await
        .unwrap();
    assert_eq!(restarted_ticket.protocol_major, 1);
    assert_eq!(restarted_ticket.supported_minors, vec![8]);
    let (outbound, _outbound_receive) = mpsc::channel(8);
    let (terminal, _terminal_receive) = watch::channel(None);
    let (restarted_grant, restarted_welcome) = third
        .admit(
            &restarted_ticket.ticket,
            &[wire::PROTOCOL_MINOR],
            "https://localhost:3000",
            "localhost:3000",
            outbound,
            terminal,
        )
        .await
        .unwrap();
    // D4: there is no arrival to transfer into. A restart must resume the
    // character exactly where the durable checkpoint left it.
    assert_eq!(
        restarted_welcome.frame.observation_center,
        observation_center_before_restart
    );
    drop(restarted_grant);

    http_reconnect_replay_and_logout(third, character_id).await;

    sqlx::query(
        "UPDATE tme.command_receipts SET full_expires_at=created_at+interval '1 microsecond' \
         WHERE account_id=$1 AND command_id=$2",
    )
    .bind(account_id.as_uuid())
    .bind(command_id.as_uuid())
    .execute(&pool)
    .await
    .unwrap();
    let tombstone = PostgresStore::new(pool.clone())
        .receipt(account_id, command_id)
        .await
        .unwrap()
        .unwrap();
    assert!(tombstone.outcome.is_none());
    let tombstone_row = sqlx::query(
        "SELECT disposition,outcome_bytes,session_id,actor_id,control_epoch, \
                client_sequence,server_sequence,before_revision,after_revision \
         FROM tme.command_receipts WHERE account_id=$1 AND command_id=$2",
    )
    .bind(account_id.as_uuid())
    .bind(command_id.as_uuid())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(tombstone_row.get::<String, _>("disposition"), "expired");
    for column in [
        "outcome_bytes",
        "session_id",
        "actor_id",
        "control_epoch",
        "client_sequence",
        "server_sequence",
        "before_revision",
        "after_revision",
    ] {
        assert!(tombstone_row.try_get_raw(column).unwrap().is_null());
    }

    let durable = sqlx::query(
        "SELECT checkpoint_bytes,checkpoint_sha256,content_digest \
         FROM tme.facets WHERE facet_id=$1",
    )
    .bind(world_id.as_uuid())
    .fetch_one(&pool)
    .await
    .unwrap();
    let checkpoint_bytes: Vec<u8> = durable.get("checkpoint_bytes");
    let checkpoint_sha: Vec<u8> = durable.get("checkpoint_sha256");
    let content_digest: Vec<u8> = durable.get("content_digest");

    sqlx::query("UPDATE tme.facets SET checkpoint_sha256=$2 WHERE facet_id=$1")
        .bind(world_id.as_uuid())
        .bind([0_u8; 32].as_slice())
        .execute(&pool)
        .await
        .unwrap();
    assert!(tme_server::operator::verify_store(&pool).await.is_err());
    sqlx::query("UPDATE tme.facets SET checkpoint_sha256=$2 WHERE facet_id=$1")
        .bind(world_id.as_uuid())
        .bind(&checkpoint_sha)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("UPDATE tme.facets SET content_digest=$2 WHERE facet_id=$1")
        .bind(world_id.as_uuid())
        .bind([0_u8; 32].as_slice())
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        PostgresState::open(&database_url, bootstrap(account_id, character_id, world_id),)
            .await
            .is_err()
    );
    sqlx::query("UPDATE tme.facets SET content_digest=$2 WHERE facet_id=$1")
        .bind(world_id.as_uuid())
        .bind(&content_digest)
        .execute(&pool)
        .await
        .unwrap();

    let invalid_checkpoint = b"{}";
    sqlx::query("UPDATE tme.facets SET checkpoint_bytes=$2,checkpoint_sha256=$3 WHERE facet_id=$1")
        .bind(world_id.as_uuid())
        .bind(invalid_checkpoint.as_slice())
        .bind(Sha256::digest(invalid_checkpoint).as_slice())
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        PostgresState::open(&database_url, bootstrap(account_id, character_id, world_id),)
            .await
            .is_err()
    );
    sqlx::query("UPDATE tme.facets SET checkpoint_bytes=$2,checkpoint_sha256=$3 WHERE facet_id=$1")
        .bind(world_id.as_uuid())
        .bind(&checkpoint_bytes)
        .bind(&checkpoint_sha)
        .execute(&pool)
        .await
        .unwrap();

    assert!(
        sqlx::query("UPDATE tme.facets SET checkpoint_schema=1 WHERE facet_id=$1")
            .bind(world_id.as_uuid())
            .execute(&pool)
            .await
            .is_err()
    );

    let session_id: Uuid =
        sqlx::query_scalar("SELECT session_id FROM tme.sessions WHERE account_id=$1 LIMIT 1")
            .bind(account_id.as_uuid())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        sqlx::query(
            "INSERT INTO tme.socket_tickets (
                 ticket_digest,session_id,account_id,character_id,facet_id,actor_id,
                 expected_control_epoch,origin,host,selected_major,expires_at
             ) VALUES ($1,$2,$3,$4,$5,'player',0,'https://localhost:3000',
                       'localhost:3000',1,statement_timestamp()+interval '30 seconds')",
        )
        .bind([9_u8; 32].as_slice())
        .bind(session_id)
        .bind(account_id.as_uuid())
        .bind(character_id.as_uuid())
        .bind(Uuid::now_v7())
        .execute(&pool)
        .await
        .is_err()
    );

    sqlx::query("UPDATE tme.characters SET actor_id='wrong_actor' WHERE character_id=$1")
        .bind(character_id.as_uuid())
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        PostgresState::open(&database_url, bootstrap(account_id, character_id, world_id),)
            .await
            .is_err()
    );
    sqlx::query("UPDATE tme.characters SET actor_id='player' WHERE character_id=$1")
        .bind(character_id.as_uuid())
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires a fenced disposable restore database"]
async fn fenced_restore_hydrates_and_commits_fresh_authenticated_command() {
    tme_server::telemetry::init();
    let database_url = std::env::var("TME_RESTORE_DATABASE_URL")
        .expect("the restore proof must provide TME_RESTORE_DATABASE_URL");
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    let account_uuid: Uuid =
        sqlx::query_scalar("SELECT account_id FROM tme.accounts WHERE username=$1")
            .bind(USERNAME)
            .fetch_one(&pool)
            .await
            .unwrap();
    let row = sqlx::query("SELECT character_id FROM tme.characters WHERE account_id=$1")
        .bind(account_uuid)
        .fetch_one(&pool)
        .await
        .unwrap();
    let account_id = wire::AccountId::new(account_uuid).unwrap();
    let character_id = wire::CharacterId::new(row.get("character_id")).unwrap();
    let world_id = wire::FacetId::new(
        sqlx::query_scalar("SELECT facet_id FROM tme.facets WHERE facet_key='world'")
            .fetch_one(&pool)
            .await
            .unwrap(),
    )
    .unwrap();
    let state = PostgresState::open(&database_url, bootstrap(account_id, character_id, world_id))
        .await
        .unwrap();
    let login = state
        .login(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            wire::LoginRequestV1 {
                username: wire::Username::new(USERNAME).unwrap(),
                password: wire::Password::new(PASSWORD).unwrap(),
            },
        )
        .await
        .unwrap();
    let token = login.session_token.expose().to_string();
    state
        .select_character(
            &token,
            wire::CharacterSelectRequestV1 {
                csrf_token: login.bootstrap.csrf_token,
                character_id,
            },
        )
        .await
        .unwrap();
    let session = state.session_bootstrap(&token).await.unwrap();
    let ticket = state
        .issue_ticket(
            &token,
            wire::SocketTicketRequestV1 {
                csrf_token: session.csrf_token,
            },
            "https://localhost:3000",
            "localhost:3000",
        )
        .await
        .unwrap();
    let (outbound, _outbound_receive) = mpsc::channel(8);
    let (terminal, _terminal_receive) = watch::channel(None);
    let (grant, welcome) = state
        .admit(
            &ticket.ticket,
            &[wire::PROTOCOL_MINOR],
            "https://localhost:3000",
            "localhost:3000",
            outbound,
            terminal,
        )
        .await
        .unwrap();
    let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let actor_id = wire::ActorId::new("player").unwrap();
    let command = wire::ClientCommandEnvelope::Command {
        command_id,
        control_epoch: wire::DecimalU64::new(grant.control.control_epoch),
        client_sequence: wire::DecimalU64::new(1),
        observed_world_revision: wire::DecimalU64::new(welcome.facet_revision),
        actor_id: actor_id.clone(),
        intent: wire::Intent::Wait,
    };
    let digest: [u8; 32] = Sha256::digest(serde_json::to_vec(&command).unwrap()).into();
    let reply = grant
        .facet
        .try_command(tme_server::facet::FacetCommand {
            connection_id: grant.control.connection_id,
            account_id,
            session_id: grant.control.session_id,
            character_id,
            command_id,
            control_epoch: grant.control.control_epoch,
            client_sequence: 1,
            observed_facet_revision: welcome.facet_revision,
            actor_id,
            intent: wire::Intent::Wait,
            request_digest: digest,
        })
        .unwrap()
        .await
        .unwrap();
    assert!(matches!(
        reply.envelope,
        wire::ServerEnvelope::CommandResult { .. }
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM tme.command_receipts WHERE account_id=$1 AND command_id=$2",
        )
        .bind(account_id.as_uuid())
        .bind(command_id.as_uuid())
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
    http_reconnect_replay_and_logout(state, character_id).await;
}
