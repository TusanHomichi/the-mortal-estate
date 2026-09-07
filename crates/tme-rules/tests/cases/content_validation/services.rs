use super::*;

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
    blocked.service_instances_mut()[0]["placement"]["location"]["position"] =
        json!({"x": 0, "y": 0});
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
