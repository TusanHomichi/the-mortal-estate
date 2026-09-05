use super::*;

#[test]
fn debug_snapshot_has_full_sorted_ledger_while_observed_frame_exposes_only_controlled_log() {
    let mut engine = engine();
    start_quest(&mut engine);
    let foreign_character: CharacterId =
        serde_json::from_str(r#""character:foreign:private""#).expect("foreign ID");
    engine.world_mut().quest_states.insert(
        foreign_character.clone(),
        std::collections::BTreeMap::from([(
            QuestId::new("harbor_escort"),
            QuestStageId::new("completed"),
        )]),
    );

    let debug = engine.snapshot();
    assert_eq!(debug.quest_states.len(), 2);
    assert_eq!(
        debug
            .quest_states
            .iter()
            .map(|row| row.character_id.as_str())
            .collect::<Vec<_>>(),
        vec!["character:foreign:private", "character:harbor:primary"]
    );
    let frame = engine
        .actor_observed_frame(&tme_rules::ActorId::from("player"))
        .expect("observed frame");
    assert_eq!(frame.action_context.quest_log.len(), 1);
    assert_eq!(frame.action_context.quest_log[0].stage_id, "awaiting_token");
    let observed_json = serde_json::to_string(&frame).expect("frame serializes");
    assert!(!observed_json.contains("character:foreign:private"));

    let mut missing_actor_npc = serde_json::to_value(&debug).expect("debug JSON");
    missing_actor_npc["actors"][0]
        .as_object_mut()
        .expect("actor object")
        .remove("npc");
    assert!(serde_json::from_value::<tme_rules::WorldSnapshotV1>(missing_actor_npc).is_err());
    let mut missing_ledger = serde_json::to_value(&debug).expect("debug JSON");
    missing_ledger
        .as_object_mut()
        .expect("debug object")
        .remove("quest_states");
    assert!(serde_json::from_value::<tme_rules::WorldSnapshotV1>(missing_ledger).is_err());

    for field in ["npcs_here", "quest_log"] {
        let mut missing = serde_json::to_value(&frame.action_context).expect("action context JSON");
        missing
            .as_object_mut()
            .expect("action context object")
            .remove(field);
        assert!(serde_json::from_value::<tme_rules::PlayerActionContextV2>(missing).is_err());
    }
}

#[test]
fn event_39_npc_and_quest_payloads_are_strict_with_required_nulls() {
    assert_eq!(tme_rules::EVENT_CONTRACT_VERSION, 41);
    let mut engine = engine();
    let opening = start_quest(&mut engine);
    let offered = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            interact(
                "wayfinder",
                "offer_signal_token",
                Some("player_signal_token"),
            ),
        )
        .expect("follow starts");

    let opening_quest = opening
        .iter()
        .find(|event| matches!(event, Event::QuestStateChanged { .. }))
        .expect("opening quest event");
    let mut missing_before = serde_json::to_value(opening_quest).expect("event JSON");
    missing_before["quest_state_changed"]
        .as_object_mut()
        .expect("quest payload")
        .remove("before_stage_id");
    assert!(serde_json::from_value::<Event>(missing_before).is_err());

    let follow = offered
        .events
        .iter()
        .find(|event| matches!(event, Event::NpcFollowChanged { .. }))
        .expect("follow event");
    for nullable_field in ["from_character_id", "to_character_id"] {
        let mut missing = serde_json::to_value(follow).expect("follow JSON");
        missing["npc_follow_changed"]
            .as_object_mut()
            .expect("follow payload")
            .remove(nullable_field);
        assert!(serde_json::from_value::<Event>(missing).is_err());
    }

    let spoke = offered
        .events
        .iter()
        .find(|event| matches!(event, Event::NpcSpoke { .. }))
        .expect("NPC response event");
    let Event::NpcSpoke {
        recipient_character_id,
        ..
    } = spoke
    else {
        unreachable!("selected NPC response")
    };
    assert_eq!(recipient_character_id.as_str(), "character:harbor:primary");
    let mut missing_recipient = serde_json::to_value(spoke).expect("NPC response JSON");
    missing_recipient["npc_spoke"]
        .as_object_mut()
        .expect("NPC response payload")
        .remove("recipient_character_id");
    assert!(serde_json::from_value::<Event>(missing_recipient).is_err());

    let mut receipt = serde_json::to_value(transaction(&offered.events)).expect("receipt JSON");
    receipt["transaction_committed"]["source"]["legacy_quest_flag"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Event>(receipt).is_err());

    let mut reward_missing_before =
        serde_json::to_value(transaction(&offered.events)).expect("receipt JSON");
    reward_missing_before["transaction_committed"]["rewards"][1]
        .as_object_mut()
        .expect("quest reward")
        .remove("before_stage_id");
    assert!(serde_json::from_value::<Event>(reward_missing_before).is_err());
}

#[test]
fn generic_service_transaction_reuses_alignment_and_quest_gates_and_quest_owner() {
    let mut value = ContentParts::tracked("service_transactions", "profile/service_transactions");
    value.push_selected(
        "quests",
        "quest/service_proof/test",
        serde_json::json!({
            "id": "service_proof",
            "title": "Service Proof",
            "stages": [{"id": "done", "label": "Proof complete", "terminal": true}]
        }),
    );
    let transaction = &mut value
        .selected_by_runtime_id_mut("service_definitions", "waystation_clerk")["capabilities"][0]["transactions"]
        [0];
    transaction["requirements"]
        .as_array_mut()
        .expect("requirements")
        .extend([
            serde_json::json!({"kind": "exact_alignment", "alignment": "lawful"}),
            serde_json::json!({"kind": "quest_unstarted", "quest_id": "service_proof"}),
        ]);
    transaction["rewards"]
        .as_array_mut()
        .expect("rewards")
        .push(serde_json::json!({
            "kind": "quest_stage",
            "quest_id": "service_proof",
            "stage_id": "done"
        }));
    let mut engine = engine_from(value);
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CommitServiceTransaction {
                service_id: "waystation_clerk".to_string(),
                capability_id: "exchanges".to_string(),
                transaction_id: "token_for_badge".to_string(),
                item_instance_id: Some("etched_token_stack".to_string()),
            },
        )
        .expect("generic service uses the shared gates and quest reward");
    assert!(events.events.iter().any(|event| matches!(
        event,
        Event::QuestStateChanged {
            quest_id,
            before_stage_id: None,
            after_stage_id,
            ..
        } if quest_id == "service_proof" && after_stage_id == "done"
    )));
    assert_eq!(
        engine.world().quest_states[&serde_json::from_str::<CharacterId>(
            r#""character:service_transactions:primary""#
        )
        .expect("service character")][&QuestId::new("service_proof")],
        QuestStageId::new("done")
    );
}

#[test]
fn four_contract_quest_and_npc_shapes_are_required_strict_and_direct() {
    fixture_value()
        .validated_seed()
        .expect("EF content graph validates");

    let mut stale = fixture_value();
    stale.world_seed["schema_version"] = serde_json::json!(24);
    assert!(
        stale
            .validated_seed()
            .expect_err("flat schema field is stale")
            .contains("unknown field `schema_version`")
    );

    let mut missing_quests = fixture_value();
    missing_quests
        .profile_value_mut()
        .as_object_mut()
        .expect("profile object")
        .remove("quests");
    missing_quests
        .validated_seed()
        .expect_err("profile quest selection is required");

    for actor_index in 0..4 {
        let mut missing_npc = fixture_value();
        missing_npc.actors_mut()[actor_index]
            .as_object_mut()
            .expect("actor object")
            .remove("npc");
        missing_npc
            .validated_seed()
            .expect_err("nullable npc key is required on every actor");
    }

    let mut unknown_outcome = fixture_value();
    unknown_outcome.actors_mut()[1]["npc"]["interactions"][0]["outcome"] =
        serde_json::json!({"kind": "set_quest_flag", "flag": "started"});
    unknown_outcome
        .validated_seed()
        .expect_err("outcomes are finite and strict");

    let mut non_npc_role = fixture_value();
    let npc = non_npc_role.actors_mut()[1]["npc"].clone();
    non_npc_role.actors_mut()[3]["npc"] = npc;
    non_npc_role
        .validated_seed()
        .expect_err("monster cannot carry NPC state");
}
