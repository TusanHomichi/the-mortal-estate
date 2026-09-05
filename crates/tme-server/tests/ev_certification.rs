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

// Preserve the exact runner-selected test name; split evidence by responsibility.
include!("certification/scenario.rs");
include!("certification/restart.rs");
include!("certification/receipts.rs");
include!("certification/process.rs");
include!("certification/ownership.rs");
