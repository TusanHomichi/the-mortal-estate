// Private restart evidence for the EV certification target.
async fn prove_child_process_restart(
    pool: &sqlx::PgPool,
    database_url: &str,
    fixtures: &[CharacterFixture],
    world_facet: wire::FacetId,
) {
    let private_temp_root = runner_private_temp_root();
    let child_root = private_temp_root.join(format!("tme-ev-child-{}", Uuid::now_v7()));
    fs::create_dir(&child_root).expect("create private EV child directory");
    let directory = TestDirectory(child_root);
    let root = &directory.0;
    let credentials = root.join("credentials");
    fs::create_dir(&credentials).expect("create private EV credential directory");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&credentials, fs::Permissions::from_mode(0o700)).unwrap();
    }
    for name in ["database-url", "auth-database-url"] {
        let path = credentials.join(name);
        fs::write(&path, format!("{database_url}\n")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }

    let content = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/test-corpus")
        .canonicalize()
        .unwrap();
    let manifest = root.join("bootstrap.json");
    let world = serde_json::json!({
        "facet_id": world_facet,
        "key": EV_WORLD_KEY,
        "simulation_seed": content.join("simulation_seeds/world_topology_gallery.json"),
        "rng_seed": 7,
    });
    let characters = fixtures
        .iter()
        .map(|fixture| {
            serde_json::json!({
                "account_id": fixture.account_id,
                "character_id": fixture.character_id,
                "slot": 1,
                "display_name": format!("EV Character {}", fixture.username),
                "actor_id": fixture.actor_id.as_str(),
            })
        })
        .collect::<Vec<_>>();
    fs::write(
        &manifest,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "catalog": content.join("catalogs/prototype_catalog_v6.json"),
            "catalog_profile": "profile/world_topology_gallery",
            "world_template": content.join("world_templates/world_topology_gallery.json"),
            "world": world,
            "characters": characters,
        }))
        .unwrap(),
    )
    .unwrap();

    let (public_address, operations_address) = unused_loopback_addresses();
    let host = public_address.to_string();
    let origin = format!("http://{host}");
    let mut first = spawn_server_child(
        &manifest,
        &credentials,
        public_address,
        operations_address,
        &host,
        &origin,
    );
    wait_for_child(public_address, operations_address, &mut first).await;
    let fixture = &fixtures[2];
    let mut client = support::login_select_connect(
        public_address,
        &host,
        &origin,
        &fixture.username,
        fixture.character_id,
    )
    .await;
    assert!(
        client.pages_enabled,
        "restart fixture did not begin from the packet's enabled preference"
    );
    let checkpoint_before_command = facet_identity(pool, world_facet).await;
    let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let command = support::command(
        &client,
        command_id,
        client.facet_revision,
        wire::Intent::SetPagesEnabled { enabled: false },
    );
    support::send_command(&mut client.socket, &command).await;
    let first_result = support::receive_result(&mut client, command_id).await;
    let first_outcome = durable_command_outcome(first_result, wire::ReplayStatus::New);
    assert_eq!(
        wire::CommandDisposition::Accepted,
        first_outcome.disposition
    );
    assert_eq!(
        first_outcome.before_revision.as_ref().unwrap().get() + 1,
        first_outcome.after_revision.as_ref().unwrap().get(),
        "restart command did not advance exactly one revision"
    );
    if client.pages_enabled {
        support::wait_for_state_sequence(
            &mut client,
            first_outcome.server_sequence.as_ref().unwrap().get(),
        )
        .await;
    }
    assert!(
        !client.pages_enabled,
        "restart command did not publish its false preference"
    );
    let checkpoint_after_command = facet_identity(pool, world_facet).await;
    assert_ne!(
        checkpoint_before_command, checkpoint_after_command,
        "restart command did not mutate the durable checkpoint"
    );
    let receipt_before_restart = receipt_identity(pool, fixture.account_id, command_id).await;

    // Prove the process emitted a real transient message before it was killed;
    // the fresh process must never reconstruct it from durable state.
    let receiver_fixture = &fixtures[1];
    let mut receiver = support::login_select_connect(
        public_address,
        &host,
        &origin,
        &receiver_fixture.username,
        receiver_fixture.character_id,
    )
    .await;
    let message_id = wire::MessageId::new(Uuid::now_v7()).unwrap();
    let social = wire::ClientCommandEnvelope::SocialMessage {
        message_id,
        control_epoch: wire::DecimalU64::new(client.control_epoch),
        actor_id: client.actor_id.clone(),
        scope: wire::SocialScope::Say,
        body: wire::SocialBody::new("EV process-restart transient").unwrap(),
    };
    support::send_command(&mut client.socket, &social).await;
    wait_for_social_message(&mut receiver, message_id).await;
    wait_for_message_result(&mut client, message_id).await;

    crash_server_child(&mut first);
    drop((client, receiver));
    assert_eq!(
        receipt_before_restart,
        receipt_identity(pool, fixture.account_id, command_id).await
    );
    let state_after_crash = facet_durable_state(pool, world_facet).await;
    assert_eq!(
        Some((true, None)),
        state_after_crash
            .presence
            .get(&fixture.character_id.to_string())
            .copied()
    );
    assert_eq!(
        Some((true, None)),
        state_after_crash
            .presence
            .get(&receiver_fixture.character_id.to_string())
            .copied()
    );

    let mut second = spawn_server_child(
        &manifest,
        &credentials,
        public_address,
        operations_address,
        &host,
        &origin,
    );
    wait_for_child(public_address, operations_address, &mut second).await;
    {
        let before = &state_after_crash;
        let after = facet_durable_state(pool, world_facet).await;
        let audits = facet_audits_between(pool, before.max_audit_id, after.max_audit_id).await;
        assert_audit_accounted_transition(before, &after, &audits);
        assert_eq!(
            Some(&1),
            audits.get("facet_presence"),
            "startup recovery did not commit exactly one facet_presence audit"
        );
        assert!(
            audits
                .keys()
                .all(|action| action == "facet_presence" || action == "facet_deadlines"),
            "startup recovery committed an unexpected action: {audits:?}"
        );
        assert_ne!(
            before.checkpoint_sha256, after.checkpoint_sha256,
            "startup recovery did not replace the killed checkpoint"
        );
        assert!(
            after
                .presence
                .iter()
                .all(|(character_id, (connected, absent_since))| {
                    let (was_connected, previous_absence) = before.presence[character_id];
                    let expected = if was_connected || previous_absence.is_none() {
                        Some(before.logical_time)
                    } else {
                        previous_absence
                    };
                    !connected && *absent_since == expected
                }),
            "startup recovery must preserve existing absence deadlines"
        );
    }
    assert_eq!(
        receipt_before_restart,
        receipt_identity(pool, fixture.account_id, command_id).await
    );
    let mut restarted = support::login_select_connect(
        public_address,
        &host,
        &origin,
        &fixture.username,
        fixture.character_id,
    )
    .await;
    let mut restarted_receiver = support::login_select_connect(
        public_address,
        &host,
        &origin,
        &receiver_fixture.username,
        receiver_fixture.character_id,
    )
    .await;
    assert_eq!(fixture.actor_id.as_str(), restarted.actor_id.as_str());
    assert!(
        !restarted.pages_enabled,
        "fresh welcome did not hydrate the committed preference"
    );
    assert_no_social_replay(&mut restarted.socket, message_id).await;
    assert_no_social_replay(&mut restarted_receiver.socket, message_id).await;

    let state_before_process_replay = facet_durable_state(pool, world_facet).await;
    support::send_command(&mut restarted.socket, &command).await;
    let replay_result = support::receive_result(&mut restarted, command_id).await;
    let replay_outcome = durable_command_outcome(replay_result, wire::ReplayStatus::Replayed);
    assert_eq!(first_outcome, replay_outcome);
    let state_after_process_replay = facet_durable_state(pool, world_facet).await;
    let replay_audits = facet_audits_between(
        pool,
        state_before_process_replay.max_audit_id,
        state_after_process_replay.max_audit_id,
    )
    .await;
    assert_audit_accounted_transition(
        &state_before_process_replay,
        &state_after_process_replay,
        &replay_audits,
    );
    assert!(
        replay_audits
            .keys()
            .all(|action| action == "facet_deadlines"),
        "durable replay committed a non-tick mutation: {replay_audits:?}"
    );
    assert_eq!(
        Some(&false),
        state_after_process_replay
            .pages_enabled
            .get(&fixture.character_id.to_string()),
        "durable replay changed the committed gameplay preference"
    );
    assert_eq!(
        receipt_before_restart,
        receipt_identity(pool, fixture.account_id, command_id).await,
        "process restart or replay replaced the durable receipt"
    );
    let _ = restarted.socket.close(None).await;
    let _ = restarted_receiver.socket.close(None).await;
    terminate_server_child(&mut second);
}
