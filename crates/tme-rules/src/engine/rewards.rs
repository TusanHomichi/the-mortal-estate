use super::combat::PhysicalAttackResolution;
use super::physical_attacks::PhysicalAttackPlan;
use super::spellcasting::SpellCommandPlan;
use super::{Engine, StepError};
use crate::events::Event;
use crate::model::{
    ActorKind, CombatRisk, MagicPracticeReceipt, MagicPrimaryAttribute, PhysicalAttackOutcome,
    PhysicalPracticeReceipt, WoundState,
};

impl Engine {
    pub(super) fn award_magic_casting_practice(
        &mut self,
        actor_index: usize,
        plan: &SpellCommandPlan,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let Some(receipt) = self.magic_casting_practice_plan(actor_index, plan)? else {
            return Ok(());
        };
        let actor = &self.world.actors[actor_index];
        events.push(Event::MagicPracticeEvaluated {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            current_class_id: receipt.current_class_id.clone(),
            spell_id: receipt.spell_id.clone(),
            spell_name: receipt.spell_name.clone(),
            track_id: receipt.track_id.clone(),
            mp_cost: receipt.mp_cost,
            cast_class: receipt.cast_class,
            primary_attribute: receipt.primary_attribute,
            primary_attribute_value: receipt.primary_attribute_value,
            base_raw_points: receipt.base_raw_points,
            primary_attribute_bonus_raw_points: receipt.primary_attribute_bonus_raw_points,
            total_raw_points: receipt.total_raw_points,
            risk_applied: receipt.risk_applied,
            reason: receipt.reason,
        });
        if receipt.total_raw_points > 0 {
            events.extend(self.award_skill_practice(
                actor_index,
                &receipt.track_id,
                receipt.total_raw_points,
            )?);
        }
        Ok(())
    }

    fn magic_casting_practice_plan(
        &self,
        actor_index: usize,
        plan: &SpellCommandPlan,
    ) -> Result<Option<MagicPracticeReceipt>, StepError> {
        let Some(character) = self.world.actors[actor_index].character.as_ref() else {
            return Ok(None);
        };
        let class_id = character.identity.current_class_id.as_str();
        let mapping = match class_id {
            "wizard" => Some((
                "wizard_magic",
                MagicPrimaryAttribute::Intelligence,
                character.attributes.intelligence,
            )),
            "thief" => Some((
                "thief_magic",
                MagicPrimaryAttribute::Intelligence,
                character.attributes.intelligence,
            )),
            "thaumaturge" => Some((
                "thaumaturge_magic",
                MagicPrimaryAttribute::Wisdom,
                character.attributes.wisdom,
            )),
            _ => None,
        };
        let (track_id, primary_attribute, primary_attribute_value, reason) = match mapping {
            Some((track_id, attribute, value)) if plan.lane == track_id => (
                plan.lane.clone(),
                Some(attribute),
                Some(value),
                "eligible_successful_cast",
            ),
            Some((_, attribute, value)) => (
                plan.lane.clone(),
                Some(attribute),
                Some(value),
                "wrong_magic_lane",
            ),
            None => (plan.lane.clone(), None, None, "class_has_no_magic_practice"),
        };
        let rules = &self.definition.catalog.rules.magic.casting_practice;
        let eligible = reason == "eligible_successful_cast";
        let (base_raw_points, primary_attribute_bonus_raw_points, total_raw_points) = if eligible {
            let mp_cost = u64::try_from(plan.mp_cost.unwrap_or(0).max(0))
                .map_err(|_| StepError::new("magic practice MP cost is invalid"))?;
            let scaled = mp_cost
                .checked_mul(rules.raw_points_per_mp)
                .ok_or_else(|| StepError::new("magic practice base amount overflow"))?;
            let base = scaled.max(rules.minimum_raw_points);
            let value = u64::try_from(primary_attribute_value.unwrap_or(0).max(0))
                .map_err(|_| StepError::new("magic practice attribute is invalid"))?;
            let bonus = value / u64::from(rules.primary_attribute_points_per_bonus);
            let total = base
                .checked_add(bonus)
                .ok_or_else(|| StepError::new("magic practice total overflow"))?;
            (base, bonus, total)
        } else {
            (0, 0, 0)
        };
        Ok(Some(MagicPracticeReceipt {
            current_class_id: class_id.to_string(),
            spell_id: plan.spell_id.clone(),
            spell_name: plan.spell_name.clone(),
            track_id,
            mp_cost: plan.mp_cost.unwrap_or(0),
            cast_class: plan.cast_class,
            primary_attribute,
            primary_attribute_value,
            base_raw_points,
            primary_attribute_bonus_raw_points,
            total_raw_points,
            risk_applied: false,
            reason: reason.to_string(),
        }))
    }

    pub(super) fn physical_combat_risk(
        &self,
        plan: &PhysicalAttackPlan,
    ) -> Result<Option<CombatRisk>, StepError> {
        let Some(character) = self.world.actors[plan.attacker_index].character.as_ref() else {
            return Ok(None);
        };
        if plan.attacker_wound_before == WoundState::NearDeath {
            return Ok(Some(CombatRisk::Overwhelming));
        }
        let level = u64::try_from(character.progression.level)
            .map_err(|_| StepError::new("physical practice attacker level is invalid"))?;
        let threshold = level
            .checked_mul(
                self.definition
                    .catalog
                    .rules
                    .combat
                    .practice
                    .life_and_death_minimum_target_xp_per_attacker_level,
            )
            .ok_or_else(|| StepError::new("physical practice risk threshold overflow"))?;
        let target_xp = u64::try_from(self.world.actors[plan.defender_index].xp_value).unwrap_or(0);
        Ok(Some(if target_xp >= threshold {
            CombatRisk::LifeAndDeath
        } else {
            CombatRisk::Practice
        }))
    }

    pub(super) fn award_physical_attack_practice(
        &mut self,
        plan: &PhysicalAttackPlan,
        resolution: PhysicalAttackResolution,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let Some(receipt) = self.physical_practice_plan(plan, resolution)? else {
            return Ok(());
        };
        let attacker = &self.world.actors[plan.attacker_index];
        events.push(Event::PhysicalPracticeEvaluated {
            actor_id: attacker.id.clone(),
            actor: attacker.name.clone(),
            track_id: receipt.track_id.clone(),
            mode: receipt.mode,
            outcome: receipt.outcome,
            risk: receipt.risk,
            base_raw_points: receipt.base_raw_points,
            fatal_blow_bonus_raw_points: receipt.fatal_blow_bonus_raw_points,
            total_raw_points: receipt.total_raw_points,
        });
        if receipt.total_raw_points > 0 {
            events.extend(self.award_skill_practice(
                plan.attacker_index,
                &receipt.track_id,
                receipt.total_raw_points,
            )?);
        }
        Ok(())
    }

    fn physical_practice_plan(
        &self,
        plan: &PhysicalAttackPlan,
        resolution: PhysicalAttackResolution,
    ) -> Result<Option<PhysicalPracticeReceipt>, StepError> {
        if self.world.actors[plan.defender_index].kind == ActorKind::Player {
            return Ok(None);
        }
        let Some(risk) = self.physical_combat_risk(plan)? else {
            return Ok(None);
        };
        let rules = &self.definition.catalog.rules.combat.practice;
        let mut base_raw_points = if resolution.practice_eligible() {
            match risk {
                CombatRisk::Practice => rules.practice_raw_points,
                CombatRisk::LifeAndDeath => rules.life_and_death_raw_points,
                CombatRisk::Overwhelming => rules.overwhelming_raw_points,
            }
        } else {
            0
        };
        if risk == CombatRisk::Practice
            && u32::from(plan.skill_level) >= rules.life_and_death_required_at_skill_level
        {
            base_raw_points = 0;
        }

        let defender = &self.world.actors[plan.defender_index];
        let defender_owned_by_attacker = defender
            .summoned
            .as_ref()
            .is_some_and(|summoned| summoned.owner_id == self.world.actors[plan.attacker_index].id);
        let fatal_blow_bonus_raw_points = if resolution.outcome()
            == PhysicalAttackOutcome::FatalBlow
            && defender.kind == ActorKind::Monster
            && !defender_owned_by_attacker
        {
            rules.fatal_blow_bonus_raw_points
        } else {
            0
        };
        let total_raw_points = base_raw_points
            .checked_add(fatal_blow_bonus_raw_points)
            .ok_or_else(|| StepError::new("physical practice amount overflow"))?;
        Ok(Some(PhysicalPracticeReceipt {
            track_id: plan.skill_track_id.clone(),
            mode: plan.mode,
            outcome: resolution.outcome(),
            risk,
            base_raw_points,
            fatal_blow_bonus_raw_points,
            total_raw_points,
        }))
    }
}
