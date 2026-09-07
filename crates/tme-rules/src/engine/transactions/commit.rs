//! Coordinated commit through the owning mutation seams.

use super::*;

impl Engine {
    pub(in crate::engine) fn commit_transaction(
        &mut self,
        actor_index: usize,
        plan: TransactionPlan,
    ) -> Result<TransactionCommitReceipt, StepError> {
        let mut costs = Vec::new();
        let mut rewards = Vec::new();
        let mut delegated_events = Vec::new();

        for cost in &plan.costs {
            match cost {
                PlannedCost::CarriedGold { amount } => {
                    let before =
                        self.carried_gold_at(actor_index, crate::model::CarriedGoldPosition::Sack)?;
                    let after = self.change_carried_gold_at(
                        actor_index,
                        crate::model::CarriedGoldPosition::Sack,
                        -*amount,
                    )?;
                    costs.push(TransactionCostReceiptV1::CarriedGold {
                        amount: *amount,
                        position: crate::model::CarriedGoldPosition::Sack,
                        before,
                        after,
                    });
                    delegated_events.push(Event::GoldChanged {
                        actor_id: plan.actor_id.clone(),
                        actor: plan.actor_name.clone(),
                        amount: -*amount,
                        new_total: after,
                    });
                }
                PlannedCost::SelectedCarriedItem { quantity } => {
                    let instance_id =
                        plan.selected_item_instance_id.as_deref().ok_or_else(|| {
                            StepError::new("captured selected-item cost has no item instance")
                        })?;
                    let definition_id = self.item_instance(instance_id)?.definition_id.clone();
                    let remaining =
                        self.consume_carried_quantity(actor_index, instance_id, *quantity)?;
                    costs.push(TransactionCostReceiptV1::SelectedCarriedItem {
                        item_instance_id: instance_id.to_string(),
                        item_definition_id: definition_id,
                        consumed_quantity: *quantity,
                        remaining_quantity: remaining,
                    });
                }
                PlannedCost::MerchantItem {
                    item_instance_id,
                    expected,
                    destination,
                    listing,
                } => {
                    let instance = self.item_instance(item_instance_id)?.clone();
                    let from = self.location_view(expected)?;
                    let to = self.location_view(destination)?;
                    self.relocate_items_with_events(
                        actor_index,
                        vec![crate::engine::inventory::ItemRelocation {
                            item_instance_id: item_instance_id.clone(),
                            expected: expected.clone(),
                            destination: destination.clone(),
                            loot_claim: None,
                            merchant_listing: Some(listing.clone()),
                        }],
                        crate::events::ItemRelocationReason::MerchantSale,
                        &mut delegated_events,
                    )?;
                    costs.push(TransactionCostReceiptV1::MerchantItem {
                        item_instance_id: item_instance_id.clone(),
                        item_definition_id: instance.definition_id,
                        quantity: instance.quantity,
                        from,
                        to,
                        pawn_listing_price_gold: listing.price_gold,
                    });
                }
                PlannedCost::GroundGoldPile {
                    gold_pile_id,
                    amount,
                    bank_id,
                    character_id,
                } => {
                    let pile = self.consume_ground_gold_pile(gold_pile_id)?;
                    if pile.amount != *amount {
                        return Err(StepError::new(
                            "captured bank deposit pile amount changed before commit",
                        ));
                    }
                    let from = crate::events::GoldLocationViewV1::Ground {
                        gold_pile_id: pile.id.clone(),
                        location: pile.location.clone(),
                    };
                    delegated_events.push(Event::GoldRelocated {
                        actor_id: plan.actor_id.clone(),
                        actor: plan.actor_name.clone(),
                        amount: *amount,
                        from: from.clone(),
                        to: crate::events::GoldLocationViewV1::Bank {
                            bank_id: bank_id.as_str().to_string(),
                            character_id: character_id.clone(),
                        },
                        reason: crate::events::GoldRelocationReason::BankDeposit,
                        loot_claim: pile.loot_claim,
                    });
                    costs.push(TransactionCostReceiptV1::GroundGoldPile {
                        gold_pile_id: pile.id,
                        amount: *amount,
                        from,
                    });
                }
                PlannedCost::BankBalance {
                    bank_id,
                    character_id,
                    amount,
                } => {
                    let bank = self
                        .world
                        .banks
                        .get_mut(bank_id)
                        .ok_or_else(|| StepError::new("captured bank state is missing"))?;
                    let before = bank.balance(character_id);
                    let after = before
                        .checked_sub(*amount)
                        .filter(|after| *after >= 0)
                        .ok_or_else(|| StepError::new("captured bank balance cannot cover cost"))?;
                    bank.balances.insert(character_id.clone(), after);
                    costs.push(TransactionCostReceiptV1::BankBalance {
                        bank_id: bank_id.as_str().to_string(),
                        character_id: character_id.clone(),
                        amount: *amount,
                        before,
                        after,
                    });
                    delegated_events.push(Event::BankBalanceChanged {
                        actor_id: plan.actor_id.clone(),
                        actor: plan.actor_name.clone(),
                        bank_id: bank_id.as_str().to_string(),
                        character_id: character_id.clone(),
                        amount: *amount,
                        before,
                        after,
                        reason: crate::events::BankBalanceChangeReasonV1::Withdrawal,
                    });
                }
            }
        }

        for reward in &plan.rewards {
            match reward {
                PlannedReward::ReturnedGold { amount, location } => {
                    let pile = self.create_ground_gold_pile(*amount, location.clone(), None)?;
                    rewards.push(TransactionRewardReceiptV1::GroundGoldPile {
                        gold_pile_id: pile.id.clone(),
                        amount: *amount,
                        to: crate::events::GoldLocationViewV1::Ground {
                            gold_pile_id: pile.id,
                            location: pile.location,
                        },
                    });
                }
                PlannedReward::LearningRate {
                    track_id,
                    before,
                    after,
                } => {
                    self.set_skill_learning_rate(actor_index, track_id, *after)?;
                    rewards.push(TransactionRewardReceiptV1::LearningRate {
                        track_id: track_id.clone(),
                        before: *before,
                        after: *after,
                    });
                }
                PlannedReward::Experience { amount } => {
                    let events = crate::engine::progression::award_character_experience(
                        self,
                        actor_index,
                        *amount,
                    )?;
                    let total_xp = self.world.actors[actor_index]
                        .character
                        .as_ref()
                        .map_or(0, |character| character.progression.experience);
                    delegated_events.extend(events);
                    rewards.push(TransactionRewardReceiptV1::Experience {
                        amount: *amount,
                        total_xp,
                    });
                }
                PlannedReward::Item {
                    item_instance_id,
                    item_definition_id,
                    position,
                } => {
                    let holder = self.item_holder_for_actor_index(actor_index)?;
                    let mut instances = BTreeMap::new();
                    instances.insert(
                        item_instance_id.clone(),
                        ItemInstanceState {
                            definition_id: item_definition_id.clone(),
                            quantity: 1,
                            knowledge: ItemKnowledgeState::default(),
                            binding: ItemBindingState::Unrestricted,
                            bow_readiness: None,
                        },
                    );
                    self.register_item_instances(
                        instances,
                        &[(
                            item_instance_id.clone(),
                            ItemLocation::Carried {
                                holder,
                                position: *position,
                            },
                        )],
                    )?;
                    rewards.push(TransactionRewardReceiptV1::Item {
                        item_instance_id: item_instance_id.clone(),
                        item_definition_id: item_definition_id.clone(),
                        position: *position,
                        quantity: 1,
                    });
                }
                PlannedReward::Class {
                    from_class_id,
                    from_class_display,
                    to_class_id,
                    to_class_display,
                    level,
                } => {
                    self.apply_transaction_class_reward(
                        actor_index,
                        from_class_id,
                        to_class_id,
                        to_class_display,
                        *level,
                    )?;
                    rewards.push(TransactionRewardReceiptV1::Class {
                        from_class_id: from_class_id.clone(),
                        from_class_display: from_class_display.clone(),
                        to_class_id: to_class_id.clone(),
                        to_class_display: to_class_display.clone(),
                    });
                }
                PlannedReward::Spell {
                    spell_id,
                    lane,
                    learned_at_level,
                } => {
                    self.apply_transaction_spell_reward(
                        actor_index,
                        spell_id,
                        lane,
                        *learned_at_level,
                    )?;
                    rewards.push(TransactionRewardReceiptV1::Spell {
                        spell_id: spell_id.clone(),
                        learned_at_level: *learned_at_level,
                    });
                }
                PlannedReward::CarriedGold { amount } => {
                    let before =
                        self.carried_gold_at(actor_index, crate::model::CarriedGoldPosition::Sack)?;
                    let after = self.change_carried_gold_at(
                        actor_index,
                        crate::model::CarriedGoldPosition::Sack,
                        *amount,
                    )?;
                    delegated_events.push(Event::GoldChanged {
                        actor_id: plan.actor_id.clone(),
                        actor: plan.actor_name.clone(),
                        amount: *amount,
                        new_total: after,
                    });
                    rewards.push(TransactionRewardReceiptV1::CarriedGold {
                        amount: *amount,
                        position: crate::model::CarriedGoldPosition::Sack,
                        before,
                        after,
                    });
                }
                PlannedReward::MerchantItem {
                    item_instance_id,
                    expected,
                    destination,
                    listing_price_gold,
                } => {
                    let instance = self.item_instance(item_instance_id)?.clone();
                    let from = self.location_view(expected)?;
                    let to = self.location_view(destination)?;
                    self.relocate_items_with_events(
                        actor_index,
                        vec![crate::engine::inventory::ItemRelocation {
                            item_instance_id: item_instance_id.clone(),
                            expected: expected.clone(),
                            destination: destination.clone(),
                            loot_claim: None,
                            merchant_listing: None,
                        }],
                        crate::events::ItemRelocationReason::MerchantPurchase,
                        &mut delegated_events,
                    )?;
                    rewards.push(TransactionRewardReceiptV1::MerchantItem {
                        item_instance_id: item_instance_id.clone(),
                        item_definition_id: instance.definition_id,
                        quantity: instance.quantity,
                        from,
                        to,
                        listing_price_gold: *listing_price_gold,
                    });
                }
                PlannedReward::ItemAppraisal {
                    item_instance_id,
                    source,
                    unit_value_gold,
                    total_value_gold,
                } => {
                    let definition_id = self.item_instance(item_instance_id)?.definition_id.clone();
                    self.apply_item_appraisal(
                        actor_index,
                        item_instance_id,
                        source.clone(),
                        *unit_value_gold,
                        *total_value_gold,
                        &mut delegated_events,
                    )?;
                    rewards.push(TransactionRewardReceiptV1::ItemAppraised {
                        item_instance_id: item_instance_id.clone(),
                        item_definition_id: definition_id,
                        unit_value_gold: *unit_value_gold,
                        total_value_gold: *total_value_gold,
                    });
                }
                PlannedReward::ItemIdentification {
                    item_instance_id,
                    source,
                    location,
                } => {
                    let definition_id = self.item_instance(item_instance_id)?.definition_id.clone();
                    self.apply_item_identification(
                        actor_index,
                        item_instance_id,
                        source.clone(),
                        location.clone(),
                        &mut delegated_events,
                    )?;
                    rewards.push(TransactionRewardReceiptV1::ItemIdentified {
                        item_instance_id: item_instance_id.clone(),
                        item_definition_id: definition_id,
                    });
                }
                PlannedReward::ItemEnchantment {
                    item_instance_id,
                    source,
                    enchantment_instance_id,
                    combat_add_rating_bonus,
                    tags,
                    remaining_rounds,
                } => {
                    let definition_id = self.item_instance(item_instance_id)?.definition_id.clone();
                    self.apply_weapon_enchantment(
                        actor_index,
                        item_instance_id,
                        source.clone(),
                        enchantment_instance_id.clone(),
                        *combat_add_rating_bonus,
                        tags.clone(),
                        *remaining_rounds,
                        &mut delegated_events,
                    )?;
                    rewards.push(TransactionRewardReceiptV1::ItemEnchanted {
                        item_instance_id: item_instance_id.clone(),
                        item_definition_id: definition_id,
                        enchantment_instance_id: enchantment_instance_id.clone(),
                        combat_add_rating_bonus: *combat_add_rating_bonus,
                        tags: tags.clone(),
                        remaining_rounds: *remaining_rounds,
                    });
                }
                PlannedReward::Restoration(restoration) => {
                    let (reward, mut events) =
                        self.apply_restoration_reward(restoration.clone())?;
                    delegated_events.append(&mut events);
                    rewards.push(reward);
                }
                PlannedReward::NpcInteraction(interaction) => {
                    let (reward, mut events) = self.apply_npc_interaction_reward(interaction)?;
                    delegated_events.append(&mut events);
                    rewards.push(reward);
                }
                PlannedReward::QuestStage(quest) => {
                    let (reward, event) = self.apply_quest_transition(quest)?;
                    delegated_events.push(event);
                    rewards.push(reward);
                }
                PlannedReward::BankBalance {
                    bank_id,
                    character_id,
                    amount,
                } => {
                    let bank = self
                        .world
                        .banks
                        .get_mut(bank_id)
                        .ok_or_else(|| StepError::new("captured bank state is missing"))?;
                    let before = bank.balance(character_id);
                    let after = before
                        .checked_add(*amount)
                        .ok_or_else(|| StepError::new("bank balance overflow"))?;
                    bank.balances.insert(character_id.clone(), after);
                    rewards.push(TransactionRewardReceiptV1::BankBalance {
                        bank_id: bank_id.as_str().to_string(),
                        character_id: character_id.clone(),
                        amount: *amount,
                        before,
                        after,
                    });
                    delegated_events.push(Event::BankBalanceChanged {
                        actor_id: plan.actor_id.clone(),
                        actor: plan.actor_name.clone(),
                        bank_id: bank_id.as_str().to_string(),
                        character_id: character_id.clone(),
                        amount: *amount,
                        before,
                        after,
                        reason: crate::events::BankBalanceChangeReasonV1::Deposit,
                    });
                }
                PlannedReward::GroundGoldPile {
                    bank_id,
                    character_id,
                    amount,
                    location,
                } => {
                    let pile = self.create_ground_gold_pile(*amount, location.clone(), None)?;
                    let to = crate::events::GoldLocationViewV1::Ground {
                        gold_pile_id: pile.id.clone(),
                        location: pile.location.clone(),
                    };
                    delegated_events.push(Event::GoldRelocated {
                        actor_id: plan.actor_id.clone(),
                        actor: plan.actor_name.clone(),
                        amount: *amount,
                        from: crate::events::GoldLocationViewV1::Bank {
                            bank_id: bank_id.as_str().to_string(),
                            character_id: character_id.clone(),
                        },
                        to: to.clone(),
                        reason: crate::events::GoldRelocationReason::BankWithdrawal,
                        loot_claim: None,
                    });
                    rewards.push(TransactionRewardReceiptV1::GroundGoldPile {
                        gold_pile_id: pile.id,
                        amount: *amount,
                        to,
                    });
                }
            }
        }

        Ok(TransactionCommitReceipt {
            source: plan.source,
            costs,
            rewards,
            delegated_events,
        })
    }
}
