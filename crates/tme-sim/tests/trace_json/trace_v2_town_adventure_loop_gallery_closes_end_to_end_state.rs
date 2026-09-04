use super::*;

#[test]
fn trace_v2_town_adventure_loop_gallery_closes_end_to_end_state() {
    let trace = run_trace_v2("town_adventure_loop_gallery.json", 7);
    let repeated = run_trace_v2("town_adventure_loop_gallery.json", 7);
    assert_eq!(
        serde_json::to_string(&trace).expect("first EH trace should serialize"),
        serde_json::to_string(&repeated).expect("repeated EH trace should serialize"),
        "the integrated town/adventure loop must be byte-for-byte deterministic"
    );

    assert_eq!(trace.header.scenario_id, "town_adventure_loop_gallery");
    assert_eq!(trace.header.seed, 7);
    assert_eq!(
        (
            trace.header.contract_version,
            trace.header.event_contract_version,
            trace.header.snapshot_contract_version,
            trace.header.observed_snapshot_contract_version,
            trace.header.action_context_contract_version,
            trace.header.intent_contract_version,
        ),
        (2, 41, 31, 30, 32, 26)
    );
    assert_eq!(trace.r#final.contract_version, 2);
    assert_eq!(trace.steps.len(), 27);
    assert!(
        trace.steps.iter().enumerate().all(|(index, step)| {
            step.step_index == index && step.command.contract_version == 26
        })
    );

    let commands: Vec<Value> = trace
        .steps
        .iter()
        .map(|step| serde_json::to_value(&step.command.intent).expect("command should serialize"))
        .collect();
    assert_eq!(
        commands,
        vec![
            json!({"physical_attack":{"mode":"fight","target_actor_id":"road_scavenger","authorization":"safe"}}),
            json!({"search_corpse":{"corpse_id":"corpse:1"}}),
            json!({"move_item":{"item_instance_id":"waystone_token","destination":{"kind":"carried","position":"sack_item_2"}}}),
            json!({"move_item":{"item_instance_id":"trade_charm","destination":{"kind":"carried","position":"sack_item_3"}}}),
            json!({"move_gold":{"source":{"kind":"ground","gold_pile_id":"gold:1"},"destination":{"kind":"carried","position":"sack"},"quantity":{"kind":"all"}}}),
            json!({"move_path":{"path":["east"]}}),
            json!({"traverse":{"kind":"stairs_up"}}),
            json!({"interact_with_npc":{"npc_actor_id":"route_keeper","interaction_id":"ask_about_waystone","item_instance_id":null}}),
            json!({"interact_with_npc":{"npc_actor_id":"route_keeper","interaction_id":"return_waystone","item_instance_id":"waystone_token"}}),
            json!({"use_item_service":{"service_id":"waystation_counter","capability_id":"appraisal","operation":"appraise","item_instance_id":"trade_charm"}}),
            json!({"sell_to_merchant":{"service_id":"waystation_counter","capability_id":"trail_wares","item_instance_id":"trade_charm"}}),
            json!({"move_gold":{"source":{"kind":"carried","position":"sack"},"destination":{"kind":"ground_here"},"quantity":{"kind":"exact","amount":15}}}),
            json!({"deposit_bank_gold":{"service_id":"waystation_counter","capability_id":"bank_access","gold_pile_id":"gold:2"}}),
            json!({"deposit_locker_item":{"service_id":"waystation_counter","capability_id":"locker_access","item_instance_id":"weathered_staff"}}),
            json!({"move_item":{"item_instance_id":"field_spell_book","destination":{"kind":"carried","position":"right_hand"}}}),
            json!({"critique":{"service_id":"waystation_counter","track_id":"wizard_magic"}}),
            json!({"train":{"service_id":"waystation_counter","offered_gold":7}}),
            json!({"learn_spell":{"spell_id":"ember_bolt"}}),
            json!({"move_item":{"item_instance_id":"field_spell_book","destination":{"kind":"carried","position":"sack_item_1"}}}),
            json!({"buy_from_merchant":{"service_id":"waystation_counter","capability_id":"trail_wares","item_instance_ids":["bright_staff_stock"]}}),
            json!({"move_item":{"item_instance_id":"bright_staff_stock","destination":{"kind":"carried","position":"right_hand"}}}),
            json!({"use_restoration_service":{"service_id":"waystation_counter","capability_id":"restoration","operation_id":"restore_hit_points","item_instance_id":null,"corpse_id":null}}),
            json!({"use_restoration_service":{"service_id":"waystation_counter","capability_id":"restoration","operation_id":"restore_magic_points","item_instance_id":null,"corpse_id":null}}),
            json!({"traverse":{"kind":"stairs_down"}}),
            json!({"physical_attack":{"mode":"fight","target_actor_id":"return_sentinel","authorization":"safe"}}),
            json!({"cast_spell":{"spell_id":"ember_bolt","target":{"actor":{"actor_id":"return_sentinel"}},"authorization":"safe"}}),
            json!("inspect"),
        ]
    );

    let first_events = &trace.steps[0].events;
    let defeated = first_events
        .iter()
        .position(|event| {
            matches!(event, tme_rules::Event::ActorDefeated {
            actor_id, credited_actor_id: Some(credited), ..
        } if actor_id == "road_scavenger" && credited == "player")
        })
        .expect("step 1 should defeat the road scavenger");
    let corpse_created = first_events
        .iter()
        .position(|event| matches!(event, tme_rules::Event::CorpseCreated {
            corpse_id, origin_actor_id, sequence, ..
        } if corpse_id.as_str() == "corpse:1" && origin_actor_id == "road_scavenger" && *sequence == 1))
        .expect("step 1 should create corpse:1");
    let retained_item = first_events
        .iter()
        .position(|event| {
            matches!(event, tme_rules::Event::ItemRelocated {
            item_instance_id,
            to: tme_rules::ItemLocationViewV1::Corpse { corpse_id, .. },
            reason: tme_rules::ItemRelocationReason::CorpseRetention,
            ..
        } if item_instance_id == "waystone_token" && corpse_id.as_str() == "corpse:1")
        })
        .expect("step 1 should retain the quest item in corpse:1");
    let retained_gold = first_events
        .iter()
        .position(|event| {
            matches!(event, tme_rules::Event::GoldRelocated {
            amount,
            to: tme_rules::GoldLocationViewV1::Corpse { corpse_id },
            reason: tme_rules::GoldRelocationReason::CorpseRetention,
            ..
        } if *amount == 20 && corpse_id.as_str() == "corpse:1")
        })
        .expect("step 1 should retain corpse gold");
    assert!(
        defeated < corpse_created
            && corpse_created < retained_item
            && retained_item < retained_gold
    );

    let search_events = &trace.steps[1].events;
    let released_item = search_events
        .iter()
        .position(|event| {
            matches!(event, tme_rules::Event::ItemRelocated {
            item_instance_id,
            reason: tme_rules::ItemRelocationReason::CorpseSearch,
            ..
        } if item_instance_id == "waystone_token")
        })
        .expect("step 2 should release the quest item");
    let released_gold = search_events
        .iter()
        .position(|event| {
            matches!(event, tme_rules::Event::GoldRelocated {
            amount,
            to: tme_rules::GoldLocationViewV1::Ground { gold_pile_id, .. },
            reason: tme_rules::GoldRelocationReason::CorpseSearch,
            ..
        } if *amount == 20 && gold_pile_id.as_str() == "gold:1")
        })
        .expect("step 2 should release gold:1");
    let searched = search_events
        .iter()
        .position(|event| {
            matches!(event, tme_rules::Event::CorpseSearched {
            corpse_id, items_released, gold_released, ..
        } if corpse_id.as_str() == "corpse:1" && *items_released == 2 && *gold_released == 20)
        })
        .expect("step 2 should search corpse:1");
    assert!(released_item < released_gold && released_gold < searched);
    assert!(trace.steps[2].events.iter().any(|event| matches!(event,
        tme_rules::Event::ItemRelocated {
            item_instance_id,
            to: tme_rules::ItemLocationViewV1::Carried { actor_id, position: tme_rules::CarriedPosition::SackItem2 },
            reason: tme_rules::ItemRelocationReason::PlayerMove,
            ..
        } if item_instance_id == "waystone_token" && actor_id == "player"
    )));
    assert!(trace.steps[3].events.iter().any(|event| matches!(event,
        tme_rules::Event::ItemRelocated {
            item_instance_id,
            to: tme_rules::ItemLocationViewV1::Carried { actor_id, position: tme_rules::CarriedPosition::SackItem3 },
            reason: tme_rules::ItemRelocationReason::PlayerMove,
            ..
        } if item_instance_id == "trade_charm" && actor_id == "player"
    )));
    assert!(trace.steps[4].events.iter().any(|event| matches!(event,
        tme_rules::Event::GoldRelocated {
            amount,
            from: tme_rules::GoldLocationViewV1::Ground { gold_pile_id, .. },
            to: tme_rules::GoldLocationViewV1::Carried { actor_id, position: tme_rules::CarriedGoldPosition::Sack },
            reason: tme_rules::GoldRelocationReason::PlayerMove,
            ..
        } if *amount == 20 && gold_pile_id.as_str() == "gold:1" && actor_id == "player"
    )));

    assert!(trace.steps[7].events.iter().any(|event| matches!(event,
        tme_rules::Event::QuestStateChanged {
            quest_id, before_stage_id: None, after_stage_id, ..
        } if quest_id == "waystone_recovery" && after_stage_id == "awaiting_waystone"
    )));
    assert!(trace.steps[8].events.iter().any(|event| matches!(event,
        tme_rules::Event::QuestStateChanged {
            quest_id, before_stage_id: Some(before), after_stage_id, ..
        } if quest_id == "waystone_recovery" && before == "awaiting_waystone" && after_stage_id == "completed"
    )));
    assert!(trace.steps[8].events.iter().any(|event| matches!(event,
        tme_rules::Event::TransactionCommitted { costs, .. }
            if costs.iter().any(|cost| matches!(cost,
                tme_rules::TransactionCostReceiptV1::SelectedCarriedItem {
                    item_instance_id, item_definition_id, consumed_quantity, remaining_quantity
                } if item_instance_id == "waystone_token"
                    && item_definition_id == "waystone_token"
                    && *consumed_quantity == 1
                    && *remaining_quantity == 0
            ))
    )));
    assert!(trace.steps[9].events.iter().any(|event| matches!(event,
        tme_rules::Event::ItemAppraised {
            item_instance_id, unit_value_gold, total_value_gold, ..
        } if item_instance_id == "trade_charm" && *unit_value_gold == 8 && *total_value_gold == 8
    )));
    assert!(trace.steps[10].events.iter().any(|event| matches!(event,
        tme_rules::Event::ItemRelocated {
            item_instance_id,
            to: tme_rules::ItemLocationViewV1::Merchant { service_id, capability_id },
            reason: tme_rules::ItemRelocationReason::MerchantSale,
            ..
        } if item_instance_id == "trade_charm"
            && service_id == "waystation_counter"
            && capability_id == "trail_wares"
    )));
    assert!(trace.steps[10].events.iter().any(|event| matches!(event,
        tme_rules::Event::GoldChanged { actor_id, amount, .. }
            if actor_id == "player" && *amount == 8
    )));
    assert!(trace.steps[12].events.iter().any(|event| matches!(event,
        tme_rules::Event::BankBalanceChanged {
            actor_id, bank_id, amount, before, after, ..
        } if actor_id == "player"
            && bank_id == "waystation_bank"
            && *amount == 15
            && *before == 0
            && *after == 15
    )));
    assert!(trace.steps[13].events.iter().any(|event| matches!(event,
        tme_rules::Event::ItemRelocated {
            item_instance_id,
            to: tme_rules::ItemLocationViewV1::Locker { vault_id, owner_character_id },
            reason: tme_rules::ItemRelocationReason::LockerDeposit,
            ..
        } if item_instance_id == "weathered_staff"
            && vault_id == "waystation_vault"
            && owner_character_id.as_str() == "character:town_adventure_loop_gallery:primary"
    )));
    assert!(trace.steps[15].events.iter().any(|event| matches!(event,
        tme_rules::Event::SkillCritiqued { actor_id, service_id, track_id, .. }
            if actor_id == "player" && service_id == "waystation_counter" && track_id == "wizard_magic"
    )));
    assert!(trace.steps[16].events.iter().any(|event| matches!(event,
        tme_rules::Event::TrainingPurchased {
            actor_id, track_id, spent_gold, previous_learning_rate, new_learning_rate, ..
        } if actor_id == "player"
            && track_id == "wizard_magic"
            && *spent_gold == 7
            && *previous_learning_rate == 1
            && *new_learning_rate == 2
    )));
    assert!(trace.steps[17].events.iter().any(|event| matches!(event,
        tme_rules::Event::SpellLearned {
            actor_id, spell_id, lane, gold_cost, spell_book_item_instance_id, ..
        } if actor_id == "player"
            && spell_id == "ember_bolt"
            && lane == "wizard_magic"
            && *gold_cost == 25
            && spell_book_item_instance_id == "field_spell_book"
    )));
    assert!(trace.steps[19].events.iter().any(|event| matches!(event,
        tme_rules::Event::ItemRelocated {
            item_instance_id,
            from: tme_rules::ItemLocationViewV1::Merchant { service_id, capability_id },
            reason: tme_rules::ItemRelocationReason::MerchantPurchase,
            ..
        } if item_instance_id == "bright_staff_stock"
            && service_id == "waystation_counter"
            && capability_id == "trail_wares"
    )));
    assert!(trace.steps[20].events.iter().any(|event| matches!(event,
        tme_rules::Event::ItemRelocated {
            item_instance_id,
            to: tme_rules::ItemLocationViewV1::Carried { actor_id, position: tme_rules::CarriedPosition::RightHand },
            reason: tme_rules::ItemRelocationReason::PlayerMove,
            ..
        } if item_instance_id == "bright_staff_stock" && actor_id == "player"
    )));
    assert!(trace.steps[21].events.iter().any(|event| matches!(event,
        tme_rules::Event::ResourceRestored {
            actor_id, resource: tme_rules::ResourceKind::Hp, before, after, maximum, ..
        } if actor_id == "player" && *before == 20 && *after == 40 && *maximum == 40
    )));
    assert!(trace.steps[22].events.iter().any(|event| matches!(event,
        tme_rules::Event::ResourceRestored {
            actor_id, resource: tme_rules::ResourceKind::Mp, before, after, maximum, ..
        } if actor_id == "player" && *before == 11 && *after == 40 && *maximum == 40
    )));
    assert!(trace.steps[24].events.iter().any(|event| matches!(event,
        tme_rules::Event::Attacked {
            attacker_id, defender_id, mode: tme_rules::PhysicalAttackMode::Fight, damage, defender_hp, ..
        } if attacker_id == "player" && defender_id == "return_sentinel" && *damage == 44 && *defender_hp == 56
    )));
    assert!(trace.steps[25].events.iter().any(|event| matches!(event,
        tme_rules::Event::SpellDamaged {
            caster_id, spell_id, target_id, damage, hp, ..
        } if caster_id == "player"
            && spell_id == "ember_bolt"
            && target_id == "return_sentinel"
            && *damage == 3
            && *hp == 53
    )));

    let arrival_context = &trace.steps[6].after_action_context;
    assert_eq!(arrival_context.position.level, "waystation");
    assert_eq!(
        arrival_context.position.position,
        tme_rules::Coord { x: 2, y: 1 }
    );
    assert_eq!(arrival_context.services_here.len(), 1);
    let arrival_service = &arrival_context.services_here[0];
    assert_eq!(arrival_service.service_id, "waystation_counter");
    assert_eq!(arrival_service.capabilities.len(), 8);
    assert!(matches!(&arrival_service.capabilities[0],
        tme_rules::ServiceCapabilityViewV1::SkillTraining { capability_id, offered_track_ids, .. }
            if capability_id == "wizard_training" && offered_track_ids == &["wizard_magic"]
    ));
    assert!(matches!(&arrival_service.capabilities[1],
        tme_rules::ServiceCapabilityViewV1::SkillCritique { capability_id, .. }
            if capability_id == "wizard_critique"
    ));
    assert!(matches!(&arrival_service.capabilities[2],
        tme_rules::ServiceCapabilityViewV1::SpellTeaching { capability_id, spell_ids, .. }
            if capability_id == "spell_teaching" && spell_ids == &["ember_bolt"]
    ));
    assert!(matches!(&arrival_service.capabilities[3],
        tme_rules::ServiceCapabilityViewV1::Merchant { capability_id, .. }
            if capability_id == "trail_wares"
    ));
    assert!(matches!(&arrival_service.capabilities[4],
        tme_rules::ServiceCapabilityViewV1::ItemService { capability_id, .. }
            if capability_id == "appraisal"
    ));
    assert!(matches!(&arrival_service.capabilities[5],
        tme_rules::ServiceCapabilityViewV1::Bank { capability_id, bank_id, .. }
            if capability_id == "bank_access" && bank_id == "waystation_bank"
    ));
    assert!(matches!(&arrival_service.capabilities[6],
        tme_rules::ServiceCapabilityViewV1::Locker { capability_id, vault_id, .. }
            if capability_id == "locker_access" && vault_id == "waystation_vault"
    ));
    assert!(matches!(&arrival_service.capabilities[7],
        tme_rules::ServiceCapabilityViewV1::Restoration { capability_id, .. }
            if capability_id == "restoration"
    ));
    let route_keeper = arrival_context
        .npcs_here
        .iter()
        .find(|npc| npc.actor_id == "route_keeper")
        .expect("the route keeper should be present at town arrival");
    let ask = route_keeper
        .interactions
        .iter()
        .find(|interaction| interaction.interaction_id == "ask_about_waystone")
        .expect("the route keeper should expose the quest-start interaction");
    assert!(ask.actions.iter().any(|action| action.enabled
        && matches!(
            action.command.as_ref().map(|command| &command.intent),
            Some(tme_rules::PlayerIntentPayloadV1::InteractWithNpc {
                npc_actor_id,
                interaction_id,
                item_instance_id: None,
            }) if npc_actor_id == "route_keeper" && interaction_id == "ask_about_waystone"
        )));

    let post_restoration = &trace.steps[22];
    let post_restoration_player = post_restoration
        .after_debug_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("step 23 should retain the player");
    let post_restoration_character = post_restoration_player
        .character
        .as_ref()
        .expect("step 23 should retain the player character sheet");
    assert_eq!(post_restoration_character.resources.hp, 40);
    assert_eq!(post_restoration_character.resources.mp, 40);
    let post_service = post_restoration
        .after_action_context
        .services_here
        .iter()
        .find(|service| service.service_id == "waystation_counter")
        .expect("step 23 action context should own the grouped service state");
    assert_eq!(post_service.capabilities.len(), 8);
    let bank = post_service
        .capabilities
        .iter()
        .find_map(|capability| match capability {
            tme_rules::ServiceCapabilityViewV1::Bank {
                capability_id,
                bank_id,
                balance_gold,
                transaction_cap_gold,
                ..
            } => Some((capability_id, bank_id, balance_gold, transaction_cap_gold)),
            _ => None,
        })
        .expect("step 23 context should expose the bank capability");
    assert_eq!(
        (bank.0.as_str(), bank.1.as_str(), *bank.2, *bank.3),
        ("bank_access", "waystation_bank", 15, 80)
    );
    let locker = post_service
        .capabilities
        .iter()
        .find_map(|capability| match capability {
            tme_rules::ServiceCapabilityViewV1::Locker {
                capability_id,
                vault_id,
                capacity,
                item_count,
                items,
                ..
            } => Some((capability_id, vault_id, capacity, item_count, items)),
            _ => None,
        })
        .expect("step 23 context should expose the locker capability");
    assert_eq!(
        (locker.0.as_str(), locker.1.as_str(), *locker.2, *locker.3),
        ("locker_access", "waystation_vault", 2, 1)
    );
    assert_eq!(locker.4.len(), 1);
    assert_eq!(locker.4[0].item_instance_id, "weathered_staff");
    let merchant = post_service
        .capabilities
        .iter()
        .find_map(|capability| match capability {
            tme_rules::ServiceCapabilityViewV1::Merchant {
                capability_id,
                listings,
                ..
            } => Some((capability_id, listings)),
            _ => None,
        })
        .expect("step 23 context should expose the merchant capability");
    assert_eq!(merchant.0, "trail_wares");
    assert!(merchant.1.iter().any(|listing| {
        listing.item.item_instance_id == "trade_charm"
            && listing.origin == tme_rules::MerchantListingOriginViewV1::PawnPool
            && listing.price_gold == 32
    }));

    let after_departure = &trace.steps[23..];
    assert!(!after_departure.iter().any(|step| matches!(
        &step.command.intent,
        tme_rules::PlayerIntentPayloadV1::DepositBankGold { .. }
            | tme_rules::PlayerIntentPayloadV1::WithdrawBankGold { .. }
            | tme_rules::PlayerIntentPayloadV1::DepositLockerItem { .. }
            | tme_rules::PlayerIntentPayloadV1::WithdrawLockerItem { .. }
            | tme_rules::PlayerIntentPayloadV1::BuyFromMerchant { .. }
            | tme_rules::PlayerIntentPayloadV1::SellToMerchant { .. }
    )));
    assert!(
        !after_departure
            .iter()
            .flat_map(|step| step.events.iter())
            .any(|event| match event {
                tme_rules::Event::BankBalanceChanged { .. } => true,
                tme_rules::Event::GoldRelocated { reason, .. } => matches!(
                    reason,
                    tme_rules::GoldRelocationReason::BankDeposit
                        | tme_rules::GoldRelocationReason::BankWithdrawal
                ),
                tme_rules::Event::ItemRelocated { reason, .. } => matches!(
                    reason,
                    tme_rules::ItemRelocationReason::MerchantPurchase
                        | tme_rules::ItemRelocationReason::MerchantSale
                        | tme_rules::ItemRelocationReason::LockerDeposit
                        | tme_rules::ItemRelocationReason::LockerWithdrawal
                ),
                tme_rules::Event::TransactionCommitted { source, .. } => matches!(
                    source,
                    tme_rules::TransactionSourceV1::MerchantPurchase { .. }
                        | tme_rules::TransactionSourceV1::MerchantSale { .. }
                        | tme_rules::TransactionSourceV1::BankDeposit { .. }
                        | tme_rules::TransactionSourceV1::BankWithdrawal { .. }
                ),
                _ => false,
            })
    );

    let danger_context = &trace.steps[23].after_action_context;
    assert_eq!(danger_context.position.level, "trailhead");
    assert_eq!(
        danger_context.position.position,
        tme_rules::Coord { x: 3, y: 1 }
    );
    let sentinel_target = danger_context
        .attack_targets
        .iter()
        .find(|target| target.actor_id == "return_sentinel")
        .expect("return context should expose the sentinel target");
    let fight = sentinel_target
        .physical_attacks
        .iter()
        .find(|attack| attack.mode == tme_rules::PhysicalAttackMode::Fight)
        .expect("return context should expose Fight");
    assert!(fight.enabled);
    assert_eq!(fight.attack_safety, tme_rules::AttackSafety::OpenHostile);
    assert!(
        matches!(fight.command.as_ref().map(|command| &command.intent),
            Some(tme_rules::PlayerIntentPayloadV1::PhysicalAttack {
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id,
                authorization: tme_rules::HostilityAuthorization::Safe,
            }) if target_actor_id == "return_sentinel"
        )
    );
    let ember = danger_context
        .spell_actions
        .iter()
        .find(|spell| spell.spell_id == "ember_bolt")
        .expect("return context should expose the learned spell descriptor");
    assert!(ember.cast.enabled);
    assert!(ember.cast.requires_target_selection);
    assert!(
        ember.cast.command.is_none(),
        "targeted spell context must remain descriptor-only"
    );
    assert!(matches!(&trace.steps[25].command.intent,
        tme_rules::PlayerIntentPayloadV1::CastSpell {
            spell_id,
            target: Some(tme_rules::SpellTarget::Actor { actor_id }),
            ..
        } if spell_id == "ember_bolt" && actor_id == "return_sentinel"
    ));

    let final_debug = &trace.r#final.final_debug_snapshot;
    let final_player = final_debug
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("final debug snapshot should retain the player");
    assert!(matches!(
        final_player.life_state,
        tme_rules::ActorLifeStateViewV1::Alive
    ));
    assert_eq!(final_player.location.level, "trailhead");
    assert_eq!(
        final_player.location.position,
        tme_rules::Coord { x: 3, y: 1 }
    );
    assert_eq!((final_player.hp, final_player.max_hp), (40, 40));
    assert_eq!(final_player.carried.gold.sack, 64);
    assert!(final_player.carried.items.iter().any(|item| {
        item.item.item_instance_id == "bright_staff_stock"
            && item.position == tme_rules::CarriedPosition::RightHand
    }));
    assert!(final_player.carried.items.iter().any(|item| {
        item.item.item_instance_id == "field_spell_book"
            && item.position == tme_rules::CarriedPosition::SackItem1
    }));
    let final_character = final_player
        .character
        .as_ref()
        .expect("final debug snapshot should retain the character sheet");
    assert_eq!(
        (
            final_character.resources.hp,
            final_character.resources.mp,
            final_character.resources.stamina
        ),
        (40, 39, 19)
    );
    assert!(
        final_character
            .known_spells
            .iter()
            .any(|spell| { spell.spell_id == "ember_bolt" && spell.lane == "wizard_magic" })
    );
    assert!(
        final_character
            .skill_ledger
            .iter()
            .any(|skill| { skill.track_id == "wizard_magic" && skill.learning_rate == 2 })
    );
    let final_quest = final_debug
        .quest_states
        .iter()
        .find(|state| state.quest.quest_id == "waystone_recovery")
        .expect("final debug snapshot should retain the quest state");
    assert_eq!(
        final_quest.character_id.as_str(),
        "character:town_adventure_loop_gallery:primary"
    );
    assert_eq!(final_quest.quest.stage_id, "completed");
    assert!(final_quest.quest.terminal);
    let final_corpse = final_debug
        .corpses
        .iter()
        .find(|corpse| corpse.corpse_id.as_str() == "corpse:1")
        .expect("final debug snapshot should retain corpse:1");
    assert_eq!(final_corpse.origin_actor_id, "road_scavenger");
    assert!(final_corpse.searched);
    let scavenger = final_debug
        .actors
        .iter()
        .find(|actor| actor.id == "road_scavenger")
        .expect("final debug snapshot should retain the defeated scavenger");
    assert!(matches!(
        scavenger.life_state,
        tme_rules::ActorLifeStateViewV1::Dead
    ));
    let sentinel = final_debug
        .actors
        .iter()
        .find(|actor| actor.id == "return_sentinel")
        .expect("final debug snapshot should retain the sentinel");
    assert!(matches!(
        sentinel.life_state,
        tme_rules::ActorLifeStateViewV1::Alive
    ));
    assert_eq!((sentinel.hp, sentinel.max_hp), (53, 100));

    let final_observed = &trace.r#final.final_observed_snapshot;
    assert_eq!(final_observed.observation_center.level, "trailhead");
    assert_eq!(
        final_observed.observation_center.position,
        tme_rules::Coord { x: 3, y: 1 }
    );
    assert!(
        !final_observed
            .actors
            .iter()
            .any(|actor| actor.id == "route_keeper")
    );
    assert!(
        final_observed
            .actors
            .iter()
            .filter(|actor| actor.id != "player")
            .all(|actor| actor.character.is_none())
    );
    assert!(trace.r#final.final_action_context.services_here.is_empty());
    let observed_value =
        serde_json::to_value(final_observed).expect("observed snapshot serializes");
    assert!(observed_value.get("quest_states").is_none());
    assert!(observed_value.get("social_relations").is_none());
    assert!(observed_value.get("spell_social").is_none());
}

#[test]
fn trace_v2_world_topology_gallery_composes_all_navigation_owners() {
    let trace = run_trace_v2("world_topology_gallery.json", 7);
    assert_eq!(trace.steps.len(), 9);
    assert!(
        trace.steps[0]
            .events
            .iter()
            .any(|event| matches!(event, tme_rules::Event::PortalCreated { .. }))
    );

    let transitions = trace
        .steps
        .iter()
        .flat_map(|step| &step.events)
        .filter_map(|event| match event {
            tme_rules::Event::WorldTransition {
                from,
                to,
                navigation,
                ..
            } => Some((from, to, navigation)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(transitions.len(), 7);
    assert!(transitions.iter().any(|(_, _, kind)| matches!(
        kind,
        tme_rules::NavigationKind::Stairs {
            direction: tme_rules::VerticalDirection::Down
        }
    )));
    assert!(transitions.iter().any(|(_, _, kind)| matches!(
        kind,
        tme_rules::NavigationKind::Climb {
            direction: tme_rules::VerticalDirection::Up
        }
    )));
    for kind in [
        tme_rules::NavigationKind::Door,
        tme_rules::NavigationKind::Pit,
        tme_rules::NavigationKind::Passage,
        tme_rules::NavigationKind::Portal,
    ] {
        assert!(
            transitions.iter().any(|(_, _, actual)| **actual == kind),
            "missing transition kind {kind:?}"
        );
    }
    assert!(transitions.iter().any(|(from, to, kind)| **kind
        == tme_rules::NavigationKind::Passage
        && from.realm == "realm_0"
        && to.realm == "realm_1"));

    let swim_step = &trace.steps[2];
    let preview = swim_step
        .preview
        .as_ref()
        .expect("layered route has a preview");
    assert!(preview.steps.iter().any(|step| matches!(
        step.outcome,
        tme_rules::PathPreviewStepOutcomeV1::Moved {
            kind: tme_rules::TransitionKindViewV1::Swim
        }
    )));
    let final_player = trace
        .r#final
        .final_debug_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("final player");
    assert_eq!(final_player.location.realm, "realm_0");
    assert_eq!(final_player.location.level, "door_hall");
}
