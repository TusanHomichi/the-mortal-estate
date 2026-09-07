use super::*;

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
