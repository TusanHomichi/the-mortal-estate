use serde::{Deserialize, Serialize};

use super::CarriedPosition;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalAttackMode {
    Fight,
    Kick,
    Jumpkick,
    Poke,
    Shoot,
    Throw,
}

impl PhysicalAttackMode {
    pub const ALL: [Self; 6] = [
        Self::Fight,
        Self::Kick,
        Self::Jumpkick,
        Self::Poke,
        Self::Shoot,
        Self::Throw,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Fight => "fight",
            Self::Kick => "kick",
            Self::Jumpkick => "jumpkick",
            Self::Poke => "poke",
            Self::Shoot => "shoot",
            Self::Throw => "throw",
        }
    }

    pub const fn is_weapon_authored(self) -> bool {
        matches!(self, Self::Fight | Self::Poke | Self::Shoot | Self::Throw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalDamageKind {
    Cutting,
    Piercing,
    Crushing,
}

impl PhysicalDamageKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cutting => "cutting",
            Self::Piercing => "piercing",
            Self::Crushing => "crushing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WoundState {
    Unhurt,
    Wounded,
    BadlyWounded,
    NearDeath,
    Dead,
}

impl WoundState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unhurt => "unhurt",
            Self::Wounded => "wounded",
            Self::BadlyWounded => "badly_wounded",
            Self::NearDeath => "near_death",
            Self::Dead => "dead",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatRisk {
    Practice,
    LifeAndDeath,
    Overwhelming,
}

impl CombatRisk {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Practice => "practice",
            Self::LifeAndDeath => "life_and_death",
            Self::Overwhelming => "overwhelming",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalAttackOutcome {
    Fumble,
    HandBlocked,
    ArmorBlocked,
    Miss,
    DamagingHit,
    FatalBlow,
}

impl PhysicalAttackOutcome {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fumble => "fumble",
            Self::HandBlocked => "hand_blocked",
            Self::ArmorBlocked => "armor_blocked",
            Self::Miss => "miss",
            Self::DamagingHit => "damaging_hit",
            Self::FatalBlow => "fatal_blow",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArmorProtectionSource {
    pub carried_position: CarriedPosition,
    pub item_instance_id: String,
    pub item_definition_id: String,
    pub block_rating: i32,
    pub encumbrance: i32,
    pub cutting_reduction: i32,
    pub piercing_reduction: i32,
    pub crushing_reduction: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArmorProtectionPlan {
    pub sources: Vec<ArmorProtectionSource>,
    pub block_rating: i32,
    pub encumbrance: i32,
    pub cutting_reduction: i32,
    pub piercing_reduction: i32,
    pub crushing_reduction: i32,
}

impl ArmorProtectionPlan {
    pub const fn reduction_for(&self, kind: PhysicalDamageKind) -> i32 {
        match kind {
            PhysicalDamageKind::Cutting => self.cutting_reduction,
            PhysicalDamageKind::Piercing => self.piercing_reduction,
            PhysicalDamageKind::Crushing => self.crushing_reduction,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalPracticeReceipt {
    pub track_id: String,
    pub mode: PhysicalAttackMode,
    pub outcome: PhysicalAttackOutcome,
    pub risk: CombatRisk,
    pub base_raw_points: u64,
    pub fatal_blow_bonus_raw_points: u64,
    pub total_raw_points: u64,
}
