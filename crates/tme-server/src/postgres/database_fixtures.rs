use super::*;

#[cfg(test)]
pub(super) async fn certify_command_reservation_race(pool: &PgPool) {
    use crate::coordinator::Reservation;

    let account_uuid = Uuid::now_v7();
    let account_id = wire::AccountId::new(account_uuid).unwrap();
    let session_id = wire::SessionId::new(Uuid::now_v7()).unwrap();
    let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let username = format!("ev_{}", &account_uuid.as_simple().to_string()[..12]);
    sqlx::query(
        "INSERT INTO tme.accounts(account_id,username,display_name) VALUES($1,$2,'EV Race')",
    )
    .bind(account_id.as_uuid())
    .bind(username)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tme.sessions \
         (session_id,account_id,token_digest,csrf_digest,idle_expires_at,absolute_expires_at) \
         VALUES($1,$2,$3,$4,statement_timestamp()+interval '1 hour', \
                statement_timestamp()+interval '1 day')",
    )
    .bind(session_id.as_uuid())
    .bind(account_id.as_uuid())
    .bind([11_u8; 32].as_slice())
    .bind([12_u8; 32].as_slice())
    .execute(pool)
    .await
    .unwrap();

    let command = wire::ClientCommandEnvelope::Command {
        command_id,
        control_epoch: wire::DecimalU64::new(1),
        client_sequence: wire::DecimalU64::new(1),
        observed_world_revision: wire::DecimalU64::new(0),
        actor_id: wire::ActorId::new("player").unwrap(),
        intent: wire::Intent::Wait,
    };
    let store = Arc::new(PostgresStore::new(pool.clone()));
    let coordinator = Arc::new(Coordinator::new(store));
    let barrier = Arc::new(tokio::sync::Barrier::new(3));
    let mut racers = Vec::new();
    for _ in 0..2 {
        let coordinator = coordinator.clone();
        let command = command.clone();
        let barrier = barrier.clone();
        racers.push(tokio::spawn(async move {
            barrier.wait().await;
            coordinator.reserve(account_id, command_id, &command).await
        }));
    }
    barrier.wait().await;

    let mut new_digest = None;
    let mut in_progress = 0;
    for racer in racers {
        match racer.await.unwrap() {
            Reservation::New { digest } => {
                assert!(new_digest.replace(digest).is_none());
            }
            Reservation::InProgress => in_progress += 1,
            _ => panic!("same-ID reservation race returned an invalid outcome"),
        }
    }
    assert_eq!(in_progress, 1);
    let digest = new_digest.expect("exactly one racer owns new execution");
    let new_result = coordinator
        .complete_authority_rejection(
            account_id,
            session_id,
            command_id,
            digest,
            wire::RejectionCode::StaleControlEpoch,
        )
        .await
        .unwrap();
    let replay = match coordinator.reserve(account_id, command_id, &command).await {
        Reservation::Replay(envelope) => *envelope,
        _ => panic!("completed same-ID reservation did not become durable replay"),
    };
    let mut expected_replay = new_result;
    let wire::ServerEnvelope::CommandResult { replay_status, .. } = &mut expected_replay else {
        panic!("authority rejection did not return a command result");
    };
    *replay_status = wire::ReplayStatus::Replayed;
    assert_eq!(replay, expected_replay);
    let receipt_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tme.command_receipts WHERE account_id=$1 AND command_id=$2",
    )
    .bind(account_id.as_uuid())
    .bind(command_id.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(receipt_count, 1);

    sqlx::query("DELETE FROM tme.audit_events WHERE account_id=$1")
        .bind(account_id.as_uuid())
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM tme.accounts WHERE account_id=$1")
        .bind(account_id.as_uuid())
        .execute(pool)
        .await
        .unwrap();
}

#[cfg(test)]
#[derive(Clone, Copy)]
pub(super) struct EvDatabaseFixture {
    pub(super) account_id: wire::AccountId,
    pub(super) character_id: wire::CharacterId,
    pub(super) session_id: wire::SessionId,
    pub(super) world_id: wire::FacetId,
}

#[cfg(test)]
pub(super) fn ev_database_engine() -> Engine {
    let mut scenario = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    scenario.extend([
        "..",
        "..",
        "content",
        "test-corpus",
        "world_topology_gallery.json",
    ]);
    tme_sim::load_engine_from_scenario(&scenario, Some(7))
        .expect("EV database certification scenario loads")
}

#[cfg(test)]
pub(super) fn ev_database_bootstrap(fixture: EvDatabaseFixture) -> PostgresBootstrap {
    PostgresBootstrap {
        world: PostgresWorldBootstrap {
            facet_id: fixture.world_id,
            key: "ev-world".to_string(),
            engine: ev_database_engine(),
        },
        characters: vec![PostgresCharacterBootstrap {
            account_id: fixture.account_id,
            character_id: fixture.character_id,
            slot: 1,
            display_name: wire::DisplayName::new("EV Fault Character").unwrap(),
            actor_id: ActorId::new("player"),
        }],
    }
}

#[cfg(test)]
pub(super) async fn ev_insert_account(pool: &PgPool, fixture: EvDatabaseFixture) {
    let username = format!(
        "ev_fault_{}",
        &fixture.account_id.as_uuid().as_simple().to_string()[..12]
    );
    sqlx::query("INSERT INTO tme.accounts(account_id,username,display_name) VALUES($1,$2,$3)")
        .bind(fixture.account_id.as_uuid())
        .bind(username)
        .bind("EV Fault Account")
        .execute(pool)
        .await
        .unwrap();
}

#[cfg(test)]
pub(super) async fn ev_insert_session(
    pool: &PgPool,
    fixture: EvDatabaseFixture,
    cookie: &str,
    csrf: &wire::CsrfToken,
) {
    sqlx::query(
        "INSERT INTO tme.sessions \
         (session_id,account_id,token_digest,csrf_digest,selected_character_id, \
          idle_expires_at,absolute_expires_at) \
         VALUES($1,$2,$3,$4,$5,statement_timestamp()+interval '1 hour', \
                statement_timestamp()+interval '1 day')",
    )
    .bind(fixture.session_id.as_uuid())
    .bind(fixture.account_id.as_uuid())
    .bind(digest(cookie).as_slice())
    .bind(digest(csrf.expose_for_validation()).as_slice())
    .bind(fixture.character_id.as_uuid())
    .execute(pool)
    .await
    .unwrap();
}

#[cfg(test)]
pub(super) async fn ev_new_csrf(state: &PostgresState, cookie: &str) -> wire::CsrfToken {
    state
        .session_bootstrap(cookie)
        .await
        .expect("EV session remains live")
        .csrf_token
}

#[cfg(test)]
pub(super) async fn ev_facet_row(
    pool: &PgPool,
    facet_id: wire::FacetId,
) -> (i64, i64, Vec<u8>, Vec<u8>) {
    let row = sqlx::query(
        "SELECT facet_revision,last_server_sequence,checkpoint_bytes,checkpoint_sha256 \
         FROM tme.facets WHERE facet_id=$1",
    )
    .bind(facet_id.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    (
        row.try_get("facet_revision").unwrap(),
        row.try_get("last_server_sequence").unwrap(),
        row.try_get("checkpoint_bytes").unwrap(),
        row.try_get("checkpoint_sha256").unwrap(),
    )
}

#[cfg(test)]
pub(super) async fn ev_command_artifacts(
    pool: &PgPool,
    account_id: wire::AccountId,
    command_id: wire::CommandId,
) -> (i64, i64) {
    let receipts = sqlx::query_scalar(
        "SELECT count(*) FROM tme.command_receipts WHERE account_id=$1 AND command_id=$2",
    )
    .bind(account_id.as_uuid())
    .bind(command_id.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    let audits = sqlx::query_scalar(
        "SELECT count(*) FROM tme.audit_events WHERE account_id=$1 AND command_id=$2",
    )
    .bind(account_id.as_uuid())
    .bind(command_id.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    (receipts, audits)
}

#[cfg(test)]
pub(super) async fn ev_certify_direct_store_failures(
    database_url: &str,
    pool: &PgPool,
    state: &Arc<PostgresState>,
    fixture: EvDatabaseFixture,
) {
    use crate::store::EvStoreFault;

    eprintln!("EV source-fault stage: direct store failures");

    let system_faults = [
        EvStoreFault::SystemSqlAcquire,
        EvStoreFault::SystemCompareAndSwap,
        EvStoreFault::SystemAudit,
        EvStoreFault::SystemCommit,
    ];
    for fault in system_faults {
        let before = ev_facet_row(pool, fixture.world_id).await;
        let checkpoint = FacetCheckpointV5::from_bytes(before.2.clone()).unwrap();
        let audit_before: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM tme.audit_events WHERE action='facet_deadlines'",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        state.store.ev_arm_fault(fault);
        let result = state
            .store
            .commit_system(crate::store::SystemCommit {
                facet_id: fixture.world_id,
                expected_server_sequence: u64::try_from(before.1).unwrap(),
                expected_revision: u64::try_from(before.0).unwrap(),
                next_server_sequence: u64::try_from(before.1).unwrap() + 1,
                next_revision: u64::try_from(before.0).unwrap() + 1,
                checkpoint: &checkpoint,
                action: "facet_deadlines",
                durable_effects: &[],
            })
            .await;
        assert!(result.is_err(), "{fault:?} must fail system persistence");
        state.store.ev_assert_fault_consumed();
        assert_eq!(ev_facet_row(pool, fixture.world_id).await, before);
        let audit_after: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM tme.audit_events WHERE action='facet_deadlines'",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(audit_after, audit_before);
    }

    let before = ev_facet_row(pool, fixture.world_id).await;
    let checkpoint = FacetCheckpointV5::from_bytes(before.2.clone()).unwrap();
    let stale_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let stale_outcome = ReceiptOutcomeV3::rejected(
        wire::RejectionCode::FutureWorldRevision,
        Some(u64::try_from(before.1).unwrap() + 1),
        Some(u64::try_from(before.0).unwrap()),
    );
    let stale = state
        .store
        .commit_command(crate::store::CommandCommit {
            account_id: fixture.account_id,
            session_id: fixture.session_id,
            character_id: fixture.character_id,
            command_id: stale_id,
            request_digest: [31; 32],
            facet_id: fixture.world_id,
            actor_id: "player",
            control_epoch: 0,
            client_sequence: 1,
            expected_server_sequence: u64::try_from(before.1).unwrap() + 1,
            expected_revision: u64::try_from(before.0).unwrap(),
            next_server_sequence: u64::try_from(before.1).unwrap() + 2,
            next_revision: u64::try_from(before.0).unwrap(),
            checkpoint: &checkpoint,
            outcome: &stale_outcome,
            durable_effects: &[],
        })
        .await;
    assert!(stale.is_err(), "natural stale CAS must fail");
    assert_eq!(ev_facet_row(pool, fixture.world_id).await, before);
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, stale_id).await,
        (0, 0)
    );

    let effect_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let effect_outcome = ReceiptOutcomeV3::rejected(
        wire::RejectionCode::FutureWorldRevision,
        Some(u64::try_from(before.1).unwrap() + 1),
        Some(u64::try_from(before.0).unwrap()),
    );
    let missing_effect =
        tme_rules::DurableGameplayEffectV1::PlayerKillAssessed(tme_rules::PlayerKillAssessmentV1 {
            facet_kill_sequence: 1,
            killer_character_id: CharacterId::new(Uuid::now_v7().to_string()),
            victim_character_id: CharacterId::new(Uuid::now_v7().to_string()),
            exempt_self_defense: false,
            consequence: tme_rules::PlayerKillConsequenceV1::AppliedHere {
                linked_karma_added: false,
            },
            logical_time: tme_rules::LogicalTime::new(1),
        });
    let effect_failure = state
        .store
        .commit_command(crate::store::CommandCommit {
            account_id: fixture.account_id,
            session_id: fixture.session_id,
            character_id: fixture.character_id,
            command_id: effect_id,
            request_digest: [33; 32],
            facet_id: fixture.world_id,
            actor_id: "player",
            control_epoch: 0,
            client_sequence: 1,
            expected_server_sequence: u64::try_from(before.1).unwrap(),
            expected_revision: u64::try_from(before.0).unwrap(),
            next_server_sequence: u64::try_from(before.1).unwrap() + 1,
            next_revision: u64::try_from(before.0).unwrap(),
            checkpoint: &checkpoint,
            outcome: &effect_outcome,
            durable_effects: std::slice::from_ref(&missing_effect),
        })
        .await;
    assert!(effect_failure.is_err());
    assert_eq!(ev_facet_row(pool, fixture.world_id).await, before);
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, effect_id).await,
        (0, 0)
    );

    let timeout_pool = PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query("SET lock_timeout='100ms'")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await
        .expect("EV timeout pool connects");
    let timeout_store = PostgresStore::new(timeout_pool.clone());
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("SELECT facet_id FROM tme.facets WHERE facet_id=$1 FOR UPDATE")
        .bind(fixture.world_id.as_uuid())
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    let lock_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let lock_outcome = ReceiptOutcomeV3::rejected(
        wire::RejectionCode::FutureWorldRevision,
        Some(u64::try_from(before.1).unwrap() + 1),
        Some(u64::try_from(before.0).unwrap()),
    );
    let locked = timeout_store
        .commit_command(crate::store::CommandCommit {
            account_id: fixture.account_id,
            session_id: fixture.session_id,
            character_id: fixture.character_id,
            command_id: lock_id,
            request_digest: [32; 32],
            facet_id: fixture.world_id,
            actor_id: "player",
            control_epoch: 0,
            client_sequence: 1,
            expected_server_sequence: u64::try_from(before.1).unwrap(),
            expected_revision: u64::try_from(before.0).unwrap(),
            next_server_sequence: u64::try_from(before.1).unwrap() + 1,
            next_revision: u64::try_from(before.0).unwrap(),
            checkpoint: &checkpoint,
            outcome: &lock_outcome,
            durable_effects: &[],
        })
        .await;
    assert!(
        locked.is_err(),
        "real PostgreSQL row lock must hit lock_timeout"
    );
    lock.rollback().await.unwrap();
    timeout_pool.close().await;
    assert_eq!(ev_facet_row(pool, fixture.world_id).await, before);
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, lock_id).await,
        (0, 0)
    );
    eprintln!("EV source-fault stage complete: direct store failures");
}

#[cfg(test)]
pub(super) fn ev_wire_command(
    grant: &AdmissionGrant,
    command_id: wire::CommandId,
    client_sequence: u64,
    observed_facet_revision: u64,
    enabled: bool,
) -> wire::ClientCommandEnvelope {
    wire::ClientCommandEnvelope::Command {
        command_id,
        control_epoch: wire::DecimalU64::new(grant.control.control_epoch),
        client_sequence: wire::DecimalU64::new(client_sequence),
        observed_world_revision: wire::DecimalU64::new(observed_facet_revision),
        actor_id: wire::ActorId::new(grant.control.actor_id.as_str()).unwrap(),
        intent: wire::Intent::SetPagesEnabled { enabled },
    }
}

#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EvCommandFault {
    None,
    CheckpointExport,
    AfterStoreCommit,
    CommitOutcomeUnknown,
}

#[cfg(test)]
pub(super) fn ev_facet_command(
    grant: &AdmissionGrant,
    command_id: wire::CommandId,
    client_sequence: u64,
    observed_facet_revision: u64,
    enabled: bool,
    request_digest: [u8; 32],
    fault: EvCommandFault,
) -> crate::facet::FacetCommand {
    crate::facet::FacetCommand {
        connection_id: grant.control.connection_id,
        account_id: grant.control.account_id,
        session_id: grant.control.session_id,
        character_id: grant.control.character_id,
        command_id,
        control_epoch: grant.control.control_epoch,
        client_sequence,
        observed_facet_revision,
        actor_id: wire::ActorId::new(grant.control.actor_id.as_str()).unwrap(),
        intent: wire::Intent::SetPagesEnabled { enabled },
        request_digest,
        certification_trace: None,
        ev_fail_checkpoint_export: fault == EvCommandFault::CheckpointExport,
        ev_fail_after_store_commit: fault == EvCommandFault::AfterStoreCommit,
    }
}

#[cfg(test)]
pub(super) async fn ev_current_state(grant: &AdmissionGrant) -> wire::ServerEnvelope {
    grant
        .facet
        .try_current_state(grant.control.connection_id)
        .expect("EV current-state request enqueues")
        .await
        .expect("EV current-state reply arrives")
        .expect("EV observer remains installed")
}

#[cfg(test)]
pub(super) fn ev_state_revision(envelope: &wire::ServerEnvelope) -> u64 {
    match envelope {
        wire::ServerEnvelope::StateUpdate { world_revision, .. } => world_revision.get(),
        other => panic!("expected EV state update, got {other:?}"),
    }
}

#[cfg(test)]
pub(super) async fn ev_admit_character(
    state: &Arc<PostgresState>,
    cookie: &str,
) -> (
    AdmissionGrant,
    FacetWelcome,
    mpsc::Sender<wire::ServerEnvelope>,
    mpsc::Receiver<wire::ServerEnvelope>,
) {
    let csrf = ev_new_csrf(state, cookie).await;
    let ticket = state
        .issue_ticket(
            cookie,
            wire::SocketTicketRequestV1 { csrf_token: csrf },
            "https://ev.invalid",
            "ev.invalid",
        )
        .await
        .expect("EV command ticket issues");
    let (outbound, outbound_receive) = mpsc::channel(crate::config::OUTBOUND_QUEUE_CAPACITY);
    let (terminal, _terminal_receive) = watch::channel(None);
    let (grant, welcome) = state
        .admit(
            &ticket.ticket,
            &[wire::PROTOCOL_MINOR],
            "https://ev.invalid",
            "ev.invalid",
            outbound.clone(),
            terminal,
        )
        .await
        .expect("EV command character admits");
    (grant, welcome, outbound, outbound_receive)
}

#[cfg(test)]
pub(super) async fn ev_reserve_new(
    state: &PostgresState,
    fixture: EvDatabaseFixture,
    command_id: wire::CommandId,
    command: &wire::ClientCommandEnvelope,
) -> [u8; 32] {
    match state
        .coordinator
        .reserve(fixture.account_id, command_id, command)
        .await
    {
        crate::coordinator::Reservation::New { digest } => digest,
        _ => panic!("EV command reservation must be new"),
    }
}

#[cfg(test)]
pub(super) async fn ev_wait_for_mailbox_state(grant: &AdmissionGrant) -> wire::ServerEnvelope {
    loop {
        match grant.facet.try_current_state(grant.control.connection_id) {
            Ok(receive) => {
                return receive
                    .await
                    .expect("EV mailbox state reply arrives")
                    .expect("EV command observer remains installed");
            }
            Err(crate::facet::FacetError::QueueFull) => tokio::task::yield_now().await,
            Err(error) => panic!("EV mailbox became unavailable: {error:?}"),
        }
    }
}
