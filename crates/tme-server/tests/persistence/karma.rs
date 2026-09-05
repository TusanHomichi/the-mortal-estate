/// Owner ruling 2026-08-20 (successor issue #3): logging off is not a karma
/// escape. Covers the four properties the ruling needs — the debt survives a
/// process restart, it applies exactly once, a rolled-back admission leaves it
/// owed rather than silently paid, and a present killer is untouched.

// Private karma proof for the persistence integration target.
#[tokio::test]
#[ignore = "requires an EV runner-owned PostgreSQL database"]
async fn absent_killer_karma_is_deferred_and_applied_exactly_once() {
    let database_url = std::env::var("TME_TEST_DATABASE_URL")
        .expect("the exact PostgreSQL runner must provide TME_TEST_DATABASE_URL");
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    truncate_all(&pool).await;

    let killer_account = wire::AccountId::new(Uuid::now_v7()).unwrap();
    let killer_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let replacement_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let victim_account = wire::AccountId::new(Uuid::now_v7()).unwrap();
    let victim_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let world_id = wire::FacetId::new(Uuid::now_v7()).unwrap();
    insert_account_named(
        &pool,
        killer_account,
        "pending_killer",
        "Pending Killer",
        11,
    )
    .await;
    insert_account_named(
        &pool,
        victim_account,
        "pending_victim",
        "Pending Victim",
        12,
    )
    .await;

    let build = || {
        social_bootstrap(
            killer_account,
            killer_character,
            replacement_character,
            victim_account,
            victim_character,
            world_id,
        )
    };

    // A kill lands while the killer is away. The mark and the deferred
    // consequence are written together, exactly as the runtime writes them.
    let first = PostgresState::open(&database_url, build()).await.unwrap();
    let (killer_session, victim_session) = open_sessions(
        &pool,
        killer_account,
        killer_character,
        victim_account,
        victim_character,
    )
    .await;
    let karma_before = admit_and_read_karma(&first, &pool, killer_account, killer_character).await;
    drop(first);

    insert_synthetic_player_kill_mark(
        &pool,
        7,
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
    insert_pending_consequence(&pool, 7, killer_account, killer_character, victim_character).await;
    schedule_marks(&pool).await;
    assert_eq!(1, pending_count(&pool, killer_account).await);
    assert!(
        !forgivable(&pool, 7).await,
        "a consequence that has not landed yet is not forgivable"
    );

    // PROPERTY 1 — the debt survives the process. This is a brand new
    // PostgresState over the same database, which is what a restart is.
    let second = PostgresState::open(&database_url, build()).await.unwrap();
    let karma_after = admit_and_read_karma(&second, &pool, killer_account, killer_character).await;
    assert_eq!(
        0,
        pending_count(&pool, killer_account).await,
        "an applied consequence must be cleared"
    );
    // One kill, one karma point — asserted concretely so this cannot pass on
    // some unrelated drift in the sheet.
    assert_eq!(
        karma_before + 1,
        karma_after,
        "the deferred consequence never reached the killer's sheet"
    );
    let linked: bool = sqlx::query_scalar(
        "SELECT linked_karma_added FROM tme.player_kill_marks WHERE facet_kill_sequence=7",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        linked,
        "the mark still claims no karma was linked after the consequence landed"
    );
    // Owner ruling 2026-08-20: forgiveness follows the karma. Once it lands the
    // victim can forgive, exactly as if the killer had never left.
    assert!(
        forgivable(&pool, 7).await,
        "karma landed but the victim still cannot forgive it"
    );

    // PROPERTY 2 — exactly once. Re-admitting must not charge them twice.
    let karma_replayed =
        admit_and_read_karma(&second, &pool, killer_account, killer_character).await;
    assert_eq!(
        karma_after, karma_replayed,
        "a second admission applied the same consequence again"
    );
    // Admission on its own never moves karma, which is what makes the
    // assertion above evidence of cause rather than coincidence.
    assert_eq!(karma_before + 1, karma_replayed);
    assert_eq!(0, pending_count(&pool, killer_account).await);
    drop(second);

    // PROPERTY 3 — a rolled-back admission leaves the debt owed. Four active
    // marks lock the account, which aborts the admission transaction AFTER the
    // clear would have run. If the clear were in its own transaction, the
    // consequence would vanish unpaid.
    // A second, distinct kill — the rules refuse to apply one kill's
    // consequence twice, which is a property of the engine, not of this table.
    insert_synthetic_player_kill_mark(
        &pool,
        8,
        killer_account,
        killer_character,
        killer_session,
        victim_account,
        victim_character,
        victim_session,
        9,
        false,
    )
    .await;
    insert_pending_consequence(&pool, 8, killer_account, killer_character, victim_character).await;
    for sequence in [11_i64, 12, 13, 14] {
        insert_synthetic_player_kill_mark(
            &pool,
            sequence,
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
    }
    schedule_marks(&pool).await;
    let third = PostgresState::open(&database_url, build()).await.unwrap();
    let checkpoint_before = world_checkpoint_sha(&pool).await;
    assert!(
        admit_character(&third, &pool, killer_account, killer_character)
            .await
            .is_err(),
        "a gameplay-mark-locked account must not be admitted"
    );
    assert_eq!(
        1,
        pending_count(&pool, killer_account).await,
        "a rolled-back admission silently discharged the debt"
    );
    assert_eq!(
        checkpoint_before,
        world_checkpoint_sha(&pool).await,
        "a rolled-back admission still moved the durable world"
    );
    drop(third);

    // PROPERTY 5 — a deferred consequence survives disaster recovery. A
    // restored database is a different database as far as the store is
    // concerned, so the restore fence must clear before anything runs, and the
    // debt must still be owed on the far side of it. The row is the very one
    // PROPERTY 3 preserved through its rolled-back admission; the marks that
    // locked the account come out so admission can proceed this time.
    for sequence in [11_i64, 12, 13, 14] {
        sqlx::query("DELETE FROM tme.player_kill_marks WHERE facet_kill_sequence=$1")
            .bind(sequence)
            .execute(&pool)
            .await
            .unwrap();
    }
    schedule_marks(&pool).await;
    sqlx::query("UPDATE tme.store_state SET database_oid='424242' WHERE singleton")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        PostgresState::open(&database_url, build()).await.is_err(),
        "a restored database must refuse to open before it is fenced"
    );
    assert_eq!(
        1,
        pending_count(&pool, killer_account).await,
        "the refused open discarded the debt"
    );
    let epoch_before: i64 =
        sqlx::query_scalar("SELECT restore_fence_epoch FROM tme.store_state WHERE singleton")
            .fetch_one(&pool)
            .await
            .unwrap();
    unsafe { std::env::set_var("DATABASE_URL", &database_url) };
    tme_server::operator::run(&[
        "store".to_string(),
        "restore-fence".to_string(),
        "--confirm-restored-database".to_string(),
    ])
    .await
    .expect("the operator fences the restored database");
    let epoch_after: i64 =
        sqlx::query_scalar("SELECT restore_fence_epoch FROM tme.store_state WHERE singleton")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(epoch_before + 1, epoch_after, "the fence did not advance");
    // The fence invalidates every session it inherited, so the killer comes
    // back through a fresh one — which is exactly the path the debt must
    // survive.
    open_sessions(
        &pool,
        killer_account,
        killer_character,
        victim_account,
        victim_character,
    )
    .await;
    let fifth = PostgresState::open(&database_url, build()).await.unwrap();
    let karma_restored =
        admit_and_read_karma(&fifth, &pool, killer_account, killer_character).await;
    assert_eq!(
        karma_replayed + 1,
        karma_restored,
        "the debt did not survive the restore"
    );
    assert_eq!(
        0,
        pending_count(&pool, killer_account).await,
        "the restored consequence was applied but not cleared"
    );
    drop(fifth);

    // PROPERTY 4 — the present-killer path writes no pending row at all.
    truncate_all(&pool).await;
    insert_account_named(
        &pool,
        killer_account,
        "pending_killer",
        "Pending Killer",
        11,
    )
    .await;
    insert_account_named(
        &pool,
        victim_account,
        "pending_victim",
        "Pending Victim",
        12,
    )
    .await;
    let fourth = PostgresState::open(&database_url, build()).await.unwrap();
    open_sessions(
        &pool,
        killer_account,
        killer_character,
        victim_account,
        victim_character,
    )
    .await;
    let _ = admit_and_read_karma(&fourth, &pool, killer_account, killer_character).await;
    let total: i64 =
        sqlx::query_scalar("SELECT count(*) FROM tme.pending_player_kill_consequences")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        0, total,
        "an ordinary admission invented a deferred consequence"
    );
}

/// Synthetic marks are inserted with raw SQL, so they miss the anchoring the
/// real durable-effects path performs on insert. This reproduces the
/// scheduler's contract: active marks carry an expiry until an account holds
/// four of them, at which point the account is locked and they carry none.
/// The duplicate-sequence branch of the durable player-kill write was dark to
/// every test, which is how a stale `facet_id` predicate survived the D4 schema
/// cut inside it. A replay must agree with what is already stored; a replay
/// carrying different facts must be refused.
#[tokio::test]
#[ignore = "requires an EV runner-owned PostgreSQL database"]
async fn replayed_player_kill_assessment_agrees_and_a_contradicting_one_is_refused() {
    let database_url = std::env::var("TME_TEST_DATABASE_URL")
        .expect("the exact PostgreSQL runner must provide TME_TEST_DATABASE_URL");
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    truncate_all(&pool).await;

    let killer_account = wire::AccountId::new(Uuid::now_v7()).unwrap();
    let killer_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let replacement_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let victim_account = wire::AccountId::new(Uuid::now_v7()).unwrap();
    let victim_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let world_id = wire::FacetId::new(Uuid::now_v7()).unwrap();
    insert_account_named(&pool, killer_account, "replay_killer", "Replay Killer", 21).await;
    insert_account_named(&pool, victim_account, "replay_victim", "Replay Victim", 22).await;
    let state = PostgresState::open(
        &database_url,
        social_bootstrap(
            killer_account,
            killer_character,
            replacement_character,
            victim_account,
            victim_character,
            world_id,
        ),
    )
    .await
    .unwrap();
    open_sessions(
        &pool,
        killer_account,
        killer_character,
        victim_account,
        victim_character,
    )
    .await;
    drop(state);

    let store = PostgresStore::new(pool.clone());
    let assessment = |logical_time: u64| tme_rules::PlayerKillAssessmentV1 {
        facet_kill_sequence: 42,
        killer_character_id: tme_rules::CharacterId::new(killer_character.to_string()),
        victim_character_id: tme_rules::CharacterId::new(victim_character.to_string()),
        exempt_self_defense: false,
        consequence: tme_rules::PlayerKillConsequenceV1::AppliedHere {
            linked_karma_added: true,
        },
        logical_time: tme_rules::LogicalTime::new(logical_time),
    };

    commit_kill(&store, &pool, world_id, &assessment(9))
        .await
        .expect("the first durable write of a kill succeeds");
    let marks: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tme.player_kill_marks WHERE facet_kill_sequence=42",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(1, marks);
    // A present killer holds a live session when their kill is assessed, so the
    // victim can forgive immediately. This slice must not have changed that.
    assert!(
        forgivable(&pool, 42).await,
        "a present killer's mark stopped being forgivable"
    );

    // The replay. Before the fix this raised a SQL error on a column the D4 cut
    // had dropped, instead of comparing the stored facts.
    commit_kill(&store, &pool, world_id, &assessment(9))
        .await
        .expect("replaying an identical kill agrees with what is stored");
    let marks: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tme.player_kill_marks WHERE facet_kill_sequence=42",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(1, marks, "a replay must not create a second mark");

    // Same sequence, different assessed time: that is not a replay, it is a
    // contradiction, and it must be refused rather than silently ignored.
    let error = commit_kill(&store, &pool, world_id, &assessment(10))
        .await
        .expect_err("a contradicting replay must be refused");
    assert!(
        error.contains("conflicts with different durable facts"),
        "unexpected refusal: {error}"
    );
}
