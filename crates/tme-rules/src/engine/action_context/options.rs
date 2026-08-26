use super::*;

impl Engine {
    fn push_spell_action_option(
        &self,
        options: &mut Vec<ActionOptionV1>,
        actor_id: &crate::model::ActorId,
        id: String,
        label: String,
        intent: PlayerIntentPayloadV1,
    ) -> Result<(), StepError> {
        let command = PlayerCommandV1 {
            contract_version: crate::view::COMMAND_CONTRACT_VERSION,
            actor_id: actor_id.clone(),
            intent,
        };
        let (enabled, blocked_reason) = self.command_option_status(&command, true, None)?;
        options.push(ActionOptionV1 {
            id,
            label,
            enabled,
            blocked_reason,
            command: Some(command),
        });
        Ok(())
    }

    fn command_option_status(
        &self,
        command: &PlayerCommandV1,
        drafted_enabled: bool,
        drafted_blocked: Option<ActionBlockedReasonV1>,
    ) -> Result<(bool, Option<ActionBlockedReasonV1>), StepError> {
        let status = self.validate_actor_command(command)?;
        if status.accepted {
            Ok((drafted_enabled, drafted_blocked))
        } else {
            Ok((false, status.blocked_reason))
        }
    }

    /// Generate typed action options from the observed action context.
    /// Each option includes an enabled flag, optional block reason, and a
    /// ready-to-submit `PlayerCommandV1` payload.
    pub fn actor_action_options(
        &self,
        actor_id: &crate::model::ActorId,
    ) -> Result<Vec<ActionOptionV1>, StepError> {
        let ctx = self.actor_observed_action_context(actor_id)?;
        let mut options: Vec<ActionOptionV1> = Vec::new();
        let actor_id = ctx.actor_id.clone();

        // Movement exits
        for exit in &ctx.exits {
            let id = format!("move_{}", exit.direction.label());
            let label = format!("Move {}", exit.direction.label());
            let command = PlayerCommandV1 {
                contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                actor_id: actor_id.clone(),
                intent: PlayerIntentPayloadV1::MovePath {
                    path: vec![exit.direction],
                },
            };
            let (enabled, blocked_reason) =
                self.command_option_status(&command, !exit.blocked, exit.blocked_reason)?;
            options.push(ActionOptionV1 {
                id: id.clone(),
                label,
                enabled,
                blocked_reason,
                command: Some(command),
            });
        }

        // Attack targets
        for target in &ctx.attack_targets {
            for physical_attack in &target.physical_attacks {
                options.push(ActionOptionV1 {
                    id: format!(
                        "physical_attack_{}_{}",
                        physical_attack.mode.label(),
                        target.actor_id
                    ),
                    label: format!("{} {}", physical_attack.mode.label(), target.actor_name),
                    enabled: physical_attack.enabled,
                    blocked_reason: physical_attack.blocked_reason,
                    command: physical_attack.command.clone(),
                });
            }
        }

        self.push_spell_action_option(
            &mut options,
            &actor_id,
            "nock".to_string(),
            "Nock bow".to_string(),
            PlayerIntentPayloadV1::Nock,
        )?;
        self.push_spell_action_option(
            &mut options,
            &actor_id,
            "unload_bow".to_string(),
            "Unload bow".to_string(),
            PlayerIntentPayloadV1::UnloadBow,
        )?;

        for corpse in &ctx.corpses_here {
            let command = PlayerCommandV1 {
                contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                actor_id: actor_id.clone(),
                intent: PlayerIntentPayloadV1::SearchCorpse {
                    corpse_id: corpse.corpse_id.clone(),
                },
            };
            let (enabled, blocked_reason) = self.command_option_status(&command, true, None)?;
            options.push(ActionOptionV1 {
                id: format!("search_corpse:{}", corpse.corpse_id),
                label: format!("Search {} corpse", corpse.origin_name),
                enabled,
                blocked_reason,
                command: Some(command),
            });
        }

        // Exact carried destinations for ground items at the actor's position.
        for item in &ctx.ground_items_here {
            let definition = self.item_definition(&item.item_instance_id)?;
            for position in CarriedPosition::all().iter().copied().filter(|position| {
                definition
                    .valid_placements
                    .contains(&position.placement_kind())
            }) {
                let command = PlayerCommandV1 {
                    contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                    actor_id: actor_id.clone(),
                    intent: PlayerIntentPayloadV1::MoveItem {
                        item_instance_id: item.item_instance_id.clone(),
                        destination: ItemMoveDestination::Carried { position },
                    },
                };
                let (enabled, blocked_reason) = self.command_option_status(&command, true, None)?;
                options.push(ActionOptionV1 {
                    id: format!("move_{}_to_{}", item.item_instance_id, position.label()),
                    label: format!("Move {} to {}", item.name, position.label()),
                    enabled,
                    blocked_reason,
                    command: Some(command),
                });
            }
        }

        // Every positioned item can move to ground or another authored position.
        for positioned in &ctx.carried.items {
            let item = &positioned.item;
            let ground_command = PlayerCommandV1 {
                contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                actor_id: actor_id.clone(),
                intent: PlayerIntentPayloadV1::MoveItem {
                    item_instance_id: item.item_instance_id.clone(),
                    destination: ItemMoveDestination::GroundHere,
                },
            };
            let (enabled, blocked_reason) =
                self.command_option_status(&ground_command, true, None)?;
            options.push(ActionOptionV1 {
                id: format!("move_{}_to_ground_here", item.item_instance_id),
                label: format!("Move {} to ground", item.name),
                enabled,
                blocked_reason,
                command: Some(ground_command),
            });

            for position in CarriedPosition::all().iter().copied().filter(|position| {
                *position != positioned.position
                    && positioned
                        .valid_placements
                        .contains(&position.placement_kind())
            }) {
                let command = PlayerCommandV1 {
                    contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                    actor_id: actor_id.clone(),
                    intent: PlayerIntentPayloadV1::MoveItem {
                        item_instance_id: item.item_instance_id.clone(),
                        destination: ItemMoveDestination::Carried { position },
                    },
                };
                let (enabled, blocked_reason) = self.command_option_status(&command, true, None)?;
                options.push(ActionOptionV1 {
                    id: format!("move_{}_to_{}", item.item_instance_id, position.label()),
                    label: format!("Move {} to {}", item.name, position.label()),
                    enabled,
                    blocked_reason,
                    command: Some(command),
                });
            }
        }

        // Positioned gold can move between carried positions and exact ground piles.
        let gold_positions = [
            CarriedGoldPosition::LeftHand,
            CarriedGoldPosition::RightHand,
            CarriedGoldPosition::Sack,
        ];
        for pile in &ctx.ground_gold_here {
            for destination in gold_positions {
                let command = PlayerCommandV1 {
                    contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                    actor_id: actor_id.clone(),
                    intent: PlayerIntentPayloadV1::MoveGold {
                        source: GoldMoveSource::Ground {
                            gold_pile_id: pile.gold_pile_id.clone(),
                        },
                        destination: GoldMoveDestination::Carried {
                            position: destination,
                        },
                        quantity: GoldMoveQuantity::All,
                    },
                };
                let (enabled, blocked_reason) = self.command_option_status(&command, true, None)?;
                options.push(ActionOptionV1 {
                    id: format!("move_gold_{}_to_{}", pile.gold_pile_id, destination.label()),
                    label: format!("Move {} gold to {}", pile.amount, destination.label()),
                    enabled,
                    blocked_reason,
                    command: Some(command),
                });
            }
        }
        for source in gold_positions {
            let amount = ctx.carried.gold.amount(source);
            if amount <= 0 {
                continue;
            }
            let ground_command = PlayerCommandV1 {
                contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                actor_id: actor_id.clone(),
                intent: PlayerIntentPayloadV1::MoveGold {
                    source: GoldMoveSource::Carried { position: source },
                    destination: GoldMoveDestination::GroundHere,
                    quantity: GoldMoveQuantity::All,
                },
            };
            let (enabled, blocked_reason) =
                self.command_option_status(&ground_command, true, None)?;
            options.push(ActionOptionV1 {
                id: format!("move_gold_{}_to_ground_here", source.label()),
                label: format!("Move {amount} gold from {} to ground", source.label()),
                enabled,
                blocked_reason,
                command: Some(ground_command),
            });
            for destination in gold_positions
                .into_iter()
                .filter(|position| *position != source)
            {
                let command = PlayerCommandV1 {
                    contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                    actor_id: actor_id.clone(),
                    intent: PlayerIntentPayloadV1::MoveGold {
                        source: GoldMoveSource::Carried { position: source },
                        destination: GoldMoveDestination::Carried {
                            position: destination,
                        },
                        quantity: GoldMoveQuantity::All,
                    },
                };
                let (enabled, blocked_reason) = self.command_option_status(&command, true, None)?;
                options.push(ActionOptionV1 {
                    id: format!("move_gold_{}_to_{}", source.label(), destination.label()),
                    label: format!(
                        "Move {amount} gold from {} to {}",
                        source.label(),
                        destination.label()
                    ),
                    enabled,
                    blocked_reason,
                    command: Some(command),
                });
            }
        }

        options.extend(ctx.item_offer_actions.iter().cloned());
        for offer in ctx
            .incoming_item_offers
            .iter()
            .chain(&ctx.outgoing_item_offers)
        {
            options.extend(offer.actions.iter().cloned());
        }

        // Drink consumables
        for item in &ctx.usable_items {
            let id = format!("drink_{}", item.item.item_instance_id);
            let label = format!("Drink {}", item.item.name);
            let command = PlayerCommandV1 {
                contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                actor_id: actor_id.clone(),
                intent: PlayerIntentPayloadV1::Drink {
                    item_instance_id: item.item.item_instance_id.clone(),
                },
            };
            let (enabled, blocked_reason) = self.command_option_status(&command, true, None)?;
            options.push(ActionOptionV1 {
                id: id.clone(),
                label,
                enabled,
                blocked_reason,
                command: Some(command),
            });
        }

        // Door actions
        for door in &ctx.door_actions {
            if door.can_open {
                let id = format!("open_{}", door.direction.label());
                let label = format!("Open door {}", door.direction.label());
                let command = PlayerCommandV1 {
                    contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                    actor_id: actor_id.clone(),
                    intent: PlayerIntentPayloadV1::Open {
                        direction: door.direction,
                    },
                };
                let (enabled, blocked_reason) = self.command_option_status(&command, true, None)?;
                options.push(ActionOptionV1 {
                    id: id.clone(),
                    label,
                    enabled,
                    blocked_reason,
                    command: Some(command),
                });
            }
            if door.can_close {
                let id = format!("close_{}", door.direction.label());
                let label = format!("Close door {}", door.direction.label());
                let command = PlayerCommandV1 {
                    contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                    actor_id: actor_id.clone(),
                    intent: PlayerIntentPayloadV1::Close {
                        direction: door.direction,
                    },
                };
                let (enabled, blocked_reason) = self.command_option_status(&command, true, None)?;
                options.push(ActionOptionV1 {
                    id: id.clone(),
                    label,
                    enabled,
                    blocked_reason,
                    command: Some(command),
                });
            }
        }

        // Explicit traversal commands are always present as typed options.
        // Their enabled state comes from the shared navigation evaluator.
        for kind in [
            ExplicitTraversalKind::StairsUp,
            ExplicitTraversalKind::StairsDown,
            ExplicitTraversalKind::ClimbUp,
            ExplicitTraversalKind::ClimbDown,
        ] {
            let command = PlayerCommandV1 {
                contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                actor_id: actor_id.clone(),
                intent: PlayerIntentPayloadV1::Traverse { kind },
            };
            let (enabled, blocked_reason) = self.command_option_status(&command, true, None)?;
            options.push(ActionOptionV1 {
                id: kind.label().to_string(),
                label: format!("traverse {}", kind.label()),
                enabled,
                blocked_reason,
                command: Some(command),
            });
        }

        // Concrete spell commands are exposed only when the descriptor has complete input.
        for spell in &ctx.spell_actions {
            if let Some(command) = spell.warm.command.clone() {
                options.push(ActionOptionV1 {
                    id: format!("warm_{}", spell.spell_id),
                    label: format!("Warm {}", spell.spell_name),
                    enabled: true,
                    blocked_reason: None,
                    command: Some(command),
                });
            }
            if let Some(command) = spell.cast.command.clone() {
                options.push(ActionOptionV1 {
                    id: format!("cast_{}", spell.spell_id),
                    label: format!("Cast {}", spell.spell_name),
                    enabled: true,
                    blocked_reason: None,
                    command: Some(command),
                });
            }
        }

        let rest = PlayerCommandV1 {
            contract_version: crate::view::COMMAND_CONTRACT_VERSION,
            actor_id: actor_id.clone(),
            intent: PlayerIntentPayloadV1::Rest,
        };
        let (rest_enabled, rest_blocked_reason) = self.command_option_status(&rest, true, None)?;
        options.push(ActionOptionV1 {
            id: "rest".to_string(),
            label: "Rest".to_string(),
            enabled: rest_enabled,
            blocked_reason: rest_blocked_reason,
            command: Some(rest),
        });
        if ctx.warmed_spell.is_some() {
            let fizzle = PlayerCommandV1 {
                contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                actor_id: actor_id.clone(),
                intent: PlayerIntentPayloadV1::FizzleWarmedSpell,
            };
            let (fizzle_enabled, fizzle_blocked_reason) =
                self.command_option_status(&fizzle, true, None)?;
            options.push(ActionOptionV1 {
                id: "fizzle_warmed_spell".to_string(),
                label: "Fizzle warmed spell".to_string(),
                enabled: fizzle_enabled,
                blocked_reason: fizzle_blocked_reason,
                command: Some(fizzle),
            });
        }

        // Profession actions
        let hide = PlayerCommandV1 {
            contract_version: crate::view::COMMAND_CONTRACT_VERSION,
            actor_id: actor_id.clone(),
            intent: PlayerIntentPayloadV1::Hide,
        };
        let (hide_enabled, hide_blocked_reason) = self.command_option_status(&hide, true, None)?;
        options.push(ActionOptionV1 {
            id: "hide".to_string(),
            label: "Hide".to_string(),
            enabled: hide_enabled,
            blocked_reason: hide_blocked_reason,
            command: Some(hide),
        });

        // Always-available actions
        let show_sack = PlayerCommandV1 {
            contract_version: crate::view::COMMAND_CONTRACT_VERSION,
            actor_id: actor_id.clone(),
            intent: PlayerIntentPayloadV1::ShowSack,
        };
        let (show_sack_enabled, show_sack_blocked_reason) =
            self.command_option_status(&show_sack, true, None)?;
        options.push(ActionOptionV1 {
            id: "show_sack".to_string(),
            label: "Show Sack".to_string(),
            enabled: show_sack_enabled,
            blocked_reason: show_sack_blocked_reason,
            command: Some(show_sack),
        });
        let wait = PlayerCommandV1 {
            contract_version: crate::view::COMMAND_CONTRACT_VERSION,
            actor_id: actor_id.clone(),
            intent: PlayerIntentPayloadV1::Wait,
        };
        let (wait_enabled, wait_blocked_reason) = self.command_option_status(&wait, true, None)?;
        options.push(ActionOptionV1 {
            id: "wait".to_string(),
            label: "Wait".to_string(),
            enabled: wait_enabled,
            blocked_reason: wait_blocked_reason,
            command: Some(wait),
        });
        let inspect = PlayerCommandV1 {
            contract_version: crate::view::COMMAND_CONTRACT_VERSION,
            actor_id: actor_id.clone(),
            intent: PlayerIntentPayloadV1::Inspect,
        };
        let (inspect_enabled, inspect_blocked_reason) =
            self.command_option_status(&inspect, true, None)?;
        options.push(ActionOptionV1 {
            id: "inspect".to_string(),
            label: "Inspect".to_string(),
            enabled: inspect_enabled,
            blocked_reason: inspect_blocked_reason,
            command: Some(inspect),
        });

        for service in &ctx.services_here {
            for capability in &service.capabilities {
                match capability {
                    ServiceCapabilityViewV1::SkillTraining { actions, .. }
                    | ServiceCapabilityViewV1::SkillCritique { actions, .. } => {
                        options.extend(actions.iter().cloned());
                    }
                    ServiceCapabilityViewV1::Bank {
                        deposit_actions,
                        withdrawal_actions,
                        ..
                    } => {
                        options.extend(deposit_actions.iter().cloned());
                        options.extend(withdrawal_actions.iter().cloned());
                    }
                    ServiceCapabilityViewV1::Locker {
                        deposit_actions,
                        withdrawal_actions,
                        ..
                    } => {
                        options.extend(deposit_actions.iter().cloned());
                        options.extend(withdrawal_actions.iter().cloned());
                    }
                    ServiceCapabilityViewV1::SpellTeaching { .. }
                    | ServiceCapabilityViewV1::ClassPromotion { .. }
                    | ServiceCapabilityViewV1::ServiceTransaction { .. }
                    | ServiceCapabilityViewV1::Merchant { .. }
                    | ServiceCapabilityViewV1::ItemService { .. }
                    | ServiceCapabilityViewV1::Restoration { .. } => {}
                }
            }
        }
        for service in &ctx.services_here {
            for capability in &service.capabilities {
                if let ServiceCapabilityViewV1::ClassPromotion { actions, .. } = capability {
                    options.extend(actions.iter().cloned());
                }
            }
        }
        for service in &ctx.services_here {
            for capability in &service.capabilities {
                if let ServiceCapabilityViewV1::SpellTeaching { actions, .. } = capability {
                    options.extend(actions.iter().cloned());
                }
            }
        }
        for service in &ctx.services_here {
            for capability in &service.capabilities {
                if let ServiceCapabilityViewV1::ServiceTransaction { transactions, .. } = capability
                {
                    for transaction in transactions {
                        options.extend(transaction.actions.iter().cloned());
                    }
                }
            }
        }
        for service in &ctx.services_here {
            for capability in &service.capabilities {
                match capability {
                    ServiceCapabilityViewV1::Merchant {
                        listings,
                        buy_all,
                        sales,
                        ..
                    } => {
                        options.extend(listings.iter().map(|listing| listing.purchase.clone()));
                        options.push(buy_all.clone());
                        options.extend(sales.iter().cloned());
                    }
                    ServiceCapabilityViewV1::ItemService { operations, .. } => {
                        for operation in operations {
                            options.extend(operation.actions.iter().cloned());
                        }
                    }
                    ServiceCapabilityViewV1::Restoration { operations, .. } => {
                        for operation in operations {
                            options.extend(operation.actions.iter().cloned());
                        }
                    }
                    _ => {}
                }
            }
        }
        for npc in &ctx.npcs_here {
            for interaction in &npc.interactions {
                options.extend(interaction.actions.iter().cloned());
            }
        }

        Ok(options)
    }
}
