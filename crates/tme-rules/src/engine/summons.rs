use crate::content::SpellEffectDef;
use crate::events::Event;
use crate::model::{
    ActorState, CarriedLayout, ItemHolderId, ItemInstanceState, ItemLocation, SpellTarget,
    SummonTemplate, SummonedActorState, WorldPosition,
};
use crate::view::ActionBlockedReasonV1;

use super::setup::{ActorInstanceState, actor_state_from_definition};
use super::spellcasting::{SpellCommandPlan, SpellEffectOutcome};
use super::{Engine, StepError};

struct SummonActorBuild {
    actor: ActorState,
    item_instances: std::collections::BTreeMap<String, ItemInstanceState>,
    placements: Vec<(String, ItemLocation)>,
}

impl Engine {
    pub(super) fn apply_summon_spell(
        &mut self,
        player_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(template_id) = effect.summon_actor_id.as_deref() else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        let Some(SpellTarget::Coordinate { position }) = plan.target.as_ref() else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        self.validate_summon_target(plan.target.as_ref())
            .map_err(|_| StepError::new("invalid_target"))?;

        let template = self
            .definition
            .catalog
            .summon_templates
            .get(template_id)
            .cloned()
            .ok_or_else(|| {
                StepError::new(format!("summon template {template_id:?} was not found"))
            })?;
        let actor_definition = self
            .definition
            .catalog
            .actor_definitions
            .get(&template.actor_definition_id)
            .cloned()
            .ok_or_else(|| StepError::new("summon actor definition was not found"))?;
        self.validate_summon_burden(effect)?;

        let next_summon_sequence = self.world.next_summon_sequence.saturating_add(1);
        let actor_id = crate::model::ActorId::new(format!(
            "summon:{}:{}:{}",
            plan.spell_id, next_summon_sequence, template.id
        ));
        let remaining_rounds = effect
            .duration
            .as_ref()
            .and_then(|duration| duration.rounds)
            .and_then(|rounds| u32::try_from(rounds).ok());
        let caster = self.world.actors[player_index].clone();
        let summoned = SummonedActorState {
            instance_id: actor_id.clone(),
            owner_id: caster.id.clone(),
            source_spell_id: plan.spell_id.clone(),
            template_id: template.id.clone(),
            remaining_rounds,
            last_ticked_at: self.current_time(),
        };
        let build = self.actor_from_summon_template(
            &template,
            actor_id.clone(),
            position.clone(),
            summoned,
        )?;
        self.world.actors.push(build.actor);
        if let Err(error) = self.register_item_instances(build.item_instances, &build.placements) {
            self.world
                .actors
                .pop()
                .expect("just-added summoned actor should be present for rollback");
            return Err(error);
        }
        self.world.next_summon_sequence = next_summon_sequence;
        events.push(Event::ActorSummoned {
            caster_id: caster.id.clone(),
            caster: caster.name,
            spell_id: plan.spell_id.clone(),
            spell_name: plan.spell_name.clone(),
            actor_id,
            actor: actor_definition.name,
            template_id: template.id,
            owner_id: caster.id,
            social: actor_definition.social,
            location: position.clone(),
            remaining_rounds,
        });
        Ok(SpellEffectOutcome::Applied)
    }

    pub(super) fn apply_summon_ticks(&mut self, events: &mut Vec<Event>) -> Result<(), StepError> {
        let now = self.current_time();
        let mut actor_index = 0;
        while actor_index < self.world.actors.len() {
            let Some(summoned) = self.world.actors[actor_index].summoned.as_mut() else {
                actor_index += 1;
                continue;
            };
            if now.elapsed_rounds_since(summoned.last_ticked_at) >= 1 {
                summoned.last_ticked_at = now;
                if let Some(remaining) = summoned.remaining_rounds.as_mut() {
                    *remaining = remaining.saturating_sub(1);
                }
            }
            if summoned.remaining_rounds == Some(0) {
                let actor = self.remove_summoned_actor_at(actor_index)?;
                let summoned = actor
                    .summoned
                    .clone()
                    .expect("summoned actor state should exist during summon cleanup");
                events.push(Event::SummonExpired {
                    actor_id: actor.id,
                    actor: actor.name,
                    instance_id: summoned.instance_id,
                    owner_id: summoned.owner_id,
                    source_spell_id: summoned.source_spell_id,
                    template_id: summoned.template_id,
                    location: actor.location,
                });
            } else {
                actor_index += 1;
            }
        }
        Ok(())
    }

    pub(super) fn remove_summoned_actor_at(
        &mut self,
        actor_index: usize,
    ) -> Result<ActorState, StepError> {
        if self
            .world
            .actors
            .get(actor_index)
            .is_none_or(|actor| actor.summoned.is_none())
        {
            return Err(StepError::new("actor is not summoned"));
        }
        let owned_instance_ids = self.ordered_actor_item_ids(actor_index)?;
        self.destroy_item_instances(&owned_instance_ids)?;
        let actor = self.world.actors.remove(actor_index);
        self.world.defeat_contributions.remove(&actor.id);
        Ok(actor)
    }

    pub(super) fn validate_summon_target(
        &self,
        target: Option<&SpellTarget>,
    ) -> Result<(), ActionBlockedReasonV1> {
        let Some(SpellTarget::Coordinate { position }) = target else {
            return Err(ActionBlockedReasonV1::InvalidTarget);
        };
        if !self.in_bounds(position) {
            return Err(ActionBlockedReasonV1::InvalidTarget);
        }
        if !self
            .effective_tile_at(position)
            .is_some_and(|tile| tile.passable)
        {
            return Err(ActionBlockedReasonV1::InvalidTarget);
        }
        if self
            .world
            .actors
            .iter()
            .any(|actor| actor.is_alive() && actor.location == *position)
        {
            return Err(ActionBlockedReasonV1::InvalidTarget);
        }
        Ok(())
    }

    pub(super) fn validate_summon_burden(&self, effect: &SpellEffectDef) -> Result<(), StepError> {
        let template_id = effect
            .summon_actor_id
            .as_deref()
            .ok_or_else(|| StepError::new("invalid_target"))?;
        let template = self
            .definition
            .catalog
            .summon_templates
            .get(template_id)
            .ok_or_else(|| StepError::new("invalid_target"))?;
        self.validate_prospective_item_instances_burden(&template.item_instances)
    }

    fn actor_from_summon_template(
        &mut self,
        template: &SummonTemplate,
        actor_id: crate::model::ActorId,
        location: WorldPosition,
        summoned: SummonedActorState,
    ) -> Result<SummonActorBuild, StepError> {
        self.validate_prospective_item_instances_burden(&template.item_instances)?;
        let definition = self
            .definition
            .catalog
            .actor_definitions
            .get(&template.actor_definition_id)
            .cloned()
            .ok_or_else(|| StepError::new("summon actor definition was not found"))?;
        let expanded_ids = template
            .item_instances
            .keys()
            .map(|local_id| (local_id.clone(), format!("{actor_id}:item:{local_id}")))
            .collect::<std::collections::BTreeMap<_, _>>();
        for expanded_id in expanded_ids.values() {
            if self.world.item_instances.contains_key(expanded_id) {
                return Err(StepError::new(format!(
                    "summoned item instance {expanded_id:?} already exists"
                )));
            }
        }
        let item_instances = template
            .item_instances
            .iter()
            .map(|(local_id, instance)| (expanded_ids[local_id].clone(), instance.clone()))
            .collect::<std::collections::BTreeMap<_, _>>();
        let holder = ItemHolderId::TransientActor(actor_id.clone());
        let placements = template
            .carried
            .items
            .iter()
            .map(|(position, local_id)| {
                (
                    expanded_ids[local_id].clone(),
                    ItemLocation::Carried {
                        holder: holder.clone(),
                        position: *position,
                    },
                )
            })
            .collect::<Vec<_>>();
        let timing = self.allocate_actor_timing(self.current_time());
        let actor = actor_state_from_definition(
            &definition,
            ActorInstanceState {
                id: actor_id,
                location: location.clone(),
                hp: definition.stats.hp.max(1),
                mp: 0,
                stamina: 10,
                timing,
                attack_ready_at: self.logical_time_after(1),
                carried: CarriedLayout {
                    items: std::collections::BTreeMap::new(),
                    gold: template.carried.gold,
                },
                npc: None,
                character_id: None,
                character: None,
                active_effects: template.active_effects.clone(),
                summoned: Some(summoned),
                ecology_origin: None,
            },
        );
        Ok(SummonActorBuild {
            actor,
            item_instances,
            placements,
        })
    }
}
