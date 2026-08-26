use crate::support::content_parts::ContentParts;
use serde_json::json;
use tme_rules::*;

fn fixture_value() -> ContentParts {
    ContentParts::tracked("resource_movement", "profile/resource_movement")
}

fn add_modifier(
    value: &mut ContentParts,
    id: &str,
    numerator: u32,
    denominator: u32,
    carried_position: Option<&str>,
    ground: bool,
) {
    value.push_selected(
        "items",
        &format!("item/{id}/test"),
        json!({
            "id": id,
            "kind": "gear",
            "name": format!("Recovery {id}"),
            "valid_placements": [
                "hand", "ring_finger", "belt_side", "sack", "inner_armor", "outer_armor"
            ],
            "economy": {"unit_burden": 0},
            "capability": {
                "mp_recovery_multiplier": {
                    "numerator": numerator,
                    "denominator": denominator,
                    "evidence_state": "original_provisional"
                }
            },
            "review_note": "Original focused MP recovery modifier."
        }),
    );
    value.item_instances_mut()[id] = json!({
        "definition_id": id,
        "binding": {"state": "unrestricted"}
    });
    if let Some(position) = carried_position {
        value.actors_mut()[0]["carried"]["items"]
            .as_array_mut()
            .expect("carried items")
            .push(json!({"item_instance_id": id, "position": position}));
    }
    if ground {
        value
            .ground_items_mut()
            .as_array_mut()
            .expect("ground items")
            .push(json!({
                "item_instance_id": id,
                "location": {
                    "realm": "realm_0", "level": "practice_hall", "position": {"x": 1, "y": 1}
                }
            }));
    }
}

fn engine_from_value(value: ContentParts) -> Engine {
    value.engine(7).expect("MP fixture should start")
}

fn mp_events(events: &[Event]) -> Vec<&Event> {
    events
        .iter()
        .filter(|event| {
            matches!(
                event,
                Event::ResourceRegenerated {
                    resource: ResourceKind::Mp,
                    ..
                }
            )
        })
        .collect()
}

#[test]
fn worn_fractional_modifier_uses_floor_and_exact_item_receipt() {
    let mut value = fixture_value();
    value.rules_source_mut()["resources"]["mp_recovery"] = json!(3);
    value.actors_mut()[0]["character"]["resources"]["mp"] = json!(2);
    value.actors_mut()[0]["character"]["resources"]["max_mp"] = json!(20);
    add_modifier(
        &mut value,
        "fractional_robe",
        3,
        2,
        Some("outer_armor"),
        false,
    );
    let mut engine = engine_from_value(value);

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("recovery boundary");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ResourceRegenerated {
            resource: ResourceKind::Mp,
            base_amount: 3,
            multiplier_numerator: 3,
            multiplier_denominator: 2,
            rounding: MagicArithmeticRounding::Down,
            modifier_item_instance_id: Some(instance_id),
            modifier_item_definition_id: Some(definition_id),
            modifier_item: Some(item),
            modifier_item_position: Some(CarriedPosition::OuterArmor),
            amount: 4,
            current: 6,
            maximum: 20,
            ..
        } if instance_id == "fractional_robe"
            && definition_id == "fractional_robe"
            && item == "Recovery fractional_robe"
    )));
}

#[test]
fn hand_ring_belt_sack_and_ground_modifiers_are_inactive() {
    let mut value = fixture_value();
    value.rules_source_mut()["resources"]["mp_recovery"] = json!(3);
    value.actors_mut()[0]["character"]["resources"]["max_mp"] = json!(20);
    for (id, position) in [
        ("hand_item", "right_hand"),
        ("ring_item", "right_finger_1"),
        ("belt_item", "belt_1"),
        ("sack_item", "sack_item_1"),
    ] {
        add_modifier(&mut value, id, 2, 1, Some(position), false);
    }
    add_modifier(&mut value, "ground_item", 2, 1, None, true);
    let mut engine = engine_from_value(value);

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("ordinary recovery boundary");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ResourceRegenerated {
            resource: ResourceKind::Mp,
            base_amount: 3,
            multiplier_numerator: 1,
            multiplier_denominator: 1,
            modifier_item_instance_id: None,
            modifier_item_definition_id: None,
            modifier_item: None,
            modifier_item_position: None,
            amount: 3,
            ..
        }
    )));
}

#[test]
fn highest_multiplier_wins_and_equal_values_use_position_order() {
    let mut value = fixture_value();
    value.rules_source_mut()["resources"]["mp_recovery"] = json!(3);
    value.actors_mut()[0]["character"]["resources"]["max_mp"] = json!(30);
    add_modifier(&mut value, "inner_best", 2, 1, Some("inner_armor"), false);
    add_modifier(&mut value, "outer_lower", 3, 2, Some("outer_armor"), false);
    let mut engine = engine_from_value(value);
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("highest modifier boundary");
    assert!(mp_events(&events.events).iter().any(|event| matches!(
        event,
        Event::ResourceRegenerated {
            modifier_item_instance_id: Some(id),
            amount: 6,
            ..
        } if id == "inner_best"
    )));

    let mut value = fixture_value();
    value.rules_source_mut()["resources"]["mp_recovery"] = json!(3);
    value.actors_mut()[0]["character"]["resources"]["max_mp"] = json!(30);
    add_modifier(&mut value, "inner_tie", 2, 1, Some("inner_armor"), false);
    add_modifier(&mut value, "outer_tie", 2, 1, Some("outer_armor"), false);
    let mut engine = engine_from_value(value);
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("tie modifier boundary");
    assert!(mp_events(&events.events).iter().any(|event| matches!(
        event,
        Event::ResourceRegenerated {
            modifier_item_instance_id: Some(id),
            modifier_item_position: Some(CarriedPosition::InnerArmor),
            ..
        } if id == "inner_tie"
    )));
}

#[test]
fn mp_modifier_clamps_and_emits_nothing_when_already_full() {
    let mut value = fixture_value();
    value.rules_source_mut()["resources"]["mp_recovery"] = json!(3);
    value.actors_mut()[0]["character"]["resources"]["mp"] = json!(19);
    value.actors_mut()[0]["character"]["resources"]["max_mp"] = json!(20);
    add_modifier(&mut value, "clamp_robe", 2, 1, Some("outer_armor"), false);
    let mut engine = engine_from_value(value);
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("clamped recovery");
    assert!(mp_events(&events.events).iter().any(|event| matches!(
        event,
        Event::ResourceRegenerated {
            base_amount: 3,
            multiplier_numerator: 2,
            multiplier_denominator: 1,
            amount: 1,
            current: 20,
            maximum: 20,
            ..
        }
    )));

    let mut value = fixture_value();
    value.actors_mut()[0]["character"]["resources"]["mp"] = json!(8);
    add_modifier(&mut value, "full_robe", 2, 1, Some("outer_armor"), false);
    let mut engine = engine_from_value(value);
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("full recovery boundary");
    assert!(mp_events(&events.events).is_empty());
}

#[test]
fn mp_recovery_ignores_injury_and_stamina_and_preserves_resource_order() {
    let mut value = fixture_value();
    value.rules_source_mut()["resources"]["mp_recovery"] = json!(2);
    value.actor_definition_mut(0)["stats"]["hp"] = json!(5);
    value.actors_mut()[0]["character"]["resources"]["hp"] = json!(5);
    value.actors_mut()[0]["character"]["resources"]["stamina"] = json!(1);
    add_modifier(&mut value, "injured_robe", 2, 1, Some("outer_armor"), false);
    let mut engine = engine_from_value(value);
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("injured recovery boundary");
    let resources = events
        .iter()
        .filter_map(|event| match event {
            Event::ResourceRegenerated { resource, .. } => Some(*resource),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(resources, [ResourceKind::Hp, ResourceKind::Mp]);
    assert!(
        mp_events(&events.events)
            .iter()
            .any(|event| matches!(event, Event::ResourceRegenerated { amount: 4, .. }))
    );

    let mut value = fixture_value();
    value.actor_definition_mut(0)["stats"]["hp"] = json!(11);
    value.actors_mut()[0]["character"]["resources"]["hp"] = json!(11);
    value.actors_mut()[0]["character"]["resources"]["stamina"] = json!(1);
    add_modifier(&mut value, "ordered_robe", 2, 1, Some("outer_armor"), false);
    let mut engine = engine_from_value(value);
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("ordered recovery boundary");
    let resources = events
        .iter()
        .filter_map(|event| match event {
            Event::ResourceRegenerated { resource, .. } => Some(*resource),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        resources,
        [ResourceKind::Hp, ResourceKind::Mp, ResourceKind::Stamina]
    );
}

#[test]
fn oversized_recovery_is_rejected_during_seed_validation() {
    let mut value = fixture_value();
    value.rules_source_mut()["resources"]["mp_recovery"] = json!(i32::MAX);
    value.actors_mut()[0]["character"]["resources"]["mp"] = json!(0);
    value.actors_mut()[0]["character"]["resources"]["max_mp"] = json!(i32::MAX);
    add_modifier(
        &mut value,
        "overflow_robe",
        2,
        1,
        Some("outer_armor"),
        false,
    );
    let error = match value.engine(7) {
        Ok(_) => panic!("oversized recovery must reject before runtime"),
        Err(error) => error,
    };
    assert!(
        error.contains("items[3].capability.mp_recovery_multiplier result exceeds supported range")
    );
}
