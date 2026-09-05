use super::*;

#[test]
fn event_34_spell_effect_family_shapes_are_exact_and_strict() {
    assert_eq!(EVENT_CONTRACT_VERSION, 41);
    let events = vec![
        Event::TransitionConcealed {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            spell_id: "conceal_door".to_string(),
            spell_name: "Conceal Door".to_string(),
            instance_id: "concealed_transition:1".to_string(),
            location: WorldPosition::new("realm_0", "crypt", Coord { x: 2, y: 3 }),
            remaining_rounds: 4,
        },
        Event::TransitionConcealmentRemoved {
            instance_id: "concealed_transition:1".to_string(),
            source_spell_id: "conceal_door".to_string(),
            source_actor_id: "player".into(),
            location: WorldPosition::new("realm_0", "crypt", Coord { x: 2, y: 3 }),
            reason: TransitionConcealmentRemovalReasonV1::Opened,
        },
        Event::BanishEvaluated {
            caster_id: "player".into(),
            caster: "Wiz".to_string(),
            spell_id: "banish".to_string(),
            spell_name: "Banish".to_string(),
            target_id: "demon".into(),
            target: "Summoned Demon".to_string(),
            eligible_trait: Some(CreatureTrait::Demon),
            owned_by_caster: true,
            success: true,
            reason: BanishResultReasonV1::Banished,
        },
        Event::ActorBanished {
            caster_id: "player".into(),
            caster: "Wiz".to_string(),
            spell_id: "banish".to_string(),
            spell_name: "Banish".to_string(),
            actor_id: "demon".into(),
            actor: "Summoned Demon".to_string(),
            instance_id: "summon:1".into(),
            owner_id: "player".into(),
            template_id: "demon_template".to_string(),
            location: WorldPosition::new("realm_0", "crypt", Coord { x: 4, y: 1 }),
        },
        Event::TurnUndeadResolved {
            caster_id: "player".into(),
            caster: "Wiz".to_string(),
            spell_id: "turn_undead".to_string(),
            spell_name: "Turn Undead".to_string(),
            considered_actor_ids: vec!["skeleton".into(), "wight".into()],
            moved_actor_ids: vec!["skeleton".into()],
            blocked_actor_ids: vec!["wight".into()],
        },
        Event::RaiseDeadEvaluated {
            caster_id: "player".into(),
            caster: "Wiz".to_string(),
            spell_id: "raise_dead".to_string(),
            spell_name: "Raise Dead".to_string(),
            corpse_id: Some(CorpseId::parse("corpse:7").expect("valid corpse id")),
            target_actor_id: Some("fallen_player".into()),
            magic_level: 6,
            roll_denominator: 20,
            success_threshold: 6,
            roll: Some(4),
            success: true,
            reason: RaiseDeadResultReasonV1::Resurrected,
        },
    ];
    let value = serde_json::to_value(&events).expect("effect-family events serialize");
    assert_eq!(
        value,
        json!([
            {"transition_concealed": {
                "actor_id": "player", "actor": "Wiz",
                "spell_id": "conceal_door", "spell_name": "Conceal Door",
                "instance_id": "concealed_transition:1",
                "location": {"realm": "realm_0", "level": "crypt", "position": {"x": 2, "y": 3}},
                "remaining_rounds": 4
            }},
            {"transition_concealment_removed": {
                "instance_id": "concealed_transition:1", "source_spell_id": "conceal_door",
                "source_actor_id": "player",
                "location": {"realm": "realm_0", "level": "crypt", "position": {"x": 2, "y": 3}},
                "reason": "opened"
            }},
            {"banish_evaluated": {
                "caster_id": "player", "caster": "Wiz", "spell_id": "banish",
                "spell_name": "Banish", "target_id": "demon", "target": "Summoned Demon",
                "eligible_trait": "demon", "owned_by_caster": true,
                "success": true, "reason": "banished"
            }},
            {"actor_banished": {
                "caster_id": "player", "caster": "Wiz", "spell_id": "banish",
                "spell_name": "Banish", "actor_id": "demon", "actor": "Summoned Demon",
                "instance_id": "summon:1", "owner_id": "player",
                "template_id": "demon_template",
                "location": {"realm": "realm_0", "level": "crypt", "position": {"x": 4, "y": 1}}
            }},
            {"turn_undead_resolved": {
                "caster_id": "player", "caster": "Wiz", "spell_id": "turn_undead",
                "spell_name": "Turn Undead", "considered_actor_ids": ["skeleton", "wight"],
                "moved_actor_ids": ["skeleton"], "blocked_actor_ids": ["wight"]
            }},
            {"raise_dead_evaluated": {
                "caster_id": "player", "caster": "Wiz", "spell_id": "raise_dead",
                "spell_name": "Raise Dead", "corpse_id": "corpse:7",
                "target_actor_id": "fallen_player", "magic_level": 6,
                "roll_denominator": 20, "success_threshold": 6, "roll": 4,
                "success": true, "reason": "resurrected"
            }}
        ])
    );

    for event in events {
        let encoded = serde_json::to_value(&event).expect("serialize event");
        let decoded: Event = serde_json::from_value(encoded).expect("round-trip event");
        assert_eq!(decoded, event);
    }
    let mut extra = value[2].clone();
    extra["banish_evaluated"]["summary"] = json!("legacy prose");
    assert!(serde_json::from_value::<Event>(extra).is_err());
}

#[test]
fn event_36_magic_learning_attempt_practice_reward_and_recovery_shapes_are_exact() {
    let events = vec![
        Event::SpellLearned {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            spell_id: "spark".to_string(),
            spell_name: "Spark".to_string(),
            lane: "wizard_magic".to_string(),
            skill_requirement: 1,
            learned_at_level: 2,
            gold_cost: 25,
            trainer_service_id: "trainer".to_string(),
            trainer: "Trainer".to_string(),
            spell_book_item_instance_id: "book_instance".to_string(),
            spell_book_item_definition_id: "spell_book".to_string(),
            spell_book: "Spell Book".to_string(),
            spell_book_character_id: "character:test:player".to_string(),
        },
        Event::ThaumAboveSkillEvaluated {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            spell_id: "spark".to_string(),
            spell_name: "Spark".to_string(),
            track_id: "thaumaturge_magic".to_string(),
            current_skill_level: 1,
            skill_requirement: 3,
            gap: 2,
            roll_denominator: 20,
            success_threshold: 18,
            roll: 17,
            success: true,
        },
        Event::MagicPracticeEvaluated {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            current_class_id: "wizard".to_string(),
            spell_id: "spark".to_string(),
            spell_name: "Spark".to_string(),
            track_id: "wizard_magic".to_string(),
            mp_cost: 3,
            cast_class: SpellCastClass::Character,
            primary_attribute: Some(MagicPrimaryAttribute::Intelligence),
            primary_attribute_value: Some(14),
            base_raw_points: 3,
            primary_attribute_bonus_raw_points: 1,
            total_raw_points: 4,
            risk_applied: false,
            reason: "eligible_successful_cast".to_string(),
        },
        Event::DefeatRewardEvaluated {
            target_id: "target".into(),
            target: "Target".to_string(),
            authored_experience: 13,
            actual_damage: 3,
            weighted_damage_numerator: 6,
            weighted_damage_denominator: 5,
            available_experience: 5,
            awarded_experience: 5,
            reason: "contribution_shared".to_string(),
        },
        Event::ResourceRegenerated {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            resource: ResourceKind::Mp,
            activity: ResourceActivity::Inactive,
            boundary_at: LogicalTime::new(2),
            base_amount: 3,
            multiplier_numerator: 3,
            multiplier_denominator: 2,
            rounding: MagicArithmeticRounding::Down,
            modifier_item_instance_id: Some("robe_instance".to_string()),
            modifier_item_definition_id: Some("robe".to_string()),
            modifier_item: Some("Recovery Robe".to_string()),
            modifier_item_position: Some(CarriedPosition::OuterArmor),
            amount: 4,
            current: 6,
            maximum: 20,
        },
        Event::SpellCastFailed {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            spell_id: "spark".to_string(),
            spell_name: "Spark".to_string(),
            target: Some(SpellTarget::Actor {
                actor_id: "target".into(),
            }),
            failure: SpellCastFailure::AboveSkillAttempt,
            mp_cost: Some(3),
            stamina_cost: Some(1),
        },
    ];
    let value = serde_json::to_value(&events).expect("DX events serialize");
    assert_eq!(
        value[0]["spell_learned"],
        json!({
            "actor_id": "player", "actor": "Wiz",
            "spell_id": "spark", "spell_name": "Spark",
            "lane": "wizard_magic", "skill_requirement": 1,
            "learned_at_level": 2, "gold_cost": 25,
            "trainer_service_id": "trainer", "trainer": "Trainer",
            "spell_book_item_instance_id": "book_instance",
            "spell_book_item_definition_id": "spell_book",
            "spell_book": "Spell Book",
            "spell_book_character_id": "character:test:player"
        })
    );
    assert_eq!(
        value[1]["thaum_above_skill_evaluated"],
        json!({
            "actor_id": "player", "actor": "Wiz",
            "spell_id": "spark", "spell_name": "Spark",
            "track_id": "thaumaturge_magic", "current_skill_level": 1,
            "skill_requirement": 3, "gap": 2, "roll_denominator": 20,
            "success_threshold": 18, "roll": 17, "success": true
        })
    );
    assert_eq!(value[2]["magic_practice_evaluated"]["total_raw_points"], 4);
    assert_eq!(
        value[3]["defeat_reward_evaluated"],
        json!({
            "target_id": "target", "target": "Target",
            "authored_experience": 13, "actual_damage": 3,
            "weighted_damage_numerator": 6, "weighted_damage_denominator": 5,
            "available_experience": 5, "awarded_experience": 5,
            "reason": "contribution_shared"
        })
    );
    assert_eq!(
        value[4]["resource_regenerated"]["modifier_item_position"],
        "outer_armor"
    );
    assert_eq!(
        value[5]["spell_cast_failed"]["failure"],
        json!({"kind": "above_skill_attempt"})
    );

    for event in events {
        let encoded = serde_json::to_value(&event).expect("serialize current Event");
        let decoded: Event =
            serde_json::from_value(encoded.clone()).expect("round trip current Event");
        assert_eq!(decoded, event);
        let variant = encoded
            .as_object()
            .expect("event object")
            .keys()
            .next()
            .expect("event variant")
            .clone();
        let mut unknown = encoded;
        unknown[&variant]["legacy"] = json!(true);
        assert!(serde_json::from_value::<Event>(unknown).is_err());
    }
}

#[test]
fn event_34_spell_save_receipt_shape_is_exact_and_rejects_predecessors() {
    let event = Event::SpellSaveResolved {
        actor_id: "target".into(),
        actor: "Target".to_string(),
        location: WorldPosition::new("realm_0", "test_room", Coord { x: 2, y: 1 }),
        effect_id: "spark".to_string(),
        resistance_tag: "arcane".to_string(),
        natural_save_twentieths: 5,
        matching_bonus_twentieths: 4,
        selected_boost_source_kind: Some(ResistanceBoostSourceKind::EquippedItem),
        selected_boost_source_id: Some("ring:ward".to_string()),
        denominator: 20,
        save_twentieths: 9,
        roll: 9,
        success: true,
        mitigation_mode: Some(SpellResistanceMitigationMode::HalfDamage),
        requested_damage: Some(5),
        resolved_damage: Some(2),
    };
    let value = serde_json::to_value(&event).expect("save event serializes");
    assert_eq!(
        value,
        json!({
            "spell_save_resolved": {
                "actor_id": "target",
                "actor": "Target",
                "location": {
                    "realm": "realm_0",
                    "level": "test_room",
                    "position": {"x": 2, "y": 1}
                },
                "effect_id": "spark",
                "resistance_tag": "arcane",
                "natural_save_twentieths": 5,
                "matching_bonus_twentieths": 4,
                "selected_boost_source_kind": "equipped_item",
                "selected_boost_source_id": "ring:ward",
                "denominator": 20,
                "save_twentieths": 9,
                "roll": 9,
                "success": true,
                "mitigation_mode": "half_damage",
                "requested_damage": 5,
                "resolved_damage": 2
            }
        })
    );

    for stale in [
        json!({"effect_resisted": {}}),
        {
            let mut missing = value.clone();
            missing["spell_save_resolved"]
                .as_object_mut()
                .expect("save body")
                .remove("roll");
            missing
        },
        {
            let mut unknown = value.clone();
            unknown["spell_save_resolved"]["resistance_tags"] = json!(["arcane"]);
            unknown
        },
    ] {
        assert!(serde_json::from_value::<Event>(stale).is_err());
    }
}

#[test]
fn event_25_rejects_old_prepared_variants_string_causes_and_unknown_fields() {
    for stale in [
        json!({"spell_prepared": {}}),
        json!({"prepared_spell_released": {}}),
        json!({"prepared_spell_cleared": {}}),
    ] {
        assert!(serde_json::from_value::<Event>(stale).is_err());
    }
    assert!(
        serde_json::from_value::<Event>(json!({
            "spell_fizzled": {
                "actor_id": "player", "actor": "Wiz",
                "spell_id": "charged_path", "spell_name": "Charged Path",
                "cause": "damage"
            }
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<Event>(json!({
            "spell_cast_failed": {
                "actor_id": "player", "actor": "Wiz",
                "spell_id": "charged_path", "spell_name": "Charged Path",
                "target": {"path": {"directions": ["west"]}},
                "failure": {"kind": "invalid_path", "reason": "out_of_bounds", "summary": "old"},
                "mp_cost": 4, "stamina_cost": 2
            }
        }))
        .is_err()
    );
}

#[test]
fn movement_events_include_actor_id() {
    let mut engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("start");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::South]),
        )
        .expect("step");

    let moved = events
        .events
        .iter()
        .find(|e| matches!(e, Event::Moved { .. }));
    assert!(moved.is_some(), "must have a Moved event");
    if let Some(Event::Moved {
        actor_id, actor, ..
    }) = moved
    {
        assert!(!actor_id.is_empty(), "actor_id must be non-empty");
        assert!(!actor.is_empty(), "display name still present");
    }
}

#[test]
fn movement_events_include_world_positions() {
    let mut engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("start");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::South]),
        )
        .expect("step");

    for event in &events.events {
        match event {
            Event::Moved { from, to, .. } => {
                assert!(from.same_site(to), "directional move stays on one site");
            }
            Event::MovementBlocked {
                from, attempted, ..
            } => {
                assert!(from.same_site(attempted), "blocked move names one site");
            }
            _ => {}
        }
    }
}

#[test]
fn combat_events_include_attacker_defender_ids() {
    let mut engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("start");
    // Move toward the monster, then attack
    let _ = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .ok();
    let _ = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .ok();
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("step");

    let attacked = events
        .events
        .iter()
        .find(|e| matches!(e, Event::Attacked { .. }));
    if let Some(Event::Attacked {
        attacker_id,
        defender_id,
        attacker,
        defender,
        ..
    }) = attacked
    {
        assert!(!attacker_id.is_empty(), "attacker_id must be present");
        assert!(!defender_id.is_empty(), "defender_id must be present");
        assert_ne!(
            attacker_id, defender_id,
            "attacker and defender must have different IDs"
        );
        // Display names still present
        assert!(!attacker.is_empty());
        assert!(!defender.is_empty());
    }
}

#[test]
fn item_events_include_explicit_item_identity() {
    let mut engine = ContentParts::tracked("supply_cache", "profile/supply_cache")
        .engine(7)
        .expect("start");
    // Take an item
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "hemp_rope".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("step");

    let taken = events
        .events
        .iter()
        .find(|e| matches!(e, Event::ItemRelocated { .. }));
    assert!(taken.is_some(), "must have ItemRelocated event");
    if let Some(Event::ItemRelocated {
        item_instance_id,
        item_definition_id,
        item,
        ..
    }) = taken
    {
        assert!(
            !item_instance_id.is_empty(),
            "item_instance_id must be non-empty"
        );
        assert!(
            !item_definition_id.is_empty(),
            "item_definition_id must be non-empty"
        );
        assert!(!item.is_empty(), "display name still present");
    }
}

#[test]
fn item_transfer_events_serialize_explicit_instance_definition_and_stack_quantity() {
    let mut engine = item_contract_engine();
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "tonic_a".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("taking the tonic stack should succeed");

    let payload = event_payload(&events.events, "item_relocated");
    assert_eq!(payload["item_instance_id"], "tonic_a");
    assert_eq!(payload["item_definition_id"], "restorative_tonic");
    assert_eq!(payload["quantity"], 2);
    assert!(
        payload.get("item_id").is_none(),
        "serialized exact-item events must not retain ambiguous item_id"
    );
}

#[test]
fn item_consumption_events_serialize_consumed_and_numeric_remaining_quantities() {
    let mut engine = item_contract_engine();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "tonic_a".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("taking the tonic stack should succeed");

    let first = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("tonic_a".to_string()),
        )
        .expect("drinking one unit should succeed");
    let first_payload = event_payload(&first.events, "item_consumed");
    assert_eq!(first_payload["item_instance_id"], "tonic_a");
    assert_eq!(first_payload["item_definition_id"], "restorative_tonic");
    assert_eq!(first_payload["quantity_consumed"], 1);
    assert_eq!(first_payload["remaining_quantity"], 1);
    assert_eq!(first_payload["reason"], "drink");
    assert!(first_payload.get("item_id").is_none());

    let second = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("tonic_a".to_string()),
        )
        .expect("drinking the final unit should succeed");
    let second_payload = event_payload(&second.events, "item_consumed");
    assert_eq!(second_payload["quantity_consumed"], 1);
    assert_eq!(
        second_payload["remaining_quantity"], 0,
        "destroyed stacks still report numeric zero"
    );
}

#[test]
fn item_consumption_reasons_serialize_with_exact_names() {
    assert_eq!(
        serde_json::to_value(ItemConsumptionReason::Drink).expect("drink reason should serialize"),
        json!("drink")
    );
}

#[test]
fn event_34_item_consumed_requires_a_reason() {
    let error = serde_json::from_value::<Event>(json!({
        "item_consumed": {
            "actor_id": "player",
            "actor": "Delver",
            "item_instance_id": "tonic_a",
            "item_definition_id": "restorative_tonic",
            "item": "Restorative Tonic",
            "quantity_consumed": 1,
            "remaining_quantity": 0,
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 1, "y": 1}
            }
        }
    }))
    .expect_err("item_consumed without reason must fail deserialization");

    assert!(
        error.to_string().contains("missing field `reason`"),
        "unexpected error: {error}"
    );
}

#[test]
fn item_enchantment_event_serializes_explicit_item_and_enchantment_identity() {
    let mut parts = item_contract_parts();
    let character = &mut parts.actors_mut()[0]["character"];
    character["identity"]["base_class_id"] = json!("wizard");
    character["identity"]["current_class_id"] = json!("wizard");
    character["identity"]["display_class"] = json!("Wizard");
    character["resources"]["mp"] = json!(20);
    character["resources"]["max_mp"] = json!(20);
    character["skill_ledger"] = json!([{"track_id": "wizard_magic", "level": 1, "critique_rank": 0, "practice_points": 0, "learning_rate": 1}]);
    character["known_spells"] =
        json!([{"spell_id": "keen_edge", "lane": "wizard_magic", "learned_at_level": 1}]);
    let wizard_growth =
        parts.catalog["rules_profiles"]["rules/first_room"]["progression"]["growth_profiles"]
            .as_array()
            .expect("first-room growth profiles")
            .iter()
            .find(|profile| profile["class_id"] == "wizard")
            .expect("clean wizard profile")
            .clone();
    parts.rules_source_mut()["progression"]["growth_profiles"]
        .as_array_mut()
        .expect("growth profiles")
        .push(wizard_growth);
    let mut blade =
        parts.catalog["items"]["item/utility_blade/utility_door_secret_item_spells"].clone();
    blade["id"] = json!("training_blade");
    blade["name"] = json!("Training Blade");
    blade["economy"]["unit_value_gold"] = json!(10);
    blade["weapon"]["attack_modes"][0]["damage_kind"] = json!("piercing");
    parts.push_selected("items", "item/training_blade/event_contract", blade);
    parts.profile_value_mut()["spells"]
        .as_array_mut()
        .expect("selected spells")
        .push(json!("spell/keen_edge/utility_door_secret_item_spells"));
    parts.item_instances_mut()["blade_a"] = json!({
        "definition_id": "training_blade",
        "binding": {"state": "unrestricted"}
    });
    parts.actors_mut()[0]["carried"]["items"] =
        json!([{"item_instance_id": "blade_a", "position": "right_hand"}]);
    let mut engine = parts.engine(7).expect("engine should start");
    let command: PlayerCommandV1 = serde_json::from_value(json!({
        "contract_version": COMMAND_CONTRACT_VERSION,
        "actor_id": "player",
        "intent": {
            "cast_spell": {
                "spell_id": "keen_edge",
                "authorization": "safe",
                "target": {
                    "item": {
                        "item_instance_id": "blade_a",
                        "location": "active_equipment"
                    }
                }
            }
        }
    }))
    .expect("current item target command should deserialize");
    let intent = engine
        .command_to_actor_intent(&command)
        .expect("command should convert to an intent");
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), intent)
        .expect("enchantment should apply");

    let payload = event_payload(&events.events, "item_enchanted");
    assert_eq!(payload["item_instance_id"], "blade_a");
    assert_eq!(payload["item_definition_id"], "training_blade");
    assert_eq!(
        payload["enchantment_instance_id"],
        "spell:keen_edge:3000ms:blade_a"
    );
    assert!(payload.get("item_id").is_none());
    assert!(payload.get("instance_id").is_none());
}

#[test]
fn final_state_actor_summary_has_id() {
    let engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("start");
    let final_events = engine.final_events();

    if let Some(Event::FinalState { actors }) = final_events.first() {
        for actor in actors {
            assert!(!actor.id.is_empty(), "ActorSummary must have id");
            assert!(!actor.name.is_empty(), "ActorSummary must have name");
        }
    }
}

#[test]
fn trace_json_includes_new_event_fields() {
    // Trace JSON round-trip: the new fields should appear in serialized output
    let mut engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("start");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::South]),
        )
        .expect("step");

    let serialized = serde_json::to_string(&events.events).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&serialized).expect("deserialize");

    // Find a moved event and check it has actor_id
    let moved_event = parsed
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v.as_object().is_some_and(|o| o.contains_key("moved")));
    assert!(moved_event.is_some(), "trace must have a moved event");
    if let Some(me) = moved_event {
        let moved_obj = &me["moved"];
        assert!(
            moved_obj["actor_id"].is_string(),
            "serialized Moved must include actor_id"
        );
        assert!(moved_obj["from"].is_object());
        assert!(moved_obj["to"].is_object());
        assert!(!moved_obj["navigation"].is_null());
    }
}
