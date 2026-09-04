use super::*;

#[test]
fn offers_preserve_binding_unwind_for_death_and_reject_missing_parties() {
    let mut bound = storage_engine();
    let sender_character_id = bound.world.actors[0]
        .character_id
        .clone()
        .expect("sender identity");
    bound
        .world
        .item_instances
        .get_mut("training_sword")
        .expect("training sword")
        .binding = ItemBindingState::Bound {
        character_id: sender_character_id.clone(),
    };
    let (recipient_index, _, _) = create_offer(&mut bound);
    bound
        .apply_accept_item_offer(
            recipient_index,
            "training_sword",
            CarriedPosition::LeftHand,
            &mut Vec::new(),
        )
        .expect("non-owner possession remains legal");
    assert_eq!(
        bound
            .item_at_position(recipient_index, CarriedPosition::LeftHand)
            .unwrap(),
        Some("training_sword")
    );
    assert_eq!(
        bound.world.item_instances["training_sword"].binding,
        ItemBindingState::Bound {
            character_id: sender_character_id,
        }
    );
    assert!(!bound.world.item_offers.contains_key("training_sword"));

    let mut recipient_defeat = storage_engine();
    let (recipient_index, _, _) = create_offer(&mut recipient_defeat);
    let mut events = Vec::new();
    recipient_defeat
        .unwind_item_offers_for_defeat(recipient_index, &mut events)
        .expect("recipient death unwind");
    assert_eq!(
        recipient_defeat
            .item_at_position(0, CarriedPosition::RightHand)
            .unwrap(),
        Some("training_sword")
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ItemOfferCompleted {
            reason: ItemOfferCompletionReasonV1::RecipientDefeated,
            ..
        }
    )));

    let mut sender_defeat = storage_engine();
    create_offer(&mut sender_defeat);
    let mut events = Vec::new();
    sender_defeat
        .unwind_item_offers_for_defeat(0, &mut events)
        .expect("sender death unwind");
    assert_eq!(
        sender_defeat
            .item_at_position(0, CarriedPosition::RightHand)
            .unwrap(),
        Some("training_sword")
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ItemOfferCompleted {
            reason: ItemOfferCompletionReasonV1::SenderDefeated,
            ..
        }
    )));

    let mut missing = storage_engine();
    create_offer(&mut missing);
    missing.world.actors.pop();
    let before = missing.world.clone();
    let error = missing
        .reconcile_separated_item_offers(&mut Vec::new())
        .expect_err("missing offer party is invariant failure");
    assert!(error.message().contains("unknown item holder"));
    assert_eq!(missing.world, before);
}

#[test]
fn death_unwinds_sender_offer_before_inventory_relocation_and_conserves_positioned_gold() {
    let mut offered = storage_engine();
    create_offer(&mut offered);
    let mut events = Vec::new();
    offered
        .resolve_actor_defeat(
            0,
            DefeatContext {
                cause: DeathCause::Physical,
                credited_actor_id: None,
                direct_social_actor_id: None,
                spell_damage_credit: None,
                hostile_authority: None,
            },
            &mut events,
        )
        .expect("sender defeat");
    assert!(offered.world.item_offers.is_empty());
    assert!(matches!(
        offered.item_location("training_sword").unwrap(),
        ItemLocation::Ground { .. }
    ));
    let completed = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::ItemOfferCompleted {
                    reason: ItemOfferCompletionReasonV1::SenderDefeated,
                    ..
                }
            )
        })
        .expect("offer completion event");
    let relocated = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::ItemRelocated {
                    item_instance_id,
                    reason: ItemRelocationReason::DeathDrop,
                    ..
                } if item_instance_id == "training_sword"
            )
        })
        .expect("ordinary death relocation event");
    assert!(completed < relocated);

    let mut gold = storage_engine();
    let sword = gold.world.actors[0]
        .carried
        .items
        .remove(&CarriedPosition::RightHand)
        .expect("right-hand sword");
    gold.world.actors[0]
        .carried
        .items
        .insert(CarriedPosition::SackItem1, sword);
    gold.world.actors[0].carried.gold = CarriedGold {
        left_hand: 10,
        right_hand: 20,
        sack: 30,
    };
    let mut events = Vec::new();
    gold.resolve_actor_defeat(
        0,
        DefeatContext {
            cause: DeathCause::Physical,
            credited_actor_id: None,
            direct_social_actor_id: None,
            spell_damage_credit: None,
            hostile_authority: None,
        },
        &mut events,
    )
    .expect("positioned-gold defeat");
    assert_eq!(gold.world.actors[0].carried.gold, CarriedGold::default());
    assert_eq!(gold.world.corpses.values().next().expect("corpse").gold, 60);
    let relocated = events
        .iter()
        .filter_map(|event| match event {
            Event::GoldRelocated {
                amount,
                from: crate::events::GoldLocationViewV1::Carried { position, .. },
                reason: crate::events::GoldRelocationReason::CorpseRetention,
                ..
            } => Some((*position, *amount)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        relocated,
        [
            (crate::model::CarriedGoldPosition::LeftHand, 10),
            (crate::model::CarriedGoldPosition::RightHand, 20),
            (crate::model::CarriedGoldPosition::Sack, 30),
        ]
    );
}
