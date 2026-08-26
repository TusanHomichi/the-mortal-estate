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

#[test]
fn catalog_six_scavenging_profiles_are_strict_bounded_and_referentially_complete() {
    const PROFILE: &str = "scavenging/original_provisional";
    for (field, value, expected) in [
        ("search_radius", json!(7), "search_radius must be at most 6"),
        (
            "balm_below_hp_percent",
            json!(0),
            "balm_below_hp_percent must be between 1 and 100",
        ),
        (
            "balm_chance_denominator",
            json!(0),
            "balm_chance must be a valid fraction",
        ),
        (
            "balm_chance_numerator",
            json!(5),
            "balm_chance must be a valid fraction",
        ),
    ] {
        let mut parts = parts("town_adventure_loop_gallery");
        parts.catalog["scavenging_profiles"][PROFILE][field] = value;
        if field == "balm_chance_numerator" {
            parts.catalog["scavenging_profiles"][PROFILE]["balm_chance_denominator"] = json!(4);
        }
        assert_has(&definition_error(&parts), expected);
    }

    let mut zero_radius = parts("town_adventure_loop_gallery");
    zero_radius.catalog["scavenging_profiles"][PROFILE]["search_radius"] = json!(0);
    assert_has(
        &definition_error(&zero_radius),
        "search_radius must be nonzero when scavenging is enabled",
    );

    let mut balm_dependency = parts("town_adventure_loop_gallery");
    balm_dependency.catalog["scavenging_profiles"][PROFILE]["collects_ground_items"] = json!(false);
    assert_has(
        &definition_error(&balm_dependency),
        "uses_healing_balm requires ground collection and equipping",
    );

    let mut unknown_reference = parts("town_adventure_loop_gallery");
    unknown_reference.actor_definition_by_actor_id_mut("road_scavenger")["scavenging_profile_id"] =
        json!("scavenging/missing");
    assert_has(
        &definition_error(&unknown_reference),
        "scavenging_profile_id references unknown selected profile",
    );

    let mut unknown_field = parts("town_adventure_loop_gallery");
    unknown_field.catalog["scavenging_profiles"][PROFILE]["legacy_radius"] = json!(6);
    assert_has(
        &decode_error(&unknown_field),
        "unknown field `legacy_radius`",
    );
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

#[test]
fn every_tracked_graph_enters_the_same_checked_definition_and_seed_seams() {
    assert_eq!(TRACKED_CASES.len(), 52);
    for case_id in TRACKED_CASES {
        let parts = parts(case_id);
        let seed = parts
            .validated_seed()
            .unwrap_or_else(|error| panic!("{case_id}: {error}"));
        Engine::new(seed, 7).unwrap_or_else(|error| panic!("{case_id}: {error}"));
    }
}

#[test]
fn structure_only_player_and_npc_seed_is_valid_without_monsters_or_ecology() {
    let mut parts = parts("npc_quest_interactions");
    parts
        .actors_mut()
        .as_array_mut()
        .expect("seed actors")
        .retain(|actor| actor["id"] != "watch_sentinel");

    let actors = parts.world_seed["actors"].as_array().expect("seed actors");
    assert_eq!(actors.len(), 3);
    assert_eq!(
        actors
            .iter()
            .map(|actor| actor["id"].as_str().expect("actor id"))
            .collect::<Vec<_>>(),
        ["player", "wayfinder", "watchkeeper"]
    );
    assert!(actors[0]["npc"].is_null());
    assert!(actors[1]["npc"].is_object());
    assert!(actors[2]["npc"].is_object());
    assert!(
        parts.world_seed["ecology_sites"]
            .as_array()
            .expect("ecology sites")
            .is_empty()
    );

    let seed = parts
        .validated_seed()
        .expect("player-plus-NPC seed without monsters or ecology");
    Engine::new(seed, 7).expect("structure-only engine");
}

#[test]
fn four_contract_decoders_are_strict_and_core_documents_reject_scripts() {
    let mut unknown_catalog = parts("first_room");
    unknown_catalog.catalog["unexpected"] = json!(true);
    assert_has(&decode_error(&unknown_catalog), "unknown field");

    let mut missing_catalog = parts("first_room");
    missing_catalog
        .catalog
        .as_object_mut()
        .unwrap()
        .remove("profiles");
    assert_has(&decode_error(&missing_catalog), "missing field");

    let mut catalog_script = parts("first_room");
    catalog_script.catalog["script"] = json!([]);
    assert_has(&decode_error(&catalog_script), "unknown field `script`");

    let mut unknown_template = parts("first_room");
    unknown_template.world_template["unexpected"] = json!(true);
    assert_has(&decode_error(&unknown_template), "unknown field");

    let mut missing_template = parts("first_room");
    missing_template
        .world_template
        .as_object_mut()
        .unwrap()
        .remove("realms");
    assert_has(&decode_error(&missing_template), "missing field");

    let mut template_script = parts("first_room");
    template_script.world_template["script"] = json!([]);
    assert_has(&decode_error(&template_script), "unknown field `script`");

    let mut unknown_seed = parts("first_room");
    unknown_seed.world_seed["unexpected"] = json!(true);
    assert_has(&decode_error(&unknown_seed), "unknown field");

    let mut missing_seed = parts("first_room");
    missing_seed
        .world_seed
        .as_object_mut()
        .unwrap()
        .remove("actors");
    assert_has(&decode_error(&missing_seed), "missing field");

    let mut seed_script = parts("first_room");
    seed_script.world_seed["script"] = json!([]);
    assert_has(&decode_error(&seed_script), "unknown field `script`");
}

#[test]
fn catalog_envelope_profile_and_registry_identity_are_validated_atomically() {
    let mut wrong_schema = parts("first_room");
    wrong_schema.catalog["schema_version"] = json!(1);
    assert_has(
        &definition_error(&wrong_schema),
        "catalog.schema_version must be 6",
    );

    let mut wrong_kind = parts("first_room");
    wrong_kind.catalog["kind"] = json!("scenario");
    assert_has(
        &definition_error(&wrong_kind),
        "catalog.kind must be \"catalog\"",
    );

    let mut empty_id = parts("first_room");
    empty_id.catalog["id"] = json!(" ");
    assert_has(&definition_error(&empty_id), "catalog.id must be non-empty");

    let mut unresolved = parts("first_room");
    unresolved.profile_value_mut()["items"][0] = json!("item/missing");
    assert_has(
        &definition_error(&unresolved),
        "references unknown registry key",
    );

    let mut exact_duplicate = parts("first_room");
    let row = selected_row(&exact_duplicate, "items", 0);
    exact_duplicate.catalog["items"]["item/exact_duplicate"] = row;
    assert_has(&definition_error(&exact_duplicate), "exactly duplicates");

    let mut runtime_collision = parts("first_room");
    let mut row = selected_row(&runtime_collision, "items", 0);
    row["name"] = json!("Different canonical row with same runtime id");
    runtime_collision.push_selected("items", "item/runtime_collision", row);
    assert_has(&definition_error(&runtime_collision), "already selected");

    let mut duplicate_profile = parts("first_room");
    duplicate_profile.catalog["profiles"]["profile/exact_duplicate"] =
        duplicate_profile.profile_value().clone();
    assert_has(&definition_error(&duplicate_profile), "exactly duplicates");

    let mut missing_profile = parts("first_room");
    missing_profile.catalog_profile = "profile/missing".to_string();
    assert_has(
        &definition_error(&missing_profile),
        "does not exist in catalog.profiles",
    );
}

#[test]
fn catalog_selection_preserves_profile_order_and_hides_unselected_rows() {
    let spell_parts = parts("spell_effects");
    let expected = spell_parts.profile_value()["spells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|key| {
            spell_parts.catalog["spells"][key.as_str().unwrap()]["id"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect::<Vec<_>>();
    let (catalog, _, _, _) = spell_parts.decode().unwrap();
    let selected = catalog
        .select(&CatalogProfileKey::new("profile/spell_effects").unwrap())
        .unwrap();
    assert_eq!(
        selected
            .spells
            .iter()
            .map(|spell| spell.id.clone())
            .collect::<Vec<_>>(),
        expected
    );

    let mut unselected = parts("first_room");
    unselected.catalog["items"]["item/unselected_bad"] = json!({
        "id": "",
        "kind": "gear",
        "name": "Unselected",
        "valid_placements": [],
        "consumable": null,
        "economy": {"unit_burden": 0},
        "review_note": null
    });
    unselected
        .definition()
        .expect("unselected definitions are invisible to assembled semantics");
}

#[test]
fn simulation_seed_three_actor_definition_and_ecology_joins_fail_closed() {
    let current = parts("creature_ecology_gallery");
    current
        .validated_seed()
        .expect("current creature ecology seed must validate");

    let mut missing_ecology_sites = parts("first_room");
    missing_ecology_sites
        .world_seed
        .as_object_mut()
        .unwrap()
        .remove("ecology_sites");
    assert_has(
        &decode_error(&missing_ecology_sites),
        "missing field `ecology_sites`",
    );

    let mut seed_override = parts("first_room");
    seed_override.actors_mut()[0]["name"] = json!("Override");
    assert_has(&decode_error(&seed_override), "unknown field `name`");

    let mut unknown_definition = parts("first_room");
    unknown_definition.actors_mut()[1]["actor_definition_id"] = json!("missing");
    assert_has(
        &seed_error(&unknown_definition),
        "references unknown or unselected actor definition",
    );

    let mut unknown_source = current.clone();
    unknown_source.world_seed["ecology_sites"][0]["source"]["spawn_group_id"] = json!("missing");
    assert_has(
        &seed_error(&unknown_source),
        "source references unknown or unselected ecology definition",
    );

    let mut missing_member = current.clone();
    missing_member.world_seed["ecology_sites"][0]["member_locations"]
        .as_object_mut()
        .unwrap()
        .remove("keeper");
    assert_has(
        &seed_error(&missing_member),
        "member_locations keys must exactly equal spawn-group members",
    );

    let mut blocked_member = current.clone();
    blocked_member.world_seed["ecology_sites"][0]["member_locations"]["runner"]["position"] =
        json!({"x": 0, "y": 0});
    assert_has(&seed_error(&blocked_member), "not traversable");

    let mut collision = current;
    collision.actors_mut()[0]["id"] = json!("ecology:gallery_pack:runner:0");
    assert_has(
        &seed_error(&collision),
        "generation-zero actor ID collides with an explicit actor",
    );
}

#[test]
fn recursive_boundary_scanning_covers_catalog_template_seed_keys_and_values() {
    // The terms below come from tests/fixtures/synthetic-terms.txt, the tracked
    // nonsense denylist that .cargo/config.toml configures for cargo-run
    // processes. They prove the REJECTION MECHANISM without the tree carrying a
    // real term. Point TME_BANNED_TERMS_FILE at a different list and this
    // assertion stops holding — by construction, not by defect: a tree that
    // carries no real term cannot write a fixture the real list rejects.
    let mut catalog_value = parts("first_room");
    catalog_value.selected_mut("items", 0)["name"] = json!("zorbelquux blade");
    assert_has(&definition_error(&catalog_value), "banned source term");

    let mut catalog_key = parts("first_room");
    catalog_key.catalog["items"]["item/zorbelquux/private"] =
        selected_row(&catalog_key, "items", 0);
    assert_has(&definition_error(&catalog_key), "banned source term");

    let mut template = parts("first_room");
    template.template_levels_source_mut()["room_0"]["cells"][1][1][0] =
        json!("TME-PLACEHOLDER floor");
    assert_has(&definition_error(&template), "TME-PLACEHOLDER");

    let mut seed = parts("first_room");
    seed.actor_definition_mut(0)["name"] = json!("quendaraff pilgrim");
    assert_has(&seed_error(&seed), "banned source term");
}

#[test]
fn marked_internal_policy_allows_marked_values_but_boundary_pairs_remain_exact() {
    let mut marked = parts("first_room");
    marked.catalog["clean_content"] = json!(false);
    marked.catalog["research_boundary"] = json!({
        "status": "internal_parity_fixture",
        "review_refs": ["Slice EI test"],
        "notes": "TME-PLACEHOLDER internal parity fixture"
    });
    marked.actor_definition_mut(0)["name"] = json!("zorbelquux reference");
    marked
        .validated_seed()
        .expect("marked graph may retain marked source strings");

    let mut mismatched = parts("first_room");
    mismatched.catalog["clean_content"] = json!(false);
    assert_has(&definition_error(&mismatched), "must select exactly");
}

#[test]
fn rules_domains_reject_zero_denominators_invalid_thresholds_and_bad_tuning() {
    let mut warmup = parts("first_room");
    warmup.rules_source_mut()["magic"]["warmup"]["units"] = json!(0);
    assert_has(&definition_error(&warmup), "warmup.units");

    let mut resistance = parts("first_room");
    resistance.rules_source_mut()["magic"]["resistance"]["denominator"] = json!(0);
    assert_has(&definition_error(&resistance), "resistance.denominator");

    let mut hit = parts("first_room");
    hit.rules_source_mut()["combat"]["hit"]["attacker_attack_stat_divisor"] = json!(0);
    assert_has(&definition_error(&hit), "attacker_attack_stat_divisor");

    let mut wounds = parts("first_room");
    wounds.rules_source_mut()["combat"]["wounds"] = json!({
        "near_death_max_percent": 60,
        "badly_wounded_max_percent": 50,
        "wounded_max_percent": 99
    });
    assert_has(&definition_error(&wounds), "near_death_max_percent");

    let mut movement = parts("first_room");
    movement.rules_source_mut()["movement"]["controlled_path_points"] = json!(0);
    assert_has(&definition_error(&movement), "controlled_path_points");

    let mut burden = parts("first_room");
    burden.rules_source_mut()["burden"]["coin_burden_per_gold"] = json!(0);
    assert_has(&definition_error(&burden), "coin_burden_per_gold");

    let mut resources = parts("first_room");
    resources.rules_source_mut()["resources"]["recovery_interval_units"] = json!(0);
    assert_has(&definition_error(&resources), "recovery_interval_units");

    let mut progression = parts("first_room");
    progression.rules_source_mut()["progression"]["level_thresholds"][1]["cumulative_experience"] =
        json!(0);
    assert_has(&definition_error(&progression), "strictly increasing");
}

#[test]
fn armor_definitions_validate_numeric_protection_worn_placement_and_kind_exclusivity() {
    for field in ["block_rating", "encumbrance"] {
        let mut value = parts("fidelity_gallery");
        value.selected_by_runtime_id_mut("items", "leather_armor")["armor"][field] = json!(-1);
        assert_has(
            &definition_error(&value),
            &format!("armor.{field} must be non-negative"),
        );
    }
    for field in ["cutting", "piercing", "crushing"] {
        let mut value = parts("fidelity_gallery");
        value.selected_by_runtime_id_mut("items", "leather_armor")["armor"]["damage_reduction"]
            [field] = json!(-1);
        assert_has(
            &definition_error(&value),
            &format!("damage_reduction.{field} must be non-negative"),
        );
    }

    let mut no_protection = parts("fidelity_gallery");
    let armor = &mut no_protection.selected_by_runtime_id_mut("items", "leather_armor")["armor"];
    armor["block_rating"] = json!(0);
    armor["damage_reduction"] = json!({"cutting": 0, "piercing": 0, "crushing": 0});
    assert_has(
        &definition_error(&no_protection),
        "must provide block_rating or damage reduction",
    );

    let mut not_worn = parts("fidelity_gallery");
    not_worn.selected_by_runtime_id_mut("items", "leather_armor")["valid_placements"] =
        json!(["hand", "sack"]);
    assert_has(
        &definition_error(&not_worn),
        "requires at least one valid worn armor placement",
    );

    for incompatible_kind in ["weapon", "consumable"] {
        let mut value = parts("fidelity_gallery");
        value.selected_by_runtime_id_mut("items", "leather_armor")["kind"] =
            json!(incompatible_kind);
        assert_has(
            &definition_error(&value),
            &format!("armor is invalid for {incompatible_kind} items"),
        );
    }
}

#[test]
fn combat_rules_validate_attack_hit_block_fumble_damage_wound_and_practice_semantics() {
    for (section, field, bad, expected) in [
        ("kick", "maximum_range", json!(1), "maximum_range must be 0"),
        (
            "kick",
            "cooldown_units",
            json!(0),
            "cooldown_units must be positive",
        ),
        (
            "kick",
            "damage_kind",
            json!("cutting"),
            "damage_kind must be crushing",
        ),
        (
            "jumpkick",
            "maximum_range_cap",
            json!(0),
            "maximum_range_cap must be in 1..=3",
        ),
        (
            "jumpkick",
            "skill_levels_per_extra_hex",
            json!(0),
            "skill_levels_per_extra_hex must be positive",
        ),
        (
            "jumpkick",
            "stamina_cost",
            json!(-1),
            "stamina_cost must be non-negative",
        ),
        (
            "jumpkick",
            "cooldown_units",
            json!(0),
            "cooldown_units must be positive",
        ),
        (
            "jumpkick",
            "damage_kind",
            json!("cutting"),
            "damage_kind must be crushing",
        ),
    ] {
        let mut value = parts("first_room");
        value.rules_source_mut()["combat"]["attack_modes"][section][field] = bad;
        assert_has(&definition_error(&value), expected);
    }

    for field in [
        "base_defender_score",
        "attacker_attack_stat_divisor",
        "attacker_skill_level_divisor",
        "defender_defense_stat_divisor",
        "defender_dexterity_divisor",
        "non_character_defender_dexterity",
    ] {
        let mut value = parts("first_room");
        value.rules_source_mut()["combat"]["hit"][field] = json!(0);
        assert_has(
            &definition_error(&value),
            &format!("hit.{field} must be positive"),
        );
    }

    for field in [
        "left_hand_selection_percent",
        "shield_percent_per_point",
        "shield_percent_cap",
        "armor_percent_per_point",
        "armor_percent_cap",
        "strength_penetration_percent_per_add",
        "armor_encumbrance_percent_per_point",
        "combat_add_penetration_percent_per_rating",
    ] {
        let mut value = parts("first_room");
        value.rules_source_mut()["combat"]["block"][field] = json!(0);
        assert_has(
            &definition_error(&value),
            &format!("block.{field} must be an integer in 1..=100"),
        );
    }

    for (per_point, cap) in [
        ("shield_percent_per_point", "shield_percent_cap"),
        ("armor_percent_per_point", "armor_percent_cap"),
    ] {
        let mut value = parts("first_room");
        value.rules_source_mut()["combat"]["block"][per_point] = json!(100);
        value.rules_source_mut()["combat"]["block"][cap] = json!(99);
        assert_has(
            &definition_error(&value),
            &format!("block.{per_point} must not exceed {cap}"),
        );
    }

    for (field, bad, expected) in [
        (
            "base_percent",
            json!(0),
            "base_percent must be an integer in 1..=100",
        ),
        (
            "minimum_percent",
            json!(0),
            "minimum_percent must be in 1..=base_percent",
        ),
        (
            "skill_levels_per_reduction",
            json!(0),
            "skill_levels_per_reduction must be positive",
        ),
    ] {
        let mut value = parts("first_room");
        value.rules_source_mut()["combat"]["fumble"][field] = bad;
        assert_has(&definition_error(&value), expected);
    }

    for (field, expected) in [
        ("minimum_damage", "minimum_damage must be positive"),
        (
            "roll_variation_modulus",
            "roll_variation_modulus must be positive",
        ),
    ] {
        let mut value = parts("first_room");
        value.rules_source_mut()["combat"]["damage"][field] = json!(0);
        assert_has(&definition_error(&value), expected);
    }
    let mut labels = parts("first_room");
    labels.rules_source_mut()["combat"]["damage"]["heavy_label_min_percent"] = json!(20);
    assert_has(
        &definition_error(&labels),
        "damage label thresholds must satisfy",
    );

    let mut wounds = parts("first_room");
    wounds.rules_source_mut()["combat"]["wounds"]["wounded_max_percent"] = json!(100);
    assert_has(&definition_error(&wounds), "wounds must satisfy");

    for field in [
        "practice_raw_points",
        "life_and_death_raw_points",
        "overwhelming_raw_points",
        "fatal_blow_bonus_raw_points",
        "life_and_death_minimum_target_xp_per_attacker_level",
    ] {
        let mut value = parts("first_room");
        value.rules_source_mut()["combat"]["practice"][field] = json!(0);
        assert_has(
            &definition_error(&value),
            &format!("practice.{field} must be positive"),
        );
    }
    for bad in [0, 20] {
        let mut value = parts("first_room");
        value.rules_source_mut()["combat"]["practice"]["life_and_death_required_at_skill_level"] =
            json!(bad);
        assert_has(&definition_error(&value), "must be in 1..=19");
    }
}

#[test]
fn item_definition_contracts_cover_identity_placement_economy_weapons_and_consumables() {
    let mut blank = parts("first_room");
    blank.selected_mut("items", 0)["id"] = json!(" ");
    assert_has(&definition_error(&blank), ".id must be non-empty");

    let mut duplicate_placement = parts("first_room");
    duplicate_placement.selected_mut("items", 0)["valid_placements"] = json!(["hand", "hand"]);
    assert_has(&definition_error(&duplicate_placement), "valid_placements");

    let mut missing_economy = parts("first_room");
    missing_economy
        .selected_mut("items", 0)
        .as_object_mut()
        .unwrap()
        .remove("economy");
    assert_has(&decode_error(&missing_economy), "missing field `economy`");

    let mut missing_weapon = parts("first_room");
    missing_weapon.selected_mut("items", 0)["weapon"] = Value::Null;
    assert_has(
        &definition_error(&missing_weapon),
        "must be present for weapons",
    );

    let mut bad_range = parts("reach_attack");
    bad_range.selected_mut("items", 0)["weapon"]["attack_modes"][0]["minimum_range"] = json!(3);
    bad_range.selected_mut("items", 0)["weapon"]["attack_modes"][0]["maximum_range"] = json!(1);
    assert_has(&definition_error(&bad_range), "minimum_range");

    let mut bad_heal = parts("balm_cache");
    let consumable_index = (0..bad_heal.selected_len("items"))
        .find(|index| selected_row(&bad_heal, "items", *index)["consumable"].is_object())
        .expect("balm cache consumable");
    bad_heal.selected_mut("items", consumable_index)["consumable"]["heal_per_round"] = json!(0);
    assert_has(&definition_error(&bad_heal), "heal_per_round");
}

#[test]
fn weapon_definitions_validate_modes_ranges_handedness_nocking_and_numeric_bounds() {
    for (field, bad, expected) in [
        (
            "skill_track_id",
            json!(" "),
            "skill_track_id must be non-empty",
        ),
        (
            "cooldown_units",
            json!(0),
            "cooldown_units must be positive",
        ),
        (
            "combat_add_rating",
            json!(-1),
            "combat_add_rating must be non-negative",
        ),
        ("block_value", json!(-1), "block_value must be non-negative"),
    ] {
        let mut value = parts("thrown_attack");
        value.selected_by_runtime_id_mut("items", "oak_javelin")["weapon"][field] = bad;
        assert_has(&definition_error(&value), expected);
    }

    let mut no_modes = parts("thrown_attack");
    no_modes.selected_by_runtime_id_mut("items", "oak_javelin")["weapon"]["attack_modes"] =
        json!([]);
    assert_has(
        &definition_error(&no_modes),
        "attack_modes must not be empty",
    );

    let mut invalid_mode = parts("thrown_attack");
    invalid_mode.selected_by_runtime_id_mut("items", "oak_javelin")["weapon"]["attack_modes"][0]
        ["mode"] = json!("kick");
    assert_has(
        &definition_error(&invalid_mode),
        "mode must be fight, poke, shoot, or throw",
    );

    let mut duplicate = parts("thrown_attack");
    duplicate.selected_by_runtime_id_mut("items", "oak_javelin")["weapon"]["attack_modes"][1]["mode"] =
        json!("fight");
    assert_has(&definition_error(&duplicate), "contains duplicate fight");

    for (case_id, item_id, mode_index, mode, bad_range, expected) in [
        (
            "thrown_attack",
            "oak_javelin",
            0,
            "fight",
            1,
            "must be 0 for fight",
        ),
        (
            "thrown_attack",
            "oak_javelin",
            0,
            "poke",
            2,
            "must be 0 or 1 for poke",
        ),
        (
            "ranged_attack",
            "elm_bow",
            0,
            "shoot",
            0,
            "must be positive for shoot",
        ),
        (
            "thrown_attack",
            "oak_javelin",
            1,
            "throw",
            0,
            "must be positive for throw",
        ),
    ] {
        let mut value = parts(case_id);
        let attack_mode = &mut value.selected_by_runtime_id_mut("items", item_id)["weapon"]["attack_modes"]
            [mode_index];
        attack_mode["mode"] = json!(mode);
        attack_mode["maximum_range"] = json!(bad_range);
        assert_has(&definition_error(&value), expected);
    }

    let mut missing_default = parts("thrown_attack");
    missing_default.selected_by_runtime_id_mut("items", "oak_javelin")["weapon"]["default_attack_mode"] =
        json!("poke");
    assert_has(
        &definition_error(&missing_default),
        "default_attack_mode must name an authored attack mode",
    );

    let mut bow_extra_mode = parts("ranged_attack");
    bow_extra_mode.selected_by_runtime_id_mut("items", "elm_bow")["weapon"]["attack_modes"]
        .as_array_mut()
        .expect("bow modes")
        .push(json!({"mode": "fight", "maximum_range": 0, "damage_kind": "crushing"}));
    assert_has(
        &definition_error(&bow_extra_mode),
        "must contain exactly shoot as the bow default",
    );

    let mut bow_without_nocking = parts("ranged_attack");
    bow_without_nocking.selected_by_runtime_id_mut("items", "elm_bow")["weapon"]
        .as_object_mut()
        .expect("bow")
        .remove("nocking");
    assert_has(
        &definition_error(&bow_without_nocking),
        "nocking must be present for bows",
    );

    let mut non_bow_nocking = parts("thrown_attack");
    non_bow_nocking.selected_by_runtime_id_mut("items", "oak_javelin")["weapon"]["nocking"] =
        json!({"unloads_on_movement": true});
    assert_has(
        &definition_error(&non_bow_nocking),
        "nocking is only valid for bows",
    );

    let mut non_bow_shoot = parts("thrown_attack");
    non_bow_shoot.selected_by_runtime_id_mut("items", "oak_javelin")["weapon"]["attack_modes"][1]
        ["mode"] = json!("shoot");
    assert_has(
        &definition_error(&non_bow_shoot),
        "shoot mode is only valid for bows",
    );
}

#[test]
fn progression_definitions_validate_threshold_profiles_growth_outcomes_and_seed_xp() {
    let mut too_few = parts("xp_progression");
    too_few.rules_source_mut()["progression"]["level_thresholds"] = json!([{
        "level": 1, "cumulative_experience": 0
    }]);
    assert_has(
        &definition_error(&too_few),
        "must contain at least two rows",
    );

    for (row, field, bad, expected) in [
        (0, "level", json!(0), "level must be positive"),
        (1, "level", json!(3), "level must be consecutive"),
        (
            0,
            "cumulative_experience",
            json!(-1),
            "must be non-negative",
        ),
        (
            1,
            "cumulative_experience",
            json!(0),
            "must be strictly increasing",
        ),
    ] {
        let mut value = parts("xp_progression");
        value.rules_source_mut()["progression"]["level_thresholds"][row][field] = bad;
        assert_has(&definition_error(&value), expected);
    }

    let mut blank_class = parts("xp_progression");
    blank_class.rules_source_mut()["progression"]["growth_profiles"][0]["class_id"] = json!(" ");
    assert_has(
        &definition_error(&blank_class),
        "class_id must be non-empty",
    );

    let mut duplicate_class = parts("knight_promotion");
    duplicate_class.rules_source_mut()["progression"]["growth_profiles"][1]["class_id"] =
        json!("fighter");
    assert_has(
        &definition_error(&duplicate_class),
        "class_id must be unique",
    );

    let mut missing_required_class = parts("xp_progression");
    missing_required_class.rules_source_mut()["progression"]["growth_profiles"][0]["class_id"] =
        json!("thief");
    assert_has(
        &seed_error(&missing_required_class),
        "must contain class_id \"fighter\"",
    );

    for (field, bad_attribute, expected) in [
        ("hit_points", "strength", "attribute constitution"),
        ("stamina_points", "constitution", "attribute strength"),
    ] {
        let mut value = parts("xp_progression");
        value.rules_source_mut()["progression"]["growth_profiles"][0][field]["attribute"] =
            json!(bad_attribute);
        assert_has(&definition_error(&value), expected);
    }

    let mut non_fixed_magic = parts("magic_profession_gallery");
    let hit_rule =
        non_fixed_magic.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]
            .clone();
    non_fixed_magic.rules_source_mut()["progression"]["growth_profiles"][0]["magic_points"] =
        hit_rule;
    assert_has(
        &definition_error(&non_fixed_magic),
        "magic_points must use kind fixed",
    );

    let mut empty_bands = parts("xp_progression");
    empty_bands.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]["bands"] =
        json!([]);
    assert_has(&definition_error(&empty_bands), "bands must be non-empty");

    for (band, bad, expected) in [
        (0, json!(1), "minimum_attribute must be zero"),
        (0, json!(-1), "minimum_attribute must be non-negative"),
        (1, json!(0), "minimum_attribute must be strictly increasing"),
    ] {
        let mut value = parts("xp_progression");
        value.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]["bands"]
            [band]["minimum_attribute"] = bad;
        assert_has(&definition_error(&value), expected);
    }

    let mut empty_outcomes = parts("xp_progression");
    empty_outcomes.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]["bands"]
        [0]["outcomes"] = json!([]);
    assert_has(
        &definition_error(&empty_outcomes),
        "outcomes must be non-empty",
    );

    for (field, bad, expected) in [
        ("amount", json!(0), "amount must be positive"),
        ("weight", json!(0), "weight must be positive"),
    ] {
        let mut value = parts("xp_progression");
        value.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]["bands"][0]["outcomes"]
            [0][field] = bad;
        assert_has(&definition_error(&value), expected);
    }

    let mut duplicate_outcome = parts("xp_progression");
    duplicate_outcome.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]["bands"]
        [0]["outcomes"][1]["amount"] = json!(8);
    assert_has(
        &definition_error(&duplicate_outcome),
        "amount must be unique",
    );

    let mut overflow = parts("xp_progression");
    let outcomes = &mut overflow.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]
        ["bands"][0]["outcomes"];
    outcomes[0]["weight"] = json!(u32::MAX);
    outcomes[1]["weight"] = json!(1);
    assert_has(
        &definition_error(&overflow),
        "weights must not overflow u32",
    );

    for (row, field, bad, expected) in [
        (0, "level", json!(0), "level must be positive"),
        (0, "level", json!(11), "within the authored threshold range"),
        (1, "level", json!(3), "strictly ascending in authored order"),
        (
            0,
            "strength_adds",
            json!(-1),
            "additions must be non-negative",
        ),
    ] {
        let mut value = parts("xp_progression");
        value.rules_source_mut()["progression"]["growth_profiles"][0]["physical_attribute_adds_by_level"]
            [row][field] = bad;
        assert_has(&definition_error(&value), expected);
    }

    let mut no_add = parts("xp_progression");
    let row = &mut no_add.rules_source_mut()["progression"]["growth_profiles"][0]["physical_attribute_adds_by_level"]
        [0];
    row["strength_adds"] = json!(0);
    row["dexterity_adds"] = json!(0);
    assert_has(
        &definition_error(&no_add),
        "must contain at least one positive addition",
    );

    let mut out_of_range_level = parts("xp_progression");
    out_of_range_level.actors_mut()[0]["character"]["progression"]["level"] = json!(11);
    assert_has(
        &seed_error(&out_of_range_level),
        "progression.level must be within authored threshold range",
    );

    let mut ahead_of_xp = parts("xp_progression");
    ahead_of_xp.actors_mut()[0]["character"]["progression"]["level"] = json!(2);
    assert_has(
        &seed_error(&ahead_of_xp),
        "progression.level must not exceed the XP-earned level",
    );
}

#[test]
fn spell_definitions_validate_typed_effect_target_casting_and_cross_references() {
    let mut blank = parts("spell_effects");
    blank.selected_mut("spells", 0)["id"] = json!(" ");
    assert_has(&definition_error(&blank), "spells[0].id must be non-empty");

    let mut bad_family = parts("spell_effects");
    bad_family.selected_mut("spells", 0)["effect"]["family"] = json!("mystery");
    assert_has(&decode_error(&bad_family), "unknown variant");

    let mut bad_damage = parts("spell_effects");
    bad_damage.selected_mut("spells", 0)["effect"]["potency"] = json!(0);
    assert_has(&definition_error(&bad_damage), "potency");

    let mut bad_target = parts("spell_effects");
    bad_target.selected_mut("spells", 2)["target"]["range"] = json!(1);
    assert_has(
        &definition_error(&bad_target),
        "target.range is invalid for self target",
    );

    let mut missing_social = parts("spell_effects");
    missing_social
        .selected_mut("spells", 0)
        .as_object_mut()
        .unwrap()
        .remove("social");
    assert_has(&decode_error(&missing_social), "missing field `social`");

    let mut bad_casting = parts("spell_effects");
    bad_casting.selected_mut("spells", 0)["casting"]["method"] = json!("ritual");
    assert_has(&decode_error(&bad_casting), "unknown variant");
}

#[test]
fn spell_casting_and_typed_metadata_profiles_accept_current_operational_shapes() {
    for case_id in [
        "spell_readiness",
        "spell_effects",
        "area_path_terrain_spells",
        "utility_door_secret_item_spells",
        "magic_profession_gallery",
    ] {
        parts(case_id)
            .definition()
            .unwrap_or_else(|error| panic!("{case_id}: {error}"));
    }

    let mut invalid_method = parts("spell_readiness");
    invalid_method.selected_by_runtime_id_mut("spells", "charged_spark")["casting"]["method"] =
        json!("slow");
    assert_has(&decode_error(&invalid_method), "unknown variant `slow`");

    let mut invalid_class = parts("spell_readiness");
    invalid_class.selected_by_runtime_id_mut("spells", "charged_spark")["casting"]["cast_class"] =
        json!("anywhere");
    assert_has(&decode_error(&invalid_class), "unknown variant `anywhere`");

    let mut missing_casting = parts("spell_readiness");
    missing_casting
        .selected_by_runtime_id_mut("spells", "charged_spark")
        .as_object_mut()
        .unwrap()
        .remove("casting");
    assert_has(
        &definition_error(&missing_casting),
        "casting is required for operational spells",
    );

    for removed in [
        "readiness",
        "target_type",
        "warmup_rounds",
        "interrupt_on_non_release_action",
        "interrupt_on_damage",
    ] {
        let mut obsolete = parts("spell_readiness");
        obsolete.selected_by_runtime_id_mut("spells", "charged_spark")[removed] = json!(true);
        assert_has(
            &decode_error(&obsolete),
            &format!("unknown field `{removed}`"),
        );
    }

    for cast_class in ["path", "path_or_character"] {
        let mut non_damage_path = parts("spell_effects");
        non_damage_path.selected_by_runtime_id_mut("spells", "mend")["casting"]["cast_class"] =
            json!(cast_class);
        assert_has(
            &definition_error(&non_damage_path),
            "cast_class may be path or path_or_character only for direct_damage",
        );
    }

    let mut direct_damage_path = parts("spell_effects");
    direct_damage_path.selected_by_runtime_id_mut("spells", "spark")["casting"]["cast_class"] =
        json!("path");
    direct_damage_path
        .definition()
        .expect("direct-damage operational path casting must remain valid");
}

#[test]
fn spell_effect_family_metadata_rejects_invalid_br_bs_bu_and_dw_semantics() {
    for case_id in [
        "spell_effects",
        "control_poison_protection",
        "utility_door_secret_item_spells",
    ] {
        parts(case_id)
            .definition()
            .unwrap_or_else(|error| panic!("{case_id}: {error}"));
    }

    for (spell_id, expected) in [
        ("spark", "positive for direct_damage spells"),
        ("mend", "positive for healing spells"),
    ] {
        let mut invalid = parts("spell_effects");
        invalid.selected_by_runtime_id_mut("spells", spell_id)["effect"]["potency"] = json!(0);
        assert_has(&definition_error(&invalid), expected);
    }

    let mut bad_stacking = parts("spell_effects");
    bad_stacking.selected_by_runtime_id_mut("spells", "strength")["effect"]["stacking"] =
        json!("stack_forever");
    assert_has(
        &definition_error(&bad_stacking),
        "effect.stacking must be one of",
    );

    let mut bad_poison = parts("control_poison_protection");
    let poison = &mut bad_poison.selected_by_runtime_id_mut("spells", "venom")["effect"];
    poison["potency"] = json!(0);
    poison["start_delay_rounds"] = json!(-1);
    let error = definition_error(&bad_poison);
    assert_has(&error, "positive for poison spells");
    assert_has(&error, "start_delay_rounds must be non-negative");

    let mut empty_boosts = parts("control_poison_protection");
    empty_boosts.selected_by_runtime_id_mut("spells", "toxin_ward")["effect"]["resistance"]["boosts"] =
        json!([]);
    assert_has(
        &definition_error(&empty_boosts),
        "resistance.boosts must be non-empty",
    );

    let mut missing_protection_resistance = parts("control_poison_protection");
    missing_protection_resistance.selected_by_runtime_id_mut("spells", "toxin_ward")["effect"]
        .as_object_mut()
        .unwrap()
        .remove("resistance");
    assert_has(
        &definition_error(&missing_protection_resistance),
        "resistance must use the boost role for protection",
    );

    let mut bad_door_action = parts("utility_door_secret_item_spells");
    bad_door_action.selected_by_runtime_id_mut("spells", "open_gate")["effect"]["door_control"]["action"] =
        json!("unlock");
    assert_has(
        &definition_error(&bad_door_action),
        "door_control.action must be one of",
    );

    let mut bad_door_target = parts("utility_door_secret_item_spells");
    bad_door_target.selected_by_runtime_id_mut("spells", "close_gate")["target"]["kind"] =
        json!("actor");
    assert_has(
        &definition_error(&bad_door_target),
        "target.kind must be coordinate or door for door_control close spells",
    );

    let mut bad_item_action = parts("utility_door_secret_item_spells");
    bad_item_action.selected_by_runtime_id_mut("spells", "identify")["effect"]["item_utility"]["action"] =
        json!("polish");
    assert_has(
        &definition_error(&bad_item_action),
        "item_utility.action must be one of",
    );

    let mut bad_locate = parts("utility_door_secret_item_spells");
    bad_locate.selected_by_runtime_id_mut("spells", "find_veiled_charm")["effect"]["locate"]["subject"] =
        json!("treasure");
    assert_has(
        &definition_error(&bad_locate),
        "locate.subject must be one of actor, item, level",
    );

    let mut bad_portal = parts("utility_door_secret_item_spells");
    let portal = bad_portal.selected_by_runtime_id_mut("spells", "blue_gate");
    portal["target"]["kind"] = json!("none");
    portal["effect"]["portal"]["target"] = json!({
        "kind": "position",
        "location": {
            "realm": "missing",
            "level": "missing",
            "position": {"x": 1, "y": 1}
        }
    });
    let error = definition_error(&bad_portal);
    assert_has(&error, "target.kind must be coordinate for portal spells");
    assert_has(
        &error,
        "target references missing realm/level missing/missing",
    );

    let mut bad_scry = parts("utility_door_secret_item_spells");
    let scry = bad_scry.selected_by_runtime_id_mut("spells", "workroom_glimpse");
    scry["target"]["kind"] = json!("coordinate");
    scry["effect"]["scry"]["scope"] = json!("map");
    let error = definition_error(&bad_scry);
    assert_has(&error, "target.kind must be none for scry spells");
    assert_has(&error, "scry.scope must be one of level, coordinate");

    let mut missing_incoming = parts("spell_effects");
    missing_incoming.selected_by_runtime_id_mut("spells", "spark")["effect"]
        .as_object_mut()
        .unwrap()
        .remove("resistance");
    assert_has(
        &definition_error(&missing_incoming),
        "resistance must use the incoming role for direct_damage",
    );

    let mut bad_control_mode = parts("control_poison_protection");
    bad_control_mode.selected_by_runtime_id_mut("spells", "self_hold")["effect"]["resistance"]["mitigation"] =
        json!({"mode": "half_damage", "rounding": "down", "minimum_damage": 1});
    assert_has(
        &definition_error(&bad_control_mode),
        "resistance.mitigation must be negate for control_status",
    );

    let mut duplicate_boost = parts("control_poison_protection");
    let boost =
        duplicate_boost.selected_by_runtime_id_mut("spells", "toxin_ward")["effect"]["resistance"]
            ["boosts"][0]
            .clone();
    duplicate_boost.selected_by_runtime_id_mut("spells", "toxin_ward")["effect"]
        ["resistance"]["boosts"]
        .as_array_mut()
        .unwrap()
        .push(boost);
    assert_has(
        &definition_error(&duplicate_boost),
        "resistance.boosts tags must be unique",
    );

    let mut obsolete_resistance_tags = parts("spell_effects");
    obsolete_resistance_tags.selected_by_runtime_id_mut("spells", "spark")["effect"]["resistance_tags"] =
        json!(["arcane"]);
    assert_has(
        &decode_error(&obsolete_resistance_tags),
        "unknown field `resistance_tags`",
    );

    let mut malformed_half_damage = parts("spell_effects");
    malformed_half_damage.selected_by_runtime_id_mut("spells", "spark")["effect"]["resistance"]["mitigation"] =
        json!({"mode": "half_damage", "rounding": "down"});
    assert_has(
        &decode_error(&malformed_half_damage),
        "missing field `minimum_damage`",
    );
}

#[test]
fn magic_and_resistance_rules_validate_evidence_arithmetic_and_actor_summon_bounds() {
    parts("spell_effects")
        .definition()
        .expect("current magic rules must validate");

    let mut missing_magic = parts("spell_effects");
    missing_magic
        .rules_source_mut()
        .as_object_mut()
        .unwrap()
        .remove("magic");
    assert_has(&decode_error(&missing_magic), "missing field `magic`");

    let mut zero_warmup = parts("spell_effects");
    zero_warmup.rules_source_mut()["magic"]["warmup"]["units"] = json!(0);
    assert_has(
        &definition_error(&zero_warmup),
        "rules.magic.warmup.units must be positive",
    );

    let mut bad_interruption = parts("spell_effects");
    bad_interruption.rules_source_mut()["magic"]["damage_interruption"]["numerator"] = json!(5);
    bad_interruption.rules_source_mut()["magic"]["damage_interruption"]["denominator"] = json!(5);
    assert_has(
        &definition_error(&bad_interruption),
        "numerator must be less than denominator",
    );

    let mut bad_comparison = parts("spell_effects");
    bad_comparison.rules_source_mut()["magic"]["damage_interruption"]["comparison"] =
        json!("at_or_above");
    assert_has(
        &decode_error(&bad_comparison),
        "unknown variant `at_or_above`",
    );

    let mut target_release = parts("spell_effects");
    target_release.rules_source_mut()["magic"]["warmup"]["evidence_state"] =
        json!("target_release");
    assert_has(
        &definition_error(&target_release),
        "target_release is allowed only in a marked internal parity fixture",
    );

    let mut marked_target_release = parts("spell_effects");
    marked_target_release.catalog["clean_content"] = json!(false);
    marked_target_release.catalog["research_boundary"] = json!({
        "status": "internal_parity_fixture",
        "review_refs": ["Slice EI test"],
        "notes": "TME-PLACEHOLDER focused target-release magic rules proof"
    });
    marked_target_release.rules_source_mut()["magic"]["warmup"]["evidence_state"] =
        json!("target_release");
    marked_target_release.rules_source_mut()["magic"]["damage_interruption"]["evidence_state"] =
        json!("target_release");
    marked_target_release.rules_source_mut()["magic"]["resistance"]["denominator_evidence_state"] =
        json!("target_release");
    marked_target_release
        .definition()
        .expect("target-release evidence is valid only under the exact marked-internal boundary");

    for field in [
        "casting_practice",
        "thaum_above_skill",
        "kill_experience",
        "mp_recovery",
    ] {
        let mut missing = parts("spell_effects");
        missing.rules_source_mut()["magic"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert_has(&decode_error(&missing), &format!("missing field `{field}`"));
    }

    let mut practice_overflow = parts("spell_effects");
    practice_overflow.rules_source_mut()["magic"]["casting_practice"]["raw_points_per_mp"] =
        json!(u64::MAX);
    assert_has(
        &definition_error(&practice_overflow),
        "casting_practice arithmetic exceeds supported range",
    );

    let mut thaum_overflow = parts("spell_effects");
    thaum_overflow.rules_source_mut()["magic"]["thaum_above_skill"]["penalty_per_missing_level"] =
        json!(u32::MAX);
    assert_has(
        &definition_error(&thaum_overflow),
        "thaum_above_skill maximum gap arithmetic exceeds supported range",
    );

    let mut bad_threshold = parts("spell_effects");
    bad_threshold.rules_source_mut()["magic"]["thaum_above_skill"]["minimum_success_threshold"] =
        json!(21);
    assert_has(
        &definition_error(&bad_threshold),
        "minimum_success_threshold must be in 1..=roll_denominator",
    );

    let mut unreduced_reward = parts("spell_effects");
    unreduced_reward.rules_source_mut()["magic"]["kill_experience"]["directed"] =
        json!({"numerator": 2, "denominator": 2});
    assert_has(
        &definition_error(&unreduced_reward),
        "kill_experience.directed must be reduced",
    );

    let mut zero_resistance = parts("spell_effects");
    zero_resistance.rules_source_mut()["magic"]["resistance"]["denominator"] = json!(0);
    assert_has(
        &definition_error(&zero_resistance),
        "rules.magic.resistance.denominator must be positive",
    );

    let mut bad_save_comparison = parts("spell_effects");
    bad_save_comparison.rules_source_mut()["magic"]["resistance"]["success_comparison"] =
        json!("roll_above");
    assert_has(
        &decode_error(&bad_save_comparison),
        "unknown variant `roll_above`",
    );

    let mut unknown_rule = parts("spell_effects");
    unknown_rule.rules_source_mut()["magic"]["resistance"]["unexpected"] = json!(true);
    assert_has(&decode_error(&unknown_rule), "unknown field `unexpected`");

    let mut actor_bound = parts("spell_effects");
    actor_bound.actor_definition_mut(0)["magic_resistance"]["natural_save_twentieths"] = json!(21);
    assert_has(
        &seed_error(&actor_bound),
        "natural_save_twentieths must not exceed rules.magic.resistance.denominator",
    );

    let mut missing_actor_resistance = parts("spell_effects");
    missing_actor_resistance
        .actor_definition_mut(0)
        .as_object_mut()
        .unwrap()
        .remove("magic_resistance");
    assert_has(
        &decode_error(&missing_actor_resistance),
        "missing field `magic_resistance`",
    );

    let mut summon_bound = parts("summons_created_creature_lifecycle");
    summon_bound.summon_actor_definition_mut(0)["magic_resistance"]["natural_save_twentieths"] =
        json!(21);
    assert_has(
        &definition_error(&summon_bound),
        "natural_save_twentieths must not exceed rules.magic.resistance.denominator",
    );
}

#[test]
fn typed_spell_target_effect_acquisition_and_item_location_shapes_are_strict() {
    for case_id in [
        "magic_profession_gallery",
        "remaining_spell_effect_families",
        "utility_door_secret_item_spells",
    ] {
        parts(case_id)
            .definition()
            .unwrap_or_else(|error| panic!("{case_id}: {error}"));
    }

    let mut bad_area = parts("magic_profession_gallery");
    bad_area.selected_by_runtime_id_mut("spells", "web_field")["target"]["area"]
        .as_object_mut()
        .unwrap()
        .remove("radius");
    assert_has(
        &definition_error(&bad_area),
        "target.area.radius must be present for area targets",
    );

    let mut bad_control = parts("magic_profession_gallery");
    let effect = bad_control.selected_by_runtime_id_mut("spells", "self_hold")["effect"]
        .as_object_mut()
        .unwrap();
    effect.remove("status_kind");
    effect.remove("duration");
    let error = definition_error(&bad_control);
    assert_has(
        &error,
        "status_kind must be present for control_status spells",
    );
    assert_has(&error, "duration must be present for control_status spells");

    let mut bad_item_target = parts("utility_door_secret_item_spells");
    bad_item_target.selected_by_runtime_id_mut("spells", "identify")["target"]["kind"] =
        json!("actor");
    assert_has(
        &definition_error(&bad_item_target),
        "target.kind must be item for item_identify spells",
    );

    let mut bad_summon = parts("remaining_spell_effect_families");
    bad_summon.selected_by_runtime_id_mut("spells", "call_demon")["effect"]
        .as_object_mut()
        .unwrap()
        .remove("summon_actor_id");
    assert_has(
        &definition_error(&bad_summon),
        "summon_actor_id must be present for summon spells",
    );

    let mut empty_terrain = parts("magic_profession_gallery");
    empty_terrain.selected_by_runtime_id_mut("spells", "web_field")["effect"]["terrain_overlay"] =
        json!({});
    assert_has(
        &definition_error(&empty_terrain),
        "terrain_overlay must declare passability, sight, hazard, or move_cost",
    );

    let mut malformed_effect = parts("spell_effects");
    malformed_effect.selected_by_runtime_id_mut("spells", "spark")["effect"]["status_kind"] =
        json!(7);
    assert_has(&decode_error(&malformed_effect), "invalid type");

    let mut malformed_duration = parts("spell_effects");
    malformed_duration.selected_by_runtime_id_mut("spells", "strength")["effect"]["duration"]["extra"] =
        json!(true);
    assert_has(&decode_error(&malformed_duration), "unknown field `extra`");

    let mut malformed_target = parts("spell_effects");
    malformed_target.selected_by_runtime_id_mut("spells", "spark")["target"]["range"] =
        json!("three");
    assert_has(&decode_error(&malformed_target), "invalid type");

    let mut malformed_area = parts("magic_profession_gallery");
    malformed_area.selected_by_runtime_id_mut("spells", "web_field")["target"]["area"]["extra"] =
        json!(true);
    assert_has(&decode_error(&malformed_area), "unknown field `extra`");

    let mut malformed_terrain = parts("magic_profession_gallery");
    malformed_terrain.selected_by_runtime_id_mut("spells", "web_field")["effect"]["terrain_overlay"]
        ["extra"] = json!(true);
    assert_has(&decode_error(&malformed_terrain), "unknown field `extra`");

    let mut malformed_acquisition = parts("magic_profession_gallery");
    malformed_acquisition.selected_by_runtime_id_mut("spells", "shadow_sting")["acquisition"]["item_definition_ids"] =
        json!(["reagent"]);
    assert_has(
        &decode_error(&malformed_acquisition),
        "unknown field `item_definition_ids`",
    );

    for obsolete_location in ["inventory", "equipment"] {
        let mut obsolete = parts("utility_door_secret_item_spells");
        obsolete.selected_by_runtime_id_mut("spells", "identify")["target"]["item_location"] =
            json!(obsolete_location);
        assert_has(&decode_error(&obsolete), "unknown variant");
    }

    let mut misplaced_location = parts("spell_effects");
    misplaced_location.selected_by_runtime_id_mut("spells", "spark")["target"]["item_location"] =
        json!("sack");
    assert_has(
        &definition_error(&misplaced_location),
        "target.item_location is only valid for item targets",
    );
}

#[test]
fn knight_spell_contract_is_three_mp_direct_unpurchased_and_untrained() {
    let current = parts("knight_promotion");
    current
        .definition()
        .expect("current Knight spell profile must validate");
    assert_eq!(current.selected_len("spells"), 5);
    for index in 0..current.selected_len("spells") {
        let spell = selected_row(&current, "spells", index);
        assert_eq!(spell["lane"], "knight_magic");
        assert_eq!(spell["mp_cost"], 3);
        assert!(spell.get("skill_requirement").is_none());
        assert!(spell.get("acquisition").is_none());
        assert_eq!(spell["casting"]["method"], "direct");
    }

    let mut trained = parts("knight_promotion");
    trained.selected_mut("spells", 0)["skill_requirement"] = json!(1);
    assert_has(
        &definition_error(&trained),
        "skill_requirement must be absent for knight_magic",
    );

    let mut cheap = parts("knight_promotion");
    cheap.selected_mut("spells", 0)["mp_cost"] = json!(2);
    assert_has(
        &definition_error(&cheap),
        "mp_cost must be 3 for knight_magic",
    );

    let mut purchased = parts("knight_promotion");
    purchased.selected_mut("spells", 0)["acquisition"] = json!({"gold_cost": 1});
    assert_has(
        &definition_error(&purchased),
        "acquisition must be absent for knight_magic",
    );

    let mut warmed = parts("knight_promotion");
    warmed.selected_mut("spells", 0)["casting"]["method"] = json!("warm_then_cast");
    assert_has(
        &definition_error(&warmed),
        "casting.method must be direct for knight_magic",
    );
}

#[test]
fn profession_action_payload_shapes_are_exact_typed_and_mutually_exclusive() {
    for case_id in ["profession_specific_actions", "martial_hand_block_actions"] {
        parts(case_id)
            .definition()
            .unwrap_or_else(|error| panic!("{case_id}: {error}"));
    }

    let mut missing_block = parts("martial_hand_block_actions");
    missing_block
        .selected_mut("profession_actions", 0)
        .as_object_mut()
        .unwrap()
        .remove("martial_hand_block");
    assert_has(
        &definition_error(&missing_block),
        "martial_hand_block must be present for martial_hand_block actions",
    );

    let mut boolean_sibling = parts("martial_hand_block_actions");
    boolean_sibling.selected_mut("profession_actions", 0)["hide"] = json!(true);
    assert_has(
        &definition_error(&boolean_sibling),
        "hide is only valid for hide actions",
    );

    let mut invalid_block = parts("martial_hand_block_actions");
    let block = &mut invalid_block.selected_mut("profession_actions", 0)["martial_hand_block"];
    block["min_hand_level"] = json!(-1);
    block["level_divisor"] = json!(0);
    block["max_chance_percent"] = json!(101);
    let error = definition_error(&invalid_block);
    assert_has(&error, "min_hand_level must be between 0 and 19");
    assert_has(&error, "level_divisor must be positive");
    assert_has(&error, "max_chance_percent must be between 1 and 100");

    let mut invalid_hide = parts("profession_specific_actions");
    let hide = &mut invalid_hide.selected_mut("profession_actions", 0)["hide"];
    hide["effect_id"] = json!("");
    hide["duration_rounds"] = json!(0);
    hide["requires_cover_or_darkness"] = json!("yes");
    hide["break_on"] = json!(["dance"]);
    hide["disallow_two_handed"] = json!(null);
    let error = definition_error(&invalid_hide);
    assert_has(&error, "hide.effect_id must be non-empty");
    assert_has(&error, "hide.duration_rounds must be positive");
    assert_has(&error, "hide.requires_cover_or_darkness must be a boolean");
    assert_has(&error, "hide.break_on[0] must be one of");
    assert_has(&error, "hide.disallow_two_handed must be a boolean");

    let mut missing_hide_fields = parts("profession_specific_actions");
    missing_hide_fields.selected_mut("profession_actions", 0)["hide"] = json!({});
    let error = definition_error(&missing_hide_fields);
    for expected in [
        "hide.effect_id must be non-empty",
        "hide.duration_rounds must be positive",
        "hide.requires_cover_or_darkness must be a boolean",
        "hide.break_on must be a list",
        "hide.disallow_two_handed must be a boolean",
    ] {
        assert_has(&error, expected);
    }

    let mut null_hide_sibling = parts("martial_hand_block_actions");
    null_hide_sibling.selected_mut("profession_actions", 0)["hide"] = Value::Null;
    assert_has(
        &definition_error(&null_hide_sibling),
        "hide is only valid for hide actions",
    );

    let mut null_block_sibling = parts("profession_specific_actions");
    null_block_sibling.selected_mut("profession_actions", 0)["martial_hand_block"] = Value::Null;
    assert_has(
        &definition_error(&null_block_sibling),
        "martial_hand_block is only valid for martial_hand_block actions",
    );
}

#[test]
fn summon_template_death_items_ownership_and_spell_metadata_are_exact() {
    let current = parts("summons_created_creature_lifecycle");
    current
        .definition()
        .expect("current summon template metadata must validate");

    let mut with_item = parts("summons_created_creature_lifecycle");
    select_registry_row_by_runtime_id(&mut with_item, "items", "healing_balm");
    let template = with_item.selected_mut("summon_templates", 0);
    template["item_instances"] = json!({
        "focus": {
            "definition_id": "healing_balm",
            "binding": {"state": "unrestricted"}
        }
    });
    template["carried"]["items"] = json!([{"item_instance_id": "focus", "position": "right_hand"}]);
    with_item
        .definition()
        .expect("summon-owned item with one carried location must validate");

    let mut missing_death = parts("summons_created_creature_lifecycle");
    missing_death
        .summon_actor_definition_mut(0)
        .as_object_mut()
        .unwrap()
        .remove("death");
    assert_has(&decode_error(&missing_death), "missing field `death`");

    let mut bad_ownership = parts("summons_created_creature_lifecycle");
    select_registry_row_by_runtime_id(&mut bad_ownership, "items", "healing_balm");
    let template = bad_ownership.selected_mut("summon_templates", 0);
    template["item_instances"] = json!({
        "owned_twice": {
            "definition_id": "healing_balm",
            "binding": {"state": "unrestricted"}
        },
        "orphan": {
            "definition_id": "healing_balm",
            "binding": {"state": "unrestricted"}
        },
        "stacked": {
            "definition_id": "healing_balm",
            "quantity": 2,
            "binding": {"state": "unrestricted"}
        }
    });
    template["carried"]["items"] = json!([
        {"item_instance_id": "owned_twice", "position": "sack_item_1"},
        {"item_instance_id": "missing", "position": "sack_item_2"},
        {"item_instance_id": "owned_twice", "position": "right_hand"},
        {"item_instance_id": "stacked", "position": "left_hand"}
    ]);
    let error = definition_error(&bad_ownership);
    assert_has(&error, "references unknown item instance");
    assert_has(
        &error,
        "item instance \"owned_twice\" is referenced more than once",
    );
    assert_has(&error, "item instance \"orphan\" has no owner or location");
    assert_has(&error, "must have quantity 1 outside the sack");

    let mut bad_template = parts("summons_created_creature_lifecycle");
    bad_template.selected_mut("summon_templates", 0)["id"] = json!("");
    let template_definition = bad_template.summon_actor_definition_mut(0);
    template_definition["social"]["owner_relation"] = json!("none");
    template_definition["ai"]["cadence_units"] = json!(0);
    let error = definition_error(&bad_template);
    assert_has(&error, "summon_templates[0].id must be non-empty");
    assert_has(
        &error,
        "owner_relation must be summoner for a summon template",
    );
    assert_has(&error, "ai.cadence_units must be positive");

    let mut bad_ai = parts("summons_created_creature_lifecycle");
    bad_ai.summon_actor_definition_mut(0)["ai"]["behavior"] = json!("bad_ai");
    assert_has(&decode_error(&bad_ai), "unknown variant `bad_ai`");

    let mut missing_template_ref = parts("summons_created_creature_lifecycle");
    missing_template_ref.selected_by_runtime_id_mut("spells", "call_echo")["effect"]["summon_actor_id"] =
        json!("missing_template");
    assert_has(
        &definition_error(&missing_template_ref),
        "is not a summon_templates id",
    );

    let mut wrong_target = parts("summons_created_creature_lifecycle");
    wrong_target.selected_by_runtime_id_mut("spells", "call_echo")["target"]["kind"] =
        json!("actor");
    assert_has(
        &definition_error(&wrong_target),
        "target.kind must be coordinate for summon spells",
    );

    let mut missing_duration = parts("summons_created_creature_lifecycle");
    missing_duration.selected_by_runtime_id_mut("spells", "call_echo")["effect"]
        .as_object_mut()
        .unwrap()
        .remove("duration");
    assert_has(
        &definition_error(&missing_duration),
        "effect.duration must be present for summon spells",
    );
}

#[test]
fn quest_summon_profession_and_storage_definitions_retain_domain_validation() {
    let mut quest = parts("npc_quest_interactions");
    quest.selected_mut("quests", 0)["stages"][1]["id"] =
        quest.selected_mut("quests", 0)["stages"][0]["id"].clone();
    assert_has(&definition_error(&quest), "stages[1].id");

    let mut summon = parts("summons_created_creature_lifecycle");
    summon.selected_mut("summon_templates", 0)["id"] = json!(" ");
    assert_has(
        &definition_error(&summon),
        "summon_templates[0].id must be non-empty",
    );

    let mut summon_ai = parts("summons_created_creature_lifecycle");
    summon_ai.summon_actor_definition_mut(0)["ai"]["cadence_units"] = json!(0);
    assert_has(&definition_error(&summon_ai), "cadence_units");

    let mut profession = parts("profession_specific_actions");
    profession.selected_mut("profession_actions", 0)["id"] = json!(" ");
    assert_has(&definition_error(&profession), "profession_actions[0].id");

    let mut bank = parts("gold_bank_locker_storage");
    bank.selected_mut("banks", 0)["transaction_cap_gold"] = json!(0);
    assert_has(&definition_error(&bank), "transaction_cap_gold");

    let mut vault = parts("gold_bank_locker_storage");
    vault.selected_mut("locker_vaults", 0)["capacity"] = json!(0);
    assert_has(&definition_error(&vault), "capacity");
}

#[test]
fn actor_seed_identity_roles_social_ai_stats_and_room_placement_are_validated() {
    let mut duplicate = parts("first_room");
    duplicate.actors_mut()[1]["id"] = duplicate.actors_mut()[0]["id"].clone();
    assert_has(&seed_error(&duplicate), "duplicates actors[0].id");

    let mut no_player = parts("first_room");
    let monster_social = no_player.actor_definition_mut(1)["social"].clone();
    let monster_ai = no_player.actor_definition_mut(1)["ai"].clone();
    no_player.actor_definition_mut(0)["kind"] = json!("monster");
    no_player.actor_definition_mut(0)["social"] = monster_social;
    no_player.actor_definition_mut(0)["ai"] = monster_ai;
    assert_has(&seed_error(&no_player), "at least one player");

    let mut player_ai = parts("first_room");
    player_ai.actor_definition_mut(0)["ai"] = player_ai.actor_definition_mut(1)["ai"].clone();
    assert_has(&definition_error(&player_ai), "ai is forbidden for players");

    let mut missing_monster_ai = parts("first_room");
    missing_monster_ai.actor_definition_mut(1)["ai"] = Value::Null;
    assert_has(
        &definition_error(&missing_monster_ai),
        "ai is required for monsters",
    );

    let mut bad_social = parts("first_room");
    bad_social.actor_definition_mut(0)["social"]["owner_relation"] = json!("summoner");
    assert_has(&definition_error(&bad_social), "owner_relation");

    let mut bad_stats = parts("first_room");
    bad_stats.actor_definition_mut(1)["stats"]["hp"] = json!(0);
    assert_has(&definition_error(&bad_stats), "stats must use positive HP");

    let mut blocked = parts("first_room");
    blocked.actors_mut()[0]["location"]["position"] = json!({"x": 0, "y": 0});
    assert_has(&seed_error(&blocked), "not traversable");

    let mut unknown_room = parts("first_room");
    unknown_room.actors_mut()[0]["location"]["level"] = json!("missing");
    assert_has(
        &seed_error(&unknown_room),
        "realm/level does not exist in the selected world template",
    );
}

#[test]
fn actor_social_authority_enforcer_ai_and_summoner_matrices_are_exact() {
    let authoritative = parts("alignment_social_law");
    authoritative
        .validated_seed()
        .expect("player, lawful NPC, and monster social shapes");
    let authored_summon = parts("summons_created_creature_lifecycle");
    authored_summon
        .validated_seed()
        .expect("summoner-owned inherent summon shape");

    let player = actor_seed_index(&authoritative, "player");
    let enforcer = actor_seed_index(&authoritative, "oath_watch");
    let civilian = actor_seed_index(&authoritative, "harbor_warden");
    let monster = actor_seed_index(&authoritative, "storm_shade");

    let mut characterless_authority = authoritative.clone();
    characterless_authority.actors_mut()[player]
        .as_object_mut()
        .expect("player")
        .remove("character");
    characterless_authority.actors_mut()[player]
        .as_object_mut()
        .expect("player")
        .remove("character_id");
    assert_has(
        &seed_error(&characterless_authority),
        "alignment_source character requires a character-backed actor",
    );

    let mut duplicate_authority = authoritative.clone();
    duplicate_authority.actor_definition_mut(player)["social"]["alignment_source"] =
        json!({"kind": "inherent", "alignment": "lawful"});
    assert_has(
        &seed_error(&duplicate_authority),
        "alignment_source must be character for a character-backed actor",
    );

    let mut stray_social = authoritative.clone();
    stray_social.actor_definition_mut(player)["social"]["team"] = json!("heroes");
    assert_has(&decode_error(&stray_social), "unknown field `team`");

    let mut stray_authority = authoritative.clone();
    stray_authority.actor_definition_mut(enforcer)["social"]["alignment_source"]["karma"] =
        json!(1);
    assert_has(&decode_error(&stray_authority), "unknown field `karma`");

    let mut invalid_enforcer = authoritative.clone();
    invalid_enforcer.actor_definition_mut(enforcer)["social"]["alignment_source"]["alignment"] =
        json!("neutral");
    assert_has(
        &seed_error(&invalid_enforcer),
        "town_enforcer requires an inherent-lawful human NPC",
    );

    let mut missing_npc_ai = authoritative.clone();
    missing_npc_ai.actor_definition_mut(civilian)["ai"] = Value::Null;
    assert_has(
        &seed_error(&missing_npc_ai),
        "ai is required for an inherent-lawful human NPC",
    );

    let mut forbidden_npc_ai = authoritative.clone();
    forbidden_npc_ai.actor_definition_mut(civilian)["social"] = json!({
        "alignment_source": {"kind": "inherent", "alignment": "neutral"},
        "nature": "human",
        "behavior": "passive",
        "owner_relation": "none"
    });
    assert_has(
        &seed_error(&forbidden_npc_ai),
        "ai is valid on an NPC only for an inherent-lawful human",
    );

    let mut ordinary_summoner = authoritative.clone();
    ordinary_summoner.actor_definition_mut(monster)["social"]["owner_relation"] = json!("summoner");
    assert_has(
        &seed_error(&ordinary_summoner),
        "summoner is valid only for summon templates",
    );

    let mut summon_character_authority = authored_summon.clone();
    summon_character_authority.summon_actor_definition_by_template_id_mut("echo_guardian")["social"]
        ["alignment_source"] = json!({"kind": "character"});
    assert_has(
        &definition_error(&summon_character_authority),
        "alignment_source must be inherent for a summon template",
    );

    let mut summon_without_relation = authored_summon;
    summon_without_relation.summon_actor_definition_by_template_id_mut("echo_guardian")["social"]
        ["owner_relation"] = json!("none");
    assert_has(
        &definition_error(&summon_without_relation),
        "owner_relation must be summoner for a summon template",
    );
}

#[test]
fn spell_social_hostility_and_town_law_follow_effect_and_target_semantics() {
    let mut harmful_not_hostile = parts("spell_effects");
    harmful_not_hostile.selected_by_runtime_id_mut("spells", "spark")["social"]["hostile_act"] =
        json!(false);
    assert_has(
        &definition_error(&harmful_not_hostile),
        "social.hostile_act must be true for the current effect family and target",
    );

    let mut harmful_terrain_law = parts("spell_effects");
    harmful_terrain_law.selected_by_runtime_id_mut("spells", "spark")["social"]["town_law"] =
        json!("terrain_alignment_violation");
    assert_has(
        &definition_error(&harmful_terrain_law),
        "terrain_alignment_violation requires a terrain, darkness, or light effect family",
    );

    let terrain_law = parts("alignment_social_law");
    terrain_law
        .definition()
        .expect("light effect may carry terrain-alignment law");
}

#[test]
fn law_zone_character_alignment_and_legacy_social_shapes_are_strict() {
    let mut town = parts("first_room");
    town.template_levels_source_mut()["room_0"]["law_zone"] = json!("town");
    town.definition().expect("town is an authored law zone");

    let mut missing_zone = parts("first_room");
    missing_zone.template_levels_source_mut()["room_0"]
        .as_object_mut()
        .expect("room")
        .remove("law_zone");
    assert_has(&decode_error(&missing_zone), "missing field `law_zone`");

    let mut invalid_zone = parts("first_room");
    invalid_zone.template_levels_source_mut()["room_0"]["law_zone"] = json!("city");
    assert_has(&decode_error(&invalid_zone), "unknown variant `city`");

    let mut allegiance = parts("alignment_social_law");
    let player = actor_seed_index(&allegiance, "player");
    allegiance.actors_mut()[player]["allegiance"] = json!("player");
    assert_has(&decode_error(&allegiance), "unknown field `allegiance`");

    let mut missing_social = parts("alignment_social_law");
    missing_social
        .actor_definition_mut(player)
        .as_object_mut()
        .expect("player")
        .remove("social");
    assert_has(&decode_error(&missing_social), "missing field `social`");

    let mut missing_alignment = parts("alignment_social_law");
    missing_alignment.actors_mut()[player]["character"]
        .as_object_mut()
        .expect("character")
        .remove("alignment_state");
    assert_has(
        &decode_error(&missing_alignment),
        "missing field `alignment_state`",
    );

    let mut invalid_alignment = parts("alignment_social_law");
    invalid_alignment.actors_mut()[player]["character"]["alignment_state"]["alignment"] =
        json!("saintly");
    assert_has(
        &decode_error(&invalid_alignment),
        "unknown variant `saintly`",
    );

    for karma in [json!(-1), json!(false), json!(u64::from(u32::MAX) + 1)] {
        let mut value = parts("alignment_social_law");
        value.actors_mut()[player]["character"]["alignment_state"]["karma_points"] = karma;
        assert_has(&decode_error(&value), "expected u32");
    }

    let mut obsolete_status = parts("alignment_social_law");
    obsolete_status.actors_mut()[player]["character"]["knighthood_state"] =
        json!({"knighted": false});
    assert_has(
        &decode_error(&obsolete_status),
        "unknown field `knighthood_state`",
    );
}

#[test]
fn character_and_starter_seed_domains_validate_identity_progression_resources_and_skills() {
    let mut missing_id = parts("character_sheet");
    missing_id.actors_mut()[0]
        .as_object_mut()
        .unwrap()
        .remove("character_id");
    assert_has(&seed_error(&missing_id), "character_id is required");

    let mut transient_id = parts("character_sheet");
    transient_id.actors_mut()[0]["character_id"] = transient_id.actors_mut()[0]["id"].clone();
    assert_has(
        &seed_error(&transient_id),
        "must differ from transient actor id",
    );

    let mut bad_attribute = parts("character_sheet");
    bad_attribute.actors_mut()[0]["character"]["attributes"]["strength"] = json!(19);
    assert_has(&seed_error(&bad_attribute), "must be between 3 and 18");

    let mut bad_resource = parts("character_sheet");
    bad_resource.actors_mut()[0]["character"]["resources"]["hp"] = json!(99);
    assert_has(&seed_error(&bad_resource), "must not exceed max_hp");

    let mut bad_level = parts("character_sheet");
    bad_level.actors_mut()[0]["character"]["progression"]["level"] = json!(2);
    assert_has(
        &seed_error(&bad_level),
        "must not exceed the XP-earned level",
    );

    let mut starter = parts("starter_circuit");
    starter.actors_mut()[0]["starter_character"]["creation"]["current_class_id"] = json!("missing");
    assert_has(&seed_error(&starter), "current_class_id");
}

#[test]
fn item_instances_have_one_definition_one_owner_valid_placement_and_safe_arithmetic() {
    let mut unknown_definition = parts("first_room");
    unknown_definition.item_instances_mut()["training_knife"]["definition_id"] = json!("missing");
    assert_has(&seed_error(&unknown_definition), "unknown item definition");

    let mut dangling = parts("first_room");
    dangling.actors_mut()[0]["carried"]["items"][0]["item_instance_id"] = json!("missing");
    assert_has(&seed_error(&dangling), "unknown item instance");

    let mut orphan = parts("first_room");
    *orphan.actors_mut()[0]["carried"]["items"]
        .as_array_mut()
        .unwrap() = Vec::new();
    assert_has(&seed_error(&orphan), "has no owner or location");

    let mut duplicate_owner = parts("first_room");
    *duplicate_owner.ground_items_mut() = json!([{
        "item_instance_id": "training_knife",
        "location": {
            "realm": "realm_0",
            "level": "room_0",
            "position": {"x": 1, "y": 1}
        }
    }]);
    assert_has(&seed_error(&duplicate_owner), "referenced more than once");

    let mut zero = parts("first_room");
    zero.item_instances_mut()["training_knife"]["quantity"] = json!(0);
    assert_has(&seed_error(&zero), "quantity must be positive");

    let mut reserved = parts("first_room");
    let row = reserved.item_instances_mut()["training_knife"].clone();
    reserved
        .item_instances_mut()
        .as_object_mut()
        .unwrap()
        .remove("training_knife");
    reserved.item_instances_mut()["summon:authored"] = row;
    reserved.actors_mut()[0]["carried"]["items"][0]["item_instance_id"] = json!("summon:authored");
    assert_has(&seed_error(&reserved), "reserved prefix");

    let mut bad_position = parts("first_room");
    bad_position.actors_mut()[0]["carried"]["items"][0]["position"] = json!("inner_armor");
    assert_has(&seed_error(&bad_position), "cannot occupy carried position");
}

#[test]
fn item_seed_binding_stack_knowledge_and_checked_value_contracts_are_exact() {
    let mut tied_stack = parts("item_instance_contract");
    tied_stack.item_instances_mut()["tonic_a"]["binding"] =
        json!({"state": "bind_on_first_character_touch"});
    assert_has(
        &seed_error(&tied_stack),
        "quantity must be 1 for a tied item instance",
    );

    let mut empty_binding = parts("item_instance_contract");
    empty_binding.item_instances_mut()["tonic_b"]["binding"] =
        json!({"state": "bound", "character_id": " "});
    assert_has(
        &seed_error(&empty_binding),
        "binding.character_id must be non-empty",
    );

    let mut active_stack = parts("item_instance_contract");
    active_stack.actors_mut()[0]["carried"]["items"] =
        json!([{"item_instance_id": "tonic_a", "position": "right_hand"}]);
    active_stack
        .ground_items_mut()
        .as_array_mut()
        .expect("ground items")
        .retain(|row| row["item_instance_id"] != "tonic_a");
    assert_has(
        &seed_error(&active_stack),
        "must have quantity 1 outside the sack",
    );

    for (economy_field, expected) in [
        (
            "unit_value_gold",
            "quantity * unit_value_gold must not overflow",
        ),
        ("unit_burden", "quantity * unit_burden must not overflow"),
    ] {
        let mut value = parts("item_instance_contract");
        value.selected_by_runtime_id_mut("items", "restorative_tonic")["economy"][economy_field] =
            json!(u64::MAX);
        assert_has(&seed_error(&value), expected);
    }

    let mut unknown_knowledge = parts("item_instance_contract");
    unknown_knowledge.item_instances_mut()["tonic_a"]["knowledge"]["guessed"] = json!(true);
    assert_has(&decode_error(&unknown_knowledge), "unknown field `guessed`");

    let mut mistyped_knowledge = parts("item_instance_contract");
    mistyped_knowledge.item_instances_mut()["tonic_a"]["knowledge"]["identified"] = json!(1);
    assert_has(&decode_error(&mistyped_knowledge), "expected a boolean");

    let mut unbound_spell_book = parts("magic_profession_gallery");
    unbound_spell_book.item_instances_mut()["spell_book"]["binding"] =
        json!({"state": "unrestricted"});
    assert_has(
        &seed_error(&unbound_spell_book),
        "binding must be bound for a Spell Book",
    );

    let mut unknown_spell_book_owner = parts("magic_profession_gallery");
    unknown_spell_book_owner.item_instances_mut()["spell_book"]["binding"] =
        json!({"state": "bound", "character_id": "character:missing"});
    assert_has(
        &seed_error(&unknown_spell_book_owner),
        "binding.character_id references no scenario character",
    );

    let mut stacked_spell_book = parts("magic_profession_gallery");
    stacked_spell_book.item_instances_mut()["spell_book"]["quantity"] = json!(2);
    assert_has(
        &seed_error(&stacked_spell_book),
        "quantity must be 1 for a Spell Book",
    );
}

#[test]
fn ground_item_seed_shape_ownership_and_room_positions_are_exact() {
    parts("first_room")
        .validated_seed()
        .expect("an explicitly empty ground-item list must validate");
    parts("supply_cache")
        .validated_seed()
        .expect("current positioned ground items must validate");

    let mut flat = parts("supply_cache");
    let row = &mut flat.ground_items_mut()[0];
    row.as_object_mut().unwrap().remove("item_instance_id");
    row["item_id"] = json!("hemp_rope");
    let error = decode_error(&flat);
    assert_has(&error, "unknown field `item_id`");

    let mut blank = parts("supply_cache");
    blank.ground_items_mut()[0]["item_instance_id"] = json!("");
    assert_has(&seed_error(&blank), "item_instance_id must be non-empty");

    let mut unknown = parts("supply_cache");
    unknown.ground_items_mut()[0]["item_instance_id"] = json!("missing_item");
    assert_has(&seed_error(&unknown), "references unknown item instance");

    let mut out_of_bounds = parts("supply_cache");
    out_of_bounds.ground_items_mut()[0]["location"]["position"] = json!({"x": 5, "y": 1});
    assert_has(&seed_error(&out_of_bounds), "out of bounds");

    let mut blocked = parts("supply_cache");
    blocked.ground_items_mut()[0]["location"]["level"] = json!("room_0");
    blocked.ground_items_mut()[0]["location"]["position"] = json!({"x": 0, "y": 0});
    assert_has(&seed_error(&blocked), "not traversable");

    let mut missing_room = parts("supply_cache");
    missing_room.ground_items_mut()[0]["location"]["level"] = json!("missing");
    assert_has(
        &seed_error(&missing_room),
        "realm/level does not exist in the selected world template",
    );

    let mut duplicate_owner = parts("supply_cache");
    duplicate_owner.ground_items_mut()[1]["item_instance_id"] = json!("hemp_rope");
    assert_has(
        &seed_error(&duplicate_owner),
        "item instance \"hemp_rope\" is referenced more than once",
    );
}

#[test]
fn active_effect_seed_rows_are_strict_and_resistance_bounded() {
    parts("status_effects")
        .validated_seed()
        .expect("current typed active-effect seed row must validate");

    let mut invalid = parts("status_effects");
    invalid.actors_mut()[0]["active_effects"][0] = json!({
        "instance_id": "",
        "effect_id": "",
        "source": {"kind": "bad", "id": ""},
        "kind": "",
        "tags": ["stun", ""],
        "potency": -1,
        "remaining_rounds": 0,
        "stacking": "bad",
        "start_delay_rounds": -1,
        "tick_interval_rounds": 0,
        "suppresses_action": true,
        "resistance_boosts": [
            {"tag": "", "bonus_twentieths": 0},
            {"tag": "stun", "bonus_twentieths": 21},
            {"tag": "stun", "bonus_twentieths": 3}
        ]
    });
    let error = seed_error(&invalid);
    for expected in [
        "instance_id must be non-empty",
        "effect_id must be non-empty",
        "source.kind is invalid",
        "source.id must be non-empty",
        "kind must be non-empty",
        "tags must contain non-empty strings",
        "potency must be non-negative",
        "remaining_rounds must be positive",
        "stacking is invalid",
        "start_delay_rounds must be non-negative",
        "tick_interval_rounds must be positive",
        "resistance_boosts[0].tag must be non-empty",
        "resistance_boosts[0].bonus_twentieths must be in range",
        "resistance_boosts[1].bonus_twentieths must be in range",
        "resistance_boosts tags must be unique",
    ] {
        assert_has(&error, expected);
    }

    let mut non_boolean = parts("status_effects");
    non_boolean.actors_mut()[0]["active_effects"][0]["suppresses_action"] = json!("yes");
    assert_has(&decode_error(&non_boolean), "invalid type");

    let mut duplicate = parts("status_effects");
    let mut second = duplicate.actors_mut()[0]["active_effects"][0].clone();
    second["effect_id"] = json!("second_effect");
    duplicate.actors_mut()[0]["active_effects"]
        .as_array_mut()
        .unwrap()
        .push(second);
    assert_has(&seed_error(&duplicate), "instance_id duplicates");
}

#[test]
fn monster_ability_actor_summon_and_spell_compatibility_are_exact() {
    let current = parts("monster_spellcasting_special_attacks");
    current
        .validated_seed()
        .expect("current actor monster-ability rows must validate");

    let mut summon = parts("summons_created_creature_lifecycle");
    summon.profile_value_mut()["spells"]
        .as_array_mut()
        .unwrap()
        .push(json!("spell/mend/spell_effects"));
    summon.summon_actor_definition_mut(0)["monster_abilities"] = json!([{
        "id": "mend_self",
        "kind": "spell",
        "spell_id": "mend",
        "cooldown_rounds": 2,
        "target_policy": "self"
    }]);
    summon
        .definition()
        .expect("summon-template monster ability must validate");

    let mut player_list = parts("monster_spellcasting_special_attacks");
    player_list.actor_definition_mut(0)["monster_abilities"] = json!([{
        "id": "forbidden",
        "kind": "spell",
        "spell_id": "ember_spit",
        "cooldown_rounds": 1,
        "target_policy": "nearest_hostile"
    }]);
    assert_has(
        &seed_error(&player_list),
        "monster_abilities is only valid for monsters",
    );

    let mut non_monster_template = parts("summons_created_creature_lifecycle");
    let template = non_monster_template.summon_actor_definition_mut(0);
    template["kind"] = json!("player");
    template["monster_abilities"] = json!([]);
    assert_has(
        &definition_error(&non_monster_template),
        "actor_definition_id must reference a monster definition with AI",
    );

    let monster_index = actor_seed_index(&current, "ember_imp");

    let mut invalid_fields = current.clone();
    let ability = &mut invalid_fields.actor_definition_mut(monster_index)["monster_abilities"][0];
    ability["id"] = json!("");
    ability["kind"] = json!("breath");
    ability["spell_id"] = json!("");
    ability["cooldown_rounds"] = json!(0);
    ability["target_policy"] = json!("furthest_hostile");
    let error = seed_error(&invalid_fields);
    for expected in [
        "id must be non-empty",
        "kind must be one of spell, special_attack",
        "spell_id must be non-empty",
        "cooldown_rounds must be >= 1",
        "target_policy must be one of nearest_hostile, self",
    ] {
        assert_has(&error, expected);
    }

    let mut duplicate = current.clone();
    let second = duplicate.actor_definition_mut(monster_index)["monster_abilities"][0].clone();
    duplicate.actor_definition_mut(monster_index)["monster_abilities"]
        .as_array_mut()
        .unwrap()
        .push(second);
    assert_has(
        &seed_error(&duplicate),
        "id duplicates actor_definitions[1].monster_abilities[0].id",
    );

    let mut unknown = current.clone();
    unknown.actor_definition_mut(monster_index)["monster_abilities"][0]["spell_id"] =
        json!("missing");
    assert_has(&seed_error(&unknown), "references unknown spell");

    let mut unsupported_target = current.clone();
    unsupported_target.selected_by_runtime_id_mut("spells", "ember_spit")["target"]["kind"] =
        json!("door");
    assert_has(
        &seed_error(&unsupported_target),
        "unsupported monster target kind",
    );

    let mut unsupported_family = current.clone();
    let spell = unsupported_family.selected_by_runtime_id_mut("spells", "ember_spit");
    spell["social"]["hostile_act"] = json!(false);
    spell["effect"] = json!({
        "family": "scry",
        "scry": {
            "scope": "level",
            "site": {"realm": "realm_0", "level": "room_0"}
        }
    });
    spell["target"] = json!({"kind": "none"});
    assert_has(
        &seed_error(&unsupported_family),
        "unsupported monster effect family",
    );

    let mut unsupported_combination = current.clone();
    unsupported_combination.actor_definition_mut(monster_index)["monster_abilities"][0]["target_policy"] =
        json!("self");
    assert_has(
        &seed_error(&unsupported_combination),
        "unsupported monster effect/target combination",
    );

    let mut indirect = current;
    indirect.selected_by_runtime_id_mut("spells", "ember_spit")["casting"]["method"] =
        json!("warm_then_cast");
    assert_has(&seed_error(&indirect), "must reference a direct-cast spell");
}

#[test]
fn npc_interactions_validate_transactions_quest_gates_exact_items_and_escort_outcomes() {
    let mut cadence = parts("npc_quest_interactions");
    cadence.actors_mut()[1]["npc"]["follow_cadence_units"] = json!(0);
    assert_has(
        &seed_error(&cadence),
        "follow_cadence_units must be positive",
    );

    let mut response = parts("npc_quest_interactions");
    response.actors_mut()[1]["npc"]["interactions"][0]["response"] = json!(" ");
    assert_has(&seed_error(&response), "response must be non-empty");

    let mut quest = parts("npc_quest_interactions");
    quest.actors_mut()[1]["npc"]["interactions"][0]["transaction"]["requirements"][0]["quest_id"] =
        json!("missing");
    assert_has(&seed_error(&quest), "unknown quest");

    let mut item = parts("npc_quest_interactions");
    item.actors_mut()[1]["npc"]["interactions"][1]["transaction"]["requirements"][1]["item_definition_id"] =
        json!("missing");
    assert_has(&seed_error(&item), "unknown item definition");

    let mut selected_cost = parts("npc_quest_interactions");
    selected_cost.actors_mut()[1]["npc"]["interactions"][1]["transaction"]["requirements"] =
        json!([]);
    assert_has(
        &seed_error(&selected_cost),
        "requires a carried_item requirement",
    );

    let mut escort = parts("npc_quest_interactions");
    escort.actors_mut()[1]["npc"]["interactions"][2]["transaction"]["requirements"] = json!([]);
    assert_has(&seed_error(&escort), "requires npc_accompanying");

    let mut reward = parts("npc_quest_interactions");
    reward.actors_mut()[2]["npc"]["interactions"][0]["transaction"]["rewards"][0]["stage_id"] =
        json!("missing");
    assert_has(&seed_error(&reward), "unknown quest/stage");
}

#[test]
fn service_instances_and_merchant_inventories_have_exact_definition_capability_and_stock_joins() {
    let mut duplicate_instance = parts("merchant_item_services");
    duplicate_instance.service_instances_mut()[1]["id"] =
        duplicate_instance.service_instances_mut()[0]["id"].clone();
    assert_has(
        &seed_error(&duplicate_instance),
        "duplicates service_instances[0].id",
    );

    let mut unknown_definition = parts("merchant_item_services");
    unknown_definition.service_instances_mut()[0]["service_definition_id"] = json!("missing");
    assert_has(
        &seed_error(&unknown_definition),
        "unknown selected service definition",
    );

    let mut blocked = parts("merchant_item_services");
    blocked.service_instances_mut()[0]["location"]["position"] = json!({"x": 0, "y": 0});
    assert_has(&seed_error(&blocked), "not traversable");

    let mut missing_inventory = parts("merchant_item_services");
    missing_inventory
        .merchant_inventories_mut()
        .as_array_mut()
        .unwrap()
        .remove(0);
    assert_has(
        &seed_error(&missing_inventory),
        "requires exactly one merchant inventory",
    );

    let mut duplicate_inventory = parts("merchant_item_services");
    let row = duplicate_inventory.merchant_inventories_mut()[0].clone();
    duplicate_inventory
        .merchant_inventories_mut()
        .as_array_mut()
        .unwrap()
        .push(row);
    assert_has(
        &seed_error(&duplicate_inventory),
        "duplicates merchant_inventories[0]",
    );

    let mut wrong_capability = parts("merchant_item_services");
    wrong_capability.merchant_inventories_mut()[0]["capability_id"] = json!("missing");
    assert_has(
        &seed_error(&wrong_capability),
        "must reference a merchant capability",
    );

    let mut bad_price = parts("merchant_item_services");
    bad_price.merchant_inventories_mut()[0]["stock"][0]["price_gold"] = json!(0);
    assert_has(&seed_error(&bad_price), "price_gold must be positive");

    let mut unknown_stock = parts("merchant_item_services");
    unknown_stock.merchant_inventories_mut()[0]["stock"][0]["item_instance_id"] = json!("missing");
    assert_has(&seed_error(&unknown_stock), "unknown item instance");

    let mut duplicate_stock = parts("merchant_item_services");
    let row = duplicate_stock.merchant_inventories_mut()[0]["stock"][0].clone();
    duplicate_stock.merchant_inventories_mut()[0]["stock"]
        .as_array_mut()
        .unwrap()
        .push(row);
    assert_has(&seed_error(&duplicate_stock), "unique within the inventory");
}

#[test]
fn immutable_service_definitions_validate_capabilities_training_transactions_and_storage_refs() {
    let mut empty_name = parts("service_transactions");
    empty_name.selected_mut("service_definitions", 0)["name"] = json!(" ");
    assert_has(&definition_error(&empty_name), "name must be non-empty");

    let mut empty_capabilities = parts("service_transactions");
    empty_capabilities.selected_mut("service_definitions", 0)["capabilities"] = json!([]);
    assert_has(
        &definition_error(&empty_capabilities),
        "capabilities must be a non-empty list",
    );

    let mut duplicate_capability = parts("service_transactions");
    let capability =
        duplicate_capability.selected_mut("service_definitions", 0)["capabilities"][0].clone();
    duplicate_capability.selected_mut("service_definitions", 0)["capabilities"]
        .as_array_mut()
        .unwrap()
        .push(capability);
    assert_has(&definition_error(&duplicate_capability), "duplicates");

    let mut bad_transaction = parts("service_transactions");
    bad_transaction.selected_mut("service_definitions", 0)["capabilities"][0]["transactions"][0]
        ["requirements"][1]["level"] = json!(0);
    assert_has(
        &definition_error(&bad_transaction),
        "level must be positive",
    );

    let mut missing_item = parts("service_transactions");
    missing_item.selected_mut("service_definitions", 0)["capabilities"][0]["transactions"][0]["requirements"]
        [3]["item_definition_id"] = json!("missing");
    assert_has(&definition_error(&missing_item), "unknown item definition");

    let mut bad_bank = parts("gold_bank_locker_storage");
    let bank_service = (0..bad_bank.selected_len("service_definitions"))
        .find(|index| {
            selected_row(&bad_bank, "service_definitions", *index)["capabilities"]
                .as_array()
                .is_some_and(|caps| caps.iter().any(|cap| cap["kind"] == "bank"))
        })
        .expect("bank service");
    let bank_capability =
        bad_bank.selected_mut("service_definitions", bank_service)["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .position(|cap| cap["kind"] == "bank")
            .unwrap();
    bad_bank.selected_mut("service_definitions", bank_service)["capabilities"][bank_capability]["bank_id"] =
        json!("missing");
    assert_has(&definition_error(&bad_bank), "unknown bank");
}

#[test]
fn teaching_promotion_and_player_sales_keep_exact_definition_and_placement_semantics() {
    let mut wrong_training_kind = parts("spell_learning_purchase_casting_xp");
    let (definition_index, teaching_index) =
        selected_service_capability(&wrong_training_kind, "spell_teaching");
    wrong_training_kind.selected_mut("service_definitions", definition_index)["capabilities"]
        [teaching_index]["training_capability_id"] = json!("critique");
    assert_has(
        &definition_error(&wrong_training_kind),
        "must reference skill_training",
    );

    let mut wrong_teaching_lane = parts("spell_learning_purchase_casting_xp");
    let (definition_index, teaching_index) =
        selected_service_capability(&wrong_teaching_lane, "spell_teaching");
    wrong_teaching_lane.selected_mut("service_definitions", definition_index)["capabilities"]
        [teaching_index]["teachings"][0]["spell_id"] = json!("prayer");
    assert_has(
        &definition_error(&wrong_teaching_lane),
        "must match the trainer magic lane",
    );

    let mut wrong_promotion_level = parts("knight_promotion");
    let (definition_index, promotion_index) =
        selected_service_capability(&wrong_promotion_level, "class_promotion");
    let transaction = &mut wrong_promotion_level
        .selected_mut("service_definitions", definition_index)["capabilities"][promotion_index]["transaction"];
    let level_index = transaction["requirements"]
        .as_array()
        .expect("promotion requirements")
        .iter()
        .position(|requirement| requirement["kind"] == "minimum_level")
        .expect("minimum level requirement");
    transaction["requirements"][level_index]["level"] = json!(7);
    assert_has(
        &definition_error(&wrong_promotion_level),
        "minimum_level must be 8",
    );

    let mut short_promotion_grant = parts("knight_promotion");
    let (definition_index, promotion_index) =
        selected_service_capability(&short_promotion_grant, "class_promotion");
    short_promotion_grant.selected_mut("service_definitions", definition_index)["capabilities"]
        [promotion_index]["transaction"]["rewards"]
        .as_array_mut()
        .expect("promotion rewards")
        .pop();
    assert_has(
        &definition_error(&short_promotion_grant),
        "must contain exactly five spell rewards",
    );

    let mut duplicate_promotion_placement = parts("knight_promotion");
    let mut duplicate = duplicate_promotion_placement.service_instances_mut()[0].clone();
    duplicate["id"] = json!("second_knight_promoter");
    duplicate_promotion_placement
        .service_instances_mut()
        .as_array_mut()
        .expect("service instances")
        .push(duplicate);
    assert_has(
        &seed_error(&duplicate_promotion_placement),
        "room/position/target",
    );

    let mut duplicate_teaching_placement = parts("spell_learning_purchase_casting_xp");
    let mut duplicate = duplicate_teaching_placement.service_instances_mut()[0].clone();
    duplicate["id"] = json!("second_wizard_trainer");
    duplicate_teaching_placement
        .service_instances_mut()
        .as_array_mut()
        .expect("service instances")
        .push(duplicate);
    assert_has(
        &seed_error(&duplicate_teaching_placement),
        "room/position/class/spell",
    );

    let mut overflowing_pawn_price = parts("merchant_item_services");
    overflowing_pawn_price.selected_mut("items", 0)["economy"]["unit_value_gold"] = json!(i64::MAX);
    assert_has(
        &seed_error(&overflowing_pawn_price),
        "player_sales cannot price item instance",
    );
}

#[test]
fn direct_engine_construction_uses_the_validated_bound_seed_without_reparse() {
    let parts = parts("first_room");
    let validated = parts.validated_seed().expect("checked seed");
    let definition = std::sync::Arc::clone(validated.definition());
    let engine = Engine::new(validated, 42).expect("engine starts from checked seed");
    assert!(std::sync::Arc::ptr_eq(&definition, engine.definition()));
    assert_eq!(engine.world().actors.len(), 2);
    assert_eq!(engine.world().actors[0].id, "player");
}
