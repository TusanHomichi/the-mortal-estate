use serde::{Deserialize, Serialize};

use crate::model::CarriedPosition;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PhysicalBlockCandidateViewV1 {
    pub source: crate::model::BlockSourceKind,
    pub carried_position: Option<CarriedPosition>,
    pub item_instance_id: Option<String>,
    pub block_value: i32,
    pub skill_track_id: Option<String>,
    pub skill_level: Option<u8>,
    pub chance_percent: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PhysicalWeaponModeViewV1 {
    pub mode: crate::model::PhysicalAttackMode,
    pub maximum_range: i32,
    pub damage_kind: crate::model::PhysicalDamageKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PhysicalWeaponViewV1 {
    pub item_instance_id: Option<String>,
    pub item_definition_id: Option<String>,
    pub skill_track_id: String,
    pub skill_level: u8,
    pub default_attack_mode: crate::model::PhysicalAttackMode,
    pub attack_modes: Vec<PhysicalWeaponModeViewV1>,
    pub cooldown_units: u32,
    pub combat_add_rating: i32,
    pub effective_combat_add_rating: i32,
    pub handedness: Option<crate::model::WeaponHandedness>,
    pub block_value: i32,
    pub nocking_unloads_on_movement: Option<bool>,
    pub offhand_occupied: bool,
    pub full_two_handed_effect: bool,
    pub bow_readiness: Option<crate::model::BowReadiness>,
    pub required_alignment: Option<crate::model::CharacterAlignment>,
    pub binding_usable: bool,
    pub alignment_usable: bool,
    pub restriction_usable: bool,
    pub eligible_block_candidates: Vec<PhysicalBlockCandidateViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ArmorDefinitionViewV1 {
    pub block_rating: i32,
    pub encumbrance: i32,
    pub cutting_reduction: i32,
    pub piercing_reduction: i32,
    pub crushing_reduction: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ArmorProtectionSourceViewV1 {
    pub carried_position: CarriedPosition,
    pub item_instance_id: String,
    pub item_definition_id: String,
    pub block_rating: i32,
    pub encumbrance: i32,
    pub cutting_reduction: i32,
    pub piercing_reduction: i32,
    pub crushing_reduction: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ArmorProtectionViewV1 {
    pub sources: Vec<ArmorProtectionSourceViewV1>,
    pub block_rating: i32,
    pub encumbrance: i32,
    pub cutting_reduction: i32,
    pub piercing_reduction: i32,
    pub crushing_reduction: i32,
}
