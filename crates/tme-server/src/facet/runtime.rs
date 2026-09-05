use super::*;

pub(super) async fn run_facet(
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
    let clock = crate::scheduler::FacetClock::new(engine.world().timing.now);
    let live_clock = store.is_some();
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
        if live_clock && prepared_transfer.is_none() {
            let now = clock.now();
            let due = engine.next_deadline().is_some_and(|at| at <= now);
            let action = matches!(
                &request,
                FacetRequest::Command { .. } | FacetRequest::InstallGrant { .. }
            );
            if (due || action)
                && now > engine.world().timing.now
                && advance_facet_to(
                    facet_id,
                    &mut engine,
                    &mut server_sequence,
                    &mut facet_revision,
                    &mut observers,
                    &mut pending_detaches,
                    &store,
                    &coordinator,
                    now,
                )
                .await
                .is_none()
            {
                return;
            }
        }
        match request {
            FacetRequest::CheckDeadlines => {}
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
                        match next_publication(facet_revision, server_sequence) {
                            Some((revision, sequence)) => {
                                engine = prepared
                                    .candidate
                                    .take()
                                    .expect("prepared transfer owns candidate");
                                facet_revision = revision;
                                server_sequence = sequence;
                                prepared.committed = true;
                                Ok(())
                            }
                            None => Err(FacetError::Transfer),
                        }
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
                        match next_publication(facet_revision, server_sequence) {
                            Some((after_revision, after_sequence)) => prepared
                                .candidate
                                .as_ref()
                                .expect("uncommitted transfer owns candidate")
                                .export_checkpoint()
                                .map(|checkpoint| PreparedFacetCheckpoint {
                                    facet_id,
                                    before_revision: facet_revision,
                                    after_revision,
                                    before_sequence: server_sequence,
                                    after_sequence,
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
            #[cfg(test)]
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

fn next_publication(revision: u64, sequence: u64) -> Option<(u64, u64)> {
    revision.checked_add(1).zip(sequence.checked_add(1))
}
