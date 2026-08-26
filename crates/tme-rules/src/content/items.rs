use serde::{Deserialize, Serialize};

use crate::content::{ArmorDef, WeaponDef};
use crate::model::{ItemBindingState, ItemCapability, ItemPlacementKind, WorldPosition};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemDef {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub valid_placements: Vec<ItemPlacementKind>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub weapon: Option<WeaponDef>,
    #[serde(default)]
    pub armor: Option<ArmorDef>,
    pub consumable: Option<ConsumableDef>,
    #[serde(default)]
    pub capability: Option<ItemCapability>,
    pub economy: ItemEconomyDef,
    pub review_note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemEconomyDef {
    #[serde(default)]
    pub unit_value_gold: Option<u64>,
    pub unit_burden: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemKnowledgeDef {
    #[serde(default)]
    pub identified: bool,
    #[serde(default)]
    pub appraised: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemInstanceSeedDef {
    pub definition_id: String,
    #[serde(default = "default_item_quantity")]
    pub quantity: u32,
    #[serde(default)]
    pub knowledge: ItemKnowledgeDef,
    pub binding: ItemBindingState,
}

fn default_item_quantity() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumableDef {
    pub effect: String,
    pub heal_per_round: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundItemSeedDef {
    pub item_instance_id: String,
    pub location: WorldPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageLabelDef {
    pub id: String,
    pub name: String,
    pub review_note: Option<String>,
}
