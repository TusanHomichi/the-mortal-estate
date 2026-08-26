use serde::{Deserialize, Serialize};

use super::{CombatRulesDef, ProgressionRulesDef, SkillRulesDef};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchBoundary {
    pub status: String,
    pub notes: String,
    pub review_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulesDef {
    pub progression: ProgressionRulesDef,
    pub movement: MovementRulesDef,
    pub burden: BurdenRulesDef,
    pub resources: ResourceRulesDef,
    pub magic: MagicRulesDef,
    pub skills: SkillRulesDef,
    pub combat: CombatRulesDef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicRuleEvidenceStateDef {
    OriginalProvisional,
    TargetRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageInterruptionComparisonDef {
    StrictlyGreater,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicSaveComparisonDef {
    RollAtOrBelow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchingResistanceBoostPolicyDef {
    HighestMatching,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellWarmupRulesDef {
    pub units: u32,
    pub evidence_state: MagicRuleEvidenceStateDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellDamageInterruptionRulesDef {
    pub comparison: DamageInterruptionComparisonDef,
    pub numerator: u32,
    pub denominator: u32,
    pub evidence_state: MagicRuleEvidenceStateDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellResistanceRulesDef {
    pub denominator: u32,
    pub denominator_evidence_state: MagicRuleEvidenceStateDef,
    pub success_comparison: MagicSaveComparisonDef,
    pub matching_boost_policy: MatchingResistanceBoostPolicyDef,
    pub resolution_evidence_state: MagicRuleEvidenceStateDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicRulesDef {
    pub warmup: SpellWarmupRulesDef,
    pub damage_interruption: SpellDamageInterruptionRulesDef,
    pub resistance: SpellResistanceRulesDef,
    pub casting_practice: MagicCastingPracticeRulesDef,
    pub thaum_above_skill: ThaumAboveSkillRulesDef,
    pub kill_experience: MagicKillExperienceRulesDef,
    pub mp_recovery: MagicMpRecoveryRulesDef,
    pub effect_families: MagicEffectFamilyRulesDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicEffectFamilyRulesDef {
    pub raise_dead: RaiseDeadRulesDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RaiseDeadRulesDef {
    pub roll_denominator: u32,
    pub success_threshold_per_magic_level: u32,
    pub minimum_success_threshold: u32,
    pub evidence_state: MagicRuleEvidenceStateDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicCastingPracticeRulesDef {
    pub minimum_raw_points: u64,
    pub raw_points_per_mp: u64,
    pub primary_attribute_points_per_bonus: u32,
    pub evidence_state: MagicRuleEvidenceStateDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThaumAboveSkillRulesDef {
    pub roll_denominator: u32,
    pub penalty_per_missing_level: u32,
    pub minimum_success_threshold: u32,
    pub evidence_state: MagicRuleEvidenceStateDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicRewardFractionDef {
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicArithmeticRoundingDef {
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicKillExperienceRulesDef {
    pub directed: MagicRewardFractionDef,
    pub area_or_illusion: MagicRewardFractionDef,
    pub fraction_evidence_state: MagicRuleEvidenceStateDef,
    pub rounding: MagicArithmeticRoundingDef,
    pub rounding_evidence_state: MagicRuleEvidenceStateDef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveMpRecoveryItemPolicyDef {
    HighestMultiplier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicMpRecoveryRulesDef {
    pub active_item_policy: ActiveMpRecoveryItemPolicyDef,
    pub rounding: MagicArithmeticRoundingDef,
    pub evidence_state: MagicRuleEvidenceStateDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BurdenRulesDef {
    pub coin_burden_per_gold: u64,
    pub lightly_loaded_max_per_strength: u64,
    pub moderately_loaded_max_per_strength: u64,
    pub heavily_loaded_max_per_strength: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceRulesDef {
    pub recovery_interval_units: u32,
    pub active_hp_recovery: i32,
    pub inactive_hp_recovery: i32,
    pub inactive_stamina_recovery: i32,
    pub mp_recovery: i32,
    pub normal_movement_stamina_cost: i32,
    pub rapid_movement_stamina_cost: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovementRulesDef {
    pub controlled_path_points: i32,
    pub automatic_step_points: i32,
}
