use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::model::{CharacterAlignment, PhysicalAttackMode, PhysicalDamageKind, WeaponHandedness};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BowNockingDef {
    pub unloads_on_movement: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponAttackModeDef {
    pub mode: PhysicalAttackMode,
    pub maximum_range: i32,
    pub damage_kind: PhysicalDamageKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponDef {
    pub skill_track_id: String,
    pub default_attack_mode: PhysicalAttackMode,
    pub attack_modes: Vec<WeaponAttackModeDef>,
    pub cooldown_units: u32,
    pub combat_add_rating: i32,
    pub handedness: WeaponHandedness,
    pub block_value: i32,
    #[serde(default)]
    pub nocking: Option<BowNockingDef>,
    #[serde(default)]
    pub required_alignment: Option<CharacterAlignment>,
}

pub(super) fn validate_weapon_definition(
    weapon: &WeaponDef,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    if weapon.skill_track_id.trim().is_empty() {
        errors.push(format!("{prefix}.skill_track_id must be non-empty"));
    }
    if weapon.attack_modes.is_empty() {
        errors.push(format!("{prefix}.attack_modes must not be empty"));
    }

    let mut modes = HashSet::new();
    for (index, attack_mode) in weapon.attack_modes.iter().enumerate() {
        let mode_prefix = format!("{prefix}.attack_modes[{index}]");
        if !attack_mode.mode.is_weapon_authored() {
            errors.push(format!(
                "{mode_prefix}.mode must be fight, poke, shoot, or throw"
            ));
        }
        if !modes.insert(attack_mode.mode) {
            errors.push(format!(
                "{prefix}.attack_modes contains duplicate {}",
                attack_mode.mode.label()
            ));
        }
        match attack_mode.mode {
            PhysicalAttackMode::Fight if attack_mode.maximum_range != 0 => {
                errors.push(format!("{mode_prefix}.maximum_range must be 0 for fight"))
            }
            PhysicalAttackMode::Poke if !(0..=1).contains(&attack_mode.maximum_range) => errors
                .push(format!(
                    "{mode_prefix}.maximum_range must be 0 or 1 for poke"
                )),
            PhysicalAttackMode::Shoot | PhysicalAttackMode::Throw
                if attack_mode.maximum_range <= 0 =>
            {
                errors.push(format!(
                    "{mode_prefix}.maximum_range must be positive for {}",
                    attack_mode.mode.label()
                ));
            }
            _ => {}
        }
    }

    if !modes.contains(&weapon.default_attack_mode) {
        errors.push(format!(
            "{prefix}.default_attack_mode must name an authored attack mode"
        ));
    }
    if weapon.cooldown_units == 0 {
        errors.push(format!("{prefix}.cooldown_units must be positive"));
    }
    if weapon.combat_add_rating < 0 {
        errors.push(format!("{prefix}.combat_add_rating must be non-negative"));
    }
    if weapon.block_value < 0 {
        errors.push(format!("{prefix}.block_value must be non-negative"));
    }

    if weapon.handedness == WeaponHandedness::Bow {
        if weapon.attack_modes.len() != 1
            || weapon.attack_modes[0].mode != PhysicalAttackMode::Shoot
            || weapon.default_attack_mode != PhysicalAttackMode::Shoot
        {
            errors.push(format!(
                "{prefix}.attack_modes must contain exactly shoot as the bow default"
            ));
        }
        if weapon.nocking.is_none() {
            errors.push(format!("{prefix}.nocking must be present for bows"));
        }
    } else {
        if weapon.nocking.is_some() {
            errors.push(format!("{prefix}.nocking is only valid for bows"));
        }
        if modes.contains(&PhysicalAttackMode::Shoot) {
            errors.push(format!("{prefix}.shoot mode is only valid for bows"));
        }
    }
}
