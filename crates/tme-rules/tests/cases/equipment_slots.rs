use crate::support::content_parts::ContentParts;
use tme_rules::{CarriedPosition, Engine, Event, ItemMoveDestination, PlayerIntent};

fn fixture_parts() -> ContentParts {
    ContentParts::tracked("equipment_slots", "profile/equipment_slots")
}

fn fixture_engine() -> Engine {
    fixture_parts().engine(7).expect("engine should start")
}

fn move_to(item_instance_id: &str, position: CarriedPosition) -> PlayerIntent {
    PlayerIntent::MoveItem {
        item_instance_id: item_instance_id.to_string(),
        destination: ItemMoveDestination::Carried { position },
    }
}

#[test]
fn exact_carried_layout_fixture_loads_authored_positions() {
    let engine = fixture_engine();
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .unwrap();

    assert_eq!(player.carried.items.len(), 2);
    assert_eq!(
        player
            .carried
            .items
            .get(&CarriedPosition::RightHand)
            .map(String::as_str),
        Some("training_knife")
    );
    assert_eq!(
        player
            .carried
            .items
            .get(&CarriedPosition::InnerArmor)
            .map(String::as_str),
        Some("leather_jerkin")
    );
}

#[test]
fn duplicate_exact_positions_fail_validation() {
    let mut value = fixture_parts();
    value.actors_mut()[0]["carried"]["items"][1]["position"] = serde_json::json!("right_hand");

    let error = match value.validated_seed() {
        Ok(_) => panic!("duplicate must fail"),
        Err(error) => error,
    };
    assert!(error.contains(
        "actors[0].carried.items[1].position duplicates actors[0].carried.items[0].position"
    ));
}

#[test]
fn snapshot_exposes_one_positioned_carried_layout() {
    let engine = fixture_engine();
    let snapshot = engine.snapshot();
    let player = snapshot
        .actors
        .iter()
        .find(|actor| actor.id == snapshot.controlled_actor_ids[0])
        .unwrap();

    assert_eq!(player.carried.items.len(), 2);
    assert_eq!(player.carried.items[0].position, CarriedPosition::RightHand);
    assert_eq!(
        player.carried.items[0].item.item_instance_id,
        "training_knife"
    );
    assert_eq!(
        player.carried.items[1].position,
        CarriedPosition::InnerArmor
    );
}

#[test]
fn item_category_and_valid_placements_parse_from_definition() {
    let mut parts = fixture_parts();
    let item = (0..parts.selected_len("items"))
        .find_map(|index| {
            let value = parts.selected_mut("items", index);
            (value["id"] == "training_knife").then(|| value.clone())
        })
        .expect("training knife definition");
    let knife: tme_rules::ItemDef = serde_json::from_value(item).expect("item definition parses");

    assert_eq!(knife.category.as_deref(), Some("weapon"));
    assert!(
        knife
            .valid_placements
            .contains(&tme_rules::ItemPlacementKind::Hand)
    );
    assert!(
        knife
            .valid_placements
            .contains(&tme_rules::ItemPlacementKind::Sack)
    );
}

#[test]
fn move_item_transitions_between_active_and_sack_positions() {
    let mut engine = fixture_engine();

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            move_to("training_knife", CarriedPosition::SackItem1),
        )
        .expect("hand to sack should succeed");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .carried
            .items[&CarriedPosition::SackItem1],
        "training_knife"
    );
    assert!(
        !engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .carried
            .items
            .contains_key(&CarriedPosition::RightHand)
    );

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            move_to("training_knife", CarriedPosition::LeftHand),
        )
        .expect("sack to open hand should succeed");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .carried
            .items[&CarriedPosition::LeftHand],
        "training_knife"
    );
}

#[test]
fn invalid_placement_does_not_mutate_layout() {
    let mut engine = fixture_engine();
    let before = engine.world().clone();

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            move_to("training_knife", CarriedPosition::Head),
        )
        .expect_err("knife cannot occupy head");
    assert!(error.message().contains("cannot occupy"));
    assert_eq!(engine.world(), &before);
}

#[test]
fn occupied_position_rejects_without_auto_swap() {
    let mut engine = fixture_engine();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "healing_balm".to_string(),
                destination: ItemMoveDestination::Carried {
                    position: CarriedPosition::SackItem1,
                },
            },
        )
        .expect("ground to sack should succeed");
    let before = engine.world().clone();

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            move_to("healing_balm", CarriedPosition::RightHand),
        )
        .expect_err("occupied hand must reject");
    assert!(error.message().contains("occupied"));
    assert_eq!(engine.world(), &before);
}

#[test]
fn moving_weapon_out_of_active_position_removes_attack_authority() {
    let mut engine = fixture_engine();
    engine.world_mut().actors[1].location.position = (3, 1).into();

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            move_to("training_knife", CarriedPosition::SackItem1),
        )
        .unwrap();
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect_err("ordinary unarmed attack cannot reach the distant target");
    assert!(error.message().contains("fight target is out of range"));
}

#[test]
fn moving_ranged_weapon_into_right_hand_grants_attack_authority() {
    let mut value = fixture_parts();
    value.push_selected(
        "items",
        "item/training_bow/test",
        serde_json::json!({
          "id": "training_bow",
          "kind": "weapon",
          "name": "Training Bow",
          "valid_placements": [
            "hand",
            "sack"
          ],
          "weapon": {
            "skill_track_id": "bow",
            "default_attack_mode": "shoot",
            "attack_modes": [{"mode": "shoot", "maximum_range": 3, "damage_kind": "piercing"}],
            "cooldown_units": 1,
            "combat_add_rating": 0,
            "handedness": "bow",
            "block_value": 0,
            "nocking": {
              "unloads_on_movement": true
            }
          },
          "economy": {
            "unit_burden": 1
          }
        }),
    );
    value.item_instances_mut()["training_bow"] = serde_json::json!({
        "definition_id": "training_bow",
        "binding": {"state": "unrestricted"}
    });
    value
        .ground_items_mut()
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "item_instance_id": "training_bow",
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 1, "y": 1}
            }
        }));
    let mut engine = value.engine(7).unwrap();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            move_to("training_knife", CarriedPosition::SackItem1),
        )
        .unwrap();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            move_to("training_bow", CarriedPosition::RightHand),
        )
        .unwrap();
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Nock)
        .expect("equipped bow should nock");

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Shoot,
                target_actor_id: "mireling".into(),
            },
        )
        .unwrap();
    assert!(events.events.iter().any(|event| matches!(
        event,
        Event::Attacked { attacker_id, .. } | Event::AttackMissed { attacker_id, .. }
            if attacker_id == "player"
    )));
}

#[test]
fn tied_initial_binding_and_absent_owner_possession_are_valid() {
    let carried_balm_value = || {
        let mut value = ContentParts::tracked("balm_cache", "profile/balm_cache");
        value
            .ground_items_mut()
            .as_array_mut()
            .expect("ground items should be an array")
            .retain(|item| item["item_instance_id"] != "healing_balm");
        value.actors_mut()[0]["carried"]["items"] = serde_json::json!([{
            "item_instance_id": "healing_balm",
            "position": "sack_item_1"
        }]);
        value
    };

    let mut absent_owner = carried_balm_value();
    absent_owner.item_instances_mut()["healing_balm"]["binding"] = serde_json::json!({
        "state": "bound",
        "character_id": "character:absent:owner"
    });
    let engine = absent_owner
        .engine(7)
        .expect("non-owner possession should be legal");
    assert_eq!(
        engine.world().actors[0]
            .carried
            .items
            .get(&CarriedPosition::SackItem1)
            .map(String::as_str),
        Some("healing_balm")
    );
    assert!(matches!(
        &engine.world().item_instances["healing_balm"].binding,
        tme_rules::ItemBindingState::Bound { character_id }
            if character_id.as_str() == "character:absent:owner"
    ));

    let mut first_touch = carried_balm_value();
    first_touch.item_instances_mut()["healing_balm"]["binding"] =
        serde_json::json!({"state": "bind_on_first_character_touch"});
    let engine = first_touch
        .engine(7)
        .expect("initial stable placement should bind");
    let player_character_id = engine.world().actors[0]
        .character_id
        .as_ref()
        .expect("balm-cache player has a stable character id");
    assert!(matches!(
        &engine.world().item_instances["healing_balm"].binding,
        tme_rules::ItemBindingState::Bound { character_id }
            if character_id == player_character_id
    ));
}
