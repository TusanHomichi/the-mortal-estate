use crate::spell_support::*;
use crate::support::content_parts::ContentParts;
use tme_rules::*;

fn bu_item_spell_engine(known_spell_ids: &[&str]) -> Engine {
    let mut parts = ContentParts::tracked(
        "utility_door_secret_item_spells",
        "profile/utility_door_secret_item_spells",
    );
    parts.profile_value_mut()["rules_profile"] = serde_json::json!("rules/first_room");
    parts.profile_value_mut()["items"] = serde_json::json!([]);
    for item in item_definitions() {
        let id = item["id"].as_str().expect("item id").to_string();
        let existing_key = parts.catalog["items"]
            .as_object()
            .expect("item registry")
            .iter()
            .find_map(|(key, value)| (value == &item).then(|| key.clone()));
        if let Some(key) = existing_key {
            parts.profile_value_mut()["items"]
                .as_array_mut()
                .expect("item selection")
                .push(serde_json::Value::String(key));
        } else {
            parts.push_selected("items", &format!("item/{id}/spell_items_test"), item);
        }
    }
    parts.profile_value_mut()["spells"] = serde_json::json!([
        "spell/identify/utility_door_secret_item_spells",
        "spell/keen_edge/utility_door_secret_item_spells",
        "spell/shape_token/utility_door_secret_item_spells"
    ]);
    parts.selected_mut("spells", 0)["target"] = serde_json::json!({"kind": "item"});
    parts.selected_mut("spells", 2)["effect"]["item_utility"]["output_item_definition_id"] =
        serde_json::json!("polished_token");
    let mut equipped_transform = parts.selected_mut("spells", 2).clone();
    equipped_transform["id"] = serde_json::json!("shape_equipped_token");
    equipped_transform["name"] = serde_json::json!("Shape Equipped Token");
    equipped_transform["target"]["item_location"] = serde_json::json!("active_equipment");
    parts.push_selected(
        "spells",
        "spell/shape_equipped_token/spell_items_test",
        equipped_transform,
    );

    *parts.template_levels_source_mut() = serde_json::json!({
        "start": {
            "law_zone": "none",
            "width": 5,
            "height": 3,
            "cells": [
                [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]],
                [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
                [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]]
            ]
        }
    });
    parts.world_template["topology"] = serde_json::json!({});
    *parts.item_instances_mut() = serde_json::json!({
        "utility_blade": {"definition_id": "utility_blade", "binding": {"state": "unrestricted"}},
        "seeing_gem": {"definition_id": "seeing_gem", "binding": {"state": "unrestricted"}},
        "raw_relic": {"definition_id": "raw_relic", "binding": {"state": "unrestricted"}},
        "ground_charm": {"definition_id": "ground_charm", "binding": {"state": "unrestricted"}}
    });
    *parts.ground_items_mut() = serde_json::json!([{
        "item_instance_id": "ground_charm",
        "location": {
            "realm": "realm_0", "level": "start", "position": {"x": 1, "y": 1}
        }
    }]);
    let known_spells = known_spell_ids
        .iter()
        .map(|spell_id| {
            serde_json::json!({
                "spell_id": spell_id, "lane": "wizard_magic", "learned_at_level": 1
            })
        })
        .collect::<Vec<_>>();
    parts.actor_definition_mut(0)["name"] = serde_json::json!("Wiz");
    parts.actor_definition_mut(0)["stats"] =
        serde_json::json!({"hp": 20, "attack": 12, "defense": 0});
    parts.actor_definition_mut(1)["name"] = serde_json::json!("Target");
    parts.actor_definition_mut(1)["stats"] =
        serde_json::json!({"hp": 200, "attack": 0, "defense": 0});
    let actors = parts.actors_mut().as_array_mut().expect("utility actors");
    actors[0]["location"]["level"] = serde_json::json!("start");
    actors[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    actors[0]["character"]["resources"] = serde_json::json!({
        "hp": 20, "max_hp": 20, "peak_hp": 20,
        "mp": 40, "max_mp": 40, "stamina": 20, "max_stamina": 20
    });
    actors[0]["character"]["known_spells"] = serde_json::Value::Array(known_spells);
    actors[0]["carried"]["items"] = serde_json::json!([
        {"item_instance_id": "utility_blade", "position": "right_hand"},
        {"item_instance_id": "seeing_gem", "position": "sack_item_1"},
        {"item_instance_id": "raw_relic", "position": "sack_item_2"}
    ]);
    actors[1]["id"] = serde_json::json!("target");
    actors[1]["location"]["level"] = serde_json::json!("start");
    actors[1]["location"]["position"] = serde_json::json!({"x": 2, "y": 1});
    parts
        .engine(1_010_580_540)
        .expect("item spell engine should start")
}

fn item_definitions() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "id": "utility_blade", "kind": "weapon", "name": "Utility Blade", "category": "sword",
            "weapon": {
                "skill_track_id": "sword", "default_attack_mode": "poke",
                "attack_modes": [{"mode": "poke", "maximum_range": 1, "damage_kind": "piercing"}],
                "cooldown_units": 1, "combat_add_rating": 0, "handedness": "one_handed", "block_value": 0
            },
            "capability": {"taxonomy_id": "sword"},
            "economy": {"unit_value_gold": 25, "unit_burden": 1},
            "valid_placements": ["hand", "belt_side", "belt_back", "sack"]
        }),
        serde_json::json!({
            "id": "seeing_gem", "kind": "trinket", "name": "Seeing Gem", "category": "trinket",
            "capability": {"taxonomy_id": "trinket", "resistance_boosts": [{"tag": "glimmer", "bonus_twentieths": 3}]},
            "economy": {"unit_value_gold": 75, "unit_burden": 1}, "valid_placements": ["hand", "sack"]
        }),
        serde_json::json!({
            "id": "raw_relic", "kind": "trinket", "name": "Raw Relic", "category": "trinket",
            "economy": {"unit_value_gold": 5, "unit_burden": 1}, "valid_placements": ["hand", "sack"]
        }),
        serde_json::json!({
            "id": "ground_charm", "kind": "trinket", "name": "Ground Charm", "category": "trinket",
            "economy": {"unit_value_gold": 11, "unit_burden": 1}, "valid_placements": ["hand", "sack"]
        }),
        serde_json::json!({
            "id": "polished_token", "kind": "trinket", "name": "Polished Token", "category": "trinket",
            "valid_placements": ["sack"], "economy": {"unit_value_gold": 17, "unit_burden": 1}
        }),
    ]
}

#[test]
fn item_identify_emits_explicit_identity_and_capability_without_value() {
    let mut engine = bu_item_spell_engine(&["identify"]);

    let carried = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "identify".to_string(),
                target: Some(SpellTarget::Item {
                    item_instance_id: "seeing_gem".to_string(),
                    location: tme_rules::SpellItemLocation::Sack,
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("identify carried item");
    assert!(carried.iter().any(|event| matches!(
        event,
        Event::ItemIdentified {
            actor_id,
            actor,
            item_instance_id,
            item_definition_id,
            item_name,
            location,
            capability,
            ..
        } if actor_id == "player"
            && actor == "Wiz"
            && item_instance_id == "seeing_gem"
            && item_definition_id == "seeing_gem"
            && item_name == "Seeing Gem"
            && location == "sack"
            && capability
                .as_ref()
                .and_then(|capability| capability.taxonomy_id.as_deref())
                == Some("trinket")
    )));

    let ground = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "identify".to_string(),
                target: Some(SpellTarget::Item {
                    item_instance_id: "ground_charm".to_string(),
                    location: tme_rules::SpellItemLocation::GroundHere,
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("identify ground item");
    assert!(ground.iter().any(|event| matches!(
        event,
        Event::ItemIdentified {
            item_instance_id,
            item_definition_id,
            item_name,
            location,
            capability,
            ..
        } if item_instance_id == "ground_charm"
            && item_definition_id == "ground_charm"
            && item_name == "Ground Charm"
            && location == "ground_here"
            && capability.is_none()
    )));
}

#[test]
fn weapon_enchant_modifies_active_weapon_damage_until_it_expires() {
    let mut enchanted = bu_item_spell_engine(&["keen_edge"]);
    let enchant_events = enchanted
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "keen_edge".to_string(),
                target: Some(SpellTarget::Item {
                    item_instance_id: "utility_blade".to_string(),
                    location: tme_rules::SpellItemLocation::ActiveEquipment,
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("enchant equipped weapon");
    let enchanted_attack = enchanted
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Poke,
                target_actor_id: "target".into(),
            },
        )
        .expect("enchanted attack");
    let enchanted_damage = attack_damage(&enchanted_attack.events);
    assert!(enchant_events.iter().any(|event| matches!(
        event,
        Event::ItemEnchanted {
            actor_id,
            actor,
            item_instance_id,
            combat_add_rating_bonus,
            tags,
            remaining_rounds,
            ..
        } if actor_id == "player"
            && actor == "Wiz"
            && item_instance_id == "utility_blade"
            && *combat_add_rating_bonus == 5
            && tags == &vec!["keen".to_string()]
            && *remaining_rounds == Some(1)
    )));

    let mut baseline = bu_item_spell_engine(&[]);
    baseline
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("align round");
    let baseline_attack = baseline
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Poke,
                target_actor_id: "target".into(),
            },
        )
        .expect("baseline attack");
    let baseline_damage = attack_damage(&baseline_attack.events);
    assert_eq!(enchanted_damage, baseline_damage + 5);

    let mut expired = bu_item_spell_engine(&["keen_edge"]);
    expired
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "keen_edge".to_string(),
                target: Some(SpellTarget::Item {
                    item_instance_id: "utility_blade".to_string(),
                    location: tme_rules::SpellItemLocation::ActiveEquipment,
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("enchant equipped weapon");
    let expiration = expired
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("expire enchant");
    assert!(expiration.iter().any(|event| matches!(
        event,
        Event::ItemEnchantmentExpired {
            item_instance_id,
            enchantment_instance_id,
            ..
        } if item_instance_id == "utility_blade"
            && enchantment_instance_id == "spell:keen_edge:1:utility_blade"
    )));
    let expired_attack = expired
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Poke,
                target_actor_id: "target".into(),
            },
        )
        .expect("expired attack");
    let expired_damage = attack_damage(&expired_attack.events);

    let mut expired_baseline = bu_item_spell_engine(&[]);
    expired_baseline
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("align 1");
    expired_baseline
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("align 2");
    let expired_baseline_attack = expired_baseline
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Poke,
                target_actor_id: "target".into(),
            },
        )
        .expect("baseline expired attack");
    assert_eq!(
        expired_damage,
        attack_damage(&expired_baseline_attack.events)
    );
}

#[test]
fn transform_item_replaces_exact_target_and_preserves_other_items() {
    let mut engine = bu_item_spell_engine(&["shape_token"]);

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "shape_token".to_string(),
                target: Some(SpellTarget::Item {
                    item_instance_id: "raw_relic".to_string(),
                    location: tme_rules::SpellItemLocation::Sack,
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("transform carried item");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::ItemTransformed {
            actor_id,
            actor,
            item_instance_id,
            old_item_definition_id,
            new_item_definition_id,
            location,
            ..
        } if actor_id == "player"
            && actor == "Wiz"
            && item_instance_id == "raw_relic"
            && old_item_definition_id == "raw_relic"
            && new_item_definition_id == "polished_token"
            && location == "sack"
    )));
    let player = &engine.world().actors[0];
    assert_eq!(
        player
            .carried
            .items
            .values()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["utility_blade", "seeing_gem", "raw_relic"]
    );
    assert!(
        engine
            .world()
            .ground_items
            .iter()
            .any(|item| item.item_instance_id == "ground_charm")
    );
}

#[test]
fn transform_item_rejects_equipped_output_that_cannot_use_original_slot() {
    let mut runtime_engine = bu_item_spell_engine(&["shape_equipped_token"]);

    let err = runtime_engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "shape_equipped_token".to_string(),
                target: Some(SpellTarget::Item {
                    item_instance_id: "utility_blade".to_string(),
                    location: tme_rules::SpellItemLocation::ActiveEquipment,
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("runtime transform should reject output that cannot stay equipped");
    assert_eq!(err.message(), "invalid_target");

    let player = &runtime_engine.world().actors[0];
    assert_eq!(
        player
            .carried
            .items
            .get(&tme_rules::CarriedPosition::RightHand)
            .map(String::as_str),
        Some("utility_blade")
    );
    assert!(
        !player
            .carried
            .items
            .values()
            .any(|item| item == "polished_token"),
        "illegal transform must not leave the output equipped"
    );

    let validation_engine = bu_item_spell_engine(&["shape_equipped_token"]);
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::CastSpell {
            spell_id: "shape_equipped_token".to_string(),
            target: Some(SpellTarget::Item {
                item_instance_id: "utility_blade".to_string(),
                location: tme_rules::SpellItemLocation::ActiveEquipment,
            }),
            authorization: tme_rules::HostilityAuthorization::Safe,
        },
    };

    let status = validation_engine
        .validate_actor_command(&command)
        .expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::InvalidTarget)
    );
}

#[test]
fn enchant_rejects_non_weapon_item_with_invalid_target() {
    let engine = bu_item_spell_engine(&["keen_edge"]);
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::CastSpell {
            spell_id: "keen_edge".to_string(),
            target: Some(SpellTarget::Item {
                item_instance_id: "seeing_gem".to_string(),
                location: tme_rules::SpellItemLocation::Sack,
            }),
            authorization: tme_rules::HostilityAuthorization::Safe,
        },
    };

    let status = engine.validate_actor_command(&command).expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::InvalidTarget)
    );
}
