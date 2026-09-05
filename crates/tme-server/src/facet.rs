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
    pub before_sequence: u64,
    pub after_sequence: u64,
    pub checkpoint: tme_rules::FacetCheckpointV5,
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
    CheckDeadlines,
    #[cfg(test)]
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
    pub(crate) checkpoint: tme_rules::FacetCheckpointV5,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CertificationStep {
    pub(crate) outcome: tme_rules::RulesOutcomeV1,
    pub(crate) projection: tme_rules::ObserverProjectionV1,
    pub(crate) server_sequence: u64,
    pub(crate) facet_revision: u64,
    pub(crate) checkpoint: tme_rules::FacetCheckpointV5,
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

mod runtime;
use runtime::*;
mod mutations;
use mutations::*;
mod commands;
use commands::*;
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

#[cfg(test)]
mod publication_tests;
