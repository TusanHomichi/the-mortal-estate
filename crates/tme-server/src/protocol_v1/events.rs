use super::*;

pub fn events(
    values: &[rules::ObservedEventV1],
) -> Result<Vec<wire::ObservedEvent>, wire::ProtocolError> {
    values
        .iter()
        .map(|value| match value {
            rules::ObservedEventV1::ActorMoved {
                actor_id: moved_actor_id,
                from,
                to,
                navigation: moved_navigation,
            } => Ok(wire::ObservedEvent::ActorMoved {
                actor_id: actor_id(moved_actor_id)?,
                from: position(from)?,
                to: position(to)?,
                navigation: navigation(*moved_navigation),
            }),
            rules::ObservedEventV1::Inspected {
                location,
                tile,
                tile_move_cost,
                exits,
                nearby_actors,
                ground_items,
            } => Ok(wire::ObservedEvent::Inspected {
                location: position(location)?,
                tile: label(tile)?,
                tile_move_cost: *tile_move_cost,
                exits: exits
                    .iter()
                    .map(|exit| {
                        Ok(wire::ObserverInspectExit {
                            direction: direction(exit.direction),
                            location: position(&exit.location)?,
                            terrain: exit.terrain.as_deref().map(label).transpose()?,
                            move_cost: exit.move_cost,
                            status: match &exit.status {
                                rules::ObserverInspectExitStatusV1::Walkable => {
                                    wire::ObserverInspectExitStatus::Walkable
                                }
                                rules::ObserverInspectExitStatusV1::BlockedTerrain => {
                                    wire::ObserverInspectExitStatus::BlockedTerrain
                                }
                                rules::ObserverInspectExitStatusV1::Door { open, target } => {
                                    wire::ObserverInspectExitStatus::Door {
                                        open: *open,
                                        target: position(target)?,
                                    }
                                }
                                rules::ObserverInspectExitStatusV1::OutOfBounds => {
                                    wire::ObserverInspectExitStatus::OutOfBounds
                                }
                            },
                        })
                    })
                    .collect::<Result<_, wire::ProtocolError>>()?,
                nearby_actors: nearby_actors
                    .iter()
                    .map(|actor| {
                        Ok(wire::ObserverInspectActor {
                            direction: direction(actor.direction),
                            actor_id: actor_id(&actor.actor_id)?,
                            actor: label(&actor.actor)?,
                            kind: match actor.kind {
                                rules::ActorKind::Player => wire::ActorKind::Player,
                                rules::ActorKind::Monster => wire::ActorKind::Monster,
                                rules::ActorKind::Npc => wire::ActorKind::Npc,
                            },
                            location: position(&actor.location)?,
                            hp: actor.hp,
                        })
                    })
                    .collect::<Result<_, wire::ProtocolError>>()?,
                ground_items: ground_items
                    .iter()
                    .map(|item| {
                        Ok(wire::ObserverInspectGroundItem {
                            item: observer_item(&item.item)?,
                            location: position(&item.location)?,
                            direction: item.direction.map(direction),
                        })
                    })
                    .collect::<Result<_, wire::ProtocolError>>()?,
            }),
            rules::ObservedEventV1::GroupChanged { group_id } => {
                Ok(wire::ObservedEvent::GroupChanged {
                    group_id: wire::DecimalU64::new(group_id.value()),
                })
            }
            rules::ObservedEventV1::GroupInvitationChanged { invitation_id } => {
                Ok(wire::ObservedEvent::GroupInvitationChanged {
                    invitation_id: wire::DecimalU64::new(invitation_id.value()),
                })
            }
            rules::ObservedEventV1::GroupPresenceChanged {
                group_id,
                character_id: changed_character_id,
                connected,
            } => Ok(wire::ObservedEvent::GroupPresenceChanged {
                group_id: wire::DecimalU64::new(group_id.value()),
                character_id: character_id(changed_character_id)?,
                connected: *connected,
            }),
            rules::ObservedEventV1::PlayerFollowChanged {
                follower_character_id,
                target_character_id,
            } => Ok(wire::ObservedEvent::PlayerFollowChanged {
                follower_character_id: character_id(follower_character_id)?,
                target_character_id: target_character_id.as_ref().map(character_id).transpose()?,
            }),
            rules::ObservedEventV1::CommunicationPreferencesChanged => {
                Ok(wire::ObservedEvent::CommunicationPreferencesChanged)
            }
            rules::ObservedEventV1::ItemOfferChanged { item_instance_id } => {
                Ok(wire::ObservedEvent::ItemOfferChanged {
                    item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
                })
            }
            rules::ObservedEventV1::DefeatRewardShare {
                character_id: recipient_character_id,
                amount,
            } => Ok(wire::ObservedEvent::DefeatRewardShare {
                character_id: character_id(recipient_character_id)?,
                amount: *amount,
            }),
            rules::ObservedEventV1::Feedback { cue } => Ok(wire::ObservedEvent::Feedback {
                cue: feedback_cue(cue)?,
            }),
        })
        .collect()
}

pub(super) fn path_navigation(value: rules::TransitionKindViewV1) -> wire::NavigationKind {
    match value {
        rules::TransitionKindViewV1::Walk => wire::NavigationKind::Walk,
        rules::TransitionKindViewV1::Swim => wire::NavigationKind::Swim,
        rules::TransitionKindViewV1::Door => wire::NavigationKind::Door,
        rules::TransitionKindViewV1::Stairs { direction } => wire::NavigationKind::Stairs {
            direction: vertical(direction),
        },
        rules::TransitionKindViewV1::Pit => wire::NavigationKind::Pit,
        rules::TransitionKindViewV1::Climb { direction } => wire::NavigationKind::Climb {
            direction: vertical(direction),
        },
        rules::TransitionKindViewV1::Passage => wire::NavigationKind::Passage,
        rules::TransitionKindViewV1::Portal => wire::NavigationKind::Portal,
    }
}

pub fn path_preview(
    value: &rules::PathPreviewV1,
) -> Result<wire::PathPreview, wire::ProtocolError> {
    let preview = wire::PathPreview {
        contract_version: value.contract_version,
        actor_id: actor_id(&value.actor_id)?,
        start: position(&value.start)?,
        pace: match value.pace {
            rules::MovementPace::Walk => wire::MovementPace::Walk,
            rules::MovementPace::Run => wire::MovementPace::Run,
            rules::MovementPace::Sprint => wire::MovementPace::Sprint,
        },
        requested_path: value.requested_path.iter().copied().map(direction).collect(),
        available_path_points: value.available_path_points,
        accepted_steps: wire::DecimalU64::new(value.accepted_steps as u64),
        steps: value
            .steps
            .iter()
            .map(|step| {
                Ok(wire::PathPreviewStep {
                    index: wire::DecimalU64::new(step.index as u64),
                    direction: direction(step.direction),
                    from: position(&step.from)?,
                    attempted: position(&step.attempted)?,
                    opens_door: step.opens_door,
                    terrain_name: step.terrain_name.as_deref().map(label).transpose()?,
                    cost: step.cost,
                    remaining_points_after: step.remaining_points_after,
                    outcome: match &step.outcome {
                        rules::PathPreviewStepOutcomeV1::Moved { kind } => {
                            wire::PathPreviewStepOutcome::Moved {
                                navigation: path_navigation(*kind),
                            }
                        }
                        rules::PathPreviewStepOutcomeV1::Transitioned { kind, to } => {
                            wire::PathPreviewStepOutcome::Transitioned {
                                navigation: path_navigation(*kind),
                                to: position(to)?,
                            }
                        }
                        rules::PathPreviewStepOutcomeV1::Blocked { reason } => {
                            wire::PathPreviewStepOutcome::Blocked {
                                reason: match reason {
                                    rules::PathPreviewBlockedReasonV1::SuppressedByStatus => {
                                        wire::PathPreviewBlockedReason::SuppressedByStatus
                                    }
                                    rules::PathPreviewBlockedReasonV1::OutOfBounds => {
                                        wire::PathPreviewBlockedReason::OutOfBounds
                                    }
                                    rules::PathPreviewBlockedReasonV1::BlockedTerrain => {
                                        wire::PathPreviewBlockedReason::BlockedTerrain
                                    }
                                    rules::PathPreviewBlockedReasonV1::InsufficientMovementPoints => {
                                        wire::PathPreviewBlockedReason::InsufficientMovementPoints
                                    }
                                },
                            }
                        }
                    },
                })
            })
            .collect::<Result<_, wire::ProtocolError>>()?,
        stop_reason: match value.stop_reason {
            rules::MovementStopReason::FullPathAccepted => {
                wire::MovementStopReason::FullPathAccepted
            }
            rules::MovementStopReason::Blocked => wire::MovementStopReason::Blocked,
            rules::MovementStopReason::Transitioned => wire::MovementStopReason::Transitioned,
            rules::MovementStopReason::ZeroStaminaLimit => {
                wire::MovementStopReason::ZeroStaminaLimit
            }
        },
        final_position: position(&value.final_position)?,
        remaining_path_points: value.remaining_path_points,
        burden: wire::Burden {
            item_burden: wire::DecimalU64::new(value.burden.item_burden),
            coin_burden: wire::DecimalU64::new(value.burden.coin_burden),
            total_burden: wire::DecimalU64::new(value.burden.total_burden),
            lightly_loaded_limit: value
                .burden
                .lightly_loaded_limit
                .map(wire::DecimalU64::new),
            moderately_loaded_limit: value
                .burden
                .moderately_loaded_limit
                .map(wire::DecimalU64::new),
            heavily_loaded_limit: value
                .burden
                .heavily_loaded_limit
                .map(wire::DecimalU64::new),
            tier: value.burden.tier.map(|tier| match tier {
                rules::BurdenTier::LightlyLoaded => wire::BurdenTier::LightlyLoaded,
                rules::BurdenTier::ModeratelyLoaded => wire::BurdenTier::ModeratelyLoaded,
                rules::BurdenTier::HeavilyLoaded => wire::BurdenTier::HeavilyLoaded,
                rules::BurdenTier::VeryHeavilyLoaded => wire::BurdenTier::VeryHeavilyLoaded,
            }),
        },
        movement_exertion: match value.movement_exertion {
            rules::MovementExertion::None => wire::MovementExertion::None,
            rules::MovementExertion::Normal => wire::MovementExertion::Normal,
            rules::MovementExertion::Rapid => wire::MovementExertion::Rapid,
        },
        stamina_before: value.stamina_before,
        stamina_cost: value.stamina_cost,
        stamina_after: value.stamina_after,
    };
    preview.validate()?;
    Ok(preview)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RulesIntent {
    Gameplay(rules::PlayerIntent),
    Social(rules::SocialIntent),
}

pub fn intent(value: &wire::Intent) -> RulesIntent {
    match value {
        wire::Intent::MovePath { path } => RulesIntent::Gameplay(rules::PlayerIntent::MovePath(
            path.iter().map(rules_direction).collect(),
        )),
        wire::Intent::Traverse { traversal } => RulesIntent::Gameplay(
            rules::PlayerIntent::Traverse(rules_explicit_traversal(*traversal)),
        ),
        wire::Intent::Open { direction } => {
            RulesIntent::Gameplay(rules::PlayerIntent::Open(rules_direction(direction)))
        }
        wire::Intent::Close { direction } => {
            RulesIntent::Gameplay(rules::PlayerIntent::Close(rules_direction(direction)))
        }
        wire::Intent::Inspect => RulesIntent::Gameplay(rules::PlayerIntent::Inspect),
        wire::Intent::Hide => RulesIntent::Gameplay(rules::PlayerIntent::Hide),
        wire::Intent::ShowSack => RulesIntent::Gameplay(rules::PlayerIntent::ShowSack),
        wire::Intent::Wait => RulesIntent::Gameplay(rules::PlayerIntent::Wait),
        wire::Intent::Rest => RulesIntent::Gameplay(rules::PlayerIntent::Rest),
        wire::Intent::PhysicalAttack {
            mode,
            target_actor_id,
            authorization,
        } => RulesIntent::Gameplay(rules::PlayerIntent::PhysicalAttack {
            mode: rules_physical_mode(*mode),
            target_actor_id: rules_actor_id(target_actor_id),
            authorization: rules_authorization(*authorization),
        }),
        wire::Intent::Nock => RulesIntent::Gameplay(rules::PlayerIntent::Nock),
        wire::Intent::UnloadBow => RulesIntent::Gameplay(rules::PlayerIntent::UnloadBow),
        wire::Intent::WarmSpell { spell_id } => {
            RulesIntent::Gameplay(rules::PlayerIntent::WarmSpell {
                spell_id: spell_id.as_str().to_string(),
            })
        }
        wire::Intent::CastSpell {
            spell_id,
            target,
            authorization,
        } => RulesIntent::Gameplay(rules::PlayerIntent::CastSpell {
            spell_id: spell_id.as_str().to_string(),
            target: target.as_ref().map(rules_spell_target),
            authorization: rules_authorization(*authorization),
        }),
        wire::Intent::CastWarmedSpell {
            target,
            authorization,
        } => RulesIntent::Gameplay(rules::PlayerIntent::CastWarmedSpell {
            target: target.as_ref().map(rules_spell_target),
            authorization: rules_authorization(*authorization),
        }),
        wire::Intent::FizzleWarmedSpell => {
            RulesIntent::Gameplay(rules::PlayerIntent::FizzleWarmedSpell)
        }
        wire::Intent::SearchCorpse { corpse_id } => {
            RulesIntent::Gameplay(rules::PlayerIntent::SearchCorpse(
                rules::CorpseId::parse(corpse_id.as_str()).expect("validated wire corpse ID"),
            ))
        }
        wire::Intent::MoveItem {
            item_instance_id,
            destination,
        } => RulesIntent::Gameplay(rules::PlayerIntent::MoveItem {
            item_instance_id: item_instance_id.as_str().to_string(),
            destination: match destination {
                wire::ItemMoveDestination::GroundHere => rules::ItemMoveDestination::GroundHere,
                wire::ItemMoveDestination::Carried { position } => {
                    rules::ItemMoveDestination::Carried {
                        position: rules_carried_position(*position),
                    }
                }
            },
        }),
        wire::Intent::MoveGold {
            source,
            destination,
            quantity,
        } => RulesIntent::Gameplay(rules::PlayerIntent::MoveGold {
            source: match source {
                wire::GoldMoveSource::Carried { position } => rules::GoldMoveSource::Carried {
                    position: rules_gold_position(*position),
                },
                wire::GoldMoveSource::Ground { gold_pile_id } => rules::GoldMoveSource::Ground {
                    gold_pile_id: rules::GoldPileId::parse(gold_pile_id.as_str())
                        .expect("validated wire gold pile ID"),
                },
            },
            destination: match destination {
                wire::GoldMoveDestination::Carried { position } => {
                    rules::GoldMoveDestination::Carried {
                        position: rules_gold_position(*position),
                    }
                }
                wire::GoldMoveDestination::GroundHere => rules::GoldMoveDestination::GroundHere,
            },
            quantity: match quantity {
                wire::GoldMoveQuantity::All => rules::GoldMoveQuantity::All,
                wire::GoldMoveQuantity::Exact { amount } => rules::GoldMoveQuantity::Exact {
                    amount: amount.get(),
                },
            },
        }),
        wire::Intent::DepositBankGold {
            service_id,
            capability_id,
            gold_pile_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::DepositBankGold {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            gold_pile_id: rules::GoldPileId::parse(gold_pile_id.as_str())
                .expect("validated wire gold pile ID"),
        }),
        wire::Intent::WithdrawBankGold {
            service_id,
            capability_id,
            amount,
        } => RulesIntent::Gameplay(rules::PlayerIntent::WithdrawBankGold {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            amount: amount.get(),
        }),
        wire::Intent::DepositLockerItem {
            service_id,
            capability_id,
            item_instance_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::DepositLockerItem {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            item_instance_id: item_instance_id.as_str().to_string(),
        }),
        wire::Intent::WithdrawLockerItem {
            service_id,
            capability_id,
            item_instance_id,
            destination,
        } => RulesIntent::Gameplay(rules::PlayerIntent::WithdrawLockerItem {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            item_instance_id: item_instance_id.as_str().to_string(),
            destination: rules_carried_position(*destination),
        }),
        wire::Intent::DrinkItem { item_instance_id } => RulesIntent::Gameplay(
            rules::PlayerIntent::Drink(item_instance_id.as_str().to_string()),
        ),
        wire::Intent::Train {
            service_id,
            offered_gold,
        } => RulesIntent::Gameplay(rules::PlayerIntent::Train {
            service_id: service_id.as_str().to_string(),
            offered_gold: offered_gold.get(),
        }),
        wire::Intent::Critique {
            service_id,
            track_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::Critique {
            service_id: service_id.as_str().to_string(),
            track_id: track_id.as_str().to_string(),
        }),
        wire::Intent::PromoteClass { target_class_id } => RulesIntent::Gameplay(
            rules::PlayerIntent::PromoteClass(target_class_id.as_str().to_string()),
        ),
        wire::Intent::LearnSpell { spell_id } => RulesIntent::Gameplay(
            rules::PlayerIntent::LearnSpell(spell_id.as_str().to_string()),
        ),
        wire::Intent::CommitServiceTransaction {
            service_id,
            capability_id,
            transaction_id,
            item_instance_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::CommitServiceTransaction {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            transaction_id: transaction_id.as_str().to_string(),
            item_instance_id: item_instance_id
                .as_ref()
                .map(|value| value.as_str().to_string()),
        }),
        wire::Intent::BuyFromMerchant {
            service_id,
            capability_id,
            item_instance_ids,
        } => RulesIntent::Gameplay(rules::PlayerIntent::BuyFromMerchant {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            item_instance_ids: item_instance_ids
                .iter()
                .map(|value| value.as_str().to_string())
                .collect(),
        }),
        wire::Intent::SellToMerchant {
            service_id,
            capability_id,
            item_instance_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::SellToMerchant {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            item_instance_id: item_instance_id.as_str().to_string(),
        }),
        wire::Intent::UseItemService {
            service_id,
            capability_id,
            operation,
            item_instance_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::UseItemService {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            operation: rules_item_service_operation(*operation),
            item_instance_id: item_instance_id.as_str().to_string(),
        }),
        wire::Intent::UseRestorationService {
            service_id,
            capability_id,
            operation_id,
            item_instance_id,
            corpse_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::UseRestorationService {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            operation_id: operation_id.as_str().to_string(),
            item_instance_id: item_instance_id
                .as_ref()
                .map(|value| value.as_str().to_string()),
            corpse_id: corpse_id.as_ref().map(|value| {
                rules::CorpseId::parse(value.as_str()).expect("validated wire corpse ID")
            }),
        }),
        wire::Intent::InteractWithNpc {
            npc_actor_id,
            interaction_id,
            item_instance_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::InteractWithNpc {
            npc_actor_id: rules_actor_id(npc_actor_id),
            interaction_id: interaction_id.as_str().to_string(),
            item_instance_id: item_instance_id
                .as_ref()
                .map(|value| value.as_str().to_string()),
        }),
        wire::Intent::ClearSelfDefense {
            attacker_character_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::ClearSelfDefense {
            attacker_character_id: rules_character_id(*attacker_character_id),
        }),
        wire::Intent::Invite {
            target_character_id,
        } => RulesIntent::Social(rules::SocialIntent::Invite {
            target_character_id: rules_character_id(*target_character_id),
        }),
        wire::Intent::AcceptInvite { invitation_id } => {
            RulesIntent::Social(rules::SocialIntent::AcceptInvite {
                invitation_id: rules::GroupInviteId::new(invitation_id.get()),
            })
        }
        wire::Intent::DeclineInvite { invitation_id } => {
            RulesIntent::Social(rules::SocialIntent::DeclineInvite {
                invitation_id: rules::GroupInviteId::new(invitation_id.get()),
            })
        }
        wire::Intent::CancelInvite { invitation_id } => {
            RulesIntent::Social(rules::SocialIntent::CancelInvite {
                invitation_id: rules::GroupInviteId::new(invitation_id.get()),
            })
        }
        wire::Intent::LeaveGroup => RulesIntent::Social(rules::SocialIntent::LeaveGroup),
        wire::Intent::RemoveMember {
            member_character_id,
        } => RulesIntent::Social(rules::SocialIntent::RemoveMember {
            member_character_id: rules_character_id(*member_character_id),
        }),
        wire::Intent::DisbandGroup => RulesIntent::Social(rules::SocialIntent::DisbandGroup),
        wire::Intent::TransferLeadership {
            member_character_id,
        } => RulesIntent::Social(rules::SocialIntent::TransferLeadership {
            member_character_id: rules_character_id(*member_character_id),
        }),
        wire::Intent::BeginFollow {
            target_character_id,
        } => RulesIntent::Social(rules::SocialIntent::BeginFollow {
            target_character_id: rules_character_id(*target_character_id),
        }),
        wire::Intent::EndFollow => RulesIntent::Social(rules::SocialIntent::EndFollow),
        wire::Intent::SetPagesEnabled { enabled } => {
            RulesIntent::Social(rules::SocialIntent::SetPagesEnabled { enabled: *enabled })
        }
        wire::Intent::Block {
            target_character_id,
        } => RulesIntent::Social(rules::SocialIntent::Block {
            target_character_id: rules_character_id(*target_character_id),
        }),
        wire::Intent::Unblock {
            target_character_id,
        } => RulesIntent::Social(rules::SocialIntent::Unblock {
            target_character_id: rules_character_id(*target_character_id),
        }),
        wire::Intent::OfferItem {
            recipient_character_id,
            item_instance_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::OfferItem {
            recipient_character_id: rules_character_id(*recipient_character_id),
            item_instance_id: item_instance_id.as_str().to_string(),
        }),
        wire::Intent::AcceptItemOffer {
            item_instance_id,
            destination,
        } => RulesIntent::Gameplay(rules::PlayerIntent::AcceptItemOffer {
            item_instance_id: item_instance_id.as_str().to_string(),
            destination: rules_carried_position(*destination),
        }),
        wire::Intent::RefuseItemOffer { item_instance_id } => {
            RulesIntent::Gameplay(rules::PlayerIntent::RefuseItemOffer {
                item_instance_id: item_instance_id.as_str().to_string(),
            })
        }
        wire::Intent::WithdrawItemOffer { item_instance_id } => {
            RulesIntent::Gameplay(rules::PlayerIntent::WithdrawItemOffer {
                item_instance_id: item_instance_id.as_str().to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(id: &str, name: &str, kind: rules::ActorKind) -> rules::ObserverFeedbackActorV1 {
        rules::ObserverFeedbackActorV1 {
            actor_id: rules::ActorId::from(id),
            name: name.to_string(),
            kind,
        }
    }

    fn location() -> rules::WorldPosition {
        rules::WorldPosition::new("realm_0", "room_0", rules::Coord { x: 1, y: 1 })
    }

    #[test]
    fn protocol_v1_feedback_conversion_preserves_safe_nulls_wide_values_and_recovery() {
        let player = actor("player", "Wayfarer", rules::ActorKind::Player);
        let target = actor("mireling", "Mireling", rules::ActorKind::Monster);
        let values = vec![
            rules::ObservedEventV1::Feedback {
                cue: rules::ObserverFeedbackCueV1::PhysicalCombat {
                    source: None,
                    target: target.clone(),
                    location: None,
                    mode: rules::PhysicalAttackMode::Fight,
                    outcome: rules::ObserverPhysicalOutcomeV1::Missed,
                },
            },
            rules::ObservedEventV1::Feedback {
                cue: rules::ObserverFeedbackCueV1::Transaction {
                    actor: player.clone(),
                    source: rules::ObserverTransactionSourceV1::BankWithdrawal {
                        service_id: "bank".to_string(),
                        capability_id: "withdraw".to_string(),
                        bank_id: "bank_1".to_string(),
                        amount: i64::MAX,
                    },
                    costs: vec![],
                    rewards: vec![rules::ObserverTransactionRewardV1::CarriedGold {
                        amount: i64::MAX,
                        position: rules::CarriedGoldPosition::Sack,
                        before: 0,
                        after: i64::MAX,
                    }],
                },
            },
            rules::ObservedEventV1::Feedback {
                cue: rules::ObserverFeedbackCueV1::Defeat {
                    actor: target,
                    location: location(),
                    cause: rules::DeathCause::Physical,
                    credited_source: None,
                },
            },
            rules::ObservedEventV1::Feedback {
                cue: rules::ObserverFeedbackCueV1::Resurrection {
                    actor: player,
                    corpse_id: Some(rules::CorpseId::parse("corpse:1").expect("corpse ID")),
                    method: rules::ResurrectionMethod::Gods,
                    destination: location(),
                    current_hp: 1,
                    current_stamina: 1,
                },
            },
        ];

        let converted = events(&values).expect("feedback conversion");
        let encoded = serde_json::to_string(&converted).expect("serialize converted feedback");
        assert!(encoded.contains(r#""source":null"#));
        assert!(encoded.contains(r#""credited_source":null"#));
        assert!(encoded.contains(r#""9223372036854775807""#));
        assert!(encoded.contains(r#""corpse_id":"corpse:1""#));
        assert!(!encoded.contains("character_id"));
    }

    #[test]
    fn protocol_v1_feedback_conversion_rejects_invalid_text() {
        let value = rules::ObservedEventV1::Feedback {
            cue: rules::ObserverFeedbackCueV1::NpcMessage {
                npc_actor_id: rules::ActorId::from("guide"),
                npc_name: "Guide".to_string(),
                interaction_id: "speak".to_string(),
                response: "line\nbreak".to_string(),
            },
        };
        assert!(events(&[value]).is_err());
    }
}
