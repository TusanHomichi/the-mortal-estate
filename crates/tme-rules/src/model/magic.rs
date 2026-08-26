use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

use super::{ActorId, Direction, LogicalTime, SpellItemLocation, WorldPosition};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicRuleEvidenceState {
    OriginalProvisional,
    TargetRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicSaveComparison {
    RollAtOrBelow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchingResistanceBoostPolicy {
    HighestMatching,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageInterruptionComparison {
    StrictlyGreater,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellWarmupRules {
    pub units: u32,
    pub evidence_state: MagicRuleEvidenceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellDamageInterruptionRules {
    pub comparison: DamageInterruptionComparison,
    pub numerator: u32,
    pub denominator: u32,
    pub evidence_state: MagicRuleEvidenceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellResistanceRules {
    pub denominator: u32,
    pub denominator_evidence_state: MagicRuleEvidenceState,
    pub success_comparison: MagicSaveComparison,
    pub matching_boost_policy: MatchingResistanceBoostPolicy,
    pub resolution_evidence_state: MagicRuleEvidenceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagicRules {
    pub warmup: SpellWarmupRules,
    pub damage_interruption: SpellDamageInterruptionRules,
    pub resistance: SpellResistanceRules,
    pub casting_practice: MagicCastingPracticeRules,
    pub thaum_above_skill: ThaumAboveSkillRules,
    pub kill_experience: MagicKillExperienceRules,
    pub mp_recovery: MagicMpRecoveryRules,
    pub effect_families: MagicEffectFamilyRules,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagicEffectFamilyRules {
    pub raise_dead: RaiseDeadRules,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaiseDeadRules {
    pub roll_denominator: u32,
    pub success_threshold_per_magic_level: u32,
    pub minimum_success_threshold: u32,
    pub evidence_state: MagicRuleEvidenceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagicCastingPracticeRules {
    pub minimum_raw_points: u64,
    pub raw_points_per_mp: u64,
    pub primary_attribute_points_per_bonus: u32,
    pub evidence_state: MagicRuleEvidenceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThaumAboveSkillRules {
    pub roll_denominator: u32,
    pub penalty_per_missing_level: u32,
    pub minimum_success_threshold: u32,
    pub evidence_state: MagicRuleEvidenceState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicArithmeticRounding {
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagicRewardFraction {
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagicKillExperienceRules {
    pub directed: MagicRewardFraction,
    pub area_or_illusion: MagicRewardFraction,
    pub fraction_evidence_state: MagicRuleEvidenceState,
    pub rounding: MagicArithmeticRounding,
    pub rounding_evidence_state: MagicRuleEvidenceState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveMpRecoveryItemPolicy {
    HighestMultiplier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagicMpRecoveryRules {
    pub active_item_policy: ActiveMpRecoveryItemPolicy,
    pub rounding: MagicArithmeticRounding,
    pub evidence_state: MagicRuleEvidenceState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicPrimaryAttribute {
    Intelligence,
    Wisdom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagicPracticeReceipt {
    pub current_class_id: String,
    pub spell_id: String,
    pub spell_name: String,
    pub track_id: String,
    pub mp_cost: i32,
    pub cast_class: SpellCastClass,
    pub primary_attribute: Option<MagicPrimaryAttribute>,
    pub primary_attribute_value: Option<i32>,
    pub base_raw_points: u64,
    pub primary_attribute_bonus_raw_points: u64,
    pub total_raw_points: u64,
    pub risk_applied: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThaumAboveSkillPlan {
    pub current_skill_level: u8,
    pub skill_requirement: u8,
    pub gap: u8,
    pub roll_denominator: u32,
    pub success_threshold: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThaumAboveSkillReceipt {
    pub current_skill_level: u8,
    pub skill_requirement: u8,
    pub gap: u8,
    pub roll_denominator: u32,
    pub success_threshold: u32,
    pub roll: u32,
    pub success: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellDamageRewardClass {
    Directed,
    AreaOrIllusion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellDamageCredit {
    pub caster_actor_id: ActorId,
    pub spell_id: String,
    pub reward_class: SpellDamageRewardClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellDamageRounding {
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellResistanceBoost {
    pub tag: String,
    pub bonus_twentieths: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResistanceBoostSourceKind {
    ActiveEffect,
    EquippedItem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum SpellResistanceMitigation {
    Negate,
    HalfDamage {
        rounding: SpellDamageRounding,
        minimum_damage: i32,
    },
    MinimumDamage {
        damage: i32,
    },
}

#[derive(Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
enum StrictSpellResistanceMitigation {
    Negate {},
    HalfDamage {
        rounding: SpellDamageRounding,
        minimum_damage: i32,
    },
    MinimumDamage {
        damage: i32,
    },
}

impl<'de> Deserialize<'de> for SpellResistanceMitigation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let strict = StrictSpellResistanceMitigation::deserialize(deserializer)
            .map_err(de::Error::custom)?;
        Ok(match strict {
            StrictSpellResistanceMitigation::Negate {} => Self::Negate,
            StrictSpellResistanceMitigation::HalfDamage {
                rounding,
                minimum_damage,
            } => Self::HalfDamage {
                rounding,
                minimum_damage,
            },
            StrictSpellResistanceMitigation::MinimumDamage { damage } => {
                Self::MinimumDamage { damage }
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellResistanceMitigationMode {
    Negate,
    HalfDamage,
    MinimumDamage,
}

impl SpellResistanceMitigation {
    pub const fn mode(&self) -> SpellResistanceMitigationMode {
        match self {
            Self::Negate => SpellResistanceMitigationMode::Negate,
            Self::HalfDamage { .. } => SpellResistanceMitigationMode::HalfDamage,
            Self::MinimumDamage { .. } => SpellResistanceMitigationMode::MinimumDamage,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellEffectFamily {
    AttributeBuff,
    Banish,
    Concealment,
    ControlStatus,
    Curse,
    Darkness,
    DirectDamage,
    DoorControl,
    FallProtection,
    Healing,
    InstantDeath,
    ItemEnchant,
    ItemIdentify,
    Light,
    Locate,
    Poison,
    PoisonCure,
    Portal,
    Protection,
    RaiseDead,
    Resistance,
    Scry,
    SecretDetection,
    Speed,
    Summon,
    TerrainOverlay,
    TurnUndead,
    Vision,
    WaterBreathing,
    WeaponEnchant,
}

impl SpellEffectFamily {
    pub const fn label(self) -> &'static str {
        match self {
            Self::AttributeBuff => "attribute_buff",
            Self::Banish => "banish",
            Self::Concealment => "concealment",
            Self::ControlStatus => "control_status",
            Self::Curse => "curse",
            Self::Darkness => "darkness",
            Self::DirectDamage => "direct_damage",
            Self::DoorControl => "door_control",
            Self::FallProtection => "fall_protection",
            Self::Healing => "healing",
            Self::InstantDeath => "instant_death",
            Self::ItemEnchant => "item_enchant",
            Self::ItemIdentify => "item_identify",
            Self::Light => "light",
            Self::Locate => "locate",
            Self::Poison => "poison",
            Self::PoisonCure => "poison_cure",
            Self::Portal => "portal",
            Self::Protection => "protection",
            Self::RaiseDead => "raise_dead",
            Self::Resistance => "resistance",
            Self::Scry => "scry",
            Self::SecretDetection => "secret_detection",
            Self::Speed => "speed",
            Self::Summon => "summon",
            Self::TerrainOverlay => "terrain_overlay",
            Self::TurnUndead => "turn_undead",
            Self::Vision => "vision",
            Self::WaterBreathing => "water_breathing",
            Self::WeaponEnchant => "weapon_enchant",
        }
    }
}

impl std::fmt::Display for SpellEffectFamily {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellTargetKind {
    Actor,
    Area,
    Coordinate,
    Direction,
    Door,
    Item,
    None,
    #[serde(rename = "self")]
    SelfTarget,
}

impl SpellTargetKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Actor => "actor",
            Self::Area => "area",
            Self::Coordinate => "coordinate",
            Self::Direction => "direction",
            Self::Door => "door",
            Self::Item => "item",
            Self::None => "none",
            Self::SelfTarget => "self",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellDurationPolicy {
    Instant,
    Permanent,
    Rounds,
    Unknown,
    UntilDispelled,
    UntilZoneChange,
}

impl SpellDurationPolicy {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Instant => "instant",
            Self::Permanent => "permanent",
            Self::Rounds => "rounds",
            Self::Unknown => "unknown",
            Self::UntilDispelled => "until_dispelled",
            Self::UntilZoneChange => "until_zone_change",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellCastingMethod {
    Direct,
    WarmThenCast,
}

impl SpellCastingMethod {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::WarmThenCast => "warm_then_cast",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellCastClass {
    Character,
    Path,
    PathOrCharacter,
    #[serde(rename = "self")]
    SelfTarget,
    NotApplicable,
}

impl SpellCastClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::Path => "path",
            Self::PathOrCharacter => "path_or_character",
            Self::SelfTarget => "self",
            Self::NotApplicable => "not_applicable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellCatalogState {
    Matched,
    OpenEvidence,
}

impl SpellCatalogState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Matched => "matched",
            Self::OpenEvidence => "open_evidence",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellCatalogEntry {
    pub spell_id: String,
    pub row_id: String,
    pub topic_id: String,
    pub acquisition_row_id: Option<String>,
    pub variant_id: String,
    pub effect_family: SpellEffectFamily,
    pub target_kind: Option<SpellTargetKind>,
    pub state: SpellCatalogState,
    pub open_question_ids: Vec<String>,
    pub resistance_tags: Vec<String>,
    pub resistance_mitigation_mode: Option<SpellResistanceMitigationMode>,
    pub client_row_id: Option<String>,
    pub client_spell_id: Option<u32>,
    pub client_verb_type: Option<u32>,
    pub client_powerable: Option<bool>,
    pub client_spell_poem_id: Option<u32>,
    pub client_offensive: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarmedSpellStatus {
    Warming,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SpellTarget {
    None,
    SelfTarget,
    Actor {
        actor_id: ActorId,
    },
    Coordinate {
        position: WorldPosition,
    },
    Area {
        center: WorldPosition,
    },
    Direction {
        direction: Direction,
    },
    Door {
        direction: Direction,
    },
    Item {
        item_instance_id: String,
        location: SpellItemLocation,
    },
    Path {
        directions: Vec<Direction>,
    },
}

impl SpellTarget {
    pub fn label(&self) -> String {
        match self {
            Self::None => "none".to_string(),
            Self::SelfTarget => "self".to_string(),
            Self::Actor { actor_id } => actor_id.to_string(),
            Self::Coordinate { position } => position.label(),
            Self::Area { center } => format!("area {}", center.label()),
            Self::Direction { direction } => direction.label().to_string(),
            Self::Door { direction } => format!("door {}", direction.label()),
            Self::Item {
                item_instance_id,
                location,
            } => format!("{}:{}", location.label(), item_instance_id),
            Self::Path { directions } => format!(
                "path {}",
                directions
                    .iter()
                    .map(|direction| direction.label())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarmedSpellState {
    pub spell_id: String,
    pub warmed_at: LogicalTime,
    pub ready_at: LogicalTime,
    pub status: WarmedSpellStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterAbilityKind {
    Spell,
    SpecialAttack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterAbilityTargetPolicy {
    NearestHostile,
    SelfTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonsterAbilityState {
    pub id: String,
    pub kind: MonsterAbilityKind,
    pub spell_id: String,
    pub cooldown_rounds: u32,
    pub target_policy: MonsterAbilityTargetPolicy,
    pub ready_at: LogicalTime,
}
