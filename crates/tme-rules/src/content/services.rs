use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

use crate::model::{ResourceKind, RestorationStatusKind};

use super::{SpellTeachingDef, TransactionDef};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerSalesDef {
    pub pawn_listing_multiplier: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemServiceOperationDef {
    Appraise {},
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainingOfferDef {
    pub track_id: String,
    pub eligible_class_ids: Vec<String>,
    pub minimum_category_level: u8,
    pub maximum_category_level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestorationOperationDef {
    pub transaction: TransactionDef,
    pub outcome: RestorationOutcomeDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RestorationOutcomeDef {
    RestoreResource { resource: ResourceKind },
    CureStatus { status: RestorationStatusKind },
    PriestResurrection,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum StrictRestorationOutcomeDef {
    RestoreResource { resource: ResourceKind },
    CureStatus { status: RestorationStatusKind },
    PriestResurrection {},
}

impl<'de> Deserialize<'de> for RestorationOutcomeDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let strict =
            StrictRestorationOutcomeDef::deserialize(deserializer).map_err(de::Error::custom)?;
        Ok(match strict {
            StrictRestorationOutcomeDef::RestoreResource { resource } => {
                Self::RestoreResource { resource }
            }
            StrictRestorationOutcomeDef::CureStatus { status } => Self::CureStatus { status },
            StrictRestorationOutcomeDef::PriestResurrection {} => Self::PriestResurrection,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceDefinitionDef {
    pub id: String,
    pub name: String,
    pub capabilities: Vec<ServiceCapabilityDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ServiceCapabilityDef {
    SkillTraining {
        id: String,
        offers: Vec<TrainingOfferDef>,
    },
    SkillCritique {
        id: String,
    },
    SpellTeaching {
        id: String,
        training_capability_id: String,
        teachings: Vec<SpellTeachingDef>,
    },
    ClassPromotion {
        id: String,
        transaction: TransactionDef,
    },
    ServiceTransaction {
        id: String,
        transactions: Vec<TransactionDef>,
    },
    Merchant {
        id: String,
        player_sales: Option<PlayerSalesDef>,
    },
    ItemService {
        id: String,
        operations: Vec<ItemServiceOperationDef>,
    },
    Restoration {
        id: String,
        operations: Vec<RestorationOperationDef>,
    },
    Bank {
        id: String,
        bank_id: String,
    },
    Locker {
        id: String,
        vault_id: String,
    },
}
