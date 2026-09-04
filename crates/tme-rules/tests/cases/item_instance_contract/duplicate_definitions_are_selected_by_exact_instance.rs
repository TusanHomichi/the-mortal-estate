use super::*;

#[test]
fn duplicate_definitions_are_selected_by_exact_instance() {
    let mut engine = contract_engine();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "tonic_b".into(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .unwrap();
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .carried
            .items[&tme_rules::CarriedPosition::SackItem1],
        "tonic_b"
    );
    assert!(
        engine
            .world()
            .ground_items
            .iter()
            .any(|item| item.item_instance_id == "tonic_a")
    );
}

#[test]
fn whole_stack_transfers_preserve_identity_and_quantity() {
    let mut engine = contract_engine();

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "tonic_a".into(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .unwrap();
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .carried
            .items[&tme_rules::CarriedPosition::SackItem1],
        "tonic_a"
    );
    assert_eq!(engine.world().item_instances["tonic_a"].quantity, 2);

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "tonic_a".into(),
                destination: tme_rules::ItemMoveDestination::GroundHere,
            },
        )
        .unwrap();
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .carried
            .items
            .is_empty()
    );
    assert_eq!(engine.world().item_instances["tonic_a"].quantity, 2);
    assert!(
        engine
            .world()
            .ground_items
            .iter()
            .any(|item| item.item_instance_id == "tonic_a")
    );
}

#[test]
fn consume_one_preserves_stack_identity_until_zero() {
    let mut engine = contract_engine();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "tonic_a".into(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .unwrap();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("tonic_a".into()),
        )
        .unwrap();
    assert_eq!(engine.world().item_instances["tonic_a"].quantity, 1);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("tonic_a".into()),
        )
        .unwrap();
    assert!(!engine.world().item_instances.contains_key("tonic_a"));
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .carried
            .items
            .is_empty()
    );
}

#[test]
fn active_position_rejects_stack_without_mutation() {
    let mut engine = contract_engine();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "tonic_a".into(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .unwrap();
    let before = engine.world().clone();

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "tonic_a".into(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect_err("a stack cannot occupy an active position");

    assert!(error.message().contains("quantity 1 outside the sack"));
    assert_eq!(engine.world(), &before);
}

#[test]
fn identify_changes_only_identified_and_exposes_no_value() {
    let mut value = contract_value();
    configure_wizard(&mut value, &["identify"]);
    select_existing(
        &mut value,
        "spells",
        "spell/identify/utility_door_secret_item_spells",
    );
    let mut engine = engine_from_value(value);

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "identify".into(),
                target: Some(SpellTarget::Item {
                    item_instance_id: "tonic_a".into(),
                    location: SpellItemLocation::GroundHere,
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("identify should apply");

    let instance = &engine.world().item_instances["tonic_a"];
    assert!(instance.knowledge.identified);
    assert!(instance.knowledge.appraised);
    assert_eq!(instance.quantity, 2);
    assert_eq!(instance.definition_id, "restorative_tonic");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ItemIdentified {
            item_instance_id,
            ..
        } if item_instance_id == "tonic_a"
    )));
}

#[test]
fn transform_preserves_identity_and_quantity_and_resets_knowledge() {
    let mut value = contract_value();
    move_tonics_to_sack(&mut value);
    configure_wizard(&mut value, &["shape_tonic"]);
    push_item(
        &mut value,
        "refined_tonic",
        json!({
            "id": "refined_tonic",
            "kind": "consumable", "valid_placements": ["hand", "sack"],
            "name": "Refined Tonic",
            "consumable": {"effect": "healing", "heal_per_round": 2},
            "economy": {"unit_value_gold": 24, "unit_burden": 3}
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
                        "output_item_definition_id": "refined_tonic"
                    }
                },
                "target": {"kind": "item", "item_location": "sack"}
        }),
    );
    let mut engine = engine_from_value(value);

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
        .expect("transform should apply");

    let transformed = &engine.world().item_instances["tonic_a"];
    assert_eq!(transformed.definition_id, "refined_tonic");
    assert_eq!(transformed.quantity, 2);
    assert!(!transformed.knowledge.identified);
    assert!(!transformed.knowledge.appraised);
    assert_eq!(
        engine.world().item_instances["tonic_b"].definition_id,
        "restorative_tonic"
    );
}

#[test]
fn transforming_equipped_non_hands_item_preserves_hands_weapon_state() {
    let mut value = contract_value();
    configure_wizard(&mut value, &["shape_amulet"]);
    for (id, item) in [
        (
            "training_bow",
            json!({
              "id": "training_bow",
              "kind": "weapon",
              "valid_placements": [
                "hand",
                "belt_side",
                "belt_back",
                "sack"
              ],
              "name": "Training Bow",
              "weapon": {
                "skill_track_id": "bow",
                "default_attack_mode": "shoot",
                "attack_modes": [{"mode": "shoot", "maximum_range": 3, "damage_kind": "piercing"}],
                "cooldown_units": 1,
                "combat_add_rating": 0,
                "handedness": "bow",
                "block_value": 0,
                "nocking": {
                  "unloads_on_movement": true
                }
              },
              "economy": {
                "unit_burden": 1
              }
            }),
        ),
        (
            "rough_amulet",
            json!({
            "id": "rough_amulet",
            "kind": "trinket", "valid_placements": ["hand", "sack", "neck"],
            "name": "Rough Amulet",
            "capability": {}
        , "economy": {"unit_burden": 1}}),
        ),
        (
            "polished_amulet",
            json!({
            "id": "polished_amulet",
            "kind": "trinket", "valid_placements": ["hand", "sack", "neck"],
            "name": "Polished Amulet",
            "capability": {}
        , "economy": {"unit_burden": 1}}),
        ),
    ] {
        push_item(&mut value, id, item);
    }
    value.item_instances_mut()["bow_a"] =
        json!({"definition_id": "training_bow", "binding": {"state": "unrestricted"}});
    value.item_instances_mut()["amulet_a"] = json!({
        "definition_id": "rough_amulet", "binding": {"state": "unrestricted"},
        "knowledge": {"identified": true, "appraised": true}
    });
    value.actors_mut()[0]["carried"]["items"] = json!([
        {"item_instance_id": "bow_a", "position": "right_hand"},
        {"item_instance_id": "amulet_a", "position": "neck"}
    ]);
    push_spell(
        &mut value,
        "shape_amulet",
        json!({
            "id": "shape_amulet",
            "name": "Shape Amulet",
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
                    "output_item_definition_id": "polished_amulet"
                }
            },
            "target": {"kind": "item", "item_location": "active_equipment"}
        }),
    );
    let mut engine = engine_from_value(value);
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Nock)
        .expect("nock the hands bow");

    let before = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context before transform");
    let watcher_before = before
        .attack_targets
        .iter()
        .find(|target| target.actor_id == "watcher")
        .expect("watcher target before transform");
    let shoot_before = watcher_before
        .physical_attacks
        .iter()
        .find(|option| option.mode == tme_rules::PhysicalAttackMode::Shoot)
        .expect("shoot option before transform");
    assert!(shoot_before.enabled, "{shoot_before:?}");
    assert_eq!(shoot_before.blocked_reason, None);

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "shape_amulet".into(),
                target: Some(SpellTarget::Item {
                    item_instance_id: "amulet_a".into(),
                    location: SpellItemLocation::ActiveEquipment,
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("equipped amulet transform should apply");

    let transformed = &engine.world().item_instances["amulet_a"];
    assert_eq!(transformed.definition_id, "polished_amulet");
    assert_eq!(transformed.quantity, 1);
    assert!(!transformed.knowledge.identified);
    assert!(!transformed.knowledge.appraised);
    let after = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context after transform");
    let watcher_after = after
        .attack_targets
        .iter()
        .find(|target| target.actor_id == "watcher")
        .expect("watcher target after transform");
    let shoot_after = watcher_after
        .physical_attacks
        .iter()
        .find(|option| option.mode == tme_rules::PhysicalAttackMode::Shoot)
        .expect("shoot option after transform");
    assert!(shoot_after.enabled);
    assert_eq!(shoot_after.blocked_reason, None);

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Shoot,
                target_actor_id: "watcher".into(),
            },
        )
        .expect("hands bow should still allow the distant attack");
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::Attacked {
                attacker_id,
                defender_id,
                ..
            } | Event::AttackMissed {
                attacker_id,
                defender_id,
                ..
            } if attacker_id == "player" && defender_id == "watcher"
        )),
        "{events:#?}"
    );
}

#[test]
fn transforming_a_nocked_bow_resets_readiness_after_the_transform_event() {
    let mut value = contract_value();
    configure_wizard(&mut value, &["shape_bow"]);
    for (id, item) in [
        (
            "training_bow",
            json!({
                "id": "training_bow",
                "kind": "weapon",
                "name": "Training Bow",
                "valid_placements": ["hand", "belt_side", "belt_back", "sack"],
                "weapon": {
                    "skill_track_id": "bow",
                    "default_attack_mode": "shoot",
                    "attack_modes": [{"mode": "shoot", "maximum_range": 3, "damage_kind": "piercing"}],
                    "cooldown_units": 1,
                    "combat_add_rating": 0,
                    "handedness": "bow",
                    "block_value": 0,
                    "nocking": {"unloads_on_movement": true}
                },
                "economy": {"unit_burden": 1}
            }),
        ),
        (
            "training_focus",
            json!({
                "id": "training_focus",
                "kind": "trinket",
                "name": "Training Focus",
                "valid_placements": ["hand", "sack"],
                "economy": {"unit_burden": 1}
            }),
        ),
    ] {
        push_item(&mut value, id, item);
    }
    value.item_instances_mut()["bow_a"] =
        json!({"definition_id": "training_bow", "binding": {"state": "unrestricted"}});
    value.actors_mut()[0]["carried"]["items"] =
        json!([{"item_instance_id": "bow_a", "position": "right_hand"}]);
    push_spell(
        &mut value,
        "shape_bow",
        json!({
            "id": "shape_bow",
            "name": "Shape Bow",
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
                    "output_item_definition_id": "training_focus"
                }
            },
            "target": {"kind": "item", "item_location": "active_equipment"}
        }),
    );
    let mut engine = engine_from_value(value);
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Nock)
        .expect("bow should nock before transformation");

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "shape_bow".into(),
                target: Some(SpellTarget::Item {
                    item_instance_id: "bow_a".into(),
                    location: SpellItemLocation::ActiveEquipment,
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("nocked bow transformation should apply");
    let transformed_index = events
        .iter()
        .position(|event| matches!(event, Event::ItemTransformed { .. }))
        .expect("transform event");
    let readiness_index = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::BowReadinessChanged {
                    item_instance_id,
                    from: tme_rules::BowReadiness::Nocked,
                    to: tme_rules::BowReadiness::Unnocked,
                    reason: tme_rules::BowReadinessChangeReason::ItemRelocated,
                    ..
                } if item_instance_id == "bow_a"
            )
        })
        .expect("readiness reset event");
    assert!(transformed_index < readiness_index);
    let transformed = &engine.world().item_instances["bow_a"];
    assert_eq!(transformed.definition_id, "training_focus");
    assert_eq!(transformed.bow_readiness, None);
}

#[test]
fn ranged_behavior_requires_current_hands_instance_to_resolve_to_weapon() {
    let mut value = contract_value();
    for (id, item) in [
        (
            "training_bow",
            json!({
              "id": "training_bow",
              "kind": "weapon",
              "valid_placements": [
                "hand",
                "belt_side",
                "belt_back",
                "sack"
              ],
              "name": "Training Bow",
              "weapon": {
                "skill_track_id": "bow",
                "default_attack_mode": "shoot",
                "attack_modes": [{"mode": "shoot", "maximum_range": 3, "damage_kind": "piercing"}],
                "cooldown_units": 1,
                "combat_add_rating": 0,
                "handedness": "bow",
                "block_value": 0,
                "nocking": {
                  "unloads_on_movement": true
                }
              },
              "economy": {
                "unit_burden": 1
              }
            }),
        ),
        (
            "training_focus",
            json!({
            "id": "training_focus",
            "kind": "trinket", "valid_placements": ["hand", "sack"],
            "name": "Training Focus",
            "capability": {}
        , "economy": {"unit_burden": 1}}),
        ),
    ] {
        push_item(&mut value, id, item);
    }
    value.item_instances_mut()["bow_a"] =
        json!({"definition_id": "training_bow", "binding": {"state": "unrestricted"}});
    value.actors_mut()[0]["carried"]["items"] =
        json!([{"item_instance_id": "bow_a", "position": "right_hand"}]);
    let mut engine = engine_from_value(value);
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Nock)
        .expect("nock the hands bow");

    let before = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context with bow");
    let watcher_before = before
        .attack_targets
        .iter()
        .find(|target| target.actor_id == "watcher")
        .expect("watcher target with bow");
    let shoot_before = watcher_before
        .physical_attacks
        .iter()
        .find(|option| option.mode == tme_rules::PhysicalAttackMode::Shoot)
        .expect("shoot option with bow");
    assert!(shoot_before.enabled, "{shoot_before:?}");
    assert_eq!(shoot_before.blocked_reason, None);

    engine
        .world_mut()
        .item_instances
        .get_mut("bow_a")
        .expect("bow instance")
        .definition_id = "training_focus".into();

    let after = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context with non-weapon in hands");
    let watcher_after = after
        .attack_targets
        .iter()
        .find(|target| target.actor_id == "watcher")
        .expect("watcher target with non-weapon in hands");
    let shoot_after = watcher_after
        .physical_attacks
        .iter()
        .find(|option| option.mode == tme_rules::PhysicalAttackMode::Shoot)
        .expect("shoot option with non-weapon");
    assert!(!shoot_after.enabled);
    assert_eq!(
        shoot_after.blocked_reason,
        Some(ActionBlockedReasonV1::RightHandNotWeapon)
    );

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Shoot,
                target_actor_id: "watcher".into(),
            },
        )
        .expect_err("non-weapon hands item must not grant a distant attack");
    assert_eq!(error.message(), "right-hand item \"bow_a\" is not a weapon");
}

#[test]
fn enchantment_state_and_expiry_target_the_item_instance() {
    let mut value = contract_value();
    configure_wizard(&mut value, &["keen_edge"]);
    push_item(
        &mut value,
        "training_blade",
        json!({
          "id": "training_blade",
          "kind": "weapon",
          "valid_placements": [
            "hand",
            "belt_side",
            "belt_back",
            "sack"
          ],
          "name": "Training Blade",
          "category": "sword",
          "weapon": {
            "skill_track_id": "sword",
            "default_attack_mode": "poke",
            "attack_modes": [{"mode": "poke", "maximum_range": 1, "damage_kind": "piercing"}],
            "cooldown_units": 1,
            "combat_add_rating": 0,
            "handedness": "one_handed",
            "block_value": 0
          },
          "capability": {
            "taxonomy_id": "sword"
          },
          "economy": {
            "unit_value_gold": 10,
            "unit_burden": 1
          }
        }),
    );
    value.item_instances_mut()["blade_a"] =
        json!({"definition_id": "training_blade", "binding": {"state": "unrestricted"}});
    value.actors_mut()[0]["carried"]["items"] =
        json!([{"item_instance_id": "blade_a", "position": "right_hand"}]);
    select_existing(
        &mut value,
        "spells",
        "spell/keen_edge/utility_door_secret_item_spells",
    );
    let mut engine = engine_from_value(value);

    engine
        .apply_realtime_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "keen_edge".into(),
                target: Some(SpellTarget::Item {
                    item_instance_id: "blade_a".into(),
                    location: SpellItemLocation::ActiveEquipment,
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("enchantment should apply");

    let state = &engine.world().item_enchantments[0];
    let state_debug = format!("{state:?}");
    assert!(
        state_debug.contains("item_instance_id: \"blade_a\""),
        "{state_debug}"
    );
    assert!(
        state_debug.contains("enchantment_instance_id: \"spell:keen_edge:3000ms:blade_a\""),
        "{state_debug}"
    );

    let expiry = engine
        .advance_action_interval()
        .expect("enchantment should tick");
    assert!(expiry.iter().any(|event| matches!(
        event,
        Event::ItemEnchantmentExpired {
            item_instance_id,
            enchantment_instance_id,
            ..
        } if item_instance_id == "blade_a"
            && enchantment_instance_id == "spell:keen_edge:3000ms:blade_a"
    )));
}

#[test]
fn validation_rejects_checked_world_burden_overflow() {
    let mut value = contract_value();
    value.selected_mut("items", 0)["economy"]["unit_burden"] = json!(u64::MAX);
    let error = value
        .validated_seed()
        .expect_err("initial stack burden must be checked during validation");
    let message = error.to_string();
    assert!(
        message.contains("item_instances[\"tonic_a\"].quantity * unit_burden must not overflow"),
        "{message}"
    );
}
