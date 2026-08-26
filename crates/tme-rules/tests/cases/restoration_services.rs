use crate::support::content_parts::ContentParts;
use serde_json::json;
use tme_rules::model::{ActorAiBehavior, ActorTimingState, CharacterId, ItemHolderId};
use tme_rules::{
    ActionBlockedReasonV1, ActorKind, ActorLifeState, COMMAND_CONTRACT_VERSION, CarriedGold,
    CarriedPosition, CorpseId, Engine, Event, ItemLocation, LogicalTime, PlayerCommandV1,
    PlayerIntent, PlayerIntentPayloadV1, ResourceKind, RestorationOutcomeViewV1,
    RestorationStatusKind, ResurrectionMethod, ServiceCapabilityViewV1, TransactionCostReceiptV1,
    TransactionRewardReceiptV1, TransactionSourceV1,
};

fn fixture_parts() -> ContentParts {
    ContentParts::tracked("restoration_services", "profile/restoration_services")
}

fn engine() -> Engine {
    fixture_parts()
        .engine(7)
        .expect("restoration engine starts")
}

fn definition_error(parts: &ContentParts) -> String {
    parts
        .definition()
        .expect_err("mutated restoration definition must fail")
}

fn use_restoration(
    operation_id: &str,
    item_instance_id: Option<&str>,
    corpse_id: Option<CorpseId>,
) -> PlayerIntent {
    PlayerIntent::UseRestorationService {
        service_id: if operation_id == "priest_resurrection" {
            "tme_temple"
        } else {
            "tme_healer"
        }
        .to_string(),
        capability_id: if operation_id == "priest_resurrection" {
            "temple_restoration"
        } else {
            "healer_restoration"
        }
        .to_string(),
        operation_id: operation_id.to_string(),
        item_instance_id: item_instance_id.map(str::to_string),
        corpse_id,
    }
}

fn final_transaction(events: &[Event]) -> &Event {
    let receipts = events
        .iter()
        .filter(|event| matches!(event, Event::TransactionCommitted { .. }))
        .collect::<Vec<_>>();
    assert_eq!(receipts.len(), 1);
    assert!(matches!(
        events.last(),
        Some(Event::ActorReadinessScheduled { .. })
            | Some(Event::LogicalTimeAdvanced { .. })
            | Some(Event::ActorReady { .. })
    ));
    receipts[0]
}

#[test]
fn restoration_fixture_and_strict_capability_validate_through_content_parts() {
    fixture_parts()
        .validated_seed()
        .expect("restoration content graph validates");

    let mut stale = fixture_parts();
    stale.catalog["schema_version"] = json!(0);
    assert!(definition_error(&stale).contains("catalog.schema_version must be 6"));

    let mut authored_reward = fixture_parts();
    authored_reward.selected_mut("service_definitions", 0)["capabilities"][0]["operations"][0]["transaction"]
        ["rewards"] = json!([{"kind": "experience", "amount": 1}]);
    assert!(definition_error(&authored_reward).contains("typed outcome is the reward"));

    let mut unknown_outcome = fixture_parts();
    unknown_outcome.selected_mut("service_definitions", 0)["capabilities"][0]["operations"][0]["outcome"] =
        json!({"kind": "restore_age"});
    assert!(definition_error(&unknown_outcome).contains("unknown variant `restore_age`"));

    let mut empty_operations = fixture_parts();
    empty_operations.selected_mut("service_definitions", 0)["capabilities"][0]["operations"] =
        json!([]);
    assert!(definition_error(&empty_operations).contains("operations must be a non-empty list"));

    let mut duplicate_operation = fixture_parts();
    duplicate_operation.selected_mut("service_definitions", 0)["capabilities"][0]["operations"]
        [1]["transaction"]["id"] = json!("restore_hit_points");
    assert!(definition_error(&duplicate_operation).contains("transaction.id duplicates"));

    let mut paid_priest = fixture_parts();
    paid_priest.selected_mut("service_definitions", 1)["capabilities"][0]["operations"][0]["transaction"]
        ["costs"] = json!([{"kind": "carried_gold", "amount": 1}]);
    assert!(definition_error(&paid_priest).contains("must not charge carried gold"));

    let mut item_priest = fixture_parts();
    item_priest.selected_mut("service_definitions", 1)["capabilities"][0]["operations"][0]["transaction"]
        ["requirements"] = json!([{
        "kind": "carried_item",
        "item_definition_id": "clearwater_token",
        "quantity": 1
    }]);
    assert!(definition_error(&item_priest).contains("must not require or consume an item"));

    // The terms below come from tests/fixtures/synthetic-terms.txt, the tracked
    // nonsense denylist that .cargo/config.toml configures for cargo-run
    // processes. They prove the REJECTION MECHANISM without the tree carrying a
    // real term. Point TME_BANNED_TERMS_FILE at a different list and this
    // assertion stops holding — by construction, not by defect: a tree that
    // carries no real term cannot write a fixture the real list rejects.
    let mut banned_label = fixture_parts();
    banned_label.selected_mut("service_definitions", 0)["capabilities"][0]["operations"][0]["transaction"]
        ["label"] = json!("zorbelquux restoration");
    assert!(definition_error(&banned_label).contains("contains banned source term"));
}

#[test]
fn grouped_discovery_is_ordered_planner_backed_and_read_only() {
    let mut engine = engine();
    let second_token = engine.world().item_instances["clearwater_token"].clone();
    engine
        .world_mut()
        .item_instances
        .insert("second_clearwater_token".to_string(), second_token);
    engine.world_mut().actors[0].carried.items.insert(
        CarriedPosition::SackItem2,
        "second_clearwater_token".to_string(),
    );
    let before = engine.world().clone();
    let first = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("restoration action context");
    let second = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("repeat restoration action context");
    assert_eq!(first, second);
    assert_eq!(engine.world(), &before);
    assert_eq!(first.services_here.len(), 1);
    let ServiceCapabilityViewV1::Restoration { operations, .. } =
        &first.services_here[0].capabilities[0]
    else {
        panic!("healer exposes restoration capability");
    };
    assert_eq!(
        operations
            .iter()
            .map(|operation| operation.operation_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "restore_hit_points",
            "restore_magic_points",
            "restore_stamina",
            "cure_blindness_for_token",
            "cure_poison",
        ]
    );
    assert!(matches!(
        operations[0].outcome,
        RestorationOutcomeViewV1::RestoreResource {
            resource: ResourceKind::Hp
        }
    ));
    assert!(matches!(
        operations[3].outcome,
        RestorationOutcomeViewV1::CureStatus {
            status: RestorationStatusKind::Blindness
        }
    ));
    assert!(operations.iter().all(|operation| {
        operation
            .actions
            .iter()
            .all(|action| action.enabled && action.command.is_some())
    }));
    assert_eq!(operations[3].actions.len(), 2);
    assert_eq!(
        operations[3]
            .actions
            .iter()
            .map(
                |action| match &action.command.as_ref().expect("cure command").intent {
                    PlayerIntentPayloadV1::UseRestorationService {
                        item_instance_id: Some(item_instance_id),
                        corpse_id: None,
                        ..
                    } => item_instance_id.as_str(),
                    other => panic!("unexpected cure command: {other:?}"),
                }
            )
            .collect::<Vec<_>>(),
        vec!["clearwater_token", "second_clearwater_token"]
    );
}

#[test]
fn missing_item_restoration_exposes_one_disabled_null_selection() {
    let mut engine = engine();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "clearwater_token".to_string(),
                destination: tme_rules::ItemMoveDestination::GroundHere,
            },
        )
        .expect("move the required token out of carried inventory");
    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("restoration action context");
    let ServiceCapabilityViewV1::Restoration { operations, .. } =
        &context.services_here[0].capabilities[0]
    else {
        panic!("healer exposes restoration capability");
    };
    let cure = operations
        .iter()
        .find(|operation| operation.operation_id == "cure_blindness_for_token")
        .expect("item-backed cure");
    assert_eq!(cure.actions.len(), 1);
    assert!(!cure.actions[0].enabled);
    assert_eq!(
        cure.actions[0].blocked_reason,
        Some(ActionBlockedReasonV1::MissingRequiredItem)
    );
    assert!(matches!(
        cure.actions[0]
            .command
            .as_ref()
            .map(|command| &command.intent),
        Some(PlayerIntentPayloadV1::UseRestorationService {
            item_instance_id: None,
            corpse_id: None,
            ..
        })
    ));
}

#[test]
fn resources_cures_gold_and_selected_item_share_one_transaction_path() {
    let mut engine = engine();

    let hp_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            use_restoration("restore_hit_points", None, None),
        )
        .expect("HP restoration");
    assert!(matches!(
        final_transaction(&hp_events.events),
        Event::TransactionCommitted {
            source: TransactionSourceV1::RestorationService { operation_id, corpse_id: None, .. },
            costs,
            rewards,
            ..
        } if operation_id == "restore_hit_points"
            && matches!(costs.as_slice(), [TransactionCostReceiptV1::CarriedGold { amount: 3, before: 10, after: 7, .. }])
            && matches!(rewards.as_slice(), [TransactionRewardReceiptV1::ResourceRestored { resource: ResourceKind::Hp, before: 6, after: 12, maximum: 12, .. }])
    ));
    assert!(hp_events.iter().any(|event| matches!(
        event,
        Event::ResourceRestored {
            resource: ResourceKind::Hp,
            before: 6,
            after: 12,
            maximum: 12,
            ..
        }
    )));

    let mp_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            use_restoration("restore_magic_points", None, None),
        )
        .expect("MP restoration");
    assert!(mp_events.iter().any(|event| matches!(
        event,
        Event::ResourceRestored {
            resource: ResourceKind::Mp,
            before: 3,
            after: 8,
            maximum: 8,
            ..
        }
    )));
    let stamina_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            use_restoration("restore_stamina", None, None),
        )
        .expect("stamina restoration");
    assert!(stamina_events.iter().any(|event| matches!(
        event,
        Event::ResourceRestored {
            resource: ResourceKind::Stamina,
            before: 4,
            after: 10,
            maximum: 10,
            ..
        }
    )));

    let blind_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            use_restoration("cure_blindness_for_token", Some("clearwater_token"), None),
        )
        .expect("selected-item blindness cure");
    assert!(matches!(
        final_transaction(&blind_events.events),
        Event::TransactionCommitted {
            costs,
            rewards,
            ..
        } if matches!(costs.as_slice(), [TransactionCostReceiptV1::SelectedCarriedItem { item_instance_id, consumed_quantity: 1, remaining_quantity: 0, .. }] if item_instance_id == "clearwater_token")
            && matches!(rewards.as_slice(), [TransactionRewardReceiptV1::StatusCured { status: RestorationStatusKind::Blindness, removed_count: 1, .. }])
    ));
    assert!(
        !engine
            .world()
            .item_instances
            .contains_key("clearwater_token")
    );
    assert_eq!(engine.world().actors[0].active_effects.len(), 1);
    assert_eq!(
        engine.world().actors[0].active_effects[0].effect_id,
        "marsh_poison"
    );

    let poison_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            use_restoration("cure_poison", None, None),
        )
        .expect("delayed poison cure");
    assert!(poison_events.iter().any(|event| matches!(
        event,
        Event::EffectRemoved { effect_id, reason, .. }
            if effect_id == "marsh_poison" && reason == "restoration_service:cure_poison"
    )));
    assert!(engine.world().actors[0].active_effects.is_empty());
    assert_eq!(engine.world().actors[0].carried.gold.sack, 4);

    for operation in [
        "restore_hit_points",
        "restore_magic_points",
        "restore_stamina",
        "cure_poison",
    ] {
        let before = engine.world().clone();
        let error = engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                use_restoration(operation, None, None),
            )
            .expect_err("completed restoration must be rejected");
        assert!(
            error.to_string().contains("already full")
                || error.to_string().contains("has no poison effect")
        );
        assert_eq!(engine.world(), &before);
    }
}

#[test]
fn temple_without_a_corpse_exposes_one_disabled_exact_selection() {
    let mut engine = engine();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![tme_rules::Direction::East, tme_rules::Direction::East]),
        )
        .expect("payer reaches temple");
    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("temple action context");
    let ServiceCapabilityViewV1::Restoration { operations, .. } =
        &context.services_here[0].capabilities[0]
    else {
        panic!("temple restoration view");
    };
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].actions.len(), 1);
    assert!(!operations[0].actions[0].enabled);
    assert_eq!(
        operations[0].actions[0].blocked_reason,
        Some(ActionBlockedReasonV1::NoSuchCorpse)
    );
    assert!(matches!(
        operations[0].actions[0]
            .command
            .as_ref()
            .map(|command| &command.intent),
        Some(PlayerIntentPayloadV1::UseRestorationService {
            item_instance_id: None,
            corpse_id: None,
            ..
        })
    ));
}

fn add_and_defeat_named_temple_target(
    engine: &mut Engine,
    actor_id: &str,
    actor_name: &str,
    character_id: &str,
    item_instance_id: &str,
) -> (CorpseId, CharacterId) {
    let target_character_id: CharacterId =
        serde_json::from_str(&format!("\"{character_id}\"")).expect("strong character id");
    let target_item = engine.world().item_instances["clearwater_token"].clone();
    let mut target = engine.world().actors[0].clone();
    target.id = actor_id.into();
    target.name = actor_name.to_string();
    target.location.position = (3, 1).into();
    target.home_location = target.location.clone();
    target.character_id = Some(target_character_id.clone());
    target.hp = 1;
    target.stamina = 4;
    target.active_effects.clear();
    target.carried.items.clear();
    target
        .carried
        .items
        .insert(CarriedPosition::SackItem1, item_instance_id.to_string());
    target.carried.gold = CarriedGold {
        left_hand: 0,
        right_hand: 0,
        sack: 5,
    };
    let character = target.character.as_mut().expect("target character");
    character.resources.hp = 1;
    character.resources.stamina = 4;
    target.timing = ActorTimingState {
        ready_at: engine.world().timing.now,
        tie_break_order: 0,
    };

    let world = engine.world_mut();
    world.actors[0].timing.ready_at = LogicalTime::new(100);
    world
        .item_instances
        .insert(item_instance_id.to_string(), target_item);
    world.actors.push(target);
    let witness = world
        .actors
        .iter_mut()
        .find(|actor| actor.id == "witness")
        .expect("witness");
    witness.location.position = (3, 2).into();
    witness.stats.attack = 100;
    witness.ai.as_mut().expect("witness AI").behavior = ActorAiBehavior::SimpleChase;

    let mut defeated = Vec::new();
    let mut corpse_id = None;
    let actor_id: tme_rules::ActorId = actor_id.into();
    for _ in 0..3 {
        let events = engine
            .apply_actor_intent(&actor_id, PlayerIntent::Wait)
            .expect("authoritative target defeat opportunity");
        corpse_id = events.iter().find_map(|event| match event {
            Event::CorpseCreated {
                corpse_id,
                origin_actor_id,
                ..
            } if origin_actor_id == &actor_id => Some(corpse_id.clone()),
            _ => None,
        });
        defeated.extend(events);
        if corpse_id.is_some() {
            break;
        }
    }
    let corpse_id =
        corpse_id.unwrap_or_else(|| panic!("target corpse missing from events: {defeated:#?}"));
    assert!(matches!(
        engine
            .world()
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .expect("fallen actor")
            .life_state,
        ActorLifeState::Ghost { .. }
    ));
    (corpse_id, target_character_id)
}

fn add_and_defeat_temple_target(engine: &mut Engine) -> (CorpseId, CharacterId) {
    add_and_defeat_named_temple_target(
        engine,
        "fallen_ally",
        "Fallen Ally",
        "character:restoration_services:target",
        "target_token",
    )
}

fn place_payer_at_temple(engine: &mut Engine) {
    let now = engine.world().timing.now;
    let world = engine.world_mut();
    world.actors[0].location.position = (3, 1).into();
    world.actors[0].home_location.position = (3, 1).into();
    world.actors[0].timing = ActorTimingState {
        ready_at: now,
        tie_break_order: 0,
    };
    let witness = world
        .actors
        .iter_mut()
        .find(|actor| actor.id == "witness")
        .expect("witness");
    witness.stats.attack = 0;
    witness.ai.as_mut().expect("witness AI").behavior = ActorAiBehavior::HoldGround;
}

#[test]
fn priest_resurrection_uses_exact_corpse_death_inventory_resource_and_timing_owners() {
    let mut engine = engine();
    let protected_before = engine.world().actors[0]
        .character
        .as_ref()
        .expect("character")
        .clone();
    let (corpse_id, target_character_id) = add_and_defeat_temple_target(&mut engine);

    {
        let world = engine.world_mut();
        let witness = world
            .actors
            .iter_mut()
            .find(|actor| actor.id == "witness")
            .expect("witness");
        witness.stats.attack = 0;
        witness.ai.as_mut().expect("witness AI").behavior = ActorAiBehavior::HoldGround;
        world.actors[0].timing = ActorTimingState {
            ready_at: world.timing.now,
            tie_break_order: 0,
        };
    }
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![tme_rules::Direction::East, tme_rules::Direction::East]),
        )
        .expect("payer reaches temple");

    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("temple action context");
    let ServiceCapabilityViewV1::Restoration { operations, .. } =
        &context.services_here[0].capabilities[0]
    else {
        panic!("temple restoration view");
    };
    assert!(matches!(
        operations[0].outcome,
        RestorationOutcomeViewV1::PriestResurrection
    ));
    assert_eq!(operations[0].actions.len(), 1);
    assert!(operations[0].actions[0].enabled);

    let intent = use_restoration("priest_resurrection", None, Some(corpse_id.clone()));
    let command = engine
        .actor_command_for_intent(&tme_rules::ActorId::from("player"), &intent)
        .expect("Command 22 conversion");
    assert!(
        engine
            .validate_actor_command(&command)
            .expect("command validation")
            .accepted
    );
    assert_eq!(
        engine
            .command_to_actor_intent(&command)
            .expect("command round trip"),
        intent
    );
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), intent)
        .expect("Priest resurrection");

    let transaction_index = events
        .iter()
        .position(|event| matches!(event, Event::TransactionCommitted { .. }))
        .expect("transaction receipt");
    let resurrection_index = events
        .iter()
        .position(|event| matches!(event, Event::ActorResurrected { .. }))
        .expect("death-owner resurrection event");
    assert!(resurrection_index < transaction_index);
    assert!(matches!(
        &events[transaction_index],
        Event::TransactionCommitted {
            source: TransactionSourceV1::RestorationService { operation_id, corpse_id: Some(actual), .. },
            costs,
            rewards,
            ..
        } if operation_id == "priest_resurrection" && actual == &corpse_id && costs.is_empty()
            && matches!(rewards.as_slice(), [TransactionRewardReceiptV1::PriestResurrection {
                target_actor_id,
                corpse_id: actual,
                method: ResurrectionMethod::Priest,
                current_hp: 11,
                current_stamina: 9,
            }] if target_actor_id == "fallen_ally" && actual == &corpse_id)
    ));
    assert!(!engine.world().corpses.contains_key(&corpse_id));
    let target = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "fallen_ally")
        .expect("resurrected target");
    assert_eq!(target.life_state, ActorLifeState::Alive);
    assert_eq!((target.hp, target.stamina), (11, 9));
    assert_eq!(target.carried.gold.sack, 5);
    assert_eq!(
        engine.item_location("target_token").expect("returned item"),
        ItemLocation::Carried {
            holder: ItemHolderId::Character(target_character_id),
            position: CarriedPosition::SackItem1,
        }
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ActorReadinessScheduled { actor_id, .. } if actor_id == "fallen_ally"
    )));

    let target_sheet = target.character.as_ref().expect("target sheet");
    assert_eq!(target_sheet.progression, protected_before.progression);
    assert_eq!(target_sheet.skill_ledger, protected_before.skill_ledger);
    assert_eq!(target_sheet.resources.mp, protected_before.resources.mp);
    assert_eq!(
        target_sheet.resources.max_hp,
        protected_before.resources.max_hp
    );
    assert_eq!(
        target_sheet.resources.max_mp,
        protected_before.resources.max_mp
    );
    assert_eq!(
        target_sheet.resources.max_stamina,
        protected_before.resources.max_stamina
    );
    assert_eq!(
        target_sheet.resources.peak_hp,
        protected_before.resources.peak_hp
    );
    assert_eq!(
        target_sheet.attributes.dexterity,
        protected_before.attributes.dexterity
    );
    assert_eq!(target_sheet.identity, protected_before.identity);
    assert_eq!(
        target_sheet.alignment_state,
        protected_before.alignment_state
    );
}

#[test]
fn temple_resurrection_actions_follow_corpse_sequence_order() {
    let mut engine = engine();
    let (first_corpse_id, _) = add_and_defeat_temple_target(&mut engine);
    let (second_corpse_id, _) = add_and_defeat_named_temple_target(
        &mut engine,
        "second_fallen_ally",
        "Second Fallen Ally",
        "character:restoration_services:second_target",
        "second_target_token",
    );
    place_payer_at_temple(&mut engine);

    let mut expected = engine
        .world()
        .corpses
        .values()
        .filter(|corpse| {
            corpse.origin_kind == ActorKind::Player
                && corpse.location.level == "room_0"
                && corpse.location.position == (3, 1).into()
        })
        .map(|corpse| (corpse.sequence, corpse.id.clone()))
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(
        expected.iter().map(|(_, id)| id).collect::<Vec<_>>(),
        vec![&first_corpse_id, &second_corpse_id]
    );

    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("temple action context");
    let ServiceCapabilityViewV1::Restoration { operations, .. } =
        &context.services_here[0].capabilities[0]
    else {
        panic!("temple restoration view");
    };
    let action_corpse_ids = operations[0]
        .actions
        .iter()
        .map(
            |action| match &action.command.as_ref().expect("corpse command").intent {
                PlayerIntentPayloadV1::UseRestorationService {
                    corpse_id: Some(corpse_id),
                    item_instance_id: None,
                    ..
                } => corpse_id,
                other => panic!("unexpected Priest command: {other:?}"),
            },
        )
        .collect::<Vec<_>>();
    assert_eq!(
        action_corpse_ids,
        expected.iter().map(|(_, id)| id).collect::<Vec<_>>()
    );
}

#[test]
fn restoration_rejects_wrong_selections_without_mutation() {
    let mut engine = engine();
    let before = engine.world().clone();
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            use_restoration(
                "restore_hit_points",
                None,
                Some(CorpseId::parse("corpse:99").unwrap()),
            ),
        )
        .expect_err("resource restoration rejects corpse input");
    assert_eq!(
        error.to_string(),
        "resource restoration does not accept a corpse selection"
    );
    assert_eq!(engine.world(), &before);

    let missing_item_before = engine.world().clone();
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            use_restoration("cure_blindness_for_token", None, None),
        )
        .expect_err("item-for-service requires exact selection");
    assert_eq!(
        error.to_string(),
        "transaction requires an exact carried item selection"
    );
    assert_eq!(engine.world(), &missing_item_before);

    let wrong_item_before = engine.world().clone();
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            use_restoration("cure_blindness_for_token", Some("missing_token"), None),
        )
        .expect_err("item-for-service rejects an unknown exact selection");
    assert!(error.to_string().contains("not carried"));
    assert_eq!(engine.world(), &wrong_item_before);

    engine.world_mut().actors[0].carried.gold.sack = 2;
    let insufficient_gold_before = engine.world().clone();
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            use_restoration("restore_hit_points", None, None),
        )
        .expect_err("insufficient gold rejects before mutation");
    assert!(error.to_string().contains("requires 3 carried gold"));
    assert_eq!(engine.world(), &insufficient_gold_before);
}

#[test]
fn priest_rejects_missing_remote_nonplayer_removed_and_item_cross_product_corpses() {
    let mut prepared = engine();
    let (corpse_id, _) = add_and_defeat_temple_target(&mut prepared);
    place_payer_at_temple(&mut prepared);

    let mut missing_selection = engine();
    place_payer_at_temple(&mut missing_selection);
    let before = missing_selection.world().clone();
    let error = missing_selection
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            use_restoration("priest_resurrection", None, None),
        )
        .expect_err("Priest requires an exact corpse");
    assert_eq!(
        error.to_string(),
        "Priest resurrection requires an exact corpse selection"
    );
    assert_eq!(missing_selection.world(), &before);

    let mut remote = prepared.clone();
    remote
        .world_mut()
        .corpses
        .get_mut(&corpse_id)
        .expect("corpse")
        .location
        .position = (2, 2).into();
    let before = remote.world().clone();
    let error = remote
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            use_restoration("priest_resurrection", None, Some(corpse_id.clone())),
        )
        .expect_err("remote corpse rejected");
    assert!(error.to_string().contains("not at the Priest service"));
    assert_eq!(remote.world(), &before);

    let mut nonplayer = prepared.clone();
    nonplayer
        .world_mut()
        .corpses
        .get_mut(&corpse_id)
        .expect("corpse")
        .origin_kind = ActorKind::Monster;
    let before = nonplayer.world().clone();
    let error = nonplayer
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            use_restoration("priest_resurrection", None, Some(corpse_id.clone())),
        )
        .expect_err("nonplayer corpse rejected");
    assert!(error.to_string().contains("requires a player corpse"));
    assert_eq!(nonplayer.world(), &before);

    let mut removed = prepared.clone();
    removed.world_mut().corpses.remove(&corpse_id);
    let before = removed.world().clone();
    let error = removed
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            use_restoration("priest_resurrection", None, Some(corpse_id.clone())),
        )
        .expect_err("removed corpse rejected");
    assert!(error.to_string().contains("does not exist"));
    assert_eq!(removed.world(), &before);

    let mut item_cross_product = prepared;
    let before = item_cross_product.world().clone();
    let error = item_cross_product
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            use_restoration(
                "priest_resurrection",
                Some("clearwater_token"),
                Some(corpse_id),
            ),
        )
        .expect_err("Priest rejects an item/corpse cross product");
    assert!(
        error
            .to_string()
            .contains("does not accept an item selection")
    );
    assert_eq!(item_cross_product.world(), &before);
}

fn normalize_overflow_actor_for_level(engine: &mut Engine) {
    let actor = &mut engine.world_mut().actors[0];
    actor.hp = 12;
    actor.stamina = 10;
    let resources = &mut actor.character.as_mut().expect("character").resources;
    resources.hp = 12;
    resources.max_hp = 12;
    resources.peak_hp = 12;
    resources.stamina = 10;
    resources.max_stamina = 10;
}

#[test]
fn post_cost_progression_failure_rolls_back_restoration_world_and_rng() {
    let mut engine = engine();
    {
        let actor = &mut engine.world_mut().actors[0];
        actor.hp = i32::MAX - 1;
        actor.stamina = 10;
        let character = actor.character.as_mut().expect("character");
        character.resources.hp = i32::MAX - 1;
        character.resources.max_hp = i32::MAX;
        character.resources.peak_hp = i32::MAX;
        character.resources.stamina = 10;
        character.progression.experience = 600;
    }
    let mut expected = engine.clone();
    let before_context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("pre-failure context");
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            use_restoration("restore_hit_points", None, None),
        )
        .expect_err("level growth after committed restoration must overflow");
    assert!(
        error.to_string().contains("level HP growth overflow"),
        "unexpected post-cost failure: {error}"
    );
    assert_eq!(engine.world(), expected.world());
    assert_eq!(
        engine
            .actor_observed_action_context(&tme_rules::ActorId::from("player"))
            .expect("rolled-back context"),
        before_context
    );

    normalize_overflow_actor_for_level(&mut engine);
    normalize_overflow_actor_for_level(&mut expected);
    let actual_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("post-rollback deterministic level growth");
    let expected_events = expected
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("control deterministic level growth");
    assert_eq!(actual_events, expected_events);
    assert_eq!(engine.world(), expected.world());
}

#[test]
fn command_26_restoration_shape_requires_both_nullable_selections() {
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::UseRestorationService {
            service_id: "tme_temple".to_string(),
            capability_id: "temple_restoration".to_string(),
            operation_id: "priest_resurrection".to_string(),
            item_instance_id: None,
            corpse_id: Some(CorpseId::parse("corpse:7").expect("corpse id")),
        },
    };
    let value = serde_json::to_value(&command).expect("serialize restoration command");
    assert_eq!(
        value,
        serde_json::json!({
            "contract_version": 26,
            "actor_id": "player",
            "intent": {
                "use_restoration_service": {
                    "service_id": "tme_temple",
                    "capability_id": "temple_restoration",
                    "operation_id": "priest_resurrection",
                    "item_instance_id": null,
                    "corpse_id": "corpse:7"
                }
            }
        })
    );
    assert_eq!(
        serde_json::from_value::<PlayerCommandV1>(value.clone()).expect("round trip"),
        command
    );
    for field in ["item_instance_id", "corpse_id"] {
        let mut missing = value.clone();
        missing["intent"]["use_restoration_service"]
            .as_object_mut()
            .expect("payload object")
            .remove(field);
        assert!(serde_json::from_value::<PlayerCommandV1>(missing).is_err());
    }
    let mut unknown = value;
    unknown["intent"]["use_restoration_service"]["legacy"] = serde_json::json!(true);
    assert!(serde_json::from_value::<PlayerCommandV1>(unknown).is_err());
}

#[test]
fn restoration_blocked_reason_codes_are_finite() {
    assert_eq!(
        ActionBlockedReasonV1::NoRestorationNeeded.code(),
        "no_restoration_needed"
    );
    assert_eq!(
        ActionBlockedReasonV1::UnsupportedRestoration.code(),
        "unsupported_restoration"
    );
    assert_eq!(
        ActionBlockedReasonV1::NoRestorationNeeded.to_string(),
        "no restoration needed"
    );
    assert_eq!(
        ActionBlockedReasonV1::UnsupportedRestoration.to_string(),
        "unsupported restoration"
    );
    assert_eq!(
        serde_json::to_value(ActionBlockedReasonV1::NoRestorationNeeded)
            .expect("blocked reason JSON"),
        serde_json::json!("no_restoration_needed")
    );
}
