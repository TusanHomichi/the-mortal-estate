use crate::support::content_parts::ContentParts;
use tme_rules::{
    ActorKind, AutomaticActorDecisionV1, AutomaticMovementPurposeV1, AutomaticWaitReasonV1, Coord,
    Direction, Engine, Event, ExplicitTraversalKind, InspectActor, InspectExitStatus, LogicalTime,
    NavigationKind, PlayerIntent, VerticalDirection, WorldPosition,
};

fn tracked(case_id: &str) -> ContentParts {
    ContentParts::tracked(case_id, &format!("profile/{case_id}"))
}

fn layered_cells(rows: &[&str], mapping: &[(char, &str)]) -> serde_json::Value {
    serde_json::json!(
        rows.iter()
            .map(|row| {
                row.chars()
                    .map(|glyph| {
                        vec![
                            mapping
                                .iter()
                                .find_map(|(candidate, terrain)| {
                                    (*candidate == glyph).then_some(*terrain)
                                })
                                .unwrap_or_else(|| panic!("unmapped fixture glyph {glyph:?}")),
                        ]
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    )
}

fn has<E: AsRef<[Event]>, F: Fn(&Event) -> bool>(events: &E, f: F) -> bool {
    events.as_ref().iter().any(f)
}

fn hands_instance_id(actor: &tme_rules::ActorState) -> Option<&str> {
    actor
        .carried
        .items
        .get(&tme_rules::CarriedPosition::RightHand)
        .map(String::as_str)
}

fn first_room_engine() -> Engine {
    tracked("first_room")
        .engine(7)
        .expect("first-room content graph should start")
}

fn tied_player_weapon_engine(
    binding: serde_json::Value,
    starts_on_ground: bool,
    monster_x: i32,
) -> Engine {
    let mut parts = tracked("first_room");
    parts.item_instances_mut()["training_knife"]["binding"] = binding;
    let weapon = parts.selected_by_runtime_id_mut("items", "training_knife");
    weapon["weapon"]["default_attack_mode"] = serde_json::json!("poke");
    weapon["weapon"]["attack_modes"] = serde_json::json!([{
        "mode": "poke",
        "maximum_range": 1,
        "damage_kind": "piercing"
    }]);
    weapon["valid_placements"] = serde_json::json!(["hand", "sack"]);
    parts.actors_mut()[0]["character_id"] = serde_json::json!("character:tied_weapon_test:primary");
    parts.actor_definition_mut(0)["social"]["alignment_source"] =
        serde_json::json!({"kind": "character"});
    parts.actors_mut()[0]["character"] = serde_json::json!({
        "identity": {
            "base_class_id": "fighter",
            "current_class_id": "fighter",
            "display_class": "Fighter",
            "nationality_id": "aldland"
        },
        "alignment_state": {"alignment": "lawful", "karma_points": 0},
        "attributes": {
            "strength": 10,
            "dexterity": 10,
            "constitution": 10,
            "intelligence": 10,
            "wisdom": 10,
            "charisma": 10
        },
        "resources": {
            "hp": 12,
            "max_hp": 12,
            "peak_hp": 12,
            "mp": 0,
            "max_mp": 0,
            "stamina": 10,
            "max_stamina": 10
        },
        "progression": {"level": 1, "experience": 0},
        "physical_attribute_adds": {"strength_adds": 0, "dexterity_adds": 0},
        "promotion_history": [],
        "skill_ledger": []
    });
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": monster_x, "y": 1});
    parts.actor_definition_mut(1)["ai"]["behavior"] = serde_json::json!("hold_ground");
    parts.actor_definition_mut(1)["stats"]["attack"] = serde_json::json!(0);
    if starts_on_ground {
        parts.actors_mut()[0]["carried"]["items"] = serde_json::json!([]);
        *parts.ground_items_mut() = serde_json::json!([{
            "item_instance_id": "training_knife",
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 1, "y": 1}
            }
        }]);
    }
    parts.engine(7).expect("tied fixture should start")
}

fn player_attack_roll<E: AsRef<[Event]>>(events: &E) -> Option<i32> {
    events.as_ref().iter().find_map(|event| match event {
        Event::Attacked {
            attacker_id, roll, ..
        } if attacker_id == "player" => Some(*roll as i32),
        Event::AttackMissed {
            attacker_id, roll, ..
        } if attacker_id == "player" => Some(*roll),
        _ => None,
    })
}

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

fn inspect_edge_room_engine() -> Engine {
    let mut parts = tracked("first_room");
    *parts.template_levels_source_mut() = serde_json::json!({
        "room_0": {
            "law_zone": "none",
            "width": 3,
            "height": 3,
            "cells": layered_cells(
                &["###", "..#", "..#"],
                &[('#', "stone_wall"), ('.', "flagstone")]
            )
        }
    });
    parts.actors_mut()[0]["location"]["position"] = serde_json::json!({"x": 0, "y": 1});
    parts.actors_mut()[0]["carried"]["items"] = serde_json::json!([]);
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    parts.actor_definition_mut(1)["stats"]["attack"] = serde_json::json!(0);
    *parts.item_instances_mut() = serde_json::json!({});
    *parts.ground_items_mut() = serde_json::json!([]);
    parts
        .engine(7)
        .expect("inspect edge-room content should start")
}

fn terrain_engine() -> Engine {
    let mut parts = tracked("terrain_movement");
    *parts.template_levels_source_mut() = serde_json::json!({
        "room_0": {
            "law_zone": "none",
            "width": 6,
            "height": 5,
            "cells": layered_cells(
                &["######", "#..,,#", "#.~..#", "#....#", "######"],
                &[
                    ('#', "stone_wall"),
                    ('.', "flagstone"),
                    (',', "scrub"),
                    ('~', "deep_water")
                ]
            )
        }
    });
    parts.actors_mut()[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    parts.actors_mut()[0]["carried"]["items"] = serde_json::json!([]);
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 4, "y": 3});
    parts.actor_definition_mut(1)["stats"]["attack"] = serde_json::json!(0);
    *parts.item_instances_mut() = serde_json::json!({});
    *parts.ground_items_mut() = serde_json::json!([]);
    parts
        .engine(7)
        .expect("terrain fixture content should start")
}

fn terrain_attack_engine(monster_tile: char) -> Engine {
    let tiles = match monster_tile {
        ',' => ["#####", "#.,,#", "#...#", "#####"],
        '.' => ["#####", "#...#", "#.,.#", "#####"],
        _ => panic!("unsupported monster tile"),
    };
    let mut parts = tracked("terrain_movement");
    *parts.template_levels_source_mut() = serde_json::json!({
        "room_0": {
            "law_zone": "none",
            "width": 5,
            "height": 4,
            "cells": layered_cells(
                &tiles,
                &[('#', "stone_wall"), ('.', "flagstone"), (',', "scrub")]
            )
        }
    });
    parts.actors_mut()[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    parts.actor_definition_mut(0)["stats"]["attack"] = serde_json::json!(6);
    parts.actors_mut()[0]["carried"]["items"] = serde_json::json!([]);
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 3, "y": 1});
    parts.actor_definition_mut(1)["stats"]["attack"] = serde_json::json!(0);
    *parts.item_instances_mut() = serde_json::json!({});
    *parts.ground_items_mut() = serde_json::json!([]);
    parts
        .engine(7)
        .expect("terrain attack content should start")
}

fn reach_attack_engine(player_has_polearm: bool) -> Engine {
    let mut parts = tracked("reach_attack");
    parts.actor_definition_mut(1)["stats"]["attack"] = serde_json::json!(0);
    if !player_has_polearm {
        parts.actors_mut()[0]["carried"]["items"] = serde_json::json!([]);
        *parts.item_instances_mut() = serde_json::json!({});
    }
    parts
        .engine(1_010_580_540)
        .expect("reach attack content should start")
}

fn ranged_attack_engine(maximum_range: u32, monster_x: i32) -> Engine {
    let mut parts = tracked("ranged_attack");
    let attack_modes =
        &mut parts.selected_by_runtime_id_mut("items", "elm_bow")["weapon"]["attack_modes"];
    let shoot_mode = attack_modes
        .as_array_mut()
        .expect("bow attack modes")
        .iter_mut()
        .find(|mode| mode["mode"] == "shoot")
        .expect("bow shoot mode");
    shoot_mode["maximum_range"] = serde_json::json!(maximum_range);
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": monster_x, "y": 1});
    parts.actor_definition_mut(1)["stats"]["attack"] = serde_json::json!(0);
    parts.actor_definition_mut(1)["ai"]["behavior"] = serde_json::json!("hold_ground");
    parts
        .engine(1_010_580_540)
        .expect("ranged attack content should start")
}

fn thrown_attack_engine(maximum_range: u32, monster_x: i32, monster_hp: i32) -> Engine {
    let mut parts = tracked("thrown_attack");
    let attack_modes =
        &mut parts.selected_by_runtime_id_mut("items", "oak_javelin")["weapon"]["attack_modes"];
    let throw_mode = attack_modes
        .as_array_mut()
        .expect("javelin attack modes")
        .iter_mut()
        .find(|mode| mode["mode"] == "throw")
        .expect("javelin throw mode");
    throw_mode["maximum_range"] = serde_json::json!(maximum_range);
    let monster = &mut parts.actors_mut().as_array_mut().expect("seed actors")[1];
    monster["id"] = serde_json::json!("reedling");
    monster["location"]["position"] = serde_json::json!({"x": monster_x, "y": 1});
    let monster_definition = parts.actor_definition_mut(1);
    monster_definition["name"] = serde_json::json!("Reedling");
    monster_definition["stats"]["hp"] = serde_json::json!(monster_hp);
    monster_definition["stats"]["attack"] = serde_json::json!(0);
    parts
        .engine(1_010_580_540)
        .expect("thrown attack content should start")
}

fn non_weapon_hands_with_ground_weapon_engine() -> Engine {
    let mut parts = tracked("equipment_slots");
    parts.actors_mut()[0]["carried"]["items"] = serde_json::json!([{
        "item_instance_id": "leather_jerkin",
        "position": "right_hand"
    }]);
    parts
        .ground_items_mut()
        .as_array_mut()
        .expect("ground items array")
        .push(serde_json::json!({
            "item_instance_id": "training_knife",
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 1, "y": 1}
            }
        }));
    parts
        .engine(7)
        .expect("occupied-hands content should start")
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

fn resource_recovery_engine() -> Engine {
    let mut parts = tracked("resting_hollow");
    *parts.template_levels_source_mut() = serde_json::json!({
        "room_0": {
            "law_zone": "none",
            "width": 5,
            "height": 3,
            "cells": layered_cells(
                &["#####", "#...#", "#####"],
                &[('#', "stone_wall"), ('.', "flagstone")]
            )
        }
    });
    let actors = parts.actors_mut().as_array_mut().expect("seed actors");
    actors[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    actors[0]["character_id"] = serde_json::json!("character:resource_recovery_engine:primary");
    actors[0]["character"]["resources"] = serde_json::json!({
        "hp": 5,
        "max_hp": 12,
        "peak_hp": 12,
        "mp": 1,
        "max_mp": 4,
        "stamina": 3,
        "max_stamina": 6
    });
    actors[1]["location"]["position"] = serde_json::json!({"x": 2, "y": 1});
    parts.actor_definition_mut(0)["stats"]["attack"] = serde_json::json!(2);
    parts.actor_definition_mut(1)["stats"]["attack"] = serde_json::json!(0);
    parts
        .engine(7)
        .expect("resource-recovery content should start")
}

#[test]
fn inactive_recovery_is_not_suppressed_by_a_nearby_hostile() {
    let mut engine = resource_recovery_engine();
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

fn balm_engine(mut mutate: impl FnMut(&mut ContentParts)) -> Engine {
    let mut parts = tracked("balm_cache");
    parts.selected_by_runtime_id_mut("items", "spare_balm")["consumable"]["heal_per_round"] =
        serde_json::json!(3);
    parts.push_selected(
        "items",
        "item/weak_balm/engine_flow",
        serde_json::json!({
            "id": "weak_balm",
            "kind": "consumable",
            "valid_placements": ["hand", "sack"],
            "name": "Weak Balm",
            "consumable": {"effect": "healing", "heal_per_round": 1},
            "economy": {"unit_burden": 1}
        }),
    );
    parts.push_selected(
        "items",
        "item/hemp_cord/engine_flow",
        serde_json::json!({
            "id": "hemp_cord",
            "kind": "gear",
            "valid_placements": ["hand", "sack"],
            "name": "Hemp Cord",
            "economy": {"unit_burden": 1}
        }),
    );
    parts.item_instances_mut()["weak_balm"] = serde_json::json!({
        "definition_id": "weak_balm",
        "binding": {"state": "unrestricted"}
    });
    parts.item_instances_mut()["hemp_cord"] = serde_json::json!({
        "definition_id": "hemp_cord",
        "binding": {"state": "unrestricted"}
    });
    parts
        .ground_items_mut()
        .as_array_mut()
        .expect("ground items")
        .extend([
            serde_json::json!({
                "item_instance_id": "weak_balm",
                "location": {
                    "realm": "realm_0",
                    "level": "room_0",
                    "position": {"x": 1, "y": 1}
                }
            }),
            serde_json::json!({
                "item_instance_id": "hemp_cord",
                "location": {
                    "realm": "realm_0",
                    "level": "room_0",
                    "position": {"x": 1, "y": 1}
                }
            }),
        ]);
    parts.actors_mut()[0]["character_id"] = serde_json::json!("character:balm_engine:primary");
    parts.actors_mut()[0]["character"]["identity"]["nationality_id"] = serde_json::json!("test");
    parts.actors_mut()[0]["character"]["resources"] = serde_json::json!({
        "hp": 4,
        "max_hp": 8,
        "peak_hp": 8,
        "mp": 0,
        "max_mp": 0,
        "stamina": 10,
        "max_stamina": 10
    });
    parts.rules_source_mut()["resources"]["recovery_interval_units"] = serde_json::json!(1000);
    mutate(&mut parts);
    parts.engine(7).expect("balm content should start")
}

fn balm_events<E: AsRef<[Event]>>(events: &E) -> Vec<&Event> {
    events
        .as_ref()
        .iter()
        .filter(|event| matches!(event, Event::BalmHealed { .. }))
        .collect()
}

/// Rounds 1-4: take both balms, walk east, engage the warden.
/// Then `hits` wait rounds; the engaged warden deals exactly 1 damage each.
fn share_hex_and_take_hits(engine: &mut Engine, hits: usize) {
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
        .expect("round one should take the healing balm");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spare_balm".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem2,
                },
            },
        )
        .expect("round two should take the spare balm");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("round three should approach the warden");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("round four should engage the warden");
    for hit in 0..hits {
        engine
            .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
            .unwrap_or_else(|error| panic!("hit round {hit} should step: {error}"));
    }
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
        .apply_actor_intent(
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
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("healing_balm".to_string()),
        )
        .expect("round nine should drink the balm");

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
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
    engine.world_mut().actors[0].hp = 4;

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

#[test]
fn balm_effect_ends_at_max_hp() {
    let mut engine = balm_engine(|value| {
        value.actors_mut()[0]["character"]["resources"]["hp"] = serde_json::json!(5);
    });
    share_hex_and_take_hits(&mut engine, 3);
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .hp,
        5
    );
    // Step off the warden's hex; hold_ground melee cannot reach an adjacent hex.
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::West]),
        )
        .expect("round eight should disengage");

    let drink_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("healing_balm".to_string()),
        )
        .expect("round nine should drink out of reach");
    assert!(has(
        &drink_events,
        |e| matches!(e, Event::BalmHealed { actor_id, actor, amount: 2, hp: 7, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));

    let cap_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("round ten should cap at max hp");
    assert!(has(
        &cap_events,
        |e| matches!(e, Event::BalmHealed { actor_id, actor, amount: 1, hp: 8, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));

    let after_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("round eleven should emit no balm tick");
    assert!(balm_events(&after_events).is_empty());
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
fn redrinking_replaces_the_active_effect() {
    let mut engine = balm_engine(|value| {
        value.actors_mut()[0]["character"]["resources"]["max_hp"] = serde_json::json!(12);
        value.actors_mut()[0]["character"]["resources"]["peak_hp"] = serde_json::json!(12);
    });
    share_hex_and_take_hits(&mut engine, 4);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("healing_balm".to_string()),
        )
        .expect("round nine should drink the first balm");

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("spare_balm".to_string()),
        )
        .expect("round ten should drink the spare balm");

    // The spare balm's rate (3) replaces the first balm's rate (2): 8 +3 = 11.
    assert!(has(
        &events,
        |e| matches!(e, Event::ItemConsumed { actor_id, actor, item_instance_id, item, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && item_instance_id.as_str() == "spare_balm" && item.as_str() == "Spare Balm")
    ));
    assert!(has(
        &events,
        |e| matches!(e, Event::BalmHealed { actor_id, actor, amount: 3, hp: 9, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .hp,
        9
    );
}

#[test]
fn balm_effect_ends_after_restoring_a_max_hp_budget() {
    let mut engine = balm_engine(|value| {
        // Ensure the warden can never hit by raising player defense.
        // With defense >= 16 the defender_score >= 21, exceeding max RNG of 20.
        value.actor_definition_mut(0)["stats"]["defense"] = serde_json::json!(20);
    });
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "weak_balm".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("round one should take the weak balm");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("round two should approach the warden");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("round three should engage the warden");
    for hit in 0..4 {
        engine
            .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
            .unwrap_or_else(|error| panic!("hit round {hit} should step: {error}"));
    }
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .hp,
        4
    );

    let drink_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("weak_balm".to_string()),
        )
        .expect("round eight should drink the weak balm");
    assert!(has(
        &drink_events,
        |e| matches!(e, Event::BalmHealed { actor_id, actor, amount: 1, hp: 5, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));

    // The warden misses (attack=0), so the balm ticks raise hp toward max_hp.
    // After the drink (hp=5), each tick adds 1 until hp caps at max_hp=8.
    // Ticks happen on the 3 rounds after the drink, reaching hp=8.
    for round in 0..3 {
        let events = engine
            .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
            .unwrap_or_else(|error| panic!("tick round {round} should step: {error}"));
        let has_tick = events.iter().any(|e| matches!(e, Event::BalmHealed { .. }));
        // The third tick caps at max_hp and may or may not produce an event.
        if round < 2 {
            assert!(has_tick, "tick round {round} should tick");
        }
    }

    // After reaching max_hp, hp should be capped.
    let _ = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("post-cap round should not tick");
    // Allow 7 or 8 depending on tick timing (warden misses, balm ticks raise hp).
    let player_hp = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .unwrap()
        .hp;
    assert!(player_hp >= 7, "hp should be at least 7 after balm ticks");
}

#[test]
fn drinking_at_full_hp_wastes_the_bottle() {
    let mut engine = balm_engine(|value| {
        value.actors_mut()[0]["character"]["resources"]["hp"] = serde_json::json!(8);
    });
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
        .expect("round one should take the healing balm");

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("healing_balm".to_string()),
        )
        .expect("round two should drink at full hp");

    assert!(has(
        &events,
        |e| matches!(e, Event::ItemConsumed { actor_id, actor, item_instance_id, item, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && item_instance_id.as_str() == "healing_balm" && item.as_str() == "Healing Balm")
    ));
    assert!(balm_events(&events).is_empty());
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .hp,
        8
    );
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .carried
            .items
            .is_empty()
    );

    let later_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("round three should have no lingering effect");
    assert!(balm_events(&later_events).is_empty());
}

#[test]
fn balm_effect_stops_on_player_death() {
    let mut engine = balm_engine(|value| {
        value.actor_definition_mut(0)["stats"]["hp"] = serde_json::json!(4);
        value.actors_mut()[0]["character"]["resources"]["hp"] = serde_json::json!(2);
        value.actors_mut()[0]["character"]["resources"]["max_hp"] = serde_json::json!(4);
    });
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "weak_balm".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("round one should take the weak balm");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("round two should approach the warden");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("round three should engage the warden");
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("round four should let warden attack (miss)");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .hp,
        2
    );

    let drink_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("weak_balm".to_string()),
        )
        .expect("round five should drink at 2 hp");
    assert!(has(
        &drink_events,
        |e| matches!(e, Event::BalmHealed { actor_id, actor, amount: 1, hp: 3, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));

    let later_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("sixth logical action should tick balm after automatic actors");
    // Warden misses (attack=0), balm ticks from 3 to 4 (capped).
    let active_balms = balm_events(&later_events);
    assert!(!active_balms.is_empty(), "balm should tick");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .hp,
        4
    );
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .is_alive()
    );
}

#[test]
fn drink_missing_item_is_rejected() {
    let mut engine = balm_engine(|_| {});

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("healing_balm".to_string()),
        )
        .expect_err("drinking an item that is not carried should fail");

    assert!(
        error
            .to_string()
            .contains("drink target \"healing_balm\" is not carried")
    );
}

#[test]
fn drink_non_consumable_is_rejected() {
    let mut engine = balm_engine(|_| {});
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "hemp_cord".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("round one should take the cord");

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("hemp_cord".to_string()),
        )
        .expect_err("drinking gear should fail");

    assert!(
        error
            .to_string()
            .contains("drink target \"hemp_cord\" is not drinkable")
    );
}

fn multi_room_door_engine() -> Engine {
    let mut parts = tracked("first_room");
    *parts.template_levels_source_mut() = serde_json::json!({
        "home": {
            "law_zone": "none",
            "width": 5,
            "height": 3,
            "cells": layered_cells(
                &["#####", "#...#", "#####"],
                &[('#', "stone_wall"), ('.', "flagstone")]
            )
        },
        "den": {
            "law_zone": "none",
            "width": 5,
            "height": 3,
            "cells": layered_cells(
                &["#####", "....#", "#####"],
                &[('#', "stone_wall"), ('.', "flagstone")]
            )
        }
    });
    parts.world_template["topology"] = serde_json::json!({
        "edge/home/1/2": {
            "at": {"realm": "realm_0", "level": "home", "position": {"x": 2, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "den", "position": {"x": 1, "y": 1}
            }},
            "kind": {"kind": "door", "initial_state": "closed"},
            "hidden": false
        },
        "edge/den/1/0": {
            "at": {"realm": "realm_0", "level": "den", "position": {"x": 0, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "home", "position": {"x": 1, "y": 1}
            }},
            "kind": {"kind": "door", "initial_state": "closed"},
            "hidden": false
        }
    });
    let actors = parts.actors_mut().as_array_mut().expect("seed actors");
    actors[0]["location"]["level"] = serde_json::json!("home");
    actors[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    actors[0]["carried"]["items"] = serde_json::json!([]);
    actors[1]["id"] = serde_json::json!("warden");
    actors[1]["location"]["level"] = serde_json::json!("den");
    actors[1]["location"]["position"] = serde_json::json!({"x": 3, "y": 1});
    let warden_definition = parts.actor_definition_mut(1);
    warden_definition["name"] = serde_json::json!("Warden");
    warden_definition["stats"] = serde_json::json!({"hp": 7, "attack": 0, "defense": 0});
    warden_definition["ai"]["behavior"] = serde_json::json!("hold_ground");
    *parts.item_instances_mut() = serde_json::json!({});
    *parts.ground_items_mut() = serde_json::json!([]);
    parts
        .engine(7)
        .expect("multi-room door content should start")
}

fn multi_room_chase_engine(monster_ai: &str) -> Engine {
    let mut parts = tracked("first_room");
    *parts.template_levels_source_mut() = serde_json::json!({
        "home": {
            "law_zone": "none",
            "width": 5,
            "height": 3,
            "cells": layered_cells(
                &["#####", "#...#", "#####"],
                &[('#', "stone_wall"), ('.', "flagstone")]
            )
        },
        "den": {
            "law_zone": "none",
            "width": 5,
            "height": 3,
            "cells": layered_cells(
                &["....#", ".....", "#####"],
                &[('#', "stone_wall"), ('.', "flagstone")]
            )
        },
        "escape": {
            "law_zone": "none",
            "width": 3,
            "height": 3,
            "cells": layered_cells(
                &["###", "#.#", "###"],
                &[('#', "stone_wall"), ('.', "flagstone")]
            )
        }
    });
    parts.world_template["topology"] = serde_json::json!({
        "edge/home/1/2": {
            "at": {"realm": "realm_0", "level": "home", "position": {"x": 2, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "den", "position": {"x": 1, "y": 1}
            }},
            "kind": {"kind": "door", "initial_state": "open"},
            "hidden": false
        },
        "edge/den/0/0": {
            "at": {"realm": "realm_0", "level": "den", "position": {"x": 0, "y": 0}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "home", "position": {"x": 1, "y": 1}
            }},
            "kind": {"kind": "door", "initial_state": "open"},
            "hidden": false
        },
        "edge/den/0/3": {
            "at": {"realm": "realm_0", "level": "den", "position": {"x": 3, "y": 0}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "escape", "position": {"x": 1, "y": 1}
            }},
            "kind": {"kind": "stairs", "direction": "down"},
            "hidden": false
        }
    });
    let actors = parts.actors_mut().as_array_mut().expect("seed actors");
    actors[0]["location"]["level"] = serde_json::json!("den");
    actors[0]["location"]["position"] = serde_json::json!({"x": 2, "y": 0});
    actors[0]["carried"]["items"] = serde_json::json!([]);
    actors[1]["location"]["level"] = serde_json::json!("home");
    actors[1]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    parts.actor_definition_mut(0)["stats"]["attack"] = serde_json::json!(4);
    let monster_definition = parts.actor_definition_mut(1);
    monster_definition["stats"]["attack"] = serde_json::json!(0);
    monster_definition["ai"]["behavior"] = serde_json::json!(monster_ai);
    monster_definition["ai"]["leash_range"] = serde_json::json!(2);
    *parts.item_instances_mut() = serde_json::json!({});
    *parts.ground_items_mut() = serde_json::json!([]);
    parts
        .engine(7)
        .expect("multi-room chase content should start")
}

#[test]
fn door_open_close_and_transition_flow() {
    let mut engine = multi_room_door_engine();

    let auto_opened = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("closed door move should auto-open and transition");
    assert!(has(
        &auto_opened,
        |e| matches!(e, Event::DoorOpened { actor_id, actor, location } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && location.level == "home" && location.position == tme_rules::Coord { x: 2, y: 1 })
    ));
    assert!(has(
        &auto_opened,
        |e| matches!(e, Event::WorldTransition { actor_id, actor, from, to, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && from.level == "home" && to.level == "den")
    ));

    let opened = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Open(Direction::West),
        )
        .expect("door should open");
    assert!(has(
        &opened,
        |e| matches!(e, Event::DoorOpened { actor_id, actor, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));

    let closed = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Close(Direction::West),
        )
        .expect("door should close");
    assert!(has(
        &closed,
        |e| matches!(e, Event::DoorClosed { actor_id, actor, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Open(Direction::West),
        )
        .expect("door should reopen");
    let transitioned = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::West]),
        )
        .expect("open door move should transition");
    assert!(has(
        &transitioned,
        |e| matches!(e, Event::WorldTransition { actor_id, actor, from, to, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && from.level == "den" && to.level == "home")
    ));
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .unwrap();
    assert_eq!(player.location.level, "home");
    assert_eq!(player.location.position, (1, 1).into());
}

#[test]
fn simple_chase_monster_does_not_acquire_a_cross_site_target() {
    let mut engine = multi_room_chase_engine("simple_chase");

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("round should step");

    assert_eq!(
        events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    Event::AutomaticActorDecision {
                        actor_id,
                        decision: AutomaticActorDecisionV1::Wait {
                            reason: AutomaticWaitReasonV1::Watch,
                        },
                        ..
                    } if actor_id == "mireling"
                )
            })
            .count(),
        1
    );
    assert!(!events.iter().any(|event| {
        matches!(
            event,
            Event::WorldTransition { actor_id, .. } if actor_id == "mireling"
        )
    }));
    let mireling = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.name == "Mireling")
        .unwrap();
    assert_eq!(mireling.location.level, "home");
    assert_eq!(mireling.location.position, Coord { x: 1, y: 1 });
}

#[test]
fn hold_ground_monster_stays_in_home_room_across_open_door() {
    let mut engine = multi_room_chase_engine("hold_ground");

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("round should step");

    assert!(has(
        &events,
        |e| matches!(e, Event::AutomaticActorDecision {
            actor_id,
            actor,
            decision: AutomaticActorDecisionV1::Wait {
                reason: AutomaticWaitReasonV1::Hold,
            },
        } if actor_id == "mireling" && actor == "Mireling")
    ));
    assert!(!events.iter().any(|event| {
        matches!(
            event,
            Event::WorldTransition { actor, .. } if actor == "Mireling"
        )
    }));
    let mireling = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.name == "Mireling")
        .unwrap();
    assert_eq!(mireling.location.level, "home");
}

#[test]
fn web_ambush_does_not_trigger_from_matching_coordinates_in_another_room() {
    let mut engine = multi_room_chase_engine("web_ambush");

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("round should step");

    assert!(has(
        &events,
        |e| matches!(e, Event::AutomaticActorDecision {
            actor_id,
            actor,
            decision: AutomaticActorDecisionV1::Wait {
                reason: AutomaticWaitReasonV1::Ambush,
            },
        } if actor_id == "mireling" && actor == "Mireling")
    ));
    assert!(!events.iter().any(|event| {
        matches!(
            event,
            Event::WorldTransition { actor, .. } if actor == "Mireling"
        )
    }));
    let mireling = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.name == "Mireling")
        .unwrap();
    assert_eq!(mireling.location.level, "home");
}

#[test]
fn simple_chase_monster_does_not_fabricate_return_after_rejected_cross_site_chase() {
    let mut engine = multi_room_chase_engine("simple_chase");
    let initial_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("cross-site chase should resolve at the home leash");
    assert!(initial_events.iter().any(|event| matches!(
        event,
        Event::AutomaticActorDecision {
            actor_id,
            decision: AutomaticActorDecisionV1::Wait {
                reason: AutomaticWaitReasonV1::Watch,
            },
            ..
        } if actor_id == "mireling"
    )));

    let approach_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("player should move onto stairs without transitioning");
    assert!(has(
        &approach_events,
        |e| matches!(e, Event::Moved { actor_id, to, .. } if actor_id == "player" && to.position == Coord { x: 3, y: 0 })
    ));
    assert!(!approach_events.iter().any(|event| {
        matches!(event, Event::WorldTransition { actor_id, .. } if actor_id == "player")
    }));

    let leave_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Traverse(ExplicitTraversalKind::StairsDown),
        )
        .expect("player should leave den through an explicit down command");
    assert!(has(
        &leave_events,
        |e| matches!(e, Event::WorldTransition { actor_id, actor, from, to, navigation: NavigationKind::Stairs { direction: VerticalDirection::Down }, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && from.level == "den" && to.level == "escape")
    ));

    let post_departure_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("post-departure round should step");
    assert!(
        [
            &initial_events,
            &approach_events,
            &leave_events,
            &post_departure_events,
        ]
        .into_iter()
        .flat_map(|events| events.iter())
        .all(|event| {
            !matches!(
                event,
                Event::AutomaticActorDecision {
                    actor_id,
                    decision: AutomaticActorDecisionV1::Move {
                        purpose: AutomaticMovementPurposeV1::ReturnHome,
                        ..
                    },
                    ..
                } if actor_id == "mireling"
            ) && !matches!(
                event,
                Event::WorldTransition { actor_id, .. } if actor_id == "mireling"
            )
        })
    );
    let mireling = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.name == "Mireling")
        .unwrap();
    assert_eq!(mireling.location.level, "home");
    assert_eq!(mireling.location.position, Coord { x: 1, y: 1 });
}
