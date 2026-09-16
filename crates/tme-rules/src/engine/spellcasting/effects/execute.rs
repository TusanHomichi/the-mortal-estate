//! The one dispatch from a committed cast to its effect family.

use crate::content::SpellEffectDef;
use crate::events::Event;
use crate::model::{SpellEffectFamily, SpellTarget};

use super::super::{
    HostileSpellOutcomeReceipt, SpellCommandPlan, SpellEffectExecution, SpellEffectOutcome,
};
use crate::engine::{Engine, StepError};

impl Engine {
    pub(in crate::engine) fn execute_actor_spell_effect(
        &mut self,
        caster_index: usize,
        plan: &SpellCommandPlan,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectExecution, StepError> {
        let Some(effect) = self
            .definition
            .catalog
            .spells
            .get(&plan.spell_id)
            .and_then(|spell| spell.effect.clone())
        else {
            return Ok(SpellEffectExecution {
                outcome: SpellEffectOutcome::Stubbed,
                hostile_spell_outcomes: Vec::new(),
            });
        };
        let hostile_act = self.definition.catalog.spells[&plan.spell_id]
            .social
            .hostile_act;
        let contact_plans = if hostile_act {
            self.hostile_spell_contact_plans(caster_index, plan, &effect)?
        } else {
            Vec::new()
        };

        if effect.family == SpellEffectFamily::TurnUndead {
            let (outcome, hostile_spell_outcomes) =
                self.apply_turn_undead_spell(caster_index, plan, &effect, &contact_plans, events)?;
            return Ok(SpellEffectExecution {
                outcome,
                hostile_spell_outcomes,
            });
        }

        let actor_contact_effect = matches!(
            effect.family,
            SpellEffectFamily::DirectDamage
                | SpellEffectFamily::AttributeBuff
                | SpellEffectFamily::Curse
                | SpellEffectFamily::ControlStatus
                | SpellEffectFamily::FallProtection
                | SpellEffectFamily::Protection
                | SpellEffectFamily::Resistance
                | SpellEffectFamily::Poison
                | SpellEffectFamily::Speed
                | SpellEffectFamily::Vision
                | SpellEffectFamily::WaterBreathing
                | SpellEffectFamily::Banish
                | SpellEffectFamily::InstantDeath
        );
        if hostile_act
            && actor_contact_effect
            && matches!(plan.target.as_ref(), Some(SpellTarget::SelfTarget))
        {
            let outcome = self.execute_single_spell_family(caster_index, plan, &effect, events)?;
            return Ok(SpellEffectExecution {
                outcome,
                hostile_spell_outcomes: Vec::new(),
            });
        }
        if hostile_act && actor_contact_effect {
            let mut hostile_spell_outcomes = Vec::new();
            let mut any_applied = false;
            let mut any_stubbed = false;
            for contact in contact_plans {
                self.commit_hostile_spell_contact(&contact, events)?;
                let first_outcome_event_index = events.len();
                let mut target_plan = plan.clone();
                target_plan.target = Some(SpellTarget::Actor {
                    actor_id: contact.target_actor_id.clone(),
                });
                target_plan.damage_credit_override = contact.spell_damage_credit.clone();
                let outcome =
                    self.execute_single_spell_family(caster_index, &target_plan, &effect, events)?;
                any_applied |= outcome == SpellEffectOutcome::Applied;
                any_stubbed |= outcome == SpellEffectOutcome::Stubbed;
                hostile_spell_outcomes.push(HostileSpellOutcomeReceipt {
                    spell_id: contact.spell_id,
                    source_actor_id: contact.source_actor_id,
                    target_actor_id: contact.target_actor_id,
                    credited_source_actor_id: contact.credited_source_actor_id,
                    reach: contact.reach,
                    outcome,
                    first_outcome_event_index,
                    one_past_last_outcome_event_index: events.len(),
                });
            }
            let outcome = if any_applied || hostile_spell_outcomes.is_empty() {
                SpellEffectOutcome::Applied
            } else if any_stubbed {
                SpellEffectOutcome::Stubbed
            } else {
                SpellEffectOutcome::Failed
            };
            return Ok(SpellEffectExecution {
                outcome,
                hostile_spell_outcomes,
            });
        }

        for contact in &contact_plans {
            self.commit_hostile_spell_contact(contact, events)?;
        }
        let first_outcome_event_index = events.len();
        let outcome = self.execute_single_spell_family(caster_index, plan, &effect, events)?;

        let one_past_last_outcome_event_index = events.len();
        Ok(SpellEffectExecution {
            outcome,
            hostile_spell_outcomes: contact_plans
                .into_iter()
                .map(|contact| HostileSpellOutcomeReceipt {
                    spell_id: contact.spell_id,
                    source_actor_id: contact.source_actor_id,
                    target_actor_id: contact.target_actor_id,
                    credited_source_actor_id: contact.credited_source_actor_id,
                    reach: contact.reach,
                    outcome,
                    first_outcome_event_index,
                    one_past_last_outcome_event_index,
                })
                .collect(),
        })
    }

    fn execute_single_spell_family(
        &mut self,
        caster_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        match effect.family {
            SpellEffectFamily::DirectDamage => {
                self.apply_direct_damage_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::Healing => {
                self.apply_healing_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::AttributeBuff
            | SpellEffectFamily::Curse
            | SpellEffectFamily::ControlStatus
            | SpellEffectFamily::FallProtection
            | SpellEffectFamily::Protection
            | SpellEffectFamily::Resistance
            | SpellEffectFamily::Poison
            | SpellEffectFamily::Speed
            | SpellEffectFamily::Vision
            | SpellEffectFamily::WaterBreathing => {
                self.apply_active_effect_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::PoisonCure => {
                self.apply_poison_cure_spell(caster_index, plan, events)
            }
            SpellEffectFamily::TerrainOverlay
            | SpellEffectFamily::Light
            | SpellEffectFamily::Darkness => {
                self.apply_tile_overlay_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::Summon => {
                self.apply_summon_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::DoorControl | SpellEffectFamily::SecretDetection => {
                self.apply_door_secret_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::ItemIdentify
            | SpellEffectFamily::ItemEnchant
            | SpellEffectFamily::WeaponEnchant => {
                self.apply_item_utility_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::Locate => {
                self.apply_locate_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::Scry => self.apply_scry_spell(caster_index, plan, effect, events),
            SpellEffectFamily::Portal => {
                self.apply_portal_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::Banish => {
                self.apply_banish_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::InstantDeath => {
                self.apply_instant_death_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::RaiseDead => {
                self.apply_raise_dead_spell(caster_index, plan, effect, events)
            }
            SpellEffectFamily::TurnUndead => unreachable!("handled above"),
            SpellEffectFamily::Concealment => {
                self.apply_concealment_spell(caster_index, plan, effect, events)
            }
        }
    }
}
