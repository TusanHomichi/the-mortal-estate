//! Observer-safe feedback projection; private answers stay with their recipient.
use super::*;
mod transactions;
use transactions::*;

impl Engine {
    fn observer_feedback_actor(&self, actor_id: &ActorId) -> Option<ObserverFeedbackActorV1> {
        self.world
            .actor(actor_id)
            .map(|actor| ObserverFeedbackActorV1 {
                actor_id: actor.id.clone(),
                name: actor.name.clone(),
                kind: actor.kind,
            })
    }

    fn observer_feedback_actor_is_visible(
        &self,
        actor_id: &ActorId,
        observer_actor_id: &ActorId,
        visible: &BTreeSet<WorldPosition>,
    ) -> bool {
        actor_id == observer_actor_id
            || self
                .world
                .actor(actor_id)
                .is_some_and(|actor| visible.contains(&actor.location))
    }

    fn observer_feedback_source(
        &self,
        actor_id: &ActorId,
        observer_actor_id: &ActorId,
        visible: &BTreeSet<WorldPosition>,
    ) -> Option<ObserverFeedbackActorV1> {
        self.observer_feedback_actor_is_visible(actor_id, observer_actor_id, visible)
            .then(|| self.observer_feedback_actor(actor_id))
            .flatten()
    }

    fn observer_combat_feedback(
        &self,
        event: &Event,
        observer_actor_id: &ActorId,
        visible: &BTreeSet<WorldPosition>,
    ) -> Result<Option<ObserverFeedbackCueV1>, StepError> {
        let actor_visible = |actor_id: &ActorId| {
            self.observer_feedback_actor_is_visible(actor_id, observer_actor_id, visible)
        };
        let actor = |actor_id: &ActorId| self.observer_feedback_actor(actor_id);
        let source = |actor_id: &ActorId| {
            self.observer_feedback_source(actor_id, observer_actor_id, visible)
        };
        let cue = match event {
            Event::Attacked {
                attacker_id,
                defender_id,
                defender_location,
                mode,
                damage,
                armor_reduction,
                wound_before,
                wound_after,
                defender_hp,
                ..
            } if attacker_id == observer_actor_id || visible.contains(defender_location) => {
                Some(ObserverFeedbackCueV1::PhysicalCombat {
                    source: source(attacker_id),
                    target: actor(defender_id).ok_or_else(|| {
                        StepError::new("physical feedback target disappeared before projection")
                    })?,
                    location: Some(defender_location.clone()),
                    mode: *mode,
                    outcome: ObserverPhysicalOutcomeV1::Hit {
                        damage: *damage,
                        armor_reduction: *armor_reduction,
                        wound_before: *wound_before,
                        wound_after: *wound_after,
                        target_hp: *defender_hp,
                    },
                })
            }
            Event::AttackMissed {
                attacker_id,
                defender_id,
                defender_location,
                mode,
                ..
            } if attacker_id == observer_actor_id || visible.contains(defender_location) => {
                Some(ObserverFeedbackCueV1::PhysicalCombat {
                    source: source(attacker_id),
                    target: actor(defender_id).ok_or_else(|| {
                        StepError::new("physical feedback target disappeared before projection")
                    })?,
                    location: Some(defender_location.clone()),
                    mode: *mode,
                    outcome: ObserverPhysicalOutcomeV1::Missed,
                })
            }
            Event::AttackBlocked {
                attacker_id,
                defender_id,
                defender_location,
                mode,
                ..
            } if attacker_id == observer_actor_id || visible.contains(defender_location) => {
                Some(ObserverFeedbackCueV1::PhysicalCombat {
                    source: source(attacker_id),
                    target: actor(defender_id).ok_or_else(|| {
                        StepError::new("physical feedback target disappeared before projection")
                    })?,
                    location: Some(defender_location.clone()),
                    mode: *mode,
                    outcome: ObserverPhysicalOutcomeV1::Blocked,
                })
            }
            Event::AttackBlockedNoSight {
                attacker_id,
                defender_id,
                mode,
                ..
            } if attacker_id == observer_actor_id
                || (actor_visible(attacker_id) && actor_visible(defender_id)) =>
            {
                Some(ObserverFeedbackCueV1::PhysicalCombat {
                    source: source(attacker_id),
                    target: actor(defender_id).ok_or_else(|| {
                        StepError::new("physical feedback target disappeared before projection")
                    })?,
                    location: None,
                    mode: *mode,
                    outcome: ObserverPhysicalOutcomeV1::NoSight,
                })
            }
            Event::AttackNotReady {
                actor_id,
                target_id,
                current_time,
                ready_at,
                mode,
                ..
            } if actor_id == observer_actor_id
                || (actor_visible(actor_id) && actor_visible(target_id)) =>
            {
                Some(ObserverFeedbackCueV1::PhysicalCombat {
                    source: source(actor_id),
                    target: actor(target_id).ok_or_else(|| {
                        StepError::new("physical feedback target disappeared before projection")
                    })?,
                    location: None,
                    mode: *mode,
                    outcome: ObserverPhysicalOutcomeV1::NotReady {
                        current_time: *current_time,
                        ready_at: *ready_at,
                    },
                })
            }
            Event::WeaponFumbled {
                attacker_id,
                mode,
                result,
                ..
            } if actor_visible(attacker_id) => Some(ObserverFeedbackCueV1::WeaponFumbled {
                actor: actor(attacker_id).ok_or_else(|| {
                    StepError::new("fumble feedback actor disappeared before projection")
                })?,
                mode: *mode,
                result: *result,
            }),
            _ => None,
        };
        Ok(cue)
    }

    fn observer_spell_feedback(
        &self,
        event: &Event,
        observer_actor_id: &ActorId,
        visible: &BTreeSet<WorldPosition>,
    ) -> Result<Option<ObserverFeedbackCueV1>, StepError> {
        let actor_visible = |actor_id: &ActorId| {
            self.observer_feedback_actor_is_visible(actor_id, observer_actor_id, visible)
        };
        let actor = |actor_id: &ActorId| self.observer_feedback_actor(actor_id);
        let source = |actor_id: &ActorId| {
            self.observer_feedback_source(actor_id, observer_actor_id, visible)
        };
        let cue = match event {
            Event::SpellWarmed {
                actor_id,
                spell_id,
                spell_name,
                warmed_at,
                ready_at,
                ..
            } if actor_visible(actor_id) => Some(ObserverFeedbackCueV1::SpellLifecycle {
                actor: actor(actor_id).ok_or_else(|| {
                    StepError::new("spell feedback actor disappeared before projection")
                })?,
                spell_id: spell_id.clone(),
                spell_name: spell_name.clone(),
                state: ObserverSpellLifecycleStateV1::Warmed {
                    warmed_at: *warmed_at,
                    ready_at: *ready_at,
                },
            }),
            Event::WarmedSpellReady {
                actor_id,
                spell_id,
                spell_name,
                ready_at,
                ..
            } if actor_visible(actor_id) => Some(ObserverFeedbackCueV1::SpellLifecycle {
                actor: actor(actor_id).ok_or_else(|| {
                    StepError::new("spell feedback actor disappeared before projection")
                })?,
                spell_id: spell_id.clone(),
                spell_name: spell_name.clone(),
                state: ObserverSpellLifecycleStateV1::Ready {
                    ready_at: *ready_at,
                },
            }),
            Event::SpellCastCommitted {
                actor_id,
                spell_id,
                spell_name,
                mp_cost,
                stamina_cost,
                ..
            } if actor_visible(actor_id) => Some(ObserverFeedbackCueV1::SpellLifecycle {
                actor: actor(actor_id).ok_or_else(|| {
                    StepError::new("spell feedback actor disappeared before projection")
                })?,
                spell_id: spell_id.clone(),
                spell_name: spell_name.clone(),
                state: ObserverSpellLifecycleStateV1::Cast {
                    mp_cost: (actor_id == observer_actor_id)
                        .then_some(*mp_cost)
                        .flatten(),
                    stamina_cost: (actor_id == observer_actor_id)
                        .then_some(*stamina_cost)
                        .flatten(),
                },
            }),
            Event::SpellFizzled {
                actor_id,
                spell_id,
                spell_name,
                cause,
                ..
            } if actor_visible(actor_id) => Some(ObserverFeedbackCueV1::SpellLifecycle {
                actor: actor(actor_id).ok_or_else(|| {
                    StepError::new("spell feedback actor disappeared before projection")
                })?,
                spell_id: spell_id.clone(),
                spell_name: spell_name.clone(),
                state: ObserverSpellLifecycleStateV1::Fizzled {
                    reason: match cause {
                        SpellFizzleCause::Replaced { .. } => ObserverSpellFizzleReasonV1::Replaced,
                        SpellFizzleCause::Canceled => ObserverSpellFizzleReasonV1::Canceled,
                        SpellFizzleCause::Rest => ObserverSpellFizzleReasonV1::Rest,
                        SpellFizzleCause::HealingBalm => ObserverSpellFizzleReasonV1::HealingBalm,
                        SpellFizzleCause::Damage { .. } => ObserverSpellFizzleReasonV1::Damage,
                        SpellFizzleCause::Defeat => ObserverSpellFizzleReasonV1::Defeat,
                    },
                },
            }),
            Event::SpellCastFailed {
                actor_id,
                spell_id,
                spell_name,
                failure,
                mp_cost,
                stamina_cost,
                ..
            } if actor_visible(actor_id) => Some(ObserverFeedbackCueV1::SpellLifecycle {
                actor: actor(actor_id).ok_or_else(|| {
                    StepError::new("spell feedback actor disappeared before projection")
                })?,
                spell_id: spell_id.clone(),
                spell_name: spell_name.clone(),
                state: ObserverSpellLifecycleStateV1::Failed {
                    reason: match failure {
                        SpellCastFailure::InvalidPath { .. } => {
                            ObserverSpellFailureReasonV1::InvalidPath
                        }
                        SpellCastFailure::AboveSkillAttempt => {
                            ObserverSpellFailureReasonV1::AboveSkillAttempt
                        }
                    },
                    mp_cost: (actor_id == observer_actor_id)
                        .then_some(*mp_cost)
                        .flatten(),
                    stamina_cost: (actor_id == observer_actor_id)
                        .then_some(*stamina_cost)
                        .flatten(),
                },
            }),
            Event::SpellDamaged {
                caster_id,
                spell_id,
                spell_name,
                target_id,
                location,
                damage,
                hp,
                ..
            } if target_id == observer_actor_id || visible.contains(location) => {
                Some(ObserverFeedbackCueV1::SpellImpact {
                    source: source(caster_id),
                    spell_id: spell_id.clone(),
                    spell_name: spell_name.clone(),
                    target: actor(target_id).ok_or_else(|| {
                        StepError::new("spell impact target disappeared before projection")
                    })?,
                    location: location.clone(),
                    outcome: ObserverSpellImpactOutcomeV1::Damaged {
                        damage: *damage,
                        target_hp: *hp,
                    },
                })
            }
            Event::SpellHealed {
                caster_id,
                spell_id,
                spell_name,
                target_id,
                location,
                amount,
                hp,
                ..
            } if target_id == observer_actor_id || visible.contains(location) => {
                Some(ObserverFeedbackCueV1::SpellImpact {
                    source: source(caster_id),
                    spell_id: spell_id.clone(),
                    spell_name: spell_name.clone(),
                    target: actor(target_id).ok_or_else(|| {
                        StepError::new("spell impact target disappeared before projection")
                    })?,
                    location: location.clone(),
                    outcome: ObserverSpellImpactOutcomeV1::Healed {
                        amount: *amount,
                        target_hp: *hp,
                    },
                })
            }
            _ => None,
        };
        Ok(cue)
    }

    fn observer_spell_resource_feedback(
        &self,
        event: &Event,
        observer_actor_id: &ActorId,
    ) -> Result<Vec<ObserverFeedbackCueV1>, StepError> {
        let (actor_id, mp_cost, stamina_cost) = match event {
            Event::SpellCastCommitted {
                actor_id,
                mp_cost,
                stamina_cost,
                ..
            }
            | Event::SpellCastFailed {
                actor_id,
                mp_cost,
                stamina_cost,
                ..
            } if actor_id == observer_actor_id => (actor_id, *mp_cost, *stamina_cost),
            _ => return Ok(Vec::new()),
        };
        let actor = self
            .world
            .actor(actor_id)
            .ok_or_else(|| StepError::new("spell resource actor disappeared before projection"))?;
        let character = actor.character.as_ref().ok_or_else(|| {
            StepError::new("controlled spell resource actor has no character sheet")
        })?;
        let feedback_actor = self.observer_feedback_actor(actor_id).ok_or_else(|| {
            StepError::new("spell resource feedback actor disappeared before projection")
        })?;
        let mut cues = Vec::with_capacity(2);
        if let Some(amount) = mp_cost.filter(|amount| *amount > 0) {
            cues.push(ObserverFeedbackCueV1::Resource {
                actor: feedback_actor.clone(),
                resource: crate::model::ResourceKind::Mp,
                reason: ObserverResourceReasonV1::SpellCost,
                amount,
                current: Some(actor.mp),
                maximum: character.resources.max_mp,
            });
        }
        if let Some(amount) = stamina_cost.filter(|amount| *amount > 0) {
            cues.push(ObserverFeedbackCueV1::Resource {
                actor: feedback_actor,
                resource: crate::model::ResourceKind::Stamina,
                reason: ObserverResourceReasonV1::SpellCost,
                amount,
                current: Some(actor.stamina),
                maximum: actor.max_stamina(),
            });
        }
        Ok(cues)
    }

    fn observer_effect_feedback(
        &self,
        event: &Event,
        observer_actor_id: &ActorId,
        visible: &BTreeSet<WorldPosition>,
    ) -> Result<Option<ObserverFeedbackCueV1>, StepError> {
        let actor = |actor_id: &ActorId| {
            self.observer_feedback_actor(actor_id)
                .ok_or_else(|| StepError::new("effect actor disappeared before projection"))
        };
        let cue = match event {
            Event::EffectApplied {
                actor_id,
                location,
                effect_id,
                kind,
                remaining_rounds,
                ..
            } if actor_id == observer_actor_id || visible.contains(location) => {
                Some(ObserverFeedbackCueV1::ActorEffect {
                    actor: actor(actor_id)?,
                    location: location.clone(),
                    effect_id: effect_id.clone(),
                    effect_kind: kind.clone(),
                    change: ObserverEffectChangeV1::Applied {
                        remaining_rounds: *remaining_rounds,
                    },
                })
            }
            Event::EffectTicked {
                actor_id,
                location,
                effect_id,
                kind,
                remaining_rounds,
                ..
            } if actor_id == observer_actor_id || visible.contains(location) => {
                Some(ObserverFeedbackCueV1::ActorEffect {
                    actor: actor(actor_id)?,
                    location: location.clone(),
                    effect_id: effect_id.clone(),
                    effect_kind: kind.clone(),
                    change: ObserverEffectChangeV1::Ticked {
                        remaining_rounds: *remaining_rounds,
                    },
                })
            }
            Event::EffectExpired {
                actor_id,
                location,
                effect_id,
                kind,
                ..
            } if actor_id == observer_actor_id || visible.contains(location) => {
                Some(ObserverFeedbackCueV1::ActorEffect {
                    actor: actor(actor_id)?,
                    location: location.clone(),
                    effect_id: effect_id.clone(),
                    effect_kind: kind.clone(),
                    change: ObserverEffectChangeV1::Expired,
                })
            }
            Event::EffectRemoved {
                actor_id,
                location,
                effect_id,
                kind,
                ..
            } if actor_id == observer_actor_id || visible.contains(location) => {
                Some(ObserverFeedbackCueV1::ActorEffect {
                    actor: actor(actor_id)?,
                    location: location.clone(),
                    effect_id: effect_id.clone(),
                    effect_kind: kind.clone(),
                    change: ObserverEffectChangeV1::Removed,
                })
            }
            Event::TileEffectApplied {
                location,
                effect_id,
                kind,
                remaining_rounds,
                ..
            } if visible.contains(location) => Some(ObserverFeedbackCueV1::TileEffect {
                location: location.clone(),
                effect_id: effect_id.clone(),
                effect_kind: kind.clone(),
                change: ObserverEffectChangeV1::Applied {
                    remaining_rounds: *remaining_rounds,
                },
            }),
            Event::TileEffectTicked {
                location,
                effect_id,
                kind,
                remaining_rounds,
                ..
            } if visible.contains(location) => Some(ObserverFeedbackCueV1::TileEffect {
                location: location.clone(),
                effect_id: effect_id.clone(),
                effect_kind: kind.clone(),
                change: ObserverEffectChangeV1::Ticked {
                    remaining_rounds: *remaining_rounds,
                },
            }),
            Event::TileEffectExpired {
                location,
                effect_id,
                kind,
                ..
            } if visible.contains(location) => Some(ObserverFeedbackCueV1::TileEffect {
                location: location.clone(),
                effect_id: effect_id.clone(),
                effect_kind: kind.clone(),
                change: ObserverEffectChangeV1::Expired,
            }),
            Event::TileEffectRemoved {
                location,
                effect_id,
                kind,
                ..
            } if visible.contains(location) => Some(ObserverFeedbackCueV1::TileEffect {
                location: location.clone(),
                effect_id: effect_id.clone(),
                effect_kind: kind.clone(),
                change: ObserverEffectChangeV1::Removed,
            }),
            Event::EffectDamaged {
                actor_id,
                location,
                effect_id,
                kind,
                damage,
                hp,
                ..
            }
            | Event::TileEffectDamaged {
                actor_id,
                location,
                effect_id,
                kind,
                damage,
                hp,
                ..
            } if actor_id == observer_actor_id || visible.contains(location) => {
                Some(ObserverFeedbackCueV1::EffectDamage {
                    actor: actor(actor_id)?,
                    location: location.clone(),
                    effect_id: effect_id.clone(),
                    effect_kind: kind.clone(),
                    damage: *damage,
                    actor_hp: *hp,
                })
            }
            _ => None,
        };
        Ok(cue)
    }

    fn observer_private_feedback(
        &self,
        event: &Event,
        observer_actor_id: &ActorId,
        observer_character_id: &crate::model::CharacterId,
    ) -> Result<Option<ObserverFeedbackCueV1>, StepError> {
        let controlled_actor = |actor_id: &ActorId| -> Result<ObserverFeedbackActorV1, StepError> {
            self.observer_feedback_actor(actor_id).ok_or_else(|| {
                StepError::new("controlled feedback actor disappeared before projection")
            })
        };
        let cue = match event {
            Event::SkillCritiqued {
                actor_id,
                service_id,
                track_id,
                track_display,
                level,
                critique_rank,
                level_title,
                ..
            } if actor_id == observer_actor_id => Some(ObserverFeedbackCueV1::SkillCritique {
                service_id: service_id.clone(),
                track_id: track_id.clone(),
                track_display: track_display.clone(),
                level: *level,
                critique_rank: *critique_rank,
                level_title: level_title.clone(),
            }),
            Event::MovementStaminaSpent {
                actor_id,
                amount,
                stamina,
                max_stamina,
                ..
            } if actor_id == observer_actor_id => Some(ObserverFeedbackCueV1::Resource {
                actor: controlled_actor(actor_id)?,
                resource: crate::model::ResourceKind::Stamina,
                reason: ObserverResourceReasonV1::MovementSpend,
                amount: *amount,
                current: Some(*stamina),
                maximum: *max_stamina,
            }),
            Event::PhysicalStaminaSpent {
                actor_id,
                amount,
                stamina,
                max_stamina,
                ..
            } if actor_id == observer_actor_id => Some(ObserverFeedbackCueV1::Resource {
                actor: controlled_actor(actor_id)?,
                resource: crate::model::ResourceKind::Stamina,
                reason: ObserverResourceReasonV1::PhysicalSpend,
                amount: *amount,
                current: Some(*stamina),
                maximum: *max_stamina,
            }),
            Event::ResourceRegenerated {
                actor_id,
                resource,
                amount,
                current,
                maximum,
                ..
            } if actor_id == observer_actor_id => Some(ObserverFeedbackCueV1::Resource {
                actor: controlled_actor(actor_id)?,
                resource: *resource,
                reason: ObserverResourceReasonV1::Regenerated,
                amount: *amount,
                current: Some(*current),
                maximum: *maximum,
            }),
            Event::ResourceRestored {
                actor_id,
                resource,
                before,
                after,
                maximum,
                ..
            } if actor_id == observer_actor_id => Some(ObserverFeedbackCueV1::Resource {
                actor: controlled_actor(actor_id)?,
                resource: *resource,
                reason: ObserverResourceReasonV1::Restored,
                amount: after.saturating_sub(*before),
                current: Some(*after),
                maximum: *maximum,
            }),
            Event::BalmHealed {
                actor_id,
                amount,
                hp,
                ..
            } if actor_id == observer_actor_id => Some(ObserverFeedbackCueV1::Resource {
                actor: controlled_actor(actor_id)?,
                resource: crate::model::ResourceKind::Hp,
                reason: ObserverResourceReasonV1::Balm,
                amount: *amount,
                current: Some(*hp),
                maximum: self
                    .world
                    .actor(actor_id)
                    .ok_or_else(|| StepError::new("resource actor disappeared before projection"))?
                    .max_hp(),
            }),
            Event::TransactionCommitted {
                actor_id,
                source,
                costs,
                rewards,
                ..
            } if actor_id == observer_actor_id => {
                if costs.len() > MAX_FEEDBACK_TRANSACTION_COSTS
                    || rewards.len() > MAX_FEEDBACK_TRANSACTION_REWARDS
                {
                    return Err(StepError::new(
                        "committed transaction exceeds observer feedback receipt bound",
                    ));
                }
                Some(ObserverFeedbackCueV1::Transaction {
                    actor: controlled_actor(actor_id)?,
                    source: observer_transaction_source(source),
                    costs: costs.iter().map(observer_transaction_cost).collect(),
                    rewards: rewards.iter().map(observer_transaction_reward).collect(),
                })
            }
            Event::QuestStateChanged {
                character_id,
                quest_id,
                before_stage_id,
                after_stage_id,
            } if character_id == observer_character_id => {
                let quest = self
                    .definition
                    .catalog
                    .quests
                    .iter()
                    .find(|(id, _)| id.as_str() == quest_id)
                    .map(|(_, quest)| quest)
                    .ok_or_else(|| StepError::new("quest feedback definition is missing"))?;
                let stage = quest
                    .stages
                    .iter()
                    .find(|(id, _)| id.as_str() == after_stage_id)
                    .map(|(_, stage)| stage)
                    .ok_or_else(|| StepError::new("quest feedback stage is missing"))?;
                Some(ObserverFeedbackCueV1::Quest {
                    quest_id: quest_id.clone(),
                    quest_title: quest.title.clone(),
                    before_stage_id: before_stage_id.clone(),
                    after_stage_id: after_stage_id.clone(),
                    after_stage_label: stage.label.clone(),
                    terminal: stage.terminal,
                })
            }
            Event::NpcSpoke {
                npc_actor_id,
                recipient_character_id,
                interaction_id,
                response,
                ..
            } if recipient_character_id == observer_character_id => {
                let scalar_count = response.chars().count();
                if response.is_empty()
                    || scalar_count > MAX_FEEDBACK_TEXT_SCALARS
                    || response.len() > MAX_FEEDBACK_TEXT_BYTES
                    || response.chars().any(char::is_control)
                {
                    return Err(StepError::new(
                        "NPC feedback response violates the bounded text contract",
                    ));
                }
                let npc = self.observer_feedback_actor(npc_actor_id).ok_or_else(|| {
                    StepError::new("NPC feedback actor disappeared before projection")
                })?;
                Some(ObserverFeedbackCueV1::NpcMessage {
                    npc_actor_id: npc.actor_id,
                    npc_name: npc.name,
                    interaction_id: interaction_id.clone(),
                    response: response.clone(),
                })
            }
            _ => None,
        };
        Ok(cue)
    }

    fn observer_death_feedback(
        &self,
        event: &Event,
        observer_actor_id: &ActorId,
        visible: &BTreeSet<WorldPosition>,
    ) -> Result<Option<ObserverFeedbackCueV1>, StepError> {
        let actor_visible = |actor_id: &ActorId| {
            self.observer_feedback_actor_is_visible(actor_id, observer_actor_id, visible)
        };
        let actor = |actor_id: &ActorId| {
            self.observer_feedback_actor(actor_id)
                .ok_or_else(|| StepError::new("death feedback actor disappeared before projection"))
        };
        let cue = match event {
            Event::ActorDefeated {
                actor_id,
                location,
                cause,
                credited_actor_id,
                ..
            } if actor_id == observer_actor_id || visible.contains(location) => {
                Some(ObserverFeedbackCueV1::Defeat {
                    actor: actor(actor_id)?,
                    location: location.clone(),
                    cause: *cause,
                    credited_source: credited_actor_id.as_ref().and_then(|source_actor_id| {
                        self.observer_feedback_source(source_actor_id, observer_actor_id, visible)
                    }),
                })
            }
            Event::CorpseCreated {
                corpse_id,
                origin_actor_id,
                origin_kind,
                origin_name,
                location,
                ..
            } if visible.contains(location) => Some(ObserverFeedbackCueV1::Corpse {
                corpse_id: corpse_id.clone(),
                origin: Some(ObserverFeedbackActorV1 {
                    actor_id: origin_actor_id.clone(),
                    name: origin_name.clone(),
                    kind: *origin_kind,
                }),
                location: location.clone(),
                change: ObserverCorpseChangeV1::Created,
            }),
            Event::CorpseRemoved {
                corpse_id,
                origin_actor_id,
                location,
                method,
            } if visible.contains(location) => Some(ObserverFeedbackCueV1::Corpse {
                corpse_id: corpse_id.clone(),
                origin: self.observer_feedback_source(origin_actor_id, observer_actor_id, visible),
                location: location.clone(),
                change: ObserverCorpseChangeV1::Removed { method: *method },
            }),
            Event::ActorLifeStateChanged {
                actor_id, from, to, ..
            } if actor_visible(actor_id) => Some(ObserverFeedbackCueV1::LifeState {
                actor: actor(actor_id)?,
                from: ObserverLifeStateV1::from(from),
                to: ObserverLifeStateV1::from(to),
            }),
            Event::ActorResurrected {
                actor_id,
                corpse_id,
                method,
                destination,
                current_hp,
                current_stamina,
                ..
            } if actor_id == observer_actor_id || visible.contains(destination) => {
                Some(ObserverFeedbackCueV1::Resurrection {
                    actor: actor(actor_id)?,
                    corpse_id: corpse_id.clone(),
                    method: *method,
                    destination: destination.clone(),
                    current_hp: *current_hp,
                    current_stamina: *current_stamina,
                })
            }
            _ => None,
        };
        Ok(cue)
    }

    pub(super) fn observer_feedback_cues(
        &self,
        event: &Event,
        observer_actor_id: &ActorId,
        observer_character_id: &crate::model::CharacterId,
        visible: &BTreeSet<WorldPosition>,
    ) -> Result<Vec<ObserverFeedbackCueV1>, StepError> {
        let mut cues = Vec::new();
        if let Some(cue) = self.observer_combat_feedback(event, observer_actor_id, visible)? {
            cues.push(cue);
        }
        if let Some(cue) = self.observer_spell_feedback(event, observer_actor_id, visible)? {
            cues.push(cue);
        }
        cues.extend(self.observer_spell_resource_feedback(event, observer_actor_id)?);
        if let Some(cue) = self.observer_effect_feedback(event, observer_actor_id, visible)? {
            cues.push(cue);
        }
        if let Some(cue) =
            self.observer_private_feedback(event, observer_actor_id, observer_character_id)?
        {
            cues.push(cue);
        }
        if let Some(cue) = self.observer_death_feedback(event, observer_actor_id, visible)? {
            cues.push(cue);
        }
        Ok(cues)
    }
}
