use serde::{Deserialize, Serialize};

use crate::model::ItemPlacementKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmorDef {
    pub block_rating: i32,
    pub encumbrance: i32,
    pub damage_reduction: ArmorDamageReductionDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmorDamageReductionDef {
    pub cutting: i32,
    pub piercing: i32,
    pub crushing: i32,
}

pub(super) fn validate_armor_definition(
    armor: &ArmorDef,
    valid_placements: &[ItemPlacementKind],
    prefix: &str,
    errors: &mut Vec<String>,
) {
    for (field, value) in [
        ("block_rating", armor.block_rating),
        ("encumbrance", armor.encumbrance),
        ("damage_reduction.cutting", armor.damage_reduction.cutting),
        ("damage_reduction.piercing", armor.damage_reduction.piercing),
        ("damage_reduction.crushing", armor.damage_reduction.crushing),
    ] {
        if value < 0 {
            errors.push(format!("{prefix}.{field} must be non-negative"));
        }
    }

    if armor.block_rating == 0
        && armor.damage_reduction.cutting == 0
        && armor.damage_reduction.piercing == 0
        && armor.damage_reduction.crushing == 0
    {
        errors.push(format!(
            "{prefix} must provide block_rating or damage reduction"
        ));
    }

    if !valid_placements.iter().any(|placement| {
        matches!(
            placement,
            ItemPlacementKind::Head
                | ItemPlacementKind::Neck
                | ItemPlacementKind::Arm
                | ItemPlacementKind::Gloves
                | ItemPlacementKind::InnerArmor
                | ItemPlacementKind::OuterArmor
                | ItemPlacementKind::Boots
        )
    }) {
        errors.push(format!(
            "{prefix} requires at least one valid worn armor placement"
        ));
    }
}
