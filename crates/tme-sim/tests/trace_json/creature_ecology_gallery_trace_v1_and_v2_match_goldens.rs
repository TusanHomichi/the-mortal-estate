use super::*;

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
