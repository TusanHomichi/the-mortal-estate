use std::fmt;

pub mod actors;
pub mod ai;
pub mod armor;
pub mod boundary;
pub mod catalog;
pub mod characters;
pub mod combat;
pub mod creatures;
pub mod items;
pub mod npcs;
mod professions;
pub mod progression;
pub mod quests;
pub mod services;
pub mod skills;
pub mod social;
pub mod spells;
pub mod storage;
pub mod terrain;
pub mod transactions;
mod validation;
pub(crate) use validation::is_spell_teaching_lane;
pub mod weapons;
pub mod world_seed;
pub mod world_template;

pub use actors::{
    ActiveEffectDef, ActorDeathDef, ActorMagicResistanceDef, ActorSeedDef, CarriedLayoutDef,
    MonsterAbilityDef, MonsterAbilityList, PositionedItemDef, SummonTemplateDef,
};
pub use ai::{ActorAiDef, ActorAwarenessDef};
pub use armor::{ArmorDamageReductionDef, ArmorDef};
pub use boundary::{
    ActiveMpRecoveryItemPolicyDef, BurdenRulesDef, DamageInterruptionComparisonDef,
    MagicArithmeticRoundingDef, MagicCastingPracticeRulesDef, MagicEffectFamilyRulesDef,
    MagicKillExperienceRulesDef, MagicMpRecoveryRulesDef, MagicRewardFractionDef,
    MagicRuleEvidenceStateDef, MagicRulesDef, MagicSaveComparisonDef,
    MatchingResistanceBoostPolicyDef, MovementRulesDef, RaiseDeadRulesDef, ResearchBoundary,
    ResourceRulesDef, RulesDef, SpellDamageInterruptionRulesDef, SpellResistanceRulesDef,
    SpellWarmupRulesDef, ThaumAboveSkillRulesDef,
};
pub use catalog::{
    CATALOG_KIND, CATALOG_SCHEMA_VERSION, CatalogProfileDef, CatalogProfileKey, CatalogRegistryKey,
    CatalogV6, SelectedCatalog,
};
pub use characters::{
    StarterAttributeBoundsDef, StarterAttributeRangeDef, StarterCharacterDef, StarterClassDef,
    StarterCreationDef, StarterEquipmentRowDef, StarterItemRefDef, StarterLoadoutDef,
    StarterNationalityDef, StarterProgressionDef, StarterRuntimeDefaultsDef, StarterSkillRowDef,
};
pub use combat::{
    CombatAttackModeRulesDef, CombatBlockRulesDef, CombatDamageRulesDef, CombatFumbleRulesDef,
    CombatHitRulesDef, CombatJumpkickRulesDef, CombatKickRulesDef, CombatPracticeRulesDef,
    CombatRulesDef, CombatTuningStatusDef, CombatWoundRulesDef,
};
pub use creatures::{
    ActorDefinitionDef, EcologyKindDef, EcologySiteDef, EcologySiteSourceDef, LairDefinitionDef,
    LootChoiceMemberDef, LootEntryDef, LootTableDef, LootTableFamilyDef,
    PhysicalDamageAffinityProfileDef, PhysicalDamageAffinityResponseDef, ScavengingProfileDef,
    SpawnGroupDef, SpawnMemberDef, SpawnResetDef,
};
pub use items::{
    ConsumableDef, DamageLabelDef, GroundItemSeedDef, ItemDef, ItemEconomyDef, ItemInstanceSeedDef,
    ItemKnowledgeDef,
};
pub use npcs::{NpcDef, NpcInteractionDef, NpcInteractionOutcomeDef};
pub use professions::{HideActionDef, MartialHandBlockDef, ProfessionActionDef};
pub use progression::{
    AttributeGrowthBandDef, CombatAddGrowthDef, GrowthAttributeDef, GrowthRuleDef,
    LevelThresholdDef, ProgressionGrowthProfileDef, ProgressionRulesDef, WeightedGrowthOutcomeDef,
};
pub use quests::{QuestDef, QuestStageDef};
pub use services::{
    ItemServiceOperationDef, PlayerSalesDef, RestorationOperationDef, RestorationOutcomeDef,
    ServiceCapabilityDef, ServiceDefinitionDef, TrainingOfferDef,
};
pub use skills::{
    SkillCatalogDef, SkillLadderDef, SkillLevelTitleDef, SkillRulesDef, SkillTrackDef,
    SkillTrackKind, TrainingRulesDef,
};
pub use social::{
    LawZoneDef, SocialAlignmentSourceDef, SocialBehaviorDef, SocialNatureDef,
    SocialOwnerRelationDef, SocialProfileDef, SpellSocialDef, TownLawClassificationDef,
};
pub use spells::{
    SpellAcquisitionDef, SpellAreaDef, SpellBanishDef, SpellCastingDef, SpellCatalogEntryDef,
    SpellDef, SpellDoorControlDef, SpellDurationDef, SpellEffectDef, SpellInstantDeathDef,
    SpellItemUtilityDef, SpellLocateDef, SpellPortalDef, SpellRaiseDeadDef, SpellResistanceDef,
    SpellScryDef, SpellTargetDef, SpellTeachingDef, SpellTerrainOverlayDef, SpellTurnUndeadDef,
};
pub use storage::{BankDef, LockerVaultDef};
pub use terrain::{TerrainDef, TerrainNavigationDef};
pub use transactions::{
    TransactionCostDef, TransactionDef, TransactionRequirementDef, TransactionRewardDef,
};
pub use validation::{
    BannedTerms, ContentBoundaryPolicy, SeedEcologyGroupView, SeedItemValidationView,
    SeedMerchantCapabilityView, SeedPromotionCapabilityView, SeedServiceCapabilityKind,
    SeedSpellTeachingPairView, SeedWorldPositionStatus, TermsError, WorldSeedValidationContext,
    boundary_policy, scan_raw_documents, scan_raw_documents_with,
};
pub use weapons::{BowNockingDef, WeaponAttackModeDef, WeaponDef};
pub use world_seed::{
    MerchantInventorySeedDef, MerchantStockSeedDef, ServiceInstanceSeedDef, WorldSeedDef,
};
pub use world_template::{
    DoorStateDef, LevelDef, PresentationModeDef, RealmDef, SceneRoleDef, StagedViewportDef,
    StaticPropDef, TopologyEdgeDef, TopologyKindDef, TopologyTargetDef, WORLD_TEMPLATE_KIND,
    WORLD_TEMPLATE_SCHEMA_VERSION, WorldTemplateV3, WorldZoomDef,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    messages: Vec<String>,
}

impl ValidationError {
    pub(crate) fn new(messages: Vec<String>) -> Self {
        Self { messages }
    }

    pub fn messages(&self) -> &[String] {
        &self.messages
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.messages.join("; "))
    }
}

impl std::error::Error for ValidationError {}
