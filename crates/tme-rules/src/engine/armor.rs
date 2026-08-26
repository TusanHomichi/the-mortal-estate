use crate::model::{
    ActorLifeState, ArmorProtectionPlan, ArmorProtectionSource, CarriedPosition, WoundState,
};

use super::{Engine, StepError};

impl Engine {
    pub(super) fn armor_protection_plan(
        &self,
        actor_index: usize,
    ) -> Result<ArmorProtectionPlan, StepError> {
        let mut plan = ArmorProtectionPlan::default();
        for position in CarriedPosition::all()
            .iter()
            .copied()
            .filter(|position| position.is_worn())
        {
            let Some(item_instance_id) = self.item_at_position(actor_index, position)? else {
                continue;
            };
            let item_instance_id = item_instance_id.to_string();
            let item_definition_id = self.item_instance(&item_instance_id)?.definition_id.clone();
            let Some(armor) = self.item_definition(&item_instance_id)?.armor.as_ref() else {
                continue;
            };
            let source = ArmorProtectionSource {
                carried_position: position,
                item_instance_id,
                item_definition_id,
                block_rating: armor.block_rating,
                encumbrance: armor.encumbrance,
                cutting_reduction: armor.damage_reduction.cutting,
                piercing_reduction: armor.damage_reduction.piercing,
                crushing_reduction: armor.damage_reduction.crushing,
            };
            plan.block_rating = plan
                .block_rating
                .checked_add(source.block_rating)
                .ok_or_else(|| StepError::new("armor block rating overflow"))?;
            plan.encumbrance = plan
                .encumbrance
                .checked_add(source.encumbrance)
                .ok_or_else(|| StepError::new("armor encumbrance overflow"))?;
            plan.cutting_reduction = plan
                .cutting_reduction
                .checked_add(source.cutting_reduction)
                .ok_or_else(|| StepError::new("cutting armor reduction overflow"))?;
            plan.piercing_reduction = plan
                .piercing_reduction
                .checked_add(source.piercing_reduction)
                .ok_or_else(|| StepError::new("piercing armor reduction overflow"))?;
            plan.crushing_reduction = plan
                .crushing_reduction
                .checked_add(source.crushing_reduction)
                .ok_or_else(|| StepError::new("crushing armor reduction overflow"))?;
            plan.sources.push(source);
        }
        Ok(plan)
    }

    pub(super) fn wound_state(&self, actor_index: usize) -> WoundState {
        let actor = &self.world.actors[actor_index];
        if actor.life_state != ActorLifeState::Alive || actor.hp <= 0 {
            return WoundState::Dead;
        }
        let max_hp = i64::from(actor.max_hp().max(1));
        let remaining_percent = i64::from(actor.hp.max(0)).saturating_mul(100) / max_hp;
        let wounds = &self.definition.catalog.rules.combat.wounds;
        if remaining_percent <= i64::from(wounds.near_death_max_percent) {
            WoundState::NearDeath
        } else if remaining_percent <= i64::from(wounds.badly_wounded_max_percent) {
            WoundState::BadlyWounded
        } else if remaining_percent <= i64::from(wounds.wounded_max_percent) {
            WoundState::Wounded
        } else {
            WoundState::Unhurt
        }
    }
}
