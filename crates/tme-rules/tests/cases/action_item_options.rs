use crate::support::content_parts::ContentParts;
use tme_rules::{
    ActionBlockedReasonV1, Engine, PlayerCommandV1, PlayerIntent, PlayerIntentPayloadV1,
};

use crate::action_context_support::common::{first_room_engine, make_command, option_by_id};
use crate::action_context_support::items::*;

fn equipment_engine() -> Engine {
    ContentParts::tracked("equipment_slots", "profile/equipment_slots")
        .engine(7)
        .expect("equipment content should start")
}

#[test]
fn exact_item_command_json_uses_item_instance_id_and_rejects_item_id() {
    let command_json = serde_json::json!({
        "contract_version": tme_rules::COMMAND_CONTRACT_VERSION,
        "actor_id": "player",
        "intent": {
            "move_item": {
                "item_instance_id": "tonic_a",
                "destination": {"kind": "carried", "position": "sack_item_1"}
            }
        }
    });
    let command: PlayerCommandV1 = serde_json::from_value(command_json)
        .expect("current exact-item command should deserialize");
    let serialized = serde_json::to_value(&command).expect("command should serialize");
    assert_eq!(
        serialized["intent"]["move_item"]["item_instance_id"],
        "tonic_a"
    );
    assert!(serialized["intent"]["move_item"].get("item_id").is_none());

    let stale = serde_json::json!({
        "contract_version": tme_rules::COMMAND_CONTRACT_VERSION,
        "actor_id": "player",
        "intent": {
            "move_item": {
                "item_id": "tonic_a",
                "destination": {"kind": "carried", "position": "sack_item_1"}
            }
        }
    });
    assert!(
        serde_json::from_value::<PlayerCommandV1>(stale).is_err(),
        "the obsolete item_id command key must not be accepted"
    );
}

#[test]
fn validate_rejects_move_nonexistent_item_to_sack() {
    let engine = first_room_engine();
    let cmd = make_command(PlayerIntentPayloadV1::MoveItem {
        item_instance_id: "nonexistent".to_string(),
        destination: tme_rules::ItemMoveDestination::Carried {
            position: tme_rules::CarriedPosition::SackItem1,
        },
    });
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::NoSuchItem)
    );
}

#[test]
fn validate_rejects_move_nonexistent_item_to_ground() {
    let engine = first_room_engine();
    let cmd = make_command(PlayerIntentPayloadV1::MoveItem {
        item_instance_id: "nonexistent".to_string(),
        destination: tme_rules::ItemMoveDestination::GroundHere,
    });
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::NoSuchItem)
    );
}

#[test]
fn validate_rejects_drink_non_consumable() {
    let engine = first_room_engine();
    let cmd = make_command(PlayerIntentPayloadV1::Drink {
        item_instance_id: "nonexistent".to_string(),
    });
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::NoSuchItem)
    );
}

#[test]
fn validate_rejects_move_when_exact_destination_is_occupied() {
    let engine = non_weapon_hands_with_ground_weapon_engine();
    let cmd = make_command(PlayerIntentPayloadV1::MoveItem {
        item_instance_id: "training_knife".to_string(),
        destination: tme_rules::ItemMoveDestination::Carried {
            position: tme_rules::CarriedPosition::RightHand,
        },
    });

    let status = engine.validate_actor_command(&cmd).expect("validate");

    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::OccupiedCarriedPosition)
    );
}

#[test]
fn validate_rejects_move_nonexistent_item_to_hand() {
    let engine = first_room_engine();
    let cmd = make_command(PlayerIntentPayloadV1::MoveItem {
        item_instance_id: "nonexistent".to_string(),
        destination: tme_rules::ItemMoveDestination::Carried {
            position: tme_rules::CarriedPosition::RightHand,
        },
    });
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::NoSuchItem)
    );
}

#[test]
fn action_options_disable_active_position_for_stacked_carried_item() {
    let engine = stacked_carried_item_engine();

    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");
    let move_to_hand = option_by_id(&options, "move_tonic_a_to_right_hand");

    assert!(
        !move_to_hand.enabled,
        "a quantity-two stack cannot occupy a hand"
    );
    assert_invalid_item_quantity_reason(move_to_hand.blocked_reason);
}

#[test]
fn validate_rejects_stacked_active_position_with_invalid_item_quantity() {
    let engine = stacked_carried_item_engine();
    let command = make_command(PlayerIntentPayloadV1::MoveItem {
        item_instance_id: "tonic_a".to_string(),
        destination: tme_rules::ItemMoveDestination::Carried {
            position: tme_rules::CarriedPosition::RightHand,
        },
    });

    let status = engine
        .validate_actor_command(&command)
        .expect("typed command should validate");

    assert!(
        !status.accepted,
        "a quantity-two stack cannot occupy a hand"
    );
    assert_invalid_item_quantity_reason(status.blocked_reason);
}

#[test]
fn action_options_include_exact_carried_and_ground_destinations() {
    let mut engine = equipment_engine();

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "training_knife".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("move knife into sack");

    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");
    let moves: Vec<_> = options
        .iter()
        .filter(|option| option.id.starts_with("move_training_knife_to_"))
        .collect();
    assert!(
        moves.len() > 2,
        "knife must expose multiple authored destinations"
    );
    assert!(
        moves
            .iter()
            .any(|option| option.id == "move_training_knife_to_ground_here")
    );
    assert!(
        moves
            .iter()
            .any(|option| option.id == "move_training_knife_to_right_hand")
    );
    for option in &moves {
        if let Some(ref cmd) = option.command {
            match &cmd.intent {
                PlayerIntentPayloadV1::MoveItem {
                    item_instance_id, ..
                } => {
                    assert_eq!(item_instance_id, "training_knife");
                }
                _ => panic!("item option must have MoveItem payload"),
            }
        }
    }
}

#[test]
fn move_options_resolve_placements_through_instance_definitions() {
    let mut value = ContentParts::tracked("equipment_slots", "profile/equipment_slots");
    value.push_selected(
        "items",
        "item/silver_amulet/test",
        serde_json::json!({
            "id": "silver_amulet",
            "kind": "accessory",
            "name": "Silver Amulet",
            "valid_placements": ["sack", "neck"]
        , "economy": {"unit_burden": 1}}),
    );
    value.item_instances_mut()["amulet_instance"] = serde_json::json!({
        "definition_id": "silver_amulet",
        "binding": {"state": "unrestricted"}
    });
    value.actors_mut()[0]["carried"]["items"]
        .as_array_mut()
        .expect("carried items")
        .push(serde_json::json!({
            "item_instance_id": "amulet_instance",
            "position": "sack_item_1"
        }));

    let engine = value.engine(7).expect("content should start");
    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");

    assert!(
        options
            .iter()
            .any(|option| option.id == "move_amulet_instance_to_neck"),
        "instance-backed item should use its definition's valid placements"
    );
    assert!(
        options
            .iter()
            .all(|option| option.id != "move_amulet_instance_to_right_hand"),
        "instance id must not be looked up as a definition id"
    );
}

#[test]
fn action_options_disable_move_for_occupied_position() {
    let mut engine = equipment_engine();

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "leather_jerkin".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("move armor into sack");

    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");
    let move_to_hand = option_by_id(&options, "move_leather_jerkin_to_right_hand");
    assert!(
        !move_to_hand.enabled,
        "move to an occupied carried position must be disabled"
    );
    assert_eq!(
        move_to_hand.blocked_reason,
        Some(ActionBlockedReasonV1::OccupiedCarriedPosition),
        "disabled move must report the exact occupied-position reason"
    );
}

#[test]
fn validate_accepts_move_item_from_sack_to_open_hand() {
    let mut engine = equipment_engine();

    // The fixture has healing_balm on ground at the player's position
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
        .expect("take healing_balm from ground");

    let cmd = make_command(PlayerIntentPayloadV1::MoveItem {
        item_instance_id: "healing_balm".to_string(),
        destination: tme_rules::ItemMoveDestination::Carried {
            position: tme_rules::CarriedPosition::LeftHand,
        },
    });
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(
        status.accepted,
        "exact move of carried item should be accepted, got blocked_reason: {:?}",
        status.blocked_reason
    );
}

#[test]
fn validate_rejects_move_item_not_in_world() {
    let engine = equipment_engine();

    let cmd = make_command(PlayerIntentPayloadV1::MoveItem {
        item_instance_id: "nonexistent".to_string(),
        destination: tme_rules::ItemMoveDestination::Carried {
            position: tme_rules::CarriedPosition::RightHand,
        },
    });
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::NoSuchItem)
    );
}

#[test]
fn validate_accepts_move_from_active_position_to_sack() {
    let engine = equipment_engine();

    let cmd = make_command(PlayerIntentPayloadV1::MoveItem {
        item_instance_id: "training_knife".to_string(),
        destination: tme_rules::ItemMoveDestination::Carried {
            position: tme_rules::CarriedPosition::SackItem1,
        },
    });
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(
        status.accepted,
        "active-to-sack move should be accepted, got blocked_reason: {:?}",
        status.blocked_reason
    );
}

#[test]
fn validate_rejects_move_to_occupied_position() {
    let engine = equipment_engine();

    let cmd = make_command(PlayerIntentPayloadV1::MoveItem {
        item_instance_id: "leather_jerkin".to_string(),
        destination: tme_rules::ItemMoveDestination::Carried {
            position: tme_rules::CarriedPosition::RightHand,
        },
    });
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::OccupiedCarriedPosition)
    );
}
