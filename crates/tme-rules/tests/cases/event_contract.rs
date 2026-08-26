use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};
use tme_rules::{
    BanishResultReasonV1, COMMAND_CONTRACT_VERSION, CarriedGoldPosition, CarriedPosition, Coord,
    CorpseId, CreatureTrait, Direction, EVENT_CONTRACT_VERSION, EcologyLifecyclePolicyV1, Engine,
    Event, GoldLocationViewV1, ItemConsumptionReason, ItemLocationViewV1, LogicalTime,
    MagicArithmeticRounding, MagicPrimaryAttribute, NavigationKind, PlayerCommandV1, PlayerIntent,
    RaiseDeadResultReasonV1, ResistanceBoostSourceKind, ResourceActivity, ResourceKind,
    RestorationStatusKind, ResurrectionMethod, SpellCastClass, SpellCastFailure,
    SpellCastingMethod, SpellFizzleCause, SpellPathFailureReason, SpellResistanceMitigationMode,
    SpellTarget, TransactionCostReceiptV1, TransactionRewardReceiptV1, TransactionSourceV1,
    TransitionConcealmentRemovalReasonV1, VerticalDirection, WorldPosition,
};

fn item_contract_parts() -> ContentParts {
    ContentParts::tracked("item_instance_contract", "profile/item_instance_contract")
}

fn item_contract_engine() -> Engine {
    item_contract_parts()
        .engine(7)
        .expect("item instance contract engine should start")
}

fn event_payload(events: &[Event], event_name: &str) -> serde_json::Value {
    let serialized = serde_json::to_value(events).expect("events should serialize");
    let values = serialized.as_array().expect("events serialize as an array");
    values
        .iter()
        .find_map(|event| event.get(event_name))
        .cloned()
        .unwrap_or_else(|| panic!("missing serialized {event_name} event"))
}

#[test]
fn event_40_ecology_lifecycle_payloads_are_exact_and_strict() {
    assert_eq!(EVENT_CONTRACT_VERSION, 40);
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
                    "due_at": 61,
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
    assert_eq!(EVENT_CONTRACT_VERSION, 40);
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
            "warmed_at": 4, "ready_at": 5
        })
    );
    assert_eq!(value[1]["warmed_spell_ready"]["ready_at"], 5);
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
    assert_eq!(EVENT_CONTRACT_VERSION, 40);
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
    assert_eq!(EVENT_CONTRACT_VERSION, 40);

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

#[test]
fn event_34_spell_effect_family_shapes_are_exact_and_strict() {
    assert_eq!(EVENT_CONTRACT_VERSION, 40);
    let events = vec![
        Event::TransitionConcealed {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            spell_id: "conceal_door".to_string(),
            spell_name: "Conceal Door".to_string(),
            instance_id: "concealed_transition:1".to_string(),
            location: WorldPosition::new("realm_0", "crypt", Coord { x: 2, y: 3 }),
            remaining_rounds: 4,
        },
        Event::TransitionConcealmentRemoved {
            instance_id: "concealed_transition:1".to_string(),
            source_spell_id: "conceal_door".to_string(),
            source_actor_id: "player".into(),
            location: WorldPosition::new("realm_0", "crypt", Coord { x: 2, y: 3 }),
            reason: TransitionConcealmentRemovalReasonV1::Opened,
        },
        Event::BanishEvaluated {
            caster_id: "player".into(),
            caster: "Wiz".to_string(),
            spell_id: "banish".to_string(),
            spell_name: "Banish".to_string(),
            target_id: "demon".into(),
            target: "Summoned Demon".to_string(),
            eligible_trait: Some(CreatureTrait::Demon),
            owned_by_caster: true,
            success: true,
            reason: BanishResultReasonV1::Banished,
        },
        Event::ActorBanished {
            caster_id: "player".into(),
            caster: "Wiz".to_string(),
            spell_id: "banish".to_string(),
            spell_name: "Banish".to_string(),
            actor_id: "demon".into(),
            actor: "Summoned Demon".to_string(),
            instance_id: "summon:1".into(),
            owner_id: "player".into(),
            template_id: "demon_template".to_string(),
            location: WorldPosition::new("realm_0", "crypt", Coord { x: 4, y: 1 }),
        },
        Event::TurnUndeadResolved {
            caster_id: "player".into(),
            caster: "Wiz".to_string(),
            spell_id: "turn_undead".to_string(),
            spell_name: "Turn Undead".to_string(),
            considered_actor_ids: vec!["skeleton".into(), "wight".into()],
            moved_actor_ids: vec!["skeleton".into()],
            blocked_actor_ids: vec!["wight".into()],
        },
        Event::RaiseDeadEvaluated {
            caster_id: "player".into(),
            caster: "Wiz".to_string(),
            spell_id: "raise_dead".to_string(),
            spell_name: "Raise Dead".to_string(),
            corpse_id: Some(CorpseId::parse("corpse:7").expect("valid corpse id")),
            target_actor_id: Some("fallen_player".into()),
            magic_level: 6,
            roll_denominator: 20,
            success_threshold: 6,
            roll: Some(4),
            success: true,
            reason: RaiseDeadResultReasonV1::Resurrected,
        },
    ];
    let value = serde_json::to_value(&events).expect("effect-family events serialize");
    assert_eq!(
        value,
        json!([
            {"transition_concealed": {
                "actor_id": "player", "actor": "Wiz",
                "spell_id": "conceal_door", "spell_name": "Conceal Door",
                "instance_id": "concealed_transition:1",
                "location": {"realm": "realm_0", "level": "crypt", "position": {"x": 2, "y": 3}},
                "remaining_rounds": 4
            }},
            {"transition_concealment_removed": {
                "instance_id": "concealed_transition:1", "source_spell_id": "conceal_door",
                "source_actor_id": "player",
                "location": {"realm": "realm_0", "level": "crypt", "position": {"x": 2, "y": 3}},
                "reason": "opened"
            }},
            {"banish_evaluated": {
                "caster_id": "player", "caster": "Wiz", "spell_id": "banish",
                "spell_name": "Banish", "target_id": "demon", "target": "Summoned Demon",
                "eligible_trait": "demon", "owned_by_caster": true,
                "success": true, "reason": "banished"
            }},
            {"actor_banished": {
                "caster_id": "player", "caster": "Wiz", "spell_id": "banish",
                "spell_name": "Banish", "actor_id": "demon", "actor": "Summoned Demon",
                "instance_id": "summon:1", "owner_id": "player",
                "template_id": "demon_template",
                "location": {"realm": "realm_0", "level": "crypt", "position": {"x": 4, "y": 1}}
            }},
            {"turn_undead_resolved": {
                "caster_id": "player", "caster": "Wiz", "spell_id": "turn_undead",
                "spell_name": "Turn Undead", "considered_actor_ids": ["skeleton", "wight"],
                "moved_actor_ids": ["skeleton"], "blocked_actor_ids": ["wight"]
            }},
            {"raise_dead_evaluated": {
                "caster_id": "player", "caster": "Wiz", "spell_id": "raise_dead",
                "spell_name": "Raise Dead", "corpse_id": "corpse:7",
                "target_actor_id": "fallen_player", "magic_level": 6,
                "roll_denominator": 20, "success_threshold": 6, "roll": 4,
                "success": true, "reason": "resurrected"
            }}
        ])
    );

    for event in events {
        let encoded = serde_json::to_value(&event).expect("serialize event");
        let decoded: Event = serde_json::from_value(encoded).expect("round-trip event");
        assert_eq!(decoded, event);
    }
    let mut extra = value[2].clone();
    extra["banish_evaluated"]["summary"] = json!("legacy prose");
    assert!(serde_json::from_value::<Event>(extra).is_err());
}

#[test]
fn event_36_magic_learning_attempt_practice_reward_and_recovery_shapes_are_exact() {
    let events = vec![
        Event::SpellLearned {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            spell_id: "spark".to_string(),
            spell_name: "Spark".to_string(),
            lane: "wizard_magic".to_string(),
            skill_requirement: 1,
            learned_at_level: 2,
            gold_cost: 25,
            trainer_service_id: "trainer".to_string(),
            trainer: "Trainer".to_string(),
            spell_book_item_instance_id: "book_instance".to_string(),
            spell_book_item_definition_id: "spell_book".to_string(),
            spell_book: "Spell Book".to_string(),
            spell_book_character_id: "character:test:player".to_string(),
        },
        Event::ThaumAboveSkillEvaluated {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            spell_id: "spark".to_string(),
            spell_name: "Spark".to_string(),
            track_id: "thaumaturge_magic".to_string(),
            current_skill_level: 1,
            skill_requirement: 3,
            gap: 2,
            roll_denominator: 20,
            success_threshold: 18,
            roll: 17,
            success: true,
        },
        Event::MagicPracticeEvaluated {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            current_class_id: "wizard".to_string(),
            spell_id: "spark".to_string(),
            spell_name: "Spark".to_string(),
            track_id: "wizard_magic".to_string(),
            mp_cost: 3,
            cast_class: SpellCastClass::Character,
            primary_attribute: Some(MagicPrimaryAttribute::Intelligence),
            primary_attribute_value: Some(14),
            base_raw_points: 3,
            primary_attribute_bonus_raw_points: 1,
            total_raw_points: 4,
            risk_applied: false,
            reason: "eligible_successful_cast".to_string(),
        },
        Event::DefeatRewardEvaluated {
            target_id: "target".into(),
            target: "Target".to_string(),
            authored_experience: 13,
            actual_damage: 3,
            weighted_damage_numerator: 6,
            weighted_damage_denominator: 5,
            available_experience: 5,
            awarded_experience: 5,
            reason: "contribution_shared".to_string(),
        },
        Event::ResourceRegenerated {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            resource: ResourceKind::Mp,
            activity: ResourceActivity::Inactive,
            boundary_at: LogicalTime::new(2),
            base_amount: 3,
            multiplier_numerator: 3,
            multiplier_denominator: 2,
            rounding: MagicArithmeticRounding::Down,
            modifier_item_instance_id: Some("robe_instance".to_string()),
            modifier_item_definition_id: Some("robe".to_string()),
            modifier_item: Some("Recovery Robe".to_string()),
            modifier_item_position: Some(CarriedPosition::OuterArmor),
            amount: 4,
            current: 6,
            maximum: 20,
        },
        Event::SpellCastFailed {
            actor_id: "player".into(),
            actor: "Wiz".to_string(),
            spell_id: "spark".to_string(),
            spell_name: "Spark".to_string(),
            target: Some(SpellTarget::Actor {
                actor_id: "target".into(),
            }),
            failure: SpellCastFailure::AboveSkillAttempt,
            mp_cost: Some(3),
            stamina_cost: Some(1),
        },
    ];
    let value = serde_json::to_value(&events).expect("DX events serialize");
    assert_eq!(
        value[0]["spell_learned"],
        json!({
            "actor_id": "player", "actor": "Wiz",
            "spell_id": "spark", "spell_name": "Spark",
            "lane": "wizard_magic", "skill_requirement": 1,
            "learned_at_level": 2, "gold_cost": 25,
            "trainer_service_id": "trainer", "trainer": "Trainer",
            "spell_book_item_instance_id": "book_instance",
            "spell_book_item_definition_id": "spell_book",
            "spell_book": "Spell Book",
            "spell_book_character_id": "character:test:player"
        })
    );
    assert_eq!(
        value[1]["thaum_above_skill_evaluated"],
        json!({
            "actor_id": "player", "actor": "Wiz",
            "spell_id": "spark", "spell_name": "Spark",
            "track_id": "thaumaturge_magic", "current_skill_level": 1,
            "skill_requirement": 3, "gap": 2, "roll_denominator": 20,
            "success_threshold": 18, "roll": 17, "success": true
        })
    );
    assert_eq!(value[2]["magic_practice_evaluated"]["total_raw_points"], 4);
    assert_eq!(
        value[3]["defeat_reward_evaluated"],
        json!({
            "target_id": "target", "target": "Target",
            "authored_experience": 13, "actual_damage": 3,
            "weighted_damage_numerator": 6, "weighted_damage_denominator": 5,
            "available_experience": 5, "awarded_experience": 5,
            "reason": "contribution_shared"
        })
    );
    assert_eq!(
        value[4]["resource_regenerated"]["modifier_item_position"],
        "outer_armor"
    );
    assert_eq!(
        value[5]["spell_cast_failed"]["failure"],
        json!({"kind": "above_skill_attempt"})
    );

    for event in events {
        let encoded = serde_json::to_value(&event).expect("serialize current Event");
        let decoded: Event =
            serde_json::from_value(encoded.clone()).expect("round trip current Event");
        assert_eq!(decoded, event);
        let variant = encoded
            .as_object()
            .expect("event object")
            .keys()
            .next()
            .expect("event variant")
            .clone();
        let mut unknown = encoded;
        unknown[&variant]["legacy"] = json!(true);
        assert!(serde_json::from_value::<Event>(unknown).is_err());
    }
}

#[test]
fn event_34_spell_save_receipt_shape_is_exact_and_rejects_predecessors() {
    let event = Event::SpellSaveResolved {
        actor_id: "target".into(),
        actor: "Target".to_string(),
        location: WorldPosition::new("realm_0", "test_room", Coord { x: 2, y: 1 }),
        effect_id: "spark".to_string(),
        resistance_tag: "arcane".to_string(),
        natural_save_twentieths: 5,
        matching_bonus_twentieths: 4,
        selected_boost_source_kind: Some(ResistanceBoostSourceKind::EquippedItem),
        selected_boost_source_id: Some("ring:ward".to_string()),
        denominator: 20,
        save_twentieths: 9,
        roll: 9,
        success: true,
        mitigation_mode: Some(SpellResistanceMitigationMode::HalfDamage),
        requested_damage: Some(5),
        resolved_damage: Some(2),
    };
    let value = serde_json::to_value(&event).expect("save event serializes");
    assert_eq!(
        value,
        json!({
            "spell_save_resolved": {
                "actor_id": "target",
                "actor": "Target",
                "location": {
                    "realm": "realm_0",
                    "level": "test_room",
                    "position": {"x": 2, "y": 1}
                },
                "effect_id": "spark",
                "resistance_tag": "arcane",
                "natural_save_twentieths": 5,
                "matching_bonus_twentieths": 4,
                "selected_boost_source_kind": "equipped_item",
                "selected_boost_source_id": "ring:ward",
                "denominator": 20,
                "save_twentieths": 9,
                "roll": 9,
                "success": true,
                "mitigation_mode": "half_damage",
                "requested_damage": 5,
                "resolved_damage": 2
            }
        })
    );

    for stale in [
        json!({"effect_resisted": {}}),
        {
            let mut missing = value.clone();
            missing["spell_save_resolved"]
                .as_object_mut()
                .expect("save body")
                .remove("roll");
            missing
        },
        {
            let mut unknown = value.clone();
            unknown["spell_save_resolved"]["resistance_tags"] = json!(["arcane"]);
            unknown
        },
    ] {
        assert!(serde_json::from_value::<Event>(stale).is_err());
    }
}

#[test]
fn event_25_rejects_old_prepared_variants_string_causes_and_unknown_fields() {
    for stale in [
        json!({"spell_prepared": {}}),
        json!({"prepared_spell_released": {}}),
        json!({"prepared_spell_cleared": {}}),
    ] {
        assert!(serde_json::from_value::<Event>(stale).is_err());
    }
    assert!(
        serde_json::from_value::<Event>(json!({
            "spell_fizzled": {
                "actor_id": "player", "actor": "Wiz",
                "spell_id": "charged_path", "spell_name": "Charged Path",
                "cause": "damage"
            }
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<Event>(json!({
            "spell_cast_failed": {
                "actor_id": "player", "actor": "Wiz",
                "spell_id": "charged_path", "spell_name": "Charged Path",
                "target": {"path": {"directions": ["west"]}},
                "failure": {"kind": "invalid_path", "reason": "out_of_bounds", "summary": "old"},
                "mp_cost": 4, "stamina_cost": 2
            }
        }))
        .is_err()
    );
}

#[test]
fn movement_events_include_actor_id() {
    let mut engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("start");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::South]),
        )
        .expect("step");

    let moved = events
        .events
        .iter()
        .find(|e| matches!(e, Event::Moved { .. }));
    assert!(moved.is_some(), "must have a Moved event");
    if let Some(Event::Moved {
        actor_id, actor, ..
    }) = moved
    {
        assert!(!actor_id.is_empty(), "actor_id must be non-empty");
        assert!(!actor.is_empty(), "display name still present");
    }
}

#[test]
fn movement_events_include_world_positions() {
    let mut engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("start");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::South]),
        )
        .expect("step");

    for event in &events.events {
        match event {
            Event::Moved { from, to, .. } => {
                assert!(from.same_site(to), "directional move stays on one site");
            }
            Event::MovementBlocked {
                from, attempted, ..
            } => {
                assert!(from.same_site(attempted), "blocked move names one site");
            }
            _ => {}
        }
    }
}

#[test]
fn combat_events_include_attacker_defender_ids() {
    let mut engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("start");
    // Move toward the monster, then attack
    let _ = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .ok();
    let _ = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .ok();
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("step");

    let attacked = events
        .events
        .iter()
        .find(|e| matches!(e, Event::Attacked { .. }));
    if let Some(Event::Attacked {
        attacker_id,
        defender_id,
        attacker,
        defender,
        ..
    }) = attacked
    {
        assert!(!attacker_id.is_empty(), "attacker_id must be present");
        assert!(!defender_id.is_empty(), "defender_id must be present");
        assert_ne!(
            attacker_id, defender_id,
            "attacker and defender must have different IDs"
        );
        // Display names still present
        assert!(!attacker.is_empty());
        assert!(!defender.is_empty());
    }
}

#[test]
fn item_events_include_explicit_item_identity() {
    let mut engine = ContentParts::tracked("supply_cache", "profile/supply_cache")
        .engine(7)
        .expect("start");
    // Take an item
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "hemp_rope".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("step");

    let taken = events
        .events
        .iter()
        .find(|e| matches!(e, Event::ItemRelocated { .. }));
    assert!(taken.is_some(), "must have ItemRelocated event");
    if let Some(Event::ItemRelocated {
        item_instance_id,
        item_definition_id,
        item,
        ..
    }) = taken
    {
        assert!(
            !item_instance_id.is_empty(),
            "item_instance_id must be non-empty"
        );
        assert!(
            !item_definition_id.is_empty(),
            "item_definition_id must be non-empty"
        );
        assert!(!item.is_empty(), "display name still present");
    }
}

#[test]
fn item_transfer_events_serialize_explicit_instance_definition_and_stack_quantity() {
    let mut engine = item_contract_engine();
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "tonic_a".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("taking the tonic stack should succeed");

    let payload = event_payload(&events.events, "item_relocated");
    assert_eq!(payload["item_instance_id"], "tonic_a");
    assert_eq!(payload["item_definition_id"], "restorative_tonic");
    assert_eq!(payload["quantity"], 2);
    assert!(
        payload.get("item_id").is_none(),
        "serialized exact-item events must not retain ambiguous item_id"
    );
}

#[test]
fn item_consumption_events_serialize_consumed_and_numeric_remaining_quantities() {
    let mut engine = item_contract_engine();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "tonic_a".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("taking the tonic stack should succeed");

    let first = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("tonic_a".to_string()),
        )
        .expect("drinking one unit should succeed");
    let first_payload = event_payload(&first.events, "item_consumed");
    assert_eq!(first_payload["item_instance_id"], "tonic_a");
    assert_eq!(first_payload["item_definition_id"], "restorative_tonic");
    assert_eq!(first_payload["quantity_consumed"], 1);
    assert_eq!(first_payload["remaining_quantity"], 1);
    assert_eq!(first_payload["reason"], "drink");
    assert!(first_payload.get("item_id").is_none());

    let second = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("tonic_a".to_string()),
        )
        .expect("drinking the final unit should succeed");
    let second_payload = event_payload(&second.events, "item_consumed");
    assert_eq!(second_payload["quantity_consumed"], 1);
    assert_eq!(
        second_payload["remaining_quantity"], 0,
        "destroyed stacks still report numeric zero"
    );
}

#[test]
fn item_consumption_reasons_serialize_with_exact_names() {
    assert_eq!(
        serde_json::to_value(ItemConsumptionReason::Drink).expect("drink reason should serialize"),
        json!("drink")
    );
}

#[test]
fn event_34_item_consumed_requires_a_reason() {
    let error = serde_json::from_value::<Event>(json!({
        "item_consumed": {
            "actor_id": "player",
            "actor": "Delver",
            "item_instance_id": "tonic_a",
            "item_definition_id": "restorative_tonic",
            "item": "Restorative Tonic",
            "quantity_consumed": 1,
            "remaining_quantity": 0,
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 1, "y": 1}
            }
        }
    }))
    .expect_err("item_consumed without reason must fail deserialization");

    assert!(
        error.to_string().contains("missing field `reason`"),
        "unexpected error: {error}"
    );
}

#[test]
fn item_enchantment_event_serializes_explicit_item_and_enchantment_identity() {
    let mut parts = item_contract_parts();
    let character = &mut parts.actors_mut()[0]["character"];
    character["identity"]["base_class_id"] = json!("wizard");
    character["identity"]["current_class_id"] = json!("wizard");
    character["identity"]["display_class"] = json!("Wizard");
    character["resources"]["mp"] = json!(20);
    character["resources"]["max_mp"] = json!(20);
    character["skill_ledger"] = json!([{"track_id": "wizard_magic", "level": 1, "critique_rank": 0, "practice_points": 0, "learning_rate": 1}]);
    character["known_spells"] =
        json!([{"spell_id": "keen_edge", "lane": "wizard_magic", "learned_at_level": 1}]);
    let wizard_growth =
        parts.catalog["rules_profiles"]["rules/first_room"]["progression"]["growth_profiles"]
            .as_array()
            .expect("first-room growth profiles")
            .iter()
            .find(|profile| profile["class_id"] == "wizard")
            .expect("clean wizard profile")
            .clone();
    parts.rules_source_mut()["progression"]["growth_profiles"]
        .as_array_mut()
        .expect("growth profiles")
        .push(wizard_growth);
    let mut blade =
        parts.catalog["items"]["item/utility_blade/utility_door_secret_item_spells"].clone();
    blade["id"] = json!("training_blade");
    blade["name"] = json!("Training Blade");
    blade["economy"]["unit_value_gold"] = json!(10);
    blade["weapon"]["attack_modes"][0]["damage_kind"] = json!("piercing");
    parts.push_selected("items", "item/training_blade/event_contract", blade);
    parts.profile_value_mut()["spells"]
        .as_array_mut()
        .expect("selected spells")
        .push(json!("spell/keen_edge/utility_door_secret_item_spells"));
    parts.item_instances_mut()["blade_a"] = json!({
        "definition_id": "training_blade",
        "binding": {"state": "unrestricted"}
    });
    parts.actors_mut()[0]["carried"]["items"] =
        json!([{"item_instance_id": "blade_a", "position": "right_hand"}]);
    let mut engine = parts.engine(7).expect("engine should start");
    let command: PlayerCommandV1 = serde_json::from_value(json!({
        "contract_version": COMMAND_CONTRACT_VERSION,
        "actor_id": "player",
        "intent": {
            "cast_spell": {
                "spell_id": "keen_edge",
                "authorization": "safe",
                "target": {
                    "item": {
                        "item_instance_id": "blade_a",
                        "location": "active_equipment"
                    }
                }
            }
        }
    }))
    .expect("current item target command should deserialize");
    let intent = engine
        .command_to_actor_intent(&command)
        .expect("command should convert to an intent");
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), intent)
        .expect("enchantment should apply");

    let payload = event_payload(&events.events, "item_enchanted");
    assert_eq!(payload["item_instance_id"], "blade_a");
    assert_eq!(payload["item_definition_id"], "training_blade");
    assert_eq!(
        payload["enchantment_instance_id"],
        "spell:keen_edge:1:blade_a"
    );
    assert!(payload.get("item_id").is_none());
    assert!(payload.get("instance_id").is_none());
}

#[test]
fn final_state_actor_summary_has_id() {
    let engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("start");
    let final_events = engine.final_events();

    if let Some(Event::FinalState { actors }) = final_events.first() {
        for actor in actors {
            assert!(!actor.id.is_empty(), "ActorSummary must have id");
            assert!(!actor.name.is_empty(), "ActorSummary must have name");
        }
    }
}

#[test]
fn trace_json_includes_new_event_fields() {
    // Trace JSON round-trip: the new fields should appear in serialized output
    let mut engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("start");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::South]),
        )
        .expect("step");

    let serialized = serde_json::to_string(&events.events).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&serialized).expect("deserialize");

    // Find a moved event and check it has actor_id
    let moved_event = parsed
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v.as_object().is_some_and(|o| o.contains_key("moved")));
    assert!(moved_event.is_some(), "trace must have a moved event");
    if let Some(me) = moved_event {
        let moved_obj = &me["moved"];
        assert!(
            moved_obj["actor_id"].is_string(),
            "serialized Moved must include actor_id"
        );
        assert!(moved_obj["from"].is_object());
        assert!(moved_obj["to"].is_object());
        assert!(!moved_obj["navigation"].is_null());
    }
}

#[test]
fn hide_events_serialize_with_contract_fields() {
    let mut engine = ContentParts::tracked(
        "profession_specific_actions",
        "profile/profession_specific_actions",
    )
    .engine(7)
    .expect("start");
    engine
        .world_mut()
        .tile_effects
        .push(tme_rules::model::TileEffectState {
            source_actor_id: None,
            instance_id: "tile:shadow:1".to_string(),
            effect_id: "shadow_veil".to_string(),
            source: tme_rules::model::ActiveEffectSource {
                kind: "spell".to_string(),
                id: "shadow_veil".to_string(),
            },
            location: WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 2 }),
            kind: "terrain_overlay".to_string(),
            tags: vec!["shadow".to_string()],
            potency: 0,
            remaining_rounds: Some(3),
            passability: None,
            sight: Some("obscured".to_string()),
            hazard: None,
            move_cost: None,
            tick_interval_rounds: 1,
            last_ticked_at: tme_rules::LogicalTime::new(0),
            hostile_authority: None,
        });

    let hide_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player0"), PlayerIntent::Hide)
        .expect("hide");
    let move_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player0"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("move");
    let hidden_serialized = serde_json::to_value(&hide_events.events).expect("serialize hide");
    let hidden_entries = hidden_serialized.as_array().expect("hide event array");

    let hidden = hidden_entries
        .iter()
        .find(|entry| entry.get("actor_hidden").is_some())
        .expect("actor_hidden event");
    let hidden_obj = &hidden["actor_hidden"];
    assert!(hidden_obj["actor_id"].is_string());
    assert!(hidden_obj["actor"].is_string());
    assert!(hidden_obj["location"].is_object());
    assert!(hidden_obj["instance_id"].is_string());
    assert!(hidden_obj["effect_id"].is_string());
    assert!(hidden_obj["remaining_rounds"].is_number() || hidden_obj["remaining_rounds"].is_null());

    let broken_serialized = serde_json::to_value(&move_events.events).expect("serialize move");
    let broken_entries = broken_serialized.as_array().expect("move event array");
    let broken = broken_entries
        .iter()
        .find(|entry| entry.get("hide_broken").is_some())
        .expect("hide_broken event");
    let broken_obj = &broken["hide_broken"];
    assert!(broken_obj["actor_id"].is_string());
    assert!(broken_obj["actor"].is_string());
    assert!(broken_obj["location"].is_object());
    assert!(broken_obj["instance_id"].is_string());
    assert!(broken_obj["effect_id"].is_string());
    assert!(broken_obj["reason"].is_string());
}

#[test]
fn martial_hand_block_events_serialize_with_contract_fields() {
    let mut engine = ContentParts::tracked(
        "martial_hand_block_actions",
        "profile/martial_hand_block_actions",
    )
    .engine(7)
    .expect("start");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player0"),
            PlayerIntent::MovePath(vec![Direction::South]),
        )
        .expect("engage");

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player0"), PlayerIntent::Wait)
        .expect("wait");
    let serialized = serde_json::to_value(&events.events).expect("serialize");
    let entries = serialized.as_array().expect("event array");
    let blocked = entries
        .iter()
        .find(|entry| entry.get("attack_blocked").is_some())
        .expect("typed attack_blocked event");
    let blocked_obj = &blocked["attack_blocked"];
    assert!(blocked_obj["attacker_id"].is_string());
    assert!(blocked_obj["attacker"].is_string());
    assert!(blocked_obj["defender_id"].is_string());
    assert!(blocked_obj["defender"].is_string());
    assert!(blocked_obj["defender_location"].is_object());
    assert_eq!(blocked_obj["source"], "right_martial_hand");
    assert_eq!(blocked_obj["carried_position"], "right_hand");
    assert!(blocked_obj["item_instance_id"].is_null());
    assert_eq!(blocked_obj["block_value"], 0);
    assert_eq!(blocked_obj["skill_track_id"], "hand");
    assert_eq!(blocked_obj["skill_level"], 19);
    assert!(blocked_obj["roll"].is_number());
    assert!(blocked_obj["chance_percent"].is_number());
}

#[test]
fn duplicate_display_names_use_ids_for_disambiguation() {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.actor_definition_mut(1)["name"] = json!("Guard");
    let actors = parts
        .actors_mut()
        .as_array_mut()
        .expect("first-room actors");
    let mut guard_a = actors[1].clone();
    guard_a["id"] = json!("guard_a");
    guard_a["location"]["position"] = json!({"x": 1, "y": 2});
    let mut guard_b = guard_a.clone();
    guard_b["id"] = json!("guard_b");
    guard_b["location"]["position"] = json!({"x": 3, "y": 1});
    actors[1] = guard_a;
    actors.push(guard_b);
    let mut engine = parts.engine(7).expect("start");

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("wait");
    // Both Guards appear in events with distinct IDs
    let guard_events: Vec<_> = events
        .events
        .iter()
        .filter(|e| matches!(e, Event::AutomaticActorDecision { actor, .. } if actor == "Guard"))
        .collect();
    assert!(!guard_events.is_empty(), "at least one Guard should act");
    // Action context should list both with distinct actor_ids
    let ctx = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("ctx");
    let guards: Vec<_> = ctx
        .attack_targets
        .iter()
        .filter(|t| t.actor_name == "Guard")
        .collect();
    assert_eq!(guards.len(), 2, "both Guards should appear");
    assert_ne!(
        guards[0].actor_id, guards[1].actor_id,
        "Guards must have distinct IDs"
    );
}

#[test]
fn event_34_preserves_death_corpse_claim_and_search_payloads() {
    assert_eq!(EVENT_CONTRACT_VERSION, 40);
    let mut engine = ContentParts::tracked("death_corpse", "profile/death_corpse")
        .engine(7)
        .expect("death gallery starts");
    let defeat_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "scavenger".into(),
            },
        )
        .expect("monster defeat");

    let defeated = event_payload(&defeat_events.events, "actor_defeated");
    assert_eq!(
        defeated,
        json!({
            "actor_id": "scavenger",
            "actor": "Courtyard Scavenger",
            "kind": "monster",
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 1, "y": 1}
            },
            "cause": "physical",
            "credited_actor_id": "player",
            "loot_claim": {
                "owner": {
                    "kind": "character",
                    "id": "character:death_corpse:primary"
                },
                "basis": "killing_blow"
            }
        })
    );
    let created = event_payload(&defeat_events.events, "corpse_created");
    assert_eq!(created["corpse_id"], "corpse:1");
    assert_eq!(created["origin_actor_id"], "scavenger");
    assert_eq!(created["origin_character_id"], Value::Null);
    assert_eq!(created["origin_kind"], "monster");
    assert_eq!(created["origin_name"], "Courtyard Scavenger");
    assert_eq!(created["sequence"], 1);
    assert_eq!(created["created_at"], 1);
    assert!(created.get("contents").is_none());
    assert!(created.get("sack_gold").is_none());

    let retained = defeat_events
        .events
        .iter()
        .find(|event| {
            matches!(
                event,
                Event::ItemRelocated {
                    item_instance_id,
                    reason: tme_rules::ItemRelocationReason::CorpseRetention,
                    ..
                } if item_instance_id == "cloth_bundle"
            )
        })
        .expect("corpse retention event");
    let retained = serde_json::to_value(retained).unwrap();
    assert_eq!(retained["item_relocated"]["to"]["kind"], "corpse");
    assert_eq!(retained["item_relocated"]["to"]["corpse_id"], "corpse:1");
    assert_eq!(
        retained["item_relocated"]["loot_claim"]["basis"],
        "killing_blow"
    );

    let search_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::SearchCorpse(CorpseId::parse("corpse:1").unwrap()),
        )
        .expect("corpse search");
    let searched = event_payload(&search_events.events, "corpse_searched");
    assert_eq!(
        searched,
        json!({
            "corpse_id": "corpse:1",
            "actor_id": "player",
            "actor": "Wayfarer",
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 1, "y": 1}
            },
            "items_released": 1,
            "gold_released": 3
        })
    );
    let gold = event_payload(&search_events.events, "gold_relocated");
    assert_eq!(gold["amount"], 3);
    assert_eq!(
        gold["from"],
        json!({"kind": "corpse", "corpse_id": "corpse:1"})
    );
    assert_eq!(gold["to"]["kind"], "ground");
    assert_eq!(gold["to"]["gold_pile_id"], "gold:1");
    assert_eq!(gold["reason"], "corpse_search");

    assert!(
        serde_json::from_value::<Event>(json!({
            "died": {"actor_id": "obsolete", "actor": "Obsolete"}
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<Event>(json!({
            "player_recovered": {"actor_id": "obsolete", "actor": "Obsolete"}
        }))
        .is_err()
    );
}

#[test]
fn event_25_automatic_decisions_reject_unknown_fields_and_variants() {
    let valid = json!({
        "kind": "move",
        "direction": "west",
        "purpose": "chase"
    });
    serde_json::from_value::<tme_rules::AutomaticActorDecisionV1>(valid.clone())
        .expect("current decision parses");

    let mut extra = valid.clone();
    extra["summary"] = json!("legacy prose");
    assert!(serde_json::from_value::<tme_rules::AutomaticActorDecisionV1>(extra).is_err());

    let mut unknown = valid;
    unknown["kind"] = json!("wander");
    assert!(serde_json::from_value::<tme_rules::AutomaticActorDecisionV1>(unknown).is_err());
}
