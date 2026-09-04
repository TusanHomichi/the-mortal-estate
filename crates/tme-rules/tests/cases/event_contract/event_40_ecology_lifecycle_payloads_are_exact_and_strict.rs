use super::*;

#[test]
fn event_40_ecology_lifecycle_payloads_are_exact_and_strict() {
    assert_eq!(EVENT_CONTRACT_VERSION, 41);
    let cases = [
        (
            Event::EcologyResetScheduled {
                site_id: "gallery_foragers".to_string(),
                generation: 0,
                member_ids: vec!["forager_west".to_string(), "forager_east".to_string()],
                due_at: LogicalTime::new(61),
                policy: EcologyLifecyclePolicyV1::FullSite,
            },
            json!({
                "ecology_reset_scheduled": {
                    "site_id": "gallery_foragers",
                    "generation": 0,
                    "member_ids": ["forager_west", "forager_east"],
                    "due_at": {"milliseconds": 183000},
                    "policy": "full_site"
                }
            }),
        ),
        (
            Event::EcologyReset {
                site_id: "gallery_foragers".to_string(),
                from_generation: 0,
                to_generation: 1,
                member_ids: vec!["forager_west".to_string()],
                policy: EcologyLifecyclePolicyV1::SlotReplenishment,
            },
            json!({
                "ecology_reset": {
                    "site_id": "gallery_foragers",
                    "from_generation": 0,
                    "to_generation": 1,
                    "member_ids": ["forager_west"],
                    "policy": "slot_replenishment"
                }
            }),
        ),
        (
            Event::EcologyActorSpawned {
                site_id: "gallery_foragers".to_string(),
                member_id: "forager_west".to_string(),
                generation: 1,
                actor_id: "ecology:gallery_foragers:forager_west:1".into(),
                actor_definition_id: "actor/first_land/kobold_forager".to_string(),
                location: WorldPosition::new(
                    "fixture_realm",
                    "last_light_lodge",
                    Coord { x: 41, y: 9 },
                ),
            },
            json!({
                "ecology_actor_spawned": {
                    "site_id": "gallery_foragers",
                    "member_id": "forager_west",
                    "generation": 1,
                    "actor_id": "ecology:gallery_foragers:forager_west:1",
                    "actor_definition_id": "actor/first_land/kobold_forager",
                    "location": {
                        "realm": "fixture_realm",
                        "level": "last_light_lodge",
                        "position": {"x": 41, "y": 9}
                    }
                }
            }),
        ),
    ];

    for (event, expected) in cases {
        let serialized = serde_json::to_value(&event).expect("ecology event serializes");
        assert_eq!(serialized, expected);
        assert_eq!(
            serde_json::from_value::<Event>(serialized.clone()).expect("ecology event decodes"),
            event
        );

        let mut unknown = serialized.clone();
        unknown
            .as_object_mut()
            .expect("event envelope")
            .values_mut()
            .next()
            .expect("event payload")
            .as_object_mut()
            .expect("event payload object")
            .insert("unknown".to_string(), json!(true));
        assert!(serde_json::from_value::<Event>(unknown).is_err());

        let mut missing = serialized;
        missing
            .as_object_mut()
            .expect("event envelope")
            .values_mut()
            .next()
            .expect("event payload")
            .as_object_mut()
            .expect("event payload object")
            .remove("site_id");
        assert!(serde_json::from_value::<Event>(missing).is_err());
    }
}

#[test]
fn event_34_world_transition_requires_typed_navigation() {
    let event = Event::WorldTransition {
        actor_id: "player".into(),
        actor: "Observer".to_string(),
        from: WorldPosition::new("realm_0", "gallery", Coord { x: 2, y: 2 }),
        to: WorldPosition::new("realm_0", "lower_gallery", Coord { x: 2, y: 2 }),
        navigation: NavigationKind::Stairs {
            direction: VerticalDirection::Down,
        },
    };
    let value = serde_json::to_value(&event).expect("transition event serializes");
    assert_eq!(
        value["world_transition"]["navigation"],
        json!({"stairs": {"direction": "down"}})
    );

    let mut missing = value;
    missing["world_transition"]
        .as_object_mut()
        .expect("event body")
        .remove("navigation");
    assert!(serde_json::from_value::<Event>(missing).is_err());
}

#[test]
fn event_37_self_defense_stable_identities_are_atomic_on_deserialization() {
    let established = json!({
        "self_defense_changed": {
            "victim_actor_id": "victim",
            "victim_character_id": "character:victim",
            "before_attacker_character_id": null,
            "after_attacker_character_id": "character:attacker",
            "reason": "established"
        }
    });
    let replaced = json!({
        "self_defense_changed": {
            "victim_actor_id": "victim",
            "victim_character_id": "character:victim",
            "before_attacker_character_id": "character:old-attacker",
            "after_attacker_character_id": "character:attacker",
            "reason": "replaced"
        }
    });
    let cleared = json!({
        "self_defense_changed": {
            "victim_actor_id": "victim",
            "victim_character_id": "character:victim",
            "before_attacker_character_id": "character:attacker",
            "after_attacker_character_id": null,
            "reason": "cleared"
        }
    });
    for value in [&established, &replaced, &cleared] {
        let event: Event = serde_json::from_value(value.clone())
            .expect("complete self-defense identity should deserialize");
        assert_eq!(
            serde_json::to_value(event).expect("self-defense event should serialize"),
            *value
        );
    }

    let mut invalid = Vec::new();
    let mut established_with_before = replaced.clone();
    established_with_before["self_defense_changed"]["reason"] = json!("established");
    invalid.push(established_with_before);
    let mut established_without_after = established.clone();
    established_without_after["self_defense_changed"]["after_attacker_character_id"] =
        serde_json::Value::Null;
    invalid.push(established_without_after);
    let mut replaced_without_before = established.clone();
    replaced_without_before["self_defense_changed"]["reason"] = json!("replaced");
    invalid.push(replaced_without_before);
    let mut replaced_without_after = replaced.clone();
    replaced_without_after["self_defense_changed"]["after_attacker_character_id"] =
        serde_json::Value::Null;
    invalid.push(replaced_without_after);
    let mut cleared_without_before = established.clone();
    cleared_without_before["self_defense_changed"]["after_attacker_character_id"] =
        serde_json::Value::Null;
    cleared_without_before["self_defense_changed"]["reason"] = json!("cleared");
    invalid.push(cleared_without_before);
    let mut cleared_with_after = replaced.clone();
    cleared_with_after["self_defense_changed"]["reason"] = json!("cleared");
    invalid.push(cleared_with_after);
    let mut missing_nullable = established.clone();
    missing_nullable["self_defense_changed"]
        .as_object_mut()
        .expect("self-defense payload")
        .remove("before_attacker_character_id");
    invalid.push(missing_nullable);
    let mut missing_after = established.clone();
    missing_after["self_defense_changed"]
        .as_object_mut()
        .expect("self-defense payload")
        .remove("after_attacker_character_id");
    invalid.push(missing_after);
    let mut unknown = established.clone();
    unknown["self_defense_changed"]["private_relation"] = json!(true);
    invalid.push(unknown);

    for value in invalid {
        assert!(
            serde_json::from_value::<Event>(value).is_err(),
            "invalid self-defense identity was accepted"
        );
    }
}

#[test]
fn event_34_warmed_lifecycle_and_path_failure_shapes_are_exact() {
    assert_eq!(EVENT_CONTRACT_VERSION, 41);
    let events = vec![
        Event::SpellWarmed {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            spell_id: "charged_path".to_string(),
            spell_name: "Charged Path".to_string(),
            warmed_at: LogicalTime::new(4),
            ready_at: LogicalTime::new(5),
        },
        Event::WarmedSpellReady {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            spell_id: "charged_path".to_string(),
            spell_name: "Charged Path".to_string(),
            ready_at: LogicalTime::new(5),
        },
        Event::WarmedSpellCast {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            spell_id: "charged_path".to_string(),
            spell_name: "Charged Path".to_string(),
            target: Some(SpellTarget::Path {
                directions: vec![Direction::East, Direction::Northeast],
            }),
        },
        Event::SpellFizzled {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            spell_id: "charged_path".to_string(),
            spell_name: "Charged Path".to_string(),
            cause: SpellFizzleCause::Damage {
                applied_damage: 3,
                hp_before: 10,
            },
        },
        Event::SpellCastFailed {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            spell_id: "charged_path".to_string(),
            spell_name: "Charged Path".to_string(),
            target: Some(SpellTarget::Path {
                directions: vec![Direction::West, Direction::West],
            }),
            failure: SpellCastFailure::InvalidPath {
                reason: SpellPathFailureReason::OutOfBounds,
            },
            mp_cost: Some(4),
            stamina_cost: Some(2),
        },
        Event::SpellCastCommitted {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            spell_id: "path_mark".to_string(),
            spell_name: "Path Mark".to_string(),
            target: Some(SpellTarget::Path {
                directions: vec![Direction::East],
            }),
            casting_method: SpellCastingMethod::Direct,
            mp_cost: Some(3),
            stamina_cost: Some(1),
        },
        Event::SpellCastStubbed {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            spell_id: "path_mark".to_string(),
            spell_name: "Path Mark".to_string(),
            target: Some(SpellTarget::Path {
                directions: vec![Direction::East],
            }),
            casting_method: SpellCastingMethod::Direct,
            lane: "wizard_magic".to_string(),
            mp_cost: Some(3),
            stamina_cost: Some(1),
        },
    ];
    let value = serde_json::to_value(&events).expect("lifecycle events serialize");
    assert_eq!(
        value[0]["spell_warmed"],
        json!({
            "actor_id": "player", "actor": "Wiz",
            "spell_id": "charged_path", "spell_name": "Charged Path",
            "warmed_at": {"milliseconds": 12000}, "ready_at": {"milliseconds": 15000}
        })
    );
    assert_eq!(
        value[1]["warmed_spell_ready"]["ready_at"],
        json!({"milliseconds": 15000})
    );
    assert_eq!(
        value[2]["warmed_spell_cast"]["target"],
        json!({"path": {"directions": ["east", "northeast"]}})
    );
    assert_eq!(
        value[3]["spell_fizzled"]["cause"],
        json!({"kind": "damage", "applied_damage": 3, "hp_before": 10})
    );
    assert_eq!(
        value[4]["spell_cast_failed"]["failure"],
        json!({"kind": "invalid_path", "reason": "out_of_bounds"})
    );
    assert_eq!(
        value[5]["spell_cast_committed"],
        json!({
            "actor_id": "player", "actor": "Wiz",
            "spell_id": "path_mark", "spell_name": "Path Mark",
            "target": {"path": {"directions": ["east"]}},
            "casting_method": "direct", "mp_cost": 3, "stamina_cost": 1
        })
    );
    assert_eq!(value[6]["spell_cast_stubbed"]["casting_method"], "direct");
    for event in events {
        let encoded = serde_json::to_value(&event).expect("serialize event");
        let decoded: Event = serde_json::from_value(encoded).expect("round-trip event");
        assert_eq!(decoded, event);
    }
}

#[test]
fn event_34_transaction_receipt_shape_is_exact_and_strict() {
    assert_eq!(EVENT_CONTRACT_VERSION, 41);
    let event = Event::TransactionCommitted {
        actor_id: "player".into(),
        actor: "Delver".to_string(),
        source: TransactionSourceV1::ServiceTransaction {
            service_id: "clerk".to_string(),
            capability_id: "exchanges".to_string(),
            transaction_id: "token_for_badge".to_string(),
        },
        costs: vec![
            TransactionCostReceiptV1::SelectedCarriedItem {
                item_instance_id: "token_stack".to_string(),
                item_definition_id: "token".to_string(),
                consumed_quantity: 1,
                remaining_quantity: 1,
            },
            TransactionCostReceiptV1::CarriedGold {
                position: CarriedGoldPosition::Sack,
                amount: 5,
                before: 20,
                after: 15,
            },
        ],
        rewards: vec![
            TransactionRewardReceiptV1::Experience {
                amount: 25,
                total_xp: 325,
            },
            TransactionRewardReceiptV1::Item {
                item_instance_id: "trail_badge".to_string(),
                item_definition_id: "badge".to_string(),
                position: CarriedPosition::Belt1,
                quantity: 1,
            },
        ],
    };
    let value = serde_json::to_value(&event).expect("transaction event serializes");
    assert_eq!(
        value,
        json!({"transaction_committed": {
            "actor_id": "player",
            "actor": "Delver",
            "source": {
                "kind": "service_transaction",
                "service_id": "clerk",
                "capability_id": "exchanges",
                "transaction_id": "token_for_badge"
            },
            "costs": [
                {
                    "kind": "selected_carried_item",
                    "item_instance_id": "token_stack",
                    "item_definition_id": "token",
                    "consumed_quantity": 1,
                    "remaining_quantity": 1
                },
                {"kind": "carried_gold", "position": "sack", "amount": 5, "before": 20, "after": 15}
            ],
            "rewards": [
                {"kind": "experience", "amount": 25, "total_xp": 325},
                {
                    "kind": "item",
                    "item_instance_id": "trail_badge",
                    "item_definition_id": "badge",
                    "position": "belt_1",
                    "quantity": 1
                }
            ]
        }})
    );
    assert_eq!(
        serde_json::from_value::<Event>(value.clone()).expect("event round trips"),
        event
    );

    let mut obsolete = value;
    obsolete["transaction_committed"]["source"]["provider_id"] = json!("clerk");
    assert!(serde_json::from_value::<Event>(obsolete).is_err());
}

#[test]
fn event_34_restoration_source_rewards_and_resource_event_are_exact_and_strict() {
    let event = Event::TransactionCommitted {
        actor_id: "payer".into(),
        actor: "Payer".to_string(),
        source: TransactionSourceV1::RestorationService {
            service_id: "temple".to_string(),
            capability_id: "restoration".to_string(),
            operation_id: "priest_resurrection".to_string(),
            corpse_id: Some(CorpseId::parse("corpse:7").expect("corpse ID")),
        },
        costs: vec![],
        rewards: vec![
            TransactionRewardReceiptV1::ResourceRestored {
                target_actor_id: "payer".into(),
                resource: ResourceKind::Hp,
                before: 4,
                after: 12,
                maximum: 12,
            },
            TransactionRewardReceiptV1::StatusCured {
                target_actor_id: "payer".into(),
                status: RestorationStatusKind::Poison,
                removed_count: 2,
            },
            TransactionRewardReceiptV1::PriestResurrection {
                target_actor_id: "fallen".into(),
                corpse_id: CorpseId::parse("corpse:7").expect("corpse ID"),
                method: ResurrectionMethod::Priest,
                current_hp: 11,
                current_stamina: 9,
            },
        ],
    };
    let value = serde_json::to_value(&event).expect("restoration event serializes");
    assert_eq!(
        value,
        json!({"transaction_committed": {
            "actor_id": "payer",
            "actor": "Payer",
            "source": {
                "kind": "restoration_service",
                "service_id": "temple",
                "capability_id": "restoration",
                "operation_id": "priest_resurrection",
                "corpse_id": "corpse:7"
            },
            "costs": [],
            "rewards": [
                {
                    "kind": "resource_restored",
                    "target_actor_id": "payer",
                    "resource": "hp",
                    "before": 4,
                    "after": 12,
                    "maximum": 12
                },
                {
                    "kind": "status_cured",
                    "target_actor_id": "payer",
                    "status": "poison",
                    "removed_count": 2
                },
                {
                    "kind": "priest_resurrection",
                    "target_actor_id": "fallen",
                    "corpse_id": "corpse:7",
                    "method": "priest",
                    "current_hp": 11,
                    "current_stamina": 9
                }
            ]
        }})
    );
    assert_eq!(
        serde_json::from_value::<Event>(value.clone()).expect("event round trips"),
        event
    );

    let mut missing_nullable = value.clone();
    missing_nullable["transaction_committed"]["source"]
        .as_object_mut()
        .expect("source object")
        .remove("corpse_id");
    assert!(serde_json::from_value::<Event>(missing_nullable).is_err());

    let resource_event = Event::ResourceRestored {
        actor_id: "payer".into(),
        actor: "Payer".to_string(),
        resource: ResourceKind::Mp,
        before: 2,
        after: 8,
        maximum: 8,
    };
    let resource_value = serde_json::to_value(&resource_event).expect("resource event serializes");
    assert_eq!(
        resource_value,
        json!({"resource_restored": {
            "actor_id": "payer",
            "actor": "Payer",
            "resource": "mp",
            "before": 2,
            "after": 8,
            "maximum": 8
        }})
    );
    assert_eq!(
        serde_json::from_value::<Event>(resource_value).expect("resource event round trips"),
        resource_event
    );
}

#[test]
fn event_34_storage_locations_receipts_and_offer_events_are_exact_and_strict() {
    assert_eq!(EVENT_CONTRACT_VERSION, 41);

    let item_locations = [
        json!({
            "kind": "locker",
            "vault_id": "shared_vault",
            "owner_character_id": "character:owner"
        }),
        json!({
            "kind": "offered",
            "sender_character_id": "character:sender",
            "recipient_character_id": "character:recipient",
            "source_position": "right_hand"
        }),
    ];
    for value in item_locations {
        let decoded =
            serde_json::from_value::<ItemLocationViewV1>(value.clone()).expect("ED item location");
        assert_eq!(serde_json::to_value(decoded).unwrap(), value);
        let mut unknown = value;
        unknown["legacy"] = json!(true);
        assert!(serde_json::from_value::<ItemLocationViewV1>(unknown).is_err());
    }

    let gold_locations = [
        json!({"kind": "carried", "actor_id": "player", "position": "left_hand"}),
        json!({
            "kind": "bank",
            "bank_id": "shared_bank",
            "character_id": "character:owner"
        }),
    ];
    for value in gold_locations {
        let decoded =
            serde_json::from_value::<GoldLocationViewV1>(value.clone()).expect("ED gold location");
        assert_eq!(serde_json::to_value(decoded).unwrap(), value);
        let mut unknown = value;
        unknown["legacy"] = json!(true);
        assert!(serde_json::from_value::<GoldLocationViewV1>(unknown).is_err());
    }

    let sources = [
        json!({
            "kind": "bank_deposit",
            "service_id": "counter_a",
            "capability_id": "bank",
            "bank_id": "shared_bank",
            "gold_pile_id": "gold:1"
        }),
        json!({
            "kind": "bank_withdrawal",
            "service_id": "counter_b",
            "capability_id": "bank",
            "bank_id": "shared_bank",
            "amount": 40
        }),
    ];
    for value in sources {
        let decoded = serde_json::from_value::<TransactionSourceV1>(value.clone())
            .expect("ED transaction source");
        assert_eq!(serde_json::to_value(decoded).unwrap(), value);
        let mut unknown = value;
        unknown["legacy"] = json!(true);
        assert!(serde_json::from_value::<TransactionSourceV1>(unknown).is_err());
    }

    let costs = [
        json!({
            "kind": "ground_gold_pile",
            "gold_pile_id": "gold:1",
            "amount": 125,
            "from": {
                "kind": "ground",
                "gold_pile_id": "gold:1",
                "location": {
                    "realm": "realm_0",
                    "level": "bank_hall",
                    "position": {"x": 1, "y": 1}
                }
            }
        }),
        json!({
            "kind": "bank_balance",
            "bank_id": "shared_bank",
            "character_id": "character:owner",
            "amount": 40,
            "before": 125,
            "after": 85
        }),
    ];
    for value in costs {
        let decoded = serde_json::from_value::<TransactionCostReceiptV1>(value.clone())
            .expect("ED transaction cost");
        assert_eq!(serde_json::to_value(decoded).unwrap(), value);
        let mut unknown = value;
        unknown["legacy"] = json!(true);
        assert!(serde_json::from_value::<TransactionCostReceiptV1>(unknown).is_err());
    }

    let rewards = [
        json!({
            "kind": "bank_balance",
            "bank_id": "shared_bank",
            "character_id": "character:owner",
            "amount": 125,
            "before": 0,
            "after": 125
        }),
        json!({
            "kind": "ground_gold_pile",
            "gold_pile_id": "gold:2",
            "amount": 40,
            "to": {
                "kind": "ground",
                "gold_pile_id": "gold:2",
                "location": {
                    "realm": "realm_0",
                    "level": "bank_hall",
                    "position": {"x": 1, "y": 1}
                }
            }
        }),
    ];
    for value in rewards {
        let decoded = serde_json::from_value::<TransactionRewardReceiptV1>(value.clone())
            .expect("ED transaction reward");
        assert_eq!(serde_json::to_value(decoded).unwrap(), value);
        let mut unknown = value;
        unknown["legacy"] = json!(true);
        assert!(serde_json::from_value::<TransactionRewardReceiptV1>(unknown).is_err());
    }

    let events = [
        json!({"bank_balance_changed": {
            "actor_id": "player",
            "actor": "Delver",
            "bank_id": "shared_bank",
            "character_id": "character:owner",
            "amount": 125,
            "before": 0,
            "after": 125,
            "reason": "deposit"
        }}),
        json!({"item_offer_created": {
            "actor_id": "sender",
            "actor": "Sender",
            "item_instance_id": "keepsake",
            "item_definition_id": "keepsake",
            "item": "Keepsake",
            "sender_character_id": "character:sender",
            "recipient_character_id": "character:recipient",
            "source_position": "right_hand"
        }}),
        json!({"item_offer_completed": {
            "actor_id": "recipient",
            "actor": "Recipient",
            "item_instance_id": "keepsake",
            "item_definition_id": "keepsake",
            "item": "Keepsake",
            "sender_character_id": "character:sender",
            "recipient_character_id": "character:recipient",
            "destination": "left_hand",
            "reason": "accepted"
        }}),
    ];
    for value in events {
        let decoded = serde_json::from_value::<Event>(value.clone()).expect("ED event");
        assert_eq!(serde_json::to_value(decoded).unwrap(), value);
        let mut unknown = value;
        unknown
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap()["legacy"] = json!(true);
        assert!(serde_json::from_value::<Event>(unknown).is_err());
    }
}
