#![allow(dead_code)]

use super::*;
use crate::support::content_parts::ContentParts;

pub(super) fn suppressed_options_engine() -> Engine {
    let mut parts = ContentParts::tracked("spell_effects", "profile/spell_effects");
    parts.actors_mut()[0]["active_effects"] = serde_json::json!([{
        "instance_id": "stun_1",
        "effect_id": "stun_effect",
        "source": {"kind": "fixture", "id": "suppressed_options"},
        "kind": "control_status",
        "tags": ["stun"],
        "potency": 0,
        "remaining_rounds": 2,
        "stacking": "replace_same_kind",
        "start_delay_rounds": 0,
        "tick_interval_rounds": 1,
        "suppresses_action": true,
        "resistance_boosts": []
    }]);
    parts.actors_mut()[1]["id"] = serde_json::json!("watcher");
    parts.actor_definition_mut(1)["name"] = serde_json::json!("Watcher");
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 3, "y": 1});
    parts.push_selected(
        "items",
        "item/healing_balm/suppressed_test",
        serde_json::json!({
            "id": "healing_balm",
            "kind": "consumable",
            "name": "Healing Balm",
            "category": "consumable",
            "consumable": {"effect": "healing", "heal_per_round": 2},
            "valid_placements": ["hand", "sack"],
            "economy": {"unit_burden": 1}
        }),
    );
    parts.item_instances_mut()["healing_balm"] = serde_json::json!({
        "definition_id": "healing_balm",
        "binding": {"state": "unrestricted"}
    });
    parts
        .ground_items_mut()
        .as_array_mut()
        .expect("ground items")
        .push(serde_json::json!({
            "item_instance_id": "healing_balm",
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 1, "y": 1}
            }
        }));
    parts.engine(7).expect("suppressed content should start")
}
