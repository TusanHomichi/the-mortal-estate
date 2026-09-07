use super::*;

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
