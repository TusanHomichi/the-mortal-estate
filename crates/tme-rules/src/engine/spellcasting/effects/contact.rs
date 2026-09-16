//! Who a hostile cast reaches, and the authorization it needs.

use crate::content::SpellEffectDef;
use crate::events::Event;
use crate::model::{ActorKind, SocialContactKind, SpellEffectFamily, SpellTarget};

use super::super::{HostileSpellContactPlan, HostileSpellReach, SpellCommandPlan};
use crate::engine::{Engine, StepError};

impl Engine {
    pub(in crate::engine) fn spell_specific_hostile_target_allowed(
        &self,
        target_index: usize,
        effect: &SpellEffectDef,
    ) -> bool {
        let Some(target) = self.world.actors.get(target_index) else {
            return false;
        };
        effect.family == SpellEffectFamily::Banish
            && target.kind != ActorKind::Player
            && effect.banish.as_ref().is_some_and(|banish| {
                banish
                    .eligible_traits
                    .iter()
                    .any(|candidate| target.creature_traits.contains(candidate))
            })
    }

    fn hostile_spell_contact_target_indices(
        &self,
        caster_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
    ) -> Vec<(usize, HostileSpellReach)> {
        if effect.family == SpellEffectFamily::TurnUndead {
            let Some(definition) = effect.turn_undead.as_ref() else {
                return Vec::new();
            };
            let mut targets = self
                .world
                .actors
                .iter()
                .enumerate()
                .filter(|(index, actor)| {
                    *index != caster_index
                        && actor.is_alive()
                        && actor.creature_traits.contains(&definition.eligible_trait)
                        && self.actor_can_see(caster_index, &actor.location.clone())
                })
                .map(|(index, _)| (index, HostileSpellReach::TurnUndeadVisibility))
                .collect::<Vec<_>>();
            targets.sort_by(|(left, _), (right, _)| {
                self.world.actors[*left]
                    .id
                    .cmp(&self.world.actors[*right].id)
            });
            return targets;
        }

        let mut targets = match plan.target.as_ref() {
            Some(SpellTarget::Actor { actor_id }) => self
                .world
                .actors
                .iter()
                .position(|actor| actor.is_alive() && actor.id == *actor_id)
                .map(|index| vec![(index, HostileSpellReach::DirectedActor)])
                .unwrap_or_default(),
            Some(SpellTarget::Path { .. }) => plan
                .path_plan
                .as_ref()
                .and_then(|path| path.final_position.as_ref())
                .map(|position| {
                    self.world
                        .actors
                        .iter()
                        .enumerate()
                        .filter(|(_, actor)| {
                            actor.is_alive()
                                && actor.id != self.world.actors[caster_index].id
                                && actor.location.same_cell(position)
                        })
                        .map(|(index, _)| (index, HostileSpellReach::PathEndpoint))
                        .collect()
                })
                .unwrap_or_default(),
            Some(SpellTarget::Area { center })
            | Some(SpellTarget::Coordinate { position: center }) => self
                .world
                .actors
                .iter()
                .enumerate()
                .filter(|(_, actor)| {
                    actor.is_alive()
                        && actor.id != self.world.actors[caster_index].id
                        && actor.location.same_cell(center)
                })
                .map(|(index, _)| (index, HostileSpellReach::AreaCenter))
                .collect(),
            _ => Vec::new(),
        };
        targets.sort_by(|(left, _), (right, _)| {
            self.world.actors[*left]
                .id
                .cmp(&self.world.actors[*right].id)
        });
        targets
    }

    pub(super) fn hostile_spell_contact_plans(
        &self,
        caster_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
    ) -> Result<Vec<HostileSpellContactPlan>, StepError> {
        let source = &self.world.actors[caster_index];
        let mut contacts = Vec::new();
        for (target_index, reach) in
            self.hostile_spell_contact_target_indices(caster_index, plan, effect)
        {
            if let Some(authorization) = plan.hostility_authorization
                && !self.spell_specific_hostile_target_allowed(target_index, effect)
            {
                let assessment = self.attack_safety_assessment(caster_index, target_index)?;
                if !assessment.safety.permits(authorization) {
                    continue;
                }
            }
            let target = &self.world.actors[target_index];
            contacts.push(HostileSpellContactPlan {
                spell_id: plan.spell_id.clone(),
                source_actor_id: source.id.clone(),
                source_character_id: source.character_id.clone(),
                target_actor_id: target.id.clone(),
                target_character_id: target.character_id.clone(),
                credited_source_actor_id: source.id.clone(),
                spell_damage_credit: plan.damage_credit(&source.id),
                authorization: plan.hostility_authorization,
                reach,
                relations: self.plan_attack_relations(
                    caster_index,
                    target_index,
                    SocialContactKind::HostileSpellContact,
                )?,
            });
        }
        Ok(contacts)
    }

    pub(super) fn commit_hostile_spell_contact(
        &mut self,
        plan: &HostileSpellContactPlan,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let source_index = self
            .world
            .actors
            .iter()
            .position(|actor| actor.id == plan.source_actor_id)
            .ok_or_else(|| StepError::new("hostile spell source changed before contact"))?;
        let target_index = self
            .world
            .actors
            .iter()
            .position(|actor| actor.id == plan.target_actor_id)
            .ok_or_else(|| StepError::new("hostile spell target changed before contact"))?;
        let source = &self.world.actors[source_index];
        let target = &self.world.actors[target_index];
        if source.character_id != plan.source_character_id
            || target.character_id != plan.target_character_id
            || plan.credited_source_actor_id != source.id
            || plan.spell_damage_credit.as_ref().is_some_and(|credit| {
                credit.caster_actor_id != source.id || credit.spell_id != plan.spell_id
            })
        {
            return Err(StepError::new(
                "hostile spell identities changed before contact",
            ));
        }
        if !self
            .definition
            .catalog
            .spells
            .get(&plan.spell_id)
            .is_some_and(|spell| spell.social.hostile_act)
        {
            return Err(StepError::new(
                "hostile spell classification changed before contact",
            ));
        }
        if let Some(authorization) = plan.authorization {
            let spell_specific_target = self
                .definition
                .catalog
                .spells
                .get(&plan.spell_id)
                .and_then(|spell| spell.effect.as_ref())
                .is_some_and(|effect| {
                    self.spell_specific_hostile_target_allowed(target_index, effect)
                });
            if !spell_specific_target
                && !self
                    .attack_safety_assessment(source_index, target_index)?
                    .safety
                    .permits(authorization)
            {
                return Err(StepError::new(
                    "hostile spell authorization changed before contact",
                ));
            }
        }
        self.commit_attack_relations(&plan.relations, events)
    }
}
