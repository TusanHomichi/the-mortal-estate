use super::*;

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
