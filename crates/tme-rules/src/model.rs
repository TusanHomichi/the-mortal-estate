use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

pub mod ai;
pub mod character;
pub mod combat;
pub mod death;
pub mod items;
pub mod magic;
pub mod npcs;
pub mod progression;
pub mod quests;
pub mod services;
pub mod social;
pub mod storage;
pub mod timing;
pub mod transactions;
pub mod weapons;

pub use ai::{
    ActorAiBehavior, ActorAiState, ActorAwarenessPolicy, ActorAwarenessState, RememberedHostile,
};
pub use character::{
    CharacterAlignment, CharacterAlignmentState, CharacterAttributes, CharacterIdentity,
    CharacterProgression, CharacterResources, CharacterSheetV1, KnownSpell, MAX_CRITIQUE_RANK,
    MAX_SKILL_LEVEL, PhysicalAttributeAdds, PromotionEntry, SkillEntry,
};
pub use combat::{
    ArmorProtectionPlan, ArmorProtectionSource, CombatRisk, PhysicalAttackMode,
    PhysicalAttackOutcome, PhysicalDamageKind, PhysicalPracticeReceipt, WoundState,
};
pub use death::{
    ActorLifeState, CorpseDisposition, CorpseId, CorpseState, DeathCause, GoldPileId,
    GroundGoldPile, LootClaim, LootClaimBasis, LootOwnerId, ResurrectionMethod,
    ResurrectionRequest,
};

pub use items::{
    AttributeBonus, CarriedGold, CarriedGoldPosition, CarriedLayout, CarriedPosition,
    GoldMoveDestination, GoldMoveQuantity, GoldMoveSource, GroundItem, ItemBindingState,
    ItemCapability, ItemEnchantmentState, ItemHolderId, ItemInstanceState, ItemKnowledgeState,
    ItemLocation, ItemMoveDestination, ItemOperationSource, ItemPlacementKind, MerchantInventoryId,
    MerchantInventoryState, MerchantListingOrigin, MerchantListingState, MpRecoveryMultiplier,
    SpellItemLocation,
};
pub use magic::{
    ActiveMpRecoveryItemPolicy, DamageInterruptionComparison, MagicArithmeticRounding,
    MagicCastingPracticeRules, MagicEffectFamilyRules, MagicKillExperienceRules,
    MagicMpRecoveryRules, MagicPracticeReceipt, MagicPrimaryAttribute, MagicRewardFraction,
    MagicRuleEvidenceState, MagicRules, MagicSaveComparison, MatchingResistanceBoostPolicy,
    MonsterAbilityKind, MonsterAbilityState, MonsterAbilityTargetPolicy, RaiseDeadRules,
    ResistanceBoostSourceKind, SpellCastClass, SpellCastingMethod, SpellCatalogEntry,
    SpellCatalogState, SpellDamageCredit, SpellDamageInterruptionRules, SpellDamageRewardClass,
    SpellDamageRounding, SpellDurationPolicy, SpellEffectFamily, SpellResistanceBoost,
    SpellResistanceMitigation, SpellResistanceMitigationMode, SpellResistanceRules, SpellTarget,
    SpellTargetKind, SpellWarmupRules, ThaumAboveSkillPlan, ThaumAboveSkillReceipt,
    ThaumAboveSkillRules, WarmedSpellState, WarmedSpellStatus,
};
pub use npcs::{NpcInteraction, NpcInteractionOutcome, NpcState};
pub use progression::{
    AttributeGrowthBand, CombatAddGrowth, GrowthAttribute, GrowthRule, LevelThreshold,
    ProgressionGrowthProfile, ProgressionRules, WeightedGrowthOutcome,
};
pub use quests::{QuestDefinition, QuestId, QuestStage, QuestStageId, QuestStateLedger};
pub use services::{
    BankCapability, ClassPromotionCapability, ItemServiceCapability, ItemServiceOperation,
    ItemServiceOperationKind, LockerCapability, MerchantCapability, PlayerSalesPolicy,
    ResolvedService, RestorationCapability, RestorationOperation, RestorationOutcome,
    RestorationStatusKind, ServiceCapability, ServiceDefinition, ServiceInstanceState,
    ServiceTransactionCapability, SkillCritiqueCapability, SkillTrainingCapability, SpellTeaching,
    SpellTeachingCapability, TrainingOffer,
};
pub use social::{
    AccountMarkAssessment, AccountMarkAssessmentReason, AlignmentConsequenceReason,
    AttackRelationPlan, AttackSafety, AttackSafetyAssessment, CharacterPresenceState,
    CommunicationPreferences, DefeatContributionKey, DefeatContributionLedger, DefeatRewardClass,
    DefeatRewardUnitContribution, DefeatRewardUnitId, DurableGameplayEffectV1,
    GROUP_DISCONNECT_GRACE_UNITS, GROUP_INVITATION_LIFETIME_UNITS, GroupId, GroupInvitationState,
    GroupInviteId, GroupMemberState, GroupMembershipKey, GroupState, HostileEffectAuthority,
    HostilityAssessment, HostilityAuthorization, HostilityReason, LawZone,
    LethalSocialConsequencePlan, LinkedPlayerKillKarmaV1, MAX_BLOCKED_CHARACTERS,
    MAX_GROUP_MEMBERS, MAX_INCOMING_GROUP_INVITATIONS, MAX_OUTGOING_GROUP_INVITATIONS,
    NpcGrudgeRelation, NpcGrudgeRelationPlan, PerceivedSocialIdentity, PlayerKillAssessmentV1,
    PlayerKillConsequenceV1, SelfDefenseRelationPlan, SelfDefenseRightV1, SelfDefenseRights,
    SocialAlignmentSource, SocialBehavior, SocialBroadcastScope, SocialContactKind, SocialIntent,
    SocialNature, SocialOwnerRelation, SocialProfile, SocialRelationLedger, SpellSocialProfile,
    TownLawClassification, TownLawConsequencePlan,
};
pub use storage::{
    BankDefinition, BankId, BankState, ItemOfferState, LockerVaultDefinition, LockerVaultId,
    LockerVaultState,
};
pub use timing::{ActionCost, ActorTimingState, LogicalTime, WorldTimingState};
pub use transactions::{Transaction, TransactionCost, TransactionRequirement, TransactionReward};
pub use weapons::{
    BlockSourceKind, BowReadiness, BowReadinessChangeReason, WeaponFumbleReason,
    WeaponFumbleResult, WeaponHandedness,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActorId(String);

impl ActorId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ActorId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for ActorId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Deref for ActorId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for ActorId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<String> for ActorId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for ActorId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl PartialEq<str> for ActorId {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for ActorId {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for ActorId {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<ActorId> for str {
    fn eq(&self, other: &ActorId) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<ActorId> for &str {
    fn eq(&self, other: &ActorId) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<ActorId> for String {
    fn eq(&self, other: &ActorId) -> bool {
        self.as_str() == other.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CharacterId(String);

impl CharacterId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coord {
    pub x: i32,
    pub y: i32,
}

impl From<(i32, i32)> for Coord {
    fn from(value: (i32, i32)) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}

impl Coord {
    pub fn step(self, direction: Direction) -> Self {
        let (dx, dy) = direction.delta();
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    pub fn manhattan_distance(self, other: Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    pub fn chebyshev_distance(self, other: Self) -> i32 {
        (self.x - other.x).abs().max((self.y - other.y).abs())
    }
}

/// A named immutable world site.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSite {
    pub realm: String,
    pub level: String,
}

impl WorldSite {
    pub fn new(realm: impl Into<String>, level: impl Into<String>) -> Self {
        Self {
            realm: realm.into(),
            level: level.into(),
        }
    }

    pub fn label(&self) -> String {
        format!("{}/{}", self.realm, self.level)
    }
}

/// A coordinate within one named realm and level.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPosition {
    pub realm: String,
    pub level: String,
    pub position: Coord,
}

impl WorldPosition {
    pub fn new(realm: impl Into<String>, level: impl Into<String>, position: Coord) -> Self {
        Self {
            realm: realm.into(),
            level: level.into(),
            position,
        }
    }

    pub fn site(&self) -> WorldSite {
        WorldSite::new(&self.realm, &self.level)
    }

    pub fn same_site(&self, other: &Self) -> bool {
        self.realm == other.realm && self.level == other.level
    }

    pub fn label(&self) -> String {
        format!(
            "{}/{}:{},{}",
            self.realm, self.level, self.position.x, self.position.y
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    North,
    Northeast,
    East,
    Southeast,
    South,
    Southwest,
    West,
    Northwest,
}

impl Direction {
    pub fn all() -> [Self; 8] {
        [
            Self::North,
            Self::Northeast,
            Self::East,
            Self::Southeast,
            Self::South,
            Self::Southwest,
            Self::West,
            Self::Northwest,
        ]
    }

    pub fn delta(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::Northeast => (1, -1),
            Self::East => (1, 0),
            Self::Southeast => (1, 1),
            Self::South => (0, 1),
            Self::Southwest => (-1, 1),
            Self::West => (-1, 0),
            Self::Northwest => (-1, -1),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::North => "north",
            Self::Northeast => "northeast",
            Self::East => "east",
            Self::Southeast => "southeast",
            Self::South => "south",
            Self::Southwest => "southwest",
            Self::West => "west",
            Self::Northwest => "northwest",
        }
    }
}

pub const MAX_CONTROLLED_PATH_STEPS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementPace {
    Walk,
    Run,
    Sprint,
}

impl MovementPace {
    pub const fn from_step_count(step_count: usize) -> Option<Self> {
        match step_count {
            1 => Some(Self::Walk),
            2 => Some(Self::Run),
            3 => Some(Self::Sprint),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Walk => "walk",
            Self::Run => "run",
            Self::Sprint => "sprint",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BurdenTier {
    LightlyLoaded,
    ModeratelyLoaded,
    HeavilyLoaded,
    VeryHeavilyLoaded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementExertion {
    None,
    Normal,
    Rapid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementStopReason {
    FullPathAccepted,
    Blocked,
    Transitioned,
    ZeroStaminaLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Hp,
    Mp,
    Stamina,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceActivity {
    Active,
    Inactive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stats {
    pub hp: i32,
    pub attack: i32,
    pub defense: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorKind {
    Player,
    Monster,
    Npc,
}

impl ActorKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Monster => "monster",
            Self::Npc => "npc",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerticalDirection {
    Up,
    Down,
}

impl VerticalDirection {
    pub fn label(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplicitTraversalKind {
    StairsUp,
    StairsDown,
    ClimbUp,
    ClimbDown,
}

impl ExplicitTraversalKind {
    pub const fn direction(self) -> VerticalDirection {
        match self {
            Self::StairsUp | Self::ClimbUp => VerticalDirection::Up,
            Self::StairsDown | Self::ClimbDown => VerticalDirection::Down,
        }
    }

    pub const fn navigation_kind(self) -> NavigationKind {
        match self {
            Self::StairsUp => NavigationKind::Stairs {
                direction: VerticalDirection::Up,
            },
            Self::StairsDown => NavigationKind::Stairs {
                direction: VerticalDirection::Down,
            },
            Self::ClimbUp => NavigationKind::Climb {
                direction: VerticalDirection::Up,
            },
            Self::ClimbDown => NavigationKind::Climb {
                direction: VerticalDirection::Down,
            },
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::StairsUp => "stairs_up",
            Self::StairsDown => "stairs_down",
            Self::ClimbUp => "climb_up",
            Self::ClimbDown => "climb_down",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavigationKind {
    Walk,
    Swim,
    Door,
    Stairs { direction: VerticalDirection },
    Pit,
    Climb { direction: VerticalDirection },
    Passage,
    Portal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationDef {
    pub kind: NavigationKind,
    pub target: WorldPosition,
    pub initial_state: Option<DoorState>,
    pub hidden: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorState {
    Open,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummonedActorState {
    pub instance_id: ActorId,
    pub owner_id: ActorId,
    pub source_spell_id: String,
    pub template_id: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_rounds: Option<u32>,
    pub last_ticked_at: LogicalTime,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActorResourceActivity {
    pub last_active_at: Option<LogicalTime>,
    pub last_recovered_at: LogicalTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorState {
    pub id: ActorId,
    pub definition_id: String,
    pub kind: ActorKind,
    pub creature_traits: Vec<CreatureTrait>,
    pub social: SocialProfile,
    pub name: String,
    pub location: WorldPosition,
    pub home_location: WorldPosition,
    pub stats: Stats,
    pub magic_resistance: ActorMagicResistanceState,
    pub physical_damage_affinity_profile_id: String,
    pub physical_damage_affinity: PhysicalDamageAffinity,
    pub hp: i32,
    pub mp: i32,
    pub stamina: i32,
    pub life_state: ActorLifeState,
    pub corpse_disposition: CorpseDisposition,
    pub resource_activity: ActorResourceActivity,
    pub timing: ActorTimingState,
    pub attack_ready_at: LogicalTime,
    pub carried: CarriedLayout,
    pub ai: Option<ActorAiState>,
    pub npc: Option<NpcState>,
    pub xp_value: i32,
    pub character_id: Option<CharacterId>,
    pub character: Option<CharacterSheetV1>,
    pub active_effects: Vec<ActiveEffectState>,
    pub balm_effect: Option<BalmEffectState>,
    pub warmed_spell: Option<WarmedSpellState>,
    pub monster_abilities: Vec<MonsterAbilityState>,
    pub summoned: Option<SummonedActorState>,
    pub ecology_origin: Option<EcologyActorOrigin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorMagicResistanceState {
    pub natural_save_twentieths: u32,
    pub evidence_state: MagicRuleEvidenceState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalDamageAffinity {
    pub cutting_numerator: u32,
    pub cutting_denominator: u32,
    pub piercing_numerator: u32,
    pub piercing_denominator: u32,
    pub crushing_numerator: u32,
    pub crushing_denominator: u32,
}

impl PhysicalDamageAffinity {
    pub const fn response(self, kind: PhysicalDamageKind) -> (u32, u32) {
        match kind {
            PhysicalDamageKind::Cutting => (self.cutting_numerator, self.cutting_denominator),
            PhysicalDamageKind::Piercing => (self.piercing_numerator, self.piercing_denominator),
            PhysicalDamageKind::Crushing => (self.crushing_numerator, self.crushing_denominator),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorDefinition {
    pub id: String,
    pub kind: ActorKind,
    pub name: String,
    pub creature_traits: Vec<CreatureTrait>,
    pub social: SocialProfile,
    pub stats: Stats,
    pub magic_resistance: ActorMagicResistanceState,
    pub corpse_disposition: CorpseDisposition,
    pub ai: Option<ActorAiState>,
    pub xp_value: i32,
    pub physical_damage_affinity_profile_id: String,
    pub physical_damage_affinity: PhysicalDamageAffinity,
    pub scavenging_profile: Option<crate::content::ScavengingProfileDef>,
    pub monster_abilities: Vec<MonsterAbilityState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcologyActorOrigin {
    pub site_id: String,
    pub member_id: String,
    pub generation: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcologyMemberSlotState {
    pub member_id: String,
    pub location: WorldPosition,
    pub actor_id: Option<ActorId>,
    pub due_at: Option<LogicalTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcologySiteState {
    pub id: String,
    pub spawn_group_id: String,
    pub generation: u32,
    pub member_slots: std::collections::BTreeMap<String, EcologyMemberSlotState>,
    pub full_clear_due_at: Option<LogicalTime>,
}

impl ActorState {
    pub fn is_alive(&self) -> bool {
        self.life_state == ActorLifeState::Alive
    }

    pub fn item_holder_id(&self) -> ItemHolderId {
        self.character_id
            .clone()
            .map(ItemHolderId::Character)
            .unwrap_or_else(|| ItemHolderId::TransientActor(self.id.clone()))
    }

    /// The effective maximum HP for this actor.
    ///
    /// For character-backed players this is `CharacterResources.max_hp`.
    /// For non-character actors falls back to `stats.hp`.
    pub fn max_hp(&self) -> i32 {
        self.character
            .as_ref()
            .map(|c| c.resources.max_hp)
            .unwrap_or(self.stats.hp)
    }

    /// The effective maximum stamina for this actor.
    ///
    /// For character-backed actors this is `CharacterResources.max_stamina`.
    /// For non-character actors falls back to 10.
    pub fn max_stamina(&self) -> i32 {
        self.character
            .as_ref()
            .map(|c| c.resources.max_stamina)
            .unwrap_or(10)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainState {
    pub id: String,
    pub name: String,
    pub passable: bool,
    pub move_cost: Option<i32>,
    pub blocks_sight: bool,
    pub traversal: Option<TerrainTraversal>,
    pub unresolved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainTraversal {
    Walk,
    Swim,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ActiveEffectSource {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveEffectStackingPolicy {
    ReplaceSameKind,
    StackInstance,
    RefreshDuration,
}

impl ActiveEffectStackingPolicy {
    pub fn label(self) -> &'static str {
        match self {
            Self::ReplaceSameKind => "replace_same_kind",
            Self::StackInstance => "stack_instance",
            Self::RefreshDuration => "refresh_duration",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveEffectState {
    pub instance_id: String,
    pub effect_id: String,
    pub source: ActiveEffectSource,
    pub source_actor_id: Option<ActorId>,
    pub hostile_authority: Option<HostileEffectAuthority>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spell_damage_credit: Option<SpellDamageCredit>,
    pub kind: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub potency: i32,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_rounds: Option<u32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until_condition: Option<String>,
    pub stacking: ActiveEffectStackingPolicy,
    #[serde(default)]
    pub start_delay_rounds: u32,
    #[serde(default = "default_tick_interval_rounds")]
    pub tick_interval_rounds: u32,
    #[serde(default)]
    pub suppresses_action: bool,
    #[serde(default)]
    pub resistance_boosts: Vec<SpellResistanceBoost>,
    #[serde(default)]
    pub last_ticked_at: LogicalTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummonTemplate {
    pub id: String,
    pub actor_definition_id: String,
    pub item_instances: std::collections::BTreeMap<String, ItemInstanceState>,
    pub carried: CarriedLayout,
    pub active_effects: Vec<ActiveEffectState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreatureTrait {
    Demon,
    Phantasm,
    Undead,
}

fn default_tick_interval_rounds() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileEffectState {
    pub instance_id: String,
    pub effect_id: String,
    pub source: ActiveEffectSource,
    pub source_actor_id: Option<ActorId>,
    pub hostile_authority: Option<HostileEffectAuthority>,
    pub location: WorldPosition,
    pub kind: String,
    pub tags: Vec<String>,
    pub potency: i32,
    pub remaining_rounds: Option<u32>,
    pub passability: Option<String>,
    pub sight: Option<String>,
    pub hazard: Option<String>,
    pub move_cost: Option<i32>,
    pub tick_interval_rounds: u32,
    pub last_ticked_at: LogicalTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortalTransitionState {
    pub instance_id: String,
    pub source_spell_id: String,
    pub source_actor_id: ActorId,
    pub location: WorldPosition,
    pub target: WorldPosition,
    pub two_way: bool,
    pub remaining_rounds: Option<u32>,
    pub last_ticked_at: LogicalTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcealedTransitionState {
    pub instance_id: String,
    pub source_spell_id: String,
    pub source_actor_id: ActorId,
    pub location: WorldPosition,
    pub remaining_rounds: u32,
    pub last_ticked_at: LogicalTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfessionActionConfig {
    pub id: String,
    pub class_ids: Vec<String>,
    pub hide: Option<HideActionConfig>,
    pub martial_hand_block: Option<MartialHandBlockConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HideActionConfig {
    pub effect_id: String,
    pub duration_rounds: u32,
    pub requires_cover_or_darkness: bool,
    pub break_on: Vec<HideBreakTrigger>,
    pub disallow_two_handed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HideBreakTrigger {
    Move,
    Attack,
    ActiveItemMove,
    Cast,
    Warm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MartialHandBlockConfig {
    pub min_hand_level: i32,
    pub level_divisor: i32,
    pub max_chance_percent: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovementRules {
    pub controlled_path_points: i32,
    pub automatic_step_points: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BurdenRules {
    pub coin_burden_per_gold: u64,
    pub lightly_loaded_max_per_strength: u64,
    pub moderately_loaded_max_per_strength: u64,
    pub heavily_loaded_max_per_strength: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRules {
    pub recovery_interval_units: u32,
    pub active_hp_recovery: i32,
    pub inactive_hp_recovery: i32,
    pub inactive_stamina_recovery: i32,
    pub mp_recovery: i32,
    pub normal_movement_stamina_cost: i32,
    pub rapid_movement_stamina_cost: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainingRules {
    pub gold_per_learning_rate: i64,
    pub experience_per_learning_rate: i32,
    pub maximum_learning_rates: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillRules {
    pub base_learning_rate: u64,
    pub practice_thresholds: Vec<u64>,
    pub training: TrainingRules,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldRules {
    pub progression: ProgressionRules,
    pub movement: MovementRules,
    pub burden: BurdenRules,
    pub resources: ResourceRules,
    pub magic: MagicRules,
    pub skills: SkillRules,
    pub combat: crate::combat::CombatRules,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelState {
    pub law_zone: LawZone,
    pub scene_role: SceneRole,
    pub presentation_mode: PresentationMode,
    pub world_zoom: [u32; 2],
    pub maximum_clear_sightline: u32,
    pub staged_viewport: Option<crate::content::StagedViewportDef>,
    pub wall_terrain_ids: Vec<String>,
    pub static_props: Vec<crate::content::StaticPropDef>,
    pub width: i32,
    pub height: i32,
    pub cells: Vec<Vec<Vec<Option<String>>>>,
}

mod player_intent;
pub use player_intent::PlayerIntent;
mod runtime_world;
pub use runtime_world::{BalmEffectState, PresentationMode, RealmState, SceneRole, World};
