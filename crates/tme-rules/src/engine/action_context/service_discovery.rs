use super::*;

impl Engine {
    pub(super) fn service_views_for_actor(
        &self,
        actor_index: usize,
    ) -> Result<Vec<ServiceViewV1>, StepError> {
        let actor_id = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("unknown actor"))?
            .id
            .clone();

        self.services_at_actor(actor_index)
            .into_iter()
            .map(|service| {
                let capabilities = service
                    .capabilities()
                    .iter()
                    .map(|capability| {
                        self.service_capability_view(actor_index, &actor_id, service, capability)
                    })
                    .collect::<Result<Vec<_>, StepError>>()?;
                Ok(ServiceViewV1 {
                    service_id: service.id().to_string(),
                    name: service.name().to_string(),
                    position: service.position().clone(),
                    capabilities,
                })
            })
            .collect()
    }

    fn service_capability_view(
        &self,
        actor_index: usize,
        actor_id: &crate::model::ActorId,
        service: ResolvedService<'_>,
        capability: &ServiceCapability,
    ) -> Result<ServiceCapabilityViewV1, StepError> {
        match capability {
            ServiceCapability::SkillTraining(capability) => {
                self.skill_training_capability_view(actor_index, actor_id, service, capability)
            }
            ServiceCapability::SkillCritique(capability) => {
                self.skill_critique_capability_view(actor_index, actor_id, service, capability)
            }
            ServiceCapability::SpellTeaching(capability) => {
                self.spell_teaching_capability_view(actor_index, actor_id, service, capability)
            }
            ServiceCapability::ClassPromotion(capability) => {
                let target_class_id = capability
                    .transaction
                    .rewards
                    .iter()
                    .find_map(|reward| match reward {
                        crate::model::TransactionReward::Class { to_class_id, .. } => {
                            Some(to_class_id.clone())
                        }
                        _ => None,
                    })
                    .ok_or_else(|| StepError::new("promotion has no class reward"))?;
                let command = PlayerCommandV1 {
                    contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                    actor_id: actor_id.clone(),
                    intent: PlayerIntentPayloadV1::PromoteClass {
                        target_class_id: target_class_id.clone(),
                    },
                };
                let status = self.validate_actor_command(&command)?;
                Ok(ServiceCapabilityViewV1::ClassPromotion {
                    capability_id: capability.id.clone(),
                    target_class_id: target_class_id.clone(),
                    actions: vec![ActionOptionV1 {
                        id: format!("promote_{target_class_id}"),
                        label: format!("Promote to {target_class_id}"),
                        enabled: status.accepted,
                        blocked_reason: status.blocked_reason,
                        command: Some(command),
                    }],
                })
            }
            ServiceCapability::ServiceTransaction(capability) => {
                let mut transactions = Vec::new();
                for transaction in &capability.transactions {
                    let carried_requirement =
                        transaction
                            .requirements
                            .iter()
                            .find_map(|requirement| match requirement {
                                crate::model::TransactionRequirement::CarriedItem {
                                    item_definition_id,
                                    quantity,
                                } => Some((item_definition_id.as_str(), *quantity)),
                                _ => None,
                            });
                    let mut selections = match carried_requirement {
                        Some((definition_id, quantity)) => self.world.actors[actor_index]
                            .carried
                            .items
                            .iter()
                            .filter_map(|(position, instance_id)| {
                                self.world
                                    .item_instances
                                    .get(instance_id)
                                    .filter(|instance| {
                                        instance.definition_id == definition_id
                                            && instance.quantity >= quantity
                                    })
                                    .map(|_| (*position, instance_id.clone()))
                            })
                            .collect::<Vec<_>>(),
                        None => Vec::new(),
                    };
                    selections.sort();
                    let selected_ids = if carried_requirement.is_some() {
                        if selections.is_empty() {
                            vec![None]
                        } else {
                            selections
                                .into_iter()
                                .map(|(_, instance_id)| Some(instance_id))
                                .collect()
                        }
                    } else {
                        vec![None]
                    };
                    let mut actions = Vec::new();
                    for selected_id in selected_ids {
                        let command = PlayerCommandV1 {
                            contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                            actor_id: actor_id.clone(),
                            intent: PlayerIntentPayloadV1::CommitServiceTransaction {
                                service_id: service.id().to_string(),
                                capability_id: capability.id.clone(),
                                transaction_id: transaction.id.clone(),
                                item_instance_id: selected_id.clone(),
                            },
                        };
                        let status = self.validate_actor_command(&command)?;
                        actions.push(ActionOptionV1 {
                            id: format!(
                                "commit_service_transaction_{}_{}_{}",
                                service.id(),
                                transaction.id,
                                selected_id.as_deref().unwrap_or("none")
                            ),
                            label: transaction.label.clone(),
                            enabled: status.accepted,
                            blocked_reason: status.blocked_reason,
                            command: Some(command),
                        });
                    }
                    transactions.push(ServiceTransactionViewV1 {
                        transaction_id: transaction.id.clone(),
                        label: transaction.label.clone(),
                        requirements: transaction
                            .requirements
                            .iter()
                            .map(TransactionRequirementViewV1::from)
                            .collect(),
                        costs: transaction
                            .costs
                            .iter()
                            .map(TransactionCostViewV1::from)
                            .collect(),
                        rewards: transaction
                            .rewards
                            .iter()
                            .map(TransactionRewardViewV1::from)
                            .collect(),
                        actions,
                    });
                }
                Ok(ServiceCapabilityViewV1::ServiceTransaction {
                    capability_id: capability.id.clone(),
                    transactions,
                })
            }
            ServiceCapability::Merchant(capability) => {
                let inventory_id =
                    crate::model::MerchantInventoryId::new(service.id(), &capability.id);
                let inventory = self
                    .world
                    .merchant_inventories
                    .get(&inventory_id)
                    .ok_or_else(|| StepError::new("merchant inventory is missing"))?;
                let mut listings = Vec::with_capacity(inventory.listings.len());
                for listing in &inventory.listings {
                    let item = self.item_instance_view(&listing.item_instance_id)?;
                    let command = PlayerCommandV1 {
                        contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                        actor_id: actor_id.clone(),
                        intent: PlayerIntentPayloadV1::BuyFromMerchant {
                            service_id: service.id().to_string(),
                            capability_id: capability.id.clone(),
                            item_instance_ids: vec![listing.item_instance_id.clone()],
                        },
                    };
                    let status = self.validate_actor_command(&command)?;
                    listings.push(MerchantListingViewV1 {
                        item,
                        origin: match listing.origin {
                            crate::model::MerchantListingOrigin::AuthoredStock => {
                                MerchantListingOriginViewV1::AuthoredStock
                            }
                            crate::model::MerchantListingOrigin::PawnPool => {
                                MerchantListingOriginViewV1::PawnPool
                            }
                        },
                        price_gold: listing.price_gold,
                        purchase: ActionOptionV1 {
                            id: format!("buy_{}", listing.item_instance_id),
                            label: format!("Buy {}", listing.item_instance_id),
                            enabled: status.accepted,
                            blocked_reason: status.blocked_reason,
                            command: Some(command),
                        },
                    });
                }

                let all_item_instance_ids = inventory
                    .listings
                    .iter()
                    .map(|listing| listing.item_instance_id.clone())
                    .collect::<Vec<_>>();
                let buy_all_command = PlayerCommandV1 {
                    contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                    actor_id: actor_id.clone(),
                    intent: PlayerIntentPayloadV1::BuyFromMerchant {
                        service_id: service.id().to_string(),
                        capability_id: capability.id.clone(),
                        item_instance_ids: all_item_instance_ids,
                    },
                };
                let buy_all_status = self.validate_actor_command(&buy_all_command)?;
                let buy_all = ActionOptionV1 {
                    id: format!("buy_all_{}_{}", service.id(), capability.id),
                    label: "Buy all".to_string(),
                    enabled: buy_all_status.accepted,
                    blocked_reason: buy_all_status.blocked_reason,
                    command: Some(buy_all_command),
                };

                let mut sales = Vec::new();
                for item_instance_id in self.carried_item_ids(actor_index)? {
                    if self
                        .merchant_sale_plan(
                            actor_index,
                            service.id(),
                            &capability.id,
                            &item_instance_id,
                        )
                        .is_err()
                    {
                        continue;
                    }
                    let item = self.item_instance_view(&item_instance_id)?;
                    let command = PlayerCommandV1 {
                        contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                        actor_id: actor_id.clone(),
                        intent: PlayerIntentPayloadV1::SellToMerchant {
                            service_id: service.id().to_string(),
                            capability_id: capability.id.clone(),
                            item_instance_id: item_instance_id.clone(),
                        },
                    };
                    let status = self.validate_actor_command(&command)?;
                    sales.push(ActionOptionV1 {
                        id: format!("sell_{item_instance_id}"),
                        label: format!("Sell {}", item.name),
                        enabled: status.accepted,
                        blocked_reason: status.blocked_reason,
                        command: Some(command),
                    });
                }

                Ok(ServiceCapabilityViewV1::Merchant {
                    capability_id: capability.id.clone(),
                    listings,
                    buy_all,
                    sales,
                })
            }
            ServiceCapability::ItemService(capability) => {
                let carried_item_ids = self.carried_item_ids(actor_index)?;
                let mut operations = Vec::with_capacity(capability.operations.len());
                for operation in &capability.operations {
                    let operation_kind = operation.kind();
                    let mut actions = Vec::with_capacity(carried_item_ids.len());
                    for item_instance_id in &carried_item_ids {
                        let definition = self.item_definition(item_instance_id)?;
                        let eligible = match operation {
                            crate::model::ItemServiceOperation::Appraise => {
                                definition.economy.unit_value_gold.is_some()
                            }
                            crate::model::ItemServiceOperation::Identify { .. } => true,
                            crate::model::ItemServiceOperation::EnchantWeapon { .. } => {
                                definition.weapon.is_some()
                            }
                        };
                        if !eligible {
                            continue;
                        }
                        let item = self.item_instance_view(item_instance_id)?;
                        let command = PlayerCommandV1 {
                            contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                            actor_id: actor_id.clone(),
                            intent: PlayerIntentPayloadV1::UseItemService {
                                service_id: service.id().to_string(),
                                capability_id: capability.id.clone(),
                                operation: operation_kind,
                                item_instance_id: item_instance_id.clone(),
                            },
                        };
                        let status = self.validate_actor_command(&command)?;
                        actions.push(ActionOptionV1 {
                            id: format!("{}_{}", operation_kind.label(), item_instance_id),
                            label: format!("{} {}", operation_kind.label(), item.name),
                            enabled: status.accepted,
                            blocked_reason: status.blocked_reason,
                            command: Some(command),
                        });
                    }
                    operations.push(ItemServiceOperationViewV1 {
                        operation: operation_kind,
                        actions,
                    });
                }
                Ok(ServiceCapabilityViewV1::ItemService {
                    capability_id: capability.id.clone(),
                    operations,
                })
            }
            ServiceCapability::Restoration(capability) => {
                let mut operations = Vec::with_capacity(capability.operations.len());
                for operation in &capability.operations {
                    let carried_requirement =
                        operation
                            .transaction
                            .requirements
                            .iter()
                            .find_map(|requirement| match requirement {
                                crate::model::TransactionRequirement::CarriedItem {
                                    item_definition_id,
                                    quantity,
                                } => Some((item_definition_id.as_str(), *quantity)),
                                _ => None,
                            });
                    let mut item_selections = match carried_requirement {
                        Some((definition_id, quantity)) => self.world.actors[actor_index]
                            .carried
                            .items
                            .iter()
                            .filter_map(|(position, instance_id)| {
                                self.world
                                    .item_instances
                                    .get(instance_id)
                                    .filter(|instance| {
                                        instance.definition_id == definition_id
                                            && instance.quantity >= quantity
                                    })
                                    .map(|_| (*position, instance_id.clone()))
                            })
                            .collect::<Vec<_>>(),
                        None => Vec::new(),
                    };
                    item_selections.sort();
                    let item_ids = if carried_requirement.is_some() {
                        if item_selections.is_empty() {
                            vec![None]
                        } else {
                            item_selections
                                .into_iter()
                                .map(|(_, instance_id)| Some(instance_id))
                                .collect()
                        }
                    } else {
                        vec![None]
                    };

                    let corpse_ids = if matches!(
                        &operation.outcome,
                        crate::model::RestorationOutcome::PriestResurrection
                    ) {
                        let mut local = self
                            .world
                            .corpses
                            .values()
                            .filter(|corpse| {
                                corpse.location.level == service.position().level
                                    && corpse.location.position == service.position().position
                            })
                            .map(|corpse| (corpse.sequence, corpse.id.clone()))
                            .collect::<Vec<_>>();
                        local.sort();
                        if local.is_empty() {
                            vec![None]
                        } else {
                            local
                                .into_iter()
                                .map(|(_, corpse_id)| Some(corpse_id))
                                .collect()
                        }
                    } else {
                        vec![None]
                    };

                    let mut actions = Vec::new();
                    for item_instance_id in &item_ids {
                        for corpse_id in &corpse_ids {
                            let command = PlayerCommandV1 {
                                contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                                actor_id: actor_id.clone(),
                                intent: PlayerIntentPayloadV1::UseRestorationService {
                                    service_id: service.id().to_string(),
                                    capability_id: capability.id.clone(),
                                    operation_id: operation.transaction.id.clone(),
                                    item_instance_id: item_instance_id.clone(),
                                    corpse_id: corpse_id.clone(),
                                },
                            };
                            let status = self.validate_actor_command(&command)?;
                            actions.push(ActionOptionV1 {
                                id: format!(
                                    "use_restoration_service_{}_{}_{}_{}",
                                    service.id(),
                                    operation.transaction.id,
                                    item_instance_id.as_deref().unwrap_or("none"),
                                    corpse_id
                                        .as_ref()
                                        .map_or("none", |corpse_id| corpse_id.as_str())
                                ),
                                label: match corpse_id {
                                    Some(corpse_id) => {
                                        format!("{} ({corpse_id})", operation.transaction.label)
                                    }
                                    None => operation.transaction.label.clone(),
                                },
                                enabled: status.accepted,
                                blocked_reason: status.blocked_reason,
                                command: Some(command),
                            });
                        }
                    }
                    operations.push(RestorationOperationViewV1 {
                        operation_id: operation.transaction.id.clone(),
                        label: operation.transaction.label.clone(),
                        requirements: operation
                            .transaction
                            .requirements
                            .iter()
                            .map(TransactionRequirementViewV1::from)
                            .collect(),
                        costs: operation
                            .transaction
                            .costs
                            .iter()
                            .map(TransactionCostViewV1::from)
                            .collect(),
                        outcome: RestorationOutcomeViewV1::from(&operation.outcome),
                        actions,
                    });
                }
                Ok(ServiceCapabilityViewV1::Restoration {
                    capability_id: capability.id.clone(),
                    operations,
                })
            }
            ServiceCapability::Bank(capability) => {
                let actor = &self.world.actors[actor_index];
                let character_id = actor
                    .character_id
                    .clone()
                    .ok_or_else(|| StepError::new("bank access requires character identity"))?;
                let bank = self
                    .world
                    .banks
                    .get(&capability.bank_id)
                    .ok_or_else(|| StepError::new("bank state is missing"))?;
                let bank_definition = self
                    .definition
                    .catalog
                    .bank_definitions
                    .get(&capability.bank_id)
                    .ok_or_else(|| StepError::new("bank definition is missing"))?;
                let mut deposit_actions = Vec::new();
                for pile in self.world.ground_gold.values().filter(|pile| {
                    pile.location.level == actor.location.level
                        && pile.location.position == actor.location.position
                }) {
                    let command = PlayerCommandV1 {
                        contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                        actor_id: actor_id.clone(),
                        intent: PlayerIntentPayloadV1::DepositBankGold {
                            service_id: service.id().to_string(),
                            capability_id: capability.id.clone(),
                            gold_pile_id: pile.id.clone(),
                        },
                    };
                    let status = self.validate_actor_command(&command)?;
                    deposit_actions.push(ActionOptionV1 {
                        id: format!("deposit_bank_gold_{}", pile.id),
                        label: format!("Deposit {} gold", pile.amount),
                        enabled: status.accepted,
                        blocked_reason: status.blocked_reason,
                        command: Some(command),
                    });
                }
                let balance = bank.balance(&character_id);
                let withdrawal_amount = balance.min(bank_definition.transaction_cap_gold);
                let withdrawal_command = PlayerCommandV1 {
                    contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                    actor_id: actor_id.clone(),
                    intent: PlayerIntentPayloadV1::WithdrawBankGold {
                        service_id: service.id().to_string(),
                        capability_id: capability.id.clone(),
                        amount: withdrawal_amount,
                    },
                };
                let withdrawal_status = self.validate_actor_command(&withdrawal_command)?;
                let withdrawal_command = (withdrawal_amount > 0).then_some(withdrawal_command);
                Ok(ServiceCapabilityViewV1::Bank {
                    capability_id: capability.id.clone(),
                    bank_id: capability.bank_id.as_str().to_string(),
                    balance_gold: balance,
                    transaction_cap_gold: bank_definition.transaction_cap_gold,
                    deposit_actions,
                    withdrawal_actions: vec![ActionOptionV1 {
                        id: format!("withdraw_bank_gold_{}_max", capability.bank_id.as_str()),
                        label: format!("Withdraw {withdrawal_amount} gold"),
                        enabled: withdrawal_status.accepted,
                        blocked_reason: withdrawal_status.blocked_reason,
                        command: withdrawal_command,
                    }],
                })
            }
            ServiceCapability::Locker(capability) => {
                let actor = &self.world.actors[actor_index];
                let character_id = actor
                    .character_id
                    .clone()
                    .ok_or_else(|| StepError::new("locker access requires character identity"))?;
                let vault = self
                    .world
                    .locker_vaults
                    .get(&capability.vault_id)
                    .ok_or_else(|| StepError::new("locker vault state is missing"))?;
                let item_ids = vault.contents(&character_id).to_vec();
                let items = item_ids
                    .iter()
                    .map(|item_id| self.item_instance_view(item_id))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut deposit_actions = Vec::new();
                for item_instance_id in self.carried_item_ids(actor_index)? {
                    let item = self.item_instance_view(&item_instance_id)?;
                    let command = PlayerCommandV1 {
                        contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                        actor_id: actor_id.clone(),
                        intent: PlayerIntentPayloadV1::DepositLockerItem {
                            service_id: service.id().to_string(),
                            capability_id: capability.id.clone(),
                            item_instance_id: item_instance_id.clone(),
                        },
                    };
                    let status = self.validate_actor_command(&command)?;
                    deposit_actions.push(ActionOptionV1 {
                        id: format!("deposit_locker_item_{item_instance_id}"),
                        label: format!("Store {}", item.name),
                        enabled: status.accepted,
                        blocked_reason: status.blocked_reason,
                        command: Some(command),
                    });
                }
                let mut withdrawal_actions = Vec::new();
                for item_instance_id in &item_ids {
                    let item = self.item_instance_view(item_instance_id)?;
                    let definition = self.item_definition(item_instance_id)?;
                    for destination in CarriedPosition::all().iter().copied().filter(|position| {
                        definition
                            .valid_placements
                            .contains(&position.placement_kind())
                    }) {
                        let command = PlayerCommandV1 {
                            contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                            actor_id: actor_id.clone(),
                            intent: PlayerIntentPayloadV1::WithdrawLockerItem {
                                service_id: service.id().to_string(),
                                capability_id: capability.id.clone(),
                                item_instance_id: item_instance_id.clone(),
                                destination,
                            },
                        };
                        let status = self.validate_actor_command(&command)?;
                        withdrawal_actions.push(ActionOptionV1 {
                            id: format!(
                                "withdraw_locker_item_{}_to_{}",
                                item_instance_id,
                                destination.label()
                            ),
                            label: format!("Withdraw {} to {}", item.name, destination.label()),
                            enabled: status.accepted,
                            blocked_reason: status.blocked_reason,
                            command: Some(command),
                        });
                    }
                }
                Ok(ServiceCapabilityViewV1::Locker {
                    capability_id: capability.id.clone(),
                    vault_id: capability.vault_id.as_str().to_string(),
                    capacity: self.definition.catalog.locker_vault_definitions
                        [&capability.vault_id]
                        .capacity,
                    item_count: u32::try_from(item_ids.len())
                        .map_err(|_| StepError::new("locker item count overflow"))?,
                    items,
                    deposit_actions,
                    withdrawal_actions,
                })
            }
        }
    }

    fn skill_training_capability_view(
        &self,
        actor_index: usize,
        actor_id: &crate::model::ActorId,
        service: ResolvedService<'_>,
        capability: &SkillTrainingCapability,
    ) -> Result<ServiceCapabilityViewV1, StepError> {
        let selected_track_id = self
            .training_focus_track_for_service_id(actor_index, service.id())
            .ok();
        let actions = selected_track_id
            .as_ref()
            .map(|track_id| {
                let offered_gold = self
                    .carried_gold_at(actor_index, crate::model::CarriedGoldPosition::Sack)
                    .unwrap_or(0);
                let command = PlayerCommandV1 {
                    contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                    actor_id: actor_id.clone(),
                    intent: PlayerIntentPayloadV1::Train {
                        service_id: service.id().to_string(),
                        offered_gold,
                    },
                };
                let status = self.validate_actor_command(&command)?;
                let command = (offered_gold > 0).then_some(command);
                let display = self
                    .skill_track_display(track_id)
                    .unwrap_or_else(|| track_id.clone());
                Ok::<ActionOptionV1, StepError>(ActionOptionV1 {
                    id: format!("train_{}", service.id()),
                    label: format!("Train {display}"),
                    enabled: status.accepted,
                    blocked_reason: status.blocked_reason,
                    command,
                })
            })
            .transpose()?
            .into_iter()
            .collect();

        Ok(ServiceCapabilityViewV1::SkillTraining {
            capability_id: capability.id.clone(),
            offered_track_ids: capability
                .offers
                .iter()
                .map(|offer| offer.track_id.clone())
                .collect(),
            selected_track_id,
            actions,
        })
    }

    fn skill_critique_capability_view(
        &self,
        actor_index: usize,
        actor_id: &crate::model::ActorId,
        service: ResolvedService<'_>,
        capability: &SkillCritiqueCapability,
    ) -> Result<ServiceCapabilityViewV1, StepError> {
        let track_ids = self
            .definition
            .catalog
            .skill_catalog
            .as_ref()
            .map(|catalog| {
                catalog
                    .tracks
                    .iter()
                    .map(|track| track.id.clone())
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .unwrap_or_default();
        let mut actions = Vec::new();
        for track_id in track_ids {
            if self
                .critique_plan(actor_index, service.id(), &track_id)
                .is_err()
            {
                continue;
            }
            let command = PlayerCommandV1 {
                contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                actor_id: actor_id.clone(),
                intent: PlayerIntentPayloadV1::Critique {
                    service_id: service.id().to_string(),
                    track_id: track_id.clone(),
                },
            };
            let status = self.validate_actor_command(&command)?;
            let display = self
                .skill_track_display(&track_id)
                .unwrap_or_else(|| track_id.clone());
            actions.push(ActionOptionV1 {
                id: format!("critique_{}_{}", service.id(), track_id),
                label: format!("Critique {display}"),
                enabled: status.accepted,
                blocked_reason: status.blocked_reason,
                command: Some(command),
            });
        }
        Ok(ServiceCapabilityViewV1::SkillCritique {
            capability_id: capability.id.clone(),
            actions,
        })
    }

    fn spell_teaching_capability_view(
        &self,
        _actor_index: usize,
        actor_id: &crate::model::ActorId,
        _service: ResolvedService<'_>,
        capability: &SpellTeachingCapability,
    ) -> Result<ServiceCapabilityViewV1, StepError> {
        let mut actions = Vec::new();
        for teaching in &capability.teachings {
            let command = PlayerCommandV1 {
                contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                actor_id: actor_id.clone(),
                intent: PlayerIntentPayloadV1::LearnSpell {
                    spell_id: teaching.spell_id.clone(),
                },
            };
            let status = self.validate_actor_command(&command)?;
            let spell_name = self
                .definition
                .catalog
                .spells
                .get(&teaching.spell_id)
                .map(|spell| spell.name.clone())
                .unwrap_or_else(|| teaching.spell_id.clone());
            actions.push(ActionOptionV1 {
                id: format!("learn_spell_{}", teaching.spell_id),
                label: format!("Learn {spell_name}"),
                enabled: status.accepted,
                blocked_reason: status.blocked_reason,
                command: Some(command),
            });
        }
        Ok(ServiceCapabilityViewV1::SpellTeaching {
            capability_id: capability.id.clone(),
            spell_ids: capability
                .teachings
                .iter()
                .map(|teaching| teaching.spell_id.clone())
                .collect(),
            actions,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn service_discovery_is_exactly_read_only_for_private_engine_state() {
        let engine = crate::engine::setup::test_engine("magic_profession_gallery");
        let before_world = engine.world.clone();
        let before_rng = engine.rng.clone();
        let before_definition = engine.definition.clone();
        let before_events = engine.initial_events.clone();

        let actor_id = crate::model::ActorId::from("player0");
        let first_v1 = engine.actor_action_context(&actor_id).expect("V1 context");
        let first_v2 = engine
            .actor_observed_action_context(&actor_id)
            .expect("V2 context");
        let first_options = engine.actor_action_options(&actor_id).expect("options");
        assert_eq!(
            first_v1,
            engine
                .actor_action_context(&actor_id)
                .expect("repeat V1 context")
        );
        assert_eq!(
            first_v2,
            engine
                .actor_observed_action_context(&actor_id)
                .expect("repeat V2 context")
        );
        assert_eq!(
            first_options,
            engine
                .actor_action_options(&actor_id)
                .expect("repeat options")
        );
        assert_eq!(engine.world, before_world);
        assert_eq!(engine.rng, before_rng);
        assert!(std::sync::Arc::ptr_eq(
            &engine.definition,
            &before_definition
        ));
        assert_eq!(engine.initial_events, before_events);
    }
}
