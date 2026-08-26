use crate::ai_support::{
    active_effect, actor_position, automatic_actor, decision, decisions, engine, engine_from_value,
    open_room_value, unrestricted, wait,
};
use tme_rules::{
    AutomaticActorDecisionV1, AutomaticMovementPurposeV1, AutomaticWaitReasonV1, Coord, Direction,
    Event,
};

fn actor(id: &str, position: Coord, behavior: &str) -> serde_json::Value {
    automatic_actor(
        id,
        "chaotic",
        position,
        behavior,
        1,
        unrestricted(),
        &["fight"],
    )
}

#[test]
fn simple_chase_prefers_diagonal_then_cardinal_reduction() {
    let mut diagonal = engine(vec![actor("chaser", Coord { x: 5, y: 3 }, "simple_chase")]);
    assert!(matches!(
        decision(&wait(&mut diagonal), "chaser"),
        AutomaticActorDecisionV1::Move {
            direction: Direction::Northwest,
            purpose: AutomaticMovementPurposeV1::Chase,
        }
    ));

    let actors = vec![actor("chaser", Coord { x: 5, y: 3 }, "simple_chase")];
    let mut value = open_room_value(actors);
    value.template_levels_source_mut()["room_0"]["cells"][2][4] = serde_json::json!(["stone_wall"]);
    let mut engine = engine_from_value(value);
    assert!(matches!(
        decision(&wait(&mut engine), "chaser"),
        AutomaticActorDecisionV1::Move {
            direction: Direction::West,
            purpose: AutomaticMovementPurposeV1::Chase,
        }
    ));
}

#[test]
fn simple_chase_waits_when_reducing_step_is_blocked_or_over_budget() {
    let actors = vec![actor("chaser", Coord { x: 5, y: 2 }, "simple_chase")];
    let mut blocked = open_room_value(actors.clone());
    blocked.template_levels_source_mut()["room_0"]["cells"][2][4] =
        serde_json::json!(["stone_wall"]);
    let mut blocked_engine = engine_from_value(blocked);
    assert_eq!(
        decision(&wait(&mut blocked_engine), "chaser"),
        &AutomaticActorDecisionV1::Wait {
            reason: AutomaticWaitReasonV1::Watch,
        }
    );

    let mut costly = open_room_value(actors);
    costly.push_selected(
        "terrains",
        "terrain/mud/automatic_actor_behaviors",
        serde_json::json!({
            "id": "mud",
            "name": "Mud",
            "navigation": {"kind": "walk", "move_cost": 2, "blocks_sight": false}
        }),
    );
    costly.template_levels_source_mut()["room_0"]["cells"][2][4] = serde_json::json!(["mud"]);
    let mut costly_engine = engine_from_value(costly);
    assert_eq!(
        decision(&wait(&mut costly_engine), "chaser"),
        &AutomaticActorDecisionV1::Wait {
            reason: AutomaticWaitReasonV1::Blocked,
        }
    );
}

#[test]
fn chase_movement_can_stack_on_hostile_hex_without_attacking() {
    let mut engine = engine(vec![actor("chaser", Coord { x: 2, y: 2 }, "simple_chase")]);
    let events = wait(&mut engine);
    assert!(matches!(
        decision(&events, "chaser"),
        AutomaticActorDecisionV1::Move {
            direction: Direction::West,
            ..
        }
    ));
    assert_eq!(actor_position(&engine, "chaser"), Coord { x: 1, y: 2 });
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::Attacked { attacker_id, .. } if attacker_id == "chaser"
    )));
}

#[test]
fn pack_forager_chases_at_half_hp_and_flees_below_half() {
    let mut healthy = engine(vec![actor("forager", Coord { x: 5, y: 2 }, "pack_forager")]);
    healthy.world_mut().actors[1].hp = 10;
    assert!(matches!(
        decision(&wait(&mut healthy), "forager"),
        AutomaticActorDecisionV1::Move {
            direction: Direction::West,
            purpose: AutomaticMovementPurposeV1::Chase,
        }
    ));

    let mut wounded = engine(vec![actor("forager", Coord { x: 5, y: 2 }, "pack_forager")]);
    wounded.world_mut().actors[1].hp = 9;
    assert!(matches!(
        decision(&wait(&mut wounded), "forager"),
        AutomaticActorDecisionV1::Move {
            direction: Direction::East,
            purpose: AutomaticMovementPurposeV1::Flee,
        }
    ));
}

#[test]
fn pack_forager_flees_shared_hex_and_waits_when_cornered() {
    let mut shared = engine(vec![actor("forager", Coord { x: 1, y: 2 }, "pack_forager")]);
    shared.world_mut().actors[1].hp = 9;
    assert!(matches!(
        decision(&wait(&mut shared), "forager"),
        AutomaticActorDecisionV1::Move {
            direction: Direction::North,
            purpose: AutomaticMovementPurposeV1::Flee,
        }
    ));

    let actors = vec![actor("forager", Coord { x: 1, y: 1 }, "pack_forager")];
    let mut value = open_room_value(actors);
    value.actors_mut()[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 2});
    let mut cornered = engine_from_value(value);
    cornered.world_mut().actors[1].hp = 9;
    assert_eq!(
        decision(&wait(&mut cornered), "forager"),
        &AutomaticActorDecisionV1::Wait {
            reason: AutomaticWaitReasonV1::Blocked,
        }
    );
}

#[test]
fn web_ambush_waits_outside_radius_then_springs_and_returns() {
    let mut engine = engine(vec![actor("spider", Coord { x: 5, y: 2 }, "web_ambush")]);
    assert_eq!(
        decision(&wait(&mut engine), "spider"),
        &AutomaticActorDecisionV1::Wait {
            reason: AutomaticWaitReasonV1::Ambush,
        }
    );

    engine.world_mut().actors[0].location.position = Coord { x: 3, y: 2 };
    assert!(matches!(
        decision(&wait(&mut engine), "spider"),
        AutomaticActorDecisionV1::Move {
            direction: Direction::West,
            purpose: AutomaticMovementPurposeV1::Chase,
        }
    ));

    engine.world_mut().actors[0].location.position = Coord { x: 1, y: 2 };
    assert!(matches!(
        decision(&wait(&mut engine), "spider"),
        AutomaticActorDecisionV1::Move {
            direction: Direction::East,
            purpose: AutomaticMovementPurposeV1::ReturnHome,
        }
    ));
}

#[test]
fn hold_ground_never_chases_but_attacks_on_shared_hex() {
    let mut far = engine(vec![actor("guard", Coord { x: 5, y: 2 }, "hold_ground")]);
    assert_eq!(
        decision(&wait(&mut far), "guard"),
        &AutomaticActorDecisionV1::Wait {
            reason: AutomaticWaitReasonV1::Hold,
        }
    );

    let mut shared = engine(vec![actor("guard", Coord { x: 1, y: 2 }, "hold_ground")]);
    assert!(matches!(
        decision(&wait(&mut shared), "guard"),
        AutomaticActorDecisionV1::PhysicalAttack { target_id, .. }
            if target_id == "player"
    ));
}

#[test]
fn leash_return_emits_exactly_one_decision() {
    let actors = vec![actor("roamer", Coord { x: 5, y: 2 }, "simple_chase")];
    let mut value = open_room_value(actors);
    let room = value.template_levels_source_mut()["room_0"].clone();
    value.template_levels_source_mut()["room_1"] = room;
    let mut engine = engine_from_value(value);
    engine.world_mut().actors[1].location.level = "room_1".to_string();
    let events = wait(&mut engine);
    assert_eq!(decisions(&events, "roamer").count(), 1);
    assert_eq!(
        decision(&events, "roamer"),
        &AutomaticActorDecisionV1::Wait {
            reason: AutomaticWaitReasonV1::ReturnBlocked,
        }
    );
    assert!(
        engine.world().actors[1]
            .ai
            .as_ref()
            .expect("AI")
            .returning_home
    );
}

#[test]
fn suppression_emits_one_typed_decision_and_no_action() {
    let mut suppressed = actor("bound", Coord { x: 5, y: 2 }, "simple_chase");
    suppressed["active_effects"] = serde_json::json!([active_effect("webbed", "web", true)]);
    let mut engine = engine(vec![suppressed]);
    let events = wait(&mut engine);
    assert_eq!(decisions(&events, "bound").count(), 1);
    assert_eq!(
        decision(&events, "bound"),
        &AutomaticActorDecisionV1::Suppressed {
            status: "web".to_string(),
        }
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::Moved { actor_id, .. } | Event::Attacked { attacker_id: actor_id, .. }
            if actor_id == "bound"
    )));
}

#[test]
fn automatic_decisions_follow_registration_order_on_ties() {
    let mut engine = engine(vec![
        actor("first", Coord { x: 5, y: 1 }, "hold_ground"),
        actor("second", Coord { x: 5, y: 3 }, "hold_ground"),
    ]);
    let events = wait(&mut engine);
    let order = events
        .iter()
        .filter_map(|event| match event {
            Event::AutomaticActorDecision { actor_id, .. } => Some(actor_id.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(order, vec!["first", "second"]);
}
