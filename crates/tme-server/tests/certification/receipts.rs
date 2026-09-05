// Private receipts evidence for the EV certification target.
fn runner_private_temp_root() -> PathBuf {
    let root = PathBuf::from(
        std::env::var_os("TME_EV_PRIVATE_TEMP_ROOT")
            .expect("the EV runner must provide TME_EV_PRIVATE_TEMP_ROOT"),
    );
    assert!(
        root.is_absolute(),
        "TME_EV_PRIVATE_TEMP_ROOT must be absolute"
    );
    let metadata = fs::symlink_metadata(&root).expect("inspect EV private temp root");
    assert!(
        metadata.file_type().is_dir(),
        "EV private temp root must be a non-link directory"
    );
    assert_eq!(
        root,
        fs::canonicalize(&root).expect("canonicalize EV private temp root"),
        "EV private temp root must already be canonical"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let process_uid = fs::metadata("/proc/self")
            .expect("inspect current EV process owner")
            .uid();
        assert_eq!(
            process_uid,
            metadata.uid(),
            "EV private temp root must be owned by the runner user"
        );
        assert_eq!(
            0o700,
            metadata.permissions().mode() & 0o777,
            "EV private temp root must have mode 0700"
        );
    }
    root
}

fn durable_command_outcome(
    envelope: wire::ServerEnvelope,
    expected_replay_status: wire::ReplayStatus,
) -> DurableCommandOutcome {
    match envelope {
        wire::ServerEnvelope::CommandResult {
            command_id,
            disposition,
            replay_status,
            server_sequence,
            before_revision,
            after_revision,
            events,
            events_truncated,
        } => {
            assert_eq!(expected_replay_status, replay_status);
            DurableCommandOutcome {
                command_id,
                disposition,
                server_sequence,
                before_revision,
                after_revision,
                events,
                events_truncated,
            }
        }
        other => panic!("expected durable command result, got {other:?}"),
    }
}

async fn receipt_identity(
    pool: &sqlx::PgPool,
    account_id: wire::AccountId,
    command_id: wire::CommandId,
) -> String {
    sqlx::query_scalar(
        "SELECT to_jsonb(receipt)::text FROM tme.command_receipts receipt \
         WHERE account_id=$1 AND command_id=$2",
    )
    .bind(account_id.as_uuid())
    .bind(command_id.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn persisted_receipt_envelope(
    pool: &sqlx::PgPool,
    account_id: wire::AccountId,
    command_id: wire::CommandId,
    replay_status: wire::ReplayStatus,
) -> wire::ServerEnvelope {
    let bytes: Vec<u8> = sqlx::query_scalar(
        "SELECT outcome_bytes FROM tme.command_receipts \
         WHERE account_id=$1 AND command_id=$2",
    )
    .bind(account_id.as_uuid())
    .bind(command_id.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    let outcome: tme_server::store::receipt::ReceiptOutcomeV3 =
        serde_json::from_slice(&bytes).expect("persisted ReceiptOutcomeV3 decodes");
    assert_eq!(
        bytes,
        outcome
            .encode()
            .expect("persisted ReceiptOutcomeV3 encodes"),
        "persisted receipt outcome is not canonical"
    );
    outcome
        .to_envelope(command_id, replay_status)
        .expect("persisted ReceiptOutcomeV3 projects to protocol")
}

async fn wait_for_receipt(pool: &sqlx::PgPool, command_id: wire::CommandId) {
    let mut watchdog = WallClockWatchdog::start(Duration::from_secs(10));
    let observe = async {
        loop {
            let count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM tme.command_receipts WHERE command_id=$1")
                    .bind(command_id.as_uuid())
                    .fetch_one(pool)
                    .await
                    .unwrap();
            if count == 1 {
                return;
            }
            wall_clock_delay(Duration::from_millis(10)).await;
        }
    };
    tokio::select! {
        () = observe => {}
        expired = &mut watchdog.expired => {
            expired.expect("receipt watchdog sender");
            panic!("disconnect-before-reply command did not commit in ten wall seconds");
        }
    }
}

async fn facet_identity(pool: &sqlx::PgPool, facet_id: wire::FacetId) -> String {
    let identity: String =
        sqlx::query_scalar("SELECT to_jsonb(facet)::text FROM tme.facets facet WHERE facet_id=$1")
            .bind(facet_id.as_uuid())
            .fetch_one(pool)
            .await
            .unwrap();
    format!("{:x}", Sha256::digest(identity.as_bytes()))
}

async fn facet_durable_state(pool: &sqlx::PgPool, facet_id: wire::FacetId) -> FacetDurableState {
    let row = sqlx::query(
        "SELECT f.facet_revision,f.last_server_sequence,f.checkpoint_bytes,f.checkpoint_sha256, \
         coalesce((SELECT max(a.audit_id) FROM tme.audit_events a),0) AS max_audit_id \
         FROM tme.facets f WHERE f.facet_id=$1",
    )
    .bind(facet_id.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    let checkpoint: Vec<u8> = row.get("checkpoint_bytes");
    let checkpoint_sha256: Vec<u8> = row.get("checkpoint_sha256");
    assert_eq!(Sha256::digest(&checkpoint).as_slice(), checkpoint_sha256);
    let payload: serde_json::Value = serde_json::from_slice(&checkpoint).unwrap();
    let world = payload["world"].as_object().expect("checkpoint world");
    let logical_time = world["timing"]["now"]["milliseconds"]
        .as_u64()
        .expect("checkpoint logical time");
    let presence = world["character_presence"]
        .as_object()
        .expect("checkpoint character presence")
        .iter()
        .map(|(character_id, value)| {
            (
                character_id.clone(),
                (
                    value["connected"]
                        .as_bool()
                        .expect("checkpoint connected presence"),
                    value["absent_since"]["milliseconds"].as_u64(),
                ),
            )
        })
        .collect();
    let pages_enabled = world["communication_preferences"]
        .as_object()
        .expect("checkpoint communication preferences")
        .iter()
        .map(|(character_id, value)| {
            (
                character_id.clone(),
                value["pages_enabled"]
                    .as_bool()
                    .expect("checkpoint pages preference"),
            )
        })
        .collect();
    FacetDurableState {
        revision: row.get("facet_revision"),
        sequence: row.get("last_server_sequence"),
        checkpoint_sha256,
        logical_time,
        presence,
        pages_enabled,
        max_audit_id: row.get("max_audit_id"),
    }
}

async fn wait_for_durable_disconnect(
    pool: &sqlx::PgPool,
    facet_id: wire::FacetId,
    character_id: wire::CharacterId,
    after_audit_id: i64,
) -> FacetDurableState {
    let character_id = character_id.to_string();
    let mut watchdog = WallClockWatchdog::start(Duration::from_secs(10));
    let observe = async {
        loop {
            let state = facet_durable_state(pool, facet_id).await;
            if state.max_audit_id > after_audit_id
                && state
                    .presence
                    .get(&character_id)
                    .is_some_and(|(connected, _)| !connected)
            {
                return state;
            }
            wall_clock_delay(Duration::from_millis(10)).await;
        }
    };
    tokio::select! {
        state = observe => state,
        expired = &mut watchdog.expired => {
            expired.expect("durable-disconnect watchdog sender");
            panic!("protocol-error disconnect did not commit in ten wall seconds");
        }
    }
}

async fn facet_audits_between(
    pool: &sqlx::PgPool,
    after_audit_id: i64,
    through_audit_id: i64,
) -> BTreeMap<String, i64> {
    sqlx::query(
        "SELECT action,count(*) AS count FROM tme.audit_events \
         WHERE audit_id>$1 AND audit_id<=$2 \
         GROUP BY action ORDER BY action",
    )
    .bind(after_audit_id)
    .bind(through_audit_id)
    .fetch_all(pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.get("action"), row.get("count")))
    .collect()
}

fn assert_audit_accounted_transition(
    before: &FacetDurableState,
    after: &FacetDurableState,
    audits: &BTreeMap<String, i64>,
) {
    let revision_delta = after
        .revision
        .checked_sub(before.revision)
        .expect("facet revision did not move backward");
    let sequence_delta = after
        .sequence
        .checked_sub(before.sequence)
        .expect("facet sequence did not move backward");
    let audit_total: i64 = audits.values().sum();
    assert_eq!(revision_delta, sequence_delta);
    assert_eq!(revision_delta, audit_total);
}

async fn wait_for_message_result(client: &mut support::Client, expected: wire::MessageId) {
    loop {
        let envelope = support::receive_envelope(&mut client.socket).await;
        client.apply(&envelope);
        if let wire::ServerEnvelope::MessageResult {
            message_id,
            disposition,
        } = envelope
            && message_id == expected
        {
            assert_eq!(wire::MessageDisposition::Accepted, disposition);
            return;
        }
    }
}

async fn wait_for_social_message(client: &mut support::Client, expected: wire::MessageId) {
    loop {
        let envelope = support::receive_envelope(&mut client.socket).await;
        client.apply(&envelope);
        if let wire::ServerEnvelope::SocialMessage { message_id, .. } = envelope
            && message_id == expected
        {
            return;
        }
    }
}

async fn assert_no_social_replay(socket: &mut support::Socket, forbidden: wire::MessageId) {
    let mut watchdog = WallClockWatchdog::start(Duration::from_millis(250));
    loop {
        tokio::select! {
            message = socket.next() => match message {
                Some(Ok(tokio_tungstenite::tungstenite::protocol::Message::Text(text))) => {
                    let envelope: wire::ServerEnvelope = serde_json::from_str(&text).unwrap();
                    assert!(
                        !matches!(envelope, wire::ServerEnvelope::SocialMessage { message_id, .. } if message_id == forbidden),
                        "transient social message replayed after process restart"
                    );
                }
                Some(Ok(tokio_tungstenite::tungstenite::protocol::Message::Ping(_)))
                | Some(Ok(tokio_tungstenite::tungstenite::protocol::Message::Pong(_))) => {}
                Some(Ok(message)) => panic!("unexpected restarted WebSocket message: {message:?}"),
                Some(Err(error)) => panic!("restarted WebSocket failed: {error}"),
                None => panic!("restarted WebSocket closed while checking transient replay"),
            },
            expired = &mut watchdog.expired => {
                expired.expect("wall-clock watchdog sender");
                break;
            }
        }
    }
}
