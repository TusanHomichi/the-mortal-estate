use super::*;

pub(super) struct CommandContext<'a> {
    pub(super) facet_id: wire::FacetId,
    pub(super) transfer_prepared: bool,
    pub(super) control_quiesced: bool,
    pub(super) store: &'a Option<SharedStore>,
    pub(super) readiness: &'a Option<Arc<crate::postgres::GameplayReadiness>>,
}

pub(super) async fn process_command(
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
pub(super) async fn persist_command(
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
