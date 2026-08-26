use super::*;

impl Engine {
    fn command_validation_time(&self) -> crate::model::LogicalTime {
        self.current_time()
    }

    pub(in crate::engine) fn command_blocked_reason_code(
        reason: ActionBlockedReasonV1,
    ) -> &'static str {
        reason.code()
    }

    /// Convert a `PlayerIntent` into its typed payload representation.
    /// This is the canonical conversion; use it instead of duplicating the match.
    pub fn player_intent_to_payload(intent: &PlayerIntent) -> PlayerIntentPayloadV1 {
        match intent {
            PlayerIntent::MovePath(p) => PlayerIntentPayloadV1::MovePath { path: p.clone() },
            PlayerIntent::Traverse(kind) => PlayerIntentPayloadV1::Traverse { kind: *kind },
            PlayerIntent::Hide => PlayerIntentPayloadV1::Hide,
            PlayerIntent::Nock => PlayerIntentPayloadV1::Nock,
            PlayerIntent::UnloadBow => PlayerIntentPayloadV1::UnloadBow,
            PlayerIntent::PhysicalAttack {
                mode,
                target_actor_id,
                authorization,
            } => PlayerIntentPayloadV1::PhysicalAttack {
                mode: *mode,
                target_actor_id: target_actor_id.clone(),
                authorization: *authorization,
            },
            PlayerIntent::SearchCorpse(corpse_id) => PlayerIntentPayloadV1::SearchCorpse {
                corpse_id: corpse_id.clone(),
            },
            PlayerIntent::MoveItem {
                item_instance_id,
                destination,
            } => PlayerIntentPayloadV1::MoveItem {
                item_instance_id: item_instance_id.clone(),
                destination: destination.clone(),
            },
            PlayerIntent::MoveGold {
                source,
                destination,
                quantity,
            } => PlayerIntentPayloadV1::MoveGold {
                source: source.clone(),
                destination: destination.clone(),
                quantity: quantity.clone(),
            },
            PlayerIntent::DepositBankGold {
                service_id,
                capability_id,
                gold_pile_id,
            } => PlayerIntentPayloadV1::DepositBankGold {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                gold_pile_id: gold_pile_id.clone(),
            },
            PlayerIntent::WithdrawBankGold {
                service_id,
                capability_id,
                amount,
            } => PlayerIntentPayloadV1::WithdrawBankGold {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                amount: *amount,
            },
            PlayerIntent::DepositLockerItem {
                service_id,
                capability_id,
                item_instance_id,
            } => PlayerIntentPayloadV1::DepositLockerItem {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                item_instance_id: item_instance_id.clone(),
            },
            PlayerIntent::WithdrawLockerItem {
                service_id,
                capability_id,
                item_instance_id,
                destination,
            } => PlayerIntentPayloadV1::WithdrawLockerItem {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                item_instance_id: item_instance_id.clone(),
                destination: *destination,
            },
            PlayerIntent::OfferItem {
                recipient_character_id,
                item_instance_id,
            } => PlayerIntentPayloadV1::OfferItem {
                recipient_character_id: recipient_character_id.clone(),
                item_instance_id: item_instance_id.clone(),
            },
            PlayerIntent::AcceptItemOffer {
                item_instance_id,
                destination,
            } => PlayerIntentPayloadV1::AcceptItemOffer {
                item_instance_id: item_instance_id.clone(),
                destination: *destination,
            },
            PlayerIntent::RefuseItemOffer { item_instance_id } => {
                PlayerIntentPayloadV1::RefuseItemOffer {
                    item_instance_id: item_instance_id.clone(),
                }
            }
            PlayerIntent::WithdrawItemOffer { item_instance_id } => {
                PlayerIntentPayloadV1::WithdrawItemOffer {
                    item_instance_id: item_instance_id.clone(),
                }
            }
            PlayerIntent::Drink(id) => PlayerIntentPayloadV1::Drink {
                item_instance_id: id.clone(),
            },
            PlayerIntent::Open(d) => PlayerIntentPayloadV1::Open { direction: *d },
            PlayerIntent::Close(d) => PlayerIntentPayloadV1::Close { direction: *d },
            PlayerIntent::ShowSack => PlayerIntentPayloadV1::ShowSack,
            PlayerIntent::Wait => PlayerIntentPayloadV1::Wait,
            PlayerIntent::Inspect => PlayerIntentPayloadV1::Inspect,
            PlayerIntent::Train {
                service_id,
                offered_gold,
            } => PlayerIntentPayloadV1::Train {
                service_id: service_id.clone(),
                offered_gold: *offered_gold,
            },
            PlayerIntent::Critique {
                service_id,
                track_id,
            } => PlayerIntentPayloadV1::Critique {
                service_id: service_id.clone(),
                track_id: track_id.clone(),
            },
            PlayerIntent::PromoteClass(target) => PlayerIntentPayloadV1::PromoteClass {
                target_class_id: target.clone(),
            },
            PlayerIntent::LearnSpell(spell_id) => PlayerIntentPayloadV1::LearnSpell {
                spell_id: spell_id.clone(),
            },
            PlayerIntent::CommitServiceTransaction {
                service_id,
                capability_id,
                transaction_id,
                item_instance_id,
            } => PlayerIntentPayloadV1::CommitServiceTransaction {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                transaction_id: transaction_id.clone(),
                item_instance_id: item_instance_id.clone(),
            },
            PlayerIntent::BuyFromMerchant {
                service_id,
                capability_id,
                item_instance_ids,
            } => PlayerIntentPayloadV1::BuyFromMerchant {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                item_instance_ids: item_instance_ids.clone(),
            },
            PlayerIntent::SellToMerchant {
                service_id,
                capability_id,
                item_instance_id,
            } => PlayerIntentPayloadV1::SellToMerchant {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                item_instance_id: item_instance_id.clone(),
            },
            PlayerIntent::UseItemService {
                service_id,
                capability_id,
                operation,
                item_instance_id,
            } => PlayerIntentPayloadV1::UseItemService {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                operation: *operation,
                item_instance_id: item_instance_id.clone(),
            },
            PlayerIntent::UseRestorationService {
                service_id,
                capability_id,
                operation_id,
                item_instance_id,
                corpse_id,
            } => PlayerIntentPayloadV1::UseRestorationService {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                operation_id: operation_id.clone(),
                item_instance_id: item_instance_id.clone(),
                corpse_id: corpse_id.clone(),
            },
            PlayerIntent::InteractWithNpc {
                npc_actor_id,
                interaction_id,
                item_instance_id,
            } => PlayerIntentPayloadV1::InteractWithNpc {
                npc_actor_id: npc_actor_id.clone(),
                interaction_id: interaction_id.clone(),
                item_instance_id: item_instance_id.clone(),
            },
            PlayerIntent::CastSpell {
                spell_id,
                target,
                authorization,
            } => PlayerIntentPayloadV1::CastSpell {
                spell_id: spell_id.clone(),
                target: target.clone(),
                authorization: *authorization,
            },
            PlayerIntent::WarmSpell { spell_id } => PlayerIntentPayloadV1::WarmSpell {
                spell_id: spell_id.clone(),
            },
            PlayerIntent::CastWarmedSpell {
                target,
                authorization,
            } => PlayerIntentPayloadV1::CastWarmedSpell {
                target: target.clone(),
                authorization: *authorization,
            },
            PlayerIntent::ClearSelfDefense {
                attacker_character_id,
            } => PlayerIntentPayloadV1::ClearSelfDefense {
                attacker_character_id: attacker_character_id.clone(),
            },
            PlayerIntent::FizzleWarmedSpell => PlayerIntentPayloadV1::FizzleWarmedSpell,
            PlayerIntent::Rest => PlayerIntentPayloadV1::Rest,
        }
    }

    pub fn actor_command_for_intent(
        &self,
        actor_id: &crate::model::ActorId,
        intent: &PlayerIntent,
    ) -> Result<PlayerCommandV1, StepError> {
        self.player_actor_index(actor_id)?;

        Ok(PlayerCommandV1 {
            contract_version: crate::view::COMMAND_CONTRACT_VERSION,
            actor_id: actor_id.clone(),
            intent: Self::player_intent_to_payload(intent),
        })
    }

    /// Validate a player command before commit. Returns a typed status
    /// with accept/reject and optional `ActionBlockedReasonV1`.
    /// Actor mismatch returns `Err(StepError)` (programming error).
    pub fn validate_actor_command(
        &self,
        command: &PlayerCommandV1,
    ) -> Result<PlayerCommandStatusV1, StepError> {
        let make_status = |accepted, blocked_reason| PlayerCommandStatusV1 {
            contract_version: crate::view::COMMAND_CONTRACT_VERSION,
            command: command.clone(),
            accepted,
            blocked_reason,
        };

        // Version gate — stale contract versions are out of bounds
        if command.contract_version != crate::view::COMMAND_CONTRACT_VERSION {
            return Ok(make_status(false, Some(ActionBlockedReasonV1::OutOfBounds)));
        }

        // Actor gate
        let player_index = self.player_actor_index(&command.actor_id)?;
        let player = &self.world.actors[player_index];
        if !player.is_alive() {
            return Ok(make_status(
                false,
                Some(ActionBlockedReasonV1::ActorNotLiving),
            ));
        }

        let suppressed = self.suppressing_effect_for_actor(player_index).is_some();
        let passive_intent = matches!(
            command.intent,
            PlayerIntentPayloadV1::Wait | PlayerIntentPayloadV1::Inspect
        );
        if suppressed && !passive_intent {
            return Ok(make_status(
                false,
                Some(ActionBlockedReasonV1::SuppressedByStatus),
            ));
        }

        match &command.intent {
            PlayerIntentPayloadV1::MovePath { path } => {
                if !(1..=MAX_CONTROLLED_PATH_STEPS).contains(&path.len()) {
                    return Ok(make_status(false, Some(ActionBlockedReasonV1::OutOfBounds)));
                }
                let plan = self.evaluate_actor_path(
                    player_index,
                    path,
                    self.definition
                        .catalog
                        .rules
                        .movement
                        .controlled_path_points,
                )?;
                if plan.accepted_steps > 0 {
                    return Ok(make_status(true, None));
                }
                let reason = plan.steps.first().and_then(|step| match &step.outcome {
                    MovementStepOutcome::Blocked { reason } => Some(match *reason {
                        MovementBlockedReason::SuppressedByStatus => {
                            ActionBlockedReasonV1::SuppressedByStatus
                        }
                        MovementBlockedReason::OutOfBounds => ActionBlockedReasonV1::OutOfBounds,
                        MovementBlockedReason::BlockedTerrain => {
                            ActionBlockedReasonV1::BlockedTerrain
                        }
                        MovementBlockedReason::InsufficientMovementPoints => {
                            ActionBlockedReasonV1::InsufficientMovementPoints
                        }
                    }),
                    _ => None,
                });
                Ok(make_status(
                    false,
                    Some(reason.unwrap_or(ActionBlockedReasonV1::BlockedTerrain)),
                ))
            }
            PlayerIntentPayloadV1::Traverse { kind } => {
                match self.evaluate_explicit_traversal(player_index, *kind) {
                    Ok(_) => Ok(make_status(true, None)),
                    Err(ExplicitTraversalBlockedReason::NoTraversalHere) => Ok(make_status(
                        false,
                        Some(ActionBlockedReasonV1::NoTraversalHere),
                    )),
                    Err(ExplicitTraversalBlockedReason::WrongDirection) => Ok(make_status(
                        false,
                        Some(ActionBlockedReasonV1::WrongTraversalKind),
                    )),
                }
            }
            PlayerIntentPayloadV1::Hide => match self.validate_hide_action(player_index) {
                Ok(_) => Ok(make_status(true, None)),
                Err(reason) => Ok(make_status(false, Some(reason))),
            },
            PlayerIntentPayloadV1::Nock => {
                let selection = match self.physical_weapon_selection(player_index) {
                    Ok(selection) if selection.is_bow() => selection,
                    _ => {
                        return Ok(make_status(
                            false,
                            Some(ActionBlockedReasonV1::RightHandNotWeapon),
                        ));
                    }
                };
                if selection.offhand_occupied {
                    Ok(make_status(
                        false,
                        Some(ActionBlockedReasonV1::LeftHandOccupied),
                    ))
                } else if selection.bow_readiness == Some(crate::model::BowReadiness::Nocked) {
                    Ok(make_status(
                        false,
                        Some(ActionBlockedReasonV1::BowAlreadyNocked),
                    ))
                } else {
                    Ok(make_status(true, None))
                }
            }
            PlayerIntentPayloadV1::UnloadBow => {
                let selection = match self.physical_weapon_selection(player_index) {
                    Ok(selection) if selection.is_bow() => selection,
                    _ => {
                        return Ok(make_status(
                            false,
                            Some(ActionBlockedReasonV1::RightHandNotWeapon),
                        ));
                    }
                };
                if selection.bow_readiness != Some(crate::model::BowReadiness::Nocked) {
                    Ok(make_status(
                        false,
                        Some(ActionBlockedReasonV1::BowNotNockedForUnload),
                    ))
                } else {
                    Ok(make_status(true, None))
                }
            }
            PlayerIntentPayloadV1::PhysicalAttack {
                mode,
                target_actor_id,
                authorization,
            } => match self.live_actor_by_id(target_actor_id) {
                None => Ok(make_status(
                    false,
                    Some(ActionBlockedReasonV1::NoSuchTarget),
                )),
                Some(target_index) => {
                    if self.current_time() < self.world.actors[player_index].attack_ready_at {
                        return Ok(make_status(false, Some(ActionBlockedReasonV1::NotReady)));
                    }
                    match self.physical_attack_plan(
                        player_index,
                        target_index,
                        *mode,
                        *authorization,
                    ) {
                        Ok(_) => Ok(make_status(true, None)),
                        Err(error) => Ok(make_status(
                            false,
                            Some(Self::physical_attack_error_reason(&error)),
                        )),
                    }
                }
            },
            PlayerIntentPayloadV1::SearchCorpse { corpse_id } => {
                match self.validate_corpse_search(player_index, corpse_id) {
                    Ok(()) => Ok(make_status(true, None)),
                    Err(reason) => Ok(make_status(false, Some(reason))),
                }
            }
            PlayerIntentPayloadV1::MoveItem {
                item_instance_id,
                destination,
            } => match self.validate_item_move(player_index, item_instance_id, destination) {
                Ok(_) => Ok(make_status(true, None)),
                Err(error) => Ok(make_status(false, Some(error.reason))),
            },
            PlayerIntentPayloadV1::MoveGold {
                source,
                destination,
                quantity,
            } => {
                match self.validate_player_move_gold(player_index, source, destination, quantity) {
                    Ok(()) => Ok(make_status(true, None)),
                    Err(error) => Ok(make_status(false, Some(error.reason()))),
                }
            }
            PlayerIntentPayloadV1::DepositBankGold {
                service_id,
                capability_id,
                gold_pile_id,
            } => match self.validate_bank_deposit(
                player_index,
                service_id,
                capability_id,
                gold_pile_id,
            ) {
                Ok(()) => Ok(make_status(true, None)),
                Err(error) => Ok(make_status(false, Some(error.reason()))),
            },
            PlayerIntentPayloadV1::WithdrawBankGold {
                service_id,
                capability_id,
                amount,
            } => match self.validate_bank_withdrawal(
                player_index,
                service_id,
                capability_id,
                *amount,
            ) {
                Ok(()) => Ok(make_status(true, None)),
                Err(error) => Ok(make_status(false, Some(error.reason()))),
            },
            PlayerIntentPayloadV1::DepositLockerItem {
                service_id,
                capability_id,
                item_instance_id,
            } => match self.validate_locker_deposit(
                player_index,
                service_id,
                capability_id,
                item_instance_id,
            ) {
                Ok(()) => Ok(make_status(true, None)),
                Err(error) => Ok(make_status(false, Some(error.reason()))),
            },
            PlayerIntentPayloadV1::WithdrawLockerItem {
                service_id,
                capability_id,
                item_instance_id,
                destination,
            } => match self.validate_locker_withdrawal(
                player_index,
                service_id,
                capability_id,
                item_instance_id,
                *destination,
            ) {
                Ok(()) => Ok(make_status(true, None)),
                Err(error) => Ok(make_status(false, Some(error.reason()))),
            },
            PlayerIntentPayloadV1::OfferItem {
                recipient_character_id,
                item_instance_id,
            } => match self.validate_item_offer(
                player_index,
                recipient_character_id,
                item_instance_id,
            ) {
                Ok(()) => Ok(make_status(true, None)),
                Err(error) => Ok(make_status(false, Some(error.reason()))),
            },
            PlayerIntentPayloadV1::AcceptItemOffer {
                item_instance_id,
                destination,
            } => {
                match self.validate_accept_item_offer(player_index, item_instance_id, *destination)
                {
                    Ok(()) => Ok(make_status(true, None)),
                    Err(error) => Ok(make_status(false, Some(error.reason()))),
                }
            }
            PlayerIntentPayloadV1::RefuseItemOffer { item_instance_id } => {
                match self.validate_refuse_item_offer(player_index, item_instance_id) {
                    Ok(()) => Ok(make_status(true, None)),
                    Err(error) => Ok(make_status(false, Some(error.reason()))),
                }
            }
            PlayerIntentPayloadV1::WithdrawItemOffer { item_instance_id } => {
                match self.validate_withdraw_item_offer(player_index, item_instance_id) {
                    Ok(()) => Ok(make_status(true, None)),
                    Err(error) => Ok(make_status(false, Some(error.reason()))),
                }
            }
            PlayerIntentPayloadV1::Drink { item_instance_id } => {
                let usable = self.collect_usable_items(player_index);
                if usable
                    .iter()
                    .any(|item| item.item.item_instance_id == *item_instance_id)
                {
                    Ok(make_status(true, None))
                } else {
                    Ok(make_status(false, Some(ActionBlockedReasonV1::NoSuchItem)))
                }
            }
            PlayerIntentPayloadV1::Open { direction }
            | PlayerIntentPayloadV1::Close { direction } => {
                let doors = self.collect_door_actions(player_index);
                match doors.iter().find(|d| d.direction == *direction) {
                    None => Ok(make_status(
                        false,
                        Some(ActionBlockedReasonV1::NoSuchTarget),
                    )),
                    Some(d) => {
                        let is_open_action =
                            matches!(&command.intent, PlayerIntentPayloadV1::Open { .. });
                        let possible = if is_open_action {
                            d.can_open
                        } else {
                            d.can_close
                        };
                        if possible {
                            Ok(make_status(true, None))
                        } else {
                            Ok(make_status(false, Some(ActionBlockedReasonV1::ClosedDoor)))
                        }
                    }
                }
            }
            PlayerIntentPayloadV1::ShowSack
            | PlayerIntentPayloadV1::Wait
            | PlayerIntentPayloadV1::Inspect => Ok(make_status(true, None)),
            PlayerIntentPayloadV1::PromoteClass { target_class_id } => {
                match self.promotion_plan(player_index, target_class_id) {
                    Ok(_) => Ok(make_status(true, None)),
                    Err(error) => Ok(make_status(false, Some(error.reason()))),
                }
            }
            PlayerIntentPayloadV1::Train {
                service_id,
                offered_gold,
            } => match self.training_plan(player_index, service_id, *offered_gold) {
                Ok(_) => Ok(make_status(true, None)),
                Err(error) => Ok(make_status(false, Some(error.reason()))),
            },
            PlayerIntentPayloadV1::Critique {
                service_id,
                track_id,
            } => match self.critique_plan(player_index, service_id, track_id) {
                Ok(_) => Ok(make_status(true, None)),
                Err(error) => Ok(make_status(false, Some(error.reason()))),
            },
            PlayerIntentPayloadV1::LearnSpell { spell_id } => {
                match self.validate_learn_spell_command(player_index, spell_id) {
                    Ok(_) => Ok(make_status(true, None)),
                    Err(reason) => Ok(make_status(false, Some(reason))),
                }
            }
            PlayerIntentPayloadV1::CommitServiceTransaction {
                service_id,
                capability_id,
                transaction_id,
                item_instance_id,
            } => match self.generic_service_transaction_plan(
                player_index,
                service_id,
                capability_id,
                transaction_id,
                item_instance_id.as_deref(),
            ) {
                Ok(_) => Ok(make_status(true, None)),
                Err(error) => Ok(make_status(false, Some(error.reason()))),
            },
            PlayerIntentPayloadV1::BuyFromMerchant {
                service_id,
                capability_id,
                item_instance_ids,
            } => match self.merchant_purchase_plan(
                player_index,
                service_id,
                capability_id,
                item_instance_ids,
            ) {
                Ok(_) => Ok(make_status(true, None)),
                Err(error) => Ok(make_status(false, Some(error.reason()))),
            },
            PlayerIntentPayloadV1::SellToMerchant {
                service_id,
                capability_id,
                item_instance_id,
            } => match self.merchant_sale_plan(
                player_index,
                service_id,
                capability_id,
                item_instance_id,
            ) {
                Ok(_) => Ok(make_status(true, None)),
                Err(error) => Ok(make_status(false, Some(error.reason()))),
            },
            PlayerIntentPayloadV1::UseItemService {
                service_id,
                capability_id,
                operation,
                item_instance_id,
            } => match self.item_service_plan(
                player_index,
                service_id,
                capability_id,
                *operation,
                item_instance_id,
            ) {
                Ok(_) => Ok(make_status(true, None)),
                Err(error) => Ok(make_status(false, Some(error.reason()))),
            },
            PlayerIntentPayloadV1::UseRestorationService {
                service_id,
                capability_id,
                operation_id,
                item_instance_id,
                corpse_id,
            } => match self.restoration_transaction_plan(
                player_index,
                service_id,
                capability_id,
                operation_id,
                item_instance_id.as_deref(),
                corpse_id.as_ref(),
            ) {
                Ok(_) => Ok(make_status(true, None)),
                Err(error) => Ok(make_status(false, Some(error.reason()))),
            },
            PlayerIntentPayloadV1::InteractWithNpc {
                npc_actor_id,
                interaction_id,
                item_instance_id,
            } => match self.npc_interaction_transaction_plan(
                player_index,
                npc_actor_id,
                interaction_id,
                item_instance_id.as_deref(),
            ) {
                Ok(_) => Ok(make_status(true, None)),
                Err(error) => Ok(make_status(false, Some(error.reason()))),
            },
            PlayerIntentPayloadV1::CastSpell {
                spell_id,
                target,
                authorization,
            } => {
                match self.validate_direct_spell_command(player_index, spell_id, target.as_ref()) {
                    Ok(mut plan) => {
                        plan.hostility_authorization = Some(*authorization);
                        match self.validate_direct_hostility_authorization(player_index, &plan) {
                            Ok(()) => Ok(make_status(true, None)),
                            Err(error) => Ok(make_status(
                                false,
                                Some(Self::physical_attack_error_reason(&error)),
                            )),
                        }
                    }
                    Err(reason) => Ok(make_status(false, Some(reason))),
                }
            }
            PlayerIntentPayloadV1::WarmSpell { spell_id } => {
                match self.validate_warm_spell_command(player_index, spell_id) {
                    Ok(_) => Ok(make_status(true, None)),
                    Err(reason) => Ok(make_status(false, Some(reason))),
                }
            }
            PlayerIntentPayloadV1::CastWarmedSpell {
                target,
                authorization,
            } => {
                match self.validate_warmed_spell_command_at_time(
                    player_index,
                    target.as_ref(),
                    self.command_validation_time(),
                ) {
                    Ok(mut plan) => {
                        plan.hostility_authorization = Some(*authorization);
                        match self.validate_direct_hostility_authorization(player_index, &plan) {
                            Ok(()) => Ok(make_status(true, None)),
                            Err(error) => Ok(make_status(
                                false,
                                Some(Self::physical_attack_error_reason(&error)),
                            )),
                        }
                    }
                    Err(reason) => Ok(make_status(false, Some(reason))),
                }
            }
            PlayerIntentPayloadV1::FizzleWarmedSpell => {
                let accepted = self.world.actors[player_index].warmed_spell.is_some();
                Ok(make_status(
                    accepted,
                    (!accepted).then_some(ActionBlockedReasonV1::NoWarmedSpell),
                ))
            }
            PlayerIntentPayloadV1::ClearSelfDefense {
                attacker_character_id,
            } => {
                let mut candidate = self.clone();
                match candidate.clear_self_defense(
                    player_index,
                    attacker_character_id,
                    &mut Vec::new(),
                ) {
                    Ok(()) => Ok(make_status(true, None)),
                    Err(_) => Ok(make_status(
                        false,
                        Some(ActionBlockedReasonV1::InvalidTarget),
                    )),
                }
            }
            PlayerIntentPayloadV1::Rest => Ok(make_status(true, None)),
        }
    }

    /// Convert a typed player command into a `PlayerIntent`.
    /// Validates contract version and actor_id.
    pub fn command_to_actor_intent(
        &self,
        command: &PlayerCommandV1,
    ) -> Result<PlayerIntent, StepError> {
        if command.contract_version != crate::view::COMMAND_CONTRACT_VERSION {
            return Err(StepError::new(format!(
                "command contract version {} != expected {}",
                command.contract_version,
                crate::view::COMMAND_CONTRACT_VERSION
            )));
        }
        let status = self.validate_actor_command(command)?;
        if !status.accepted {
            return Err(StepError::new(Self::command_blocked_reason_code(
                status
                    .blocked_reason
                    .expect("rejected command should carry a blocked reason"),
            )));
        }
        match &command.intent {
            PlayerIntentPayloadV1::MovePath { path } => Ok(PlayerIntent::MovePath(path.clone())),
            PlayerIntentPayloadV1::Traverse { kind } => Ok(PlayerIntent::Traverse(*kind)),
            PlayerIntentPayloadV1::Hide => Ok(PlayerIntent::Hide),
            PlayerIntentPayloadV1::Nock => Ok(PlayerIntent::Nock),
            PlayerIntentPayloadV1::UnloadBow => Ok(PlayerIntent::UnloadBow),
            PlayerIntentPayloadV1::PhysicalAttack {
                mode,
                target_actor_id,
                authorization,
            } => Ok(PlayerIntent::PhysicalAttack {
                mode: *mode,
                target_actor_id: target_actor_id.clone(),
                authorization: *authorization,
            }),
            PlayerIntentPayloadV1::SearchCorpse { corpse_id } => {
                Ok(PlayerIntent::SearchCorpse(corpse_id.clone()))
            }
            PlayerIntentPayloadV1::MoveItem {
                item_instance_id,
                destination,
            } => Ok(PlayerIntent::MoveItem {
                item_instance_id: item_instance_id.clone(),
                destination: destination.clone(),
            }),
            PlayerIntentPayloadV1::MoveGold {
                source,
                destination,
                quantity,
            } => Ok(PlayerIntent::MoveGold {
                source: source.clone(),
                destination: destination.clone(),
                quantity: quantity.clone(),
            }),
            PlayerIntentPayloadV1::DepositBankGold {
                service_id,
                capability_id,
                gold_pile_id,
            } => Ok(PlayerIntent::DepositBankGold {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                gold_pile_id: gold_pile_id.clone(),
            }),
            PlayerIntentPayloadV1::WithdrawBankGold {
                service_id,
                capability_id,
                amount,
            } => Ok(PlayerIntent::WithdrawBankGold {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                amount: *amount,
            }),
            PlayerIntentPayloadV1::DepositLockerItem {
                service_id,
                capability_id,
                item_instance_id,
            } => Ok(PlayerIntent::DepositLockerItem {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                item_instance_id: item_instance_id.clone(),
            }),
            PlayerIntentPayloadV1::WithdrawLockerItem {
                service_id,
                capability_id,
                item_instance_id,
                destination,
            } => Ok(PlayerIntent::WithdrawLockerItem {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                item_instance_id: item_instance_id.clone(),
                destination: *destination,
            }),
            PlayerIntentPayloadV1::OfferItem {
                recipient_character_id,
                item_instance_id,
            } => Ok(PlayerIntent::OfferItem {
                recipient_character_id: recipient_character_id.clone(),
                item_instance_id: item_instance_id.clone(),
            }),
            PlayerIntentPayloadV1::AcceptItemOffer {
                item_instance_id,
                destination,
            } => Ok(PlayerIntent::AcceptItemOffer {
                item_instance_id: item_instance_id.clone(),
                destination: *destination,
            }),
            PlayerIntentPayloadV1::RefuseItemOffer { item_instance_id } => {
                Ok(PlayerIntent::RefuseItemOffer {
                    item_instance_id: item_instance_id.clone(),
                })
            }
            PlayerIntentPayloadV1::WithdrawItemOffer { item_instance_id } => {
                Ok(PlayerIntent::WithdrawItemOffer {
                    item_instance_id: item_instance_id.clone(),
                })
            }
            PlayerIntentPayloadV1::Drink { item_instance_id } => {
                Ok(PlayerIntent::Drink(item_instance_id.clone()))
            }
            PlayerIntentPayloadV1::Open { direction } => Ok(PlayerIntent::Open(*direction)),
            PlayerIntentPayloadV1::Close { direction } => Ok(PlayerIntent::Close(*direction)),
            PlayerIntentPayloadV1::ShowSack => Ok(PlayerIntent::ShowSack),
            PlayerIntentPayloadV1::Wait => Ok(PlayerIntent::Wait),
            PlayerIntentPayloadV1::Inspect => Ok(PlayerIntent::Inspect),
            PlayerIntentPayloadV1::Train {
                service_id,
                offered_gold,
            } => Ok(PlayerIntent::Train {
                service_id: service_id.clone(),
                offered_gold: *offered_gold,
            }),
            PlayerIntentPayloadV1::Critique {
                service_id,
                track_id,
            } => Ok(PlayerIntent::Critique {
                service_id: service_id.clone(),
                track_id: track_id.clone(),
            }),
            PlayerIntentPayloadV1::PromoteClass { target_class_id } => {
                Ok(PlayerIntent::PromoteClass(target_class_id.clone()))
            }
            PlayerIntentPayloadV1::LearnSpell { spell_id } => {
                Ok(PlayerIntent::LearnSpell(spell_id.clone()))
            }
            PlayerIntentPayloadV1::CommitServiceTransaction {
                service_id,
                capability_id,
                transaction_id,
                item_instance_id,
            } => Ok(PlayerIntent::CommitServiceTransaction {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                transaction_id: transaction_id.clone(),
                item_instance_id: item_instance_id.clone(),
            }),
            PlayerIntentPayloadV1::BuyFromMerchant {
                service_id,
                capability_id,
                item_instance_ids,
            } => Ok(PlayerIntent::BuyFromMerchant {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                item_instance_ids: item_instance_ids.clone(),
            }),
            PlayerIntentPayloadV1::SellToMerchant {
                service_id,
                capability_id,
                item_instance_id,
            } => Ok(PlayerIntent::SellToMerchant {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                item_instance_id: item_instance_id.clone(),
            }),
            PlayerIntentPayloadV1::UseItemService {
                service_id,
                capability_id,
                operation,
                item_instance_id,
            } => Ok(PlayerIntent::UseItemService {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                operation: *operation,
                item_instance_id: item_instance_id.clone(),
            }),
            PlayerIntentPayloadV1::UseRestorationService {
                service_id,
                capability_id,
                operation_id,
                item_instance_id,
                corpse_id,
            } => Ok(PlayerIntent::UseRestorationService {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                operation_id: operation_id.clone(),
                item_instance_id: item_instance_id.clone(),
                corpse_id: corpse_id.clone(),
            }),
            PlayerIntentPayloadV1::InteractWithNpc {
                npc_actor_id,
                interaction_id,
                item_instance_id,
            } => Ok(PlayerIntent::InteractWithNpc {
                npc_actor_id: npc_actor_id.clone(),
                interaction_id: interaction_id.clone(),
                item_instance_id: item_instance_id.clone(),
            }),
            PlayerIntentPayloadV1::WarmSpell { spell_id } => Ok(PlayerIntent::WarmSpell {
                spell_id: spell_id.clone(),
            }),
            PlayerIntentPayloadV1::CastWarmedSpell {
                target,
                authorization,
            } => Ok(PlayerIntent::CastWarmedSpell {
                target: target.clone(),
                authorization: *authorization,
            }),
            PlayerIntentPayloadV1::FizzleWarmedSpell => Ok(PlayerIntent::FizzleWarmedSpell),
            PlayerIntentPayloadV1::Rest => Ok(PlayerIntent::Rest),
            PlayerIntentPayloadV1::CastSpell {
                spell_id,
                target,
                authorization,
            } => Ok(PlayerIntent::CastSpell {
                spell_id: spell_id.clone(),
                target: target.clone(),
                authorization: *authorization,
            }),
            PlayerIntentPayloadV1::ClearSelfDefense {
                attacker_character_id,
            } => Ok(PlayerIntent::ClearSelfDefense {
                attacker_character_id: attacker_character_id.clone(),
            }),
        }
    }
}
