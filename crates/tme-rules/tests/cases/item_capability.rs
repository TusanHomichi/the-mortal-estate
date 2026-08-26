use crate::support::content_parts::ContentParts;
use serde_json::json;
use tme_rules::{CarriedPosition, ItemMoveDestination, PlayerIntent};

#[test]
fn item_economy_uses_unsigned_definition_values() {
    let economy: tme_rules::content::items::ItemEconomyDef = serde_json::from_value(json!({
        "unit_value_gold": 12,
        "unit_burden": 3
    }))
    .expect("item economy should parse");
    assert_eq!(economy.unit_value_gold, Some(12));
    assert_eq!(economy.unit_burden, 3);
}

#[test]
fn item_without_capability_loads_fine() {
    let parts = ContentParts::tracked("first_room", "profile/first_room");
    let definition = parts.definition().expect("first-room definition");
    let knife = definition
        .catalog()
        .item("training_knife")
        .expect("training knife");
    assert!(knife.capability.is_none());
}

#[test]
fn item_with_capability_parses_correctly() {
    let parts = ContentParts::tracked("fidelity_gallery", "profile/fidelity_gallery");
    let definition = parts.definition().expect("capability definition");

    let sword = definition
        .catalog()
        .item("training_sword")
        .expect("training sword");
    assert_eq!(
        sword
            .capability
            .as_ref()
            .and_then(|capability| capability.taxonomy_id.as_deref()),
        Some("training_sword")
    );
    assert!(sword.armor.is_none());
    assert_eq!(
        sword.weapon.as_ref().expect("weapon").handedness,
        tme_rules::model::WeaponHandedness::OneHanded
    );

    let armor = definition
        .catalog()
        .item("leather_armor")
        .and_then(|item| item.armor.as_ref())
        .expect("typed armor");
    assert_eq!(armor.block_rating, 2);
    assert_eq!(armor.damage_reduction.crushing, 4);
}

#[test]
fn capability_rejects_obsolete_weapon_hands_field() {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.selected_by_runtime_id_mut("items", "training_knife")["weapon"]["hands_required"] =
        json!(1);
    let error = parts.decode().expect_err("obsolete field must fail");
    assert!(error.contains("unknown field `hands_required`"), "{error}");
}

#[test]
fn capability_rejects_armor_on_consumable() {
    let mut parts = ContentParts::tracked("balm_cache", "profile/balm_cache");
    parts.selected_by_runtime_id_mut("items", "healing_balm")["armor"] = json!({
        "block_rating": 1,
        "encumbrance": 0,
        "damage_reduction": {"cutting": 0, "piercing": 0, "crushing": 0}
    });
    let error = parts
        .definition()
        .expect_err("armor on consumable must fail");
    assert!(
        error.contains("armor is invalid for consumable items"),
        "{error}"
    );
}

#[test]
fn capability_rejects_both_class_allow_and_deny() {
    let mut parts = ContentParts::tracked("fidelity_gallery", "profile/fidelity_gallery");
    let capability = &mut parts.selected_by_runtime_id_mut("items", "training_sword")["capability"];
    capability["class_restrict_allow"] = json!(["fighter"]);
    capability["class_restrict_deny"] = json!(["wizard"]);
    let error = parts.decode().expect_err("obsolete class fields must fail");
    assert!(error.contains("unknown field"), "{error}");
}

#[test]
fn belt_and_sack_capability_is_inactive_until_moved_to_active_position() {
    let mut parts = ContentParts::tracked("character_sheet", "profile/character_sheet");
    parts.push_selected(
        "items",
        "item/ember_charm/item_capability_test",
        json!({
            "id": "ember_charm",
            "kind": "accessory",
            "name": "Ember Charm",
            "valid_placements": ["belt_side", "sack", "neck"],
            "capability": {
                "resistance_boosts": [{"tag": "ember", "bonus_twentieths": 3}]
            },
            "economy": {"unit_burden": 1}
        }),
    );
    parts.item_instances_mut()["ember_charm"] = json!({
        "definition_id": "ember_charm",
        "binding": {"state": "unrestricted"}
    });
    parts.actors_mut()[0]["carried"]["items"]
        .as_array_mut()
        .expect("carried items")
        .push(json!({"item_instance_id": "ember_charm", "position": "belt_1"}));
    let mut engine = parts.engine(7).expect("capability graph");

    assert!(
        engine
            .actor_resistance_boosts(&"player".into())
            .expect("belt query")
            .is_empty()
    );
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "ember_charm".to_string(),
                destination: ItemMoveDestination::Carried {
                    position: CarriedPosition::SackItem1,
                },
            },
        )
        .expect("belt to sack");
    assert!(
        engine
            .actor_resistance_boosts(&"player".into())
            .expect("sack query")
            .is_empty()
    );
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "ember_charm".to_string(),
                destination: ItemMoveDestination::Carried {
                    position: CarriedPosition::Neck,
                },
            },
        )
        .expect("sack to neck");
    let boosts = engine
        .actor_resistance_boosts(&"player".into())
        .expect("active query");
    assert_eq!(boosts.len(), 1);
    assert_eq!(boosts[0].tag, "ember");
    assert_eq!(boosts[0].bonus_twentieths, 3);
}
