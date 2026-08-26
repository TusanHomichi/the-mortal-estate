use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MovementRulesViewV1 {
    pub controlled_path_points: i32,
    pub automatic_step_points: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BurdenRulesViewV1 {
    pub coin_burden_per_gold: u64,
    pub lightly_loaded_max_per_strength: u64,
    pub moderately_loaded_max_per_strength: u64,
    pub heavily_loaded_max_per_strength: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ResourceRulesViewV1 {
    pub recovery_interval_units: u32,
    pub active_hp_recovery: i32,
    pub inactive_hp_recovery: i32,
    pub inactive_stamina_recovery: i32,
    pub mp_recovery: i32,
    pub normal_movement_stamina_cost: i32,
    pub rapid_movement_stamina_cost: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicRuleEvidenceStateViewV1 {
    OriginalProvisional,
    TargetRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageInterruptionComparisonViewV1 {
    StrictlyGreater,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SpellWarmupRulesViewV1 {
    pub units: u32,
    pub evidence_state: MagicRuleEvidenceStateViewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SpellDamageInterruptionRulesViewV1 {
    pub comparison: DamageInterruptionComparisonViewV1,
    pub numerator: u32,
    pub denominator: u32,
    pub evidence_state: MagicRuleEvidenceStateViewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SpellResistanceRulesViewV1 {
    pub denominator: u32,
    pub denominator_evidence_state: MagicRuleEvidenceStateViewV1,
    pub success_comparison: crate::model::MagicSaveComparison,
    pub matching_boost_policy: crate::model::MatchingResistanceBoostPolicy,
    pub resolution_evidence_state: MagicRuleEvidenceStateViewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MagicRulesViewV1 {
    pub warmup: SpellWarmupRulesViewV1,
    pub damage_interruption: SpellDamageInterruptionRulesViewV1,
    pub resistance: SpellResistanceRulesViewV1,
    pub casting_practice: MagicCastingPracticeRulesViewV1,
    pub thaum_above_skill: ThaumAboveSkillRulesViewV1,
    pub kill_experience: MagicKillExperienceRulesViewV1,
    pub mp_recovery: MagicMpRecoveryRulesViewV1,
    pub effect_families: MagicEffectFamilyRulesViewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MagicEffectFamilyRulesViewV1 {
    pub raise_dead: RaiseDeadRulesViewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RaiseDeadRulesViewV1 {
    pub roll_denominator: u32,
    pub success_threshold_per_magic_level: u32,
    pub minimum_success_threshold: u32,
    pub evidence_state: MagicRuleEvidenceStateViewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MagicCastingPracticeRulesViewV1 {
    pub minimum_raw_points: u64,
    pub raw_points_per_mp: u64,
    pub primary_attribute_points_per_bonus: u32,
    pub evidence_state: MagicRuleEvidenceStateViewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ThaumAboveSkillRulesViewV1 {
    pub roll_denominator: u32,
    pub penalty_per_missing_level: u32,
    pub minimum_success_threshold: u32,
    pub evidence_state: MagicRuleEvidenceStateViewV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicArithmeticRoundingViewV1 {
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MagicRewardFractionViewV1 {
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MagicKillExperienceRulesViewV1 {
    pub directed: MagicRewardFractionViewV1,
    pub area_or_illusion: MagicRewardFractionViewV1,
    pub fraction_evidence_state: MagicRuleEvidenceStateViewV1,
    pub rounding: MagicArithmeticRoundingViewV1,
    pub rounding_evidence_state: MagicRuleEvidenceStateViewV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveMpRecoveryItemPolicyViewV1 {
    HighestMultiplier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MagicMpRecoveryRulesViewV1 {
    pub active_item_policy: ActiveMpRecoveryItemPolicyViewV1,
    pub rounding: MagicArithmeticRoundingViewV1,
    pub evidence_state: MagicRuleEvidenceStateViewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TrainingRulesViewV1 {
    pub gold_per_learning_rate: i64,
    pub experience_per_learning_rate: i32,
    pub maximum_learning_rates: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SkillRulesViewV1 {
    pub base_learning_rate: u64,
    pub practice_thresholds: Vec<u64>,
    pub training: TrainingRulesViewV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatTuningStatusViewV1 {
    OriginalProvisional,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CombatKickRulesViewV1 {
    pub maximum_range: i32,
    pub cooldown_units: u32,
    pub damage_kind: crate::model::PhysicalDamageKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CombatJumpkickRulesViewV1 {
    pub maximum_range_cap: i32,
    pub skill_levels_per_extra_hex: u32,
    pub stamina_cost: i32,
    pub cooldown_units: u32,
    pub damage_kind: crate::model::PhysicalDamageKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CombatAttackModeRulesViewV1 {
    pub kick: CombatKickRulesViewV1,
    pub jumpkick: CombatJumpkickRulesViewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CombatHitRulesViewV1 {
    pub base_defender_score: i32,
    pub attacker_attack_stat_divisor: i32,
    pub attacker_skill_level_divisor: i32,
    pub defender_defense_stat_divisor: i32,
    pub defender_dexterity_divisor: i32,
    pub non_character_defender_dexterity: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CombatBlockRulesViewV1 {
    pub left_hand_selection_percent: u32,
    pub shield_percent_per_point: u32,
    pub shield_percent_cap: u32,
    pub armor_percent_per_point: u32,
    pub armor_percent_cap: u32,
    pub strength_penetration_percent_per_add: u32,
    pub armor_encumbrance_percent_per_point: u32,
    pub combat_add_penetration_percent_per_rating: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CombatFumbleRulesViewV1 {
    pub base_percent: u32,
    pub minimum_percent: u32,
    pub skill_levels_per_reduction: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CombatDamageRulesViewV1 {
    pub minimum_damage: i32,
    pub roll_variation_modulus: u32,
    pub moderate_label_min_percent: u32,
    pub heavy_label_min_percent: u32,
    pub severe_label_min_percent: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CombatWoundRulesViewV1 {
    pub near_death_max_percent: u32,
    pub badly_wounded_max_percent: u32,
    pub wounded_max_percent: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CombatPracticeRulesViewV1 {
    pub practice_raw_points: u64,
    pub life_and_death_raw_points: u64,
    pub overwhelming_raw_points: u64,
    pub fatal_blow_bonus_raw_points: u64,
    pub life_and_death_minimum_target_xp_per_attacker_level: u64,
    pub life_and_death_required_at_skill_level: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CombatRulesViewV1 {
    pub tuning_status: CombatTuningStatusViewV1,
    pub attack_modes: CombatAttackModeRulesViewV1,
    pub hit: CombatHitRulesViewV1,
    pub block: CombatBlockRulesViewV1,
    pub fumble: CombatFumbleRulesViewV1,
    pub damage: CombatDamageRulesViewV1,
    pub wounds: CombatWoundRulesViewV1,
    pub practice: CombatPracticeRulesViewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LevelThresholdViewV1 {
    pub level: i32,
    pub cumulative_experience: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrowthAttributeViewV1 {
    Strength,
    Constitution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WeightedGrowthOutcomeViewV1 {
    pub amount: i32,
    pub weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AttributeGrowthBandViewV1 {
    pub minimum_attribute: i32,
    pub outcomes: Vec<WeightedGrowthOutcomeViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GrowthRuleViewV1 {
    Fixed {
        outcomes: Vec<WeightedGrowthOutcomeViewV1>,
    },
    AttributeBands {
        attribute: GrowthAttributeViewV1,
        bands: Vec<AttributeGrowthBandViewV1>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CombatAddGrowthViewV1 {
    pub level: i32,
    pub strength_adds: i32,
    pub dexterity_adds: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProgressionGrowthProfileViewV1 {
    pub class_id: String,
    pub hit_points: GrowthRuleViewV1,
    pub magic_points: Option<GrowthRuleViewV1>,
    pub stamina_points: GrowthRuleViewV1,
    pub physical_attribute_adds_by_level: Vec<CombatAddGrowthViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProgressionRulesViewV1 {
    pub level_thresholds: Vec<LevelThresholdViewV1>,
    pub growth_profiles: Vec<ProgressionGrowthProfileViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RulesViewV1 {
    pub progression: ProgressionRulesViewV1,
    pub skills: SkillRulesViewV1,
    pub movement: MovementRulesViewV1,
    pub burden: BurdenRulesViewV1,
    pub resources: ResourceRulesViewV1,
    pub magic: MagicRulesViewV1,
    pub combat: CombatRulesViewV1,
}
