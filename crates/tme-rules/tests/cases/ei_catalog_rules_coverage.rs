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

fn selected_item<'a>(parts: &'a mut ContentParts, id: &str) -> &'a mut Value {
    parts.selected_by_runtime_id_mut("items", id)
}

fn object_with(key: &str, value: Value) -> Value {
    let mut object = serde_json::Map::new();
    object.insert(key.to_string(), value);
    Value::Object(object)
}

#[test]
fn legacy_currency_items_and_zero_coin_burden_remain_rejected() {
    let mut currency = parts("first_room");
    selected_item(&mut currency, "training_knife")["kind"] = json!("currency");
    assert_has(
        &definition_error(&currency),
        "kind must not be currency; use carried.gold",
    );

    let mut zero_coin_burden = parts("first_room");
    zero_coin_burden.rules_source_mut()["burden"]["coin_burden_per_gold"] = json!(0);
    assert_has(
        &definition_error(&zero_coin_burden),
        "coin_burden_per_gold must be positive",
    );
}

#[test]
fn legacy_actor_and_item_capability_alias_fields_have_no_route() {
    let mut actor_alias = parts("first_room");
    actor_alias.actors_mut()[0]["weapon_id"] = json!("training_knife");
    assert_has(&decode_error(&actor_alias), "unknown field `weapon_id`");

    let mut capability_alias = parts("first_room");
    selected_item(&mut capability_alias, "training_knife")["capability"] = json!({"value_gold": 1});
    assert_has(
        &decode_error(&capability_alias),
        "unknown field `value_gold`",
    );
}

#[test]
fn legacy_canonical_damage_label_set_is_still_required() {
    let mut incomplete = parts("first_room");
    let one_label = incomplete.profile_value()["damage_labels"][0].clone();
    incomplete.profile_value_mut()["damage_labels"] = json!([one_label]);
    assert_has(
        &definition_error(&incomplete),
        "damage_labels must declare exactly these ids",
    );
}

#[test]
fn legacy_rules_root_and_movement_burden_resource_children_are_required_and_strict() {
    let mut missing_rules = parts("first_room");
    missing_rules
        .profile_value_mut()
        .as_object_mut()
        .expect("profile object")
        .remove("rules_profile");
    assert_has(
        &decode_error(&missing_rules),
        "missing field `rules_profile`",
    );

    let mut previous = parts("first_room");
    previous.catalog["schema_version"] = json!(0);
    assert_has(
        &definition_error(&previous),
        "catalog.schema_version must be 6",
    );

    for required in ["movement", "burden", "resources"] {
        let mut missing = parts("first_room");
        missing
            .rules_source_mut()
            .as_object_mut()
            .expect("rules object")
            .remove(required);
        let error = decode_error(&missing);
        assert_has(&error, "missing field");
        assert_has(&error, required);
    }

    for removed in ["player_round_budget", "monster_round_budget"] {
        let mut obsolete = parts("first_room");
        obsolete.rules_source_mut()["movement"][removed] = json!(3);
        assert_has(
            &decode_error(&obsolete),
            &format!("unknown field `{removed}`"),
        );
    }

    for removed in ["recovery", "regeneration"] {
        let mut obsolete = parts("first_room");
        obsolete.world_seed[removed] = json!({"interval": 2});
        assert_has(
            &decode_error(&obsolete),
            &format!("unknown field `{removed}`"),
        );
    }
}

#[test]
fn legacy_resource_and_burden_tuning_matrix_is_preserved() {
    for field in [
        "recovery_interval_units",
        "active_hp_recovery",
        "inactive_hp_recovery",
        "inactive_stamina_recovery",
        "mp_recovery",
        "normal_movement_stamina_cost",
        "rapid_movement_stamina_cost",
    ] {
        let mut invalid = parts("first_room");
        invalid.rules_source_mut()["resources"][field] = json!(0);
        assert_has(
            &definition_error(&invalid),
            &format!("resources.{field} must be positive"),
        );
    }

    let mut inactive_not_greater = parts("first_room");
    inactive_not_greater.rules_source_mut()["resources"]["inactive_hp_recovery"] = json!(1);
    assert_has(
        &definition_error(&inactive_not_greater),
        "inactive_hp_recovery must be greater than active_hp_recovery",
    );

    let mut rapid_not_greater = parts("first_room");
    rapid_not_greater.rules_source_mut()["resources"]["rapid_movement_stamina_cost"] = json!(1);
    assert_has(
        &definition_error(&rapid_not_greater),
        "rapid_movement_stamina_cost must be greater than normal_movement_stamina_cost",
    );

    for (field, bad) in [
        ("lightly_loaded_max_per_strength", json!(0)),
        ("moderately_loaded_max_per_strength", json!(10_000)),
        ("heavily_loaded_max_per_strength", json!(20_000)),
    ] {
        let mut invalid = parts("first_room");
        invalid.rules_source_mut()["burden"][field] = bad;
        assert_has(&definition_error(&invalid), &format!("burden.{field}"));
    }

    let mut overflow = parts("character_sheet");
    overflow.rules_source_mut()["burden"] = json!({
        "coin_burden_per_gold": 1,
        "lightly_loaded_max_per_strength": u64::MAX - 2,
        "moderately_loaded_max_per_strength": u64::MAX - 1,
        "heavily_loaded_max_per_strength": u64::MAX
    });
    assert_has(
        &seed_error(&overflow),
        "effective character strength must not overflow",
    );
}

#[test]
fn legacy_item_economy_is_required_typed_and_complete() {
    let mut missing_economy = parts("first_room");
    selected_item(&mut missing_economy, "training_knife")
        .as_object_mut()
        .expect("item object")
        .remove("economy");
    assert_has(&decode_error(&missing_economy), "missing field `economy`");

    let mut missing_burden = parts("first_room");
    selected_item(&mut missing_burden, "training_knife")["economy"] = json!({});
    assert_has(
        &decode_error(&missing_burden),
        "missing field `unit_burden`",
    );

    let mut wrong_burden = parts("first_room");
    selected_item(&mut wrong_burden, "training_knife")["economy"]["unit_burden"] = json!("heavy");
    assert_has(&decode_error(&wrong_burden), "invalid type: string");
}

#[test]
fn legacy_movement_budget_type_value_and_unknown_field_matrix_is_preserved() {
    let mut non_positive = parts("first_room");
    non_positive.rules_source_mut()["movement"]["controlled_path_points"] = json!(0);
    non_positive.rules_source_mut()["movement"]["automatic_step_points"] = json!(-1);
    let error = definition_error(&non_positive);
    assert_has(&error, "controlled_path_points must be positive");
    assert_has(&error, "automatic_step_points must be positive");

    let mut boolean = parts("first_room");
    boolean.rules_source_mut()["movement"]["controlled_path_points"] = json!(true);
    assert_has(&decode_error(&boolean), "invalid type: boolean");

    let mut obsolete = parts("first_room");
    obsolete.rules_source_mut()["movement"]["speed"] = json!(3);
    assert_has(&decode_error(&obsolete), "unknown field `speed`");
}

#[test]
fn legacy_explicit_poke_shoot_and_throw_weapon_modes_remain_valid() {
    let mut fight_and_poke = parts("thrown_attack");
    let weapon = &mut selected_item(&mut fight_and_poke, "oak_javelin")["weapon"];
    weapon["default_attack_mode"] = json!("poke");
    weapon["attack_modes"] = json!([
        {"mode": "fight", "maximum_range": 0, "damage_kind": "cutting"},
        {"mode": "poke", "maximum_range": 1, "damage_kind": "piercing"}
    ]);
    fight_and_poke
        .definition()
        .expect("an explicit fight/poke weapon is valid");

    parts("ranged_attack")
        .definition()
        .expect("the explicit bow shoot mode is valid");
    parts("thrown_attack")
        .definition()
        .expect("the explicit throw mode is valid");
}

#[test]
fn legacy_weapon_mode_ranges_duplicates_defaults_damage_and_removed_fields_are_strict() {
    let mut missing_range = parts("thrown_attack");
    selected_item(&mut missing_range, "oak_javelin")["weapon"]["attack_modes"][1]
        .as_object_mut()
        .expect("attack mode")
        .remove("maximum_range");
    assert_has(
        &decode_error(&missing_range),
        "missing field `maximum_range`",
    );

    let mut fight_range = parts("thrown_attack");
    selected_item(&mut fight_range, "oak_javelin")["weapon"]["attack_modes"][0]["maximum_range"] =
        json!(3);
    assert_has(&definition_error(&fight_range), "must be 0 for fight");

    let mut obsolete_range = parts("first_room");
    selected_item(&mut obsolete_range, "training_knife")["attack_range"] = json!(3);
    assert_has(
        &decode_error(&obsolete_range),
        "unknown field `attack_range`",
    );

    let mut duplicate = parts("thrown_attack");
    selected_item(&mut duplicate, "oak_javelin")["weapon"]["attack_modes"][1]["mode"] =
        json!("fight");
    assert_has(&definition_error(&duplicate), "contains duplicate fight");

    let mut missing_default = parts("thrown_attack");
    selected_item(&mut missing_default, "oak_javelin")["weapon"]["default_attack_mode"] =
        json!("poke");
    assert_has(
        &definition_error(&missing_default),
        "default_attack_mode must name an authored attack mode",
    );

    let mut bad_damage = parts("thrown_attack");
    selected_item(&mut bad_damage, "oak_javelin")["weapon"]["attack_modes"][0]["damage_kind"] =
        json!("future");
    assert_has(&decode_error(&bad_damage), "unknown variant `future`");

    let mut removed_profile = parts("ranged_attack");
    selected_item(&mut removed_profile, "elm_bow")["weapon"]["attack_profile"] =
        json!("ranged_limited");
    assert_has(
        &decode_error(&removed_profile),
        "unknown field `attack_profile`",
    );
}

#[test]
fn legacy_healing_consumable_positive_and_semantic_value_matrix_is_preserved() {
    parts("balm_cache")
        .definition()
        .expect("canonical healing consumables are valid");

    let mut bad_effect = parts("balm_cache");
    selected_item(&mut bad_effect, "healing_balm")["consumable"]["effect"] = json!("haste");
    assert_has(&definition_error(&bad_effect), "effect must be healing");

    let mut zero_healing = parts("balm_cache");
    selected_item(&mut zero_healing, "healing_balm")["consumable"]["heal_per_round"] = json!(0);
    assert_has(
        &definition_error(&zero_healing),
        "heal_per_round must be positive",
    );

    let mut boolean_healing = parts("balm_cache");
    selected_item(&mut boolean_healing, "healing_balm")["consumable"]["heal_per_round"] =
        json!(true);
    assert_has(&decode_error(&boolean_healing), "invalid type: boolean");
}

#[test]
fn legacy_consumable_presence_kind_and_object_shape_matrix_is_preserved() {
    let mut missing = parts("balm_cache");
    selected_item(&mut missing, "healing_balm")["consumable"] = Value::Null;
    assert_has(
        &definition_error(&missing),
        "consumable must be present for consumables",
    );

    let mut on_gear = parts("balm_cache");
    selected_item(&mut on_gear, "healing_balm")["kind"] = json!("gear");
    assert_has(
        &definition_error(&on_gear),
        "consumable is only valid for consumables",
    );

    let mut non_object = parts("balm_cache");
    selected_item(&mut non_object, "healing_balm")["consumable"] = json!([]);
    assert_has(&decode_error(&non_object), "expected struct ConsumableDef");

    let mut unknown = parts("balm_cache");
    selected_item(&mut unknown, "healing_balm")["consumable"]["potency"] = json!(9);
    assert_has(&decode_error(&unknown), "unknown field `potency`");
}

#[test]
fn legacy_item_capability_unknown_fields_and_semantic_matrix_is_preserved() {
    for removed in [
        "armor_value",
        "valid_slots",
        "hands_required",
        "skill_track_id",
        "class_restrict_allow",
        "class_restrict_deny",
    ] {
        let mut invalid = parts("first_room");
        selected_item(&mut invalid, "training_knife")["capability"] =
            object_with(removed, json!(1));
        assert_has(
            &decode_error(&invalid),
            &format!("unknown field `{removed}`"),
        );
    }

    let mut consumable_block = parts("balm_cache");
    selected_item(&mut consumable_block, "healing_balm")["capability"] = json!({"block_value": 1});
    assert_has(
        &definition_error(&consumable_block),
        "block_value is invalid for consumable items",
    );

    let mut negative_block = parts("first_room");
    selected_item(&mut negative_block, "training_knife")["capability"] = json!({"block_value": -1});
    assert_has(
        &definition_error(&negative_block),
        "block_value must be >= 0",
    );

    let mut empty_taxonomy = parts("first_room");
    selected_item(&mut empty_taxonomy, "training_knife")["capability"] = json!({"taxonomy_id": ""});
    assert_has(
        &definition_error(&empty_taxonomy),
        "taxonomy_id must not be empty",
    );

    for field in ["attribute_adds", "resource_adds"] {
        let mut empty_stat = parts("first_room");
        selected_item(&mut empty_stat, "training_knife")["capability"] =
            object_with(field, json!([{"stat": "", "value": 1}]));
        assert_has(
            &definition_error(&empty_stat),
            &format!("{field}[0].stat must not be empty"),
        );
    }
}

#[test]
fn legacy_item_capability_accepts_typed_resistance_boosts() {
    let value = parts("status_effects");
    value
        .definition()
        .expect("the typed resistance-boost capability remains valid");
    let mut inspected = value.clone();
    assert_eq!(
        selected_item(&mut inspected, "steady_charm")["capability"]["resistance_boosts"],
        json!([{"tag": "stun", "bonus_twentieths": 3}])
    );
}

fn set_mp_recovery_item(
    parts: &mut ContentParts,
    numerator: u32,
    denominator: u32,
    evidence_state: &str,
    placements: Value,
) {
    let item = selected_item(parts, "steady_charm");
    item["valid_placements"] = placements;
    item["capability"] = json!({
        "mp_recovery_multiplier": {
            "numerator": numerator,
            "denominator": denominator,
            "evidence_state": evidence_state
        }
    });
}

#[test]
fn legacy_mp_recovery_multiplier_contract_matrix_is_preserved() {
    let mut valid = parts("status_effects");
    set_mp_recovery_item(
        &mut valid,
        3,
        2,
        "original_provisional",
        json!(["hand", "sack", "neck"]),
    );
    valid
        .definition()
        .expect("a reduced non-decreasing worn MP multiplier is valid");

    for (numerator, denominator, evidence, placements, expected) in [
        (
            1,
            2,
            "original_provisional",
            json!(["neck"]),
            "must not reduce MP recovery",
        ),
        (
            4,
            2,
            "original_provisional",
            json!(["neck"]),
            "must be reduced",
        ),
        (
            2,
            1,
            "target_release",
            json!(["neck"]),
            "evidence_state must be original_provisional",
        ),
        (
            2,
            1,
            "original_provisional",
            json!(["hand", "sack"]),
            "requires a worn valid placement",
        ),
        (
            u32::MAX,
            1,
            "original_provisional",
            json!(["neck"]),
            "result exceeds supported range",
        ),
    ] {
        let mut invalid = parts("status_effects");
        set_mp_recovery_item(&mut invalid, numerator, denominator, evidence, placements);
        assert_has(&definition_error(&invalid), expected);
    }
}
