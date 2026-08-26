use crate::support::content_parts::ContentParts;
use serde_json::json;
use tme_rules::{CarriedPosition, Engine, WoundState};

fn fixture() -> ContentParts {
    ContentParts::tracked("first_room", "profile/first_room")
}

fn engine(parts: ContentParts) -> Engine {
    parts.engine(7).expect("engine should start")
}

fn add_test_armor(parts: &mut ContentParts) {
    parts.push_selected(
        "items",
        "item/test_mail/test",
        json!({
            "id": "test_mail",
            "kind": "armor",
            "name": "Test Mail",
            "valid_placements": ["hand", "sack", "inner_armor", "outer_armor"],
            "economy": {"unit_burden": 1},
            "armor": {
                "block_rating": 2,
                "encumbrance": 1,
                "damage_reduction": {"cutting": 4, "piercing": 3, "crushing": 2}
            }
        }),
    );
    for instance_id in ["mail_inner", "mail_outer", "mail_sack"] {
        parts.item_instances_mut()[instance_id] = json!({
            "definition_id": "test_mail",
            "binding": {"state": "unrestricted"}
        });
    }
}

#[test]
fn only_worn_armor_is_aggregated_once_in_deterministic_order() {
    let mut parts = fixture();
    add_test_armor(&mut parts);
    parts.actors_mut()[0]["carried"]["items"]
        .as_array_mut()
        .unwrap()
        .extend([
            json!({"item_instance_id": "mail_outer", "position": "outer_armor"}),
            json!({"item_instance_id": "mail_sack", "position": "sack_item_1"}),
            json!({"item_instance_id": "mail_inner", "position": "inner_armor"}),
        ]);

    let snapshot = engine(parts).snapshot();
    let player = snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .unwrap();
    let armor = &player.armor_protection;
    assert_eq!(armor.sources.len(), 2, "sack armor must not contribute");
    assert_eq!(
        armor.sources[0].carried_position,
        CarriedPosition::InnerArmor
    );
    assert_eq!(
        armor.sources[1].carried_position,
        CarriedPosition::OuterArmor
    );
    assert_eq!(armor.block_rating, 4);
    assert_eq!(armor.encumbrance, 2);
    assert_eq!(armor.cutting_reduction, 8);
    assert_eq!(armor.piercing_reduction, 6);
    assert_eq!(armor.crushing_reduction, 4);
}

#[test]
fn hand_and_sack_armor_do_not_create_protection() {
    let mut parts = fixture();
    add_test_armor(&mut parts);
    parts.actors_mut()[0]["carried"]["items"] = json!([
        {"item_instance_id": "mail_sack", "position": "sack_item_1"},
        {"item_instance_id": "mail_outer", "position": "left_hand"},
        {"item_instance_id": "training_knife", "position": "right_hand"}
    ]);
    *parts.ground_items_mut() = json!([{
        "item_instance_id": "mail_inner",
        "location": {
            "realm": "realm_0", "level": "room_0", "position": {"x": 1, "y": 1}
        }
    }]);

    let snapshot = engine(parts).snapshot();
    let armor = &snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .unwrap()
        .armor_protection;
    assert!(armor.sources.is_empty());
    assert_eq!((armor.block_rating, armor.encumbrance), (0, 0));
    assert_eq!(
        (
            armor.cutting_reduction,
            armor.piercing_reduction,
            armor.crushing_reduction,
        ),
        (0, 0, 0)
    );
}

#[test]
fn wound_state_is_derived_from_current_hp_without_a_state_mirror() {
    let mut engine = engine(fixture());
    for (hp, expected) in [
        (12, WoundState::Unhurt),
        (11, WoundState::Wounded),
        (6, WoundState::BadlyWounded),
        (2, WoundState::NearDeath),
        (0, WoundState::Dead),
    ] {
        engine.world_mut().actors[0].hp = hp;
        let snapshot = engine.snapshot();
        let player = snapshot
            .actors
            .iter()
            .find(|actor| actor.id == "player")
            .unwrap();
        assert_eq!(player.wound_state, expected, "hp={hp}");
    }
}
