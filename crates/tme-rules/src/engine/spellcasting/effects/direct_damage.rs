//! Directed, area and path damage, including its target selection.

use crate::events::Event;
use crate::model::{DeathCause, SpellTarget};

use super::super::{SpellCommandPlan, SpellEffectOutcome};
use crate::engine::death::DefeatContext;
use crate::engine::{Engine, StepError};

impl Engine {
    pub(super) fn apply_direct_damage_spell(
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
                            && actor.location.same_cell(position)
                    })
                }),
            Some(SpellTarget::Area { center })
            | Some(SpellTarget::Coordinate { position: center }) => {
                self.world.actors.iter().position(|actor| {
                    actor.is_alive()
                        && actor.id != self.world.actors[player_index].id
                        && actor.location.same_cell(center)
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
}
