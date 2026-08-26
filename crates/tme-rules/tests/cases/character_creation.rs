use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};
use tme_rules::{CarriedPosition, Event};

fn starter_scenario() -> ContentParts {
    let mut scenario = ContentParts::tracked("first_room", "profile/first_room");
    scenario.catalog["spells"]["spell/spark/first_room"]["lane"] = json!("delver_magic");
    scenario.catalog["id"] = json!("internal_starter_contract");
    scenario.catalog["clean_content"] = json!(false);
    scenario.catalog["research_boundary"] = json!({
        "status": "internal_parity_fixture",
        "notes": "TME-PLACEHOLDER: generated internal parity runtime fixture.",
        "review_refs": ["focused starter contract test"]
    });
    scenario.catalog["items"] = json!({
        "item/starter_blade/character_creation": {
          "id": "starter_blade",
          "kind": "weapon",
          "name": "Starter Blade",
          "weapon": {
            "skill_track_id": "sword",
            "default_attack_mode": "fight",
            "attack_modes": [{"mode": "fight", "maximum_range": 0, "damage_kind": "cutting"}],
            "cooldown_units": 1,
            "combat_add_rating": 1,
            "handedness": "one_handed",
            "block_value": 0
          },
          "valid_placements": [
            "hand"
          ],
          "economy": {
            "unit_burden": 0
          }
        },
        "item/side_blade/character_creation": {
          "id": "side_blade",
          "kind": "weapon",
          "name": "Side Blade",
          "weapon": {
            "skill_track_id": "sword",
            "default_attack_mode": "fight",
            "attack_modes": [{"mode": "fight", "maximum_range": 0, "damage_kind": "cutting"}],
            "cooldown_units": 1,
            "combat_add_rating": 0,
            "handedness": "one_handed",
            "block_value": 0
          },
          "valid_placements": [
            "belt_side"
          ],
          "economy": {
            "unit_burden": 0
          }
        },
        "item/side_guard/character_creation": {
            "id": "side_guard",
            "kind": "shield",
            "name": "Side Guard",
            "valid_placements": ["belt_side"],
            "economy": {"unit_burden": 0}
        },
        "item/padded_coat/character_creation": {
            "id": "padded_coat",
            "kind": "armor",
            "name": "Padded Coat",
            "valid_placements": ["inner_armor"],
            "economy": {"unit_burden": 0}
        },
        "item/field_book/character_creation": {
            "id": "field_book",
            "kind": "book",
            "name": "Field Book",
            "valid_placements": ["sack"],
            "economy": {"unit_burden": 0}
        }
    });
    scenario.profile_value_mut()["items"] = json!([
        "item/starter_blade/character_creation",
        "item/side_blade/character_creation",
        "item/side_guard/character_creation",
        "item/padded_coat/character_creation",
        "item/field_book/character_creation"
    ]);
    *scenario.item_instances_mut() = json!({
        "start_right": {"definition_id": "starter_blade", "binding": {"state": "unrestricted"}},
        "start_belt_0": {"definition_id": "side_blade", "binding": {"state": "unrestricted"}},
        "start_belt_1": {"definition_id": "side_guard", "binding": {"state": "unrestricted"}},
        "start_armor": {"definition_id": "padded_coat", "binding": {"state": "unrestricted"}},
        "start_book": {"definition_id": "field_book", "binding": {"state": "unrestricted"}}
    });
    let player_index = scenario
        .actors_mut()
        .as_array_mut()
        .expect("actors should be an array")
        .iter()
        .position(|actor| actor["id"] == "player")
        .expect("player should exist");
    scenario.actor_definition_mut(player_index)["stats"]["hp"] = json!(9);
    scenario.actor_definition_mut(player_index)["social"]["alignment_source"] =
        json!({"kind": "character"});
    let player = &mut scenario.actors_mut()[player_index];
    player["character_id"] = json!("character:internal:starter");
    player["carried"] = json!({
        "items": [
            {"item_instance_id": "start_right", "position": "right_hand"},
            {"item_instance_id": "start_belt_0", "position": "belt_1"},
            {"item_instance_id": "start_belt_1", "position": "belt_2"},
            {"item_instance_id": "start_armor", "position": "inner_armor"},
            {"item_instance_id": "start_book", "position": "sack_item_1"}
        ],
        "gold": {"left_hand": 0, "right_hand": 0, "sack": 2}
    });
    player["starter_character"] = json!({
        "profile_id": "internal_test_profile",
        "class": {"id": "delver", "display": "Delver", "is_starter": true},
        "nationality": {"id": "northreach", "display": "Northreach"},
        "creation": {
            "attributes": {
                "strength": 12,
                "dexterity": 11,
                "constitution": 10,
                "intelligence": 9,
                "wisdom": 8,
                "charisma": 7
            },
            "bounds": {
                "strength": {"inborn": 10, "creation_cap": 13},
                "dexterity": {"inborn": 10, "creation_cap": 12},
                "constitution": {"inborn": 9, "creation_cap": 11},
                "intelligence": {"inborn": 9, "creation_cap": 11},
                "wisdom": {"inborn": 8, "creation_cap": 10},
                "charisma": {"inborn": 7, "creation_cap": 9}
            },
            "creation_points_available": 4,
            "creation_points_spent": 4
        },
        "progression": {"level": 2, "experience": 100},
        "runtime_defaults": {
            "alignment_state": {"alignment": "lawful", "karma_points": 0},
            "resources": {"hp": 9, "max_hp": 9, "peak_hp": 9, "mp": 2, "max_mp": 2, "stamina": 8, "max_stamina": 8},
            "physical_attribute_adds": {"strength_adds": 1, "dexterity_adds": 0},
            "open_question_ids": ["test_resources_open", "test_adds_open"]
        },
        "initial_skills": [
            {
                "track_id": "blade",
                "level": 3,
                "critique_rank": 0,
                "practice_points": 0,
                "learning_rate": 1
            }
        ],
        "initial_known_spells": [
            {
                "spell_id": "spark",
                "lane": "delver_magic",
                "learned_at_level": 2
            }
        ],
        "loadout": {
            "gold": {"left_hand": 0, "right_hand": 0, "sack": 2},
            "right_hand": {
                "source_row_id": "test.right",
                "item_definition_id": "starter_blade",
                "item_instance_id": "start_right",
                "rating_scale_id": "weapon",
                "documented_rating_id": "trained"
            },
            "ordered_belt": [
                {
                    "source_row_id": "test.belt.0",
                    "item_definition_id": "side_blade",
                    "item_instance_id": "start_belt_0",
                    "rating_scale_id": "weapon",
                    "documented_rating_id": "learning"
                },
                {
                    "source_row_id": "test.belt.1",
                    "item_definition_id": "side_guard",
                    "item_instance_id": "start_belt_1",
                    "rating_scale_id": "not_applicable",
                    "documented_rating_id": "not_applicable"
                }
            ],
            "inner_armor": {"item_definition_id": "padded_coat", "item_instance_id": "start_armor"},
            "loot_sack_present": true,
            "spell_book": {"item_definition_id": "field_book", "item_instance_id": "start_book"},
            "documented_skills": [
                {"source_row_id": "test.skill.hands", "skill_id": "martial_arts", "rating_scale_id": "martial_arts", "documented_rating_id": "novice"},
                {"source_row_id": "test.skill.magic", "skill_id": "magic", "rating_scale_id": "magic", "documented_rating_id": "not_applicable"},
                {"source_row_id": "test.skill.theft", "skill_id": "theft", "rating_scale_id": "thievery", "documented_rating_id": "untrained"}
            ]
        },
        "open_evidence": ["test_creation_open", "test_positions_open"]
    });
    let delver_profile = json!({
        "class_id": "delver",
        "hit_points": {
            "kind": "attribute_bands",
            "attribute": "constitution",
            "bands": [
                {"minimum_attribute": 0, "outcomes": [{"amount": 7, "weight": 1}, {"amount": 8, "weight": 1}]},
                {"minimum_attribute": 10, "outcomes": [{"amount": 8, "weight": 1}, {"amount": 9, "weight": 1}]},
                {"minimum_attribute": 15, "outcomes": [{"amount": 9, "weight": 1}, {"amount": 10, "weight": 1}]}
            ]
        },
        "magic_points": null,
        "stamina_points": {
            "kind": "attribute_bands",
            "attribute": "strength",
            "bands": [
                {"minimum_attribute": 0, "outcomes": [{"amount": 3, "weight": 1}, {"amount": 4, "weight": 1}]},
                {"minimum_attribute": 10, "outcomes": [{"amount": 4, "weight": 1}, {"amount": 5, "weight": 1}]},
                {"minimum_attribute": 15, "outcomes": [{"amount": 5, "weight": 1}, {"amount": 6, "weight": 1}]}
            ]
        },
        "physical_attribute_adds_by_level": [
            {"level": 3, "strength_adds": 1, "dexterity_adds": 1},
            {"level": 4, "strength_adds": 1, "dexterity_adds": 1},
            {"level": 8, "strength_adds": 1, "dexterity_adds": 1}
        ]
    });
    scenario.rules_source_mut()["progression"]["growth_profiles"]
        .as_array_mut()
        .expect("first-room growth profiles")
        .push(delver_profile);
    scenario
}

fn parse(value: &ContentParts) -> Result<tme_rules::engine::ValidatedWorldSeed, String> {
    value.validated_seed()
}

fn player_mut(value: &mut ContentParts) -> &mut Value {
    value
        .actors_mut()
        .as_array_mut()
        .expect("actors should be an array")
        .iter_mut()
        .find(|actor| actor["id"] == "player")
        .expect("player should exist")
}

#[test]
fn internal_starter_builds_one_authoritative_runtime_character() {
    let engine = starter_scenario()
        .engine(7)
        .expect("starter scenario should initialize");
    let player = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.kind == tme_rules::ActorKind::Player)
        .expect("player should exist");
    let character = player
        .character
        .as_ref()
        .expect("starter should build a sheet");

    assert_eq!(character.identity.base_class_id, "delver");
    assert_eq!(character.identity.current_class_id, "delver");
    assert_eq!(character.identity.display_class, "Delver");
    assert_eq!(character.identity.nationality_id, "northreach");
    assert_eq!(character.attributes.strength, 12);
    assert_eq!(character.progression.level, 2);
    assert_eq!(character.progression.experience, 100);
    assert_eq!(character.resources.hp, 9);
    assert_eq!(character.physical_attribute_adds.strength_adds, 1);
    assert_eq!(character.skill_ledger.len(), 1);
    assert_eq!(character.skill_ledger[0].track_id, "blade");
    assert_eq!(character.skill_ledger[0].level, 3);
    assert_eq!(character.skill_ledger[0].critique_rank, 0);
    assert_eq!(character.skill_ledger[0].practice_points, 0);
    assert_eq!(character.skill_ledger[0].learning_rate, 1);
    assert_eq!(character.known_spells.len(), 1);
    assert_eq!(character.known_spells[0].spell_id, "spark");
    assert_eq!(character.known_spells[0].lane, "delver_magic");
    assert_eq!(character.known_spells[0].learned_at_level, 2);
    assert_eq!(player.hp, 9);
    assert_eq!(player.mp, 2);
    assert_eq!(player.stamina, 8);
    assert_eq!(player.carried.gold.sack, 2);
    assert_eq!(
        player.carried.items.get(&CarriedPosition::RightHand),
        Some(&"start_right".to_string())
    );
    let initial_identity = engine
        .initial_events()
        .into_iter()
        .find_map(|event| match event {
            Event::ActorStatus {
                actor_id,
                character_identity,
                ..
            } if actor_id == "player" => character_identity,
            _ => None,
        })
        .expect("initial player status should include the starter identity");
    assert_eq!(initial_identity.current_class_id, "delver");
    assert_eq!(initial_identity.nationality_id, "northreach");
}

#[test]
fn internal_starter_accepts_closed_evidence_lists() {
    let mut value = starter_scenario();
    player_mut(&mut value)["starter_character"]["runtime_defaults"]["open_question_ids"] =
        json!([]);
    player_mut(&mut value)["starter_character"]["open_evidence"] = json!([]);

    parse(&value).expect("closed starter evidence lists may be empty");
}

#[test]
fn internal_starter_requires_an_explicit_learning_rate() {
    let mut value = starter_scenario();
    player_mut(&mut value)["starter_character"]["initial_skills"][0]
        .as_object_mut()
        .expect("starter skill entry")
        .remove("learning_rate");
    let error = parse(&value).expect_err("starter skills cannot infer a learning rate");
    assert!(error.contains("missing field `learning_rate`"), "{error}");
}

#[test]
fn internal_starter_known_spells_must_be_exact_and_resolved() {
    let mut unknown = starter_scenario();
    player_mut(&mut unknown)["starter_character"]["initial_known_spells"][0]["spell_id"] =
        json!("missing_spell");
    let error = parse(&unknown).expect_err("unknown initial spell must fail");
    assert!(error.contains("references unknown spell"), "{error}");

    let mut wrong_lane = starter_scenario();
    player_mut(&mut wrong_lane)["starter_character"]["initial_known_spells"][0]["lane"] =
        json!("wrong_magic");
    let error = parse(&wrong_lane).expect_err("wrong initial spell lane must fail");
    assert!(
        error.contains("lane must match the referenced spell lane"),
        "{error}"
    );

    let mut wrong_level = starter_scenario();
    player_mut(&mut wrong_level)["starter_character"]["initial_known_spells"][0]["learned_at_level"] =
        json!(1);
    let error = parse(&wrong_level).expect_err("wrong initial spell level must fail");
    assert!(
        error.contains("learned_at_level must equal starter progression level"),
        "{error}"
    );

    let mut duplicate = starter_scenario();
    let row = player_mut(&mut duplicate)["starter_character"]["initial_known_spells"][0].clone();
    player_mut(&mut duplicate)["starter_character"]["initial_known_spells"]
        .as_array_mut()
        .expect("starter known spells")
        .push(row);
    let error = parse(&duplicate).expect_err("duplicate initial spell must fail");
    assert!(error.contains("spell_id must be unique"), "{error}");
}

#[test]
fn malformed_starter_references_report_errors_without_panicking() {
    let mut missing_reference = starter_scenario();
    player_mut(&mut missing_reference)["starter_character"]["loadout"]["right_hand"]
        .as_object_mut()
        .expect("starter right-hand row")
        .remove("item_instance_id");
    let error = parse(&missing_reference).expect_err("missing starter reference must fail");
    assert!(
        error.contains("missing field `item_instance_id`"),
        "{error}"
    );

    let mut malformed_carried = starter_scenario();
    player_mut(&mut malformed_carried)["carried"]["items"] = json!(7);
    let error = parse(&malformed_carried).expect_err("malformed carried list must fail");
    assert!(error.contains("invalid type"), "{error}");
}

#[test]
fn clean_content_rejects_the_internal_starter_role() {
    let mut value = starter_scenario();
    value.catalog["clean_content"] = json!(true);
    value.catalog["research_boundary"] = json!({
        "status": "clean_original_fixture",
        "notes": "Original focused test fixture.",
        "review_refs": ["focused test"]
    });
    let error = parse(&value).expect_err("clean starter must fail");
    assert!(error.contains("starter_character is only valid in an internal_parity_fixture"));
}

#[test]
fn internal_boundary_requires_the_marker_and_exact_pairing() {
    let mut missing_marker = starter_scenario();
    missing_marker.catalog["research_boundary"]["notes"] = json!("Generated fixture.");
    missing_marker.actor_definition_mut(0)["name"] = json!("zorbelquux");
    let error = parse(&missing_marker).expect_err("missing marker must fail");
    assert!(error.contains("must contain TME-PLACEHOLDER"));

    let mut missing_refs = starter_scenario();
    missing_refs.catalog["research_boundary"]["review_refs"] = json!([]);
    missing_refs.actor_definition_mut(0)["name"] = json!("zorbelquux");
    let error = parse(&missing_refs).expect_err("missing review refs must fail");
    assert!(error.contains("research_boundary.review_refs must be non-empty"));

    let mut wrong_pair = starter_scenario();
    wrong_pair.catalog["clean_content"] = json!(true);
    assert!(
        parse(&wrong_pair)
            .expect_err("mismatched pair must fail")
            .contains("must select exactly clean_original_fixture or internal_parity_fixture")
    );
}

#[test]
fn character_roles_are_mutually_exclusive_and_require_stable_identity() {
    let mut both = starter_scenario();
    player_mut(&mut both)["character"] = json!({
        "identity": {"base_class_id": "delver", "current_class_id": "delver", "display_class": "Delver", "nationality_id": "northreach"},
        "alignment_state": {"alignment": "lawful", "karma_points": 0},
        "attributes": {"strength": 12, "dexterity": 11, "constitution": 10, "intelligence": 9, "wisdom": 8, "charisma": 7},
        "resources": {"hp": 9, "max_hp": 9, "peak_hp": 9, "mp": 2, "max_mp": 2, "stamina": 8, "max_stamina": 8},
        "progression": {"level": 2, "experience": 100},
        "physical_attribute_adds": {"strength_adds": 1, "dexterity_adds": 0}
    });
    assert!(
        parse(&both)
            .expect_err("both roles must fail")
            .contains("must not contain both character and starter_character")
    );

    let mut no_id = starter_scenario();
    player_mut(&mut no_id)
        .as_object_mut()
        .expect("player should be an object")
        .remove("character_id");
    assert!(
        parse(&no_id)
            .expect_err("missing stable id must fail")
            .contains("character_id is required when a character role is present")
    );
}

#[test]
fn starter_rejects_non_starter_class_and_bad_pool_math() {
    let mut value = starter_scenario();
    player_mut(&mut value)["starter_character"]["class"]["is_starter"] = json!(false);
    player_mut(&mut value)["starter_character"]["creation"]["attributes"]["strength"] = json!(14);
    let error = parse(&value).expect_err("invalid class and pool must fail");
    assert!(error.contains("class.is_starter must be true"));
    assert!(error.contains("creation.attributes.strength must be at most 13"));
    assert!(error.contains("creation_points_spent must equal recomputed spend"));
}

#[test]
fn starter_rejects_bad_loadout_definition_position_gold_and_extras() {
    let mut value = starter_scenario();
    let player = player_mut(&mut value);
    player["carried"]["gold"]["sack"] = json!(3);
    player["carried"]["items"][0]["position"] = json!("belt_3");
    player["starter_character"]["loadout"]["inner_armor"]["item_definition_id"] =
        json!("wrong_definition");
    player["carried"]["items"]
        .as_array_mut()
        .expect("carried items should be an array")
        .push(json!({"item_instance_id": "extra_item", "position": "sack_item_2"}));
    value.item_instances_mut()["extra_item"] =
        json!({"definition_id": "field_book", "binding": {"state": "unrestricted"}});

    let error = parse(&value).expect_err("bad resolved loadout must fail");
    assert!(error.contains("carried.gold must equal"));
    assert!(error.contains("right_hand.item_instance_id has invalid carried position"));
    assert!(error.contains("inner_armor.item_definition_id does not match"));
    assert!(error.contains("carried.items must equal the starter resolved loadout"));
}

#[test]
fn starter_rejects_obsolete_item_class_restrictions() {
    let mut value = starter_scenario();
    let obsolete_field = ["class_", "restrict_allow"].concat();
    value.selected_mut("items", 0)["capability"] = json!({});
    value.selected_mut("items", 0)["capability"][&obsolete_field] = json!(["warden"]);

    let error = parse(&value).expect_err("obsolete class restriction must fail");
    assert!(error.contains("unknown field"));
}

#[test]
fn starter_rejects_non_player_use() {
    let mut value = starter_scenario();
    let monster_social = value.actor_definition_mut(1)["social"].clone();
    let monster_ai = value.actor_definition_mut(1)["ai"].clone();
    value.actor_definition_mut(0)["kind"] = json!("monster");
    value.actor_definition_mut(0)["social"] = monster_social;
    value.actor_definition_mut(0)["ai"] = monster_ai;
    let error = parse(&value).expect_err("non-player starter must fail");
    assert!(
        error.contains("starter_character is only valid for players"),
        "{error}"
    );
}

#[test]
fn starter_rejects_duplicate_open_ids() {
    let mut value = starter_scenario();
    player_mut(&mut value)["starter_character"]["open_evidence"] =
        json!(["test_creation_open", "test_creation_open"]);
    let error = parse(&value).expect_err("duplicate open ids must fail");
    assert!(error.contains("open_evidence[1] must be unique"));
}
