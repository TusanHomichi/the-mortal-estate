//! Effect families that move, bind or unmake an actor.

use crate::content::SpellEffectDef;
use crate::events::{
    AutomaticWaitReasonV1, BanishResultReasonV1, Event, RaiseDeadResultReasonV1,
    TransitionConcealmentRemovalReasonV1,
};
use crate::model::{
    ActorKind, ConcealedTransitionState, CreatureTrait, DeathCause, NavigationKind,
    ResurrectionMethod, ResurrectionRequest, SpellTarget, WorldPosition,
};

use super::super::{
    HostileSpellContactPlan, HostileSpellOutcomeReceipt, SpellCommandPlan, SpellEffectOutcome,
};
use crate::engine::death::DefeatContext;
use crate::engine::{Engine, StepError};

impl Engine {
    pub(super) fn apply_banish_spell(
        &mut self,
        caster_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(definition) = effect.banish.as_ref() else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        let Some(SpellTarget::Actor { actor_id }) = plan.target.as_ref() else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        let (caster_id, caster_name) = {
            let caster = &self.world.actors[caster_index];
            (caster.id.clone(), caster.name.clone())
        };
        let Some(target_index) = self
            .world
            .actors
            .iter()
            .position(|actor| actor.is_alive() && actor.id == *actor_id)
        else {
            events.push(Event::BanishEvaluated {
                caster_id,
                caster: caster_name,
                spell_id: plan.spell_id.clone(),
                spell_name: plan.spell_name.clone(),
                target_id: actor_id.clone(),
                target: actor_id.to_string(),
                eligible_trait: None,
                owned_by_caster: false,
                success: false,
                reason: BanishResultReasonV1::InvalidTarget,
            });
            return Ok(SpellEffectOutcome::Failed);
        };
        let target = &self.world.actors[target_index];
        let eligible_trait = definition
            .eligible_traits
            .iter()
            .copied()
            .find(|candidate| target.creature_traits.contains(candidate));
        let owned_by_caster = target
            .summoned
            .as_ref()
            .is_some_and(|summoned| summoned.owner_id == caster_id);
        let target_name = target.name.clone();
        let success = eligible_trait == Some(CreatureTrait::Demon) && owned_by_caster;
        let reason = if success {
            BanishResultReasonV1::Banished
        } else if eligible_trait.is_none() {
            BanishResultReasonV1::IneligibleTrait
        } else {
            BanishResultReasonV1::WillpowerFormulaOpen
        };
        events.push(Event::BanishEvaluated {
            caster_id: caster_id.clone(),
            caster: caster_name.clone(),
            spell_id: plan.spell_id.clone(),
            spell_name: plan.spell_name.clone(),
            target_id: actor_id.clone(),
            target: target_name.clone(),
            eligible_trait,
            owned_by_caster,
            success,
            reason,
        });
        if !success {
            return Ok(SpellEffectOutcome::Failed);
        }
        let removed = self.remove_summoned_actor_at(target_index)?;
        let summoned = removed
            .summoned
            .expect("successful Banish target must be summoned");
        events.push(Event::ActorBanished {
            caster_id,
            caster: caster_name,
            spell_id: plan.spell_id.clone(),
            spell_name: plan.spell_name.clone(),
            actor_id: removed.id,
            actor: removed.name,
            instance_id: summoned.instance_id,
            owner_id: summoned.owner_id,
            template_id: summoned.template_id,
            location: removed.location,
        });
        Ok(SpellEffectOutcome::Applied)
    }

    pub(super) fn apply_concealment_spell(
        &mut self,
        caster_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        match plan.target.as_ref() {
            Some(SpellTarget::SelfTarget) => {
                let Some((_, hide_config)) = self.hide_action_config_for_actor(caster_index) else {
                    return Ok(SpellEffectOutcome::Failed);
                };
                if !self.actor_has_concealment_cover_or_darkness(caster_index)
                    || !self.hide_equipment_allowed(caster_index, hide_config)
                {
                    return Ok(SpellEffectOutcome::Failed);
                }
                let outcome = self.apply_active_effect_spell(caster_index, plan, effect, events)?;
                if outcome != SpellEffectOutcome::Applied {
                    return Ok(outcome);
                }
                let applied = self.world.actors[caster_index]
                    .active_effects
                    .iter()
                    .rev()
                    .find(|active| {
                        active.source.kind == "spell"
                            && active.source.id == plan.spell_id
                            && active.tags.iter().any(|tag| tag == "hidden")
                    })
                    .expect("validated concealment must apply one hidden effect");
                let actor = &self.world.actors[caster_index];
                events.push(Event::ActorHidden {
                    actor_id: actor.id.clone(),
                    actor: actor.name.clone(),
                    location: actor.location.clone(),
                    instance_id: applied.instance_id.clone(),
                    effect_id: applied.effect_id.clone(),
                    remaining_rounds: applied.remaining_rounds,
                });
                Ok(SpellEffectOutcome::Applied)
            }
            Some(SpellTarget::Door { direction }) => {
                let caster = &self.world.actors[caster_index];
                let location = WorldPosition::new(
                    &caster.location.realm,
                    &caster.location.level,
                    caster.location.position.step(*direction),
                );
                let Some(transition) = self.effective_transition_at(&location) else {
                    return Ok(SpellEffectOutcome::Failed);
                };
                if transition.kind != NavigationKind::Door
                    || self.effective_door_state_at(&location) != Some(false)
                {
                    return Ok(SpellEffectOutcome::Failed);
                }
                let Some(remaining_rounds) =
                    self.spell_effect_remaining_rounds(effect.duration.as_ref())
                else {
                    return Ok(SpellEffectOutcome::Stubbed);
                };
                let (actor_id, actor) = (caster.id.clone(), caster.name.clone());
                self.remove_navigation_concealment_at(
                    &location,
                    TransitionConcealmentRemovalReasonV1::Replaced,
                    events,
                );
                let instance_id = format!(
                    "transition:{}:{}:{}:{}:{}",
                    plan.spell_id,
                    self.current_time(),
                    location.level,
                    location.position.x,
                    location.position.y
                );
                self.world
                    .concealed_transitions
                    .push(ConcealedTransitionState {
                        instance_id: instance_id.clone(),
                        source_spell_id: plan.spell_id.clone(),
                        source_actor_id: actor_id.clone(),
                        location: location.clone(),
                        remaining_rounds,
                        last_ticked_at: self.current_time(),
                    });
                events.push(Event::TransitionConcealed {
                    actor_id,
                    actor,
                    spell_id: plan.spell_id.clone(),
                    spell_name: plan.spell_name.clone(),
                    instance_id,
                    location,
                    remaining_rounds,
                });
                Ok(SpellEffectOutcome::Applied)
            }
            _ => Ok(SpellEffectOutcome::Stubbed),
        }
    }

    pub(super) fn apply_instant_death_spell(
        &mut self,
        caster_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(definition) = effect.instant_death.as_ref() else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        let Some(target_index) = self.resolve_spell_effect_target_index(caster_index, plan) else {
            return Ok(SpellEffectOutcome::Failed);
        };
        let magic_level = i32::from(self.skill_level_for_actor(caster_index, &plan.lane));
        let requested_damage = magic_level
            .checked_mul(definition.damage_per_magic_level)
            .ok_or_else(|| StepError::new("instant death damage overflow"))?;
        let damage = self
            .resolve_spell_resistance(
                target_index,
                &plan.spell_id,
                effect,
                Some(requested_damage),
                events,
            )
            .and_then(|resolution| resolution.resolved_damage)
            .unwrap_or(requested_damage);
        if damage == 0 {
            return Ok(SpellEffectOutcome::Applied);
        }
        let (caster_id, caster_name) = {
            let caster = &self.world.actors[caster_index];
            (caster.id.clone(), caster.name.clone())
        };
        let (target_id, target_name, target_location) = {
            let target = &self.world.actors[target_index];
            (
                target.id.clone(),
                target.name.clone(),
                target.location.clone(),
            )
        };
        let spell_id = plan.spell_id.clone();
        let spell_name = plan.spell_name.clone();
        let spell_damage_credit = plan.damage_credit(&caster_id);
        self.apply_damage_and_resolve_defeat(
            target_index,
            damage,
            DefeatContext {
                cause: DeathCause::OtherMagic,
                credited_actor_id: Some(caster_id.clone()),
                direct_social_actor_id: Some(caster_id.clone()),
                spell_damage_credit,
                hostile_authority: None,
            },
            events,
            move |outcome| Event::SpellDamaged {
                caster_id,
                caster: caster_name,
                spell_id,
                spell_name,
                target_id,
                target: target_name,
                location: target_location,
                damage_kind: Some("death".to_string()),
                damage: outcome.applied,
                hp: outcome.hp_after,
            },
            |_| {},
        )?;
        Ok(SpellEffectOutcome::Applied)
    }

    pub(super) fn apply_raise_dead_spell(
        &mut self,
        caster_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(definition) = effect.raise_dead.as_ref() else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        debug_assert_eq!(definition.method, ResurrectionMethod::Thaumaturge);
        let (caster_id, caster_name, caster_location) = {
            let caster = &self.world.actors[caster_index];
            (
                caster.id.clone(),
                caster.name.clone(),
                caster.location.clone(),
            )
        };
        let corpse = self
            .world
            .corpses
            .values()
            .filter(|corpse| corpse.location == caster_location)
            .max_by_key(|corpse| corpse.sequence)
            .cloned();
        let magic_level = self.skill_level_for_actor(caster_index, &plan.lane);
        let rules = &self
            .definition
            .catalog
            .rules
            .magic
            .effect_families
            .raise_dead;
        let threshold = u32::from(magic_level)
            .checked_mul(rules.success_threshold_per_magic_level)
            .ok_or_else(|| StepError::new("raise dead threshold overflow"))?
            .clamp(rules.minimum_success_threshold, rules.roll_denominator);
        let Some(corpse) = corpse else {
            events.push(Event::RaiseDeadEvaluated {
                caster_id,
                caster: caster_name,
                spell_id: plan.spell_id.clone(),
                spell_name: plan.spell_name.clone(),
                corpse_id: None,
                target_actor_id: None,
                magic_level,
                roll_denominator: rules.roll_denominator,
                success_threshold: threshold,
                roll: None,
                success: false,
                reason: RaiseDeadResultReasonV1::NoCorpse,
            });
            return Ok(SpellEffectOutcome::Failed);
        };
        if corpse.origin_kind != ActorKind::Player {
            events.push(Event::RaiseDeadEvaluated {
                caster_id,
                caster: caster_name,
                spell_id: plan.spell_id.clone(),
                spell_name: plan.spell_name.clone(),
                corpse_id: Some(corpse.id),
                target_actor_id: Some(corpse.origin_actor_id),
                magic_level,
                roll_denominator: rules.roll_denominator,
                success_threshold: threshold,
                roll: None,
                success: false,
                reason: RaiseDeadResultReasonV1::NonPlayerCorpse,
            });
            return Ok(SpellEffectOutcome::Failed);
        }
        let roll = self
            .rng
            .roll_bounded(rules.roll_denominator)
            .map_err(StepError::new)?;
        let success = roll <= threshold;
        events.push(Event::RaiseDeadEvaluated {
            caster_id: caster_id.clone(),
            caster: caster_name,
            spell_id: plan.spell_id.clone(),
            spell_name: plan.spell_name.clone(),
            corpse_id: Some(corpse.id.clone()),
            target_actor_id: Some(corpse.origin_actor_id.clone()),
            magic_level,
            roll_denominator: rules.roll_denominator,
            success_threshold: threshold,
            roll: Some(roll),
            success,
            reason: if success {
                RaiseDeadResultReasonV1::Resurrected
            } else {
                RaiseDeadResultReasonV1::RollFailed
            },
        });
        if !success {
            return Ok(SpellEffectOutcome::Failed);
        }
        let target_index = self
            .world
            .actors
            .iter()
            .position(|actor| actor.id == corpse.origin_actor_id)
            .ok_or_else(|| StepError::new("raise dead corpse actor is missing"))?;
        let max_hp = self.world.actors[target_index].max_hp();
        let current_hp = max_hp / 2 + max_hp % 2;
        let resurrected_actor_id = corpse.origin_actor_id;
        let resurrection_events = self.apply_resurrection_request(ResurrectionRequest {
            actor_id: resurrected_actor_id.clone(),
            corpse_id: Some(corpse.id),
            method: ResurrectionMethod::Thaumaturge,
            destination: caster_location,
            current_hp,
            current_stamina: 0,
        })?;
        events.extend(resurrection_events);
        self.schedule_resurrected_actor(&resurrected_actor_id, events)?;
        Ok(SpellEffectOutcome::Applied)
    }

    pub(super) fn apply_turn_undead_spell(
        &mut self,
        caster_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        contact_plans: &[HostileSpellContactPlan],
        events: &mut Vec<Event>,
    ) -> Result<(SpellEffectOutcome, Vec<HostileSpellOutcomeReceipt>), StepError> {
        let Some(definition) = effect.turn_undead.as_ref() else {
            return Ok((SpellEffectOutcome::Stubbed, Vec::new()));
        };
        let (caster_id, caster_name) = {
            let caster = &self.world.actors[caster_index];
            (caster.id.clone(), caster.name.clone())
        };
        let mut considered_actor_ids = self
            .world
            .actors
            .iter()
            .enumerate()
            .filter(|(index, actor)| {
                *index != caster_index
                    && actor.is_alive()
                    && actor.creature_traits.contains(&definition.eligible_trait)
                    && self.actor_can_see(caster_index, &actor.location.clone())
            })
            .map(|(_, actor)| actor.id.clone())
            .collect::<Vec<_>>();
        considered_actor_ids.sort();
        if contact_plans.len() != considered_actor_ids.len() {
            return Err(StepError::new(
                "turn-undead contact targets changed before effect",
            ));
        }
        let mut moved_actor_ids = Vec::new();
        let mut blocked_actor_ids = Vec::new();
        let mut hostile_spell_outcomes = Vec::new();
        for (position, actor_id) in considered_actor_ids.iter().enumerate() {
            let contact = contact_plans.get(position);
            if contact.is_some_and(|contact| contact.target_actor_id != *actor_id) {
                return Err(StepError::new(
                    "turn-undead contact target order changed before effect",
                ));
            }
            if let Some(contact) = contact {
                self.commit_hostile_spell_contact(contact, events)?;
            }
            let first_outcome_event_index = events.len();
            let Some(actor_index) = self
                .world
                .actors
                .iter()
                .position(|actor| actor.id == *actor_id)
            else {
                continue;
            };
            if let Some(direction) = self.flee_direction_from(actor_index, caster_index) {
                self.commit_automatic_move(
                    actor_index,
                    direction,
                    crate::events::AutomaticMovementPurposeV1::Turned,
                    events,
                )?;
                moved_actor_ids.push(actor_id.clone());
            } else {
                blocked_actor_ids.push(actor_id.clone());
                self.commit_automatic_wait(actor_index, AutomaticWaitReasonV1::Blocked, events);
            }
            let one_past_last_outcome_event_index = events.len();
            if let Some(contact) = contact {
                hostile_spell_outcomes.push(HostileSpellOutcomeReceipt {
                    spell_id: contact.spell_id.clone(),
                    source_actor_id: contact.source_actor_id.clone(),
                    target_actor_id: contact.target_actor_id.clone(),
                    credited_source_actor_id: contact.credited_source_actor_id.clone(),
                    reach: contact.reach,
                    outcome: SpellEffectOutcome::Applied,
                    first_outcome_event_index,
                    one_past_last_outcome_event_index,
                });
            }
        }
        events.push(Event::TurnUndeadResolved {
            caster_id,
            caster: caster_name,
            spell_id: plan.spell_id.clone(),
            spell_name: plan.spell_name.clone(),
            considered_actor_ids,
            moved_actor_ids,
            blocked_actor_ids,
        });
        Ok((SpellEffectOutcome::Applied, hostile_spell_outcomes))
    }

    pub(super) fn apply_locate_spell(
        &mut self,
        player_index: usize,
        _plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(locate) = effect.locate.as_ref() else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        let (actor_id, actor) = {
            let player = &self.world.actors[player_index];
            (player.id.clone(), player.name.clone())
        };
        let (site, location, hint) = self.resolve_locate_hint(player_index, locate);
        events.push(Event::Located {
            actor_id,
            actor,
            subject: locate.subject.clone(),
            id: locate.id.clone(),
            site,
            location,
            hint,
        });
        Ok(SpellEffectOutcome::Applied)
    }

    pub(super) fn apply_scry_spell(
        &mut self,
        player_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(scry) = effect.scry.as_ref() else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        let (actor_id, actor) = {
            let player = &self.world.actors[player_index];
            (player.id.clone(), player.name.clone())
        };
        let (site, location, hint) = self.resolve_scry_hint(player_index, &plan.spell_id, scry);
        events.push(Event::Located {
            actor_id,
            actor,
            subject: "scry".to_string(),
            id: plan.spell_id.clone(),
            site,
            location,
            hint,
        });
        Ok(SpellEffectOutcome::Applied)
    }
}
