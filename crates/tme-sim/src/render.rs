use tme_rules::events::{
    AutomaticActorDecisionV1, AutomaticMovementPurposeV1, AutomaticWaitReasonV1,
    BanishResultReasonV1, RaiseDeadResultReasonV1, SpellCastFailure, SpellFizzleCause,
    SpellPathFailureReason, TransitionConcealmentRemovalReasonV1,
};
use tme_rules::{ActorKind, Event, InspectExitStatus, NavigationKind};
pub(crate) fn render_events(events: &[Event]) -> Vec<String> {
    events.iter().flat_map(render_event).collect()
}

fn option_label<T: std::fmt::Display>(value: &Option<T>) -> String {
    value
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "none".to_string())
}

fn mitigation_mode_label(mode: Option<tme_rules::SpellResistanceMitigationMode>) -> &'static str {
    match mode {
        Some(tme_rules::SpellResistanceMitigationMode::Negate) => "negate",
        Some(tme_rules::SpellResistanceMitigationMode::HalfDamage) => "half_damage",
        Some(tme_rules::SpellResistanceMitigationMode::MinimumDamage) => "minimum_damage",
        None => "none",
    }
}

fn death_cause_label(cause: tme_rules::DeathCause) -> &'static str {
    match cause {
        tme_rules::DeathCause::Physical => "physical",
        tme_rules::DeathCause::Poison => "poison",
        tme_rules::DeathCause::Fire => "fire",
        tme_rules::DeathCause::OtherMagic => "other_magic",
        tme_rules::DeathCause::Hazard => "hazard",
    }
}

fn life_state_label(state: &tme_rules::ActorLifeState) -> &'static str {
    match state {
        tme_rules::ActorLifeState::Alive => "alive",
        tme_rules::ActorLifeState::Ghost { .. } => "ghost",
        tme_rules::ActorLifeState::AwaitingResurrection { .. } => "awaiting_resurrection",
        tme_rules::ActorLifeState::Dead => "dead",
    }
}

fn life_state_view_label(state: &tme_rules::ActorLifeStateViewV1) -> &'static str {
    match state {
        tme_rules::ActorLifeStateViewV1::Alive => "alive",
        tme_rules::ActorLifeStateViewV1::Ghost { .. } => "ghost",
        tme_rules::ActorLifeStateViewV1::AwaitingResurrection { .. } => "awaiting_resurrection",
        tme_rules::ActorLifeStateViewV1::Dead => "dead",
    }
}

fn loot_claim_label(claim: &tme_rules::LootClaim) -> String {
    let owner = match &claim.owner {
        tme_rules::LootOwnerId::Character(id) => id.as_str().to_string(),
        tme_rules::LootOwnerId::TransientActor(id) => format!("actor:{id}"),
    };
    let basis = match claim.basis {
        tme_rules::LootClaimBasis::KillingBlow => "killing_blow",
        tme_rules::LootClaimBasis::CharacterDeathPile => "character_death_pile",
    };
    format!("{owner}/{basis}")
}

fn claim_suffix(claim: &Option<tme_rules::LootClaim>) -> String {
    claim
        .as_ref()
        .map(|claim| format!(" claim={}", loot_claim_label(claim)))
        .unwrap_or_default()
}

fn gold_location_label(location: &tme_rules::GoldLocationViewV1) -> String {
    match location {
        tme_rules::GoldLocationViewV1::Carried { actor_id, position } => {
            format!("carried:{actor_id}:{}", position.label())
        }
        tme_rules::GoldLocationViewV1::Corpse { corpse_id } => corpse_id.to_string(),
        tme_rules::GoldLocationViewV1::Ground {
            gold_pile_id,
            location,
        } => format!("ground:{gold_pile_id}@{}", location.label()),
        tme_rules::GoldLocationViewV1::Bank {
            bank_id,
            character_id,
        } => format!("bank:{bank_id}:{}", character_id.as_str()),
    }
}

fn resurrection_method_label(method: tme_rules::ResurrectionMethod) -> &'static str {
    match method {
        tme_rules::ResurrectionMethod::Gods => "gods",
        tme_rules::ResurrectionMethod::Priest => "priest",
        tme_rules::ResurrectionMethod::Thaumaturge => "thaumaturge",
    }
}

fn gold_reason_label(reason: tme_rules::GoldRelocationReason) -> &'static str {
    match reason {
        tme_rules::GoldRelocationReason::PlayerMove => "player_move",
        tme_rules::GoldRelocationReason::Scavenging => "scavenging",
        tme_rules::GoldRelocationReason::BankDeposit => "bank_deposit",
        tme_rules::GoldRelocationReason::BankWithdrawal => "bank_withdrawal",
        tme_rules::GoldRelocationReason::DeathDrop => "death_drop",
        tme_rules::GoldRelocationReason::CorpseRetention => "corpse_retention",
        tme_rules::GoldRelocationReason::CorpseSearch => "corpse_search",
        tme_rules::GoldRelocationReason::ResurrectionReturn => "resurrection_return",
    }
}

fn alignment_label(alignment: tme_rules::CharacterAlignment) -> &'static str {
    match alignment {
        tme_rules::CharacterAlignment::Lawful => "lawful",
        tme_rules::CharacterAlignment::Neutral => "neutral",
        tme_rules::CharacterAlignment::Chaotic => "chaotic",
        tme_rules::CharacterAlignment::Evil => "evil",
    }
}

fn render_event(event: &Event) -> Vec<String> {
    match event {
        Event::ScenarioLoaded {
            name,
            realms,
            levels,
            ..
        } => {
            let realm_list = realms
                .iter()
                .map(|r| r.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let level_list = levels
                .iter()
                .map(|site| site.label())
                .collect::<Vec<_>>()
                .join(", ");
            vec![format!(
                "loaded \"{name}\" realms=[{realm_list}] levels=[{level_list}]"
            )]
        }
        Event::ActorStatus {
            actor,
            kind,
            location,
            hp,
            actor_id: _,
            character_identity,
        } => {
            let class_tag = character_identity
                .as_ref()
                .map(|ci| format!(" ({})", ci.display_class))
                .unwrap_or_default();
            vec![format!(
                "{} {actor}{class_tag} at {} hp={hp}",
                role_label(*kind),
                location.label()
            )]
        }
        Event::ActorReady {
            actor,
            kind,
            logical_time,
            ..
        } => {
            vec![format!(
                "{} ready: {actor} at {logical_time}",
                role_label(*kind)
            )]
        }
        Event::PlayerIntent {
            actor,
            logical_time,
            intent,
            ..
        } => vec![format!("{actor} intent at {logical_time}: {intent}")],
        Event::ActorReadinessScheduled {
            actor,
            cost_units,
            ready_at,
            ..
        } => vec![format!(
            "readiness scheduled: {actor} cost={cost_units} ready_at={ready_at}"
        )],
        Event::GroupInvitationCreated {
            invitation_id,
            issuer_character_id,
            target_character_id,
            ..
        } => vec![format!(
            "group invitation {}: {} invited {}",
            invitation_id.value(),
            issuer_character_id.as_str(),
            target_character_id.as_str()
        )],
        Event::GroupInvitationResolved {
            invitation_id,
            resolution,
            ..
        } => vec![format!(
            "group invitation {} resolved: {resolution:?}",
            invitation_id.value()
        )],
        Event::GroupChanged {
            group_id,
            reason,
            leader_character_id,
            member_character_ids,
            ..
        } => vec![format!(
            "group {} changed: {reason:?} leader={leader_character_id:?} members={}",
            group_id.value(),
            member_character_ids.len()
        )],
        Event::GroupPresenceChanged {
            group_id,
            character_id,
            connected,
            ..
        } => vec![format!(
            "group {} presence: {} connected={connected}",
            group_id.value(),
            character_id.as_str()
        )],
        Event::PlayerFollowChanged {
            follower_character_id,
            target_character_id,
            reason,
        } => vec![format!(
            "follow changed: {} target={target_character_id:?} reason={reason:?}",
            follower_character_id.as_str()
        )],
        Event::CommunicationPreferenceChanged {
            character_id,
            pages_enabled,
        } => vec![format!(
            "communication preferences changed: {} pages_enabled={pages_enabled}",
            character_id.as_str()
        )],
        Event::CharacterBlockChanged {
            character_id,
            target_character_id,
            blocked,
        } => vec![format!(
            "communication block changed: {} target={} blocked={blocked}",
            character_id.as_str(),
            target_character_id.as_str()
        )],
        Event::LogicalTimeAdvanced { to, .. } => vec![format!("time {to}")],
        Event::Inspected {
            actor,
            location,
            tile,
            tile_move_cost,
            exits,
            nearby_actors,
            ground_items,
            actor_id: _,
        } => {
            let mut lines = vec![format!(
                "{actor} inspected at {}: tile={tile}",
                location.label()
            )];
            lines.push(match tile_move_cost {
                Some(cost) => format!("  current tile: terrain={tile} cost={cost}"),
                None => format!("  current tile: terrain={tile} impassable"),
            });
            lines.extend(exits.iter().map(|exit| {
                let status = match &exit.status {
                    InspectExitStatus::Walkable => "walkable".to_string(),
                    InspectExitStatus::BlockedTerrain => "blocked terrain".to_string(),
                    InspectExitStatus::OutOfBounds => "out of bounds".to_string(),
                    InspectExitStatus::Door { state, target } => {
                        if state == "closed" {
                            format!("door (closed; opens on movement) -> {}", target.label())
                        } else {
                            format!("door ({state}) -> {}", target.label())
                        }
                    }
                };
                let terrain_label = exit
                    .terrain
                    .as_ref()
                    .map(|terrain| format!(" terrain={terrain}"))
                    .unwrap_or_default();
                let cost_label = exit
                    .move_cost
                    .map(|cost| format!(" cost={cost}"))
                    .unwrap_or_default();
                format!(
                    "  {} -> {}: {status}{terrain_label}{cost_label}",
                    exit.direction.label(),
                    exit.location.label()
                )
            }));
            lines.extend(nearby_actors.iter().map(|nearby| {
                let class_tag = nearby
                    .character_identity
                    .as_ref()
                    .map(|ci| format!(" ({})", ci.display_class))
                    .unwrap_or_default();
                format!(
                    "  nearby {}: {} {}{class_tag} at ({},{}) hp={}",
                    nearby.direction.label(),
                    role_label(nearby.kind),
                    nearby.actor,
                    nearby.location.position.x,
                    nearby.location.position.y,
                    nearby.hp
                )
            }));
            lines.extend(ground_items.iter().map(|ground| match ground.direction {
                None => format!(
                    "  ground here: {} at ({},{})",
                    ground.item.name, ground.location.position.x, ground.location.position.y
                ),
                Some(direction) => format!(
                    "  ground {}: {} at ({},{})",
                    direction.label(),
                    ground.item.name,
                    ground.location.position.x,
                    ground.location.position.y
                ),
            }));
            lines
        }
        Event::AutomaticActorDecision {
            actor, decision, ..
        } => {
            let intent = match decision {
                AutomaticActorDecisionV1::Suppressed { status } => {
                    format!("suppressed by {status}")
                }
                AutomaticActorDecisionV1::UseAbility {
                    spell_name, target, ..
                } => target.as_ref().map_or_else(
                    || format!("use {spell_name}"),
                    |target| format!("use {spell_name} on {target}"),
                ),
                AutomaticActorDecisionV1::PhysicalAttack { target, mode, .. } => {
                    format!("{} {target}", mode.label())
                }
                AutomaticActorDecisionV1::Nock { item, .. } => format!("nock {item}"),
                AutomaticActorDecisionV1::DrinkBalm { item_instance_id } => {
                    format!("drink balm {item_instance_id}")
                }
                AutomaticActorDecisionV1::SearchCorpse { corpse_id } => {
                    format!("search {corpse_id}")
                }
                AutomaticActorDecisionV1::CollectItem {
                    item_instance_id,
                    destination,
                } => format!("collect {item_instance_id} into {}", destination.label()),
                AutomaticActorDecisionV1::CollectGold {
                    gold_pile_id,
                    amount,
                } => format!("collect {amount} gold from {gold_pile_id}"),
                AutomaticActorDecisionV1::Move { direction, purpose } => {
                    let verb = match purpose {
                        AutomaticMovementPurposeV1::Chase => "move",
                        AutomaticMovementPurposeV1::Flee => "flee",
                        AutomaticMovementPurposeV1::Turned => "flee (turned)",
                        AutomaticMovementPurposeV1::Search => "search",
                        AutomaticMovementPurposeV1::Scavenge => "scavenge",
                        AutomaticMovementPurposeV1::ReturnHome => "return",
                    };
                    format!("{verb} {}", direction.label())
                }
                AutomaticActorDecisionV1::Wait { reason } => match reason {
                    AutomaticWaitReasonV1::Watch => "watch".to_string(),
                    AutomaticWaitReasonV1::Hold => "hold".to_string(),
                    AutomaticWaitReasonV1::Blocked => "wait".to_string(),
                    AutomaticWaitReasonV1::ReturnBlocked => "return blocked".to_string(),
                    AutomaticWaitReasonV1::Home => "home".to_string(),
                    AutomaticWaitReasonV1::Ambush => "wait".to_string(),
                },
            };
            vec![format!("{actor} intent: {intent}")]
        }
        Event::Moved {
            actor, from, to, ..
        } => vec![format!(
            "{actor} moved from ({},{}) to ({},{})",
            from.position.x, from.position.y, to.position.x, to.position.y
        )],
        Event::MovementStarted {
            actor,
            pace,
            requested_steps,
            accepted_steps,
            available_path_points,
            burden_tier,
            exertion,
            stamina_cost,
            stop_reason,
            ..
        } => vec![format!(
            "{actor} {}: requested={requested_steps} accepted={accepted_steps} points={available_path_points} burden={burden_tier:?} exertion={exertion:?} stamina_cost={stamina_cost:?} stop={stop_reason:?}",
            pace.label()
        )],
        Event::MovementCostPaid {
            actor,
            direction,
            terrain,
            cost,
            remaining_points,
            destination,
            ..
        } => vec![format!(
            "  {actor} step {} -> ({},{}): terrain={terrain} cost={cost} remaining={remaining_points}",
            direction.label(),
            destination.position.x,
            destination.position.y
        )],
        Event::MovementBlocked {
            actor,
            from,
            attempted,
            reason,
            ..
        } => vec![format!(
            "{actor} blocked from ({},{}) to ({},{}): {reason}",
            from.position.x, from.position.y, attempted.position.x, attempted.position.y
        )],
        Event::AttackBlockedNoSight {
            attacker, defender, ..
        } => {
            vec![format!("{attacker} cannot see {defender}: attack blocked")]
        }
        Event::AttackNotReady {
            actor,
            target,
            ready_at,
            ..
        } => {
            vec![format!(
                "{actor} cannot attack {target} until time {ready_at}"
            )]
        }
        Event::Attacked {
            attacker,
            defender,
            roll,
            damage,
            label,
            defender_hp,
            ..
        } => vec![format!(
            "{attacker} attacked {defender}: roll={roll} damage={damage} label={} hp={defender_hp}",
            label.label()
        )],
        Event::AttackBlocked {
            attacker,
            defender,
            source,
            ..
        } => {
            vec![format!(
                "{attacker}'s attack against {defender} was blocked by {}",
                source.label()
            )]
        }
        Event::BowReadinessChanged {
            actor,
            item_instance_id,
            to,
            reason,
            ..
        } => vec![format!(
            "{actor}'s {item_instance_id} is now {} ({})",
            to.label(),
            reason.label()
        )],
        Event::WeaponFumbled {
            attacker,
            item_instance_id,
            reason,
            ..
        } => vec![format!(
            "{attacker} fumbled {item_instance_id} ({})",
            reason.label()
        )],
        Event::AttackMissed {
            attacker,
            defender,
            attacker_score,
            defender_score,
            roll,
            ..
        } => vec![format!(
            "{attacker} missed {defender}: roll={roll} attack_score={attacker_score} defense_score={defender_score}"
        )],
        Event::ProtectionApplied {
            defender,
            amount,
            damage_kind,
            armor_sources,
            ..
        } => {
            vec![format!(
                "{defender}'s armor ({} source{}) reduced {} damage by {amount}",
                armor_sources.len(),
                if armor_sources.len() == 1 { "" } else { "s" },
                damage_kind.label()
            )]
        }
        Event::PhysicalDamageAffinityApplied {
            defender,
            damage_kind,
            input_damage,
            numerator,
            denominator,
            adjusted_damage,
            ..
        } => vec![format!(
            "{defender}'s {} affinity adjusted {input_damage} by {numerator}/{denominator} to {adjusted_damage}",
            damage_kind.label()
        )],
        Event::EcologyResetScheduled {
            site_id,
            generation,
            member_ids,
            due_at,
            policy,
        } => vec![format!(
            "ecology site {site_id} generation {generation} {} materialization for [{}] scheduled for {}",
            match policy {
                tme_rules::EcologyLifecyclePolicyV1::FullSite => "full-site",
                tme_rules::EcologyLifecyclePolicyV1::SlotReplenishment => "slot",
            },
            member_ids.join(","),
            due_at.value(),
        )],
        Event::EcologyReset {
            site_id,
            from_generation,
            to_generation,
            member_ids,
            policy,
        } => vec![format!(
            "ecology site {site_id} {} materialized [{}]: generation {from_generation} -> {to_generation}",
            match policy {
                tme_rules::EcologyLifecyclePolicyV1::FullSite => "full-site",
                tme_rules::EcologyLifecyclePolicyV1::SlotReplenishment => "slot",
            },
            member_ids.join(","),
        )],
        Event::EcologyActorSpawned {
            site_id,
            member_id,
            generation,
            actor_id,
            location,
            ..
        } => vec![format!(
            "ecology site {site_id} spawned {member_id} as {actor_id} generation {generation} at {}",
            location.label()
        )],
        Event::PhysicalStaminaSpent {
            actor,
            mode,
            amount,
            stamina,
            max_stamina,
            ..
        } => vec![format!(
            "{actor} spent {amount} stamina on {} ({stamina}/{max_stamina})",
            mode.label()
        )],
        Event::PhysicalPracticeEvaluated {
            actor,
            track_id,
            mode,
            outcome,
            risk,
            total_raw_points,
            ..
        } => vec![format!(
            "{actor}'s {} {} practice on {track_id}: risk={} raw={total_raw_points}",
            mode.label(),
            outcome.label(),
            risk.label()
        )],
        Event::DefeatContributionRecorded {
            contributor_character_id,
            target_id,
            reward_class,
            applied_damage,
            ..
        } => vec![format!(
            "contribution to {target_id}: character={contributor_character_id:?} class={reward_class:?} damage={applied_damage}"
        )],
        Event::DefeatRewardEvaluated {
            target,
            available_experience,
            awarded_experience,
            reason,
            ..
        } => vec![format!(
            "shared defeat reward for {target}: available={available_experience} awarded={awarded_experience} reason={reason}"
        )],
        Event::DefeatRewardShareAwarded { actor, amount, .. } => {
            vec![format!("{actor} received {amount} shared defeat XP")]
        }
        Event::ThaumAboveSkillEvaluated {
            actor,
            spell_name,
            gap,
            roll,
            success_threshold,
            success,
            ..
        } => vec![format!(
            "{actor}'s above-skill {spell_name} attempt: gap={gap} roll={roll} threshold={success_threshold} success={success}"
        )],
        Event::MagicPracticeEvaluated {
            actor,
            spell_name,
            track_id,
            total_raw_points,
            reason,
            ..
        } => vec![format!(
            "{actor}'s {spell_name} practice on {track_id}: raw={total_raw_points} reason={reason}"
        )],
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
        Event::SkillCritiqued {
            actor,
            track_id,
            track_display,
            level,
            critique_rank,
            level_title,
            ..
        } => {
            let display = track_display.as_deref().unwrap_or(track_id);
            let rank = critique_rank
                .map(|rank| format!("critique {rank}"))
                .unwrap_or_else(|| "no critique rank".to_string());
            let title = level_title
                .as_deref()
                .map(|title| format!(" ({title})"))
                .unwrap_or_default();
            vec![format!(
                "{actor}'s {display} critique: level {level}, {rank}{title}"
            )]
        }
        Event::ClassPromoted {
            actor,
            from_class,
            to_class,
            granted_item,
            granted_item_position,
            granted_spells,
            ..
        } => {
            let spells = granted_spells
                .iter()
                .map(|spell| spell.spell_name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            vec![format!(
                "{actor} promoted from {from_class} to {to_class}, received {granted_item} in {}, and gained {spells}!",
                granted_item_position.label()
            )]
        }
        Event::SpellLearned {
            actor,
            spell_name,
            lane,
            gold_cost,
            trainer,
            spell_book,
            ..
        } => {
            vec![format!(
                "{trainer} records {spell_name} ({lane}) in {actor}'s retained {spell_book} for {gold_cost} gold"
            )]
        }
        Event::SpellCastStubbed {
            actor,
            spell_name,
            target,
            lane,
            ..
        } => {
            let target_text = target
                .as_ref()
                .map(|t| format!(" at {}", t.label()))
                .unwrap_or_default();
            vec![format!(
                "{actor} casts {spell_name}{target_text} (lane: {lane}) [stubbed]"
            )]
        }
        Event::SpellCastCommitted { .. } => Vec::new(),
        Event::ActorSummoned {
            caster,
            actor,
            location,
            remaining_rounds,
            ..
        } => {
            let duration = remaining_rounds
                .map(|rounds| format!(" for {rounds} rounds"))
                .unwrap_or_default();
            vec![format!(
                "--- {caster} summoned {actor} at {}{duration} ---",
                location.label()
            )]
        }
        Event::SummonExpired {
            actor, location, ..
        } => vec![format!("--- {actor} faded from {} ---", location.label())],
        Event::BanishEvaluated {
            caster,
            target,
            success,
            reason,
            ..
        } => {
            let reason = match reason {
                BanishResultReasonV1::Banished => "banished",
                BanishResultReasonV1::InvalidTarget => "invalid target",
                BanishResultReasonV1::IneligibleTrait => "ineligible creature",
                BanishResultReasonV1::WillpowerFormulaOpen => "willpower contest unresolved",
            };
            vec![format!(
                "{caster} tests Banish against {target}: {} ({reason}).",
                if *success { "success" } else { "failure" }
            )]
        }
        Event::ActorBanished {
            caster,
            actor,
            location,
            ..
        } => vec![format!(
            "--- {caster} banished {actor} from {} ---",
            location.label()
        )],
        Event::TurnUndeadResolved {
            caster,
            considered_actor_ids,
            moved_actor_ids,
            blocked_actor_ids,
            ..
        } => vec![format!(
            "{caster} turns undead: considered=[{}] moved=[{}] blocked=[{}].",
            considered_actor_ids.join(","),
            moved_actor_ids.join(","),
            blocked_actor_ids.join(",")
        )],
        Event::RaiseDeadEvaluated {
            caster,
            corpse_id,
            roll,
            success_threshold,
            roll_denominator,
            success,
            reason,
            ..
        } => {
            let reason = match reason {
                RaiseDeadResultReasonV1::Resurrected => "resurrected",
                RaiseDeadResultReasonV1::NoCorpse => "no corpse",
                RaiseDeadResultReasonV1::NonPlayerCorpse => "non-player corpse",
                RaiseDeadResultReasonV1::RollFailed => "roll failed",
            };
            vec![format!(
                "{caster} attempts Raise Dead on {}: roll={} threshold={success_threshold}/{roll_denominator} result={} ({reason}).",
                option_label(corpse_id),
                option_label(roll),
                if *success { "success" } else { "failure" }
            )]
        }
        Event::SpellDamaged {
            caster,
            spell_name,
            target,
            damage,
            hp,
            ..
        } => {
            vec![format!(
                "{caster}'s {spell_name} hits {target} for {damage} damage ({hp} hp)."
            )]
        }
        Event::SpellHealed {
            caster,
            spell_name,
            target,
            amount,
            hp,
            ..
        } => {
            vec![format!(
                "{caster}'s {spell_name} heals {target} for {amount} hp ({hp} hp)."
            )]
        }
        Event::SpellWarmed {
            actor,
            spell_name,
            ready_at,
            ..
        } => {
            vec![format!(
                "{actor} warms {spell_name} (ready at time {ready_at})"
            )]
        }
        Event::WarmedSpellReady {
            actor, spell_name, ..
        } => {
            vec![format!("{actor}'s {spell_name} is ready")]
        }
        Event::WarmedSpellCast {
            actor, spell_name, ..
        } => {
            vec![format!("{actor} casts warmed {spell_name}")]
        }
        Event::SpellFizzled {
            actor,
            spell_name,
            cause,
            ..
        } => {
            let reason = match cause {
                SpellFizzleCause::Replaced { .. } => "replaced",
                SpellFizzleCause::Canceled => "canceled",
                SpellFizzleCause::Rest => "rest",
                SpellFizzleCause::HealingBalm => "healing balm",
                SpellFizzleCause::Damage { .. } => "damage",
                SpellFizzleCause::Defeat => "defeat",
            };
            vec![format!("{actor}'s {spell_name} fizzles: {reason}")]
        }
        Event::SpellCastFailed {
            actor,
            spell_name,
            failure,
            ..
        } => {
            let reason = match failure {
                SpellCastFailure::InvalidPath { reason } => {
                    let reason = match reason {
                        SpellPathFailureReason::OutOfBounds => "out of bounds",
                        SpellPathFailureReason::NotVisible => "not visible",
                        SpellPathFailureReason::OutOfRange => "out of range",
                    };
                    format!("invalid path ({reason})")
                }
                SpellCastFailure::AboveSkillAttempt => "above-skill attempt".to_string(),
            };
            vec![format!("{actor}'s {spell_name} fails: {reason}")]
        }
        Event::ActorHidden {
            actor,
            effect_id,
            remaining_rounds,
            ..
        } => {
            let duration = remaining_rounds
                .map(|rounds| format!(" for {rounds} rounds"))
                .unwrap_or_default();
            vec![format!("{actor} melts into cover ({effect_id}){duration}.")]
        }
        Event::HideBroken { actor, reason, .. } => {
            vec![format!("{actor} is revealed: {reason}.")]
        }
        Event::EffectApplied {
            actor,
            effect_id,
            kind,
            remaining_rounds,
            ..
        } => {
            let duration = remaining_rounds
                .map(|rounds| format!(" for {rounds} rounds"))
                .unwrap_or_default();
            vec![format!("{actor} gains {effect_id} ({kind}){duration}.")]
        }
        Event::TileEffectApplied {
            effect_id,
            location,
            remaining_rounds,
            passability,
            sight,
            hazard,
            move_cost,
            ..
        } => {
            vec![format!(
                "tile effect applied: {effect_id} at {} passability={} sight={} hazard={} cost={} remaining={}",
                location.label(),
                option_label(passability),
                option_label(sight),
                option_label(hazard),
                option_label(move_cost),
                option_label(remaining_rounds)
            )]
        }
        Event::SpellSaveResolved {
            actor,
            effect_id,
            resistance_tag,
            natural_save_twentieths,
            matching_bonus_twentieths,
            denominator,
            save_twentieths,
            roll,
            success,
            mitigation_mode,
            requested_damage,
            resolved_damage,
            ..
        } => {
            let result = if *success { "success" } else { "failure" };
            vec![format!(
                "{actor} resolves {effect_id} save [{resistance_tag}]: roll={roll} threshold={save_twentieths}/{denominator} natural={natural_save_twentieths} boost={matching_bonus_twentieths} result={result} mitigation={} damage={}->{}.",
                mitigation_mode_label(*mitigation_mode),
                option_label(requested_damage),
                option_label(resolved_damage)
            )]
        }
        Event::EffectTicked {
            actor,
            effect_id,
            remaining_rounds,
            ..
        } => {
            let duration = remaining_rounds
                .map(|rounds| format!(" ({rounds} rounds remain)"))
                .unwrap_or_default();
            vec![format!("{effect_id} ticks on {actor}{duration}.")]
        }
        Event::TileEffectTicked {
            effect_id,
            location,
            remaining_rounds,
            ..
        } => {
            vec![format!(
                "tile effect ticked: {effect_id} at {} remaining={}",
                location.label(),
                option_label(remaining_rounds)
            )]
        }
        Event::EffectDamaged {
            actor,
            effect_id,
            damage,
            hp,
            ..
        } => {
            vec![format!(
                "{effect_id} damages {actor} for {damage} ({hp} hp)."
            )]
        }
        Event::TileEffectDamaged {
            actor,
            effect_id,
            damage,
            hp,
            ..
        } => {
            vec![format!(
                "tile effect damaged: {actor} by {effect_id} damage={damage} hp={hp}"
            )]
        }
        Event::EffectExpired {
            actor, effect_id, ..
        } => {
            vec![format!("{effect_id} expires on {actor}.")]
        }
        Event::TileEffectExpired {
            effect_id,
            location,
            ..
        } => {
            vec![format!(
                "tile effect expired: {effect_id} at {}",
                location.label()
            )]
        }
        Event::EffectRemoved {
            actor,
            effect_id,
            reason,
            ..
        } => {
            vec![format!("{effect_id} is removed from {actor}: {reason}.")]
        }
        Event::TileEffectRemoved {
            effect_id,
            location,
            reason,
            ..
        } => {
            vec![format!(
                "tile effect removed: {effect_id} at {} reason={reason}",
                location.label()
            )]
        }
        Event::ActionSuppressedByStatus {
            actor,
            intent,
            effect_id,
            ..
        } => {
            vec![format!("{actor}'s {intent} is suppressed by {effect_id}.")]
        }
        Event::NpcSpoke {
            npc,
            interaction_id,
            response,
            ..
        } => vec![format!("{npc} [{interaction_id}]: {response}")],
        Event::NpcFollowChanged {
            npc,
            from_character_id,
            to_character_id,
            ..
        } => {
            let from = from_character_id
                .as_ref()
                .map_or("none", tme_rules::CharacterId::as_str);
            let to = to_character_id
                .as_ref()
                .map_or("none", tme_rules::CharacterId::as_str);
            vec![format!("{npc} follow target: {from} -> {to}")]
        }
        Event::NpcFollowDecision { npc, decision, .. } => {
            let decision = match decision {
                tme_rules::NpcFollowDecisionV1::Move { direction } => {
                    format!("move {}", direction.label())
                }
                tme_rules::NpcFollowDecisionV1::Wait { reason } => {
                    let reason = match reason {
                        tme_rules::NpcFollowWaitReasonV1::AtTarget => "at_target",
                        tme_rules::NpcFollowWaitReasonV1::Blocked => "blocked",
                        tme_rules::NpcFollowWaitReasonV1::RouteUnavailable => "route_unavailable",
                    };
                    format!("wait {reason}")
                }
            };
            vec![format!("{npc} follow decision: {decision}")]
        }
        Event::SelfDefenseChanged(change) => vec![format!(
            "self-defense for {}: {} -> {}",
            change.victim_character_id.as_str(),
            change
                .before_attacker_character_id
                .as_ref()
                .map_or("none", tme_rules::CharacterId::as_str),
            change
                .after_attacker_character_id
                .as_ref()
                .map_or("none", tme_rules::CharacterId::as_str)
        )],
        Event::NpcGrudgeEstablished {
            npc_actor_id,
            attacker_actor_id,
            ..
        } => vec![format!(
            "{npc_actor_id} now retaliates against {attacker_actor_id}"
        )],
        Event::AlignmentChanged {
            actor_id,
            before,
            after,
            ..
        } => vec![format!(
            "{actor_id} alignment: {} -> {}",
            alignment_label(*before),
            alignment_label(*after)
        )],
        Event::KarmaChanged {
            actor_id,
            before,
            after,
            ..
        } => vec![format!("{actor_id} karma: {before} -> {after}")],
        Event::AccountMarkAssessed {
            killer_actor_id,
            victim_actor_id,
            assessed,
            ..
        } => vec![format!(
            "account mark assessed: killer={killer_actor_id} victim={victim_actor_id} add={assessed}"
        )],
        Event::ClassDemoted {
            actor_id,
            from_class_id,
            to_class_id,
            ..
        } => vec![format!(
            "{actor_id} demoted: {from_class_id} -> {to_class_id}"
        )],
        Event::QuestStateChanged {
            character_id,
            quest_id,
            before_stage_id,
            after_stage_id,
        } => vec![format!(
            "quest {quest_id} for {}: {} -> {after_stage_id}",
            character_id.as_str(),
            before_stage_id.as_deref().unwrap_or("unstarted")
        )],
        Event::TransactionCommitted {
            actor,
            source,
            costs,
            rewards,
            ..
        } => match source {
            tme_rules::TransactionSourceV1::ServiceTransaction { transaction_id, .. } => {
                vec![format!(
                    "{actor} completes service transaction {transaction_id}."
                )]
            }
            tme_rules::TransactionSourceV1::SkillTraining { .. }
            | tme_rules::TransactionSourceV1::SpellLearning { .. }
            | tme_rules::TransactionSourceV1::ClassPromotion { .. } => Vec::new(),
            tme_rules::TransactionSourceV1::MerchantPurchase {
                item_instance_ids, ..
            } => vec![format!(
                "{actor} completes purchase of {}.",
                item_instance_ids.join(", ")
            )],
            tme_rules::TransactionSourceV1::MerchantSale {
                item_instance_id, ..
            } => vec![format!("{actor} completes sale of {item_instance_id}.")],
            tme_rules::TransactionSourceV1::ItemService {
                operation,
                item_instance_id,
                ..
            } => vec![format!(
                "{actor} completes {} for {item_instance_id}.",
                operation.label()
            )],
            tme_rules::TransactionSourceV1::RestorationService { operation_id, .. } => {
                vec![format!(
                    "{actor} completes restoration service {operation_id}."
                )]
            }
            tme_rules::TransactionSourceV1::NpcInteraction {
                npc_actor_id,
                interaction_id,
            } => vec![
                format!("{actor} completes NPC interaction {interaction_id} with {npc_actor_id}."),
                format!(
                    "  costs: {}",
                    serde_json::to_string(costs)
                        .expect("transaction cost receipts should serialize")
                ),
                format!(
                    "  rewards: {}",
                    serde_json::to_string(rewards)
                        .expect("transaction reward receipts should serialize")
                ),
            ],
            tme_rules::TransactionSourceV1::BankDeposit {
                bank_id,
                gold_pile_id,
                ..
            } => vec![format!(
                "{actor} completes bank deposit of {gold_pile_id} at {bank_id}."
            )],
            tme_rules::TransactionSourceV1::BankWithdrawal {
                bank_id, amount, ..
            } => vec![format!(
                "{actor} completes withdrawal of {amount} gold from {bank_id}."
            )],
        },
        Event::FinalState { actors } => {
            let mut lines = vec!["final state".to_string()];
            lines.extend(actors.iter().map(|actor| {
                let status = life_state_view_label(&actor.life_state);
                let class_tag = actor
                    .character_identity
                    .as_ref()
                    .map(|ci| format!(" ({})", ci.display_class))
                    .unwrap_or_default();
                let ecology_identity = if actor.id.as_str().starts_with("ecology:") {
                    format!(" [{}]", actor.id)
                } else {
                    String::new()
                };
                format!(
                    "{}{class_tag}{ecology_identity} at {} hp={} {status}",
                    actor.name,
                    actor.location.label(),
                    actor.hp
                )
            }));
            lines
        }
    }
}

fn role_label(kind: ActorKind) -> &'static str {
    match kind {
        ActorKind::Player => "player",
        ActorKind::Monster => "monster",
        ActorKind::Npc => "npc",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tme_rules::{
        CharacterAlignment, Coord, Event, ItemConsumptionReason, SocialAlignmentSource,
        SocialBehavior, SocialNature, SocialOwnerRelation, SocialProfile,
    };

    fn item_view(item_instance_id: &str, name: &str) -> tme_rules::ItemInstanceViewV1 {
        tme_rules::ItemInstanceViewV1 {
            item_instance_id: item_instance_id.to_string(),
            item_definition_id: item_instance_id.to_string(),
            name: name.to_string(),
            quantity: 1,
            identified: false,
            appraised: false,
            known_unit_value_gold: None,
            known_stack_value_gold: None,
            unit_burden: 1,
            stack_burden: 1,
            binding: tme_rules::ItemBindingViewV1::Unrestricted,
            bow_readiness: None,
        }
    }

    fn positioned_item(
        item_instance_id: &str,
        name: &str,
        position: tme_rules::CarriedPosition,
    ) -> tme_rules::PositionedItemViewV1 {
        tme_rules::PositionedItemViewV1 {
            item: item_view(item_instance_id, name),
            position,
            category: None,
            valid_placements: vec![tme_rules::ItemPlacementKind::Sack],
            capability: None,
            armor: None,
        }
    }

    #[test]
    fn renders_exact_item_relocation_and_sack_events() {
        let lines = render_events(&[
            Event::ItemRelocated {
                actor_id: "player0".into(),
                actor: "Delver".to_string(),
                item_instance_id: "hemp_rope".to_string(),
                item_definition_id: "hemp_rope".to_string(),
                item: "Hemp Rope".to_string(),
                quantity: 1,
                from: tme_rules::ItemLocationViewV1::Ground {
                    location: tme_rules::WorldPosition::new(
                        "realm_0",
                        "room_0",
                        tme_rules::Coord { x: 1, y: 1 },
                    ),
                },
                to: tme_rules::ItemLocationViewV1::Carried {
                    actor_id: "player0".into(),
                    position: tme_rules::CarriedPosition::SackItem1,
                },
                reason: tme_rules::ItemRelocationReason::PlayerMove,
                loot_claim: None,
            },
            Event::ItemRelocated {
                actor_id: "player0".into(),
                actor: "Delver".to_string(),
                item_instance_id: "hemp_rope".to_string(),
                item_definition_id: "hemp_rope".to_string(),
                item: "Hemp Rope".to_string(),
                quantity: 1,
                from: tme_rules::ItemLocationViewV1::Carried {
                    actor_id: "player0".into(),
                    position: tme_rules::CarriedPosition::SackItem1,
                },
                to: tme_rules::ItemLocationViewV1::Ground {
                    location: tme_rules::WorldPosition::new(
                        "realm_0",
                        "room_0",
                        tme_rules::Coord { x: 2, y: 1 },
                    ),
                },
                reason: tme_rules::ItemRelocationReason::PlayerMove,
                loot_claim: None,
            },
            Event::SackShown {
                actor_id: "player0".into(),
                actor: "Delver".to_string(),
                items: vec![
                    positioned_item(
                        "hemp_rope",
                        "Hemp Rope",
                        tme_rules::CarriedPosition::SackItem1,
                    ),
                    positioned_item(
                        "waterskin",
                        "Waterskin",
                        tme_rules::CarriedPosition::SackItem2,
                    ),
                ],
                gold: 3,
            },
            Event::SackShown {
                actor_id: "player0".into(),
                actor: "Delver".to_string(),
                items: vec![],
                gold: 0,
            },
        ]);

        assert_eq!(
            lines,
            vec![
                "Delver moved Hemp Rope from (1,1) to sack_item_1".to_string(),
                "Delver moved Hemp Rope from sack_item_1 to (2,1)".to_string(),
                "Delver's sack:".to_string(),
                "  sack_item_1: Hemp Rope".to_string(),
                "  sack_item_2: Waterskin".to_string(),
                "  gold: 3".to_string(),
                "Delver's sack is empty".to_string(),
            ]
        );
    }

    #[test]
    fn renders_offer_creation_refusal_and_reserved_hand_return_truthfully() {
        let sender_character_id: tme_rules::CharacterId =
            serde_json::from_value(serde_json::json!("character:sender")).unwrap();
        let recipient_character_id: tme_rules::CharacterId =
            serde_json::from_value(serde_json::json!("character:recipient")).unwrap();
        let lines = render_events(&[
            Event::ItemOfferCreated {
                actor_id: "sender_actor".into(),
                actor: "Sender".to_string(),
                item_instance_id: "field_case".to_string(),
                item_definition_id: "field_case".to_string(),
                item: "Field Case".to_string(),
                sender_character_id: sender_character_id.clone(),
                recipient_character_id: recipient_character_id.clone(),
                source_position: tme_rules::CarriedPosition::RightHand,
            },
            Event::ItemRelocated {
                actor_id: "recipient_actor".into(),
                actor: "Recipient".to_string(),
                item_instance_id: "field_case".to_string(),
                item_definition_id: "field_case".to_string(),
                item: "Field Case".to_string(),
                quantity: 1,
                from: tme_rules::ItemLocationViewV1::Offered {
                    sender_character_id: sender_character_id.clone(),
                    recipient_character_id: recipient_character_id.clone(),
                    source_position: tme_rules::CarriedPosition::RightHand,
                },
                to: tme_rules::ItemLocationViewV1::Carried {
                    actor_id: "sender_actor".into(),
                    position: tme_rules::CarriedPosition::RightHand,
                },
                reason: tme_rules::ItemRelocationReason::OfferReturned,
                loot_claim: None,
            },
            Event::ItemOfferCompleted {
                actor_id: "recipient_actor".into(),
                actor: "Recipient".to_string(),
                item_instance_id: "field_case".to_string(),
                item_definition_id: "field_case".to_string(),
                item: "Field Case".to_string(),
                sender_character_id,
                recipient_character_id,
                destination: tme_rules::CarriedPosition::RightHand,
                reason: tme_rules::ItemOfferCompletionReasonV1::Refused,
            },
        ]);
        assert_eq!(
            lines,
            [
                "Sender offered Field Case from right_hand to character character:recipient",
                "Field Case returned to sender_actor's right_hand",
                "Recipient completed the Field Case offer: refused to right_hand",
            ]
        );
    }

    #[test]
    fn renders_npc_transaction_receipts_in_exact_order() {
        let character_id: tme_rules::CharacterId =
            serde_json::from_value(serde_json::json!("character:harbor:primary")).unwrap();
        let lines = render_events(&[Event::TransactionCommitted {
            actor_id: "player".into(),
            actor: "Delver".to_string(),
            source: tme_rules::TransactionSourceV1::NpcInteraction {
                npc_actor_id: "wayfinder".into(),
                interaction_id: "offer_signal_token".to_string(),
            },
            costs: vec![tme_rules::TransactionCostReceiptV1::SelectedCarriedItem {
                item_instance_id: "player_signal_token".to_string(),
                item_definition_id: "signal_token".to_string(),
                consumed_quantity: 1,
                remaining_quantity: 0,
            }],
            rewards: vec![
                tme_rules::TransactionRewardReceiptV1::NpcInteraction {
                    npc_actor_id: "wayfinder".into(),
                    interaction_id: "offer_signal_token".to_string(),
                    outcome: tme_rules::NpcInteractionOutcome::BeginFollow,
                },
                tme_rules::TransactionRewardReceiptV1::QuestStage {
                    character_id,
                    quest_id: "harbor_escort".to_string(),
                    before_stage_id: Some("awaiting_token".to_string()),
                    after_stage_id: "escorting_guide".to_string(),
                },
            ],
        }]);

        assert_eq!(
            lines,
            [
                "Delver completes NPC interaction offer_signal_token with wayfinder.",
                "  costs: [{\"kind\":\"selected_carried_item\",\"item_instance_id\":\"player_signal_token\",\"item_definition_id\":\"signal_token\",\"consumed_quantity\":1,\"remaining_quantity\":0}]",
                "  rewards: [{\"kind\":\"npc_interaction\",\"npc_actor_id\":\"wayfinder\",\"interaction_id\":\"offer_signal_token\",\"outcome\":{\"kind\":\"begin_follow\"}},{\"kind\":\"quest_stage\",\"character_id\":\"character:harbor:primary\",\"quest_id\":\"harbor_escort\",\"before_stage_id\":\"awaiting_token\",\"after_stage_id\":\"escorting_guide\"}]",
            ]
        );
    }

    #[test]
    fn renders_actor_defeat_corpse_and_ghost_events() {
        let corpse_id = tme_rules::CorpseId::parse("corpse:1").unwrap();
        let lines = render_events(&[
            Event::ActorDefeated {
                actor_id: "player0".into(),
                actor: "Delver".to_string(),
                kind: tme_rules::ActorKind::Player,
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "room_0",
                    tme_rules::Coord { x: 0, y: 0 },
                ),
                cause: tme_rules::DeathCause::Physical,
                credited_actor_id: None,
                loot_claim: None,
            },
            Event::CorpseCreated {
                corpse_id: corpse_id.clone(),
                origin_actor_id: "player0".into(),
                origin_character_id: None,
                origin_kind: tme_rules::ActorKind::Player,
                origin_name: "Delver".to_string(),
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "room_0",
                    tme_rules::Coord { x: 0, y: 0 },
                ),
                created_at: tme_rules::LogicalTime::FIRST,
                sequence: 1,
                loot_claim: None,
            },
            Event::ActorLifeStateChanged {
                actor_id: "player0".into(),
                actor: "Delver".to_string(),
                from: tme_rules::ActorLifeState::Alive,
                to: tme_rules::ActorLifeState::Ghost {
                    corpse_id,
                    defeated_at: tme_rules::LogicalTime::FIRST,
                },
            },
        ]);

        assert_eq!(
            lines,
            vec![
                "Delver was defeated: cause=physical".to_string(),
                "corpse corpse:1 created for Delver at realm_0/room_0:0,0".to_string(),
                "Delver life state: alive -> ghost".to_string(),
            ]
        );
    }

    #[test]
    fn renders_defeat_drop_events() {
        let lines = render_events(&[
            Event::ItemRelocated {
                actor_id: "player0".into(),
                actor: "Delver".to_string(),
                item_instance_id: "hemp_rope".to_string(),
                item_definition_id: "hemp_rope".to_string(),
                item: "Hemp Rope".to_string(),
                quantity: 1,
                from: tme_rules::ItemLocationViewV1::Carried {
                    actor_id: "player0".into(),
                    position: tme_rules::CarriedPosition::SackItem1,
                },
                to: tme_rules::ItemLocationViewV1::Ground {
                    location: tme_rules::WorldPosition::new(
                        "realm_0",
                        "room_0",
                        tme_rules::Coord { x: 1, y: 1 },
                    ),
                },
                reason: tme_rules::ItemRelocationReason::DeathDrop,
                loot_claim: None,
            },
            Event::ItemRelocated {
                actor_id: "player0".into(),
                actor: "Delver".to_string(),
                item_instance_id: "iron_dirk".to_string(),
                item_definition_id: "iron_dirk".to_string(),
                item: "Iron Dirk".to_string(),
                quantity: 1,
                from: tme_rules::ItemLocationViewV1::Carried {
                    actor_id: "player0".into(),
                    position: tme_rules::CarriedPosition::RightHand,
                },
                to: tme_rules::ItemLocationViewV1::Ground {
                    location: tme_rules::WorldPosition::new(
                        "realm_0",
                        "room_0",
                        tme_rules::Coord { x: 1, y: 1 },
                    ),
                },
                reason: tme_rules::ItemRelocationReason::DeathDrop,
                loot_claim: None,
            },
        ]);

        assert_eq!(
            lines,
            vec![
                "Delver's Hemp Rope fell to the ground at (1,1)".to_string(),
                "Delver's Iron Dirk fell to the ground at (1,1)".to_string(),
            ]
        );
    }

    #[test]
    fn renders_resource_recovery_events() {
        let lines = render_events(&[Event::ResourceRegenerated {
            actor_id: "player0".into(),
            actor: "Delver".to_string(),
            resource: tme_rules::ResourceKind::Hp,
            activity: tme_rules::ResourceActivity::Inactive,
            boundary_at: tme_rules::LogicalTime::new(2),
            base_amount: 1,
            multiplier_numerator: 1,
            multiplier_denominator: 1,
            rounding: tme_rules::MagicArithmeticRounding::Down,
            modifier_item_instance_id: None,
            modifier_item_definition_id: None,
            modifier_item: None,
            modifier_item_position: None,
            amount: 1,
            current: 9,
            maximum: 12,
        }]);

        assert_eq!(
            lines,
            vec!["Delver regenerated 1 Hp (9/12, Inactive, time 2)".to_string()]
        );
    }

    #[test]
    fn renders_door_and_transition_events() {
        let lines = render_events(&[
            Event::DoorOpened {
                actor_id: "player0".into(),
                actor: "Delver".to_string(),
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "entrance_hall",
                    Coord { x: 4, y: 1 },
                ),
            },
            Event::WorldTransition {
                actor_id: "player0".into(),
                actor: "Delver".to_string(),
                from: tme_rules::WorldPosition::new(
                    "realm_0",
                    "entrance_hall",
                    Coord { x: 4, y: 1 },
                ),
                to: tme_rules::WorldPosition::new("realm_0", "guard_post", Coord { x: 1, y: 1 }),
                navigation: NavigationKind::Door,
            },
            Event::DoorClosed {
                actor_id: "player0".into(),
                actor: "Delver".to_string(),
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "guard_post",
                    Coord { x: 0, y: 1 },
                ),
            },
        ]);

        assert_eq!(
            lines,
            vec![
                "--- Door opened: realm_0/entrance_hall:4,1 ---".to_string(),
                "--- Delver transitions via Door: realm_0/entrance_hall:4,1 -> realm_0/guard_post:1,1 ---".to_string(),
                "--- Door closed: realm_0/guard_post:0,1 ---".to_string(),
            ]
        );
    }

    #[test]
    fn renders_balm_events() {
        let lines = render_events(&[
            Event::ItemConsumed {
                actor_id: "player0".into(),
                actor: "Delver".to_string(),
                item_instance_id: "healing_balm".to_string(),
                item_definition_id: "healing_balm".to_string(),
                item: "Healing Balm".to_string(),
                quantity_consumed: 1,
                remaining_quantity: 0,
                reason: ItemConsumptionReason::Drink,
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "room_0",
                    tme_rules::Coord { x: 0, y: 0 },
                ),
            },
            Event::BalmHealed {
                actor_id: "player0".into(),
                actor: "Delver".to_string(),
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "room_0",
                    tme_rules::Coord { x: 0, y: 0 },
                ),
                amount: 2,
                hp: 6,
            },
        ]);

        assert_eq!(
            lines,
            vec![
                "Delver drinks the Healing Balm and the empty bottle shatters".to_string(),
                "The balm knits Delver's wounds: regained 2 hp (6 hp)".to_string(),
            ]
        );
    }

    #[test]
    fn renders_item_consumption_reasons() {
        let lines = render_events(&[Event::ItemConsumed {
            actor_id: "player0".into(),
            actor: "Delver".to_string(),
            item_instance_id: "healing_balm".to_string(),
            item_definition_id: "healing_balm".to_string(),
            item: "Healing Balm".to_string(),
            quantity_consumed: 1,
            remaining_quantity: 0,
            reason: ItemConsumptionReason::Drink,
            location: tme_rules::WorldPosition::new(
                "realm_0",
                "room_0",
                tme_rules::Coord { x: 0, y: 0 },
            ),
        }]);

        assert_eq!(
            lines,
            vec!["Delver drinks the Healing Balm and the empty bottle shatters".to_string(),]
        );
    }

    #[test]
    fn renders_spell_learned_events() {
        let lines = render_events(&[Event::SpellLearned {
            actor_id: "player0".into(),
            actor: "Wiz".to_string(),
            spell_id: "spark".to_string(),
            spell_name: "Spark".to_string(),
            lane: "wizard_magic".to_string(),
            skill_requirement: 1,
            learned_at_level: 2,
            gold_cost: 25,
            trainer_service_id: "wizard_trainer".to_string(),
            trainer: "Wizard Trainer".to_string(),
            spell_book_item_instance_id: "spell_book".to_string(),
            spell_book_item_definition_id: "spell_book".to_string(),
            spell_book: "Spell Book".to_string(),
            spell_book_character_id: "character:player0".to_string(),
        }]);

        assert_eq!(
            lines,
            vec![
                "Wizard Trainer records Spark (wizard_magic) in Wiz's retained Spell Book for 25 gold"
                    .to_string()
            ]
        );
    }

    #[test]
    fn renders_dx_magic_attempt_practice_and_reward_receipts() {
        let lines = render_events(&[
            Event::ThaumAboveSkillEvaluated {
                actor_id: "player0".into(),
                actor: "Thaum".to_string(),
                spell_id: "spark".to_string(),
                spell_name: "Spark".to_string(),
                track_id: "thaumaturge_magic".to_string(),
                current_skill_level: 1,
                skill_requirement: 3,
                gap: 2,
                roll_denominator: 20,
                success_threshold: 18,
                roll: 17,
                success: true,
            },
            Event::MagicPracticeEvaluated {
                actor_id: "player0".into(),
                actor: "Wiz".to_string(),
                current_class_id: "wizard".to_string(),
                spell_id: "spark".to_string(),
                spell_name: "Spark".to_string(),
                track_id: "wizard_magic".to_string(),
                mp_cost: 3,
                cast_class: tme_rules::SpellCastClass::Character,
                primary_attribute: Some(tme_rules::MagicPrimaryAttribute::Intelligence),
                primary_attribute_value: Some(14),
                base_raw_points: 3,
                primary_attribute_bonus_raw_points: 1,
                total_raw_points: 4,
                risk_applied: false,
                reason: "eligible_successful_cast".to_string(),
            },
            Event::DefeatRewardEvaluated {
                target_id: "target".into(),
                target: "Mireling".to_string(),
                authored_experience: 13,
                actual_damage: 3,
                weighted_damage_numerator: 6,
                weighted_damage_denominator: 5,
                available_experience: 5,
                awarded_experience: 5,
                reason: "contribution_shared".to_string(),
            },
        ]);

        assert_eq!(
            lines,
            vec![
                "Thaum's above-skill Spark attempt: gap=2 roll=17 threshold=18 success=true"
                    .to_string(),
                "Wiz's Spark practice on wizard_magic: raw=4 reason=eligible_successful_cast"
                    .to_string(),
                "shared defeat reward for Mireling: available=5 awarded=5 reason=contribution_shared"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn renders_summon_lifecycle_events() {
        let lines = render_events(&[
            Event::ActorSummoned {
                caster_id: "player0".into(),
                caster: "Wiz".to_string(),
                spell_id: "call_echo".to_string(),
                spell_name: "Call Echo".to_string(),
                actor_id: "summon:call_echo:1:echo_guardian".into(),
                actor: "Echo Guardian".to_string(),
                template_id: "echo_guardian".to_string(),
                owner_id: "player0".into(),
                social: SocialProfile {
                    alignment_source: SocialAlignmentSource::Inherent {
                        alignment: CharacterAlignment::Lawful,
                    },
                    nature: SocialNature::Other,
                    behavior: SocialBehavior::AlignmentCreature,
                    owner_relation: SocialOwnerRelation::Summoner,
                },
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "start",
                    tme_rules::Coord { x: 2, y: 1 },
                ),
                remaining_rounds: Some(2),
            },
            Event::SummonExpired {
                actor_id: "summon:call_echo:1:echo_guardian".into(),
                actor: "Echo Guardian".to_string(),
                instance_id: "summon:call_echo:1:echo_guardian".into(),
                owner_id: "player0".into(),
                source_spell_id: "call_echo".to_string(),
                template_id: "echo_guardian".to_string(),
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "start",
                    tme_rules::Coord { x: 2, y: 1 },
                ),
            },
        ]);

        assert_eq!(
            lines,
            vec![
                "--- Wiz summoned Echo Guardian at realm_0/start:2,1 for 2 rounds ---".to_string(),
                "--- Echo Guardian faded from realm_0/start:2,1 ---".to_string(),
            ]
        );
    }

    #[test]
    fn renders_tile_effect_events() {
        let lines = render_events(&[
            Event::TileEffectApplied {
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "room_0",
                    tme_rules::Coord { x: 2, y: 1 },
                ),
                instance_id: "tile:web:1".to_string(),
                effect_id: "web_field".to_string(),
                source_kind: "spell".to_string(),
                source_id: "web_field".to_string(),
                kind: "terrain_overlay".to_string(),
                tags: vec!["web".to_string()],
                potency: 0,
                remaining_rounds: Some(2),
                passability: Some("hindered".to_string()),
                sight: Some("obscured".to_string()),
                hazard: None,
                move_cost: Some(2),
            },
            Event::TileEffectTicked {
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "room_0",
                    tme_rules::Coord { x: 2, y: 1 },
                ),
                instance_id: "tile:web:1".to_string(),
                effect_id: "web_field".to_string(),
                kind: "terrain_overlay".to_string(),
                tags: vec!["web".to_string()],
                potency: 0,
                remaining_rounds: Some(1),
            },
            Event::TileEffectDamaged {
                actor_id: "target".into(),
                actor: "Target".to_string(),
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "room_0",
                    tme_rules::Coord { x: 2, y: 1 },
                ),
                instance_id: "tile:ember:1".to_string(),
                effect_id: "ember_cloud".to_string(),
                kind: "terrain_overlay".to_string(),
                tags: vec!["fire".to_string()],
                damage: 2,
                hp: 6,
            },
            Event::TileEffectExpired {
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "room_0",
                    tme_rules::Coord { x: 2, y: 1 },
                ),
                instance_id: "tile:web:1".to_string(),
                effect_id: "web_field".to_string(),
                kind: "terrain_overlay".to_string(),
            },
            Event::TileEffectRemoved {
                location: tme_rules::WorldPosition::new(
                    "realm_0",
                    "room_0",
                    tme_rules::Coord { x: 2, y: 1 },
                ),
                instance_id: "tile:web:1".to_string(),
                effect_id: "web_field".to_string(),
                kind: "terrain_overlay".to_string(),
                reason: "clear_sight".to_string(),
            },
        ]);

        assert_eq!(
            lines,
            vec![
                "tile effect applied: web_field at realm_0/room_0:2,1 passability=hindered sight=obscured hazard=none cost=2 remaining=2".to_string(),
                "tile effect ticked: web_field at realm_0/room_0:2,1 remaining=1".to_string(),
                "tile effect damaged: Target by ember_cloud damage=2 hp=6".to_string(),
                "tile effect expired: web_field at realm_0/room_0:2,1".to_string(),
                "tile effect removed: web_field at realm_0/room_0:2,1 reason=clear_sight".to_string(),
            ]
        );
    }
}
