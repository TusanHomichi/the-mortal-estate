use super::*;

#[test]
fn summon_item_instances_use_deterministic_expanded_ids() {
    let mut engine = summon_item_engine(|value| {
        value.selected_mut("spells", 0)["effect"]["duration"]["rounds"] = serde_json::json!(4);
    });

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("first summon should succeed");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 3, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("second summon should succeed");

    let world = engine.world();
    let first_item_instance_id = "summon:call_echo:1:echo_guard:item:focus";
    let second_item_instance_id = "summon:call_echo:2:echo_guard:item:focus";
    assert!(world.item_instances.contains_key(first_item_instance_id));
    assert!(world.item_instances.contains_key(second_item_instance_id));
    for (actor_id, item_instance_id) in [
        ("summon:call_echo:1:echo_guard", first_item_instance_id),
        ("summon:call_echo:2:echo_guard", second_item_instance_id),
    ] {
        let actor = world
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .expect("summoned actor should exist");
        assert!(actor.character_id.is_none());
        assert_eq!(actor.carried.items.len(), 1);
        assert_eq!(
            actor
                .carried
                .items
                .get(&tme_rules::CarriedPosition::RightHand)
                .map(String::as_str),
            Some(item_instance_id)
        );
    }
}

#[test]
fn summoned_bow_instances_are_initialized_from_the_weapon_definition() {
    let mut engine = summon_item_engine(|value| {
        value.selected_mut("items", 0)["weapon"]["skill_track_id"] = serde_json::json!("bow");
        value.selected_mut("items", 0)["weapon"]["default_attack_mode"] =
            serde_json::json!("shoot");
        value.selected_mut("items", 0)["weapon"]["attack_modes"] = serde_json::json!([{
            "mode": "shoot",
            "maximum_range": 3,
            "damage_kind": "piercing"
        }]);
        value.selected_mut("items", 0)["weapon"]["handedness"] = serde_json::json!("bow");
        value.selected_mut("items", 0)["weapon"]["nocking"] =
            serde_json::json!({"unloads_on_movement": true});
    });
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("bow-bearing summon should spawn");
    let item_instance_id = "summon:call_echo:1:echo_guard:item:focus";
    let readiness = engine.world().item_instances[item_instance_id]
        .bow_readiness
        .expect("summoned bow must have readiness state");
    if readiness == tme_rules::BowReadiness::Nocked {
        assert!(events.iter().any(|event| matches!(
            event,
            Event::BowReadinessChanged {
                item_instance_id: changed_id,
                from: tme_rules::BowReadiness::Unnocked,
                to: tme_rules::BowReadiness::Nocked,
                ..
            } if changed_id == item_instance_id
        )));
    }
}

#[test]
fn summon_item_expiry_destroys_still_owned_instance() {
    let mut engine = summon_item_engine(|_| {});
    let actor_id = "summon:call_echo:1:echo_guard";
    let item_instance_id = "summon:call_echo:1:echo_guard:item:focus";

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("summon should succeed");
    assert!(engine.world().item_instances.contains_key(item_instance_id));

    let expiry_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("second wait should expire summon");

    assert!(expiry_events.iter().any(
        |event| matches!(event, Event::SummonExpired { actor_id: expired_id, .. } if expired_id == actor_id)
    ));
    assert!(!engine.world().item_instances.contains_key(item_instance_id));
    assert!(
        !engine
            .world()
            .ground_items
            .iter()
            .any(|item| item.item_instance_id == item_instance_id)
    );
}

#[test]
fn summon_item_defeat_drop_survives_later_expiry() {
    let mut engine = summon_item_engine(|value| {
        value.summon_actor_definition_mut(0)["social"]["alignment_source"] =
            serde_json::json!({"kind": "inherent", "alignment": "chaotic"});
        value.summon_actor_definition_mut(0)["stats"]["hp"] = serde_json::json!(1);
        value.selected_mut("spells", 0)["effect"]["duration"]["rounds"] = serde_json::json!(3);
        equip_one_handed_ranged_weapon(value, "player", "player_test_bow", 3);
    });
    let actor_id = "summon:call_echo:1:echo_guard";
    let item_instance_id = "summon:call_echo:1:echo_guard:item:focus";
    let defeat_position = Coord { x: 2, y: 1 };

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", defeat_position),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("hostile summon should succeed");
    {
        engine
            .world_mut()
            .actors
            .iter_mut()
            .find(|actor| actor.id == actor_id)
            .and_then(|actor| actor.summoned.as_mut())
            .expect("summon authority")
            .owner_id = "external_owner".into();
        let player = engine
            .world_mut()
            .actors
            .iter_mut()
            .find(|actor| actor.id == "player")
            .expect("player");
        player.stats.attack = 20;
    }

    let defeat_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: actor_id.into(),
            },
        )
        .expect("player attack should defeat summon");

    assert!(defeat_events.iter().any(
        |event| matches!(event, Event::ActorDefeated { actor_id: defeated_id, .. } if defeated_id == actor_id)
    ));
    assert!(defeat_events.iter().any(|event| matches!(
        event,
        Event::ItemRelocated {
            actor_id: defeated_id,
            item_instance_id: dropped_item_instance_id,
            to: tme_rules::ItemLocationViewV1::Ground { location },
            reason: tme_rules::ItemRelocationReason::DeathDrop,
            ..
        } if defeated_id == actor_id
            && dropped_item_instance_id == item_instance_id
            && location == &WorldPosition::new("realm_0", "start", defeat_position)
    )));
    let defeated = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == actor_id)
        .expect("defeated summon remains until expiry");
    assert!(defeated.carried.items.is_empty());
    assert!(engine.world().item_instances.contains_key(item_instance_id));
    assert!(engine.world().ground_items.iter().any(|item| {
        item.item_instance_id == item_instance_id
            && item.location == WorldPosition::new("realm_0", "start", defeat_position)
    }));

    let expiry_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("second post-defeat wait should expire summon actor");

    assert!(expiry_events.iter().any(
        |event| matches!(event, Event::SummonExpired { actor_id: expired_id, .. } if expired_id == actor_id)
    ));
    assert!(engine.world().item_instances.contains_key(item_instance_id));
    assert!(engine.world().ground_items.iter().any(|item| {
        item.item_instance_id == item_instance_id
            && item.location == WorldPosition::new("realm_0", "start", defeat_position)
    }));
}

#[test]
fn summon_item_same_round_defeat_and_expiry_preserve_all_drops() {
    let mut engine = summon_item_engine(|value| {
        value.summon_actor_definition_mut(0)["social"]["alignment_source"] =
            serde_json::json!({"kind": "inherent", "alignment": "chaotic"});
        value.summon_actor_definition_mut(0)["stats"]["hp"] = serde_json::json!(1);
        value.selected_mut("summon_templates", 0)["item_instances"]["token"] = serde_json::json!({
            "definition_id": "echo_focus",
            "binding": {"state": "unrestricted"}
        });
        value.selected_mut("summon_templates", 0)["carried"]["items"]
            .as_array_mut()
            .expect("summon carried items")
            .push(serde_json::json!({
                "item_instance_id": "token",
                "position": "sack_item_1"
            }));
        value.selected_mut("spells", 0)["effect"]["duration"]["rounds"] = serde_json::json!(2);
        equip_one_handed_ranged_weapon(value, "player", "player_test_bow", 3);
    });
    let actor_id = "summon:call_echo:1:echo_guard";
    let inventory_item_instance_id = "summon:call_echo:1:echo_guard:item:token";
    let equipment_item_instance_id = "summon:call_echo:1:echo_guard:item:focus";
    let defeat_position = Coord { x: 2, y: 1 };

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", defeat_position),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("hostile summon should succeed");
    {
        engine
            .world_mut()
            .actors
            .iter_mut()
            .find(|actor| actor.id == actor_id)
            .and_then(|actor| actor.summoned.as_mut())
            .expect("summon authority")
            .owner_id = "external_owner".into();
        let player = engine
            .world_mut()
            .actors
            .iter_mut()
            .find(|actor| actor.id == "player")
            .expect("player");
        player.stats.attack = 20;
    }

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: actor_id.into(),
            },
        )
        .expect("defeat and expiry should resolve in one round");

    let died_index = events
        .iter()
        .position(|event| matches!(event, Event::ActorDefeated { actor_id: defeated_id, .. } if defeated_id == actor_id))
        .expect("summon defeat event");
    let dropped = events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| match event {
            Event::ItemRelocated {
                actor_id: defeated_id,
                item_instance_id,
                reason: tme_rules::ItemRelocationReason::DeathDrop,
                ..
            } if defeated_id == actor_id => Some((index, item_instance_id.as_str())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(dropped.len(), 2);
    assert_eq!(dropped[0].1, equipment_item_instance_id);
    assert_eq!(dropped[1].1, inventory_item_instance_id);
    let expired_index = events
        .iter()
        .position(|event| matches!(event, Event::SummonExpired { actor_id: expired_id, .. } if expired_id == actor_id))
        .expect("same-round summon expiry event");
    assert!(died_index < dropped[0].0);
    assert!(dropped[0].0 < dropped[1].0);
    assert!(dropped[1].0 < expired_index);

    assert!(
        !engine
            .world()
            .actors
            .iter()
            .any(|actor| actor.id == actor_id)
    );
    for item_instance_id in [inventory_item_instance_id, equipment_item_instance_id] {
        assert!(engine.world().item_instances.contains_key(item_instance_id));
        assert!(engine.world().ground_items.iter().any(|item| {
            item.item_instance_id == item_instance_id
                && item.location == WorldPosition::new("realm_0", "start", defeat_position)
        }));
    }
}

#[test]
fn seeded_effect_emits_applied_initial_event() {
    let engine = status_engine();
    assert!(engine.initial_events().iter().any(|event| {
        matches!(event, Event::EffectApplied {
            actor_id,
            instance_id,
            effect_id,
            kind,
            ..
        } if actor_id == "player"
            && instance_id == "rooted_1"
            && effect_id == "generic_root"
            && kind == "control_status")
    }));
}

#[test]
fn active_effect_suppresses_non_passive_action_and_then_expires() {
    let mut engine = status_engine();

    let first = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("suppressed action should consume a round and emit an event");
    assert!(first.iter().any(|event| {
        matches!(event, Event::ActionSuppressedByStatus {
            actor_id,
            intent,
            instance_id,
            ..
        } if actor_id == "player" && intent == "walk east" && instance_id == "rooted_1")
    }));
    assert!(first.iter().any(|event| {
        matches!(event, Event::EffectTicked {
            actor_id,
            instance_id,
            remaining_rounds,
            ..
        } if actor_id == "player" && instance_id == "rooted_1" && *remaining_rounds == Some(1))
    }));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .location
            .position
            .x,
        1
    );

    let second = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("wait should be allowed");
    assert!(second.iter().any(|event| {
        matches!(event, Event::EffectExpired {
            actor_id,
            instance_id,
            ..
        } if actor_id == "player" && instance_id == "rooted_1")
    }));
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .active_effects
            .is_empty()
    );

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("movement should work after expiration");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .location
            .position
            .x,
        2
    );
}

#[test]
fn suppressing_effect_blocks_monster_ability_and_movement() {
    let mut engine = suppressed_monster_ability_engine();

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("suppressed monster round should resolve");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::AutomaticActorDecision {
            actor_id,
            actor,
            decision: AutomaticActorDecisionV1::Suppressed { status },
        } if actor_id == "ember_imp" && actor == "Ember Imp" && status == "stun"
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::SpellDamaged { caster_id, .. } if caster_id == "ember_imp"
    )));
    assert!(!events.iter().any(|event| {
        matches!(
            event,
            Event::Moved { actor, .. } | Event::Attacked { attacker: actor, .. }
                if actor == "Ember Imp"
        )
    }));
    let monster = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "ember_imp")
        .expect("monster");
    assert_eq!(monster.location.position, tme_rules::Coord { x: 4, y: 1 });
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .hp,
        12
    );
}

#[test]
fn command_validation_reports_suppressed_status() {
    let engine = status_engine();
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::MovePath {
            path: vec![Direction::East],
        },
    };
    let status = engine.validate_actor_command(&command).expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::SuppressedByStatus)
    );
}

#[test]
fn path_preview_reports_suppressed_status() {
    let engine = status_engine();
    let preview = engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .expect("preview");
    assert_eq!(preview.stop_reason, MovementStopReason::Blocked);
    let first = preview.steps.first().expect("blocked step");
    assert!(matches!(
        first.outcome,
        PathPreviewStepOutcomeV1::Blocked {
            reason: PathPreviewBlockedReasonV1::SuppressedByStatus
        }
    ));
    assert_eq!(preview.final_position.position.x, 1);
}

#[test]
fn thief_hide_applies_hidden_effect_and_breaks_on_move() {
    let mut engine = thief_hide_engine_on_dark_tile();

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player0"), PlayerIntent::Hide)
        .expect("hide succeeds");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ActorHidden {
            actor_id, effect_id, ..
        } if actor_id == "player0" && effect_id == "hidden"
    )));
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player0"))
            .expect("player")
            .active_effects
            .iter()
            .any(|effect| effect.kind == "hidden" && effect.tags.iter().any(|tag| tag == "hidden"))
    );

    let move_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player0"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("move succeeds");
    assert!(move_events.iter().any(|event| matches!(
        event,
        Event::HideBroken {
            actor_id, reason, ..
        } if actor_id == "player0" && reason == "move"
    )));
    assert!(
        !engine
            .world()
            .actor(&tme_rules::ActorId::from("player0"))
            .expect("player")
            .active_effects
            .iter()
            .any(|effect| effect.kind == "hidden")
    );
}

#[test]
fn thief_hide_survives_blocked_movement_attempt() {
    let mut engine = thief_hide_engine_on_dark_tile();
    engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == "player0")
        .expect("player")
        .location
        .position = tme_rules::Coord { x: 1, y: 1 };

    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player0"), PlayerIntent::Hide)
        .expect("hide succeeds");

    let move_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player0"),
            PlayerIntent::MovePath(vec![Direction::West]),
        )
        .expect("blocked movement returns events");
    assert!(move_events.iter().any(|event| matches!(
        event,
        Event::MovementBlocked {
            actor_id, reason, ..
        } if actor_id == "player0" && reason == "blocked terrain"
    )));
    assert!(!move_events.iter().any(|event| {
        matches!(event, Event::HideBroken { actor_id, .. } if actor_id == "player0")
    }));
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player0"))
            .expect("player")
            .active_effects
            .iter()
            .any(|effect| effect.kind == "hidden")
    );
}

#[test]
fn thief_hide_survives_not_ready_and_no_sight_attack_attempts() {
    let mut not_ready = thief_hide_engine_on_dark_tile_with(|value| {
        equip_one_handed_ranged_weapon(value, "player0", "player_test_bow", 5);
    });
    {
        let player = not_ready
            .world_mut()
            .actors
            .iter_mut()
            .find(|actor| actor.id == "player0")
            .expect("player");
        player.attack_ready_at = tme_rules::LogicalTime::new(99);
    }
    not_ready
        .apply_actor_intent(&tme_rules::ActorId::from("player0"), PlayerIntent::Hide)
        .expect("hide succeeds");

    let attack_events = not_ready
        .apply_actor_intent(
            &tme_rules::ActorId::from("player0"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "watcher".into(),
            },
        )
        .expect("not-ready attack returns events");
    assert!(attack_events.iter().any(|event| {
        matches!(event, Event::AttackNotReady { actor_id, .. } if actor_id == "player0")
    }));
    assert!(!attack_events.iter().any(|event| {
        matches!(event, Event::HideBroken { actor_id, .. } if actor_id == "player0")
    }));
    assert!(
        not_ready
            .world()
            .actor(&tme_rules::ActorId::from("player0"))
            .expect("player")
            .active_effects
            .iter()
            .any(|effect| effect.kind == "hidden")
    );

    let mut no_sight = thief_hide_engine_on_dark_tile_with(|value| {
        equip_one_handed_ranged_weapon(value, "player0", "player_test_bow", 5);
    });
    no_sight
        .apply_actor_intent(&tme_rules::ActorId::from("player0"), PlayerIntent::Hide)
        .expect("hide succeeds");
    no_sight
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == "player0")
        .expect("player")
        .active_effects
        .push(ActiveEffectState {
            spell_damage_credit: None,
            source_actor_id: None,
            hostile_authority: None,
            instance_id: "test:blind:player0".to_string(),
            effect_id: "test_blind".to_string(),
            source: ActiveEffectSource {
                kind: "test".to_string(),
                id: "blind".to_string(),
            },
            kind: "control_status".to_string(),
            tags: vec!["blind".to_string()],
            potency: 0,
            remaining_rounds: Some(3),
            until_condition: None,
            stacking: ActiveEffectStackingPolicy::RefreshDuration,
            start_delay_rounds: 0,
            tick_interval_rounds: 1,
            suppresses_action: false,
            resistance_boosts: vec![],
            last_ticked_at: tme_rules::LogicalTime::new(0),
        });

    let attack_events = no_sight
        .apply_actor_intent(
            &tme_rules::ActorId::from("player0"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "watcher".into(),
            },
        )
        .expect("no-sight attack returns events");
    assert!(attack_events.iter().any(|event| {
        matches!(event, Event::AttackBlockedNoSight { attacker_id, .. } if attacker_id == "player0")
    }));
    assert!(!attack_events.iter().any(|event| {
        matches!(event, Event::HideBroken { actor_id, .. } if actor_id == "player0")
    }));
    assert!(
        no_sight
            .world()
            .actor(&tme_rules::ActorId::from("player0"))
            .expect("player")
            .active_effects
            .iter()
            .any(|effect| effect.kind == "hidden")
    );
}

#[test]
fn actor_resistance_boosts_include_active_effects_and_equipped_items() {
    let mut engine = status_engine();
    engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == "player")
        .expect("player")
        .active_effects
        .first_mut()
        .expect("seeded effect")
        .resistance_boosts = vec![
        tme_rules::SpellResistanceBoost {
            tag: "poison".to_string(),
            bonus_twentieths: 3,
        },
        tme_rules::SpellResistanceBoost {
            tag: "stun".to_string(),
            bonus_twentieths: 4,
        },
        tme_rules::SpellResistanceBoost {
            tag: "poison".to_string(),
            bonus_twentieths: 2,
        },
    ];
    let boosts = engine
        .actor_resistance_boosts(&"player".into())
        .expect("resistance boosts");
    assert_eq!(
        boosts
            .iter()
            .map(|boost| (boost.tag.as_str(), boost.bonus_twentieths))
            .collect::<Vec<_>>(),
        vec![("poison", 3), ("poison", 2), ("stun", 4), ("stun", 3)]
    );
}
