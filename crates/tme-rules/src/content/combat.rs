use serde::{Deserialize, Serialize};

use crate::model::PhysicalDamageKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatTuningStatusDef {
    OriginalProvisional,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatRulesDef {
    pub tuning_status: CombatTuningStatusDef,
    pub attack_modes: CombatAttackModeRulesDef,
    pub hit: CombatHitRulesDef,
    pub block: CombatBlockRulesDef,
    pub fumble: CombatFumbleRulesDef,
    pub damage: CombatDamageRulesDef,
    pub wounds: CombatWoundRulesDef,
    pub practice: CombatPracticeRulesDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatAttackModeRulesDef {
    pub kick: CombatKickRulesDef,
    pub jumpkick: CombatJumpkickRulesDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatKickRulesDef {
    pub maximum_range: i32,
    pub cooldown_units: u32,
    pub damage_kind: PhysicalDamageKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatJumpkickRulesDef {
    pub maximum_range_cap: i32,
    pub skill_levels_per_extra_hex: u32,
    pub stamina_cost: i32,
    pub cooldown_units: u32,
    pub damage_kind: PhysicalDamageKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatHitRulesDef {
    pub base_defender_score: i32,
    pub attacker_attack_stat_divisor: i32,
    pub attacker_skill_level_divisor: i32,
    pub defender_defense_stat_divisor: i32,
    pub defender_dexterity_divisor: i32,
    pub non_character_defender_dexterity: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatBlockRulesDef {
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
#[serde(deny_unknown_fields)]
pub struct CombatFumbleRulesDef {
    pub base_percent: u32,
    pub minimum_percent: u32,
    pub skill_levels_per_reduction: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatDamageRulesDef {
    pub minimum_damage: i32,
    pub roll_variation_modulus: u32,
    pub moderate_label_min_percent: u32,
    pub heavy_label_min_percent: u32,
    pub severe_label_min_percent: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatWoundRulesDef {
    pub near_death_max_percent: u32,
    pub badly_wounded_max_percent: u32,
    pub wounded_max_percent: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPracticeRulesDef {
    pub practice_raw_points: u64,
    pub life_and_death_raw_points: u64,
    pub overwhelming_raw_points: u64,
    pub fatal_blow_bonus_raw_points: u64,
    pub life_and_death_minimum_target_xp_per_attacker_level: u64,
    pub life_and_death_required_at_skill_level: u32,
}

impl CombatRulesDef {
    pub(super) fn validate_intrinsic(&self, prefix: &str, errors: &mut Vec<String>) {
        if self.attack_modes.kick.maximum_range != 0 {
            errors.push(format!(
                "{prefix}.attack_modes.kick.maximum_range must be 0"
            ));
        }
        if self.attack_modes.kick.cooldown_units == 0 {
            errors.push(format!(
                "{prefix}.attack_modes.kick.cooldown_units must be positive"
            ));
        }
        if self.attack_modes.kick.damage_kind != PhysicalDamageKind::Crushing {
            errors.push(format!(
                "{prefix}.attack_modes.kick.damage_kind must be crushing"
            ));
        }

        let jumpkick = &self.attack_modes.jumpkick;
        if !(1..=3).contains(&jumpkick.maximum_range_cap) {
            errors.push(format!(
                "{prefix}.attack_modes.jumpkick.maximum_range_cap must be in 1..=3"
            ));
        }
        if jumpkick.skill_levels_per_extra_hex == 0 {
            errors.push(format!(
                "{prefix}.attack_modes.jumpkick.skill_levels_per_extra_hex must be positive"
            ));
        }
        if jumpkick.stamina_cost < 0 {
            errors.push(format!(
                "{prefix}.attack_modes.jumpkick.stamina_cost must be non-negative"
            ));
        }
        if jumpkick.cooldown_units == 0 {
            errors.push(format!(
                "{prefix}.attack_modes.jumpkick.cooldown_units must be positive"
            ));
        }
        if jumpkick.damage_kind != PhysicalDamageKind::Crushing {
            errors.push(format!(
                "{prefix}.attack_modes.jumpkick.damage_kind must be crushing"
            ));
        }

        let hit_values = [
            ("base_defender_score", self.hit.base_defender_score),
            (
                "attacker_attack_stat_divisor",
                self.hit.attacker_attack_stat_divisor,
            ),
            (
                "attacker_skill_level_divisor",
                self.hit.attacker_skill_level_divisor,
            ),
            (
                "defender_defense_stat_divisor",
                self.hit.defender_defense_stat_divisor,
            ),
            (
                "defender_dexterity_divisor",
                self.hit.defender_dexterity_divisor,
            ),
            (
                "non_character_defender_dexterity",
                self.hit.non_character_defender_dexterity,
            ),
        ];
        for (field, value) in hit_values {
            if value <= 0 {
                errors.push(format!("{prefix}.hit.{field} must be positive"));
            }
        }

        let block_percentages = [
            (
                "left_hand_selection_percent",
                self.block.left_hand_selection_percent,
            ),
            (
                "shield_percent_per_point",
                self.block.shield_percent_per_point,
            ),
            ("shield_percent_cap", self.block.shield_percent_cap),
            (
                "armor_percent_per_point",
                self.block.armor_percent_per_point,
            ),
            ("armor_percent_cap", self.block.armor_percent_cap),
            (
                "strength_penetration_percent_per_add",
                self.block.strength_penetration_percent_per_add,
            ),
            (
                "armor_encumbrance_percent_per_point",
                self.block.armor_encumbrance_percent_per_point,
            ),
            (
                "combat_add_penetration_percent_per_rating",
                self.block.combat_add_penetration_percent_per_rating,
            ),
        ];
        for (field, value) in block_percentages {
            if !(1..=100).contains(&value) {
                errors.push(format!(
                    "{prefix}.block.{field} must be an integer in 1..=100"
                ));
            }
        }
        if self.block.shield_percent_per_point > self.block.shield_percent_cap {
            errors.push(format!(
                "{prefix}.block.shield_percent_per_point must not exceed shield_percent_cap"
            ));
        }
        if self.block.armor_percent_per_point > self.block.armor_percent_cap {
            errors.push(format!(
                "{prefix}.block.armor_percent_per_point must not exceed armor_percent_cap"
            ));
        }

        if !(1..=100).contains(&self.fumble.base_percent) {
            errors.push(format!(
                "{prefix}.fumble.base_percent must be an integer in 1..=100"
            ));
        }
        if !(1..=self.fumble.base_percent).contains(&self.fumble.minimum_percent) {
            errors.push(format!(
                "{prefix}.fumble.minimum_percent must be in 1..=base_percent"
            ));
        }
        if self.fumble.skill_levels_per_reduction == 0 {
            errors.push(format!(
                "{prefix}.fumble.skill_levels_per_reduction must be positive"
            ));
        }

        if self.damage.minimum_damage <= 0 {
            errors.push(format!("{prefix}.damage.minimum_damage must be positive"));
        }
        if self.damage.roll_variation_modulus == 0 {
            errors.push(format!(
                "{prefix}.damage.roll_variation_modulus must be positive"
            ));
        }
        let moderate = self.damage.moderate_label_min_percent;
        let heavy = self.damage.heavy_label_min_percent;
        let severe = self.damage.severe_label_min_percent;
        if moderate == 0 || moderate >= heavy || heavy >= severe || severe > 100 {
            errors.push(format!(
                "{prefix}.damage label thresholds must satisfy 0 < moderate_label_min_percent < heavy_label_min_percent < severe_label_min_percent <= 100"
            ));
        }

        let wounds = &self.wounds;
        if wounds.near_death_max_percent == 0
            || wounds.near_death_max_percent >= wounds.badly_wounded_max_percent
            || wounds.badly_wounded_max_percent >= wounds.wounded_max_percent
            || wounds.wounded_max_percent >= 100
        {
            errors.push(format!(
                "{prefix}.wounds must satisfy 0 < near_death_max_percent < badly_wounded_max_percent < wounded_max_percent < 100"
            ));
        }

        let practice = &self.practice;
        for (field, value) in [
            ("practice_raw_points", practice.practice_raw_points),
            (
                "life_and_death_raw_points",
                practice.life_and_death_raw_points,
            ),
            ("overwhelming_raw_points", practice.overwhelming_raw_points),
            (
                "fatal_blow_bonus_raw_points",
                practice.fatal_blow_bonus_raw_points,
            ),
            (
                "life_and_death_minimum_target_xp_per_attacker_level",
                practice.life_and_death_minimum_target_xp_per_attacker_level,
            ),
        ] {
            if value == 0 {
                errors.push(format!("{prefix}.practice.{field} must be positive"));
            }
        }
        if !(1..=19).contains(&practice.life_and_death_required_at_skill_level) {
            errors.push(format!(
                "{prefix}.practice.life_and_death_required_at_skill_level must be in 1..=19"
            ));
        }
    }
}
