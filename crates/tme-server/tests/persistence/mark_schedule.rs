#[allow(clippy::too_many_arguments)]
// Private mark schedule proof for the persistence integration target.
async fn exercise_player_kill_mark_schedule(
    pool: &sqlx::PgPool,
    address: SocketAddr,
    host: &str,
    origin: &str,
    killer_account: wire::AccountId,
    killer_character: wire::CharacterId,
    killer_token: &str,
    killer_csrf: &wire::CsrfToken,
    victim_account: wire::AccountId,
    victim_character: wire::CharacterId,
    victim_token: &str,
    victim_csrf: &wire::CsrfToken,
) -> Uuid {
    let killer_session: Uuid = sqlx::query_scalar(
        "SELECT session_id FROM tme.sessions WHERE account_id=$1 \
         AND selected_character_id=$2 AND revoked_at IS NULL",
    )
    .bind(killer_account.as_uuid())
    .bind(killer_character.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    let victim_session: Uuid = sqlx::query_scalar(
        "SELECT session_id FROM tme.sessions WHERE account_id=$1 \
         AND selected_character_id=$2 AND revoked_at IS NULL",
    )
    .bind(victim_account.as_uuid())
    .bind(victim_character.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    let store = PostgresStore::new(pool.clone());

    let oldest = insert_synthetic_player_kill_mark(
        pool,
        100,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
        40,
        false,
    )
    .await;
    insert_expired_schedule_sentinel(
        pool,
        200,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
    )
    .await;
    store
        .reconcile_player_kill_marks(killer_account.as_uuid())
        .await
        .unwrap();
    assert_mark_schedule(pool, killer_account, &[(oldest, 2)]).await;

    let middle = insert_synthetic_player_kill_mark(
        pool,
        101,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
        30,
        false,
    )
    .await;
    insert_expired_schedule_sentinel(
        pool,
        201,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
    )
    .await;
    store
        .reconcile_player_kill_marks(killer_account.as_uuid())
        .await
        .unwrap();
    assert_mark_schedule(pool, killer_account, &[(oldest, 4), (middle, 2)]).await;

    let newest = insert_synthetic_player_kill_mark(
        pool,
        102,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
        20,
        false,
    )
    .await;
    insert_expired_schedule_sentinel(
        pool,
        202,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
    )
    .await;
    store
        .reconcile_player_kill_marks(killer_account.as_uuid())
        .await
        .unwrap();
    assert_mark_schedule(
        pool,
        killer_account,
        &[(oldest, 6), (middle, 4), (newest, 2)],
    )
    .await;

    let lockout = insert_synthetic_player_kill_mark(
        pool,
        103,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
        10,
        false,
    )
    .await;
    insert_expired_schedule_sentinel(
        pool,
        203,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
    )
    .await;
    store
        .reconcile_player_kill_marks(killer_account.as_uuid())
        .await
        .unwrap();
    assert_mark_schedule_paused(pool, killer_account, &[oldest, middle, newest, lockout]).await;
    let locked = post_json(
        address,
        host,
        origin,
        "/v4/socket-tickets",
        killer_token,
        &wire::SocketTicketRequestV1 {
            csrf_token: killer_csrf.clone(),
        },
    )
    .await;
    assert_eq!(locked.status, 423);
    assert_eq!(
        serde_json::from_slice::<wire::ControlErrorV1>(&locked.body).unwrap(),
        wire::ControlErrorV1 {
            code: wire::ControlErrorCode::GameplayMarkLocked,
        }
    );

    let forgiveness = post_json_with_csrf(
        address,
        host,
        origin,
        &format!("/v4/player-kill-marks/{lockout}/forgive"),
        victim_token,
        victim_csrf,
        &wire::ForgivePlayerKillMarkRequestV1 {
            request_id: wire::CommandId::new(Uuid::now_v7()).unwrap(),
        },
    )
    .await;
    assert_eq!(
        forgiveness.status,
        200,
        "{}",
        String::from_utf8_lossy(&forgiveness.body)
    );
    let forgiveness: wire::ForgivePlayerKillMarkResultV1 =
        serde_json::from_slice(&forgiveness.body).unwrap();
    assert_eq!(forgiveness.control_api_version, wire::CONTROL_API_VERSION);
    assert_mark_schedule(
        pool,
        killer_account,
        &[(oldest, 6), (middle, 4), (newest, 2)],
    )
    .await;

    sqlx::query(
        "UPDATE tme.player_kill_marks \
         SET expires_at=statement_timestamp()-interval '1 second' WHERE mark_id=$1",
    )
    .bind(newest)
    .execute(pool)
    .await
    .unwrap();
    store
        .reconcile_player_kill_marks(killer_account.as_uuid())
        .await
        .unwrap();
    assert_mark_schedule(pool, killer_account, &[(oldest, 4), (middle, 2)]).await;
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT expired_at IS NOT NULL AND expires_at IS NULL \
             FROM tme.player_kill_marks WHERE mark_id=$1",
        )
        .bind(newest)
        .fetch_one(pool)
        .await
        .unwrap()
    );

    let exit_mark = insert_synthetic_player_kill_mark(
        pool,
        104,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
        5,
        false,
    )
    .await;
    sqlx::query(
        "UPDATE tme.player_kill_marks SET linked_karma_added=true, \
         karma_forgiveness_eligible=true WHERE mark_id=$1",
    )
    .bind(exit_mark)
    .execute(pool)
    .await
    .unwrap();
    insert_expired_schedule_sentinel(
        pool,
        204,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
    )
    .await;
    store
        .reconcile_player_kill_marks(killer_account.as_uuid())
        .await
        .unwrap();
    assert_mark_schedule(
        pool,
        killer_account,
        &[(oldest, 6), (middle, 4), (exit_mark, 2)],
    )
    .await;
    prove_serializable_mark_reconciliation_retry(pool, killer_account).await;
    exit_mark
}

async fn prove_serializable_mark_reconciliation_retry(
    pool: &sqlx::PgPool,
    killer_account: wire::AccountId,
) {
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    sqlx::query("UPDATE tme.accounts SET display_name=display_name WHERE account_id=$1")
        .bind(killer_account.as_uuid())
        .execute(&mut *blocker)
        .await
        .unwrap();

    let first_store = PostgresStore::new(pool.clone());
    let second_store = first_store.clone();
    let account_id = killer_account.as_uuid();
    let first =
        tokio::spawn(async move { first_store.reconcile_player_kill_marks(account_id).await });
    let second =
        tokio::spawn(async move { second_store.reconcile_player_kill_marks(account_id).await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    blocker.commit().await.unwrap();
    let outcomes = [first.await.unwrap(), second.await.unwrap()];
    assert!(
        outcomes.iter().any(Result::is_err),
        "concurrent serializable reconciliation did not exercise a retryable conflict"
    );
    assert!(
        outcomes
            .iter()
            .filter_map(|result| result.as_ref().err())
            .all(|error| error.contains("could not serialize access due to concurrent update"))
    );

    let retry = PostgresStore::new(pool.clone());
    retry
        .reconcile_player_kill_marks(killer_account.as_uuid())
        .await
        .unwrap();
    retry.verify_player_kill_marks().await.unwrap();
}

#[allow(clippy::too_many_arguments)]
async fn insert_synthetic_player_kill_mark(
    pool: &sqlx::PgPool,
    sequence: i64,
    killer_account: wire::AccountId,
    killer_character: wire::CharacterId,
    killer_session: Uuid,
    victim_account: wire::AccountId,
    victim_character: wire::CharacterId,
    victim_session: Uuid,
    assessed_seconds_ago: i64,
    expires_in_past: bool,
) -> Uuid {
    let name = format!("https://tme.invalid/ids/player-kill-mark/v1/{sequence}");
    let mark_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, name.as_bytes());
    sqlx::query(
        "INSERT INTO tme.player_kill_marks \
         (mark_id,facet_kill_sequence,killer_account_id,killer_character_id, \
          victim_account_id,victim_character_id,killer_session_id,victim_session_id, \
          assessed_at,assessed_logical_millis,linked_karma_added, \
          karma_forgiveness_eligible,expires_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8, \
                 statement_timestamp()-make_interval(secs=>$9),$2,false,false, \
                 CASE WHEN $10 THEN statement_timestamp()-interval '1 second' ELSE NULL END)",
    )
    .bind(mark_id)
    .bind(sequence)
    .bind(killer_account.as_uuid())
    .bind(killer_character.as_uuid())
    .bind(victim_account.as_uuid())
    .bind(victim_character.as_uuid())
    .bind(killer_session)
    .bind(victim_session)
    .bind(assessed_seconds_ago)
    .bind(expires_in_past)
    .execute(pool)
    .await
    .unwrap();
    mark_id
}

#[allow(clippy::too_many_arguments)]
async fn insert_expired_schedule_sentinel(
    pool: &sqlx::PgPool,
    sequence: i64,
    killer_account: wire::AccountId,
    killer_character: wire::CharacterId,
    killer_session: Uuid,
    victim_account: wire::AccountId,
    victim_character: wire::CharacterId,
    victim_session: Uuid,
) {
    insert_synthetic_player_kill_mark(
        pool,
        sequence,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
        60,
        true,
    )
    .await;
}

async fn assert_mark_schedule(
    pool: &sqlx::PgPool,
    killer_account: wire::AccountId,
    expected: &[(Uuid, i64)],
) {
    let rows = sqlx::query(
        "SELECT mark_id,EXTRACT(EPOCH FROM (expires_at-statement_timestamp()))::double precision AS seconds \
         FROM tme.player_kill_marks WHERE killer_account_id=$1 \
         AND forgiven_at IS NULL AND expired_at IS NULL ORDER BY assessed_at,mark_id",
    )
    .bind(killer_account.as_uuid())
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), expected.len());
    for (row, (expected_id, weeks)) in rows.iter().zip(expected) {
        assert_eq!(row.get::<Uuid, _>("mark_id"), *expected_id);
        let actual: f64 = row.get("seconds");
        let wanted = (*weeks as f64) * 7.0 * 24.0 * 60.0 * 60.0;
        assert!(
            (actual - wanted).abs() < 10.0,
            "mark {expected_id} has {actual} seconds, expected {wanted}"
        );
    }
}

async fn assert_mark_schedule_paused(
    pool: &sqlx::PgPool,
    killer_account: wire::AccountId,
    expected_ids: &[Uuid],
) {
    let rows = sqlx::query(
        "SELECT mark_id,expires_at IS NULL AS paused FROM tme.player_kill_marks \
         WHERE killer_account_id=$1 AND forgiven_at IS NULL AND expired_at IS NULL \
         ORDER BY assessed_at,mark_id",
    )
    .bind(killer_account.as_uuid())
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), expected_ids.len());
    for (row, expected_id) in rows.iter().zip(expected_ids) {
        assert_eq!(row.get::<Uuid, _>("mark_id"), *expected_id);
        assert!(row.get::<bool, _>("paused"));
    }
}
