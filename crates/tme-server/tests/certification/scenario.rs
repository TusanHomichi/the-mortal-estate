// Private scenario evidence for the EV certification target.
#[tokio::test(flavor = "current_thread", start_paused = true)]
#[ignore = "requires the non-skipping EV PostgreSQL 18 runner"]
async fn ev_postgres_certification() {
    let time_anchor = PausedTimeAnchor::start().await;
    let paused_before_wall_delay = tokio::time::Instant::now();
    wall_clock_delay(Duration::from_millis(25)).await;
    assert_eq!(
        paused_before_wall_delay,
        tokio::time::Instant::now(),
        "real I/O wait auto-advanced the paused Tokio clock"
    );
    time_anchor.assert_running();

    let database_url = std::env::var("TME_EV_DATABASE_URL")
        .expect("the EV runner must provide TME_EV_DATABASE_URL");
    let expected_database = std::env::var("TME_EV_DATABASE_NAME")
        .expect("the EV runner must provide TME_EV_DATABASE_NAME");
    let expected_sentinel = std::env::var("TME_EV_DATABASE_SENTINEL")
        .expect("the EV runner must provide TME_EV_DATABASE_SENTINEL");
    let expected_role = std::env::var("TME_EV_DATABASE_ROLE")
        .expect("the EV runner must provide TME_EV_DATABASE_ROLE");
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    assert_runner_identity(
        &pool,
        &expected_database,
        &expected_sentinel,
        &expected_role,
    )
    .await;

    // D4: this process hosts exactly one canonical world; all eight certified
    // characters live in it.
    let world_facet = wire::FacetId::new(Uuid::now_v7()).unwrap();
    let (world_engine, world_actors) = engine_with_characters(8);

    let mut fixtures = Vec::new();
    for (index, actor_id) in world_actors.into_iter().enumerate() {
        fixtures.push(CharacterFixture {
            account_id: wire::AccountId::new(Uuid::now_v7()).unwrap(),
            character_id: wire::CharacterId::new(Uuid::now_v7()).unwrap(),
            username: format!("ev_{}", index + 1),
            actor_id,
        });
    }
    assert_eq!(8, fixtures.len());
    for (index, fixture) in fixtures.iter().enumerate() {
        insert_account(&pool, fixture, (index + 1) as u8).await;
    }

    let backend = PostgresState::open(
        &database_url,
        PostgresBootstrap {
            world: PostgresWorldBootstrap {
                facet_id: world_facet,
                key: EV_WORLD_KEY.to_string(),
                engine: world_engine,
            },
            characters: fixtures
                .iter()
                .map(|fixture| PostgresCharacterBootstrap {
                    account_id: fixture.account_id,
                    character_id: fixture.character_id,
                    slot: 1,
                    display_name: wire::DisplayName::new(format!(
                        "EV Character {}",
                        fixture.username
                    ))
                    .unwrap(),
                    actor_id: fixture.actor_id.clone(),
                })
                .collect(),
        },
    )
    .await
    .unwrap();
    assert!(backend.gameplay_ready());
    let scheduler_live_time = tokio::time::Instant::now();
    wall_clock_delay(Duration::from_millis(25)).await;
    assert_eq!(
        scheduler_live_time,
        tokio::time::Instant::now(),
        "live production schedulers advanced without the test clock owner"
    );
    time_anchor.assert_running();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let operations_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let operations_address = operations_listener.local_addr().unwrap();
    let host = address.to_string();
    let origin = format!("http://{host}");
    let config = ServerConfig::new(address, host.clone(), origin.clone()).unwrap();
    let state = AppState::postgres(config, backend.clone());
    let (stop_send, stop_receive) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(tme_server::runtime::serve_production(
        listener,
        operations_listener,
        state,
        async {
            let _ = stop_receive.await;
        },
    ));

    // Install all eight grants against the one world. Each observer must land
    // in that world on its own bootstrapped actor, and the durable presence
    // commits must mutate the world row without minting a command receipt.
    let world_before_presence = facet_identity(&pool, world_facet).await;
    let mut clients = Vec::new();
    for fixture in &fixtures {
        let client = support::login_select_connect(
            address,
            &host,
            &origin,
            &fixture.username,
            fixture.character_id,
        )
        .await;
        assert_eq!(client.actor_id.as_str(), fixture.actor_id.as_str());
        clients.push(client);
    }
    assert_eq!(8, clients.len());
    assert_ne!(
        world_before_presence,
        facet_identity(&pool, world_facet).await,
        "eight presence commits did not mutate the world"
    );
    let receipt_count: i64 = sqlx::query_scalar("SELECT count(*) FROM tme.command_receipts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        0, receipt_count,
        "presence pressure created a command receipt"
    );

    // Twelve toggle/wait pairs paced to the authoritative pulse, plus the
    // standing headroom for real PostgreSQL I/O on a loaded runner.
    let mut packet_watchdog =
        WallClockWatchdog::start(WAIT_ROUNDS * STANDARD_ACTION_DURATION + PACKET_IO_HEADROOM);
    let packet = async {
        let baseline = facet_baseline(&pool, world_facet).await;

        let mut command_sequences: BTreeMap<wire::FacetId, BTreeSet<u64>> = BTreeMap::new();
        let mut request_digests = BTreeMap::<Uuid, [u8; 32]>::new();
        let mut pair_started = None;
        for round in 0..(2 * WAIT_ROUNDS) {
            let is_wait = round % 2 == 1;
            if !is_wait {
                pair_started = Some(Instant::now());
            }
            if is_wait {
                assert!(clients.iter().all(|client| client.can_act));
            }
            let prior_times: Vec<u64> = clients.iter().map(|client| client.logical_time).collect();
            let command_ids: Vec<_> = (0..clients.len())
                .map(|_| wire::CommandId::new(Uuid::now_v7()).unwrap())
                .collect();
            let commands: Vec<_> = clients
                .iter()
                .zip(&command_ids)
                .map(|(client, command_id)| {
                    let intent = if is_wait {
                        wire::Intent::Wait
                    } else {
                        wire::Intent::SetPagesEnabled {
                            enabled: (round / 2) % 2 == 1,
                        }
                    };
                    support::command(client, *command_id, client.facet_revision, intent)
                })
                .collect();
            record_request_digests(&commands, &mut request_digests);
            let mut overlap_lock = if round == 0 {
                let mut transaction = pool.begin().await.unwrap();
                sqlx::query("SELECT facet_id FROM tme.facets WHERE facet_id=$1 FOR UPDATE")
                    .bind(world_facet.as_uuid())
                    .fetch_one(&mut *transaction)
                    .await
                    .unwrap();
                Some(transaction)
            } else {
                None
            };
            barrier_send(&mut clients, &commands).await;
            if let Some(transaction) = overlap_lock.take() {
                // One command is dequeued and blocked on the held row lock; the
                // other seven must be queued behind it.
                wait_for_mailbox_depth(operations_address, 7).await;
                let world_receipts: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM tme.command_receipts")
                        .fetch_one(&pool)
                        .await
                        .unwrap();
                assert_eq!(
                    0, world_receipts,
                    "a world receipt committed through the controlled row lock"
                );
                transaction.rollback().await.unwrap();
            }

            for (client, command_id) in clients.iter_mut().zip(command_ids) {
                let result = support::receive_result(client, command_id).await;
                match result {
                    wire::ServerEnvelope::CommandResult {
                        disposition: wire::CommandDisposition::Accepted,
                        replay_status: wire::ReplayStatus::New,
                        server_sequence: Some(server_sequence),
                        before_revision: Some(before_revision),
                        after_revision: Some(after_revision),
                        ..
                    } => {
                        assert_eq!(before_revision.get() + 1, after_revision.get());
                        client.facet_revision = client.facet_revision.max(after_revision.get());
                        assert!(
                            command_sequences
                                .entry(world_facet)
                                .or_default()
                                .insert(server_sequence.get())
                        );
                        client.next_client_sequence += 1;
                    }
                    other => panic!("accepted EV command returned {other:?}"),
                }
            }
            if is_wait {
                // One authoritative pulse, at the one cadence the server owns
                // (D5; `tme_server::STANDARD_ACTION_DURATION`). Advancing anything less
                // strikes no boundary and no client would ever become ready.
                tokio::time::advance(STANDARD_ACTION_DURATION).await;
                tokio::task::yield_now().await;
                for (client, prior_time) in clients.iter_mut().zip(prior_times) {
                    support::wait_until_ready_after(client, prior_time).await;
                }
                // Each toggle/wait pair also occupies a real pulse of wall time,
                // so eight clients are certified against real PostgreSQL I/O at
                // the cadence they will actually be played at.
                let elapsed = pair_started
                    .take()
                    .expect("toggle/wait pair start")
                    .elapsed();
                if let Some(remaining) = STANDARD_ACTION_DURATION.checked_sub(elapsed) {
                    wall_clock_delay(remaining).await;
                }
            }
        }

        for _ in 0..8 {
            let command_ids: Vec<_> = (0..clients.len())
                .map(|_| wire::CommandId::new(Uuid::now_v7()).unwrap())
                .collect();
            let commands: Vec<_> = clients
                .iter()
                .zip(&command_ids)
                .map(|(client, command_id)| {
                    support::command(client, *command_id, u64::MAX, wire::Intent::Wait)
                })
                .collect();
            record_request_digests(&commands, &mut request_digests);
            barrier_send(&mut clients, &commands).await;
            for (client, command_id) in clients.iter_mut().zip(command_ids) {
                let result = support::receive_result(client, command_id).await;
                let server_sequence = match result {
                    wire::ServerEnvelope::CommandResult {
                        disposition:
                            wire::CommandDisposition::Rejected {
                                code: wire::RejectionCode::FutureWorldRevision,
                            },
                        replay_status: wire::ReplayStatus::New,
                        server_sequence: Some(server_sequence),
                        before_revision: Some(before_revision),
                        after_revision: Some(after_revision),
                        ..
                    } => {
                        assert_eq!(before_revision, after_revision);
                        server_sequence.get()
                    }
                    other => panic!("future-revision command returned {other:?}"),
                };
                assert!(
                    command_sequences
                        .entry(world_facet)
                        .or_default()
                        .insert(server_sequence)
                );
                support::wait_for_state_sequence(client, server_sequence).await;
            }
        }
        (baseline, command_sequences, request_digests)
    };
    let (baseline, command_sequences, request_digests) = tokio::select! {
        packet_result = packet => packet_result,
        expired = &mut packet_watchdog.expired => {
            expired.expect("wall-clock watchdog sender");
            panic!("the canonical 256-command packet exceeded 60 wall-clock seconds");
        }
    };
    drop(packet_watchdog);
    assert!(
        clients
            .iter()
            .all(|client| client.next_client_sequence == 25)
    );
    assert!(clients.iter().all(|client| client.pages_enabled));
    // Every result in the packet reported the one world, and its 256 server
    // sequences were distinct within that world.
    assert_eq!(
        BTreeSet::from([world_facet]),
        command_sequences.keys().copied().collect::<BTreeSet<_>>()
    );
    assert_eq!(256, command_sequences[&world_facet].len());
    join_all(clients.iter_mut().map(support::assert_no_terminal_result)).await;
    assert_packet_rows(&pool, &fixtures, &request_digests).await;
    assert_observer_ownership(&clients, &baseline);
    assert_final_checkpoints(&pool, world_facet, &baseline, &fixtures).await;

    // Send one accepted command, disconnect before consuming its terminal result,
    // then reconnect and prove the exact old envelope is a durable replay.
    let replay_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let replay_command = support::command(
        &clients[0],
        replay_id,
        clients[0].facet_revision,
        wire::Intent::SetPagesEnabled { enabled: false },
    );
    let checkpoint_before_disconnect = facet_identity(&pool, world_facet).await;
    support::send_command(&mut clients[0].socket, &replay_command).await;
    clients[0].socket.close(None).await.unwrap();
    wait_for_receipt(&pool, replay_id).await;
    let receipt_after_disconnect = receipt_identity(&pool, fixtures[0].account_id, replay_id).await;
    let expected_replay = persisted_receipt_envelope(
        &pool,
        fixtures[0].account_id,
        replay_id,
        wire::ReplayStatus::Replayed,
    )
    .await;
    let checkpoint_after_disconnect = facet_identity(&pool, world_facet).await;
    assert_ne!(
        checkpoint_before_disconnect, checkpoint_after_disconnect,
        "disconnect-before-reply command did not mutate its checkpoint"
    );
    let (socket, welcome) =
        support::replacement_connection(address, &host, &origin, &clients[0]).await;
    clients[0].socket = socket;
    clients[0].apply(&welcome);
    assert!(
        !clients[0].pages_enabled,
        "replacement welcome did not hydrate the disconnected command"
    );
    clients[0].next_client_sequence = 1;
    let checkpoint_before_replay = facet_identity(&pool, world_facet).await;
    support::send_command(&mut clients[0].socket, &replay_command).await;
    let replay_envelope = support::receive_result(&mut clients[0], replay_id).await;
    assert_eq!(
        expected_replay, replay_envelope,
        "disconnect retry did not return the exact persisted receipt outcome"
    );
    let replay_outcome = durable_command_outcome(replay_envelope, wire::ReplayStatus::Replayed);
    assert_eq!(
        wire::CommandDisposition::Accepted,
        replay_outcome.disposition
    );
    assert_eq!(
        receipt_after_disconnect,
        receipt_identity(&pool, fixtures[0].account_id, replay_id).await,
        "durable replay replaced the original receipt"
    );
    assert_eq!(
        checkpoint_before_replay,
        facet_identity(&pool, world_facet).await,
        "durable replay executed the mutation twice"
    );

    let state_before_mismatch = facet_durable_state(&pool, world_facet).await;
    assert_eq!(
        Some(&(true, None)),
        state_before_mismatch
            .presence
            .get(&fixtures[0].character_id.to_string())
    );
    let mismatch = support::command(
        &clients[0],
        replay_id,
        clients[0].facet_revision,
        wire::Intent::Wait,
    );
    support::send_command(&mut clients[0].socket, &mismatch).await;
    loop {
        let envelope = support::receive_envelope(&mut clients[0].socket).await;
        clients[0].apply(&envelope);
        match envelope {
            wire::ServerEnvelope::Error {
                code: wire::ErrorCode::MalformedProtocol,
            } => break,
            wire::ServerEnvelope::CommandResult { command_id, .. } => {
                panic!("digest mismatch received an unexpected terminal result: {command_id}")
            }
            wire::ServerEnvelope::Error { code } => {
                panic!("digest mismatch returned the wrong error: {code:?}")
            }
            wire::ServerEnvelope::StateUpdate { .. } => {}
            other => panic!("digest mismatch received an unexpected envelope: {other:?}"),
        }
    }
    assert_connection_closes_without_envelope(&mut clients[0].socket).await;
    assert_eq!(
        receipt_after_disconnect,
        receipt_identity(&pool, fixtures[0].account_id, replay_id).await,
        "same-ID different-digest input replaced the durable receipt"
    );
    let state_after_mismatch = wait_for_durable_disconnect(
        &pool,
        world_facet,
        fixtures[0].character_id,
        state_before_mismatch.max_audit_id,
    )
    .await;
    let mismatch_audits = facet_audits_between(
        &pool,
        state_before_mismatch.max_audit_id,
        state_after_mismatch.max_audit_id,
    )
    .await;
    assert_audit_accounted_transition(
        &state_before_mismatch,
        &state_after_mismatch,
        &mismatch_audits,
    );
    assert_eq!(Some(&1), mismatch_audits.get("facet_presence"));
    assert!(
        mismatch_audits
            .keys()
            .all(|action| action == "facet_presence" || action == "facet_deadlines")
    );
    assert_eq!(
        Some(&false),
        state_after_mismatch
            .pages_enabled
            .get(&fixtures[0].character_id.to_string()),
        "same-ID different-digest input changed the gameplay preference"
    );

    // Two independently issued one-use tickets race from the same durable
    // epoch. Exactly one may install a grant and receive authoritative state.
    let old_epoch = clients[1].control_epoch;
    let left_ticket =
        support::issue_ticket(address, &host, &origin, &clients[1].token, &clients[1].csrf).await;
    let right_ticket =
        support::issue_ticket(address, &host, &origin, &clients[1].token, &clients[1].csrf).await;
    let (left, right) = tokio::join!(
        support::connect_ticket(address, &host, &origin, &left_ticket),
        support::connect_ticket(address, &host, &origin, &right_ticket),
    );
    let left_won = matches!(left.1, wire::ServerEnvelope::ServerWelcome { .. });
    let right_won = matches!(right.1, wire::ServerEnvelope::ServerWelcome { .. });
    assert_ne!(
        left_won, right_won,
        "exactly one ticket must receive a welcome"
    );
    let ((replacement_socket, replacement_welcome), (mut loser_socket, loser_envelope)) =
        if left_won {
            (left, right)
        } else {
            (right, left)
        };
    let replacement_epoch = match &replacement_welcome {
        wire::ServerEnvelope::ServerWelcome { control_epoch, .. } => control_epoch.get(),
        other => panic!("winning ticket did not receive a welcome: {other:?}"),
    };
    assert_eq!(old_epoch.checked_add(1).unwrap(), replacement_epoch);
    assert!(matches!(
        loser_envelope,
        wire::ServerEnvelope::Error {
            code: wire::ErrorCode::Unavailable
        }
    ));
    assert_connection_closes_without_envelope(&mut loser_socket).await;
    let durable_epoch: i64 =
        sqlx::query_scalar("SELECT control_epoch FROM tme.characters WHERE character_id=$1")
            .bind(fixtures[1].character_id.as_uuid())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(i64::try_from(replacement_epoch).unwrap(), durable_epoch);
    assert_eq!(
        support::wait_for_draining(&mut clients[1]).await,
        wire::DrainingReason::ControlReplaced
    );
    clients[1].socket = replacement_socket;
    clients[1].apply(&replacement_welcome);
    clients[1].next_client_sequence = 1;

    for client in &mut clients {
        let _ = client.socket.close(None).await;
    }
    let _ = stop_send.send(());
    tokio::time::advance(Duration::from_secs(5)).await;
    server.await.unwrap().unwrap();
    drop(backend);
    tokio::task::yield_now().await;

    prove_child_process_restart(&pool, &database_url, &fixtures, world_facet).await;
    pool.close().await;
    time_anchor.release().await;
}

async fn assert_connection_closes_without_envelope(socket: &mut support::Socket) {
    let mut watchdog = WallClockWatchdog::start(Duration::from_secs(5));
    tokio::select! {
        message = socket.next() => match message {
            None
            | Some(Ok(tokio_tungstenite::tungstenite::protocol::Message::Close(_)))
            | Some(Err(_)) => {}
            Some(Ok(message)) => {
                panic!("losing one-use ticket received data after its terminal error: {message:?}")
            }
        },
        expired = &mut watchdog.expired => {
            expired.expect("wall-clock watchdog sender");
            panic!("losing one-use ticket did not close within five wall-clock seconds");
        }
    }
}

async fn barrier_send(clients: &mut [support::Client], commands: &[wire::ClientCommandEnvelope]) {
    assert_eq!(clients.len(), commands.len());
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(clients.len() + 1));
    let release = barrier.clone();
    let sends = clients.iter_mut().zip(commands).map(|(client, command)| {
        let barrier = barrier.clone();
        async move {
            barrier.wait().await;
            support::send_command(&mut client.socket, command).await;
        }
    });
    let (_, results) = tokio::join!(release.wait(), join_all(sends));
    assert_eq!(results.len(), commands.len());
}

fn record_request_digests(
    commands: &[wire::ClientCommandEnvelope],
    digests: &mut BTreeMap<Uuid, [u8; 32]>,
) {
    for command in commands {
        let wire::ClientCommandEnvelope::Command { command_id, .. } = command else {
            panic!("canonical packet contains a non-gameplay command")
        };
        let digest: [u8; 32] = Sha256::digest(serde_json::to_vec(command).unwrap()).into();
        assert!(
            digests.insert(command_id.as_uuid(), digest).is_none(),
            "canonical packet reused a command ID"
        );
    }
}

async fn wait_for_mailbox_depth(operations_address: SocketAddr, minimum: u64) {
    let mut watchdog = WallClockWatchdog::start(Duration::from_secs(10));
    let observe = async {
        loop {
            let (status_code, status) = support::operations_status(operations_address).await;
            assert_eq!(status_code, 200, "operations status became unready");
            let depth = status["maximum_mailbox_depth"]
                .as_u64()
                .expect("numeric operations mailbox depth");
            if depth >= minimum {
                return;
            }
            wall_clock_delay(Duration::from_millis(10)).await;
        }
    };
    tokio::select! {
        () = observe => {}
        expired = &mut watchdog.expired => {
            expired.expect("mailbox-depth watchdog sender");
            panic!("controlled overlap did not queue {minimum} world commands");
        }
    }
}
