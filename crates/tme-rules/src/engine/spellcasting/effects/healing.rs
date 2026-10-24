//! Restoring a target's pool without a condition.

use crate::events::Event;
use crate::model::SpellTarget;

use super::super::{SpellCommandPlan, SpellEffectOutcome};
use crate::engine::{Engine, StepError};

impl Engine {
    pub(super) fn apply_healing_spell(
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
}
