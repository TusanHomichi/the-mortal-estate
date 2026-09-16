//! Portal placement and item utility effects.

use crate::content::SpellEffectDef;
use crate::events::Event;
use crate::model::{PortalTransitionState, SpellTarget};

use super::super::{SpellCommandPlan, SpellEffectOutcome};
use crate::engine::{Engine, StepError};

impl Engine {
    pub(super) fn apply_portal_spell(
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

    pub(super) fn apply_item_utility_spell(
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
}
