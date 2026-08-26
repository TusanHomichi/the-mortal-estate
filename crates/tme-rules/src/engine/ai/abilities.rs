use crate::events::{AutomaticActorDecisionV1, Event};
use crate::model::{MonsterAbilityState, MonsterAbilityTargetPolicy, SpellTarget};

use super::super::spellcasting::SpellEffectOutcome;
use super::super::{Engine, StepError};

impl Engine {
    fn mark_automatic_ability_used(&mut self, actor_index: usize, ability_id: &str) {
        let ready_at = self.logical_time_after({
            self.world.actors[actor_index]
                .monster_abilities
                .iter()
                .find(|ability| ability.id == ability_id)
                .map_or(1, |ability| ability.cooldown_rounds)
        });
        if let Some(ability) = self.world.actors[actor_index]
            .monster_abilities
            .iter_mut()
            .find(|ability| ability.id == ability_id)
        {
            ability.ready_at = ready_at;
        }
    }

    fn automatic_ability_target_label(
        &self,
        actor_index: usize,
        target: Option<&SpellTarget>,
    ) -> (Option<crate::model::ActorId>, Option<String>) {
        match target {
            Some(SpellTarget::Actor { actor_id }) => {
                let name = self
                    .world
                    .actors
                    .iter()
                    .find(|actor| actor.id == *actor_id)
                    .map(|actor| actor.name.clone())
                    .unwrap_or_else(|| actor_id.to_string());
                (Some(actor_id.clone()), Some(name))
            }
            Some(SpellTarget::SelfTarget) => {
                let actor = &self.world.actors[actor_index];
                (Some(actor.id.clone()), Some(actor.name.clone()))
            }
            Some(target) => (None, Some(target.label())),
            None => (None, None),
        }
    }

    fn automatic_ability_target_index(
        &self,
        target_index: usize,
        ability: &MonsterAbilityState,
    ) -> Option<usize> {
        match ability.target_policy {
            MonsterAbilityTargetPolicy::NearestHostile => Some(target_index),
            MonsterAbilityTargetPolicy::SelfTarget => None,
        }
    }

    pub(super) fn try_automatic_ability(
        &mut self,
        actor_index: usize,
        target_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<bool, StepError> {
        let current_time = self.current_time();
        let caster_id = self.world.actors[actor_index].id.clone();
        let abilities = self.world.actors[actor_index].monster_abilities.clone();
        for ability in abilities {
            if current_time < ability.ready_at {
                continue;
            }
            let ability_target = self.automatic_ability_target_index(target_index, &ability);
            let Ok(plan) = self.monster_spell_plan(
                actor_index,
                ability_target,
                &ability.spell_id,
                ability.target_policy,
            ) else {
                continue;
            };
            let before_ability = self.clone();
            let mut ability_events = Vec::new();
            match self
                .execute_actor_spell_effect(actor_index, &plan, &mut ability_events)?
                .outcome
            {
                SpellEffectOutcome::Applied | SpellEffectOutcome::Failed => {
                    let actor_index = self
                        .world
                        .actors
                        .iter()
                        .position(|actor| actor.id == caster_id)
                        .ok_or_else(|| StepError::new("automatic spell caster disappeared"))?;
                    let (target_id, target) =
                        self.automatic_ability_target_label(actor_index, plan.target.as_ref());
                    self.emit_automatic_decision(
                        actor_index,
                        AutomaticActorDecisionV1::UseAbility {
                            ability_id: ability.id.clone(),
                            spell_id: plan.spell_id.clone(),
                            spell_name: plan.spell_name.clone(),
                            target_id,
                            target,
                        },
                        events,
                    );
                    events.extend(ability_events);
                    self.mark_automatic_ability_used(actor_index, &ability.id);
                    return Ok(true);
                }
                SpellEffectOutcome::Stubbed => {
                    *self = before_ability;
                }
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use crate::{CatalogRegistryKey, GameDefinition, SpellEffectFamily};

    #[test]
    fn invalid_automatic_spell_definition_is_rejected_before_engine_creation() {
        let (mut catalog, profile, template, _seed) =
            crate::engine::setup::test_parts("monster_spellcasting_special_attacks");
        let spell = catalog
            .spells
            .get_mut(&CatalogRegistryKey::from(
                "spell/venom_bite/magic_profession_gallery",
            ))
            .expect("source catalog poison should exist");
        let effect = spell
            .effect
            .as_mut()
            .expect("source catalog poison should have an effect");
        effect.family = SpellEffectFamily::DirectDamage;
        effect.potency = None;

        let error = GameDefinition::from_content(catalog, profile, template)
            .expect_err("invalid automatic spell must fail the immutable definition gate");

        assert!(error.messages().iter().any(|message| {
            message.contains("effect.potency must be positive for direct_damage spells")
        }));
    }
}
