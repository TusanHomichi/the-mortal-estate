use std::collections::BTreeMap;

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

use super::{
    ActorId, BowReadiness, CharacterId, CorpseId, GoldPileId, LogicalTime, LootClaim, WorldPosition,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellItemLocation {
    Sack,
    ActiveEquipment,
    GroundHere,
}

impl SpellItemLocation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Sack => "sack",
            Self::ActiveEquipment => "active_equipment",
            Self::GroundHere => "ground_here",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemPlacementKind {
    Hand,
    RingFinger,
    BeltSide,
    BeltBack,
    Sack,
    Head,
    Neck,
    Arm,
    Gloves,
    InnerArmor,
    OuterArmor,
    Boots,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarriedGoldPosition {
    LeftHand,
    RightHand,
    Sack,
}

impl CarriedGoldPosition {
    pub const fn label(self) -> &'static str {
        match self {
            Self::LeftHand => "left_hand",
            Self::RightHand => "right_hand",
            Self::Sack => "sack",
        }
    }

    pub const fn hand_position(self) -> Option<CarriedPosition> {
        match self {
            Self::LeftHand => Some(CarriedPosition::LeftHand),
            Self::RightHand => Some(CarriedPosition::RightHand),
            Self::Sack => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarriedGold {
    pub left_hand: i64,
    pub right_hand: i64,
    pub sack: i64,
}

impl CarriedGold {
    pub const fn amount(self, position: CarriedGoldPosition) -> i64 {
        match position {
            CarriedGoldPosition::LeftHand => self.left_hand,
            CarriedGoldPosition::RightHand => self.right_hand,
            CarriedGoldPosition::Sack => self.sack,
        }
    }

    pub fn amount_mut(&mut self, position: CarriedGoldPosition) -> &mut i64 {
        match position {
            CarriedGoldPosition::LeftHand => &mut self.left_hand,
            CarriedGoldPosition::RightHand => &mut self.right_hand,
            CarriedGoldPosition::Sack => &mut self.sack,
        }
    }

    pub fn checked_total(self) -> Option<i64> {
        self.left_hand
            .checked_add(self.right_hand)?
            .checked_add(self.sack)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarriedPosition {
    LeftHand,
    RightHand,
    #[serde(rename = "left_finger_1")]
    LeftFinger1,
    #[serde(rename = "left_finger_2")]
    LeftFinger2,
    #[serde(rename = "left_finger_3")]
    LeftFinger3,
    #[serde(rename = "left_finger_4")]
    LeftFinger4,
    #[serde(rename = "right_finger_1")]
    RightFinger1,
    #[serde(rename = "right_finger_2")]
    RightFinger2,
    #[serde(rename = "right_finger_3")]
    RightFinger3,
    #[serde(rename = "right_finger_4")]
    RightFinger4,
    #[serde(rename = "belt_1")]
    Belt1,
    #[serde(rename = "belt_2")]
    Belt2,
    #[serde(rename = "belt_3")]
    Belt3,
    #[serde(rename = "belt_4")]
    Belt4,
    BeltBack,
    #[serde(rename = "sack_item_1")]
    SackItem1,
    #[serde(rename = "sack_item_2")]
    SackItem2,
    #[serde(rename = "sack_item_3")]
    SackItem3,
    #[serde(rename = "sack_item_4")]
    SackItem4,
    #[serde(rename = "sack_item_5")]
    SackItem5,
    #[serde(rename = "sack_item_6")]
    SackItem6,
    #[serde(rename = "sack_item_7")]
    SackItem7,
    #[serde(rename = "sack_item_8")]
    SackItem8,
    #[serde(rename = "sack_item_9")]
    SackItem9,
    #[serde(rename = "sack_item_10")]
    SackItem10,
    #[serde(rename = "sack_item_11")]
    SackItem11,
    #[serde(rename = "sack_item_12")]
    SackItem12,
    #[serde(rename = "sack_item_13")]
    SackItem13,
    #[serde(rename = "sack_item_14")]
    SackItem14,
    #[serde(rename = "sack_item_15")]
    SackItem15,
    #[serde(rename = "sack_item_16")]
    SackItem16,
    #[serde(rename = "sack_item_17")]
    SackItem17,
    #[serde(rename = "sack_item_18")]
    SackItem18,
    #[serde(rename = "sack_item_19")]
    SackItem19,
    #[serde(rename = "sack_item_20")]
    SackItem20,
    Head,
    Neck,
    LeftArm,
    RightArm,
    Gloves,
    InnerArmor,
    OuterArmor,
    Boots,
}

impl CarriedPosition {
    pub const ALL: [Self; 43] = [
        Self::LeftHand,
        Self::RightHand,
        Self::LeftFinger1,
        Self::LeftFinger2,
        Self::LeftFinger3,
        Self::LeftFinger4,
        Self::RightFinger1,
        Self::RightFinger2,
        Self::RightFinger3,
        Self::RightFinger4,
        Self::Belt1,
        Self::Belt2,
        Self::Belt3,
        Self::Belt4,
        Self::BeltBack,
        Self::SackItem1,
        Self::SackItem2,
        Self::SackItem3,
        Self::SackItem4,
        Self::SackItem5,
        Self::SackItem6,
        Self::SackItem7,
        Self::SackItem8,
        Self::SackItem9,
        Self::SackItem10,
        Self::SackItem11,
        Self::SackItem12,
        Self::SackItem13,
        Self::SackItem14,
        Self::SackItem15,
        Self::SackItem16,
        Self::SackItem17,
        Self::SackItem18,
        Self::SackItem19,
        Self::SackItem20,
        Self::Head,
        Self::Neck,
        Self::LeftArm,
        Self::RightArm,
        Self::Gloves,
        Self::InnerArmor,
        Self::OuterArmor,
        Self::Boots,
    ];

    pub fn all() -> &'static [Self] {
        &Self::ALL
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::LeftHand => "left_hand",
            Self::RightHand => "right_hand",
            Self::LeftFinger1 => "left_finger_1",
            Self::LeftFinger2 => "left_finger_2",
            Self::LeftFinger3 => "left_finger_3",
            Self::LeftFinger4 => "left_finger_4",
            Self::RightFinger1 => "right_finger_1",
            Self::RightFinger2 => "right_finger_2",
            Self::RightFinger3 => "right_finger_3",
            Self::RightFinger4 => "right_finger_4",
            Self::Belt1 => "belt_1",
            Self::Belt2 => "belt_2",
            Self::Belt3 => "belt_3",
            Self::Belt4 => "belt_4",
            Self::BeltBack => "belt_back",
            Self::SackItem1 => "sack_item_1",
            Self::SackItem2 => "sack_item_2",
            Self::SackItem3 => "sack_item_3",
            Self::SackItem4 => "sack_item_4",
            Self::SackItem5 => "sack_item_5",
            Self::SackItem6 => "sack_item_6",
            Self::SackItem7 => "sack_item_7",
            Self::SackItem8 => "sack_item_8",
            Self::SackItem9 => "sack_item_9",
            Self::SackItem10 => "sack_item_10",
            Self::SackItem11 => "sack_item_11",
            Self::SackItem12 => "sack_item_12",
            Self::SackItem13 => "sack_item_13",
            Self::SackItem14 => "sack_item_14",
            Self::SackItem15 => "sack_item_15",
            Self::SackItem16 => "sack_item_16",
            Self::SackItem17 => "sack_item_17",
            Self::SackItem18 => "sack_item_18",
            Self::SackItem19 => "sack_item_19",
            Self::SackItem20 => "sack_item_20",
            Self::Head => "head",
            Self::Neck => "neck",
            Self::LeftArm => "left_arm",
            Self::RightArm => "right_arm",
            Self::Gloves => "gloves",
            Self::InnerArmor => "inner_armor",
            Self::OuterArmor => "outer_armor",
            Self::Boots => "boots",
        }
    }

    pub fn placement_kind(self) -> ItemPlacementKind {
        match self {
            Self::LeftHand | Self::RightHand => ItemPlacementKind::Hand,
            Self::LeftFinger1
            | Self::LeftFinger2
            | Self::LeftFinger3
            | Self::LeftFinger4
            | Self::RightFinger1
            | Self::RightFinger2
            | Self::RightFinger3
            | Self::RightFinger4 => ItemPlacementKind::RingFinger,
            Self::Belt1 | Self::Belt2 | Self::Belt3 | Self::Belt4 => ItemPlacementKind::BeltSide,
            Self::BeltBack => ItemPlacementKind::BeltBack,
            Self::SackItem1
            | Self::SackItem2
            | Self::SackItem3
            | Self::SackItem4
            | Self::SackItem5
            | Self::SackItem6
            | Self::SackItem7
            | Self::SackItem8
            | Self::SackItem9
            | Self::SackItem10
            | Self::SackItem11
            | Self::SackItem12
            | Self::SackItem13
            | Self::SackItem14
            | Self::SackItem15
            | Self::SackItem16
            | Self::SackItem17
            | Self::SackItem18
            | Self::SackItem19
            | Self::SackItem20 => ItemPlacementKind::Sack,
            Self::Head => ItemPlacementKind::Head,
            Self::Neck => ItemPlacementKind::Neck,
            Self::LeftArm | Self::RightArm => ItemPlacementKind::Arm,
            Self::Gloves => ItemPlacementKind::Gloves,
            Self::InnerArmor => ItemPlacementKind::InnerArmor,
            Self::OuterArmor => ItemPlacementKind::OuterArmor,
            Self::Boots => ItemPlacementKind::Boots,
        }
    }

    pub fn is_sack_item(self) -> bool {
        self.placement_kind() == ItemPlacementKind::Sack
    }

    pub fn is_belt(self) -> bool {
        matches!(
            self.placement_kind(),
            ItemPlacementKind::BeltSide | ItemPlacementKind::BeltBack
        )
    }

    pub fn is_active_equipment(self) -> bool {
        !self.is_sack_item() && !self.is_belt()
    }

    pub const fn is_worn(self) -> bool {
        matches!(
            self,
            Self::Head
                | Self::Neck
                | Self::LeftArm
                | Self::RightArm
                | Self::Gloves
                | Self::InnerArmor
                | Self::OuterArmor
                | Self::Boots
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ItemMoveDestination {
    GroundHere,
    Carried { position: CarriedPosition },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "snake_case")]
enum StrictItemMoveDestination {
    GroundHere {},
    Carried { position: CarriedPosition },
}

impl<'de> Deserialize<'de> for ItemMoveDestination {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let strict =
            StrictItemMoveDestination::deserialize(deserializer).map_err(de::Error::custom)?;
        Ok(match strict {
            StrictItemMoveDestination::GroundHere {} => Self::GroundHere,
            StrictItemMoveDestination::Carried { position } => Self::Carried { position },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "snake_case")]
pub enum GoldMoveSource {
    Carried { position: CarriedGoldPosition },
    Ground { gold_pile_id: GoldPileId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GoldMoveDestination {
    Carried { position: CarriedGoldPosition },
    GroundHere,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "snake_case")]
enum StrictGoldMoveDestination {
    Carried { position: CarriedGoldPosition },
    GroundHere {},
}

impl<'de> Deserialize<'de> for GoldMoveDestination {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let strict =
            StrictGoldMoveDestination::deserialize(deserializer).map_err(de::Error::custom)?;
        Ok(match strict {
            StrictGoldMoveDestination::Carried { position } => Self::Carried { position },
            StrictGoldMoveDestination::GroundHere {} => Self::GroundHere,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GoldMoveQuantity {
    All,
    Exact { amount: i64 },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "snake_case")]
enum StrictGoldMoveQuantity {
    All {},
    Exact { amount: i64 },
}

impl<'de> Deserialize<'de> for GoldMoveQuantity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let strict =
            StrictGoldMoveQuantity::deserialize(deserializer).map_err(de::Error::custom)?;
        Ok(match strict {
            StrictGoldMoveQuantity::All {} => Self::All,
            StrictGoldMoveQuantity::Exact { amount } => Self::Exact { amount },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ItemBindingState {
    Unrestricted,
    BindOnFirstCharacterTouch,
    Bound { character_id: CharacterId },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, tag = "state", rename_all = "snake_case")]
enum StrictItemBindingState {
    Unrestricted {},
    BindOnFirstCharacterTouch {},
    Bound { character_id: CharacterId },
}

impl<'de> Deserialize<'de> for ItemBindingState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let strict =
            StrictItemBindingState::deserialize(deserializer).map_err(de::Error::custom)?;
        Ok(match strict {
            StrictItemBindingState::Unrestricted {} => Self::Unrestricted,
            StrictItemBindingState::BindOnFirstCharacterTouch {} => Self::BindOnFirstCharacterTouch,
            StrictItemBindingState::Bound { character_id } => Self::Bound { character_id },
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CarriedLayout {
    pub items: BTreeMap<CarriedPosition, String>,
    pub gold: CarriedGold,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ItemKnowledgeState {
    pub identified: bool,
    pub appraised: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemInstanceState {
    pub definition_id: String,
    pub quantity: u32,
    pub knowledge: ItemKnowledgeState,
    pub binding: ItemBindingState,
    pub bow_readiness: Option<BowReadiness>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemEnchantmentState {
    pub enchantment_instance_id: String,
    pub source: ItemOperationSource,
    pub item_instance_id: String,
    pub combat_add_rating_bonus: i32,
    pub tags: Vec<String>,
    pub remaining_rounds: Option<u32>,
    pub last_ticked_at: LogicalTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemOperationSource {
    Spell {
        spell_id: String,
        actor_id: ActorId,
    },
    Service {
        service_id: String,
        capability_id: String,
    },
}

/// Key-value bonus to an attribute or resource stat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeBonus {
    pub stat: String,
    pub value: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MpRecoveryMultiplier {
    pub numerator: u32,
    pub denominator: u32,
    pub evidence_state: crate::model::MagicRuleEvidenceState,
}

/// Optional item capability fields for data-driven combat, skill, stamina,
/// and spell systems.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemCapability {
    /// Taxonomy category from the parity pack (e.g. "sword", "leather_armor").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taxonomy_id: Option<String>,
    /// Skill tracks this item selects when held for non-weapon training.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub training_focus_for: Option<Vec<String>>,
    /// Character-bound personal Spell Book lanes this item can train/teach.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spell_book_for: Option<Vec<String>>,
    /// Shield block value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_value: Option<i32>,
    /// Attribute bonuses when equipped (e.g. +2 strength).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute_adds: Option<Vec<AttributeBonus>>,
    /// Resource bonuses when equipped (e.g. +10 max_hp).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_adds: Option<Vec<AttributeBonus>>,
    /// Spell lanes this item acts as a focus for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spell_focus_for: Option<Vec<String>>,
    /// Resistance/protection boosts granted while equipped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resistance_boosts: Option<Vec<crate::model::SpellResistanceBoost>>,
    /// Rational multiplier applied to base MP recovery while worn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mp_recovery_multiplier: Option<MpRecoveryMultiplier>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundItem {
    pub item_instance_id: String,
    pub location: WorldPosition,
    pub loot_claim: Option<LootClaim>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carried_positions_have_exact_canonical_order_and_json_labels() {
        let labels = CarriedPosition::all()
            .iter()
            .map(|position| position.label())
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            [
                "left_hand",
                "right_hand",
                "left_finger_1",
                "left_finger_2",
                "left_finger_3",
                "left_finger_4",
                "right_finger_1",
                "right_finger_2",
                "right_finger_3",
                "right_finger_4",
                "belt_1",
                "belt_2",
                "belt_3",
                "belt_4",
                "belt_back",
                "sack_item_1",
                "sack_item_2",
                "sack_item_3",
                "sack_item_4",
                "sack_item_5",
                "sack_item_6",
                "sack_item_7",
                "sack_item_8",
                "sack_item_9",
                "sack_item_10",
                "sack_item_11",
                "sack_item_12",
                "sack_item_13",
                "sack_item_14",
                "sack_item_15",
                "sack_item_16",
                "sack_item_17",
                "sack_item_18",
                "sack_item_19",
                "sack_item_20",
                "head",
                "neck",
                "left_arm",
                "right_arm",
                "gloves",
                "inner_armor",
                "outer_armor",
                "boots",
            ]
        );
        for position in CarriedPosition::all() {
            let json = serde_json::to_string(position).expect("position should serialize");
            assert_eq!(json, format!("\"{}\"", position.label()));
            assert_eq!(
                serde_json::from_str::<CarriedPosition>(&json).expect("position should round trip"),
                *position
            );
        }
    }

    #[test]
    fn every_carried_position_maps_to_its_locked_placement_and_activity_class() {
        let expected = [
            (ItemPlacementKind::Hand, true, false, false),
            (ItemPlacementKind::Hand, true, false, false),
            (ItemPlacementKind::RingFinger, true, false, false),
            (ItemPlacementKind::RingFinger, true, false, false),
            (ItemPlacementKind::RingFinger, true, false, false),
            (ItemPlacementKind::RingFinger, true, false, false),
            (ItemPlacementKind::RingFinger, true, false, false),
            (ItemPlacementKind::RingFinger, true, false, false),
            (ItemPlacementKind::RingFinger, true, false, false),
            (ItemPlacementKind::RingFinger, true, false, false),
            (ItemPlacementKind::BeltSide, false, false, true),
            (ItemPlacementKind::BeltSide, false, false, true),
            (ItemPlacementKind::BeltSide, false, false, true),
            (ItemPlacementKind::BeltSide, false, false, true),
            (ItemPlacementKind::BeltBack, false, false, true),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Sack, false, true, false),
            (ItemPlacementKind::Head, true, false, false),
            (ItemPlacementKind::Neck, true, false, false),
            (ItemPlacementKind::Arm, true, false, false),
            (ItemPlacementKind::Arm, true, false, false),
            (ItemPlacementKind::Gloves, true, false, false),
            (ItemPlacementKind::InnerArmor, true, false, false),
            (ItemPlacementKind::OuterArmor, true, false, false),
            (ItemPlacementKind::Boots, true, false, false),
        ];
        assert_eq!(CarriedPosition::all().len(), expected.len());
        for (position, (placement, active, sack, belt)) in
            CarriedPosition::all().iter().zip(expected)
        {
            assert_eq!(position.placement_kind(), placement, "{}", position.label());
            assert_eq!(
                position.is_active_equipment(),
                active,
                "{}",
                position.label()
            );
            assert_eq!(position.is_sack_item(), sack, "{}", position.label());
            assert_eq!(position.is_belt(), belt, "{}", position.label());
        }
    }

    #[test]
    fn item_move_destinations_have_exact_tagged_json() {
        assert_eq!(
            serde_json::to_value(ItemMoveDestination::GroundHere).expect("ground destination"),
            serde_json::json!({"kind": "ground_here"})
        );
        assert_eq!(
            serde_json::to_value(ItemMoveDestination::Carried {
                position: CarriedPosition::RightHand,
            })
            .expect("carried destination"),
            serde_json::json!({"kind": "carried", "position": "right_hand"})
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemHolderId {
    Character(CharacterId),
    TransientActor(ActorId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemLocation {
    Ground {
        position: WorldPosition,
    },
    Carried {
        holder: ItemHolderId,
        position: CarriedPosition,
    },
    Corpse {
        corpse_id: CorpseId,
        position: CarriedPosition,
    },
    Merchant {
        inventory_id: MerchantInventoryId,
    },
    Locker {
        vault_id: super::LockerVaultId,
        owner_character_id: CharacterId,
    },
    Offered {
        sender_character_id: CharacterId,
        recipient_character_id: CharacterId,
        source_position: CarriedPosition,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MerchantInventoryId {
    pub service_id: String,
    pub capability_id: String,
}

impl MerchantInventoryId {
    pub fn new(service_id: impl Into<String>, capability_id: impl Into<String>) -> Self {
        Self {
            service_id: service_id.into(),
            capability_id: capability_id.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MerchantListingOrigin {
    AuthoredStock,
    PawnPool,
}

impl MerchantListingOrigin {
    pub const fn label(self) -> &'static str {
        match self {
            Self::AuthoredStock => "authored_stock",
            Self::PawnPool => "pawn_pool",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerchantListingState {
    pub item_instance_id: String,
    pub origin: MerchantListingOrigin,
    pub price_gold: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MerchantInventoryState {
    pub listings: Vec<MerchantListingState>,
}
