use serde::{Deserialize, Serialize};

use crate::model::{
    BurdenTier, CarriedGold, CarriedPosition, ItemBindingState, ItemCapability, ItemPlacementKind,
    WorldPosition,
};

use super::{ArmorDefinitionViewV1, AttributeBonusViewV1, LootClaimViewV1};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ItemInstanceViewV1 {
    pub item_instance_id: String,
    pub item_definition_id: String,
    pub name: String,
    pub quantity: u32,
    pub identified: bool,
    pub appraised: bool,
    pub known_unit_value_gold: Option<u64>,
    pub known_stack_value_gold: Option<u64>,
    pub unit_burden: u64,
    pub stack_burden: u64,
    pub binding: ItemBindingViewV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bow_readiness: Option<crate::model::BowReadiness>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ItemBindingViewV1 {
    Unrestricted,
    BindOnFirstCharacterTouch,
    Bound,
}

impl From<&ItemBindingState> for ItemBindingViewV1 {
    fn from(binding: &ItemBindingState) -> Self {
        match binding {
            ItemBindingState::Unrestricted => Self::Unrestricted,
            ItemBindingState::BindOnFirstCharacterTouch => Self::BindOnFirstCharacterTouch,
            ItemBindingState::Bound { .. } => Self::Bound,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PositionedItemViewV1 {
    #[serde(flatten)]
    pub item: ItemInstanceViewV1,
    pub position: CarriedPosition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub valid_placements: Vec<ItemPlacementKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability: Option<ItemCapabilityViewV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub armor: Option<ArmorDefinitionViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CarriedLayoutViewV1 {
    pub items: Vec<PositionedItemViewV1>,
    pub gold: CarriedGold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BurdenViewV1 {
    pub item_burden: u64,
    pub coin_burden: u64,
    pub total_burden: u64,
    pub lightly_loaded_limit: Option<u64>,
    pub moderately_loaded_limit: Option<u64>,
    pub heavily_loaded_limit: Option<u64>,
    pub tier: Option<BurdenTier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ItemCapabilityViewV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taxonomy_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub training_focus_for: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spell_book_for: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_value: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute_adds: Option<Vec<AttributeBonusViewV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_adds: Option<Vec<AttributeBonusViewV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spell_focus_for: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resistance_boosts: Option<Vec<crate::model::SpellResistanceBoost>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mp_recovery_multiplier: Option<crate::model::MpRecoveryMultiplier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GroundItemViewV1 {
    #[serde(flatten)]
    pub item: ItemInstanceViewV1,
    pub location: WorldPosition,
    pub loot_claim: Option<LootClaimViewV1>,
}

impl From<&ItemCapability> for ItemCapabilityViewV1 {
    fn from(c: &ItemCapability) -> Self {
        Self {
            taxonomy_id: c.taxonomy_id.clone(),
            training_focus_for: c.training_focus_for.clone(),
            spell_book_for: c.spell_book_for.clone(),
            block_value: c.block_value,
            attribute_adds: c
                .attribute_adds
                .as_ref()
                .map(|v| v.iter().map(AttributeBonusViewV1::from).collect()),
            resource_adds: c
                .resource_adds
                .as_ref()
                .map(|v| v.iter().map(AttributeBonusViewV1::from).collect()),
            spell_focus_for: c.spell_focus_for.clone(),
            resistance_boosts: c.resistance_boosts.clone(),
            mp_recovery_multiplier: c.mp_recovery_multiplier.clone(),
        }
    }
}
