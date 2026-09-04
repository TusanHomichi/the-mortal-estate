use super::*;

pub(super) fn transaction_requirement(
    value: &rules::TransactionRequirementViewV1,
) -> Result<wire::TransactionRequirement, wire::ProtocolError> {
    Ok(match value {
        rules::TransactionRequirementViewV1::CurrentClass { class_id } => {
            wire::TransactionRequirement::CurrentClass {
                class_id: label(class_id)?,
            }
        }
        rules::TransactionRequirementViewV1::MinimumLevel { level } => {
            wire::TransactionRequirement::MinimumLevel { level: *level }
        }
        rules::TransactionRequirementViewV1::ExactKarma { karma_points } => {
            wire::TransactionRequirement::ExactKarma {
                karma_points: *karma_points,
            }
        }
        rules::TransactionRequirementViewV1::ExactAlignment { alignment } => {
            wire::TransactionRequirement::ExactAlignment {
                alignment: character_alignment(*alignment),
            }
        }
        rules::TransactionRequirementViewV1::MinimumSkillLevel { track_id, level } => {
            wire::TransactionRequirement::MinimumSkillLevel {
                track_id: label(track_id)?,
                level: *level,
            }
        }
        rules::TransactionRequirementViewV1::MinimumCarriedGold { amount } => {
            wire::TransactionRequirement::MinimumCarriedGold {
                amount: wire::DecimalI64::new(*amount),
            }
        }
        rules::TransactionRequirementViewV1::CarriedItem {
            item_definition_id,
            quantity,
        } => wire::TransactionRequirement::CarriedItem {
            item_definition_id: label(item_definition_id)?,
            quantity: *quantity,
        },
        rules::TransactionRequirementViewV1::CarriedPositionEmpty { position } => {
            wire::TransactionRequirement::CarriedPositionEmpty {
                position: carried_position(*position),
            }
        }
        rules::TransactionRequirementViewV1::SpellUnknown { spell_id } => {
            wire::TransactionRequirement::SpellUnknown {
                spell_id: label(spell_id)?,
            }
        }
        rules::TransactionRequirementViewV1::QuestUnstarted { quest_id } => {
            wire::TransactionRequirement::QuestUnstarted {
                quest_id: label(quest_id)?,
            }
        }
        rules::TransactionRequirementViewV1::QuestAtStage { quest_id, stage_id } => {
            wire::TransactionRequirement::QuestAtStage {
                quest_id: label(quest_id)?,
                stage_id: label(stage_id)?,
            }
        }
        rules::TransactionRequirementViewV1::NpcAccompanying { npc_actor_id } => {
            wire::TransactionRequirement::NpcAccompanying {
                npc_actor_id: actor_id(npc_actor_id)?,
            }
        }
    })
}

pub(super) fn transaction_cost(value: &rules::TransactionCostViewV1) -> wire::TransactionCost {
    match value {
        rules::TransactionCostViewV1::CarriedGold { amount } => {
            wire::TransactionCost::CarriedGold {
                amount: wire::DecimalI64::new(*amount),
            }
        }
        rules::TransactionCostViewV1::SelectedCarriedItem { quantity } => {
            wire::TransactionCost::SelectedCarriedItem {
                quantity: *quantity,
            }
        }
    }
}

pub(super) fn transaction_reward(
    value: &rules::TransactionRewardViewV1,
) -> Result<wire::TransactionReward, wire::ProtocolError> {
    Ok(match value {
        rules::TransactionRewardViewV1::Experience { amount } => {
            wire::TransactionReward::Experience { amount: *amount }
        }
        rules::TransactionRewardViewV1::Item {
            item_instance_id,
            item_definition_id,
            position,
        } => wire::TransactionReward::Item {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            item_definition_id: label(item_definition_id)?,
            position: carried_position(*position),
        },
        rules::TransactionRewardViewV1::Class {
            to_class_id,
            to_class_display,
        } => wire::TransactionReward::Class {
            to_class_id: label(to_class_id)?,
            to_class_display: label(to_class_display)?,
        },
        rules::TransactionRewardViewV1::Spell { spell_id } => wire::TransactionReward::Spell {
            spell_id: label(spell_id)?,
        },
        rules::TransactionRewardViewV1::QuestStage { quest_id, stage_id } => {
            wire::TransactionReward::QuestStage {
                quest_id: label(quest_id)?,
                stage_id: label(stage_id)?,
            }
        }
    })
}

pub(super) fn action_options(
    values: &[rules::ActionOptionV1],
) -> Result<Vec<wire::ObserverActionOption>, wire::ProtocolError> {
    values.iter().map(observer_action_option).collect()
}

pub(super) fn service_transaction(
    value: &rules::ServiceTransactionViewV1,
) -> Result<wire::ServiceTransaction, wire::ProtocolError> {
    Ok(wire::ServiceTransaction {
        transaction_id: label(&value.transaction_id)?,
        label: label(&value.label)?,
        requirements: value
            .requirements
            .iter()
            .map(transaction_requirement)
            .collect::<Result<_, _>>()?,
        costs: value.costs.iter().map(transaction_cost).collect(),
        rewards: value
            .rewards
            .iter()
            .map(transaction_reward)
            .collect::<Result<_, _>>()?,
        actions: action_options(&value.actions)?,
    })
}

pub(super) fn restoration_outcome(
    value: &rules::RestorationOutcomeViewV1,
) -> wire::RestorationOutcome {
    match value {
        rules::RestorationOutcomeViewV1::RestoreResource { resource } => {
            wire::RestorationOutcome::RestoreResource {
                resource: match resource {
                    rules::ResourceKind::Hp => wire::ResourceKind::Hp,
                    rules::ResourceKind::Mp => wire::ResourceKind::Mp,
                    rules::ResourceKind::Stamina => wire::ResourceKind::Stamina,
                },
            }
        }
        rules::RestorationOutcomeViewV1::CureStatus { status } => {
            wire::RestorationOutcome::CureStatus {
                status: match status {
                    rules::RestorationStatusKind::Blindness => {
                        wire::RestorationStatusKind::Blindness
                    }
                    rules::RestorationStatusKind::Poison => wire::RestorationStatusKind::Poison,
                },
            }
        }
        rules::RestorationOutcomeViewV1::PriestResurrection => {
            wire::RestorationOutcome::PriestResurrection
        }
    }
}

pub(super) fn service_capability(
    value: &rules::ServiceCapabilityViewV1,
) -> Result<wire::ServiceCapability, wire::ProtocolError> {
    Ok(match value {
        rules::ServiceCapabilityViewV1::SkillTraining {
            capability_id,
            offered_track_ids,
            selected_track_id,
            actions,
        } => wire::ServiceCapability::SkillTraining {
            capability_id: label(capability_id)?,
            offered_track_ids: offered_track_ids
                .iter()
                .map(|value| label(value))
                .collect::<Result<_, _>>()?,
            selected_track_id: selected_track_id.as_deref().map(label).transpose()?,
            actions: action_options(actions)?,
        },
        rules::ServiceCapabilityViewV1::SkillCritique {
            capability_id,
            actions,
        } => wire::ServiceCapability::SkillCritique {
            capability_id: label(capability_id)?,
            actions: action_options(actions)?,
        },
        rules::ServiceCapabilityViewV1::SpellTeaching {
            capability_id,
            spell_ids,
            actions,
        } => wire::ServiceCapability::SpellTeaching {
            capability_id: label(capability_id)?,
            spell_ids: spell_ids
                .iter()
                .map(|value| label(value))
                .collect::<Result<_, _>>()?,
            actions: action_options(actions)?,
        },
        rules::ServiceCapabilityViewV1::ClassPromotion {
            capability_id,
            target_class_id,
            actions,
        } => wire::ServiceCapability::ClassPromotion {
            capability_id: label(capability_id)?,
            target_class_id: label(target_class_id)?,
            actions: action_options(actions)?,
        },
        rules::ServiceCapabilityViewV1::ServiceTransaction {
            capability_id,
            transactions,
        } => wire::ServiceCapability::ServiceTransaction {
            capability_id: label(capability_id)?,
            transactions: transactions
                .iter()
                .map(service_transaction)
                .collect::<Result<_, _>>()?,
        },
        rules::ServiceCapabilityViewV1::Merchant {
            capability_id,
            listings,
            buy_all,
            sales,
        } => wire::ServiceCapability::Merchant {
            capability_id: label(capability_id)?,
            listings: listings
                .iter()
                .map(|listing| {
                    Ok(wire::MerchantListing {
                        item: owned_item(&listing.item)?,
                        origin: match listing.origin {
                            rules::MerchantListingOriginViewV1::AuthoredStock => {
                                wire::MerchantListingOrigin::AuthoredStock
                            }
                            rules::MerchantListingOriginViewV1::PawnPool => {
                                wire::MerchantListingOrigin::PawnPool
                            }
                        },
                        price_gold: wire::DecimalI64::new(listing.price_gold),
                        purchase: observer_action_option(&listing.purchase)?,
                    })
                })
                .collect::<Result<_, wire::ProtocolError>>()?,
            buy_all: observer_action_option(buy_all)?,
            sales: action_options(sales)?,
        },
        rules::ServiceCapabilityViewV1::ItemService {
            capability_id,
            operations,
        } => wire::ServiceCapability::ItemService {
            capability_id: label(capability_id)?,
            operations: operations
                .iter()
                .map(|operation| {
                    Ok(wire::ItemServiceOperation {
                        operation: item_service_operation(operation.operation),
                        actions: action_options(&operation.actions)?,
                    })
                })
                .collect::<Result<_, wire::ProtocolError>>()?,
        },
        rules::ServiceCapabilityViewV1::Restoration {
            capability_id,
            operations,
        } => wire::ServiceCapability::Restoration {
            capability_id: label(capability_id)?,
            operations: operations
                .iter()
                .map(|operation| {
                    Ok(wire::RestorationOperation {
                        operation_id: label(&operation.operation_id)?,
                        label: label(&operation.label)?,
                        requirements: operation
                            .requirements
                            .iter()
                            .map(transaction_requirement)
                            .collect::<Result<_, _>>()?,
                        costs: operation.costs.iter().map(transaction_cost).collect(),
                        outcome: restoration_outcome(&operation.outcome),
                        actions: action_options(&operation.actions)?,
                    })
                })
                .collect::<Result<_, wire::ProtocolError>>()?,
        },
        rules::ServiceCapabilityViewV1::Bank {
            capability_id,
            bank_id,
            balance_gold,
            transaction_cap_gold,
            deposit_actions,
            withdrawal_actions,
        } => wire::ServiceCapability::Bank {
            capability_id: label(capability_id)?,
            bank_id: label(bank_id)?,
            balance_gold: wire::DecimalI64::new(*balance_gold),
            transaction_cap_gold: wire::DecimalI64::new(*transaction_cap_gold),
            deposit_actions: action_options(deposit_actions)?,
            withdrawal_actions: action_options(withdrawal_actions)?,
        },
        rules::ServiceCapabilityViewV1::Locker {
            capability_id,
            vault_id,
            capacity,
            item_count,
            items,
            deposit_actions,
            withdrawal_actions,
        } => wire::ServiceCapability::Locker {
            capability_id: label(capability_id)?,
            vault_id: label(vault_id)?,
            capacity: *capacity,
            item_count: *item_count,
            items: items.iter().map(owned_item).collect::<Result<_, _>>()?,
            deposit_actions: action_options(deposit_actions)?,
            withdrawal_actions: action_options(withdrawal_actions)?,
        },
    })
}

pub(super) fn service(value: &rules::ServiceViewV1) -> Result<wire::Service, wire::ProtocolError> {
    Ok(wire::Service {
        service_id: label(&value.service_id)?,
        name: label(&value.name)?,
        position: position(&value.position)?,
        capabilities: value
            .capabilities
            .iter()
            .map(service_capability)
            .collect::<Result<_, _>>()?,
    })
}

pub(super) fn npc(value: &rules::NpcViewV1) -> Result<wire::Npc, wire::ProtocolError> {
    Ok(wire::Npc {
        actor_id: actor_id(&value.actor_id)?,
        name: label(&value.name)?,
        following_character_id: value
            .following_character_id
            .as_ref()
            .map(character_id)
            .transpose()?,
        interactions: value
            .interactions
            .iter()
            .map(|interaction| {
                Ok(wire::NpcInteraction {
                    interaction_id: label(&interaction.interaction_id)?,
                    label: label(&interaction.label)?,
                    requirements: interaction
                        .requirements
                        .iter()
                        .map(transaction_requirement)
                        .collect::<Result<_, _>>()?,
                    costs: interaction.costs.iter().map(transaction_cost).collect(),
                    rewards: interaction
                        .rewards
                        .iter()
                        .map(transaction_reward)
                        .collect::<Result<_, _>>()?,
                    outcome: match &interaction.outcome {
                        rules::NpcInteractionOutcome::Speak => wire::NpcInteractionOutcome::Speak,
                        rules::NpcInteractionOutcome::BeginFollow => {
                            wire::NpcInteractionOutcome::BeginFollow
                        }
                        rules::NpcInteractionOutcome::EndFollow => {
                            wire::NpcInteractionOutcome::EndFollow
                        }
                        rules::NpcInteractionOutcome::CompleteEscort { npc_actor_id } => {
                            wire::NpcInteractionOutcome::CompleteEscort {
                                npc_actor_id: actor_id(npc_actor_id)?,
                            }
                        }
                        rules::NpcInteractionOutcome::Climb { direction: value } => {
                            wire::NpcInteractionOutcome::Climb {
                                direction: vertical(*value),
                            }
                        }
                    },
                    actions: action_options(&interaction.actions)?,
                })
            })
            .collect::<Result<_, wire::ProtocolError>>()?,
    })
}

pub(super) fn quest(
    value: &rules::QuestStateViewV1,
) -> Result<wire::QuestState, wire::ProtocolError> {
    Ok(wire::QuestState {
        quest_id: label(&value.quest_id)?,
        quest_title: label(&value.quest_title)?,
        stage_id: label(&value.stage_id)?,
        stage_label: label(&value.stage_label)?,
        terminal: value.terminal,
    })
}

pub(super) fn loot_claim(
    value: &rules::LootClaimViewV1,
) -> Result<wire::LootClaim, wire::ProtocolError> {
    Ok(wire::LootClaim {
        owner: match &value.owner {
            rules::LootOwnerId::Character(value) => {
                wire::LootOwner::Character(character_id(value)?)
            }
            rules::LootOwnerId::TransientActor(value) => {
                wire::LootOwner::TransientActor(actor_id(value)?)
            }
        },
        basis: match value.basis {
            rules::LootClaimBasis::KillingBlow => wire::LootClaimBasis::KillingBlow,
            rules::LootClaimBasis::CharacterDeathPile => wire::LootClaimBasis::CharacterDeathPile,
        },
    })
}

pub(crate) fn rules_direction(value: &wire::Direction) -> rules::Direction {
    match value {
        wire::Direction::North => rules::Direction::North,
        wire::Direction::Northeast => rules::Direction::Northeast,
        wire::Direction::East => rules::Direction::East,
        wire::Direction::Southeast => rules::Direction::Southeast,
        wire::Direction::South => rules::Direction::South,
        wire::Direction::Southwest => rules::Direction::Southwest,
        wire::Direction::West => rules::Direction::West,
        wire::Direction::Northwest => rules::Direction::Northwest,
    }
}

pub(super) fn rules_gold_position(value: wire::CarriedGoldPosition) -> rules::CarriedGoldPosition {
    match value {
        wire::CarriedGoldPosition::LeftHand => rules::CarriedGoldPosition::LeftHand,
        wire::CarriedGoldPosition::RightHand => rules::CarriedGoldPosition::RightHand,
        wire::CarriedGoldPosition::Sack => rules::CarriedGoldPosition::Sack,
    }
}

pub(super) fn gold_position(value: rules::CarriedGoldPosition) -> wire::CarriedGoldPosition {
    match value {
        rules::CarriedGoldPosition::LeftHand => wire::CarriedGoldPosition::LeftHand,
        rules::CarriedGoldPosition::RightHand => wire::CarriedGoldPosition::RightHand,
        rules::CarriedGoldPosition::Sack => wire::CarriedGoldPosition::Sack,
    }
}

pub(super) fn item_service_operation(
    value: rules::ItemServiceOperationKind,
) -> wire::ItemServiceOperationKind {
    match value {
        rules::ItemServiceOperationKind::Appraise => wire::ItemServiceOperationKind::Appraise,
        rules::ItemServiceOperationKind::Identify => wire::ItemServiceOperationKind::Identify,
        rules::ItemServiceOperationKind::EnchantWeapon => {
            wire::ItemServiceOperationKind::EnchantWeapon
        }
    }
}

pub(super) fn rules_item_service_operation(
    value: wire::ItemServiceOperationKind,
) -> rules::ItemServiceOperationKind {
    match value {
        wire::ItemServiceOperationKind::Appraise => rules::ItemServiceOperationKind::Appraise,
        wire::ItemServiceOperationKind::Identify => rules::ItemServiceOperationKind::Identify,
        wire::ItemServiceOperationKind::EnchantWeapon => {
            rules::ItemServiceOperationKind::EnchantWeapon
        }
    }
}
