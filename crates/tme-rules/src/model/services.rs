use serde::{Deserialize, Serialize};

use super::{Transaction, WorldPosition};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellTeaching {
    pub spell_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainingOffer {
    pub track_id: String,
    pub eligible_class_ids: Vec<String>,
    pub minimum_category_level: u8,
    pub maximum_category_level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillTrainingCapability {
    pub id: String,
    pub offers: Vec<TrainingOffer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillCritiqueCapability {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellTeachingCapability {
    pub id: String,
    pub training_capability_id: String,
    pub teachings: Vec<SpellTeaching>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassPromotionCapability {
    pub id: String,
    pub transaction: Transaction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceTransactionCapability {
    pub id: String,
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSalesPolicy {
    pub pawn_listing_multiplier: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerchantCapability {
    pub id: String,
    pub player_sales: Option<PlayerSalesPolicy>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemServiceOperationKind {
    Appraise,
    Identify,
    EnchantWeapon,
}

impl ItemServiceOperationKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Appraise => "appraise",
            Self::Identify => "identify",
            Self::EnchantWeapon => "enchant_weapon",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemServiceOperation {
    Appraise,
    Identify {
        gold_cost: i64,
    },
    EnchantWeapon {
        gold_cost: i64,
        combat_add_rating_bonus: i32,
        tags: Vec<String>,
        remaining_rounds: Option<u32>,
    },
}

impl ItemServiceOperation {
    pub const fn kind(&self) -> ItemServiceOperationKind {
        match self {
            Self::Appraise => ItemServiceOperationKind::Appraise,
            Self::Identify { .. } => ItemServiceOperationKind::Identify,
            Self::EnchantWeapon { .. } => ItemServiceOperationKind::EnchantWeapon,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemServiceCapability {
    pub id: String,
    pub operations: Vec<ItemServiceOperation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestorationStatusKind {
    Blindness,
    Poison,
}

impl RestorationStatusKind {
    pub const fn effect_tag(self) -> &'static str {
        match self {
            Self::Blindness => "blind",
            Self::Poison => "poison",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Blindness => "blindness",
            Self::Poison => "poison",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestorationOutcome {
    RestoreResource { resource: super::ResourceKind },
    CureStatus { status: RestorationStatusKind },
    PriestResurrection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestorationOperation {
    pub transaction: Transaction,
    pub outcome: RestorationOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestorationCapability {
    pub id: String,
    pub operations: Vec<RestorationOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankCapability {
    pub id: String,
    pub bank_id: super::BankId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockerCapability {
    pub id: String,
    pub vault_id: super::LockerVaultId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceCapability {
    SkillTraining(SkillTrainingCapability),
    SkillCritique(SkillCritiqueCapability),
    SpellTeaching(SpellTeachingCapability),
    ClassPromotion(ClassPromotionCapability),
    ServiceTransaction(ServiceTransactionCapability),
    Merchant(MerchantCapability),
    ItemService(ItemServiceCapability),
    Restoration(RestorationCapability),
    Bank(BankCapability),
    Locker(LockerCapability),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDefinition {
    pub id: String,
    pub name: String,
    pub capabilities: Vec<ServiceCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceInstanceState {
    pub id: String,
    pub definition_id: String,
    pub position: WorldPosition,
}

/// A read-only join of mutable placement state and immutable service policy.
///
/// The joined view deliberately borrows both owners. Service names and
/// capabilities are never copied into mutable [`super::World`] state.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedService<'a> {
    instance: &'a ServiceInstanceState,
    definition: &'a ServiceDefinition,
}

impl<'a> ResolvedService<'a> {
    pub(crate) fn new(
        instance: &'a ServiceInstanceState,
        definition: &'a ServiceDefinition,
    ) -> Self {
        Self {
            instance,
            definition,
        }
    }

    pub fn id(self) -> &'a str {
        &self.instance.id
    }

    pub fn definition_id(self) -> &'a str {
        &self.instance.definition_id
    }

    pub fn name(self) -> &'a str {
        &self.definition.name
    }

    pub fn position(self) -> &'a WorldPosition {
        &self.instance.position
    }

    pub fn capabilities(self) -> &'a [ServiceCapability] {
        &self.definition.capabilities
    }
}
