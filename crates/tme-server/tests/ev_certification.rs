const STANDARD_ACTION_DURATION: std::time::Duration = std::time::Duration::from_millis(3_000);
mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread::JoinHandle as ThreadJoinHandle;
use std::time::{Duration, Instant};

use argon2::password_hash::{PasswordHasher, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use futures_util::{StreamExt, future::join_all};
use sha2::{Digest, Sha256};
use sqlx::Row;
use tme_protocol as wire;
use tme_server::{
    AppState, PostgresBootstrap, PostgresCharacterBootstrap, PostgresState, PostgresWorldBootstrap,
    ServerConfig,
};
use uuid::Uuid;

/// The durable key of the one world both the in-process backend and the child
/// process bootstrap. Startup fails closed when they disagree.
const EV_WORLD_KEY: &str = "ev-world";

#[derive(Clone)]
struct CharacterFixture {
    account_id: wire::AccountId,
    character_id: wire::CharacterId,
    username: String,
    actor_id: tme_rules::ActorId,
}

#[derive(Clone)]
struct FacetBaseline {
    checkpoint: Vec<u8>,
    content_digest: Vec<u8>,
    ownership: CheckpointOwnership,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CheckpointOwnership {
    actor_ids: BTreeSet<String>,
    character_ids: BTreeSet<String>,
    item_ids: BTreeSet<String>,
    item_state: Vec<(String, serde_json::Value)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FacetDurableState {
    revision: i64,
    sequence: i64,
    checkpoint_sha256: Vec<u8>,
    logical_time: u64,
    presence: BTreeMap<String, (bool, Option<u64>)>,
    pages_enabled: BTreeMap<String, bool>,
    max_audit_id: i64,
}

#[derive(Debug, PartialEq, Eq)]
struct DurableCommandOutcome {
    command_id: wire::CommandId,
    disposition: wire::CommandDisposition,
    server_sequence: Option<wire::DecimalU64>,
    before_revision: Option<wire::DecimalU64>,
    after_revision: Option<wire::DecimalU64>,
    events: Vec<wire::ObservedEvent>,
    events_truncated: bool,
}

struct TestDirectory(PathBuf);

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

struct WallClockWatchdog {
    cancel: Option<mpsc::Sender<()>>,
    expired: tokio::sync::oneshot::Receiver<()>,
    thread: Option<ThreadJoinHandle<()>>,
}

struct PausedTimeAnchor {
    release: Option<mpsc::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl PausedTimeAnchor {
    async fn start() -> Self {
        let (release, release_receive) = mpsc::channel();
        let (entered, entered_receive) = tokio::sync::oneshot::channel();
        let task = tokio::task::spawn_blocking(move || {
            // Tokio 1.52 explicitly inhibits paused-clock auto-advance while a
            // blocking task is active. The entry acknowledgement makes that
            // guarantee effective before the test's first external DB await.
            let _ = entered.send(());
            let _ = release_receive.recv();
        });
        entered_receive.await.expect("paused-time anchor entered");
        Self {
            release: Some(release),
            task: Some(task),
        }
    }

    fn assert_running(&self) {
        assert!(
            !self
                .task
                .as_ref()
                .expect("paused-time anchor task")
                .is_finished(),
            "paused-time anchor exited early"
        );
    }

    async fn release(mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
        self.task
            .take()
            .expect("paused-time anchor task")
            .await
            .expect("paused-time anchor joins");
    }
}

impl Drop for PausedTimeAnchor {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
    }
}

impl WallClockWatchdog {
    fn start(duration: Duration) -> Self {
        let (cancel, cancel_receive) = mpsc::channel();
        let (expired_send, expired) = tokio::sync::oneshot::channel();
        let thread = std::thread::spawn(move || {
            if cancel_receive.recv_timeout(duration).is_err() {
                let _ = expired_send.send(());
            }
        });
        Self {
            cancel: Some(cancel),
            expired,
            thread: Some(thread),
        }
    }
}

impl Drop for WallClockWatchdog {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Toggle/wait pairs in the certification packet; each ends on one pulse.
const WAIT_ROUNDS: u32 = 12;

/// Wall time the packet may spend on real database I/O on top of its pacing.
const PACKET_IO_HEADROOM: Duration = Duration::from_secs(48);

async fn wall_clock_delay(duration: Duration) {
    if duration.is_zero() {
        return;
    }
    let (send, receive) = tokio::sync::oneshot::channel();
    let thread = std::thread::spawn(move || {
        std::thread::sleep(duration);
        let _ = send.send(());
    });
    receive.await.expect("wall-clock delay sender");
    thread.join().expect("wall-clock delay thread");
}

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
    let left_ticket = support::issue_ticket(
        address,
        &host,
        &origin,
        &clients[1].cookie,
        &clients[1].csrf,
    )
    .await;
    let right_ticket = support::issue_ticket(
        address,
        &host,
        &origin,
        &clients[1].cookie,
        &clients[1].csrf,
    )
    .await;
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

fn unused_loopback_addresses() -> (SocketAddr, SocketAddr) {
    let public = TcpListener::bind("127.0.0.1:0").unwrap();
    let operations = TcpListener::bind("127.0.0.1:0").unwrap();
    let addresses = (
        public.local_addr().unwrap(),
        operations.local_addr().unwrap(),
    );
    assert_ne!(addresses.0, addresses.1);
    addresses
}

fn spawn_server_child(
    manifest: &std::path::Path,
    credentials: &std::path::Path,
    public_address: SocketAddr,
    operations_address: SocketAddr,
    host: &str,
    origin: &str,
) -> ChildGuard {
    ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_tme-server"))
            .arg("serve")
            .env_clear()
            .env("CREDENTIALS_DIRECTORY", credentials)
            .env("TME_PUBLIC_LISTEN", public_address.to_string())
            .env("TME_OPS_LISTEN", operations_address.to_string())
            .env("TME_PUBLIC_HOST", host)
            .env("TME_PUBLIC_ORIGIN", origin)
            .env("TME_BOOTSTRAP_MANIFEST", manifest)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    )
}

fn terminate_server_child(child: &mut ChildGuard) {
    let pid = child.0.id();
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "failed to signal server child {pid}");
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success(), "server child {pid} failed during drain");
            return;
        }
        assert!(
            Instant::now() < deadline,
            "server child {pid} did not drain before its deadline"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn crash_server_child(child: &mut ChildGuard) {
    let pid = child.0.id();
    child
        .0
        .kill()
        .unwrap_or_else(|error| panic!("failed to kill server child {pid}: {error}"));
    let status = child
        .0
        .wait()
        .unwrap_or_else(|error| panic!("failed to reap killed server child {pid}: {error}"));
    assert!(
        !status.success(),
        "killed server child {pid} exited successfully"
    );
}

async fn wait_for_child(
    address: SocketAddr,
    operations_address: SocketAddr,
    child: &mut ChildGuard,
) {
    let pid = child.0.id();
    tokio::task::spawn_blocking(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok()
                && TcpStream::connect_timeout(&operations_address, Duration::from_millis(100))
                    .is_ok()
            {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "server child {pid} did not bind before its deadline"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    })
    .await
    .unwrap();
    assert!(
        child.0.try_wait().unwrap().is_none(),
        "server child exited early"
    );
    let mut watchdog = WallClockWatchdog::start(Duration::from_secs(20));
    let ready = async {
        loop {
            let (status, body) = support::operations_status(operations_address).await;
            if status == 200 && body["gameplay_ready"] == serde_json::Value::Bool(true) {
                return;
            }
            wall_clock_delay(Duration::from_millis(25)).await;
        }
    };
    tokio::select! {
        () = ready => {}
        expired = &mut watchdog.expired => {
            expired.expect("child readiness watchdog sender");
            panic!("server child {pid} did not become ready before its deadline");
        }
    }
}

async fn assert_runner_identity(pool: &sqlx::PgPool, database: &str, sentinel: &str, role: &str) {
    let row = sqlx::query(
        "SELECT current_database() AS database, current_user AS role, \
         shobj_description(d.oid,'pg_database') AS comment \
         FROM pg_database d WHERE d.datname=current_database()",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(database, row.get::<String, _>("database"));
    assert_eq!(role, row.get::<String, _>("role"));
    assert_eq!(
        format!("tme_ev:{sentinel}"),
        row.get::<String, _>("comment")
    );
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT database_oid=(SELECT oid::text FROM pg_database WHERE datname=current_database()) \
             FROM tme.store_state WHERE singleton"
        )
        .fetch_one(pool)
        .await
        .unwrap()
    );
}

async fn insert_account(pool: &sqlx::PgPool, fixture: &CharacterFixture, salt_byte: u8) {
    let params = Params::new(65_536, 3, 4, Some(32)).unwrap();
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let salt = SaltString::encode_b64(&[salt_byte; 16]).unwrap();
    let phc = argon
        .hash_password(support::PASSWORD.as_bytes(), &salt)
        .unwrap()
        .to_string();
    sqlx::query("INSERT INTO tme.accounts(account_id,username,display_name) VALUES($1,$2,$3)")
        .bind(fixture.account_id.as_uuid())
        .bind(&fixture.username)
        .bind(format!("EV {}", fixture.username))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tme.account_credentials(account_id,password_phc) VALUES($1,$2)")
        .bind(fixture.account_id.as_uuid())
        .bind(phc)
        .execute(pool)
        .await
        .unwrap();
}

/// Grows the scenario's single seeded player into `count` distinct actors, all
/// inside the one world this process hosts.
fn engine_with_characters(count: usize) -> (tme_rules::Engine, Vec<tme_rules::ActorId>) {
    let prefix = "ev";
    let mut engine = scenario_engine();
    let original = engine.world().actors[0].clone();
    let original_character = original.character_id.clone().unwrap();
    let preferences = engine
        .world()
        .communication_preferences
        .get(&original_character)
        .cloned()
        .unwrap_or_default();
    let presence = engine
        .world()
        .character_presence
        .get(&original_character)
        .copied()
        .unwrap();
    let quest = engine
        .world()
        .quest_states
        .get(&original_character)
        .cloned();
    let mut actors = vec![original.id.clone()];
    for index in 1..count {
        let temporary_character =
            tme_rules::CharacterId::new(format!("prototype:ev:{prefix}:{index}"));
        let mut actor = original.clone();
        actor.id = tme_rules::ActorId::new(format!("{prefix}_{index}"));
        actor.name = format!("EV {prefix} {index}");
        actor.character_id = Some(temporary_character.clone());
        actor.timing.tie_break_order += u64::try_from(index * 100).unwrap();
        actor.carried.items.clear();
        actor.carried.gold = Default::default();
        actors.push(actor.id.clone());
        engine.world_mut().actors.push(actor);
        engine
            .world_mut()
            .communication_preferences
            .insert(temporary_character.clone(), preferences.clone());
        engine
            .world_mut()
            .character_presence
            .insert(temporary_character.clone(), presence);
        if let Some(quest) = &quest {
            engine
                .world_mut()
                .quest_states
                .insert(temporary_character, quest.clone());
        }
    }
    engine.export_checkpoint().unwrap();
    (engine, actors)
}

fn scenario_engine() -> tme_rules::Engine {
    let mut scenario = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    scenario.extend([
        "..",
        "..",
        "content",
        "test-corpus",
        "world_topology_gallery.json",
    ]);
    tme_sim::load_engine_from_scenario(&scenario, Some(7)).unwrap()
}

async fn facet_baseline(pool: &sqlx::PgPool, facet_id: wire::FacetId) -> FacetBaseline {
    let row = sqlx::query(
        "SELECT checkpoint_bytes,checkpoint_sha256,content_digest FROM tme.facets WHERE facet_id=$1",
    )
    .bind(facet_id.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    let checkpoint: Vec<u8> = row.get("checkpoint_bytes");
    assert_eq!(
        Sha256::digest(&checkpoint).as_slice(),
        row.get::<Vec<u8>, _>("checkpoint_sha256")
    );
    FacetBaseline {
        ownership: checkpoint_ownership(&checkpoint),
        checkpoint,
        content_digest: row.get("content_digest"),
    }
}

fn checkpoint_ownership(checkpoint: &[u8]) -> CheckpointOwnership {
    let payload: serde_json::Value = serde_json::from_slice(checkpoint).unwrap();
    let actors = payload["world"]["actors"]
        .as_array()
        .expect("checkpoint world actors");
    let actor_ids = actors
        .iter()
        .map(|actor| {
            actor["id"]
                .as_str()
                .expect("checkpoint actor ID")
                .to_string()
        })
        .collect();
    let character_ids = actors
        .iter()
        .filter_map(|actor| actor["character_id"].as_str().map(str::to_string))
        .collect();
    let mut item_ids = payload["world"]["item_instances"]
        .as_object()
        .expect("checkpoint item instances")
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    collect_string_fields(&payload, "item_instance_id", &mut item_ids);
    let mut item_state = Vec::new();
    collect_item_state(&payload, "$", &mut item_state);
    CheckpointOwnership {
        actor_ids,
        character_ids,
        item_ids,
        item_state,
    }
}

fn collect_string_fields(
    value: &serde_json::Value,
    selected_key: &str,
    result: &mut BTreeSet<String>,
) {
    match value {
        serde_json::Value::Object(values) => {
            for (key, value) in values {
                if key == selected_key
                    && let Some(value) = value.as_str()
                {
                    result.insert(value.to_string());
                }
                collect_string_fields(value, selected_key, result);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_string_fields(value, selected_key, result);
            }
        }
        _ => {}
    }
}

fn collect_item_state(
    value: &serde_json::Value,
    path: &str,
    result: &mut Vec<(String, serde_json::Value)>,
) {
    match value {
        serde_json::Value::Object(values) => {
            for (key, value) in values {
                let child_path = format!("{path}/{key}");
                if key.contains("item") {
                    result.push((child_path, value.clone()));
                } else {
                    collect_item_state(value, &child_path, result);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                collect_item_state(value, &format!("{path}/{index}"), result);
            }
        }
        _ => {}
    }
}

fn assert_observer_ownership(clients: &[support::Client], baseline: &FacetBaseline) {
    for client in clients {
        // D4: the wire carries no world identity at all, so an observer must
        // never see one. An empty set here is the regression guard: if a world
        // id reappears in any envelope field named *facet_id*, this fails.
        assert!(
            client.observed_facet_ids.is_empty(),
            "an envelope leaked a world identity to an observer: {:?}",
            client.observed_facet_ids
        );
        let ownership = &baseline.ownership;
        assert!(
            client.observed_actor_ids.is_subset(&ownership.actor_ids),
            "observer received an actor absent from its facet checkpoint"
        );
        assert!(
            client
                .observed_character_ids
                .is_subset(&ownership.character_ids),
            "observer received a character absent from its facet checkpoint"
        );
        assert!(
            client.observed_item_ids.is_subset(&ownership.item_ids),
            "observer received an item absent from its facet checkpoint"
        );
    }
}

async fn assert_packet_rows(
    pool: &sqlx::PgPool,
    fixtures: &[CharacterFixture],
    request_digests: &BTreeMap<Uuid, [u8; 32]>,
) {
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM tme.command_receipts")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(256, total);
    assert_eq!(256, request_digests.len());
    let digest_rows = sqlx::query("SELECT command_id,request_digest FROM tme.command_receipts")
        .fetch_all(pool)
        .await
        .unwrap();
    assert_eq!(256, digest_rows.len());
    for row in digest_rows {
        let command_id: Uuid = row.get("command_id");
        assert_eq!(
            request_digests[&command_id].as_slice(),
            row.get::<Vec<u8>, _>("request_digest"),
            "durable receipt request digest was not canonical"
        );
    }
    let accepted: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tme.command_receipts WHERE disposition='accepted' AND after_revision=before_revision+1",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let rejected: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tme.command_receipts WHERE disposition='rejected' AND after_revision=before_revision",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(192, accepted);
    assert_eq!(64, rejected);
    let world_count: i64 = sqlx::query_scalar("SELECT count(*) FROM tme.command_receipts")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(256, world_count);
    let distinct: i64 =
        sqlx::query_scalar("SELECT count(DISTINCT server_sequence) FROM tme.command_receipts")
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(256, distinct);
    let crossed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tme.command_receipts r JOIN tme.characters c USING(account_id) \
         WHERE r.actor_id<>c.actor_id",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(0, crossed);
    for fixture in fixtures {
        let rows = sqlx::query(
            "SELECT client_sequence,disposition FROM tme.command_receipts WHERE account_id=$1 ORDER BY server_sequence",
        )
        .bind(fixture.account_id.as_uuid())
        .fetch_all(pool)
        .await
        .unwrap();
        assert_eq!(32, rows.len());
        for (index, row) in rows.iter().enumerate() {
            let expected_sequence = if index < 24 { index as i64 + 1 } else { 25 };
            assert_eq!(expected_sequence, row.get::<i64, _>("client_sequence"));
            assert_eq!(
                if index < 24 { "accepted" } else { "rejected" },
                row.get::<String, _>("disposition")
            );
        }
    }
}

async fn assert_final_checkpoints(
    pool: &sqlx::PgPool,
    world_facet: wire::FacetId,
    baseline: &FacetBaseline,
    fixtures: &[CharacterFixture],
) {
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM tme.facets")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(1, rows, "the durable store grew a second world");
    let row = sqlx::query(
        "SELECT checkpoint_bytes,checkpoint_sha256,content_digest FROM tme.facets WHERE facet_id=$1",
    )
    .bind(world_facet.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    let checkpoint: Vec<u8> = row.get("checkpoint_bytes");
    assert_ne!(baseline.checkpoint, checkpoint);
    assert_eq!(
        baseline.content_digest,
        row.get::<Vec<u8>, _>("content_digest")
    );
    assert_eq!(
        Sha256::digest(&checkpoint).as_slice(),
        row.get::<Vec<u8>, _>("checkpoint_sha256")
    );
    let final_ownership = checkpoint_ownership(&checkpoint);
    assert_eq!(
        baseline.ownership.item_state, final_ownership.item_state,
        "toggle/wait work changed world-owned item state"
    );
    let own_actor_ids = fixtures
        .iter()
        .map(|fixture| fixture.actor_id.as_str().to_string())
        .collect::<BTreeSet<_>>();
    let own_character_ids = fixtures
        .iter()
        .map(|fixture| fixture.character_id.as_uuid().to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(8, own_actor_ids.len());
    assert_eq!(8, own_character_ids.len());
    assert!(own_actor_ids.is_subset(&final_ownership.actor_ids));
    assert!(own_character_ids.is_subset(&final_ownership.character_ids));
}
