// Private social proof for the persistence integration target.
async fn two_client_social_socket_smoke(database_url: &str, pool: &sqlx::PgPool) {
    let first_account = wire::AccountId::new(Uuid::now_v7()).unwrap();
    let second_account = wire::AccountId::new(Uuid::now_v7()).unwrap();
    let first_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let replacement_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let second_character = wire::CharacterId::new(Uuid::now_v7()).unwrap();
    let shared_facet = wire::FacetId::new(Uuid::now_v7()).unwrap();
    insert_account_named(pool, first_account, "social_one", "Social One", 11).await;
    insert_account_named(pool, second_account, "social_two", "Social Two", 12).await;

    let backend = PostgresState::open(
        database_url,
        social_bootstrap(
            first_account,
            first_character,
            replacement_character,
            second_account,
            second_character,
            shared_facet,
        ),
    )
    .await
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let host = address.to_string();
    let origin = format!("http://{host}");
    let config = ServerConfig::new(address, host.clone(), origin.clone()).unwrap();
    let state = AppState::postgres(config, backend.clone());
    let (stop_send, stop_receive) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(tme_server::runtime::serve(listener, state, async {
        let _ = stop_receive.await;
    }));

    let (mut first, first_welcome, first_token, first_csrf) =
        login_select_connect_with_session(address, &host, &origin, "social_one", first_character)
            .await;
    let (mut second, second_welcome, second_token, second_csrf) =
        login_select_connect_with_session(address, &host, &origin, "social_two", second_character)
            .await;
    let (first_actor, first_epoch, first_revision) = welcome_parts(&first_welcome);
    let (second_actor, second_epoch, second_revision) = welcome_parts(&second_welcome);

    let preview_id = wire::PreviewId::new(Uuid::now_v7()).unwrap();
    let preview = send_socket_path_preview(
        &mut first,
        preview_id,
        first_actor.clone(),
        first_epoch,
        first_revision,
        vec![wire::Direction::North],
    )
    .await;
    assert!(matches!(
        preview,
        wire::ServerEnvelope::PathPreviewResult {
            disposition: wire::PathPreviewDisposition::Previewed,
            control_epoch,
            actor_id,
            preview: Some(_),
            ..
        } if control_epoch.get() == first_epoch && actor_id == first_actor
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM tme.command_receipts WHERE account_id=$1 AND command_id=$2",
        )
        .bind(first_account.as_uuid())
        .bind(preview_id.as_uuid())
        .fetch_one(pool)
        .await
        .unwrap(),
        0,
        "path preview must not create a durable receipt"
    );

    send_durable_socket_command(
        &mut first,
        first_actor.clone(),
        first_epoch,
        1,
        first_revision,
        wire::Intent::Invite {
            target_character_id: second_character,
        },
    )
    .await;
    let invitation_id = wait_for_invitation(&mut second).await;
    send_durable_socket_command(
        &mut second,
        second_actor.clone(),
        second_epoch,
        1,
        second_revision,
        wire::Intent::AcceptInvite { invitation_id },
    )
    .await;

    let group_message_id = wire::MessageId::new(Uuid::now_v7()).unwrap();
    let group_message = wire::ClientCommandEnvelope::SocialMessage {
        message_id: group_message_id,
        control_epoch: wire::DecimalU64::new(first_epoch),
        actor_id: first_actor.clone(),
        scope: wire::SocialScope::Group,
        body: wire::SocialBody::new("meet by the fountain").unwrap(),
    };
    first
        .send(Message::Text(
            serde_json::to_string(&group_message).unwrap().into(),
        ))
        .await
        .unwrap();
    assert_eq!(
        wait_for_message_result(&mut first, group_message_id).await,
        wire::MessageDisposition::Accepted
    );
    let delivered = wait_for_social_message(&mut second, group_message_id).await;
    assert!(matches!(delivered, wire::SocialScope::Group));

    first
        .send(Message::Text(
            serde_json::to_string(&group_message).unwrap().into(),
        ))
        .await
        .unwrap();
    assert_eq!(
        wait_for_message_result(&mut first, group_message_id).await,
        wire::MessageDisposition::Accepted
    );
    assert_no_social_replay(&mut second, group_message_id).await;

    send_durable_socket_command(
        &mut first,
        first_actor.clone(),
        first_epoch,
        2,
        first_revision,
        wire::Intent::TransferLeadership {
            member_character_id: second_character,
        },
    )
    .await;
    send_durable_socket_command(
        &mut first,
        first_actor.clone(),
        first_epoch,
        3,
        first_revision,
        wire::Intent::BeginFollow {
            target_character_id: second_character,
        },
    )
    .await;
    send_durable_socket_command(
        &mut second,
        second_actor.clone(),
        second_epoch,
        2,
        second_revision,
        wire::Intent::DisbandGroup,
    )
    .await;
    send_durable_socket_command(
        &mut first,
        first_actor.clone(),
        first_epoch,
        4,
        first_revision,
        wire::Intent::Block {
            target_character_id: second_character,
        },
    )
    .await;

    let page_id = wire::MessageId::new(Uuid::now_v7()).unwrap();
    second
        .send(Message::Text(
            serde_json::to_string(&wire::ClientCommandEnvelope::SocialMessage {
                message_id: page_id,
                control_epoch: wire::DecimalU64::new(second_epoch),
                actor_id: second_actor,
                scope: wire::SocialScope::Page {
                    target_character_id: first_character,
                },
                body: wire::SocialBody::new("can you hear me?").unwrap(),
            })
            .unwrap()
            .into(),
        ))
        .await
        .unwrap();
    assert_eq!(
        wait_for_message_result(&mut second, page_id).await,
        wire::MessageDisposition::Unavailable
    );

    let safe = send_socket_command_result(
        &mut first,
        first_actor.clone(),
        first_epoch,
        5,
        first_revision,
        wire::Intent::PhysicalAttack {
            mode: wire::PhysicalAttackMode::Fight,
            target_actor_id: wire::ActorId::new("player2").unwrap(),
            authorization: wire::HostilityAuthorization::Safe,
        },
    )
    .await;
    assert!(matches!(
        safe,
        wire::ServerEnvelope::CommandResult {
            disposition: wire::CommandDisposition::Rejected {
                code: wire::RejectionCode::RulesRejected
            },
            ..
        }
    ));
    let unsafe_result = send_socket_command_result(
        &mut first,
        first_actor.clone(),
        first_epoch,
        6,
        first_revision,
        wire::Intent::PhysicalAttack {
            mode: wire::PhysicalAttackMode::Fight,
            target_actor_id: wire::ActorId::new("player2").unwrap(),
            authorization: wire::HostilityAuthorization::ConfirmedUnsafe,
        },
    )
    .await;
    assert!(
        matches!(
            &unsafe_result,
            wire::ServerEnvelope::CommandResult {
                disposition: wire::CommandDisposition::Accepted,
                ..
            }
        ),
        "unexpected unsafe PvP result: {unsafe_result:?}"
    );
    let mark_uuid: Uuid = sqlx::query_scalar(
        "SELECT mark_id FROM tme.player_kill_marks WHERE killer_account_id=$1 \
         AND victim_account_id=$2 AND forgiven_at IS NULL AND expired_at IS NULL",
    )
    .bind(first_account.as_uuid())
    .bind(second_account.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    let linked: bool = sqlx::query_scalar(
        "SELECT linked_karma_added AND karma_forgiveness_eligible \
         FROM tme.player_kill_marks WHERE mark_id=$1",
    )
    .bind(mark_uuid)
    .fetch_one(pool)
    .await
    .unwrap();
    assert!(linked);
    let request = wire::ForgivePlayerKillMarkRequestV1 {
        request_id: wire::CommandId::new(Uuid::now_v7()).unwrap(),
    };
    let forgiveness = post_json_with_csrf(
        address,
        &host,
        &origin,
        &format!("/v4/player-kill-marks/{mark_uuid}/forgive"),
        &second_token,
        &second_csrf,
        &request,
    )
    .await;
    assert_eq!(
        forgiveness.status,
        200,
        "{}",
        String::from_utf8_lossy(&forgiveness.body)
    );
    let forgiven: wire::ForgivePlayerKillMarkResultV1 =
        serde_json::from_slice(&forgiveness.body).unwrap();
    assert_eq!(forgiven.control_api_version, wire::CONTROL_API_VERSION);
    assert_eq!(forgiven.replay_status, wire::ReplayStatus::New);
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT forgiven_at IS NOT NULL AND expired_at IS NULL \
             FROM tme.player_kill_marks WHERE mark_id=$1",
        )
        .bind(mark_uuid)
        .fetch_one(pool)
        .await
        .unwrap()
    );

    let exit_mark = exercise_player_kill_mark_schedule(
        pool,
        address,
        &host,
        &origin,
        first_account,
        first_character,
        &first_token,
        &first_csrf,
        second_account,
        second_character,
        &second_token,
        &second_csrf,
    )
    .await;

    // D4: the durable store holds exactly one world, so social scope can never
    // be a question of which copy a player happens to be looking at.
    let worlds: i64 = sqlx::query_scalar("SELECT count(*) FROM tme.facets")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(worlds, 1);

    let replacement = post_json(
        address,
        &host,
        &origin,
        "/v4/characters/select",
        &first_token,
        &wire::CharacterSelectRequestV1 {
            csrf_token: first_csrf.clone(),
            character_id: replacement_character,
        },
    )
    .await;
    assert_eq!(
        replacement.status,
        200,
        "{}",
        String::from_utf8_lossy(&replacement.body)
    );
    let replacement: wire::CharacterSelectionV1 =
        serde_json::from_slice(&replacement.body).unwrap();
    assert_eq!(replacement.control_api_version, wire::CONTROL_API_VERSION);
    assert_eq!(replacement.character.character_id, replacement_character);
    assert_eq!(
        wait_for_draining(&mut first).await,
        (wire::DrainingReason::SessionEnded, false)
    );
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT karma_forgiveness_eligible FROM tme.player_kill_marks WHERE mark_id=$1",
        )
        .bind(exit_mark)
        .fetch_one(pool)
        .await
        .unwrap(),
        "character replacement closes the linked-karma forgiveness window"
    );
    tme_server::operator::verify_store(pool).await.unwrap();
    let corrupt_mark_id = Uuid::now_v7();
    sqlx::query("UPDATE tme.player_kill_marks SET mark_id=$2 WHERE mark_id=$1")
        .bind(exit_mark)
        .bind(corrupt_mark_id)
        .execute(pool)
        .await
        .unwrap();
    assert!(
        tme_server::operator::verify_store(pool).await.is_err(),
        "store verification accepted a mark whose ID contradicted its durable identity"
    );
    sqlx::query("UPDATE tme.player_kill_marks SET mark_id=$2 WHERE mark_id=$1")
        .bind(corrupt_mark_id)
        .bind(exit_mark)
        .execute(pool)
        .await
        .unwrap();
    tme_server::operator::verify_store(pool).await.unwrap();
    second.close(None).await.unwrap();
    let _ = stop_send.send(());
    server.await.unwrap().unwrap();
    drop(backend);
    tokio::time::sleep(Duration::from_millis(250)).await;
}
