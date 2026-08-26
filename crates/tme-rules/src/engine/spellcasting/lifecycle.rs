use crate::content::SpellDef;
use crate::events::{Event, SpellCastFailure, SpellFizzleCause};
use crate::model::{
    LogicalTime, MonsterAbilityTargetPolicy, SpellCastClass, SpellCastingMethod, SpellTarget,
    SpellTargetKind, ThaumAboveSkillPlan, ThaumAboveSkillReceipt, WarmedSpellState,
    WarmedSpellStatus,
};
use crate::view::ActionBlockedReasonV1;

use super::{SpellCommandPlan, SpellEffectOutcome};
use crate::engine::{Engine, StepError};

impl Engine {
    pub(in crate::engine) fn shared_player_spell_plan(
        &self,
        player_index: usize,
        spell_id: &str,
    ) -> Result<SpellCommandPlan, ActionBlockedReasonV1> {
        let player = &self.world.actors[player_index];
        let spell = self
            .definition
            .catalog
            .spells
            .get(spell_id)
            .ok_or(ActionBlockedReasonV1::NoSuchSpell)?;
        let casting = spell
            .casting
            .as_ref()
            .ok_or(ActionBlockedReasonV1::InvalidTarget)?;
        let character = player
            .character
            .as_ref()
            .ok_or(ActionBlockedReasonV1::SpellNotKnown)?;
        let known_spell = character
            .known_spells
            .iter()
            .find(|known| known.spell_id == spell_id)
            .ok_or(ActionBlockedReasonV1::SpellNotKnown)?;
        if let Some(ref lane) = spell.lane {
            if known_spell.lane != *lane {
                return Err(ActionBlockedReasonV1::WrongClass);
            }
            let accessible_lanes = crate::engine::action_context::class_spell_lanes(
                character.identity.current_class_id.as_str(),
            );
            if !accessible_lanes.contains(&lane.as_str()) {
                return Err(ActionBlockedReasonV1::WrongClass);
            }
            if lane == "knight_magic"
                && !self
                    .has_worn_spell_focus(player_index, lane)
                    .map_err(|_| ActionBlockedReasonV1::MissingRequiredItem)?
            {
                return Err(ActionBlockedReasonV1::MissingRequiredItem);
            }
        }
        let mut thaum_above_skill = None;
        if let Some(requirement) = spell.skill_requirement {
            let lane = spell.lane.as_deref().unwrap_or("");
            let current_skill_level = character
                .skill_ledger
                .iter()
                .find(|entry| entry.track_id == lane)
                .map_or(0, |entry| entry.level);
            if i32::from(current_skill_level) < requirement {
                if character.identity.current_class_id == "thaumaturge"
                    && lane == "thaumaturge_magic"
                {
                    let skill_requirement = u8::try_from(requirement)
                        .map_err(|_| ActionBlockedReasonV1::SkillLevelTooLow)?;
                    let gap = skill_requirement
                        .checked_sub(current_skill_level)
                        .ok_or(ActionBlockedReasonV1::SkillLevelTooLow)?;
                    let rules = &self.definition.catalog.rules.magic.thaum_above_skill;
                    let penalty = u32::from(gap)
                        .checked_mul(rules.penalty_per_missing_level)
                        .ok_or(ActionBlockedReasonV1::SkillLevelTooLow)?;
                    let success_threshold = rules
                        .roll_denominator
                        .saturating_sub(penalty)
                        .max(rules.minimum_success_threshold);
                    thaum_above_skill = Some(ThaumAboveSkillPlan {
                        current_skill_level,
                        skill_requirement,
                        gap,
                        roll_denominator: rules.roll_denominator,
                        success_threshold,
                    });
                } else {
                    return Err(ActionBlockedReasonV1::SkillLevelTooLow);
                }
            }
        }
        Ok(SpellCommandPlan {
            spell_id: spell.id.clone(),
            spell_name: spell.name.clone(),
            lane: spell.lane.clone().unwrap_or_default(),
            mp_cost: spell.mp_cost,
            stamina_cost: spell.stamina_cost,
            target: None,
            casting_method: casting.method,
            cast_class: casting.cast_class,
            path_plan: None,
            thaum_above_skill,
            hostility_authorization: None,
            damage_credit_override: None,
        })
    }

    pub(in crate::engine) fn require_spell_resources(
        &self,
        player_index: usize,
        plan: &SpellCommandPlan,
    ) -> Result<(), ActionBlockedReasonV1> {
        let player = &self.world.actors[player_index];
        if plan.mp_cost.is_some_and(|cost| player.mp < cost) {
            return Err(ActionBlockedReasonV1::InsufficientMagicPoints);
        }
        if plan.stamina_cost.is_some_and(|cost| player.stamina < cost) {
            return Err(ActionBlockedReasonV1::InsufficientStamina);
        }
        Ok(())
    }

    fn plan_cast_target(
        &self,
        player_index: usize,
        spell: &SpellDef,
        plan: &mut SpellCommandPlan,
        target: Option<&SpellTarget>,
    ) -> Result<(), ActionBlockedReasonV1> {
        match plan.cast_class {
            SpellCastClass::Path => {
                let Some(SpellTarget::Path { directions }) = target else {
                    return Err(ActionBlockedReasonV1::InvalidTarget);
                };
                plan.path_plan = Some(self.evaluate_spell_path(player_index, spell, directions)?);
                plan.target = Some(SpellTarget::Path {
                    directions: directions.clone(),
                });
            }
            SpellCastClass::PathOrCharacter => match target {
                Some(SpellTarget::Path { directions }) => {
                    plan.path_plan =
                        Some(self.evaluate_spell_path(player_index, spell, directions)?);
                    plan.target = Some(SpellTarget::Path {
                        directions: directions.clone(),
                    });
                }
                Some(SpellTarget::Actor { .. }) => {
                    plan.target = target.cloned();
                    self.validate_spell_target(player_index, spell, plan.target.as_ref())?;
                }
                _ => return Err(ActionBlockedReasonV1::InvalidTarget),
            },
            _ => {
                plan.target = self.normalize_spell_target(spell, target);
                self.validate_spell_target(player_index, spell, plan.target.as_ref())?;
            }
        }
        Ok(())
    }

    pub(in crate::engine) fn validate_direct_spell_command(
        &self,
        player_index: usize,
        spell_id: &str,
        target: Option<&SpellTarget>,
    ) -> Result<SpellCommandPlan, ActionBlockedReasonV1> {
        let mut plan = self.validate_direct_spell_eligibility(player_index, spell_id)?;
        let spell = &self.definition.catalog.spells[spell_id];
        self.plan_cast_target(player_index, spell, &mut plan, target)?;
        Ok(plan)
    }

    pub(in crate::engine) fn validate_direct_spell_eligibility(
        &self,
        player_index: usize,
        spell_id: &str,
    ) -> Result<SpellCommandPlan, ActionBlockedReasonV1> {
        let plan = self.shared_player_spell_plan(player_index, spell_id)?;
        if plan.casting_method != SpellCastingMethod::Direct {
            return Err(ActionBlockedReasonV1::SpellRequiresWarming);
        }
        self.require_spell_resources(player_index, &plan)?;
        Ok(plan)
    }

    pub(in crate::engine) fn validate_warm_spell_command(
        &self,
        player_index: usize,
        spell_id: &str,
    ) -> Result<SpellCommandPlan, ActionBlockedReasonV1> {
        let plan = self.shared_player_spell_plan(player_index, spell_id)?;
        if plan.casting_method != SpellCastingMethod::WarmThenCast {
            return Err(ActionBlockedReasonV1::SpellCastsDirectly);
        }
        Ok(plan)
    }

    pub(in crate::engine) fn validate_warmed_spell_command(
        &self,
        player_index: usize,
        target: Option<&SpellTarget>,
    ) -> Result<SpellCommandPlan, ActionBlockedReasonV1> {
        self.validate_warmed_spell_command_at_time(player_index, target, self.current_time())
    }

    pub(in crate::engine) fn validate_warmed_spell_command_at_time(
        &self,
        player_index: usize,
        target: Option<&SpellTarget>,
        _validation_time: LogicalTime,
    ) -> Result<SpellCommandPlan, ActionBlockedReasonV1> {
        let warmed = self.world.actors[player_index]
            .warmed_spell
            .as_ref()
            .ok_or(ActionBlockedReasonV1::NoWarmedSpell)?;
        if warmed.status != WarmedSpellStatus::Ready {
            return Err(ActionBlockedReasonV1::SpellStillWarming);
        }
        let spell_id = warmed.spell_id.clone();
        let mut plan = self.validate_warmed_spell_eligibility(player_index)?;
        let spell = &self.definition.catalog.spells[&spell_id];
        self.plan_cast_target(player_index, spell, &mut plan, target)?;
        Ok(plan)
    }

    pub(in crate::engine) fn validate_warmed_spell_eligibility(
        &self,
        player_index: usize,
    ) -> Result<SpellCommandPlan, ActionBlockedReasonV1> {
        let warmed = self.world.actors[player_index]
            .warmed_spell
            .as_ref()
            .ok_or(ActionBlockedReasonV1::NoWarmedSpell)?;
        if warmed.status != WarmedSpellStatus::Ready {
            return Err(ActionBlockedReasonV1::SpellStillWarming);
        }
        let plan = self.shared_player_spell_plan(player_index, &warmed.spell_id)?;
        if plan.casting_method != SpellCastingMethod::WarmThenCast {
            return Err(ActionBlockedReasonV1::SpellCastsDirectly);
        }
        self.require_spell_resources(player_index, &plan)?;
        Ok(plan)
    }

    pub(in crate::engine) fn commit_authored_spell_costs(
        &mut self,
        actor_index: usize,
        plan: &SpellCommandPlan,
    ) -> Result<(), StepError> {
        if let Some(mp_cost) = plan.mp_cost {
            self.change_mp(actor_index, -mp_cost)?;
        }
        if let Some(stamina_cost) = plan.stamina_cost {
            self.change_stamina(actor_index, -stamina_cost)?;
        }
        Ok(())
    }

    fn take_warmed_spell(&mut self, actor_index: usize) -> Option<WarmedSpellState> {
        self.world.actors[actor_index].warmed_spell.take()
    }

    pub(in crate::engine) fn fizzle_warmed_spell(
        &mut self,
        actor_index: usize,
        cause: SpellFizzleCause,
        events: &mut Vec<Event>,
    ) -> bool {
        let Some(warmed) = self.take_warmed_spell(actor_index) else {
            return false;
        };
        let actor = &self.world.actors[actor_index];
        let spell_name = self
            .definition
            .catalog
            .spells
            .get(&warmed.spell_id)
            .map(|spell| spell.name.clone())
            .unwrap_or_else(|| warmed.spell_id.clone());
        events.push(Event::SpellFizzled {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            spell_id: warmed.spell_id,
            spell_name,
            cause,
        });
        true
    }

    pub(in crate::engine) fn apply_player_warm_spell(
        &mut self,
        player_index: usize,
        spell_id: &str,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let plan = self
            .validate_warm_spell_command(player_index, spell_id)
            .map_err(Self::spell_command_error)?;
        self.fizzle_warmed_spell(
            player_index,
            SpellFizzleCause::Replaced {
                replacing_spell_id: plan.spell_id.clone(),
                replacing_spell_name: plan.spell_name.clone(),
            },
            events,
        );
        let warmed_at = self.current_time();
        let ready_at =
            warmed_at.saturating_add_rounds(self.definition.catalog.rules.magic.warmup.units);
        self.world.actors[player_index].warmed_spell = Some(WarmedSpellState {
            spell_id: plan.spell_id.clone(),
            warmed_at,
            ready_at,
            status: WarmedSpellStatus::Warming,
        });
        let actor = &self.world.actors[player_index];
        events.push(Event::SpellWarmed {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            spell_id: plan.spell_id,
            spell_name: plan.spell_name,
            warmed_at,
            ready_at,
        });
        Ok(())
    }

    pub(in crate::engine) fn apply_player_fizzle_warmed_spell(
        &mut self,
        player_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        if !self.fizzle_warmed_spell(player_index, SpellFizzleCause::Canceled, events) {
            return Err(Self::spell_command_error(
                ActionBlockedReasonV1::NoWarmedSpell,
            ));
        }
        Ok(())
    }

    pub(in crate::engine) fn apply_player_rest(
        &mut self,
        player_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        self.fizzle_warmed_spell(player_index, SpellFizzleCause::Rest, events);
        Ok(())
    }

    pub(in crate::engine) fn transition_warmed_spells_ready(
        &mut self,
        boundary_at: LogicalTime,
        events: &mut Vec<Event>,
    ) {
        for actor_index in 0..self.world.actors.len() {
            let transition = self.world.actors[actor_index]
                .warmed_spell
                .as_ref()
                .filter(|warmed| {
                    warmed.status == WarmedSpellStatus::Warming && boundary_at >= warmed.ready_at
                })
                .map(|warmed| (warmed.spell_id.clone(), warmed.ready_at));
            let Some((spell_id, ready_at)) = transition else {
                continue;
            };
            self.world.actors[actor_index]
                .warmed_spell
                .as_mut()
                .expect("readiness transition retained warmed state")
                .status = WarmedSpellStatus::Ready;
            let actor = &self.world.actors[actor_index];
            let spell_name = self
                .definition
                .catalog
                .spells
                .get(&spell_id)
                .map(|spell| spell.name.clone())
                .unwrap_or_else(|| spell_id.clone());
            events.push(Event::WarmedSpellReady {
                actor_id: actor.id.clone(),
                actor: actor.name.clone(),
                spell_id,
                spell_name,
                ready_at,
            });
        }
    }

    pub(in crate::engine) fn fizzle_warmed_spell_for_damage(
        &mut self,
        actor_index: usize,
        applied_damage: i32,
        hp_before: i32,
        events: &mut Vec<Event>,
    ) -> bool {
        if applied_damage <= 0 || hp_before <= 0 {
            return false;
        }
        let rule = &self.definition.catalog.rules.magic.damage_interruption;
        let exceeds = i64::from(applied_damage) * i64::from(rule.denominator)
            > i64::from(hp_before) * i64::from(rule.numerator);
        exceeds
            && self.fizzle_warmed_spell(
                actor_index,
                SpellFizzleCause::Damage {
                    applied_damage,
                    hp_before,
                },
                events,
            )
    }

    pub(in crate::engine) fn apply_player_direct_cast(
        &mut self,
        player_index: usize,
        spell_id: &str,
        target: Option<&SpellTarget>,
        authorization: crate::model::HostilityAuthorization,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let mut plan = self
            .validate_direct_spell_command(player_index, spell_id, target)
            .map_err(Self::spell_command_error)?;
        plan.hostility_authorization = Some(authorization);
        self.validate_direct_hostility_authorization(player_index, &plan)?;
        self.commit_authored_spell_costs(player_index, &plan)?;
        if !self.commit_thaum_above_skill_attempt(player_index, &plan, events) {
            return Ok(());
        }
        if self.emit_committed_path_failure(player_index, &plan, events) {
            return Ok(());
        }
        self.execute_committed_spell(player_index, &plan, false, events)
    }

    pub(in crate::engine) fn validate_direct_hostility_authorization(
        &self,
        player_index: usize,
        plan: &SpellCommandPlan,
    ) -> Result<(), StepError> {
        let Some(spell) = self.definition.catalog.spells.get(&plan.spell_id) else {
            return Ok(());
        };
        if !spell.social.hostile_act {
            return Ok(());
        }
        let Some(SpellTarget::Actor { actor_id }) = plan.target.as_ref() else {
            return Ok(());
        };
        let target_index = self
            .live_actor_by_id(actor_id)
            .ok_or_else(|| StepError::new("invalid_hostile_target"))?;
        if spell
            .effect
            .as_ref()
            .is_some_and(|effect| self.spell_specific_hostile_target_allowed(target_index, effect))
        {
            return Ok(());
        }
        let authorization = plan
            .hostility_authorization
            .ok_or_else(|| StepError::new("player hostile spell has no authorization"))?;
        let assessment = self.attack_safety_assessment(player_index, target_index)?;
        if assessment.safety.permits(authorization) {
            Ok(())
        } else {
            Err(StepError::new(
                if matches!(assessment.safety, crate::model::AttackSafety::Protected) {
                    "protected_target_requires_confirmation"
                } else {
                    "invalid_hostile_target"
                },
            ))
        }
    }

    pub(in crate::engine) fn apply_player_cast_warmed_spell(
        &mut self,
        player_index: usize,
        target: Option<&SpellTarget>,
        authorization: crate::model::HostilityAuthorization,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let mut plan = self
            .validate_warmed_spell_command(player_index, target)
            .map_err(Self::spell_command_error)?;
        plan.hostility_authorization = Some(authorization);
        self.validate_direct_hostility_authorization(player_index, &plan)?;
        self.commit_authored_spell_costs(player_index, &plan)?;
        self.take_warmed_spell(player_index)
            .expect("validated warmed cast retained its slot");
        if !self.commit_thaum_above_skill_attempt(player_index, &plan, events) {
            return Ok(());
        }
        if self.emit_committed_path_failure(player_index, &plan, events) {
            return Ok(());
        }
        self.execute_committed_spell(player_index, &plan, true, events)
    }

    fn emit_committed_path_failure(
        &self,
        actor_index: usize,
        plan: &SpellCommandPlan,
        events: &mut Vec<Event>,
    ) -> bool {
        let Some(reason) = plan
            .path_plan
            .as_ref()
            .and_then(|path_plan| path_plan.failure)
        else {
            return false;
        };
        let actor = &self.world.actors[actor_index];
        events.push(Event::SpellCastFailed {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            spell_id: plan.spell_id.clone(),
            spell_name: plan.spell_name.clone(),
            target: plan.target.clone(),
            failure: SpellCastFailure::InvalidPath { reason },
            mp_cost: plan.mp_cost,
            stamina_cost: plan.stamina_cost,
        });
        true
    }

    fn commit_thaum_above_skill_attempt(
        &mut self,
        actor_index: usize,
        plan: &SpellCommandPlan,
        events: &mut Vec<Event>,
    ) -> bool {
        let Some(attempt) = plan.thaum_above_skill else {
            return true;
        };
        debug_assert_eq!(attempt.roll_denominator, 20);
        let roll = self.rng.roll_d20();
        let receipt = ThaumAboveSkillReceipt {
            current_skill_level: attempt.current_skill_level,
            skill_requirement: attempt.skill_requirement,
            gap: attempt.gap,
            roll_denominator: attempt.roll_denominator,
            success_threshold: attempt.success_threshold,
            roll,
            success: roll <= attempt.success_threshold,
        };
        let actor = &self.world.actors[actor_index];
        events.push(Event::ThaumAboveSkillEvaluated {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            spell_id: plan.spell_id.clone(),
            spell_name: plan.spell_name.clone(),
            track_id: plan.lane.clone(),
            current_skill_level: receipt.current_skill_level,
            skill_requirement: receipt.skill_requirement,
            gap: receipt.gap,
            roll_denominator: receipt.roll_denominator,
            success_threshold: receipt.success_threshold,
            roll: receipt.roll,
            success: receipt.success,
        });
        if !receipt.success {
            events.push(Event::SpellCastFailed {
                actor_id: actor.id.clone(),
                actor: actor.name.clone(),
                spell_id: plan.spell_id.clone(),
                spell_name: plan.spell_name.clone(),
                target: plan.target.clone(),
                failure: SpellCastFailure::AboveSkillAttempt,
                mp_cost: plan.mp_cost,
                stamina_cost: plan.stamina_cost,
            });
        }
        receipt.success
    }

    fn execute_committed_spell(
        &mut self,
        actor_index: usize,
        plan: &SpellCommandPlan,
        warmed: bool,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        if let Some(town_law) = self.town_law_consequence_plan(
            actor_index,
            &plan.spell_id,
            &self.world.actors[actor_index].location.site(),
        )? {
            self.commit_town_law_consequence(&town_law, events)?;
        }
        let (actor_id, actor_name) = {
            let actor = &self.world.actors[actor_index];
            (actor.id.clone(), actor.name.clone())
        };
        events.push(Event::SpellCastCommitted {
            actor_id: actor_id.clone(),
            actor: actor_name.clone(),
            spell_id: plan.spell_id.clone(),
            spell_name: plan.spell_name.clone(),
            target: plan.target.clone(),
            casting_method: plan.casting_method,
            mp_cost: plan.mp_cost,
            stamina_cost: plan.stamina_cost,
        });
        if warmed {
            events.push(Event::WarmedSpellCast {
                actor_id: actor_id.clone(),
                actor: actor_name.clone(),
                spell_id: plan.spell_id.clone(),
                spell_name: plan.spell_name.clone(),
                target: plan.target.clone(),
            });
        }
        match self
            .execute_actor_spell_effect(actor_index, plan, events)?
            .outcome
        {
            SpellEffectOutcome::Applied => {
                let current_actor_index = self
                    .world
                    .actors
                    .iter()
                    .position(|actor| actor.id == actor_id)
                    .ok_or_else(|| StepError::new("spell caster disappeared during effect"))?;
                self.award_magic_casting_practice(current_actor_index, plan, events)?;
            }
            SpellEffectOutcome::Failed => {}
            SpellEffectOutcome::Stubbed => {
                events.push(Event::SpellCastStubbed {
                    actor_id: actor_id.clone(),
                    actor: actor_name,
                    spell_id: plan.spell_id.clone(),
                    spell_name: plan.spell_name.clone(),
                    target: plan.target.clone(),
                    casting_method: plan.casting_method,
                    lane: plan.lane.clone(),
                    mp_cost: plan.mp_cost,
                    stamina_cost: plan.stamina_cost,
                });
                let current_actor_index = self
                    .world
                    .actors
                    .iter()
                    .position(|actor| actor.id == actor_id)
                    .ok_or_else(|| StepError::new("spell caster disappeared during effect"))?;
                self.award_magic_casting_practice(current_actor_index, plan, events)?;
            }
        }
        Ok(())
    }

    pub(in crate::engine) fn monster_spell_plan(
        &self,
        caster_index: usize,
        target_index: Option<usize>,
        spell_id: &str,
        target_policy: MonsterAbilityTargetPolicy,
    ) -> Result<SpellCommandPlan, StepError> {
        let caster = self
            .world
            .actors
            .get(caster_index)
            .filter(|actor| actor.is_alive())
            .ok_or_else(|| StepError::new("invalid_caster"))?;
        let spell = self
            .definition
            .catalog
            .spells
            .get(spell_id)
            .ok_or_else(|| StepError::new("no_such_spell"))?;
        let casting = spell
            .casting
            .as_ref()
            .filter(|casting| casting.method == SpellCastingMethod::Direct)
            .ok_or_else(|| StepError::new("unsupported monster ability casting method"))?;
        let resolved_target = match (
            spell.target.as_ref().map(|target| target.kind),
            target_policy,
        ) {
            (Some(SpellTargetKind::SelfTarget), _)
            | (_, MonsterAbilityTargetPolicy::SelfTarget) => Some(SpellTarget::SelfTarget),
            (Some(SpellTargetKind::Actor), MonsterAbilityTargetPolicy::NearestHostile) => {
                let target_index = target_index.ok_or_else(|| StepError::new("invalid_target"))?;
                let target = self
                    .world
                    .actors
                    .get(target_index)
                    .filter(|actor| actor.is_alive())
                    .ok_or_else(|| StepError::new("invalid_target"))?;
                let target_def = spell.target.as_ref().expect("actor target has definition");
                let caster_rc = caster.location.clone();
                let target_rc = target.location.clone();
                if caster.location.level != target.location.level {
                    return Err(StepError::new("invalid_target"));
                }
                self.validate_target_visibility_and_range(
                    caster_index,
                    target_def,
                    &caster_rc,
                    &target_rc,
                )
                .map_err(Self::spell_command_error)?;
                Some(SpellTarget::Actor {
                    actor_id: target.id.clone(),
                })
            }
            (
                Some(SpellTargetKind::Coordinate) | Some(SpellTargetKind::Area),
                MonsterAbilityTargetPolicy::NearestHostile,
            ) => {
                let target_index = target_index.ok_or_else(|| StepError::new("invalid_target"))?;
                let target = self
                    .world
                    .actors
                    .get(target_index)
                    .filter(|actor| actor.is_alive())
                    .ok_or_else(|| StepError::new("invalid_target"))?;
                let target_def = spell
                    .target
                    .as_ref()
                    .expect("location target has definition");
                let caster_rc = caster.location.clone();
                let target_rc = target.location.clone();
                if caster.location.level != target.location.level {
                    return Err(StepError::new("invalid_target"));
                }
                self.validate_target_visibility_and_range(
                    caster_index,
                    target_def,
                    &caster_rc,
                    &target_rc,
                )
                .map_err(Self::spell_command_error)?;
                match target_def.kind {
                    SpellTargetKind::Area => Some(SpellTarget::Area { center: target_rc }),
                    SpellTargetKind::Coordinate => Some(SpellTarget::Coordinate {
                        position: target_rc,
                    }),
                    _ => unreachable!("location target is coordinate or area"),
                }
            }
            (Some(SpellTargetKind::None) | None, _) => None,
            _ => return Err(StepError::new("unsupported monster ability target")),
        };
        Ok(SpellCommandPlan {
            spell_id: spell.id.clone(),
            spell_name: spell.name.clone(),
            lane: spell.lane.clone().unwrap_or_default(),
            mp_cost: None,
            stamina_cost: None,
            target: resolved_target,
            casting_method: casting.method,
            cast_class: casting.cast_class,
            path_plan: None,
            thaum_above_skill: None,
            hostility_authorization: None,
            damage_credit_override: None,
        })
    }
}
