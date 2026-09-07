use super::*;

#[test]
fn summon_template_death_items_ownership_and_spell_metadata_are_exact() {
    let current = parts("summons_created_creature_lifecycle");
    current
        .definition()
        .expect("current summon template metadata must validate");

    let mut with_item = parts("summons_created_creature_lifecycle");
    select_registry_row_by_runtime_id(&mut with_item, "items", "healing_balm");
    let template = with_item.selected_mut("summon_templates", 0);
    template["item_instances"] = json!({
        "focus": {
            "definition_id": "healing_balm",
            "binding": {"state": "unrestricted"}
        }
    });
    template["carried"]["items"] = json!([{"item_instance_id": "focus", "position": "right_hand"}]);
    with_item
        .definition()
        .expect("summon-owned item with one carried location must validate");

    let mut missing_death = parts("summons_created_creature_lifecycle");
    missing_death
        .summon_actor_definition_mut(0)
        .as_object_mut()
        .unwrap()
        .remove("death");
    assert_has(&decode_error(&missing_death), "missing field `death`");

    let mut bad_ownership = parts("summons_created_creature_lifecycle");
    select_registry_row_by_runtime_id(&mut bad_ownership, "items", "healing_balm");
    let template = bad_ownership.selected_mut("summon_templates", 0);
    template["item_instances"] = json!({
        "owned_twice": {
            "definition_id": "healing_balm",
            "binding": {"state": "unrestricted"}
        },
        "orphan": {
            "definition_id": "healing_balm",
            "binding": {"state": "unrestricted"}
        },
        "stacked": {
            "definition_id": "healing_balm",
            "quantity": 2,
            "binding": {"state": "unrestricted"}
        }
    });
    template["carried"]["items"] = json!([
        {"item_instance_id": "owned_twice", "position": "sack_item_1"},
        {"item_instance_id": "missing", "position": "sack_item_2"},
        {"item_instance_id": "owned_twice", "position": "right_hand"},
        {"item_instance_id": "stacked", "position": "left_hand"}
    ]);
    let error = definition_error(&bad_ownership);
    assert_has(&error, "references unknown item instance");
    assert_has(
        &error,
        "item instance \"owned_twice\" is referenced more than once",
    );
    assert_has(&error, "item instance \"orphan\" has no owner or location");
    assert_has(&error, "must have quantity 1 outside the sack");

    let mut bad_template = parts("summons_created_creature_lifecycle");
    bad_template.selected_mut("summon_templates", 0)["id"] = json!("");
    let template_definition = bad_template.summon_actor_definition_mut(0);
    template_definition["social"]["owner_relation"] = json!("none");
    template_definition["ai"]["cadence_units"] = json!(0);
    let error = definition_error(&bad_template);
    assert_has(&error, "summon_templates[0].id must be non-empty");
    assert_has(
        &error,
        "owner_relation must be summoner for a summon template",
    );
    assert_has(&error, "ai.cadence_units must be positive");

    let mut bad_ai = parts("summons_created_creature_lifecycle");
    bad_ai.summon_actor_definition_mut(0)["ai"]["behavior"] = json!("bad_ai");
    assert_has(&decode_error(&bad_ai), "unknown variant `bad_ai`");

    let mut missing_template_ref = parts("summons_created_creature_lifecycle");
    missing_template_ref.selected_by_runtime_id_mut("spells", "call_echo")["effect"]["summon_actor_id"] =
        json!("missing_template");
    assert_has(
        &definition_error(&missing_template_ref),
        "is not a summon_templates id",
    );

    let mut wrong_target = parts("summons_created_creature_lifecycle");
    wrong_target.selected_by_runtime_id_mut("spells", "call_echo")["target"]["kind"] =
        json!("actor");
    assert_has(
        &definition_error(&wrong_target),
        "target.kind must be coordinate for summon spells",
    );

    let mut missing_duration = parts("summons_created_creature_lifecycle");
    missing_duration.selected_by_runtime_id_mut("spells", "call_echo")["effect"]
        .as_object_mut()
        .unwrap()
        .remove("duration");
    assert_has(
        &definition_error(&missing_duration),
        "effect.duration must be present for summon spells",
    );
}

#[test]
fn quest_summon_profession_and_storage_definitions_retain_domain_validation() {
    let mut quest = parts("npc_quest_interactions");
    quest.selected_mut("quests", 0)["stages"][1]["id"] =
        quest.selected_mut("quests", 0)["stages"][0]["id"].clone();
    assert_has(&definition_error(&quest), "stages[1].id");

    let mut summon = parts("summons_created_creature_lifecycle");
    summon.selected_mut("summon_templates", 0)["id"] = json!(" ");
    assert_has(
        &definition_error(&summon),
        "summon_templates[0].id must be non-empty",
    );

    let mut summon_ai = parts("summons_created_creature_lifecycle");
    summon_ai.summon_actor_definition_mut(0)["ai"]["cadence_units"] = json!(0);
    assert_has(&definition_error(&summon_ai), "cadence_units");

    let mut profession = parts("profession_specific_actions");
    profession.selected_mut("profession_actions", 0)["id"] = json!(" ");
    assert_has(&definition_error(&profession), "profession_actions[0].id");

    let mut bank = parts("gold_bank_locker_storage");
    bank.selected_mut("banks", 0)["transaction_cap_gold"] = json!(0);
    assert_has(&definition_error(&bank), "transaction_cap_gold");

    let mut vault = parts("gold_bank_locker_storage");
    vault.selected_mut("locker_vaults", 0)["capacity"] = json!(0);
    assert_has(&definition_error(&vault), "capacity");
}

#[test]
fn actor_seed_identity_roles_social_ai_stats_and_room_placement_are_validated() {
    let mut duplicate = parts("first_room");
    duplicate.actors_mut()[1]["id"] = duplicate.actors_mut()[0]["id"].clone();
    assert_has(&seed_error(&duplicate), "duplicates actors[0].id");

    let mut no_player = parts("first_room");
    let monster_social = no_player.actor_definition_mut(1)["social"].clone();
    let monster_ai = no_player.actor_definition_mut(1)["ai"].clone();
    no_player.actor_definition_mut(0)["kind"] = json!("monster");
    no_player.actor_definition_mut(0)["social"] = monster_social;
    no_player.actor_definition_mut(0)["ai"] = monster_ai;
    assert_has(&seed_error(&no_player), "at least one player");

    let mut player_ai = parts("first_room");
    player_ai.actor_definition_mut(0)["ai"] = player_ai.actor_definition_mut(1)["ai"].clone();
    assert_has(&definition_error(&player_ai), "ai is forbidden for players");

    let mut missing_monster_ai = parts("first_room");
    missing_monster_ai.actor_definition_mut(1)["ai"] = Value::Null;
    assert_has(
        &definition_error(&missing_monster_ai),
        "ai is required for monsters",
    );

    let mut bad_social = parts("first_room");
    bad_social.actor_definition_mut(0)["social"]["owner_relation"] = json!("summoner");
    assert_has(&definition_error(&bad_social), "owner_relation");

    let mut bad_stats = parts("first_room");
    bad_stats.actor_definition_mut(1)["stats"]["hp"] = json!(0);
    assert_has(&definition_error(&bad_stats), "stats must use positive HP");

    let mut blocked = parts("first_room");
    blocked.actors_mut()[0]["location"]["position"] = json!({"x": 0, "y": 0});
    assert_has(&seed_error(&blocked), "not traversable");

    let mut unknown_room = parts("first_room");
    unknown_room.actors_mut()[0]["location"]["level"] = json!("missing");
    assert_has(
        &seed_error(&unknown_room),
        "realm/level does not exist in the selected world template",
    );
}

#[test]
fn actor_social_authority_enforcer_ai_and_summoner_matrices_are_exact() {
    let authoritative = parts("alignment_social_law");
    authoritative
        .validated_seed()
        .expect("player, lawful NPC, and monster social shapes");
    let authored_summon = parts("summons_created_creature_lifecycle");
    authored_summon
        .validated_seed()
        .expect("summoner-owned inherent summon shape");

    let player = actor_seed_index(&authoritative, "player");
    let enforcer = actor_seed_index(&authoritative, "oath_watch");
    let civilian = actor_seed_index(&authoritative, "harbor_warden");
    let monster = actor_seed_index(&authoritative, "storm_shade");

    let mut characterless_authority = authoritative.clone();
    characterless_authority.actors_mut()[player]
        .as_object_mut()
        .expect("player")
        .remove("character");
    characterless_authority.actors_mut()[player]
        .as_object_mut()
        .expect("player")
        .remove("character_id");
    assert_has(
        &seed_error(&characterless_authority),
        "alignment_source character requires a character-backed actor",
    );

    let mut duplicate_authority = authoritative.clone();
    duplicate_authority.actor_definition_mut(player)["social"]["alignment_source"] =
        json!({"kind": "inherent", "alignment": "lawful"});
    assert_has(
        &seed_error(&duplicate_authority),
        "alignment_source must be character for a character-backed actor",
    );

    let mut stray_social = authoritative.clone();
    stray_social.actor_definition_mut(player)["social"]["team"] = json!("heroes");
    assert_has(&decode_error(&stray_social), "unknown field `team`");

    let mut stray_authority = authoritative.clone();
    stray_authority.actor_definition_mut(enforcer)["social"]["alignment_source"]["karma"] =
        json!(1);
    assert_has(&decode_error(&stray_authority), "unknown field `karma`");

    let mut invalid_enforcer = authoritative.clone();
    invalid_enforcer.actor_definition_mut(enforcer)["social"]["alignment_source"]["alignment"] =
        json!("neutral");
    assert_has(
        &seed_error(&invalid_enforcer),
        "town_enforcer requires an inherent-lawful human NPC",
    );

    let mut missing_npc_ai = authoritative.clone();
    missing_npc_ai.actor_definition_mut(civilian)["ai"] = Value::Null;
    assert_has(
        &seed_error(&missing_npc_ai),
        "ai is required for an inherent-lawful human NPC",
    );

    let mut forbidden_npc_ai = authoritative.clone();
    forbidden_npc_ai.actor_definition_mut(civilian)["social"] = json!({
        "alignment_source": {"kind": "inherent", "alignment": "neutral"},
        "nature": "human",
        "behavior": "passive",
        "owner_relation": "none"
    });
    assert_has(
        &seed_error(&forbidden_npc_ai),
        "ai is valid on an NPC only for an inherent-lawful human",
    );

    let mut ordinary_summoner = authoritative.clone();
    ordinary_summoner.actor_definition_mut(monster)["social"]["owner_relation"] = json!("summoner");
    assert_has(
        &seed_error(&ordinary_summoner),
        "summoner is valid only for summon templates",
    );

    let mut summon_character_authority = authored_summon.clone();
    summon_character_authority.summon_actor_definition_by_template_id_mut("echo_guardian")["social"]
        ["alignment_source"] = json!({"kind": "character"});
    assert_has(
        &definition_error(&summon_character_authority),
        "alignment_source must be inherent for a summon template",
    );

    let mut summon_without_relation = authored_summon;
    summon_without_relation.summon_actor_definition_by_template_id_mut("echo_guardian")["social"]
        ["owner_relation"] = json!("none");
    assert_has(
        &definition_error(&summon_without_relation),
        "owner_relation must be summoner for a summon template",
    );
}

#[test]
fn spell_social_hostility_and_town_law_follow_effect_and_target_semantics() {
    let mut harmful_not_hostile = parts("spell_effects");
    harmful_not_hostile.selected_by_runtime_id_mut("spells", "spark")["social"]["hostile_act"] =
        json!(false);
    assert_has(
        &definition_error(&harmful_not_hostile),
        "social.hostile_act must be true for the current effect family and target",
    );

    let mut harmful_terrain_law = parts("spell_effects");
    harmful_terrain_law.selected_by_runtime_id_mut("spells", "spark")["social"]["town_law"] =
        json!("terrain_alignment_violation");
    assert_has(
        &definition_error(&harmful_terrain_law),
        "terrain_alignment_violation requires a terrain, darkness, or light effect family",
    );

    let terrain_law = parts("alignment_social_law");
    terrain_law
        .definition()
        .expect("light effect may carry terrain-alignment law");
}

#[test]
fn law_zone_character_alignment_and_legacy_social_shapes_are_strict() {
    let mut town = parts("first_room");
    town.template_levels_source_mut()["room_0"]["law_zone"] = json!("town");
    town.definition().expect("town is an authored law zone");

    let mut missing_zone = parts("first_room");
    missing_zone.template_levels_source_mut()["room_0"]
        .as_object_mut()
        .expect("room")
        .remove("law_zone");
    assert_has(&decode_error(&missing_zone), "missing field `law_zone`");

    let mut invalid_zone = parts("first_room");
    invalid_zone.template_levels_source_mut()["room_0"]["law_zone"] = json!("city");
    assert_has(&decode_error(&invalid_zone), "unknown variant `city`");

    let mut allegiance = parts("alignment_social_law");
    let player = actor_seed_index(&allegiance, "player");
    allegiance.actors_mut()[player]["allegiance"] = json!("player");
    assert_has(&decode_error(&allegiance), "unknown field `allegiance`");

    let mut missing_social = parts("alignment_social_law");
    missing_social
        .actor_definition_mut(player)
        .as_object_mut()
        .expect("player")
        .remove("social");
    assert_has(&decode_error(&missing_social), "missing field `social`");

    let mut missing_alignment = parts("alignment_social_law");
    missing_alignment.actors_mut()[player]["character"]
        .as_object_mut()
        .expect("character")
        .remove("alignment_state");
    assert_has(
        &decode_error(&missing_alignment),
        "missing field `alignment_state`",
    );

    let mut invalid_alignment = parts("alignment_social_law");
    invalid_alignment.actors_mut()[player]["character"]["alignment_state"]["alignment"] =
        json!("saintly");
    assert_has(
        &decode_error(&invalid_alignment),
        "unknown variant `saintly`",
    );

    for karma in [json!(-1), json!(false), json!(u64::from(u32::MAX) + 1)] {
        let mut value = parts("alignment_social_law");
        value.actors_mut()[player]["character"]["alignment_state"]["karma_points"] = karma;
        assert_has(&decode_error(&value), "expected u32");
    }

    let mut obsolete_status = parts("alignment_social_law");
    obsolete_status.actors_mut()[player]["character"]["knighthood_state"] =
        json!({"knighted": false});
    assert_has(
        &decode_error(&obsolete_status),
        "unknown field `knighthood_state`",
    );
}

#[test]
fn character_and_starter_seed_domains_validate_identity_progression_resources_and_skills() {
    let mut missing_id = parts("character_sheet");
    missing_id.actors_mut()[0]
        .as_object_mut()
        .unwrap()
        .remove("character_id");
    assert_has(&seed_error(&missing_id), "character_id is required");

    let mut transient_id = parts("character_sheet");
    transient_id.actors_mut()[0]["character_id"] = transient_id.actors_mut()[0]["id"].clone();
    assert_has(
        &seed_error(&transient_id),
        "must differ from transient actor id",
    );

    let mut bad_attribute = parts("character_sheet");
    bad_attribute.actors_mut()[0]["character"]["attributes"]["strength"] = json!(19);
    assert_has(&seed_error(&bad_attribute), "must be between 3 and 18");

    let mut bad_resource = parts("character_sheet");
    bad_resource.actors_mut()[0]["character"]["resources"]["hp"] = json!(99);
    assert_has(&seed_error(&bad_resource), "must not exceed max_hp");

    let mut bad_level = parts("character_sheet");
    bad_level.actors_mut()[0]["character"]["progression"]["level"] = json!(2);
    assert_has(
        &seed_error(&bad_level),
        "must not exceed the XP-earned level",
    );

    let mut starter = parts("starter_circuit");
    starter.actors_mut()[0]["starter_character"]["creation"]["current_class_id"] = json!("missing");
    assert_has(&seed_error(&starter), "current_class_id");
}
