use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) async fn advance_facet_to(
    facet_id: wire::FacetId,
    engine: &mut Engine,
    server_sequence: &mut u64,
    facet_revision: &mut u64,
    observers: &mut BTreeMap<wire::ConnectionId, Observer>,
    pending_detaches: &mut BTreeMap<wire::ConnectionId, PendingDetach>,
    store: &Option<SharedStore>,
    coordinator: &Option<Arc<crate::coordinator::Coordinator>>,
    target: tme_rules::LogicalTime,
) -> Option<tme_rules::RulesOutcomeV1> {
    let mut candidate = engine.clone();
    let outcome = candidate.advance_to(target).ok()?;
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
        "facet_deadlines",
    )
    .await
    {
        return None;
    }
    Some(outcome)
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(super) async fn advance_facet_tick(
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
    let outcome = candidate.advance_action_interval().ok()?;
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
        "facet_deadlines",
    )
    .await
    {
        return None;
    }
    Some(outcome)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn commit_system_mutation(
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
