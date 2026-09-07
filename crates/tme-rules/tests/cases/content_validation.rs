use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};
use tme_rules::{CatalogProfileKey, Engine};

const TRACKED_CASES: &[&str] = &[
    "alignment_social_law",
    "area_path_terrain_spells",
    "balm_cache",
    "character_sheet",
    "combat_labels",
    "control_poison_protection",
    "creature_ecology_gallery",
    "death_corpse",
    "equipment_slots",
    "fidelity_gallery",
    "first_land_structure",
    "first_room",
    "gargoyle_threshold",
    "gold_bank_locker_storage",
    "gold_training",
    "inspect_room",
    "item_instance_contract",
    "knight_promotion",
    "knight_social_consequence",
    "knight_support_actions",
    "kobold_warren",
    "magic_profession_gallery",
    "martial_hand_block_actions",
    "merchant_item_services",
    "monster_spellcasting_special_attacks",
    "npc_quest_interactions",
    "profession_specific_actions",
    "progression_gallery",
    "ranged_attack",
    "reach_attack",
    "remaining_spell_effect_families",
    "resource_movement",
    "resting_hollow",
    "restoration_services",
    "service_transactions",
    "skill_progression",
    "spell_effects",
    "spell_learning_purchase_casting_xp",
    "spell_readiness",
    "spider_gallery",
    "starter_circuit",
    "status_effects",
    "summons_created_creature_lifecycle",
    "supply_cache",
    "terrain_movement",
    "thrown_attack",
    "town_adventure_loop_gallery",
    "troll_track",
    "undercroft_loop",
    "utility_door_secret_item_spells",
    "world_topology_gallery",
    "xp_progression",
];

fn profile_for(case_id: &str) -> String {
    if case_id == "inspect_room" {
        "profile/combat_labels".to_string()
    } else {
        format!("profile/{case_id}")
    }
}

fn parts(case_id: &str) -> ContentParts {
    ContentParts::tracked(case_id, &profile_for(case_id))
}

fn definition_error(parts: &ContentParts) -> String {
    parts
        .definition()
        .expect_err("definition mutation must fail")
}

fn seed_error(parts: &ContentParts) -> String {
    parts.validated_seed().expect_err("seed mutation must fail")
}

fn decode_error(parts: &ContentParts) -> String {
    parts.decode().expect_err("strict decode must fail")
}

fn assert_has(error: &str, expected: &str) {
    assert!(
        error.contains(expected),
        "expected {expected:?} in diagnostic:\n{error}"
    );
}

fn selected_key(parts: &ContentParts, registry: &str, index: usize) -> String {
    parts.profile_value()[registry][index]
        .as_str()
        .unwrap_or_else(|| panic!("{registry}[{index}] selected key"))
        .to_string()
}

fn selected_row(parts: &ContentParts, registry: &str, index: usize) -> Value {
    let key = selected_key(parts, registry, index);
    parts.catalog[registry][key].clone()
}

fn select_registry_row_by_runtime_id(parts: &mut ContentParts, registry: &str, id: &str) {
    let key = parts.catalog[registry]
        .as_object()
        .unwrap_or_else(|| panic!("{registry} registry"))
        .iter()
        .find_map(|(key, row)| (row["id"] == id).then(|| key.clone()))
        .unwrap_or_else(|| panic!("{registry} row with runtime id {id:?}"));
    let selected = parts.profile_value_mut()[registry]
        .as_array_mut()
        .unwrap_or_else(|| panic!("{registry} profile selection"));
    assert!(
        !selected.iter().any(|selected_key| selected_key == &key),
        "{registry} row {key:?} already selected"
    );
    selected.push(Value::String(key));
}

fn selected_service_capability(parts: &ContentParts, kind: &str) -> (usize, usize) {
    for definition_index in 0..parts.selected_len("service_definitions") {
        let definition = selected_row(parts, "service_definitions", definition_index);
        if let Some(capability_index) =
            definition["capabilities"]
                .as_array()
                .and_then(|capabilities| {
                    capabilities
                        .iter()
                        .position(|capability| capability["kind"] == kind)
                })
        {
            return (definition_index, capability_index);
        }
    }
    panic!("selected service capability {kind:?}");
}

fn actor_seed_index(parts: &ContentParts, actor_id: &str) -> usize {
    parts.world_seed["actors"]
        .as_array()
        .expect("seed actors")
        .iter()
        .position(|actor| actor["id"] == actor_id)
        .unwrap_or_else(|| panic!("seed actor {actor_id:?}"))
}

#[path = "content_validation/actors.rs"]
mod actors;
#[path = "content_validation/combat.rs"]
mod combat;
#[path = "content_validation/inventory.rs"]
mod inventory;
#[path = "content_validation/magic.rs"]
mod magic;
#[path = "content_validation/progression.rs"]
mod progression;
#[path = "content_validation/registry.rs"]
mod registry;
#[path = "content_validation/services.rs"]
mod services;
