use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use tme_protocol as wire;
use tme_rules::Engine;
use tokio::sync::{mpsc, oneshot, watch};

use crate::admission::ControlGrant;
use crate::config::FACET_MAILBOX_CAPACITY;
use crate::store::receipt::ReceiptOutcomeV3;
use crate::store::{CommandCommit, CommandCommitError, SharedStore, SystemCommit};

#[derive(Clone)]
pub struct FacetHandle {
    facet_id: wire::FacetId,
    sender: mpsc::Sender<FacetRequest>,
    _lifecycle: Arc<FacetLifecycle>,
}

struct FacetLifecycle {
    facet: tokio::task::AbortHandle,
    supervisor: tokio::task::JoinHandle<()>,
    scheduler: Option<tokio::task::JoinHandle<()>>,
}

impl Drop for FacetLifecycle {
    fn drop(&mut self) {
        self.facet.abort();
        self.supervisor.abort();
        if let Some(scheduler) = &self.scheduler {
            scheduler.abort();
        }
    }
}

impl FacetHandle {
    pub fn spawn(engine: Engine) -> Self {
        let facet_id = wire::FacetId::new(uuid::Uuid::now_v7()).expect("UUIDv7 is valid");
        Self::spawn_with_id(facet_id, engine)
    }

    pub fn spawn_with_id(facet_id: wire::FacetId, engine: Engine) -> Self {
        Self::spawn_state(FacetTaskState {
            facet_id,
            engine,
            facet_revision: 0,
            server_sequence: 0,
            store: None,
            readiness: None,
            coordinator: None,
            startup: None,
            #[cfg(test)]
            startup_trace: None,
        })
    }

    #[cfg(test)]
    fn spawn_certification(
        facet_id: wire::FacetId,
        engine: Engine,
        actor_id: tme_rules::ActorId,
    ) -> (Self, oneshot::Receiver<CertificationStep>) {
        let (startup_trace, receive) = oneshot::channel();
        (
            Self::spawn_state(FacetTaskState {
                facet_id,
                engine,
                facet_revision: 0,
                server_sequence: 0,
                store: None,
                readiness: None,
                coordinator: None,
                startup: None,
                startup_trace: Some((actor_id, startup_trace)),
            }),
            receive,
        )
    }

    pub(crate) fn spawn_persisted(
        facet_id: wire::FacetId,
        engine: Engine,
        facet_revision: u64,
        server_sequence: u64,
        store: SharedStore,
        readiness: Arc<crate::postgres::GameplayReadiness>,
        coordinator: Arc<crate::coordinator::Coordinator>,
    ) -> (Self, oneshot::Receiver<()>) {
        let (startup, started) = oneshot::channel();
        (
            Self::spawn_state(FacetTaskState {
                facet_id,
                engine,
                facet_revision,
                server_sequence,
                store: Some(store),
                readiness: Some(readiness),
                coordinator: Some(coordinator),
                startup: Some(startup),
                #[cfg(test)]
                startup_trace: None,
            }),
            started,
        )
    }

    fn spawn_state(state: FacetTaskState) -> Self {
        let facet_id = state.facet_id;
        let (sender, receiver) = mpsc::channel(FACET_MAILBOX_CAPACITY);
        let scheduler = state
            .store
            .is_some()
            .then(|| crate::scheduler::spawn(sender.clone(), state.readiness.clone()));
        let readiness_guard = FacetReadinessGuard(state.readiness.clone());
        let task = tokio::spawn(run_facet(state, receiver, readiness_guard));
        let facet = task.abort_handle();
        let supervisor = tokio::spawn(async move {
            if task.await.is_err() {
                crate::telemetry::record_facet_task_panic();
            }
        });
        Self {
            facet_id,
            sender,
            _lifecycle: Arc::new(FacetLifecycle {
                facet,
                supervisor,
                scheduler,
            }),
        }
    }

    pub fn facet_id(&self) -> wire::FacetId {
        self.facet_id
    }

    pub(crate) fn mailbox_depth(&self) -> usize {
        FACET_MAILBOX_CAPACITY.saturating_sub(self.sender.capacity())
    }

    #[cfg(test)]
    pub(crate) fn ev_abort_facet_task(&self) {
        self._lifecycle.facet.abort();
    }

    #[cfg(test)]
    pub(crate) fn ev_abort_scheduler_task(&self) {
        self._lifecycle
            .scheduler
            .as_ref()
            .expect("persisted EV facet has a scheduler")
            .abort();
    }

    pub async fn install_grant(
        &self,
        grant: ControlGrant,
        outbound: mpsc::Sender<wire::ServerEnvelope>,
        terminal: watch::Sender<Option<wire::DrainingReason>>,
    ) -> Result<FacetWelcome, FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .send(FacetRequest::InstallGrant {
                grant,
                outbound,
                terminal,
                #[cfg(test)]
                certification_trace: None,
                reply,
            })
            .await
            .map_err(|_| FacetError::Unavailable)?;
        receive.await.map_err(|_| FacetError::Unavailable)?
    }

    pub fn try_command(
        &self,
        command: FacetCommand,
    ) -> Result<oneshot::Receiver<FacetCommandReply>, FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .try_send(FacetRequest::Command { command, reply })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => FacetError::QueueFull,
                mpsc::error::TrySendError::Closed(_) => FacetError::Unavailable,
            })?;
        Ok(receive)
    }

    pub fn try_path_preview(
        &self,
        preview: FacetPathPreview,
    ) -> Result<oneshot::Receiver<FacetPathPreviewReply>, FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .try_send(FacetRequest::PathPreview { preview, reply })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => FacetError::QueueFull,
                mpsc::error::TrySendError::Closed(_) => FacetError::Unavailable,
            })?;
        Ok(receive)
    }

    pub fn try_current_state(
        &self,
        connection_id: wire::ConnectionId,
    ) -> Result<oneshot::Receiver<Result<wire::ServerEnvelope, FacetError>>, FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .try_send(FacetRequest::CurrentState {
                connection_id,
                reply,
            })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => FacetError::QueueFull,
                mpsc::error::TrySendError::Closed(_) => FacetError::Unavailable,
            })?;
        Ok(receive)
    }

    #[cfg(test)]
    pub(crate) async fn ev_hold_mailbox(&self) -> (oneshot::Sender<()>, oneshot::Receiver<()>) {
        let (entered, entered_receive) = oneshot::channel();
        let (release, release_receive) = oneshot::channel();
        self.sender
            .send(FacetRequest::Hold {
                entered,
                release: release_receive,
            })
            .await
            .expect("EV facet mailbox remains available");
        (release, entered_receive)
    }

    #[cfg(test)]
    pub(crate) fn ev_try_inspect(
        &self,
        character_id: tme_rules::CharacterId,
    ) -> Result<oneshot::Receiver<FacetInspection>, FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .try_send(FacetRequest::Inspect {
                character_id,
                reply,
            })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => FacetError::QueueFull,
                mpsc::error::TrySendError::Closed(_) => FacetError::Unavailable,
            })?;
        Ok(receive)
    }

    #[cfg(test)]
    pub(crate) fn ev_try_tick(
        &self,
        actor_id: tme_rules::ActorId,
    ) -> Result<oneshot::Receiver<CertificationStep>, FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .try_send(FacetRequest::CertificationTick { actor_id, reply })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => FacetError::QueueFull,
                mpsc::error::TrySendError::Closed(_) => FacetError::Unavailable,
            })?;
        Ok(receive)
    }

    #[cfg(test)]
    pub(crate) fn ev_try_detach(
        &self,
        connection_id: wire::ConnectionId,
    ) -> Result<oneshot::Receiver<CertificationStep>, FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .try_send(FacetRequest::CertificationDetach {
                connection_id,
                reply,
            })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => FacetError::QueueFull,
                mpsc::error::TrySendError::Closed(_) => FacetError::Unavailable,
            })?;
        Ok(receive)
    }

    pub async fn detach(&self, connection_id: wire::ConnectionId) -> Result<(), FacetError> {
        self.sender
            .send(FacetRequest::Detach { connection_id })
            .await
            .map_err(|_| FacetError::Unavailable)
    }

    pub(crate) async fn begin_revoke_grant(
        &self,
        connection_id: wire::ConnectionId,
        reason: wire::DrainingReason,
    ) -> Result<oneshot::Receiver<()>, FacetError> {
        let (marked, marked_receive) = oneshot::channel();
        let (completion, completion_receive) = oneshot::channel();
        self.sender
            .send(FacetRequest::RevokeGrant {
                connection_id,
                reason,
                marked,
                completion,
            })
            .await
            .map_err(|_| FacetError::Unavailable)?;
        marked_receive.await.map_err(|_| FacetError::Unavailable)?;
        Ok(completion_receive)
    }

    pub async fn commit_transfer(&self, transfer_epoch: u64) -> Result<(), FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .send(FacetRequest::CommitTransfer {
                transfer_epoch,
                reply,
            })
            .await
            .map_err(|_| FacetError::Unavailable)?;
        receive.await.map_err(|_| FacetError::Unavailable)?
    }

    pub async fn rollback_transfer(&self, transfer_epoch: u64) -> Result<(), FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .send(FacetRequest::RollbackTransfer {
                transfer_epoch,
                reply,
            })
            .await
            .map_err(|_| FacetError::Unavailable)?;
        receive.await.map_err(|_| FacetError::Unavailable)?
    }

    pub async fn publish_transfer(&self, transfer_epoch: u64) -> Result<(), FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .send(FacetRequest::PublishTransfer {
                transfer_epoch,
                reply,
            })
            .await
            .map_err(|_| FacetError::Unavailable)?;
        receive.await.map_err(|_| FacetError::Unavailable)?
    }

    pub(crate) async fn prepared_checkpoint(
        &self,
        transfer_epoch: u64,
    ) -> Result<PreparedFacetCheckpoint, FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .send(FacetRequest::PreparedCheckpoint {
                transfer_epoch,
                reply,
            })
            .await
            .map_err(|_| FacetError::Unavailable)?;
        receive.await.map_err(|_| FacetError::Unavailable)?
    }

    pub(crate) async fn prepare_control(
        &self,
        character_id: wire::CharacterId,
    ) -> Result<(), FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .send(FacetRequest::PrepareControl {
                character_id,
                reply,
            })
            .await
            .map_err(|_| FacetError::Unavailable)?;
        receive.await.map_err(|_| FacetError::Unavailable)?
    }

    pub(crate) async fn prepare_player_kill_forgiveness(
        &self,
        mutation_epoch: u64,
        assessment: tme_rules::PlayerKillAssessmentV1,
    ) -> Result<(), FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .send(FacetRequest::PreparePlayerKillForgiveness {
                mutation_epoch,
                assessment,
                reply,
            })
            .await
            .map_err(|_| FacetError::Unavailable)?;
        receive.await.map_err(|_| FacetError::Unavailable)?
    }

    /// Applies every consequence a returning killer owes, as ONE candidate
    /// engine. Owner ruling 2026-08-20 (#3). Returns the per-kill
    /// `linked_karma_added` the rules produced, in the order given, so the
    /// caller can correct the marks in the transaction that persists this.
    pub(crate) async fn prepare_pending_kill_consequences(
        &self,
        mutation_epoch: u64,
        assessments: Vec<tme_rules::PlayerKillAssessmentV1>,
    ) -> Result<Vec<bool>, FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .send(FacetRequest::PreparePendingKillConsequences {
                mutation_epoch,
                assessments,
                reply,
            })
            .await
            .map_err(|_| FacetError::Unavailable)?;
        receive.await.map_err(|_| FacetError::Unavailable)?
    }

    pub(crate) async fn prepare_character_exit(
        &self,
        mutation_epoch: u64,
        character_id: tme_rules::CharacterId,
    ) -> Result<(), FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .send(FacetRequest::PrepareCharacterExit {
                mutation_epoch,
                character_id,
                reply,
            })
            .await
            .map_err(|_| FacetError::Unavailable)?;
        receive.await.map_err(|_| FacetError::Unavailable)?
    }

    pub(crate) async fn resume_control(
        &self,
        character_id: wire::CharacterId,
    ) -> Result<(), FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .send(FacetRequest::ResumeControl {
                character_id,
                reply,
            })
            .await
            .map_err(|_| FacetError::Unavailable)?;
        receive.await.map_err(|_| FacetError::Unavailable)
    }

    pub(crate) async fn social_message(
        &self,
        grant: ControlGrant,
        message_id: wire::MessageId,
        scope: wire::SocialScope,
        body: wire::SocialBody,
    ) -> Result<FacetSocialOutcome, FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .send(FacetRequest::SocialMessage {
                grant,
                message_id,
                scope,
                body,
                reply,
            })
            .await
            .map_err(|_| FacetError::Unavailable)?;
        receive.await.map_err(|_| FacetError::Unavailable)
    }

    pub(crate) async fn deliver_page(
        &self,
        target: ControlGrant,
        message_id: wire::MessageId,
        sender_character_id: wire::CharacterId,
        sender_name: wire::DisplayName,
        body: wire::SocialBody,
    ) -> Result<bool, FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .send(FacetRequest::DeliverPage {
                target,
                message_id,
                sender_character_id,
                sender_name,
                body,
                reply,
            })
            .await
            .map_err(|_| FacetError::Unavailable)?;
        receive.await.map_err(|_| FacetError::Unavailable)
    }
}

#[derive(Debug)]
pub enum FacetError {
    ActorAlreadyAttached,
    InvalidActor,
    QueueFull,
    Unavailable,
    Projection,
    Transfer,
}

pub struct FacetWelcome {
    pub server_sequence: u64,
    pub facet_revision: u64,
    pub static_scene_context: wire::StaticSceneContext,
    pub frame: wire::ObserverFrame,
}

pub(crate) enum FacetSocialOutcome {
    Complete(wire::MessageDisposition),
    PagePrepared {
        target_character_id: wire::CharacterId,
        sender_name: wire::DisplayName,
    },
}

pub struct FacetCommand {
    pub connection_id: wire::ConnectionId,
    pub account_id: wire::AccountId,
    pub session_id: wire::SessionId,
    pub character_id: wire::CharacterId,
    pub command_id: wire::CommandId,
    pub control_epoch: u64,
    pub client_sequence: u64,
    pub observed_facet_revision: u64,
    pub actor_id: wire::ActorId,
    pub intent: wire::Intent,
    pub request_digest: [u8; 32],
    #[cfg(test)]
    pub(crate) certification_trace: Option<oneshot::Sender<CertificationStep>>,
    #[cfg(test)]
    pub(crate) ev_fail_checkpoint_export: bool,
    #[cfg(test)]
    pub(crate) ev_fail_after_store_commit: bool,
}

pub struct FacetPathPreview {
    pub grant: ControlGrant,
    pub preview_id: wire::PreviewId,
    pub control_epoch: u64,
    pub observed_facet_revision: u64,
    pub actor_id: wire::ActorId,
    pub path: Vec<wire::Direction>,
}

#[derive(Debug, Clone)]
pub struct FacetPathPreviewReply {
    pub envelope: wire::ServerEnvelope,
}

#[derive(Debug, Clone)]
pub struct FacetCommandReply {
    pub envelope: wire::ServerEnvelope,
}

pub(crate) struct PreparedFacetCheckpoint {
    pub facet_id: wire::FacetId,
    pub before_revision: u64,
    pub after_revision: u64,
    pub server_sequence: u64,
    pub checkpoint: tme_rules::FacetCheckpointV4,
}

struct Observer {
    grant: ControlGrant,
    outbound: mpsc::Sender<wire::ServerEnvelope>,
    terminal: watch::Sender<Option<wire::DrainingReason>>,
    expected_client_sequence: u64,
}

struct PendingDetach {
    grant: ControlGrant,
    _outbound: mpsc::Sender<wire::ServerEnvelope>,
    terminal: watch::Sender<Option<wire::DrainingReason>>,
    reason: Option<wire::DrainingReason>,
    completions: Vec<oneshot::Sender<()>>,
    #[cfg(test)]
    certification_trace: Option<oneshot::Sender<CertificationStep>>,
}

pub(super) enum FacetRequest {
    #[cfg(test)]
    Hold {
        entered: oneshot::Sender<()>,
        release: oneshot::Receiver<()>,
    },
    #[cfg(test)]
    Inspect {
        character_id: tme_rules::CharacterId,
        reply: oneshot::Sender<FacetInspection>,
    },
    InstallGrant {
        grant: ControlGrant,
        outbound: mpsc::Sender<wire::ServerEnvelope>,
        terminal: watch::Sender<Option<wire::DrainingReason>>,
        #[cfg(test)]
        certification_trace: Option<oneshot::Sender<CertificationStep>>,
        reply: oneshot::Sender<Result<FacetWelcome, FacetError>>,
    },
    Command {
        command: FacetCommand,
        reply: oneshot::Sender<FacetCommandReply>,
    },
    PathPreview {
        preview: FacetPathPreview,
        reply: oneshot::Sender<FacetPathPreviewReply>,
    },
    CurrentState {
        connection_id: wire::ConnectionId,
        reply: oneshot::Sender<Result<wire::ServerEnvelope, FacetError>>,
    },
    Detach {
        connection_id: wire::ConnectionId,
    },
    #[cfg(test)]
    CertificationDetach {
        connection_id: wire::ConnectionId,
        reply: oneshot::Sender<CertificationStep>,
    },
    RevokeGrant {
        connection_id: wire::ConnectionId,
        reason: wire::DrainingReason,
        marked: oneshot::Sender<()>,
        completion: oneshot::Sender<()>,
    },
    CommitTransfer {
        transfer_epoch: u64,
        reply: oneshot::Sender<Result<(), FacetError>>,
    },
    RollbackTransfer {
        transfer_epoch: u64,
        reply: oneshot::Sender<Result<(), FacetError>>,
    },
    PublishTransfer {
        transfer_epoch: u64,
        reply: oneshot::Sender<Result<(), FacetError>>,
    },
    PreparedCheckpoint {
        transfer_epoch: u64,
        reply: oneshot::Sender<Result<PreparedFacetCheckpoint, FacetError>>,
    },
    PrepareControl {
        character_id: wire::CharacterId,
        reply: oneshot::Sender<Result<(), FacetError>>,
    },
    PreparePlayerKillForgiveness {
        mutation_epoch: u64,
        assessment: tme_rules::PlayerKillAssessmentV1,
        reply: oneshot::Sender<Result<(), FacetError>>,
    },
    PreparePendingKillConsequences {
        mutation_epoch: u64,
        assessments: Vec<tme_rules::PlayerKillAssessmentV1>,
        reply: oneshot::Sender<Result<Vec<bool>, FacetError>>,
    },
    PrepareCharacterExit {
        mutation_epoch: u64,
        character_id: tme_rules::CharacterId,
        reply: oneshot::Sender<Result<(), FacetError>>,
    },
    ResumeControl {
        character_id: wire::CharacterId,
        reply: oneshot::Sender<()>,
    },
    Tick,
    #[cfg(test)]
    CertificationTick {
        actor_id: tme_rules::ActorId,
        reply: oneshot::Sender<CertificationStep>,
    },
    SocialMessage {
        grant: ControlGrant,
        message_id: wire::MessageId,
        scope: wire::SocialScope,
        body: wire::SocialBody,
        reply: oneshot::Sender<FacetSocialOutcome>,
    },
    DeliverPage {
        target: ControlGrant,
        message_id: wire::MessageId,
        sender_character_id: wire::CharacterId,
        sender_name: wire::DisplayName,
        body: wire::SocialBody,
        reply: oneshot::Sender<bool>,
    },
}

#[cfg(test)]
pub(super) struct FacetInspection {
    pub(crate) active_observers: usize,
    pub(crate) pending_detaches: usize,
    pub(crate) connected: bool,
    pub(crate) server_sequence: u64,
    pub(crate) facet_revision: u64,
    pub(crate) projection: tme_rules::ObserverProjectionV1,
    pub(crate) checkpoint: tme_rules::FacetCheckpointV4,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CertificationStep {
    pub(crate) outcome: tme_rules::RulesOutcomeV1,
    pub(crate) projection: tme_rules::ObserverProjectionV1,
    pub(crate) server_sequence: u64,
    pub(crate) facet_revision: u64,
    pub(crate) checkpoint: tme_rules::FacetCheckpointV4,
}

struct PreparedTransfer {
    epoch: u64,
    candidate: Option<Engine>,
    committed: bool,
}

struct FacetReadinessGuard(Option<Arc<crate::postgres::GameplayReadiness>>);

struct FacetTaskState {
    facet_id: wire::FacetId,
    engine: Engine,
    facet_revision: u64,
    server_sequence: u64,
    store: Option<SharedStore>,
    readiness: Option<Arc<crate::postgres::GameplayReadiness>>,
    coordinator: Option<Arc<crate::coordinator::Coordinator>>,
    startup: Option<oneshot::Sender<()>>,
    #[cfg(test)]
    startup_trace: Option<(tme_rules::ActorId, oneshot::Sender<CertificationStep>)>,
}

impl Drop for FacetReadinessGuard {
    fn drop(&mut self) {
        if let Some(readiness) = &self.0 {
            readiness.fail();
        }
    }
}

async fn run_facet(
    state: FacetTaskState,
    mut receiver: mpsc::Receiver<FacetRequest>,
    readiness_guard: FacetReadinessGuard,
) {
    let FacetTaskState {
        facet_id,
        mut engine,
        mut facet_revision,
        mut server_sequence,
        store,
        readiness,
        coordinator,
        mut startup,
        #[cfg(test)]
        mut startup_trace,
    } = state;
    let _readiness_guard = readiness_guard;
    let mut observers = BTreeMap::<wire::ConnectionId, Observer>::new();
    let mut pending_detaches = BTreeMap::<wire::ConnectionId, PendingDetach>::new();
    let mut quiesced_characters = BTreeSet::<wire::CharacterId>::new();
    let mut prepared_transfer: Option<PreparedTransfer> = None;

    let mut recovery_candidate = engine.clone();
    let recovery_outcome = match recovery_candidate.mark_all_characters_disconnected() {
        Ok(outcome) => outcome,
        Err(_) => return,
    };
    #[cfg(test)]
    let recovery_trace_outcome = recovery_outcome.clone();
    if !commit_system_mutation(
        facet_id,
        &mut engine,
        &mut server_sequence,
        &mut facet_revision,
        &mut observers,
        &mut pending_detaches,
        recovery_candidate,
        recovery_outcome,
        &store,
        &coordinator,
        "facet_presence",
    )
    .await
    {
        return;
    }
    #[cfg(test)]
    if let Some((actor_id, trace)) = startup_trace.take()
        && trace
            .send(certification_step(
                &engine,
                &actor_id,
                recovery_trace_outcome,
                server_sequence,
                facet_revision,
            ))
            .is_err()
    {
        return;
    }
    if startup
        .take()
        .is_some_and(|startup| startup.send(()).is_err())
    {
        return;
    }

    while let Some(request) = receiver.recv().await {
        match request {
            #[cfg(test)]
            FacetRequest::Hold { entered, release } => {
                let _ = entered.send(());
                let _ = release.await;
            }
            #[cfg(test)]
            FacetRequest::Inspect {
                character_id,
                reply,
            } => {
                let connected = engine
                    .world()
                    .character_presence
                    .get(&character_id)
                    .is_some_and(|presence| presence.connected);
                let actor_id = engine
                    .world()
                    .actors
                    .iter()
                    .find(|actor| actor.character_id.as_ref() == Some(&character_id))
                    .map(|actor| actor.id.clone())
                    .expect("certification character has an actor");
                let _ = reply.send(FacetInspection {
                    active_observers: observers.len(),
                    pending_detaches: pending_detaches.len(),
                    connected,
                    server_sequence,
                    facet_revision,
                    projection: engine
                        .observer_projection(&actor_id, &[])
                        .expect("certification actor projection succeeds"),
                    checkpoint: engine
                        .export_checkpoint()
                        .expect("certification engine exports Checkpoint 3"),
                });
            }
            FacetRequest::InstallGrant {
                grant,
                outbound,
                terminal,
                #[cfg(test)]
                certification_trace,
                reply,
            } => {
                let result = if grant.facet_id != facet_id || prepared_transfer.is_some() {
                    Err(FacetError::InvalidActor)
                } else {
                    let mut candidate = engine.clone();
                    let rules_character_id =
                        tme_rules::CharacterId::new(grant.character_id.to_string());
                    let presence = candidate.apply_connection_presence(
                        &rules_character_id,
                        grant.control_epoch,
                        true,
                    );
                    #[cfg(test)]
                    let certification_outcome = presence.clone().ok();
                    let presence_committed = match presence {
                        Ok(outcome) => {
                            commit_system_mutation(
                                facet_id,
                                &mut engine,
                                &mut server_sequence,
                                &mut facet_revision,
                                &mut observers,
                                &mut pending_detaches,
                                candidate,
                                outcome,
                                &store,
                                &coordinator,
                                "facet_presence",
                            )
                            .await
                        }
                        Err(_) => false,
                    };
                    if !presence_committed {
                        let _ = reply.send(Err(FacetError::Unavailable));
                        return;
                    }
                    #[cfg(test)]
                    if let Some(trace) = certification_trace {
                        let Some(outcome) = certification_outcome else {
                            let _ = reply.send(Err(FacetError::Unavailable));
                            return;
                        };
                        if trace
                            .send(certification_step(
                                &engine,
                                &grant.actor_id,
                                outcome,
                                server_sequence,
                                facet_revision,
                            ))
                            .is_err()
                        {
                            let _ = reply.send(Err(FacetError::Unavailable));
                            return;
                        }
                    }
                    match engine.observer_projection(&grant.actor_id, &[]) {
                        Ok(projection) => match (
                            crate::protocol_v1::static_scene_context(
                                &projection.static_scene_context,
                            ),
                            crate::protocol_v1::frame(&projection.frame),
                        ) {
                            (Ok(static_scene_context), Ok(frame)) => {
                                let replaced = observers
                                    .iter()
                                    .filter_map(|(connection_id, observer)| {
                                        (observer.grant.character_id == grant.character_id)
                                            .then_some(*connection_id)
                                    })
                                    .collect::<Vec<_>>();
                                for connection_id in replaced {
                                    if let Some(observer) = observers.remove(&connection_id) {
                                        observer.terminal.send_replace(Some(
                                            wire::DrainingReason::ControlReplaced,
                                        ));
                                    }
                                }
                                let character_id = grant.character_id;
                                observers.insert(
                                    grant.connection_id,
                                    Observer {
                                        grant,
                                        outbound,
                                        terminal,
                                        expected_client_sequence: 1,
                                    },
                                );
                                quiesced_characters.remove(&character_id);
                                Ok(FacetWelcome {
                                    server_sequence,
                                    facet_revision,
                                    static_scene_context,
                                    frame,
                                })
                            }
                            _ => Err(FacetError::Projection),
                        },
                        Err(_) => Err(FacetError::InvalidActor),
                    }
                };
                let _ = reply.send(result);
            }
            FacetRequest::Detach { connection_id } => {
                mark_pending_detach(&mut observers, &mut pending_detaches, connection_id, None);
            }
            #[cfg(test)]
            FacetRequest::CertificationDetach {
                connection_id,
                reply,
            } => {
                mark_pending_detach(&mut observers, &mut pending_detaches, connection_id, None);
                let Some(pending) = pending_detaches.get_mut(&connection_id) else {
                    return;
                };
                pending.certification_trace = Some(reply);
            }
            FacetRequest::RevokeGrant {
                connection_id,
                reason,
                marked,
                completion,
            } => {
                mark_pending_detach(
                    &mut observers,
                    &mut pending_detaches,
                    connection_id,
                    Some(reason),
                );
                if let Some(pending) = pending_detaches.get_mut(&connection_id) {
                    pending.completions.push(completion);
                } else {
                    let _ = completion.send(());
                }
                let _ = marked.send(());
            }
            FacetRequest::CommitTransfer {
                transfer_epoch,
                reply,
            } => {
                let result = match prepared_transfer.as_mut() {
                    Some(prepared) if prepared.epoch == transfer_epoch && !prepared.committed => {
                        engine = prepared
                            .candidate
                            .take()
                            .expect("prepared transfer owns candidate");
                        facet_revision = facet_revision.saturating_add(1);
                        prepared.committed = true;
                        Ok(())
                    }
                    _ => Err(FacetError::Transfer),
                };
                let _ = reply.send(result);
            }
            FacetRequest::RollbackTransfer {
                transfer_epoch,
                reply,
            } => {
                let result = match prepared_transfer.as_ref() {
                    Some(prepared) if prepared.epoch == transfer_epoch && !prepared.committed => {
                        prepared_transfer = None;
                        Ok(())
                    }
                    _ => Err(FacetError::Transfer),
                };
                let _ = reply.send(result);
            }
            FacetRequest::PublishTransfer {
                transfer_epoch,
                reply,
            } => {
                let result = match prepared_transfer.take() {
                    Some(prepared) if prepared.epoch == transfer_epoch && prepared.committed => {
                        let connection_ids = observers.keys().copied().collect::<Vec<_>>();
                        for connection_id in connection_ids {
                            let delivered = observers.get(&connection_id).is_some_and(|observer| {
                                send_issuer_update(
                                    &engine,
                                    server_sequence,
                                    facet_revision,
                                    observer,
                                )
                            });
                            if !delivered {
                                mark_pending_detach(
                                    &mut observers,
                                    &mut pending_detaches,
                                    connection_id,
                                    None,
                                );
                            }
                        }
                        Ok(())
                    }
                    Some(prepared) => {
                        prepared_transfer = Some(prepared);
                        Err(FacetError::Transfer)
                    }
                    None => Err(FacetError::Transfer),
                };
                let _ = reply.send(result);
            }
            FacetRequest::PreparedCheckpoint {
                transfer_epoch,
                reply,
            } => {
                let result = match prepared_transfer.as_ref() {
                    Some(prepared) if prepared.epoch == transfer_epoch && !prepared.committed => {
                        match facet_revision.checked_add(1) {
                            Some(after_revision) => prepared
                                .candidate
                                .as_ref()
                                .expect("uncommitted transfer owns candidate")
                                .export_checkpoint()
                                .map(|checkpoint| PreparedFacetCheckpoint {
                                    facet_id,
                                    before_revision: facet_revision,
                                    after_revision,
                                    server_sequence,
                                    checkpoint,
                                })
                                .map_err(|_| FacetError::Transfer),
                            None => Err(FacetError::Transfer),
                        }
                    }
                    _ => Err(FacetError::Transfer),
                };
                let _ = reply.send(result);
            }
            FacetRequest::PrepareControl {
                character_id,
                reply,
            } => {
                let result = if quiesced_characters.insert(character_id) {
                    Ok(())
                } else {
                    Err(FacetError::Unavailable)
                };
                let _ = reply.send(result);
            }
            FacetRequest::PreparePlayerKillForgiveness {
                mutation_epoch,
                assessment,
                reply,
            } => {
                let result = if prepared_transfer.is_some() {
                    Err(FacetError::Transfer)
                } else {
                    let mut candidate = engine.clone();
                    match candidate.apply_player_kill_karma_forgiveness(&assessment) {
                        Ok(_outcome) => {
                            prepared_transfer = Some(PreparedTransfer {
                                epoch: mutation_epoch,
                                candidate: Some(candidate),
                                committed: false,
                            });
                            Ok(())
                        }
                        Err(_) => Err(FacetError::Transfer),
                    }
                };
                let _ = reply.send(result);
            }
            FacetRequest::PreparePendingKillConsequences {
                mutation_epoch,
                assessments,
                reply,
            } => {
                let result = if prepared_transfer.is_some() {
                    Err(FacetError::Transfer)
                } else {
                    // All of them land on one candidate, so a partial
                    // application can never become durable: either the whole
                    // debt is paid in this admission or none of it is.
                    let mut candidate = engine.clone();
                    let mut linked = Vec::with_capacity(assessments.len());
                    let mut failed = false;
                    for assessment in &assessments {
                        match candidate.apply_absent_killer_player_kill_consequence(assessment) {
                            Ok((_outcome, linked_karma_added)) => linked.push(linked_karma_added),
                            Err(_) => {
                                failed = true;
                                break;
                            }
                        }
                    }
                    if failed {
                        Err(FacetError::Transfer)
                    } else {
                        prepared_transfer = Some(PreparedTransfer {
                            epoch: mutation_epoch,
                            candidate: Some(candidate),
                            committed: false,
                        });
                        Ok(linked)
                    }
                };
                let _ = reply.send(result);
            }
            FacetRequest::PrepareCharacterExit {
                mutation_epoch,
                character_id,
                reply,
            } => {
                let result = if prepared_transfer.is_some() {
                    Err(FacetError::Transfer)
                } else {
                    let mut candidate = engine.clone();
                    let _outcome = candidate.apply_character_session_exit(&character_id);
                    prepared_transfer = Some(PreparedTransfer {
                        epoch: mutation_epoch,
                        candidate: Some(candidate),
                        committed: false,
                    });
                    Ok(())
                };
                let _ = reply.send(result);
            }
            FacetRequest::ResumeControl {
                character_id,
                reply,
            } => {
                quiesced_characters.remove(&character_id);
                let _ = reply.send(());
            }
            FacetRequest::CurrentState {
                connection_id,
                reply,
            } => {
                let result = current_state(
                    &engine,
                    server_sequence,
                    facet_revision,
                    observers.get(&connection_id),
                );
                let _ = reply.send(result);
            }
            FacetRequest::Command { command, reply } => {
                let connection_id = command.connection_id;
                let context = CommandContext {
                    facet_id,
                    transfer_prepared: prepared_transfer.is_some(),
                    control_quiesced: quiesced_characters.contains(&command.character_id),
                    store: &store,
                    readiness: &readiness,
                };
                let issuer_delivery_failed = process_command(
                    context,
                    &mut engine,
                    &mut server_sequence,
                    &mut facet_revision,
                    &mut observers,
                    &mut pending_detaches,
                    command,
                )
                .await
                .is_some_and(|result| reply.send(result).is_err());
                if issuer_delivery_failed {
                    mark_pending_detach(&mut observers, &mut pending_detaches, connection_id, None);
                }
            }
            FacetRequest::PathPreview { preview, reply } => {
                let response = process_path_preview(
                    &engine,
                    facet_revision,
                    &observers,
                    prepared_transfer.is_some()
                        || quiesced_characters.contains(&preview.grant.character_id),
                    preview,
                );
                let _ = reply.send(response);
            }
            FacetRequest::Tick => {
                if prepared_transfer.is_some() {
                    continue;
                }
                if advance_facet_tick(
                    facet_id,
                    &mut engine,
                    &mut server_sequence,
                    &mut facet_revision,
                    &mut observers,
                    &mut pending_detaches,
                    &store,
                    &coordinator,
                )
                .await
                .is_none()
                {
                    return;
                }
            }
            #[cfg(test)]
            FacetRequest::CertificationTick { actor_id, reply } => {
                if prepared_transfer.is_some() {
                    return;
                }
                let Some(outcome) = advance_facet_tick(
                    facet_id,
                    &mut engine,
                    &mut server_sequence,
                    &mut facet_revision,
                    &mut observers,
                    &mut pending_detaches,
                    &store,
                    &coordinator,
                )
                .await
                else {
                    return;
                };
                if reply
                    .send(certification_step(
                        &engine,
                        &actor_id,
                        outcome,
                        server_sequence,
                        facet_revision,
                    ))
                    .is_err()
                {
                    return;
                }
            }
            FacetRequest::SocialMessage {
                grant,
                message_id,
                scope,
                body,
                reply,
            } => {
                let unavailable =
                    || FacetSocialOutcome::Complete(wire::MessageDisposition::Unavailable);
                if prepared_transfer.is_some()
                    || quiesced_characters.contains(&grant.character_id)
                    || observers
                        .get(&grant.connection_id)
                        .is_none_or(|observer| observer.grant != grant)
                {
                    let _ = reply.send(unavailable());
                    continue;
                }
                let sender_character_id =
                    tme_rules::CharacterId::new(grant.character_id.to_string());
                let Some(sender) = engine.world().actor(&grant.actor_id) else {
                    let _ = reply.send(unavailable());
                    continue;
                };
                let Ok(sender_name) = wire::DisplayName::new(&sender.name) else {
                    let _ = reply.send(unavailable());
                    continue;
                };
                if let wire::SocialScope::Page {
                    target_character_id,
                } = scope
                {
                    let allowed = engine.page_source_allows(
                        &sender_character_id,
                        &tme_rules::CharacterId::new(target_character_id.to_string()),
                    );
                    let _ = reply.send(if allowed {
                        FacetSocialOutcome::PagePrepared {
                            target_character_id,
                            sender_name,
                        }
                    } else {
                        unavailable()
                    });
                    continue;
                }
                let rules_scope = match scope {
                    wire::SocialScope::Say => tme_rules::SocialBroadcastScope::Say,
                    wire::SocialScope::Shout => tme_rules::SocialBroadcastScope::Shout,
                    wire::SocialScope::Group => tme_rules::SocialBroadcastScope::Group,
                    wire::SocialScope::Page { .. } => unreachable!(),
                };
                let recipient_ids =
                    match engine.social_broadcast_recipients(&sender_character_id, rules_scope) {
                        Ok(recipient_ids) => recipient_ids,
                        Err(_) if rules_scope == tme_rules::SocialBroadcastScope::Group => {
                            let _ = reply.send(FacetSocialOutcome::Complete(
                                wire::MessageDisposition::NotGrouped,
                            ));
                            continue;
                        }
                        Err(_) => {
                            let _ = reply.send(unavailable());
                            continue;
                        }
                    };
                let recipient_ids = recipient_ids
                    .into_iter()
                    .map(|character_id| character_id.as_str().to_string())
                    .collect::<BTreeSet<_>>();
                let envelope = wire::ServerEnvelope::SocialMessage {
                    message_id,
                    scope,
                    sender_character_id: grant.character_id,
                    sender_name,
                    body,
                };
                let mut slow = Vec::new();
                for (connection_id, observer) in &observers {
                    if recipient_ids.contains(&observer.grant.character_id.to_string())
                        && observer.outbound.try_send(envelope.clone()).is_err()
                    {
                        slow.push(*connection_id);
                    }
                }
                for connection_id in slow {
                    mark_pending_detach(&mut observers, &mut pending_detaches, connection_id, None);
                }
                let _ = reply.send(FacetSocialOutcome::Complete(
                    wire::MessageDisposition::Accepted,
                ));
            }
            FacetRequest::DeliverPage {
                target,
                message_id,
                sender_character_id,
                sender_name,
                body,
                reply,
            } => {
                let target_character_id =
                    tme_rules::CharacterId::new(target.character_id.to_string());
                let allowed = prepared_transfer.is_none()
                    && !quiesced_characters.contains(&target.character_id)
                    && engine.page_target_allows(
                        &target_character_id,
                        &tme_rules::CharacterId::new(sender_character_id.to_string()),
                    )
                    && observers
                        .get(&target.connection_id)
                        .is_some_and(|observer| observer.grant == target);
                if !allowed {
                    let _ = reply.send(false);
                    continue;
                }
                let envelope = wire::ServerEnvelope::SocialMessage {
                    message_id,
                    scope: wire::SocialScope::Page {
                        target_character_id: target.character_id,
                    },
                    sender_character_id,
                    sender_name,
                    body,
                };
                let delivered = observers
                    .get(&target.connection_id)
                    .is_some_and(|observer| observer.outbound.try_send(envelope).is_ok());
                if !delivered {
                    mark_pending_detach(
                        &mut observers,
                        &mut pending_detaches,
                        target.connection_id,
                        None,
                    );
                }
                let _ = reply.send(delivered);
            }
        }
        if prepared_transfer.is_none()
            && !drain_pending_detaches(
                facet_id,
                &mut engine,
                &mut server_sequence,
                &mut facet_revision,
                &mut observers,
                &mut pending_detaches,
                &store,
                &coordinator,
            )
            .await
        {
            return;
        }
    }
}

fn process_path_preview(
    engine: &Engine,
    facet_revision: u64,
    observers: &BTreeMap<wire::ConnectionId, Observer>,
    control_quiesced: bool,
    preview: FacetPathPreview,
) -> FacetPathPreviewReply {
    let observer = observers.get(&preview.grant.connection_id);
    let rejection = match observer {
        None => Some(wire::PathPreviewRejectionCode::WrongActor),
        Some(observer)
            if observer.grant.account_id != preview.grant.account_id
                || observer.grant.session_id != preview.grant.session_id
                || observer.grant.character_id != preview.grant.character_id =>
        {
            Some(wire::PathPreviewRejectionCode::StaleControlEpoch)
        }
        Some(observer) if preview.actor_id.as_str() != observer.grant.actor_id.as_str() => {
            Some(wire::PathPreviewRejectionCode::WrongActor)
        }
        Some(observer) if preview.control_epoch != observer.grant.control_epoch => {
            Some(wire::PathPreviewRejectionCode::StaleControlEpoch)
        }
        Some(_) if preview.observed_facet_revision > facet_revision => {
            Some(wire::PathPreviewRejectionCode::FutureWorldRevision)
        }
        Some(_) if control_quiesced => Some(wire::PathPreviewRejectionCode::StaleControlEpoch),
        Some(_) => None,
    };
    let response_actor_id = observer
        .map(|observer| &observer.grant.actor_id)
        .unwrap_or(&preview.grant.actor_id);
    let wire_actor_id = crate::protocol_v1::actor_id(response_actor_id)
        .expect("installed control grant actor ID remains wire-valid");
    let (disposition, wire_preview) = if let Some(code) = rejection {
        (wire::PathPreviewDisposition::Rejected { code }, None)
    } else {
        let path = preview
            .path
            .iter()
            .map(crate::protocol_v1::rules_direction)
            .collect::<Vec<_>>();
        match engine.preview_actor_path(&preview.grant.actor_id, &path) {
            Ok(value) => match crate::protocol_v1::path_preview(&value) {
                Ok(value) => (wire::PathPreviewDisposition::Previewed, Some(value)),
                Err(_) => (
                    wire::PathPreviewDisposition::Rejected {
                        code: wire::PathPreviewRejectionCode::RulesRejected,
                    },
                    None,
                ),
            },
            Err(_) => (
                wire::PathPreviewDisposition::Rejected {
                    code: wire::PathPreviewRejectionCode::RulesRejected,
                },
                None,
            ),
        }
    };
    let envelope = wire::ServerEnvelope::PathPreviewResult {
        preview_id: preview.preview_id,
        disposition,
        control_epoch: wire::DecimalU64::new(preview.grant.control_epoch),
        actor_id: wire_actor_id,
        world_revision: wire::DecimalU64::new(facet_revision),
        preview: wire_preview,
    };
    envelope
        .validate()
        .expect("facet path-preview result remains wire-valid");
    FacetPathPreviewReply { envelope }
}

fn mark_pending_detach(
    observers: &mut BTreeMap<wire::ConnectionId, Observer>,
    pending_detaches: &mut BTreeMap<wire::ConnectionId, PendingDetach>,
    connection_id: wire::ConnectionId,
    reason: Option<wire::DrainingReason>,
) {
    if let Some(pending) = pending_detaches.get_mut(&connection_id) {
        if reason.is_some() {
            pending.reason = reason;
        }
        return;
    }
    let Some(observer) = observers.remove(&connection_id) else {
        return;
    };
    let Observer {
        grant,
        outbound,
        terminal,
        expected_client_sequence: _,
    } = observer;
    pending_detaches.insert(
        connection_id,
        PendingDetach {
            grant,
            _outbound: outbound,
            terminal,
            reason,
            completions: Vec::new(),
            #[cfg(test)]
            certification_trace: None,
        },
    );
}

#[allow(clippy::too_many_arguments)]
async fn drain_pending_detaches(
    facet_id: wire::FacetId,
    engine: &mut Engine,
    server_sequence: &mut u64,
    facet_revision: &mut u64,
    observers: &mut BTreeMap<wire::ConnectionId, Observer>,
    pending_detaches: &mut BTreeMap<wire::ConnectionId, PendingDetach>,
    store: &Option<SharedStore>,
    coordinator: &Option<Arc<crate::coordinator::Coordinator>>,
) -> bool {
    while let Some(connection_id) = pending_detaches.keys().next().copied() {
        let Some(pending) = pending_detaches.get(&connection_id) else {
            return false;
        };
        let character_id = tme_rules::CharacterId::new(pending.grant.character_id.to_string());
        let control_epoch = pending.grant.control_epoch;
        #[cfg(test)]
        let actor_id = pending.grant.actor_id.clone();
        let mut candidate = engine.clone();
        let Ok(outcome) = candidate.apply_connection_presence(&character_id, control_epoch, false)
        else {
            return false;
        };
        #[cfg(test)]
        let certification_outcome = outcome.clone();
        if !commit_system_mutation(
            facet_id,
            engine,
            server_sequence,
            facet_revision,
            observers,
            pending_detaches,
            candidate,
            outcome,
            store,
            coordinator,
            "facet_presence",
        )
        .await
        {
            return false;
        }
        if let Some(pending) = pending_detaches.remove(&connection_id) {
            #[cfg(test)]
            if let Some(trace) = pending.certification_trace
                && trace
                    .send(certification_step(
                        engine,
                        &actor_id,
                        certification_outcome,
                        *server_sequence,
                        *facet_revision,
                    ))
                    .is_err()
            {
                return false;
            }
            if let Some(reason) = pending.reason {
                pending.terminal.send_replace(Some(reason));
            }
            for completion in pending.completions {
                let _ = completion.send(());
            }
        }
    }
    true
}

#[allow(clippy::too_many_arguments)]
async fn advance_facet_tick(
    facet_id: wire::FacetId,
    engine: &mut Engine,
    server_sequence: &mut u64,
    facet_revision: &mut u64,
    observers: &mut BTreeMap<wire::ConnectionId, Observer>,
    pending_detaches: &mut BTreeMap<wire::ConnectionId, PendingDetach>,
    store: &Option<SharedStore>,
    coordinator: &Option<Arc<crate::coordinator::Coordinator>>,
) -> Option<tme_rules::RulesOutcomeV1> {
    let mut candidate = engine.clone();
    let outcome = candidate.advance_realtime_boundary().ok()?;
    if !commit_system_mutation(
        facet_id,
        engine,
        server_sequence,
        facet_revision,
        observers,
        pending_detaches,
        candidate,
        outcome.clone(),
        store,
        coordinator,
        "facet_tick",
    )
    .await
    {
        return None;
    }
    Some(outcome)
}

#[allow(clippy::too_many_arguments)]
async fn commit_system_mutation(
    facet_id: wire::FacetId,
    engine: &mut Engine,
    server_sequence: &mut u64,
    facet_revision: &mut u64,
    observers: &mut BTreeMap<wire::ConnectionId, Observer>,
    pending_detaches: &mut BTreeMap<wire::ConnectionId, PendingDetach>,
    candidate: Engine,
    outcome: tme_rules::RulesOutcomeV1,
    store: &Option<SharedStore>,
    coordinator: &Option<Arc<crate::coordinator::Coordinator>>,
    action: &'static str,
) -> bool {
    if !outcome.state_changed {
        *engine = candidate;
        return true;
    }
    let Some(next_sequence) = server_sequence.checked_add(1) else {
        return false;
    };
    let Some(next_revision) = facet_revision.checked_add(1) else {
        return false;
    };
    let mut updates = Vec::new();
    for (connection_id, observer) in observers.iter() {
        let Ok(projection) =
            candidate.observer_projection(&observer.grant.actor_id, &outcome.events)
        else {
            return false;
        };
        let Ok(events) = crate::protocol_v1::events(&projection.events) else {
            return false;
        };
        let Ok(frame) = crate::protocol_v1::frame(&projection.frame) else {
            return false;
        };
        let Ok(static_scene_context) =
            crate::protocol_v1::static_scene_context(&projection.static_scene_context)
        else {
            return false;
        };
        let update = wire::ServerEnvelope::StateUpdate {
            server_sequence: wire::DecimalU64::new(next_sequence),
            world_revision: wire::DecimalU64::new(next_revision),
            events,
            events_truncated: projection.events_truncated,
            static_scene_context,
            frame,
        };
        if wire::encode_server_envelope(&update).is_err() {
            return false;
        }
        updates.push((*connection_id, update));
    }
    if let Some(store) = store {
        let Ok(checkpoint) = candidate.export_checkpoint() else {
            return false;
        };
        let commit = SystemCommit {
            facet_id,
            expected_server_sequence: *server_sequence,
            expected_revision: *facet_revision,
            next_server_sequence: next_sequence,
            next_revision,
            checkpoint: &checkpoint,
            action,
            durable_effects: &outcome.durable_effects,
        };
        let started = std::time::Instant::now();
        let committed = if let Some(coordinator) = coordinator {
            coordinator.commit_system(commit).await
        } else {
            store.commit_system(commit).await
        };
        crate::telemetry::record_system_commit(committed.is_ok(), started.elapsed());
        if let Err(error) = committed {
            tracing::error!(
                facet_id = %facet_id,
                action,
                error = %error,
                "durable system mutation failed"
            );
            return false;
        }
    }
    *engine = candidate;
    *server_sequence = next_sequence;
    *facet_revision = next_revision;
    let mut slow = Vec::new();
    for (connection_id, update) in updates {
        if observers
            .get(&connection_id)
            .is_some_and(|observer| observer.outbound.try_send(update).is_err())
        {
            slow.push(connection_id);
        }
    }
    for connection_id in slow {
        mark_pending_detach(observers, pending_detaches, connection_id, None);
    }
    true
}

struct CommandContext<'a> {
    facet_id: wire::FacetId,
    transfer_prepared: bool,
    control_quiesced: bool,
    store: &'a Option<SharedStore>,
    readiness: &'a Option<Arc<crate::postgres::GameplayReadiness>>,
}

async fn process_command(
    context: CommandContext<'_>,
    engine: &mut Engine,
    server_sequence: &mut u64,
    facet_revision: &mut u64,
    observers: &mut BTreeMap<wire::ConnectionId, Observer>,
    pending_detaches: &mut BTreeMap<wire::ConnectionId, PendingDetach>,
    command: FacetCommand,
) -> Option<FacetCommandReply> {
    let next_sequence = server_sequence.checked_add(1)?;
    let before_revision = *facet_revision;
    let observer_state = observers.get(&command.connection_id).map(|observer| {
        (
            observer.grant.clone(),
            observer.expected_client_sequence,
            observer.outbound.capacity() > 0,
        )
    });
    let rejection_code = match &observer_state {
        None => Some(wire::RejectionCode::WrongActor),
        Some((grant, _, _))
            if command.account_id != grant.account_id
                || command.session_id != grant.session_id
                || command.character_id != grant.character_id =>
        {
            Some(wire::RejectionCode::StaleControlEpoch)
        }
        Some((grant, _, _)) if command.actor_id.as_str() != grant.actor_id.as_str() => {
            Some(wire::RejectionCode::WrongActor)
        }
        Some((grant, _, _)) if command.control_epoch != grant.control_epoch => {
            Some(wire::RejectionCode::StaleControlEpoch)
        }
        Some((_, expected_sequence, _)) if command.client_sequence != *expected_sequence => {
            Some(wire::RejectionCode::OutOfOrderClientSequence)
        }
        Some(_) if command.observed_facet_revision > before_revision => {
            Some(wire::RejectionCode::FutureWorldRevision)
        }
        Some(_) if context.transfer_prepared || context.control_quiesced => {
            Some(wire::RejectionCode::StaleControlEpoch)
        }
        Some(_) => None,
    };
    if let Some(code) = rejection_code {
        let outcome = ReceiptOutcomeV3::rejected(code, Some(next_sequence), Some(before_revision));
        if !persist_command(
            context.store,
            context.readiness,
            engine,
            context.facet_id,
            &command,
            *server_sequence,
            before_revision,
            next_sequence,
            before_revision,
            &outcome,
            &[],
        )
        .await
        {
            return None;
        }
        *server_sequence = next_sequence;
        let delivered = observers
            .get(&command.connection_id)
            .is_some_and(|observer| {
                send_issuer_update(engine, next_sequence, before_revision, observer)
            });
        if !delivered {
            mark_pending_detach(observers, pending_detaches, command.connection_id, None);
        }
        return outcome
            .to_envelope(command.command_id, wire::ReplayStatus::New)
            .ok()
            .map(|envelope| FacetCommandReply { envelope });
    }

    let (grant, _, issuer_has_capacity) = observer_state.expect("validated observer exists");
    let mut candidate = engine.clone();
    let rules_outcome = match match crate::protocol_v1::intent(&command.intent) {
        crate::protocol_v1::RulesIntent::Gameplay(intent) => {
            candidate.apply_realtime_actor_intent(&grant.actor_id, intent)
        }
        crate::protocol_v1::RulesIntent::Social(intent) => {
            candidate.apply_social_intent(&grant.actor_id, intent)
        }
    } {
        Ok(outcome) => outcome,
        Err(_) => {
            let outcome = ReceiptOutcomeV3::rejected(
                wire::RejectionCode::RulesRejected,
                Some(next_sequence),
                Some(before_revision),
            );
            if !persist_command(
                context.store,
                context.readiness,
                engine,
                context.facet_id,
                &command,
                *server_sequence,
                before_revision,
                next_sequence,
                before_revision,
                &outcome,
                &[],
            )
            .await
            {
                return None;
            }
            *server_sequence = next_sequence;
            if let Some(observer) = observers.get_mut(&command.connection_id) {
                observer.expected_client_sequence =
                    observer.expected_client_sequence.saturating_add(1);
            }
            let delivered = observers
                .get(&command.connection_id)
                .is_some_and(|observer| {
                    send_issuer_update(engine, next_sequence, before_revision, observer)
                });
            if !delivered {
                mark_pending_detach(observers, pending_detaches, command.connection_id, None);
            }
            return outcome
                .to_envelope(command.command_id, wire::ReplayStatus::New)
                .ok()
                .map(|envelope| FacetCommandReply { envelope });
        }
    };
    let next_revision = if rules_outcome.state_changed {
        before_revision.checked_add(1)?
    } else {
        before_revision
    };
    let issuer_projection = candidate
        .observer_projection(&grant.actor_id, &rules_outcome.events)
        .ok();
    let mut updates = Vec::new();
    let mut projection_failed = issuer_projection.is_none() || !issuer_has_capacity;
    let mut projection_failure_connection = projection_failed.then_some(command.connection_id);
    if !projection_failed {
        for (connection_id, attached) in observers.iter() {
            let converted = candidate
                .observer_projection(&attached.grant.actor_id, &rules_outcome.events)
                .map_err(|_| ())
                .and_then(|projection| {
                    Ok(wire::ServerEnvelope::StateUpdate {
                        server_sequence: wire::DecimalU64::new(next_sequence),
                        world_revision: wire::DecimalU64::new(next_revision),
                        events: crate::protocol_v1::events(&projection.events).map_err(|_| ())?,
                        events_truncated: projection.events_truncated,
                        static_scene_context: crate::protocol_v1::static_scene_context(
                            &projection.static_scene_context,
                        )
                        .map_err(|_| ())?,
                        frame: crate::protocol_v1::frame(&projection.frame).map_err(|_| ())?,
                    })
                });
            match converted {
                Ok(update) if wire::encode_server_envelope(&update).is_ok() => {
                    updates.push((*connection_id, update));
                }
                _ => {
                    projection_failed = true;
                    projection_failure_connection = Some(*connection_id);
                    break;
                }
            }
        }
    }
    if projection_failed {
        let outcome = ReceiptOutcomeV3::rejected(
            wire::RejectionCode::ProjectionFailed,
            Some(next_sequence),
            Some(before_revision),
        );
        if !persist_command(
            context.store,
            context.readiness,
            engine,
            context.facet_id,
            &command,
            *server_sequence,
            before_revision,
            next_sequence,
            before_revision,
            &outcome,
            &[],
        )
        .await
        {
            return None;
        }
        *server_sequence = next_sequence;
        if let Some(connection_id) = projection_failure_connection {
            mark_pending_detach(observers, pending_detaches, connection_id, None);
        }
        return outcome
            .to_envelope(command.command_id, wire::ReplayStatus::New)
            .ok()
            .map(|envelope| FacetCommandReply { envelope });
    }

    let issuer_projection = issuer_projection.expect("projection was checked");
    let outcome = ReceiptOutcomeV3::accepted(
        next_sequence,
        before_revision,
        next_revision,
        issuer_projection.events,
        issuer_projection.events_truncated,
    );
    let envelope = outcome
        .to_envelope(command.command_id, wire::ReplayStatus::New)
        .ok()?;
    if wire::encode_server_envelope(&envelope).is_err()
        || !persist_command(
            context.store,
            context.readiness,
            &candidate,
            context.facet_id,
            &command,
            *server_sequence,
            before_revision,
            next_sequence,
            next_revision,
            &outcome,
            &rules_outcome.durable_effects,
        )
        .await
    {
        return None;
    }
    #[cfg(test)]
    if command.ev_fail_after_store_commit {
        if let Some(readiness) = context.readiness {
            readiness.fail();
        }
        return None;
    }
    *engine = candidate;
    *server_sequence = next_sequence;
    *facet_revision = next_revision;
    if let Some(observer) = observers.get_mut(&command.connection_id) {
        observer.expected_client_sequence = observer.expected_client_sequence.saturating_add(1);
    }
    let mut slow = Vec::new();
    for (connection_id, update) in updates {
        if observers
            .get(&connection_id)
            .is_some_and(|observer| observer.outbound.try_send(update).is_err())
        {
            slow.push(connection_id);
        }
    }
    for connection_id in slow {
        mark_pending_detach(observers, pending_detaches, connection_id, None);
    }
    #[cfg(test)]
    if let Some(trace) = command.certification_trace
        && trace
            .send(certification_step(
                engine,
                &grant.actor_id,
                rules_outcome,
                *server_sequence,
                *facet_revision,
            ))
            .is_err()
    {
        return None;
    }
    Some(FacetCommandReply { envelope })
}

#[allow(clippy::too_many_arguments)]
async fn persist_command(
    store: &Option<SharedStore>,
    readiness: &Option<Arc<crate::postgres::GameplayReadiness>>,
    candidate: &Engine,
    owner_facet_id: wire::FacetId,
    command: &FacetCommand,
    expected_sequence: u64,
    expected_revision: u64,
    next_sequence: u64,
    next_revision: u64,
    outcome: &ReceiptOutcomeV3,
    durable_effects: &[tme_rules::DurableGameplayEffectV1],
) -> bool {
    let Some(store) = store else {
        return true;
    };
    #[cfg(test)]
    if command.ev_fail_checkpoint_export {
        return false;
    }
    let Ok(checkpoint) = candidate.export_checkpoint() else {
        return false;
    };
    let started = std::time::Instant::now();
    let result = store
        .commit_command(CommandCommit {
            account_id: command.account_id,
            session_id: command.session_id,
            character_id: command.character_id,
            command_id: command.command_id,
            request_digest: command.request_digest,
            facet_id: owner_facet_id,
            actor_id: command.actor_id.as_str(),
            control_epoch: command.control_epoch,
            client_sequence: command.client_sequence,
            expected_server_sequence: expected_sequence,
            expected_revision,
            next_server_sequence: next_sequence,
            next_revision,
            checkpoint: &checkpoint,
            outcome,
            durable_effects,
        })
        .await;
    crate::telemetry::record_command_commit(result.is_ok(), started.elapsed());
    match result {
        Ok(()) => true,
        Err(CommandCommitError::Definite) => false,
        Err(CommandCommitError::OutcomeUnknown) => {
            if let Some(readiness) = readiness {
                readiness.fail();
            }
            false
        }
    }
}

fn current_state(
    engine: &Engine,
    server_sequence: u64,
    facet_revision: u64,
    observer: Option<&Observer>,
) -> Result<wire::ServerEnvelope, FacetError> {
    let observer = observer.ok_or(FacetError::InvalidActor)?;
    let projection = engine
        .observer_projection(&observer.grant.actor_id, &[])
        .map_err(|_| FacetError::Projection)?;
    let update = wire::ServerEnvelope::StateUpdate {
        server_sequence: wire::DecimalU64::new(server_sequence),
        world_revision: wire::DecimalU64::new(facet_revision),
        events: Vec::new(),
        events_truncated: false,
        static_scene_context: crate::protocol_v1::static_scene_context(
            &projection.static_scene_context,
        )
        .map_err(|_| FacetError::Projection)?,
        frame: crate::protocol_v1::frame(&projection.frame).map_err(|_| FacetError::Projection)?,
    };
    wire::encode_server_envelope(&update).map_err(|_| FacetError::Projection)?;
    Ok(update)
}

fn send_issuer_update(
    engine: &Engine,
    server_sequence: u64,
    facet_revision: u64,
    observer: &Observer,
) -> bool {
    if let Ok(update) = current_state(engine, server_sequence, facet_revision, Some(observer)) {
        observer.outbound.try_send(update).is_ok()
    } else {
        false
    }
}

#[cfg(test)]
fn certification_step(
    engine: &Engine,
    actor_id: &tme_rules::ActorId,
    outcome: tme_rules::RulesOutcomeV1,
    server_sequence: u64,
    facet_revision: u64,
) -> CertificationStep {
    CertificationStep {
        projection: engine
            .observer_projection(actor_id, &outcome.events)
            .expect("certification actor projection succeeds"),
        checkpoint: engine
            .export_checkpoint()
            .expect("certification state exports Checkpoint 3"),
        outcome,
        server_sequence,
        facet_revision,
    }
}

#[cfg(test)]
#[path = "facet/certification_tests.rs"]
mod certification_tests;
