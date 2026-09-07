use super::*;

#[test]
fn catalog_six_scavenging_profiles_are_strict_bounded_and_referentially_complete() {
    const PROFILE: &str = "scavenging/original_provisional";
    for (field, value, expected) in [
        ("search_radius", json!(7), "search_radius must be at most 6"),
        (
            "balm_below_hp_percent",
            json!(0),
            "balm_below_hp_percent must be between 1 and 100",
        ),
        (
            "balm_chance_denominator",
            json!(0),
            "balm_chance must be a valid fraction",
        ),
        (
            "balm_chance_numerator",
            json!(5),
            "balm_chance must be a valid fraction",
        ),
    ] {
        let mut parts = parts("town_adventure_loop_gallery");
        parts.catalog["scavenging_profiles"][PROFILE][field] = value;
        if field == "balm_chance_numerator" {
            parts.catalog["scavenging_profiles"][PROFILE]["balm_chance_denominator"] = json!(4);
        }
        assert_has(&definition_error(&parts), expected);
    }

    let mut zero_radius = parts("town_adventure_loop_gallery");
    zero_radius.catalog["scavenging_profiles"][PROFILE]["search_radius"] = json!(0);
    assert_has(
        &definition_error(&zero_radius),
        "search_radius must be nonzero when scavenging is enabled",
    );

    let mut balm_dependency = parts("town_adventure_loop_gallery");
    balm_dependency.catalog["scavenging_profiles"][PROFILE]["collects_ground_items"] = json!(false);
    assert_has(
        &definition_error(&balm_dependency),
        "uses_healing_balm requires ground collection and equipping",
    );

    let mut unknown_reference = parts("town_adventure_loop_gallery");
    unknown_reference.actor_definition_by_actor_id_mut("road_scavenger")["scavenging_profile_id"] =
        json!("scavenging/missing");
    assert_has(
        &definition_error(&unknown_reference),
        "scavenging_profile_id references unknown selected profile",
    );

    let mut unknown_field = parts("town_adventure_loop_gallery");
    unknown_field.catalog["scavenging_profiles"][PROFILE]["legacy_radius"] = json!(6);
    assert_has(
        &decode_error(&unknown_field),
        "unknown field `legacy_radius`",
    );
}

#[test]
fn every_tracked_graph_enters_the_same_checked_definition_and_seed_seams() {
    assert_eq!(TRACKED_CASES.len(), 52);
    for case_id in TRACKED_CASES {
        let parts = parts(case_id);
        let seed = parts
            .validated_seed()
            .unwrap_or_else(|error| panic!("{case_id}: {error}"));
        Engine::new(seed, 7).unwrap_or_else(|error| panic!("{case_id}: {error}"));
    }
}

#[test]
fn structure_only_player_and_npc_seed_is_valid_without_monsters_or_ecology() {
    let mut parts = parts("npc_quest_interactions");
    parts
        .actors_mut()
        .as_array_mut()
        .expect("seed actors")
        .retain(|actor| actor["id"] != "watch_sentinel");

    let actors = parts.world_seed["actors"].as_array().expect("seed actors");
    assert_eq!(actors.len(), 3);
    assert_eq!(
        actors
            .iter()
            .map(|actor| actor["id"].as_str().expect("actor id"))
            .collect::<Vec<_>>(),
        ["player", "wayfinder", "watchkeeper"]
    );
    assert!(actors[0]["npc"].is_null());
    assert!(actors[1]["npc"].is_object());
    assert!(actors[2]["npc"].is_object());
    assert!(
        parts.world_seed["ecology_sites"]
            .as_array()
            .expect("ecology sites")
            .is_empty()
    );

    let seed = parts
        .validated_seed()
        .expect("player-plus-NPC seed without monsters or ecology");
    Engine::new(seed, 7).expect("structure-only engine");
}

#[test]
fn four_contract_decoders_are_strict_and_core_documents_reject_scripts() {
    let mut unknown_catalog = parts("first_room");
    unknown_catalog.catalog["unexpected"] = json!(true);
    assert_has(&decode_error(&unknown_catalog), "unknown field");

    let mut missing_catalog = parts("first_room");
    missing_catalog
        .catalog
        .as_object_mut()
        .unwrap()
        .remove("profiles");
    assert_has(&decode_error(&missing_catalog), "missing field");

    let mut catalog_script = parts("first_room");
    catalog_script.catalog["script"] = json!([]);
    assert_has(&decode_error(&catalog_script), "unknown field `script`");

    let mut unknown_template = parts("first_room");
    unknown_template.world_template["unexpected"] = json!(true);
    assert_has(&decode_error(&unknown_template), "unknown field");

    let mut missing_template = parts("first_room");
    missing_template
        .world_template
        .as_object_mut()
        .unwrap()
        .remove("realms");
    assert_has(&decode_error(&missing_template), "missing field");

    let mut template_script = parts("first_room");
    template_script.world_template["script"] = json!([]);
    assert_has(&decode_error(&template_script), "unknown field `script`");

    let mut unknown_seed = parts("first_room");
    unknown_seed.world_seed["unexpected"] = json!(true);
    assert_has(&decode_error(&unknown_seed), "unknown field");

    let mut missing_seed = parts("first_room");
    missing_seed
        .world_seed
        .as_object_mut()
        .unwrap()
        .remove("actors");
    assert_has(&decode_error(&missing_seed), "missing field");

    let mut seed_script = parts("first_room");
    seed_script.world_seed["script"] = json!([]);
    assert_has(&decode_error(&seed_script), "unknown field `script`");
}

#[test]
fn catalog_envelope_profile_and_registry_identity_are_validated_atomically() {
    let mut wrong_schema = parts("first_room");
    wrong_schema.catalog["schema_version"] = json!(1);
    assert_has(
        &definition_error(&wrong_schema),
        "catalog.schema_version must be 6",
    );

    let mut wrong_kind = parts("first_room");
    wrong_kind.catalog["kind"] = json!("scenario");
    assert_has(
        &definition_error(&wrong_kind),
        "catalog.kind must be \"catalog\"",
    );

    let mut empty_id = parts("first_room");
    empty_id.catalog["id"] = json!(" ");
    assert_has(&definition_error(&empty_id), "catalog.id must be non-empty");

    let mut unresolved = parts("first_room");
    unresolved.profile_value_mut()["items"][0] = json!("item/missing");
    assert_has(
        &definition_error(&unresolved),
        "references unknown registry key",
    );

    let mut exact_duplicate = parts("first_room");
    let row = selected_row(&exact_duplicate, "items", 0);
    exact_duplicate.catalog["items"]["item/exact_duplicate"] = row;
    assert_has(&definition_error(&exact_duplicate), "exactly duplicates");

    let mut runtime_collision = parts("first_room");
    let mut row = selected_row(&runtime_collision, "items", 0);
    row["name"] = json!("Different canonical row with same runtime id");
    runtime_collision.push_selected("items", "item/runtime_collision", row);
    assert_has(&definition_error(&runtime_collision), "already selected");

    let mut duplicate_profile = parts("first_room");
    duplicate_profile.catalog["profiles"]["profile/exact_duplicate"] =
        duplicate_profile.profile_value().clone();
    assert_has(&definition_error(&duplicate_profile), "exactly duplicates");

    let mut missing_profile = parts("first_room");
    missing_profile.catalog_profile = "profile/missing".to_string();
    assert_has(
        &definition_error(&missing_profile),
        "does not exist in catalog.profiles",
    );
}

#[test]
fn catalog_selection_preserves_profile_order_and_hides_unselected_rows() {
    let spell_parts = parts("spell_effects");
    let expected = spell_parts.profile_value()["spells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|key| {
            spell_parts.catalog["spells"][key.as_str().unwrap()]["id"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect::<Vec<_>>();
    let (catalog, _, _, _) = spell_parts.decode().unwrap();
    let selected = catalog
        .select(&CatalogProfileKey::new("profile/spell_effects").unwrap())
        .unwrap();
    assert_eq!(
        selected
            .spells
            .iter()
            .map(|spell| spell.id.clone())
            .collect::<Vec<_>>(),
        expected
    );

    let mut unselected = parts("first_room");
    unselected.catalog["items"]["item/unselected_bad"] = json!({
        "id": "",
        "kind": "gear",
        "name": "Unselected",
        "valid_placements": [],
        "consumable": null,
        "economy": {"unit_burden": 0},
        "review_note": null
    });
    unselected
        .definition()
        .expect("unselected definitions are invisible to assembled semantics");
}

#[test]
fn simulation_seed_three_actor_definition_and_ecology_joins_fail_closed() {
    let current = parts("creature_ecology_gallery");
    current
        .validated_seed()
        .expect("current creature ecology seed must validate");

    let mut missing_ecology_sites = parts("first_room");
    missing_ecology_sites
        .world_seed
        .as_object_mut()
        .unwrap()
        .remove("ecology_sites");
    assert_has(
        &decode_error(&missing_ecology_sites),
        "missing field `ecology_sites`",
    );

    let mut seed_override = parts("first_room");
    seed_override.actors_mut()[0]["name"] = json!("Override");
    assert_has(&decode_error(&seed_override), "unknown field `name`");

    let mut unknown_definition = parts("first_room");
    unknown_definition.actors_mut()[1]["actor_definition_id"] = json!("missing");
    assert_has(
        &seed_error(&unknown_definition),
        "references unknown or unselected actor definition",
    );

    let mut unknown_source = current.clone();
    unknown_source.world_seed["ecology_sites"][0]["source"]["spawn_group_id"] = json!("missing");
    assert_has(
        &seed_error(&unknown_source),
        "source references unknown or unselected ecology definition",
    );

    let mut missing_member = current.clone();
    missing_member.world_seed["ecology_sites"][0]["member_locations"]
        .as_object_mut()
        .unwrap()
        .remove("keeper");
    assert_has(
        &seed_error(&missing_member),
        "member_locations keys must exactly equal spawn-group members",
    );

    let mut blocked_member = current.clone();
    blocked_member.world_seed["ecology_sites"][0]["member_locations"]["runner"]["position"] =
        json!({"x": 0, "y": 0});
    assert_has(&seed_error(&blocked_member), "not traversable");

    let mut collision = current;
    collision.actors_mut()[0]["id"] = json!("ecology:gallery_pack:runner:0");
    assert_has(
        &seed_error(&collision),
        "generation-zero actor ID collides with an explicit actor",
    );
}

#[test]
fn recursive_boundary_scanning_covers_catalog_template_seed_keys_and_values() {
    // The terms below come from tests/fixtures/synthetic-terms.txt, the tracked
    // nonsense denylist that .cargo/config.toml configures for cargo-run
    // processes. They prove the REJECTION MECHANISM without the tree carrying a
    // real term. Point TME_BANNED_TERMS_FILE at a different list and this
    // assertion stops holding — by construction, not by defect: a tree that
    // carries no real term cannot write a fixture the real list rejects.
    let mut catalog_value = parts("first_room");
    catalog_value.selected_mut("items", 0)["name"] = json!("zorbelquux blade");
    assert_has(&definition_error(&catalog_value), "banned source term");

    let mut catalog_key = parts("first_room");
    catalog_key.catalog["items"]["item/zorbelquux/private"] =
        selected_row(&catalog_key, "items", 0);
    assert_has(&definition_error(&catalog_key), "banned source term");

    let mut template = parts("first_room");
    template.template_levels_source_mut()["room_0"]["cells"][1][1][0] =
        json!("TME-PLACEHOLDER floor");
    assert_has(&definition_error(&template), "TME-PLACEHOLDER");

    let mut seed = parts("first_room");
    seed.actor_definition_mut(0)["name"] = json!("quendaraff pilgrim");
    assert_has(&seed_error(&seed), "banned source term");
}

#[test]
fn marked_internal_policy_allows_marked_values_but_boundary_pairs_remain_exact() {
    let mut marked = parts("first_room");
    marked.catalog["clean_content"] = json!(false);
    marked.catalog["research_boundary"] = json!({
        "status": "internal_parity_fixture",
        "review_refs": ["Slice EI test"],
        "notes": "TME-PLACEHOLDER internal parity fixture"
    });
    marked.actor_definition_mut(0)["name"] = json!("zorbelquux reference");
    marked
        .validated_seed()
        .expect("marked graph may retain marked source strings");

    let mut mismatched = parts("first_room");
    mismatched.catalog["clean_content"] = json!(false);
    assert_has(&definition_error(&mismatched), "must select exactly");
}
