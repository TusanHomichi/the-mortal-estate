use super::*;

pub(super) fn render(event: &Event) -> Vec<String> {
    match event {
        Event::ItemRelocated {
            actor,
            item,
            from,
            to,
            reason,
            ..
        } => {
            use tme_rules::{ItemLocationViewV1, ItemRelocationReason};
            let ground_position = |location: &ItemLocationViewV1| match location {
                ItemLocationViewV1::Ground { location, .. } => Some(location.position),
                ItemLocationViewV1::Carried { .. }
                | ItemLocationViewV1::Corpse { .. }
                | ItemLocationViewV1::Merchant { .. }
                | ItemLocationViewV1::Locker { .. }
                | ItemLocationViewV1::Offered { .. } => None,
            };
            let carried_position = |location: &ItemLocationViewV1| match location {
                ItemLocationViewV1::Carried { position, .. } => Some(position.label()),
                ItemLocationViewV1::Ground { .. }
                | ItemLocationViewV1::Corpse { .. }
                | ItemLocationViewV1::Merchant { .. }
                | ItemLocationViewV1::Locker { .. }
                | ItemLocationViewV1::Offered { .. } => None,
            };
            let corpse_id = |location: &ItemLocationViewV1| match location {
                ItemLocationViewV1::Corpse { corpse_id, .. } => Some(corpse_id.clone()),
                ItemLocationViewV1::Ground { .. }
                | ItemLocationViewV1::Carried { .. }
                | ItemLocationViewV1::Merchant { .. }
                | ItemLocationViewV1::Locker { .. }
                | ItemLocationViewV1::Offered { .. } => None,
            };
            vec![match reason {
                ItemRelocationReason::Thrown => {
                    let position = ground_position(to).expect("thrown item lands on ground");
                    format!("{item} lands at ({},{})", position.x, position.y)
                }
                ItemRelocationReason::DeathDrop => {
                    let position = ground_position(to).expect("defeat item lands on ground");
                    format!(
                        "{actor}'s {item} fell to the ground at ({},{})",
                        position.x, position.y
                    )
                }
                ItemRelocationReason::CorpseRetention => {
                    let corpse_id = corpse_id(to).expect("retained item enters a corpse");
                    format!("{actor}'s {item} was retained in {corpse_id}")
                }
                ItemRelocationReason::CorpseSearch => {
                    let corpse_id = corpse_id(from).expect("searched item leaves a corpse");
                    let position = ground_position(to).expect("searched item reaches ground");
                    format!(
                        "{actor} released {item} from {corpse_id} at ({},{})",
                        position.x, position.y
                    )
                }
                ItemRelocationReason::ResurrectionReturn => {
                    let corpse_id = corpse_id(from).expect("restored item leaves a corpse");
                    let destination =
                        carried_position(to).expect("restored item returns to carried layout");
                    format!("{actor} restored {item} from {corpse_id} to {destination}")
                }
                ItemRelocationReason::WeaponFumble => {
                    let position = ground_position(to).expect("fumbled item lands on ground");
                    format!(
                        "{actor} fumbled {item}; it lands at ({},{})",
                        position.x, position.y
                    )
                }
                ItemRelocationReason::PlayerMove => match (
                    ground_position(from),
                    carried_position(from),
                    ground_position(to),
                    carried_position(to),
                ) {
                    (Some(position), _, _, Some(destination)) => format!(
                        "{actor} moved {item} from ({},{}) to {destination}",
                        position.x, position.y
                    ),
                    (_, Some(source), Some(position), _) => format!(
                        "{actor} moved {item} from {source} to ({},{})",
                        position.x, position.y
                    ),
                    (_, Some(source), _, Some(destination)) => {
                        format!("{actor} moved {item} from {source} to {destination}")
                    }
                    _ => format!("{actor} moved {item}"),
                },
                ItemRelocationReason::Scavenging => {
                    let destination = carried_position(to).unwrap_or("carried");
                    format!("{actor} scavenged {item} into {destination}")
                }
                ItemRelocationReason::MerchantPurchase => {
                    let destination =
                        carried_position(to).expect("merchant purchase enters the carried layout");
                    format!("{actor} bought {item} into {destination}")
                }
                ItemRelocationReason::MerchantSale => {
                    format!("{actor} sold {item} to a merchant")
                }
                ItemRelocationReason::LockerDeposit => {
                    format!("{actor} stored {item} in a locker")
                }
                ItemRelocationReason::LockerWithdrawal => {
                    let destination =
                        carried_position(to).expect("locker withdrawal enters carried layout");
                    format!("{actor} withdrew {item} to {destination}")
                }
                ItemRelocationReason::OfferCreated => {
                    format!("{actor} offered {item}")
                }
                ItemRelocationReason::OfferAccepted => {
                    let destination =
                        carried_position(to).expect("accepted offer enters carried layout");
                    format!("{actor} accepted {item} into {destination}")
                }
                ItemRelocationReason::OfferReturned => {
                    let (destination_actor_id, destination) = match to {
                        ItemLocationViewV1::Carried { actor_id, position } => {
                            (actor_id, position.label())
                        }
                        _ => panic!("returned offer enters carried layout"),
                    };
                    format!("{item} returned to {destination_actor_id}'s {destination}")
                }
            }]
        }
        Event::ItemBound { actor, item, .. } => {
            vec![format!("{item} is now tied to {actor}")]
        }
        Event::ActorDefeated {
            actor,
            cause,
            loot_claim,
            ..
        } => vec![format!(
            "{actor} was defeated: cause={}{}",
            death_cause_label(*cause),
            claim_suffix(loot_claim)
        )],
        Event::CorpseCreated {
            corpse_id,
            origin_name,
            location,
            loot_claim,
            ..
        } => vec![format!(
            "corpse {corpse_id} created for {origin_name} at {}{}",
            location.label(),
            claim_suffix(loot_claim)
        )],
        Event::CorpseSearched {
            corpse_id,
            actor,
            items_released,
            gold_released,
            ..
        } => vec![format!(
            "{actor} searched {corpse_id}: items_released={items_released} gold_released={gold_released}"
        )],
        Event::CorpseRemoved {
            corpse_id, method, ..
        } => vec![format!(
            "corpse {corpse_id} removed by {}",
            resurrection_method_label(*method)
        )],
        Event::ActorLifeStateChanged {
            actor, from, to, ..
        } => vec![format!(
            "{actor} life state: {} -> {}",
            life_state_label(from),
            life_state_label(to)
        )],
        Event::ResurrectionRequested {
            actor,
            cause,
            method,
            ..
        } => vec![format!(
            "resurrection requested for {actor}: cause={} method={}",
            death_cause_label(*cause),
            resurrection_method_label(*method)
        )],
        Event::ActorResurrected {
            actor,
            corpse_id,
            method,
            destination,
            current_hp,
            current_stamina,
            ..
        } => vec![format!(
            "{actor} resurrected by {} from {} at {} hp={current_hp} stamina={current_stamina}",
            resurrection_method_label(*method),
            option_label(corpse_id),
            destination.label()
        )],
        Event::GoldRelocated {
            actor,
            amount,
            from,
            to,
            reason,
            loot_claim,
            ..
        } => vec![format!(
            "{actor} relocated {amount} gold: {} -> {} reason={}{}",
            gold_location_label(from),
            gold_location_label(to),
            gold_reason_label(*reason),
            claim_suffix(loot_claim)
        )],
        Event::BankBalanceChanged {
            actor,
            bank_id,
            amount,
            before,
            after,
            reason,
            ..
        } => vec![format!(
            "{actor} {} {amount} gold at bank {bank_id}: {before} -> {after}",
            match reason {
                tme_rules::BankBalanceChangeReasonV1::Deposit => "deposited",
                tme_rules::BankBalanceChangeReasonV1::Withdrawal => "withdrew",
            }
        )],
        Event::ItemOfferCreated {
            actor,
            item,
            recipient_character_id,
            source_position,
            ..
        } => vec![format!(
            "{actor} offered {item} from {} to character {}",
            source_position.label(),
            recipient_character_id.as_str()
        )],
        Event::ItemOfferCompleted {
            actor,
            item,
            destination,
            reason,
            ..
        } => vec![format!(
            "{actor} completed the {item} offer: {} to {}",
            match reason {
                tme_rules::ItemOfferCompletionReasonV1::Accepted => "accepted",
                tme_rules::ItemOfferCompletionReasonV1::Refused => "refused",
                tme_rules::ItemOfferCompletionReasonV1::Withdrawn => "withdrawn",
                tme_rules::ItemOfferCompletionReasonV1::Separated => "separated",
                tme_rules::ItemOfferCompletionReasonV1::SenderDefeated => "sender_defeated",
                tme_rules::ItemOfferCompletionReasonV1::RecipientDefeated => {
                    "recipient_defeated"
                }
            },
            destination.label()
        )],
        Event::ResourceRegenerated {
            actor,
            resource,
            activity,
            boundary_at,
            amount,
            current,
            maximum,
            ..
        } => vec![format!(
            "{actor} regenerated {amount} {resource:?} ({current}/{maximum}, {activity:?}, time {boundary_at})"
        )],
        Event::ResourceRestored {
            actor,
            resource,
            before,
            after,
            maximum,
            ..
        } => vec![format!(
            "{actor} restored {resource:?} from {before} to {after}/{maximum}"
        )],
        Event::ItemConsumed {
            actor,
            item,
            reason,
            ..
        } => vec![match reason {
            tme_rules::ItemConsumptionReason::Drink => {
                format!("{actor} drinks the {item} and the empty bottle shatters")
            }
        }],
        Event::BalmHealed {
            actor, amount, hp, ..
        } => {
            vec![format!(
                "The balm knits {actor}'s wounds: regained {amount} hp ({hp} hp)"
            )]
        }
        Event::DoorOpened { location, .. } => {
            vec![format!("--- Door opened: {} ---", location.label())]
        }
        Event::DoorClosed { location, .. } => {
            vec![format!("--- Door closed: {} ---", location.label())]
        }
        Event::SecretTransitionRevealed {
            location,
            transition_kind,
            ..
        } => {
            vec![format!(
                "--- Secret {transition_kind} revealed: {} ---",
                location.label()
            )]
        }
        Event::SecretTransitionHidden {
            location,
            transition_kind,
            ..
        } => {
            vec![format!(
                "--- Secret {transition_kind} hidden: {} ---",
                location.label()
            )]
        }
        Event::TransitionConcealed {
            actor,
            spell_name,
            location,
            remaining_rounds,
            ..
        } => vec![format!(
            "--- {actor}'s {spell_name} conceals the transition at {} for {remaining_rounds} rounds ---",
            location.label()
        )],
        Event::TransitionConcealmentRemoved {
            location, reason, ..
        } => {
            let reason = match reason {
                TransitionConcealmentRemovalReasonV1::Revealed => "revealed",
                TransitionConcealmentRemovalReasonV1::Opened => "opened",
                TransitionConcealmentRemovalReasonV1::Expired => "expired",
                TransitionConcealmentRemovalReasonV1::Replaced => "replaced",
            };
            vec![format!(
                "--- Transition concealment at {} was {reason} ---",
                location.label()
            )]
        }
        Event::WorldTransition {
            actor,
            from,
            to,
            navigation,
            ..
        } => match navigation {
            NavigationKind::Stairs { direction } => vec![format!(
                "--- {actor} uses {} stairs: {} -> {} ---",
                direction.label(),
                from.label(),
                to.label()
            )],
            kind => vec![format!(
                "--- {actor} transitions via {kind:?}: {} -> {} ---",
                from.label(),
                to.label()
            )],
        },
        Event::SackShown {
            actor, items, gold, ..
        } => {
            if items.is_empty() && *gold == 0 {
                vec![format!("{actor}'s sack is empty")]
            } else {
                let mut lines = vec![format!("{actor}'s sack:")];
                lines.extend(items.iter().map(|positioned| {
                    format!(
                        "  {}: {}",
                        positioned.position.label(),
                        positioned.item.name
                    )
                }));
                lines.push(format!("  gold: {gold}"));
                lines
            }
        }
        Event::ItemIdentified {
            actor,
            item_name,
            location,
            ..
        } => vec![format!("{actor} identified {item_name} in {location}")],
        Event::ItemAppraised {
            actor,
            item_name,
            total_value_gold,
            ..
        } => vec![format!(
            "{actor} appraised {item_name} at {total_value_gold} gold"
        )],
        Event::ItemEnchanted {
            item_definition_id,
            combat_add_rating_bonus,
            remaining_rounds,
            ..
        } => {
            let duration = remaining_rounds
                .map(|rounds| format!(" for {rounds} rounds"))
                .unwrap_or_default();
            vec![format!(
                "{item_definition_id} gains {combat_add_rating_bonus:+} combat add{duration}"
            )]
        }
        Event::ItemEnchantmentExpired {
            item_definition_id, ..
        } => {
            vec![format!("{item_definition_id}'s enchantment faded")]
        }
        Event::ItemTransformed {
            old_item_definition_id,
            new_item_definition_id,
            location,
            ..
        } => vec![format!(
            "{old_item_definition_id} became {new_item_definition_id} in {location}"
        )],
        Event::Located {
            subject, id, hint, ..
        } => vec![format!("located {subject} {id}: {hint}")],
        Event::PortalCreated {
            location,
            target,
            remaining_rounds,
            two_way,
            ..
        } => {
            let duration = remaining_rounds
                .map(|rounds| format!(" for {rounds} rounds"))
                .unwrap_or_default();
            let direction = if *two_way { "two-way" } else { "one-way" };
            vec![format!(
                "--- Portal created ({direction}): {} -> {}{duration} ---",
                location.label(),
                target.label()
            )]
        }
        Event::PortalExpired { location, .. } => {
            vec![format!("--- Portal expired: {} ---", location.label())]
        }
        Event::ExperienceAwarded {
            actor,
            amount,
            total_xp,
            ..
        } => {
            vec![format!(
                "{actor} gained {amount} experience points (total: {total_xp})"
            )]
        }
        Event::LevelGained {
            actor,
            current_class_id,
            new_level,
            total_xp,
            hp_growth,
            hp,
            max_hp,
            peak_hp,
            mp_growth,
            mp,
            max_mp,
            stamina_growth,
            stamina,
            max_stamina,
            ..
        } => {
            vec![
                format!(
                    "{actor} reached level {new_level} as {current_class_id}! (xp: {total_xp})"
                ),
                format!("  hp +{hp_growth}: {hp}/{max_hp} (peak {peak_hp})"),
                format!("  mp +{mp_growth}: {mp}/{max_mp}"),
                format!("  stamina +{stamina_growth}: {stamina}/{max_stamina}"),
            ]
        }
        Event::PhysicalAttributeAddsChanged {
            actor,
            strength_adds,
            dexterity_adds,
            ..
        } => {
            vec![format!(
                "{actor} combat adds: STR +{strength_adds} DEX +{dexterity_adds}"
            )]
        }
        Event::MovementStaminaSpent {
            actor,
            stamina,
            max_stamina,
            amount,
            pace,
            exertion,
            ..
        } => vec![format!(
            "{actor} spent {amount} stamina ({} {exertion:?}): {stamina}/{max_stamina}",
            pace.label()
        )],
        Event::SkillPracticeAwarded {
            actor,
            track_id,
            track_display,
            raw_amount,
            learning_rate,
            credited_amount,
            practice_points,
            ..
        } => {
            let display = track_display.as_deref().unwrap_or(track_id);
            if raw_amount == credited_amount {
                let unit = if *credited_amount == 1 {
                    "practice point"
                } else {
                    "practice points"
                };
                vec![format!(
                    "{actor} gained {credited_amount} {display} {unit} (pool: {practice_points})"
                )]
            } else {
                let raw_unit = if *raw_amount == 1 {
                    "practice point"
                } else {
                    "practice points"
                };
                let credited_unit = if *credited_amount == 1 {
                    "practice point"
                } else {
                    "practice points"
                };
                vec![format!(
                    "{actor} gained {raw_amount} raw {display} {raw_unit} at learning rate {learning_rate}, credited as {credited_amount} {credited_unit} (pool: {practice_points})"
                )]
            }
        }
        Event::SkillPositionChanged {
            actor,
            track_id,
            track_display,
            new_level,
            new_critique_rank,
            level_title,
            ..
        } => {
            let display = track_display.as_deref().unwrap_or(track_id);
            let title = level_title
                .as_deref()
                .map(|title| format!(" ({title})"))
                .unwrap_or_default();
            vec![format!(
                "{actor}'s {display} skill advanced to level {new_level}, critique {new_critique_rank}{title}"
            )]
        }
        Event::GoldChanged { actor, amount, .. } => {
            if *amount < 0 {
                vec![format!("{actor} spent {} gold", -amount)]
            } else {
                vec![format!("{actor} gained {} gold", amount)]
            }
        }
        Event::TrainingPurchased {
            actor,
            service_id,
            track_id,
            offered_gold,
            spent_gold,
            unspent_gold,
            previous_learning_rate,
            new_learning_rate,
            ..
        } => vec![format!(
            "{actor} trained {track_id} with {service_id}: learning rate {previous_learning_rate} -> {new_learning_rate}, offered {offered_gold} gold, spent {spent_gold}, retained {unspent_gold}"
        )],

        _ => unreachable!("event family is selected by the exhaustive dispatcher"),
    }
}
