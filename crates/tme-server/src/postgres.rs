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
use tme_rules::{ActorId, CharacterId, Engine, FacetCheckpointV4};
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

impl PostgresState {
    pub async fn open(
        database_url: &str,
        bootstrap: PostgresBootstrap,
    ) -> Result<Arc<Self>, String> {
        Self::open_with_credentials(database_url, database_url, bootstrap).await
    }

    pub async fn open_with_credentials(
        database_url: &str,
        auth_database_url: &str,
        bootstrap: PostgresBootstrap,
    ) -> Result<Arc<Self>, String> {
        validate_bootstrap(&bootstrap)?;
        let pool = runtime_pool(database_url).await?;
        let auth_pool = auth_pool(auth_database_url).await?;
        migrations::verify(&pool).await?;
        crate::operator::verify_cluster_identity(&pool).await?;
        let store = Arc::new(PostgresStore::new(pool));
        let auth = AuthService::new().await?;

        let (facet_id, key, engine, revision, sequence) =
            recover_or_initialize(&store, bootstrap).await?;
        store.verify_player_kill_marks().await?;
        store.reconcile_all_player_kill_marks().await?;
        store.verify_player_kill_marks().await?;
        let ready = Arc::new(GameplayReadiness::new());
        let coordinator = Arc::new(Coordinator::new(store.clone()));
        let (handle, startup) = FacetHandle::spawn_persisted(
            facet_id,
            engine,
            revision,
            sequence,
            store.clone(),
            ready.clone(),
            coordinator.clone(),
        );
        startup
            .await
            .map_err(|_| "the persisted world failed before startup acknowledgement".to_string())?;
        let state = Arc::new(Self {
            store,
            auth_pool,
            world: Arc::new(RegisteredFacet {
                facet_id,
                handle,
                key,
            }),
            auth,
            coordinator,
            ready,
            next_transfer_epoch: Arc::new(AtomicU64::new(1)),
            live: Arc::new(Mutex::new(LiveState::default())),
            login_limits: Arc::new(Mutex::new(LoginLimits::default())),
            required_tasks: Arc::new(RequiredTaskLifecycle::default()),
        });
        state.reconcile_expired_sessions().await?;
        spawn_player_kill_mark_reconciler(&state)?;
        state.ready.seal_ready()?;
        Ok(state)
    }

    pub fn gameplay_ready(&self) -> bool {
        self.ready.is_ready()
    }

    async fn commit_gameplay_transaction(
        &self,
        transaction: Transaction<'_, Postgres>,
    ) -> Result<(), sqlx::Error> {
        let result = transaction.commit().await;
        if result.is_err() {
            self.ready.fail();
        }
        result
    }

    pub(crate) fn coordinator(&self) -> Arc<Coordinator> {
        self.coordinator.clone()
    }

    pub fn facet_id_for_key(&self, key: &str) -> Option<wire::FacetId> {
        (self.world.key == key).then_some(self.world.facet_id)
    }

    pub(crate) fn maximum_mailbox_depth(&self) -> usize {
        self.world.handle.mailbox_depth()
    }

    pub(crate) async fn restore_fence_epoch(&self) -> Result<u64, String> {
        let value: i64 =
            sqlx::query_scalar("SELECT restore_fence_epoch FROM tme.store_state WHERE singleton")
                .fetch_one(self.store.pool())
                .await
                .map_err(|error| error.to_string())?;
        checked_u64(value)
    }

    pub async fn login(
        &self,
        source: IpAddr,
        request: wire::LoginRequestV1,
    ) -> Result<LoginSuccess, LoginError> {
        if !self.gameplay_ready() {
            return Err(LoginError::Unavailable);
        }
        {
            let mut limits = self
                .login_limits
                .lock()
                .map_err(|_| LoginError::Unavailable)?;
            if !limits.allow_source(source) {
                return Err(LoginError::RateLimited);
            }
        }
        let row = sqlx::query(
            "SELECT a.account_id, c.password_phc FROM tme.accounts a \
             JOIN tme.account_credentials c USING (account_id) \
             WHERE a.username=$1 AND a.status='active'",
        )
        .bind(request.username.as_str())
        .fetch_optional(&self.auth_pool)
        .await
        .map_err(|_| LoginError::Unavailable)?;
        let (account_id, phc) = match row {
            Some(row) => {
                let id: Uuid = row
                    .try_get("account_id")
                    .map_err(|_| LoginError::Unavailable)?;
                let account_id = wire::AccountId::new(id).map_err(|_| LoginError::Unavailable)?;
                let mut limits = self
                    .login_limits
                    .lock()
                    .map_err(|_| LoginError::Unavailable)?;
                if !limits.allow_account(account_id) {
                    return Err(LoginError::RateLimited);
                }
                let phc = row
                    .try_get("password_phc")
                    .map_err(|_| LoginError::Unavailable)?;
                (Some(account_id), phc)
            }
            None => (None, self.auth.dummy_phc.as_ref().clone()),
        };
        let verification = self
            .auth
            .verify(request.password.expose_for_verification(), phc)
            .await
            .map_err(|_| LoginError::Unavailable)?;
        let Some(account_id) = account_id.filter(|_| verification.verified) else {
            return Err(LoginError::InvalidCredentials);
        };
        let replacement = if verification.needs_rehash {
            Some(
                self.auth
                    .hash(request.password.expose_for_verification())
                    .await
                    .map_err(|_| LoginError::Unavailable)?,
            )
        } else {
            None
        };
        let session_cookie = random_secret().map_err(|_| LoginError::Unavailable)?;
        let csrf = random_csrf().map_err(|_| LoginError::Unavailable)?;
        let session_id =
            wire::SessionId::new(Uuid::now_v7()).map_err(|_| LoginError::Unavailable)?;
        let mut tx = serializable(&self.auth_pool)
            .await
            .map_err(|_| LoginError::Unavailable)?;
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM tme.sessions WHERE account_id=$1 AND revoked_at IS NULL \
             AND idle_expires_at > statement_timestamp() AND absolute_expires_at > statement_timestamp()",
        )
        .bind(account_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| LoginError::Unavailable)?;
        if count >= MAX_SESSIONS_PER_ACCOUNT as i64 {
            return Err(LoginError::Unavailable);
        }
        if let Some(replacement) = replacement {
            sqlx::query(
                "UPDATE tme.account_credentials SET password_phc=$2, \
                 credential_updated_at=statement_timestamp() WHERE account_id=$1",
            )
            .bind(account_id.as_uuid())
            .bind(replacement)
            .execute(&mut *tx)
            .await
            .map_err(|_| LoginError::Unavailable)?;
        }
        sqlx::query(
            "INSERT INTO tme.sessions \
             (session_id,account_id,token_digest,csrf_digest,idle_expires_at,absolute_expires_at) \
             VALUES ($1,$2,$3,$4,statement_timestamp()+make_interval(secs=>$5), \
                     statement_timestamp()+make_interval(secs=>$6))",
        )
        .bind(session_id.as_uuid())
        .bind(account_id.as_uuid())
        .bind(digest(session_cookie.expose()).as_slice())
        .bind(digest(csrf.expose_for_validation()).as_slice())
        .bind(checked_i64(SESSION_IDLE.as_secs()).map_err(|_| LoginError::Unavailable)?)
        .bind(checked_i64(SESSION_ABSOLUTE.as_secs()).map_err(|_| LoginError::Unavailable)?)
        .execute(&mut *tx)
        .await
        .map_err(|_| LoginError::Unavailable)?;
        audit(
            &mut tx,
            AuditEvent {
                account_id: Some(account_id.as_uuid()),
                session_id: Some(session_id.as_uuid()),
                character_id: None,
                command_id: None,
                actor: "runtime",
                action: "login",
                result: "success",
            },
        )
        .await
        .map_err(|_| LoginError::Unavailable)?;
        tx.commit().await.map_err(|_| LoginError::Unavailable)?;
        if let Ok(mut limits) = self.login_limits.lock() {
            limits.refund_source(source);
            limits.clear_account(account_id);
        }
        let bootstrap = self
            .bootstrap_for(session_id, account_id, csrf, None)
            .await
            .map_err(|_| LoginError::Unavailable)?;
        Ok(LoginSuccess {
            session_cookie,
            bootstrap,
        })
    }

    pub async fn session_bootstrap(
        &self,
        session_cookie: &str,
    ) -> Result<wire::SessionBootstrapV1, SessionError> {
        let csrf = random_csrf().map_err(|_| SessionError::Unavailable)?;
        let mut tx = serializable(self.store.pool())
            .await
            .map_err(|_| SessionError::Unavailable)?;
        let session = active_session(&mut tx, session_cookie, true)
            .await?
            .ok_or(SessionError::AuthenticationRequired)?;
        sqlx::query("UPDATE tme.sessions SET csrf_digest=$2 WHERE session_id=$1")
            .bind(session.session_id.as_uuid())
            .bind(digest(csrf.expose_for_validation()).as_slice())
            .execute(&mut *tx)
            .await
            .map_err(|_| SessionError::Unavailable)?;
        tx.commit().await.map_err(|_| SessionError::Unavailable)?;
        self.bootstrap_for(
            session.session_id,
            session.account_id,
            csrf,
            session.selected_character_id,
        )
        .await
    }

    /// Builds the candidate engine carrying every consequence this returning
    /// killer owes, and hands back the checkpoint plus the per-kill
    /// `linked_karma_added` the rules produced. Nothing is durable yet; the
    /// caller's transaction decides whether this lands.
    async fn prepare_pending_consequences(
        &self,
        facet: &FacetHandle,
        pending: &[crate::store::PendingKillConsequence],
    ) -> Result<(u64, crate::facet::PreparedFacetCheckpoint, Vec<bool>), SessionError> {
        let epoch = self
            .next_transfer_epoch
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| SessionError::Unavailable)?;
        let assessments = pending
            .iter()
            .map(|owed| owed.assessment.clone())
            .collect::<Vec<_>>();
        let linked = facet
            .prepare_pending_kill_consequences(epoch, assessments)
            .await
            .map_err(|_| SessionError::Unavailable)?;
        let checkpoint = match facet.prepared_checkpoint(epoch).await {
            Ok(checkpoint) => checkpoint,
            Err(_) => {
                let _ = facet.rollback_transfer(epoch).await;
                return Err(SessionError::Unavailable);
            }
        };
        if checkpoint.facet_id != self.world.facet_id || linked.len() != pending.len() {
            let _ = facet.rollback_transfer(epoch).await;
            return Err(SessionError::Unavailable);
        }
        Ok((epoch, checkpoint, linked))
    }

    async fn prepare_character_exit_candidate(
        &self,
        character_id: wire::CharacterId,
    ) -> Result<PreparedCharacterExit, SessionError> {
        let epoch = self
            .next_transfer_epoch
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| SessionError::Unavailable)?;
        let rules_character_id = CharacterId::new(character_id.to_string());
        let mut facets = Vec::new();
        {
            let handle = self.world.handle.clone();
            if handle
                .prepare_character_exit(epoch, rules_character_id.clone())
                .await
                .is_err()
            {
                Self::rollback_character_exit(epoch, &facets).await;
                return Err(SessionError::Unavailable);
            }
            let checkpoint = match handle.prepared_checkpoint(epoch).await {
                Ok(checkpoint) => checkpoint,
                Err(_) => {
                    let _ = handle.rollback_transfer(epoch).await;
                    Self::rollback_character_exit(epoch, &facets).await;
                    return Err(SessionError::Unavailable);
                }
            };
            facets.push((handle, checkpoint));
        }
        facets.sort_by_key(|(_, checkpoint)| checkpoint.facet_id);
        Ok(PreparedCharacterExit { epoch, facets })
    }

    async fn persist_prepared_facets(
        tx: &mut Transaction<'_, Postgres>,
        facets: &[(FacetHandle, crate::facet::PreparedFacetCheckpoint)],
    ) -> Result<(), SessionError> {
        for (_, checkpoint) in facets {
            let row = sqlx::query(
                "SELECT facet_revision,last_server_sequence FROM tme.facets \
                 WHERE facet_id=$1 FOR UPDATE",
            )
            .bind(checkpoint.facet_id.as_uuid())
            .fetch_one(&mut **tx)
            .await
            .map_err(unavailable)?;
            if checked_u64(row.try_get("facet_revision").map_err(unavailable)?)
                .map_err(unavailable)?
                != checkpoint.before_revision
                || checked_u64(row.try_get("last_server_sequence").map_err(unavailable)?)
                    .map_err(unavailable)?
                    != checkpoint.server_sequence
            {
                return Err(SessionError::Unavailable);
            }
            let updated = sqlx::query(
                "UPDATE tme.facets SET checkpoint_bytes=$2,checkpoint_sha256=$3, \
                 facet_revision=$4,updated_at=statement_timestamp() WHERE facet_id=$1 \
                 AND facet_revision=$5 AND last_server_sequence=$6",
            )
            .bind(checkpoint.facet_id.as_uuid())
            .bind(checkpoint.checkpoint.as_bytes())
            .bind(checkpoint.checkpoint.sha256().as_slice())
            .bind(checked_i64(checkpoint.after_revision).map_err(unavailable)?)
            .bind(checked_i64(checkpoint.before_revision).map_err(unavailable)?)
            .bind(checked_i64(checkpoint.server_sequence).map_err(unavailable)?)
            .execute(&mut **tx)
            .await
            .map_err(unavailable)?;
            if updated.rows_affected() != 1 {
                return Err(SessionError::Unavailable);
            }
        }
        Ok(())
    }

    async fn rollback_character_exit(
        epoch: u64,
        facets: &[(FacetHandle, crate::facet::PreparedFacetCheckpoint)],
    ) {
        for (handle, _) in facets {
            let _ = handle.rollback_transfer(epoch).await;
        }
    }

    async fn publish_character_exit(
        &self,
        prepared: &PreparedCharacterExit,
    ) -> Result<(), SessionError> {
        for (handle, _) in &prepared.facets {
            if handle.commit_transfer(prepared.epoch).await.is_err() {
                self.ready.fail();
                return Err(SessionError::Unavailable);
            }
        }
        for (handle, _) in &prepared.facets {
            if handle.publish_transfer(prepared.epoch).await.is_err() {
                self.ready.fail();
                return Err(SessionError::Unavailable);
            }
        }
        Ok(())
    }

    pub async fn select_character(
        &self,
        session_cookie: &str,
        request: wire::CharacterSelectRequestV1,
    ) -> Result<wire::CharacterSelectionV1, SessionError> {
        let _transition = self.coordinator.transition().await;
        let mut tx = serializable(self.store.pool()).await.map_err(unavailable)?;
        let session = active_session(&mut tx, session_cookie, true)
            .await?
            .ok_or(SessionError::AuthenticationRequired)?;
        validate_csrf(session.csrf_digest, &request.csrf_token)?;
        let character = character_for_account(&mut tx, request.character_id, session.account_id)
            .await?
            .ok_or(SessionError::CharacterNotOwned)?;
        let replacing_character = session
            .selected_character_id
            .filter(|selected| *selected != character.character_id);
        let replaced_grant = replacing_character.and_then(|replaced| {
            self.live
                .lock()
                .ok()
                .and_then(|live| live.active_grants.get(&replaced).cloned())
        });
        let prepared_exit = match replacing_character {
            Some(replaced) => Some(self.prepare_character_exit_candidate(replaced).await?),
            None => None,
        };
        let durable = async {
            if let (Some(replaced), Some(prepared)) = (replacing_character, &prepared_exit) {
                Self::persist_prepared_facets(&mut tx, &prepared.facets).await?;
                sqlx::query(
                    "UPDATE tme.player_kill_marks SET karma_forgiveness_eligible=false \
                     WHERE forgiven_at IS NULL AND expired_at IS NULL \
                     AND karma_forgiveness_eligible AND ( \
                        (killer_character_id=$1 AND killer_session_id=$2) OR \
                        (victim_character_id=$1 AND victim_session_id=$2))",
                )
                .bind(replaced.as_uuid())
                .bind(session.session_id.as_uuid())
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
                sqlx::query(
                    "UPDATE tme.characters SET control_epoch=control_epoch+1 \
                     WHERE character_id=$1 AND account_id=$2",
                )
                .bind(replaced.as_uuid())
                .bind(session.account_id.as_uuid())
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
                sqlx::query(
                    "DELETE FROM tme.socket_tickets WHERE session_id=$1 AND character_id=$2 \
                     AND consumed_at IS NULL",
                )
                .bind(session.session_id.as_uuid())
                .bind(replaced.as_uuid())
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
            }
            sqlx::query("UPDATE tme.sessions SET selected_character_id=$2 WHERE session_id=$1")
                .bind(session.session_id.as_uuid())
                .bind(character.character_id.as_uuid())
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
            self.commit_gameplay_transaction(tx)
                .await
                .map_err(unavailable)
        }
        .await;
        if let Err(error) = durable {
            if let Some(prepared) = &prepared_exit {
                Self::rollback_character_exit(prepared.epoch, &prepared.facets).await;
            }
            return Err(error);
        }
        let revocation = if let Some(grant) = replaced_grant {
            if let Ok(mut live) = self.live.lock() {
                live.active_grants.remove(&grant.character_id);
            }
            if grant.facet_id == self.world.facet_id {
                let facet = &self.world;
                Some(
                    facet
                        .handle
                        .begin_revoke_grant(grant.connection_id, wire::DrainingReason::SessionEnded)
                        .await
                        .map_err(|_| SessionError::Unavailable)?,
                )
            } else {
                None
            }
        } else {
            None
        };
        if let Some(prepared) = &prepared_exit {
            self.publish_character_exit(prepared).await?;
        }
        if let Some(revocation) = revocation {
            revocation.await.map_err(|_| SessionError::Unavailable)?;
        }
        Ok(selection(&character))
    }

    pub async fn issue_ticket(
        &self,
        session_cookie: &str,
        request: wire::SocketTicketRequestV1,
        origin: &str,
        host: &str,
    ) -> Result<wire::SocketTicketV1, SessionError> {
        let _transition = self.coordinator.transition().await;
        let ticket = random_ticket().map_err(|_| SessionError::Unavailable)?;
        let mut tx = serializable(self.store.pool()).await.map_err(unavailable)?;
        let session = active_session(&mut tx, session_cookie, true)
            .await?
            .ok_or(SessionError::AuthenticationRequired)?;
        validate_csrf(session.csrf_digest, &request.csrf_token)?;
        sqlx::query("SELECT account_id FROM tme.accounts WHERE account_id=$1 FOR UPDATE")
            .bind(session.account_id.as_uuid())
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?;
        crate::store::reschedule_player_kill_marks_raw(
            &mut tx,
            session.account_id.as_uuid(),
            false,
        )
        .await
        .map_err(unavailable)?;
        let active_marks: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM tme.player_kill_marks WHERE killer_account_id=$1 \
             AND forgiven_at IS NULL AND expired_at IS NULL",
        )
        .bind(session.account_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(unavailable)?;
        if active_marks >= 4 {
            return Err(SessionError::GameplayMarkLocked);
        }
        let character_id = session
            .selected_character_id
            .ok_or(SessionError::CharacterNotSelected)?;
        let character = character_for_account(&mut tx, character_id, session.account_id)
            .await?
            .ok_or(SessionError::Unavailable)?;
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM tme.socket_tickets WHERE session_id=$1 AND \
             (consumed_at IS NULL OR consumed_at > statement_timestamp()-interval '30 seconds')",
        )
        .bind(session.session_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(unavailable)?;
        if count >= MAX_TICKETS_PER_SESSION as i64 {
            return Err(SessionError::Unavailable);
        }
        sqlx::query(
            "INSERT INTO tme.socket_tickets \
             (ticket_digest,session_id,account_id,character_id,actor_id, \
              expected_control_epoch,origin,host,selected_major,expires_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,1, \
                     statement_timestamp()+make_interval(secs=>$9))",
        )
        .bind(digest(ticket.expose_for_admission()).as_slice())
        .bind(session.session_id.as_uuid())
        .bind(session.account_id.as_uuid())
        .bind(character.character_id.as_uuid())
        .bind(character.actor_id.as_str())
        .bind(checked_i64(character.control_epoch).map_err(unavailable)?)
        .bind(origin)
        .bind(host)
        .bind(checked_i64(TICKET_LIFETIME.as_secs()).map_err(unavailable)?)
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        Ok(wire::SocketTicketV1 {
            ticket,
            protocol_major: wire::PROTOCOL_MAJOR,
            supported_minors: vec![wire::PROTOCOL_MINOR],
            expires_in_seconds: wire::DecimalU64::new(TICKET_LIFETIME.as_secs()),
        })
    }

    pub async fn admit(
        &self,
        ticket: &wire::AdmissionTicket,
        supported_minors: &[u16],
        origin: &str,
        host: &str,
        outbound: mpsc::Sender<wire::ServerEnvelope>,
        terminal: watch::Sender<Option<wire::DrainingReason>>,
    ) -> Result<(AdmissionGrant, FacetWelcome), AdmissionError> {
        let _transition = self.coordinator.transition().await;
        if !self.gameplay_ready() {
            return Err(AdmissionError::Unavailable);
        }
        let mut tx = serializable(self.store.pool())
            .await
            .map_err(|_| AdmissionError::Unavailable)?;
        let row = sqlx::query(
            "SELECT t.session_id,t.account_id,t.character_id,t.actor_id, \
                    t.expected_control_epoch,t.origin,t.host, \
                    t.expires_at <= statement_timestamp() AS expired,t.consumed_at IS NOT NULL AS consumed \
             FROM tme.socket_tickets t WHERE ticket_digest=$1 FOR UPDATE",
        )
        .bind(digest(ticket.expose_for_admission()).as_slice())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| AdmissionError::Unavailable)?
        .ok_or(AdmissionError::InvalidTicket)?;
        if row
            .try_get::<bool, _>("consumed")
            .map_err(|_| AdmissionError::Unavailable)?
        {
            return Err(AdmissionError::ConsumedTicket);
        }
        sqlx::query("UPDATE tme.socket_tickets SET consumed_at=statement_timestamp() WHERE ticket_digest=$1")
            .bind(digest(ticket.expose_for_admission()).as_slice())
            .execute(&mut *tx)
            .await
            .map_err(|_| AdmissionError::Unavailable)?;
        if !supported_minors.contains(&wire::PROTOCOL_MINOR) {
            tx.commit().await.map_err(|_| AdmissionError::Unavailable)?;
            return Err(AdmissionError::UnsupportedVersion);
        }
        if row
            .try_get::<bool, _>("expired")
            .map_err(|_| AdmissionError::Unavailable)?
        {
            tx.commit().await.map_err(|_| AdmissionError::Unavailable)?;
            return Err(AdmissionError::ExpiredTicket);
        }
        if row
            .try_get::<String, _>("origin")
            .map_err(|_| AdmissionError::Unavailable)?
            != origin
        {
            tx.commit().await.map_err(|_| AdmissionError::Unavailable)?;
            return Err(AdmissionError::OriginRejected);
        }
        if row
            .try_get::<String, _>("host")
            .map_err(|_| AdmissionError::Unavailable)?
            != host
        {
            tx.commit().await.map_err(|_| AdmissionError::Unavailable)?;
            return Err(AdmissionError::HostRejected);
        }
        let session_id = wire::SessionId::new(
            row.try_get("session_id")
                .map_err(|_| AdmissionError::Unavailable)?,
        )
        .map_err(|_| AdmissionError::Unavailable)?;
        let account_id = wire::AccountId::new(
            row.try_get("account_id")
                .map_err(|_| AdmissionError::Unavailable)?,
        )
        .map_err(|_| AdmissionError::Unavailable)?;
        let character_id = wire::CharacterId::new(
            row.try_get("character_id")
                .map_err(|_| AdmissionError::Unavailable)?,
        )
        .map_err(|_| AdmissionError::Unavailable)?;
        // D4: one world. The admitted grant binds to the world this process
        // hosts; the ticket never named one.
        let facet_id = self.world.facet_id;
        let actor_id = ActorId::new(
            row.try_get::<String, _>("actor_id")
                .map_err(|_| AdmissionError::Unavailable)?,
        );
        let expected_epoch = checked_u64(
            row.try_get("expected_control_epoch")
                .map_err(|_| AdmissionError::Unavailable)?,
        )
        .map_err(|_| AdmissionError::Unavailable)?;
        let session_ok: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM tme.sessions WHERE session_id=$1 AND account_id=$2 \
             AND selected_character_id=$3 AND revoked_at IS NULL \
             AND idle_expires_at>statement_timestamp() AND absolute_expires_at>statement_timestamp())",
        )
        .bind(session_id.as_uuid())
        .bind(account_id.as_uuid())
        .bind(character_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| AdmissionError::Unavailable)?;
        if !session_ok {
            tx.commit().await.map_err(|_| AdmissionError::Unavailable)?;
            return Err(AdmissionError::Unavailable);
        }
        sqlx::query("SELECT account_id FROM tme.accounts WHERE account_id=$1 FOR UPDATE")
            .bind(account_id.as_uuid())
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| AdmissionError::Unavailable)?;
        crate::store::reschedule_player_kill_marks_raw(&mut tx, account_id.as_uuid(), false)
            .await
            .map_err(|_| AdmissionError::Unavailable)?;
        let active_marks: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM tme.player_kill_marks WHERE killer_account_id=$1 \
             AND forgiven_at IS NULL AND expired_at IS NULL",
        )
        .bind(account_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| AdmissionError::Unavailable)?;
        if active_marks >= 4 {
            tx.commit().await.map_err(|_| AdmissionError::Unavailable)?;
            return Err(AdmissionError::GameplayMarkLocked);
        }
        // Ticket consumption and static request validation finish before the
        // facet is quiesced. The control transaction begins afterward so a
        // scheduler commit cannot invalidate an old serializable snapshot.
        tx.commit().await.map_err(|_| AdmissionError::Unavailable)?;
        let next_epoch = expected_epoch
            .checked_add(1)
            .ok_or(AdmissionError::Unavailable)?;
        if facet_id != self.world.facet_id {
            return Err(AdmissionError::Unavailable);
        }
        let facet = self.world.handle.clone();
        {
            let mut live = self.live.lock().map_err(|_| AdmissionError::Unavailable)?;
            if !live.transitioning.insert(character_id) {
                return Err(AdmissionError::Unavailable);
            }
        }
        if facet.prepare_control(character_id).await.is_err() {
            self.clear_transition(character_id);
            return Err(AdmissionError::Unavailable);
        }
        let grant = ControlGrant::new(
            account_id,
            session_id,
            wire::ConnectionId::new(Uuid::now_v7()).map_err(|_| AdmissionError::Unavailable)?,
            character_id,
            facet_id,
            actor_id.clone(),
            next_epoch,
        );
        // Owner ruling 2026-08-20 (#3): a killer who logged off before a delayed
        // kill landed still owes the karma. Pay it here, before they see the
        // world, and clear it in the same transaction that makes the applied
        // sheet durable. The candidate is prepared BEFORE the control
        // transaction opens, for the same reason forgiveness does it: the facet
        // task must never be waiting on a SQL row this transaction holds.
        let pending = match crate::store::pending_kill_consequences(
            self.store.pool(),
            account_id.as_uuid(),
            character_id.as_uuid(),
        )
        .await
        {
            Ok(pending) => pending,
            Err(_) => {
                self.clear_transition(character_id);
                let _ = facet.resume_control(character_id).await;
                return Err(AdmissionError::Unavailable);
            }
        };
        let prepared_pending = if pending.is_empty() {
            None
        } else {
            match self.prepare_pending_consequences(&facet, &pending).await {
                Ok(prepared) => Some(prepared),
                Err(_) => {
                    self.clear_transition(character_id);
                    let _ = facet.resume_control(character_id).await;
                    return Err(AdmissionError::Unavailable);
                }
            }
        };
        let mut control_tx = match serializable(self.store.pool()).await {
            Ok(tx) => tx,
            Err(_) => {
                if let Some((epoch, _, _)) = &prepared_pending {
                    let _ = facet.rollback_transfer(*epoch).await;
                }
                self.clear_transition(character_id);
                let _ = facet.resume_control(character_id).await;
                return Err(AdmissionError::Unavailable);
            }
        };
        let durable = async {
            sqlx::query("SELECT facet_id FROM tme.facets WHERE facet_id=$1 FOR UPDATE")
                .bind(facet_id.as_uuid())
                .fetch_one(&mut *control_tx)
                .await
                .map_err(|error| error.to_string())?;
            sqlx::query("SELECT account_id FROM tme.accounts WHERE account_id=$1 FOR UPDATE")
                .bind(account_id.as_uuid())
                .fetch_one(&mut *control_tx)
                .await
                .map_err(|error| error.to_string())?;
            if let Some((_, checkpoint, linked)) = &prepared_pending {
                let updated = sqlx::query(
                    "UPDATE tme.facets SET checkpoint_bytes=$2,checkpoint_sha256=$3, \
                     facet_revision=$4,updated_at=statement_timestamp() WHERE facet_id=$1 \
                     AND facet_revision=$5 AND last_server_sequence=$6",
                )
                .bind(checkpoint.facet_id.as_uuid())
                .bind(checkpoint.checkpoint.as_bytes())
                .bind(checkpoint.checkpoint.sha256().as_slice())
                .bind(checked_i64(checkpoint.after_revision)?)
                .bind(checked_i64(checkpoint.before_revision)?)
                .bind(checked_i64(checkpoint.server_sequence)?)
                .execute(&mut *control_tx)
                .await
                .map_err(|error| error.to_string())?;
                if updated.rows_affected() != 1 {
                    return Err("world revision moved during admission".to_string());
                }
                // Clearing rides the same transaction as the checkpoint above.
                // Crash before it commits and the rows survive to be applied at
                // the next admission; crash after and they are gone for good.
                for (owed, linked_karma_added) in pending.iter().zip(linked) {
                    crate::store::clear_pending_kill_consequence_raw(
                        &mut control_tx,
                        owed.facet_kill_sequence,
                        *linked_karma_added,
                    )
                    .await
                    .map_err(|error| error.to_string())?;
                }
            }
            crate::store::reschedule_player_kill_marks_raw(
                &mut control_tx,
                account_id.as_uuid(),
                false,
            )
            .await
            .map_err(|error| error.to_string())?;
            let active_marks: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM tme.player_kill_marks WHERE killer_account_id=$1 \
                 AND forgiven_at IS NULL AND expired_at IS NULL",
            )
            .bind(account_id.as_uuid())
            .fetch_one(&mut *control_tx)
            .await
            .map_err(|error| error.to_string())?;
            if active_marks >= 4 {
                return Err("account became gameplay-mark locked".to_string());
            }
            let session_ok: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM tme.sessions WHERE session_id=$1 \
                 AND account_id=$2 AND selected_character_id=$3 AND revoked_at IS NULL \
                 AND idle_expires_at>statement_timestamp() \
                 AND absolute_expires_at>statement_timestamp())",
            )
            .bind(session_id.as_uuid())
            .bind(account_id.as_uuid())
            .bind(character_id.as_uuid())
            .fetch_one(&mut *control_tx)
            .await
            .map_err(|error| error.to_string())?;
            if !session_ok {
                return Err("session authority changed during admission".to_string());
            }
            let updated = sqlx::query(
                "UPDATE tme.characters SET control_epoch=$2 WHERE character_id=$1 \
                 AND account_id=$3 AND actor_id=$4 AND control_epoch=$5",
            )
            .bind(character_id.as_uuid())
            .bind(checked_i64(next_epoch)?)
            .bind(account_id.as_uuid())
            .bind(actor_id.as_str())
            .bind(checked_i64(expected_epoch)?)
            .execute(&mut *control_tx)
            .await
            .map_err(|error| error.to_string())?;
            if updated.rows_affected() != 1 {
                return Err("character control epoch changed".to_string());
            }
            sqlx::query("UPDATE tme.sessions SET last_seen_at=statement_timestamp(), idle_expires_at=LEAST(absolute_expires_at,statement_timestamp()+make_interval(secs=>$2)) WHERE session_id=$1")
                .bind(session_id.as_uuid())
                .bind(checked_i64(SESSION_IDLE.as_secs())?)
                .execute(&mut *control_tx)
                .await
                .map_err(|error| error.to_string())?;
            audit(
                &mut control_tx,
                AuditEvent {
                    account_id: Some(account_id.as_uuid()),
                    session_id: Some(session_id.as_uuid()),
                    character_id: Some(character_id.as_uuid()),
                    command_id: None,
                    actor: "runtime",
                    action: "admit",
                    result: "success",
                },
            )
            .await?;
            self.commit_gameplay_transaction(control_tx)
                .await
                .map_err(|error| error.to_string())
        }
        .await;
        if durable.is_err() {
            if let Some((epoch, _, _)) = &prepared_pending {
                let _ = facet.rollback_transfer(*epoch).await;
            }
            self.clear_transition(character_id);
            let _ = facet.resume_control(character_id).await;
            return Err(AdmissionError::Unavailable);
        }
        if let Some((epoch, _, _)) = &prepared_pending
            && (facet.commit_transfer(*epoch).await.is_err()
                || facet.publish_transfer(*epoch).await.is_err())
        {
            self.ready.fail();
            return Err(AdmissionError::Unavailable);
        }
        let welcome = match facet.install_grant(grant.clone(), outbound, terminal).await {
            Ok(value) => value,
            Err(_) => {
                self.ready.fail();
                return Err(AdmissionError::Unavailable);
            }
        };
        {
            let mut live = self.live.lock().map_err(|_| AdmissionError::Unavailable)?;
            live.transitioning.remove(&character_id);
            live.active_grants.insert(character_id, grant.clone());
        }
        Ok((
            AdmissionGrant {
                control: grant,
                facet,
            },
            welcome,
        ))
    }

    pub async fn authorize_grant(&self, grant: &ControlGrant) -> bool {
        if !self.gameplay_ready()
            || !self.live.lock().ok().is_some_and(|live| {
                !live.transitioning.contains(&grant.character_id)
                    && live.active_grants.get(&grant.character_id) == Some(grant)
            })
        {
            return false;
        }
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM tme.sessions s JOIN tme.characters c \
             ON c.character_id=$3 WHERE s.session_id=$1 AND s.account_id=$2 \
             AND s.revoked_at IS NULL AND s.idle_expires_at>statement_timestamp() \
             AND s.absolute_expires_at>statement_timestamp() AND c.control_epoch=$4 \
             AND c.account_id=$2 AND c.actor_id=$5)",
        )
        .bind(grant.session_id.as_uuid())
        .bind(grant.account_id.as_uuid())
        .bind(grant.character_id.as_uuid())
        .bind(checked_i64(grant.control_epoch).unwrap_or(-1))
        .bind(grant.actor_id.as_str())
        .fetch_one(self.store.pool())
        .await
        .unwrap_or(false)
    }

    pub(crate) async fn deliver_page(
        &self,
        target_character_id: wire::CharacterId,
        message_id: wire::MessageId,
        sender_character_id: wire::CharacterId,
        sender_name: wire::DisplayName,
        body: wire::SocialBody,
    ) -> bool {
        if !self.gameplay_ready() {
            return false;
        }
        let target = self.live.lock().ok().and_then(|live| {
            (!live.transitioning.contains(&target_character_id))
                .then(|| live.active_grants.get(&target_character_id).cloned())
                .flatten()
        });
        let Some(target) = target else {
            return false;
        };
        if target.facet_id != self.world.facet_id {
            return false;
        }
        let facet = self.world.handle.clone();
        facet
            .deliver_page(target, message_id, sender_character_id, sender_name, body)
            .await
            .unwrap_or(false)
    }

    pub async fn logout(
        &self,
        session_cookie: &str,
        request: wire::LogoutRequestV1,
    ) -> Result<(), SessionError> {
        let _transition = self.coordinator.transition().await;
        let mut tx = serializable(self.store.pool()).await.map_err(unavailable)?;
        let session = active_session(&mut tx, session_cookie, false)
            .await?
            .ok_or(SessionError::AuthenticationRequired)?;
        validate_csrf(session.csrf_digest, &request.csrf_token)?;
        let grants = {
            let live = self.live.lock().map_err(|_| SessionError::Unavailable)?;
            live.active_grants
                .values()
                .filter(|grant| grant.session_id == session.session_id)
                .cloned()
                .collect::<Vec<_>>()
        };
        let prepared_exit = match session.selected_character_id {
            Some(character_id) => Some(self.prepare_character_exit_candidate(character_id).await?),
            None => None,
        };
        let durable = async {
            if let Some(prepared) = &prepared_exit {
                Self::persist_prepared_facets(&mut tx, &prepared.facets).await?;
            }
            sqlx::query(
                "UPDATE tme.sessions SET revoked_at=statement_timestamp() WHERE session_id=$1",
            )
            .bind(session.session_id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
            sqlx::query(
                "UPDATE tme.player_kill_marks SET karma_forgiveness_eligible=false \
                 WHERE forgiven_at IS NULL AND expired_at IS NULL \
                 AND karma_forgiveness_eligible \
                 AND (killer_session_id=$1 OR victim_session_id=$1)",
            )
            .bind(session.session_id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
            sqlx::query(
                "DELETE FROM tme.socket_tickets WHERE session_id=$1 AND consumed_at IS NULL",
            )
            .bind(session.session_id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
            if let Some(character_id) = session.selected_character_id {
                sqlx::query(
                    "UPDATE tme.characters SET control_epoch=control_epoch+1 \
                     WHERE character_id=$1 AND account_id=$2",
                )
                .bind(character_id.as_uuid())
                .bind(session.account_id.as_uuid())
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
            }
            audit(
                &mut tx,
                AuditEvent {
                    account_id: Some(session.account_id.as_uuid()),
                    session_id: Some(session.session_id.as_uuid()),
                    character_id: session
                        .selected_character_id
                        .map(|character_id| character_id.as_uuid()),
                    command_id: None,
                    actor: "runtime",
                    action: "logout",
                    result: "success",
                },
            )
            .await
            .map_err(unavailable)?;
            self.commit_gameplay_transaction(tx)
                .await
                .map_err(unavailable)
        }
        .await;
        if let Err(error) = durable {
            if let Some(prepared) = &prepared_exit {
                Self::rollback_character_exit(prepared.epoch, &prepared.facets).await;
            }
            return Err(error);
        }
        let mut revocations = Vec::new();
        for grant in grants {
            if let Ok(mut live) = self.live.lock() {
                live.active_grants.remove(&grant.character_id);
            }
            if grant.facet_id == self.world.facet_id {
                let facet = &self.world;
                revocations.push(
                    facet
                        .handle
                        .begin_revoke_grant(grant.connection_id, wire::DrainingReason::SessionEnded)
                        .await
                        .map_err(|_| SessionError::Unavailable)?,
                );
            }
        }
        if let Some(prepared) = &prepared_exit {
            self.publish_character_exit(prepared).await?;
        }
        for revocation in revocations {
            revocation.await.map_err(|_| SessionError::Unavailable)?;
        }
        Ok(())
    }

    async fn reconcile_expired_sessions(&self) -> Result<(), String> {
        let session_ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT session_id FROM tme.sessions WHERE revoked_at IS NULL \
             AND (idle_expires_at<=statement_timestamp() OR \
                  absolute_expires_at<=statement_timestamp()) \
             ORDER BY session_id LIMIT 64",
        )
        .fetch_all(self.store.pool())
        .await
        .map_err(|error| error.to_string())?;
        for session_id in session_ids {
            self.expire_session(session_id).await?;
        }
        Ok(())
    }

    async fn expire_session(&self, session_id: Uuid) -> Result<(), String> {
        let _transition = self.coordinator.transition().await;
        let mut tx = serializable(self.store.pool()).await?;
        let row = sqlx::query(
            "SELECT session_id,account_id,csrf_digest,selected_character_id \
             FROM tme.sessions WHERE session_id=$1 AND revoked_at IS NULL \
             AND (idle_expires_at<=statement_timestamp() OR \
                  absolute_expires_at<=statement_timestamp()) FOR UPDATE",
        )
        .bind(session_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
        let Some(row) = row else {
            tx.rollback().await.map_err(|error| error.to_string())?;
            return Ok(());
        };
        let session = decode_session(row).map_err(|error| format!("{error:?}"))?;
        let grants = self
            .live
            .lock()
            .map_err(|_| "live grant registry is unavailable".to_string())?
            .active_grants
            .values()
            .filter(|grant| grant.session_id == session.session_id)
            .cloned()
            .collect::<Vec<_>>();
        let mut prepared = Vec::<(FacetHandle, crate::facet::PreparedFacetCheckpoint)>::new();
        let mut exit_epoch = None;
        if let Some(character_id) = session.selected_character_id {
            let epoch = self
                .next_transfer_epoch
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                    value.checked_add(1)
                })
                .map_err(|_| "character-exit epoch overflow".to_string())?;
            exit_epoch = Some(epoch);
            let rules_character_id = CharacterId::new(character_id.to_string());
            {
                let handle = self.world.handle.clone();
                if let Err(error) = handle
                    .prepare_character_exit(epoch, rules_character_id.clone())
                    .await
                {
                    for (prepared_handle, _) in &prepared {
                        let _ = prepared_handle.rollback_transfer(epoch).await;
                    }
                    return Err(format!(
                        "expired-session facet preparation failed: {error:?}"
                    ));
                }
                match handle.prepared_checkpoint(epoch).await {
                    Ok(checkpoint) => prepared.push((handle, checkpoint)),
                    Err(error) => {
                        let _ = handle.rollback_transfer(epoch).await;
                        for (prepared_handle, _) in &prepared {
                            let _ = prepared_handle.rollback_transfer(epoch).await;
                        }
                        return Err(format!(
                            "expired-session checkpoint preparation failed: {error:?}"
                        ));
                    }
                }
            }
            prepared.sort_by_key(|(_, checkpoint)| checkpoint.facet_id);
        }

        let durable = async {
            for (_, checkpoint) in &prepared {
                let row = sqlx::query(
                    "SELECT facet_revision,last_server_sequence FROM tme.facets \
                     WHERE facet_id=$1 FOR UPDATE",
                )
                .bind(checkpoint.facet_id.as_uuid())
                .fetch_one(&mut *tx)
                .await
                .map_err(|error| error.to_string())?;
                if checked_u64(
                    row.try_get("facet_revision")
                        .map_err(|error| error.to_string())?,
                )? != checkpoint.before_revision
                    || checked_u64(
                        row.try_get("last_server_sequence")
                            .map_err(|error| error.to_string())?,
                    )? != checkpoint.server_sequence
                {
                    return Err("expired-session facet revision changed".to_string());
                }
                let updated = sqlx::query(
                    "UPDATE tme.facets SET checkpoint_bytes=$2,checkpoint_sha256=$3, \
                     facet_revision=$4,updated_at=statement_timestamp() WHERE facet_id=$1 \
                     AND facet_revision=$5 AND last_server_sequence=$6",
                )
                .bind(checkpoint.facet_id.as_uuid())
                .bind(checkpoint.checkpoint.as_bytes())
                .bind(checkpoint.checkpoint.sha256().as_slice())
                .bind(checked_i64(checkpoint.after_revision)?)
                .bind(checked_i64(checkpoint.before_revision)?)
                .bind(checked_i64(checkpoint.server_sequence)?)
                .execute(&mut *tx)
                .await
                .map_err(|error| error.to_string())?;
                if updated.rows_affected() != 1 {
                    return Err("expired-session facet update lost its fence".to_string());
                }
            }
            sqlx::query(
                "UPDATE tme.sessions SET revoked_at=statement_timestamp() WHERE session_id=$1",
            )
            .bind(session.session_id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
            sqlx::query(
                "UPDATE tme.player_kill_marks SET karma_forgiveness_eligible=false \
                 WHERE forgiven_at IS NULL AND expired_at IS NULL \
                 AND karma_forgiveness_eligible \
                 AND (killer_session_id=$1 OR victim_session_id=$1)",
            )
            .bind(session.session_id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
            sqlx::query(
                "DELETE FROM tme.socket_tickets WHERE session_id=$1 AND consumed_at IS NULL",
            )
            .bind(session.session_id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
            if let Some(character_id) = session.selected_character_id {
                sqlx::query(
                    "UPDATE tme.characters SET control_epoch=control_epoch+1 \
                     WHERE character_id=$1 AND account_id=$2",
                )
                .bind(character_id.as_uuid())
                .bind(session.account_id.as_uuid())
                .execute(&mut *tx)
                .await
                .map_err(|error| error.to_string())?;
            }
            audit(
                &mut tx,
                AuditEvent {
                    account_id: Some(session.account_id.as_uuid()),
                    session_id: Some(session.session_id.as_uuid()),
                    character_id: session
                        .selected_character_id
                        .map(|character_id| character_id.as_uuid()),
                    command_id: None,
                    actor: "runtime",
                    action: "session_expire",
                    result: "success",
                },
            )
            .await?;
            self.commit_gameplay_transaction(tx)
                .await
                .map_err(|error| error.to_string())
        }
        .await;
        if let Err(error) = durable {
            if let Some(epoch) = exit_epoch {
                for (handle, _) in &prepared {
                    let _ = handle.rollback_transfer(epoch).await;
                }
            }
            return Err(error);
        }
        let mut revocations = Vec::new();
        for grant in grants {
            if let Ok(mut live) = self.live.lock() {
                live.active_grants.remove(&grant.character_id);
            }
            if grant.facet_id == self.world.facet_id {
                let facet = &self.world;
                revocations.push(
                    facet
                        .handle
                        .begin_revoke_grant(grant.connection_id, wire::DrainingReason::SessionEnded)
                        .await
                        .map_err(|error| {
                            format!("expired-session grant revoke failed: {error:?}")
                        })?,
                );
            }
        }
        if let Some(epoch) = exit_epoch {
            for (handle, _) in &prepared {
                handle
                    .commit_transfer(epoch)
                    .await
                    .map_err(|error| format!("expired-session commit failed: {error:?}"))?;
            }
            for (handle, _) in &prepared {
                handle
                    .publish_transfer(epoch)
                    .await
                    .map_err(|error| format!("expired-session publish failed: {error:?}"))?;
            }
        }
        for revocation in revocations {
            revocation
                .await
                .map_err(|_| "expired-session grant revoke failed: unavailable".to_string())?;
        }
        Ok(())
    }

    pub async fn forgive_player_kill_mark(
        &self,
        session_cookie: &str,
        csrf_token: &wire::CsrfToken,
        mark_id: wire::PlayerKillMarkId,
        request: wire::ForgivePlayerKillMarkRequestV1,
    ) -> Result<wire::ForgivePlayerKillMarkResultV1, SessionError> {
        let _transition = self.coordinator.transition().await;
        let request_digest: [u8; 32] = Sha256::digest(
            serde_json::to_vec(&serde_json::json!({
                "mark_id": mark_id,
                "request": request,
            }))
            .map_err(unavailable)?,
        )
        .into();

        // Authenticate and discover immutable routing facts without retaining
        // SQL row locks while a facet candidate is prepared.
        let mut discovery = serializable(self.store.pool()).await.map_err(unavailable)?;
        let discovered_session = active_session(&mut discovery, session_cookie, true)
            .await?
            .ok_or(SessionError::AuthenticationRequired)?;
        validate_csrf(discovered_session.csrf_digest, csrf_token)?;
        if let Some(row) = sqlx::query(
            "SELECT request_digest,disposition,outcome_schema,outcome_bytes \
             FROM tme.command_receipts WHERE account_id=$1 AND command_id=$2 FOR UPDATE",
        )
        .bind(discovered_session.account_id.as_uuid())
        .bind(request.request_id.as_uuid())
        .fetch_optional(&mut *discovery)
        .await
        .map_err(unavailable)?
        {
            let stored_digest: Vec<u8> = row.try_get("request_digest").map_err(unavailable)?;
            let disposition: String = row.try_get("disposition").map_err(unavailable)?;
            let outcome_schema: i16 = row.try_get("outcome_schema").map_err(unavailable)?;
            let outcome_bytes: Option<Vec<u8>> =
                row.try_get("outcome_bytes").map_err(unavailable)?;
            if stored_digest.as_slice() != request_digest
                || disposition != "accepted"
                || outcome_schema != 3
                || outcome_bytes.is_none()
            {
                return Err(SessionError::ForgivenessUnavailable);
            }
            discovery.commit().await.map_err(unavailable)?;
            return Ok(wire::ForgivePlayerKillMarkResultV1 {
                control_api_version: wire::CONTROL_API_VERSION,
                mark_id,
                replay_status: wire::ReplayStatus::Replayed,
            });
        }

        let discovered_mark = sqlx::query(
            "SELECT facet_kill_sequence,assessed_logical_time::text AS assessed_logical_time, \
                    killer_account_id,killer_character_id,victim_account_id,victim_character_id, \
                    killer_session_id,victim_session_id,linked_karma_added, \
                    karma_forgiveness_eligible,(forgiven_at IS NOT NULL) AS forgiven, \
                    (expired_at IS NOT NULL) AS expired \
             FROM tme.player_kill_marks WHERE mark_id=$1",
        )
        .bind(mark_id.as_uuid())
        .fetch_optional(&mut *discovery)
        .await
        .map_err(unavailable)?
        .ok_or(SessionError::ForgivenessUnavailable)?;
        let origin_sequence_i64: i64 = discovered_mark
            .try_get("facet_kill_sequence")
            .map_err(unavailable)?;
        let origin_sequence = checked_u64(origin_sequence_i64).map_err(unavailable)?;
        let assessed_logical_time: String = discovered_mark
            .try_get("assessed_logical_time")
            .map_err(unavailable)?;
        let logical_time = assessed_logical_time.parse::<u64>().map_err(unavailable)?;
        let killer_account_id: Uuid = discovered_mark
            .try_get("killer_account_id")
            .map_err(unavailable)?;
        let killer_character_uuid: Uuid = discovered_mark
            .try_get("killer_character_id")
            .map_err(unavailable)?;
        let killer_character_id =
            wire::CharacterId::new(killer_character_uuid).map_err(unavailable)?;
        let victim_account_id: Uuid = discovered_mark
            .try_get("victim_account_id")
            .map_err(unavailable)?;
        let victim_character_uuid: Uuid = discovered_mark
            .try_get("victim_character_id")
            .map_err(unavailable)?;
        let victim_character_id =
            wire::CharacterId::new(victim_character_uuid).map_err(unavailable)?;
        let killer_session_id: Option<Uuid> = discovered_mark
            .try_get("killer_session_id")
            .map_err(unavailable)?;
        let victim_session_id: Uuid = discovered_mark
            .try_get("victim_session_id")
            .map_err(unavailable)?;
        let linked_karma_added: bool = discovered_mark
            .try_get("linked_karma_added")
            .map_err(unavailable)?;
        let karma_forgiveness_eligible: bool = discovered_mark
            .try_get("karma_forgiveness_eligible")
            .map_err(unavailable)?;
        if victim_account_id != discovered_session.account_id.as_uuid()
            || discovered_mark
                .try_get::<bool, _>("forgiven")
                .map_err(unavailable)?
            || discovered_mark
                .try_get::<bool, _>("expired")
                .map_err(unavailable)?
        {
            return Err(SessionError::ForgivenessUnavailable);
        }

        let prepared_forgiveness = if linked_karma_added && karma_forgiveness_eligible {
            // One world: the killer's character is hosted here or nowhere.
            let _: Uuid = sqlx::query_scalar(
                "SELECT character_id FROM tme.characters WHERE character_id=$1 AND account_id=$2",
            )
            .bind(killer_character_uuid)
            .bind(killer_account_id)
            .fetch_optional(&mut *discovery)
            .await
            .map_err(unavailable)?
            .ok_or(SessionError::Unavailable)?;
            let handle = self.world.handle.clone();
            let epoch = self
                .next_transfer_epoch
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                    value.checked_add(1)
                })
                .map_err(|_| SessionError::Unavailable)?;
            let assessment = tme_rules::PlayerKillAssessmentV1 {
                facet_kill_sequence: origin_sequence,
                killer_character_id: CharacterId::new(killer_character_id.to_string()),
                victim_character_id: CharacterId::new(victim_character_id.to_string()),
                exempt_self_defense: false,
                consequence: tme_rules::PlayerKillConsequenceV1::AppliedHere {
                    linked_karma_added: true,
                },
                logical_time: tme_rules::LogicalTime::new(logical_time),
            };
            discovery.commit().await.map_err(unavailable)?;
            if handle
                .prepare_player_kill_forgiveness(epoch, assessment)
                .await
                .is_err()
            {
                return Err(SessionError::Unavailable);
            }
            let checkpoint = match handle.prepared_checkpoint(epoch).await {
                Ok(checkpoint) => checkpoint,
                Err(_) => {
                    let _ = handle.rollback_transfer(epoch).await;
                    return Err(SessionError::Unavailable);
                }
            };
            if checkpoint.facet_id != self.world.facet_id {
                let _ = handle.rollback_transfer(epoch).await;
                return Err(SessionError::Unavailable);
            }
            Some((handle, checkpoint, epoch))
        } else {
            discovery.commit().await.map_err(unavailable)?;
            None
        };

        let durable = async {
            let mut tx = serializable(self.store.pool()).await.map_err(unavailable)?;

            // All gameplay transactions take durable facet rows before
            // accounts and marks. Preparing the in-memory candidate first
            // quiesces ordinary commands without holding a SQL account lock.
            if let Some((_, checkpoint, _)) = &prepared_forgiveness {
                let row = sqlx::query(
                    "SELECT facet_revision,last_server_sequence FROM tme.facets \
                     WHERE facet_id=$1 FOR UPDATE",
                )
                .bind(checkpoint.facet_id.as_uuid())
                .fetch_one(&mut *tx)
                .await
                .map_err(unavailable)?;
                if checked_u64(row.try_get("facet_revision").map_err(unavailable)?)
                    .map_err(unavailable)?
                    != checkpoint.before_revision
                    || checked_u64(
                        row.try_get("last_server_sequence")
                            .map_err(unavailable)?,
                    )
                    .map_err(unavailable)?
                        != checkpoint.server_sequence
                {
                    return Err(SessionError::Unavailable);
                }
            }

            let session = active_session(&mut tx, session_cookie, false)
                .await?
                .ok_or(SessionError::AuthenticationRequired)?;
            validate_csrf(session.csrf_digest, csrf_token)?;
            if session.account_id != discovered_session.account_id {
                return Err(SessionError::ForgivenessUnavailable);
            }

            if let Some(row) = sqlx::query(
                "SELECT request_digest,disposition,outcome_schema,outcome_bytes \
                 FROM tme.command_receipts WHERE account_id=$1 AND command_id=$2 FOR UPDATE",
            )
            .bind(session.account_id.as_uuid())
            .bind(request.request_id.as_uuid())
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?
            {
                let stored_digest: Vec<u8> =
                    row.try_get("request_digest").map_err(unavailable)?;
                let disposition: String = row.try_get("disposition").map_err(unavailable)?;
                let outcome_schema: i16 =
                    row.try_get("outcome_schema").map_err(unavailable)?;
                let outcome_bytes: Option<Vec<u8>> =
                    row.try_get("outcome_bytes").map_err(unavailable)?;
                if stored_digest.as_slice() != request_digest
                    || disposition != "accepted"
                    || outcome_schema != 3
                    || outcome_bytes.is_none()
                {
                    return Err(SessionError::ForgivenessUnavailable);
                }
                tx.commit().await.map_err(unavailable)?;
                return Ok(true);
            }

            let mut account_ids = vec![killer_account_id, victim_account_id];
            account_ids.sort_unstable();
            account_ids.dedup();
            let locked: Vec<Uuid> = sqlx::query_scalar(
                "SELECT account_id FROM tme.accounts WHERE account_id=ANY($1) \
                 ORDER BY account_id FOR UPDATE",
            )
            .bind(account_ids.clone())
            .fetch_all(&mut *tx)
            .await
            .map_err(unavailable)?;
            if locked != account_ids {
                return Err(SessionError::Unavailable);
            }

            let mark = sqlx::query(
                "SELECT facet_kill_sequence,assessed_logical_time::text AS assessed_logical_time, \
                        killer_account_id,killer_character_id,victim_account_id,victim_character_id, \
                        killer_session_id,victim_session_id,linked_karma_added, \
                        karma_forgiveness_eligible,(forgiven_at IS NOT NULL) AS forgiven, \
                        (expired_at IS NOT NULL) AS expired \
                 FROM tme.player_kill_marks WHERE mark_id=$1 FOR UPDATE",
            )
            .bind(mark_id.as_uuid())
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?
            .ok_or(SessionError::ForgivenessUnavailable)?;
            if mark
                    .try_get::<i64, _>("facet_kill_sequence")
                    .map_err(unavailable)?
                    != origin_sequence_i64
                || mark
                    .try_get::<String, _>("assessed_logical_time")
                    .map_err(unavailable)?
                    != assessed_logical_time
                || mark
                    .try_get::<Uuid, _>("killer_account_id")
                    .map_err(unavailable)?
                    != killer_account_id
                || mark
                    .try_get::<Uuid, _>("killer_character_id")
                    .map_err(unavailable)?
                    != killer_character_uuid
                || mark
                    .try_get::<Uuid, _>("victim_account_id")
                    .map_err(unavailable)?
                    != victim_account_id
                || mark
                    .try_get::<Uuid, _>("victim_character_id")
                    .map_err(unavailable)?
                    != victim_character_uuid
                || mark
                    .try_get::<Option<Uuid>, _>("killer_session_id")
                    .map_err(unavailable)?
                    != killer_session_id
                || mark
                    .try_get::<Uuid, _>("victim_session_id")
                    .map_err(unavailable)?
                    != victim_session_id
                || mark
                    .try_get::<bool, _>("linked_karma_added")
                    .map_err(unavailable)?
                    != linked_karma_added
                || mark
                    .try_get::<bool, _>("karma_forgiveness_eligible")
                    .map_err(unavailable)?
                    != karma_forgiveness_eligible
                || mark.try_get::<bool, _>("forgiven").map_err(unavailable)?
                || mark.try_get::<bool, _>("expired").map_err(unavailable)?
                || victim_account_id != session.account_id.as_uuid()
            {
                return Err(SessionError::ForgivenessUnavailable);
            }

            crate::store::reschedule_player_kill_marks_raw(
                &mut tx,
                killer_account_id,
                false,
            )
            .await
            .map_err(unavailable)?;
            let still_active: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM tme.player_kill_marks WHERE mark_id=$1 \
                 AND forgiven_at IS NULL AND expired_at IS NULL)",
            )
            .bind(mark_id.as_uuid())
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?;
            if !still_active {
                return Err(SessionError::ForgivenessUnavailable);
            }

            let active_count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM tme.player_kill_marks WHERE killer_account_id=$1 \
                 AND forgiven_at IS NULL AND expired_at IS NULL",
            )
            .bind(killer_account_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?;
            let (same_facet, no_gameplay) = self
                .live
                .lock()
                .map(|live| {
                    let killer = live.active_grants.get(&killer_character_id);
                    let victim = live.active_grants.get(&victim_character_id);
                    let same_facet = killer.zip(victim).is_some_and(|(killer, victim)| {
                        killer.account_id.as_uuid() == killer_account_id
                            && victim.account_id == session.account_id
                            && Some(killer.session_id.as_uuid()) == killer_session_id
                            && victim.session_id.as_uuid() == victim_session_id
                            && killer.facet_id == victim.facet_id
                    });
                    let no_gameplay = !live.active_grants.values().any(|grant| {
                        grant.account_id.as_uuid() == killer_account_id
                            || grant.account_id == session.account_id
                    });
                    (same_facet, no_gameplay)
                })
                .map_err(|_| SessionError::Unavailable)?;
            let killer_causal_session_active = match killer_session_id {
                Some(killer_session_id) => sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM tme.sessions WHERE session_id=$1 \
                     AND account_id=$2 AND selected_character_id=$3 AND revoked_at IS NULL \
                     AND idle_expires_at>statement_timestamp() \
                     AND absolute_expires_at>statement_timestamp())",
                )
                .bind(killer_session_id)
                .bind(killer_account_id)
                .bind(killer_character_uuid)
                .fetch_one(&mut *tx)
                .await
                .map_err(unavailable)?,
                None => false,
            };
            if !(same_facet
                || (active_count >= 4 && no_gameplay && killer_causal_session_active))
            {
                return Err(SessionError::ForgivenessUnavailable);
            }

            if let Some((_, checkpoint, _)) = &prepared_forgiveness {
                let updated = sqlx::query(
                    "UPDATE tme.facets SET checkpoint_bytes=$2,checkpoint_sha256=$3, \
                     facet_revision=$4,updated_at=statement_timestamp() WHERE facet_id=$1 \
                     AND facet_revision=$5 AND last_server_sequence=$6",
                )
                .bind(checkpoint.facet_id.as_uuid())
                .bind(checkpoint.checkpoint.as_bytes())
                .bind(checkpoint.checkpoint.sha256().as_slice())
                .bind(checked_i64(checkpoint.after_revision).map_err(unavailable)?)
                .bind(checked_i64(checkpoint.before_revision).map_err(unavailable)?)
                .bind(checked_i64(checkpoint.server_sequence).map_err(unavailable)?)
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
                if updated.rows_affected() != 1 {
                    return Err(SessionError::Unavailable);
                }
            }

            let updated = sqlx::query(
                "UPDATE tme.player_kill_marks SET forgiven_at=tme.mark_now(), \
                 forgiven_by_account_id=$2,expires_at=NULL WHERE mark_id=$1 \
                 AND forgiven_at IS NULL AND expired_at IS NULL",
            )
            .bind(mark_id.as_uuid())
            .bind(session.account_id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
            if updated.rows_affected() != 1 {
                return Err(SessionError::ForgivenessUnavailable);
            }
            crate::store::reschedule_player_kill_marks_raw(
                &mut tx,
                killer_account_id,
                true,
            )
            .await
            .map_err(unavailable)?;
            let outcome = ReceiptOutcomeV3::accepted_control()
                .encode()
                .map_err(unavailable)?;
            let inserted = sqlx::query(
                "INSERT INTO tme.command_receipts \
                 (account_id,command_id,request_digest,session_id,outcome_schema,disposition, \
                  outcome_bytes,full_expires_at) \
                 VALUES ($1,$2,$3,$4,3,'accepted',$5,statement_timestamp()+interval '90 days') \
                 ON CONFLICT DO NOTHING",
            )
            .bind(session.account_id.as_uuid())
            .bind(request.request_id.as_uuid())
            .bind(request_digest.as_slice())
            .bind(session.session_id.as_uuid())
            .bind(outcome)
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
            if inserted.rows_affected() != 1 {
                return Err(SessionError::ForgivenessUnavailable);
            }
            audit(
                &mut tx,
                AuditEvent {
                    account_id: Some(session.account_id.as_uuid()),
                    session_id: Some(session.session_id.as_uuid()),
                    character_id: Some(victim_character_uuid),
                    command_id: Some(request.request_id.as_uuid()),
                    actor: "runtime",
                    action: "mark_forgive",
                    result: "success",
                },
            )
            .await
            .map_err(unavailable)?;
            self.commit_gameplay_transaction(tx)
                .await
                .map_err(unavailable)?;
            Ok(false)
        }
        .await;

        let replayed = match durable {
            Ok(replayed) => replayed,
            Err(error) => {
                if let Some((handle, _, epoch)) = &prepared_forgiveness {
                    let _ = handle.rollback_transfer(*epoch).await;
                }
                return Err(error);
            }
        };
        if replayed {
            if let Some((handle, _, epoch)) = &prepared_forgiveness {
                let _ = handle.rollback_transfer(*epoch).await;
            }
            return Ok(wire::ForgivePlayerKillMarkResultV1 {
                control_api_version: wire::CONTROL_API_VERSION,
                mark_id,
                replay_status: wire::ReplayStatus::Replayed,
            });
        }
        if let Some((handle, _, epoch)) = prepared_forgiveness
            && (handle.commit_transfer(epoch).await.is_err()
                || handle.publish_transfer(epoch).await.is_err())
        {
            self.ready.fail();
            return Err(SessionError::Unavailable);
        }
        Ok(wire::ForgivePlayerKillMarkResultV1 {
            control_api_version: wire::CONTROL_API_VERSION,
            mark_id,
            replay_status: wire::ReplayStatus::New,
        })
    }

    /// Control handover marks a character in transition while its grant moves
    /// between connections. It is not world selection: there is one world.
    fn clear_transition(&self, character_id: wire::CharacterId) {
        if let Ok(mut live) = self.live.lock() {
            live.transitioning.remove(&character_id);
        }
    }

    async fn bootstrap_for(
        &self,
        session_id: wire::SessionId,
        account_id: wire::AccountId,
        csrf_token: wire::CsrfToken,
        selected_character_id: Option<wire::CharacterId>,
    ) -> Result<wire::SessionBootstrapV1, SessionError> {
        self.store
            .reconcile_player_kill_marks(account_id.as_uuid())
            .await
            .map_err(unavailable)?;
        let row = sqlx::query(
            "SELECT display_name FROM tme.accounts WHERE account_id=$1 AND status='active'",
        )
        .bind(account_id.as_uuid())
        .fetch_optional(self.store.pool())
        .await
        .map_err(unavailable)?
        .ok_or(SessionError::Unavailable)?;
        let display_name = wire::DisplayName::new(
            row.try_get::<String, _>("display_name")
                .map_err(unavailable)?,
        )
        .map_err(|_| SessionError::Unavailable)?;
        let rows = sqlx::query("SELECT character_id,account_id,slot,display_name,actor_id,control_epoch FROM tme.characters WHERE account_id=$1 ORDER BY slot,character_id")
            .bind(account_id.as_uuid()).fetch_all(self.store.pool()).await.map_err(unavailable)?;
        let characters = rows
            .into_iter()
            .map(decode_character)
            .collect::<Result<Vec<_>, _>>()?;
        let mark_rows = sqlx::query(
            "SELECT m.mark_id,m.victim_character_id,c.display_name AS victim_display_name, \
                    to_char(m.assessed_at AT TIME ZONE 'UTC', \
                            'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS assessed_at, \
                    CASE WHEN m.expires_at IS NULL THEN NULL ELSE \
                         to_char(m.expires_at AT TIME ZONE 'UTC', \
                                 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') END AS expires_at \
             FROM tme.player_kill_marks m \
             JOIN tme.characters c ON c.character_id=m.victim_character_id \
             WHERE m.killer_account_id=$1 AND m.forgiven_at IS NULL AND m.expired_at IS NULL \
             ORDER BY m.assessed_at,m.mark_id",
        )
        .bind(account_id.as_uuid())
        .fetch_all(self.store.pool())
        .await
        .map_err(unavailable)?;
        let active_marks = mark_rows
            .into_iter()
            .map(|row| {
                Ok(wire::PlayerKillMarkSummaryV1 {
                    mark_id: wire::PlayerKillMarkId::new(
                        row.try_get("mark_id").map_err(unavailable)?,
                    )
                    .map_err(|_| SessionError::Unavailable)?,
                    victim_character_id: wire::CharacterId::new(
                        row.try_get("victim_character_id").map_err(unavailable)?,
                    )
                    .map_err(|_| SessionError::Unavailable)?,
                    victim_display_name: wire::DisplayName::new(
                        row.try_get::<String, _>("victim_display_name")
                            .map_err(unavailable)?,
                    )
                    .map_err(|_| SessionError::Unavailable)?,
                    assessed_at: wire::WireLabel::new(
                        row.try_get::<String, _>("assessed_at")
                            .map_err(unavailable)?,
                    )
                    .map_err(|_| SessionError::Unavailable)?,
                    expires_at: row
                        .try_get::<Option<String>, _>("expires_at")
                        .map_err(unavailable)?
                        .map(wire::WireLabel::new)
                        .transpose()
                        .map_err(|_| SessionError::Unavailable)?,
                })
            })
            .collect::<Result<Vec<_>, SessionError>>()?;
        let forgivable_rows = sqlx::query(
            "SELECT m.mark_id,m.killer_account_id,m.killer_character_id,c.display_name AS killer_display_name, \
                    to_char(m.assessed_at AT TIME ZONE 'UTC', \
                            'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS assessed_at, \
                    (SELECT count(*) FROM tme.player_kill_marks active \
                     WHERE active.killer_account_id=m.killer_account_id \
                     AND active.forgiven_at IS NULL AND active.expired_at IS NULL) AS killer_active_count \
             FROM tme.player_kill_marks m \
             JOIN tme.characters c ON c.character_id=m.killer_character_id \
             WHERE m.victim_account_id=$1 AND m.forgiven_at IS NULL AND m.expired_at IS NULL \
             ORDER BY m.assessed_at,m.mark_id",
        )
        .bind(account_id.as_uuid())
        .fetch_all(self.store.pool())
        .await
        .map_err(unavailable)?;
        let mut forgivable_marks = Vec::new();
        for row in forgivable_rows {
            let killer_account_id: Uuid = row.try_get("killer_account_id").map_err(unavailable)?;
            let killer_character_id =
                wire::CharacterId::new(row.try_get("killer_character_id").map_err(unavailable)?)
                    .map_err(|_| SessionError::Unavailable)?;
            let same_facet = self.live.lock().ok().is_some_and(|live| {
                let killer = live.active_grants.get(&killer_character_id);
                let victim = live
                    .active_grants
                    .values()
                    .find(|grant| grant.account_id == account_id);
                killer.zip(victim).is_some_and(|(killer, victim)| {
                    killer.facet_id == victim.facet_id
                        && killer.account_id.as_uuid() == killer_account_id
                })
            });
            let killer_locked = row
                .try_get::<i64, _>("killer_active_count")
                .map_err(unavailable)?
                >= 4;
            let lobby_eligible = if killer_locked {
                let no_gameplay = self.live.lock().ok().is_some_and(|live| {
                    !live.active_grants.values().any(|grant| {
                        grant.account_id == account_id
                            || grant.account_id.as_uuid() == killer_account_id
                    })
                });
                let killer_session: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM tme.sessions WHERE account_id=$1 \
                     AND revoked_at IS NULL AND idle_expires_at>statement_timestamp() \
                     AND absolute_expires_at>statement_timestamp())",
                )
                .bind(killer_account_id)
                .fetch_one(self.store.pool())
                .await
                .map_err(unavailable)?;
                no_gameplay && killer_session
            } else {
                false
            };
            if same_facet || lobby_eligible {
                forgivable_marks.push(wire::ForgivablePlayerKillMarkV1 {
                    mark_id: wire::PlayerKillMarkId::new(
                        row.try_get("mark_id").map_err(unavailable)?,
                    )
                    .map_err(|_| SessionError::Unavailable)?,
                    killer_character_id,
                    killer_display_name: wire::DisplayName::new(
                        row.try_get::<String, _>("killer_display_name")
                            .map_err(unavailable)?,
                    )
                    .map_err(|_| SessionError::Unavailable)?,
                    assessed_at: wire::WireLabel::new(
                        row.try_get::<String, _>("assessed_at")
                            .map_err(unavailable)?,
                    )
                    .map_err(|_| SessionError::Unavailable)?,
                });
            }
        }
        let active_count = u32::try_from(active_marks.len()).map_err(unavailable)?;
        Ok(wire::SessionBootstrapV1 {
            control_api_version: wire::CONTROL_API_VERSION,
            account: wire::AccountSummaryV1 {
                account_id,
                display_name,
            },
            session: wire::SessionSummaryV1 {
                session_id,
                idle_timeout_seconds: wire::DecimalU64::new(SESSION_IDLE.as_secs()),
                absolute_timeout_seconds: wire::DecimalU64::new(SESSION_ABSOLUTE.as_secs()),
            },
            csrf_token,
            characters: characters.iter().map(character_summary).collect(),
            selected_character_id,
            player_kill_marks: wire::PlayerKillMarkStateV1 {
                active_count,
                gameplay_locked: active_count >= 4,
                active_marks,
                forgivable_marks,
            },
        })
    }
}

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

async fn runtime_pool(database_url: &str) -> Result<PgPool, String> {
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

async fn auth_pool(database_url: &str) -> Result<PgPool, String> {
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

async fn recover_or_initialize(
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
    let checkpoint = FacetCheckpointV4::from_bytes(bytes).map_err(|error| error.to_string())?;
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

async fn verify_character_assertions(
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

fn identity_digest(value: &tme_rules::ContentIdentityV1) -> Result<[u8; 32], String> {
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

fn hex_nibble(value: u8) -> Result<u8, String> {
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

fn validate_directory(
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

async fn verify_loaded_directory(pool: &PgPool, engine: &Engine) -> Result<(), String> {
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

async fn active_session(
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

async fn character_for_account(
    tx: &mut Transaction<'_, Postgres>,
    id: wire::CharacterId,
    account: wire::AccountId,
) -> Result<Option<CharacterRow>, SessionError> {
    sqlx::query("SELECT character_id,account_id,slot,display_name,actor_id,control_epoch FROM tme.characters WHERE character_id=$1 AND account_id=$2 FOR UPDATE")
        .bind(id.as_uuid()).bind(account.as_uuid()).fetch_optional(&mut **tx).await.map_err(unavailable)?.map(decode_character).transpose()
}

fn decode_session(row: sqlx::postgres::PgRow) -> Result<SessionRow, SessionError> {
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

fn decode_character(row: sqlx::postgres::PgRow) -> Result<CharacterRow, SessionError> {
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

fn selection(character: &CharacterRow) -> wire::CharacterSelectionV1 {
    wire::CharacterSelectionV1 {
        control_api_version: wire::CONTROL_API_VERSION,
        character: character_summary(character),
    }
}
fn character_summary(character: &CharacterRow) -> wire::CharacterSummaryV1 {
    wire::CharacterSummaryV1 {
        character_id: character.character_id,
        slot: character.slot,
        display_name: character.display_name.clone(),
    }
}
fn validate_csrf(value: [u8; 32], token: &wire::CsrfToken) -> Result<(), SessionError> {
    if digest(token.expose_for_validation()) == value {
        Ok(())
    } else {
        Err(SessionError::CsrfRejected)
    }
}
fn digest(value: &str) -> [u8; 32] {
    Sha256::digest(value.as_bytes()).into()
}
fn random_secret() -> Result<OpaqueSecret, String> {
    Ok(OpaqueSecret(
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes::<32>()?),
    ))
}
fn random_csrf() -> Result<wire::CsrfToken, String> {
    wire::CsrfToken::new(
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes::<32>()?),
    )
    .map_err(|error| error.to_string())
}
fn random_ticket() -> Result<wire::AdmissionTicket, String> {
    wire::AdmissionTicket::new(
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes::<32>()?),
    )
    .map_err(|error| error.to_string())
}
fn validate_ascii_key(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 64
        || !value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
    {
        Err("key must contain 1-64 printable ASCII bytes".to_string())
    } else {
        Ok(())
    }
}

fn unavailable(_: impl std::fmt::Display) -> SessionError {
    SessionError::Unavailable
}

#[cfg(test)]
async fn certify_command_reservation_race(pool: &PgPool) {
    use crate::coordinator::Reservation;

    let account_uuid = Uuid::now_v7();
    let account_id = wire::AccountId::new(account_uuid).unwrap();
    let session_id = wire::SessionId::new(Uuid::now_v7()).unwrap();
    let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let username = format!("ev_{}", &account_uuid.as_simple().to_string()[..12]);
    sqlx::query(
        "INSERT INTO tme.accounts(account_id,username,display_name) VALUES($1,$2,'EV Race')",
    )
    .bind(account_id.as_uuid())
    .bind(username)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tme.sessions \
         (session_id,account_id,token_digest,csrf_digest,idle_expires_at,absolute_expires_at) \
         VALUES($1,$2,$3,$4,statement_timestamp()+interval '1 hour', \
                statement_timestamp()+interval '1 day')",
    )
    .bind(session_id.as_uuid())
    .bind(account_id.as_uuid())
    .bind([11_u8; 32].as_slice())
    .bind([12_u8; 32].as_slice())
    .execute(pool)
    .await
    .unwrap();

    let command = wire::ClientCommandEnvelope::Command {
        command_id,
        control_epoch: wire::DecimalU64::new(1),
        client_sequence: wire::DecimalU64::new(1),
        observed_world_revision: wire::DecimalU64::new(0),
        actor_id: wire::ActorId::new("player").unwrap(),
        intent: wire::Intent::Wait,
    };
    let store = Arc::new(PostgresStore::new(pool.clone()));
    let coordinator = Arc::new(Coordinator::new(store));
    let barrier = Arc::new(tokio::sync::Barrier::new(3));
    let mut racers = Vec::new();
    for _ in 0..2 {
        let coordinator = coordinator.clone();
        let command = command.clone();
        let barrier = barrier.clone();
        racers.push(tokio::spawn(async move {
            barrier.wait().await;
            coordinator.reserve(account_id, command_id, &command).await
        }));
    }
    barrier.wait().await;

    let mut new_digest = None;
    let mut in_progress = 0;
    for racer in racers {
        match racer.await.unwrap() {
            Reservation::New { digest } => {
                assert!(new_digest.replace(digest).is_none());
            }
            Reservation::InProgress => in_progress += 1,
            _ => panic!("same-ID reservation race returned an invalid outcome"),
        }
    }
    assert_eq!(in_progress, 1);
    let digest = new_digest.expect("exactly one racer owns new execution");
    let new_result = coordinator
        .complete_authority_rejection(
            account_id,
            session_id,
            command_id,
            digest,
            wire::RejectionCode::StaleControlEpoch,
        )
        .await
        .unwrap();
    let replay = match coordinator.reserve(account_id, command_id, &command).await {
        Reservation::Replay(envelope) => *envelope,
        _ => panic!("completed same-ID reservation did not become durable replay"),
    };
    let mut expected_replay = new_result;
    let wire::ServerEnvelope::CommandResult { replay_status, .. } = &mut expected_replay else {
        panic!("authority rejection did not return a command result");
    };
    *replay_status = wire::ReplayStatus::Replayed;
    assert_eq!(replay, expected_replay);
    let receipt_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tme.command_receipts WHERE account_id=$1 AND command_id=$2",
    )
    .bind(account_id.as_uuid())
    .bind(command_id.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(receipt_count, 1);

    sqlx::query("DELETE FROM tme.audit_events WHERE account_id=$1")
        .bind(account_id.as_uuid())
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM tme.accounts WHERE account_id=$1")
        .bind(account_id.as_uuid())
        .execute(pool)
        .await
        .unwrap();
}

#[cfg(test)]
#[derive(Clone, Copy)]
struct EvDatabaseFixture {
    account_id: wire::AccountId,
    character_id: wire::CharacterId,
    session_id: wire::SessionId,
    world_id: wire::FacetId,
}

#[cfg(test)]
fn ev_database_engine() -> Engine {
    let mut scenario = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    scenario.extend([
        "..",
        "..",
        "content",
        "test-corpus",
        "world_topology_gallery.json",
    ]);
    tme_sim::load_engine_from_scenario(&scenario, Some(7))
        .expect("EV database certification scenario loads")
}

#[cfg(test)]
fn ev_database_bootstrap(fixture: EvDatabaseFixture) -> PostgresBootstrap {
    PostgresBootstrap {
        world: PostgresWorldBootstrap {
            facet_id: fixture.world_id,
            key: "ev-world".to_string(),
            engine: ev_database_engine(),
        },
        characters: vec![PostgresCharacterBootstrap {
            account_id: fixture.account_id,
            character_id: fixture.character_id,
            slot: 1,
            display_name: wire::DisplayName::new("EV Fault Character").unwrap(),
            actor_id: ActorId::new("player"),
        }],
    }
}

#[cfg(test)]
async fn ev_insert_account(pool: &PgPool, fixture: EvDatabaseFixture) {
    let username = format!(
        "ev_fault_{}",
        &fixture.account_id.as_uuid().as_simple().to_string()[..12]
    );
    sqlx::query("INSERT INTO tme.accounts(account_id,username,display_name) VALUES($1,$2,$3)")
        .bind(fixture.account_id.as_uuid())
        .bind(username)
        .bind("EV Fault Account")
        .execute(pool)
        .await
        .unwrap();
}

#[cfg(test)]
async fn ev_insert_session(
    pool: &PgPool,
    fixture: EvDatabaseFixture,
    cookie: &str,
    csrf: &wire::CsrfToken,
) {
    sqlx::query(
        "INSERT INTO tme.sessions \
         (session_id,account_id,token_digest,csrf_digest,selected_character_id, \
          idle_expires_at,absolute_expires_at) \
         VALUES($1,$2,$3,$4,$5,statement_timestamp()+interval '1 hour', \
                statement_timestamp()+interval '1 day')",
    )
    .bind(fixture.session_id.as_uuid())
    .bind(fixture.account_id.as_uuid())
    .bind(digest(cookie).as_slice())
    .bind(digest(csrf.expose_for_validation()).as_slice())
    .bind(fixture.character_id.as_uuid())
    .execute(pool)
    .await
    .unwrap();
}

#[cfg(test)]
async fn ev_new_csrf(state: &PostgresState, cookie: &str) -> wire::CsrfToken {
    state
        .session_bootstrap(cookie)
        .await
        .expect("EV session remains live")
        .csrf_token
}

#[cfg(test)]
async fn ev_facet_row(pool: &PgPool, facet_id: wire::FacetId) -> (i64, i64, Vec<u8>, Vec<u8>) {
    let row = sqlx::query(
        "SELECT facet_revision,last_server_sequence,checkpoint_bytes,checkpoint_sha256 \
         FROM tme.facets WHERE facet_id=$1",
    )
    .bind(facet_id.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    (
        row.try_get("facet_revision").unwrap(),
        row.try_get("last_server_sequence").unwrap(),
        row.try_get("checkpoint_bytes").unwrap(),
        row.try_get("checkpoint_sha256").unwrap(),
    )
}

#[cfg(test)]
async fn ev_command_artifacts(
    pool: &PgPool,
    account_id: wire::AccountId,
    command_id: wire::CommandId,
) -> (i64, i64) {
    let receipts = sqlx::query_scalar(
        "SELECT count(*) FROM tme.command_receipts WHERE account_id=$1 AND command_id=$2",
    )
    .bind(account_id.as_uuid())
    .bind(command_id.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    let audits = sqlx::query_scalar(
        "SELECT count(*) FROM tme.audit_events WHERE account_id=$1 AND command_id=$2",
    )
    .bind(account_id.as_uuid())
    .bind(command_id.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    (receipts, audits)
}

#[cfg(test)]
async fn ev_certify_direct_store_failures(
    database_url: &str,
    pool: &PgPool,
    state: &Arc<PostgresState>,
    fixture: EvDatabaseFixture,
) {
    use crate::store::EvStoreFault;

    eprintln!("EV source-fault stage: direct store failures");

    let system_faults = [
        EvStoreFault::SystemSqlAcquire,
        EvStoreFault::SystemCompareAndSwap,
        EvStoreFault::SystemAudit,
        EvStoreFault::SystemCommit,
    ];
    for fault in system_faults {
        let before = ev_facet_row(pool, fixture.world_id).await;
        let checkpoint = FacetCheckpointV4::from_bytes(before.2.clone()).unwrap();
        let audit_before: i64 =
            sqlx::query_scalar("SELECT count(*) FROM tme.audit_events WHERE action='facet_tick'")
                .fetch_one(pool)
                .await
                .unwrap();
        state.store.ev_arm_fault(fault);
        let result = state
            .store
            .commit_system(crate::store::SystemCommit {
                facet_id: fixture.world_id,
                expected_server_sequence: u64::try_from(before.1).unwrap(),
                expected_revision: u64::try_from(before.0).unwrap(),
                next_server_sequence: u64::try_from(before.1).unwrap() + 1,
                next_revision: u64::try_from(before.0).unwrap() + 1,
                checkpoint: &checkpoint,
                action: "facet_tick",
                durable_effects: &[],
            })
            .await;
        assert!(result.is_err(), "{fault:?} must fail system persistence");
        state.store.ev_assert_fault_consumed();
        assert_eq!(ev_facet_row(pool, fixture.world_id).await, before);
        let audit_after: i64 =
            sqlx::query_scalar("SELECT count(*) FROM tme.audit_events WHERE action='facet_tick'")
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(audit_after, audit_before);
    }

    let before = ev_facet_row(pool, fixture.world_id).await;
    let checkpoint = FacetCheckpointV4::from_bytes(before.2.clone()).unwrap();
    let stale_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let stale_outcome = ReceiptOutcomeV3::rejected(
        wire::RejectionCode::FutureWorldRevision,
        Some(u64::try_from(before.1).unwrap() + 1),
        Some(u64::try_from(before.0).unwrap()),
    );
    let stale = state
        .store
        .commit_command(crate::store::CommandCommit {
            account_id: fixture.account_id,
            session_id: fixture.session_id,
            character_id: fixture.character_id,
            command_id: stale_id,
            request_digest: [31; 32],
            facet_id: fixture.world_id,
            actor_id: "player",
            control_epoch: 0,
            client_sequence: 1,
            expected_server_sequence: u64::try_from(before.1).unwrap() + 1,
            expected_revision: u64::try_from(before.0).unwrap(),
            next_server_sequence: u64::try_from(before.1).unwrap() + 2,
            next_revision: u64::try_from(before.0).unwrap(),
            checkpoint: &checkpoint,
            outcome: &stale_outcome,
            durable_effects: &[],
        })
        .await;
    assert!(stale.is_err(), "natural stale CAS must fail");
    assert_eq!(ev_facet_row(pool, fixture.world_id).await, before);
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, stale_id).await,
        (0, 0)
    );

    let effect_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let effect_outcome = ReceiptOutcomeV3::rejected(
        wire::RejectionCode::FutureWorldRevision,
        Some(u64::try_from(before.1).unwrap() + 1),
        Some(u64::try_from(before.0).unwrap()),
    );
    let missing_effect =
        tme_rules::DurableGameplayEffectV1::PlayerKillAssessed(tme_rules::PlayerKillAssessmentV1 {
            facet_kill_sequence: 1,
            killer_character_id: CharacterId::new(Uuid::now_v7().to_string()),
            victim_character_id: CharacterId::new(Uuid::now_v7().to_string()),
            exempt_self_defense: false,
            consequence: tme_rules::PlayerKillConsequenceV1::AppliedHere {
                linked_karma_added: false,
            },
            logical_time: tme_rules::LogicalTime::new(1),
        });
    let effect_failure = state
        .store
        .commit_command(crate::store::CommandCommit {
            account_id: fixture.account_id,
            session_id: fixture.session_id,
            character_id: fixture.character_id,
            command_id: effect_id,
            request_digest: [33; 32],
            facet_id: fixture.world_id,
            actor_id: "player",
            control_epoch: 0,
            client_sequence: 1,
            expected_server_sequence: u64::try_from(before.1).unwrap(),
            expected_revision: u64::try_from(before.0).unwrap(),
            next_server_sequence: u64::try_from(before.1).unwrap() + 1,
            next_revision: u64::try_from(before.0).unwrap(),
            checkpoint: &checkpoint,
            outcome: &effect_outcome,
            durable_effects: std::slice::from_ref(&missing_effect),
        })
        .await;
    assert!(effect_failure.is_err());
    assert_eq!(ev_facet_row(pool, fixture.world_id).await, before);
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, effect_id).await,
        (0, 0)
    );

    let timeout_pool = PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query("SET lock_timeout='100ms'")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await
        .expect("EV timeout pool connects");
    let timeout_store = PostgresStore::new(timeout_pool.clone());
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("SELECT facet_id FROM tme.facets WHERE facet_id=$1 FOR UPDATE")
        .bind(fixture.world_id.as_uuid())
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    let lock_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let lock_outcome = ReceiptOutcomeV3::rejected(
        wire::RejectionCode::FutureWorldRevision,
        Some(u64::try_from(before.1).unwrap() + 1),
        Some(u64::try_from(before.0).unwrap()),
    );
    let locked = timeout_store
        .commit_command(crate::store::CommandCommit {
            account_id: fixture.account_id,
            session_id: fixture.session_id,
            character_id: fixture.character_id,
            command_id: lock_id,
            request_digest: [32; 32],
            facet_id: fixture.world_id,
            actor_id: "player",
            control_epoch: 0,
            client_sequence: 1,
            expected_server_sequence: u64::try_from(before.1).unwrap(),
            expected_revision: u64::try_from(before.0).unwrap(),
            next_server_sequence: u64::try_from(before.1).unwrap() + 1,
            next_revision: u64::try_from(before.0).unwrap(),
            checkpoint: &checkpoint,
            outcome: &lock_outcome,
            durable_effects: &[],
        })
        .await;
    assert!(
        locked.is_err(),
        "real PostgreSQL row lock must hit lock_timeout"
    );
    lock.rollback().await.unwrap();
    timeout_pool.close().await;
    assert_eq!(ev_facet_row(pool, fixture.world_id).await, before);
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, lock_id).await,
        (0, 0)
    );
    eprintln!("EV source-fault stage complete: direct store failures");
}

#[cfg(test)]
fn ev_wire_command(
    grant: &AdmissionGrant,
    command_id: wire::CommandId,
    client_sequence: u64,
    observed_facet_revision: u64,
    enabled: bool,
) -> wire::ClientCommandEnvelope {
    wire::ClientCommandEnvelope::Command {
        command_id,
        control_epoch: wire::DecimalU64::new(grant.control.control_epoch),
        client_sequence: wire::DecimalU64::new(client_sequence),
        observed_world_revision: wire::DecimalU64::new(observed_facet_revision),
        actor_id: wire::ActorId::new(grant.control.actor_id.as_str()).unwrap(),
        intent: wire::Intent::SetPagesEnabled { enabled },
    }
}

#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum EvCommandFault {
    None,
    CheckpointExport,
    AfterStoreCommit,
    CommitOutcomeUnknown,
}

#[cfg(test)]
fn ev_facet_command(
    grant: &AdmissionGrant,
    command_id: wire::CommandId,
    client_sequence: u64,
    observed_facet_revision: u64,
    enabled: bool,
    request_digest: [u8; 32],
    fault: EvCommandFault,
) -> crate::facet::FacetCommand {
    crate::facet::FacetCommand {
        connection_id: grant.control.connection_id,
        account_id: grant.control.account_id,
        session_id: grant.control.session_id,
        character_id: grant.control.character_id,
        command_id,
        control_epoch: grant.control.control_epoch,
        client_sequence,
        observed_facet_revision,
        actor_id: wire::ActorId::new(grant.control.actor_id.as_str()).unwrap(),
        intent: wire::Intent::SetPagesEnabled { enabled },
        request_digest,
        certification_trace: None,
        ev_fail_checkpoint_export: fault == EvCommandFault::CheckpointExport,
        ev_fail_after_store_commit: fault == EvCommandFault::AfterStoreCommit,
    }
}

#[cfg(test)]
async fn ev_current_state(grant: &AdmissionGrant) -> wire::ServerEnvelope {
    grant
        .facet
        .try_current_state(grant.control.connection_id)
        .expect("EV current-state request enqueues")
        .await
        .expect("EV current-state reply arrives")
        .expect("EV observer remains installed")
}

#[cfg(test)]
fn ev_state_revision(envelope: &wire::ServerEnvelope) -> u64 {
    match envelope {
        wire::ServerEnvelope::StateUpdate { world_revision, .. } => world_revision.get(),
        other => panic!("expected EV state update, got {other:?}"),
    }
}

#[cfg(test)]
async fn ev_admit_character(
    state: &Arc<PostgresState>,
    cookie: &str,
) -> (
    AdmissionGrant,
    FacetWelcome,
    mpsc::Sender<wire::ServerEnvelope>,
    mpsc::Receiver<wire::ServerEnvelope>,
) {
    let csrf = ev_new_csrf(state, cookie).await;
    let ticket = state
        .issue_ticket(
            cookie,
            wire::SocketTicketRequestV1 { csrf_token: csrf },
            "https://ev.invalid",
            "ev.invalid",
        )
        .await
        .expect("EV command ticket issues");
    let (outbound, outbound_receive) = mpsc::channel(crate::config::OUTBOUND_QUEUE_CAPACITY);
    let (terminal, _terminal_receive) = watch::channel(None);
    let (grant, welcome) = state
        .admit(
            &ticket.ticket,
            &[wire::PROTOCOL_MINOR],
            "https://ev.invalid",
            "ev.invalid",
            outbound.clone(),
            terminal,
        )
        .await
        .expect("EV command character admits");
    (grant, welcome, outbound, outbound_receive)
}

#[cfg(test)]
async fn ev_reserve_new(
    state: &PostgresState,
    fixture: EvDatabaseFixture,
    command_id: wire::CommandId,
    command: &wire::ClientCommandEnvelope,
) -> [u8; 32] {
    match state
        .coordinator
        .reserve(fixture.account_id, command_id, command)
        .await
    {
        crate::coordinator::Reservation::New { digest } => digest,
        _ => panic!("EV command reservation must be new"),
    }
}

#[cfg(test)]
async fn ev_wait_for_mailbox_state(grant: &AdmissionGrant) -> wire::ServerEnvelope {
    loop {
        match grant.facet.try_current_state(grant.control.connection_id) {
            Ok(receive) => {
                return receive
                    .await
                    .expect("EV mailbox state reply arrives")
                    .expect("EV command observer remains installed");
            }
            Err(crate::facet::FacetError::QueueFull) => tokio::task::yield_now().await,
            Err(error) => panic!("EV mailbox became unavailable: {error:?}"),
        }
    }
}

#[cfg(test)]
async fn ev_certify_command_pipeline(
    pool: &PgPool,
    state: &Arc<PostgresState>,
    fixture: EvDatabaseFixture,
    cookie: &str,
) {
    use crate::coordinator::Reservation;
    use crate::store::EvStoreFault;

    eprintln!("EV source-fault stage: command pipeline");

    let (grant, welcome, outbound, mut outbound_receive) = ev_admit_character(state, cookie).await;
    assert_eq!(grant.control.facet_id, fixture.world_id);
    let initial = ev_current_state(&grant).await;
    assert_eq!(ev_state_revision(&initial), welcome.facet_revision);

    let reservation_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let reservation_command = ev_wire_command(
        &grant,
        reservation_id,
        1,
        ev_state_revision(&initial),
        false,
    );
    state.store.ev_arm_fault(EvStoreFault::ReceiptSqlAcquire);
    assert!(matches!(
        state
            .coordinator
            .reserve(fixture.account_id, reservation_id, &reservation_command)
            .await,
        Reservation::Unavailable
    ));
    state.store.ev_assert_fault_consumed();
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, reservation_id).await,
        (0, 0)
    );
    let digest = ev_reserve_new(state, fixture, reservation_id, &reservation_command).await;
    state
        .coordinator
        .release(fixture.account_id, reservation_id, digest);

    for fault in [
        EvStoreFault::AuthorityRejectionInsert,
        EvStoreFault::AuthorityRejectionCommit,
    ] {
        let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
        let command = ev_wire_command(&grant, command_id, 1, ev_state_revision(&initial), false);
        let digest = ev_reserve_new(state, fixture, command_id, &command).await;
        state.store.ev_arm_fault(fault);
        assert!(
            state
                .coordinator
                .complete_authority_rejection(
                    fixture.account_id,
                    fixture.session_id,
                    command_id,
                    digest,
                    wire::RejectionCode::StaleControlEpoch,
                )
                .await
                .is_err(),
            "{fault:?} must fail authority-rejection persistence"
        );
        state.store.ev_assert_fault_consumed();
        assert_eq!(
            ev_command_artifacts(pool, fixture.account_id, command_id).await,
            (0, 0)
        );
        let retry_digest = ev_reserve_new(state, fixture, command_id, &command).await;
        assert_eq!(retry_digest, digest);
        state
            .coordinator
            .release(fixture.account_id, command_id, retry_digest);
    }

    let ambiguous_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let ambiguous_command =
        ev_wire_command(&grant, ambiguous_id, 1, ev_state_revision(&initial), false);
    let ambiguous_digest = ev_reserve_new(state, fixture, ambiguous_id, &ambiguous_command).await;
    state
        .store
        .ev_arm_fault(EvStoreFault::AuthorityRejectionOutcomeUnknown);
    assert!(
        state
            .coordinator
            .complete_authority_rejection(
                fixture.account_id,
                fixture.session_id,
                ambiguous_id,
                ambiguous_digest,
                wire::RejectionCode::StaleControlEpoch,
            )
            .await
            .is_err(),
        "lost authority-rejection commit result must not report success"
    );
    state.store.ev_assert_fault_consumed();
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, ambiguous_id).await,
        (1, 1)
    );
    assert!(matches!(
        state
            .coordinator
            .reserve(fixture.account_id, ambiguous_id, &ambiguous_command)
            .await,
        Reservation::Replay(envelope)
            if matches!(
                *envelope,
                wire::ServerEnvelope::CommandResult {
                    disposition: wire::CommandDisposition::Rejected {
                        code: wire::RejectionCode::StaleControlEpoch,
                    },
                    replay_status: wire::ReplayStatus::Replayed,
                    ..
                }
            )
    ));

    let queue_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let queue_command = ev_wire_command(&grant, queue_id, 1, ev_state_revision(&initial), false);
    let queue_digest = ev_reserve_new(state, fixture, queue_id, &queue_command).await;
    let (release, entered) = grant.facet.ev_hold_mailbox().await;
    entered.await.unwrap();
    let rules_character_id = CharacterId::new(fixture.character_id.to_string());
    let mut queued = 0;
    while grant
        .facet
        .ev_try_inspect(rules_character_id.clone())
        .is_ok()
    {
        queued += 1;
    }
    assert_eq!(queued, crate::config::FACET_MAILBOX_CAPACITY);
    assert!(matches!(
        grant.facet.try_command(ev_facet_command(
            &grant,
            queue_id,
            1,
            ev_state_revision(&initial),
            false,
            queue_digest,
            EvCommandFault::None,
        )),
        Err(crate::facet::FacetError::QueueFull)
    ));
    state
        .coordinator
        .release(fixture.account_id, queue_id, queue_digest);
    release.send(()).unwrap();
    let drained = ev_wait_for_mailbox_state(&grant).await;
    assert_eq!(drained, initial);
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, queue_id).await,
        (0, 0)
    );

    let failures = [
        ("checkpoint_export", None),
        ("sql_acquire", Some(EvStoreFault::CommandSqlAcquire)),
        ("row_lock", Some(EvStoreFault::CommandRowLock)),
        ("receipt_insert", Some(EvStoreFault::CommandReceiptInsert)),
        ("durable_effects", Some(EvStoreFault::CommandDurableEffects)),
        (
            "compare_and_swap",
            Some(EvStoreFault::CommandCompareAndSwap),
        ),
        ("audit", Some(EvStoreFault::CommandAudit)),
        ("commit", Some(EvStoreFault::CommandCommit)),
    ];
    for (label, fault) in failures {
        let before_state = ev_current_state(&grant).await;
        let before_store = ev_facet_row(pool, fixture.world_id).await;
        let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
        let command = ev_wire_command(
            &grant,
            command_id,
            1,
            ev_state_revision(&before_state),
            false,
        );
        let digest = ev_reserve_new(state, fixture, command_id, &command).await;
        if let Some(fault) = fault {
            state.store.ev_arm_fault(fault);
        }
        let receive = grant
            .facet
            .try_command(ev_facet_command(
                &grant,
                command_id,
                1,
                ev_state_revision(&before_state),
                false,
                digest,
                if label == "checkpoint_export" {
                    EvCommandFault::CheckpointExport
                } else {
                    EvCommandFault::None
                },
            ))
            .unwrap();
        assert!(receive.await.is_err(), "{label} emitted a success reply");
        state
            .coordinator
            .release(fixture.account_id, command_id, digest);
        if fault.is_some() {
            state.store.ev_assert_fault_consumed();
        }
        let after_state = ev_current_state(&grant).await;
        assert_eq!(after_state, before_state, "{label} swapped in-memory state");
        assert_eq!(
            ev_facet_row(pool, fixture.world_id).await,
            before_store,
            "{label} changed the durable checkpoint"
        );
        assert_eq!(
            ev_command_artifacts(pool, fixture.account_id, command_id).await,
            (0, 0),
            "{label} leaked receipt or audit"
        );
        assert!(outbound_receive.try_recv().is_err(), "{label} fanned out");
    }

    let accepted_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let before = ev_current_state(&grant).await;
    let before_revision = ev_state_revision(&before);
    let accepted_wire = ev_wire_command(&grant, accepted_id, 1, before_revision, false);
    let (left, right) = tokio::join!(
        state
            .coordinator
            .reserve(fixture.account_id, accepted_id, &accepted_wire),
        state
            .coordinator
            .reserve(fixture.account_id, accepted_id, &accepted_wire),
    );
    let accepted_digest = match (left, right) {
        (Reservation::New { digest }, Reservation::InProgress)
        | (Reservation::InProgress, Reservation::New { digest }) => digest,
        _ => panic!("same-ID gameplay reservation race must be new/in-progress"),
    };
    let (release, entered) = grant.facet.ev_hold_mailbox().await;
    entered.await.unwrap();
    let accepted_receive = grant
        .facet
        .try_command(ev_facet_command(
            &grant,
            accepted_id,
            1,
            before_revision,
            false,
            accepted_digest,
            EvCommandFault::None,
        ))
        .unwrap();
    let tick_receive = grant
        .facet
        .ev_try_tick(grant.control.actor_id.clone())
        .unwrap();
    release.send(()).unwrap();
    let accepted = accepted_receive.await.unwrap();
    let tick = tick_receive.await.unwrap();
    state
        .coordinator
        .finish(fixture.account_id, accepted_id, accepted_digest);
    assert!(matches!(
        accepted.envelope,
        wire::ServerEnvelope::CommandResult {
            disposition: wire::CommandDisposition::Accepted,
            replay_status: wire::ReplayStatus::New,
            ..
        }
    ));
    let after = ev_current_state(&grant).await;
    assert_eq!(ev_state_revision(&after), tick.facet_revision);
    assert_eq!(tick.facet_revision, before_revision + 2);
    assert!(tick.outcome.state_changed);
    let wire::ServerEnvelope::StateUpdate { frame, .. } = &after else {
        unreachable!()
    };
    assert!(
        !frame.social.pages_enabled,
        "in-memory swap is authoritative"
    );
    assert!(matches!(
        outbound_receive.try_recv(),
        Ok(wire::ServerEnvelope::StateUpdate { .. })
    ));
    assert!(matches!(
        outbound_receive.try_recv(),
        Ok(wire::ServerEnvelope::StateUpdate { .. })
    ));
    let durable_after_tick = ev_facet_row(pool, fixture.world_id).await;
    assert_eq!(
        durable_after_tick.0,
        i64::try_from(tick.facet_revision).unwrap()
    );
    assert_eq!(
        durable_after_tick.1,
        i64::try_from(tick.server_sequence).unwrap()
    );
    assert_eq!(durable_after_tick.2, tick.checkpoint.as_bytes());
    assert_eq!(durable_after_tick.3, tick.checkpoint.sha256());
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, accepted_id).await,
        (1, 1)
    );
    let replay_before = ev_facet_row(pool, fixture.world_id).await;
    assert!(matches!(
        state
            .coordinator
            .reserve(fixture.account_id, accepted_id, &accepted_wire)
            .await,
        Reservation::Replay(_)
    ));
    assert_eq!(
        ev_facet_row(pool, fixture.world_id).await,
        replay_before,
        "same-ID gameplay replay executed a second mutation"
    );

    while outbound_receive.try_recv().is_ok() {}
    for _ in 0..crate::config::OUTBOUND_QUEUE_CAPACITY {
        outbound
            .try_send(wire::ServerEnvelope::ServerDraining {
                reason: wire::DrainingReason::Shutdown,
                reconnect_hint: false,
            })
            .unwrap();
    }
    let publication_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let publication_wire = ev_wire_command(&grant, publication_id, 2, tick.facet_revision, true);
    let publication_digest =
        ev_reserve_new(state, fixture, publication_id, &publication_wire).await;
    let publication_reply = grant
        .facet
        .try_command(ev_facet_command(
            &grant,
            publication_id,
            2,
            tick.facet_revision,
            true,
            publication_digest,
            EvCommandFault::None,
        ))
        .unwrap()
        .await
        .unwrap();
    state
        .coordinator
        .finish(fixture.account_id, publication_id, publication_digest);
    assert!(matches!(
        publication_reply.envelope,
        wire::ServerEnvelope::CommandResult {
            disposition: wire::CommandDisposition::Rejected {
                code: wire::RejectionCode::ProjectionFailed,
            },
            replay_status: wire::ReplayStatus::New,
            ..
        }
    ));
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, publication_id).await,
        (1, 1)
    );
    let detached = grant
        .facet
        .try_current_state(grant.control.connection_id)
        .unwrap()
        .await
        .unwrap();
    assert!(
        detached.is_err(),
        "failed publication must detach the observer"
    );
    for _ in 0..crate::config::OUTBOUND_QUEUE_CAPACITY {
        assert!(matches!(
            outbound_receive.try_recv(),
            Ok(wire::ServerEnvelope::ServerDraining { .. })
        ));
    }
    assert!(
        outbound_receive.try_recv().is_err(),
        "projection failure published a state update into the saturated queue"
    );

    let (presence_grant, presence_welcome, _presence_outbound, _presence_receive) =
        ev_admit_character(state, cookie).await;
    let presence_command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let presence_wire = ev_wire_command(
        &presence_grant,
        presence_command_id,
        1,
        presence_welcome.facet_revision,
        true,
    );
    let presence_digest = ev_reserve_new(state, fixture, presence_command_id, &presence_wire).await;
    let (release, entered) = presence_grant.facet.ev_hold_mailbox().await;
    entered.await.unwrap();
    let presence_command = presence_grant
        .facet
        .try_command(ev_facet_command(
            &presence_grant,
            presence_command_id,
            1,
            presence_welcome.facet_revision,
            true,
            presence_digest,
            EvCommandFault::None,
        ))
        .unwrap();
    let presence_detach = presence_grant
        .facet
        .ev_try_detach(presence_grant.control.connection_id)
        .unwrap();
    release.send(()).unwrap();
    let presence_reply = presence_command.await.unwrap();
    let presence_step = presence_detach.await.unwrap();
    state
        .coordinator
        .finish(fixture.account_id, presence_command_id, presence_digest);
    assert!(matches!(
        presence_reply.envelope,
        wire::ServerEnvelope::CommandResult {
            disposition: wire::CommandDisposition::Accepted,
            ..
        }
    ));
    assert_eq!(
        presence_step.facet_revision,
        presence_welcome.facet_revision + 2,
        "command and presence commits must occupy consecutive mailbox revisions"
    );
    let durable_presence = ev_facet_row(pool, fixture.world_id).await;
    assert_eq!(
        durable_presence.0,
        i64::try_from(presence_step.facet_revision).unwrap()
    );
    assert_eq!(
        durable_presence.1,
        i64::try_from(presence_step.server_sequence).unwrap()
    );
    assert_eq!(
        durable_presence.2.as_slice(),
        presence_step.checkpoint.as_bytes()
    );

    let (replacement, replacement_welcome, _replacement_outbound, _replacement_receive) =
        ev_admit_character(state, cookie).await;
    let reply_loss_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let reply_loss_wire = ev_wire_command(
        &replacement,
        reply_loss_id,
        1,
        replacement_welcome.facet_revision,
        false,
    );
    let reply_loss_digest = ev_reserve_new(state, fixture, reply_loss_id, &reply_loss_wire).await;
    let (release, entered) = replacement.facet.ev_hold_mailbox().await;
    entered.await.unwrap();
    let reply_loss = replacement
        .facet
        .try_command(ev_facet_command(
            &replacement,
            reply_loss_id,
            1,
            replacement_welcome.facet_revision,
            false,
            reply_loss_digest,
            EvCommandFault::None,
        ))
        .unwrap();
    drop(reply_loss);
    release.send(()).unwrap();
    loop {
        if state
            .store
            .receipt(fixture.account_id, reply_loss_id)
            .await
            .unwrap()
            .is_some()
        {
            break;
        }
        tokio::task::yield_now().await;
    }
    state
        .coordinator
        .release(fixture.account_id, reply_loss_id, reply_loss_digest);
    let expected = state
        .store
        .receipt(fixture.account_id, reply_loss_id)
        .await
        .unwrap()
        .unwrap()
        .outcome
        .unwrap()
        .to_envelope(reply_loss_id, wire::ReplayStatus::Replayed)
        .unwrap();
    let replay = match state
        .coordinator
        .reserve(fixture.account_id, reply_loss_id, &reply_loss_wire)
        .await
    {
        Reservation::Replay(envelope) => *envelope,
        _ => panic!("committed command with lost reply must replay"),
    };
    assert_eq!(replay, expected);
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, reply_loss_id).await,
        (1, 1)
    );

    let rows = sqlx::query(
        "SELECT command_id,before_revision,after_revision,server_sequence \
         FROM tme.command_receipts WHERE account_id=$1 AND command_id=ANY($2) \
         ORDER BY server_sequence",
    )
    .bind(fixture.account_id.as_uuid())
    .bind(vec![
        accepted_id.as_uuid(),
        publication_id.as_uuid(),
        reply_loss_id.as_uuid(),
    ])
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 3);
    let mut previous_sequence = None;
    for row in rows {
        let command_id: Uuid = row.try_get("command_id").unwrap();
        let before: i64 = row.try_get("before_revision").unwrap();
        let after: i64 = row.try_get("after_revision").unwrap();
        let sequence: i64 = row.try_get("server_sequence").unwrap();
        if command_id == publication_id.as_uuid() {
            assert_eq!(after, before, "projection rejection changed the revision");
        } else {
            assert_eq!(after, before + 1);
        }
        if let Some(previous) = previous_sequence {
            assert!(
                sequence > previous,
                "mailbox sequences remain strictly ordered"
            );
        }
        previous_sequence = Some(sequence);
    }
    eprintln!("EV source-fault stage complete: command pipeline");
}

#[cfg(test)]
async fn ev_certify_command_postcommit_reload(
    database_url: &str,
    pool: &PgPool,
    state: Arc<PostgresState>,
    fixture: EvDatabaseFixture,
    cookie: &str,
    fault: EvCommandFault,
    enabled: bool,
) -> Arc<PostgresState> {
    use crate::coordinator::Reservation;

    eprintln!("EV source-fault stage: command postcommit reload");
    let (grant, welcome, _outbound, mut outbound_receive) =
        ev_admit_character(&state, cookie).await;
    let before_memory = ev_current_state(&grant).await;
    let before_store = ev_facet_row(pool, fixture.world_id).await;
    let command_id = wire::CommandId::new(Uuid::now_v7()).unwrap();
    let wire_command = ev_wire_command(&grant, command_id, 1, welcome.facet_revision, enabled);
    let digest = ev_reserve_new(&state, fixture, command_id, &wire_command).await;
    if fault == EvCommandFault::CommitOutcomeUnknown {
        state
            .store
            .ev_arm_fault(crate::store::EvStoreFault::CommandCommitOutcomeUnknown);
    }
    let receive = grant
        .facet
        .try_command(ev_facet_command(
            &grant,
            command_id,
            1,
            welcome.facet_revision,
            enabled,
            digest,
            fault,
        ))
        .unwrap();
    assert!(
        receive.await.is_err(),
        "postcommit command fault emitted a success reply"
    );
    if fault == EvCommandFault::CommitOutcomeUnknown {
        state.store.ev_assert_fault_consumed();
    }
    state
        .coordinator
        .release(fixture.account_id, command_id, digest);
    assert!(!state.gameplay_ready());
    assert_eq!(ev_current_state(&grant).await, before_memory);
    assert!(outbound_receive.try_recv().is_err());
    let committed_store = ev_facet_row(pool, fixture.world_id).await;
    assert_eq!(committed_store.0, before_store.0 + 1);
    assert_eq!(committed_store.1, before_store.1 + 1);
    assert_ne!(committed_store.2, before_store.2);
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, command_id).await,
        (1, 1)
    );
    let expected_replay = state
        .store
        .receipt(fixture.account_id, command_id)
        .await
        .unwrap()
        .unwrap()
        .outcome
        .unwrap()
        .to_envelope(command_id, wire::ReplayStatus::Replayed)
        .unwrap();

    drop(state);
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    let reloaded = PostgresState::open(database_url, ev_database_bootstrap(fixture))
        .await
        .expect("command postcommit reload opens");
    assert!(reloaded.gameplay_ready());
    let replay = match reloaded
        .coordinator
        .reserve(fixture.account_id, command_id, &wire_command)
        .await
    {
        Reservation::Replay(envelope) => *envelope,
        _ => panic!("postcommit reload did not hydrate the durable receipt"),
    };
    assert_eq!(replay, expected_replay);
    assert_eq!(
        ev_command_artifacts(pool, fixture.account_id, command_id).await,
        (1, 1)
    );
    let (_hydrated_grant, hydrated_welcome, _hydrated_outbound, _hydrated_receive) =
        ev_admit_character(&reloaded, cookie).await;
    assert_eq!(
        hydrated_welcome.frame.social.pages_enabled, enabled,
        "reload did not hydrate the committed command checkpoint",
    );
    eprintln!("EV source-fault stage complete: command postcommit reload");
    reloaded
}

#[cfg(test)]
async fn ev_assert_required_task_revokes_readiness(state: &PostgresState, label: &str) {
    for _ in 0..64 {
        if !state.gameplay_ready() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        !state.gameplay_ready(),
        "required {label} exit must revoke readiness"
    );
    assert!(
        state.ready.seal_ready().is_err(),
        "required {label} exit must make readiness irreversible"
    );
}

#[cfg(test)]
#[tokio::test(flavor = "current_thread", start_paused = true)]
#[ignore = "requires the exact runner-owned EV PostgreSQL 18 database"]
async fn ev_database_fault_certification() {
    let database_url =
        std::env::var("TME_EV_DATABASE_URL").expect("EV runner must provide TME_EV_DATABASE_URL");
    let expected_database =
        std::env::var("TME_EV_DATABASE_NAME").expect("EV runner must provide TME_EV_DATABASE_NAME");
    let expected_sentinel = std::env::var("TME_EV_DATABASE_SENTINEL")
        .expect("EV runner must provide TME_EV_DATABASE_SENTINEL");
    let expected_role =
        std::env::var("TME_EV_DATABASE_ROLE").expect("EV runner must provide TME_EV_DATABASE_ROLE");
    assert!(expected_database.starts_with("tme_ev_"));
    assert!(!expected_sentinel.is_empty());
    assert!(expected_role.starts_with("tme_ev_role_"));

    let (anchor_entered, anchor_entered_receive) = oneshot::channel();
    let anchor = tokio::spawn(async move {
        let _ = anchor_entered.send(());
        loop {
            tokio::task::yield_now().await;
        }
    });
    anchor_entered_receive
        .await
        .expect("EV paused-time yield anchor enters");
    assert!(!anchor.is_finished());

    let pool = runtime_pool(&database_url)
        .await
        .expect("runner-owned EV database connects");
    let row = sqlx::query(
        "SELECT current_database() AS database_name,current_user AS role_name,\
         shobj_description(oid,'pg_database') AS database_comment,\
         current_setting('server_version_num')::integer AS server_version_num \
         FROM pg_database WHERE datname=current_database()",
    )
    .fetch_one(&pool)
    .await
    .expect("runner-owned EV database identity is readable");
    assert_eq!(
        row.try_get::<String, _>("database_name").unwrap(),
        expected_database
    );
    assert_eq!(
        row.try_get::<String, _>("role_name").unwrap(),
        expected_role
    );
    assert_eq!(
        row.try_get::<String, _>("database_comment").unwrap(),
        format!("tme_ev:{expected_sentinel}")
    );
    assert!((180_000..190_000).contains(&row.try_get::<i32, _>("server_version_num").unwrap()));
    migrations::verify(&pool)
        .await
        .expect("runner-owned EV database has the exact tracked migrations");
    certify_command_reservation_race(&pool).await;

    let fixture = EvDatabaseFixture {
        account_id: wire::AccountId::new(Uuid::now_v7()).unwrap(),
        character_id: wire::CharacterId::new(Uuid::now_v7()).unwrap(),
        session_id: wire::SessionId::new(Uuid::now_v7()).unwrap(),
        world_id: wire::FacetId::new(Uuid::now_v7()).unwrap(),
    };
    let cookie = "ev-source-fault-cookie";
    let csrf = wire::CsrfToken::new("A".repeat(43)).unwrap();
    ev_insert_account(&pool, fixture).await;
    let state = PostgresState::open(&database_url, ev_database_bootstrap(fixture))
        .await
        .expect("EV source-fault service opens");
    ev_insert_session(&pool, fixture, cookie, &csrf).await;
    let logical_before = tokio::time::Instant::now();
    let wall_before = std::time::Instant::now();
    let (wall_send, wall_receive) = oneshot::channel();
    let wall_thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(25));
        let _ = wall_send.send(());
    });
    wall_receive.await.expect("EV wall-clock probe completes");
    wall_thread.join().expect("EV wall-clock probe joins");
    assert!(wall_before.elapsed() >= Duration::from_millis(25));
    assert_eq!(
        tokio::time::Instant::now(),
        logical_before,
        "real wall time must not advance paused logical time"
    );
    assert!(!anchor.is_finished());

    ev_certify_direct_store_failures(&database_url, &pool, &state, fixture).await;
    ev_certify_command_pipeline(&pool, &state, fixture, cookie).await;
    let state = ev_certify_command_postcommit_reload(
        &database_url,
        &pool,
        state,
        fixture,
        cookie,
        EvCommandFault::CommitOutcomeUnknown,
        true,
    )
    .await;
    let state = ev_certify_command_postcommit_reload(
        &database_url,
        &pool,
        state,
        fixture,
        cookie,
        EvCommandFault::AfterStoreCommit,
        false,
    )
    .await;
    assert!(state.gameplay_ready());

    state.world.handle.ev_abort_facet_task();
    ev_assert_required_task_revokes_readiness(&state, "persisted facet").await;
    drop(state);
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    let state = PostgresState::open(&database_url, ev_database_bootstrap(fixture))
        .await
        .expect("EV reload after persisted-facet abort opens");
    assert!(state.gameplay_ready());

    state.world.handle.ev_abort_scheduler_task();
    ev_assert_required_task_revokes_readiness(&state, "facet scheduler").await;
    drop(state);
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    let state = PostgresState::open(&database_url, ev_database_bootstrap(fixture))
        .await
        .expect("EV reload after scheduler abort opens");
    assert!(state.gameplay_ready());

    let reconciler = state.required_tasks.abort_reconciler();
    assert!(
        reconciler.await.unwrap_err().is_cancelled(),
        "EV reconciler abort must cancel the live task"
    );
    ev_assert_required_task_revokes_readiness(&state, "reconciler").await;
    drop(state);
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    pool.close().await;
    anchor.abort();
    let _ = anchor.await;
}

#[cfg(test)]
mod certification_tests {
    use super::*;

    #[test]
    fn failed_startup_readiness_cannot_be_resealed() {
        let readiness = GameplayReadiness::new();
        readiness.fail();
        assert!(readiness.seal_ready().is_err());
        assert!(!readiness.is_ready());
    }

    #[test]
    fn required_task_exit_revokes_sealed_readiness() {
        let readiness = GameplayReadiness::new();
        readiness.seal_ready().unwrap();
        assert!(readiness.is_ready());
        readiness.fail();
        assert!(!readiness.is_ready());
        assert!(readiness.seal_ready().is_err());
    }
}
