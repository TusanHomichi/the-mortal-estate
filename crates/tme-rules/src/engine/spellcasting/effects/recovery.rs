//! Effects that clear a condition instead of applying one.

use crate::events::Event;

use super::super::{SpellCommandPlan, SpellEffectOutcome};
use crate::engine::{Engine, StepError};

impl Engine {
    pub(super) fn apply_poison_cure_spell(
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
}
