use crate::ai_support::{
    automatic_actor, decision, engine, engine_from_value, open_room_value, unrestricted, wait,
};
use tme_rules::{
    AutomaticActorDecisionV1, AutomaticMovementPurposeV1, Coord, Direction, Event, LogicalTime,
    PhysicalAttackMode,
};

fn weapon_definition(
    id: &str,
    name: &str,
    handedness: &str,
    attack_modes: serde_json::Value,
    nocking: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut weapon = serde_json::json!({
        "skill_track_id": "staff",
        "default_attack_mode": attack_modes[0]["mode"],
        "attack_modes": attack_modes,
        "cooldown_units": 1,
        "combat_add_rating": 1,
        "handedness": handedness,
        "block_value": 0,
    });
    if let Some(nocking) = nocking {
        weapon["nocking"] = nocking;
    }
    serde_json::json!({
        "id": id,
        "kind": "weapon",
        "name": name,
        "weapon": weapon,
        "valid_placements": ["hand", "belt_side", "belt_back", "sack"],
        "economy": {"unit_burden": 1},
    })
}

fn armed_engine(
    actor_x: i32,
    ai_modes: &[&str],
    definition: serde_json::Value,
) -> tme_rules::Engine {
    let definition_id = definition["id"].as_str().expect("weapon id").to_string();
    let instance_id = format!("{definition_id}_instance");
    let mut actor = automatic_actor(
        "attacker",
        "chaotic",
        Coord { x: actor_x, y: 2 },
        "simple_chase",
        1,
        unrestricted(),
        ai_modes,
    );
    actor["carried"] = serde_json::json!({
        "items": [{"item_instance_id": instance_id, "position": "right_hand"}],
        "gold": {"left_hand": 0, "right_hand": 0, "sack": 0},
    });
    let mut value = open_room_value(vec![actor]);
    value.push_selected(
        "items",
        &format!("item/{definition_id}/automatic_actor_physical"),
        definition,
    );
    value.item_instances_mut()[&instance_id] = serde_json::json!({
        "definition_id": definition_id,
        "binding": {"state": "unrestricted"},
    });
    engine_from_value(value)
}

fn polearm() -> serde_json::Value {
    weapon_definition(
        "ash_polearm",
        "Ash Polearm",
        "two_handed",
        serde_json::json!([
            {"mode": "poke", "maximum_range": 1, "damage_kind": "piercing"}
        ]),
        None,
    )
}

fn test_bow() -> serde_json::Value {
    weapon_definition(
        "test_bow",
        "Test Bow",
        "bow",
        serde_json::json!([
            {"mode": "shoot", "maximum_range": 3, "damage_kind": "piercing"}
        ]),
        Some(serde_json::json!({"unloads_on_movement": true})),
    )
}

fn javelin() -> serde_json::Value {
    weapon_definition(
        "oak_javelin",
        "Oak Javelin",
        "one_handed",
        serde_json::json!([
            {"mode": "fight", "maximum_range": 0, "damage_kind": "piercing"},
            {"mode": "throw", "maximum_range": 3, "damage_kind": "piercing"}
        ]),
        None,
    )
}

#[test]
fn authored_order_skips_unavailable_mode_and_uses_later_legal_mode() {
    let mut engine = armed_engine(2, &["shoot", "poke"], polearm());
    let events = wait(&mut engine);
    assert!(matches!(
        decision(&events, "attacker"),
        AutomaticActorDecisionV1::PhysicalAttack {
            mode: PhysicalAttackMode::Poke,
            ..
        }
    ));
}

#[test]
fn jumpkick_at_distance_then_fight_on_shared_hex() {
    let mut engine = engine(vec![automatic_actor(
        "attacker",
        "chaotic",
        Coord { x: 2, y: 2 },
        "simple_chase",
        1,
        unrestricted(),
        &["jumpkick", "fight"],
    )]);
    let first = wait(&mut engine);
    assert!(matches!(
        decision(&first, "attacker"),
        AutomaticActorDecisionV1::PhysicalAttack {
            mode: PhysicalAttackMode::Jumpkick,
            ..
        }
    ));
    engine.world_mut().actors[1].location.position = Coord { x: 1, y: 2 };
    let second = wait(&mut engine);
    assert!(matches!(
        decision(&second, "attacker"),
        AutomaticActorDecisionV1::PhysicalAttack {
            mode: PhysicalAttackMode::Fight,
            ..
        }
    ));
}

#[test]
fn kick_is_selected_on_shared_hex() {
    let mut engine = engine(vec![automatic_actor(
        "attacker",
        "chaotic",
        Coord { x: 1, y: 2 },
        "hold_ground",
        1,
        unrestricted(),
        &["kick"],
    )]);
    assert!(matches!(
        decision(&wait(&mut engine), "attacker"),
        AutomaticActorDecisionV1::PhysicalAttack {
            mode: PhysicalAttackMode::Kick,
            ..
        }
    ));
}

#[test]
fn unavailable_authored_mode_does_not_infer_fight_fallback() {
    let mut engine = engine(vec![automatic_actor(
        "attacker",
        "chaotic",
        Coord { x: 3, y: 2 },
        "simple_chase",
        1,
        unrestricted(),
        &["shoot"],
    )]);
    assert_eq!(
        decision(&wait(&mut engine), "attacker"),
        &AutomaticActorDecisionV1::Move {
            direction: Direction::West,
            purpose: AutomaticMovementPurposeV1::Chase,
        }
    );
}

#[test]
fn unnocked_shoot_nocks_before_later_legal_mode_then_shoots() {
    let mut engine = armed_engine(2, &["shoot", "jumpkick"], test_bow());
    let first = wait(&mut engine);
    assert!(matches!(
        decision(&first, "attacker"),
        AutomaticActorDecisionV1::Nock {
            item_definition_id,
            ..
        } if item_definition_id == "test_bow"
    ));
    assert!(!first.iter().any(|event| matches!(
        event,
        Event::Attacked { attacker_id, .. } if attacker_id == "attacker"
    )));

    let second = wait(&mut engine);
    assert!(matches!(
        decision(&second, "attacker"),
        AutomaticActorDecisionV1::PhysicalAttack {
            mode: PhysicalAttackMode::Shoot,
            ..
        }
    ));
}

#[test]
fn thrown_weapon_relocates_then_unavailable_throw_becomes_chase() {
    let mut engine = armed_engine(3, &["throw"], javelin());
    let first = wait(&mut engine);
    assert!(matches!(
        decision(&first, "attacker"),
        AutomaticActorDecisionV1::PhysicalAttack {
            mode: PhysicalAttackMode::Throw,
            ..
        }
    ));
    assert!(first.iter().any(|event| matches!(
        event,
        Event::ItemRelocated {
            reason: tme_rules::ItemRelocationReason::Thrown,
            ..
        }
    )));

    let second = wait(&mut engine);
    assert!(matches!(
        decision(&second, "attacker"),
        AutomaticActorDecisionV1::Move {
            purpose: AutomaticMovementPurposeV1::Chase,
            ..
        }
    ));
}

#[test]
fn automatic_attack_uses_shared_damage_death_and_reward_path() {
    let mut attacker = automatic_actor(
        "attacker",
        "chaotic",
        Coord { x: 3, y: 2 },
        "hold_ground",
        1,
        unrestricted(),
        &["fight"],
    );
    attacker["stats"]["attack"] = serde_json::json!(100);
    let mut target = automatic_actor(
        "target",
        "lawful",
        Coord { x: 3, y: 2 },
        "hold_ground",
        1,
        unrestricted(),
        &["fight"],
    );
    target["stats"]["hp"] = serde_json::json!(1);
    target["xp_value"] = serde_json::json!(3);
    let mut value = open_room_value(vec![attacker, target]);
    value.actor_definition_mut(0)["social"]["alignment_source"] =
        serde_json::json!({"kind": "inherent", "alignment": "neutral"});
    let mut engine = engine_from_value(value);
    let events = wait(&mut engine);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::Attacked { attacker_id, .. } if attacker_id == "attacker"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ActorDefeated { actor_id, .. } if actor_id == "target"
    )));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::PhysicalPracticeEvaluated { .. }))
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Event::DefeatRewardEvaluated {
            awarded_experience: 0,
            reason,
            ..
        } if reason == "no_eligible_player_contribution"
    )));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::ExperienceAwarded { .. }))
    );
}

#[test]
fn attack_not_ready_still_consumes_authored_cadence() {
    let mut actor = automatic_actor(
        "attacker",
        "chaotic",
        Coord { x: 1, y: 2 },
        "hold_ground",
        2,
        unrestricted(),
        &["fight"],
    );
    actor["stats"]["attack"] = serde_json::json!(10);
    let mut engine = engine(vec![actor]);
    engine.world_mut().actors[1].attack_ready_at = LogicalTime::new(5);
    let events = wait(&mut engine);
    assert!(matches!(
        decision(&events, "attacker"),
        AutomaticActorDecisionV1::PhysicalAttack { .. }
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::AttackNotReady { actor_id, .. } if actor_id == "attacker"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ActorReadinessScheduled {
            actor_id,
            cost_units: 2,
            ..
        } if actor_id == "attacker"
    )));
}
