use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};

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

fn huge_number(source: &str) -> Value {
    serde_json::from_str(source).expect("JSON number")
}

fn select_existing(parts: &mut ContentParts, registry: &str, key: &str) {
    assert!(
        parts.catalog[registry].get(key).is_some(),
        "registry row {key}"
    );
    parts.profile_value_mut()[registry]
        .as_array_mut()
        .unwrap_or_else(|| panic!("{registry} selection"))
        .push(json!(key));
}

#[test]
fn stable_character_ids_are_required_typed_nontransient_and_unique() {
    parts("character_sheet")
        .validated_seed()
        .expect("current stable character id must validate");

    let mut missing = parts("character_sheet");
    missing.actors_mut()[0]
        .as_object_mut()
        .unwrap()
        .remove("character_id");
    assert_has(
        &seed_error(&missing),
        "character_id is required when a character role is present",
    );

    let mut without_character = parts("first_room");
    without_character.actors_mut()[0]["character_id"] = json!("character:first_room:primary");
    assert_has(
        &seed_error(&without_character),
        "character_id is only valid when character is present",
    );

    let mut blank = parts("character_sheet");
    blank.actors_mut()[0]["character_id"] = json!("   ");
    assert_has(&seed_error(&blank), "character_id must be non-empty");

    let mut numeric = parts("character_sheet");
    numeric.actors_mut()[0]["character_id"] = json!(7);
    assert_has(&decode_error(&numeric), "invalid type");

    let mut transient = parts("character_sheet");
    transient.actors_mut()[0]["character_id"] = transient.actors_mut()[0]["id"].clone();
    assert_has(
        &seed_error(&transient),
        "character_id must differ from transient actor id",
    );

    let mut duplicate = parts("character_sheet");
    duplicate.actors_mut()[1]["character"] = duplicate.actors_mut()[0]["character"].clone();
    duplicate.actors_mut()[1]["character_id"] = duplicate.actors_mut()[0]["character_id"].clone();
    assert_has(
        &seed_error(&duplicate),
        "character_id duplicates actors[0].character_id",
    );
}

#[test]
fn carried_layout_positions_and_positioned_gold_are_exact() {
    let mut missing = parts("first_room");
    missing.actors_mut()[0]
        .as_object_mut()
        .unwrap()
        .remove("carried");
    assert_has(&decode_error(&missing), "missing field `carried`");

    let mut negative = parts("first_room");
    negative.actors_mut()[0]["carried"]["gold"]["sack"] = json!(-1);
    assert_has(&seed_error(&negative), "gold values must be non-negative");

    let mut obsolete_scalar = parts("first_room");
    obsolete_scalar.actors_mut()[0]["carried"]["sack_gold"] = json!(1);
    assert_has(&decode_error(&obsolete_scalar), "unknown field `sack_gold`");

    let mut missing_gold_position = parts("first_room");
    missing_gold_position.actors_mut()[0]["carried"]["gold"]
        .as_object_mut()
        .unwrap()
        .remove("left_hand");
    assert_has(
        &decode_error(&missing_gold_position),
        "missing field `left_hand`",
    );

    let mut item_and_gold = parts("first_room");
    item_and_gold.actors_mut()[0]["carried"]["gold"]["right_hand"] = json!(1);
    assert_has(
        &seed_error(&item_and_gold),
        "cannot place an item and gold in right_hand",
    );

    let mut gold_overflow = parts("first_room");
    gold_overflow.actors_mut()[0]["carried"]["gold"] =
        json!({"left_hand": i64::MAX, "right_hand": 1, "sack": 0});
    assert_has(
        &seed_error(&gold_overflow),
        "gold total must fit a signed 64-bit integer",
    );

    let mut invalid_position = parts("first_room");
    invalid_position.actors_mut()[0]["carried"]["items"][0]["position"] = json!("pack");
    assert_has(&decode_error(&invalid_position), "unknown variant `pack`");

    let mut incompatible = parts("first_room");
    incompatible.actors_mut()[0]["carried"]["items"][0]["position"] = json!("head");
    assert_has(
        &seed_error(&incompatible),
        "cannot occupy carried position \"head\"",
    );

    let mut old_character_gold = parts("character_sheet");
    old_character_gold.actors_mut()[0]["character"]["gold"] = json!(10);
    assert_has(&decode_error(&old_character_gold), "unknown field `gold`");

    let mut old_valid_slots = parts("first_room");
    old_valid_slots.selected_mut("items", 0)["capability"] = json!({"valid_slots": ["hands"]});
    assert_has(
        &decode_error(&old_valid_slots),
        "unknown field `valid_slots`",
    );

    let mut duplicate_position = parts("first_room");
    duplicate_position.item_instances_mut()["training_knife_second"] = json!({
        "definition_id": "training_knife",
        "binding": {"state": "unrestricted"}
    });
    duplicate_position.actors_mut()[0]["carried"]["items"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "item_instance_id": "training_knife_second",
            "position": "right_hand"
        }));
    assert_has(
        &seed_error(&duplicate_position),
        "position duplicates actors[0].carried.items[0].position",
    );
}

#[test]
fn item_instances_require_binding_definition_one_owner_and_safe_placement() {
    let mut missing_binding = parts("first_room");
    missing_binding.item_instances_mut()["training_knife"]
        .as_object_mut()
        .unwrap()
        .remove("binding");
    assert_has(&decode_error(&missing_binding), "missing field `binding`");

    let mut tied_stack = parts("first_room");
    tied_stack.item_instances_mut()["training_knife"]["quantity"] = json!(2);
    tied_stack.item_instances_mut()["training_knife"]["binding"] =
        json!({"state": "bind_on_first_character_touch"});
    tied_stack.actors_mut()[0]["carried"]["items"][0]["position"] = json!("sack_item_1");
    assert_has(
        &seed_error(&tied_stack),
        "quantity must be 1 for a tied item instance",
    );

    let mut unknown_definition = parts("first_room");
    unknown_definition.item_instances_mut()["training_knife"]["definition_id"] =
        json!("missing_definition");
    assert_has(
        &seed_error(&unknown_definition),
        "references unknown item definition",
    );

    for obsolete in ["inventory", "equipment"] {
        let mut old_actor_shape = parts("first_room");
        old_actor_shape.actors_mut()[0][obsolete] = json!([]);
        assert_has(
            &decode_error(&old_actor_shape),
            &format!("unknown field `{obsolete}`"),
        );
    }

    let mut dangling_ground = parts("first_room");
    dangling_ground
        .ground_items_mut()
        .as_array_mut()
        .unwrap()
        .push(json!({
            "item_instance_id": "missing_ground",
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 1, "y": 1}
            }
        }));
    assert_has(
        &seed_error(&dangling_ground),
        "references unknown item instance",
    );

    let mut multiply_owned = parts("first_room");
    multiply_owned
        .ground_items_mut()
        .as_array_mut()
        .unwrap()
        .push(json!({
            "item_instance_id": "training_knife",
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 1, "y": 1}
            }
        }));
    assert_has(
        &seed_error(&multiply_owned),
        "item instance \"training_knife\" is referenced more than once",
    );

    let mut orphan = parts("first_room");
    orphan.actors_mut()[0]["carried"]["items"] = json!([]);
    assert_has(
        &seed_error(&orphan),
        "item instance \"training_knife\" has no owner or location",
    );

    let mut zero_quantity = parts("first_room");
    zero_quantity.item_instances_mut()["training_knife"]["quantity"] = json!(0);
    assert_has(&seed_error(&zero_quantity), "quantity must be positive");

    let mut stacked_in_hand = parts("first_room");
    stacked_in_hand.item_instances_mut()["training_knife"]["quantity"] = json!(2);
    assert_has(
        &seed_error(&stacked_in_hand),
        "must have quantity 1 outside the sack",
    );

    let mut reserved = parts("first_room");
    let row = reserved.item_instances_mut()["training_knife"].clone();
    reserved
        .item_instances_mut()
        .as_object_mut()
        .unwrap()
        .remove("training_knife");
    reserved.item_instances_mut()["summon:training_knife"] = row;
    reserved.actors_mut()[0]["carried"]["items"][0]["item_instance_id"] =
        json!("summon:training_knife");
    assert_has(&seed_error(&reserved), "must not use reserved prefix");
}

#[test]
fn item_instance_values_integer_ranges_and_knowledge_are_checked_before_observation() {
    let mut stack_overflow = parts("first_room");
    stack_overflow.selected_mut("items", 0)["economy"]["unit_value_gold"] = json!(u64::MAX);
    stack_overflow.item_instances_mut()["training_knife"]["quantity"] = json!(2);
    stack_overflow.item_instances_mut()["training_knife"]["knowledge"] =
        json!({"identified": false, "appraised": false});
    stack_overflow.actors_mut()[0]["carried"]["items"][0]["position"] = json!("sack_item_1");
    assert_has(
        &seed_error(&stack_overflow),
        "quantity * unit_value_gold must not overflow",
    );

    let mut summon_overflow = parts("summons_created_creature_lifecycle");
    select_existing(
        &mut summon_overflow,
        "items",
        "item/restorative_tonic/item_instance_contract",
    );
    summon_overflow.catalog["items"]["item/restorative_tonic/item_instance_contract"]["economy"]
        ["unit_value_gold"] = json!(u64::MAX);
    let template = summon_overflow.selected_mut("summon_templates", 0);
    template["item_instances"] = json!({
        "tonic": {
            "definition_id": "restorative_tonic",
            "quantity": 2,
            "knowledge": {"identified": false, "appraised": false},
            "binding": {"state": "unrestricted"}
        }
    });
    template["carried"]["items"] =
        json!([{"item_instance_id": "tonic", "position": "sack_item_1"}]);
    assert_has(
        &definition_error(&summon_overflow),
        "quantity * unit_value_gold must be <= 18446744073709551615",
    );

    let mut burden_overflow = parts("first_room");
    burden_overflow.rules_source_mut()["burden"]["coin_burden_per_gold"] =
        huge_number("18446744073709551616");
    assert_has(&decode_error(&burden_overflow), "invalid type");

    let mut value_overflow = parts("first_room");
    value_overflow.selected_mut("items", 0)["economy"]["unit_value_gold"] =
        huge_number("18446744073709551616");
    assert_has(&decode_error(&value_overflow), "invalid type");

    let mut quantity_overflow = parts("first_room");
    quantity_overflow.item_instances_mut()["training_knife"]["quantity"] =
        huge_number("4294967296");
    assert_has(&decode_error(&quantity_overflow), "expected u32");

    let mut negative_value = parts("first_room");
    negative_value.selected_mut("items", 0)["economy"]["unit_value_gold"] = json!(-1);
    assert_has(&decode_error(&negative_value), "invalid value");

    let mut malformed_burden = parts("first_room");
    malformed_burden.selected_mut("items", 0)["economy"]["unit_burden"] = json!("heavy");
    assert_has(&decode_error(&malformed_burden), "invalid type");

    for (field, value) in [("identified", json!("yes")), ("appraised", json!(1))] {
        let mut malformed = parts("first_room");
        malformed.item_instances_mut()["training_knife"]["knowledge"][field] = value;
        assert_has(&decode_error(&malformed), "invalid type");
    }
}

#[test]
fn item_transform_output_references_an_existing_definition() {
    let mut missing_output = parts("utility_door_secret_item_spells");
    missing_output.selected_by_runtime_id_mut("spells", "shape_token")["effect"]["item_utility"]
        ["output_item_definition_id"] = json!("missing_output");
    assert_has(
        &definition_error(&missing_output),
        "references unknown item definition",
    );
}

#[test]
fn actor_overlap_and_line_of_sight_memory_are_explicit_seed_semantics() {
    let mut overlap = parts("first_room");
    overlap.actors_mut()[1]["location"]["position"] =
        overlap.actors_mut()[0]["location"]["position"].clone();
    overlap
        .validated_seed()
        .expect("actors may intentionally share one passable hex");

    let current = parts("monster_spellcasting_special_attacks");
    current
        .validated_seed()
        .expect("line-of-sight memory awareness is an accepted actor seed shape");
    let actor_index = current.world_seed["actors"]
        .as_array()
        .unwrap()
        .iter()
        .position(|actor| actor["id"] == "ember_imp")
        .expect("line-of-sight-memory actor");

    let mut zero_memory = current.clone();
    zero_memory.actor_definition_mut(actor_index)["ai"]["awareness"]["memory_opportunities"] =
        json!(0);
    assert_has(
        &seed_error(&zero_memory),
        "memory_opportunities must be positive",
    );

    let mut unknown_awareness = current.clone();
    unknown_awareness.actor_definition_mut(actor_index)["ai"]["awareness"]["extra"] = json!(true);
    assert_has(&decode_error(&unknown_awareness), "unknown field `extra`");

    let mut zero_leash = current;
    zero_leash.actor_definition_mut(actor_index)["ai"]["leash_range"] = json!(0);
    assert_has(&seed_error(&zero_leash), "leash_range must be positive");
}

#[test]
fn spell_book_definition_lane_binding_owner_and_quantity_are_exact() {
    parts("spell_learning_purchase_casting_xp")
        .validated_seed()
        .expect("current bound single Spell Book must validate");

    let mut wrong_lane = parts("spell_learning_purchase_casting_xp");
    wrong_lane.selected_by_runtime_id_mut("items", "spell_book")["capability"]["spell_book_for"] =
        json!(["wizard_magic", "knight_magic"]);
    assert_has(
        &definition_error(&wrong_lane),
        "must be wizard_magic, thaumaturge_magic, or thief_magic",
    );

    let mut unrestricted = parts("spell_learning_purchase_casting_xp");
    unrestricted.item_instances_mut()["spell_book"]["binding"] = json!({"state": "unrestricted"});
    assert_has(
        &seed_error(&unrestricted),
        "binding must be bound for a Spell Book",
    );

    let mut unknown_owner = parts("spell_learning_purchase_casting_xp");
    unknown_owner.item_instances_mut()["spell_book"]["binding"] =
        json!({"state": "bound", "character_id": "character:missing"});
    assert_has(
        &seed_error(&unknown_owner),
        "binding.character_id references no scenario character",
    );

    let mut stacked = parts("spell_learning_purchase_casting_xp");
    stacked.item_instances_mut()["spell_book"]["quantity"] = json!(2);
    assert_has(&seed_error(&stacked), "quantity must be 1 for a Spell Book");
}

#[test]
fn quest_npc_shapes_gates_rewards_and_outcome_links_are_strict() {
    parts("npc_quest_interactions")
        .validated_seed()
        .expect("current quest and NPC graph must validate");

    let mut missing_registry = parts("npc_quest_interactions");
    missing_registry
        .catalog
        .as_object_mut()
        .unwrap()
        .remove("quests");
    assert_has(&decode_error(&missing_registry), "missing field `quests`");

    let mut unknown_quest_field = parts("npc_quest_interactions");
    unknown_quest_field.selected_mut("quests", 0)["legacy_flag"] = json!(true);
    assert_has(
        &decode_error(&unknown_quest_field),
        "unknown field `legacy_flag`",
    );

    let mut duplicate_stage = parts("npc_quest_interactions");
    let first = duplicate_stage.selected_mut("quests", 0)["stages"][0].clone();
    duplicate_stage.selected_mut("quests", 0)["stages"]
        .as_array_mut()
        .unwrap()
        .push(first);
    assert_has(&definition_error(&duplicate_stage), "stages[3].id");

    let mut cadence = parts("npc_quest_interactions");
    cadence.actors_mut()[1]["npc"]["follow_cadence_units"] = json!(0);
    assert_has(
        &seed_error(&cadence),
        "follow_cadence_units must be positive",
    );

    let mut unknown_outcome = parts("npc_quest_interactions");
    unknown_outcome.actors_mut()[1]["npc"]["interactions"][0]["outcome"]["quest_flag"] =
        json!("started");
    assert_has(
        &decode_error(&unknown_outcome),
        "unknown field `quest_flag`",
    );

    let mut npc_on_monster = parts("npc_quest_interactions");
    npc_on_monster.actors_mut()[3]["npc"] = npc_on_monster.actors_mut()[1]["npc"].clone();
    assert_has(&seed_error(&npc_on_monster), "npc is only valid for NPCs");

    let mut unknown_quest = parts("npc_quest_interactions");
    unknown_quest.actors_mut()[1]["npc"]["interactions"][0]["transaction"]["requirements"][0]["quest_id"] =
        json!("missing");
    assert_has(&seed_error(&unknown_quest), "references unknown quest");

    let mut no_advance = parts("npc_quest_interactions");
    no_advance.actors_mut()[1]["npc"]["interactions"][1]["transaction"]["rewards"][0]["stage_id"] =
        json!("awaiting_token");
    assert_has(
        &seed_error(&no_advance),
        "stage_id must advance beyond its quest gate",
    );

    let mut no_accompanying_gate = parts("npc_quest_interactions");
    no_accompanying_gate.actors_mut()[2]["npc"]["interactions"][0]["transaction"]["requirements"]
        .as_array_mut()
        .unwrap()
        .retain(|requirement| requirement["kind"] != "npc_accompanying");
    assert_has(
        &seed_error(&no_accompanying_gate),
        "outcome requires a matching npc_accompanying gate",
    );

    let mut bad_direction = parts("npc_quest_interactions");
    bad_direction.actors_mut()[1]["npc"]["interactions"][2]["outcome"]["direction"] =
        json!("sideways");
    assert_has(&decode_error(&bad_direction), "unknown variant `sideways`");
}
