use serde::{Deserialize, Serialize};

use crate::model::PhysicalDamageKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatTuningStatus {
    OriginalProvisional,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatRules {
    pub tuning_status: CombatTuningStatus,
    pub attack_modes: CombatAttackModeRules,
    pub hit: CombatHitRules,
    pub block: CombatBlockRules,
    pub fumble: CombatFumbleRules,
    pub damage: CombatDamageRules,
    pub wounds: CombatWoundRules,
    pub practice: CombatPracticeRules,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatAttackModeRules {
    pub kick: CombatKickRules,
    pub jumpkick: CombatJumpkickRules,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatKickRules {
    pub maximum_range: i32,
    pub cooldown_units: u32,
    pub damage_kind: PhysicalDamageKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatJumpkickRules {
    pub maximum_range_cap: i32,
    pub skill_levels_per_extra_hex: u32,
    pub stamina_cost: i32,
    pub cooldown_units: u32,
    pub damage_kind: PhysicalDamageKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatHitRules {
    pub base_defender_score: i32,
    pub attacker_attack_stat_divisor: i32,
    pub attacker_skill_level_divisor: i32,
    pub defender_defense_stat_divisor: i32,
    pub defender_dexterity_divisor: i32,
    pub non_character_defender_dexterity: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatBlockRules {
    pub left_hand_selection_percent: u32,
    pub shield_percent_per_point: u32,
    pub shield_percent_cap: u32,
    pub armor_percent_per_point: u32,
    pub armor_percent_cap: u32,
    pub strength_penetration_percent_per_add: u32,
    pub armor_encumbrance_percent_per_point: u32,
    pub combat_add_penetration_percent_per_rating: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatFumbleRules {
    pub base_percent: u32,
    pub minimum_percent: u32,
    pub skill_levels_per_reduction: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatDamageRules {
    pub minimum_damage: i32,
    pub roll_variation_modulus: u32,
    pub moderate_label_min_percent: u32,
    pub heavy_label_min_percent: u32,
    pub severe_label_min_percent: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatWoundRules {
    pub near_death_max_percent: u32,
    pub badly_wounded_max_percent: u32,
    pub wounded_max_percent: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatPracticeRules {
    pub practice_raw_points: u64,
    pub life_and_death_raw_points: u64,
    pub overwhelming_raw_points: u64,
    pub fatal_blow_bonus_raw_points: u64,
    pub life_and_death_minimum_target_xp_per_attacker_level: u64,
    pub life_and_death_required_at_skill_level: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageLabel {
    Light,
    Moderate,
    Heavy,
    Severe,
    Fatal,
}

impl DamageLabel {
    pub fn for_hit(
        damage: i32,
        defender_hp_before: i32,
        defender_hp_after: i32,
        moderate_min_percent: u32,
        heavy_min_percent: u32,
        severe_min_percent: u32,
    ) -> Self {
        if defender_hp_after <= 0 {
            return Self::Fatal;
        }

        let hp_before = i64::from(defender_hp_before.max(1));
        let percent = i64::from(damage.max(0)) * 100 / hp_before;

        if percent < i64::from(moderate_min_percent) {
            Self::Light
        } else if percent < i64::from(heavy_min_percent) {
            Self::Moderate
        } else if percent < i64::from(severe_min_percent) {
            Self::Heavy
        } else {
            Self::Severe
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Moderate => "moderate",
            Self::Heavy => "heavy",
            Self::Severe => "severe",
            Self::Fatal => "fatal",
        }
    }
}
