use crate::model::{GrowthAttribute, GrowthRule};
use crate::view::{
    ActiveMpRecoveryItemPolicyViewV1, AttributeGrowthBandViewV1, BurdenRulesViewV1,
    CombatAddGrowthViewV1, CombatAttackModeRulesViewV1, CombatBlockRulesViewV1,
    CombatDamageRulesViewV1, CombatHitRulesViewV1, CombatJumpkickRulesViewV1,
    CombatKickRulesViewV1, CombatPracticeRulesViewV1, CombatRulesViewV1, CombatTuningStatusViewV1,
    CombatWoundRulesViewV1, DamageInterruptionComparisonViewV1, GrowthAttributeViewV1,
    GrowthRuleViewV1, LevelThresholdViewV1, MagicArithmeticRoundingViewV1,
    MagicCastingPracticeRulesViewV1, MagicEffectFamilyRulesViewV1, MagicKillExperienceRulesViewV1,
    MagicMpRecoveryRulesViewV1, MagicRewardFractionViewV1, MagicRuleEvidenceStateViewV1,
    MagicRulesViewV1, MovementRulesViewV1, ProgressionGrowthProfileViewV1, ProgressionRulesViewV1,
    RaiseDeadRulesViewV1, ResourceRulesViewV1, RulesViewV1, SkillRulesViewV1,
    SpellDamageInterruptionRulesViewV1, SpellWarmupRulesViewV1, ThaumAboveSkillRulesViewV1,
    TrainingRulesViewV1, WeightedGrowthOutcomeViewV1,
};

use super::super::Engine;

fn growth_rule_view(rule: &GrowthRule) -> GrowthRuleViewV1 {
    match rule {
        GrowthRule::Fixed { outcomes } => GrowthRuleViewV1::Fixed {
            outcomes: outcomes
                .iter()
                .map(|outcome| WeightedGrowthOutcomeViewV1 {
                    amount: outcome.amount,
                    weight: outcome.weight,
                })
                .collect(),
        },
        GrowthRule::AttributeBands { attribute, bands } => GrowthRuleViewV1::AttributeBands {
            attribute: match attribute {
                GrowthAttribute::Strength => GrowthAttributeViewV1::Strength,
                GrowthAttribute::Constitution => GrowthAttributeViewV1::Constitution,
            },
            bands: bands
                .iter()
                .map(|band| AttributeGrowthBandViewV1 {
                    minimum_attribute: band.minimum_attribute,
                    outcomes: band
                        .outcomes
                        .iter()
                        .map(|outcome| WeightedGrowthOutcomeViewV1 {
                            amount: outcome.amount,
                            weight: outcome.weight,
                        })
                        .collect(),
                })
                .collect(),
        },
    }
}

fn magic_evidence_view(
    state: crate::model::MagicRuleEvidenceState,
) -> MagicRuleEvidenceStateViewV1 {
    match state {
        crate::model::MagicRuleEvidenceState::OriginalProvisional => {
            MagicRuleEvidenceStateViewV1::OriginalProvisional
        }
        crate::model::MagicRuleEvidenceState::TargetRelease => {
            MagicRuleEvidenceStateViewV1::TargetRelease
        }
    }
}

impl Engine {
    pub(super) fn rules_view(&self) -> RulesViewV1 {
        let rules = &self.definition.catalog.rules;
        RulesViewV1 {
            progression: ProgressionRulesViewV1 {
                level_thresholds: rules
                    .progression
                    .level_thresholds
                    .iter()
                    .map(|row| LevelThresholdViewV1 {
                        level: row.level,
                        cumulative_experience: row.cumulative_experience,
                    })
                    .collect(),
                growth_profiles: rules
                    .progression
                    .growth_profiles
                    .values()
                    .map(|profile| ProgressionGrowthProfileViewV1 {
                        class_id: profile.class_id.clone(),
                        hit_points: growth_rule_view(&profile.hit_points),
                        magic_points: profile.magic_points.as_ref().map(growth_rule_view),
                        stamina_points: growth_rule_view(&profile.stamina_points),
                        physical_attribute_adds_by_level: profile
                            .physical_attribute_adds_by_level
                            .iter()
                            .map(|row| CombatAddGrowthViewV1 {
                                level: row.level,
                                strength_adds: row.strength_adds,
                                dexterity_adds: row.dexterity_adds,
                            })
                            .collect(),
                    })
                    .collect(),
            },
            skills: SkillRulesViewV1 {
                base_learning_rate: rules.skills.base_learning_rate,
                practice_thresholds: rules.skills.practice_thresholds.clone(),
                training: TrainingRulesViewV1 {
                    gold_per_learning_rate: rules.skills.training.gold_per_learning_rate,
                    experience_per_learning_rate: rules
                        .skills
                        .training
                        .experience_per_learning_rate,
                    maximum_learning_rates: rules.skills.training.maximum_learning_rates.clone(),
                },
            },
            movement: MovementRulesViewV1 {
                controlled_path_points: rules.movement.controlled_path_points,
                automatic_step_points: rules.movement.automatic_step_points,
            },
            burden: BurdenRulesViewV1 {
                coin_burden_per_gold: rules.burden.coin_burden_per_gold,
                lightly_loaded_max_per_strength: rules.burden.lightly_loaded_max_per_strength,
                moderately_loaded_max_per_strength: rules.burden.moderately_loaded_max_per_strength,
                heavily_loaded_max_per_strength: rules.burden.heavily_loaded_max_per_strength,
            },
            resources: ResourceRulesViewV1 {
                recovery_interval_units: rules.resources.recovery_interval_units,
                active_hp_recovery: rules.resources.active_hp_recovery,
                inactive_hp_recovery: rules.resources.inactive_hp_recovery,
                inactive_stamina_recovery: rules.resources.inactive_stamina_recovery,
                mp_recovery: rules.resources.mp_recovery,
                normal_movement_stamina_cost: rules.resources.normal_movement_stamina_cost,
                rapid_movement_stamina_cost: rules.resources.rapid_movement_stamina_cost,
            },
            magic: MagicRulesViewV1 {
                warmup: SpellWarmupRulesViewV1 {
                    units: rules.magic.warmup.units,
                    evidence_state: match rules.magic.warmup.evidence_state {
                        crate::model::MagicRuleEvidenceState::OriginalProvisional => {
                            MagicRuleEvidenceStateViewV1::OriginalProvisional
                        }
                        crate::model::MagicRuleEvidenceState::TargetRelease => {
                            MagicRuleEvidenceStateViewV1::TargetRelease
                        }
                    },
                },
                damage_interruption: SpellDamageInterruptionRulesViewV1 {
                    comparison: match rules.magic.damage_interruption.comparison {
                        crate::model::DamageInterruptionComparison::StrictlyGreater => {
                            DamageInterruptionComparisonViewV1::StrictlyGreater
                        }
                    },
                    numerator: rules.magic.damage_interruption.numerator,
                    denominator: rules.magic.damage_interruption.denominator,
                    evidence_state: match rules.magic.damage_interruption.evidence_state {
                        crate::model::MagicRuleEvidenceState::OriginalProvisional => {
                            MagicRuleEvidenceStateViewV1::OriginalProvisional
                        }
                        crate::model::MagicRuleEvidenceState::TargetRelease => {
                            MagicRuleEvidenceStateViewV1::TargetRelease
                        }
                    },
                },
                resistance: crate::view::SpellResistanceRulesViewV1 {
                    denominator: rules.magic.resistance.denominator,
                    denominator_evidence_state: match rules
                        .magic
                        .resistance
                        .denominator_evidence_state
                    {
                        crate::model::MagicRuleEvidenceState::OriginalProvisional => {
                            MagicRuleEvidenceStateViewV1::OriginalProvisional
                        }
                        crate::model::MagicRuleEvidenceState::TargetRelease => {
                            MagicRuleEvidenceStateViewV1::TargetRelease
                        }
                    },
                    success_comparison: rules.magic.resistance.success_comparison,
                    matching_boost_policy: rules.magic.resistance.matching_boost_policy,
                    resolution_evidence_state: match rules
                        .magic
                        .resistance
                        .resolution_evidence_state
                    {
                        crate::model::MagicRuleEvidenceState::OriginalProvisional => {
                            MagicRuleEvidenceStateViewV1::OriginalProvisional
                        }
                        crate::model::MagicRuleEvidenceState::TargetRelease => {
                            MagicRuleEvidenceStateViewV1::TargetRelease
                        }
                    },
                },
                casting_practice: MagicCastingPracticeRulesViewV1 {
                    minimum_raw_points: rules.magic.casting_practice.minimum_raw_points,
                    raw_points_per_mp: rules.magic.casting_practice.raw_points_per_mp,
                    primary_attribute_points_per_bonus: rules
                        .magic
                        .casting_practice
                        .primary_attribute_points_per_bonus,
                    evidence_state: magic_evidence_view(
                        rules.magic.casting_practice.evidence_state,
                    ),
                },
                thaum_above_skill: ThaumAboveSkillRulesViewV1 {
                    roll_denominator: rules.magic.thaum_above_skill.roll_denominator,
                    penalty_per_missing_level: rules
                        .magic
                        .thaum_above_skill
                        .penalty_per_missing_level,
                    minimum_success_threshold: rules
                        .magic
                        .thaum_above_skill
                        .minimum_success_threshold,
                    evidence_state: magic_evidence_view(
                        rules.magic.thaum_above_skill.evidence_state,
                    ),
                },
                kill_experience: MagicKillExperienceRulesViewV1 {
                    directed: MagicRewardFractionViewV1 {
                        numerator: rules.magic.kill_experience.directed.numerator,
                        denominator: rules.magic.kill_experience.directed.denominator,
                    },
                    area_or_illusion: MagicRewardFractionViewV1 {
                        numerator: rules.magic.kill_experience.area_or_illusion.numerator,
                        denominator: rules.magic.kill_experience.area_or_illusion.denominator,
                    },
                    fraction_evidence_state: magic_evidence_view(
                        rules.magic.kill_experience.fraction_evidence_state,
                    ),
                    rounding: match rules.magic.kill_experience.rounding {
                        crate::model::MagicArithmeticRounding::Down => {
                            MagicArithmeticRoundingViewV1::Down
                        }
                    },
                    rounding_evidence_state: magic_evidence_view(
                        rules.magic.kill_experience.rounding_evidence_state,
                    ),
                },
                mp_recovery: MagicMpRecoveryRulesViewV1 {
                    active_item_policy: match rules.magic.mp_recovery.active_item_policy {
                        crate::model::ActiveMpRecoveryItemPolicy::HighestMultiplier => {
                            ActiveMpRecoveryItemPolicyViewV1::HighestMultiplier
                        }
                    },
                    rounding: match rules.magic.mp_recovery.rounding {
                        crate::model::MagicArithmeticRounding::Down => {
                            MagicArithmeticRoundingViewV1::Down
                        }
                    },
                    evidence_state: magic_evidence_view(rules.magic.mp_recovery.evidence_state),
                },
                effect_families: MagicEffectFamilyRulesViewV1 {
                    raise_dead: RaiseDeadRulesViewV1 {
                        roll_denominator: rules.magic.effect_families.raise_dead.roll_denominator,
                        success_threshold_per_magic_level: rules
                            .magic
                            .effect_families
                            .raise_dead
                            .success_threshold_per_magic_level,
                        minimum_success_threshold: rules
                            .magic
                            .effect_families
                            .raise_dead
                            .minimum_success_threshold,
                        evidence_state: magic_evidence_view(
                            rules.magic.effect_families.raise_dead.evidence_state,
                        ),
                    },
                },
            },
            combat: CombatRulesViewV1 {
                tuning_status: match rules.combat.tuning_status {
                    crate::combat::CombatTuningStatus::OriginalProvisional => {
                        CombatTuningStatusViewV1::OriginalProvisional
                    }
                },
                attack_modes: CombatAttackModeRulesViewV1 {
                    kick: CombatKickRulesViewV1 {
                        maximum_range: rules.combat.attack_modes.kick.maximum_range,
                        cooldown_units: rules.combat.attack_modes.kick.cooldown_units,
                        damage_kind: rules.combat.attack_modes.kick.damage_kind,
                    },
                    jumpkick: CombatJumpkickRulesViewV1 {
                        maximum_range_cap: rules.combat.attack_modes.jumpkick.maximum_range_cap,
                        skill_levels_per_extra_hex: rules
                            .combat
                            .attack_modes
                            .jumpkick
                            .skill_levels_per_extra_hex,
                        stamina_cost: rules.combat.attack_modes.jumpkick.stamina_cost,
                        cooldown_units: rules.combat.attack_modes.jumpkick.cooldown_units,
                        damage_kind: rules.combat.attack_modes.jumpkick.damage_kind,
                    },
                },
                hit: CombatHitRulesViewV1 {
                    base_defender_score: rules.combat.hit.base_defender_score,
                    attacker_attack_stat_divisor: rules.combat.hit.attacker_attack_stat_divisor,
                    attacker_skill_level_divisor: rules.combat.hit.attacker_skill_level_divisor,
                    defender_defense_stat_divisor: rules.combat.hit.defender_defense_stat_divisor,
                    defender_dexterity_divisor: rules.combat.hit.defender_dexterity_divisor,
                    non_character_defender_dexterity: rules
                        .combat
                        .hit
                        .non_character_defender_dexterity,
                },
                block: CombatBlockRulesViewV1 {
                    left_hand_selection_percent: rules.combat.block.left_hand_selection_percent,
                    shield_percent_per_point: rules.combat.block.shield_percent_per_point,
                    shield_percent_cap: rules.combat.block.shield_percent_cap,
                    armor_percent_per_point: rules.combat.block.armor_percent_per_point,
                    armor_percent_cap: rules.combat.block.armor_percent_cap,
                    strength_penetration_percent_per_add: rules
                        .combat
                        .block
                        .strength_penetration_percent_per_add,
                    armor_encumbrance_percent_per_point: rules
                        .combat
                        .block
                        .armor_encumbrance_percent_per_point,
                    combat_add_penetration_percent_per_rating: rules
                        .combat
                        .block
                        .combat_add_penetration_percent_per_rating,
                },
                fumble: crate::view::CombatFumbleRulesViewV1 {
                    base_percent: rules.combat.fumble.base_percent,
                    minimum_percent: rules.combat.fumble.minimum_percent,
                    skill_levels_per_reduction: rules.combat.fumble.skill_levels_per_reduction,
                },
                damage: CombatDamageRulesViewV1 {
                    minimum_damage: rules.combat.damage.minimum_damage,
                    roll_variation_modulus: rules.combat.damage.roll_variation_modulus,
                    moderate_label_min_percent: rules.combat.damage.moderate_label_min_percent,
                    heavy_label_min_percent: rules.combat.damage.heavy_label_min_percent,
                    severe_label_min_percent: rules.combat.damage.severe_label_min_percent,
                },
                wounds: CombatWoundRulesViewV1 {
                    near_death_max_percent: rules.combat.wounds.near_death_max_percent,
                    badly_wounded_max_percent: rules.combat.wounds.badly_wounded_max_percent,
                    wounded_max_percent: rules.combat.wounds.wounded_max_percent,
                },
                practice: CombatPracticeRulesViewV1 {
                    practice_raw_points: rules.combat.practice.practice_raw_points,
                    life_and_death_raw_points: rules.combat.practice.life_and_death_raw_points,
                    overwhelming_raw_points: rules.combat.practice.overwhelming_raw_points,
                    fatal_blow_bonus_raw_points: rules.combat.practice.fatal_blow_bonus_raw_points,
                    life_and_death_minimum_target_xp_per_attacker_level: rules
                        .combat
                        .practice
                        .life_and_death_minimum_target_xp_per_attacker_level,
                    life_and_death_required_at_skill_level: rules
                        .combat
                        .practice
                        .life_and_death_required_at_skill_level,
                },
            },
        }
    }
}
