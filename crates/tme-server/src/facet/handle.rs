use super::*;

impl FacetHandle {
    pub(crate) async fn creation_profiles(
        &self,
    ) -> Result<Vec<tme_rules::CharacterCreationProfileDef>, FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .send(FacetRequest::CreationProfiles { reply })
            .await
            .map_err(|_| FacetError::Unavailable)?;
        receive.await.map_err(|_| FacetError::Unavailable)
    }

    pub(crate) async fn prepare_character_creation(
        &self,
        transfer_epoch: u64,
        character_id: tme_rules::CharacterId,
        draft: wire::CharacterCreationDraftV1,
    ) -> Result<tme_rules::ActorId, FacetError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .send(FacetRequest::PrepareCharacterCreation {
                transfer_epoch,
                character_id,
                draft,
                reply,
            })
            .await
            .map_err(|_| FacetError::Unavailable)?;
        receive.await.map_err(|_| FacetError::Unavailable)?
    }
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
    pub(super) fn spawn_certification(
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
