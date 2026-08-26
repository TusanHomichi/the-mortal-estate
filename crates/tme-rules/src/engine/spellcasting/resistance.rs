use crate::content::{SpellEffectDef, SpellResistanceDef};
use crate::events::Event;
use crate::model::{ResistanceBoostSourceKind, SpellResistanceBoost, SpellResistanceMitigation};

use crate::engine::Engine;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorResistanceBoost {
    pub tag: String,
    pub bonus_twentieths: u32,
    pub source_kind: ResistanceBoostSourceKind,
    pub source_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SpellResistanceResolution {
    pub success: bool,
    pub resolved_damage: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SpellResistancePlan {
    pub(super) actor_index: usize,
    pub(super) actor_id: crate::model::ActorId,
    pub(super) effect_id: String,
    pub(super) resistance_tag: String,
    pub(super) natural_save_twentieths: u32,
    pub(super) selected_boost: Option<ActorResistanceBoost>,
    pub(super) denominator: u32,
    pub(super) save_twentieths: u32,
    pub(super) mitigation: SpellResistanceMitigation,
    pub(super) requested_damage: Option<i32>,
}

impl Engine {
    pub(in crate::engine) fn actor_resistance_boosts_for_index(
        &self,
        actor_index: usize,
    ) -> Vec<ActorResistanceBoost> {
        let actor = &self.world.actors[actor_index];
        let mut boosts = Vec::new();
        for effect in &actor.active_effects {
            for boost in &effect.resistance_boosts {
                boosts.push(ActorResistanceBoost {
                    tag: boost.tag.clone(),
                    bonus_twentieths: boost.bonus_twentieths,
                    source_kind: ResistanceBoostSourceKind::ActiveEffect,
                    source_id: effect.instance_id.clone(),
                });
            }
        }
        if let Ok(active_items) = self.active_item_ids(actor_index) {
            for item_instance_id in active_items {
                if let Ok(item) = self.item_definition(&item_instance_id)
                    && let Some(capability) = item.capability.as_ref()
                    && let Some(item_boosts) = capability.resistance_boosts.as_ref()
                {
                    for boost in item_boosts {
                        boosts.push(ActorResistanceBoost {
                            tag: boost.tag.clone(),
                            bonus_twentieths: boost.bonus_twentieths,
                            source_kind: ResistanceBoostSourceKind::EquippedItem,
                            source_id: item_instance_id.clone(),
                        });
                    }
                }
            }
        }
        boosts.sort_by(|left, right| {
            left.tag
                .cmp(&right.tag)
                .then_with(|| right.bonus_twentieths.cmp(&left.bonus_twentieths))
                .then_with(|| left.source_kind.cmp(&right.source_kind))
                .then_with(|| left.source_id.cmp(&right.source_id))
        });
        boosts
    }

    pub fn actor_resistance_boosts(
        &self,
        actor_id: &crate::model::ActorId,
    ) -> Result<Vec<ActorResistanceBoost>, crate::engine::StepError> {
        let actor_index = self
            .world
            .actors
            .iter()
            .position(|actor| &actor.id == actor_id)
            .ok_or_else(|| crate::engine::StepError::new(format!("unknown actor: {actor_id}")))?;
        Ok(self.actor_resistance_boosts_for_index(actor_index))
    }

    pub(super) fn resistance_boosts_from_effect(
        effect: &SpellEffectDef,
    ) -> Vec<SpellResistanceBoost> {
        match effect.resistance.as_ref() {
            Some(SpellResistanceDef::Boost { boosts }) => boosts.clone(),
            _ => Vec::new(),
        }
    }

    pub(super) fn plan_spell_resistance(
        &self,
        actor_index: usize,
        effect_id: &str,
        effect: &SpellEffectDef,
        requested_damage: Option<i32>,
    ) -> Option<SpellResistancePlan> {
        let SpellResistanceDef::Incoming { tag, mitigation } = effect.resistance.as_ref()? else {
            return None;
        };
        let rules = &self.definition.catalog.rules.magic.resistance;
        let denominator = rules.denominator;
        let natural = self.world.actors[actor_index]
            .magic_resistance
            .natural_save_twentieths;
        let selected = self
            .actor_resistance_boosts_for_index(actor_index)
            .into_iter()
            .find(|boost| boost.tag == *tag);
        let matching_bonus = selected.as_ref().map_or(0, |boost| boost.bonus_twentieths);
        let save_twentieths = natural.saturating_add(matching_bonus).min(denominator);
        Some(SpellResistancePlan {
            actor_index,
            actor_id: self.world.actors[actor_index].id.clone(),
            effect_id: effect_id.to_string(),
            resistance_tag: tag.clone(),
            natural_save_twentieths: natural,
            selected_boost: selected,
            denominator,
            save_twentieths,
            mitigation: mitigation.clone(),
            requested_damage,
        })
    }

    pub(super) fn commit_spell_resistance(
        &mut self,
        plan: SpellResistancePlan,
        events: &mut Vec<Event>,
    ) -> SpellResistanceResolution {
        let roll = self.rng.roll_d20();
        let success = roll <= plan.save_twentieths;
        let resolved_damage = plan.requested_damage.map(|requested| {
            if !success {
                return requested;
            }
            match &plan.mitigation {
                SpellResistanceMitigation::Negate => 0,
                SpellResistanceMitigation::HalfDamage { minimum_damage, .. } => {
                    requested.min((requested / 2).max(*minimum_damage))
                }
                SpellResistanceMitigation::MinimumDamage { damage } => requested.min(*damage),
            }
        });
        let actor = &self.world.actors[plan.actor_index];
        debug_assert_eq!(actor.id, plan.actor_id);
        events.push(Event::SpellSaveResolved {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            location: actor.location.clone(),
            effect_id: plan.effect_id,
            resistance_tag: plan.resistance_tag,
            natural_save_twentieths: plan.natural_save_twentieths,
            matching_bonus_twentieths: plan
                .selected_boost
                .as_ref()
                .map_or(0, |boost| boost.bonus_twentieths),
            selected_boost_source_kind: plan.selected_boost.as_ref().map(|boost| boost.source_kind),
            selected_boost_source_id: plan
                .selected_boost
                .as_ref()
                .map(|boost| boost.source_id.clone()),
            denominator: plan.denominator,
            save_twentieths: plan.save_twentieths,
            roll,
            success,
            mitigation_mode: success.then(|| plan.mitigation.mode()),
            requested_damage: plan.requested_damage,
            resolved_damage,
        });
        SpellResistanceResolution {
            success,
            resolved_damage,
        }
    }

    pub(super) fn resolve_spell_resistance(
        &mut self,
        actor_index: usize,
        effect_id: &str,
        effect: &SpellEffectDef,
        requested_damage: Option<i32>,
        events: &mut Vec<Event>,
    ) -> Option<SpellResistanceResolution> {
        let plan = self.plan_spell_resistance(actor_index, effect_id, effect, requested_damage)?;
        Some(self.commit_spell_resistance(plan, events))
    }
}
