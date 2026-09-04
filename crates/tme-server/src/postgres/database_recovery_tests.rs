use super::*;

#[cfg(test)]
pub(super) async fn ev_certify_command_postcommit_reload(
    database_url: &str,
    pool: &PgPool,
    state: Arc<PostgresState>,
    fixture: EvDatabaseFixture,
    cookie: &str,
    fault: EvCommandFault,
    enabled: bool,
) -> Arc<PostgresState> {
    use crate::coordinator::Reservation;

    eprintln!("EV source-fault stage: command postcommit reload");
    let (grant, welcome, _outbound, mut outbound_receive) =
        ev_admit_character(&state, cookie).await;
    let before_memory = ev_current_state(&grant).await;
    let before_store = ev_facet_row(pool, fixture.world_id).await;
    let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let wire_command = ev_wire_command(&grant, command_id, 1, welcome.facet_revision, enabled);
    let digest = ev_reserve_new(&state, fixture, command_id, &wire_command).await;
    if fault == EvCommandFault::CommitOutcomeUnknown {
        state
            .store
            .ev_arm_fault(crate::store::EvStoreFault::CommandCommitOutcomeUnknown);
    }
    let receive = grant
        .facet
        .try_command(ev_facet_command(
            &grant,
            command_id,
            1,
            welcome.facet_revision,
            enabled,
            digest,
            fault,
        ))
        .unwrap();
    assert!(
        receive.await.is_err(),
        "postcommit command fault emitted a success reply"
    );
    if fault == EvCommandFault::CommitOutcomeUnknown {
        state.store.ev_assert_fault_consumed();
    }
    state
        .coordinator
        .release(fixture.account_id, command_id, digest);
    assert!(!state.gameplay_ready());
    assert_eq!(ev_current_state(&grant).await, before_memory);
    assert!(outbound_receive.try_recv().is_err());
    let committed_store = ev_facet_row(pool, fixture.world_id).await;
    assert_eq!(committed_store.0, before_store.0 + 1);
    assert_eq!(committed_store.1, before_store.1 + 1);
    assert_ne!(committed_store.2, before_store.2);
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, command_id).await,
        (1, 1)
    );
    let expected_replay = state
        .store
        .receipt(fixture.account_id, command_id)
        .await
        .unwrap()
        .unwrap()
        .outcome
        .unwrap()
        .to_envelope(command_id, wire::ReplayStatus::Replayed)
        .unwrap();

    drop(state);
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    let reloaded = PostgresState::open(database_url, ev_database_bootstrap(fixture))
        .await
        .expect("command postcommit reload opens");
    assert!(reloaded.gameplay_ready());
    let replay = match reloaded
        .coordinator
        .reserve(fixture.account_id, command_id, &wire_command)
        .await
    {
        Reservation::Replay(envelope) => *envelope,
        _ => panic!("postcommit reload did not hydrate the durable receipt"),
    };
    assert_eq!(replay, expected_replay);
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, command_id).await,
        (1, 1)
    );
    let (_hydrated_grant, hydrated_welcome, _hydrated_outbound, _hydrated_receive) =
        ev_admit_character(&reloaded, cookie).await;
    assert_eq!(
        hydrated_welcome.frame.social.pages_enabled, enabled,
        "reload did not hydrate the committed command checkpoint",
    );
    eprintln!("EV source-fault stage complete: command postcommit reload");
    reloaded
}

#[cfg(test)]
pub(super) async fn ev_assert_required_task_revokes_readiness(state: &PostgresState, label: &str) {
    for _ in 0..64 {
        if !state.gameplay_ready() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        !state.gameplay_ready(),
        "required {label} exit must revoke readiness"
    );
    assert!(
        state.ready.seal_ready().is_err(),
        "required {label} exit must make readiness irreversible"
    );
}

#[cfg(test)]
#[tokio::test(flavor = "current_thread", start_paused = true)]
#[ignore = "requires the exact runner-owned EV PostgreSQL 18 database"]
pub(super) async fn ev_database_fault_certification() {
    let database_url =
        std::env::var("TME_EV_DATABASE_URL").expect("EV runner must provide TME_EV_DATABASE_URL");
    let expected_database =
        std::env::var("TME_EV_DATABASE_NAME").expect("EV runner must provide TME_EV_DATABASE_NAME");
    let expected_sentinel = std::env::var("TME_EV_DATABASE_SENTINEL")
        .expect("EV runner must provide TME_EV_DATABASE_SENTINEL");
    let expected_role =
        std::env::var("TME_EV_DATABASE_ROLE").expect("EV runner must provide TME_EV_DATABASE_ROLE");
    assert!(expected_database.starts_with("tme_ev_"));
    assert!(!expected_sentinel.is_empty());
    assert!(expected_role.starts_with("tme_ev_role_"));

    let (anchor_entered, anchor_entered_receive) = oneshot::channel();
    let anchor = tokio::spawn(async move {
        let _ = anchor_entered.send(());
        loop {
            tokio::task::yield_now().await;
        }
    });
    anchor_entered_receive
        .await
        .expect("EV paused-time yield anchor enters");
    assert!(!anchor.is_finished());

    let pool = runtime_pool(&database_url)
        .await
        .expect("runner-owned EV database connects");
    let row = sqlx::query(
        "SELECT current_database() AS database_name,current_user AS role_name,\
         shobj_description(oid,'pg_database') AS database_comment,\
         current_setting('server_version_num')::integer AS server_version_num \
         FROM pg_database WHERE datname=current_database()",
    )
    .fetch_one(&pool)
    .await
    .expect("runner-owned EV database identity is readable");
    assert_eq!(
        row.try_get::<String, _>("database_name").unwrap(),
        expected_database
    );
    assert_eq!(
        row.try_get::<String, _>("role_name").unwrap(),
        expected_role
    );
    assert_eq!(
        row.try_get::<String, _>("database_comment").unwrap(),
        format!("tme_ev:{expected_sentinel}")
    );
    assert!((180_000..190_000).contains(&row.try_get::<i32, _>("server_version_num").unwrap()));
    migrations::verify(&pool)
        .await
        .expect("runner-owned EV database has the exact tracked migrations");
    certify_command_reservation_race(&pool).await;

    let fixture = EvDatabaseFixture {
        account_id: wire::AccountId::new(Uuid::now_v7()).unwrap(),
        character_id: wire::CharacterId::new(Uuid::now_v7()).unwrap(),
        session_id: wire::SessionId::new(Uuid::now_v7()).unwrap(),
        world_id: wire::FacetId::new(Uuid::now_v7()).unwrap(),
    };
    let cookie = "ev-source-fault-cookie";
    let csrf = wire::CsrfToken::new("A".repeat(43)).unwrap();
    ev_insert_account(&pool, fixture).await;
    let state = PostgresState::open(&database_url, ev_database_bootstrap(fixture))
        .await
        .expect("EV source-fault service opens");
    ev_insert_session(&pool, fixture, cookie, &csrf).await;
    let logical_before = tokio::time::Instant::now();
    let wall_before = std::time::Instant::now();
    let (wall_send, wall_receive) = oneshot::channel();
    let wall_thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(25));
        let _ = wall_send.send(());
    });
    wall_receive.await.expect("EV wall-clock probe completes");
    wall_thread.join().expect("EV wall-clock probe joins");
    assert!(wall_before.elapsed() >= Duration::from_millis(25));
    assert_eq!(
        tokio::time::Instant::now(),
        logical_before,
        "real wall time must not advance paused logical time"
    );
    assert!(!anchor.is_finished());

    ev_certify_direct_store_failures(&database_url, &pool, &state, fixture).await;
    ev_certify_command_pipeline(&pool, &state, fixture, cookie).await;
    let state = ev_certify_command_postcommit_reload(
        &database_url,
        &pool,
        state,
        fixture,
        cookie,
        EvCommandFault::CommitOutcomeUnknown,
        true,
    )
    .await;
    let state = ev_certify_command_postcommit_reload(
        &database_url,
        &pool,
        state,
        fixture,
        cookie,
        EvCommandFault::AfterStoreCommit,
        false,
    )
    .await;
    assert!(state.gameplay_ready());

    state.world.handle.ev_abort_facet_task();
    ev_assert_required_task_revokes_readiness(&state, "persisted facet").await;
    drop(state);
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    let state = PostgresState::open(&database_url, ev_database_bootstrap(fixture))
        .await
        .expect("EV reload after persisted-facet abort opens");
    assert!(state.gameplay_ready());

    state.world.handle.ev_abort_scheduler_task();
    ev_assert_required_task_revokes_readiness(&state, "facet scheduler").await;
    drop(state);
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    let state = PostgresState::open(&database_url, ev_database_bootstrap(fixture))
        .await
        .expect("EV reload after scheduler abort opens");
    assert!(state.gameplay_ready());

    let reconciler = state.required_tasks.abort_reconciler();
    assert!(
        reconciler.await.unwrap_err().is_cancelled(),
        "EV reconciler abort must cancel the live task"
    );
    ev_assert_required_task_revokes_readiness(&state, "reconciler").await;
    drop(state);
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    pool.close().await;
    anchor.abort();
    let _ = anchor.await;
}

#[cfg(test)]
mod certification_tests {
    use super::*;

    #[test]
    fn failed_startup_readiness_cannot_be_resealed() {
        let readiness = GameplayReadiness::new();
        readiness.fail();
        assert!(readiness.seal_ready().is_err());
        assert!(!readiness.is_ready());
    }

    #[test]
    fn required_task_exit_revokes_sealed_readiness() {
        let readiness = GameplayReadiness::new();
        readiness.seal_ready().unwrap();
        assert!(readiness.is_ready());
        readiness.fail();
        assert!(!readiness.is_ready());
        assert!(readiness.seal_ready().is_err());
    }
}
