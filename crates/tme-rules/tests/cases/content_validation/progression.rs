use super::*;

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
