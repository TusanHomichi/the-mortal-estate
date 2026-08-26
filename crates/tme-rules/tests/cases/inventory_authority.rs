use crate::support::content_parts::ContentParts;
use serde_json::{Map, Value, json};
use tme_rules::{
    ACTION_CONTEXT_CONTRACT_VERSION, COMMAND_CONTRACT_VERSION, CarriedPosition, Direction,
    EVENT_CONTRACT_VERSION, Engine, Event, ItemBindingState, ItemHolderId, ItemLocation,
    ItemLocationViewV1, ItemMoveDestination, ItemRelocationReason,
    OBSERVED_SNAPSHOT_CONTRACT_VERSION, PATH_PREVIEW_CONTRACT_VERSION, PlayerIntent,
    SNAPSHOT_CONTRACT_VERSION, TRACE_CONTRACT_VERSION, TRACE_V2_CONTRACT_VERSION, WorldPosition,
};

fn move_item(item_instance_id: &str, destination: ItemMoveDestination) -> PlayerIntent {
    PlayerIntent::MoveItem {
        item_instance_id: item_instance_id.to_string(),
        destination,
    }
}

fn move_to(item_instance_id: &str, position: CarriedPosition) -> PlayerIntent {
    move_item(item_instance_id, ItemMoveDestination::Carried { position })
}

fn move_to_ground(item_instance_id: &str) -> PlayerIntent {
    move_item(item_instance_id, ItemMoveDestination::GroundHere)
}

fn ground_items(items: &[(&str, i32, i32)]) -> Value {
    Value::Array(
        items
            .iter()
            .map(|(item_instance_id, x, y)| {
                json!({
                    "item_instance_id": item_instance_id,
                    "location": {
                        "realm": "realm_0", "level": "room_0", "position": {"x": x, "y": y}
                    }
                })
            })
            .collect(),
    )
}

fn inventory_parts(ground_items: Value) -> ContentParts {
    let item_instances = ground_items
        .as_array()
        .expect("ground items")
        .iter()
        .map(|ground| {
            let id = ground["item_instance_id"].as_str().expect("instance id");
            (
                id.to_string(),
                json!({"definition_id": id, "binding": {"state": "unrestricted"}}),
            )
        })
        .collect::<Map<_, _>>();

    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.profile_value_mut()["items"] = json!([]);
    for (key, item) in [
        (
            "item/hemp_rope/inventory_authority",
            json!({
                "id": "hemp_rope", "kind": "gear", "name": "Hemp Rope",
                "valid_placements": ["hand", "sack"], "economy": {"unit_burden": 1}
            }),
        ),
        (
            "item/waterskin/inventory_authority",
            json!({
                "id": "waterskin", "kind": "gear", "name": "Waterskin",
                "valid_placements": ["hand", "sack"], "economy": {"unit_burden": 1}
            }),
        ),
        (
            "item/elm_bow/inventory_authority",
            json!({
                "id": "elm_bow", "kind": "weapon", "name": "Elm Bow",
                "valid_placements": ["hand", "belt_back", "sack"],
                "weapon": {
                    "skill_track_id": "bow", "default_attack_mode": "shoot",
                    "attack_modes": [{"mode": "shoot", "maximum_range": 3, "damage_kind": "piercing"}],
                    "cooldown_units": 1, "combat_add_rating": 1, "handedness": "bow",
                    "block_value": 0, "nocking": {"unloads_on_movement": true}
                },
                "economy": {"unit_burden": 1}
            }),
        ),
    ] {
        parts.push_selected("items", key, item);
    }
    *parts.item_instances_mut() = Value::Object(item_instances);
    *parts.ground_items_mut() = ground_items;
    parts.actors_mut()[0]["carried"] = json!({
        "items": [], "gold": {"left_hand": 0, "right_hand": 0, "sack": 7}
    });
    parts
}

fn engine_from_ground_items(ground_items: Value) -> Engine {
    inventory_parts(ground_items)
        .engine(7)
        .expect("inventory graph should start")
}

#[test]
fn engine_seeds_authored_ground_items_and_inspection_order() {
    let mut engine =
        engine_from_ground_items(ground_items(&[("hemp_rope", 1, 1), ("waterskin", 2, 1)]));
    assert_eq!(engine.world().ground_items.len(), 2);
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Inspect)
        .unwrap();
    let inspected = events
        .iter()
        .find_map(|event| match event {
            Event::Inspected { ground_items, .. } => Some(ground_items),
            _ => None,
        })
        .expect("inspect event");
    assert_eq!(inspected[0].item.name, "Hemp Rope");
    assert_eq!(inspected[0].direction, None);
    assert_eq!(inspected[1].item.name, "Waterskin");
    assert_eq!(inspected[1].direction, Some(Direction::East));
}

#[test]
fn move_item_relocates_ground_item_to_exact_sack_position() {
    let mut engine = engine_from_ground_items(ground_items(&[("hemp_rope", 1, 1)]));
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            move_to("hemp_rope", CarriedPosition::SackItem3),
        )
        .expect("move");
    assert!(matches!(
        events.iter().find(|event| matches!(event, Event::ItemRelocated { .. })),
        Some(Event::ItemRelocated {
            actor_id,
            item_instance_id,
            from: ItemLocationViewV1::Ground { .. },
            to: ItemLocationViewV1::Carried { position: CarriedPosition::SackItem3, .. },
            reason: ItemRelocationReason::PlayerMove,
            ..
        }) if actor_id == "player" && item_instance_id == "hemp_rope"
    ));
    assert_eq!(
        engine.item_location("hemp_rope").unwrap(),
        ItemLocation::Carried {
            holder: ItemHolderId::TransientActor("player".into()),
            position: CarriedPosition::SackItem3,
        }
    );
}

#[test]
fn move_item_rejects_unknown_or_out_of_reach_sources_atomically() {
    let mut engine = engine_from_ground_items(ground_items(&[("waterskin", 2, 1)]));
    let before = engine.world().clone();
    assert!(
        engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                move_to("missing_item", CarriedPosition::SackItem1)
            )
            .expect_err("unknown")
            .message()
            .contains("missing_item")
    );
    assert_eq!(engine.world(), &before);
    assert!(
        engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                move_to("waterskin", CarriedPosition::SackItem1)
            )
            .expect_err("distant")
            .message()
            .contains("not in reach")
    );
    assert_eq!(engine.world(), &before);
}

#[test]
fn move_item_rejects_occupied_position_and_invalid_placement_atomically() {
    let mut engine =
        engine_from_ground_items(ground_items(&[("hemp_rope", 1, 1), ("waterskin", 1, 1)]));
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            move_to("hemp_rope", CarriedPosition::SackItem1),
        )
        .unwrap();
    let before = engine.world().clone();
    assert!(
        engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                move_to("waterskin", CarriedPosition::SackItem1)
            )
            .expect_err("occupied")
            .message()
            .contains("occupied")
    );
    assert_eq!(engine.world(), &before);
    assert!(
        engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                move_to("waterskin", CarriedPosition::Head)
            )
            .expect_err("placement")
            .message()
            .contains("cannot occupy")
    );
    assert_eq!(engine.world(), &before);
}

#[test]
fn move_item_moves_between_carried_positions_and_ground() {
    let mut engine = engine_from_ground_items(ground_items(&[("elm_bow", 1, 1)]));
    let holder = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .unwrap()
        .item_holder_id();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            move_to("elm_bow", CarriedPosition::RightHand),
        )
        .unwrap();
    assert_eq!(
        engine.item_location("elm_bow").unwrap(),
        ItemLocation::Carried {
            holder,
            position: CarriedPosition::RightHand,
        }
    );
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .unwrap();
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            move_to_ground("elm_bow"),
        )
        .unwrap();
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ItemRelocated {
            to: ItemLocationViewV1::Ground { location },
            reason: ItemRelocationReason::PlayerMove,
            ..
        } if location.position == (2, 1).into()
    )));
    assert_eq!(
        engine.item_location("elm_bow").unwrap(),
        ItemLocation::Ground {
            position: WorldPosition::new("realm_0", "room_0", (2, 1).into()),
        }
    );
}

#[test]
fn show_sack_reports_positioned_items_and_gold_only() {
    let mut engine =
        engine_from_ground_items(ground_items(&[("hemp_rope", 1, 1), ("waterskin", 1, 1)]));
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            move_to("waterskin", CarriedPosition::SackItem4),
        )
        .unwrap();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            move_to("hemp_rope", CarriedPosition::RightHand),
        )
        .unwrap();
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::ShowSack)
        .unwrap();
    let (items, gold) = events
        .iter()
        .find_map(|event| match event {
            Event::SackShown { items, gold, .. } => Some((items, gold)),
            _ => None,
        })
        .expect("sack event");
    assert_eq!(*gold, 7);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].item.item_instance_id, "waterskin");
    assert_eq!(items[0].position, CarriedPosition::SackItem4);
}

#[test]
fn stable_and_transient_holder_queries_preserve_exact_position() {
    let character_engine = ContentParts::tracked("character_sheet", "profile/character_sheet")
        .engine(7)
        .unwrap();
    let stable = character_engine.world().actors[0].item_holder_id();
    assert!(matches!(stable, ItemHolderId::Character(_)));
    assert_eq!(
        character_engine.item_location("training_knife").unwrap(),
        ItemLocation::Carried {
            holder: stable,
            position: CarriedPosition::RightHand,
        }
    );
    assert_eq!(
        engine_from_ground_items(json!([])).world().actors[0].item_holder_id(),
        ItemHolderId::TransientActor("player".into())
    );
}

#[test]
fn bind_on_first_character_touch_binds_after_relocation_event() {
    let mut parts = ContentParts::tracked("character_sheet", "profile/character_sheet");
    parts.push_selected(
        "items",
        "item/tied_token/inventory_authority",
        json!({
            "id": "tied_token", "kind": "gear", "name": "Tied Token",
            "valid_placements": ["sack"], "economy": {"unit_burden": 1}
        }),
    );
    parts.item_instances_mut()["tied_token"] = json!({
        "definition_id": "tied_token", "binding": {"state": "bind_on_first_character_touch"}
    });
    parts
        .ground_items_mut()
        .as_array_mut()
        .unwrap()
        .push(json!({
            "item_instance_id": "tied_token",
            "location": {
                "realm": "realm_0", "level": "room_0", "position": {"x": 1, "y": 1}
            }
        }));
    let mut engine = parts.engine(7).unwrap();
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            move_to("tied_token", CarriedPosition::SackItem1),
        )
        .unwrap();
    let relocated = events
        .iter()
        .position(|event| matches!(event, Event::ItemRelocated { .. }))
        .unwrap();
    let bound = events
        .iter()
        .position(|event| matches!(event, Event::ItemBound { .. }))
        .unwrap();
    assert!(relocated < bound);
    let character_id = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .unwrap()
        .character_id
        .clone()
        .unwrap();
    assert_eq!(
        engine.world().item_instances["tied_token"].binding,
        ItemBindingState::Bound { character_id }
    );
}

#[test]
fn transient_actor_touch_does_not_bind_item() {
    let mut parts = inventory_parts(ground_items(&[("hemp_rope", 1, 1)]));
    parts.item_instances_mut()["hemp_rope"]["binding"] =
        json!({"state": "bind_on_first_character_touch"});
    let mut engine = parts.engine(7).unwrap();
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            move_to("hemp_rope", CarriedPosition::SackItem1),
        )
        .unwrap();
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::ItemBound { .. }))
    );
    assert_eq!(
        engine.world().item_instances["hemp_rope"].binding,
        ItemBindingState::BindOnFirstCharacterTouch
    );
}

#[test]
fn current_contract_versions_cover_inventory_surfaces() {
    assert_eq!(EVENT_CONTRACT_VERSION, 40);
    assert_eq!(SNAPSHOT_CONTRACT_VERSION, 30);
    assert_eq!(OBSERVED_SNAPSHOT_CONTRACT_VERSION, 29);
    assert_eq!(ACTION_CONTEXT_CONTRACT_VERSION, 31);
    assert_eq!(COMMAND_CONTRACT_VERSION, 26);
    assert_eq!(PATH_PREVIEW_CONTRACT_VERSION, 8);
    assert_eq!(TRACE_CONTRACT_VERSION, 1);
    assert_eq!(TRACE_V2_CONTRACT_VERSION, 2);
}
