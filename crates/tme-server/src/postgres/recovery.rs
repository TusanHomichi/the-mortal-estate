use super::*;

pub(super) async fn runtime_pool(database_url: &str) -> Result<PgPool, String> {
    let options = PgConnectOptions::from_str(database_url).map_err(|error| error.to_string())?;
    PgPoolOptions::new()
        .max_connections(16)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(60))
        .max_lifetime(Duration::from_secs(30 * 60))
        .after_connect(|connection, _| Box::pin(async move {
            connection.execute("SET statement_timeout='30s'; SET lock_timeout='5s'; SET idle_in_transaction_session_timeout='30s'").await?;
            Ok(())
        }))
        .connect_with(options)
        .await
        .map_err(|error| error.to_string())
}

pub(super) async fn auth_pool(database_url: &str) -> Result<PgPool, String> {
    PgPoolOptions::new()
        .max_connections(2)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(60))
        .after_connect(|connection, _| {
            Box::pin(async move {
                connection.execute("SET statement_timeout='30s'; SET lock_timeout='5s'; SET idle_in_transaction_session_timeout='30s'").await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await
        .map_err(|error| error.to_string())
}

pub(super) async fn recover_or_initialize(
    store: &SharedStore,
    bootstrap: PostgresBootstrap,
) -> Result<(wire::FacetId, String, Engine, u64, u64), String> {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM tme.facets")
        .fetch_one(store.pool())
        .await
        .map_err(|error| error.to_string())?;
    let PostgresWorldBootstrap {
        facet_id,
        key,
        mut engine,
    } = bootstrap.world;
    if count == 0 {
        let mut tx = serializable(store.pool()).await?;
        for character in &bootstrap.characters {
            engine = engine
                .prepare_character_id_rekey(
                    &character.actor_id,
                    CharacterId::new(character.character_id.to_string()),
                )
                .map_err(|error| error.to_string())?;
        }
        validate_directory(&engine, &bootstrap.characters)?;
        let identity = engine.definition().content_identity();
        let checkpoint = engine
            .export_checkpoint()
            .map_err(|error| error.to_string())?;
        sqlx::query("INSERT INTO tme.facets (facet_id,facet_key,catalog_id,profile_id,template_id,content_digest,checkpoint_schema,checkpoint_bytes,checkpoint_sha256) VALUES ($1,$2,$3,$4,$5,$6,3,$7,$8)")
            .bind(facet_id.as_uuid()).bind(&key).bind(&identity.catalog_id).bind(&identity.catalog_profile).bind(&identity.world_template_id)
            .bind(identity_digest(identity)?.as_slice()).bind(checkpoint.as_bytes()).bind(checkpoint.sha256().as_slice())
            .execute(&mut *tx).await.map_err(|error| error.to_string())?;
        for character in &bootstrap.characters {
            sqlx::query("INSERT INTO tme.characters (character_id,account_id,slot,display_name,actor_id) VALUES ($1,$2,$3,$4,$5)")
                .bind(character.character_id.as_uuid()).bind(character.account_id.as_uuid())
                .bind(i16::from(character.slot)).bind(character.display_name.as_str()).bind(character.actor_id.as_str())
                .execute(&mut *tx).await.map_err(|error| error.to_string())?;
        }
        tx.commit().await.map_err(|error| error.to_string())?;
    }
    verify_character_assertions(store.pool(), &bootstrap.characters).await?;
    let rows = sqlx::query("SELECT facet_id,facet_key,catalog_id,profile_id,template_id,content_digest,checkpoint_schema,facet_revision,last_server_sequence,checkpoint_bytes,checkpoint_sha256 FROM tme.facets")
        .fetch_all(store.pool()).await.map_err(|error| error.to_string())?;
    // D4: this process hosts exactly one world. A second durable row is a
    // divergent copy and must fail closed rather than be silently selected.
    let [row] = rows.as_slice() else {
        return Err("the durable store must hold exactly one world".to_string());
    };
    let durable_id =
        wire::FacetId::new(row.try_get("facet_id").map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    if durable_id != facet_id
        || row
            .try_get::<String, _>("facet_key")
            .map_err(|error| error.to_string())?
            != key
        || row
            .try_get::<i16, _>("checkpoint_schema")
            .map_err(|error| error.to_string())?
            != 3
    {
        return Err("the durable world identity differs from bootstrap".to_string());
    }
    let bytes: Vec<u8> = row
        .try_get("checkpoint_bytes")
        .map_err(|error| error.to_string())?;
    let sha: Vec<u8> = row
        .try_get("checkpoint_sha256")
        .map_err(|error| error.to_string())?;
    let checkpoint = FacetCheckpointV5::from_bytes(bytes).map_err(|error| error.to_string())?;
    if checkpoint.sha256().as_slice() != sha.as_slice() {
        return Err("durable checkpoint hash mismatch".to_string());
    }
    let identity = engine.definition().content_identity();
    if row
        .try_get::<String, _>("catalog_id")
        .map_err(|error| error.to_string())?
        != identity.catalog_id
        || row
            .try_get::<String, _>("profile_id")
            .map_err(|error| error.to_string())?
            != identity.catalog_profile
        || row
            .try_get::<String, _>("template_id")
            .map_err(|error| error.to_string())?
            != identity.world_template_id
        || row
            .try_get::<Vec<u8>, _>("content_digest")
            .map_err(|error| error.to_string())?
            .as_slice()
            != identity_digest(identity)?.as_slice()
    {
        return Err("durable world content identity mismatch".to_string());
    }
    let hydrated = Engine::hydrate_checkpoint(engine.definition().clone(), &checkpoint)
        .map_err(|error| error.to_string())?;
    let revision = checked_u64(
        row.try_get("facet_revision")
            .map_err(|error| error.to_string())?,
    )?;
    let sequence = checked_u64(
        row.try_get("last_server_sequence")
            .map_err(|error| error.to_string())?,
    )?;
    verify_loaded_directory(store.pool(), &hydrated).await?;
    Ok((facet_id, key, hydrated, revision, sequence))
}

pub(super) async fn verify_character_assertions(
    pool: &PgPool,
    expected: &[PostgresCharacterBootstrap],
) -> Result<(), String> {
    let configured = expected
        .iter()
        .map(|value| {
            (
                value.character_id.as_uuid(),
                value.account_id.as_uuid(),
                i16::from(value.slot),
                value.display_name.as_str().to_string(),
                value.actor_id.as_str().to_string(),
            )
        })
        .collect::<BTreeSet<_>>();
    let rows = sqlx::query(
        "SELECT character_id,account_id,slot,display_name,actor_id \
         FROM tme.characters ORDER BY character_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    let durable = rows
        .into_iter()
        .map(|row| {
            Ok((
                row.try_get::<Uuid, _>("character_id")?,
                row.try_get::<Uuid, _>("account_id")?,
                row.try_get::<i16, _>("slot")?,
                row.try_get::<String, _>("display_name")?,
                row.try_get::<String, _>("actor_id")?,
            ))
        })
        .collect::<Result<BTreeSet<_>, sqlx::Error>>()
        .map_err(|error| error.to_string())?;
    if configured != durable {
        return Err("durable character directory differs from bootstrap assertions".to_string());
    }
    Ok(())
}

pub(super) fn identity_digest(value: &tme_rules::ContentIdentityV1) -> Result<[u8; 32], String> {
    let text = value.definition_sha256.as_bytes();
    if text.len() != 64 {
        return Err("content identity digest is not 64 hexadecimal bytes".to_string());
    }
    let mut digest = [0_u8; 32];
    for (index, pair) in text.as_chunks::<2>().0.iter().enumerate() {
        digest[index] = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Ok(digest)
}

pub(super) fn hex_nibble(value: u8) -> Result<u8, String> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err("content identity digest is not lowercase hexadecimal".to_string()),
    }
}

pub(crate) fn validate_bootstrap(value: &PostgresBootstrap) -> Result<(), String> {
    validate_ascii_key(&value.world.key)?;
    let mut character_ids = BTreeSet::new();
    let mut slots = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut actors = BTreeSet::new();
    for character in &value.characters {
        if !character_ids.insert(character.character_id)
            || !(1..=8).contains(&character.slot)
            || !slots.insert((character.account_id, character.slot))
            || !names.insert((character.account_id, character.display_name.as_str()))
            || !actors.insert(character.actor_id.clone())
        {
            return Err("character bootstrap directory is invalid".to_string());
        }
    }
    Ok(())
}

pub(super) fn validate_directory(
    engine: &Engine,
    characters: &[PostgresCharacterBootstrap],
) -> Result<(), String> {
    let actual = engine
        .world()
        .controlled_actors()
        .map(|actor| actor.id.clone())
        .collect::<BTreeSet<_>>();
    let expected = characters
        .iter()
        .map(|character| character.actor_id.clone())
        .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(
            "every controlled actor must have exactly one durable directory owner".to_string(),
        );
    }
    Ok(())
}

pub(super) async fn verify_loaded_directory(pool: &PgPool, engine: &Engine) -> Result<(), String> {
    let rows = sqlx::query("SELECT actor_id FROM tme.characters ORDER BY actor_id")
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
    let mut durable = BTreeSet::<ActorId>::new();
    for row in rows {
        durable.insert(ActorId::new(
            row.try_get::<String, _>("actor_id")
                .map_err(|error| error.to_string())?,
        ));
    }
    let actual = engine
        .world()
        .controlled_actors()
        .map(|actor| actor.id.clone())
        .collect::<BTreeSet<_>>();
    if durable != actual {
        return Err("durable character directory differs from checkpoint ownership".to_string());
    }
    Ok(())
}

pub(super) async fn active_session(
    tx: &mut Transaction<'_, Postgres>,
    cookie: &str,
    refresh: bool,
) -> Result<Option<SessionRow>, SessionError> {
    let row=sqlx::query("SELECT session_id,account_id,csrf_digest,selected_character_id FROM tme.sessions WHERE token_digest=$1 AND revoked_at IS NULL AND idle_expires_at>statement_timestamp() AND absolute_expires_at>statement_timestamp() FOR UPDATE")
        .bind(digest(cookie).as_slice()).fetch_optional(&mut **tx).await.map_err(unavailable)?;
    let Some(row) = row else { return Ok(None) };
    let session = decode_session(row)?;
    if refresh {
        sqlx::query("UPDATE tme.sessions SET last_seen_at=statement_timestamp(),idle_expires_at=LEAST(absolute_expires_at,statement_timestamp()+make_interval(secs=>$2)) WHERE session_id=$1")
        .bind(session.session_id.as_uuid()).bind(checked_i64(SESSION_IDLE.as_secs()).map_err(unavailable)?).execute(&mut **tx).await.map_err(unavailable)?;
    }
    Ok(Some(session))
}

pub(super) async fn character_for_account(
    tx: &mut Transaction<'_, Postgres>,
    id: wire::CharacterId,
    account: wire::AccountId,
) -> Result<Option<CharacterRow>, SessionError> {
    sqlx::query("SELECT character_id,account_id,slot,display_name,actor_id,control_epoch FROM tme.characters WHERE character_id=$1 AND account_id=$2 FOR UPDATE")
        .bind(id.as_uuid()).bind(account.as_uuid()).fetch_optional(&mut **tx).await.map_err(unavailable)?.map(decode_character).transpose()
}

pub(super) fn decode_session(row: sqlx::postgres::PgRow) -> Result<SessionRow, SessionError> {
    let csrf: Vec<u8> = row.try_get("csrf_digest").map_err(unavailable)?;
    Ok(SessionRow {
        session_id: wire::SessionId::new(row.try_get("session_id").map_err(unavailable)?)
            .map_err(|_| SessionError::Unavailable)?,
        account_id: wire::AccountId::new(row.try_get("account_id").map_err(unavailable)?)
            .map_err(|_| SessionError::Unavailable)?,
        csrf_digest: csrf.try_into().map_err(|_| SessionError::Unavailable)?,
        selected_character_id: row
            .try_get::<Option<Uuid>, _>("selected_character_id")
            .map_err(unavailable)?
            .map(wire::CharacterId::new)
            .transpose()
            .map_err(|_| SessionError::Unavailable)?,
    })
}

pub(super) fn decode_character(row: sqlx::postgres::PgRow) -> Result<CharacterRow, SessionError> {
    let slot: i16 = row.try_get("slot").map_err(unavailable)?;
    Ok(CharacterRow {
        character_id: wire::CharacterId::new(row.try_get("character_id").map_err(unavailable)?)
            .map_err(|_| SessionError::Unavailable)?,
        slot: u8::try_from(slot).map_err(|_| SessionError::Unavailable)?,
        display_name: wire::DisplayName::new(
            row.try_get::<String, _>("display_name")
                .map_err(unavailable)?,
        )
        .map_err(|_| SessionError::Unavailable)?,
        actor_id: ActorId::new(row.try_get::<String, _>("actor_id").map_err(unavailable)?),
        control_epoch: checked_u64(row.try_get("control_epoch").map_err(unavailable)?)
            .map_err(unavailable)?,
    })
}

pub(super) fn selection(character: &CharacterRow) -> wire::CharacterSelectionV1 {
    wire::CharacterSelectionV1 {
        control_api_version: wire::CONTROL_API_VERSION,
        character: character_summary(character),
    }
}
pub(super) fn character_summary(character: &CharacterRow) -> wire::CharacterSummaryV1 {
    wire::CharacterSummaryV1 {
        character_id: character.character_id,
        slot: character.slot,
        display_name: character.display_name.clone(),
    }
}
pub(super) fn validate_csrf(value: [u8; 32], token: &wire::CsrfToken) -> Result<(), SessionError> {
    if digest(token.expose_for_validation()) == value {
        Ok(())
    } else {
        Err(SessionError::CsrfRejected)
    }
}
pub(super) fn digest(value: &str) -> [u8; 32] {
    Sha256::digest(value.as_bytes()).into()
}
pub(super) fn random_secret() -> Result<OpaqueSecret, String> {
    Ok(OpaqueSecret(
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes::<32>()?),
    ))
}
pub(super) fn random_csrf() -> Result<wire::CsrfToken, String> {
    wire::CsrfToken::new(
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes::<32>()?),
    )
    .map_err(|error| error.to_string())
}
pub(super) fn random_ticket() -> Result<wire::AdmissionTicket, String> {
    wire::AdmissionTicket::new(
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes::<32>()?),
    )
    .map_err(|error| error.to_string())
}
pub(super) fn validate_ascii_key(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 64
        || !value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
    {
        Err("key must contain 1-64 printable ASCII bytes".to_string())
    } else {
        Ok(())
    }
}

pub(super) fn unavailable(_: impl std::fmt::Display) -> SessionError {
    SessionError::Unavailable
}
