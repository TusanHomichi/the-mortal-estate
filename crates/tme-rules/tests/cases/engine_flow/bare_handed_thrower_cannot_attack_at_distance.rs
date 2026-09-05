use super::*;

#[test]
fn bare_handed_thrower_cannot_attack_at_distance() {
    let mut engine = thrown_attack_engine(3, 4, 9);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "reedling".into(),
            },
        )
        .expect("first throw should resolve");

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "reedling".into(),
            },
        )
        .expect_err("bare-handed distance attack should fail");

    assert!(error.to_string().contains("fight target is out of range"));
}

#[test]
fn same_hex_thrown_attacker_melee_attacks_without_release() {
    let mut engine = thrown_attack_engine(3, 2, 9);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("engaging move should resolve");

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "reedling".into(),
            },
        )
        .expect("engaged attack should resolve");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::Attacked { attacker, .. } | Event::AttackMissed { attacker, .. }
            if attacker == "Delver"
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::ItemRelocated {
            reason: tme_rules::ItemRelocationReason::Thrown,
            ..
        }
    )));
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .unwrap();
    assert_eq!(hands_instance_id(player), Some("oak_javelin"));
    assert!(engine.world().ground_items.is_empty());
}

#[test]
fn thrown_attack_rejects_targets_beyond_declared_range() {
    let mut engine = thrown_attack_engine(2, 4, 9);

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "reedling".into(),
            },
        )
        .expect_err("out-of-range throw should fail");

    assert!(error.to_string().contains("is out of range"));
}

#[test]
fn retrieve_restores_thrown_weapon_from_ground() {
    let mut engine = thrown_attack_engine(3, 2, 9);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "reedling".into(),
            },
        )
        .expect("throw should resolve");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("walk to landing hex should resolve");

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "oak_javelin".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("retrieve should resolve");

    assert!(has(
        &events,
        |e| matches!(e, Event::ItemRelocated { actor_id, actor, item_instance_id, item, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && item_instance_id.as_str() == "oak_javelin" && item.as_str() == "Oak Javelin")
    ));
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .unwrap();
    assert_eq!(hands_instance_id(player), Some("oak_javelin"));
    assert!(engine.world().ground_items.is_empty());
}

#[test]
fn retrieve_rejects_non_weapon_hands_occupancy_without_mutation() {
    let mut engine = non_weapon_hands_with_ground_weapon_engine();
    let carried_before = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .unwrap()
        .carried
        .clone();
    let ground_before = engine.world().ground_items.clone();

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "training_knife".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect_err("occupied hands slot must reject retrieval");

    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .carried,
        carried_before,
        "failed move must not change carried layout"
    );
    assert_eq!(
        engine.world().ground_items,
        ground_before,
        "failed retrieval must leave the weapon on the ground"
    );
}

#[test]
fn retrieve_rejects_a_stacked_ground_weapon() {
    let mut engine = thrown_attack_engine(3, 2, 9);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "reedling".into(),
            },
        )
        .expect("throw should resolve");
    engine
        .world_mut()
        .item_instances
        .get_mut("oak_javelin")
        .expect("thrown instance")
        .quantity = 2;
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("walk to landing hex should resolve");

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "oak_javelin".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect_err("stacked weapon must not become equipment");

    assert!(error.message().contains("quantity 1 outside the sack"));
    assert!(
        engine
            .world()
            .ground_items
            .iter()
            .any(|item| { item.item_instance_id == "oak_javelin" })
    );
    assert_eq!(
        hands_instance_id(
            engine
                .world()
                .actor(&tme_rules::ActorId::from("player"))
                .unwrap()
        ),
        None
    );
}

#[test]
fn retrieve_fails_for_unknown_item() {
    let mut engine = thrown_attack_engine(3, 2, 9);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "reedling".into(),
            },
        )
        .expect("throw should resolve");

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "elm_bow".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect_err("unknown retrieve should fail");

    assert!(
        error
            .to_string()
            .contains("unknown item instance \"elm_bow\"")
    );
}

#[test]
fn retrieve_fails_when_actor_is_not_on_item_hex() {
    let mut engine = thrown_attack_engine(3, 2, 9);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "reedling".into(),
            },
        )
        .expect("throw should resolve");

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "oak_javelin".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect_err("distant retrieve should fail");

    assert!(error.to_string().contains("is not in reach"));
}

#[test]
fn move_fails_when_item_is_already_at_destination() {
    let mut engine = thrown_attack_engine(3, 4, 9);

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "oak_javelin".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect_err("same-position move should fail");

    assert!(
        error
            .to_string()
            .contains("already at the requested destination")
    );
}

#[test]
fn inspect_reports_ground_items_on_current_and_adjacent_hexes() {
    let mut engine = thrown_attack_engine(3, 2, 9);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "reedling".into(),
            },
        )
        .expect("throw should resolve");

    let adjacent_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Inspect)
        .expect("inspect should resolve");
    let adjacent_inspect = adjacent_events
        .iter()
        .find_map(|event| match event {
            Event::Inspected { ground_items, .. } => Some(ground_items.clone()),
            _ => None,
        })
        .expect("inspect event should exist");
    assert_eq!(adjacent_inspect.len(), 1);
    assert_eq!(adjacent_inspect[0].item.name, "Oak Javelin");
    assert_eq!(adjacent_inspect[0].location.position, (2, 1).into());
    assert_eq!(adjacent_inspect[0].direction, Some(Direction::East));

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("walk to landing hex should resolve");
    let here_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Inspect)
        .expect("inspect should resolve");
    let here_inspect = here_events
        .iter()
        .find_map(|event| match event {
            Event::Inspected { ground_items, .. } => Some(ground_items.clone()),
            _ => None,
        })
        .expect("inspect event should exist");
    assert_eq!(here_inspect.len(), 1);
    assert_eq!(here_inspect[0].direction, None);
}

#[test]
fn inspect_reports_local_context_without_moving_player() {
    let mut engine = inspect_edge_room_engine();

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Inspect)
        .expect("inspect turn should step");

    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .position,
        (0, 1).into()
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::Moved { actor, .. } if actor == "Delver"
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::Attacked { attacker, .. } if attacker == "Delver"
    )));

    let inspect_index = events
        .iter()
        .position(|event| matches!(event, Event::Inspected { .. }))
        .expect("inspect event should be emitted");
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::AutomaticActorDecision { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::LogicalTimeAdvanced { .. }))
    );
    assert!(inspect_index > 0);

    let inspected = events
        .iter()
        .find_map(|event| match event {
            Event::Inspected {
                actor,
                location,
                tile,
                tile_move_cost,
                exits,
                nearby_actors,
                ..
            } => Some((actor, location, tile, tile_move_cost, exits, nearby_actors)),
            _ => None,
        })
        .expect("inspect event should be emitted");

    assert_eq!(inspected.0, "Delver");
    assert_eq!(inspected.1.position, (0, 1).into());
    assert_eq!(inspected.2, "Flagstone");
    assert_eq!(*inspected.3, Some(1));
    assert_eq!(inspected.4.len(), 8);
    assert!(inspected.4.iter().any(|exit| {
        exit.direction == Direction::Northeast
            && exit.location.position == (1, 0).into()
            && exit.status == InspectExitStatus::BlockedTerrain
    }));
    assert!(inspected.4.iter().any(|exit| {
        exit.direction == Direction::South
            && exit.terrain.as_deref() == Some("Flagstone")
            && exit.move_cost == Some(1)
            && matches!(exit.status, InspectExitStatus::Walkable)
    }));
    assert_eq!(
        inspected.5,
        &vec![InspectActor {
            location: WorldPosition::new("realm_0", "room_0", (1, 1).into()),
            actor_id: "mireling".into(),
            direction: Direction::East,
            actor: "Mireling".to_string(),
            kind: ActorKind::Monster,
            hp: 7,
            character_identity: None,
        }]
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::Moved { actor_id, .. } if actor_id == "mireling"
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::Attacked { attacker, defender, .. }
            if attacker == "Mireling" && defender == "Delver"
    )));
}

#[test]
fn exact_move_destinations_replace_take_and_retrieve_behaviors() {
    let mut engine = thrown_attack_engine(3, 2, 9);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            tme_rules::PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "reedling".into(),
            },
        )
        .expect("throw should resolve");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            tme_rules::PlayerIntent::MovePath(vec![tme_rules::Direction::East]),
        )
        .expect("move should resolve");

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "oak_javelin".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("take should succeed");

    assert!(has(
        &events,
        |e| matches!(e, Event::ItemRelocated { actor_id, actor, item_instance_id, item, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && item_instance_id.as_str() == "oak_javelin" && item.as_str() == "Oak Javelin")
    ));
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .unwrap();
    assert_eq!(hands_instance_id(player), None);
    assert_eq!(
        player
            .carried
            .items
            .get(&tme_rules::CarriedPosition::SackItem1)
            .map(String::as_str),
        Some("oak_javelin")
    );
    assert!(engine.world().ground_items.is_empty());

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "oak_javelin".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("the same item can move from sack to hand");
    assert_eq!(
        hands_instance_id(
            engine
                .world()
                .actor(&tme_rules::ActorId::from("player"))
                .unwrap()
        ),
        Some("oak_javelin")
    );
}

#[test]
fn inactive_recovery_is_not_suppressed_by_a_nearby_hostile() {
    let mut engine = resource_recovery_engine();
    engine.world_mut().actors[0]
        .resource_activity
        .last_recovered_at = LogicalTime::ZERO;
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("inactive boundary should resolve");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ResourceRegenerated {
            actor_id,
            resource: tme_rules::ResourceKind::Hp,
            activity: tme_rules::ResourceActivity::Inactive,
            amount: 2,
            current: 7,
            ..
        } if actor_id == "player"
    )));
}

#[test]
fn drink_heals_immediately_and_consumes_the_bottle() {
    let mut engine = balm_engine(|_| {});
    share_hex_and_take_hits(&mut engine, 4);
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .hp,
        4
    );

    let events = engine
        .apply_realtime_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("healing_balm".to_string()),
        )
        .expect("round nine should drink the balm");

    assert!(has(
        &events,
        |e| matches!(e, Event::ItemConsumed { actor_id, actor, item_instance_id, item, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && item_instance_id.as_str() == "healing_balm" && item.as_str() == "Healing Balm")
    ));
    assert!(has(
        &events,
        |e| matches!(e, Event::BalmHealed { actor_id, actor, amount: 2, hp: 6, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));
    // After the drink the warden misses, so hp stays at 6.
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .hp,
        6
    );
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .carried
            .items
            .values()
            .all(|item| item != "healing_balm")
    );
}

#[test]
fn healing_balm_consumption_fizzles_before_the_first_tick_and_failures_preserve_slot() {
    let mut engine = balm_engine(|_| {});
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "healing_balm".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("take healing balm");
    let player_index = engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "player")
        .expect("player index");
    engine.world_mut().actors[player_index].warmed_spell = Some(tme_rules::WarmedSpellState {
        spell_id: "balm_interrupted_spell".to_string(),
        warmed_at: LogicalTime::FIRST,
        ready_at: LogicalTime::new(99),
        status: tme_rules::WarmedSpellStatus::Warming,
    });

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("healing_balm".to_string()),
        )
        .expect("drink healing balm");
    let consumed = events
        .iter()
        .position(|event| matches!(event, Event::ItemConsumed { .. }))
        .expect("item consumption event");
    let fizzled = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::SpellFizzled {
                    cause: tme_rules::SpellFizzleCause::HealingBalm,
                    ..
                }
            )
        })
        .expect("balm fizzle event");
    let healed = events
        .iter()
        .position(|event| matches!(event, Event::BalmHealed { .. }))
        .expect("first balm tick");
    assert!(consumed < fizzled && fizzled < healed);
    assert!(engine.world().actors[player_index].warmed_spell.is_none());

    let mut rejected = balm_engine(|_| {});
    let player_index = rejected
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "player")
        .expect("player index");
    rejected.world_mut().actors[player_index].warmed_spell = Some(tme_rules::WarmedSpellState {
        spell_id: "preserved_spell".to_string(),
        warmed_at: LogicalTime::FIRST,
        ready_at: LogicalTime::new(99),
        status: tme_rules::WarmedSpellStatus::Warming,
    });
    rejected
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("missing_balm".to_string()),
        )
        .expect_err("missing drink target rejects");
    assert_eq!(
        rejected.world().actors[player_index]
            .warmed_spell
            .as_ref()
            .expect("rejected drink preserves slot")
            .spell_id,
        "preserved_spell"
    );
}

#[test]
fn balm_ticks_each_boundary_before_the_due_monster_phase() {
    let mut engine = balm_engine(|_| {});
    share_hex_and_take_hits(&mut engine, 4);
    engine
        .apply_realtime_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("healing_balm".to_string()),
        )
        .expect("round nine should drink the balm");

    let events = engine
        .advance_action_interval()
        .expect("round ten should tick the balm before the warden attacks");

    // Balm ticks from 6 to 8 before the warden misses, leaving hp at 8.
    assert!(has(
        &events,
        |e| matches!(e, Event::BalmHealed { actor_id, actor, amount: 2, hp: 8, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));
    let attack_position = events
        .iter()
        .position(|event| matches!(event, Event::AttackMissed { .. }))
        .expect("warden should attack");
    let balm_position = events
        .iter()
        .position(|event| matches!(event, Event::BalmHealed { .. }))
        .expect("balm should tick");
    assert!(balm_position < attack_position);
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .hp,
        8
    );
}

#[test]
fn balm_ticks_before_active_hp_recovery_even_while_engaged() {
    let mut engine = balm_engine(|parts| {
        parts.rules_source_mut()["resources"]["recovery_interval_units"] = serde_json::json!(1);
    });
    share_hex_and_take_hits(&mut engine, 4);
    engine.world_mut().actors[0].hp = 2;
    engine.world_mut().actors[0]
        .character
        .as_mut()
        .unwrap()
        .resources
        .hp = 2;

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("healing_balm".to_string()),
        )
        .expect("round nine should drink while engaged");

    assert!(!balm_events(&events).is_empty());
    let balm_position = events
        .iter()
        .position(|event| matches!(event, Event::BalmHealed { .. }))
        .expect("balm tick");
    let recovery_position = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::ResourceRegenerated {
                    resource: tme_rules::ResourceKind::Hp,
                    ..
                }
            )
        })
        .expect("active hp recovery");
    assert!(balm_position < recovery_position);
}
