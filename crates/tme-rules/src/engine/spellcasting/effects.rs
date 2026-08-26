use crate::content::{SpellDurationDef, SpellEffectDef, SpellLocateDef, SpellScryDef};
use crate::events::{
    AutomaticWaitReasonV1, BanishResultReasonV1, Event, RaiseDeadResultReasonV1,
    TransitionConcealmentRemovalReasonV1,
};
use crate::model::{
    ActiveEffectSource, ActiveEffectStackingPolicy, ActiveEffectState, ActorKind,
    ConcealedTransitionState, CreatureTrait, DeathCause, HostileEffectAuthority, NavigationKind,
    PortalTransitionState, ResurrectionMethod, ResurrectionRequest, SocialContactKind,
    SpellDurationPolicy, SpellEffectFamily, SpellTarget, TileEffectState, WorldPosition,
};

use super::targeting::transition_kind_label;
use super::{
    DoorSecretAction, HostileSpellContactPlan, HostileSpellOutcomeReceipt, HostileSpellReach,
    SpellCommandPlan, SpellEffectExecution, SpellEffectOutcome,
};
use crate::engine::death::DefeatContext;
use crate::engine::{Engine, StepError};

impl Engine {
    pub(in crate::engine) fn spell_specific_hostile_target_allowed(
        &self,
        target_index: usize,
        effect: &SpellEffectDef,
    ) -> bool {
        let Some(target) = self.world.actors.get(target_index) else {
            return false;
        };
        effect.family == SpellEffectFamily::Banish
            && target.kind != ActorKind::Player
            && effect.banish.as_ref().is_some_and(|banish| {
                banish
                    .eligible_traits
                    .iter()
                    .any(|candidate| target.creature_traits.contains(candidate))
            })
    }

    fn hostile_spell_contact_target_indices(
        &self,
        caster_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
    ) -> Vec<(usize, HostileSpellReach)> {
        if effect.family == SpellEffectFamily::TurnUndead {
            let Some(definition) = effect.turn_undead.as_ref() else {
                return Vec::new();
            };
            let mut targets = self
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
                .map(|(index, _)| (index, HostileSpellReach::TurnUndeadVisibility))
                .collect::<Vec<_>>();
            targets.sort_by(|(left, _), (right, _)| {
                self.world.actors[*left]
                    .id
                    .cmp(&self.world.actors[*right].id)
            });
            return targets;
        }

        let mut targets = match plan.target.as_ref() {
            Some(SpellTarget::Actor { actor_id }) => self
                .world
                .actors
                .iter()
                .position(|actor| actor.is_alive() && actor.id == *actor_id)
                .map(|index| vec![(index, HostileSpellReach::DirectedActor)])
                .unwrap_or_default(),
            Some(SpellTarget::Path { .. }) => plan
                .path_plan
                .as_ref()
                .and_then(|path| path.final_position.as_ref())
                .map(|position| {
                    self.world
                        .actors
                        .iter()
                        .enumerate()
                        .filter(|(_, actor)| {
                            actor.is_alive()
                                && actor.id != self.world.actors[caster_index].id
                                && actor.location.level == position.level
                                && actor.location.position == position.position
                        })
                        .map(|(index, _)| (index, HostileSpellReach::PathEndpoint))
                        .collect()
                })
                .unwrap_or_default(),
            Some(SpellTarget::Area { center })
            | Some(SpellTarget::Coordinate { position: center }) => self
                .world
                .actors
                .iter()
                .enumerate()
                .filter(|(_, actor)| {
                    actor.is_alive()
                        && actor.id != self.world.actors[caster_index].id
                        && actor.location.level == center.level
                        && actor.location.position == center.position
                })
                .map(|(index, _)| (index, HostileSpellReach::AreaCenter))
                .collect(),
            _ => Vec::new(),
        };
        targets.sort_by(|(left, _), (right, _)| {
            self.world.actors[*left]
                .id
                .cmp(&self.world.actors[*right].id)
        });
        targets
    }

    fn hostile_spell_contact_plans(
        &self,
        caster_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
    ) -> Result<Vec<HostileSpellContactPlan>, StepError> {
        let source = &self.world.actors[caster_index];
        let mut contacts = Vec::new();
        for (target_index, reach) in
            self.hostile_spell_contact_target_indices(caster_index, plan, effect)
        {
            if let Some(authorization) = plan.hostility_authorization
                && !self.spell_specific_hostile_target_allowed(target_index, effect)
            {
                let assessment = self.attack_safety_assessment(caster_index, target_index)?;
                if !assessment.safety.permits(authorization) {
                    continue;
                }
            }
            let target = &self.world.actors[target_index];
            contacts.push(HostileSpellContactPlan {
                spell_id: plan.spell_id.clone(),
                source_actor_id: source.id.clone(),
                source_character_id: source.character_id.clone(),
                target_actor_id: target.id.clone(),
                target_character_id: target.character_id.clone(),
                credited_source_actor_id: source.id.clone(),
                spell_damage_credit: plan.damage_credit(&source.id),
                authorization: plan.hostility_authorization,
                reach,
                relations: self.plan_attack_relations(
                    caster_index,
                    target_index,
                    SocialContactKind::HostileSpellContact,
                )?,
            });
        }
        Ok(contacts)
    }

    fn commit_hostile_spell_contact(
        &mut self,
        plan: &HostileSpellContactPlan,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let source_index = self
            .world
            .actors
            .iter()
            .position(|actor| actor.id == plan.source_actor_id)
            .ok_or_else(|| StepError::new("hostile spell source changed before contact"))?;
        let target_index = self
            .world
            .actors
            .iter()
            .position(|actor| actor.id == plan.target_actor_id)
            .ok_or_else(|| StepError::new("hostile spell target changed before contact"))?;
        let source = &self.world.actors[source_index];
        let target = &self.world.actors[target_index];
        if source.character_id != plan.source_character_id
            || target.character_id != plan.target_character_id
            || plan.credited_source_actor_id != source.id
            || plan.spell_damage_credit.as_ref().is_some_and(|credit| {
                credit.caster_actor_id != source.id || credit.spell_id != plan.spell_id
            })
        {
            return Err(StepError::new(
                "hostile spell identities changed before contact",
            ));
        }
        if !self
            .definition
            .catalog
            .spells
            .get(&plan.spell_id)
            .is_some_and(|spell| spell.social.hostile_act)
        {
            return Err(StepError::new(
                "hostile spell classification changed before contact",
            ));
        }
        if let Some(authorization) = plan.authorization {
            let spell_specific_target = self
                .definition
                .catalog
                .spells
                .get(&plan.spell_id)
                .and_then(|spell| spell.effect.as_ref())
                .is_some_and(|effect| {
                    self.spell_specific_hostile_target_allowed(target_index, effect)
                });
            if !spell_specific_target
                && !self
                    .attack_safety_assessment(source_index, target_index)?
                    .safety
                    .permits(authorization)
            {
                return Err(StepError::new(
                    "hostile spell authorization changed before contact",
                ));
            }
        }
        self.commit_attack_relations(&plan.relations, events)
    }

    pub(in crate::engine) fn execute_actor_spell_effect(
        &mut self,
        caster_index: usize,
        plan: &SpellCommandPlan,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectExecution, StepError> {
        let Some(effect) = self
            .definition
            .catalog
            .spells
            .get(&plan.spell_id)
            .and_then(|spell| spell.effect.clone())
        else {
            return Ok(SpellEffectExecution {
                outcome: SpellEffectOutcome::Stubbed,
                hostile_spell_outcomes: Vec::new(),
            });
        };
        let hostile_act = self.definition.catalog.spells[&plan.spell_id]
            .social
            .hostile_act;
        let contact_plans = if hostile_act {
            self.hostile_spell_contact_plans(caster_index, plan, &effect)?
        } else {
            Vec::new()
        };

        if effect.family == SpellEffectFamily::TurnUndead {
            let (outcome, hostile_spell_outcomes) =
                self.apply_turn_undead_spell(caster_index, plan, &effect, &contact_plans, events)?;
            return Ok(SpellEffectExecution {
                outcome,
                hostile_spell_outcomes,
            });
        }

        let actor_contact_effect = matches!(
            effect.family,
            SpellEffectFamily::DirectDamage
                | SpellEffectFamily::AttributeBuff
                | SpellEffectFamily::Curse
                | SpellEffectFamily::ControlStatus
                | SpellEffectFamily::FallProtection
                | SpellEffectFamily::Protection
                | SpellEffectFamily::Resistance
                | SpellEffectFamily::Poison
                | SpellEffectFamily::Speed
                | SpellEffectFamily::Vision
                | SpellEffectFamily::WaterBreathing
                | SpellEffectFamily::Banish
                | SpellEffectFamily::InstantDeath
        );
        if hostile_act
            && actor_contact_effect
            && matches!(plan.target.as_ref(), Some(SpellTarget::SelfTarget))
        {
            let outcome = self.execute_single_spell_family(caster_index, plan, &effect, events)?;
            return Ok(SpellEffectExecution {
                outcome,
                hostile_spell_outcomes: Vec::new(),
            });
        }
        if hostile_act && actor_contact_effect {
            let mut hostile_spell_outcomes = Vec::new();
            let mut any_applied = false;
            let mut any_stubbed = false;
            for contact in contact_plans {
                self.commit_hostile_spell_contact(&contact, events)?;
                let first_outcome_event_index = events.len();
                let mut target_plan = plan.clone();
                target_plan.target = Some(SpellTarget::Actor {
                    actor_id: contact.target_actor_id.clone(),
                });
                target_plan.damage_credit_override = contact.spell_damage_credit.clone();
                let outcome =
                    self.execute_single_spell_family(caster_index, &target_plan, &effect, events)?;
                any_applied |= outcome == SpellEffectOutcome::Applied;
                any_stubbed |= outcome == SpellEffectOutcome::Stubbed;
                hostile_spell_outcomes.push(HostileSpellOutcomeReceipt {
                    spell_id: contact.spell_id,
                    source_actor_id: contact.source_actor_id,
                    target_actor_id: contact.target_actor_id,
                    credited_source_actor_id: contact.credited_source_actor_id,
                    reach: contact.reach,
                    outcome,
                    first_outcome_event_index,
                    one_past_last_outcome_event_index: events.len(),
                });
            }
            let outcome = if any_applied || hostile_spell_outcomes.is_empty() {
                SpellEffectOutcome::Applied
            } else if any_stubbed {
                SpellEffectOutcome::Stubbed
            } else {
                SpellEffectOutcome::Failed
            };
            return Ok(SpellEffectExecution {
                outcome,
                hostile_spell_outcomes,
            });
        }

        for contact in &contact_plans {
            self.commit_hostile_spell_contact(contact, events)?;
        }
        let first_outcome_event_index = events.len();
        let outcome = self.execute_single_spell_family(caster_index, plan, &effect, events)?;

        let one_past_last_outcome_event_index = events.len();
        Ok(SpellEffectExecution {
            outcome,
            hostile_spell_outcomes: contact_plans
                .into_iter()
                .map(|contact| HostileSpellOutcomeReceipt {
                    spell_id: contact.spell_id,
                    source_actor_id: contact.source_actor_id,
                    target_actor_id: contact.target_actor_id,
                    credited_source_actor_id: contact.credited_source_actor_id,
                    reach: contact.reach,
                    outcome,
                    first_outcome_event_index,
                    one_past_last_outcome_event_index,
                })
                .collect(),
        })
    }

    fn execute_single_spell_family(
        &mut self,
        caster_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        match effect.family {
            SpellEffectFamily::DirectDamage => {
                self.apply_direct_damage_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::Healing => {
                self.apply_healing_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::AttributeBuff
            | SpellEffectFamily::Curse
            | SpellEffectFamily::ControlStatus
            | SpellEffectFamily::FallProtection
            | SpellEffectFamily::Protection
            | SpellEffectFamily::Resistance
            | SpellEffectFamily::Poison
            | SpellEffectFamily::Speed
            | SpellEffectFamily::Vision
            | SpellEffectFamily::WaterBreathing => {
                self.apply_active_effect_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::PoisonCure => {
                self.apply_poison_cure_spell(caster_index, plan, events)
            }
            SpellEffectFamily::TerrainOverlay
            | SpellEffectFamily::Light
            | SpellEffectFamily::Darkness => {
                self.apply_tile_overlay_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::Summon => {
                self.apply_summon_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::DoorControl | SpellEffectFamily::SecretDetection => {
                self.apply_door_secret_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::ItemIdentify
            | SpellEffectFamily::ItemEnchant
            | SpellEffectFamily::WeaponEnchant => {
                self.apply_item_utility_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::Locate => {
                self.apply_locate_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::Scry => self.apply_scry_spell(caster_index, plan, effect, events),
            SpellEffectFamily::Portal => {
                self.apply_portal_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::Banish => {
                self.apply_banish_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::InstantDeath => {
                self.apply_instant_death_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::RaiseDead => {
                self.apply_raise_dead_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::TurnUndead => unreachable!("handled above"),
            SpellEffectFamily::Concealment => {
                self.apply_concealment_spell(caster_index, plan, effect, events)
            }
        }
    }

    fn apply_banish_spell(
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

    fn apply_concealment_spell(
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

    fn apply_instant_death_spell(
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

    fn apply_raise_dead_spell(
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

    fn apply_turn_undead_spell(
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

    fn apply_locate_spell(
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

    fn apply_scry_spell(
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

    fn apply_portal_spell(
        &mut self,
        player_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(portal) = effect.portal.as_ref() else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        let Some(SpellTarget::Coordinate { position }) = plan.target.as_ref() else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        self.validate_portal_anchor(player_index, position)
            .map_err(Self::spell_command_error)?;
        if !self.portal_target_is_authored_and_passable(&portal.target) {
            return Err(StepError::new("invalid_target"));
        }
        let target = self
            .resolve_topology_target(&portal.target)
            .ok_or_else(|| StepError::new("invalid_target"))?;
        let (actor_id, actor) = {
            let player = &self.world.actors[player_index];
            (player.id.clone(), player.name.clone())
        };
        let instance_id = format!(
            "portal:{}:{}:{}:{}:{}",
            plan.spell_id,
            self.current_time(),
            position.level,
            position.position.x,
            position.position.y
        );
        let remaining_rounds = self.spell_effect_remaining_rounds(effect.duration.as_ref());
        self.world
            .portal_transitions
            .retain(|existing| existing.location != *position);
        self.world.portal_transitions.push(PortalTransitionState {
            instance_id: instance_id.clone(),
            source_spell_id: plan.spell_id.clone(),
            source_actor_id: actor_id.clone(),
            location: position.clone(),
            target: target.clone(),
            two_way: portal.two_way,
            remaining_rounds,
            last_ticked_at: self.current_time(),
        });
        events.push(Event::PortalCreated {
            actor_id,
            actor,
            instance_id,
            location: position.clone(),
            target,
            remaining_rounds,
            two_way: portal.two_way,
        });
        Ok(SpellEffectOutcome::Applied)
    }

    fn apply_item_utility_spell(
        &mut self,
        player_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(item_utility) = effect.item_utility.as_ref() else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        let Some(SpellTarget::Item {
            item_instance_id,
            location,
        }) = plan.target.as_ref()
        else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        let resolved = self
            .resolve_spell_item(player_index, item_instance_id, *location)
            .ok_or_else(|| StepError::new("invalid_target"))?;
        let actor_id = self.world.actors[player_index].id.clone();
        let actor = self.world.actors[player_index].name.clone();
        let source = crate::model::ItemOperationSource::Spell {
            spell_id: plan.spell_id.clone(),
            actor_id: actor_id.clone(),
        };
        match item_utility.action.as_str() {
            "identify" => {
                self.apply_item_identification(
                    player_index,
                    &resolved.item_instance_id,
                    source,
                    resolved.location.label().to_string(),
                    events,
                )?;
                Ok(SpellEffectOutcome::Applied)
            }
            "enchant_weapon" => {
                if !resolved.is_weapon {
                    return Err(StepError::new("invalid_target"));
                }
                let mut tags = item_utility.tags.clone();
                if let Some(status_kind) = effect.status_kind.as_ref() {
                    tags.push(status_kind.clone());
                }
                tags.sort();
                tags.dedup();
                let enchantment_instance_id = format!(
                    "spell:{}:{}:{}",
                    plan.spell_id,
                    self.current_time(),
                    resolved.item_instance_id
                );
                let remaining_rounds = self.spell_effect_remaining_rounds(effect.duration.as_ref());
                self.apply_weapon_enchantment(
                    player_index,
                    &resolved.item_instance_id,
                    source,
                    enchantment_instance_id,
                    item_utility.combat_add_rating_bonus.unwrap_or(0),
                    tags,
                    remaining_rounds,
                    events,
                )?;
                Ok(SpellEffectOutcome::Applied)
            }
            "transform_item" => {
                let Some(output_item_definition_id) =
                    item_utility.output_item_definition_id.as_deref()
                else {
                    return Ok(SpellEffectOutcome::Stubbed);
                };
                events.push(Event::ItemTransformed {
                    actor_id,
                    actor,
                    item_instance_id: item_instance_id.clone(),
                    old_item_definition_id: resolved.item_definition_id,
                    new_item_definition_id: output_item_definition_id.to_string(),
                    quantity: resolved.quantity,
                    location: location.label().to_string(),
                });
                self.replace_spell_item(
                    player_index,
                    item_instance_id,
                    *location,
                    output_item_definition_id,
                    events,
                )?;
                Ok(SpellEffectOutcome::Applied)
            }
            _ => Ok(SpellEffectOutcome::Stubbed),
        }
    }

    fn resolve_locate_hint(
        &self,
        player_index: usize,
        locate: &SpellLocateDef,
    ) -> (
        Option<crate::model::WorldSite>,
        Option<WorldPosition>,
        String,
    ) {
        match locate.subject.as_str() {
            "actor" => {
                let Some(actor) = self
                    .world
                    .actors
                    .iter()
                    .find(|actor| actor.is_alive() && actor.id.as_str() == locate.id)
                else {
                    return (
                        None,
                        None,
                        format!("actor {} is hidden or not found", locate.id),
                    );
                };
                if !self.actor_observed_by_player(player_index, actor) {
                    return (
                        None,
                        None,
                        format!("actor {} is hidden or unobserved", locate.id),
                    );
                }
                (
                    Some(actor.location.site()),
                    Some(actor.location.clone()),
                    format!(
                        "actor {} located in {} at {},{}",
                        locate.id,
                        actor.location.level,
                        actor.location.position.x,
                        actor.location.position.y
                    ),
                )
            }
            "item" => {
                if let Some(item) = self.ground_items().iter().find(|item| {
                    self.item_instance(&item.item_instance_id)
                        .is_ok_and(|instance| instance.definition_id == locate.id)
                }) {
                    if !self.world_position_observed_by_player(player_index, &item.location.clone())
                    {
                        return (
                            None,
                            None,
                            format!("item {} is hidden or unobserved", locate.id),
                        );
                    }
                    return (
                        Some(item.location.site()),
                        Some(item.location.clone()),
                        format!(
                            "item {} located in {} at {},{}",
                            locate.id,
                            item.location.level,
                            item.location.position.x,
                            item.location.position.y
                        ),
                    );
                }
                if let Some(actor) = self
                    .world
                    .actors
                    .iter()
                    .enumerate()
                    .find(|(actor_index, actor)| {
                        actor.is_alive()
                            && self.carried_item_ids(*actor_index).is_ok_and(|carried| {
                                carried.iter().any(|instance_id| {
                                    self.item_instance(instance_id)
                                        .is_ok_and(|instance| instance.definition_id == locate.id)
                                })
                            })
                    })
                    .map(|(_, actor)| actor)
                {
                    if !self.actor_observed_by_player(player_index, actor) {
                        return (
                            None,
                            None,
                            format!("item {} is hidden or unobserved", locate.id),
                        );
                    }
                    return (
                        Some(actor.location.site()),
                        Some(actor.location.clone()),
                        format!(
                            "item {} located in {} at {},{}",
                            locate.id,
                            actor.location.level,
                            actor.location.position.x,
                            actor.location.position.y
                        ),
                    );
                }
                (
                    None,
                    None,
                    format!("item {} is hidden or not found", locate.id),
                )
            }
            "level" => {
                let player_realm = &self.world.actors[player_index].location.realm;
                let site = crate::model::WorldSite::new(player_realm, &locate.id);
                if self
                    .definition
                    .world_template
                    .realms
                    .get(&site.realm)
                    .and_then(|realm| realm.levels.get(&site.level))
                    .is_none()
                {
                    return (
                        None,
                        None,
                        format!("level {} is hidden or not found", locate.id),
                    );
                }
                if !self.level_known_for_locate(player_index, &site) {
                    return (
                        None,
                        None,
                        format!("level {} is hidden or unobserved", locate.id),
                    );
                }
                (Some(site), None, format!("level {} located", locate.id))
            }
            _ => (None, None, "locate subject is unsupported".to_string()),
        }
    }

    fn resolve_scry_hint(
        &self,
        player_index: usize,
        spell_id: &str,
        scry: &SpellScryDef,
    ) -> (
        Option<crate::model::WorldSite>,
        Option<WorldPosition>,
        String,
    ) {
        match scry.scope.as_str() {
            "level" => {
                if self.level_known_for_locate(player_index, &scry.site) {
                    (
                        Some(scry.site.clone()),
                        None,
                        format!("scry {spell_id} located level {}", scry.site.label()),
                    )
                } else {
                    (
                        None,
                        None,
                        format!("scry {spell_id} is hidden or unobserved"),
                    )
                }
            }
            "coordinate" => {
                let Some(position) = scry.position else {
                    return (
                        None,
                        None,
                        format!("scry {spell_id} is hidden or unobserved"),
                    );
                };
                let location = WorldPosition::new(&scry.site.realm, &scry.site.level, position);
                if self.world_position_observed_by_player(player_index, &location) {
                    (
                        Some(scry.site.clone()),
                        Some(location),
                        format!(
                            "scry {spell_id} located in {} at {},{}",
                            scry.site.level, position.x, position.y
                        ),
                    )
                } else {
                    (
                        None,
                        None,
                        format!("scry {spell_id} is hidden or unobserved"),
                    )
                }
            }
            _ => (
                None,
                None,
                format!("scry {spell_id} is hidden or unobserved"),
            ),
        }
    }

    fn actor_observed_by_player(
        &self,
        player_index: usize,
        actor: &crate::model::ActorState,
    ) -> bool {
        self.world_position_observed_by_player(player_index, &actor.location.clone())
    }

    fn world_position_observed_by_player(
        &self,
        player_index: usize,
        position: &WorldPosition,
    ) -> bool {
        let player = &self.world.actors[player_index];
        if !player.location.same_site(position) {
            return false;
        }
        self.visible_tiles_for_actor_id(&player.id)
            .is_ok_and(|visible| visible.contains(position))
    }

    fn level_known_for_locate(&self, player_index: usize, site: &crate::model::WorldSite) -> bool {
        let player = &self.world.actors[player_index];
        player.location.site() == *site
            || self
                .automatic_navigation_edges_from(&player.location.site())
                .iter()
                .any(|(_, transition)| transition.target.site() == *site)
    }

    fn apply_door_secret_spell(
        &mut self,
        player_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(action) = Self::door_secret_action(effect) else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        match action {
            DoorSecretAction::Open | DoorSecretAction::Close => {
                self.apply_door_control_spell(player_index, plan, action, events)
            }
            DoorSecretAction::RevealSecret | DoorSecretAction::HideSecret => {
                self.apply_secret_transition_spell(player_index, plan, effect, action, events)
            }
        }
    }

    fn apply_door_control_spell(
        &mut self,
        player_index: usize,
        plan: &SpellCommandPlan,
        action: DoorSecretAction,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        match plan.target.as_ref() {
            Some(SpellTarget::Door { direction }) => match action {
                DoorSecretAction::Open => self.apply_door_open(player_index, *direction, events)?,
                DoorSecretAction::Close => {
                    self.apply_door_close(player_index, *direction, events)?
                }
                _ => return Ok(SpellEffectOutcome::Stubbed),
            },
            Some(SpellTarget::Coordinate { position }) => {
                let player = &self.world.actors[player_index];
                if !player.location.same_site(position) {
                    return Ok(SpellEffectOutcome::Stubbed);
                }
                let transition = self
                    .effective_transition_at(position)
                    .ok_or_else(|| StepError::new("invalid_target"))?;
                if transition.kind != NavigationKind::Door {
                    return Ok(SpellEffectOutcome::Stubbed);
                }
                if action == DoorSecretAction::Close
                    && self
                        .world
                        .actors
                        .iter()
                        .any(|actor| actor.is_alive() && actor.location == *position)
                {
                    return Err(StepError::new("invalid_target"));
                }
                let state = self
                    .world
                    .door_states
                    .get_mut(position)
                    .ok_or_else(|| StepError::new("invalid_target"))?;
                let (actor_id, actor) = {
                    let player = &self.world.actors[player_index];
                    (player.id.clone(), player.name.clone())
                };
                match action {
                    DoorSecretAction::Open => {
                        *state = true;
                        events.push(Event::DoorOpened {
                            actor_id,
                            actor,
                            location: position.clone(),
                        });
                    }
                    DoorSecretAction::Close => {
                        *state = false;
                        events.push(Event::DoorClosed {
                            actor_id,
                            actor,
                            location: position.clone(),
                        });
                    }
                    _ => return Ok(SpellEffectOutcome::Stubbed),
                }
            }
            _ => return Ok(SpellEffectOutcome::Stubbed),
        }
        Ok(SpellEffectOutcome::Applied)
    }

    fn apply_secret_transition_spell(
        &mut self,
        player_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        action: DoorSecretAction,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let positions = match plan.target.as_ref() {
            Some(SpellTarget::Coordinate { position }) => vec![position.clone()],
            None | Some(SpellTarget::None) => {
                let range = self
                    .definition
                    .catalog
                    .spells
                    .get(&plan.spell_id)
                    .and_then(|spell| Self::door_secret_range(spell, effect));
                self.matching_secret_transition_targets(player_index, action, range, None)
            }
            _ => return Ok(SpellEffectOutcome::Stubbed),
        };
        if positions.is_empty() {
            return Ok(SpellEffectOutcome::Stubbed);
        }
        let (actor_id, actor) = {
            let player = &self.world.actors[player_index];
            (player.id.clone(), player.name.clone())
        };
        for position in positions {
            let Some(transitions) = self.definition.world_template.navigation.get(&position) else {
                continue;
            };
            let Some(transition) = transitions
                .iter()
                .find(|transition| transition.hidden)
                .or_else(|| {
                    self.is_navigation_concealed(&position)
                        .then(|| transitions.first())
                        .flatten()
                })
            else {
                continue;
            };
            let transition_kind = transition_kind_label(transition.kind).to_string();
            let authored_hidden = transition.hidden;
            match action {
                DoorSecretAction::RevealSecret => {
                    self.remove_navigation_concealment_at(
                        &position,
                        TransitionConcealmentRemovalReasonV1::Revealed,
                        events,
                    );
                    if authored_hidden && !self.is_navigation_revealed(&position) {
                        self.set_navigation_revealed(&position, true)?;
                        events.push(Event::SecretTransitionRevealed {
                            actor_id: actor_id.clone(),
                            actor: actor.clone(),
                            location: position,
                            transition_kind,
                        });
                    }
                }
                DoorSecretAction::HideSecret => {
                    self.set_navigation_revealed(&position, false)?;
                    events.push(Event::SecretTransitionHidden {
                        actor_id: actor_id.clone(),
                        actor: actor.clone(),
                        location: position,
                        transition_kind,
                    });
                }
                _ => return Ok(SpellEffectOutcome::Stubbed),
            }
        }
        Ok(SpellEffectOutcome::Applied)
    }

    fn apply_tile_overlay_spell(
        &mut self,
        caster_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(overlay) = effect.terrain_overlay.as_ref() else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        if overlay.passability.is_none()
            && overlay.sight.is_none()
            && overlay.hazard.is_none()
            && overlay.move_cost.is_none()
        {
            return Ok(SpellEffectOutcome::Stubbed);
        }
        let Some(positions) = self.tile_positions_for_spell_target(plan, effect) else {
            return Ok(SpellEffectOutcome::Stubbed);
        };

        if overlay.passability.as_deref() == Some("remove_overlay")
            || overlay.sight.as_deref() == Some("remove_overlay")
        {
            let remove_passability = overlay.passability.as_deref() == Some("remove_overlay");
            let remove_sight = overlay.sight.as_deref() == Some("remove_overlay");
            for position in positions {
                self.remove_tile_effects_at(
                    &position,
                    remove_passability,
                    remove_sight,
                    &plan.spell_id,
                    events,
                );
            }
            return Ok(SpellEffectOutcome::Applied);
        }

        let mut tags = self.spell_effect_tags(effect);
        if let Some(hazard) = overlay.hazard.as_ref()
            && hazard != "unknown"
        {
            tags.push(hazard.clone());
        }
        tags.sort();
        tags.dedup();

        for position in positions {
            let effect_state = TileEffectState {
                instance_id: format!(
                    "tile:{}:{}:{}:{}:{}",
                    plan.spell_id,
                    self.current_time(),
                    position.level,
                    position.position.x,
                    position.position.y
                ),
                effect_id: plan.spell_id.clone(),
                source: ActiveEffectSource {
                    kind: "spell".to_string(),
                    id: plan.spell_id.clone(),
                },
                source_actor_id: Some(self.world.actors[caster_index].id.clone()),
                hostile_authority: plan.hostility_authorization.and_then(|authorization| {
                    self.world.actors[caster_index].character_id.clone().map(
                        |credited_character_id| HostileEffectAuthority {
                            credited_actor_id: self.world.actors[caster_index].id.clone(),
                            credited_character_id,
                            authorization,
                        },
                    )
                }),
                location: position,
                kind: effect.family.label().to_string(),
                tags: tags.clone(),
                potency: effect.potency.unwrap_or(0),
                remaining_rounds: self.spell_effect_remaining_rounds(effect.duration.as_ref()),
                passability: overlay.passability.clone(),
                sight: overlay.sight.clone(),
                hazard: overlay.hazard.clone(),
                move_cost: overlay.move_cost,
                tick_interval_rounds: self
                    .spell_effect_tick_interval_rounds(effect.duration.as_ref()),
                last_ticked_at: self.current_time(),
            };
            self.apply_tile_effect_state(effect_state, events);
        }

        Ok(SpellEffectOutcome::Applied)
    }

    fn apply_active_effect_spell(
        &mut self,
        player_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(target_index) = self.resolve_spell_effect_target_index(player_index, plan) else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        if matches!(
            effect.family,
            SpellEffectFamily::ControlStatus | SpellEffectFamily::Poison
        ) && let Some(resolution) =
            self.resolve_spell_resistance(target_index, &plan.spell_id, effect, None, events)
            && resolution.success
        {
            return Ok(SpellEffectOutcome::Applied);
        }
        let target_actor_id = self.world.actors[target_index].id.clone();
        let effect_state = ActiveEffectState {
            instance_id: format!(
                "spell:{}:{}:{}",
                plan.spell_id,
                self.current_time(),
                target_actor_id
            ),
            effect_id: plan.spell_id.clone(),
            source: ActiveEffectSource {
                kind: "spell".to_string(),
                id: plan.spell_id.clone(),
            },
            source_actor_id: Some(self.world.actors[player_index].id.clone()),
            hostile_authority: (target_index != player_index)
                .then_some(plan.hostility_authorization)
                .flatten()
                .and_then(|authorization| {
                    self.world.actors[player_index].character_id.clone().map(
                        |credited_character_id| HostileEffectAuthority {
                            credited_actor_id: self.world.actors[player_index].id.clone(),
                            credited_character_id,
                            authorization,
                        },
                    )
                }),
            spell_damage_credit: if effect.family == SpellEffectFamily::Poison {
                plan.damage_credit(&self.world.actors[player_index].id)
            } else {
                None
            },
            kind: effect.family.label().to_string(),
            tags: self.spell_effect_tags(effect),
            potency: effect.potency.unwrap_or(0),
            remaining_rounds: self.spell_effect_remaining_rounds(effect.duration.as_ref()),
            until_condition: None,
            stacking: self.spell_effect_stacking(effect),
            start_delay_rounds: self.spell_effect_start_delay_rounds(effect),
            tick_interval_rounds: self.spell_effect_tick_interval_rounds(effect.duration.as_ref()),
            suppresses_action: self.spell_effect_suppresses_action(effect),
            resistance_boosts: Self::resistance_boosts_from_effect(effect),
            last_ticked_at: self.current_time(),
        };
        self.apply_spell_active_effect_state(target_index, effect_state, events);
        Ok(SpellEffectOutcome::Applied)
    }

    fn apply_poison_cure_spell(
        &mut self,
        player_index: usize,
        plan: &SpellCommandPlan,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(target_index) = self.resolve_spell_effect_target_index(player_index, plan) else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        self.remove_active_effects_matching_tag_from_actor(
            target_index,
            "poison",
            "poison_cure",
            events,
        );
        Ok(SpellEffectOutcome::Applied)
    }

    fn apply_direct_damage_spell(
        &mut self,
        player_index: usize,
        plan: &SpellCommandPlan,
        effect: &crate::content::SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(potency) = effect.potency else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        let target_index = match plan.target.as_ref() {
            Some(SpellTarget::Actor { actor_id }) => self
                .world
                .actors
                .iter()
                .position(|actor| actor.is_alive() && actor.id == *actor_id),
            Some(SpellTarget::Path { .. }) => plan
                .path_plan
                .as_ref()
                .and_then(|path| path.final_position.as_ref())
                .and_then(|position| {
                    self.world.actors.iter().position(|actor| {
                        actor.is_alive()
                            && actor.id != self.world.actors[player_index].id
                            && actor.location.level == position.level
                            && actor.location.position == position.position
                    })
                }),
            Some(SpellTarget::Area { center })
            | Some(SpellTarget::Coordinate { position: center }) => {
                self.world.actors.iter().position(|actor| {
                    actor.is_alive()
                        && actor.id != self.world.actors[player_index].id
                        && actor.location.level == center.level
                        && actor.location.position == center.position
                })
            }
            _ => None,
        };
        let Some(target_index) = target_index else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        let damage = self
            .resolve_spell_resistance(target_index, &plan.spell_id, effect, Some(potency), events)
            .and_then(|resolution| resolution.resolved_damage)
            .unwrap_or(potency);
        if damage == 0 {
            return Ok(SpellEffectOutcome::Applied);
        }

        let (caster_id, caster_name, spell_id, spell_name) = {
            let caster = &self.world.actors[player_index];
            (
                caster.id.clone(),
                caster.name.clone(),
                plan.spell_id.clone(),
                plan.spell_name.clone(),
            )
        };

        let (target_id, target_name, location) = {
            let target = &self.world.actors[target_index];
            (
                target.id.clone(),
                target.name.clone(),
                target.location.clone(),
            )
        };
        let damage_kind = effect.damage_kind.clone();
        let cause = if damage_kind.as_deref() == Some("fire") {
            DeathCause::Fire
        } else {
            DeathCause::OtherMagic
        };
        let spell_damage_credit = plan.damage_credit(&caster_id);
        self.apply_damage_and_resolve_defeat(
            target_index,
            damage,
            DefeatContext {
                cause,
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
                location,
                damage_kind,
                damage: outcome.applied,
                hp: outcome.hp_after,
            },
            |_| {},
        )?;
        Ok(SpellEffectOutcome::Applied)
    }

    fn apply_healing_spell(
        &mut self,
        player_index: usize,
        plan: &SpellCommandPlan,
        effect: &crate::content::SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(potency) = effect.potency else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        let target_index = match plan.target.as_ref() {
            Some(SpellTarget::SelfTarget) => player_index,
            Some(SpellTarget::Actor { actor_id }) => match self
                .world
                .actors
                .iter()
                .position(|actor| actor.is_alive() && actor.id == *actor_id)
            {
                Some(index) => index,
                None => return Ok(SpellEffectOutcome::Stubbed),
            },
            _ => return Ok(SpellEffectOutcome::Stubbed),
        };

        let (caster_id, caster_name, spell_id, spell_name) = {
            let caster = &self.world.actors[player_index];
            (
                caster.id.clone(),
                caster.name.clone(),
                plan.spell_id.clone(),
                plan.spell_name.clone(),
            )
        };
        let delta = self.change_hp(target_index, potency)?;
        let (target_id, target_name, location) = {
            let target = &self.world.actors[target_index];
            (
                target.id.clone(),
                target.name.clone(),
                target.location.clone(),
            )
        };

        events.push(Event::SpellHealed {
            caster_id,
            caster: caster_name,
            spell_id,
            spell_name,
            target_id,
            target: target_name,
            location,
            amount: delta.actual,
            hp: delta.current,
        });
        Ok(SpellEffectOutcome::Applied)
    }

    fn resolve_spell_effect_target_index(
        &self,
        player_index: usize,
        plan: &SpellCommandPlan,
    ) -> Option<usize> {
        match plan.target.as_ref() {
            Some(SpellTarget::SelfTarget) => Some(player_index),
            Some(SpellTarget::Actor { actor_id }) => self
                .world
                .actors
                .iter()
                .position(|actor| actor.is_alive() && actor.id == *actor_id),
            _ => None,
        }
    }

    fn spell_effect_tags(&self, effect: &SpellEffectDef) -> Vec<String> {
        let mut tags = Vec::new();
        if let Some(status_kind) = effect.status_kind.as_ref() {
            tags.push(status_kind.clone());
        }
        tags
    }

    fn spell_effect_remaining_rounds(&self, duration: Option<&SpellDurationDef>) -> Option<u32> {
        let duration = duration?;
        if duration.policy != SpellDurationPolicy::Rounds {
            return None;
        }
        duration
            .rounds
            .and_then(|rounds| u32::try_from(rounds).ok())
    }

    fn spell_effect_start_delay_rounds(&self, effect: &SpellEffectDef) -> u32 {
        effect
            .start_delay_rounds
            .and_then(|rounds| u32::try_from(rounds).ok())
            .unwrap_or(0)
    }

    fn spell_effect_tick_interval_rounds(&self, duration: Option<&SpellDurationDef>) -> u32 {
        duration
            .and_then(|duration| duration.tick_interval_rounds)
            .and_then(|rounds| u32::try_from(rounds).ok())
            .unwrap_or(1)
    }

    fn spell_effect_suppresses_action(&self, effect: &SpellEffectDef) -> bool {
        effect.suppresses_action.unwrap_or(matches!(
            effect.status_kind.as_deref(),
            Some("stun" | "fear")
        ))
    }

    fn spell_effect_stacking(&self, effect: &SpellEffectDef) -> ActiveEffectStackingPolicy {
        match effect.stacking.as_deref() {
            Some("stack_instance") => ActiveEffectStackingPolicy::StackInstance,
            Some("refresh_duration") => ActiveEffectStackingPolicy::RefreshDuration,
            _ => ActiveEffectStackingPolicy::ReplaceSameKind,
        }
    }

    fn apply_spell_active_effect_state(
        &mut self,
        actor_index: usize,
        effect_state: ActiveEffectState,
        events: &mut Vec<Event>,
    ) {
        let actor = &mut self.world.actors[actor_index];
        let matching_index = actor.active_effects.iter().position(|existing| {
            existing.source.kind == effect_state.source.kind
                && existing.source.id == effect_state.source.id
        });
        match effect_state.stacking {
            ActiveEffectStackingPolicy::ReplaceSameKind => {
                actor.active_effects.retain(|existing| {
                    !((existing.source.kind == effect_state.source.kind
                        && existing.source.id == effect_state.source.id)
                        || existing.effect_id == effect_state.effect_id)
                });
                actor.active_effects.push(effect_state.clone());
                self.emit_effect_applied(actor_index, &effect_state, events);
            }
            ActiveEffectStackingPolicy::RefreshDuration => {
                let refreshed_effect = if let Some(index) = matching_index {
                    let mut refreshed_effect = effect_state.clone();
                    refreshed_effect.instance_id = actor.active_effects[index].instance_id.clone();
                    actor.active_effects[index] = refreshed_effect.clone();
                    refreshed_effect
                } else {
                    actor.active_effects.push(effect_state.clone());
                    effect_state
                };
                self.emit_effect_applied(actor_index, &refreshed_effect, events);
            }
            ActiveEffectStackingPolicy::StackInstance => {
                actor.active_effects.push(effect_state.clone());
                self.emit_effect_applied(actor_index, &effect_state, events);
            }
        }
    }

    fn emit_effect_applied(
        &self,
        actor_index: usize,
        effect_state: &ActiveEffectState,
        events: &mut Vec<Event>,
    ) {
        let actor = &self.world.actors[actor_index];
        events.push(Event::EffectApplied {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            location: actor.location.clone(),
            instance_id: effect_state.instance_id.clone(),
            effect_id: effect_state.effect_id.clone(),
            source_kind: effect_state.source.kind.clone(),
            source_id: effect_state.source.id.clone(),
            kind: effect_state.kind.clone(),
            tags: effect_state.tags.clone(),
            potency: effect_state.potency,
            remaining_rounds: effect_state.remaining_rounds,
        });
    }
}
