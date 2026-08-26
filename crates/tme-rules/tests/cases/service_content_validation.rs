use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};
use tme_rules::SelectedCatalog;
use tme_rules::content::{ServiceCapabilityDef, TransactionRequirementDef, TransactionRewardDef};

fn trainer_parts() -> ContentParts {
    ContentParts::tracked(
        "spell_learning_purchase_casting_xp",
        "profile/spell_learning_purchase_casting_xp",
    )
}

fn promotion_parts() -> ContentParts {
    ContentParts::tracked("knight_promotion", "profile/knight_promotion")
}

fn definition_error(parts: &ContentParts) -> String {
    parts
        .definition()
        .expect_err("mutated service definition must fail")
}

fn seed_error(parts: &ContentParts) -> String {
    parts
        .validated_seed()
        .expect_err("mutated service placement must fail")
}

fn selected_catalog(parts: &ContentParts) -> SelectedCatalog {
    let (catalog, profile, _, _) = parts.decode().expect("strict four-part decode");
    catalog.select(&profile).expect("selected catalog")
}

fn selected_key(parts: &ContentParts, registry: &str, index: usize) -> String {
    parts.profile_value()[registry][index]
        .as_str()
        .unwrap_or_else(|| panic!("{registry}[{index}] key"))
        .to_string()
}

fn selected_row(parts: &ContentParts, registry: &str, index: usize) -> Value {
    let key = selected_key(parts, registry, index);
    parts.catalog[registry][key].clone()
}

fn selected_index_by_id(parts: &ContentParts, registry: &str, id: &str) -> usize {
    (0..parts.selected_len(registry))
        .find(|index| selected_row(parts, registry, *index)["id"] == id)
        .unwrap_or_else(|| panic!("selected {registry} row {id:?}"))
}

fn assert_contains(message: &str, expected: &str) {
    assert!(
        message.contains(expected),
        "expected {expected:?} in diagnostic:\n{message}"
    );
}

fn add_skill_track(parts: &mut ContentParts, track: Value) {
    parts.skill_catalog_mut().expect("selected skill catalog")["tracks"]
        .as_array_mut()
        .expect("skill tracks")
        .push(track);
}

fn add_second_service_definition(
    parts: &mut ContentParts,
    definition_id: &str,
    instance_id: &str,
    position: Value,
) -> usize {
    let mut definition = selected_row(parts, "service_definitions", 0);
    definition["id"] = json!(definition_id);
    definition["name"] = json!(format!("Second {definition_id}"));
    parts.push_selected(
        "service_definitions",
        &format!("test/{definition_id}"),
        definition,
    );
    let mut instance = parts.service_instances_mut()[0].clone();
    instance["id"] = json!(instance_id);
    instance["service_definition_id"] = json!(definition_id);
    instance["location"]["position"] = position;
    parts
        .service_instances_mut()
        .as_array_mut()
        .expect("service instances")
        .push(instance);
    parts.selected_len("service_definitions") - 1
}

#[test]
fn selected_trainer_definition_keeps_one_field_teachings_and_authored_capability_order() {
    let parts = trainer_parts();
    parts.definition().expect("trainer definition validates");
    let selected = selected_catalog(&parts);

    assert!(matches!(
        selected.service_definitions[0].capabilities.as_slice(),
        [
            ServiceCapabilityDef::SkillTraining { id: training_id, .. },
            ServiceCapabilityDef::SkillCritique { id: critique_id },
            ServiceCapabilityDef::SpellTeaching {
                id: teaching_id,
                training_capability_id,
                teachings,
            },
        ] if training_id == "training"
            && critique_id == "critique"
            && teaching_id == "spell_teaching"
            && training_capability_id == "training"
            && teachings.len() == 1
            && teachings[0].spell_id == "spark"
    ));
}

#[test]
fn split_service_definition_and_instance_shapes_are_strict() {
    for field in [
        "room",
        "position",
        "training_offers",
        "promotion",
        "teaches_spells",
    ] {
        let mut parts = trainer_parts();
        parts.selected_mut("service_definitions", 0)[field] = match field {
            "room" => json!("room_0"),
            "position" => json!({"x": 1, "y": 1}),
            "promotion" => json!({}),
            _ => json!([]),
        };
        assert_contains(
            &definition_error(&parts),
            &format!("unknown field `{field}`"),
        );
    }

    let mut missing_capabilities = trainer_parts();
    missing_capabilities
        .selected_mut("service_definitions", 0)
        .as_object_mut()
        .expect("service definition")
        .remove("capabilities");
    assert_contains(
        &definition_error(&missing_capabilities),
        "missing field `capabilities`",
    );

    let mut instance_policy = trainer_parts();
    instance_policy.service_instances_mut()[0]["name"] = json!("copied policy");
    assert_contains(&seed_error(&instance_policy), "unknown field `name`");

    let mut missing_position = trainer_parts();
    missing_position.service_instances_mut()[0]["location"]
        .as_object_mut()
        .expect("service location")
        .remove("position");
    assert_contains(&seed_error(&missing_position), "missing field `position`");
}

#[test]
fn capability_ids_kinds_and_cardinality_are_strict() {
    let mut missing_id = trainer_parts();
    missing_id.selected_mut("service_definitions", 0)["capabilities"][0]
        .as_object_mut()
        .expect("capability")
        .remove("id");
    assert_contains(&definition_error(&missing_id), "missing field `id`");

    let mut empty_id = trainer_parts();
    empty_id.selected_mut("service_definitions", 0)["capabilities"][0]["id"] = json!(" ");
    assert_contains(&definition_error(&empty_id), ".id must be non-empty");

    let mut duplicate_id = trainer_parts();
    duplicate_id.selected_mut("service_definitions", 0)["capabilities"][1]["id"] =
        json!("training");
    assert_contains(&definition_error(&duplicate_id), ".id duplicates");

    let mut unknown_kind = trainer_parts();
    unknown_kind.selected_mut("service_definitions", 0)["capabilities"][0]["kind"] =
        json!("commerce");
    assert_contains(
        &definition_error(&unknown_kind),
        "unknown variant `commerce`",
    );

    let mut wrong_variant_field = trainer_parts();
    wrong_variant_field.selected_mut("service_definitions", 0)["capabilities"][1]["offers"] =
        json!([]);
    assert_contains(
        &definition_error(&wrong_variant_field),
        "unknown field `offers`",
    );

    let mut duplicate_kind = trainer_parts();
    let mut duplicate =
        duplicate_kind.selected_mut("service_definitions", 0)["capabilities"][1].clone();
    duplicate["id"] = json!("second_critique");
    duplicate_kind.selected_mut("service_definitions", 0)["capabilities"]
        .as_array_mut()
        .expect("capabilities")
        .push(duplicate);
    assert_contains(&definition_error(&duplicate_kind), ".kind duplicates");

    let mut empty_capabilities = trainer_parts();
    empty_capabilities.selected_mut("service_definitions", 0)["capabilities"] = json!([]);
    assert_contains(
        &definition_error(&empty_capabilities),
        "capabilities must be a non-empty list",
    );
}

#[test]
fn training_offers_are_required_while_critique_remains_independent() {
    let mut missing = trainer_parts();
    missing.selected_mut("service_definitions", 0)["capabilities"][0]
        .as_object_mut()
        .expect("training capability")
        .remove("offers");
    assert_contains(&definition_error(&missing), "missing field `offers`");

    let mut empty = trainer_parts();
    empty.selected_mut("service_definitions", 0)["capabilities"][0]["offers"] = json!([]);
    assert_contains(&definition_error(&empty), "offers must be a non-empty list");

    let mut extra_catalog_track = trainer_parts();
    add_skill_track(
        &mut extra_catalog_track,
        json!({
            "id": "mace",
            "display": "Mace",
            "kind": "weapon",
            "ladder_id": "magic_measure",
            "eligible_class_ids": ["wizard"]
        }),
    );
    extra_catalog_track
        .definition()
        .expect("critique does not require exhaustive training offers");
    let selected = selected_catalog(&extra_catalog_track);
    assert!(matches!(
        selected.service_definitions[0].capabilities[1],
        ServiceCapabilityDef::SkillCritique { .. }
    ));
    assert!(matches!(
        &selected.service_definitions[0].capabilities[0],
        ServiceCapabilityDef::SkillTraining { offers, .. }
            if offers.iter().all(|offer| offer.track_id != "mace")
    ));
}

#[test]
fn spell_teaching_requires_one_exact_same_definition_magic_training_reference() {
    for (reference, expected) in [
        (" ", "training_capability_id must be non-empty"),
        (
            "missing_training",
            "does not reference a capability in the same service definition",
        ),
        (
            "critique",
            "must reference skill_training in the same service definition",
        ),
        (
            "spell_teaching",
            "must reference skill_training in the same service definition",
        ),
    ] {
        let mut parts = trainer_parts();
        parts.selected_mut("service_definitions", 0)["capabilities"][2]["training_capability_id"] =
            json!(reference);
        assert_contains(&definition_error(&parts), expected);
    }

    let mut removed_training = trainer_parts();
    removed_training.selected_mut("service_definitions", 0)["capabilities"]
        .as_array_mut()
        .expect("capabilities")
        .remove(0);
    assert_contains(
        &definition_error(&removed_training),
        "does not reference a capability in the same service definition",
    );

    let mut no_magic = trainer_parts();
    add_skill_track(
        &mut no_magic,
        json!({
            "id": "mace",
            "display": "Mace",
            "kind": "weapon",
            "ladder_id": "magic_measure",
            "eligible_class_ids": ["wizard"]
        }),
    );
    no_magic.selected_mut("service_definitions", 0)["capabilities"][0]["offers"][0]["track_id"] =
        json!("mace");
    assert_contains(
        &definition_error(&no_magic),
        "training with exactly one magic-lane offer",
    );

    let mut two_magic = trainer_parts();
    let duplicate =
        two_magic.selected_mut("service_definitions", 0)["capabilities"][0]["offers"][0].clone();
    two_magic.selected_mut("service_definitions", 0)["capabilities"][0]["offers"]
        .as_array_mut()
        .expect("offers")
        .push(duplicate);
    assert_contains(
        &definition_error(&two_magic),
        "training with exactly one magic-lane offer",
    );
}

#[test]
fn spell_teachings_are_strict_unique_lane_matched_and_authored_ordered() {
    let mut empty = trainer_parts();
    empty.selected_mut("service_definitions", 0)["capabilities"][2]["teachings"] = json!([]);
    assert_contains(
        &definition_error(&empty),
        "teachings must be a non-empty list",
    );

    let mut duplicate = trainer_parts();
    duplicate.selected_mut("service_definitions", 0)["capabilities"][2]["teachings"] =
        json!([{"spell_id": "spark"}, {"spell_id": "spark"}]);
    assert_contains(
        &definition_error(&duplicate),
        "spell_id must be unique within the capability",
    );

    let mut extra_field = trainer_parts();
    extra_field.selected_mut("service_definitions", 0)["capabilities"][2]["teachings"][0]["gold_cost"] =
        json!(1);
    assert_contains(&definition_error(&extra_field), "unknown field `gold_cost`");

    let mut wrong_lane = trainer_parts();
    wrong_lane.selected_mut("service_definitions", 0)["capabilities"][2]["teachings"][0]["spell_id"] =
        json!("prayer");
    assert_contains(
        &definition_error(&wrong_lane),
        "must match the trainer magic lane",
    );

    let mut ordered = trainer_parts();
    let mut second_spell = selected_row(&ordered, "spells", 0);
    second_spell["id"] = json!("second_spark");
    second_spell["name"] = json!("Second Spark");
    ordered.push_selected("spells", "test/second_spark", second_spell);
    ordered.selected_mut("service_definitions", 0)["capabilities"][2]["teachings"] =
        json!([{"spell_id": "spark"}, {"spell_id": "second_spark"}]);
    ordered
        .definition()
        .expect("two ordered teachings validate");
    let selected = selected_catalog(&ordered);
    let ServiceCapabilityDef::SpellTeaching { teachings, .. } =
        &selected.service_definitions[0].capabilities[2]
    else {
        panic!("third capability is spell teaching");
    };
    assert_eq!(
        teachings
            .iter()
            .map(|teaching| teaching.spell_id.as_str())
            .collect::<Vec<_>>(),
        ["spark", "second_spark"]
    );
}

#[test]
fn duplicate_colocated_class_spell_teaching_is_rejected_at_the_seed_boundary() {
    let mut parts = trainer_parts();
    let mut duplicate = parts.service_instances_mut()[0].clone();
    duplicate["id"] = json!("second_wizard_trainer");
    parts
        .service_instances_mut()
        .as_array_mut()
        .expect("service instances")
        .push(duplicate);
    assert_contains(&seed_error(&parts), "room/position/class/spell");
}

#[test]
fn class_promotion_exact_terms_and_authored_spell_order_are_locked() {
    let parts = promotion_parts();
    parts.definition().expect("promotion definition validates");
    let selected = selected_catalog(&parts);
    let ServiceCapabilityDef::ClassPromotion { transaction, .. } =
        &selected.service_definitions[0].capabilities[0]
    else {
        panic!("first capability is class promotion");
    };
    assert!(matches!(
        &transaction.requirements[0],
        TransactionRequirementDef::CurrentClass { class_id } if class_id == "fighter"
    ));
    assert!(matches!(
        &transaction.requirements[1],
        TransactionRequirementDef::MinimumLevel { level } if *level == 8
    ));
    assert!(matches!(
        &transaction.requirements[2],
        TransactionRequirementDef::ExactKarma { karma_points } if *karma_points == 0
    ));
    assert_eq!(
        transaction
            .rewards
            .iter()
            .filter_map(|reward| match reward {
                TransactionRewardDef::Spell { spell_id } => Some(spell_id.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>(),
        ["blessed_edge", "valor", "cleanse", "beacon", "trail_sense"]
    );

    for (field, replacement, expected) in [
        (
            "source",
            json!("wizard"),
            "must promote current class fighter to knight",
        ),
        (
            "target",
            json!("wizard"),
            "must promote current class fighter to knight",
        ),
        ("level", json!(7), "minimum_level must be 8"),
        ("karma", json!(1), "exact_karma must be 0"),
        (
            "empty_position",
            json!("left_hand"),
            "carried_position_empty must use right_hand",
        ),
    ] {
        let mut parts = promotion_parts();
        let transaction =
            &mut parts.selected_mut("service_definitions", 0)["capabilities"][0]["transaction"];
        match field {
            "source" => transaction["requirements"][0]["class_id"] = replacement,
            "target" => transaction["rewards"][0]["to_class_id"] = replacement,
            "level" => transaction["requirements"][1]["level"] = replacement,
            "karma" => transaction["requirements"][2]["karma_points"] = replacement,
            _ => transaction["requirements"][3]["position"] = replacement,
        }
        assert_contains(&definition_error(&parts), expected);
    }
}

#[test]
fn promotion_grant_instance_ring_and_spell_set_are_closed_contracts() {
    let mut registered = promotion_parts();
    registered.item_instances_mut()["oath_ring:knight_promotion:primary"] = json!({
        "definition_id": "oath_ring",
        "binding": {"state": "unrestricted"},
        "quantity": 1,
        "knowledge": {"identified": false}
    });
    registered.actors_mut()[0]["carried"]["items"] = json!([{
        "item_instance_id": "oath_ring:knight_promotion:primary",
        "position": "right_hand"
    }]);
    assert_contains(
        &seed_error(&registered),
        "item grant \"oath_ring:knight_promotion:primary\" must not already be registered",
    );

    let oath_ring = selected_index_by_id(&promotion_parts(), "items", "oath_ring");
    let mut missing_placement = promotion_parts();
    missing_placement.selected_mut("items", oath_ring)["valid_placements"] =
        json!(["hand", "sack"]);
    assert_contains(
        &definition_error(&missing_placement),
        "must allow hand and ring_finger placement",
    );

    let mut missing_focus = promotion_parts();
    missing_focus
        .selected_mut("items", oath_ring)
        .as_object_mut()
        .expect("oath ring")
        .remove("capability");
    assert_contains(&definition_error(&missing_focus), "must focus knight_magic");

    let mut unknown_item = promotion_parts();
    unknown_item.selected_mut("service_definitions", 0)["capabilities"][0]["transaction"]["rewards"]
        [1]["item_definition_id"] = json!("missing_ring");
    assert_contains(&definition_error(&unknown_item), "unknown item definition");

    let mut short = promotion_parts();
    short.selected_mut("service_definitions", 0)["capabilities"][0]["transaction"]["rewards"]
        .as_array_mut()
        .expect("promotion rewards")
        .pop();
    assert_contains(
        &definition_error(&short),
        "must contain exactly five spell rewards",
    );

    let mut duplicate_spell = promotion_parts();
    duplicate_spell.selected_mut("service_definitions", 0)["capabilities"][0]["transaction"]["rewards"]
        [6]["spell_id"] = json!("blessed_edge");
    assert_contains(
        &definition_error(&duplicate_spell),
        "spell_id must be unique",
    );

    let mut unknown_spell = promotion_parts();
    unknown_spell.selected_mut("service_definitions", 0)["capabilities"][0]["transaction"]["rewards"]
        [6]["spell_id"] = json!("missing_spell");
    assert_contains(&definition_error(&unknown_spell), "unknown spell");

    let trail_sense = selected_index_by_id(&promotion_parts(), "spells", "trail_sense");
    let mut wrong_lane = promotion_parts();
    wrong_lane.selected_mut("spells", trail_sense)["lane"] = json!("wizard_magic");
    assert_contains(
        &definition_error(&wrong_lane),
        "must reference a knight_magic spell",
    );
}

#[test]
fn promotion_placement_keys_and_grant_instance_ids_are_unique() {
    let mut duplicate_placement = promotion_parts();
    let second_definition = add_second_service_definition(
        &mut duplicate_placement,
        "second_promoter",
        "second_promoter",
        json!({"x": 1, "y": 1}),
    );
    duplicate_placement.selected_mut("service_definitions", second_definition)["capabilities"][0]
        ["transaction"]["rewards"][1]["item_instance_id"] =
        json!("oath_ring:knight_promotion:second");
    assert_contains(&seed_error(&duplicate_placement), "room/position/target");

    let mut duplicate_grant = promotion_parts();
    add_second_service_definition(
        &mut duplicate_grant,
        "second_promoter",
        "second_promoter",
        json!({"x": 1, "y": 2}),
    );
    assert_contains(&definition_error(&duplicate_grant), "item grant");
    assert_contains(&definition_error(&duplicate_grant), "duplicates");
}
