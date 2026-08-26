use crate::support::content_parts::ContentParts;
use tme_rules::model::{
    ItemOperationSource, ItemServiceOperationKind, MerchantInventoryId, MerchantListingOrigin,
};
use tme_rules::{
    ActionBlockedReasonV1, CarriedGoldPosition, CarriedPosition, Engine, Event, ItemLocationViewV1,
    MerchantListingOriginViewV1, PlayerCommandV1, PlayerIntent, ServiceCapabilityViewV1,
    TransactionCostReceiptV1, TransactionRewardReceiptV1, TransactionSourceV1,
};

fn value() -> ContentParts {
    ContentParts::tracked("merchant_item_services", "profile/merchant_item_services")
}

fn engine_from(value: ContentParts) -> Engine {
    value.engine(7).expect("fixture validates and starts")
}

fn transaction_receipt<E: AsRef<[Event]>>(events: &E) -> &Event {
    let events = events.as_ref();
    let receipts = events
        .iter()
        .filter(|event| matches!(event, Event::TransactionCommitted { .. }))
        .collect::<Vec<_>>();
    assert_eq!(receipts.len(), 1, "one final transaction receipt");
    let receipt_index = events
        .iter()
        .position(|event| std::ptr::eq(event, receipts[0]))
        .expect("receipt position");
    let last_delegated_index = events
        .iter()
        .enumerate()
        .filter(|(_, event)| {
            matches!(
                event,
                Event::GoldChanged { .. }
                    | Event::ItemRelocated { .. }
                    | Event::ItemAppraised { .. }
                    | Event::ItemIdentified { .. }
                    | Event::ItemEnchanted { .. }
            )
        })
        .map(|(index, _)| index)
        .max()
        .expect("delegated transaction event");
    assert!(last_delegated_index < receipt_index);
    receipts[0]
}

fn item_is_carried(engine: &Engine, item_instance_id: &str) -> bool {
    engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .carried
        .items
        .values()
        .any(|actual| actual == item_instance_id)
}

#[test]
fn grouped_discovery_is_ordered_command_ready_and_read_only() {
    let engine = engine_from(value());
    let before_world = engine.world().clone();
    let first = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("first discovery");
    let first_flat = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("first flat actions");
    let second = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("second discovery");
    let second_flat = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("second flat actions");
    assert_eq!(first, second);
    assert_eq!(first_flat, second_flat);
    assert_eq!(engine.world(), &before_world);
    assert_eq!(first.services_here.len(), 2);

    let ServiceCapabilityViewV1::Merchant {
        listings,
        buy_all,
        sales,
        ..
    } = &first.services_here[0].capabilities[0]
    else {
        panic!("first capability is merchant");
    };
    assert_eq!(
        listings
            .iter()
            .map(|listing| (
                listing.item.item_instance_id.as_str(),
                listing.origin,
                listing.price_gold,
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "copper_compass_stock",
                MerchantListingOriginViewV1::AuthoredStock,
                6,
            ),
            (
                "trail_lantern_stock",
                MerchantListingOriginViewV1::AuthoredStock,
                8,
            ),
        ]
    );
    assert!(listings.iter().all(|listing| listing.purchase.enabled));
    assert!(buy_all.enabled);
    assert_eq!(sales.len(), 3);

    let ServiceCapabilityViewV1::ItemService { operations, .. } =
        &first.services_here[0].capabilities[1]
    else {
        panic!("second capability is item service");
    };
    assert_eq!(
        operations
            .iter()
            .map(|operation| operation.operation)
            .collect::<Vec<_>>(),
        vec![
            ItemServiceOperationKind::Appraise,
            ItemServiceOperationKind::Identify,
            ItemServiceOperationKind::EnchantWeapon,
        ]
    );
    let appraise_moon = operations[0]
        .actions
        .iter()
        .find(|action| action.id == "appraise_moon_pebble")
        .expect("moon appraisal action");
    assert!(appraise_moon.enabled);
    let identify_river = operations[1]
        .actions
        .iter()
        .find(|action| action.id == "identify_river_blade")
        .expect("already identified action");
    assert!(!identify_river.enabled);
    assert_eq!(
        identify_river.blocked_reason,
        Some(ActionBlockedReasonV1::AlreadyComplete)
    );
    assert_eq!(operations[2].actions.len(), 1);
    assert_eq!(operations[2].actions[0].id, "enchant_weapon_river_blade");
}

#[test]
fn merchant_and_item_services_preserve_identity_and_commit_exact_receipts() {
    let mut engine = engine_from(value());
    let inventory_id = MerchantInventoryId::new("crossroads_counter", "shop_and_pawn");

    let appraisal = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::UseItemService {
                service_id: "crossroads_counter".to_string(),
                capability_id: "item_care".to_string(),
                operation: ItemServiceOperationKind::Appraise,
                item_instance_id: "moon_pebble".to_string(),
            },
        )
        .expect("appraisal commits");
    assert!(
        engine.world().item_instances["moon_pebble"]
            .knowledge
            .appraised
    );
    assert!(
        engine.world().item_instances["moon_pebble"]
            .knowledge
            .identified
    );
    assert!(matches!(
        transaction_receipt(&appraisal),
        Event::TransactionCommitted {
            source: TransactionSourceV1::ItemService {
                operation: ItemServiceOperationKind::Appraise,
                item_instance_id,
                ..
            },
            costs,
            rewards,
            ..
        } if item_instance_id == "moon_pebble"
            && costs.is_empty()
            && matches!(rewards.as_slice(), [TransactionRewardReceiptV1::ItemAppraised {
                unit_value_gold: 7,
                total_value_gold: 7,
                ..
            }])
    ));

    let sale = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::SellToMerchant {
                service_id: "crossroads_counter".to_string(),
                capability_id: "shop_and_pawn".to_string(),
                item_instance_id: "moon_pebble".to_string(),
            },
        )
        .expect("sale commits");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .carried
            .gold
            .sack,
        107
    );
    assert!(!item_is_carried(&engine, "moon_pebble"));
    let pawn = engine.world().merchant_inventories[&inventory_id]
        .listings
        .last()
        .expect("pawn listing");
    assert_eq!(pawn.item_instance_id, "moon_pebble");
    assert_eq!(pawn.origin, MerchantListingOrigin::PawnPool);
    assert_eq!(pawn.price_gold, 28);
    assert!(matches!(
        transaction_receipt(&sale),
        Event::TransactionCommitted {
            source: TransactionSourceV1::MerchantSale { item_instance_id, .. },
            costs,
            rewards,
            ..
        } if item_instance_id == "moon_pebble"
            && matches!(costs.as_slice(), [TransactionCostReceiptV1::MerchantItem {
                pawn_listing_price_gold: 28,
                ..
            }])
            && matches!(rewards.as_slice(), [TransactionRewardReceiptV1::CarriedGold {
                position: CarriedGoldPosition::Sack,
                amount: 7,
                before: 100,
                after: 107,
            }])
    ));

    let repurchase = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::BuyFromMerchant {
                service_id: "crossroads_counter".to_string(),
                capability_id: "shop_and_pawn".to_string(),
                item_instance_ids: vec!["moon_pebble".to_string()],
            },
        )
        .expect("pawn repurchase commits");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .carried
            .gold
            .sack,
        79
    );
    assert!(item_is_carried(&engine, "moon_pebble"));
    assert!(
        engine.world().item_instances["moon_pebble"]
            .knowledge
            .appraised
    );
    assert!(
        engine.world().merchant_inventories[&inventory_id]
            .listings
            .iter()
            .all(|listing| listing.item_instance_id != "moon_pebble")
    );
    assert!(matches!(
        transaction_receipt(&repurchase),
        Event::TransactionCommitted {
            source: TransactionSourceV1::MerchantPurchase { item_instance_ids, .. },
            costs,
            rewards,
            ..
        } if item_instance_ids == &["moon_pebble"]
            && matches!(costs.as_slice(), [TransactionCostReceiptV1::CarriedGold {
                position: CarriedGoldPosition::Sack,
                amount: 28,
                before: 107,
                after: 79,
            }])
            && matches!(rewards.as_slice(), [TransactionRewardReceiptV1::MerchantItem {
                item_instance_id,
                listing_price_gold: 28,
                from: ItemLocationViewV1::Merchant { .. },
                to: ItemLocationViewV1::Carried {
                    position: CarriedPosition::SackItem1,
                    ..
                },
                ..
            }] if item_instance_id == "moon_pebble")
    ));

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::BuyFromMerchant {
                service_id: "crossroads_counter".to_string(),
                capability_id: "shop_and_pawn".to_string(),
                item_instance_ids: vec![
                    "copper_compass_stock".to_string(),
                    "trail_lantern_stock".to_string(),
                ],
            },
        )
        .expect("buy-all commits");
    assert!(
        engine.world().merchant_inventories[&inventory_id]
            .listings
            .is_empty()
    );
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .carried
            .gold
            .sack,
        65
    );

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::BuyFromMerchant {
                service_id: "trail_vendor".to_string(),
                capability_id: "wares".to_string(),
                item_instance_ids: vec!["berry_tonic_stock".to_string()],
            },
        )
        .expect("vendor purchase commits");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .carried
            .gold
            .sack,
        60
    );

    let identification = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::UseItemService {
                service_id: "crossroads_counter".to_string(),
                capability_id: "item_care".to_string(),
                operation: ItemServiceOperationKind::Identify,
                item_instance_id: "sealed_charm".to_string(),
            },
        )
        .expect("identification commits");
    assert!(
        engine.world().item_instances["sealed_charm"]
            .knowledge
            .identified
    );
    assert!(
        !engine.world().item_instances["sealed_charm"]
            .knowledge
            .appraised
    );
    assert!(matches!(
        transaction_receipt(&identification),
        Event::TransactionCommitted {
            source: TransactionSourceV1::ItemService {
                operation: ItemServiceOperationKind::Identify,
                ..
            },
            rewards,
            ..
        } if matches!(rewards.as_slice(), [TransactionRewardReceiptV1::ItemIdentified { .. }])
    ));

    let enchantment = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::UseItemService {
                service_id: "crossroads_counter".to_string(),
                capability_id: "item_care".to_string(),
                operation: ItemServiceOperationKind::EnchantWeapon,
                item_instance_id: "river_blade".to_string(),
            },
        )
        .expect("enchantment commits");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .carried
            .gold
            .sack,
        55
    );
    assert!(
        !engine.world().item_instances["river_blade"]
            .knowledge
            .appraised
    );
    let applied = engine
        .world()
        .item_enchantments
        .iter()
        .find(|row| row.item_instance_id == "river_blade")
        .expect("service enchantment");
    assert_eq!(applied.combat_add_rating_bonus, 2);
    assert_eq!(applied.tags, ["bright_edge"]);
    assert!(matches!(
        &applied.source,
        ItemOperationSource::Service {
            service_id,
            capability_id,
        } if service_id == "crossroads_counter" && capability_id == "item_care"
    ));
    assert!(matches!(
        transaction_receipt(&enchantment),
        Event::TransactionCommitted {
            rewards,
            ..
        } if matches!(rewards.as_slice(), [TransactionRewardReceiptV1::ItemEnchanted {
            combat_add_rating_bonus: 2,
            tags,
            remaining_rounds: None,
            ..
        }] if tags == &["bright_edge"])
    ));
}

#[test]
fn rejected_merchant_and_service_commands_roll_back_the_complete_world() {
    let mut engine = engine_from(value());
    let before = engine.world().clone();
    let before_context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context before rejection");
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::BuyFromMerchant {
                service_id: "crossroads_counter".to_string(),
                capability_id: "shop_and_pawn".to_string(),
                item_instance_ids: vec![
                    "trail_lantern_stock".to_string(),
                    "copper_compass_stock".to_string(),
                ],
            },
        )
        .expect_err("reordered multi-item subset is rejected");
    assert!(error.to_string().contains("complete ordered inventory"));
    assert_eq!(engine.world(), &before);
    assert_eq!(
        engine
            .actor_observed_action_context(&tme_rules::ActorId::from("player"))
            .expect("context after rejection"),
        before_context
    );

    let mut no_gold = value();
    no_gold.actors_mut()[0]["carried"]["gold"]["sack"] = serde_json::json!(0);
    let mut engine = engine_from(no_gold);
    let before = engine.world().clone();
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::UseItemService {
                service_id: "crossroads_counter".to_string(),
                capability_id: "item_care".to_string(),
                operation: ItemServiceOperationKind::Identify,
                item_instance_id: "sealed_charm".to_string(),
            },
        )
        .expect_err("unfunded service is rejected");
    assert!(error.to_string().contains("gold cannot cover"));
    assert_eq!(engine.world(), &before);
}

#[test]
fn public_ec_wire_shapes_are_exact_required_and_strict() {
    let commands = [
        serde_json::json!({
            "contract_version": 26,
            "actor_id": "player",
            "intent": {"buy_from_merchant": {
                "service_id": "counter",
                "capability_id": "wares",
                "item_instance_ids": ["stock_1"]
            }}
        }),
        serde_json::json!({
            "contract_version": 26,
            "actor_id": "player",
            "intent": {"sell_to_merchant": {
                "service_id": "counter",
                "capability_id": "wares",
                "item_instance_id": "carried_1"
            }}
        }),
        serde_json::json!({
            "contract_version": 26,
            "actor_id": "player",
            "intent": {"use_item_service": {
                "service_id": "counter",
                "capability_id": "item_care",
                "operation": "appraise",
                "item_instance_id": "carried_1"
            }}
        }),
    ];
    for command in commands {
        let decoded =
            serde_json::from_value::<PlayerCommandV1>(command.clone()).expect("exact EC command");
        assert_eq!(
            serde_json::to_value(decoded).expect("serialize command"),
            command
        );

        let mut extra = command.clone();
        extra["intent"]
            .as_object_mut()
            .expect("intent enum")
            .values_mut()
            .next()
            .expect("intent payload")
            .as_object_mut()
            .expect("intent fields")
            .insert("extra".to_string(), serde_json::json!(true));
        assert!(serde_json::from_value::<PlayerCommandV1>(extra).is_err());

        let mut missing = command;
        missing["intent"]
            .as_object_mut()
            .expect("intent enum")
            .values_mut()
            .next()
            .expect("intent payload")
            .as_object_mut()
            .expect("intent fields")
            .remove("service_id");
        assert!(serde_json::from_value::<PlayerCommandV1>(missing).is_err());
    }

    let sources = [
        serde_json::json!({
            "kind": "merchant_purchase", "service_id": "counter",
            "capability_id": "wares", "item_instance_ids": ["stock_1"]
        }),
        serde_json::json!({
            "kind": "merchant_sale", "service_id": "counter",
            "capability_id": "wares", "item_instance_id": "carried_1"
        }),
        serde_json::json!({
            "kind": "item_service", "service_id": "counter",
            "capability_id": "item_care", "operation": "identify",
            "item_instance_id": "carried_1"
        }),
    ];
    for source in sources {
        let decoded = serde_json::from_value::<TransactionSourceV1>(source.clone())
            .expect("exact EC transaction source");
        assert_eq!(
            serde_json::to_value(decoded).expect("serialize source"),
            source
        );
        let mut extra = source.clone();
        extra["extra"] = serde_json::json!(true);
        assert!(serde_json::from_value::<TransactionSourceV1>(extra).is_err());
        let mut missing = source;
        missing
            .as_object_mut()
            .expect("source fields")
            .remove("service_id");
        assert!(serde_json::from_value::<TransactionSourceV1>(missing).is_err());
    }

    let service_source = serde_json::json!({
        "kind": "service", "service_id": "counter", "capability_id": "item_care"
    });
    let decoded = serde_json::from_value::<ItemOperationSource>(service_source.clone())
        .expect("exact item-operation source");
    assert_eq!(
        serde_json::to_value(decoded).expect("serialize item-operation source"),
        service_source
    );
    let mut extra_source = service_source.clone();
    extra_source["extra"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ItemOperationSource>(extra_source).is_err());

    let merchant_location = serde_json::json!({
        "kind": "merchant", "service_id": "counter", "capability_id": "wares"
    });
    let decoded = serde_json::from_value::<ItemLocationViewV1>(merchant_location.clone())
        .expect("exact merchant location");
    assert_eq!(
        serde_json::to_value(decoded).expect("serialize merchant location"),
        merchant_location
    );
    let mut extra_location = merchant_location.clone();
    extra_location["extra"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ItemLocationViewV1>(extra_location).is_err());
    let mut missing_location = merchant_location;
    missing_location
        .as_object_mut()
        .expect("location fields")
        .remove("capability_id");
    assert!(serde_json::from_value::<ItemLocationViewV1>(missing_location).is_err());

    let merchant_cost = serde_json::json!({
        "kind": "merchant_item", "item_instance_id": "carried_1",
        "item_definition_id": "carried", "quantity": 1,
        "from": {"kind": "carried", "actor_id": "player", "position": "sack_item_1"},
        "to": {"kind": "merchant", "service_id": "counter", "capability_id": "wares"},
        "pawn_listing_price_gold": 28
    });
    let decoded = serde_json::from_value::<TransactionCostReceiptV1>(merchant_cost.clone())
        .expect("exact merchant cost receipt");
    assert_eq!(
        serde_json::to_value(decoded).expect("serialize cost"),
        merchant_cost
    );
    let mut extra_cost = merchant_cost.clone();
    extra_cost["extra"] = serde_json::json!(true);
    assert!(serde_json::from_value::<TransactionCostReceiptV1>(extra_cost).is_err());
    let mut missing_cost = merchant_cost;
    missing_cost
        .as_object_mut()
        .expect("cost fields")
        .remove("pawn_listing_price_gold");
    assert!(serde_json::from_value::<TransactionCostReceiptV1>(missing_cost).is_err());

    let rewards = [
        serde_json::json!({"kind": "carried_gold", "position": "sack", "amount": 7, "before": 100, "after": 107}),
        serde_json::json!({
            "kind": "merchant_item", "item_instance_id": "stock_1",
            "item_definition_id": "stock", "quantity": 1,
            "from": {"kind": "merchant", "service_id": "counter", "capability_id": "wares"},
            "to": {"kind": "carried", "actor_id": "player", "position": "sack_item_1"},
            "listing_price_gold": 6
        }),
        serde_json::json!({
            "kind": "item_appraised", "item_instance_id": "carried_1",
            "item_definition_id": "carried", "unit_value_gold": 7, "total_value_gold": 7
        }),
        serde_json::json!({
            "kind": "item_identified", "item_instance_id": "carried_1",
            "item_definition_id": "carried"
        }),
        serde_json::json!({
            "kind": "item_enchanted", "item_instance_id": "blade_1",
            "item_definition_id": "blade", "enchantment_instance_id": "service:1",
            "combat_add_rating_bonus": 2, "tags": ["bright_edge"],
            "remaining_rounds": null
        }),
    ];
    for reward in rewards {
        let decoded = serde_json::from_value::<TransactionRewardReceiptV1>(reward.clone())
            .expect("exact EC reward receipt");
        assert_eq!(
            serde_json::to_value(decoded).expect("serialize reward"),
            reward
        );
        let mut extra = reward.clone();
        extra["extra"] = serde_json::json!(true);
        assert!(serde_json::from_value::<TransactionRewardReceiptV1>(extra).is_err());
        let mut missing = reward;
        let required = if missing["kind"] == "carried_gold" {
            "amount"
        } else {
            "item_instance_id"
        };
        missing
            .as_object_mut()
            .expect("reward fields")
            .remove(required);
        assert!(serde_json::from_value::<TransactionRewardReceiptV1>(missing).is_err());
    }

    assert!(
        serde_json::from_value::<ItemServiceOperationKind>(serde_json::json!("repair")).is_err()
    );
}

#[test]
fn rust_content_rejects_malformed_merchant_and_item_service_contracts() {
    let cases = [
        (
            "non-positive stock price",
            Box::new(|fixture: &mut ContentParts| {
                fixture.merchant_inventories_mut()[0]["stock"][0]["price_gold"] =
                    serde_json::json!(0);
            }) as Box<dyn Fn(&mut ContentParts)>,
            "price_gold must be positive",
        ),
        (
            "tied stock",
            Box::new(|fixture: &mut ContentParts| {
                fixture.item_instances_mut()["copper_compass_stock"]["binding"] =
                    serde_json::json!({"state": "bind_on_first_character_touch"});
            }),
            "must reference an unrestricted item",
        ),
        (
            "duplicate operation",
            Box::new(|fixture: &mut ContentParts| {
                fixture.selected_mut("service_definitions", 0)["capabilities"][1]["operations"]
                    .as_array_mut()
                    .expect("operations")
                    .push(serde_json::json!({"kind": "appraise"}));
            }),
            "kind must be unique within the capability",
        ),
        (
            "unsorted tags",
            Box::new(|fixture: &mut ContentParts| {
                fixture.selected_mut("service_definitions", 0)["capabilities"][1]["operations"]
                    [2]["tags"] = serde_json::json!(["z", "a"]);
            }),
            "tags must be sorted and unique",
        ),
        (
            "repair operation",
            Box::new(|fixture: &mut ContentParts| {
                fixture.selected_mut("service_definitions", 0)["capabilities"][1]["operations"] =
                    serde_json::json!([{"kind": "repair"}]);
            }),
            "unknown variant `repair`",
        ),
    ];

    for (name, mutate, expected) in cases {
        let mut fixture = value();
        mutate(&mut fixture);
        let error = match fixture.validated_seed() {
            Ok(_) => panic!("{name}"),
            Err(error) => error,
        };
        assert!(
            error.contains(expected),
            "{name}: expected {expected:?}, got {error:?}"
        );
    }
}
