use serde::{Deserialize, Serialize};

use crate::model::{ActiveEffectState, TileEffectState, WorldPosition};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActiveEffectSourceViewV1 {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActiveEffectViewV1 {
    pub instance_id: String,
    pub effect_id: String,
    pub source: ActiveEffectSourceViewV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spell_damage_credit: Option<crate::model::SpellDamageCredit>,
    pub kind: String,
    pub tags: Vec<String>,
    pub potency: i32,
    pub remaining_rounds: Option<u32>,
    pub until_condition: Option<String>,
    pub stacking: String,
    pub start_delay_rounds: u32,
    pub tick_interval_rounds: u32,
    pub suppresses_action: bool,
    pub resistance_boosts: Vec<crate::model::SpellResistanceBoost>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MagicResistanceBoostViewV1 {
    pub tag: String,
    pub bonus_twentieths: u32,
    pub source_kind: crate::model::ResistanceBoostSourceKind,
    pub source_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MagicResistanceViewV1 {
    pub natural_save_twentieths: u32,
    pub evidence_state: crate::model::MagicRuleEvidenceState,
    pub boosts: Vec<MagicResistanceBoostViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TileEffectViewV1 {
    pub instance_id: String,
    pub effect_id: String,
    pub source: ActiveEffectSourceViewV1,
    pub location: WorldPosition,
    pub kind: String,
    pub tags: Vec<String>,
    pub potency: i32,
    pub remaining_rounds: Option<u32>,
    pub passability: Option<String>,
    pub sight: Option<String>,
    pub hazard: Option<String>,
    pub move_cost: Option<i32>,
}

impl From<&ActiveEffectState> for ActiveEffectViewV1 {
    fn from(effect: &ActiveEffectState) -> Self {
        Self {
            instance_id: effect.instance_id.clone(),
            effect_id: effect.effect_id.clone(),
            source: ActiveEffectSourceViewV1 {
                kind: effect.source.kind.clone(),
                id: effect.source.id.clone(),
            },
            spell_damage_credit: effect.spell_damage_credit.clone(),
            kind: effect.kind.clone(),
            tags: effect.tags.clone(),
            potency: effect.potency,
            remaining_rounds: effect.remaining_rounds,
            until_condition: effect.until_condition.clone(),
            stacking: effect.stacking.label().to_string(),
            start_delay_rounds: effect.start_delay_rounds,
            tick_interval_rounds: effect.tick_interval_rounds,
            suppresses_action: effect.suppresses_action,
            resistance_boosts: effect.resistance_boosts.clone(),
        }
    }
}

impl From<&TileEffectState> for TileEffectViewV1 {
    fn from(effect: &TileEffectState) -> Self {
        Self {
            instance_id: effect.instance_id.clone(),
            effect_id: effect.effect_id.clone(),
            source: ActiveEffectSourceViewV1 {
                kind: effect.source.kind.clone(),
                id: effect.source.id.clone(),
            },
            location: effect.location.clone(),
            kind: effect.kind.clone(),
            tags: effect.tags.clone(),
            potency: effect.potency,
            remaining_rounds: effect.remaining_rounds,
            passability: effect.passability.clone(),
            sight: effect.sight.clone(),
            hazard: effect.hazard.clone(),
            move_cost: effect.move_cost,
        }
    }
}
