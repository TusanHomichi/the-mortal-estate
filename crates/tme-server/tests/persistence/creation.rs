#[tokio::test]
#[ignore = "requires an EV runner-owned PostgreSQL database"]
async fn character_creation_is_atomic_idempotent_and_survives_restart() {
    let database_url = std::env::var("TME_TEST_DATABASE_URL").unwrap();
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    let account = wire::AccountId::new(Uuid::now_v7()).unwrap();
    let seeded = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let world = wire::FacetId::new(Uuid::now_v7()).unwrap();
    insert_account(&pool, account).await;
    let bootstrap = || {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/test-corpus/town_adventure_loop_gallery.json");
        PostgresBootstrap {
            world: PostgresWorldBootstrap { facet_id: world, key: "creation-proof".into(),
                engine: tme_sim::load_engine_from_scenario(&path, Some(7)).unwrap() },
            characters: vec![PostgresCharacterBootstrap { account_id: account, character_id: seeded, slot: 1,
                display_name: wire::DisplayName::new("Seeded").unwrap(), actor_id: tme_rules::ActorId::new("player") }],
        }
    };
    let first = PostgresState::open(&database_url, bootstrap()).await.unwrap();
    let login = first.login(IpAddr::V4(Ipv4Addr::LOCALHOST), wire::LoginRequestV1 {
        username: wire::Username::new(USERNAME).unwrap(), password: wire::Password::new(PASSWORD).unwrap(),
    }).await.unwrap();
    let token = login.session_token.expose().to_string();
    let options = first.character_creation_options(&token).await.unwrap();
    let option = &options.options[0];
    let request = wire::CharacterCreateRequestV1 {
        csrf_token: login.bootstrap.csrf_token.clone(), request_id: wire::CommandId::new(Uuid::now_v7()).unwrap(),
        draft: wire::CharacterCreationDraftV1 { profile_id: option.profile_id.clone(),
            display_name: wire::DisplayName::new("New Arrival").unwrap(), attributes: option.suggested.clone() },
    };
    let mut invalid = request.clone(); invalid.draft.attributes.strength = i32::MAX;
    assert_eq!(first.create_character(&token, invalid).await.unwrap_err(), tme_server::postgres::SessionError::CharacterCreationRefused);
    invalid = request.clone(); invalid.csrf_token = wire::CsrfToken::new("A".repeat(43)).unwrap();
    assert_eq!(first.create_character(&token, invalid).await.unwrap_err(), tme_server::postgres::SessionError::CsrfRejected);
    // The row insertion fails after the prepared rules candidate exists. Neither
    // the candidate nor its directory/receipt may leak from the failed transaction.
    sqlx::raw_sql("CREATE FUNCTION tme.refuse_creation() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected creation fault'; END $$; \
        CREATE TRIGGER refuse_creation BEFORE INSERT ON tme.character_creations FOR EACH ROW EXECUTE FUNCTION tme.refuse_creation();")
        .execute(&pool).await.unwrap();
    assert_eq!(first.create_character(&token, request.clone()).await.unwrap_err(), tme_server::postgres::SessionError::Unavailable);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM tme.characters").fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);
    sqlx::raw_sql("DROP TRIGGER refuse_creation ON tme.character_creations; DROP FUNCTION tme.refuse_creation();")
        .execute(&pool).await.unwrap();
    let (one, two) = tokio::join!(first.create_character(&token, request.clone()), first.create_character(&token, request.clone()));
    let one = one.unwrap(); let two = two.unwrap();
    assert_eq!(one.character, two.character);
    assert_ne!(one.replay_status, two.replay_status);
    assert_eq!(one.character.slot, 2);
    let mut conflicting = request.clone(); conflicting.draft.display_name = wire::DisplayName::new("Different").unwrap();
    assert_eq!(first.create_character(&token, conflicting).await.unwrap_err(), tme_server::postgres::SessionError::CharacterCreationConflict);
    // Hold the durable write, cancel the caller, then allow the operation to
    // finish. Losing the request future must not strand a prepared world.
    let mut blocker = pool.acquire().await.unwrap();
    sqlx::query("SELECT pg_advisory_lock(73318)").execute(&mut *blocker).await.unwrap();
    sqlx::raw_sql("CREATE FUNCTION tme.pause_creation() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock(73318); RETURN NEW; END $$; \
        CREATE TRIGGER pause_creation BEFORE INSERT ON tme.character_creations FOR EACH ROW EXECUTE FUNCTION tme.pause_creation();")
        .execute(&pool).await.unwrap();
    let mut cancelled_request = request.clone();
    cancelled_request.request_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    cancelled_request.draft.display_name = wire::DisplayName::new("Interrupted Arrival").unwrap();
    let state = first.clone(); let pending = cancelled_request.clone(); let pending_token = token.clone();
    let caller = tokio::spawn(async move { state.create_character(&pending_token, pending).await });
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND objid=73318 AND NOT granted)")
                .fetch_one(&pool).await.unwrap();
            if waiting { break; }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    }).await.unwrap();
    caller.abort(); assert!(caller.await.unwrap_err().is_cancelled());
    sqlx::query("SELECT pg_advisory_unlock(73318)").execute(&mut *blocker).await.unwrap();
    sqlx::raw_sql("DROP TRIGGER pause_creation ON tme.character_creations; DROP FUNCTION tme.pause_creation();")
        .execute(&pool).await.unwrap();
    let cancelled_replay = first.create_character(&token, cancelled_request).await.unwrap();
    assert_eq!(cancelled_replay.replay_status, wire::ReplayStatus::Replayed);
    assert_eq!(cancelled_replay.character.slot, 3);
    drop(first);
    let recovered = PostgresState::open(&database_url, bootstrap()).await.unwrap();
    let replay = recovered.create_character(&token, request).await.unwrap();
    assert_eq!(replay.character, one.character); assert_eq!(replay.replay_status, wire::ReplayStatus::Replayed);
    let session = recovered.session_bootstrap(&token).await.unwrap();
    assert_eq!(session.characters.len(), 3);
    recovered.select_character(&token, wire::CharacterSelectRequestV1 {
        csrf_token: session.csrf_token, character_id: one.character.character_id,
    }).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM tme.character_creations").fetch_one(&pool).await.unwrap();
    assert_eq!(count, 2);
}
