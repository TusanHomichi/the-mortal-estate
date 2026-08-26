use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};
use tme_rules::{Direction, Engine, Event, LogicalTime, PlayerIntent};

fn actor_index(engine: &Engine, actor_id: &str) -> usize {
    engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == actor_id)
        .expect("actor")
}

fn martial_artist_block_engine(hand_level: u8, armed: bool) -> Engine {
    martial_artist_block_engine_for_class("martial_artist", hand_level, armed)
}

fn martial_artist_block_engine_for_class(class_id: &str, hand_level: u8, armed: bool) -> Engine {
    let mut parts = ContentParts::tracked(
        "martial_hand_block_actions",
        "profile/martial_hand_block_actions",
    );
    let character = &mut parts.actors_mut()[0]["character"];
    character["identity"]["base_class_id"] = json!(class_id);
    character["identity"]["current_class_id"] = json!(class_id);
    character["identity"]["display_class"] = json!(if class_id == "martial_artist" {
        "Martial Artist"
    } else {
        "Fighter"
    });
    character["skill_ledger"][0]["level"] = json!(hand_level);

    if class_id != "martial_artist" {
        parts.rules_source_mut()["progression"]["growth_profiles"][0]["class_id"] = json!(class_id);
    }

    if armed {
        parts.profile_value_mut()["items"]
            .as_array_mut()
            .expect("profile items")
            .push(Value::String("item/training_knife/first_room".to_string()));
        parts.item_instances_mut()["training_knife"] = json!({
            "definition_id": "training_knife",
            "binding": {"state": "unrestricted"}
        });
        parts.actors_mut()[0]["carried"]["items"] = json!([{
            "item_instance_id": "training_knife",
            "position": "right_hand"
        }]);
    }

    let mut engine = parts.engine(7).expect("martial hand-block graph");
    let monster = actor_index(&engine, "sparring_beast");
    engine.world_mut().actors[monster].attack_ready_at = LogicalTime::new(2);
    engine
        .apply_realtime_actor_intent(
            &tme_rules::ActorId::from("player0"),
            PlayerIntent::MovePath(vec![Direction::South]),
        )
        .expect("engage monster");
    engine
}

#[test]
fn martial_artist_unarmed_hand_level_can_block_before_damage() {
    let mut engine = martial_artist_block_engine(19, false);
    let events = engine.advance_realtime_boundary().expect("monster attacks");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::AttackBlocked {
            defender,
            source: tme_rules::model::BlockSourceKind::RightMartialHand,
            chance_percent,
            ..
        } if defender == "Hand Adept" && *chance_percent == 100
    )));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player0"))
            .expect("player")
            .hp,
        20
    );
}

#[test]
fn martial_hand_block_requires_martial_artist_unarmed_defender() {
    let mut fighter = martial_artist_block_engine_for_class("fighter", 19, false);
    let fighter_events = fighter
        .advance_realtime_boundary()
        .expect("monster attacks");
    assert!(!fighter_events.iter().any(|event| matches!(
        event,
        Event::AttackBlocked {
            source: tme_rules::model::BlockSourceKind::RightMartialHand,
            ..
        }
    )));

    let mut armed = martial_artist_block_engine(19, true);
    let armed_events = armed.advance_realtime_boundary().expect("monster attacks");
    assert!(!armed_events.iter().any(|event| matches!(
        event,
        Event::AttackBlocked {
            source: tme_rules::model::BlockSourceKind::RightMartialHand,
            ..
        }
    )));
}

#[test]
fn martial_hand_block_requires_hand_level_at_or_above_minimum() {
    let mut engine = martial_artist_block_engine(0, false);
    let events = engine.advance_realtime_boundary().expect("monster attacks");

    assert!(!events.iter().any(|event| matches!(
        event,
        Event::AttackBlocked {
            source: tme_rules::model::BlockSourceKind::RightMartialHand,
            ..
        }
    )));
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player0"))
            .expect("player")
            .hp
            < 20
    );
}
