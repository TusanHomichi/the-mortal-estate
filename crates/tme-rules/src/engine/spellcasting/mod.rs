use crate::model::{
    AttackRelationPlan, CharacterId, HostilityAuthorization, SpellCastClass, SpellCastingMethod,
    SpellDamageCredit, SpellDamageRewardClass, SpellTarget, ThaumAboveSkillPlan,
};

mod effects;
mod lifecycle;
mod resistance;
mod targeting;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SpellEffectOutcome {
    Applied,
    Failed,
    Stubbed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HostileSpellReach {
    DirectedActor,
    PathEndpoint,
    AreaCenter,
    TurnUndeadVisibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HostileSpellContactPlan {
    pub spell_id: String,
    pub source_actor_id: crate::model::ActorId,
    pub source_character_id: Option<CharacterId>,
    pub target_actor_id: crate::model::ActorId,
    pub target_character_id: Option<CharacterId>,
    pub credited_source_actor_id: crate::model::ActorId,
    pub spell_damage_credit: Option<SpellDamageCredit>,
    pub authorization: Option<HostilityAuthorization>,
    pub reach: HostileSpellReach,
    pub relations: AttackRelationPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HostileSpellOutcomeReceipt {
    pub spell_id: String,
    pub source_actor_id: crate::model::ActorId,
    pub target_actor_id: crate::model::ActorId,
    pub credited_source_actor_id: crate::model::ActorId,
    pub reach: HostileSpellReach,
    pub outcome: SpellEffectOutcome,
    pub first_outcome_event_index: usize,
    pub one_past_last_outcome_event_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SpellEffectExecution {
    pub outcome: SpellEffectOutcome,
    pub hostile_spell_outcomes: Vec<HostileSpellOutcomeReceipt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DoorSecretAction {
    Open,
    Close,
    RevealSecret,
    HideSecret,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SpellCommandPlan {
    pub spell_id: String,
    pub spell_name: String,
    pub lane: String,
    pub mp_cost: Option<i32>,
    pub stamina_cost: Option<i32>,
    pub target: Option<SpellTarget>,
    pub casting_method: SpellCastingMethod,
    pub cast_class: SpellCastClass,
    pub path_plan: Option<targeting::SpellPathPlan>,
    pub thaum_above_skill: Option<ThaumAboveSkillPlan>,
    pub hostility_authorization: Option<HostilityAuthorization>,
    pub damage_credit_override: Option<SpellDamageCredit>,
}

impl SpellCommandPlan {
    pub(super) fn damage_credit(
        &self,
        caster_actor_id: &crate::model::ActorId,
    ) -> Option<SpellDamageCredit> {
        if let Some(damage_credit) = self.damage_credit_override.as_ref() {
            return Some(damage_credit.clone());
        }
        let reward_class = match self.cast_class {
            SpellCastClass::Character => SpellDamageRewardClass::Directed,
            SpellCastClass::Path => SpellDamageRewardClass::AreaOrIllusion,
            SpellCastClass::PathOrCharacter => match self.target.as_ref() {
                Some(SpellTarget::Actor { .. }) => SpellDamageRewardClass::Directed,
                Some(
                    SpellTarget::Path { .. }
                    | SpellTarget::Coordinate { .. }
                    | SpellTarget::Area { .. },
                ) => SpellDamageRewardClass::AreaOrIllusion,
                _ => return None,
            },
            SpellCastClass::SelfTarget | SpellCastClass::NotApplicable => return None,
        };
        Some(SpellDamageCredit {
            caster_actor_id: caster_actor_id.clone(),
            spell_id: self.spell_id.clone(),
            reward_class,
        })
    }
}
