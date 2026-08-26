use std::path::PathBuf;
use std::time::Duration;

use sha2::{Digest, Sha256};

use super::*;

#[derive(Clone)]
struct DeterminismFixture {
    engine: Engine,
    actors: [tme_rules::ActorId; 2],
    wire_characters: [wire::CharacterId; 2],
    rules_characters: [tme_rules::CharacterId; 2],
}

#[derive(Clone, Copy)]
enum CertificationCommitKind {
    Command,
    System,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct CertificationCursor {
    server_sequence: u64,
    facet_revision: u64,
}

impl CertificationCursor {
    fn record(&mut self, kind: CertificationCommitKind, state_changed: bool) {
        if matches!(kind, CertificationCommitKind::Command) || state_changed {
            self.server_sequence = self.server_sequence.checked_add(1).unwrap();
        }
        if state_changed {
            self.facet_revision = self.facet_revision.checked_add(1).unwrap();
        }
    }
}

fn certification_engine(character_id: wire::CharacterId) -> (Engine, tme_rules::CharacterId) {
    let mut scenario = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    scenario.extend([
        "..",
        "..",
        "content",
        "test-corpus",
        "world_topology_gallery.json",
    ]);
    let engine = tme_sim::load_engine_from_scenario(&scenario, Some(7))
        .expect("certification scenario loads");
    let rules_character_id = tme_rules::CharacterId::new(character_id.to_string());
    let engine = engine
        .prepare_character_id_rekey(
            &tme_rules::ActorId::new("player"),
            rules_character_id.clone(),
        )
        .expect("certification character rekeys");
    (engine, rules_character_id)
}

fn deterministic_fixture() -> DeterminismFixture {
    let mut scenario = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    scenario.extend(["..", "..", "content", "test-corpus", "character_sheet.json"]);
    let loaded = tme_sim::load_engine_from_scenario(&scenario, Some(7))
        .expect("seed-7 character-sheet scenario loads");
    let wire_characters = [
        wire::CharacterId::new(uuid::Uuid::from_u128(0x101)).unwrap(),
        wire::CharacterId::new(uuid::Uuid::from_u128(0x102)).unwrap(),
    ];
    let rules_characters = [
        tme_rules::CharacterId::new(wire_characters[0].to_string()),
        tme_rules::CharacterId::new(wire_characters[1].to_string()),
    ];
    let actors = [
        tme_rules::ActorId::new("player"),
        tme_rules::ActorId::new("comparison_actor"),
    ];
    let mut engine = loaded
        .prepare_character_id_rekey(&actors[0], rules_characters[0].clone())
        .expect("primary character rekeys before fixture cloning");
    let mut comparison = engine
        .world()
        .actor(&actors[0])
        .expect("clean player exists")
        .clone();
    comparison.id = actors[1].clone();
    comparison.name = "Comparison Actor".to_string();
    comparison.character_id = Some(rules_characters[1].clone());
    comparison.location.position = (1, 1).into();
    comparison.home_location = comparison.location.clone();
    comparison.carried.items.clear();
    comparison.carried.gold = Default::default();
    comparison.timing.tie_break_order = engine.world().timing.next_tie_break_order;

    let primary_preferences = engine
        .world()
        .communication_preferences
        .get(&rules_characters[0])
        .cloned()
        .expect("primary communication preferences exist");
    let primary_presence = engine
        .world()
        .character_presence
        .get(&rules_characters[0])
        .copied()
        .expect("primary presence exists");
    let primary_quests = engine
        .world()
        .quest_states
        .get(&rules_characters[0])
        .cloned();
    let world = engine.world_mut();
    world.actors.retain(|actor| actor.id == actors[0]);
    world.actors.push(comparison);
    world.timing.next_tie_break_order = world.timing.next_tie_break_order.checked_add(1).unwrap();
    world
        .communication_preferences
        .retain(|character_id, _| character_id == &rules_characters[0]);
    world
        .communication_preferences
        .insert(rules_characters[1].clone(), primary_preferences);
    world
        .character_presence
        .retain(|character_id, _| character_id == &rules_characters[0]);
    world
        .character_presence
        .insert(rules_characters[1].clone(), primary_presence);
    world
        .quest_states
        .retain(|character_id, _| character_id == &rules_characters[0]);
    if let Some(quests) = primary_quests {
        world
            .quest_states
            .insert(rules_characters[1].clone(), quests);
    }
    world.ecology_sites.clear();
    world.social_relations = Default::default();
    world.groups.clear();
    world.group_invitations.clear();
    world.player_follow_targets.clear();
    world.defeat_contributions.clear();
    world.item_offers.clear();
    world.linked_player_kill_karma.clear();

    let exported = engine
        .export_checkpoint()
        .expect("two-actor fixture exports Checkpoint 3");
    let canonical = tme_rules::FacetCheckpointV4::from_bytes(exported.as_bytes().to_vec())
        .expect("fixture Checkpoint 3 bytes are canonical");
    let engine = Engine::hydrate_checkpoint(engine.definition().clone(), &canonical)
        .expect("fixture hydrates through Checkpoint 3 once");
    assert_eq!(
        engine.export_checkpoint().unwrap().as_bytes(),
        canonical.as_bytes()
    );
    assert_eq!(engine.world().actors.len(), 2);
    assert!(
        engine
            .world()
            .actors
            .iter()
            .all(|actor| actor.kind == tme_rules::ActorKind::Player)
    );
    DeterminismFixture {
        engine,
        actors,
        wire_characters,
        rules_characters,
    }
}

fn deterministic_grant(
    index: usize,
    facet_id: wire::FacetId,
    fixture: &DeterminismFixture,
) -> ControlGrant {
    ControlGrant::new(
        wire::AccountId::new(uuid::Uuid::from_u128(0x201 + index as u128)).unwrap(),
        wire::SessionId::new(uuid::Uuid::from_u128(0x211 + index as u128)).unwrap(),
        wire::ConnectionId::new(uuid::Uuid::from_u128(0x221 + index as u128)).unwrap(),
        fixture.wire_characters[index],
        facet_id,
        fixture.actors[index].clone(),
        1,
    )
}

fn direct_step(
    engine: &Engine,
    actor_id: &tme_rules::ActorId,
    outcome: tme_rules::RulesOutcomeV1,
    cursor: &CertificationCursor,
) -> CertificationStep {
    certification_step(
        engine,
        actor_id,
        outcome,
        cursor.server_sequence,
        cursor.facet_revision,
    )
}

fn checkpoint_field(checkpoint: &tme_rules::FacetCheckpointV4, pointer: &str) -> serde_json::Value {
    serde_json::from_slice::<serde_json::Value>(checkpoint.as_bytes())
        .unwrap()
        .pointer(pointer)
        .unwrap_or_else(|| panic!("Checkpoint 3 lacks {pointer}"))
        .clone()
}

fn compare_common_boundary(
    label: &str,
    actor_id: &tme_rules::ActorId,
    direct: &CertificationStep,
    facet_steps: &[CertificationStep],
    facet_cursor: &mut CertificationCursor,
    commit_kinds: &[CertificationCommitKind],
    digest: &mut Sha256,
) {
    assert_eq!(
        facet_steps.len(),
        commit_kinds.len(),
        "{label} {actor_id}: commit accounting"
    );
    for (index, (step, kind)) in facet_steps.iter().zip(commit_kinds).enumerate() {
        facet_cursor.record(*kind, step.outcome.state_changed);
        assert_eq!(
            step.server_sequence, facet_cursor.server_sequence,
            "{label} {actor_id}: server sequence after substep {index}"
        );
        assert_eq!(
            step.facet_revision, facet_cursor.facet_revision,
            "{label} {actor_id}: facet revision after substep {index}"
        );
    }
    let facet_events = facet_steps
        .iter()
        .flat_map(|step| step.outcome.events.iter().cloned())
        .collect::<Vec<_>>();
    assert_eq!(
        direct.outcome.events, facet_events,
        "{label} {actor_id}: ordered raw rules events"
    );
    let facet_durable_effects = facet_steps
        .iter()
        .flat_map(|step| step.outcome.durable_effects.iter().cloned())
        .collect::<Vec<_>>();
    assert_eq!(
        direct.outcome.durable_effects, facet_durable_effects,
        "{label} {actor_id}: ordered durable rules effects"
    );
    assert_eq!(
        direct.outcome.state_changed,
        facet_steps.iter().any(|step| step.outcome.state_changed),
        "{label} {actor_id}: state-changed result"
    );
    let facet_observed = facet_steps
        .iter()
        .flat_map(|step| step.projection.events.iter().cloned())
        .collect::<Vec<_>>();
    assert_eq!(
        direct.projection.events, facet_observed,
        "{label} {actor_id}: ordered Observer Projection 6 events"
    );
    assert_eq!(
        direct.projection.frame,
        facet_steps.last().unwrap().projection.frame,
        "{label} {actor_id}: Observer Projection 6 frame"
    );
    assert_eq!(
        direct.projection.events_truncated,
        facet_steps
            .iter()
            .any(|step| step.projection.events_truncated),
        "{label} {actor_id}: Observer Projection 6 truncation"
    );
    let facet_final = facet_steps.last().unwrap();
    assert_eq!(
        direct.server_sequence, facet_final.server_sequence,
        "{label} {actor_id}: common server sequence"
    );
    assert_eq!(
        direct.facet_revision, facet_final.facet_revision,
        "{label} {actor_id}: common facet revision"
    );
    assert_eq!(
        checkpoint_field(&direct.checkpoint, "/world/timing/now"),
        checkpoint_field(&facet_final.checkpoint, "/world/timing/now"),
        "{label} {actor_id}: logical time"
    );
    assert_eq!(
        checkpoint_field(&direct.checkpoint, "/rng_state"),
        checkpoint_field(&facet_final.checkpoint, "/rng_state"),
        "{label} {actor_id}: RNG state"
    );
    assert_eq!(
        direct.checkpoint.as_bytes(),
        facet_final.checkpoint.as_bytes(),
        "{label} {actor_id}: exact Checkpoint 3 bytes"
    );
    assert_eq!(
        direct.checkpoint.sha256(),
        facet_final.checkpoint.sha256(),
        "{label} {actor_id}: exact Checkpoint 3 SHA-256"
    );

    digest.update((label.len() as u64).to_be_bytes());
    digest.update(label.as_bytes());
    digest.update((actor_id.as_str().len() as u64).to_be_bytes());
    digest.update(actor_id.as_str().as_bytes());
    digest.update(serde_json::to_vec(&direct.outcome.events).unwrap());
    digest.update(serde_json::to_vec(&direct.outcome.durable_effects).unwrap());
    digest.update(serde_json::to_vec(&direct.projection).unwrap());
    digest.update(direct.server_sequence.to_be_bytes());
    digest.update(direct.facet_revision.to_be_bytes());
    digest.update(direct.checkpoint.as_bytes());
    digest.update(direct.checkpoint.sha256());
    for (step, kind) in facet_steps.iter().zip(commit_kinds) {
        digest.update(match kind {
            CertificationCommitKind::Command => [0],
            CertificationCommitKind::System => [1],
        });
        digest.update([u8::from(step.outcome.state_changed)]);
        digest.update(serde_json::to_vec(&step.outcome.events).unwrap());
        digest.update(serde_json::to_vec(&step.outcome.durable_effects).unwrap());
        digest.update(serde_json::to_vec(&step.projection).unwrap());
        digest.update(step.server_sequence.to_be_bytes());
        digest.update(step.facet_revision.to_be_bytes());
    }
}

fn grant(
    facet_id: wire::FacetId,
    character_id: wire::CharacterId,
    connection_id: wire::ConnectionId,
) -> ControlGrant {
    ControlGrant::new(
        wire::AccountId::new(uuid::Uuid::now_v7()).unwrap(),
        wire::SessionId::new(uuid::Uuid::now_v7()).unwrap(),
        connection_id,
        character_id,
        facet_id,
        tme_rules::ActorId::new("player"),
        1,
    )
}

async fn install_with_trace(
    handle: &FacetHandle,
    grant: ControlGrant,
) -> (
    CertificationStep,
    mpsc::Receiver<wire::ServerEnvelope>,
    watch::Receiver<Option<wire::DrainingReason>>,
) {
    let (outbound, outbound_receive) = mpsc::channel(crate::config::OUTBOUND_QUEUE_CAPACITY);
    let (terminal, terminal_receive) = watch::channel(None);
    let (reply, receive) = oneshot::channel();
    let (trace, trace_receive) = oneshot::channel();
    handle
        .sender
        .send(FacetRequest::InstallGrant {
            grant,
            outbound,
            terminal,
            certification_trace: Some(trace),
            reply,
        })
        .await
        .unwrap();
    let welcome = receive.await.unwrap().unwrap();
    let trace = trace_receive.await.unwrap();
    assert_eq!(welcome.server_sequence, trace.server_sequence);
    assert_eq!(welcome.facet_revision, trace.facet_revision);
    (trace, outbound_receive, terminal_receive)
}

async fn command_with_trace(
    handle: &FacetHandle,
    grant: &ControlGrant,
    command_id: wire::CommandId,
    client_sequence: u64,
    observed_facet_revision: u64,
    intent: wire::Intent,
) -> CertificationStep {
    let typed = wire::ClientCommandEnvelope::Command {
        command_id,
        control_epoch: wire::DecimalU64::new(grant.control_epoch),
        client_sequence: wire::DecimalU64::new(client_sequence),
        observed_world_revision: wire::DecimalU64::new(observed_facet_revision),
        actor_id: wire::ActorId::new(grant.actor_id.as_str()).unwrap(),
        intent: intent.clone(),
    };
    let request_digest = Sha256::digest(serde_json::to_vec(&typed).unwrap()).into();
    let (trace, trace_receive) = oneshot::channel();
    let receive = handle
        .try_command(FacetCommand {
            connection_id: grant.connection_id,
            account_id: grant.account_id,
            session_id: grant.session_id,
            character_id: grant.character_id,
            command_id,
            control_epoch: grant.control_epoch,
            client_sequence,
            observed_facet_revision,
            actor_id: wire::ActorId::new(grant.actor_id.as_str()).unwrap(),
            intent,
            request_digest,
            certification_trace: Some(trace),
            ev_fail_checkpoint_export: false,
            ev_fail_after_store_commit: false,
        })
        .unwrap();
    let reply = receive.await.unwrap();
    assert!(matches!(
        reply.envelope,
        wire::ServerEnvelope::CommandResult {
            disposition: wire::CommandDisposition::Accepted,
            replay_status: wire::ReplayStatus::New,
            ..
        }
    ));
    trace_receive.await.unwrap()
}

async fn tick_with_trace(handle: &FacetHandle, actor_id: tme_rules::ActorId) -> CertificationStep {
    let (reply, receive) = oneshot::channel();
    handle
        .sender
        .send(FacetRequest::CertificationTick { actor_id, reply })
        .await
        .unwrap();
    receive.await.unwrap()
}

async fn detach_with_trace(
    handle: &FacetHandle,
    connection_id: wire::ConnectionId,
) -> CertificationStep {
    let (reply, receive) = oneshot::channel();
    handle
        .sender
        .send(FacetRequest::CertificationDetach {
            connection_id,
            reply,
        })
        .await
        .unwrap();
    receive.await.unwrap()
}

fn drain_outbound(receivers: &mut [mpsc::Receiver<wire::ServerEnvelope>]) {
    for receiver in receivers {
        while receiver.try_recv().is_ok() {}
    }
}

fn apply_direct_intent(
    engine: &mut Engine,
    actor_id: &tme_rules::ActorId,
    intent: &wire::Intent,
) -> tme_rules::RulesOutcomeV1 {
    match crate::protocol_v1::intent(intent) {
        crate::protocol_v1::RulesIntent::Gameplay(intent) => engine
            .apply_actor_intent(actor_id, intent)
            .expect("direct ordinary simulation intent succeeds"),
        crate::protocol_v1::RulesIntent::Social(intent) => engine
            .apply_social_intent(actor_id, intent)
            .expect("direct social intent succeeds"),
    }
}

fn apply_direct_realtime_intent(
    engine: &mut Engine,
    actor_id: &tme_rules::ActorId,
    intent: &wire::Intent,
) -> tme_rules::RulesOutcomeV1 {
    match crate::protocol_v1::intent(intent) {
        crate::protocol_v1::RulesIntent::Gameplay(intent) => engine
            .apply_realtime_actor_intent(actor_id, intent)
            .expect("direct realtime simulation intent succeeds"),
        crate::protocol_v1::RulesIntent::Social(intent) => engine
            .apply_social_intent(actor_id, intent)
            .expect("direct social intent succeeds"),
    }
}

fn compare_typed_substeps(
    label: &str,
    actor_id: &tme_rules::ActorId,
    direct: &[CertificationStep],
    facet: &[CertificationStep],
) {
    assert_eq!(
        direct.len(),
        facet.len(),
        "{label} {actor_id}: typed command/tick count"
    );
    for (index, (direct, facet)) in direct.iter().zip(facet).enumerate() {
        assert_eq!(
            direct, facet,
            "{label} {actor_id}: typed command/tick substep {index}"
        );
    }
}

async fn assert_common_ready(
    label: &str,
    handle: &FacetHandle,
    direct: &Engine,
    character_id: tme_rules::CharacterId,
    actor_id: &tme_rules::ActorId,
    cursor: &CertificationCursor,
) {
    let facet = inspect(handle, character_id).await;
    let direct_projection = direct.observer_projection(actor_id, &[]).unwrap();
    assert!(
        direct_projection.frame.can_act,
        "{label} {actor_id}: direct actor is not ready"
    );
    assert!(
        facet.projection.frame.can_act,
        "{label} {actor_id}: facet actor is not ready"
    );
    assert_eq!(
        direct_projection, facet.projection,
        "{label} {actor_id}: ready-boundary projection"
    );
    assert_eq!(facet.server_sequence, cursor.server_sequence);
    assert_eq!(facet.facet_revision, cursor.facet_revision);
    assert_eq!(
        direct.export_checkpoint().unwrap().as_bytes(),
        facet.checkpoint.as_bytes(),
        "{label} {actor_id}: ready-boundary checkpoint"
    );
}

async fn run_determinism_oracle(fixture: DeterminismFixture) -> [u8; 32] {
    assert_eq!(wire::PROTOCOL_MAJOR, 1);
    assert_eq!(wire::PROTOCOL_MINOR, 8);
    let initial = fixture.engine.export_checkpoint().unwrap();
    let mut direct = fixture.engine.clone();
    let facet_engine = fixture.engine.clone();
    assert_eq!(
        direct.export_checkpoint().unwrap().as_bytes(),
        initial.as_bytes()
    );
    assert_eq!(
        facet_engine.export_checkpoint().unwrap().as_bytes(),
        initial.as_bytes()
    );

    let facet_id = wire::FacetId::new(uuid::Uuid::from_u128(0x301)).unwrap();
    let (handle, startup_receive) =
        FacetHandle::spawn_certification(facet_id, facet_engine, fixture.actors[0].clone());
    let startup = startup_receive.await.unwrap();
    let startup_outcome = direct.mark_all_characters_disconnected().unwrap();
    let mut direct_cursor = CertificationCursor::default();
    direct_cursor.record(
        CertificationCommitKind::System,
        startup_outcome.state_changed,
    );
    let direct_startup = direct_step(&direct, &fixture.actors[0], startup_outcome, &direct_cursor);
    let mut facet_cursor = CertificationCursor::default();
    let mut semantic = Sha256::new();
    compare_common_boundary(
        "startup_disconnect",
        &fixture.actors[0],
        &direct_startup,
        &[startup],
        &mut facet_cursor,
        &[CertificationCommitKind::System],
        &mut semantic,
    );

    let grants = [
        deterministic_grant(0, facet_id, &fixture),
        deterministic_grant(1, facet_id, &fixture),
    ];
    let mut outbound = Vec::new();
    let mut terminals = Vec::new();
    for (index, grant) in grants.iter().enumerate() {
        let outcome = direct
            .apply_connection_presence(&fixture.rules_characters[index], 1, true)
            .unwrap();
        direct_cursor.record(CertificationCommitKind::System, outcome.state_changed);
        let direct_presence = direct_step(&direct, &fixture.actors[index], outcome, &direct_cursor);
        let (facet_presence, receive, terminal) = install_with_trace(&handle, grant.clone()).await;
        compare_common_boundary(
            if index == 0 {
                "presence_primary_on"
            } else {
                "presence_comparison_on"
            },
            &fixture.actors[index],
            &direct_presence,
            &[facet_presence],
            &mut facet_cursor,
            &[CertificationCommitKind::System],
            &mut semantic,
        );
        outbound.push(receive);
        terminals.push(terminal);
        drain_outbound(&mut outbound);
    }

    let schedule = [
        (
            "pages_off",
            wire::Intent::SetPagesEnabled { enabled: false },
            0_usize,
        ),
        ("wait", wire::Intent::Wait, 1),
        (
            "move_east",
            wire::Intent::MovePath {
                path: vec![wire::Direction::East],
            },
            1,
        ),
        (
            "move_west",
            wire::Intent::MovePath {
                path: vec![wire::Direction::West],
            },
            1,
        ),
        (
            "fight_comparison",
            wire::Intent::PhysicalAttack {
                mode: wire::PhysicalAttackMode::Fight,
                target_actor_id: wire::ActorId::new(fixture.actors[1].as_str()).unwrap(),
                authorization: wire::HostilityAuthorization::ConfirmedUnsafe,
            },
            1,
        ),
    ];
    for (index, (label, intent, ticks)) in schedule.into_iter().enumerate() {
        if !matches!(intent, wire::Intent::SetPagesEnabled { .. }) {
            assert_common_ready(
                label,
                &handle,
                &direct,
                fixture.rules_characters[0].clone(),
                &fixture.actors[0],
                &facet_cursor,
            )
            .await;
        }

        let mut typed_direct = direct.clone();
        let mut typed_cursor = direct_cursor;
        let typed_command_outcome =
            apply_direct_realtime_intent(&mut typed_direct, &fixture.actors[0], &intent);
        typed_cursor.record(
            CertificationCommitKind::Command,
            typed_command_outcome.state_changed,
        );
        let mut typed_direct_steps = vec![direct_step(
            &typed_direct,
            &fixture.actors[0],
            typed_command_outcome,
            &typed_cursor,
        )];
        for _ in 0..ticks {
            let tick_outcome = typed_direct
                .advance_realtime_boundary()
                .expect("direct typed realtime boundary succeeds");
            typed_cursor.record(CertificationCommitKind::System, tick_outcome.state_changed);
            typed_direct_steps.push(direct_step(
                &typed_direct,
                &fixture.actors[0],
                tick_outcome,
                &typed_cursor,
            ));
        }

        let direct_outcome = apply_direct_intent(&mut direct, &fixture.actors[0], &intent);
        direct_cursor.record(
            CertificationCommitKind::Command,
            direct_outcome.state_changed,
        );
        for step in typed_direct_steps.iter().skip(1) {
            direct_cursor.record(CertificationCommitKind::System, step.outcome.state_changed);
        }
        assert_eq!(
            direct_cursor, typed_cursor,
            "{label} {}: ordinary and typed direct commit accounting",
            fixture.actors[0]
        );
        let direct_boundary =
            direct_step(&direct, &fixture.actors[0], direct_outcome, &direct_cursor);
        let mut facet_steps = vec![
            command_with_trace(
                &handle,
                &grants[0],
                wire::CommandId::new(uuid::Uuid::from_u128(0x401 + index as u128)).unwrap(),
                index as u64 + 1,
                facet_cursor.facet_revision,
                intent,
            )
            .await,
        ];
        for _ in 0..ticks {
            facet_steps.push(tick_with_trace(&handle, fixture.actors[0].clone()).await);
        }
        let mut kinds = vec![CertificationCommitKind::Command];
        kinds.extend(std::iter::repeat_n(CertificationCommitKind::System, ticks));
        compare_typed_substeps(label, &fixture.actors[0], &typed_direct_steps, &facet_steps);
        compare_common_boundary(
            label,
            &fixture.actors[0],
            &direct_boundary,
            &facet_steps,
            &mut facet_cursor,
            &kinds,
            &mut semantic,
        );
        drain_outbound(&mut outbound);
    }

    for (index, grant) in grants.iter().enumerate() {
        let outcome = direct
            .apply_connection_presence(&fixture.rules_characters[index], 1, false)
            .unwrap();
        direct_cursor.record(CertificationCommitKind::System, outcome.state_changed);
        let direct_presence = direct_step(&direct, &fixture.actors[index], outcome, &direct_cursor);
        let facet_presence = detach_with_trace(&handle, grant.connection_id).await;
        compare_common_boundary(
            if index == 0 {
                "presence_primary_off"
            } else {
                "presence_comparison_off"
            },
            &fixture.actors[index],
            &direct_presence,
            &[facet_presence],
            &mut facet_cursor,
            &[CertificationCommitKind::System],
            &mut semantic,
        );
        drain_outbound(&mut outbound);
    }
    drop(terminals);
    semantic.finalize().into()
}

#[tokio::test]
async fn fixed_enqueue_sim_and_facet_paths_are_byte_exact_and_repeatable() {
    let fixture = deterministic_fixture();
    let first = run_determinism_oracle(fixture.clone()).await;
    let second = run_determinism_oracle(fixture).await;
    let semantic_sha256 = first
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(first, second, "semantic trace SHA-256 must repeat exactly");
    assert_eq!(
        semantic_sha256, "9cda8c8b76b554c1531a71ba2e04d937462ec964a7570858f501858a2a88c772",
        "semantic trace SHA-256 is an approval-visible determinism contract"
    );
}

fn command(grant: &ControlGrant, command_id: wire::CommandId) -> FacetCommand {
    FacetCommand {
        connection_id: grant.connection_id,
        account_id: grant.account_id,
        session_id: grant.session_id,
        character_id: grant.character_id,
        command_id,
        control_epoch: grant.control_epoch,
        client_sequence: 1,
        observed_facet_revision: 1,
        actor_id: wire::ActorId::new(grant.actor_id.as_str()).unwrap(),
        intent: wire::Intent::SetPagesEnabled { enabled: false },
        request_digest: [7; 32],
        certification_trace: None,
        ev_fail_checkpoint_export: false,
        ev_fail_after_store_commit: false,
    }
}

async fn hold(handle: &FacetHandle) -> (oneshot::Sender<()>, oneshot::Receiver<()>) {
    let (entered, entered_receive) = oneshot::channel();
    let (release, release_receive) = oneshot::channel();
    handle
        .sender
        .send(FacetRequest::Hold {
            entered,
            release: release_receive,
        })
        .await
        .unwrap();
    (release, entered_receive)
}

struct InstallRequestHarness {
    request: FacetRequest,
    welcome: oneshot::Receiver<Result<FacetWelcome, FacetError>>,
    outbound: mpsc::Receiver<wire::ServerEnvelope>,
}

fn install_request(grant: ControlGrant) -> InstallRequestHarness {
    let (outbound, outbound_receive) = mpsc::channel(crate::config::OUTBOUND_QUEUE_CAPACITY);
    let (terminal, _terminal_receive) = watch::channel(None);
    let (reply, receive) = oneshot::channel();
    InstallRequestHarness {
        request: FacetRequest::InstallGrant {
            grant,
            outbound,
            terminal,
            certification_trace: None,
            reply,
        },
        welcome: receive,
        outbound: outbound_receive,
    }
}

async fn inspect(handle: &FacetHandle, character_id: tme_rules::CharacterId) -> FacetInspection {
    let (reply, receive) = oneshot::channel();
    handle
        .sender
        .send(FacetRequest::Inspect {
            character_id,
            reply,
        })
        .await
        .unwrap();
    receive.await.unwrap()
}

struct TwoObserverHarness {
    handle: FacetHandle,
    fixture: DeterminismFixture,
    grants: [ControlGrant; 2],
    outbound: [mpsc::Sender<wire::ServerEnvelope>; 2],
    outbound_receive: [mpsc::Receiver<wire::ServerEnvelope>; 2],
}

async fn two_observers_from_fixture(fixture: DeterminismFixture) -> TwoObserverHarness {
    let facet_id = wire::FacetId::new(uuid::Uuid::from_u128(0x501)).unwrap();
    let handle = FacetHandle::spawn_with_id(facet_id, fixture.engine.clone());
    let grants = [
        deterministic_grant(0, facet_id, &fixture),
        deterministic_grant(1, facet_id, &fixture),
    ];
    let (first_outbound, first_receive) = mpsc::channel(crate::config::OUTBOUND_QUEUE_CAPACITY);
    let (second_outbound, second_receive) = mpsc::channel(crate::config::OUTBOUND_QUEUE_CAPACITY);
    let (first_terminal, _first_terminal_receive) = watch::channel(None);
    let (second_terminal, _second_terminal_receive) = watch::channel(None);
    handle
        .install_grant(grants[0].clone(), first_outbound.clone(), first_terminal)
        .await
        .unwrap();
    handle
        .install_grant(grants[1].clone(), second_outbound.clone(), second_terminal)
        .await
        .unwrap();
    let mut harness = TwoObserverHarness {
        handle,
        fixture,
        grants,
        outbound: [first_outbound, second_outbound],
        outbound_receive: [first_receive, second_receive],
    };
    drain_outbound(&mut harness.outbound_receive);
    harness
}

async fn two_observers() -> TwoObserverHarness {
    two_observers_from_fixture(deterministic_fixture()).await
}

fn path_preview_request(grant: &ControlGrant, observed_facet_revision: u64) -> FacetPathPreview {
    FacetPathPreview {
        grant: grant.clone(),
        preview_id: wire::PreviewId::new(uuid::Uuid::now_v7()).unwrap(),
        control_epoch: grant.control_epoch,
        observed_facet_revision,
        actor_id: wire::ActorId::new(grant.actor_id.as_str()).unwrap(),
        path: vec![wire::Direction::North],
    }
}

#[tokio::test]
async fn path_preview_is_serialized_authorized_and_non_mutating() {
    let mut harness = two_observers().await;
    let before = inspect(&harness.handle, harness.fixture.rules_characters[0].clone()).await;

    let response = harness
        .handle
        .try_path_preview(path_preview_request(
            &harness.grants[0],
            before.facet_revision,
        ))
        .unwrap()
        .await
        .unwrap()
        .envelope;
    let successful_preview = match response {
        wire::ServerEnvelope::PathPreviewResult {
            disposition: wire::PathPreviewDisposition::Previewed,
            control_epoch,
            actor_id,
            world_revision,
            preview: Some(preview),
            ..
        } => {
            assert_eq!(control_epoch.get(), harness.grants[0].control_epoch);
            assert_eq!(actor_id.as_str(), harness.grants[0].actor_id.as_str());
            assert_eq!(world_revision.get(), before.facet_revision);
            assert_eq!(preview.contract_version, 8);
            assert_eq!(preview.actor_id, actor_id);
            preview
        }
        other => panic!("unexpected path preview response: {other:?}"),
    };
    let current = Engine::hydrate_checkpoint(
        harness.fixture.engine.definition().clone(),
        &before.checkpoint,
    )
    .expect("current facet checkpoint hydrates for Path 8 parity");
    let direct = current
        .preview_actor_path(&harness.grants[0].actor_id, &[tme_rules::Direction::North])
        .expect("direct Path 8 preview");
    assert_eq!(
        successful_preview,
        crate::protocol_v1::path_preview(&direct).unwrap()
    );

    // D4: there is no "wrong world" rejection to exercise. A client cannot name
    // a world, so it cannot name the wrong one.
    for (mut request, expected) in [
        {
            let mut request = path_preview_request(&harness.grants[0], before.facet_revision);
            request.actor_id = wire::ActorId::new("not-the-controlled-actor").unwrap();
            (request, wire::PathPreviewRejectionCode::WrongActor)
        },
        {
            let mut request = path_preview_request(&harness.grants[0], before.facet_revision);
            request.control_epoch += 1;
            (request, wire::PathPreviewRejectionCode::StaleControlEpoch)
        },
    ] {
        request.preview_id = wire::PreviewId::new(uuid::Uuid::now_v7()).unwrap();
        let response = harness
            .handle
            .try_path_preview(request)
            .unwrap()
            .await
            .unwrap()
            .envelope;
        assert!(matches!(
            response,
            wire::ServerEnvelope::PathPreviewResult {
                disposition: wire::PathPreviewDisposition::Rejected { code },
                preview: None,
                ..
            } if code == expected
        ));
    }

    let stale_revision = harness
        .handle
        .try_path_preview(path_preview_request(&harness.grants[0], 0))
        .unwrap()
        .await
        .unwrap()
        .envelope;
    assert!(matches!(
        stale_revision,
        wire::ServerEnvelope::PathPreviewResult {
            disposition: wire::PathPreviewDisposition::Previewed,
            preview: Some(_),
            ..
        }
    ));

    let mut future = path_preview_request(&harness.grants[0], before.facet_revision + 1);
    future.preview_id = wire::PreviewId::new(uuid::Uuid::now_v7()).unwrap();
    let rejected = harness
        .handle
        .try_path_preview(future)
        .unwrap()
        .await
        .unwrap()
        .envelope;
    assert!(matches!(
        rejected,
        wire::ServerEnvelope::PathPreviewResult {
            disposition: wire::PathPreviewDisposition::Rejected {
                code: wire::PathPreviewRejectionCode::FutureWorldRevision,
            },
            preview: None,
            ..
        }
    ));

    let mut dead_fixture = deterministic_fixture();
    dead_fixture.engine.world_mut().actors[0].life_state = tme_rules::ActorLifeState::Dead;
    let dead_harness = two_observers_from_fixture(dead_fixture).await;
    let rules_rejected = dead_harness
        .handle
        .try_path_preview(path_preview_request(&dead_harness.grants[0], 0))
        .unwrap()
        .await
        .unwrap()
        .envelope;
    assert!(matches!(
        rules_rejected,
        wire::ServerEnvelope::PathPreviewResult {
            disposition: wire::PathPreviewDisposition::Rejected {
                code: wire::PathPreviewRejectionCode::RulesRejected,
            },
            preview: None,
            ..
        }
    ));

    let after = inspect(&harness.handle, harness.fixture.rules_characters[0].clone()).await;
    assert_eq!(after.server_sequence, before.server_sequence);
    assert_eq!(after.facet_revision, before.facet_revision);
    assert_eq!(after.checkpoint.as_bytes(), before.checkpoint.as_bytes());
    assert_eq!(after.projection, before.projection);
    assert!(matches!(
        harness.outbound_receive[0].try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    assert!(matches!(
        harness.outbound_receive[1].try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}

#[tokio::test]
async fn path_preview_uses_the_bounded_facet_mailbox() {
    let harness = two_observers().await;
    let (release, entered) = hold(&harness.handle).await;
    entered.await.unwrap();
    let mut queued = Vec::new();
    for _ in 0..FACET_MAILBOX_CAPACITY {
        queued.push(
            harness
                .handle
                .ev_try_inspect(harness.fixture.rules_characters[0].clone())
                .unwrap(),
        );
    }
    assert!(matches!(
        harness
            .handle
            .try_path_preview(path_preview_request(&harness.grants[0], 0)),
        Err(FacetError::QueueFull)
    ));
    release.send(()).unwrap();
    for receive in queued {
        receive.await.unwrap();
    }
}

fn saturate_outbound(outbound: &mpsc::Sender<wire::ServerEnvelope>) {
    assert_eq!(outbound.capacity(), crate::config::OUTBOUND_QUEUE_CAPACITY);
    for _ in 0..crate::config::OUTBOUND_QUEUE_CAPACITY {
        outbound
            .try_send(wire::ServerEnvelope::ServerDraining {
                reason: wire::DrainingReason::Shutdown,
                reconnect_hint: false,
            })
            .unwrap();
    }
    assert_eq!(outbound.capacity(), 0);
    assert!(matches!(
        outbound.try_send(wire::ServerEnvelope::ServerDraining {
            reason: wire::DrainingReason::Shutdown,
            reconnect_hint: false,
        }),
        Err(mpsc::error::TrySendError::Full(_))
    ));
}

async fn assert_connection_state(harness: &TwoObserverHarness, index: usize, connected: bool) {
    let inspection = inspect(
        &harness.handle,
        harness.fixture.rules_characters[index].clone(),
    )
    .await;
    assert_eq!(inspection.connected, connected);
    assert_eq!(inspection.pending_detaches, 0);
}

#[tokio::test]
async fn full_mailbox_detach_finishes_presence_cleanup() {
    let facet_id = wire::FacetId::new(uuid::Uuid::now_v7()).unwrap();
    let character_id = wire::CharacterId::new(uuid::Uuid::now_v7()).unwrap();
    let connection_id = wire::ConnectionId::new(uuid::Uuid::now_v7()).unwrap();
    let (engine, rules_character_id) = certification_engine(character_id);
    let handle = FacetHandle::spawn_with_id(facet_id, engine);
    let (outbound, _outbound_receive) = mpsc::channel(crate::config::OUTBOUND_QUEUE_CAPACITY);
    let (terminal, _terminal_receive) = watch::channel(None);
    handle
        .install_grant(
            grant(facet_id, character_id, connection_id),
            outbound,
            terminal,
        )
        .await
        .unwrap();
    let before = inspect(&handle, rules_character_id.clone()).await;
    let (release, entered_receive) = hold(&handle).await;
    entered_receive.await.unwrap();
    let mut inspections = Vec::new();
    for _ in 0..FACET_MAILBOX_CAPACITY {
        inspections.push(handle.ev_try_inspect(rules_character_id.clone()).unwrap());
    }
    assert_eq!(handle.mailbox_depth(), FACET_MAILBOX_CAPACITY);
    assert!(matches!(
        handle.ev_try_inspect(rules_character_id.clone()),
        Err(FacetError::QueueFull)
    ));

    let detach_handle = handle.clone();
    let detach = tokio::spawn(async move { detach_handle.detach(connection_id).await });
    tokio::task::yield_now().await;
    assert!(!detach.is_finished());
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), detach)
        .await
        .expect("bounded detach enqueue completes")
        .unwrap()
        .unwrap();
    for inspection in inspections {
        inspection.await.unwrap();
    }

    let inspection = inspect(&handle, rules_character_id).await;
    assert_eq!(inspection.active_observers, 0);
    assert_eq!(inspection.pending_detaches, 0);
    assert!(!inspection.connected);
    assert_eq!(
        checkpoint_field(&inspection.checkpoint, "/world/timing/now")
            .as_u64()
            .unwrap(),
        checkpoint_field(&before.checkpoint, "/world/timing/now")
            .as_u64()
            .unwrap()
    );
    assert_eq!(inspection.server_sequence, before.server_sequence + 1);
    assert_eq!(inspection.facet_revision, before.facet_revision + 1);
}

#[tokio::test]
async fn full_production_outbound_queue_finishes_pending_presence_cleanup() {
    let facet_id = wire::FacetId::new(uuid::Uuid::now_v7()).unwrap();
    let character_id = wire::CharacterId::new(uuid::Uuid::now_v7()).unwrap();
    let connection_id = wire::ConnectionId::new(uuid::Uuid::now_v7()).unwrap();
    let (engine, rules_character_id) = certification_engine(character_id);
    let handle = FacetHandle::spawn_with_id(facet_id, engine);
    let (outbound, _outbound_receive) = mpsc::channel(crate::config::OUTBOUND_QUEUE_CAPACITY);
    let (terminal, _terminal_receive) = watch::channel(None);
    handle
        .install_grant(
            grant(facet_id, character_id, connection_id),
            outbound.clone(),
            terminal,
        )
        .await
        .unwrap();
    for _ in 0..crate::config::OUTBOUND_QUEUE_CAPACITY {
        outbound
            .try_send(wire::ServerEnvelope::ServerDraining {
                reason: wire::DrainingReason::Shutdown,
                reconnect_hint: false,
            })
            .unwrap();
    }
    assert_eq!(outbound.capacity(), 0);
    assert!(matches!(
        outbound.try_send(wire::ServerEnvelope::ServerDraining {
            reason: wire::DrainingReason::Shutdown,
            reconnect_hint: false,
        }),
        Err(mpsc::error::TrySendError::Full(_))
    ));

    handle.sender.send(FacetRequest::Tick).await.unwrap();
    let inspection = inspect(&handle, rules_character_id).await;
    assert_eq!(inspection.active_observers, 0);
    assert_eq!(inspection.pending_detaches, 0);
    assert!(!inspection.connected);
}

#[tokio::test]
async fn explicit_revoke_keeps_socket_path_until_terminal_and_presence_commit() {
    let facet_id = wire::FacetId::new(uuid::Uuid::now_v7()).unwrap();
    let character_id = wire::CharacterId::new(uuid::Uuid::now_v7()).unwrap();
    let connection_id = wire::ConnectionId::new(uuid::Uuid::now_v7()).unwrap();
    let (engine, rules_character_id) = certification_engine(character_id);
    let handle = FacetHandle::spawn_with_id(facet_id, engine);
    let (outbound, mut outbound_receive) = mpsc::channel(crate::config::OUTBOUND_QUEUE_CAPACITY);
    let (terminal, mut terminal_receive) = watch::channel(None);
    handle
        .install_grant(
            grant(facet_id, character_id, connection_id),
            outbound,
            terminal,
        )
        .await
        .unwrap();

    let mutation_epoch = 1;
    handle
        .prepare_character_exit(mutation_epoch, rules_character_id.clone())
        .await
        .unwrap();
    let mut completion = handle
        .begin_revoke_grant(connection_id, wire::DrainingReason::SessionEnded)
        .await
        .unwrap();
    assert!(matches!(
        outbound_receive.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    assert_eq!(*terminal_receive.borrow_and_update(), None);
    assert!(matches!(
        completion.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));

    handle.rollback_transfer(mutation_epoch).await.unwrap();
    completion.await.unwrap();
    terminal_receive.changed().await.unwrap();
    assert_eq!(
        *terminal_receive.borrow_and_update(),
        Some(wire::DrainingReason::SessionEnded)
    );
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(1), outbound_receive.recv())
            .await
            .unwrap(),
        None
    );
    let inspection = inspect(&handle, rules_character_id).await;
    assert_eq!(inspection.active_observers, 0);
    assert_eq!(inspection.pending_detaches, 0);
    assert!(!inspection.connected);
}

#[tokio::test]
async fn slow_mutation_fanout_observer_is_detached_after_presence_cleanup() {
    let harness = two_observers().await;
    saturate_outbound(&harness.outbound[1]);

    let reply = harness
        .handle
        .try_command(command(
            &harness.grants[0],
            wire::CommandId::new(uuid::Uuid::from_u128(0x511)).unwrap(),
        ))
        .unwrap()
        .await
        .unwrap();
    assert!(matches!(
        reply.envelope,
        wire::ServerEnvelope::CommandResult {
            disposition: wire::CommandDisposition::Accepted,
            replay_status: wire::ReplayStatus::New,
            ..
        }
    ));
    assert_connection_state(&harness, 0, true).await;
    assert_connection_state(&harness, 1, false).await;
}

#[tokio::test]
async fn multiple_slow_system_fanout_observers_are_drained_iteratively() {
    let harness = two_observers().await;
    saturate_outbound(&harness.outbound[0]);
    saturate_outbound(&harness.outbound[1]);

    harness
        .handle
        .sender
        .send(FacetRequest::Tick)
        .await
        .unwrap();
    assert_connection_state(&harness, 0, false).await;
    assert_connection_state(&harness, 1, false).await;
}

#[tokio::test]
async fn slow_transient_social_recipient_is_detached_after_presence_cleanup() {
    let harness = two_observers().await;
    saturate_outbound(&harness.outbound[1]);

    let outcome = harness
        .handle
        .social_message(
            harness.grants[0].clone(),
            wire::MessageId::new(uuid::Uuid::from_u128(0x521)).unwrap(),
            wire::SocialScope::Say,
            wire::SocialBody::new("bounded EV social fanout").unwrap(),
        )
        .await
        .unwrap();
    assert!(matches!(
        outcome,
        FacetSocialOutcome::Complete(wire::MessageDisposition::Accepted)
    ));
    assert_connection_state(&harness, 0, true).await;
    assert_connection_state(&harness, 1, false).await;
}

#[tokio::test]
async fn slow_page_recipient_is_detached_after_presence_cleanup() {
    let harness = two_observers().await;
    saturate_outbound(&harness.outbound[1]);

    let delivered = harness
        .handle
        .deliver_page(
            harness.grants[1].clone(),
            wire::MessageId::new(uuid::Uuid::from_u128(0x531)).unwrap(),
            harness.grants[0].character_id,
            wire::DisplayName::new("EV Sender").unwrap(),
            wire::SocialBody::new("bounded EV page fanout").unwrap(),
        )
        .await
        .unwrap();
    assert!(!delivered);
    assert_connection_state(&harness, 0, true).await;
    assert_connection_state(&harness, 1, false).await;
}

#[tokio::test]
async fn slow_issuer_current_state_update_detaches_after_rejection() {
    let harness = two_observers().await;
    saturate_outbound(&harness.outbound[0]);
    let mut rejected = command(
        &harness.grants[0],
        wire::CommandId::new(uuid::Uuid::from_u128(0x541)).unwrap(),
    );
    rejected.client_sequence = 2;

    let reply = harness.handle.try_command(rejected).unwrap().await.unwrap();
    assert!(matches!(
        reply.envelope,
        wire::ServerEnvelope::CommandResult {
            disposition: wire::CommandDisposition::Rejected {
                code: wire::RejectionCode::OutOfOrderClientSequence,
            },
            replay_status: wire::ReplayStatus::New,
            ..
        }
    ));
    assert_connection_state(&harness, 0, false).await;
    assert_connection_state(&harness, 1, true).await;
}

#[tokio::test]
async fn dropped_terminal_result_receiver_detaches_after_committed_command() {
    let harness = two_observers().await;
    let (release, entered) = hold(&harness.handle).await;
    entered.await.unwrap();
    let receive = harness
        .handle
        .try_command(command(
            &harness.grants[0],
            wire::CommandId::new(uuid::Uuid::from_u128(0x551)).unwrap(),
        ))
        .unwrap();
    drop(receive);
    release.send(()).unwrap();

    assert_connection_state(&harness, 0, false).await;
    assert_connection_state(&harness, 1, true).await;
}

#[tokio::test]
async fn queued_old_control_work_is_ordered_on_both_sides_of_replacement() {
    let facet_id = wire::FacetId::new(uuid::Uuid::now_v7()).unwrap();
    let character_id = wire::CharacterId::new(uuid::Uuid::now_v7()).unwrap();
    let old_connection = wire::ConnectionId::new(uuid::Uuid::now_v7()).unwrap();
    let new_connection = wire::ConnectionId::new(uuid::Uuid::now_v7()).unwrap();
    let (engine, _) = certification_engine(character_id);
    let handle = FacetHandle::spawn_with_id(facet_id, engine);
    let old_grant = grant(facet_id, character_id, old_connection);
    let (old_outbound, _old_outbound_receive) =
        mpsc::channel(crate::config::OUTBOUND_QUEUE_CAPACITY);
    let (old_terminal, mut old_terminal_receive) = watch::channel(None);
    handle
        .install_grant(old_grant.clone(), old_outbound, old_terminal)
        .await
        .unwrap();
    let mut replacement = old_grant.clone();
    replacement.connection_id = new_connection;

    let (first_release, first_entered) = hold(&handle).await;
    first_entered.await.unwrap();
    let mut old_before = handle
        .try_command(command(
            &old_grant,
            wire::CommandId::new(uuid::Uuid::now_v7()).unwrap(),
        ))
        .unwrap();
    let (middle_release, middle_entered) = hold(&handle).await;
    let InstallRequestHarness {
        request: replacement_request,
        welcome: mut replacement_receive,
        outbound: _replacement_outbound,
    } = install_request(replacement.clone());
    handle.sender.send(replacement_request).await.unwrap();
    first_release.send(()).unwrap();
    middle_entered.await.unwrap();
    let old_before_result = old_before.try_recv().unwrap().envelope;
    assert!(
        matches!(
            old_before_result,
            wire::ServerEnvelope::CommandResult {
                disposition: wire::CommandDisposition::Accepted,
                replay_status: wire::ReplayStatus::New,
                ..
            }
        ),
        "old work before replacement returned {old_before_result:?}"
    );
    assert!(matches!(
        replacement_receive.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
    ));
    middle_release.send(()).unwrap();
    replacement_receive.await.unwrap().unwrap();
    old_terminal_receive.changed().await.unwrap();
    assert_eq!(
        *old_terminal_receive.borrow_and_update(),
        Some(wire::DrainingReason::ControlReplaced)
    );

    let second_character_id = wire::CharacterId::new(uuid::Uuid::now_v7()).unwrap();
    let second_old_connection = wire::ConnectionId::new(uuid::Uuid::now_v7()).unwrap();
    let second_new_connection = wire::ConnectionId::new(uuid::Uuid::now_v7()).unwrap();
    let (second_engine, _) = certification_engine(second_character_id);
    let second = FacetHandle::spawn_with_id(facet_id, second_engine);
    let second_old_grant = grant(facet_id, second_character_id, second_old_connection);
    let (second_outbound, _second_outbound_receive) =
        mpsc::channel(crate::config::OUTBOUND_QUEUE_CAPACITY);
    let (second_terminal, _second_terminal_receive) = watch::channel(None);
    second
        .install_grant(second_old_grant.clone(), second_outbound, second_terminal)
        .await
        .unwrap();
    let mut second_replacement = second_old_grant.clone();
    second_replacement.connection_id = second_new_connection;

    let (second_release, second_entered) = hold(&second).await;
    second_entered.await.unwrap();
    let InstallRequestHarness {
        request: second_replacement_request,
        welcome: mut second_replacement_receive,
        outbound: _second_replacement_outbound,
    } = install_request(second_replacement);
    second
        .sender
        .send(second_replacement_request)
        .await
        .unwrap();
    let (after_replacement_release, after_replacement_entered) = hold(&second).await;
    let mut old_after = second
        .try_command(command(
            &second_old_grant,
            wire::CommandId::new(uuid::Uuid::now_v7()).unwrap(),
        ))
        .unwrap();
    second_release.send(()).unwrap();
    after_replacement_entered.await.unwrap();
    second_replacement_receive.try_recv().unwrap().unwrap();
    assert!(matches!(
        old_after.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
    ));
    after_replacement_release.send(()).unwrap();
    assert!(matches!(
        old_after.await.unwrap().envelope,
        wire::ServerEnvelope::CommandResult {
            disposition: wire::CommandDisposition::Rejected {
                code: wire::RejectionCode::WrongActor
            },
            replay_status: wire::ReplayStatus::New,
            ..
        }
    ));
}

#[tokio::test]
async fn equal_facet_clones_start_identically_and_mutate_independently() {
    let character_id = wire::CharacterId::new(uuid::Uuid::now_v7()).unwrap();
    let (engine, rules_character_id) = certification_engine(character_id);
    let first = FacetHandle::spawn_with_id(
        wire::FacetId::new(uuid::Uuid::now_v7()).unwrap(),
        engine.clone(),
    );
    let second =
        FacetHandle::spawn_with_id(wire::FacetId::new(uuid::Uuid::now_v7()).unwrap(), engine);
    let first_initial = inspect(&first, rules_character_id.clone()).await;
    let second_initial = inspect(&second, rules_character_id.clone()).await;
    assert_eq!(
        first_initial.checkpoint.as_bytes(),
        second_initial.checkpoint.as_bytes()
    );
    assert_eq!(
        first_initial.server_sequence,
        second_initial.server_sequence
    );
    assert_eq!(first_initial.facet_revision, second_initial.facet_revision);
    assert_eq!(first_initial.projection, second_initial.projection);
    assert_eq!(
        checkpoint_field(&first_initial.checkpoint, "/world/timing/now"),
        checkpoint_field(&second_initial.checkpoint, "/world/timing/now")
    );
    assert_eq!(
        checkpoint_field(&first_initial.checkpoint, "/rng_state"),
        checkpoint_field(&second_initial.checkpoint, "/rng_state")
    );

    first.sender.send(FacetRequest::Tick).await.unwrap();
    let first_mutated = inspect(&first, rules_character_id.clone()).await;
    let second_unchanged = inspect(&second, rules_character_id).await;
    assert_ne!(
        first_mutated.checkpoint.as_bytes(),
        first_initial.checkpoint.as_bytes()
    );
    assert_eq!(
        second_unchanged.checkpoint.as_bytes(),
        second_initial.checkpoint.as_bytes()
    );
    assert_eq!(
        second_unchanged.server_sequence,
        second_initial.server_sequence
    );
    assert_eq!(
        second_unchanged.facet_revision,
        second_initial.facet_revision
    );
    assert_eq!(second_unchanged.projection, second_initial.projection);
    assert_eq!(
        checkpoint_field(&second_unchanged.checkpoint, "/world/timing/now"),
        checkpoint_field(&second_initial.checkpoint, "/world/timing/now")
    );
    assert_eq!(
        checkpoint_field(&second_unchanged.checkpoint, "/rng_state"),
        checkpoint_field(&second_initial.checkpoint, "/rng_state")
    );
    assert_ne!(first_mutated.server_sequence, first_initial.server_sequence);
    assert_ne!(first_mutated.facet_revision, first_initial.facet_revision);
    assert_ne!(
        checkpoint_field(&first_mutated.checkpoint, "/world/timing/now"),
        checkpoint_field(&first_initial.checkpoint, "/world/timing/now")
    );
}
