#![allow(dead_code)]

use super::*;
use crate::support::content_parts::ContentParts;

pub(crate) fn non_weapon_hands_with_ground_weapon_engine() -> Engine {
    let mut parts = ContentParts::tracked("equipment_slots", "profile/equipment_slots");
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
        .expect("occupied hands content should start")
}

pub(crate) fn stacked_carried_item_engine() -> Engine {
    let mut engine =
        ContentParts::tracked("item_instance_contract", "profile/item_instance_contract")
            .engine(7)
            .expect("item-instance content should start");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "tonic_a".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("take stacked tonic");
    engine
}

pub(crate) fn assert_invalid_item_quantity_reason(reason: Option<ActionBlockedReasonV1>) {
    assert_eq!(reason, Some(ActionBlockedReasonV1::InvalidItemQuantity));
    let reason = reason.expect("stacked active-position move should have one blocked reason");
    assert_eq!(reason.code(), "invalid_item_quantity");
    assert_eq!(reason.to_string(), "invalid item quantity");
    assert_eq!(
        serde_json::to_value(reason).expect("blocked reason should serialize"),
        serde_json::json!("invalid_item_quantity")
    );
}
