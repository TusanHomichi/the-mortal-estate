use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine as _;
use sha2::{Digest, Sha256};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Executor, PgPool, Postgres, Row, Transaction};
use tme_protocol as wire;
use tme_rules::{ActorId, CharacterId, Engine, FacetCheckpointV5};
#[cfg(test)]
use tokio::sync::oneshot;
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

use crate::admission::{AdmissionError, AdmissionGrant, ControlGrant};
use crate::auth::{AuthService, LoginLimits, random_bytes};
use crate::config::TICKET_LIFETIME;
use crate::coordinator::Coordinator;
use crate::facet::{FacetHandle, FacetWelcome};
use crate::store::migrations;
use crate::store::receipt::ReceiptOutcomeV3;
use crate::store::{
    AuditEvent, PostgresStore, SharedStore, audit, checked_i64, checked_u64, serializable,
};

pub use crate::auth::{
    ARGON2_CONCURRENCY, ARGON2_LANES, ARGON2_MEMORY_KIB, ARGON2_OUTPUT_BYTES, ARGON2_PASSES,
    MAX_BLOCKLIST_ENTRIES, MAX_LOGIN_SOURCE_BUCKETS, MIN_BLOCKLIST_ENTRIES,
};

pub const MAX_ACCOUNTS: usize = 64;
pub const MAX_CHARACTERS_PER_ACCOUNT: usize = 8;
pub const MAX_SESSIONS_PER_ACCOUNT: usize = 8;
pub const MAX_TICKETS_PER_SESSION: usize = 16;
pub const MAX_DURABLE_TICKET_RECORDS: usize =
    MAX_ACCOUNTS * MAX_SESSIONS_PER_ACCOUNT * MAX_TICKETS_PER_SESSION;
pub const MAX_RECEIPTS_PER_ACCOUNT: usize = 65_536;
pub const SESSION_IDLE: Duration = Duration::from_secs(24 * 60 * 60);
pub const SESSION_ABSOLUTE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// The one canonical world this server process hosts. D4: players never choose
/// among divergent copies; separate instances are separate processes and
/// databases (tests, development, private staging, disaster recovery).
pub struct PostgresWorldBootstrap {
    pub facet_id: wire::FacetId,
    pub key: String,
    pub engine: Engine,
}

pub struct PostgresCharacterBootstrap {
    pub account_id: wire::AccountId,
    pub character_id: wire::CharacterId,
    pub slot: u8,
    pub display_name: wire::DisplayName,
    pub actor_id: ActorId,
}

pub struct PostgresBootstrap {
    pub world: PostgresWorldBootstrap,
    pub characters: Vec<PostgresCharacterBootstrap>,
}

#[derive(Clone)]
pub struct PostgresState {
    store: SharedStore,
    auth_pool: PgPool,
    world: Arc<RegisteredFacet>,
    auth: AuthService,
    coordinator: Arc<Coordinator>,
    ready: Arc<GameplayReadiness>,
    next_transfer_epoch: Arc<AtomicU64>,
    live: Arc<Mutex<LiveState>>,
    login_limits: Arc<Mutex<LoginLimits>>,
    required_tasks: Arc<RequiredTaskLifecycle>,
}

#[derive(Default)]
struct GameplayReadinessState {
    failed: bool,
}

pub(crate) struct GameplayReadiness {
    visible: AtomicBool,
    state: Mutex<GameplayReadinessState>,
}

impl GameplayReadiness {
    fn new() -> Self {
        Self {
            visible: AtomicBool::new(false),
            state: Mutex::new(GameplayReadinessState::default()),
        }
    }

    fn seal_ready(&self) -> Result<(), String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "gameplay readiness coordinator is unavailable".to_string())?;
        if state.failed {
            return Err("a required runtime task failed during startup".to_string());
        }
        self.visible.store(true, Ordering::Release);
        Ok(())
    }

    pub(crate) fn fail(&self) {
        let Ok(mut state) = self.state.lock() else {
            self.visible.store(false, Ordering::Release);
            return;
        };
        state.failed = true;
        self.visible.store(false, Ordering::Release);
    }

    fn is_ready(&self) -> bool {
        self.visible.load(Ordering::Acquire)
    }
}

#[derive(Default)]
struct LiveState {
    active_grants: BTreeMap<wire::CharacterId, ControlGrant>,
    transitioning: BTreeSet<wire::CharacterId>,
}

#[derive(Clone)]
struct RegisteredFacet {
    facet_id: wire::FacetId,
    handle: FacetHandle,
    key: String,
}

struct PreparedCharacterExit {
    epoch: u64,
    facets: Vec<(FacetHandle, crate::facet::PreparedFacetCheckpoint)>,
}

#[derive(Clone)]
pub struct OpaqueSecret(String);

impl OpaqueSecret {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for OpaqueSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("OpaqueSecret([REDACTED])")
    }
}

pub struct LoginSuccess {
    pub session_cookie: OpaqueSecret,
    pub bootstrap: wire::SessionBootstrapV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginError {
    InvalidCredentials,
    RateLimited,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    AuthenticationRequired,
    CsrfRejected,
    CharacterNotOwned,
    CharacterNotSelected,
    GameplayMarkLocked,
    ForgivenessUnavailable,
    Unavailable,
}

struct SessionRow {
    session_id: wire::SessionId,
    account_id: wire::AccountId,
    csrf_digest: [u8; 32],
    selected_character_id: Option<wire::CharacterId>,
}

struct CharacterRow {
    character_id: wire::CharacterId,
    slot: u8,
    display_name: wire::DisplayName,
    actor_id: ActorId,
    control_epoch: u64,
}

mod admission;
mod bootstrap_view;
mod forgiveness;
mod recovery;
mod selection;
mod session_end;
mod session_start;
pub(crate) use recovery::*;
#[cfg(test)]
mod database_fixtures;
#[cfg(test)]
use database_fixtures::*;
#[cfg(test)]
mod database_pipeline;
#[cfg(test)]
use database_pipeline::*;
#[cfg(test)]
mod database_recovery_tests;

struct RequiredTaskReadinessGuard(Arc<GameplayReadiness>);

impl Drop for RequiredTaskReadinessGuard {
    fn drop(&mut self) {
        self.0.fail();
    }
}

#[derive(Default)]
struct RequiredTaskLifecycle {
    reconciler: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl RequiredTaskLifecycle {
    fn install_reconciler(&self, handle: tokio::task::JoinHandle<()>) -> Result<(), String> {
        let mut reconciler = self
            .reconciler
            .lock()
            .map_err(|_| "required-task lifecycle is unavailable".to_string())?;
        if reconciler.is_some() {
            return Err("player-kill reconciler is already installed".to_string());
        }
        *reconciler = Some(handle);
        Ok(())
    }

    #[cfg(test)]
    fn abort_reconciler(&self) -> tokio::task::JoinHandle<()> {
        let handle = self
            .reconciler
            .lock()
            .expect("EV required-task lifecycle lock")
            .take()
            .expect("EV reconciler is installed");
        handle.abort();
        handle
    }
}

impl Drop for RequiredTaskLifecycle {
    fn drop(&mut self) {
        if let Ok(reconciler) = self.reconciler.get_mut()
            && let Some(handle) = reconciler.take()
        {
            handle.abort();
        }
    }
}

fn spawn_player_kill_mark_reconciler(state: &Arc<PostgresState>) -> Result<(), String> {
    let readiness = state.ready.clone();
    let weak_state = Arc::downgrade(state);
    let readiness_guard = RequiredTaskReadinessGuard(readiness);
    let handle = tokio::spawn(async move {
        let _readiness_guard = readiness_guard;
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval.tick().await;
        loop {
            interval.tick().await;
            let Some(state) = weak_state.upgrade() else {
                return;
            };
            if state.store.reconcile_all_player_kill_marks().await.is_err()
                || state.reconcile_expired_sessions().await.is_err()
            {
                state.ready.fail();
                return;
            }
        }
    });
    state.required_tasks.install_reconciler(handle)
}
