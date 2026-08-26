//! Focused tests for weapon combat-add application.

use crate::support::content_parts::ContentParts;
use serde_json::json;
use tme_rules::{Coord, Direction, Engine, Event, LogicalTime, PhysicalAttackMode, PlayerIntent};

fn actor_index(engine: &Engine, actor_id: &str) -> usize {
    engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == actor_id)
        .expect("actor")
}

fn melee_engine(combat_add_rating: i32, player_attack: i32, monster_hp: i32) -> Engine {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.selected_by_runtime_id_mut("items", "training_knife")["weapon"]["combat_add_rating"] =
        json!(combat_add_rating);
    parts.actor_definition_mut(0)["stats"]["attack"] = json!(player_attack);
    parts.actors_mut()[1]["id"] = json!("target");
    parts.actor_definition_mut(1)["name"] = json!("Target");
    parts.actors_mut()[1]["location"]["position"] = json!({"x": 1, "y": 2});
    parts.actor_definition_mut(1)["stats"]["hp"] = json!(monster_hp);
    parts.actor_definition_mut(1)["stats"]["attack"] = json!(1);
    parts.actor_definition_mut(1)["ai"]["behavior"] = json!("hold_ground");

    let mut engine = parts.engine(1_010_580_540).expect("melee graph");
    let target = actor_index(&engine, "target");
    engine.world_mut().actors[target].attack_ready_at = LogicalTime::new(99);
    engine
}

fn unarmed_engine(player_attack: i32) -> Engine {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.actor_definition_mut(0)["stats"]["attack"] = json!(player_attack);
    parts.actors_mut()[0]["carried"]["items"] = json!([]);
    parts
        .item_instances_mut()
        .as_object_mut()
        .expect("item registry")
        .remove("training_knife");
    parts.actors_mut()[1]["id"] = json!("target");
    parts.actor_definition_mut(1)["name"] = json!("Target");
    parts.actors_mut()[1]["location"]["position"] = json!({"x": 1, "y": 2});
    parts.actor_definition_mut(1)["stats"]["hp"] = json!(12);
    parts.actor_definition_mut(1)["stats"]["attack"] = json!(1);
    parts.actor_definition_mut(1)["ai"]["behavior"] = json!("hold_ground");
    let mut engine = parts.engine(7).expect("unarmed graph");
    let target = actor_index(&engine, "target");
    engine.world_mut().actors[target].attack_ready_at = LogicalTime::new(99);
    engine
}

fn thrown_engine(rng_seed: u64) -> Engine {
    let mut parts = ContentParts::tracked("thrown_attack", "profile/thrown_attack");
    parts.actor_definition_mut(0)["stats"]["attack"] = json!(4);
    parts.actors_mut()[1]["id"] = json!("target");
    parts.actor_definition_mut(1)["name"] = json!("Target");
    parts.actor_definition_mut(1)["stats"]["hp"] = json!(12);
    parts.actor_definition_mut(1)["stats"]["attack"] = json!(1);
    parts.actor_definition_mut(1)["ai"]["behavior"] = json!("hold_ground");
    parts.selected_by_runtime_id_mut("items", "oak_javelin")["weapon"]["combat_add_rating"] =
        json!(1);
    parts.engine(rng_seed).expect("thrown graph")
}

fn attack(engine: &mut Engine, mode: PhysicalAttackMode) -> Vec<Event> {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode,
                target_actor_id: "target".into(),
            },
        )
        .expect("attack")
        .events
}

#[test]
fn combat_add_zero_unchanged_damage() {
    let mut engine = melee_engine(0, 6, 12);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::South]),
        )
        .expect("move");
    let events = attack(&mut engine, PhysicalAttackMode::Fight);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::Attacked {
            roll: 11,
            damage: 8,
            defender_hp: 4,
            ..
        }
    )));
}

#[test]
fn combat_add_two_adds_exactly_two() {
    let mut engine = melee_engine(2, 4, 12);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::South]),
        )
        .expect("move");
    let events = attack(&mut engine, PhysicalAttackMode::Fight);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::Attacked {
            roll: 11,
            damage: 8,
            defender_hp: 4,
            ..
        }
    )));
}

#[test]
fn unarmed_no_bonus() {
    let mut engine = unarmed_engine(6);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::South]),
        )
        .expect("move");
    let events = attack(&mut engine, PhysicalAttackMode::Fight);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::Attacked {
            roll: 11,
            damage: 8,
            ..
        }
    )));
}

#[test]
fn thrown_weapon_applies_bonus_when_wielded() {
    let mut engine = thrown_engine(1_010_580_540);
    let events = attack(&mut engine, PhysicalAttackMode::Throw);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::Attacked {
            roll: 11,
            damage: 7,
            defender_hp: 5,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ItemRelocated {
            item,
            reason: tme_rules::ItemRelocationReason::Thrown,
            ..
        } if item == "Oak Javelin"
    )));
}

#[test]
fn unarmed_after_thrown_no_bonus() {
    let mut engine = thrown_engine(7);
    attack(&mut engine, PhysicalAttackMode::Throw);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("approach");
    let target_index = actor_index(&engine, "target");
    let player_index = actor_index(&engine, "player");
    let target = engine.world().actors[target_index].location.position;
    engine.world_mut().actors[player_index].location.position = Coord {
        x: target.x,
        y: target.y,
    };
    let events = attack(&mut engine, PhysicalAttackMode::Fight);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::AttackMissed {
            attacker,
            attacker_score: 2,
            ..
        } if attacker == "Delver"
    )));
}
