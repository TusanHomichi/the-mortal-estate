// Private socket commands proof for the persistence integration target.
async fn login_select_connect_with_session(
    address: SocketAddr,
    host: &str,
    origin: &str,
    username: &str,
    character_id: wire::CharacterId,
) -> (Socket, wire::ServerEnvelope, String, wire::CsrfToken) {
    let login = http_request(
        address,
        host,
        origin,
        "POST",
        "/v4/login",
        None,
        Some(
            &serde_json::to_string(&wire::LoginRequestV1 {
                username: wire::Username::new(username).unwrap(),
                password: wire::Password::new(PASSWORD).unwrap(),
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(login.status, 200);
    assert!(!login.headers.contains_key("set-cookie"));
    let login: wire::LoginResponseV1 = serde_json::from_slice(&login.body).unwrap();
    let token = login.session_token.expose_for_validation().to_string();
    let bootstrap = login.bootstrap;
    assert_eq!(bootstrap.control_api_version, wire::CONTROL_API_VERSION);
    let selection = post_json(
        address,
        host,
        origin,
        "/v4/characters/select",
        &token,
        &wire::CharacterSelectRequestV1 {
            csrf_token: bootstrap.csrf_token.clone(),
            character_id,
        },
    )
    .await;
    assert_eq!(selection.status, 200);
    let selection: wire::CharacterSelectionV1 = serde_json::from_slice(&selection.body).unwrap();
    assert_eq!(selection.control_api_version, wire::CONTROL_API_VERSION);
    let ticket = issue_ticket(address, host, origin, &token, &bootstrap.csrf_token).await;
    let (socket, welcome) = connect_character(address, host, origin, &ticket.ticket).await;
    (socket, welcome, token, bootstrap.csrf_token)
}

#[allow(clippy::too_many_arguments)]
async fn send_durable_socket_command(
    socket: &mut Socket,
    actor_id: wire::ActorId,
    control_epoch: u64,
    client_sequence: u64,
    observed_facet_revision: u64,
    intent: wire::Intent,
) {
    let result = send_socket_command_result(
        socket,
        actor_id,
        control_epoch,
        client_sequence,
        observed_facet_revision,
        intent,
    )
    .await;
    assert!(matches!(
        result,
        wire::ServerEnvelope::CommandResult {
            disposition: wire::CommandDisposition::Accepted,
            replay_status: wire::ReplayStatus::New,
            ..
        }
    ));
}

#[allow(clippy::too_many_arguments)]
async fn send_socket_command_result(
    socket: &mut Socket,
    actor_id: wire::ActorId,
    control_epoch: u64,
    client_sequence: u64,
    observed_facet_revision: u64,
    intent: wire::Intent,
) -> wire::ServerEnvelope {
    let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    socket
        .send(Message::Text(
            serde_json::to_string(&wire::ClientCommandEnvelope::Command {
                command_id,
                control_epoch: wire::DecimalU64::new(control_epoch),
                client_sequence: wire::DecimalU64::new(client_sequence),
                observed_world_revision: wire::DecimalU64::new(observed_facet_revision),
                actor_id,
                intent,
            })
            .unwrap()
            .into(),
        ))
        .await
        .unwrap();
    loop {
        let envelope = receive_envelope(socket).await;
        if matches!(
            &envelope,
            wire::ServerEnvelope::CommandResult {
                command_id: received,
                ..
            } if *received == command_id
        ) {
            return envelope;
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn send_socket_path_preview(
    socket: &mut Socket,
    preview_id: wire::PreviewId,
    actor_id: wire::ActorId,
    control_epoch: u64,
    observed_facet_revision: u64,
    path: Vec<wire::Direction>,
) -> wire::ServerEnvelope {
    socket
        .send(Message::Text(
            serde_json::to_string(&wire::ClientCommandEnvelope::PathPreview {
                preview_id,
                control_epoch: wire::DecimalU64::new(control_epoch),
                observed_world_revision: wire::DecimalU64::new(observed_facet_revision),
                actor_id,
                path,
            })
            .unwrap()
            .into(),
        ))
        .await
        .unwrap();
    loop {
        let envelope = receive_envelope(socket).await;
        if matches!(
            &envelope,
            wire::ServerEnvelope::PathPreviewResult {
                preview_id: received,
                ..
            } if *received == preview_id
        ) {
            return envelope;
        }
    }
}

async fn wait_for_invitation(socket: &mut Socket) -> wire::DecimalU64 {
    loop {
        if let wire::ServerEnvelope::StateUpdate { frame, .. } = receive_envelope(socket).await
            && let Some(invitation) = frame.social.incoming_invitations.first()
        {
            return invitation.invitation_id;
        }
    }
}

async fn wait_for_message_result(
    socket: &mut Socket,
    expected: wire::MessageId,
) -> wire::MessageDisposition {
    loop {
        if let wire::ServerEnvelope::MessageResult {
            message_id,
            disposition,
        } = receive_envelope(socket).await
            && message_id == expected
        {
            return disposition;
        }
    }
}

async fn wait_for_social_message(
    socket: &mut Socket,
    expected: wire::MessageId,
) -> wire::SocialScope {
    loop {
        if let wire::ServerEnvelope::SocialMessage {
            message_id, scope, ..
        } = receive_envelope(socket).await
            && message_id == expected
        {
            return scope;
        }
    }
}

async fn assert_no_social_replay(socket: &mut Socket, message_id: wire::MessageId) {
    let result = tokio::time::timeout(Duration::from_millis(300), async {
        loop {
            if let wire::ServerEnvelope::SocialMessage {
                message_id: received,
                ..
            } = receive_envelope(socket).await
                && received == message_id
            {
                return;
            }
        }
    })
    .await;
    assert!(result.is_err(), "exact message retry was redelivered");
}
