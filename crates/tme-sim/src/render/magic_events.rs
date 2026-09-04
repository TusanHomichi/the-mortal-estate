use super::*;

pub(super) fn render(event: &Event) -> Vec<String> {
    match event {
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
        _ => unreachable!("event family is selected by the exhaustive dispatcher"),
    }
}
