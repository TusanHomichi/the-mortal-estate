use crate::support::content_parts::ContentParts;
use tme_rules::*;

fn learn_spell_runtime_engine(mutate: impl FnOnce(&mut ContentParts)) -> Engine {
    let mut parts = ContentParts::tracked(
        "spell_learning_purchase_casting_xp",
        "profile/spell_learning_purchase_casting_xp",
    );
    *parts.template_levels_source_mut() = serde_json::json!({
        "start": {
            "law_zone": "none",
            "width": 5,
            "height": 3,
            "cells": [
                [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]],
                [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
                [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]]
            ]
        }
    });
    parts.service_instances_mut()[0]["location"]["level"] = serde_json::json!("start");
    for actor in parts
        .actors_mut()
        .as_array_mut()
        .expect("seed actors")
        .iter_mut()
    {
        actor["location"]["level"] = serde_json::json!("start");
    }
    parts.ground_items_mut()[0]["location"]["level"] = serde_json::json!("start");

    let spell = parts.selected_mut("spells", 0);
    spell["id"] = serde_json::json!("find_target");
    spell["name"] = serde_json::json!("Find Target");
    spell["social"] = serde_json::json!({"hostile_act": false, "town_law": "permitted"});
    spell["lane"] = serde_json::json!("wizard_magic");
    spell["skill_requirement"] = serde_json::json!(1);
    spell["mp_cost"] = serde_json::json!(1);
    spell["stamina_cost"] = serde_json::json!(0);
    spell["effect"] = serde_json::json!({
        "family": "locate",
        "locate": {"subject": "actor", "id": "target", "observed_only": false}
    });
    spell["target"] = serde_json::json!({"kind": "none"});
    spell["acquisition"] = serde_json::json!({"gold_cost": 25});
    spell["casting"] = serde_json::json!({"method": "direct", "cast_class": "not_applicable"});
    parts.profile_value_mut()["spells"] =
        serde_json::json!(["spell/spark/spell_learning_purchase_casting_xp"]);

    let service = parts.selected_mut("service_definitions", 0);
    let teaching = service["capabilities"]
        .as_array_mut()
        .expect("typed service capabilities")
        .iter_mut()
        .find(|capability| capability["kind"] == "spell_teaching")
        .expect("spell teaching capability");
    teaching["teachings"] = serde_json::json!([{"spell_id": "find_target"}]);

    let player = &mut parts.actors_mut()[0];
    player["character"]["progression"] = serde_json::json!({"level": 2, "experience": 100});
    player["character"]["known_spells"] = serde_json::json!([]);
    player["carried"]["gold"]["sack"] = serde_json::json!(40);
    mutate(&mut parts);
    parts.engine(7).expect("learning engine should start")
}

#[test]
fn learn_spell_deducts_cost_retains_book_and_casts_locate_spell() {
    let mut engine = learn_spell_runtime_engine(|_| {});
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spell_book".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("should take the personal Spell Book");

    let learn_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::LearnSpell("find_target".to_string()),
        )
        .expect("should learn spell from local teacher");
    assert!(
        !learn_events
            .events
            .iter()
            .any(|event| matches!(event, Event::ItemConsumed { .. }))
    );
    assert!(learn_events.events.iter().any(|event| matches!(
        event,
        Event::GoldChanged { amount, new_total, .. } if *amount == -25 && *new_total == 15
    )));
    assert!(learn_events.events.iter().any(|event| matches!(
        event,
        Event::SpellLearned {
            spell_id,
            spell_name,
            lane,
            learned_at_level,
            gold_cost,
            spell_book_item_instance_id,
            spell_book_item_definition_id,
            ..
        } if spell_id == "find_target"
            && spell_name == "Find Target"
            && lane == "wizard_magic"
            && *learned_at_level == 2
            && *gold_cost == 25
            && spell_book_item_instance_id == "spell_book"
            && spell_book_item_definition_id == "spell_book"
    )));
    let gold_index = learn_events
        .events
        .iter()
        .position(|event| matches!(event, Event::GoldChanged { .. }))
        .expect("learning should report its gold cost");
    let learned_index = learn_events
        .events
        .iter()
        .position(|event| matches!(event, Event::SpellLearned { .. }))
        .expect("learning should report the learned spell");
    let receipt_index = learn_events
        .events
        .iter()
        .position(|event| matches!(event, Event::TransactionCommitted { .. }))
        .expect("learning should finish with a shared transaction receipt");
    assert!(
        gold_index < learned_index && learned_index < receipt_index,
        "spell-learning events must report gold and learning before the receipt"
    );
    assert!(matches!(
        &learn_events[receipt_index],
        Event::TransactionCommitted {
            source: tme_rules::TransactionSourceV1::SpellLearning {
                spell_id,
                ..
            },
            ..
        } if spell_id == "find_target"
    ));

    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player exists");
    let character = player.character.as_ref().expect("character exists");
    assert_eq!(player.carried.gold.sack, 15);
    assert_eq!(character.known_spells.len(), 1);
    assert_eq!(character.known_spells[0].spell_id, "find_target");
    assert_eq!(character.known_spells[0].lane, "wizard_magic");
    assert_eq!(character.known_spells[0].learned_at_level, 2);
    assert!(
        player
            .carried
            .items
            .get(&CarriedPosition::RightHand)
            .map(String::as_str)
            == Some("spell_book"),
        "the personal Spell Book must be retained"
    );

    let cast_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "find_target".to_string(),
                target: None,
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("learned spell should cast immediately");
    assert!(cast_events.events.iter().any(|event| matches!(
        event,
        Event::Located {
            subject,
            id,
            location,
            ..
        } if subject == "actor"
            && id == "target"
            && location.as_ref().is_some_and(|position| {
                position.level == "start" && position.position == Coord { x: 3, y: 1 }
            })
    )));
}

#[test]
fn learn_spell_runtime_rejects_wrong_owner_book_without_mutation() {
    let mut engine = learn_spell_runtime_engine(|_| {});
    engine
        .world_mut()
        .item_instances
        .get_mut("spell_book")
        .unwrap()
        .binding = ItemBindingState::Bound {
        character_id: serde_json::from_str("\"character:someone_else\"").unwrap(),
    };
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spell_book".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("ordinary item movement permits carrying another character's book");

    let err = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::LearnSpell("find_target".to_string()),
        )
        .expect_err("another character's book must not satisfy learning");
    assert_eq!(err.message(), "spell_book_not_owned");

    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player exists");
    let character = player.character.as_ref().expect("character exists");
    assert_eq!(player.carried.gold.sack, 40, "gold should remain untouched");
    assert!(
        character.known_spells.is_empty(),
        "spell should not be learned"
    );
    assert_eq!(
        player.carried.items.len(),
        1,
        "inventory should remain untouched"
    );
    assert_eq!(
        player
            .carried
            .items
            .get(&tme_rules::CarriedPosition::RightHand)
            .map(String::as_str),
        Some("spell_book")
    );
}

#[test]
fn learn_spell_runtime_rejects_already_known_spell() {
    let mut engine = learn_spell_runtime_engine(|parts| {
        parts.actors_mut()[0]["character"]["known_spells"] = serde_json::json!([{
            "spell_id": "find_target",
            "lane": "wizard_magic",
            "learned_at_level": 1
        }]);
    });
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spell_book".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("should take personal Spell Book");

    let err = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::LearnSpell("find_target".to_string()),
        )
        .expect_err("already known spell should reject");
    assert_eq!(err.message(), "spell_already_known");
}

#[test]
fn training_provider_without_spell_teaching_cannot_teach_spells() {
    let mut engine = learn_spell_runtime_engine(|parts| {
        parts.selected_mut("service_definitions", 0)["capabilities"]
            .as_array_mut()
            .expect("typed service capabilities")
            .retain(|capability| capability["kind"] != "spell_teaching");
    });

    let err = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::LearnSpell("find_target".to_string()),
        )
        .expect_err("skill training alone must not authorize spell teaching");
    assert_eq!(err.message(), "no_service");
}
