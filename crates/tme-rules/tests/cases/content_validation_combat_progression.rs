use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};

fn parts(case_id: &str) -> ContentParts {
    ContentParts::tracked(case_id, &format!("profile/{case_id}"))
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

fn json_number(source: &str) -> Value {
    serde_json::from_str(source).expect("literal JSON number")
}

fn rules_source(parts: &ContentParts) -> &Value {
    let key = parts.profile_value()["rules_profile"]
        .as_str()
        .expect("rules profile key");
    &parts.catalog["rules_profiles"][key]
}

#[test]
fn legacy_canonical_combat_rules_remain_explicit_and_valid() {
    let value = parts("first_room");
    value
        .definition()
        .expect("the canonical current combat rules are valid");
    let combat = &rules_source(&value)["combat"];
    assert_eq!(combat["tuning_status"], json!("original_provisional"));
    assert_eq!(
        combat["attack_modes"],
        json!({
            "kick": {
                "maximum_range": 0,
                "cooldown_units": 1,
                "damage_kind": "crushing"
            },
            "jumpkick": {
                "maximum_range_cap": 3,
                "skill_levels_per_extra_hex": 3,
                "stamina_cost": 1,
                "cooldown_units": 1,
                "damage_kind": "crushing"
            }
        })
    );
    assert_eq!(
        combat["block"],
        json!({
            "left_hand_selection_percent": 75,
            "shield_percent_per_point": 10,
            "shield_percent_cap": 90,
            "armor_percent_per_point": 8,
            "armor_percent_cap": 80,
            "strength_penetration_percent_per_add": 2,
            "armor_encumbrance_percent_per_point": 2,
            "combat_add_penetration_percent_per_rating": 2
        })
    );
    assert_eq!(
        combat["damage"],
        json!({
            "minimum_damage": 1,
            "roll_variation_modulus": 3,
            "moderate_label_min_percent": 20,
            "heavy_label_min_percent": 40,
            "severe_label_min_percent": 70
        })
    );
    assert_eq!(
        combat["wounds"],
        json!({
            "near_death_max_percent": 20,
            "badly_wounded_max_percent": 50,
            "wounded_max_percent": 99
        })
    );
    assert_eq!(
        combat["practice"],
        json!({
            "practice_raw_points": 1,
            "life_and_death_raw_points": 2,
            "overwhelming_raw_points": 1,
            "fatal_blow_bonus_raw_points": 1,
            "life_and_death_minimum_target_xp_per_attacker_level": 5,
            "life_and_death_required_at_skill_level": 8
        })
    );
}

#[test]
fn legacy_combat_contract_rejects_previous_missing_unknown_and_incomplete_shapes() {
    let mut wrong_catalog_version = parts("first_room");
    wrong_catalog_version.catalog["schema_version"] = json!(0);
    assert_has(
        &definition_error(&wrong_catalog_version),
        "catalog.schema_version must be 6",
    );

    let mut missing_combat = parts("first_room");
    missing_combat
        .rules_source_mut()
        .as_object_mut()
        .expect("rules object")
        .remove("combat");
    assert_has(&decode_error(&missing_combat), "missing field `combat`");

    let mut obsolete_sibling = parts("first_room");
    obsolete_sibling.rules_source_mut()["combat_legacy"] = json!({});
    assert_has(
        &decode_error(&obsolete_sibling),
        "unknown field `combat_legacy`",
    );

    let mut obsolete_formula = parts("first_room");
    obsolete_formula.rules_source_mut()["combat"]["formula"] = json!("legacy");
    assert_has(&decode_error(&obsolete_formula), "unknown field `formula`");

    let mut invalid_status = parts("first_room");
    invalid_status.rules_source_mut()["combat"]["tuning_status"] = json!("target_matched");
    assert_has(&decode_error(&invalid_status), "unknown variant");

    for required in [
        "attack_modes",
        "hit",
        "block",
        "fumble",
        "damage",
        "wounds",
        "practice",
    ] {
        let mut missing = parts("first_room");
        missing.rules_source_mut()["combat"]
            .as_object_mut()
            .expect("combat object")
            .remove(required);
        let error = decode_error(&missing);
        assert_has(&error, "missing field");
        assert_has(&error, required);
    }
}

#[test]
fn legacy_kick_and_jumpkick_contract_matrix_is_preserved() {
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
            json!(4),
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
            "damage_kind",
            json!("piercing"),
            "damage_kind must be crushing",
        ),
    ] {
        let mut invalid = parts("first_room");
        invalid.rules_source_mut()["combat"]["attack_modes"][section][field] = bad;
        assert_has(&definition_error(&invalid), expected);
    }

    let mut boolean_kick_cooldown = parts("first_room");
    boolean_kick_cooldown.rules_source_mut()["combat"]["attack_modes"]["kick"]["cooldown_units"] =
        json!(false);
    assert_has(
        &decode_error(&boolean_kick_cooldown),
        "invalid type: boolean",
    );
}

#[test]
fn legacy_combat_positive_integer_and_rust_width_matrix_is_preserved() {
    for (section, field, bad) in [
        ("hit", "base_defender_score", json!(false)),
        ("hit", "attacker_attack_stat_divisor", json!(0)),
        ("hit", "defender_dexterity_divisor", json!(-1)),
    ] {
        let mut invalid = parts("first_room");
        invalid.rules_source_mut()["combat"][section][field] = bad;
        let error = invalid
            .definition()
            .expect_err("invalid positive integer must fail");
        assert!(
            error.contains(field) || error.contains("invalid type"),
            "{field}: {error}"
        );
    }

    for (section, field, overflow) in [
        ("hit", "base_defender_score", json_number("2147483648")),
        ("damage", "minimum_damage", json_number("2147483648")),
        (
            "damage",
            "roll_variation_modulus",
            json_number("4294967296"),
        ),
        (
            "practice",
            "practice_raw_points",
            json_number("18446744073709551616"),
        ),
    ] {
        let mut invalid = parts("first_room");
        invalid.rules_source_mut()["combat"][section][field] = overflow;
        invalid
            .definition()
            .expect_err("Rust destination-width overflow must fail");
    }
}

#[test]
fn legacy_combat_block_percent_and_cap_matrix_is_preserved() {
    for (field, bad) in [
        ("left_hand_selection_percent", json!(0)),
        ("shield_percent_per_point", json!(0)),
        ("shield_percent_cap", json!(101)),
        ("armor_percent_per_point", json!(false)),
        ("armor_percent_cap", json!(0)),
        ("armor_percent_cap", json!(-1)),
        ("strength_penetration_percent_per_add", json!(101)),
        ("armor_encumbrance_percent_per_point", json!(0)),
        ("combat_add_penetration_percent_per_rating", json!(101)),
    ] {
        let mut invalid = parts("first_room");
        invalid.rules_source_mut()["combat"]["block"][field] = bad;
        invalid
            .definition()
            .expect_err("invalid block percentage must fail");
    }

    for (per_point, cap) in [
        ("shield_percent_per_point", "shield_percent_cap"),
        ("armor_percent_per_point", "armor_percent_cap"),
    ] {
        let mut invalid = parts("first_room");
        invalid.rules_source_mut()["combat"]["block"][per_point] = json!(10);
        invalid.rules_source_mut()["combat"]["block"][cap] = json!(9);
        assert_has(
            &definition_error(&invalid),
            &format!("{per_point} must not exceed {cap}"),
        );
    }
}

#[test]
fn legacy_combat_damage_label_threshold_matrix_is_preserved() {
    for (moderate, heavy, severe) in [
        (0, 40, 70),
        (20, 20, 70),
        (40, 20, 70),
        (20, 70, 70),
        (20, 40, 101),
    ] {
        let mut invalid = parts("first_room");
        let damage = &mut invalid.rules_source_mut()["combat"]["damage"];
        damage["moderate_label_min_percent"] = json!(moderate);
        damage["heavy_label_min_percent"] = json!(heavy);
        damage["severe_label_min_percent"] = json!(severe);
        assert_has(
            &definition_error(&invalid),
            "damage label thresholds must satisfy",
        );
    }
}

#[test]
fn legacy_combat_wound_threshold_matrix_is_preserved() {
    for (near, badly, wounded) in [(0, 50, 99), (20, 20, 99), (50, 20, 99), (20, 50, 100)] {
        let mut invalid = parts("first_room");
        let wounds = &mut invalid.rules_source_mut()["combat"]["wounds"];
        wounds["near_death_max_percent"] = json!(near);
        wounds["badly_wounded_max_percent"] = json!(badly);
        wounds["wounded_max_percent"] = json!(wounded);
        assert_has(&definition_error(&invalid), "wounds must satisfy");
    }
}

#[test]
fn legacy_combat_practice_and_skill_cutoff_matrix_is_preserved() {
    for (field, bad, expected) in [
        ("life_and_death_raw_points", json!(0), "must be positive"),
        ("overwhelming_raw_points", json!(0), "must be positive"),
        ("fatal_blow_bonus_raw_points", json!(0), "must be positive"),
        (
            "life_and_death_minimum_target_xp_per_attacker_level",
            json!(0),
            "must be positive",
        ),
        (
            "life_and_death_required_at_skill_level",
            json!(0),
            "must be in 1..=19",
        ),
        (
            "life_and_death_required_at_skill_level",
            json!(20),
            "must be in 1..=19",
        ),
    ] {
        let mut invalid = parts("first_room");
        invalid.rules_source_mut()["combat"]["practice"][field] = bad;
        assert_has(&definition_error(&invalid), expected);
    }

    let mut boolean = parts("first_room");
    boolean.rules_source_mut()["combat"]["practice"]["overwhelming_raw_points"] = json!(false);
    assert_has(&decode_error(&boolean), "invalid type: boolean");

    let mut negative = parts("first_room");
    negative.rules_source_mut()["combat"]["practice"]["fatal_blow_bonus_raw_points"] = json!(-1);
    assert_has(&decode_error(&negative), "invalid value");
}

#[test]
fn legacy_progression_rules_are_required_and_exact() {
    let mut missing = parts("character_sheet");
    missing
        .rules_source_mut()
        .as_object_mut()
        .expect("rules object")
        .remove("progression");
    assert_has(&decode_error(&missing), "missing field `progression`");

    let mut obsolete_formula = parts("character_sheet");
    obsolete_formula.rules_source_mut()["progression"]["formula"] = json!("level * 100");
    assert_has(&decode_error(&obsolete_formula), "unknown field `formula`");
}

#[test]
fn legacy_progression_threshold_semantic_matrix_is_preserved() {
    let mut too_few = parts("character_sheet");
    too_few.rules_source_mut()["progression"]["level_thresholds"] =
        json!([{"level": 1, "cumulative_experience": 0}]);
    assert_has(
        &definition_error(&too_few),
        "must contain at least two rows",
    );

    for (row, field, bad, expected) in [
        (0, "level", json!(0), "level must be positive"),
        (1, "level", json!(4), "level must be consecutive"),
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
        let mut invalid = parts("character_sheet");
        invalid.rules_source_mut()["progression"]["level_thresholds"][row][field] = bad;
        assert_has(&definition_error(&invalid), expected);
    }
}

#[test]
fn legacy_progression_threshold_rust_width_and_boolean_matrix_is_preserved() {
    for (row, field, bad) in [
        (0, "level", json!(true)),
        (0, "level", json_number("2147483648")),
        (0, "cumulative_experience", json!(true)),
        (
            0,
            "cumulative_experience",
            json_number("9223372036854775808"),
        ),
    ] {
        let mut invalid = parts("character_sheet");
        invalid.rules_source_mut()["progression"]["level_thresholds"][row][field] = bad;
        invalid
            .definition()
            .expect_err("threshold destination-width/type mismatch must fail");
    }
}

#[test]
fn legacy_progression_profiles_are_unique_and_cover_current_and_promotion_classes() {
    let mut duplicate = parts("knight_promotion");
    duplicate.rules_source_mut()["progression"]["growth_profiles"][1]["class_id"] =
        json!("fighter");
    assert_has(&definition_error(&duplicate), "class_id must be unique");

    let mut missing_current = parts("character_sheet");
    missing_current.rules_source_mut()["progression"]["growth_profiles"] = json!([]);
    missing_current.rules_source_mut()["movement"]["controlled_path_points"] = json!(4);
    assert_has(
        &seed_error(&missing_current),
        "must contain class_id \"fighter\"",
    );

    let mut missing_promotion = parts("knight_promotion");
    missing_promotion.rules_source_mut()["progression"]["growth_profiles"]
        .as_array_mut()
        .expect("growth profiles")
        .retain(|profile| profile["class_id"] != "knight");
    missing_promotion.rules_source_mut()["movement"]["controlled_path_points"] = json!(4);
    assert_has(
        &seed_error(&missing_promotion),
        "must contain class_id \"knight\"",
    );
}

#[test]
fn legacy_characterless_seed_accepts_empty_progression_profiles() {
    let mut characterless = parts("first_room");
    characterless.rules_source_mut()["progression"]["growth_profiles"] = json!([]);
    characterless.rules_source_mut()["movement"]["controlled_path_points"] = json!(4);
    characterless
        .validated_seed()
        .expect("a characterless seed needs no growth profile");
}

#[test]
fn legacy_progression_growth_rule_kind_matrix_is_preserved() {
    let mut fixed_hp = parts("character_sheet");
    fixed_hp.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"] =
        json!({"kind": "fixed", "outcomes": [{"amount": 1, "weight": 1}]});
    assert_has(&definition_error(&fixed_hp), "hit_points must use");

    let mut wrong_stamina_attribute = parts("character_sheet");
    wrong_stamina_attribute.rules_source_mut()["progression"]["growth_profiles"][0]["stamina_points"]
        ["attribute"] = json!("constitution");
    assert_has(
        &definition_error(&wrong_stamina_attribute),
        "stamina_points must use",
    );

    let mut wrong_hp_attribute = parts("character_sheet");
    wrong_hp_attribute.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]["attribute"] =
        json!("strength");
    assert_has(
        &definition_error(&wrong_hp_attribute),
        "hit_points must use",
    );

    let mut non_fixed_magic = parts("character_sheet");
    let hit_points =
        non_fixed_magic.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]
            .clone();
    non_fixed_magic.rules_source_mut()["progression"]["growth_profiles"][0]["magic_points"] =
        hit_points;
    assert_has(
        &definition_error(&non_fixed_magic),
        "magic_points must use kind fixed",
    );
}

#[test]
fn legacy_progression_attribute_band_order_matrix_is_preserved() {
    let mut nonzero_first = parts("character_sheet");
    nonzero_first.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]["bands"]
        [0]["minimum_attribute"] = json!(1);
    assert_has(
        &definition_error(&nonzero_first),
        "minimum_attribute must be zero",
    );

    let mut unordered = parts("character_sheet");
    unordered.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]["bands"][1]["minimum_attribute"] =
        json!(0);
    assert_has(
        &definition_error(&unordered),
        "minimum_attribute must be strictly increasing",
    );
}

#[test]
fn legacy_progression_outcome_amount_weight_and_overflow_matrix_is_preserved() {
    let mut zero_amount = parts("character_sheet");
    zero_amount.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]["bands"][0]
        ["outcomes"][0]["amount"] = json!(0);
    assert_has(&definition_error(&zero_amount), "amount must be positive");

    let mut duplicate_amount = parts("character_sheet");
    let first =
        duplicate_amount.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]["bands"][0]
            ["outcomes"][0]["amount"]
            .clone();
    duplicate_amount.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]["bands"]
        [0]["outcomes"][1]["amount"] = first;
    assert_has(
        &definition_error(&duplicate_amount),
        "amount must be unique",
    );

    let mut zero_weight = parts("character_sheet");
    zero_weight.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]["bands"][0]
        ["outcomes"][0]["weight"] = json!(0);
    assert_has(&definition_error(&zero_weight), "weight must be positive");

    let mut overflow = parts("character_sheet");
    overflow.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]["bands"][0]["outcomes"]
        [0]["weight"] = json!(u32::MAX);
    overflow.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]["bands"][0]["outcomes"]
        [1]["weight"] = json!(1);
    assert_has(
        &definition_error(&overflow),
        "weights must not overflow u32",
    );

    let mut boolean = parts("character_sheet");
    boolean.rules_source_mut()["progression"]["growth_profiles"][0]["hit_points"]["bands"][0]["outcomes"]
        [0]["weight"] = json!(true);
    assert_has(&decode_error(&boolean), "invalid type: boolean");
}

#[test]
fn legacy_progression_combat_add_order_range_and_positive_matrix_is_preserved() {
    let mut duplicate_level = parts("character_sheet");
    let first_level = duplicate_level.rules_source_mut()["progression"]["growth_profiles"][0]
        ["physical_attribute_adds_by_level"][0]["level"]
        .clone();
    duplicate_level.rules_source_mut()["progression"]["growth_profiles"][0]["physical_attribute_adds_by_level"]
        [1]["level"] = first_level;
    assert_has(&definition_error(&duplicate_level), "strictly ascending");

    let mut out_of_range = parts("character_sheet");
    out_of_range.rules_source_mut()["progression"]["growth_profiles"][0]["physical_attribute_adds_by_level"]
        [0]["level"] = json!(11);
    assert_has(
        &definition_error(&out_of_range),
        "within the authored threshold range",
    );

    let mut negative = parts("character_sheet");
    negative.rules_source_mut()["progression"]["growth_profiles"][0]["physical_attribute_adds_by_level"]
        [0]["strength_adds"] = json!(-1);
    assert_has(
        &definition_error(&negative),
        "additions must be non-negative",
    );

    let mut no_add = parts("character_sheet");
    let row = &mut no_add.rules_source_mut()["progression"]["growth_profiles"][0]["physical_attribute_adds_by_level"]
        [0];
    row["strength_adds"] = json!(0);
    row["dexterity_adds"] = json!(0);
    assert_has(
        &definition_error(&no_add),
        "must contain at least one positive addition",
    );
}

#[test]
fn legacy_actor_xp_value_is_monster_only_and_checked_to_i32() {
    let mut boolean = parts("character_sheet");
    boolean.actor_definition_mut(1)["xp_value"] = json!(true);
    assert_has(&decode_error(&boolean), "invalid type: boolean");

    let mut negative = parts("character_sheet");
    negative.actor_definition_mut(1)["xp_value"] = json!(-1);
    assert_has(&seed_error(&negative), "xp_value must be non-negative");

    let mut overflow = parts("character_sheet");
    overflow.actor_definition_mut(1)["xp_value"] = json_number("2147483648");
    overflow
        .validated_seed()
        .expect_err("xp_value must fit the Rust i32 destination");

    let mut player_xp = parts("character_sheet");
    player_xp.actor_definition_mut(0)["xp_value"] = json!(1);
    assert_has(
        &seed_error(&player_xp),
        "xp_value is only valid for monsters",
    );
}

#[test]
fn legacy_pending_progression_is_valid_but_level_is_bounded_by_authored_xp() {
    let mut pending = parts("character_sheet");
    pending.actors_mut()[0]["character"]["progression"] = json!({"level": 1, "experience": 600});
    pending
        .validated_seed()
        .expect("banked XP may leave a valid pending level");

    let mut ahead = parts("character_sheet");
    ahead.actors_mut()[0]["character"]["progression"] = json!({"level": 2, "experience": 0});
    assert_has(
        &seed_error(&ahead),
        "level must not exceed the XP-earned level",
    );

    let mut outside = parts("character_sheet");
    outside.actors_mut()[0]["character"]["progression"] = json!({"level": 11, "experience": 10000});
    assert_has(
        &seed_error(&outside),
        "level must be within authored threshold range",
    );
}

#[test]
fn legacy_peak_hp_and_living_resource_invariant_matrix_is_preserved() {
    let mut missing_peak = parts("character_sheet");
    missing_peak.actors_mut()[0]["character"]["resources"]
        .as_object_mut()
        .expect("resources")
        .remove("peak_hp");
    assert_has(&decode_error(&missing_peak), "missing field `peak_hp`");

    let mut low_peak = parts("character_sheet");
    low_peak.actors_mut()[0]["character"]["resources"]["peak_hp"] = json!(11);
    assert_has(&seed_error(&low_peak), "max_hp must not exceed peak_hp");

    for field in ["hp", "max_hp", "max_stamina"] {
        let mut zero = parts("character_sheet");
        zero.actors_mut()[0]["character"]["resources"][field] = json!(0);
        let error = seed_error(&zero);
        assert!(
            error.contains(field) || error.contains("hp and max_hp"),
            "{field}: {error}"
        );
    }
}

#[test]
fn legacy_zero_current_stamina_and_zero_max_mp_remain_valid() {
    let mut zero_current = parts("character_sheet");
    let resources = &mut zero_current.actors_mut()[0]["character"]["resources"];
    resources["stamina"] = json!(0);
    resources["mp"] = json!(0);
    resources["max_mp"] = json!(0);
    zero_current
        .validated_seed()
        .expect("zero current stamina and an absent MP pool remain valid");
}
