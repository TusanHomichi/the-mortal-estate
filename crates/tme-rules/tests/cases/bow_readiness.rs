use crate::support::content_parts::ContentParts;
use serde_json::json;
use tme_rules::{
    BowReadiness, BowReadinessChangeReason, CarriedPosition, Direction, Engine, Event,
    ItemMoveDestination, PlayerIntent,
};

fn value() -> ContentParts {
    ContentParts::tracked("ranged_attack", "profile/ranged_attack")
}

fn engine(parts: ContentParts, seed: u64) -> Engine {
    parts.engine(seed).expect("engine")
}

#[test]
fn nock_and_explicit_unload_are_typed_standard_actions() {
    let mut engine = engine(value(), 7);
    let nock = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Nock)
        .expect("nock");
    assert!(nock.iter().any(|event| matches!(
        event,
        Event::BowReadinessChanged {
            from: BowReadiness::Unnocked,
            to: BowReadiness::Nocked,
            reason: BowReadinessChangeReason::Nocked,
            ..
        }
    )));
    let snapshot = engine.snapshot();
    let player = snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .unwrap();
    let selected = player.physical_weapon.as_ref().unwrap();
    assert_eq!(selected.item_definition_id.as_deref(), Some("elm_bow"));
    assert_eq!(selected.nocking_unloads_on_movement, Some(true));
    assert_eq!(selected.bow_readiness, Some(BowReadiness::Nocked));
    assert_eq!(
        player
            .carried
            .items
            .iter()
            .find(|item| item.item.item_instance_id == "elm_bow")
            .unwrap()
            .item
            .bow_readiness,
        Some(BowReadiness::Nocked)
    );
    let unload = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::UnloadBow)
        .expect("unload");
    assert!(unload.iter().any(|event| matches!(
        event,
        Event::BowReadinessChanged {
            to: BowReadiness::Unnocked,
            reason: BowReadinessChangeReason::ExplicitUnload,
            ..
        }
    )));
}

#[test]
fn bow_attack_requires_and_consumes_one_nock() {
    let mut engine = engine(value(), 7);
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Shoot,
                target_actor_id: "reedling".into(),
            },
        )
        .expect_err("unnocked attack");
    assert!(error.message().contains("bow is not nocked"));
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Nock)
        .expect("nock");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Shoot,
                target_actor_id: "reedling".into(),
            },
        )
        .expect("shot");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::BowReadinessChanged {
            reason: BowReadinessChangeReason::Shot,
            to: BowReadiness::Unnocked,
            ..
        }
    )));
}

#[test]
fn successful_movement_unloads_authored_bows_once() {
    let mut engine = engine(value(), 7);
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Nock)
        .expect("nock");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("move");
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                Event::BowReadinessChanged {
                    reason: BowReadinessChangeReason::Movement,
                    ..
                }
            ))
            .count(),
        1
    );
}

#[test]
fn occupying_left_hand_unloads_bow_through_inventory_transaction() {
    let mut fixture = value();
    fixture.push_selected(
        "items",
        "item/token/test",
        json!({
            "id":"token","kind":"gear","name":"Token",
            "valid_placements":["hand","sack"],"economy":{"unit_burden":0}
        }),
    );
    fixture.item_instances_mut()["token"] = json!({
        "definition_id":"token","binding":{"state":"unrestricted"}
    });
    fixture.actors_mut()[0]["carried"]["items"]
        .as_array_mut()
        .unwrap()
        .push(json!({"item_instance_id":"token","position":"sack_item_1"}));
    let mut engine = engine(fixture, 7);
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Nock)
        .expect("nock");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "token".to_string(),
                destination: ItemMoveDestination::Carried {
                    position: CarriedPosition::LeftHand,
                },
            },
        )
        .expect("occupy left hand");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::BowReadinessChanged {
            reason: BowReadinessChangeReason::LeftHandOccupied,
            ..
        }
    )));
}

#[test]
fn relocating_a_bow_away_from_right_hand_unloads_once() {
    let mut engine = engine(value(), 7);
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Nock)
        .expect("nock");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "elm_bow".to_string(),
                destination: ItemMoveDestination::Carried {
                    position: CarriedPosition::SackItem1,
                },
            },
        )
        .expect("belted bow relocation");
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                Event::BowReadinessChanged {
                    item_instance_id,
                    reason: BowReadinessChangeReason::LeftRightHand,
                    ..
                } if item_instance_id == "elm_bow"
            ))
            .count(),
        1
    );
}

#[test]
fn command_status_exposes_nock_and_unload_state() {
    let mut engine = engine(value(), 7);
    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");
    assert!(
        options
            .iter()
            .any(|option| option.id == "nock" && option.enabled)
    );
    assert!(
        options
            .iter()
            .any(|option| option.id == "unload_bow" && !option.enabled)
    );
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Nock)
        .expect("nock");
    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");
    assert!(
        options
            .iter()
            .any(|option| option.id == "nock" && !option.enabled)
    );
    assert!(
        options
            .iter()
            .any(|option| option.id == "unload_bow" && option.enabled)
    );
}
