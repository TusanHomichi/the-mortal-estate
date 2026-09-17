//! Shared helpers every effect family reaches for.

use crate::content::{SpellDurationDef, SpellEffectDef};
use crate::events::Event;
use crate::model::{
    ActiveEffectStackingPolicy, ActiveEffectState, SpellDurationPolicy, SpellTarget,
};

use super::super::SpellCommandPlan;
use crate::engine::Engine;

impl Engine {
    pub(super) fn resolve_spell_effect_target_index(
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

    pub(super) fn spell_effect_tags(&self, effect: &SpellEffectDef) -> Vec<String> {
        let mut tags = Vec::new();
        if let Some(status_kind) = effect.status_kind.as_ref() {
            tags.push(status_kind.clone());
        }
        tags
    }

    pub(super) fn spell_effect_remaining_rounds(
        &self,
        duration: Option<&SpellDurationDef>,
    ) -> Option<u32> {
        let duration = duration?;
        if duration.policy != SpellDurationPolicy::Rounds {
            return None;
        }
        duration
            .rounds
            .and_then(|rounds| u32::try_from(rounds).ok())
    }

    pub(super) fn spell_effect_start_delay_rounds(&self, effect: &SpellEffectDef) -> u32 {
        effect
            .start_delay_rounds
            .and_then(|rounds| u32::try_from(rounds).ok())
            .unwrap_or(0)
    }

    pub(super) fn spell_effect_tick_interval_rounds(
        &self,
        duration: Option<&SpellDurationDef>,
    ) -> u32 {
        duration
            .and_then(|duration| duration.tick_interval_rounds)
            .and_then(|rounds| u32::try_from(rounds).ok())
            .unwrap_or(1)
    }

    pub(super) fn spell_effect_suppresses_action(&self, effect: &SpellEffectDef) -> bool {
        effect.suppresses_action.unwrap_or(matches!(
            effect.status_kind.as_deref(),
            Some("stun" | "fear")
        ))
    }

    pub(super) fn spell_effect_stacking(
        &self,
        effect: &SpellEffectDef,
    ) -> ActiveEffectStackingPolicy {
        match effect.stacking.as_deref() {
            Some("stack_instance") => ActiveEffectStackingPolicy::StackInstance,
            Some("refresh_duration") => ActiveEffectStackingPolicy::RefreshDuration,
            _ => ActiveEffectStackingPolicy::ReplaceSameKind,
        }
    }

    pub(super) fn apply_spell_active_effect_state(
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
