use std::collections::BTreeMap;

use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};

fn parts(case_id: &str) -> ContentParts {
    let profile = format!("profile/{case_id}");
    ContentParts::tracked(case_id, &profile)
}

fn definition_error(parts: &ContentParts) -> String {
    parts
        .definition()
        .expect_err("mutated definition must fail")
}

fn seed_error(parts: &ContentParts) -> String {
    parts.validated_seed().expect_err("mutated seed must fail")
}

fn decode_error(parts: &ContentParts) -> String {
    parts
        .decode()
        .expect_err("mutated graph must fail")
        .to_string()
}

fn assert_has(error: &str, expected: &str) {
    assert!(
        error.contains(expected),
        "expected {expected:?} in validation error: {error}"
    );
}

fn selected_index_by_id(parts: &ContentParts, registry: &str, id: &str) -> usize {
    parts.profile_value()[registry]
        .as_array()
        .expect("registry selection")
        .iter()
        .position(|key| {
            let key = key.as_str().expect("registry key");
            parts.catalog[registry][key]["id"] == id
        })
        .unwrap_or_else(|| panic!("selected {registry} row {id:?}"))
}

fn capability_index(
    parts: &ContentParts,
    service_id: &str,
    capability_kind: &str,
) -> (usize, usize) {
    let service_index = selected_index_by_id(parts, "service_definitions", service_id);
    let key = parts.profile_value()["service_definitions"][service_index]
        .as_str()
        .expect("service registry key");
    let capability_index = parts.catalog["service_definitions"][key]["capabilities"]
        .as_array()
        .expect("service capabilities")
        .iter()
        .position(|capability| capability["kind"] == capability_kind)
        .unwrap_or_else(|| panic!("{service_id:?} capability {capability_kind:?}"));
    (service_index, capability_index)
}

fn capability_mut<'a>(
    parts: &'a mut ContentParts,
    service_id: &str,
    capability_kind: &str,
) -> &'a mut Value {
    let (service_index, capability_index) = capability_index(parts, service_id, capability_kind);
    &mut parts.selected_mut("service_definitions", service_index)["capabilities"][capability_index]
}

#[test]
fn legacy_service_inventory_totals_are_exact_across_catalog_profiles() {
    let parts = parts("first_room");
    let profiles = parts.catalog["profiles"]
        .as_object()
        .expect("catalog profiles");
    let mut service_profiles = 0_usize;
    let mut service_definitions = 0_usize;
    let mut capability_kinds = BTreeMap::<String, usize>::new();

    for profile in profiles.values() {
        let selected = profile["service_definitions"]
            .as_array()
            .expect("profile service definitions");
        if !selected.is_empty() {
            service_profiles += 1;
        }
        service_definitions += selected.len();
        for key in selected {
            let key = key.as_str().expect("service registry key");
            for capability in parts.catalog["service_definitions"][key]["capabilities"]
                .as_array()
                .expect("service capabilities")
            {
                let kind = capability["kind"]
                    .as_str()
                    .expect("capability kind")
                    .to_string();
                *capability_kinds.entry(kind).or_default() += 1;
            }
        }
    }

    assert_eq!(service_profiles, 12);
    // One of these is the identity proof land's keeper, selected by
    // `profile/first_land_structure`: this catalog is the only definition
    // registry the project carries, so an authored land's service lives in it.
    assert_eq!(service_definitions, 17);
    assert_eq!(capability_kinds.values().sum::<usize>(), 35);
    assert_eq!(
        capability_kinds,
        BTreeMap::from([
            ("bank".to_string(), 4),
            ("class_promotion".to_string(), 2),
            ("item_service".to_string(), 2),
            ("locker".to_string(), 4),
            ("merchant".to_string(), 3),
            ("restoration".to_string(), 4),
            ("service_transaction".to_string(), 1),
            ("skill_critique".to_string(), 6),
            ("skill_training".to_string(), 6),
            ("spell_teaching".to_string(), 3),
        ])
    );
}

#[test]
fn legacy_restoration_operation_and_outcome_matrix_is_strict() {
    parts("restoration_services")
        .definition()
        .expect("current restoration definitions validate");

    let mut unknown = parts("restoration_services");
    capability_mut(&mut unknown, "tme_healer", "restoration")["operations"][0]["legacy"] =
        json!(true);
    assert_has(&decode_error(&unknown), "unknown field `legacy`");

    let mut resource = parts("restoration_services");
    capability_mut(&mut resource, "tme_healer", "restoration")["operations"][0]["outcome"]["resource"] =
        json!("focus");
    assert_has(&definition_error(&resource), "unknown variant `focus`");

    let mut status = parts("restoration_services");
    capability_mut(&mut status, "tme_healer", "restoration")["operations"][3]["outcome"]["status"] =
        json!("disease");
    assert_has(&definition_error(&status), "unknown variant `disease`");

    let mut kind = parts("restoration_services");
    capability_mut(&mut kind, "tme_healer", "restoration")["operations"][2]["outcome"] =
        json!({"kind": "full_recovery", "amount": 5});
    assert_has(&decode_error(&kind), "unknown variant `full_recovery`");

    let mut duplicate = parts("restoration_services");
    let first_id =
        capability_mut(&mut duplicate, "tme_healer", "restoration")["operations"][0]["transaction"]
            ["id"]
            .clone();
    capability_mut(&mut duplicate, "tme_healer", "restoration")["operations"][4]["transaction"]["id"] =
        first_id;
    assert_has(&definition_error(&duplicate), ".transaction.id duplicates");
}

#[test]
fn legacy_restoration_rewards_are_typed_and_priest_resurrection_is_free() {
    let mut reward = parts("restoration_services");
    capability_mut(&mut reward, "tme_healer", "restoration")["operations"][0]["transaction"]["rewards"] =
        json!([{"kind": "experience", "amount": 1}]);
    assert_has(
        &definition_error(&reward),
        "rewards must be empty for restoration",
    );

    let mut gold = parts("restoration_services");
    let priest =
        &mut capability_mut(&mut gold, "tme_temple", "restoration")["operations"][0]["transaction"];
    priest["requirements"] = json!([{"kind": "minimum_carried_gold", "amount": 1}]);
    priest["costs"] = json!([{"kind": "carried_gold", "amount": 1}]);
    let error = definition_error(&gold);
    assert_has(
        &error,
        "must not charge carried gold for priest_resurrection",
    );

    let mut item = parts("restoration_services");
    let priest =
        &mut capability_mut(&mut item, "tme_temple", "restoration")["operations"][0]["transaction"];
    priest["requirements"] = json!([{
        "kind": "carried_item",
        "item_definition_id": "clearwater_token",
        "quantity": 1
    }]);
    priest["costs"] = json!([{"kind": "selected_carried_item", "quantity": 1}]);
    assert_has(
        &definition_error(&item),
        "must not require or consume an item for priest_resurrection",
    );
}

#[test]
fn legacy_storage_definitions_and_service_references_are_strict() {
    parts("gold_bank_locker_storage")
        .validated_seed()
        .expect("current bank, vault, and service references validate");

    let mut cap = parts("gold_bank_locker_storage");
    cap.selected_mut("banks", 0)["transaction_cap_gold"] = json!(0);
    assert_has(
        &definition_error(&cap),
        "transaction_cap_gold must be positive",
    );

    let mut duplicate = parts("gold_bank_locker_storage");
    let mut row = duplicate.selected_mut("banks", 0).clone();
    row["transaction_cap_gold"] = json!(1);
    duplicate.push_selected("banks", "bank/duplicate/ei", row);
    assert_has(&definition_error(&duplicate), "already selected");

    let mut capacity = parts("gold_bank_locker_storage");
    capacity.selected_mut("locker_vaults", 0)["capacity"] = json!(0);
    assert_has(&definition_error(&capacity), "capacity must be positive");

    let mut unknown = parts("gold_bank_locker_storage");
    unknown.selected_mut("locker_vaults", 0)["legacy"] = json!(true);
    assert_has(&decode_error(&unknown), "unknown field `legacy`");

    let mut bank_ref = parts("gold_bank_locker_storage");
    capability_mut(&mut bank_ref, "west_counter", "bank")["bank_id"] = json!("missing_bank");
    assert_has(&definition_error(&bank_ref), "unknown bank");

    let mut vault_ref = parts("gold_bank_locker_storage");
    capability_mut(&mut vault_ref, "west_counter", "locker")["vault_id"] = json!("missing_vault");
    assert_has(&definition_error(&vault_ref), "unknown locker vault");
}

#[test]
fn legacy_bank_and_locker_capability_fields_are_exact() {
    let mut bank = parts("gold_bank_locker_storage");
    capability_mut(&mut bank, "west_counter", "bank")["transaction_cap_gold"] = json!(1);
    assert_has(&decode_error(&bank), "unknown field `transaction_cap_gold`");

    let mut locker = parts("gold_bank_locker_storage");
    capability_mut(&mut locker, "west_counter", "locker")
        .as_object_mut()
        .expect("locker capability")
        .remove("vault_id");
    assert_has(&decode_error(&locker), "missing field `vault_id`");
}

#[test]
fn legacy_merchant_stock_policy_and_item_boundaries_are_strict() {
    parts("merchant_item_services")
        .validated_seed()
        .expect("current merchant definitions and inventories validate");

    let mut empty = parts("merchant_item_services");
    empty.merchant_inventories_mut()[1]["stock"] = json!([]);
    assert_has(&seed_error(&empty), "stock must be non-empty");

    let mut duplicate = parts("merchant_item_services");
    duplicate.merchant_inventories_mut()[0]["stock"][1]["item_instance_id"] =
        duplicate.merchant_inventories_mut()[0]["stock"][0]["item_instance_id"].clone();
    assert_has(&seed_error(&duplicate), "unique within the inventory");

    let mut price = parts("merchant_item_services");
    price.merchant_inventories_mut()[0]["stock"][0]["price_gold"] = json!(0);
    assert_has(&seed_error(&price), "price_gold must be positive");

    let mut multiplier = parts("merchant_item_services");
    capability_mut(&mut multiplier, "crossroads_counter", "merchant")["player_sales"]["pawn_listing_multiplier"] =
        json!(true);
    assert_has(&decode_error(&multiplier), "expected u32");

    let mut tied = parts("merchant_item_services");
    tied.item_instances_mut()["copper_compass_stock"]["binding"] =
        json!({"state": "bind_on_first_character_touch"});
    assert_has(&seed_error(&tied), "must reference an unrestricted item");

    let mut no_sack = parts("merchant_item_services");
    no_sack.selected_by_runtime_id_mut("items", "trail_lantern")["valid_placements"] =
        json!(["hand"]);
    assert_has(&seed_error(&no_sack), "must permit sack placement");
}

#[test]
fn legacy_item_service_operations_are_strict_unique_and_repair_free() {
    let mut appraise = parts("merchant_item_services");
    capability_mut(&mut appraise, "crossroads_counter", "item_service")["operations"][0]["gold_cost"] =
        json!(1);
    assert_has(&decode_error(&appraise), "unknown field `gold_cost`");

    let mut identify = parts("merchant_item_services");
    capability_mut(&mut identify, "crossroads_counter", "item_service")["operations"][1]["gold_cost"] =
        json!(-1);
    assert_has(
        &definition_error(&identify),
        "gold_cost must be non-negative",
    );

    let mut enchant = parts("merchant_item_services");
    let operation =
        &mut capability_mut(&mut enchant, "crossroads_counter", "item_service")["operations"][2];
    operation["tags"] = json!(["z", "a", "a"]);
    operation["remaining_rounds"] = json!(0);
    let error = definition_error(&enchant);
    assert_has(&error, "tags must be sorted and unique");
    assert_has(&error, "remaining_rounds must be positive");

    let mut duplicate = parts("merchant_item_services");
    capability_mut(&mut duplicate, "crossroads_counter", "item_service")["operations"]
        .as_array_mut()
        .expect("item-service operations")
        .push(json!({"kind": "appraise"}));
    assert_has(
        &definition_error(&duplicate),
        ".kind must be unique within the capability",
    );

    let mut repair = parts("merchant_item_services");
    capability_mut(&mut repair, "crossroads_counter", "item_service")["operations"] =
        json!([{"kind": "repair"}]);
    assert_has(&decode_error(&repair), "unknown variant `repair`");
}

#[test]
fn legacy_service_definition_and_instance_shell_is_strict_and_position_checked() {
    for (field, replacement) in [
        ("kind", json!("trainer")),
        ("training_offers", json!([])),
        ("promotion", Value::Null),
        ("teaches_spells", json!([])),
    ] {
        let mut value = parts("spell_learning_purchase_casting_xp");
        value.selected_mut("service_definitions", 0)[field] = replacement;
        assert_has(&decode_error(&value), &format!("unknown field `{field}`"));
    }

    let mut missing_location = parts("spell_learning_purchase_casting_xp");
    missing_location.service_instances_mut()[0]
        .as_object_mut()
        .expect("service instance")
        .remove("location");
    assert_has(&decode_error(&missing_location), "missing field `location`");

    let mut missing_position = parts("spell_learning_purchase_casting_xp");
    missing_position.service_instances_mut()[0]["location"]
        .as_object_mut()
        .expect("service location")
        .remove("position");
    assert_has(&decode_error(&missing_position), "missing field `position`");

    let mut out_of_bounds = parts("spell_learning_purchase_casting_xp");
    out_of_bounds.service_instances_mut()[0]["location"]["position"] = json!({"x": 99, "y": 1});
    assert_has(&seed_error(&out_of_bounds), "out of bounds");

    let mut no_capabilities = parts("spell_learning_purchase_casting_xp");
    no_capabilities
        .selected_mut("service_definitions", 0)
        .as_object_mut()
        .expect("service definition")
        .remove("capabilities");
    assert_has(
        &decode_error(&no_capabilities),
        "missing field `capabilities`",
    );
}

#[test]
fn malformed_nested_service_types_return_errors_without_panicking() {
    let mut kind = parts("spell_learning_purchase_casting_xp");
    kind.selected_mut("service_definitions", 0)["capabilities"][0]["kind"] = json!([]);
    assert_has(&decode_error(&kind), "invalid type");

    let mut track = parts("spell_learning_purchase_casting_xp");
    capability_mut(&mut track, "wizard_trainer", "skill_training")["offers"][0]["track_id"] =
        json!([]);
    assert_has(&decode_error(&track), "invalid type");

    let mut reward = parts("knight_promotion");
    capability_mut(&mut reward, "knight_promoter", "class_promotion")["transaction"]["rewards"]
        [1]["item_definition_id"] = json!({});
    assert_has(&decode_error(&reward), "invalid type");

    let mut spells = parts("knight_promotion");
    spells.profile_value_mut()["spells"] = Value::Null;
    assert_has(&decode_error(&spells), "invalid type");
}

#[test]
fn obsolete_service_and_martial_skill_config_names_are_rejected() {
    let mut service = parts("gold_training");
    service.selected_mut("service_definitions", 0)["max_rank"] = json!(4);
    assert_has(&decode_error(&service), "unknown field `max_rank`");

    let mut martial = parts("martial_hand_block_actions");
    martial.selected_mut("profession_actions", 0)["martial_hand_block"]["min_hand_rank"] = json!(1);
    assert_has(
        &definition_error(&martial),
        "has unknown field: min_hand_rank",
    );

    let mut divisor = parts("martial_hand_block_actions");
    divisor.selected_mut("profession_actions", 0)["martial_hand_block"]["rank_divisor"] = json!(2);
    assert_has(
        &definition_error(&divisor),
        "has unknown field: rank_divisor",
    );
}

#[test]
fn legacy_training_focus_tracks_are_nonempty_unique_and_resolved() {
    let mut valid = parts("skill_progression");
    valid.push_selected(
        "items",
        "item/training_token/ei",
        json!({
            "id": "training_token",
            "kind": "tool",
            "name": "Training Token",
            "valid_placements": ["hand", "sack"],
            "capability": {"training_focus_for": ["sword"]},
            "economy": {"unit_burden": 1}
        }),
    );
    valid
        .definition()
        .expect("one resolved training focus track validates");

    let mut malformed = parts("skill_progression");
    malformed.push_selected(
        "items",
        "item/training_token/ei",
        json!({
            "id": "training_token",
            "kind": "tool",
            "name": "Training Token",
            "valid_placements": ["hand", "sack"],
            "capability": {"training_focus_for": ["sword", "sword", "", "missing"]},
            "economy": {"unit_burden": 1}
        }),
    );
    let error = definition_error(&malformed);
    assert_has(&error, "training_focus_for must not contain duplicates");
    assert_has(&error, "training_focus_for[2] must be non-empty");
    assert_has(&error, "references unknown skill catalog track");

    let mut empty = parts("skill_progression");
    empty.push_selected(
        "items",
        "item/training_token/ei",
        json!({
            "id": "training_token",
            "kind": "tool",
            "name": "Training Token",
            "valid_placements": ["hand", "sack"],
            "capability": {"training_focus_for": []},
            "economy": {"unit_burden": 1}
        }),
    );
    assert_has(
        &definition_error(&empty),
        "training_focus_for must not be empty",
    );
}

#[test]
fn malformed_transaction_numbers_duplicates_and_references_return_errors() {
    let mut boolean = parts("service_transactions");
    capability_mut(&mut boolean, "waystation_clerk", "service_transaction")["transactions"][0]["requirements"]
        [1]["level"] = json!(true);
    assert_has(&decode_error(&boolean), "invalid type");

    let mut overflow = parts("service_transactions");
    capability_mut(&mut overflow, "waystation_clerk", "service_transaction")["transactions"][0]["costs"]
        [0]["quantity"] = json!(u64::from(u32::MAX) + 1);
    assert_has(&decode_error(&overflow), "expected u32");

    let mut reference = parts("service_transactions");
    capability_mut(&mut reference, "waystation_clerk", "service_transaction")["transactions"][0]
        ["requirements"][3]["item_definition_id"] = json!("missing");
    assert_has(&definition_error(&reference), "unknown item definition");

    let mut duplicate = parts("service_transactions");
    let transaction =
        capability_mut(&mut duplicate, "waystation_clerk", "service_transaction")["transactions"]
            [0]
        .clone();
    capability_mut(&mut duplicate, "waystation_clerk", "service_transaction")["transactions"]
        .as_array_mut()
        .expect("transactions")
        .push(transaction);
    assert_has(
        &definition_error(&duplicate),
        "transactions[1].id duplicates",
    );
}

#[test]
fn selected_item_cost_requires_one_matching_carried_item_requirement() {
    let mut value = parts("service_transactions");
    capability_mut(&mut value, "waystation_clerk", "service_transaction")["transactions"][0]
        ["requirements"]
        .as_array_mut()
        .expect("requirements")
        .remove(3);
    assert_has(
        &definition_error(&value),
        "selected_carried_item cost requires one carried_item requirement",
    );
}

#[test]
fn restoration_allows_free_typed_outcome_but_generic_transaction_does_not() {
    let restoration = parts("restoration_services");
    let (service_index, capability_index) =
        capability_index(&restoration, "tme_healer", "restoration");
    let key = restoration.profile_value()["service_definitions"][service_index]
        .as_str()
        .expect("service key");
    let operation = &restoration.catalog["service_definitions"][key]["capabilities"]
        [capability_index]["operations"][2];
    assert_eq!(operation["transaction"]["costs"], json!([]));
    assert_eq!(operation["transaction"]["rewards"], json!([]));
    restoration
        .definition()
        .expect("free typed restoration outcome validates");

    let mut generic = parts("service_transactions");
    let transaction = &mut capability_mut(&mut generic, "waystation_clerk", "service_transaction")
        ["transactions"][0];
    transaction["costs"] = json!([]);
    transaction["rewards"] = json!([]);
    assert_has(
        &definition_error(&generic),
        "must contain at least one cost or reward",
    );
}
