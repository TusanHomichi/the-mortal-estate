// Private fixtures proof for the persistence integration target.
async fn insert_account(pool: &sqlx::PgPool, account_id: wire::AccountId) {
    insert_account_named(pool, account_id, USERNAME, "Durable Tester", 7).await;
}

async fn insert_account_named(
    pool: &sqlx::PgPool,
    account_id: wire::AccountId,
    username: &str,
    display_name: &str,
    salt_byte: u8,
) {
    let params = Params::new(65_536, 3, 4, Some(32)).unwrap();
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let salt = SaltString::encode_b64(&[salt_byte; 16]).unwrap();
    let phc = argon
        .hash_password(PASSWORD.as_bytes(), &salt)
        .unwrap()
        .to_string();
    sqlx::query("INSERT INTO tme.accounts(account_id,username,display_name) VALUES($1,$2,$3)")
        .bind(account_id.as_uuid())
        .bind(username)
        .bind(display_name)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tme.account_credentials(account_id,password_phc) VALUES($1,$2)")
        .bind(account_id.as_uuid())
        .bind(phc)
        .execute(pool)
        .await
        .unwrap();
}

fn bootstrap(
    account_id: wire::AccountId,
    character_id: wire::CharacterId,
    world_id: wire::FacetId,
) -> PostgresBootstrap {
    let engine = scenario_engine();
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .realm,
        "realm_0"
    );
    PostgresBootstrap {
        world: PostgresWorldBootstrap {
            facet_id: world_id,
            key: "world".to_string(),
            engine,
        },
        characters: vec![PostgresCharacterBootstrap {
            account_id,
            character_id,
            slot: 1,
            display_name: wire::DisplayName::new("Wayfarer").unwrap(),
            actor_id: tme_rules::ActorId::from("player"),
        }],
    }
}
