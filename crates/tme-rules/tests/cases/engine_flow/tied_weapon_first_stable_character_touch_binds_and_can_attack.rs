use super::*;

#[test]
fn tied_weapon_first_stable_character_touch_binds_and_can_attack() {
    let mut engine = tied_player_weapon_engine(
        serde_json::json!({"state": "bind_on_first_character_touch"}),
        true,
        2,
    );
    let character_id = engine.world().actors[0]
        .character_id
        .clone()
        .expect("player should have a stable character id");

    let pickup = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "training_knife".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("stable character should pick up tied weapon");
    assert_eq!(
        pickup
            .iter()
            .filter(|event| matches!(event, Event::ItemBound { .. }))
            .count(),
        1
    );
    assert!(matches!(
        &engine.world().item_instances["training_knife"].binding,
        tme_rules::ItemBindingState::Bound {
            character_id: owner
        } if owner == &character_id
    ));

    let attack = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Poke,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("bound owner should use tied weapon");
    assert!(player_attack_roll(&attack).is_some());
    assert!(!attack.iter().any(|event| matches!(
        event,
        Event::ItemRelocated {
            reason: tme_rules::ItemRelocationReason::WeaponFumble,
            ..
        }
    )));
}

#[test]
fn tied_weapon_non_owner_fumbles_before_rng_and_lands_at_attacker_position() {
    let absent_owner = "character:absent:owner";
    let mut tied = tied_player_weapon_engine(
        serde_json::json!({"state": "bound", "character_id": absent_owner}),
        false,
        2,
    );
    let mut unrestricted =
        tied_player_weapon_engine(serde_json::json!({"state": "unrestricted"}), false, 2);

    let fumble = tied
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Poke,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("valid non-owner attack attempt should commit as a fumble");
    let fumbles: Vec<&Event> = fumble
        .iter()
        .filter(|event| {
            matches!(
                event,
                Event::ItemRelocated {
                    reason: tme_rules::ItemRelocationReason::WeaponFumble,
                    ..
                }
            )
        })
        .collect();
    assert_eq!(fumbles.len(), 1);
    assert!(matches!(
        fumbles[0],
        Event::ItemRelocated {
            actor_id,
            item_instance_id,
            from: tme_rules::ItemLocationViewV1::Carried {
                actor_id: holder_id,
                position: tme_rules::CarriedPosition::RightHand,
            },
            to: tme_rules::ItemLocationViewV1::Ground {
                location,
            },
            ..
        } if actor_id == "player"
            && holder_id == "player"
            && item_instance_id == "training_knife"
            && location.level == "room_0"
            && location.position == tme_rules::Coord { x: 1, y: 1 }
    ));
    assert!(!fumble.iter().any(|event| matches!(
        event,
        Event::Attacked { .. } | Event::AttackMissed { .. } | Event::SkillPracticeAwarded { .. }
    )));
    assert!(
        !tied.world().actors[0]
            .carried
            .items
            .contains_key(&tme_rules::CarriedPosition::RightHand)
    );
    assert!(tied.world().ground_items.iter().any(|item| {
        item.item_instance_id == "training_knife"
            && item.location.level == "room_0"
            && item.location.position == tme_rules::Coord { x: 1, y: 1 }
    }));
    assert!(matches!(
        &tied.world().item_instances["training_knife"].binding,
        tme_rules::ItemBindingState::Bound { character_id }
            if character_id.as_str() == absent_owner
    ));

    unrestricted
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("baseline round one should resolve without RNG");
    tied.world_mut()
        .item_instances
        .get_mut("training_knife")
        .expect("tied weapon instance")
        .binding = tme_rules::ItemBindingState::Unrestricted;
    tied.apply_actor_intent(
        &tme_rules::ActorId::from("player"),
        PlayerIntent::MoveItem {
            item_instance_id: "training_knife".to_string(),
            destination: tme_rules::ItemMoveDestination::Carried {
                position: tme_rules::CarriedPosition::RightHand,
            },
        },
    )
    .expect("test should restore the weapon without RNG");
    unrestricted
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("baseline round two should resolve without RNG");

    let after_fumble = tied
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Poke,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("post-fumble attack should resolve");
    let baseline = unrestricted
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Poke,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("baseline attack should resolve");
    assert_eq!(
        player_attack_roll(&after_fumble),
        player_attack_roll(&baseline),
        "tied fumble must not consume combat RNG"
    );
}

#[test]
fn tied_weapon_invalid_target_range_and_readiness_do_not_fumble() {
    let binding = || {
        serde_json::json!({
            "state": "bound",
            "character_id": "character:absent:owner"
        })
    };

    let mut missing_target = tied_player_weapon_engine(binding(), false, 2);
    missing_target
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Poke,
                target_actor_id: "missing".into(),
            },
        )
        .expect_err("missing target should fail before tied use");
    assert_eq!(
        hands_instance_id(&missing_target.world().actors[0]),
        Some("training_knife")
    );
    assert!(missing_target.world().ground_items.is_empty());

    let mut out_of_range = tied_player_weapon_engine(binding(), false, 3);
    out_of_range
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Poke,
                target_actor_id: "mireling".into(),
            },
        )
        .expect_err("out-of-range target should fail before tied use");
    assert_eq!(
        hands_instance_id(&out_of_range.world().actors[0]),
        Some("training_knife")
    );
    assert!(out_of_range.world().ground_items.is_empty());

    let mut not_ready = tied_player_weapon_engine(binding(), false, 2);
    not_ready.world_mut().actors[0].attack_ready_at = LogicalTime::new(2);
    let events = not_ready
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Poke,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("not-ready attack is a committed no-attack action");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::AttackNotReady { .. }))
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::ItemRelocated {
            reason: tme_rules::ItemRelocationReason::WeaponFumble,
            ..
        }
    )));
    assert_eq!(
        hands_instance_id(&not_ready.world().actors[0]),
        Some("training_knife")
    );
}

#[test]
fn seed_7_rolls_match_documented_sequence() {
    let mut rng = tme_rules::DeterministicRng::new(7);

    assert_eq!(rng.roll_d20(), 11);
    assert_eq!(rng.roll_d20(), 2);
    assert_eq!(rng.roll_d20(), 9);
}

#[test]
fn first_room_two_turn_flow_is_deterministic() {
    let mut engine = first_room_engine();

    let turn_one = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("turn one should step");
    assert_eq!(
        turn_one[0],
        Event::ActorReady {
            actor_id: "player".into(),
            actor: "Delver".to_string(),
            kind: ActorKind::Player,
            logical_time: LogicalTime::FIRST,
        }
    );
    assert!(has(
        &turn_one,
        |e| matches!(e, Event::Moved { actor_id, actor, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));
    assert!(has(
        &turn_one,
        |e| matches!(e, Event::Moved { actor_id, actor, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));
    assert!(has(
        &turn_one,
        |e| matches!(e, Event::AttackMissed { attacker_id, attacker, defender_id, defender, roll: 11, .. } if attacker_id.as_str() == "mireling" && attacker.as_str() == "Mireling" && defender_id.as_str() == "player" && defender.as_str() == "Delver")
    ));

    let turn_two = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("turn two should step");
    assert!(has(
        &turn_two,
        |e| matches!(e, Event::AttackMissed { attacker_id, attacker, defender_id, defender, roll: 9, .. } if attacker_id.as_str() == "player" && attacker.as_str() == "Delver" && defender_id.as_str() == "mireling" && defender.as_str() == "Mireling")
    ));

    let final_events = engine.final_events();
    assert_eq!(final_events.len(), 1);
    match &final_events[0] {
        Event::FinalState { actors } => {
            assert_eq!(actors[0].name, "Delver");
            assert_eq!(actors[0].hp, 12);
            assert!(matches!(
                actors[0].life_state,
                tme_rules::view::ActorLifeStateViewV1::Alive
            ));
            assert_eq!(actors[1].name, "Mireling");
            assert_eq!(actors[1].hp, 7);
            assert!(matches!(
                actors[1].life_state,
                tme_rules::view::ActorLifeStateViewV1::Alive
            ));
        }
        event => panic!("expected final state event, got {event:?}"),
    }
}

#[test]
fn movement_into_wall_is_blocked_by_rules() {
    let mut engine = first_room_engine();

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::West]),
        )
        .expect("turn should step");

    assert!(has(
        &events,
        |e| matches!(e, Event::MovementBlocked { actor_id, actor, reason, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && reason.as_str() == "blocked terrain")
    ));
}

#[test]
fn player_spends_budget_across_mixed_terrain_path() {
    let mut engine = terrain_engine();

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("path should resolve");

    assert!(has(
        &events,
        |e| matches!(e, Event::Moved { actor_id, actor, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));
    assert!(has(
        &events,
        |e| matches!(e, Event::Moved { actor_id, actor, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));
    assert!(has(
        &events,
        |e| matches!(e, Event::MovementStarted { actor_id, actor, available_path_points: 3, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));
    assert!(has(
        &events,
        |e| matches!(e, Event::MovementCostPaid { actor_id, actor, direction: Direction::East, terrain, cost: 1, remaining_points: 2, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && terrain.as_str() == "Flagstone")
    ));
    assert!(has(
        &events,
        |e| matches!(e, Event::MovementCostPaid { actor_id, actor, direction: Direction::East, terrain, cost: 2, remaining_points: 0, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && terrain.as_str() == "Scrub")
    ));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .position,
        (3, 1).into()
    );
}

#[test]
fn insufficient_budget_stops_before_entering_costly_tile() {
    let mut engine = terrain_engine();

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East, Direction::East]),
        )
        .expect("path should resolve");

    assert!(has(
        &events,
        |e| matches!(e, Event::MovementBlocked { actor_id, actor, reason, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && reason.as_str() == "insufficient movement points")
    ));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .position,
        (3, 1).into()
    );
}

#[test]
fn blocked_step_does_not_spend_remaining_budget() {
    let mut engine = terrain_engine();

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::South, Direction::East, Direction::East]),
        )
        .expect("path should resolve");

    assert!(has(
        &events,
        |e| matches!(e, Event::MovementBlocked { actor_id, actor, reason, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && reason.as_str() == "blocked terrain")
    ));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .position,
        (1, 2).into()
    );
}

#[test]
fn occupied_destination_spends_cost_and_path_continues() {
    let mut engine = terrain_attack_engine('.');

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East, Direction::South]),
        )
        .expect("path should resolve");

    assert!(has(
        &events,
        |e| matches!(e, Event::MovementCostPaid { actor_id, actor, direction: Direction::East, terrain, cost: 1, remaining_points: 1, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && terrain.as_str() == "Flagstone")
    ));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::Attacked { attacker, defender, .. }
            if attacker == "Delver" && defender == "Mireling"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::Moved { actor, to, .. } if actor == "Delver" && to.position == (3, 2).into()
    )));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .position,
        (3, 2).into()
    );
}

#[test]
fn occupied_destination_still_requires_terrain_budget() {
    let mut engine = terrain_attack_engine(',');

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("path should resolve");

    assert!(has(
        &events,
        |e| matches!(e, Event::MovementBlocked { actor_id, actor, reason, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && reason.as_str() == "insufficient movement points")
    ));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .position,
        (2, 1).into()
    );
}

#[test]
fn moving_into_hostile_tile_is_ordinary_movement() {
    let mut engine = terrain_attack_engine('.');

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("path should resolve");

    assert!(has(
        &events,
        |e| matches!(e, Event::MovementCostPaid { actor_id, actor, direction: Direction::East, terrain, cost: 1, remaining_points: 1, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && terrain.as_str() == "Flagstone")
    ));
    assert!(has(
        &events,
        |e| matches!(e, Event::Moved { actor_id, actor, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::Attacked { attacker, defender, .. }
            if attacker == "Delver" && defender == "Mireling"
    )));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .position,
        (3, 1).into()
    );
}

#[test]
fn same_hex_player_can_attack_on_next_readiness() {
    let mut engine = terrain_attack_engine('.');

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("movement should resolve");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack should resolve");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::Attacked { attacker, defender, .. }
            | Event::AttackMissed { attacker, defender, .. }
            if attacker == "Delver" && defender == "Mireling"
    )));
}

#[test]
fn player_poke_attacks_neighbor_without_engagement() {
    let mut engine = reach_attack_engine(true);

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Poke,
                target_actor_id: "reedling".into(),
            },
        )
        .expect("reach attack should resolve");

    assert!(has(
        &events,
        |e| matches!(e, Event::Attacked { attacker_id, attacker, defender_id, defender, roll: 11, damage: 7, label: tme_rules::DamageLabel::Fatal, defender_hp: 0, .. } if attacker_id.as_str() == "player" && attacker.as_str() == "Delver" && defender_id.as_str() == "reedling" && defender.as_str() == "Reedling")
    ));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Moved { actor, .. } if actor == "Delver"))
    );
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .position,
        (1, 1).into()
    );
}

#[test]
fn player_shoot_attacks_beyond_neighboring_reach_without_engagement() {
    let mut engine = ranged_attack_engine(3, 4);
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Nock)
        .expect("ranged bow should nock");

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Shoot,
                target_actor_id: "reedling".into(),
            },
        )
        .expect("ranged attack should resolve");

    assert!(has(
        &events,
        |e| matches!(e, Event::Attacked { attacker_id, attacker, defender_id, defender, roll: 11, damage: 7, label: tme_rules::DamageLabel::Fatal, defender_hp: 0, .. } if attacker_id.as_str() == "player" && attacker.as_str() == "Delver" && defender_id.as_str() == "reedling" && defender.as_str() == "Reedling")
    ));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::Moved { actor, .. } if actor == "Delver"
    )));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .position,
        (1, 1).into()
    );
}

#[test]
fn player_shoot_rejects_targets_beyond_declared_range() {
    let mut engine = ranged_attack_engine(2, 4);
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Nock)
        .expect("ranged bow should nock");

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Shoot,
                target_actor_id: "reedling".into(),
            },
        )
        .expect_err("out-of-range attack should fail");

    assert!(error.to_string().contains("is out of range"));
}

#[test]
fn ordinary_melee_still_rejects_adjacent_attack_without_engagement() {
    let mut engine = reach_attack_engine(false);

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "reedling".into(),
            },
        )
        .expect_err("ordinary adjacent attack should fail");

    assert!(error.to_string().contains("fight target is out of range"));
}

#[test]
fn player_thrown_attack_releases_weapon_into_defender_hex() {
    let mut engine = thrown_attack_engine(3, 4, 9);

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "reedling".into(),
            },
        )
        .expect("thrown attack should resolve");

    assert!(has(
        &events,
        |e| matches!(e, Event::Attacked { attacker_id, attacker, defender_id, defender, roll: 11, damage: 7, label: tme_rules::DamageLabel::Severe, defender_hp: 2, .. } if attacker_id.as_str() == "player" && attacker.as_str() == "Delver" && defender_id.as_str() == "reedling" && defender.as_str() == "Reedling")
    ));
    assert!(has(
        &events,
        |e| matches!(e, Event::ItemRelocated { item_instance_id, item, actor_id, actor, reason: tme_rules::ItemRelocationReason::Thrown, .. } if item_instance_id.as_str() == "oak_javelin" && item.as_str() == "Oak Javelin" && actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));

    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .unwrap();
    assert_eq!(hands_instance_id(player), None);
    assert_eq!(engine.world().ground_items.len(), 1);
    assert_eq!(
        engine.world().ground_items[0].item_instance_id,
        "oak_javelin"
    );
    assert_eq!(
        engine.world().ground_items[0].location.position,
        (4, 1).into()
    );
}

#[test]
fn killing_throw_lands_weapon_in_dead_defender_hex() {
    let mut engine = thrown_attack_engine(3, 4, 5);

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "reedling".into(),
            },
        )
        .expect("killing throw should resolve");

    assert!(has(
        &events,
        |e| matches!(e, Event::ActorDefeated { actor_id, actor, .. } if actor_id.as_str() == "reedling" && actor.as_str() == "Reedling")
    ));
    assert!(has(
        &events,
        |e| matches!(e, Event::ItemRelocated { item_instance_id, item, actor_id, actor, reason: tme_rules::ItemRelocationReason::Thrown, .. } if item_instance_id.as_str() == "oak_javelin" && item.as_str() == "Oak Javelin" && actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));
    assert_eq!(
        engine.world().ground_items[0].location.position,
        (4, 1).into()
    );
}
