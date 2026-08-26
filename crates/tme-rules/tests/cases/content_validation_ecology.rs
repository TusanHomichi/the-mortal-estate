use crate::support::content_parts::ContentParts;
use serde_json::json;

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

fn decode_error(parts: &ContentParts) -> String {
    parts.decode().expect_err("strict decode must fail")
}

fn assert_has(error: &str, expected: &str) {
    assert!(
        error.contains(expected),
        "expected {expected:?} in diagnostic:\n{error}"
    );
}

#[test]
fn creature_definition_affinity_loot_spawn_and_lair_contracts_fail_closed() {
    let current = parts("creature_ecology_gallery");
    current
        .definition()
        .expect("current creature definition graph must validate");

    let mut duplicate_affinity_kind = current.clone();
    duplicate_affinity_kind.selected_mut("physical_damage_affinity_profiles", 0)["responses"][1]
        ["damage_kind"] = json!("cutting");
    let error = definition_error(&duplicate_affinity_kind);
    assert_has(&error, "damage_kind must be unique");
    assert_has(&error, "responses is missing piercing");

    let mut zero_affinity_denominator = current.clone();
    zero_affinity_denominator.selected_mut("physical_damage_affinity_profiles", 1)["responses"]
        [0]["denominator"] = json!(0);
    assert_has(
        &definition_error(&zero_affinity_denominator),
        "denominator must be positive",
    );

    let mut unknown_affinity = current.clone();
    unknown_affinity.selected_mut("actor_definitions", 1)["physical_damage_affinity_profile_id"] =
        json!("missing");
    assert_has(
        &definition_error(&unknown_affinity),
        "references unknown selected profile",
    );

    let mut bad_loot_chance = current.clone();
    bad_loot_chance.selected_mut("loot_tables", 0)["entries"][1]["chance_numerator"] = json!(3);
    assert_has(
        &definition_error(&bad_loot_chance),
        "chance must be within 1..=denominator",
    );

    let mut unknown_loot_item = current.clone();
    unknown_loot_item.selected_mut("loot_tables", 0)["entries"][0]["item_definition_id"] =
        json!("missing");
    assert_has(
        &definition_error(&unknown_loot_item),
        "item_definition_id is not selected",
    );

    let mut duplicate_loot_position = current.clone();
    duplicate_loot_position.selected_mut("loot_tables", 0)["entries"][1]["kind"] = json!("item");
    duplicate_loot_position.selected_mut("loot_tables", 0)["entries"][1]["item_definition_id"] =
        json!("flint");
    duplicate_loot_position.selected_mut("loot_tables", 0)["entries"][1]["quantity"] = json!(1);
    duplicate_loot_position.selected_mut("loot_tables", 0)["entries"][1]["position"] =
        json!("sack_item_1");
    duplicate_loot_position.selected_mut("loot_tables", 0)["entries"][1]
        .as_object_mut()
        .unwrap()
        .remove("minimum_amount");
    duplicate_loot_position.selected_mut("loot_tables", 0)["entries"][1]
        .as_object_mut()
        .unwrap()
        .remove("maximum_amount");
    assert_has(
        &definition_error(&duplicate_loot_position),
        "duplicates a possible carried position",
    );

    let mut duplicate_gold_sack = current.clone();
    let gold = duplicate_gold_sack.selected_mut("loot_tables", 0)["entries"][1].clone();
    duplicate_gold_sack.selected_mut("loot_tables", 0)["entries"]
        .as_array_mut()
        .unwrap()
        .push(gold);
    duplicate_gold_sack.selected_mut("loot_tables", 0)["entries"][2]["id"] = json!("second_coins");
    assert_has(
        &definition_error(&duplicate_gold_sack),
        "duplicates a possible carried position",
    );

    let mut non_monster_member = current.clone();
    non_monster_member.selected_mut("spawn_groups", 0)["members"][0]["actor_definition_id"] =
        json!("actor/creature_ecology_gallery/player");
    assert_has(
        &definition_error(&non_monster_member),
        "must reference a selected monster",
    );

    let mut bad_pack_cardinality = current.clone();
    bad_pack_cardinality.selected_mut("spawn_groups", 0)["members"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert_has(
        &definition_error(&bad_pack_cardinality),
        "members cardinality does not match ecology_kind",
    );

    let mut non_lair_group = current;
    non_lair_group.selected_mut("lair_definitions", 0)["spawn_group_id"] = json!("gallery_pack");
    assert_has(
        &definition_error(&non_lair_group),
        "must reference a selected lair group",
    );
}

#[test]
fn catalog_six_loot_families_choices_caps_ranges_and_positions_fail_closed() {
    let current = parts("first_land_structure");
    current
        .definition()
        .expect("current Catalog 6 first-land loot graph");

    let mut missing_family = current.clone();
    missing_family
        .selected_mut("loot_tables", 0)
        .as_object_mut()
        .unwrap()
        .remove("family");
    assert_has(&decode_error(&missing_family), "missing field `family`");

    let mut unknown_family = current.clone();
    unknown_family.selected_mut("loot_tables", 0)["family"] = json!("legacy");
    assert_has(&decode_error(&unknown_family), "unknown variant `legacy`");

    let mut missing_cap = current.clone();
    missing_cap
        .selected_mut("loot_tables", 0)
        .as_object_mut()
        .unwrap()
        .remove("maximum_non_gold_drops");
    assert_has(
        &definition_error(&missing_cap),
        "maximum_non_gold_drops is required for an ordinary table",
    );

    for cap in [0, 3] {
        let mut invalid_cap = current.clone();
        invalid_cap.selected_mut("loot_tables", 0)["maximum_non_gold_drops"] = json!(cap);
        assert_has(
            &definition_error(&invalid_cap),
            "maximum_non_gold_drops must be within 1..=2",
        );
    }

    let mut signature_with_cap = current.clone();
    signature_with_cap.selected_mut("loot_tables", 0)["family"] = json!("signature");
    assert_has(
        &definition_error(&signature_with_cap),
        "maximum_non_gold_drops is forbidden for a signature table",
    );

    let mut empty_entries = current.clone();
    empty_entries.selected_mut("loot_tables", 0)["entries"] = json!([]);
    assert_has(
        &definition_error(&empty_entries),
        "entries must be non-empty",
    );

    let mut short_choice = current.clone();
    short_choice.selected_mut("loot_tables", 1)["entries"][1]["members"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert_has(
        &definition_error(&short_choice),
        "members must contain at least two rows",
    );

    let mut duplicate_member_id = current.clone();
    duplicate_member_id.selected_mut("loot_tables", 1)["entries"][1]["members"][1]["member_id"] =
        json!("rusted_knife");
    assert_has(
        &definition_error(&duplicate_member_id),
        "member_id must be non-empty and unique",
    );

    let mut duplicate_member_definition = current.clone();
    duplicate_member_definition.selected_mut("loot_tables", 1)["entries"][1]["members"][1]["item_definition_id"] =
        json!("rusted_knife");
    assert_has(
        &definition_error(&duplicate_member_definition),
        "item_definition_id must be unique within its choice group",
    );

    let mut unknown_member_definition = current.clone();
    unknown_member_definition.selected_mut("loot_tables", 1)["entries"][1]["members"][1]["item_definition_id"] =
        json!("missing");
    assert_has(
        &definition_error(&unknown_member_definition),
        "item_definition_id is not selected",
    );

    let mut zero_member_quantity = current.clone();
    zero_member_quantity.selected_mut("loot_tables", 1)["entries"][1]["members"][0]["quantity"] =
        json!(0);
    assert_has(
        &definition_error(&zero_member_quantity),
        "quantity must be positive",
    );

    let mut invalid_member_position = current.clone();
    invalid_member_position.selected_mut("loot_tables", 1)["entries"][1]["members"][1]["position"] =
        json!("belt_1");
    assert_has(
        &definition_error(&invalid_member_position),
        "position is not valid for the selected item definition",
    );

    let mut independent_collision = current.clone();
    independent_collision.selected_mut("loot_tables", 1)["entries"][2]["position"] =
        json!("right_hand");
    assert_has(
        &definition_error(&independent_collision),
        "duplicates a possible carried position from an independent outcome",
    );

    let mut invalid_direct_position = current.clone();
    invalid_direct_position.selected_mut("loot_tables", 0)["entries"][0]["position"] =
        json!("belt_back");
    assert_has(
        &definition_error(&invalid_direct_position),
        "position is not valid for the selected item definition",
    );

    for (minimum, maximum) in [(0, 3), (4, 3), (1, i64::from(u32::MAX) + 1)] {
        let mut bad_gold = current.clone();
        bad_gold.selected_mut("loot_tables", 0)["entries"][3]["minimum_amount"] = json!(minimum);
        bad_gold.selected_mut("loot_tables", 0)["entries"][3]["maximum_amount"] = json!(maximum);
        assert_has(
            &definition_error(&bad_gold),
            "gold range must be positive, ordered, and bounded",
        );
    }

    let mut signature_overflow = current;
    signature_overflow.selected_mut("loot_tables", 0)["family"] = json!("signature");
    signature_overflow
        .selected_mut("loot_tables", 0)
        .as_object_mut()
        .unwrap()
        .remove("maximum_non_gold_drops");
    let fourth_item = signature_overflow.selected_mut("loot_tables", 0)["entries"][0].clone();
    signature_overflow.selected_mut("loot_tables", 0)["entries"]
        .as_array_mut()
        .unwrap()
        .push(fourth_item);
    signature_overflow.selected_mut("loot_tables", 0)["entries"][4]["id"] =
        json!("fourth_non_gold");
    signature_overflow.selected_mut("loot_tables", 0)["entries"][4]["position"] = json!("belt_2");
    assert_has(
        &definition_error(&signature_overflow),
        "signature table may select at most three non-gold results",
    );
}

#[test]
fn catalog_six_reset_policies_and_signature_full_site_requirement_fail_closed() {
    let current = parts("creature_ecology_gallery");

    let mut removed_policy = current.clone();
    removed_policy.selected_mut("spawn_groups", 0)["reset"] =
        json!({"trigger": "all_defeated", "delay_units": 2});
    assert_has(&decode_error(&removed_policy), "missing field `policy`");

    let mut legacy_trigger = current.clone();
    legacy_trigger.selected_mut("spawn_groups", 0)["reset"] =
        json!({"policy": "full_site", "trigger": "all_defeated", "delay_units": 2});
    assert_has(&decode_error(&legacy_trigger), "unknown field `trigger`");

    let mut zero_full_site = current.clone();
    zero_full_site.selected_mut("spawn_groups", 0)["reset"]["delay_units"] = json!(0);
    assert_has(
        &definition_error(&zero_full_site),
        "reset.delay_units must be positive",
    );

    for reset in [
        json!({
            "policy": "slot_replenishment",
            "slot_delay_units": 0,
            "full_clear_delay_units": 3
        }),
        json!({
            "policy": "slot_replenishment",
            "slot_delay_units": 2,
            "full_clear_delay_units": 0
        }),
        json!({
            "policy": "slot_replenishment",
            "slot_delay_units": 2,
            "full_clear_delay_units": 2
        }),
    ] {
        let mut invalid = current.clone();
        invalid.selected_mut("spawn_groups", 0)["reset"] = reset;
        assert_has(&definition_error(&invalid), "reset.");
    }

    let mut lair_replenishment = current;
    lair_replenishment.selected_mut("spawn_groups", 1)["reset"] = json!({
        "policy": "slot_replenishment",
        "slot_delay_units": 2,
        "full_clear_delay_units": 3
    });
    assert_has(
        &definition_error(&lair_replenishment),
        "must reference a full-site reset group",
    );
}

/// The shared catalog's registry sizes, exactly.
///
/// Two of these counts include the identity proof land's own definitions —
/// `actor-definition/identity_proof/threshold_keeper` and
/// `service/identity_proof/threshold_keeper` — because this catalog is the only
/// definition registry the project carries and an authored land resolves in it.
/// See `content/test-corpus/README.md`.
#[test]
fn gate_two_catalog_inventory_is_exact_and_derived_from_the_registered_maps() {
    let catalog = &parts("first_land_structure").catalog;
    let expected = [
        ("rules_profiles", 15),
        ("skill_catalogs", 7),
        ("damage_labels", 15),
        ("items", 76),
        ("spells", 73),
        ("quests", 2),
        ("summon_templates", 2),
        ("profession_actions", 3),
        ("service_definitions", 16),
        ("banks", 3),
        ("locker_vaults", 3),
        ("terrains", 86),
        ("scavenging_profiles", 1),
        ("actor_definitions", 164),
        ("physical_damage_affinity_profiles", 3),
        ("loot_tables", 16),
        ("spawn_groups", 40),
        ("lair_definitions", 6),
    ];
    let actual = expected
        .iter()
        .map(|(registry, _)| {
            catalog[*registry]
                .as_object()
                .unwrap_or_else(|| panic!("{registry} registry"))
                .len()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        expected.iter().map(|(_, count)| *count).collect::<Vec<_>>()
    );
    assert_eq!(actual.iter().sum::<usize>(), 531);
    assert_eq!(
        catalog["profiles"]
            .as_object()
            .expect("catalog profiles")
            .len(),
        51
    );
}

#[test]
fn gate_two_first_land_reset_matrix_and_signature_lair_sources_are_exact() {
    let first_land = parts("first_land_structure");
    let signature_delays = [
        ("spawn/testland/surface_great_bear", 450),
        ("spawn/testland/surface_east_portal_magus", 450),
        ("spawn/testland/upper_halls_hidden_lair", 900),
        ("spawn/testland/old_temple_summoning_lair", 900),
        ("spawn/testland/old_temple_crypt_lair", 900),
    ]
    .into_iter()
    .collect::<std::collections::BTreeMap<_, _>>();
    let selected_groups =
        first_land.catalog["profiles"]["profile/first_land_structure"]["spawn_groups"]
            .as_array()
            .expect("first-land spawn-group selection");
    let mut ordinary_count = 0;
    let mut signature_count = 0;
    for key in selected_groups {
        let key = key.as_str().expect("spawn-group key");
        let group = &first_land.catalog["spawn_groups"][key];
        let id = group["id"].as_str().expect("spawn-group id");
        if !id.starts_with("spawn/testland/") {
            continue;
        }
        if let Some(expected_delay) = signature_delays.get(id) {
            signature_count += 1;
            assert_eq!(
                group["reset"],
                json!({"policy": "full_site", "delay_units": expected_delay})
            );
        } else {
            ordinary_count += 1;
            assert_eq!(
                group["reset"],
                json!({
                    "policy": "slot_replenishment",
                    "slot_delay_units": 60,
                    "full_clear_delay_units": 180
                })
            );
        }
    }
    assert_eq!(ordinary_count, 33);
    assert_eq!(signature_count, 5);

    let ecology_sites = first_land.world_seed["ecology_sites"]
        .as_array()
        .expect("first-land ecology sites");
    for (group_id, _) in signature_delays {
        let suffix = group_id
            .strip_prefix("spawn/testland/")
            .expect("Testland signature group");
        let site = ecology_sites
            .iter()
            .find(|site| site["id"] == suffix)
            .unwrap_or_else(|| panic!("signature site {suffix}"));
        assert_eq!(
            site["source"],
            json!({
                "kind": "lair",
                "lair_definition_id": format!("lair/testland/{suffix}")
            })
        );
    }
}
