//! Closing kicks use the real realtime boundary, not a test teleport followed
//! by an attack. Authored fixtures keep all automatic actors dormant here.

use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};
use tme_rules::{
    ActionBlockedReasonV1, ActorId, Coord, Direction, Engine, Event, HostilityAuthorization,
    LogicalTime, NavigationKind, PhysicalAttackMode, PlayerIntent, WorldPosition,
};

fn actor_id() -> ActorId {
    ActorId::from("player")
}

fn place(x: i32, y: i32) -> Value {
    json!({"realm": "realm_0", "level": "room_0", "position": {"x": x, "y": y}})
}

fn parts(target: Coord) -> ContentParts {
    let mut parts = ContentParts::tracked("skill_progression", "profile/skill_progression");
    let cells: Vec<Vec<Value>> = (0..9)
        .map(|y| {
            (0..9)
                .map(|x| {
                    json!([if x == 0 || y == 0 || x == 8 || y == 8 {
                        "stone_wall"
                    } else {
                        "flagstone"
                    }])
                })
                .collect()
        })
        .collect();
    let room = &mut parts.template_levels_source_mut()["room_0"];
    room["width"] = json!(9);
    room["height"] = json!(9);
    room["maximum_clear_sightline"] = json!(7);
    room["cells"] = json!(cells);
    parts.actors_mut()[0]["location"] = place(4, 4);
    parts.actors_mut()[0]["carried"]["items"] = json!([]);
    *parts.item_instances_mut() = json!({});
    let identity = &mut parts.actors_mut()[0]["character"]["identity"];
    identity["base_class_id"] = json!("martial_artist");
    identity["current_class_id"] = json!("martial_artist");
    identity["display_class"] = json!("Martial Artist");
    let skills = parts.skill_catalog_mut().expect("skill catalog");
    let mut hand = skills["tracks"][0].clone();
    hand["id"] = json!("hand");
    hand["display"] = json!("Hand");
    hand["kind"] = json!("martial_arts");
    skills["tracks"].as_array_mut().unwrap().push(hand);
    parts.actors_mut()[0]["character"]["skill_ledger"] = json!([{
        "track_id": "hand", "level": 6, "critique_rank": 0,
        "practice_points": 0, "learning_rate": 1
    }]);
    parts.actors_mut()[1]["location"] = place(target.x, target.y);
    parts.actor_definition_mut(1)["ai"]["behavior"] = json!("hold_ground");
    parts.actor_definition_mut(1)["stats"]["hp"] = json!(1000);
    parts
}

fn engine(parts: ContentParts) -> Engine {
    let mut engine = parts.engine(7).expect("valid closing-kick fixture");
    engine.world_mut().actors[1].timing.ready_at = LogicalTime::new(u64::MAX);
    engine
}

fn intent(mode: PhysicalAttackMode) -> PlayerIntent {
    PlayerIntent::PhysicalAttack {
        mode,
        target_actor_id: "mireling".into(),
        authorization: HostilityAuthorization::Safe,
    }
}

fn attack(event: &Event, mode: PhysicalAttackMode) -> bool {
    matches!(event,
        Event::Attacked { attacker_id, mode: actual, .. }
        | Event::AttackMissed { attacker_id, mode: actual, .. }
        | Event::AttackBlocked { attacker_id, mode: actual, .. }
        if attacker_id == "player" && *actual == mode
    )
}

fn reject_unchanged(engine: &mut Engine, reason: ActionBlockedReasonV1) {
    let before = engine.export_checkpoint().expect("before checkpoint");
    let command = engine
        .actor_command_for_intent(&actor_id(), &intent(PhysicalAttackMode::Jumpkick))
        .unwrap();
    let status = engine.validate_actor_command(&command).unwrap();
    assert!(!status.accepted);
    assert_eq!(status.blocked_reason, Some(reason));
    assert_eq!(
        engine.export_checkpoint().unwrap(),
        before,
        "preview is read-only"
    );
    engine
        .apply_realtime_actor_intent(&actor_id(), intent(PhysicalAttackMode::Jumpkick))
        .expect_err("illegal kick must reject");
    assert_eq!(
        engine.export_checkpoint().unwrap(),
        before,
        "full state and RNG roll back"
    );
}

#[test]
fn all_octants_land_and_attack_once_with_one_cost_and_deadline() {
    for direction in Direction::all() {
        for distance in 1..=3 {
            let (dx, dy) = direction.delta();
            let target = Coord {
                x: 4 + dx * distance,
                y: 4 + dy * distance,
            };
            let mut engine = engine(parts(target));
            let before = engine.export_checkpoint().unwrap();
            let command = engine
                .actor_command_for_intent(&actor_id(), &intent(PhysicalAttackMode::Jumpkick))
                .unwrap();
            assert!(engine.validate_actor_command(&command).unwrap().accepted);
            assert_eq!(engine.export_checkpoint().unwrap(), before);
            let now = engine.world().timing.now;
            let stamina = engine.world().actors[0].stamina;
            let result = engine
                .apply_realtime_actor_intent(&actor_id(), intent(PhysicalAttackMode::Jumpkick))
                .unwrap();
            assert_eq!(
                engine.world().actors[0].location,
                engine.world().actors[1].location
            );
            assert_eq!(
                engine.world().timing.now,
                now,
                "no separate approach interval"
            );
            assert_eq!(
                engine.world().actors[0].timing.ready_at,
                now.saturating_add_rounds(1)
            );
            assert_eq!(
                engine.world().actors[0].attack_ready_at,
                now.saturating_add_rounds(1)
            );
            assert_eq!(engine.world().actors[0].stamina, stamina - 1);
            assert_eq!(
                result
                    .events
                    .iter()
                    .filter(|e| attack(e, PhysicalAttackMode::Jumpkick))
                    .count(),
                1
            );
            assert_eq!(
                result
                    .events
                    .iter()
                    .filter(|e| matches!(e,
                        Event::ActorReadinessScheduled { actor_id, .. } if actor_id == "player"
                    ))
                    .count(),
                1
            );
            assert_eq!(
                result
                    .events
                    .iter()
                    .filter(|e| matches!(e,
                        Event::PhysicalStaminaSpent { actor_id, .. } if actor_id == "player"
                    ))
                    .count(),
                1
            );
            assert!(!result.events.iter().any(|e| matches!(
                e,
                Event::MovementStaminaSpent { .. } | Event::MovementStarted { .. }
            )));
            let last_move = result
                .events
                .iter()
                .rposition(|e| {
                    matches!(e,
                        Event::Moved { actor_id, .. } if actor_id == "player"
                    )
                })
                .unwrap();
            let outcome = result
                .events
                .iter()
                .position(|e| attack(e, PhysicalAttackMode::Jumpkick))
                .unwrap();
            assert!(last_move < outcome, "land before resolving the kick");
            let projection = engine
                .observer_projection(&actor_id(), &result.events)
                .unwrap();
            let route = projection
                .events
                .iter()
                .filter_map(|event| match event {
                    tme_rules::view::ObservedEventV1::ActorMoved {
                        actor_id, from, to, ..
                    } if actor_id == "player" => Some((from.clone(), to.clone())),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(route.len(), distance as usize);
            assert_eq!(route[0].0.position, Coord { x: 4, y: 4 });
            assert_eq!(route.last().unwrap().1.position, target);
            assert!(route.windows(2).all(|pair| pair[0].1 == pair[1].0));
        }
    }
}

#[test]
fn unequal_axis_approach_is_deterministic_and_has_no_extra_steps() {
    let mut engine = engine(parts(Coord { x: 7, y: 5 }));
    let result = engine
        .apply_realtime_actor_intent(&actor_id(), intent(PhysicalAttackMode::Jumpkick))
        .unwrap();
    let points = result
        .events
        .iter()
        .filter_map(|event| match event {
            Event::Moved { to, .. } => Some(to.position),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        points,
        vec![
            Coord { x: 5, y: 5 },
            Coord { x: 6, y: 5 },
            Coord { x: 7, y: 5 }
        ]
    );
}

#[test]
fn the_next_action_can_punch_and_a_same_tile_jumpkick_is_refused() {
    let mut engine = engine(parts(Coord { x: 7, y: 4 }));
    engine
        .apply_realtime_actor_intent(&actor_id(), intent(PhysicalAttackMode::Jumpkick))
        .unwrap();
    engine
        .advance_to(engine.world().actors[0].timing.ready_at)
        .unwrap();
    reject_unchanged(&mut engine, ActionBlockedReasonV1::OutOfRange);
    let result = engine
        .apply_realtime_actor_intent(&actor_id(), intent(PhysicalAttackMode::Fight))
        .unwrap();
    assert_eq!(
        result
            .events
            .iter()
            .filter(|e| attack(e, PhysicalAttackMode::Fight))
            .count(),
        1
    );
    assert!(
        !result
            .events
            .iter()
            .any(|e| matches!(e, Event::Moved { .. }))
    );
}

#[test]
fn skill_limited_reach_and_insufficient_stamina_remain_authoritative() {
    let mut limited = parts(Coord { x: 6, y: 4 });
    limited.actors_mut()[0]["character"]["skill_ledger"][0]["level"] = json!(0);
    reject_unchanged(&mut engine(limited), ActionBlockedReasonV1::OutOfRange);
    let mut far = parts(Coord { x: 7, y: 4 });
    far.actors_mut()[0]["location"] = place(1, 4);
    reject_unchanged(&mut engine(far), ActionBlockedReasonV1::OutOfRange);
    for x in [5, 6, 7] {
        let mut empty = parts(Coord { x, y: 4 });
        empty.actors_mut()[0]["character"]["resources"]["stamina"] = json!(0);
        reject_unchanged(
            &mut engine(empty),
            ActionBlockedReasonV1::InsufficientStamina,
        );
    }
}

#[test]
fn a_blocked_late_step_rolls_back_everything_and_does_not_detour() {
    let mut blocked = parts(Coord { x: 7, y: 4 });
    blocked.template_levels_source_mut()["room_0"]["cells"][4][6] = json!(["stone_wall"]);
    reject_unchanged(&mut engine(blocked), ActionBlockedReasonV1::BlockedTerrain);
}

#[test]
fn a_visible_diagonal_target_does_not_authorize_corner_cutting() {
    let mut blocked = parts(Coord { x: 5, y: 5 });
    blocked.template_levels_source_mut()["room_0"]["cells"][4][5] = json!(["stone_wall"]);
    reject_unchanged(&mut engine(blocked), ActionBlockedReasonV1::BlockedTerrain);
}

fn local_door(parts: &mut ContentParts, x: i32, initial_state: &str) {
    parts.world_template["topology"] = json!({"edge/approach": {
        "at": place(x, 4), "target": {"kind": "position", "location": place(x, 4)},
        "kind": {"kind": "local_door", "initial_state": initial_state}, "hidden": false
    }});
}

#[test]
fn closed_doors_never_open_for_a_kick_including_the_landing_tile() {
    for door_x in [5, 6, 7] {
        let mut closed = parts(Coord { x: 7, y: 4 });
        local_door(&mut closed, door_x, "closed");
        reject_unchanged(&mut engine(closed), ActionBlockedReasonV1::ClosedDoor);
    }
}

#[test]
fn an_open_local_door_preserves_the_full_approach_chain() {
    let mut open = parts(Coord { x: 7, y: 4 });
    local_door(&mut open, 5, "open");
    let mut engine = engine(open);
    let result = engine
        .apply_realtime_actor_intent(&actor_id(), intent(PhysicalAttackMode::Jumpkick))
        .unwrap();
    assert_eq!(
        engine.world().actors[0].location.position,
        Coord { x: 7, y: 4 }
    );
    assert!(
        !result
            .events
            .iter()
            .any(|e| matches!(e, Event::DoorOpened { .. }))
    );
    let projection = engine
        .observer_projection(&actor_id(), &result.events)
        .unwrap();
    let modes = projection
        .events
        .iter()
        .filter_map(|event| match event {
            tme_rules::view::ObservedEventV1::ActorMoved { navigation, .. } => Some(*navigation),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        modes,
        [
            NavigationKind::Door,
            NavigationKind::Walk,
            NavigationKind::Walk
        ]
    );
}

#[test]
fn automatic_transitions_cannot_be_used_as_a_kick_shortcut() {
    for kind in ["passage", "portal", "pit"] {
        let mut transition = parts(Coord { x: 7, y: 4 });
        transition.world_template["topology"] = json!({"edge/approach": {
            "at": place(5, 4), "target": {"kind": "position", "location": place(7, 4)},
            "kind": {"kind": kind}, "hidden": false
        }});
        reject_unchanged(
            &mut engine(transition),
            ActionBlockedReasonV1::BlockedTerrain,
        );
    }
}

#[test]
fn swimming_and_unaffordable_terrain_do_not_acquire_air_traversal() {
    let mut water = parts(Coord { x: 7, y: 4 });
    let mut terrain = water
        .selected_by_runtime_id_mut("terrains", "flagstone")
        .clone();
    terrain["id"] = json!("approach_water");
    terrain["name"] = json!("Approach Water");
    terrain["traversal"] = json!("swim");
    water.push_selected("terrains", "terrain/approach_water", terrain);
    water.template_levels_source_mut()["room_0"]["cells"][4][6] = json!(["approach_water"]);
    reject_unchanged(
        &mut engine(water.clone()),
        ActionBlockedReasonV1::BlockedTerrain,
    );
    // A door event must not conceal the underlying swimming terrain.
    local_door(&mut water, 6, "open");
    reject_unchanged(&mut engine(water), ActionBlockedReasonV1::BlockedTerrain);
    let mut costly = parts(Coord { x: 7, y: 4 });
    costly.selected_by_runtime_id_mut("terrains", "flagstone")["move_cost"] = json!(2);
    reject_unchanged(
        &mut engine(costly),
        ActionBlockedReasonV1::InsufficientMovementPoints,
    );
}

#[test]
fn misses_still_land_but_do_not_damage_the_defender() {
    let mut missed = parts(Coord { x: 7, y: 4 });
    missed.actor_definition_mut(1)["stats"]["defense"] = json!(1000);
    let mut engine = engine(missed);
    let hp = engine.world().actors[1].hp;
    let result = engine
        .apply_realtime_actor_intent(&actor_id(), intent(PhysicalAttackMode::Jumpkick))
        .unwrap();
    assert!(result.events.iter().any(|e| matches!(
        e,
        Event::AttackMissed {
            mode: PhysicalAttackMode::Jumpkick,
            ..
        }
    )));
    assert_eq!(
        engine.world().actors[0].location,
        engine.world().actors[1].location
    );
    assert_eq!(engine.world().actors[1].hp, hp);
}

#[test]
fn a_shield_block_still_lands_and_pays_the_single_kick_cost() {
    let mut blocked = parts(Coord { x: 7, y: 4 });
    blocked.rules_source_mut()["combat"]["block"]["shield_percent_cap"] = json!(100);
    blocked.push_selected(
        "items",
        "item/approach_guard",
        json!({
            "id": "approach_guard", "kind": "shield", "name": "Approach Guard",
            "valid_placements": ["hand"], "capability": {"block_value": 100},
            "economy": {"unit_burden": 0}
        }),
    );
    blocked.item_instances_mut()["approach_guard"] = json!({
        "definition_id": "approach_guard", "binding": {"state": "unrestricted"}
    });
    blocked.actors_mut()[1]["carried"]["items"] = json!([{
        "item_instance_id": "approach_guard", "position": "left_hand"
    }]);
    let mut engine = engine(blocked); // Seed 7's first d20 is 11, below the block cap.
    let result = engine
        .apply_realtime_actor_intent(&actor_id(), intent(PhysicalAttackMode::Jumpkick))
        .unwrap();
    assert!(result.events.iter().any(|e| matches!(
        e,
        Event::AttackBlocked {
            mode: PhysicalAttackMode::Jumpkick,
            ..
        }
    )));
    assert_eq!(
        engine.world().actors[0].location,
        engine.world().actors[1].location
    );
    assert_eq!(engine.world().actors[0].stamina, 9);
}

#[test]
fn target_changes_are_resolved_at_submission_not_from_old_preview_placement() {
    let mut engine = engine(parts(Coord { x: 7, y: 4 }));
    let command = engine
        .actor_command_for_intent(&actor_id(), &intent(PhysicalAttackMode::Jumpkick))
        .unwrap();
    assert!(engine.validate_actor_command(&command).unwrap().accepted);
    engine.world_mut().actors[1].location.position = Coord { x: 4, y: 6 };
    engine
        .apply_realtime_actor_intent(&actor_id(), intent(PhysicalAttackMode::Jumpkick))
        .unwrap();
    assert_eq!(
        engine.world().actors[0].location.position,
        Coord { x: 4, y: 6 }
    );
}

#[test]
fn disappearing_or_out_of_range_targets_never_leave_a_partial_approach() {
    let mut seed = parts(Coord { x: 3, y: 4 });
    seed.actors_mut()[0]["location"] = place(1, 4);
    let mut moved = engine(seed);
    moved.world_mut().actors[1].location.position = Coord { x: 7, y: 4 };
    reject_unchanged(&mut moved, ActionBlockedReasonV1::OutOfRange);
    let mut removed = engine(parts(Coord { x: 7, y: 4 }));
    removed.world_mut().actors.remove(1);
    reject_unchanged(&mut removed, ActionBlockedReasonV1::NoSuchTarget);
}

#[test]
fn insufficient_attack_readiness_never_moves_or_spends_kick_stamina() {
    let mut engine = engine(parts(Coord { x: 7, y: 4 }));
    engine.world_mut().actors[0].attack_ready_at = LogicalTime::new(60_000);
    let from = engine.world().actors[0].location.clone();
    let result = engine
        .apply_realtime_actor_intent(&actor_id(), intent(PhysicalAttackMode::Jumpkick))
        .unwrap();
    assert!(
        result
            .events
            .iter()
            .any(|e| matches!(e, Event::AttackNotReady { .. }))
    );
    assert_eq!(engine.world().actors[0].location, from);
    assert_eq!(engine.world().actors[0].stamina, 10);
    assert!(
        !result
            .events
            .iter()
            .any(|e| matches!(e, Event::Moved { .. } | Event::PhysicalStaminaSpent { .. }))
    );
}

#[test]
fn movement_bow_side_effect_is_preserved_without_charging_a_walk() {
    let mut bow = parts(Coord { x: 7, y: 4 });
    bow.push_selected(
        "items",
        "item/approach_bow",
        json!({
            "id": "approach_bow", "kind": "weapon", "name": "Approach Bow",
            "valid_placements": ["hand"], "economy": {"unit_burden": 0},
            "weapon": {
                "skill_track_id": "sword", "default_attack_mode": "shoot",
                "attack_modes": [{"mode": "shoot", "maximum_range": 3, "damage_kind": "piercing"}],
                "cooldown_units": 1, "combat_add_rating": 0, "handedness": "bow",
                "block_value": 0, "nocking": {"unloads_on_movement": true}
            }
        }),
    );
    bow.item_instances_mut()["approach_bow"] = json!({
        "definition_id": "approach_bow", "binding": {"state": "unrestricted"}
    });
    bow.actors_mut()[0]["carried"]["items"] = json!([{
        "item_instance_id": "approach_bow", "position": "right_hand"
    }]);
    let mut engine = engine(bow);
    engine
        .apply_realtime_actor_intent(&actor_id(), PlayerIntent::Nock)
        .unwrap();
    engine
        .advance_to(engine.world().actors[0].timing.ready_at)
        .unwrap();
    let stamina = engine.world().actors[0].stamina;
    let result = engine
        .apply_realtime_actor_intent(&actor_id(), intent(PhysicalAttackMode::Jumpkick))
        .unwrap();
    assert!(result.events.iter().any(|e| matches!(
        e,
        Event::BowReadinessChanged {
            reason: tme_rules::BowReadinessChangeReason::Movement,
            ..
        }
    )));
    assert_eq!(engine.world().actors[0].stamina, stamina - 1);
    assert_eq!(
        engine.world().actors[0].location,
        WorldPosition::new("realm_0", "room_0", Coord { x: 7, y: 4 })
    );
}

#[test]
fn paired_doors_do_not_teleport_a_closing_attack() {
    let mut paired = parts(Coord { x: 7, y: 4 });
    paired.world_template["topology"] = json!({
        "edge/near": {
            "at": place(5, 4), "target": {"kind": "position", "location": place(6, 4)},
            "kind": {"kind": "door", "binding_id": "approach_pair", "endpoint_id": "near",
                "reciprocal_endpoint_id": "far", "initial_state": "open"}, "hidden": false
        },
        "edge/far": {
            "at": place(6, 4), "target": {"kind": "position", "location": place(5, 4)},
            "kind": {"kind": "door", "binding_id": "approach_pair", "endpoint_id": "far",
                "reciprocal_endpoint_id": "near", "initial_state": "open"}, "hidden": false
        }
    });
    reject_unchanged(&mut engine(paired), ActionBlockedReasonV1::BlockedTerrain);
}

#[test]
fn a_dead_target_cannot_accept_a_closing_attack() {
    let mut engine = engine(parts(Coord { x: 7, y: 4 }));
    engine.world_mut().actors[1].hp = 0;
    engine.world_mut().actors[1].life_state = tme_rules::ActorLifeState::Dead;
    reject_unchanged(&mut engine, ActionBlockedReasonV1::NoSuchTarget);
}

#[test]
fn suppressed_and_blind_attempts_keep_the_existing_feedback_without_travel() {
    for (tag, suppresses, reason) in [
        ("stunned", true, ActionBlockedReasonV1::SuppressedByStatus),
        ("blind", false, ActionBlockedReasonV1::BlockedBySight),
    ] {
        let mut affected = parts(Coord { x: 7, y: 4 });
        affected.actors_mut()[0]["active_effects"] = json!([{
            "instance_id": "approach_status", "effect_id": "approach_status",
            "source": {"kind": "fixture", "id": "closing_kick_tests"},
            "kind": "control_status", "tags": [tag], "potency": 0,
            "remaining_rounds": 20, "stacking": "refresh_duration",
            "start_delay_rounds": 0, "tick_interval_rounds": 1,
            "suppresses_action": suppresses, "resistance_boosts": []
        }]);
        let mut engine = engine(affected);
        let command = engine
            .actor_command_for_intent(&actor_id(), &intent(PhysicalAttackMode::Jumpkick))
            .unwrap();
        let before = engine.export_checkpoint().unwrap();
        let preview = engine.validate_actor_command(&command).unwrap();
        assert!(!preview.accepted);
        assert_eq!(preview.blocked_reason, Some(reason));
        assert_eq!(engine.export_checkpoint().unwrap(), before);
        let from = engine.world().actors[0].location.clone();
        let result = engine
            .apply_realtime_actor_intent(&actor_id(), intent(PhysicalAttackMode::Jumpkick))
            .unwrap();
        // These legacy feedback cases still schedule a normal action; they are
        // not transactional errors and must not be called an unchanged world.
        assert!(result.events.iter().any(|event| if suppresses {
            matches!(event, Event::ActionSuppressedByStatus { .. })
        } else {
            matches!(event, Event::AttackBlockedNoSight { .. })
        }));
        assert_eq!(engine.world().actors[0].location, from);
        assert_eq!(engine.world().actors[0].stamina, 10);
        assert!(!result.events.iter().any(|event| matches!(
            event,
            Event::Moved { .. } | Event::PhysicalStaminaSpent { .. }
        ) || attack(
            event,
            PhysicalAttackMode::Jumpkick
        )));
    }
}

#[test]
fn a_late_practice_failure_rolls_back_the_approach_damage_cost_deadline_and_rng() {
    let mut seeded = parts(Coord { x: 7, y: 4 });
    seeded.actor_definition_mut(1)["xp_value"] = json!(1000);
    seeded.rules_source_mut()["combat"]["practice"]["life_and_death_raw_points"] = json!(2);
    let mut engine = engine(seeded);
    engine.world_mut().actors[0]
        .character
        .as_mut()
        .unwrap()
        .skill_ledger[0]
        .learning_rate = u64::MAX;
    let before = engine.export_checkpoint().unwrap();
    let error = engine
        .apply_realtime_actor_intent(&actor_id(), intent(PhysicalAttackMode::Jumpkick))
        .unwrap_err();
    assert_eq!(
        error.message(),
        "skill \"hand\" practice credit must not overflow"
    );
    assert_eq!(engine.export_checkpoint().unwrap(), before);
}

#[test]
fn a_difficult_but_affordable_landing_pays_only_physical_stamina() {
    let mut costly = parts(Coord { x: 5, y: 4 });
    costly.selected_by_runtime_id_mut("terrains", "flagstone")["move_cost"] = json!(2);
    let mut engine = engine(costly);
    let result = engine
        .apply_realtime_actor_intent(&actor_id(), intent(PhysicalAttackMode::Jumpkick))
        .unwrap();
    assert_eq!(
        engine.world().actors[0].location,
        engine.world().actors[1].location
    );
    assert_eq!(engine.world().actors[0].stamina, 9);
    assert!(
        !result
            .events
            .iter()
            .any(|event| matches!(event, Event::MovementStaminaSpent { .. }))
    );
}
