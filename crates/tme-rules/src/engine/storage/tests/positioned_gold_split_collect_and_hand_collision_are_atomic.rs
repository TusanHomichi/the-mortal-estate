use super::*;

#[test]
fn positioned_gold_split_collect_and_hand_collision_are_atomic() {
    let mut engine = storage_engine();
    let mut events = Vec::new();
    engine
        .apply_player_move_gold(
            0,
            &GoldMoveSource::Carried {
                position: crate::model::CarriedGoldPosition::Sack,
            },
            &GoldMoveDestination::Carried {
                position: crate::model::CarriedGoldPosition::LeftHand,
            },
            &GoldMoveQuantity::Exact { amount: 40 },
            &mut events,
        )
        .expect("split into open hand");
    engine
        .apply_player_move_gold(
            0,
            &GoldMoveSource::Carried {
                position: crate::model::CarriedGoldPosition::LeftHand,
            },
            &GoldMoveDestination::GroundHere,
            &GoldMoveQuantity::Exact { amount: 15 },
            &mut events,
        )
        .expect("split to ground");
    assert_eq!(engine.world.actors[0].carried.gold.left_hand, 25);
    assert_eq!(engine.world.actors[0].carried.gold.sack, 460);
    assert_eq!(engine.world.ground_gold.values().next().unwrap().amount, 15);

    let claim = LootClaim {
        owner: LootOwnerId::Character(
            engine.world.actors[0]
                .character_id
                .clone()
                .expect("stable character"),
        ),
        basis: LootClaimBasis::KillingBlow,
    };
    let claimed = engine
        .create_ground_gold_pile(
            9,
            engine.world.actors[0].location.clone(),
            Some(claim.clone()),
        )
        .expect("claimed pile");
    engine
        .apply_player_move_gold(
            0,
            &GoldMoveSource::Ground {
                gold_pile_id: claimed.id.clone(),
            },
            &GoldMoveDestination::Carried {
                position: crate::model::CarriedGoldPosition::LeftHand,
            },
            &GoldMoveQuantity::Exact { amount: 4 },
            &mut events,
        )
        .expect("partial claimed-pile collection");
    assert_eq!(engine.world.ground_gold[&claimed.id].amount, 5);
    assert_eq!(
        engine.world.ground_gold[&claimed.id].loot_claim,
        Some(claim.clone())
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Event::GoldRelocated {
            amount: 4,
            loot_claim: Some(actual),
            ..
        } if actual == &claim
    )));

    let before = engine.world.clone();
    let error = engine
        .apply_player_move_gold(
            0,
            &GoldMoveSource::Carried {
                position: crate::model::CarriedGoldPosition::Sack,
            },
            &GoldMoveDestination::Carried {
                position: crate::model::CarriedGoldPosition::RightHand,
            },
            &GoldMoveQuantity::Exact { amount: 1 },
            &mut Vec::new(),
        )
        .expect_err("item and gold cannot share a hand");
    assert!(error.message().contains("occupied by an item"));
    assert_eq!(engine.world, before);
}

#[test]
fn bank_transactions_share_branches_and_emit_atomic_coordinator_receipts() {
    let mut engine = storage_engine();
    let player = engine.world.actors[0].clone();
    let character_id = player.character_id.expect("stable character");
    let pile = engine
        .create_ground_gold_pile(125, player.location.clone(), None)
        .expect("ground pile");
    let mut deposit_events = Vec::new();
    engine
        .apply_bank_deposit(
            0,
            "storage_counter_a",
            "bank_a",
            &pile.id,
            &mut deposit_events,
        )
        .expect("deposit through branch A");
    assert_eq!(
        engine.world.banks[&BankId::new("shared_bank")].balance(&character_id),
        125
    );
    assert!(!engine.world.ground_gold.contains_key(&pile.id));
    assert!(deposit_events.iter().any(|event| matches!(
        event,
        Event::BankBalanceChanged {
            before: 0,
            after: 125,
            reason: BankBalanceChangeReasonV1::Deposit,
            ..
        }
    )));
    assert!(deposit_events.iter().any(|event| matches!(
        event,
        Event::TransactionCommitted {
            source: TransactionSourceV1::BankDeposit { bank_id, .. },
            costs,
            rewards,
            ..
        } if bank_id == "shared_bank" && costs.len() == 1 && rewards.len() == 1
    )));

    let mut withdrawal_events = Vec::new();
    engine
        .apply_bank_withdrawal(0, "storage_counter_b", "bank_b", 40, &mut withdrawal_events)
        .expect("withdrawal through branch B");
    assert_eq!(
        engine.world.banks[&BankId::new("shared_bank")].balance(&character_id),
        85
    );
    assert_eq!(engine.world.ground_gold.values().next().unwrap().amount, 40);
    assert!(withdrawal_events.iter().any(|event| matches!(
        event,
        Event::BankBalanceChanged {
            before: 125,
            after: 85,
            reason: BankBalanceChangeReasonV1::Withdrawal,
            ..
        }
    )));

    let over_cap = engine
        .create_ground_gold_pile(201, player.location.clone(), None)
        .expect("over-cap pile");
    let before = engine.world.clone();
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            crate::model::PlayerIntent::DepositBankGold {
                service_id: "storage_counter_a".to_string(),
                capability_id: "bank_a".to_string(),
                gold_pile_id: over_cap.id,
            },
        )
        .expect_err("over-cap deposit");
    assert!(error.message().contains("transaction limit"));
    assert_eq!(engine.world, before);

    let next_sequence = engine.world.next_gold_sequence;
    engine.world.next_gold_sequence = u64::MAX;
    let before = engine.world.clone();
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            crate::model::PlayerIntent::WithdrawBankGold {
                service_id: "storage_counter_b".to_string(),
                capability_id: "bank_b".to_string(),
                amount: 1,
            },
        )
        .expect_err("late pile allocation failure must roll back the debit");
    assert!(error.message().contains("ground gold sequence overflow"));
    assert_eq!(engine.world, before);
    engine.world.next_gold_sequence = next_sequence;

    assert_eq!(
        engine.world.banks[&BankId::new("isolated_bank")].balance(&character_id),
        0
    );
    let isolated_pile = engine
        .create_ground_gold_pile(50, player.location.clone(), None)
        .expect("isolated bank pile");
    engine
        .apply_bank_deposit(
            0,
            "storage_counter_c",
            "bank_c",
            &isolated_pile.id,
            &mut Vec::new(),
        )
        .expect("deposit into isolated bank");
    assert_eq!(
        engine.world.banks[&BankId::new("isolated_bank")].balance(&character_id),
        50
    );
    assert_eq!(
        engine.world.banks[&BankId::new("shared_bank")].balance(&character_id),
        85
    );

    let (recipient_index, recipient_character_id) = add_recipient(&mut engine);
    assert_eq!(
        engine.world.banks[&BankId::new("shared_bank")].balance(&recipient_character_id),
        0
    );
    let recipient_pile = engine
        .create_ground_gold_pile(20, player.location.clone(), None)
        .expect("recipient pile");
    engine
        .apply_bank_deposit(
            recipient_index,
            "storage_counter_a",
            "bank_a",
            &recipient_pile.id,
            &mut Vec::new(),
        )
        .expect("recipient deposit");
    assert_eq!(
        engine.world.banks[&BankId::new("shared_bank")].balance(&recipient_character_id),
        20
    );
    assert_eq!(
        engine.world.banks[&BankId::new("shared_bank")].balance(&character_id),
        85
    );
}

#[test]
fn locker_relocation_is_ordered_capacity_bounded_and_shared_by_vault() {
    let mut engine = storage_engine();
    let character_id = engine.world.actors[0]
        .character_id
        .clone()
        .expect("stable character");
    let second = engine.world.item_instances["training_sword"].clone();
    engine
        .world
        .item_instances
        .insert("training_sword_2".to_string(), second);
    let third = engine.world.item_instances["training_sword"].clone();
    engine
        .world
        .item_instances
        .insert("training_sword_3".to_string(), third);
    engine.world.actors[0]
        .carried
        .items
        .insert(CarriedPosition::LeftHand, "training_sword_2".to_string());
    engine.world.actors[0]
        .carried
        .items
        .insert(CarriedPosition::SackItem1, "training_sword_3".to_string());

    let mut events = Vec::new();
    engine
        .apply_locker_deposit(
            0,
            "storage_counter_a",
            "locker_a",
            "training_sword",
            &mut events,
        )
        .expect("locker deposit");
    let vault_id = LockerVaultId::new("shared_vault");
    engine
        .apply_locker_deposit(
            0,
            "storage_counter_b",
            "locker_b",
            "training_sword_2",
            &mut events,
        )
        .expect("second ordered locker deposit");
    assert_eq!(
        engine.world.locker_vaults[&vault_id].contents(&character_id),
        ["training_sword", "training_sword_2"]
    );
    let before = engine.world.clone();
    let error = engine
        .validate_locker_deposit(0, "storage_counter_b", "locker_b", "training_sword_3")
        .expect_err("full locker");
    assert_eq!(error.reason(), ActionBlockedReasonV1::LockerFull);
    assert_eq!(engine.world, before);

    engine
        .apply_locker_withdrawal(
            0,
            "storage_counter_b",
            "locker_b",
            "training_sword",
            CarriedPosition::RightHand,
            &mut events,
        )
        .expect("shared-vault withdrawal");
    assert_eq!(
        engine.world.locker_vaults[&vault_id].contents(&character_id),
        ["training_sword_2"]
    );
    assert!(matches!(
        engine.item_location("training_sword").unwrap(),
        ItemLocation::Carried {
            position: CarriedPosition::RightHand,
            ..
        }
    ));

    engine
        .apply_locker_deposit(
            0,
            "storage_counter_c",
            "locker_c",
            "training_sword_3",
            &mut events,
        )
        .expect("separate vault deposit");
    assert_eq!(
        engine.world.locker_vaults[&LockerVaultId::new("isolated_vault")].contents(&character_id),
        ["training_sword_3"]
    );
    assert_eq!(
        engine.world.locker_vaults[&vault_id].contents(&character_id),
        ["training_sword_2"]
    );

    let (recipient_index, recipient_character_id) = add_recipient(&mut engine);
    let fourth = engine.world.item_instances["training_sword"].clone();
    engine
        .world
        .item_instances
        .insert("training_sword_4".to_string(), fourth);
    engine.world.actors[recipient_index]
        .carried
        .items
        .insert(CarriedPosition::LeftHand, "training_sword_4".to_string());
    engine
        .apply_locker_deposit(
            recipient_index,
            "storage_counter_a",
            "locker_a",
            "training_sword_4",
            &mut events,
        )
        .expect("recipient has an isolated locker within the shared vault");
    assert_eq!(
        engine.world.locker_vaults[&vault_id].contents(&recipient_character_id),
        ["training_sword_4"]
    );
    assert_eq!(
        engine.world.locker_vaults[&vault_id].contents(&character_id),
        ["training_sword_2"]
    );
    engine
        .validate_world_item_locations()
        .expect("locker locations remain authoritative");
}

#[test]
fn action_context_projects_command_ready_bank_locker_and_offer_surfaces() {
    let mut engine = storage_engine();
    let player = engine.world.actors[0].clone();
    let pile = engine
        .create_ground_gold_pile(125, player.location.clone(), None)
        .expect("ground pile");
    let (recipient_index, recipient_character_id) = add_recipient(&mut engine);
    let mut later_recipient = engine.world.actors[recipient_index].clone();
    later_recipient.id = "later_recipient".into();
    later_recipient.name = "Later Recipient".to_string();
    later_recipient.character_id = Some(character_id("character:storage:z_recipient"));
    later_recipient.timing.tie_break_order = engine.world.timing.next_tie_break_order;
    engine.world.timing.next_tie_break_order += 1;
    engine.world.actors.insert(1, later_recipient);

    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("observed storage context");
    assert_eq!(context.contract_version, 32);
    assert_eq!(context.carried.gold.sack, 500);
    assert_eq!(context.item_offer_actions.len(), 2);
    assert!(matches!(
        context.item_offer_actions[0]
            .command
            .as_ref()
            .map(|command| &command.intent),
        Some(PlayerIntentPayloadV1::OfferItem {
            recipient_character_id: recipient,
            item_instance_id,
        }) if recipient == &recipient_character_id && item_instance_id == "training_sword"
    ));

    let counter = context
        .services_here
        .iter()
        .find(|service| service.service_id == "storage_counter_a")
        .expect("storage counter");
    let bank = counter
        .capabilities
        .iter()
        .find_map(|capability| match capability {
            ServiceCapabilityViewV1::Bank {
                balance_gold,
                transaction_cap_gold,
                deposit_actions,
                withdrawal_actions,
                ..
            } => Some((
                *balance_gold,
                *transaction_cap_gold,
                deposit_actions,
                withdrawal_actions,
            )),
            _ => None,
        })
        .expect("bank view");
    assert_eq!((bank.0, bank.1), (0, 200));
    assert_eq!(bank.2.len(), 1);
    assert!(bank.2[0].enabled);
    assert!(matches!(
        bank.2[0].command.as_ref().map(|command| &command.intent),
        Some(PlayerIntentPayloadV1::DepositBankGold { gold_pile_id, .. })
            if gold_pile_id == &pile.id
    ));
    assert_eq!(bank.3.len(), 1);
    assert!(!bank.3[0].enabled);
    assert!(bank.3[0].blocked_reason.is_some());
    assert!(bank.3[0].command.is_none());

    let locker = counter
        .capabilities
        .iter()
        .find_map(|capability| match capability {
            ServiceCapabilityViewV1::Locker {
                capacity,
                item_count,
                deposit_actions,
                withdrawal_actions,
                ..
            } => Some((*capacity, *item_count, deposit_actions, withdrawal_actions)),
            _ => None,
        })
        .expect("locker view");
    assert_eq!((locker.0, locker.1), (2, 0));
    assert_eq!(locker.2.len(), 1);
    assert!(locker.2[0].enabled);
    assert!(locker.3.is_empty());

    engine
        .apply_item_offer(
            0,
            &recipient_character_id,
            "training_sword",
            &mut Vec::new(),
        )
        .expect("offer");
    let offered = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("offered context");
    assert!(offered.item_offer_actions.is_empty());
    assert!(offered.incoming_item_offers.is_empty());
    assert_eq!(offered.outgoing_item_offers.len(), 1);
    assert_eq!(offered.outgoing_item_offers[0].actions.len(), 1);
    assert!(matches!(
        offered.outgoing_item_offers[0].actions[0]
            .command
            .as_ref()
            .map(|command| &command.intent),
        Some(PlayerIntentPayloadV1::WithdrawItemOffer { item_instance_id })
            if item_instance_id == "training_sword"
    ));
}

#[test]
fn offers_accept_refuse_withdraw_and_separation_return_the_reserved_hand() {
    let mut accepted = storage_engine();
    let (recipient_index, recipient_character_id, _) = create_offer(&mut accepted);
    assert_eq!(
        accepted
            .item_at_position(0, CarriedPosition::RightHand)
            .unwrap(),
        Some("training_sword")
    );
    assert!(matches!(
        accepted.item_location("training_sword").unwrap(),
        ItemLocation::Offered { .. }
    ));
    let mut events = Vec::new();
    accepted
        .apply_accept_item_offer(
            recipient_index,
            "training_sword",
            CarriedPosition::LeftHand,
            &mut events,
        )
        .expect("offer acceptance");
    assert!(accepted.world.item_offers.is_empty());
    assert!(matches!(
        accepted.item_location("training_sword").unwrap(),
        ItemLocation::Carried {
            holder: ItemHolderId::Character(owner),
            position: CarriedPosition::LeftHand,
        } if owner == recipient_character_id
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ItemOfferCompleted {
            reason: ItemOfferCompletionReasonV1::Accepted,
            ..
        }
    )));

    let mut refused = storage_engine();
    let (recipient_index, _, _) = create_offer(&mut refused);
    let mut events = Vec::new();
    refused
        .apply_refuse_item_offer(recipient_index, "training_sword", &mut events)
        .expect("offer refusal");
    assert_eq!(
        refused
            .item_at_position(0, CarriedPosition::RightHand)
            .unwrap(),
        Some("training_sword")
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ItemOfferCompleted {
            reason: ItemOfferCompletionReasonV1::Refused,
            ..
        }
    )));

    let mut withdrawn = storage_engine();
    create_offer(&mut withdrawn);
    let mut events = Vec::new();
    withdrawn
        .apply_withdraw_item_offer(0, "training_sword", &mut events)
        .expect("sender withdrawal");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ItemOfferCompleted {
            reason: ItemOfferCompletionReasonV1::Withdrawn,
            ..
        }
    )));

    let mut separated = storage_engine();
    let (recipient_index, _, _) = create_offer(&mut separated);
    separated.world.actors[recipient_index].location.position.x += 1;
    let mut events = Vec::new();
    separated
        .reconcile_separated_item_offers(&mut events)
        .expect("separation return");
    assert_eq!(
        separated
            .item_at_position(0, CarriedPosition::RightHand)
            .unwrap(),
        Some("training_sword")
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ItemOfferCompleted {
            reason: ItemOfferCompletionReasonV1::Separated,
            ..
        }
    )));
}

#[test]
fn offer_reservations_exclude_ordinary_moves_and_reject_collisions() {
    let mut engine = storage_engine();
    let second = engine.world.item_instances["training_sword"].clone();
    engine
        .world
        .item_instances
        .insert("training_sword_2".to_string(), second);
    engine.world.actors[0]
        .carried
        .items
        .insert(CarriedPosition::LeftHand, "training_sword_2".to_string());
    let (recipient_index, recipient_character_id, _) = create_offer(&mut engine);
    let sender_character_id = engine.world.actors[0]
        .character_id
        .clone()
        .expect("sender identity");
    engine
        .validate_world_item_locations()
        .expect("offer starts with one authoritative location");

    let before = engine.world.clone();
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            crate::model::PlayerIntent::MoveItem {
                item_instance_id: "training_sword".to_string(),
                destination: crate::model::ItemMoveDestination::GroundHere,
            },
        )
        .expect_err("offered item cannot move through ordinary inventory");
    assert!(error.message().contains("reserved by an item offer"));
    assert_eq!(engine.world, before);

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            crate::model::PlayerIntent::MoveGold {
                source: GoldMoveSource::Carried {
                    position: crate::model::CarriedGoldPosition::Sack,
                },
                destination: GoldMoveDestination::Carried {
                    position: crate::model::CarriedGoldPosition::RightHand,
                },
                quantity: GoldMoveQuantity::Exact { amount: 1 },
            },
        )
        .expect_err("reserved hand cannot receive gold");
    assert!(error.message().contains("occupied by an item"));
    assert_eq!(engine.world, before);

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            crate::model::PlayerIntent::MoveItem {
                item_instance_id: "training_sword_2".to_string(),
                destination: crate::model::ItemMoveDestination::Carried {
                    position: CarriedPosition::RightHand,
                },
            },
        )
        .expect_err("reserved hand cannot receive another item");
    assert!(error.message().contains("occupied"));
    assert_eq!(engine.world, before);

    assert_eq!(
        engine
            .validate_accept_item_offer(0, "training_sword", CarriedPosition::SackItem1)
            .expect_err("sender cannot accept")
            .reason(),
        ActionBlockedReasonV1::InvalidTarget
    );
    assert_eq!(
        engine
            .validate_refuse_item_offer(0, "training_sword")
            .expect_err("sender cannot refuse")
            .reason(),
        ActionBlockedReasonV1::InvalidTarget
    );
    assert_eq!(
        engine
            .validate_withdraw_item_offer(recipient_index, "training_sword")
            .expect_err("recipient cannot withdraw")
            .reason(),
        ActionBlockedReasonV1::InvalidTarget
    );

    let mut duplicate_reservation = engine.clone();
    duplicate_reservation.world.item_offers.insert(
        "training_sword_2".to_string(),
        crate::model::ItemOfferState {
            sender_character_id: sender_character_id.clone(),
            recipient_character_id: recipient_character_id.clone(),
            source_position: CarriedPosition::RightHand,
        },
    );
    let error = duplicate_reservation
        .validate_world_item_locations()
        .expect_err("one sender hand cannot back two offers");
    assert!(
        error
            .message()
            .contains("multiple offers reserve the same sender hand")
    );

    let mut carried_collision = engine.clone();
    carried_collision.world.actors[0]
        .carried
        .items
        .remove(&CarriedPosition::LeftHand);
    carried_collision.world.actors[0]
        .carried
        .items
        .insert(CarriedPosition::RightHand, "training_sword_2".to_string());
    let error = carried_collision
        .validate_world_item_locations()
        .expect_err("reserved hand cannot also contain a carried item");
    assert!(
        error
            .message()
            .contains("offered source hand also contains a carried item")
    );

    let mut gold_collision = engine;
    gold_collision.world.actors[0].carried.gold.right_hand = 1;
    let error = gold_collision
        .validate_world_item_locations()
        .expect_err("reserved hand cannot also contain carried gold");
    assert!(
        error
            .message()
            .contains("offered source hand also contains carried gold")
    );
}
