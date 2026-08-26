use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponHandedness {
    OneHanded,
    TwoHanded,
    Bow,
}

impl WeaponHandedness {
    pub const fn label(self) -> &'static str {
        match self {
            Self::OneHanded => "one_handed",
            Self::TwoHanded => "two_handed",
            Self::Bow => "bow",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BowReadiness {
    Unnocked,
    Nocked,
}

impl BowReadiness {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unnocked => "unnocked",
            Self::Nocked => "nocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BowReadinessChangeReason {
    Nocked,
    ExplicitUnload,
    Shot,
    Fumble,
    Movement,
    LeftHandOccupied,
    LeftRightHand,
    ItemRelocated,
}

impl BowReadinessChangeReason {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Nocked => "nocked",
            Self::ExplicitUnload => "explicit_unload",
            Self::Shot => "shot",
            Self::Fumble => "fumble",
            Self::Movement => "movement",
            Self::LeftHandOccupied => "left_hand_occupied",
            Self::LeftRightHand => "left_right_hand",
            Self::ItemRelocated => "item_relocated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponFumbleReason {
    General,
    TiedToOtherCharacter,
    AlignmentMismatch,
}

impl WeaponFumbleReason {
    pub const fn label(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::TiedToOtherCharacter => "tied_to_other_character",
            Self::AlignmentMismatch => "alignment_mismatch",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponFumbleResult {
    Dropped,
    BowUnnocked,
}

impl WeaponFumbleResult {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Dropped => "dropped",
            Self::BowUnnocked => "bow_unnocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockSourceKind {
    LeftShield,
    LeftWeapon,
    RightWeapon,
    RightMartialHand,
    Armor,
}

impl BlockSourceKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::LeftShield => "left_shield",
            Self::LeftWeapon => "left_weapon",
            Self::RightWeapon => "right_weapon",
            Self::RightMartialHand => "right_martial_hand",
            Self::Armor => "armor",
        }
    }
}
