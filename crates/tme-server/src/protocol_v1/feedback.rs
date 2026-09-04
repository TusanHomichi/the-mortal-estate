use super::*;

pub(super) fn feedback_actor(
    value: &rules::ObserverFeedbackActorV1,
) -> Result<wire::FeedbackActor, wire::ProtocolError> {
    Ok(wire::FeedbackActor {
        actor_id: actor_id(&value.actor_id)?,
        name: label(&value.name)?,
        kind: actor_kind(value.kind),
    })
}

pub(super) fn feedback_wound(value: rules::WoundState) -> wire::FeedbackWoundState {
    match value {
        rules::WoundState::Unhurt => wire::FeedbackWoundState::Unhurt,
        rules::WoundState::Wounded => wire::FeedbackWoundState::Wounded,
        rules::WoundState::BadlyWounded => wire::FeedbackWoundState::BadlyWounded,
        rules::WoundState::NearDeath => wire::FeedbackWoundState::NearDeath,
        rules::WoundState::Dead => wire::FeedbackWoundState::Dead,
    }
}

pub(super) fn feedback_effect_change(
    value: &rules::ObserverEffectChangeV1,
) -> wire::FeedbackEffectChange {
    match value {
        rules::ObserverEffectChangeV1::Applied { remaining_rounds } => {
            wire::FeedbackEffectChange::Applied {
                remaining_rounds: *remaining_rounds,
            }
        }
        rules::ObserverEffectChangeV1::Ticked { remaining_rounds } => {
            wire::FeedbackEffectChange::Ticked {
                remaining_rounds: *remaining_rounds,
            }
        }
        rules::ObserverEffectChangeV1::Expired => wire::FeedbackEffectChange::Expired {},
        rules::ObserverEffectChangeV1::Removed => wire::FeedbackEffectChange::Removed {},
    }
}

pub(super) fn feedback_transaction_source(
    value: &rules::ObserverTransactionSourceV1,
) -> Result<wire::FeedbackTransactionSource, wire::ProtocolError> {
    Ok(match value {
        rules::ObserverTransactionSourceV1::SkillTraining {
            service_id,
            capability_id,
            track_id,
        } => wire::FeedbackTransactionSource::SkillTraining {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            track_id: label(track_id)?,
        },
        rules::ObserverTransactionSourceV1::SpellLearning {
            service_id,
            capability_id,
            spell_id,
        } => wire::FeedbackTransactionSource::SpellLearning {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            spell_id: label(spell_id)?,
        },
        rules::ObserverTransactionSourceV1::ClassPromotion {
            service_id,
            capability_id,
            transaction_id,
            target_class_id,
        } => wire::FeedbackTransactionSource::ClassPromotion {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            transaction_id: label(transaction_id)?,
            target_class_id: label(target_class_id)?,
        },
        rules::ObserverTransactionSourceV1::ServiceTransaction {
            service_id,
            capability_id,
            transaction_id,
        } => wire::FeedbackTransactionSource::ServiceTransaction {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            transaction_id: label(transaction_id)?,
        },
        rules::ObserverTransactionSourceV1::MerchantPurchase {
            service_id,
            capability_id,
            item_instance_ids,
        } => wire::FeedbackTransactionSource::MerchantPurchase {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            item_instance_ids: item_instance_ids
                .iter()
                .map(wire::ItemInstanceId::new)
                .collect::<Result<_, _>>()?,
        },
        rules::ObserverTransactionSourceV1::MerchantSale {
            service_id,
            capability_id,
            item_instance_id,
        } => wire::FeedbackTransactionSource::MerchantSale {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
        },
        rules::ObserverTransactionSourceV1::ItemService {
            service_id,
            capability_id,
            operation,
            item_instance_id,
        } => wire::FeedbackTransactionSource::ItemService {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            operation: item_service_operation(*operation),
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
        },
        rules::ObserverTransactionSourceV1::RestorationService {
            service_id,
            capability_id,
            operation_id,
            corpse_id,
        } => wire::FeedbackTransactionSource::RestorationService {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            operation_id: label(operation_id)?,
            corpse_id: corpse_id
                .as_ref()
                .map(|id| wire::CorpseId::new(id.as_str()))
                .transpose()?,
        },
        rules::ObserverTransactionSourceV1::NpcInteraction {
            npc_actor_id,
            interaction_id,
        } => wire::FeedbackTransactionSource::NpcInteraction {
            npc_actor_id: actor_id(npc_actor_id)?,
            interaction_id: label(interaction_id)?,
        },
        rules::ObserverTransactionSourceV1::BankDeposit {
            service_id,
            capability_id,
            bank_id,
            gold_pile_id,
        } => wire::FeedbackTransactionSource::BankDeposit {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            bank_id: label(bank_id)?,
            gold_pile_id: label(gold_pile_id.as_str())?,
        },
        rules::ObserverTransactionSourceV1::BankWithdrawal {
            service_id,
            capability_id,
            bank_id,
            amount,
        } => wire::FeedbackTransactionSource::BankWithdrawal {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            bank_id: label(bank_id)?,
            amount: wire::DecimalI64::new(*amount),
        },
    })
}

pub(super) fn feedback_transaction_cost(
    value: &rules::ObserverTransactionCostV1,
) -> Result<wire::FeedbackTransactionCost, wire::ProtocolError> {
    Ok(match value {
        rules::ObserverTransactionCostV1::CarriedGold {
            amount,
            position,
            before,
            after,
        } => wire::FeedbackTransactionCost::CarriedGold {
            amount: wire::DecimalI64::new(*amount),
            position: gold_position(*position),
            before: wire::DecimalI64::new(*before),
            after: wire::DecimalI64::new(*after),
        },
        rules::ObserverTransactionCostV1::GroundGoldPile {
            gold_pile_id,
            amount,
        } => wire::FeedbackTransactionCost::GroundGoldPile {
            gold_pile_id: label(gold_pile_id.as_str())?,
            amount: wire::DecimalI64::new(*amount),
        },
        rules::ObserverTransactionCostV1::BankBalance {
            bank_id,
            amount,
            before,
            after,
        } => wire::FeedbackTransactionCost::BankBalance {
            bank_id: label(bank_id)?,
            amount: wire::DecimalI64::new(*amount),
            before: wire::DecimalI64::new(*before),
            after: wire::DecimalI64::new(*after),
        },
        rules::ObserverTransactionCostV1::SelectedCarriedItem {
            item_instance_id,
            item_definition_id,
            consumed_quantity,
            remaining_quantity,
        } => wire::FeedbackTransactionCost::SelectedCarriedItem {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            item_definition_id: label(item_definition_id)?,
            consumed_quantity: *consumed_quantity,
            remaining_quantity: *remaining_quantity,
        },
        rules::ObserverTransactionCostV1::MerchantItem {
            item_instance_id,
            item_definition_id,
            quantity,
            pawn_listing_price_gold,
        } => wire::FeedbackTransactionCost::MerchantItem {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            item_definition_id: label(item_definition_id)?,
            quantity: *quantity,
            pawn_listing_price_gold: wire::DecimalI64::new(*pawn_listing_price_gold),
        },
    })
}

pub(super) fn feedback_transaction_reward(
    value: &rules::ObserverTransactionRewardV1,
) -> Result<wire::FeedbackTransactionReward, wire::ProtocolError> {
    Ok(match value {
        rules::ObserverTransactionRewardV1::LearningRate {
            track_id,
            before,
            after,
        } => wire::FeedbackTransactionReward::LearningRate {
            track_id: label(track_id)?,
            before: wire::DecimalU64::new(*before),
            after: wire::DecimalU64::new(*after),
        },
        rules::ObserverTransactionRewardV1::Experience { amount, total_xp } => {
            wire::FeedbackTransactionReward::Experience {
                amount: *amount,
                total_xp: wire::DecimalI64::new(*total_xp),
            }
        }
        rules::ObserverTransactionRewardV1::Item {
            item_instance_id,
            item_definition_id,
            position,
            quantity,
        } => wire::FeedbackTransactionReward::Item {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            item_definition_id: label(item_definition_id)?,
            position: carried_position(*position),
            quantity: *quantity,
        },
        rules::ObserverTransactionRewardV1::Class {
            from_class_id,
            from_class_display,
            to_class_id,
            to_class_display,
        } => wire::FeedbackTransactionReward::Class {
            from_class_id: label(from_class_id)?,
            from_class_display: label(from_class_display)?,
            to_class_id: label(to_class_id)?,
            to_class_display: label(to_class_display)?,
        },
        rules::ObserverTransactionRewardV1::Spell {
            spell_id,
            learned_at_level,
        } => wire::FeedbackTransactionReward::Spell {
            spell_id: label(spell_id)?,
            learned_at_level: *learned_at_level,
        },
        rules::ObserverTransactionRewardV1::CarriedGold {
            amount,
            position,
            before,
            after,
        } => wire::FeedbackTransactionReward::CarriedGold {
            amount: wire::DecimalI64::new(*amount),
            position: gold_position(*position),
            before: wire::DecimalI64::new(*before),
            after: wire::DecimalI64::new(*after),
        },
        rules::ObserverTransactionRewardV1::BankBalance {
            bank_id,
            amount,
            before,
            after,
        } => wire::FeedbackTransactionReward::BankBalance {
            bank_id: label(bank_id)?,
            amount: wire::DecimalI64::new(*amount),
            before: wire::DecimalI64::new(*before),
            after: wire::DecimalI64::new(*after),
        },
        rules::ObserverTransactionRewardV1::GroundGoldPile {
            gold_pile_id,
            amount,
        } => wire::FeedbackTransactionReward::GroundGoldPile {
            gold_pile_id: label(gold_pile_id.as_str())?,
            amount: wire::DecimalI64::new(*amount),
        },
        rules::ObserverTransactionRewardV1::MerchantItem {
            item_instance_id,
            item_definition_id,
            quantity,
            listing_price_gold,
        } => wire::FeedbackTransactionReward::MerchantItem {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            item_definition_id: label(item_definition_id)?,
            quantity: *quantity,
            listing_price_gold: wire::DecimalI64::new(*listing_price_gold),
        },
        rules::ObserverTransactionRewardV1::ItemAppraised {
            item_instance_id,
            item_definition_id,
            unit_value_gold,
            total_value_gold,
        } => wire::FeedbackTransactionReward::ItemAppraised {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            item_definition_id: label(item_definition_id)?,
            unit_value_gold: wire::DecimalU64::new(*unit_value_gold),
            total_value_gold: wire::DecimalU64::new(*total_value_gold),
        },
        rules::ObserverTransactionRewardV1::ItemIdentified {
            item_instance_id,
            item_definition_id,
        } => wire::FeedbackTransactionReward::ItemIdentified {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            item_definition_id: label(item_definition_id)?,
        },
        rules::ObserverTransactionRewardV1::ItemEnchanted {
            item_instance_id,
            item_definition_id,
            enchantment_instance_id,
            combat_add_rating_bonus,
            tags,
            remaining_rounds,
        } => wire::FeedbackTransactionReward::ItemEnchanted {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            item_definition_id: label(item_definition_id)?,
            enchantment_instance_id: label(enchantment_instance_id)?,
            combat_add_rating_bonus: *combat_add_rating_bonus,
            tags: tags
                .iter()
                .map(|tag| label(tag))
                .collect::<Result<_, _>>()?,
            remaining_rounds: *remaining_rounds,
        },
        rules::ObserverTransactionRewardV1::ResourceRestored {
            resource,
            before,
            after,
            maximum,
        } => wire::FeedbackTransactionReward::ResourceRestored {
            resource: resource_kind(*resource),
            before: *before,
            after: *after,
            maximum: *maximum,
        },
        rules::ObserverTransactionRewardV1::StatusCured {
            status,
            removed_count,
        } => wire::FeedbackTransactionReward::StatusCured {
            status: restoration_status(*status),
            removed_count: *removed_count,
        },
        rules::ObserverTransactionRewardV1::PriestResurrection {
            corpse_id,
            method,
            current_hp,
            current_stamina,
        } => wire::FeedbackTransactionReward::PriestResurrection {
            corpse_id: wire::CorpseId::new(corpse_id.as_str())?,
            method: feedback_resurrection_method(*method),
            current_hp: *current_hp,
            current_stamina: *current_stamina,
        },
        rules::ObserverTransactionRewardV1::NpcInteraction {
            npc_actor_id,
            interaction_id,
            outcome,
        } => wire::FeedbackTransactionReward::NpcInteraction {
            npc_actor_id: actor_id(npc_actor_id)?,
            interaction_id: label(interaction_id)?,
            outcome: npc_interaction_outcome(outcome)?,
        },
        rules::ObserverTransactionRewardV1::QuestStage {
            quest_id,
            before_stage_id,
            after_stage_id,
        } => wire::FeedbackTransactionReward::QuestStage {
            quest_id: label(quest_id)?,
            before_stage_id: before_stage_id.as_deref().map(label).transpose()?,
            after_stage_id: label(after_stage_id)?,
        },
    })
}

pub(super) fn feedback_cue(
    value: &rules::ObserverFeedbackCueV1,
) -> Result<wire::FeedbackCue, wire::ProtocolError> {
    Ok(match value {
        rules::ObserverFeedbackCueV1::PhysicalCombat {
            source,
            target,
            location,
            mode,
            outcome,
        } => wire::FeedbackCue::PhysicalCombat {
            source: source.as_ref().map(feedback_actor).transpose()?,
            target: feedback_actor(target)?,
            location: location.as_ref().map(position).transpose()?,
            mode: physical_mode(*mode),
            outcome: match outcome {
                rules::ObserverPhysicalOutcomeV1::Hit {
                    damage,
                    armor_reduction,
                    wound_before,
                    wound_after,
                    target_hp,
                } => wire::FeedbackPhysicalOutcome::Hit {
                    damage: *damage,
                    armor_reduction: *armor_reduction,
                    wound_before: feedback_wound(*wound_before),
                    wound_after: feedback_wound(*wound_after),
                    target_hp: *target_hp,
                },
                rules::ObserverPhysicalOutcomeV1::Missed => {
                    wire::FeedbackPhysicalOutcome::Missed {}
                }
                rules::ObserverPhysicalOutcomeV1::Blocked => {
                    wire::FeedbackPhysicalOutcome::Blocked {}
                }
                rules::ObserverPhysicalOutcomeV1::NoSight => {
                    wire::FeedbackPhysicalOutcome::NoSight {}
                }
                rules::ObserverPhysicalOutcomeV1::NotReady {
                    current_time,
                    ready_at,
                } => wire::FeedbackPhysicalOutcome::NotReady {
                    current_time: wire::DecimalU64::new(current_time.as_millis()),
                    ready_at: wire::DecimalU64::new(ready_at.as_millis()),
                },
            },
        },
        rules::ObserverFeedbackCueV1::WeaponFumbled {
            actor,
            mode,
            result,
        } => wire::FeedbackCue::WeaponFumbled {
            actor: feedback_actor(actor)?,
            mode: physical_mode(*mode),
            result: match result {
                rules::WeaponFumbleResult::Dropped => wire::FeedbackWeaponFumbleResult::Dropped,
                rules::WeaponFumbleResult::BowUnnocked => {
                    wire::FeedbackWeaponFumbleResult::BowUnnocked
                }
            },
        },
        rules::ObserverFeedbackCueV1::SpellLifecycle {
            actor,
            spell_id,
            spell_name,
            state,
        } => wire::FeedbackCue::SpellLifecycle {
            actor: feedback_actor(actor)?,
            spell_id: label(spell_id)?,
            spell_name: label(spell_name)?,
            state: match state {
                rules::ObserverSpellLifecycleStateV1::Warmed {
                    warmed_at,
                    ready_at,
                } => wire::FeedbackSpellLifecycleState::Warmed {
                    warmed_at: wire::DecimalU64::new(warmed_at.as_millis()),
                    ready_at: wire::DecimalU64::new(ready_at.as_millis()),
                },
                rules::ObserverSpellLifecycleStateV1::Ready { ready_at } => {
                    wire::FeedbackSpellLifecycleState::Ready {
                        ready_at: wire::DecimalU64::new(ready_at.as_millis()),
                    }
                }
                rules::ObserverSpellLifecycleStateV1::Cast {
                    mp_cost,
                    stamina_cost,
                } => wire::FeedbackSpellLifecycleState::Cast {
                    mp_cost: *mp_cost,
                    stamina_cost: *stamina_cost,
                },
                rules::ObserverSpellLifecycleStateV1::Fizzled { reason } => {
                    wire::FeedbackSpellLifecycleState::Fizzled {
                        reason: match reason {
                            rules::ObserverSpellFizzleReasonV1::Replaced => {
                                wire::FeedbackSpellFizzleReason::Replaced
                            }
                            rules::ObserverSpellFizzleReasonV1::Canceled => {
                                wire::FeedbackSpellFizzleReason::Canceled
                            }
                            rules::ObserverSpellFizzleReasonV1::Rest => {
                                wire::FeedbackSpellFizzleReason::Rest
                            }
                            rules::ObserverSpellFizzleReasonV1::HealingBalm => {
                                wire::FeedbackSpellFizzleReason::HealingBalm
                            }
                            rules::ObserverSpellFizzleReasonV1::Damage => {
                                wire::FeedbackSpellFizzleReason::Damage
                            }
                            rules::ObserverSpellFizzleReasonV1::Defeat => {
                                wire::FeedbackSpellFizzleReason::Defeat
                            }
                        },
                    }
                }
                rules::ObserverSpellLifecycleStateV1::Failed {
                    reason,
                    mp_cost,
                    stamina_cost,
                } => wire::FeedbackSpellLifecycleState::Failed {
                    reason: match reason {
                        rules::ObserverSpellFailureReasonV1::InvalidPath => {
                            wire::FeedbackSpellFailureReason::InvalidPath
                        }
                        rules::ObserverSpellFailureReasonV1::AboveSkillAttempt => {
                            wire::FeedbackSpellFailureReason::AboveSkillAttempt
                        }
                    },
                    mp_cost: *mp_cost,
                    stamina_cost: *stamina_cost,
                },
            },
        },
        rules::ObserverFeedbackCueV1::SpellImpact {
            source,
            spell_id,
            spell_name,
            target,
            location,
            outcome,
        } => wire::FeedbackCue::SpellImpact {
            source: source.as_ref().map(feedback_actor).transpose()?,
            spell_id: label(spell_id)?,
            spell_name: label(spell_name)?,
            target: feedback_actor(target)?,
            location: position(location)?,
            outcome: match outcome {
                rules::ObserverSpellImpactOutcomeV1::Damaged { damage, target_hp } => {
                    wire::FeedbackSpellImpactOutcome::Damaged {
                        damage: *damage,
                        target_hp: *target_hp,
                    }
                }
                rules::ObserverSpellImpactOutcomeV1::Healed { amount, target_hp } => {
                    wire::FeedbackSpellImpactOutcome::Healed {
                        amount: *amount,
                        target_hp: *target_hp,
                    }
                }
            },
        },
        rules::ObserverFeedbackCueV1::ActorEffect {
            actor,
            location,
            effect_id,
            effect_kind,
            change,
        } => wire::FeedbackCue::ActorEffect {
            actor: feedback_actor(actor)?,
            location: position(location)?,
            effect_id: label(effect_id)?,
            effect_kind: label(effect_kind)?,
            change: feedback_effect_change(change),
        },
        rules::ObserverFeedbackCueV1::TileEffect {
            location,
            effect_id,
            effect_kind,
            change,
        } => wire::FeedbackCue::TileEffect {
            location: position(location)?,
            effect_id: label(effect_id)?,
            effect_kind: label(effect_kind)?,
            change: feedback_effect_change(change),
        },
        rules::ObserverFeedbackCueV1::EffectDamage {
            actor,
            location,
            effect_id,
            effect_kind,
            damage,
            actor_hp,
        } => wire::FeedbackCue::EffectDamage {
            actor: feedback_actor(actor)?,
            location: position(location)?,
            effect_id: label(effect_id)?,
            effect_kind: label(effect_kind)?,
            damage: *damage,
            actor_hp: *actor_hp,
        },
        rules::ObserverFeedbackCueV1::Resource {
            actor,
            resource,
            reason,
            amount,
            current,
            maximum,
        } => wire::FeedbackCue::Resource {
            actor: feedback_actor(actor)?,
            resource: resource_kind(*resource),
            reason: match reason {
                rules::ObserverResourceReasonV1::MovementSpend => {
                    wire::FeedbackResourceReason::MovementSpend
                }
                rules::ObserverResourceReasonV1::PhysicalSpend => {
                    wire::FeedbackResourceReason::PhysicalSpend
                }
                rules::ObserverResourceReasonV1::SpellCost => {
                    wire::FeedbackResourceReason::SpellCost
                }
                rules::ObserverResourceReasonV1::Regenerated => {
                    wire::FeedbackResourceReason::Regenerated
                }
                rules::ObserverResourceReasonV1::Restored => wire::FeedbackResourceReason::Restored,
                rules::ObserverResourceReasonV1::Balm => wire::FeedbackResourceReason::Balm,
            },
            amount: *amount,
            current: *current,
            maximum: *maximum,
        },
        rules::ObserverFeedbackCueV1::Transaction {
            actor,
            source,
            costs,
            rewards,
        } => wire::FeedbackCue::Transaction {
            actor: feedback_actor(actor)?,
            source: feedback_transaction_source(source)?,
            costs: costs
                .iter()
                .map(feedback_transaction_cost)
                .collect::<Result<_, _>>()?,
            rewards: rewards
                .iter()
                .map(feedback_transaction_reward)
                .collect::<Result<_, _>>()?,
        },
        rules::ObserverFeedbackCueV1::Quest {
            quest_id,
            quest_title,
            before_stage_id,
            after_stage_id,
            after_stage_label,
            terminal,
        } => wire::FeedbackCue::Quest {
            quest_id: label(quest_id)?,
            quest_title: label(quest_title)?,
            before_stage_id: before_stage_id.as_deref().map(label).transpose()?,
            after_stage_id: label(after_stage_id)?,
            after_stage_label: label(after_stage_label)?,
            terminal: *terminal,
        },
        rules::ObserverFeedbackCueV1::NpcMessage {
            npc_actor_id,
            npc_name,
            interaction_id,
            response,
        } => wire::FeedbackCue::NpcMessage {
            npc_actor_id: actor_id(npc_actor_id)?,
            npc_name: label(npc_name)?,
            interaction_id: label(interaction_id)?,
            response: wire::FeedbackText::new(response)?,
        },
        rules::ObserverFeedbackCueV1::Defeat {
            actor,
            location,
            cause,
            credited_source,
        } => wire::FeedbackCue::Defeat {
            actor: feedback_actor(actor)?,
            location: position(location)?,
            cause: match cause {
                rules::DeathCause::Physical => wire::FeedbackDeathCause::Physical,
                rules::DeathCause::Poison => wire::FeedbackDeathCause::Poison,
                rules::DeathCause::Fire => wire::FeedbackDeathCause::Fire,
                rules::DeathCause::OtherMagic => wire::FeedbackDeathCause::OtherMagic,
                rules::DeathCause::Hazard => wire::FeedbackDeathCause::Hazard,
            },
            credited_source: credited_source.as_ref().map(feedback_actor).transpose()?,
        },
        rules::ObserverFeedbackCueV1::Corpse {
            corpse_id,
            origin,
            location,
            change,
        } => wire::FeedbackCue::Corpse {
            corpse_id: wire::CorpseId::new(corpse_id.as_str())?,
            origin: origin.as_ref().map(feedback_actor).transpose()?,
            location: position(location)?,
            change: match change {
                rules::ObserverCorpseChangeV1::Created => wire::FeedbackCorpseChange::Created {},
                rules::ObserverCorpseChangeV1::Removed { method } => {
                    wire::FeedbackCorpseChange::Removed {
                        method: feedback_resurrection_method(*method),
                    }
                }
            },
        },
        rules::ObserverFeedbackCueV1::LifeState { actor, from, to } => {
            wire::FeedbackCue::LifeState {
                actor: feedback_actor(actor)?,
                from: life_state(*from),
                to: life_state(*to),
            }
        }
        rules::ObserverFeedbackCueV1::Resurrection {
            actor,
            corpse_id,
            method,
            destination,
            current_hp,
            current_stamina,
        } => wire::FeedbackCue::Resurrection {
            actor: feedback_actor(actor)?,
            corpse_id: corpse_id
                .as_ref()
                .map(|id| wire::CorpseId::new(id.as_str()))
                .transpose()?,
            method: feedback_resurrection_method(*method),
            destination: position(destination)?,
            current_hp: *current_hp,
            current_stamina: *current_stamina,
        },
    })
}
