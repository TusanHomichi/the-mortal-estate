use crate::support::content_parts::ContentParts;
use tme_rules::{
    ActionBlockedReasonV1, COMMAND_CONTRACT_VERSION, Coord, Engine, PlayerCommandV1, PlayerIntent,
    PlayerIntentPayloadV1,
    model::{ActiveEffectSource, TileEffectState},
};

use crate::action_context_support::common::option_by_id;
use crate::action_context_support::professions::{
    profession_action_engine, profession_action_engine_with,
};

fn add_greatsword(parts: &mut ContentParts) {
    parts.push_selected(
        "items",
        "item/greatsword/profession_test",
        serde_json::json!({
            "id": "greatsword",
            "kind": "weapon",
            "name": "Greatsword",
            "weapon": {
                "skill_track_id": "sword",
                "default_attack_mode": "fight",
                "attack_modes": [{"mode": "fight", "maximum_range": 0, "damage_kind": "cutting"}],
                "cooldown_units": 1,
                "combat_add_rating": 0,
                "handedness": "two_handed",
                "block_value": 0
            },
            "valid_placements": ["hand", "belt_side", "belt_back", "sack"],
            "economy": {"unit_burden": 1}
        }),
    );
    parts.actors_mut()[0]["carried"]["items"] = serde_json::json!([
        {"item_instance_id": "greatsword", "position": "right_hand"}
    ]);
    parts.item_instances_mut()["greatsword"] = serde_json::json!({
        "definition_id": "greatsword",
        "binding": {"state": "unrestricted"}
    });
}

#[test]
fn hide_command_contract_round_trips_with_version_16() {
    let engine = profession_action_engine("thief", &["thief"]);
    assert_eq!(COMMAND_CONTRACT_VERSION, 26, "EU uses command contract v26");

    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player0".into(),
        intent: PlayerIntentPayloadV1::Hide,
    };

    let intent = engine
        .command_to_actor_intent(&command)
        .expect("hide command should convert");
    assert_eq!(intent, PlayerIntent::Hide);
    assert_eq!(
        Engine::player_intent_to_payload(&intent),
        PlayerIntentPayloadV1::Hide
    );
}

#[test]
fn hide_command_returns_no_profession_action_when_missing() {
    let engine = profession_action_engine_with("thief", &[], |json| {
        json.profile_value_mut()["profession_actions"] = serde_json::json!([]);
    });
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player0".into(),
        intent: PlayerIntentPayloadV1::Hide,
    };

    let status = engine
        .validate_actor_command(&command)
        .expect("command validation should succeed");

    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::NoProfessionAction)
    );
    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player0"))
        .expect("options");
    let hide = option_by_id(&options, "hide");
    assert!(!hide.enabled);
    assert_eq!(
        hide.blocked_reason,
        Some(ActionBlockedReasonV1::NoProfessionAction)
    );
}

#[test]
fn hide_action_option_requires_cover_or_darkness() {
    let engine = profession_action_engine_with("thief", &["thief"], |json| {
        json.selected_mut("profession_actions", 0)["hide"]["requires_cover_or_darkness"] =
            serde_json::json!(true);
    });
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player0".into(),
        intent: PlayerIntentPayloadV1::Hide,
    };

    let status = engine
        .validate_actor_command(&command)
        .expect("command validation should succeed");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::NoCoverOrDarkness)
    );

    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player0"))
        .expect("options");
    let hide = option_by_id(&options, "hide");
    assert!(!hide.enabled);
    assert_eq!(
        hide.blocked_reason,
        Some(ActionBlockedReasonV1::NoCoverOrDarkness)
    );
}

#[test]
fn hide_requires_cover_or_darkness_and_rejects_forbidden_equipment() {
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player0".into(),
        intent: PlayerIntentPayloadV1::Hide,
    };

    let open = profession_action_engine_with("thief", &["thief"], |json| {
        json.selected_mut("profession_actions", 0)["hide"]["requires_cover_or_darkness"] =
            serde_json::json!(true);
    });
    assert_eq!(
        open.validate_actor_command(&command)
            .expect("status")
            .blocked_reason,
        Some(ActionBlockedReasonV1::NoCoverOrDarkness)
    );

    let mut equipped = profession_action_engine_with("thief", &["thief"], |json| {
        json.selected_mut("profession_actions", 0)["hide"]["requires_cover_or_darkness"] =
            serde_json::json!(true);
        add_greatsword(json);
    });
    equipped.world_mut().tile_effects.push(TileEffectState {
        source_actor_id: None,
        instance_id: "tile:shadow:hide".to_string(),
        effect_id: "shadow_veil".to_string(),
        source: ActiveEffectSource {
            kind: "spell".to_string(),
            id: "shadow_veil".to_string(),
        },
        location: tme_rules::WorldPosition::new("realm_0", "room_0", Coord { x: 3, y: 3 }),
        kind: "terrain_overlay".to_string(),
        tags: vec!["shadow".to_string()],
        potency: 0,
        remaining_rounds: Some(2),
        passability: None,
        sight: Some("obscured".to_string()),
        hazard: None,
        move_cost: None,
        tick_interval_rounds: 1,
        last_ticked_at: tme_rules::LogicalTime::new(0),
        hostile_authority: None,
    });
    assert_eq!(
        equipped
            .validate_actor_command(&command)
            .expect("status")
            .blocked_reason,
        Some(ActionBlockedReasonV1::ForbiddenEquipment)
    );
}

#[test]
fn hide_action_option_accepts_adjacent_cover_or_darkness() {
    let mut engine = profession_action_engine_with("thief", &["thief"], |json| {
        json.selected_mut("profession_actions", 0)["hide"]["requires_cover_or_darkness"] =
            serde_json::json!(true);
    });
    engine.world_mut().tile_effects.push(TileEffectState {
        source_actor_id: None,
        instance_id: "tile:shadow:1".to_string(),
        effect_id: "shadow_veil".to_string(),
        source: ActiveEffectSource {
            kind: "spell".to_string(),
            id: "shadow_veil".to_string(),
        },
        location: tme_rules::WorldPosition::new("realm_0", "room_0", Coord { x: 4, y: 3 }),
        kind: "terrain_overlay".to_string(),
        tags: vec!["shadow".to_string()],
        potency: 0,
        remaining_rounds: Some(2),
        passability: None,
        sight: Some("obscured".to_string()),
        hazard: None,
        move_cost: None,
        tick_interval_rounds: 1,
        last_ticked_at: tme_rules::LogicalTime::new(0),
        hostile_authority: None,
    });

    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player0".into(),
        intent: PlayerIntentPayloadV1::Hide,
    };

    let status = engine
        .validate_actor_command(&command)
        .expect("command validation should succeed");
    assert!(status.accepted);
    assert_eq!(status.blocked_reason, None);

    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player0"))
        .expect("options");
    let hide = option_by_id(&options, "hide");
    assert!(hide.enabled);
    assert_eq!(hide.blocked_reason, None);
}

#[test]
fn hide_command_rejects_two_handed_equipment() {
    let engine = profession_action_engine_with("thief", &["thief"], |json| {
        add_greatsword(json);
    });
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player0".into(),
        intent: PlayerIntentPayloadV1::Hide,
    };

    let status = engine
        .validate_actor_command(&command)
        .expect("command validation should succeed");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::ForbiddenEquipment)
    );
}

#[test]
fn hide_action_option_is_typed_and_class_gated() {
    let thief = profession_action_engine("thief", &["thief"]);
    let options = thief
        .actor_action_options(&tme_rules::ActorId::from("player0"))
        .expect("options");
    let hide = options
        .iter()
        .find(|option| option.id == "hide")
        .expect("hide option");
    assert!(hide.enabled);
    assert_eq!(hide.blocked_reason, None);
    assert_eq!(
        hide.command.as_ref().map(|command| &command.intent),
        Some(&PlayerIntentPayloadV1::Hide)
    );

    let fighter = profession_action_engine("fighter", &["thief"]);
    let fighter_hide = fighter
        .actor_action_options(&tme_rules::ActorId::from("player0"))
        .expect("options")
        .into_iter()
        .find(|option| option.id == "hide")
        .expect("hide option remains visible as disabled profession action");
    assert!(!fighter_hide.enabled);
    assert_eq!(
        fighter_hide.blocked_reason,
        Some(ActionBlockedReasonV1::WrongClass)
    );
}

#[test]
fn hide_command_rejects_non_thief_even_with_misconfigured_hide_action() {
    let engine = profession_action_engine("fighter", &["fighter"]);
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player0".into(),
        intent: PlayerIntentPayloadV1::Hide,
    };

    let status = engine
        .validate_actor_command(&command)
        .expect("command validation should succeed");

    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::WrongClass)
    );
}

#[test]
fn hide_command_rejects_thief_when_only_authored_hide_excludes_thief() {
    let engine = profession_action_engine("thief", &["fighter"]);
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player0".into(),
        intent: PlayerIntentPayloadV1::Hide,
    };

    let status = engine
        .validate_actor_command(&command)
        .expect("command validation should succeed");

    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::WrongClass)
    );

    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player0"))
        .expect("options");
    let hide = option_by_id(&options, "hide");
    assert!(!hide.enabled);
    assert_eq!(hide.blocked_reason, Some(ActionBlockedReasonV1::WrongClass));
}

#[test]
fn unrelated_hide_config_does_not_block_matching_thief_hide_config() {
    let engine = profession_action_engine_with("thief", &["thief"], |json| {
        json.selected_mut("profession_actions", 0)["hide"]["requires_cover_or_darkness"] =
            serde_json::json!(false);
        json.push_selected(
            "profession_actions",
            "profession/wizard_hide/test",
            serde_json::json!({
                "id": "wizard_hide",
                "kind": "hide",
                "class_ids": ["wizard"],
                "hide": {
                    "effect_id": "hidden_wizard",
                    "duration_rounds": 3,
                    "requires_cover_or_darkness": true,
                    "break_on": ["move", "attack", "active_item_move", "cast", "warm"],
                    "disallow_two_handed": true
                }
            }),
        );
    });
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player0".into(),
        intent: PlayerIntentPayloadV1::Hide,
    };

    let status = engine
        .validate_actor_command(&command)
        .expect("command validation should succeed");

    assert!(status.accepted);
    assert_eq!(status.blocked_reason, None);

    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player0"))
        .expect("options");
    let hide = option_by_id(&options, "hide");
    assert!(hide.enabled);
    assert_eq!(hide.blocked_reason, None);
}

#[test]
fn later_same_class_hide_config_does_not_override_selected_hide_config() {
    let engine = profession_action_engine_with("thief", &["thief"], |json| {
        json.selected_mut("profession_actions", 0)["hide"]["requires_cover_or_darkness"] =
            serde_json::json!(false);
        json.selected_mut("profession_actions", 0)["hide"]["disallow_two_handed"] =
            serde_json::json!(false);
        json.push_selected(
            "profession_actions",
            "profession/thief_shadow_hide/test",
            serde_json::json!({
                "id": "thief_shadow_hide",
                "kind": "hide",
                "class_ids": ["thief"],
                "hide": {
                    "effect_id": "shadow_hidden",
                    "duration_rounds": 3,
                    "requires_cover_or_darkness": true,
                    "break_on": ["move", "attack", "active_item_move", "cast", "warm"],
                    "disallow_two_handed": true
                }
            }),
        );
    });
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player0".into(),
        intent: PlayerIntentPayloadV1::Hide,
    };

    let status = engine
        .validate_actor_command(&command)
        .expect("command validation should succeed");

    assert!(status.accepted);
    assert_eq!(status.blocked_reason, None);

    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player0"))
        .expect("options");
    let hide = option_by_id(&options, "hide");
    assert!(hide.enabled);
    assert_eq!(hide.blocked_reason, None);
}

#[test]
fn step_hide_rejects_non_thief_direct_intent() {
    let mut engine = profession_action_engine("fighter", &["fighter"]);

    let error = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player0"), PlayerIntent::Hide)
        .expect_err("direct hide should reject non-thieves");

    assert_eq!(error.to_string(), "wrong_class");
}
