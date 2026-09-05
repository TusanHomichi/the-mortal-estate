use super::*;

#[test]
fn transform_rejects_prospective_burden_overflow_atomically() {
    let mut value = contract_value();
    move_tonics_to_sack(&mut value);
    configure_wizard(&mut value, &["shape_tonic"]);
    push_item(
        &mut value,
        "impossibly_heavy_tonic",
        json!({
            "id": "impossibly_heavy_tonic",
            "kind": "consumable", "valid_placements": ["hand", "sack"],
            "name": "Impossibly Heavy Tonic",
            "consumable": {"effect": "healing", "heal_per_round": 2},
            "economy": {"unit_value_gold": 24, "unit_burden": u64::MAX}
        }),
    );
    push_spell(
        &mut value,
        "shape_tonic",
        json!({
                "id": "shape_tonic",
                "name": "Shape Tonic",
                "status": "draft",
                "lane": "wizard_magic",
                "skill_requirement": 1,
                "mp_cost": 1,
                "stamina_cost": 0,
                "social": {"hostile_act": false, "town_law": "permitted"},
                "casting": {"method": "direct", "cast_class": "not_applicable"},
                "effect": {
                    "family": "item_enchant",
                    "item_utility": {
                        "action": "transform_item",
                        "output_item_definition_id": "impossibly_heavy_tonic"
                    }
                },
                "target": {"kind": "item", "item_location": "sack"}
        }),
    );
    let mut engine = engine_from_value(value);
    let before = engine.world().clone();

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "shape_tonic".into(),
                target: Some(SpellTarget::Item {
                    item_instance_id: "tonic_a".into(),
                    location: SpellItemLocation::Sack,
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("prospective transform burden must be checked");

    assert_eq!(engine.world(), &before);
}

#[test]
fn transform_rejects_prospective_stack_value_overflow_atomically() {
    let mut value = contract_value();
    move_tonics_to_sack(&mut value);
    configure_wizard(&mut value, &["shape_tonic"]);
    push_item(
        &mut value,
        "priceless_tonic",
        json!({
            "id": "priceless_tonic",
            "kind": "consumable", "valid_placements": ["hand", "sack"],
            "name": "Priceless Tonic",
            "consumable": {"effect": "healing", "heal_per_round": 2},
            "economy": {"unit_value_gold": u64::MAX, "unit_burden": 3}
        }),
    );
    push_spell(
        &mut value,
        "shape_tonic",
        json!({
                "id": "shape_tonic",
                "name": "Shape Tonic",
                "status": "draft",
                "lane": "wizard_magic",
                "skill_requirement": 1,
                "mp_cost": 1,
                "stamina_cost": 0,
                "social": {"hostile_act": false, "town_law": "permitted"},
                "casting": {"method": "direct", "cast_class": "not_applicable"},
                "effect": {
                    "family": "item_enchant",
                    "item_utility": {
                        "action": "transform_item",
                        "output_item_definition_id": "priceless_tonic"
                    }
                },
                "target": {"kind": "item", "item_location": "sack"}
        }),
    );
    let mut engine = engine_from_value(value);
    let before = engine.world().clone();

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "shape_tonic".into(),
                target: Some(SpellTarget::Item {
                    item_instance_id: "tonic_a".into(),
                    location: SpellItemLocation::Sack,
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("prospective transform stack value must be checked");

    assert_eq!(error.message(), "invalid_target");
    assert_eq!(engine.world(), &before);
}

#[test]
fn validation_rejects_summon_template_burden_overflow() {
    let mut value = contract_value();
    configure_wizard(&mut value, &["call_echo"]);
    push_item(
        &mut value,
        "heavy_focus",
        json!({
            "id": "heavy_focus",
            "kind": "trinket", "valid_placements": ["hand", "sack"],
            "name": "Heavy Focus",
            "economy": {"unit_value_gold": 1, "unit_burden": u64::MAX}
        }),
    );
    value.push_selected(
        "actor_definitions",
        "actor/echo_guard/item_instance_contract_test",
        json!({
            "id": "actor/summon/echo_guard",
            "name": "Echo Guard",
            "kind": "monster",
            "creature_traits": [],
            "magic_resistance": {"natural_save_twentieths": 5, "evidence_state": "original_provisional"},
            "death": {"remains": "none"},
            "social": {"alignment_source":{"kind":"inherent","alignment":"lawful"},"nature":"other","behavior":"alignment_creature","owner_relation":"summoner"},
            "stats": {"hp": 4, "attack": 1, "defense": 1},
            "ai": {"behavior": "simple_chase", "cadence_units": 1, "aggro_radius": 7, "leash_range": 12, "awareness": {"mode": "unrestricted"}, "physical_attack_modes": ["fight"]},
            "xp_value": 0,
            "physical_damage_affinity_profile_id": "ordinary",
            "monster_abilities": []
        }),
    );
    value.push_selected(
        "summon_templates",
        "summon/echo_guard/item_instance_contract_test",
        json!({
            "id": "echo_guard",
            "actor_definition_id": "actor/summon/echo_guard",
            "item_instances": {
                "focus": {"definition_id": "heavy_focus", "binding": {"state": "unrestricted"}, "quantity": 2}
            },
            "carried": {
                "items": [{"item_instance_id": "focus", "position": "sack_item_1"}],
                "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}
            },
            "active_effects": []
        }),
    );
    push_spell(
        &mut value,
        "call_echo",
        json!({
                "id": "call_echo",
                "name": "Call Echo",
                "status": "draft",
                "lane": "wizard_magic",
                "skill_requirement": 1,
                "mp_cost": 1,
                "stamina_cost": 0,
                "social": {"hostile_act": false, "town_law": "permitted"},
                "casting": {"method": "direct", "cast_class": "not_applicable"},
                "effect": {
                    "family": "summon",
                    "summon_actor_id": "echo_guard",
                    "duration": {"policy": "rounds", "rounds": 2}
                },
                "target": {"kind": "coordinate", "range": 2, "requires_visible": true},
        }),
    );
    let error = value
        .validated_seed()
        .expect_err("summon template burden must be checked during validation");
    assert!(error.to_string().contains(
        "summon_templates[0].item_instances[\"focus\"].quantity * unit_burden must be <= 18446744073709551615"
    ));
}
