use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};
use tme_rules::{
    ActionBlockedReasonV1, Engine, Event, PlayerIntent, SpellItemLocation, SpellTarget,
};

fn contract_value() -> ContentParts {
    ContentParts::tracked("item_instance_contract", "profile/item_instance_contract")
}

fn engine_from_value(mut value: ContentParts) -> Engine {
    let class_id = value.world_seed["actors"][0]["character"]["identity"]["current_class_id"]
        .as_str()
        .expect("current class")
        .to_string();
    let clean_profiles =
        value.catalog["rules_profiles"]["rules/first_room"]["progression"]["growth_profiles"]
            .as_array()
            .expect("clean growth profiles")
            .clone();
    let profiles = value.rules_source_mut()["progression"]["growth_profiles"]
        .as_array_mut()
        .expect("growth profiles");
    if !profiles
        .iter()
        .any(|profile| profile["class_id"] == class_id)
    {
        profiles.push(
            clean_profiles
                .iter()
                .find(|profile| profile["class_id"] == class_id)
                .unwrap_or_else(|| panic!("missing clean profile for {class_id:?}"))
                .clone(),
        );
    }
    value.engine(7).expect("engine should start")
}

fn contract_engine() -> Engine {
    contract_value().engine(7).expect("CD engine should start")
}

fn move_tonics_to_sack(value: &mut ContentParts) {
    value.actors_mut()[0]["carried"]["items"] = json!([
        {"item_instance_id": "tonic_b", "position": "sack_item_1"},
        {"item_instance_id": "tonic_a", "position": "sack_item_2"}
    ]);
    *value.ground_items_mut() = json!([]);
}

fn configure_wizard(value: &mut ContentParts, known_spell_ids: &[&str]) {
    let character = &mut value.actors_mut()[0]["character"];
    character["identity"]["base_class_id"] = json!("wizard");
    character["identity"]["current_class_id"] = json!("wizard");
    character["identity"]["display_class"] = json!("Wizard");
    character["resources"]["mp"] = json!(20);
    character["resources"]["max_mp"] = json!(20);
    character["skill_ledger"] = json!([{"track_id": "wizard_magic", "level": 1, "critique_rank": 0, "practice_points": 0, "learning_rate": 1}]);
    character["known_spells"] = Value::Array(
        known_spell_ids
            .iter()
            .map(|spell_id| {
                json!({
                    "spell_id": spell_id,
                    "lane": "wizard_magic",
                    "learned_at_level": 1
                })
            })
            .collect(),
    );
}

fn push_item(value: &mut ContentParts, id: &str, item: Value) {
    value.push_selected(
        "items",
        &format!("item/{id}/item_instance_contract_test"),
        item,
    );
}

fn push_spell(value: &mut ContentParts, id: &str, spell: Value) {
    value.push_selected(
        "spells",
        &format!("spell/{id}/item_instance_contract_test"),
        spell,
    );
}

fn select_existing(value: &mut ContentParts, registry: &str, key: &str) {
    value.profile_value_mut()[registry]
        .as_array_mut()
        .unwrap_or_else(|| panic!("{registry} profile selection"))
        .push(Value::String(key.to_string()));
}

#[path = "item_instance_contract/duplicate_definitions_are_selected_by_exact_instance.rs"]
mod duplicate_definitions_are_selected_by_exact_instance;

#[path = "item_instance_contract/transform_rejects_prospective_burden_overflow_atomically.rs"]
mod transform_rejects_prospective_burden_overflow_atomically;
