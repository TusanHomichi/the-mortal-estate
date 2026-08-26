#![allow(dead_code)]

use super::*;
use crate::action_context_support::common::ensure_current_class_growth_profile;
use crate::support::content_parts::ContentParts;

pub(crate) fn learn_spell_context_engine(mutate: impl FnOnce(&mut ContentParts)) -> Engine {
    let mut parts = ContentParts::tracked(
        "spell_learning_purchase_casting_xp",
        "profile/spell_learning_purchase_casting_xp",
    );
    let hall = parts.world_template["realms"]["realm_0"]["levels"]["room_0"].clone();
    parts.template_levels_source_mut()["hall"] = hall;
    let spell = parts.selected_mut("spells", 0);
    spell["id"] = serde_json::json!("find_target");
    spell["name"] = serde_json::json!("Find Target");
    spell["social"] = serde_json::json!({"hostile_act": false, "town_law": "permitted"});
    spell["skill_requirement"] = serde_json::json!(1);
    spell["mp_cost"] = serde_json::json!(1);
    spell["stamina_cost"] = serde_json::json!(0);
    spell["effect"] = serde_json::json!({
        "family": "locate",
        "locate": {"subject": "actor", "id": "target"}
    });
    spell["target"] = serde_json::json!({"kind": "none"});
    spell["acquisition"] = serde_json::json!({"gold_cost": 25});
    spell["casting"] = serde_json::json!({"method": "direct", "cast_class": "not_applicable"});
    parts.selected_mut("service_definitions", 0)["capabilities"][2]["teachings"] =
        serde_json::json!([{"spell_id": "find_target"}]);
    let player = &mut parts.actors_mut()[0];
    player["character"]["progression"] = serde_json::json!({"level": 2, "experience": 100});
    player["character"]["known_spells"] = serde_json::json!([]);
    player["carried"]["gold"]["sack"] = serde_json::json!(40);
    mutate(&mut parts);
    ensure_current_class_growth_profile(&mut parts);
    parts.engine(7).expect("engine should start")
}
