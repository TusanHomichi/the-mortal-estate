use serde_json::{Value, json};
use std::path::PathBuf;
use tme_rules::{
    ACTION_CONTEXT_CONTRACT_VERSION, COMMAND_CONTRACT_VERSION, EVENT_CONTRACT_VERSION,
    OBSERVED_SNAPSHOT_CONTRACT_VERSION, SNAPSHOT_CONTRACT_VERSION, TRACE_V2_CONTRACT_VERSION,
    TraceV1, TraceV2,
};

fn scenario_path(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/test-corpus")
        .join(name)
        .to_string_lossy()
        .to_string()
}

fn run_trace(scenario: &str, seed: u64) -> TraceV1 {
    let args = vec![
        "tme-sim".to_string(),
        "--scenario".to_string(),
        scenario_path(scenario),
        "--seed".to_string(),
        seed.to_string(),
        "--trace-json".to_string(),
    ];
    let output = tme_sim::run_from_args(args).expect("trace run should succeed");
    serde_json::from_str(&output).expect("trace output must be valid JSON")
}

#[test]
fn creature_ecology_gallery_trace_v1_and_v2_match_goldens() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (flag, golden) in [
        (
            "--trace-json",
            "trace_v1_creature_ecology_gallery_seed_7.json",
        ),
        (
            "--trace-json-v2",
            "trace_v2_creature_ecology_gallery_seed_7.json",
        ),
    ] {
        let output = tme_sim::run_from_args([
            "tme-sim".to_string(),
            "--scenario".to_string(),
            scenario_path("creature_ecology_gallery.json"),
            "--seed".to_string(),
            "7".to_string(),
            flag.to_string(),
        ])
        .expect("creature ecology trace should run");
        let expected = std::fs::read_to_string(manifest_dir.join("tests/golden").join(golden))
            .expect("creature ecology trace golden should read");
        assert_eq!(output, expected);
    }
}

#[test]
fn trace_json_output_is_valid_and_has_header() {
    let trace = run_trace("first_room.json", 42);

    assert_eq!(trace.header.contract_version, 1);
    assert_eq!(
        trace.header.initial_snapshot.contract_version,
        SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(trace.header.seed, 42);
    assert!(!trace.header.scenario_id.is_empty());
    // Initial snapshot must include the player
    assert!(
        trace
            .header
            .initial_snapshot
            .actors
            .iter()
            .any(|a| a.kind == tme_rules::ActorKind::Player),
        "initial snapshot must include the player"
    );
}

#[test]
fn freshly_generated_v1_and_v2_traces_pass_the_rust_consistency_owner() {
    for flag in ["--trace-json", "--trace-json-v2"] {
        let output = tme_sim::run_from_args([
            "tme-sim".to_string(),
            "--scenario".to_string(),
            scenario_path("first_room.json"),
            "--seed".to_string(),
            "7".to_string(),
            flag.to_string(),
        ])
        .expect("fresh trace should run");
        tme_sim::validate_trace_json(&output).expect("fresh trace must be consistent");
    }
}

#[test]
fn trace_json_steps_match_script_length() {
    let trace = run_trace("first_room.json", 42);

    // first_room has a script with steps; trace must have at least one step
    assert!(
        !trace.steps.is_empty(),
        "trace must have at least one step for a scripted scenario"
    );

    // Each step must have an intent label and events
    for step in &trace.steps {
        assert!(
            !step.intent_label.is_empty(),
            "each step must have an intent"
        );
        assert!(
            !step.events.is_empty(),
            "each step must have at least one event"
        );
        assert_eq!(step.contract_version, 1);
        assert_eq!(
            step.after_snapshot.contract_version,
            SNAPSHOT_CONTRACT_VERSION
        );
        // after_snapshot must include the player
        assert!(
            step.after_snapshot
                .actors
                .iter()
                .any(|a| a.kind == tme_rules::ActorKind::Player),
            "after_snapshot must include the player"
        );
    }
}

#[test]
fn trace_json_events_are_deserializable() {
    let trace = run_trace("first_room.json", 42);

    // The first addressed commit starts with an actor-ready event.
    let first_events = &trace.steps[0].events;
    let has_actor_ready = first_events
        .iter()
        .any(|e| matches!(e, tme_rules::Event::ActorReady { .. }));
    assert!(
        has_actor_ready,
        "first step should contain ActorReady event"
    );
}

#[test]
fn trace_json_has_final_snapshot() {
    let trace = run_trace("first_room.json", 42);

    assert_eq!(trace.r#final.contract_version, 1);
    assert_eq!(
        trace.r#final.final_snapshot.contract_version,
        SNAPSHOT_CONTRACT_VERSION
    );
    // Final logical time should be >= the initial logical time.
    assert!(
        trace.r#final.final_snapshot.logical_time >= trace.header.initial_snapshot.logical_time,
        "final logical time must be >= initial logical time"
    );
}

#[test]
fn trace_json_preview_included_for_move_path() {
    let trace = run_trace("terrain_movement.json", 42);

    // Every Walk/Run/Sprint step uses the sole MovePath route and has a preview.
    let move_steps_with_preview: Vec<_> = trace
        .steps
        .iter()
        .filter(|step| {
            matches!(
                step.intent_label.split_whitespace().next(),
                Some("walk" | "run" | "sprint")
            ) && step.preview.is_some()
        })
        .collect();
    let move_steps: Vec<_> = trace
        .steps
        .iter()
        .filter(|step| {
            matches!(
                step.intent_label.split_whitespace().next(),
                Some("walk" | "run" | "sprint")
            )
        })
        .collect();
    assert!(
        !move_steps.is_empty(),
        "terrain_movement should have move steps"
    );
    assert!(
        !move_steps_with_preview.is_empty(),
        "at least one move step must have a non-None preview"
    );
}

#[test]
fn trace_json_is_deterministic() {
    let trace_a = run_trace("first_room.json", 42);
    let trace_b = run_trace("first_room.json", 42);
    // Compare raw JSON strings for determinism
    let output_a = serde_json::to_string(&trace_a).expect("serialize");
    let output_b = serde_json::to_string(&trace_b).expect("serialize");
    assert_eq!(output_a, output_b, "trace output must be deterministic");
}

#[test]
fn trace_json_spell_learning_fixture_includes_learn_intent_and_event() {
    let trace = run_trace("spell_learning_purchase_casting_xp.json", 7);

    let learn_step = trace
        .steps
        .iter()
        .find(|step| step.intent_label == "learn_spell spark")
        .expect("trace should include learn_spell spark");

    assert!(learn_step.events.iter().any(|event| matches!(
        event,
        tme_rules::Event::SpellLearned {
            spell_id,
            spell_name,
            lane,
            gold_cost,
            spell_book_item_instance_id,
            spell_book_item_definition_id,
            ..
        } if spell_id == "spark"
            && spell_name == "Spark"
            && lane == "wizard_magic"
            && *gold_cost == 25
            && spell_book_item_instance_id == "spell_book"
            && spell_book_item_definition_id == "spell_book"
    )));
    let gold_index = learn_step
        .events
        .iter()
        .position(|event| matches!(event, tme_rules::Event::GoldChanged { .. }))
        .expect("learning trace should report the gold cost");
    let learned_index = learn_step
        .events
        .iter()
        .position(|event| matches!(event, tme_rules::Event::SpellLearned { .. }))
        .expect("learning trace should report completion");
    assert!(gold_index < learned_index);
}

// ---------------------------------------------------------------------------
// Trace V2 tests
// ---------------------------------------------------------------------------

fn run_trace_v2(scenario: &str, seed: u64) -> TraceV2 {
    let args = vec![
        "tme-sim".to_string(),
        "--scenario".to_string(),
        scenario_path(scenario),
        "--seed".to_string(),
        seed.to_string(),
        "--trace-json-v2".to_string(),
    ];
    let output = tme_sim::run_from_args(args).expect("trace v2 run should succeed");
    serde_json::from_str(&output).expect("trace v2 output must be valid JSON")
}

fn add_unknown_field(value: &Value, pointer: &str) -> Value {
    let mut mutated = value.clone();
    mutated
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("missing JSON pointer {pointer}"))
        .as_object_mut()
        .unwrap_or_else(|| panic!("JSON pointer {pointer} is not an object"))
        .insert("private_fact".to_string(), json!({"quarantined": true}));
    mutated
}

#[test]
fn trace_contract_deserialization_rejects_unknown_envelopes_and_required_nullable_omissions() {
    let trace = run_trace_v2("terrain_movement.json", 7);
    let value = serde_json::to_value(&trace).expect("trace should serialize");
    let preview_step_index = trace
        .steps
        .iter()
        .position(|step| step.preview.is_some())
        .expect("terrain trace should contain a preview");
    let pointers = [
        "",
        "/header",
        "/steps/0",
        "/final",
        "/header/initial_debug_snapshot",
        "/header/initial_observed_snapshot",
        "/header/initial_debug_snapshot/actors/0",
        "/header/initial_observed_snapshot/actors/0",
        "/header/initial_debug_snapshot/realms/0",
        "/header/initial_observed_snapshot/realms/0",
        "/header/initial_debug_snapshot/realms/0/levels/0",
        "/header/initial_observed_snapshot/realms/0/levels/0",
        "/header/initial_debug_snapshot/realms/0/levels/0/tiles/0",
        "/header/initial_observed_snapshot/realms/0/levels/0/tiles/0",
        "/header/initial_action_context",
    ];
    for pointer in pointers {
        assert!(
            serde_json::from_value::<TraceV2>(add_unknown_field(&value, pointer)).is_err(),
            "Trace V2 accepted an unknown field at {pointer}"
        );
    }
    for pointer in [
        format!("/steps/{preview_step_index}/preview"),
        format!("/steps/{preview_step_index}/preview/steps/0"),
    ] {
        assert!(
            serde_json::from_value::<TraceV2>(add_unknown_field(&value, &pointer)).is_err(),
            "Trace V2 accepted an unknown field at {pointer}"
        );
    }

    let mut missing_character_id = value.clone();
    missing_character_id["header"]["initial_debug_snapshot"]["actors"][0]
        .as_object_mut()
        .expect("debug actor")
        .remove("character_id");
    assert!(serde_json::from_value::<TraceV2>(missing_character_id).is_err());

    let mut missing_preview = value.clone();
    missing_preview["steps"][preview_step_index]
        .as_object_mut()
        .expect("trace step")
        .remove("preview");
    assert!(serde_json::from_value::<TraceV2>(missing_preview).is_err());

    let mut unknown_event = value;
    unknown_event["steps"][0]["events"][0] = json!({"future_private_event": {}});
    assert!(serde_json::from_value::<TraceV2>(unknown_event).is_err());

    let v1_value = serde_json::to_value(run_trace("terrain_movement.json", 7))
        .expect("Trace V1 should serialize");
    assert!(serde_json::from_value::<TraceV1>(add_unknown_field(&v1_value, "")).is_err());
}

#[test]
fn trace_v2_header_has_all_version_fields() {
    let trace = run_trace_v2("first_room.json", 42);
    assert_eq!(trace.header.contract_version, TRACE_V2_CONTRACT_VERSION);
    assert_eq!(trace.header.seed, 42);
    assert!(!trace.header.scenario_id.is_empty());
    assert!(trace.header.event_contract_version > 0);
    assert!(trace.header.snapshot_contract_version > 0);
    assert!(trace.header.observed_snapshot_contract_version > 0);
    assert!(trace.header.action_context_contract_version > 0);
    assert!(trace.header.intent_contract_version > 0);
}

#[test]
fn trace_v2_includes_observed_snapshot() {
    let trace = run_trace_v2("first_room.json", 42);
    // Observed snapshot must have actors (the player should be visible)
    assert!(
        !trace.header.initial_observed_snapshot.actors.is_empty(),
        "observed snapshot must include visible actors"
    );
}

#[test]
fn trace_v2_includes_action_context() {
    let trace = run_trace_v2("first_room.json", 42);
    let ctx = &trace.header.initial_action_context;
    assert!(
        !ctx.actor_id.is_empty(),
        "action context must have actor_id"
    );
    assert_eq!(ctx.exits.len(), 8, "must have 8 directional exits");
}

#[test]
fn trace_v2_magic_gallery_exposes_typed_multi_capability_service() {
    let trace = run_trace_v2("magic_profession_gallery.json", 7);
    let services = &trace.header.initial_action_context.services_here;
    assert_eq!(services.len(), 1);
    let service = &services[0];
    assert_eq!(service.service_id, "thief_trainer");
    assert_eq!(service.capabilities.len(), 3);

    match &service.capabilities[0] {
        tme_rules::ServiceCapabilityViewV1::SkillTraining {
            capability_id,
            offered_track_ids,
            selected_track_id,
            actions,
        } => {
            assert_eq!(capability_id, "training");
            assert_eq!(offered_track_ids.len(), 1);
            assert_eq!(offered_track_ids[0], "thief_magic");
            assert_eq!(selected_track_id, &None);
            assert!(actions.is_empty());
        }
        other => panic!("expected typed training capability, got {other:?}"),
    }
    match &service.capabilities[1] {
        tme_rules::ServiceCapabilityViewV1::SkillCritique {
            capability_id,
            actions,
        } => {
            assert_eq!(capability_id, "critique");
            assert!(actions.iter().any(|action| matches!(
                &action.command,
                Some(tme_rules::PlayerCommandV1 {
                    intent: tme_rules::PlayerIntentPayloadV1::Critique { track_id, .. },
                    ..
                }) if track_id == "thief_magic"
            )));
        }
        other => panic!("expected typed critique capability, got {other:?}"),
    }
    match &service.capabilities[2] {
        tme_rules::ServiceCapabilityViewV1::SpellTeaching {
            capability_id,
            spell_ids,
            actions,
        } => {
            assert_eq!(capability_id, "spell_teaching");
            assert_eq!(spell_ids.len(), 1);
            assert_eq!(spell_ids[0], "shadow_sting");
            assert_eq!(actions.len(), 1);
        }
        other => panic!("expected typed teaching capability, got {other:?}"),
    }
}

#[test]
fn trace_v2_steps_have_typed_commands() {
    let trace = run_trace_v2("first_room.json", 42);
    assert!(!trace.steps.is_empty(), "must have steps");
    for step in &trace.steps {
        assert!(
            !step.command.actor_id.is_empty(),
            "command must have actor_id"
        );
        assert_eq!(step.command.contract_version, COMMAND_CONTRACT_VERSION);
        assert!(!step.intent_label.is_empty(), "step must have intent_label");
    }
}

#[test]
fn trace_v2_steps_have_observed_snapshots() {
    let trace = run_trace_v2("first_room.json", 42);
    for step in &trace.steps {
        assert!(
            !step.after_observed_snapshot.actors.is_empty(),
            "observed snapshot must have actors"
        );
    }
}

#[test]
fn trace_v2_steps_have_action_context() {
    let trace = run_trace_v2("first_room.json", 42);
    for step in &trace.steps {
        assert!(!step.after_action_context.actor_id.is_empty());
        assert_eq!(step.after_action_context.exits.len(), 8);
    }
}

#[test]
fn trace_v2_is_deterministic() {
    let trace_a = run_trace_v2("first_room.json", 42);
    let trace_b = run_trace_v2("first_room.json", 42);
    let output_a = serde_json::to_string(&trace_a).expect("serialize");
    let output_b = serde_json::to_string(&trace_b).expect("serialize");
    assert_eq!(output_a, output_b, "trace v2 output must be deterministic");
}

#[test]
fn trace_v2_has_final_snapshot() {
    let trace = run_trace_v2("first_room.json", 42);
    assert_eq!(trace.r#final.contract_version, TRACE_V2_CONTRACT_VERSION);
    assert!(
        trace.r#final.final_observed_snapshot.logical_time
            >= trace.header.initial_observed_snapshot.logical_time
    );
}

#[test]
fn trace_v2_spell_casting_steps_use_typed_commands() {
    let trace = run_trace_v2("spell_readiness.json", 7);
    let command_names: Vec<String> = trace
        .steps
        .iter()
        .map(|step| {
            let value = serde_json::to_value(&step.command.intent).expect("intent serializes");
            if let Some(name) = value.as_str() {
                name.to_string()
            } else {
                value
                    .as_object()
                    .expect("intent object")
                    .keys()
                    .next()
                    .expect("variant")
                    .clone()
            }
        })
        .collect();

    assert!(command_names.contains(&"cast_spell".to_string()));
    assert!(command_names.contains(&"warm_spell".to_string()));
    assert!(command_names.contains(&"cast_warmed_spell".to_string()));

    let cast_step = trace
        .steps
        .iter()
        .find(|step| {
            matches!(
                &step.command.intent,
                tme_rules::PlayerIntentPayloadV1::CastWarmedSpell { .. }
            )
        })
        .expect("trace should include a warmed cast step");

    match &cast_step.command.intent {
        tme_rules::PlayerIntentPayloadV1::CastWarmedSpell {
            target: Some(tme_rules::SpellTarget::Actor { actor_id }),
            ..
        } => assert_eq!(actor_id, "watcher"),
        other => panic!("expected resolved actor target on warmed cast step, got {other:?}"),
    }
}

#[test]
fn trace_v2_spell_effects_fixture_exposes_real_effects() {
    let trace = run_trace_v2("spell_effects.json", 7);

    let spell_damage = trace
        .steps
        .iter()
        .flat_map(|step| step.events.iter())
        .find_map(|event| match event {
            tme_rules::Event::SpellDamaged {
                spell_id,
                target_id,
                location,
                ..
            } if spell_id == "spark" && target_id == "target" => Some(location),
            _ => None,
        })
        .expect("trace should expose spell damage target room/position");
    assert_eq!(spell_damage.level, "room_0");
    assert_eq!(spell_damage.position, tme_rules::Coord { x: 3, y: 1 });

    let spell_heal = trace
        .steps
        .iter()
        .flat_map(|step| step.events.iter())
        .find_map(|event| match event {
            tme_rules::Event::SpellHealed {
                spell_id,
                target_id,
                location,
                ..
            } if spell_id == "mend" && target_id == "player" => Some(location),
            _ => None,
        })
        .expect("trace should expose spell healing target room/position");
    assert_eq!(spell_heal.level, "room_0");
    assert_eq!(spell_heal.position, tme_rules::Coord { x: 1, y: 1 });
    assert!(trace.steps.iter().any(|step| {
        step.events.iter().any(|event| {
            matches!(
                event,
                tme_rules::Event::EffectApplied { source_kind, .. } if source_kind == "spell"
            )
        })
    }));

    let final_player = trace
        .r#final
        .final_debug_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("player present in final snapshot");
    let final_resources = &final_player
        .character
        .as_ref()
        .expect("player should expose character resources")
        .resources;
    assert!(final_resources.mp < final_resources.max_mp);

    let final_target = trace
        .r#final
        .final_debug_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "target")
        .expect("target present in final snapshot");
    assert!(final_target.hp < 8);
}

#[test]
fn trace_v2_area_path_terrain_fixture_exposes_bt_tile_effects() {
    let trace = run_trace_v2("area_path_terrain_spells.json", 7);

    assert_eq!(trace.header.contract_version, TRACE_V2_CONTRACT_VERSION);
    assert_eq!(trace.header.event_contract_version, EVENT_CONTRACT_VERSION);
    assert!(trace.steps.iter().flat_map(|step| step.events.iter()).any(
        |event| matches!(event, tme_rules::Event::TileEffectApplied { effect_id, .. } if effect_id == "web_field")
    ));
    assert!(
        trace
            .steps
            .iter()
            .any(|step| !step.after_debug_snapshot.tile_effects.is_empty())
    );
    assert!(trace.steps.iter().any(|step| {
        !step.after_action_context.tile_effects_here.is_empty()
            || step
                .after_action_context
                .exits
                .iter()
                .any(|exit| !exit.tile_effects.is_empty())
    }));
}

#[test]
fn trace_v2_utility_door_secret_item_fixture_exposes_bu_events() {
    let trace = run_trace_v2("utility_door_secret_item_spells.json", 7);

    assert_eq!(trace.header.contract_version, TRACE_V2_CONTRACT_VERSION);
    assert_eq!(trace.header.event_contract_version, EVENT_CONTRACT_VERSION);
    assert_eq!(
        trace.header.snapshot_contract_version,
        SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.observed_snapshot_contract_version,
        OBSERVED_SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.action_context_contract_version,
        ACTION_CONTEXT_CONTRACT_VERSION
    );
    assert!(trace.steps.iter().any(|step| matches!(
        &step.command.intent,
        tme_rules::PlayerIntentPayloadV1::CastSpell {
            spell_id,
            target: Some(tme_rules::SpellTarget::None),
            ..
        } if spell_id == "workroom_glimpse"
    )));

    let events: Vec<&tme_rules::Event> = trace
        .steps
        .iter()
        .flat_map(|step| step.events.iter())
        .collect();

    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::SecretTransitionRevealed { location, transition_kind, .. }
            if location.level == "workroom"
                && location.position == tme_rules::Coord { x: 3, y: 1 }
                && transition_kind == "stairs"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::SecretTransitionHidden { location, transition_kind, .. }
            if location.level == "workroom"
                && location.position == tme_rules::Coord { x: 3, y: 1 }
                && transition_kind == "stairs"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::ItemIdentified {
            item_instance_id,
            item_definition_id,
            location,
            capability,
            ..
        }
            if item_instance_id == "ground_charm"
                && item_definition_id == "ground_charm"
                && location == "ground_here"
                && capability.as_ref().and_then(|capability| capability.taxonomy_id.as_deref()) == Some("trinket")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::ItemEnchanted {
            item_instance_id,
            combat_add_rating_bonus,
            tags,
            remaining_rounds,
            ..
        } if item_instance_id == "utility_blade"
            && *combat_add_rating_bonus == 5
            && tags == &vec!["keen".to_string()]
            && *remaining_rounds == Some(1)
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::ItemEnchantmentExpired {
            item_instance_id,
            enchantment_instance_id,
            ..
        }
            if item_instance_id == "utility_blade"
                && enchantment_instance_id.starts_with("spell:keen_edge:")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::ItemTransformed {
            item_instance_id,
            old_item_definition_id,
            new_item_definition_id,
            location,
            ..
        }
            if item_instance_id == "raw_relic"
                && old_item_definition_id == "raw_relic"
                && new_item_definition_id == "recall_token"
                && location == "sack"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::Located { subject, id, location, hint, .. }
            if subject == "item"
                && id == "ground_charm"
                && location.as_ref().is_some_and(|position| position.level == "workroom" && position.position == tme_rules::Coord { x: 1, y: 1 })
                && hint == "item ground_charm located in workroom at 1,1"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::Located { subject, id, location, hint, .. }
            if subject == "item"
                && id == "veiled_charm"
                && location.is_none()
                && hint == "item veiled_charm is hidden or unobserved"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::Located { subject, id, location, hint, .. }
            if subject == "scry"
                && id == "workroom_glimpse"
                && location.as_ref().is_some_and(|position| position.level == "workroom" && position.position == tme_rules::Coord { x: 1, y: 1 })
                && hint == "scry workroom_glimpse located in workroom at 1,1"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::PortalCreated {
            instance_id,
            location,
            target,
            remaining_rounds,
            two_way,
            ..
        } if instance_id.starts_with("portal:blue_gate:")
            && location.level == "workroom"
            && location.position == tme_rules::Coord { x: 2, y: 1 }
            && target.level == "vault"
            && target.position == tme_rules::Coord { x: 1, y: 1 }
            && *remaining_rounds == Some(1)
            && *two_way
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::PortalExpired { instance_id, location }
            if instance_id.starts_with("portal:blue_gate:")
                && location.level == "workroom"
                && location.position == tme_rules::Coord { x: 2, y: 1 }
    )));

    let portal_creation_step = trace
        .steps
        .iter()
        .find(|step| {
            step.events
                .iter()
                .any(|event| matches!(event, tme_rules::Event::PortalCreated { .. }))
        })
        .expect("portal creation step should exist");
    assert!(
        portal_creation_step
            .after_observed_snapshot
            .realms
            .iter()
            .find(|realm| realm.id == "realm_0")
            .expect("realm_0")
            .levels
            .iter()
            .find(|level| level.id == "workroom")
            .expect("workroom in observed snapshot")
            .tiles
            .iter()
            .any(|tile| {
                tile.position == tme_rules::Coord { x: 2, y: 1 }
                    && tile.transition.as_ref().is_some_and(|transition| {
                        transition.kind == tme_rules::TransitionKindViewV1::Portal
                    })
            }),
        "portal transition should be visible while active"
    );
    let portal_expiration_step = trace
        .steps
        .iter()
        .find(|step| {
            step.events
                .iter()
                .any(|event| matches!(event, tme_rules::Event::PortalExpired { .. }))
        })
        .expect("portal expiration step should exist");
    let expired_tile = portal_expiration_step
        .after_observed_snapshot
        .realms
        .iter()
        .find(|realm| realm.id == "realm_0")
        .expect("realm_0")
        .levels
        .iter()
        .find(|level| level.id == "vault")
        .expect("vault in observed snapshot")
        .tiles
        .iter()
        .find(|tile| tile.position == tme_rules::Coord { x: 1, y: 1 })
        .expect("vault-side portal tile should be present");
    assert_eq!(
        expired_tile.observation,
        tme_rules::TileObservationV1::Visible
    );
    assert!(
        !expired_tile
            .transition
            .as_ref()
            .is_some_and(|transition| transition.kind == tme_rules::TransitionKindViewV1::Portal),
        "portal transition should be absent from the observed tile after expiration"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, tme_rules::Event::SpellCastStubbed { .. })),
        "utility gallery should not emit SpellCastStubbed"
    );
}

#[test]
fn scripted_spell_readiness_matches_golden() {
    let output = tme_sim::run_from_args(vec![
        "tme-sim".to_string(),
        "--scenario".to_string(),
        scenario_path("spell_readiness.json"),
        "--seed".to_string(),
        "7".to_string(),
    ])
    .expect("scripted run succeeds");
    let expected = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("golden")
            .join("spell_readiness_seed_7.txt"),
    )
    .expect("golden should read")
    .replace("\r\n", "\n");

    assert_eq!(output, expected);
}

#[test]
fn trace_v2_golden_deserializes_with_correct_contract_versions() {
    let golden_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/trace_v2_first_room_seed_7.json");
    let json = std::fs::read_to_string(&golden_path).expect("read golden");
    let trace: TraceV2 = serde_json::from_str(&json).expect("deserialize");

    // Header contract versions must match current constants
    assert_eq!(trace.header.contract_version, TRACE_V2_CONTRACT_VERSION);
    assert_eq!(
        trace.header.event_contract_version, EVENT_CONTRACT_VERSION,
        "event contract version mismatch"
    );
    assert_eq!(
        trace.header.snapshot_contract_version, SNAPSHOT_CONTRACT_VERSION,
        "snapshot contract version mismatch"
    );
    assert_eq!(
        trace.header.observed_snapshot_contract_version, OBSERVED_SNAPSHOT_CONTRACT_VERSION,
        "observed snapshot contract version mismatch"
    );
    assert_eq!(
        trace.header.action_context_contract_version, ACTION_CONTEXT_CONTRACT_VERSION,
        "action context contract version mismatch"
    );
    assert_eq!(
        trace.header.intent_contract_version, COMMAND_CONTRACT_VERSION,
        "intent contract version mismatch"
    );
    assert_eq!(
        trace.header.initial_debug_snapshot.contract_version,
        SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.initial_observed_snapshot.contract_version,
        OBSERVED_SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.initial_action_context.contract_version,
        ACTION_CONTEXT_CONTRACT_VERSION
    );
    let debug_combat = &trace.header.initial_debug_snapshot.rules.combat;
    let observed_combat = &trace.header.initial_observed_snapshot.rules.combat;
    assert_eq!(debug_combat, observed_combat);
    assert_eq!(
        debug_combat.tuning_status,
        tme_rules::CombatTuningStatusViewV1::OriginalProvisional
    );
    assert_eq!(debug_combat.hit.base_defender_score, 10);
    assert_eq!(debug_combat.hit.attacker_attack_stat_divisor, 2);
    assert_eq!(debug_combat.hit.attacker_skill_level_divisor, 2);
    assert_eq!(debug_combat.hit.defender_defense_stat_divisor, 2);
    assert_eq!(debug_combat.hit.defender_dexterity_divisor, 3);
    assert_eq!(debug_combat.hit.non_character_defender_dexterity, 10);
    assert_eq!(debug_combat.block.shield_percent_per_point, 10);
    assert_eq!(debug_combat.block.shield_percent_cap, 90);
    assert_eq!(debug_combat.block.armor_percent_per_point, 8);
    assert_eq!(debug_combat.block.armor_percent_cap, 80);
    assert_eq!(debug_combat.block.strength_penetration_percent_per_add, 2);
    assert_eq!(debug_combat.block.armor_encumbrance_percent_per_point, 2);
    assert_eq!(
        debug_combat.block.combat_add_penetration_percent_per_rating,
        2
    );
    assert_eq!(debug_combat.damage.minimum_damage, 1);
    assert_eq!(debug_combat.damage.roll_variation_modulus, 3);
    assert_eq!(debug_combat.damage.moderate_label_min_percent, 20);
    assert_eq!(debug_combat.damage.heavy_label_min_percent, 40);
    assert_eq!(debug_combat.damage.severe_label_min_percent, 70);
    assert_eq!(debug_combat.wounds.near_death_max_percent, 20);
    assert_eq!(debug_combat.wounds.badly_wounded_max_percent, 50);
    assert_eq!(debug_combat.wounds.wounded_max_percent, 99);
    assert_eq!(debug_combat.practice.practice_raw_points, 1);
    assert_eq!(debug_combat.practice.life_and_death_raw_points, 2);
    assert_eq!(debug_combat.practice.overwhelming_raw_points, 1);

    // Every step must have a command and after_observed_snapshot
    for step in &trace.steps {
        assert!(!step.command.actor_id.is_empty());
        assert_eq!(step.command.contract_version, COMMAND_CONTRACT_VERSION);
        assert!(!step.intent_label.is_empty());
        assert_eq!(
            step.after_debug_snapshot.contract_version,
            SNAPSHOT_CONTRACT_VERSION
        );
        assert_eq!(
            step.after_observed_snapshot.contract_version,
            OBSERVED_SNAPSHOT_CONTRACT_VERSION
        );
        assert_eq!(
            step.after_action_context.contract_version,
            ACTION_CONTEXT_CONTRACT_VERSION
        );
        assert!(step.after_observed_snapshot.logical_time.value() > 0);
    }
    assert_eq!(trace.r#final.contract_version, TRACE_V2_CONTRACT_VERSION);
    assert_eq!(
        trace.r#final.final_debug_snapshot.contract_version,
        SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.r#final.final_observed_snapshot.contract_version,
        OBSERVED_SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.r#final.final_action_context.contract_version,
        ACTION_CONTEXT_CONTRACT_VERSION
    );
}

#[test]
fn trace_v2_status_effect_fixture_exposes_effect_contracts() {
    let trace = run_trace_v2("status_effects.json", 7);
    assert_eq!(trace.header.event_contract_version, EVENT_CONTRACT_VERSION);
    assert_eq!(
        trace.header.snapshot_contract_version,
        SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.observed_snapshot_contract_version,
        OBSERVED_SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.action_context_contract_version,
        ACTION_CONTEXT_CONTRACT_VERSION
    );
    let player = trace
        .header
        .initial_debug_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("player actor");
    assert_eq!(player.active_effects.len(), 1);
    assert!(trace.steps.iter().flat_map(|step| &step.events).any(|event| {
        matches!(event, tme_rules::Event::EffectExpired { instance_id, .. } if instance_id == "rooted_1")
    }));
}

#[test]
fn trace_v2_control_poison_protection_fixture_exposes_bs_effects() {
    let trace = run_trace_v2("control_poison_protection.json", 7);

    let mut saw_applied = false;
    let mut saw_resistance_applied = false;
    let mut saw_ticked = false;
    let mut saw_spell_damage = false;
    let mut saw_failed_save = false;
    let mut saw_successful_negate = false;
    let mut saw_action_suppressed = false;

    for event in trace.steps.iter().flat_map(|step| step.events.iter()) {
        match event {
            tme_rules::Event::EffectApplied { .. } => saw_applied = true,
            tme_rules::Event::EffectTicked { .. } => saw_ticked = true,
            tme_rules::Event::SpellDamaged { spell_id, .. } if spell_id == "flame" => {
                saw_spell_damage = true;
            }
            tme_rules::Event::SpellSaveResolved {
                effect_id,
                resistance_tag,
                natural_save_twentieths: 5,
                matching_bonus_twentieths: 3,
                denominator: 20,
                save_twentieths: 8,
                roll: 11,
                success: false,
                mitigation_mode: None,
                requested_damage: Some(3),
                resolved_damage: Some(3),
                ..
            } if effect_id == "flame" && resistance_tag == "fire" => {
                saw_failed_save = true;
            }
            tme_rules::Event::SpellSaveResolved {
                effect_id,
                resistance_tag,
                natural_save_twentieths: 5,
                matching_bonus_twentieths: 0,
                denominator: 20,
                save_twentieths: 5,
                roll: 2,
                success: true,
                mitigation_mode: Some(tme_rules::SpellResistanceMitigationMode::Negate),
                requested_damage: None,
                resolved_damage: None,
                ..
            } if effect_id == "venom" && resistance_tag == "poison" => {
                saw_successful_negate = true;
            }
            tme_rules::Event::ActionSuppressedByStatus { .. } => saw_action_suppressed = true,
            _ => {}
        }

        if matches!(
            event,
            tme_rules::Event::EffectApplied {
                actor_id,
                effect_id,
                kind,
                ..
            } if actor_id == "target" && effect_id == "ember_skin" && kind == "resistance"
        ) {
            saw_resistance_applied = true;
        }
    }

    assert!(saw_applied, "fixture should emit effect_applied");
    assert!(
        saw_resistance_applied,
        "fixture should apply a concrete resistance-family effect"
    );
    assert!(saw_ticked, "fixture should emit effect_ticked");
    assert!(saw_spell_damage, "failed save should preserve spell damage");
    assert!(
        saw_failed_save,
        "fixture should expose the exact failed save"
    );
    assert!(
        saw_successful_negate,
        "fixture should expose the exact successful poison negation"
    );
    assert!(
        saw_action_suppressed,
        "fixture should emit ActionSuppressedByStatus"
    );

    let final_target = trace
        .r#final
        .final_observed_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "target")
        .expect("target should remain visible in final observed snapshot");
    assert!(
        final_target
            .magic_resistance
            .boosts
            .iter()
            .any(|boost| boost.tag == "poison"),
        "target should expose the poison resistance boost in the final observed snapshot"
    );
    assert!(
        trace
            .r#final
            .final_action_context
            .magic_resistance
            .boosts
            .iter()
            .any(|boost| boost.tag == "fire"),
        "player action context should expose the final fire resistance boost"
    );
}

#[test]
fn trace_v2_summon_fixture_exposes_created_creature_lifecycle() {
    let trace = run_trace_v2("summons_created_creature_lifecycle.json", 7);

    assert!(
        !trace
            .steps
            .iter()
            .flat_map(|step| step.events.iter())
            .any(|event| matches!(event, tme_rules::Event::SpellCastStubbed { .. })),
        "summon fixture should not emit SpellCastStubbed"
    );

    let summoned_event = trace
        .steps
        .iter()
        .flat_map(|step| step.events.iter())
        .find_map(|event| match event {
            tme_rules::Event::ActorSummoned {
                actor_id,
                owner_id,
                template_id,
                location,
                ..
            } if actor_id == "summon:call_echo:1:echo_guardian" => {
                Some((owner_id, template_id, location))
            }
            _ => None,
        })
        .expect("trace should include actor_summoned for echo_guardian");
    assert_eq!(summoned_event.0, "player");
    assert_eq!(summoned_event.1, "echo_guardian");
    assert_eq!(summoned_event.2.level, "start");
    assert_eq!(summoned_event.2.position, tme_rules::Coord { x: 2, y: 1 });

    let summon_step = trace
        .steps
        .iter()
        .find(|step| {
            step.events.iter().any(|event| {
                matches!(
                    event,
                    tme_rules::Event::ActorSummoned { actor_id, .. }
                        if actor_id == "summon:call_echo:1:echo_guardian"
                )
            })
        })
        .expect("trace should include a summon step");
    let summoned_actor = summon_step
        .after_debug_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "summon:call_echo:1:echo_guardian")
        .expect("summoned actor should appear in step snapshot");
    assert_eq!(summoned_actor.owner_id.as_deref(), Some("player"));
    let summoned_meta = summoned_actor
        .summoned
        .as_ref()
        .expect("summoned actor should expose summon metadata");
    assert_eq!(summoned_meta.template_id, "echo_guardian");
    let summoned_actor_observed = summon_step
        .after_observed_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "summon:call_echo:1:echo_guardian")
        .expect("summoned actor should appear in observed snapshot after summon");
    assert_eq!(summoned_actor_observed.owner_id.as_deref(), Some("player"));
    let summoned_meta_observed = summoned_actor_observed
        .summoned
        .as_ref()
        .expect("summoned actor should expose summon metadata in observed snapshot");
    assert_eq!(summoned_meta_observed.template_id, "echo_guardian");

    assert!(trace.steps.iter().flat_map(|step| step.events.iter()).any(
        |event| matches!(
            event,
            tme_rules::Event::SummonExpired {
                actor_id,
                template_id,
                ..
            } if actor_id == "summon:call_echo:1:echo_guardian" && template_id == "echo_guardian"
        )
    ));
    assert!(
        !trace
            .r#final
            .final_debug_snapshot
            .actors
            .iter()
            .any(|actor| actor.id == "summon:call_echo:1:echo_guardian"),
        "expired summon should be absent from final snapshot"
    );
    assert!(
        !trace
            .r#final
            .final_observed_snapshot
            .actors
            .iter()
            .any(|actor| actor.id == "summon:call_echo:1:echo_guardian"),
        "expired summon should be absent from final observed snapshot"
    );
}

#[test]
fn trace_v2_profession_actions_fixture_exposes_bx_events_and_contract() {
    let thief_trace = run_trace_v2("profession_specific_actions.json", 7);
    let martial_trace = run_trace_v2("martial_hand_block_actions.json", 7);
    let knight_trace = run_trace_v2("knight_support_actions.json", 7);

    assert_eq!(
        thief_trace.header.intent_contract_version,
        COMMAND_CONTRACT_VERSION
    );
    assert_eq!(
        martial_trace.header.intent_contract_version,
        COMMAND_CONTRACT_VERSION
    );
    assert_eq!(
        knight_trace.header.intent_contract_version,
        COMMAND_CONTRACT_VERSION
    );

    let event_names: std::collections::BTreeSet<_> = thief_trace
        .steps
        .iter()
        .chain(martial_trace.steps.iter())
        .chain(knight_trace.steps.iter())
        .flat_map(|step| step.events.iter())
        .filter_map(|event| {
            let value = serde_json::to_value(event).expect("event serializes");
            value
                .as_object()
                .and_then(|object| object.keys().next())
                .cloned()
        })
        .collect();

    assert!(event_names.contains("actor_hidden"));
    assert!(event_names.contains("hide_broken"));
    assert!(event_names.contains("attack_blocked"));
    assert!(event_names.contains("item_enchanted"));
    assert!(event_names.contains("effect_applied"));
    assert!(event_names.contains("effect_removed"));
    assert!(event_names.contains("tile_effect_applied"));
    assert!(!event_names.contains("spell_cast_stubbed"));

    let hide_step = thief_trace
        .steps
        .iter()
        .find(|step| step.intent_label == "hide")
        .expect("hide step");
    assert!(matches!(
        hide_step.command.intent,
        tme_rules::PlayerIntentPayloadV1::Hide
    ));
    assert!(
        hide_step
            .after_observed_snapshot
            .actors
            .iter()
            .find(|actor| actor.id == "player0" || actor.id == "player")
            .expect("player observed")
            .active_effects
            .iter()
            .any(|effect| effect.kind == "hidden")
    );
}

#[test]
fn trace_v2_monster_ability_fixture_exposes_by_events() {
    let trace = run_trace_v2("monster_spellcasting_special_attacks.json", 7);

    let mut saw_monster_intent = false;
    let mut saw_spell_damage = false;
    let mut saw_poison_resisted = false;
    let mut saw_effect_tick = false;

    for event in trace.steps.iter().flat_map(|step| step.events.iter()) {
        match event {
            tme_rules::Event::AutomaticActorDecision {
                actor_id,
                decision: tme_rules::AutomaticActorDecisionV1::UseAbility { spell_name, .. },
                ..
            } if actor_id == "ember_imp" && spell_name == "Ember Spit" => {
                saw_monster_intent = true;
            }
            tme_rules::Event::SpellDamaged {
                caster_id,
                spell_id,
                target_id,
                damage_kind,
                damage,
                ..
            } if caster_id == "ember_imp"
                && spell_id == "ember_spit"
                && target_id == "player"
                && damage_kind.as_deref() == Some("fire")
                && *damage > 0 =>
            {
                saw_spell_damage = true;
            }
            tme_rules::Event::SpellSaveResolved {
                actor_id,
                effect_id,
                resistance_tag,
                ..
            } if actor_id == "player"
                && effect_id == "venom_bite"
                && resistance_tag == "poison" =>
            {
                saw_poison_resisted = true;
            }
            tme_rules::Event::EffectTicked {
                actor_id,
                effect_id,
                kind,
                ..
            } if actor_id == "player" && effect_id == "poison_ward" && kind == "protection" => {
                saw_effect_tick = true;
            }
            _ => {}
        }
    }

    assert!(
        saw_monster_intent,
        "fixture should emit monster ability intent"
    );
    assert!(saw_spell_damage, "fixture should emit monster spell damage");
    assert!(
        saw_poison_resisted,
        "fixture should emit poison resistance mitigation"
    );
    assert!(
        saw_effect_tick,
        "fixture should tick the seeded protection effect"
    );
}

#[test]
fn trace_v2_remaining_spell_effect_families_exposes_all_dy_routes() {
    let trace = run_trace_v2("remaining_spell_effect_families.json", 7);

    assert_eq!(trace.header.contract_version, TRACE_V2_CONTRACT_VERSION);
    assert_eq!(trace.header.event_contract_version, EVENT_CONTRACT_VERSION);
    assert_eq!(
        trace.header.snapshot_contract_version,
        SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.observed_snapshot_contract_version,
        OBSERVED_SNAPSHOT_CONTRACT_VERSION
    );

    let mut cast_spell_ids = trace
        .steps
        .iter()
        .filter_map(|step| match &step.command.intent {
            tme_rules::PlayerIntentPayloadV1::CastSpell { spell_id, .. } => Some(spell_id.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    cast_spell_ids.sort();
    cast_spell_ids.dedup();
    assert_eq!(
        cast_spell_ids,
        vec![
            "banish",
            "breathe_water",
            "call_demon",
            "death",
            "feather_fall",
            "hide_door",
            "hide_in_shadows",
            "night_vision",
            "raise_dead",
            "sense_secret",
            "shadow_cloud",
            "speed",
            "turn_undead",
        ]
    );

    let events = trace
        .steps
        .iter()
        .flat_map(|step| step.events.iter())
        .collect::<Vec<_>>();
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, tme_rules::Event::SpellCastStubbed { .. })),
        "remaining-family gallery should not emit SpellCastStubbed"
    );
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::ActorDefeated { actor_id, .. } if actor_id == "fallen_ally"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::RaiseDeadEvaluated { spell_id, .. } if spell_id == "raise_dead"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::TileEffectApplied { effect_id, .. }
            if effect_id == "shadow_cloud"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::ActorSummoned { actor_id, .. }
            if actor_id == "summon:call_demon:1:bound_demon"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::BanishEvaluated {
            target_id,
            owned_by_caster: false,
            ..
        } if target_id == "foreign_demon"
    )));
    assert!(
        !events.iter().any(|event| matches!(
            event,
            tme_rules::Event::ActorBanished { actor_id, .. }
                if actor_id == "summon:call_demon:1:bound_demon"
        )),
        "the caster's owned summon remains an invalid hostile target"
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, tme_rules::Event::TransitionConcealed { .. }))
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, tme_rules::Event::TransitionConcealmentRemoved { .. }))
    );
    for effect_id in ["night_vision", "feather_fall", "speed", "breathe_water"] {
        assert!(events.iter().any(|event| matches!(
            event,
            tme_rules::Event::EffectApplied { effect_id: applied, .. }
                if applied == effect_id
        )));
    }
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::TurnUndeadResolved { moved_actor_ids, .. }
            if moved_actor_ids == &vec!["mobile_undead".to_string()]
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::ActorHidden { actor_id, .. } if actor_id == "player"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::HideBroken { actor_id, .. } if actor_id == "player"
    )));

    let final_actors = &trace.r#final.final_debug_snapshot.actors;
    assert!(
        final_actors
            .iter()
            .any(|actor| actor.id == "summon:call_demon:1:bound_demon"),
        "owned summon must survive because no hostile command may target it"
    );
    let fallen = final_actors
        .iter()
        .find(|actor| actor.id == "fallen_ally")
        .expect("fallen ally remains in the final debug snapshot");
    assert!(matches!(
        &fallen.life_state,
        tme_rules::ActorLifeStateViewV1::Dead
    ));
    let player = final_actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("player remains in final state");
    assert_eq!(player.location.position, tme_rules::Coord { x: 3, y: 1 });
}

#[test]
fn trace_v2_magic_profession_gallery_exposes_dx_contract() {
    let trace = run_trace_v2("magic_profession_gallery.json", 7);

    assert_eq!(trace.header.contract_version, TRACE_V2_CONTRACT_VERSION);
    assert_eq!(trace.header.event_contract_version, EVENT_CONTRACT_VERSION);
    assert_eq!(
        trace.header.snapshot_contract_version,
        SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.observed_snapshot_contract_version,
        OBSERVED_SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.action_context_contract_version,
        ACTION_CONTEXT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.intent_contract_version,
        COMMAND_CONTRACT_VERSION
    );

    let initial_player = trace
        .header
        .initial_debug_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player0")
        .expect("initial player");
    let initial_character = initial_player.character.as_ref().expect("character sheet");
    assert_eq!(initial_character.resources.mp, 40);
    assert_eq!(initial_player.carried.gold.sack, 20);
    assert!(
        !initial_character
            .known_spells
            .iter()
            .any(|spell| spell.spell_id == "shadow_sting"),
        "fixture should learn shadow_sting during the script"
    );

    let mut saw_self_protection_cast = false;
    let mut saw_coordinate_cast = false;
    let mut saw_actor_damage_cast = false;
    let mut saw_actor_curse_cast = false;
    let mut saw_area_overlay_cast = false;
    let mut saw_self_control_cast = false;
    let mut saw_learn_command = false;
    let mut saw_hide_command = false;
    let mut saw_move_path_command = false;

    for step in &trace.steps {
        match &step.command.intent {
            tme_rules::PlayerIntentPayloadV1::CastSpell {
                spell_id,
                target: Some(tme_rules::SpellTarget::SelfTarget),
                ..
            } if spell_id == "toxin_ward" => saw_self_protection_cast = true,
            tme_rules::PlayerIntentPayloadV1::CastSpell {
                spell_id,
                target: Some(tme_rules::SpellTarget::Coordinate { position }),
                ..
            } if spell_id == "shadow_veil"
                && position.level == "room_0"
                && position.position == (tme_rules::Coord { x: 1, y: 2 }) =>
            {
                saw_coordinate_cast = true;
            }
            tme_rules::PlayerIntentPayloadV1::CastSpell {
                spell_id,
                target: Some(tme_rules::SpellTarget::Actor { actor_id }),
                ..
            } if spell_id == "shadow_sting" && actor_id == "target_dummy" => {
                saw_actor_damage_cast = true;
            }
            tme_rules::PlayerIntentPayloadV1::CastSpell {
                spell_id,
                target: Some(tme_rules::SpellTarget::Actor { actor_id }),
                ..
            } if spell_id == "dimming_hex" && actor_id == "target_dummy" => {
                saw_actor_curse_cast = true;
            }
            tme_rules::PlayerIntentPayloadV1::CastSpell {
                spell_id,
                target: Some(tme_rules::SpellTarget::Area { center }),
                ..
            } if spell_id == "web_field"
                && center.level == "room_0"
                && center.position == (tme_rules::Coord { x: 2, y: 2 }) =>
            {
                saw_area_overlay_cast = true;
            }
            tme_rules::PlayerIntentPayloadV1::CastSpell {
                spell_id,
                target: Some(tme_rules::SpellTarget::SelfTarget),
                ..
            } if spell_id == "self_hold" => saw_self_control_cast = true,
            tme_rules::PlayerIntentPayloadV1::LearnSpell { spell_id }
                if spell_id == "shadow_sting" =>
            {
                saw_learn_command = true;
            }
            tme_rules::PlayerIntentPayloadV1::Hide => saw_hide_command = true,
            tme_rules::PlayerIntentPayloadV1::MovePath { path }
                if path == &[tme_rules::Direction::East] =>
            {
                saw_move_path_command = true;
            }
            _ => {}
        }
    }

    assert!(
        saw_self_protection_cast,
        "trace should expose self protection cast"
    );
    assert!(saw_coordinate_cast, "trace should expose coordinate cast");
    assert!(
        saw_actor_damage_cast,
        "trace should expose actor-targeted damage cast"
    );
    assert!(
        saw_actor_curse_cast,
        "trace should expose actor-targeted Curse cast"
    );
    assert!(
        saw_area_overlay_cast,
        "trace should expose area-targeted overlay cast"
    );
    assert!(
        saw_self_control_cast,
        "trace should expose self control cast"
    );
    assert!(saw_learn_command, "trace should expose learn_spell command");
    assert!(saw_hide_command, "trace should expose hide command");
    assert!(
        saw_move_path_command,
        "trace should expose move_path command"
    );

    let mut saw_learning_book_retained = false;
    let mut saw_spell_learned = false;
    let mut saw_gold_spent = false;
    let mut saw_spell_damage = false;
    let mut saw_magic_practice_receipt = false;
    let mut saw_skill_practice = false;
    let mut saw_effect_applied = false;
    let mut saw_curse_applied = false;
    let mut saw_control_applied = false;
    let mut saw_effect_ticked = false;
    let mut saw_effect_expired = false;
    let mut saw_spell_save = false;
    let mut saw_action_suppressed = false;
    let mut saw_tile_effect_applied = false;
    let mut saw_tile_effect_expired = false;
    let mut saw_actor_hidden = false;
    let mut saw_hide_broken = false;
    let mut saw_monster_intent = false;
    let mut saw_web_movement_cost = false;

    for event in trace.steps.iter().flat_map(|step| step.events.iter()) {
        match event {
            tme_rules::Event::SpellLearned {
                actor_id,
                spell_id,
                lane,
                gold_cost,
                spell_book_item_instance_id,
                spell_book_item_definition_id,
                ..
            } if actor_id == "player0"
                && spell_id == "shadow_sting"
                && lane == "thief_magic"
                && *gold_cost == 10
                && spell_book_item_instance_id == "spell_book"
                && spell_book_item_definition_id == "spell_book" =>
            {
                saw_spell_learned = true;
                saw_learning_book_retained = true;
            }
            tme_rules::Event::GoldChanged {
                actor_id,
                amount,
                new_total,
                ..
            } if actor_id == "player0" && *amount == -10 && *new_total == 10 => {
                saw_gold_spent = true;
            }
            tme_rules::Event::SpellDamaged {
                caster_id,
                spell_id,
                target_id,
                damage_kind,
                damage,
                ..
            } if caster_id == "player0"
                && spell_id == "shadow_sting"
                && target_id == "target_dummy"
                && damage_kind.as_deref() == Some("shadow")
                && *damage == 3 =>
            {
                saw_spell_damage = true;
            }
            tme_rules::Event::SkillPracticeAwarded {
                actor_id,
                track_id,
                raw_amount,
                learning_rate,
                credited_amount,
                ..
            } if actor_id == "player0"
                && track_id == "thief_magic"
                && *raw_amount == 3
                && *learning_rate == 1
                && *credited_amount == 3 =>
            {
                saw_skill_practice = true;
            }
            tme_rules::Event::MagicPracticeEvaluated {
                actor_id,
                current_class_id,
                spell_id,
                track_id,
                mp_cost,
                primary_attribute: Some(tme_rules::MagicPrimaryAttribute::Intelligence),
                primary_attribute_value: Some(11),
                base_raw_points,
                primary_attribute_bonus_raw_points,
                total_raw_points,
                risk_applied,
                reason,
                ..
            } if actor_id == "player0"
                && current_class_id == "thief"
                && spell_id == "shadow_sting"
                && track_id == "thief_magic"
                && *mp_cost == 2
                && *base_raw_points == 2
                && *primary_attribute_bonus_raw_points == 1
                && *total_raw_points == 3
                && !risk_applied
                && reason == "eligible_successful_cast" =>
            {
                saw_magic_practice_receipt = true;
            }
            tme_rules::Event::EffectApplied {
                actor_id,
                effect_id,
                kind,
                ..
            } if actor_id == "player0" && effect_id == "toxin_ward" && kind == "protection" => {
                saw_effect_applied = true;
            }
            tme_rules::Event::EffectApplied {
                actor_id,
                effect_id,
                kind,
                ..
            } if actor_id == "target_dummy" && effect_id == "dimming_hex" && kind == "curse" => {
                saw_curse_applied = true;
            }
            tme_rules::Event::EffectApplied {
                actor_id,
                effect_id,
                kind,
                ..
            } if actor_id == "player0" && effect_id == "self_hold" && kind == "control_status" => {
                saw_control_applied = true;
            }
            tme_rules::Event::EffectTicked {
                actor_id,
                effect_id,
                kind,
                ..
            } if actor_id == "player0" && effect_id == "toxin_ward" && kind == "protection" => {
                saw_effect_ticked = true;
            }
            tme_rules::Event::EffectExpired {
                actor_id,
                effect_id,
                kind,
                ..
            } if actor_id == "player0" && effect_id == "toxin_ward" && kind == "protection" => {
                saw_effect_expired = true;
            }
            tme_rules::Event::SpellSaveResolved {
                actor_id,
                effect_id,
                resistance_tag,
                ..
            } if actor_id == "player0"
                && effect_id == "venom_bite"
                && resistance_tag == "poison" =>
            {
                saw_spell_save = true;
            }
            tme_rules::Event::ActionSuppressedByStatus {
                actor_id,
                intent,
                effect_id,
                kind,
                ..
            } if actor_id == "player0"
                && intent == "walk east"
                && effect_id == "self_hold"
                && kind == "control_status" =>
            {
                saw_action_suppressed = true;
            }
            tme_rules::Event::TileEffectApplied {
                effect_id,
                location,
                sight,
                move_cost,
                ..
            } if effect_id == "web_field"
                && location.position == (tme_rules::Coord { x: 2, y: 2 })
                && sight.as_deref() == Some("obscured")
                && *move_cost == Some(2) =>
            {
                saw_tile_effect_applied = true;
            }
            tme_rules::Event::TileEffectExpired { effect_id, .. } if effect_id == "web_field" => {
                saw_tile_effect_expired = true;
            }
            tme_rules::Event::ActorHidden {
                actor_id,
                effect_id,
                ..
            } if actor_id == "player0" && effect_id == "hidden" => {
                saw_actor_hidden = true;
            }
            tme_rules::Event::HideBroken {
                actor_id,
                effect_id,
                reason,
                ..
            } if actor_id == "player0" && effect_id == "hidden" && reason == "active_item_move" => {
                saw_hide_broken = true;
            }
            tme_rules::Event::AutomaticActorDecision {
                actor_id,
                decision: tme_rules::AutomaticActorDecisionV1::UseAbility { spell_name, .. },
                ..
            } if actor_id == "viperling" && spell_name == "Venom Bite" => {
                saw_monster_intent = true;
            }
            tme_rules::Event::MovementCostPaid {
                actor_id,
                terrain,
                cost,
                destination,
                ..
            } if actor_id == "player0"
                && terrain.contains("web_field")
                && *cost == 2
                && destination.position == (tme_rules::Coord { x: 2, y: 2 }) =>
            {
                saw_web_movement_cost = true;
            }
            _ => {}
        }
    }

    assert!(
        !trace
            .steps
            .iter()
            .flat_map(|step| step.events.iter())
            .any(|event| matches!(event, tme_rules::Event::SpellCastStubbed { .. })),
        "gallery should not emit SpellCastStubbed"
    );
    assert!(
        saw_learning_book_retained,
        "gallery should retain and identify the exact Spell Book"
    );
    assert!(saw_spell_learned, "gallery should emit SpellLearned");
    assert!(saw_gold_spent, "gallery should emit GoldChanged");
    assert!(
        saw_spell_damage,
        "gallery should emit targeted spell damage"
    );
    assert!(
        saw_magic_practice_receipt,
        "gallery should emit the exact magic practice calculation receipt"
    );
    assert!(
        saw_skill_practice,
        "gallery should emit casting skill practice"
    );
    assert!(saw_effect_applied, "gallery should apply protection effect");
    assert!(
        saw_curse_applied,
        "gallery should apply original Curse effect"
    );
    assert!(saw_control_applied, "gallery should apply control effect");
    assert!(saw_effect_ticked, "gallery should tick active effects");
    assert!(saw_effect_expired, "gallery should expire active effects");
    assert!(saw_spell_save, "gallery should emit a spell-save receipt");
    assert!(saw_action_suppressed, "gallery should suppress an action");
    assert!(saw_tile_effect_applied, "gallery should apply tile overlay");
    assert!(
        saw_tile_effect_expired,
        "gallery should expire tile overlay"
    );
    assert!(saw_actor_hidden, "gallery should emit ActorHidden");
    assert!(saw_hide_broken, "gallery should emit HideBroken");
    assert!(
        saw_monster_intent,
        "gallery should emit monster ability intent"
    );
    assert!(
        saw_web_movement_cost,
        "gallery should charge web overlay movement cost"
    );

    let final_player = trace
        .r#final
        .final_debug_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player0")
        .expect("final player");
    let final_character = final_player.character.as_ref().expect("character sheet");
    assert!(
        final_character.resources.mp < initial_character.resources.mp,
        "spell casting should spend MP across the scenario"
    );
    assert_eq!(final_player.carried.gold.sack, 10);
    assert!(
        final_character
            .known_spells
            .iter()
            .any(|spell| spell.spell_id == "shadow_sting" && spell.lane == "thief_magic"),
        "learned spell should appear in final snapshot"
    );
}

#[test]
fn trace_v2_town_adventure_loop_gallery_closes_end_to_end_state() {
    let trace = run_trace_v2("town_adventure_loop_gallery.json", 7);
    let repeated = run_trace_v2("town_adventure_loop_gallery.json", 7);
    assert_eq!(
        serde_json::to_string(&trace).expect("first EH trace should serialize"),
        serde_json::to_string(&repeated).expect("repeated EH trace should serialize"),
        "the integrated town/adventure loop must be byte-for-byte deterministic"
    );

    assert_eq!(trace.header.scenario_id, "town_adventure_loop_gallery");
    assert_eq!(trace.header.seed, 7);
    assert_eq!(
        (
            trace.header.contract_version,
            trace.header.event_contract_version,
            trace.header.snapshot_contract_version,
            trace.header.observed_snapshot_contract_version,
            trace.header.action_context_contract_version,
            trace.header.intent_contract_version,
        ),
        (2, 40, 30, 29, 31, 26)
    );
    assert_eq!(trace.r#final.contract_version, 2);
    assert_eq!(trace.steps.len(), 27);
    assert!(
        trace.steps.iter().enumerate().all(|(index, step)| {
            step.step_index == index && step.command.contract_version == 26
        })
    );

    let commands: Vec<Value> = trace
        .steps
        .iter()
        .map(|step| serde_json::to_value(&step.command.intent).expect("command should serialize"))
        .collect();
    assert_eq!(
        commands,
        vec![
            json!({"physical_attack":{"mode":"fight","target_actor_id":"road_scavenger","authorization":"safe"}}),
            json!({"search_corpse":{"corpse_id":"corpse:1"}}),
            json!({"move_item":{"item_instance_id":"waystone_token","destination":{"kind":"carried","position":"sack_item_2"}}}),
            json!({"move_item":{"item_instance_id":"trade_charm","destination":{"kind":"carried","position":"sack_item_3"}}}),
            json!({"move_gold":{"source":{"kind":"ground","gold_pile_id":"gold:1"},"destination":{"kind":"carried","position":"sack"},"quantity":{"kind":"all"}}}),
            json!({"move_path":{"path":["east"]}}),
            json!({"traverse":{"kind":"stairs_up"}}),
            json!({"interact_with_npc":{"npc_actor_id":"route_keeper","interaction_id":"ask_about_waystone","item_instance_id":null}}),
            json!({"interact_with_npc":{"npc_actor_id":"route_keeper","interaction_id":"return_waystone","item_instance_id":"waystone_token"}}),
            json!({"use_item_service":{"service_id":"waystation_counter","capability_id":"appraisal","operation":"appraise","item_instance_id":"trade_charm"}}),
            json!({"sell_to_merchant":{"service_id":"waystation_counter","capability_id":"trail_wares","item_instance_id":"trade_charm"}}),
            json!({"move_gold":{"source":{"kind":"carried","position":"sack"},"destination":{"kind":"ground_here"},"quantity":{"kind":"exact","amount":15}}}),
            json!({"deposit_bank_gold":{"service_id":"waystation_counter","capability_id":"bank_access","gold_pile_id":"gold:2"}}),
            json!({"deposit_locker_item":{"service_id":"waystation_counter","capability_id":"locker_access","item_instance_id":"weathered_staff"}}),
            json!({"move_item":{"item_instance_id":"field_spell_book","destination":{"kind":"carried","position":"right_hand"}}}),
            json!({"critique":{"service_id":"waystation_counter","track_id":"wizard_magic"}}),
            json!({"train":{"service_id":"waystation_counter","offered_gold":7}}),
            json!({"learn_spell":{"spell_id":"ember_bolt"}}),
            json!({"move_item":{"item_instance_id":"field_spell_book","destination":{"kind":"carried","position":"sack_item_1"}}}),
            json!({"buy_from_merchant":{"service_id":"waystation_counter","capability_id":"trail_wares","item_instance_ids":["bright_staff_stock"]}}),
            json!({"move_item":{"item_instance_id":"bright_staff_stock","destination":{"kind":"carried","position":"right_hand"}}}),
            json!({"use_restoration_service":{"service_id":"waystation_counter","capability_id":"restoration","operation_id":"restore_hit_points","item_instance_id":null,"corpse_id":null}}),
            json!({"use_restoration_service":{"service_id":"waystation_counter","capability_id":"restoration","operation_id":"restore_magic_points","item_instance_id":null,"corpse_id":null}}),
            json!({"traverse":{"kind":"stairs_down"}}),
            json!({"physical_attack":{"mode":"fight","target_actor_id":"return_sentinel","authorization":"safe"}}),
            json!({"cast_spell":{"spell_id":"ember_bolt","target":{"actor":{"actor_id":"return_sentinel"}},"authorization":"safe"}}),
            json!("inspect"),
        ]
    );

    let first_events = &trace.steps[0].events;
    let defeated = first_events
        .iter()
        .position(|event| {
            matches!(event, tme_rules::Event::ActorDefeated {
            actor_id, credited_actor_id: Some(credited), ..
        } if actor_id == "road_scavenger" && credited == "player")
        })
        .expect("step 1 should defeat the road scavenger");
    let corpse_created = first_events
        .iter()
        .position(|event| matches!(event, tme_rules::Event::CorpseCreated {
            corpse_id, origin_actor_id, sequence, ..
        } if corpse_id.as_str() == "corpse:1" && origin_actor_id == "road_scavenger" && *sequence == 1))
        .expect("step 1 should create corpse:1");
    let retained_item = first_events
        .iter()
        .position(|event| {
            matches!(event, tme_rules::Event::ItemRelocated {
            item_instance_id,
            to: tme_rules::ItemLocationViewV1::Corpse { corpse_id, .. },
            reason: tme_rules::ItemRelocationReason::CorpseRetention,
            ..
        } if item_instance_id == "waystone_token" && corpse_id.as_str() == "corpse:1")
        })
        .expect("step 1 should retain the quest item in corpse:1");
    let retained_gold = first_events
        .iter()
        .position(|event| {
            matches!(event, tme_rules::Event::GoldRelocated {
            amount,
            to: tme_rules::GoldLocationViewV1::Corpse { corpse_id },
            reason: tme_rules::GoldRelocationReason::CorpseRetention,
            ..
        } if *amount == 20 && corpse_id.as_str() == "corpse:1")
        })
        .expect("step 1 should retain corpse gold");
    assert!(
        defeated < corpse_created
            && corpse_created < retained_item
            && retained_item < retained_gold
    );

    let search_events = &trace.steps[1].events;
    let released_item = search_events
        .iter()
        .position(|event| {
            matches!(event, tme_rules::Event::ItemRelocated {
            item_instance_id,
            reason: tme_rules::ItemRelocationReason::CorpseSearch,
            ..
        } if item_instance_id == "waystone_token")
        })
        .expect("step 2 should release the quest item");
    let released_gold = search_events
        .iter()
        .position(|event| {
            matches!(event, tme_rules::Event::GoldRelocated {
            amount,
            to: tme_rules::GoldLocationViewV1::Ground { gold_pile_id, .. },
            reason: tme_rules::GoldRelocationReason::CorpseSearch,
            ..
        } if *amount == 20 && gold_pile_id.as_str() == "gold:1")
        })
        .expect("step 2 should release gold:1");
    let searched = search_events
        .iter()
        .position(|event| {
            matches!(event, tme_rules::Event::CorpseSearched {
            corpse_id, items_released, gold_released, ..
        } if corpse_id.as_str() == "corpse:1" && *items_released == 2 && *gold_released == 20)
        })
        .expect("step 2 should search corpse:1");
    assert!(released_item < released_gold && released_gold < searched);
    assert!(trace.steps[2].events.iter().any(|event| matches!(event,
        tme_rules::Event::ItemRelocated {
            item_instance_id,
            to: tme_rules::ItemLocationViewV1::Carried { actor_id, position: tme_rules::CarriedPosition::SackItem2 },
            reason: tme_rules::ItemRelocationReason::PlayerMove,
            ..
        } if item_instance_id == "waystone_token" && actor_id == "player"
    )));
    assert!(trace.steps[3].events.iter().any(|event| matches!(event,
        tme_rules::Event::ItemRelocated {
            item_instance_id,
            to: tme_rules::ItemLocationViewV1::Carried { actor_id, position: tme_rules::CarriedPosition::SackItem3 },
            reason: tme_rules::ItemRelocationReason::PlayerMove,
            ..
        } if item_instance_id == "trade_charm" && actor_id == "player"
    )));
    assert!(trace.steps[4].events.iter().any(|event| matches!(event,
        tme_rules::Event::GoldRelocated {
            amount,
            from: tme_rules::GoldLocationViewV1::Ground { gold_pile_id, .. },
            to: tme_rules::GoldLocationViewV1::Carried { actor_id, position: tme_rules::CarriedGoldPosition::Sack },
            reason: tme_rules::GoldRelocationReason::PlayerMove,
            ..
        } if *amount == 20 && gold_pile_id.as_str() == "gold:1" && actor_id == "player"
    )));

    assert!(trace.steps[7].events.iter().any(|event| matches!(event,
        tme_rules::Event::QuestStateChanged {
            quest_id, before_stage_id: None, after_stage_id, ..
        } if quest_id == "waystone_recovery" && after_stage_id == "awaiting_waystone"
    )));
    assert!(trace.steps[8].events.iter().any(|event| matches!(event,
        tme_rules::Event::QuestStateChanged {
            quest_id, before_stage_id: Some(before), after_stage_id, ..
        } if quest_id == "waystone_recovery" && before == "awaiting_waystone" && after_stage_id == "completed"
    )));
    assert!(trace.steps[8].events.iter().any(|event| matches!(event,
        tme_rules::Event::TransactionCommitted { costs, .. }
            if costs.iter().any(|cost| matches!(cost,
                tme_rules::TransactionCostReceiptV1::SelectedCarriedItem {
                    item_instance_id, item_definition_id, consumed_quantity, remaining_quantity
                } if item_instance_id == "waystone_token"
                    && item_definition_id == "waystone_token"
                    && *consumed_quantity == 1
                    && *remaining_quantity == 0
            ))
    )));
    assert!(trace.steps[9].events.iter().any(|event| matches!(event,
        tme_rules::Event::ItemAppraised {
            item_instance_id, unit_value_gold, total_value_gold, ..
        } if item_instance_id == "trade_charm" && *unit_value_gold == 8 && *total_value_gold == 8
    )));
    assert!(trace.steps[10].events.iter().any(|event| matches!(event,
        tme_rules::Event::ItemRelocated {
            item_instance_id,
            to: tme_rules::ItemLocationViewV1::Merchant { service_id, capability_id },
            reason: tme_rules::ItemRelocationReason::MerchantSale,
            ..
        } if item_instance_id == "trade_charm"
            && service_id == "waystation_counter"
            && capability_id == "trail_wares"
    )));
    assert!(trace.steps[10].events.iter().any(|event| matches!(event,
        tme_rules::Event::GoldChanged { actor_id, amount, .. }
            if actor_id == "player" && *amount == 8
    )));
    assert!(trace.steps[12].events.iter().any(|event| matches!(event,
        tme_rules::Event::BankBalanceChanged {
            actor_id, bank_id, amount, before, after, ..
        } if actor_id == "player"
            && bank_id == "waystation_bank"
            && *amount == 15
            && *before == 0
            && *after == 15
    )));
    assert!(trace.steps[13].events.iter().any(|event| matches!(event,
        tme_rules::Event::ItemRelocated {
            item_instance_id,
            to: tme_rules::ItemLocationViewV1::Locker { vault_id, owner_character_id },
            reason: tme_rules::ItemRelocationReason::LockerDeposit,
            ..
        } if item_instance_id == "weathered_staff"
            && vault_id == "waystation_vault"
            && owner_character_id.as_str() == "character:town_adventure_loop_gallery:primary"
    )));
    assert!(trace.steps[15].events.iter().any(|event| matches!(event,
        tme_rules::Event::SkillCritiqued { actor_id, service_id, track_id, .. }
            if actor_id == "player" && service_id == "waystation_counter" && track_id == "wizard_magic"
    )));
    assert!(trace.steps[16].events.iter().any(|event| matches!(event,
        tme_rules::Event::TrainingPurchased {
            actor_id, track_id, spent_gold, previous_learning_rate, new_learning_rate, ..
        } if actor_id == "player"
            && track_id == "wizard_magic"
            && *spent_gold == 7
            && *previous_learning_rate == 1
            && *new_learning_rate == 2
    )));
    assert!(trace.steps[17].events.iter().any(|event| matches!(event,
        tme_rules::Event::SpellLearned {
            actor_id, spell_id, lane, gold_cost, spell_book_item_instance_id, ..
        } if actor_id == "player"
            && spell_id == "ember_bolt"
            && lane == "wizard_magic"
            && *gold_cost == 25
            && spell_book_item_instance_id == "field_spell_book"
    )));
    assert!(trace.steps[19].events.iter().any(|event| matches!(event,
        tme_rules::Event::ItemRelocated {
            item_instance_id,
            from: tme_rules::ItemLocationViewV1::Merchant { service_id, capability_id },
            reason: tme_rules::ItemRelocationReason::MerchantPurchase,
            ..
        } if item_instance_id == "bright_staff_stock"
            && service_id == "waystation_counter"
            && capability_id == "trail_wares"
    )));
    assert!(trace.steps[20].events.iter().any(|event| matches!(event,
        tme_rules::Event::ItemRelocated {
            item_instance_id,
            to: tme_rules::ItemLocationViewV1::Carried { actor_id, position: tme_rules::CarriedPosition::RightHand },
            reason: tme_rules::ItemRelocationReason::PlayerMove,
            ..
        } if item_instance_id == "bright_staff_stock" && actor_id == "player"
    )));
    assert!(trace.steps[21].events.iter().any(|event| matches!(event,
        tme_rules::Event::ResourceRestored {
            actor_id, resource: tme_rules::ResourceKind::Hp, before, after, maximum, ..
        } if actor_id == "player" && *before == 21 && *after == 40 && *maximum == 40
    )));
    assert!(trace.steps[22].events.iter().any(|event| matches!(event,
        tme_rules::Event::ResourceRestored {
            actor_id, resource: tme_rules::ResourceKind::Mp, before, after, maximum, ..
        } if actor_id == "player" && *before == 11 && *after == 40 && *maximum == 40
    )));
    assert!(trace.steps[24].events.iter().any(|event| matches!(event,
        tme_rules::Event::Attacked {
            attacker_id, defender_id, mode: tme_rules::PhysicalAttackMode::Fight, damage, defender_hp, ..
        } if attacker_id == "player" && defender_id == "return_sentinel" && *damage == 44 && *defender_hp == 56
    )));
    assert!(trace.steps[25].events.iter().any(|event| matches!(event,
        tme_rules::Event::SpellDamaged {
            caster_id, spell_id, target_id, damage, hp, ..
        } if caster_id == "player"
            && spell_id == "ember_bolt"
            && target_id == "return_sentinel"
            && *damage == 3
            && *hp == 53
    )));

    let arrival_context = &trace.steps[6].after_action_context;
    assert_eq!(arrival_context.position.level, "waystation");
    assert_eq!(
        arrival_context.position.position,
        tme_rules::Coord { x: 2, y: 1 }
    );
    assert_eq!(arrival_context.services_here.len(), 1);
    let arrival_service = &arrival_context.services_here[0];
    assert_eq!(arrival_service.service_id, "waystation_counter");
    assert_eq!(arrival_service.capabilities.len(), 8);
    assert!(matches!(&arrival_service.capabilities[0],
        tme_rules::ServiceCapabilityViewV1::SkillTraining { capability_id, offered_track_ids, .. }
            if capability_id == "wizard_training" && offered_track_ids == &["wizard_magic"]
    ));
    assert!(matches!(&arrival_service.capabilities[1],
        tme_rules::ServiceCapabilityViewV1::SkillCritique { capability_id, .. }
            if capability_id == "wizard_critique"
    ));
    assert!(matches!(&arrival_service.capabilities[2],
        tme_rules::ServiceCapabilityViewV1::SpellTeaching { capability_id, spell_ids, .. }
            if capability_id == "spell_teaching" && spell_ids == &["ember_bolt"]
    ));
    assert!(matches!(&arrival_service.capabilities[3],
        tme_rules::ServiceCapabilityViewV1::Merchant { capability_id, .. }
            if capability_id == "trail_wares"
    ));
    assert!(matches!(&arrival_service.capabilities[4],
        tme_rules::ServiceCapabilityViewV1::ItemService { capability_id, .. }
            if capability_id == "appraisal"
    ));
    assert!(matches!(&arrival_service.capabilities[5],
        tme_rules::ServiceCapabilityViewV1::Bank { capability_id, bank_id, .. }
            if capability_id == "bank_access" && bank_id == "waystation_bank"
    ));
    assert!(matches!(&arrival_service.capabilities[6],
        tme_rules::ServiceCapabilityViewV1::Locker { capability_id, vault_id, .. }
            if capability_id == "locker_access" && vault_id == "waystation_vault"
    ));
    assert!(matches!(&arrival_service.capabilities[7],
        tme_rules::ServiceCapabilityViewV1::Restoration { capability_id, .. }
            if capability_id == "restoration"
    ));
    let route_keeper = arrival_context
        .npcs_here
        .iter()
        .find(|npc| npc.actor_id == "route_keeper")
        .expect("the route keeper should be present at town arrival");
    let ask = route_keeper
        .interactions
        .iter()
        .find(|interaction| interaction.interaction_id == "ask_about_waystone")
        .expect("the route keeper should expose the quest-start interaction");
    assert!(ask.actions.iter().any(|action| action.enabled
        && matches!(
            action.command.as_ref().map(|command| &command.intent),
            Some(tme_rules::PlayerIntentPayloadV1::InteractWithNpc {
                npc_actor_id,
                interaction_id,
                item_instance_id: None,
            }) if npc_actor_id == "route_keeper" && interaction_id == "ask_about_waystone"
        )));

    let post_restoration = &trace.steps[22];
    let post_restoration_player = post_restoration
        .after_debug_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("step 23 should retain the player");
    let post_restoration_character = post_restoration_player
        .character
        .as_ref()
        .expect("step 23 should retain the player character sheet");
    assert_eq!(post_restoration_character.resources.hp, 40);
    assert_eq!(post_restoration_character.resources.mp, 40);
    let post_service = post_restoration
        .after_action_context
        .services_here
        .iter()
        .find(|service| service.service_id == "waystation_counter")
        .expect("step 23 action context should own the grouped service state");
    assert_eq!(post_service.capabilities.len(), 8);
    let bank = post_service
        .capabilities
        .iter()
        .find_map(|capability| match capability {
            tme_rules::ServiceCapabilityViewV1::Bank {
                capability_id,
                bank_id,
                balance_gold,
                transaction_cap_gold,
                ..
            } => Some((capability_id, bank_id, balance_gold, transaction_cap_gold)),
            _ => None,
        })
        .expect("step 23 context should expose the bank capability");
    assert_eq!(
        (bank.0.as_str(), bank.1.as_str(), *bank.2, *bank.3),
        ("bank_access", "waystation_bank", 15, 80)
    );
    let locker = post_service
        .capabilities
        .iter()
        .find_map(|capability| match capability {
            tme_rules::ServiceCapabilityViewV1::Locker {
                capability_id,
                vault_id,
                capacity,
                item_count,
                items,
                ..
            } => Some((capability_id, vault_id, capacity, item_count, items)),
            _ => None,
        })
        .expect("step 23 context should expose the locker capability");
    assert_eq!(
        (locker.0.as_str(), locker.1.as_str(), *locker.2, *locker.3),
        ("locker_access", "waystation_vault", 2, 1)
    );
    assert_eq!(locker.4.len(), 1);
    assert_eq!(locker.4[0].item_instance_id, "weathered_staff");
    let merchant = post_service
        .capabilities
        .iter()
        .find_map(|capability| match capability {
            tme_rules::ServiceCapabilityViewV1::Merchant {
                capability_id,
                listings,
                ..
            } => Some((capability_id, listings)),
            _ => None,
        })
        .expect("step 23 context should expose the merchant capability");
    assert_eq!(merchant.0, "trail_wares");
    assert!(merchant.1.iter().any(|listing| {
        listing.item.item_instance_id == "trade_charm"
            && listing.origin == tme_rules::MerchantListingOriginViewV1::PawnPool
            && listing.price_gold == 32
    }));

    let after_departure = &trace.steps[23..];
    assert!(!after_departure.iter().any(|step| matches!(
        &step.command.intent,
        tme_rules::PlayerIntentPayloadV1::DepositBankGold { .. }
            | tme_rules::PlayerIntentPayloadV1::WithdrawBankGold { .. }
            | tme_rules::PlayerIntentPayloadV1::DepositLockerItem { .. }
            | tme_rules::PlayerIntentPayloadV1::WithdrawLockerItem { .. }
            | tme_rules::PlayerIntentPayloadV1::BuyFromMerchant { .. }
            | tme_rules::PlayerIntentPayloadV1::SellToMerchant { .. }
    )));
    assert!(
        !after_departure
            .iter()
            .flat_map(|step| step.events.iter())
            .any(|event| match event {
                tme_rules::Event::BankBalanceChanged { .. } => true,
                tme_rules::Event::GoldRelocated { reason, .. } => matches!(
                    reason,
                    tme_rules::GoldRelocationReason::BankDeposit
                        | tme_rules::GoldRelocationReason::BankWithdrawal
                ),
                tme_rules::Event::ItemRelocated { reason, .. } => matches!(
                    reason,
                    tme_rules::ItemRelocationReason::MerchantPurchase
                        | tme_rules::ItemRelocationReason::MerchantSale
                        | tme_rules::ItemRelocationReason::LockerDeposit
                        | tme_rules::ItemRelocationReason::LockerWithdrawal
                ),
                tme_rules::Event::TransactionCommitted { source, .. } => matches!(
                    source,
                    tme_rules::TransactionSourceV1::MerchantPurchase { .. }
                        | tme_rules::TransactionSourceV1::MerchantSale { .. }
                        | tme_rules::TransactionSourceV1::BankDeposit { .. }
                        | tme_rules::TransactionSourceV1::BankWithdrawal { .. }
                ),
                _ => false,
            })
    );

    let danger_context = &trace.steps[23].after_action_context;
    assert_eq!(danger_context.position.level, "trailhead");
    assert_eq!(
        danger_context.position.position,
        tme_rules::Coord { x: 3, y: 1 }
    );
    let sentinel_target = danger_context
        .attack_targets
        .iter()
        .find(|target| target.actor_id == "return_sentinel")
        .expect("return context should expose the sentinel target");
    let fight = sentinel_target
        .physical_attacks
        .iter()
        .find(|attack| attack.mode == tme_rules::PhysicalAttackMode::Fight)
        .expect("return context should expose Fight");
    assert!(fight.enabled);
    assert_eq!(fight.attack_safety, tme_rules::AttackSafety::OpenHostile);
    assert!(
        matches!(fight.command.as_ref().map(|command| &command.intent),
            Some(tme_rules::PlayerIntentPayloadV1::PhysicalAttack {
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id,
                authorization: tme_rules::HostilityAuthorization::Safe,
            }) if target_actor_id == "return_sentinel"
        )
    );
    let ember = danger_context
        .spell_actions
        .iter()
        .find(|spell| spell.spell_id == "ember_bolt")
        .expect("return context should expose the learned spell descriptor");
    assert!(ember.cast.enabled);
    assert!(ember.cast.requires_target_selection);
    assert!(
        ember.cast.command.is_none(),
        "targeted spell context must remain descriptor-only"
    );
    assert!(matches!(&trace.steps[25].command.intent,
        tme_rules::PlayerIntentPayloadV1::CastSpell {
            spell_id,
            target: Some(tme_rules::SpellTarget::Actor { actor_id }),
            ..
        } if spell_id == "ember_bolt" && actor_id == "return_sentinel"
    ));

    let final_debug = &trace.r#final.final_debug_snapshot;
    let final_player = final_debug
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("final debug snapshot should retain the player");
    assert!(matches!(
        final_player.life_state,
        tme_rules::ActorLifeStateViewV1::Alive
    ));
    assert_eq!(final_player.location.level, "trailhead");
    assert_eq!(
        final_player.location.position,
        tme_rules::Coord { x: 3, y: 1 }
    );
    assert_eq!((final_player.hp, final_player.max_hp), (40, 40));
    assert_eq!(final_player.carried.gold.sack, 64);
    assert!(final_player.carried.items.iter().any(|item| {
        item.item.item_instance_id == "bright_staff_stock"
            && item.position == tme_rules::CarriedPosition::RightHand
    }));
    assert!(final_player.carried.items.iter().any(|item| {
        item.item.item_instance_id == "field_spell_book"
            && item.position == tme_rules::CarriedPosition::SackItem1
    }));
    let final_character = final_player
        .character
        .as_ref()
        .expect("final debug snapshot should retain the character sheet");
    assert_eq!(
        (
            final_character.resources.hp,
            final_character.resources.mp,
            final_character.resources.stamina
        ),
        (40, 38, 19)
    );
    assert!(
        final_character
            .known_spells
            .iter()
            .any(|spell| { spell.spell_id == "ember_bolt" && spell.lane == "wizard_magic" })
    );
    assert!(
        final_character
            .skill_ledger
            .iter()
            .any(|skill| { skill.track_id == "wizard_magic" && skill.learning_rate == 2 })
    );
    let final_quest = final_debug
        .quest_states
        .iter()
        .find(|state| state.quest.quest_id == "waystone_recovery")
        .expect("final debug snapshot should retain the quest state");
    assert_eq!(
        final_quest.character_id.as_str(),
        "character:town_adventure_loop_gallery:primary"
    );
    assert_eq!(final_quest.quest.stage_id, "completed");
    assert!(final_quest.quest.terminal);
    let final_corpse = final_debug
        .corpses
        .iter()
        .find(|corpse| corpse.corpse_id.as_str() == "corpse:1")
        .expect("final debug snapshot should retain corpse:1");
    assert_eq!(final_corpse.origin_actor_id, "road_scavenger");
    assert!(final_corpse.searched);
    let scavenger = final_debug
        .actors
        .iter()
        .find(|actor| actor.id == "road_scavenger")
        .expect("final debug snapshot should retain the defeated scavenger");
    assert!(matches!(
        scavenger.life_state,
        tme_rules::ActorLifeStateViewV1::Dead
    ));
    let sentinel = final_debug
        .actors
        .iter()
        .find(|actor| actor.id == "return_sentinel")
        .expect("final debug snapshot should retain the sentinel");
    assert!(matches!(
        sentinel.life_state,
        tme_rules::ActorLifeStateViewV1::Alive
    ));
    assert_eq!((sentinel.hp, sentinel.max_hp), (53, 100));

    let final_observed = &trace.r#final.final_observed_snapshot;
    assert_eq!(final_observed.observation_center.level, "trailhead");
    assert_eq!(
        final_observed.observation_center.position,
        tme_rules::Coord { x: 3, y: 1 }
    );
    assert!(
        !final_observed
            .actors
            .iter()
            .any(|actor| actor.id == "route_keeper")
    );
    assert!(
        final_observed
            .actors
            .iter()
            .filter(|actor| actor.id != "player")
            .all(|actor| actor.character.is_none())
    );
    assert!(trace.r#final.final_action_context.services_here.is_empty());
    let observed_value =
        serde_json::to_value(final_observed).expect("observed snapshot serializes");
    assert!(observed_value.get("quest_states").is_none());
    assert!(observed_value.get("social_relations").is_none());
    assert!(observed_value.get("spell_social").is_none());
}

#[test]
fn trace_v2_world_topology_gallery_composes_all_navigation_owners() {
    let trace = run_trace_v2("world_topology_gallery.json", 7);
    assert_eq!(trace.steps.len(), 9);
    assert!(
        trace.steps[0]
            .events
            .iter()
            .any(|event| matches!(event, tme_rules::Event::PortalCreated { .. }))
    );

    let transitions = trace
        .steps
        .iter()
        .flat_map(|step| &step.events)
        .filter_map(|event| match event {
            tme_rules::Event::WorldTransition {
                from,
                to,
                navigation,
                ..
            } => Some((from, to, navigation)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(transitions.len(), 7);
    assert!(transitions.iter().any(|(_, _, kind)| matches!(
        kind,
        tme_rules::NavigationKind::Stairs {
            direction: tme_rules::VerticalDirection::Down
        }
    )));
    assert!(transitions.iter().any(|(_, _, kind)| matches!(
        kind,
        tme_rules::NavigationKind::Climb {
            direction: tme_rules::VerticalDirection::Up
        }
    )));
    for kind in [
        tme_rules::NavigationKind::Door,
        tme_rules::NavigationKind::Pit,
        tme_rules::NavigationKind::Passage,
        tme_rules::NavigationKind::Portal,
    ] {
        assert!(
            transitions.iter().any(|(_, _, actual)| **actual == kind),
            "missing transition kind {kind:?}"
        );
    }
    assert!(transitions.iter().any(|(from, to, kind)| **kind
        == tme_rules::NavigationKind::Passage
        && from.realm == "realm_0"
        && to.realm == "realm_1"));

    let swim_step = &trace.steps[2];
    let preview = swim_step
        .preview
        .as_ref()
        .expect("layered route has a preview");
    assert!(preview.steps.iter().any(|step| matches!(
        step.outcome,
        tme_rules::PathPreviewStepOutcomeV1::Moved {
            kind: tme_rules::TransitionKindViewV1::Swim
        }
    )));
    let final_player = trace
        .r#final
        .final_debug_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("final player");
    assert_eq!(final_player.location.realm, "realm_0");
    assert_eq!(final_player.location.level, "door_hall");
}
