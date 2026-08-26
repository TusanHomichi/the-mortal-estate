use crate::support::content_parts::ContentParts;
use tme_rules::{
    ActionBlockedReasonV1, CarriedGoldPosition, Coord, Engine, Event, GroundItem, PlayerIntent,
    PlayerIntentPayloadV1, ServiceCapabilityViewV1, TransactionCostReceiptV1,
    TransactionRewardReceiptV1, TransactionSourceV1, WorldPosition,
};

fn value() -> ContentParts {
    ContentParts::tracked("service_transactions", "profile/service_transactions")
}

fn engine_from(parts: ContentParts) -> Engine {
    parts.engine(7).expect("engine starts")
}

fn transaction_command(engine: &Engine) -> tme_rules::PlayerCommandV1 {
    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context");
    let capability = &context.services_here[0].capabilities[0];
    let ServiceCapabilityViewV1::ServiceTransaction { transactions, .. } = capability else {
        panic!("generic transaction capability");
    };
    assert_eq!(transactions.len(), 1);
    assert_eq!(transactions[0].requirements.len(), 4);
    assert_eq!(transactions[0].costs.len(), 2);
    assert_eq!(transactions[0].rewards.len(), 2);
    assert_eq!(transactions[0].actions.len(), 1);
    let action = &transactions[0].actions[0];
    assert!(action.enabled, "action should be ready: {action:?}");
    action.command.clone().expect("typed command")
}

#[test]
fn generic_service_transaction_discovers_commits_and_blocks_replay() {
    let mut engine = engine_from(value());
    let command = transaction_command(&engine);
    assert_eq!(command.contract_version, 26);
    assert!(matches!(
        &command.intent,
        PlayerIntentPayloadV1::CommitServiceTransaction {
            service_id,
            capability_id,
            transaction_id,
            item_instance_id: Some(item_instance_id),
        } if service_id == "waystation_clerk"
            && capability_id == "exchanges"
            && transaction_id == "token_for_badge"
            && item_instance_id == "etched_token_stack"
    ));
    let intent = engine
        .command_to_actor_intent(&command)
        .expect("command converts");
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), intent)
        .expect("transaction commits");
    assert!(
        events
            .events
            .iter()
            .any(|event| matches!(event, Event::PlayerIntent { .. }))
    );
    assert!(matches!(
        events
            .events
            .iter()
            .find(|event| matches!(event, Event::GoldChanged { .. })),
        Some(Event::GoldChanged {
            amount: -5,
            new_total: 15,
            ..
        })
    ));
    assert!(matches!(
        events
            .events
            .iter()
            .find(|event| matches!(event, Event::ExperienceAwarded { .. })),
        Some(Event::ExperienceAwarded {
            amount: 25,
            total_xp: 325,
            ..
        })
    ));
    let Event::TransactionCommitted {
        source,
        costs,
        rewards,
        ..
    } = events
        .events
        .iter()
        .rev()
        .find(|event| matches!(event, Event::TransactionCommitted { .. }))
        .expect("final transaction event")
    else {
        unreachable!()
    };
    assert!(matches!(
        source,
        TransactionSourceV1::ServiceTransaction {
            service_id,
            capability_id,
            transaction_id,
        } if service_id == "waystation_clerk"
            && capability_id == "exchanges"
            && transaction_id == "token_for_badge"
    ));
    assert!(matches!(
        costs.as_slice(),
        [
            TransactionCostReceiptV1::SelectedCarriedItem {
                consumed_quantity: 1,
                remaining_quantity: 1,
                ..
            },
            TransactionCostReceiptV1::CarriedGold {
                position: CarriedGoldPosition::Sack,
                amount: 5,
                before: 20,
                after: 15,
            }
        ]
    ));
    assert!(matches!(
        rewards.as_slice(),
        [
            TransactionRewardReceiptV1::Experience {
                amount: 25,
                total_xp: 325,
            },
            TransactionRewardReceiptV1::Item { quantity: 1, .. }
        ]
    ));
    assert_eq!(
        engine.world().item_instances["etched_token_stack"].quantity,
        1
    );
    assert!(
        engine
            .world()
            .item_instances
            .contains_key("trail_badge:service_transactions:primary")
    );

    let replay = engine
        .actor_command_for_intent(
            &tme_rules::ActorId::from("player"),
            &PlayerIntent::CommitServiceTransaction {
                service_id: "waystation_clerk".to_string(),
                capability_id: "exchanges".to_string(),
                transaction_id: "token_for_badge".to_string(),
                item_instance_id: Some("etched_token_stack".to_string()),
            },
        )
        .expect("replay command");
    let status = engine
        .validate_actor_command(&replay)
        .expect("replay validates structurally");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::AlreadyComplete)
    );
}

#[test]
fn exact_item_selection_failures_are_typed_and_read_only() {
    let engine = engine_from(value());
    let before = engine.world().clone();
    for (selection, expected) in [
        (None, ActionBlockedReasonV1::MissingRequiredItem),
        (
            Some("training_sword".to_string()),
            ActionBlockedReasonV1::MissingRequiredItem,
        ),
        (
            Some("missing".to_string()),
            ActionBlockedReasonV1::MissingRequiredItem,
        ),
    ] {
        let command = engine
            .actor_command_for_intent(
                &tme_rules::ActorId::from("player"),
                &PlayerIntent::CommitServiceTransaction {
                    service_id: "waystation_clerk".to_string(),
                    capability_id: "exchanges".to_string(),
                    transaction_id: "token_for_badge".to_string(),
                    item_instance_id: selection,
                },
            )
            .expect("command");
        let status = engine.validate_actor_command(&command).expect("status");
        assert_eq!(status.blocked_reason, Some(expected));
        assert_eq!(engine.world(), &before);
    }
}

#[test]
fn late_reward_failure_rolls_back_costs_and_all_world_state() {
    let mut engine = engine_from(value());
    let command = transaction_command(&engine);
    engine.world_mut().ground_items.push(GroundItem {
        item_instance_id: "corrupt_unregistered_location".to_string(),
        location: WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 }),
        loot_claim: None,
    });
    let before = engine.world().clone();
    let intent = engine
        .command_to_actor_intent(&command)
        .expect("preflight accepts command");
    let error = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), intent)
        .expect_err("late grant burden must fail");
    assert!(
        error
            .to_string()
            .contains("item location references unknown instance")
    );
    assert_eq!(engine.world(), &before);
}
