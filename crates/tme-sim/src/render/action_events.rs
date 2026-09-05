use super::*;

pub(super) fn render(event: &Event) -> Vec<String> {
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

        _ => unreachable!("event family is selected by the exhaustive dispatcher"),
    }
}
