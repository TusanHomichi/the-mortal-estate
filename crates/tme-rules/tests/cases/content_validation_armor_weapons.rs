use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};

fn parts(case_id: &str) -> ContentParts {
    ContentParts::tracked(case_id, &format!("profile/{case_id}"))
}

fn rejection(parts: &ContentParts) -> String {
    parts
        .definition()
        .expect_err("the authored source mutation must be rejected")
}

fn assert_rejected(parts: &ContentParts, context: &str) -> String {
    let error = rejection(parts);
    assert!(!error.is_empty(), "{context} must return a diagnostic");
    error
}

fn assert_rejected_with(parts: &ContentParts, expected: &str, context: &str) {
    let error = assert_rejected(parts, context);
    assert!(
        error.contains(expected),
        "{context}: expected {expected:?} in diagnostic:\n{error}"
    );
}

fn armor_parts() -> ContentParts {
    parts("fidelity_gallery")
}

fn armor_mut(parts: &mut ContentParts) -> &mut Value {
    &mut parts.selected_by_runtime_id_mut("items", "leather_armor")["armor"]
}

fn weapon_parts() -> ContentParts {
    parts("first_room")
}

fn weapon_item_mut(parts: &mut ContentParts) -> &mut Value {
    parts.selected_by_runtime_id_mut("items", "training_knife")
}

fn weapon_mut(parts: &mut ContentParts) -> &mut Value {
    &mut weapon_item_mut(parts)["weapon"]
}

fn rules_source_mut(parts: &mut ContentParts) -> &mut Value {
    let key = parts.profile_value()["rules_profile"]
        .as_str()
        .expect("rules profile key")
        .to_string();
    &mut parts.catalog["rules_profiles"][&key]
}

#[test]
fn legacy_python_armor_test_canonical_typed_armor_is_accepted() {
    armor_parts()
        .definition()
        .expect("the canonical typed armor definition must be accepted");
}

#[test]
fn legacy_python_armor_test_armor_object_and_exact_fields_are_strict() {
    let mut non_object = armor_parts();
    *armor_mut(&mut non_object) = json!([]);
    assert_rejected_with(
        &non_object,
        "expected struct ArmorDef with 3 elements",
        "armor must be an object",
    );

    let mut unknown_armor = armor_parts();
    armor_mut(&mut unknown_armor)["legacy"] = json!(1);
    assert_rejected_with(
        &unknown_armor,
        "unknown field `legacy`",
        "armor rejects an unknown field",
    );

    let mut unknown_reduction = armor_parts();
    armor_mut(&mut unknown_reduction)["damage_reduction"]["magic"] = json!(1);
    assert_rejected_with(
        &unknown_reduction,
        "unknown field `magic`",
        "armor damage reduction rejects an unknown field",
    );
}

#[test]
fn legacy_python_armor_test_each_numeric_field_rejects_missing_boolean_negative_and_overflow() {
    let fields = [
        (None, "block_rating"),
        (None, "encumbrance"),
        (Some("damage_reduction"), "cutting"),
        (Some("damage_reduction"), "piercing"),
        (Some("damage_reduction"), "crushing"),
    ];
    let mutations = [
        ("missing", None),
        ("boolean", Some(json!(false))),
        ("negative", Some(json!(-1))),
        ("i32 overflow", Some(json!(2_147_483_648_i64))),
    ];

    for (parent, field) in fields {
        for (mutation, bad_value) in &mutations {
            let mut value = armor_parts();
            let owner = match parent {
                Some(parent) => &mut armor_mut(&mut value)[parent],
                None => armor_mut(&mut value),
            };
            match bad_value {
                Some(bad_value) => owner[field] = bad_value.clone(),
                None => {
                    owner
                        .as_object_mut()
                        .expect("numeric armor owner")
                        .remove(field);
                }
            }

            let path = parent
                .map(|parent| format!("{parent}.{field}"))
                .unwrap_or_else(|| field.to_string());
            let context = format!("armor.{path} rejects {mutation}");
            let error = assert_rejected(&value, &context);
            match *mutation {
                "missing" => assert!(error.contains("missing field"), "{context}: {error}"),
                "boolean" => assert!(
                    error.contains("invalid type: boolean"),
                    "{context}: {error}"
                ),
                "negative" => {
                    assert!(error.contains("must be non-negative"), "{context}: {error}")
                }
                "i32 overflow" => {
                    assert!(error.contains("expected i32"), "{context}: {error}")
                }
                _ => unreachable!(),
            }
        }
    }
}

#[test]
fn legacy_python_armor_test_zero_protection_and_non_object_reduction_are_rejected() {
    let mut zero = armor_parts();
    let armor = armor_mut(&mut zero);
    armor["block_rating"] = json!(0);
    armor["damage_reduction"] = json!({"cutting": 0, "piercing": 0, "crushing": 0});
    assert_rejected_with(
        &zero,
        "must provide block_rating or damage reduction",
        "zero-protection armor",
    );

    let mut non_object = armor_parts();
    armor_mut(&mut non_object)["damage_reduction"] = json!([]);
    assert_rejected_with(
        &non_object,
        "expected struct ArmorDamageReductionDef with 3 elements",
        "damage_reduction must be an object",
    );
}

#[test]
fn legacy_python_armor_test_armor_requires_a_worn_placement() {
    let mut value = armor_parts();
    value.selected_by_runtime_id_mut("items", "leather_armor")["valid_placements"] =
        json!(["hand", "sack"]);
    assert_rejected_with(
        &value,
        "requires at least one valid worn armor placement",
        "typed armor without a worn placement",
    );
}

#[test]
fn legacy_python_armor_test_weapon_and_consumable_cannot_also_be_armor() {
    let mut armor_source = armor_parts();
    let typed_armor = armor_mut(&mut armor_source).clone();

    let mut weapon = weapon_parts();
    let weapon_item = weapon_item_mut(&mut weapon);
    weapon_item["armor"] = typed_armor.clone();
    weapon_item["valid_placements"]
        .as_array_mut()
        .expect("weapon placements")
        .push(json!("outer_armor"));
    assert_rejected_with(
        &weapon,
        "armor is invalid for weapon items",
        "weapon with typed armor",
    );

    let mut consumable = parts("balm_cache");
    let consumable_item = consumable.selected_by_runtime_id_mut("items", "healing_balm");
    consumable_item["armor"] = typed_armor;
    consumable_item["valid_placements"]
        .as_array_mut()
        .expect("consumable placements")
        .push(json!("outer_armor"));
    assert_rejected_with(
        &consumable,
        "armor is invalid for consumable items",
        "consumable with typed armor",
    );
}

#[test]
fn legacy_python_armor_test_removed_scalar_armor_field_is_unknown() {
    let mut value = weapon_parts();
    weapon_item_mut(&mut value)["capability"] = json!({"armor_value": 2});
    assert_rejected_with(
        &value,
        "unknown field `armor_value`",
        "removed scalar armor capability",
    );
}

#[test]
fn legacy_python_armor_test_shield_can_remain_a_hand_blocker_without_typed_armor() {
    let mut value = weapon_parts();
    value.push_selected(
        "items",
        "item/test_shield/legacy_python_armor",
        json!({
            "id": "test_shield",
            "kind": "armor",
            "name": "Test Shield",
            "valid_placements": ["hand", "sack"],
            "economy": {"unit_burden": 1},
            "capability": {"block_value": 2}
        }),
    );
    value
        .definition()
        .expect("a hand-blocking shield does not require typed worn armor");
}

#[test]
fn legacy_python_weapons_test_scenario_26_is_the_only_accepted_schema() {
    for old_or_wrong_version in [0, 26] {
        let mut catalog = weapon_parts();
        catalog.catalog["schema_version"] = json!(old_or_wrong_version);
        assert_rejected_with(
            &catalog,
            "catalog.schema_version must be 6",
            "Catalog 1 rejects a non-current version",
        );

        let mut template = weapon_parts();
        template.world_template["schema_version"] = json!(old_or_wrong_version);
        assert_rejected_with(
            &template,
            "world_template.schema_version must be 3",
            "World Template 1 rejects a non-current version",
        );
    }
}

#[test]
fn legacy_python_weapons_test_weapon_object_is_required_exactly_for_weapons() {
    let mut missing = weapon_parts();
    weapon_item_mut(&mut missing)
        .as_object_mut()
        .expect("weapon item")
        .remove("weapon");
    assert_rejected_with(
        &missing,
        "weapon must be present for weapons",
        "weapon item without weapon definition",
    );

    let mut extra = weapon_parts();
    weapon_item_mut(&mut extra)["kind"] = json!("gear");
    assert_rejected_with(
        &extra,
        "weapon is only valid for weapons",
        "non-weapon item with weapon definition",
    );
}

#[test]
fn legacy_python_weapons_test_old_split_item_and_weapon_fields_are_unknown() {
    for (owner, field, value) in [
        ("item", "combat_add_rating", json!(1)),
        ("item", "attack_profile", json!("melee_in_hex")),
        ("item", "attack_range", json!(3)),
        ("item", "attack_cooldown_rounds", json!(1)),
        ("weapon", "attack_profile", json!("melee_in_hex")),
        ("weapon", "attack_range", json!(3)),
    ] {
        let mut parts = weapon_parts();
        match owner {
            "item" => weapon_item_mut(&mut parts)[field] = value,
            "weapon" => weapon_mut(&mut parts)[field] = value,
            _ => unreachable!(),
        }
        assert_rejected_with(
            &parts,
            &format!("unknown field `{field}`"),
            &format!("removed {owner}.{field}"),
        );
    }
}

#[test]
fn legacy_python_weapons_test_track_numeric_handedness_and_alignment_bounds_are_strict() {
    for (field, bad, expected) in [
        (
            "skill_track_id",
            json!(""),
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
        ("block_value", json!(false), "invalid type: boolean"),
        ("handedness", json!("future"), "unknown variant `future`"),
        (
            "required_alignment",
            json!("future"),
            "unknown variant `future`",
        ),
    ] {
        let mut value = weapon_parts();
        weapon_mut(&mut value)[field] = bad;
        assert_rejected_with(&value, expected, &format!("strict weapon field {field}"));
    }
}

#[test]
fn legacy_python_weapons_test_attack_modes_are_non_empty_unique_and_name_the_default() {
    let mut empty = weapon_parts();
    weapon_mut(&mut empty)["attack_modes"] = json!([]);
    assert_rejected_with(
        &empty,
        "attack_modes must not be empty",
        "empty weapon attack_modes",
    );

    let mut future = weapon_parts();
    weapon_mut(&mut future)["attack_modes"] =
        json!([{"mode": "future", "maximum_range": 0, "damage_kind": "cutting"}]);
    assert_rejected_with(
        &future,
        "unknown variant `future`",
        "unknown weapon attack mode",
    );

    let mut duplicate = weapon_parts();
    weapon_mut(&mut duplicate)["attack_modes"] = json!([
        {"mode": "fight", "maximum_range": 0, "damage_kind": "cutting"},
        {"mode": "fight", "maximum_range": 0, "damage_kind": "cutting"}
    ]);
    assert_rejected_with(
        &duplicate,
        "contains duplicate fight",
        "duplicate weapon attack mode",
    );

    let mut missing_default = weapon_parts();
    weapon_mut(&mut missing_default)["default_attack_mode"] = json!("throw");
    assert_rejected_with(
        &missing_default,
        "default_attack_mode must name an authored attack mode",
        "default mode absent from authored rows",
    );
}

#[test]
fn legacy_python_weapons_test_mode_rows_have_strict_fields_ranges_and_damage_kinds() {
    for (row, expected, context) in [
        (
            json!({"mode": "fight", "maximum_range": 1, "damage_kind": "cutting"}),
            "maximum_range must be 0 for fight",
            "fight range",
        ),
        (
            json!({"mode": "poke", "maximum_range": 2, "damage_kind": "piercing"}),
            "maximum_range must be 0 or 1 for poke",
            "poke range",
        ),
        (
            json!({"mode": "throw", "maximum_range": 0, "damage_kind": "piercing"}),
            "maximum_range must be positive for throw",
            "throw range",
        ),
        (
            json!({"mode": "fight", "maximum_range": false, "damage_kind": "cutting"}),
            "invalid type: boolean",
            "range type",
        ),
        (
            json!({"mode": "fight", "maximum_range": 0, "damage_kind": "future"}),
            "unknown variant `future`",
            "damage kind",
        ),
        (
            json!({"mode": "fight", "maximum_range": 0, "damage_kind": "cutting", "legacy": true}),
            "unknown field `legacy`",
            "mode row unknown field",
        ),
    ] {
        let mut value = weapon_parts();
        let mode = row["mode"].as_str().unwrap_or("fight").to_string();
        let weapon = weapon_mut(&mut value);
        weapon["default_attack_mode"] = json!(mode);
        weapon["attack_modes"] = json!([row]);
        assert_rejected_with(&value, expected, context);
    }
}

#[test]
fn legacy_python_weapons_test_fight_poke_and_throw_can_share_one_non_bow_weapon() {
    let mut value = weapon_parts();
    let weapon = weapon_mut(&mut value);
    weapon["default_attack_mode"] = json!("fight");
    weapon["attack_modes"] = json!([
        {"mode": "fight", "maximum_range": 0, "damage_kind": "cutting"},
        {"mode": "poke", "maximum_range": 1, "damage_kind": "piercing"},
        {"mode": "throw", "maximum_range": 3, "damage_kind": "piercing"}
    ]);
    value
        .definition()
        .expect("one non-bow weapon may author fight, poke, and throw in order");
}

#[test]
fn legacy_python_weapons_test_handedness_nocking_and_shoot_mode_are_coupled() {
    let mut bow_without_nocking = weapon_parts();
    let weapon = weapon_mut(&mut bow_without_nocking);
    weapon["handedness"] = json!("bow");
    weapon["default_attack_mode"] = json!("shoot");
    weapon["attack_modes"] =
        json!([{"mode": "shoot", "maximum_range": 3, "damage_kind": "piercing"}]);
    weapon.as_object_mut().unwrap().remove("nocking");
    assert_rejected_with(
        &bow_without_nocking,
        "nocking must be present for bows",
        "bow without nocking",
    );

    let mut non_bow_nocking = weapon_parts();
    weapon_mut(&mut non_bow_nocking)["nocking"] = json!({"unloads_on_movement": true});
    assert_rejected_with(
        &non_bow_nocking,
        "nocking is only valid for bows",
        "non-bow with nocking",
    );

    let mut non_bow_shoot = weapon_parts();
    let weapon = weapon_mut(&mut non_bow_shoot);
    weapon["default_attack_mode"] = json!("shoot");
    weapon["attack_modes"] =
        json!([{"mode": "shoot", "maximum_range": 3, "damage_kind": "piercing"}]);
    assert_rejected_with(
        &non_bow_shoot,
        "shoot mode is only valid for bows",
        "non-bow with shoot mode",
    );

    let mut bad_nocking_type = weapon_parts();
    let weapon = weapon_mut(&mut bad_nocking_type);
    weapon["handedness"] = json!("bow");
    weapon["default_attack_mode"] = json!("shoot");
    weapon["attack_modes"] =
        json!([{"mode": "shoot", "maximum_range": 3, "damage_kind": "piercing"}]);
    weapon["nocking"] = json!({"unloads_on_movement": "yes"});
    assert_rejected_with(
        &bad_nocking_type,
        "invalid type: string",
        "bow nocking boolean type",
    );
}

#[test]
fn legacy_python_weapons_test_old_capability_weapon_and_class_fields_are_rejected() {
    for (field, bad) in [
        ("hands_required", json!(1)),
        ("skill_track_id", json!("sword")),
        ("class_restrict_allow", json!(["fighter"])),
        ("class_restrict_deny", json!(["wizard"])),
    ] {
        let mut value = weapon_parts();
        let mut capability = serde_json::Map::new();
        capability.insert(field.to_string(), bad);
        weapon_item_mut(&mut value)["capability"] = Value::Object(capability);
        assert_rejected_with(
            &value,
            &format!("unknown field `{field}`"),
            &format!("removed capability.{field}"),
        );
    }
}

#[test]
fn legacy_python_weapons_test_combat_selection_and_fumble_bounds_are_strict() {
    for (section, field, bad, expected) in [
        (
            "block",
            "left_hand_selection_percent",
            json!(0),
            "integer in 1..=100",
        ),
        ("fumble", "base_percent", json!(0), "integer in 1..=100"),
        ("fumble", "minimum_percent", json!(6), "in 1..=base_percent"),
        (
            "fumble",
            "skill_levels_per_reduction",
            json!(0),
            "must be positive",
        ),
    ] {
        let mut value = weapon_parts();
        rules_source_mut(&mut value)["combat"][section][field] = bad;
        assert_rejected_with(&value, expected, &format!("combat.{section}.{field}"));
    }
}

#[test]
fn legacy_python_weapons_test_deleted_hide_class_restriction_flag_is_unknown() {
    let mut value = parts("profession_specific_actions");
    value.selected_by_runtime_id_mut("profession_actions", "thief_hide")["hide"]["disallow_class_restricted_equipment"] =
        json!(true);
    assert_rejected_with(
        &value,
        "hide has unknown field: disallow_class_restricted_equipment",
        "removed hide class-restriction flag",
    );
}
