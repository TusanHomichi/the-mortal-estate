// Private http lifecycle proof for the persistence integration target.
async fn http_reconnect_replay_and_logout(
    backend: std::sync::Arc<PostgresState>,
    character_id: wire::CharacterId,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind HTTP integration listener");
    let address = listener.local_addr().expect("HTTP integration address");
    let host = address.to_string();
    let origin = format!("http://{host}");
    let config = ServerConfig::new(address, host.clone(), origin.clone()).expect("server config");
    let state = AppState::postgres(config, backend);
    let (stop_send, stop_receive) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(tme_server::runtime::serve(listener, state, async {
        let _ = stop_receive.await;
    }));

    let ready = http_request(address, &host, &origin, "GET", "/health/ready", None, None).await;
    assert_eq!(ready.status, 200);

    for old_root in [concat!("/v", "2"), concat!("/v", "3")] {
        let removed_routes = [
            ("POST", format!("{old_root}/login")),
            ("GET", format!("{old_root}/session")),
            ("POST", format!("{old_root}/logout")),
            ("POST", format!("{old_root}/characters/select")),
            ("POST", format!("{old_root}/socket-tickets")),
            ("POST", format!("{old_root}/characters/switch-facet")),
            // D4: the world selector is retired at the current root too, not just
            // behind the predecessor's version prefix.
            ("POST", "/v4/characters/switch-facet".to_string()),
            ("GET", "/v4/facets".to_string()),
            (
                "POST",
                format!("{old_root}/player-kill-marks/{}/forgive", Uuid::now_v7()),
            ),
            ("GET", format!("{old_root}/socket")),
        ];
        for (method, path) in removed_routes {
            let response = http_request(address, &host, &origin, method, &path, None, None).await;
            assert_eq!(response.status, 404, "removed route {method} {path}");
        }
    }

    let login = http_request(
        address,
        &host,
        &origin,
        "POST",
        "/v4/login",
        None,
        Some(
            &serde_json::to_string(&wire::LoginRequestV1 {
                username: wire::Username::new(USERNAME).unwrap(),
                password: wire::Password::new(PASSWORD).unwrap(),
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(
        login.status,
        200,
        "{}",
        String::from_utf8_lossy(&login.body)
    );
    assert!(!login.headers.contains_key("set-cookie"));
    let login: wire::LoginResponseV1 = serde_json::from_slice(&login.body).unwrap();
    let token = login.session_token.expose_for_validation().to_string();
    let bootstrap = login.bootstrap;
    assert_eq!(bootstrap.control_api_version, wire::CONTROL_API_VERSION);

    for (header, credential) in [
        ("authorization", format!("Bearer {token}")),
        ("cookie", format!("__Host-tme_session={token}")),
    ] {
        let mut request = format!("ws://{address}/v4/socket")
            .into_client_request()
            .unwrap();
        request
            .headers_mut()
            .insert("host", HeaderValue::from_str(&host).unwrap());
        request
            .headers_mut()
            .insert("origin", HeaderValue::from_str(&origin).unwrap());
        request.headers_mut().insert(
            "sec-websocket-protocol",
            HeaderValue::from_static(wire::WEBSOCKET_SUBPROTOCOL),
        );
        request
            .headers_mut()
            .insert(header, HeaderValue::from_str(&credential).unwrap());
        let error = tokio_tungstenite::connect_async(request)
            .await
            .expect_err("socket credentials must be refused");
        assert!(
            matches!(error, tokio_tungstenite::tungstenite::Error::Http(response) if response.status().as_u16() == 403)
        );
    }

    let session = http_request(
        address,
        &host,
        &origin,
        "POST",
        "/v4/session",
        Some(&token),
        Some("{}"),
    )
    .await;
    assert_eq!(session.status, 200);
    let session_bootstrap: wire::SessionBootstrapV1 =
        serde_json::from_slice(&session.body).unwrap();
    assert_eq!(
        session_bootstrap.control_api_version,
        wire::CONTROL_API_VERSION
    );

    let selection = post_json(
        address,
        &host,
        &origin,
        "/v4/characters/select",
        &token,
        &wire::CharacterSelectRequestV1 {
            csrf_token: session_bootstrap.csrf_token.clone(),
            character_id,
        },
    )
    .await;
    assert_eq!(selection.status, 200);
    let selection_body: wire::CharacterSelectionV1 =
        serde_json::from_slice(&selection.body).unwrap();
    assert_eq!(
        selection_body.control_api_version,
        wire::CONTROL_API_VERSION
    );

    let first_ticket = issue_ticket(
        address,
        &host,
        &origin,
        &token,
        &session_bootstrap.csrf_token,
    )
    .await;
    let (mut first, first_welcome) =
        connect_character(address, &host, &origin, &first_ticket.ticket).await;
    let (actor_id, control_epoch, facet_revision) = welcome_parts(&first_welcome);
    let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let command = wire::ClientCommandEnvelope::Command {
        command_id,
        control_epoch: wire::DecimalU64::new(control_epoch),
        client_sequence: wire::DecimalU64::new(1),
        observed_world_revision: wire::DecimalU64::new(facet_revision),
        actor_id,
        intent: wire::Intent::Wait,
    };
    first
        .send(Message::Text(
            serde_json::to_string(&command).unwrap().into(),
        ))
        .await
        .unwrap();
    assert!(matches!(
        receive_envelope(&mut first).await,
        wire::ServerEnvelope::CommandResult {
            replay_status: wire::ReplayStatus::New,
            ..
        }
    ));
    assert!(matches!(
        receive_envelope(&mut first).await,
        wire::ServerEnvelope::StateUpdate { .. }
    ));

    let second_ticket = issue_ticket(
        address,
        &host,
        &origin,
        &token,
        &session_bootstrap.csrf_token,
    )
    .await;
    let (mut second, second_welcome) =
        connect_character(address, &host, &origin, &second_ticket.ticket).await;
    assert_eq!(
        wait_for_draining(&mut first).await,
        (wire::DrainingReason::ControlReplaced, true)
    );
    let (_, second_epoch, _) = welcome_parts(&second_welcome);
    assert_eq!(second_epoch, control_epoch + 1);
    second
        .send(Message::Text(
            serde_json::to_string(&command).unwrap().into(),
        ))
        .await
        .unwrap();
    assert!(matches!(
        receive_envelope(&mut second).await,
        wire::ServerEnvelope::CommandResult {
            replay_status: wire::ReplayStatus::Replayed,
            ..
        }
    ));
    assert!(matches!(
        receive_envelope(&mut second).await,
        wire::ServerEnvelope::StateUpdate { .. }
    ));

    let logout = post_json(
        address,
        &host,
        &origin,
        "/v4/logout",
        &token,
        &wire::LogoutRequestV1 {
            csrf_token: session_bootstrap.csrf_token,
        },
    )
    .await;
    assert_eq!(logout.status, 204);
    assert_eq!(
        wait_for_draining(&mut second).await,
        (wire::DrainingReason::SessionEnded, false)
    );

    let _ = stop_send.send(());
    server.await.unwrap().unwrap();
}
