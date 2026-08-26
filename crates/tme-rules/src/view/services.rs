use serde::{Deserialize, Deserializer, Serialize};

use crate::model::WorldPosition;

use super::{
    ActionOptionV1, ItemInstanceViewV1, ServiceTransactionViewV1, TransactionCostViewV1,
    TransactionRequirementViewV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MerchantListingOriginViewV1 {
    AuthoredStock,
    PawnPool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MerchantListingViewV1 {
    pub item: ItemInstanceViewV1,
    pub origin: MerchantListingOriginViewV1,
    pub price_gold: i64,
    pub purchase: ActionOptionV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ItemServiceOperationViewV1 {
    pub operation: crate::model::ItemServiceOperationKind,
    pub actions: Vec<ActionOptionV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "snake_case")]
pub enum RestorationOutcomeViewV1 {
    RestoreResource {
        resource: crate::model::ResourceKind,
    },
    CureStatus {
        status: crate::model::RestorationStatusKind,
    },
    PriestResurrection,
}

impl From<&crate::model::RestorationOutcome> for RestorationOutcomeViewV1 {
    fn from(value: &crate::model::RestorationOutcome) -> Self {
        match value {
            crate::model::RestorationOutcome::RestoreResource { resource } => {
                Self::RestoreResource {
                    resource: *resource,
                }
            }
            crate::model::RestorationOutcome::CureStatus { status } => {
                Self::CureStatus { status: *status }
            }
            crate::model::RestorationOutcome::PriestResurrection => Self::PriestResurrection,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RestorationOperationViewV1 {
    pub operation_id: String,
    pub label: String,
    pub requirements: Vec<TransactionRequirementViewV1>,
    pub costs: Vec<TransactionCostViewV1>,
    pub outcome: RestorationOutcomeViewV1,
    pub actions: Vec<ActionOptionV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ServiceViewV1 {
    pub service_id: String,
    pub name: String,
    pub position: WorldPosition,
    pub capabilities: Vec<ServiceCapabilityViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "snake_case")]
pub enum ServiceCapabilityViewV1 {
    SkillTraining {
        capability_id: String,
        offered_track_ids: Vec<String>,
        #[serde(deserialize_with = "deserialize_required_nullable_string")]
        selected_track_id: Option<String>,
        actions: Vec<ActionOptionV1>,
    },
    SkillCritique {
        capability_id: String,
        actions: Vec<ActionOptionV1>,
    },
    SpellTeaching {
        capability_id: String,
        spell_ids: Vec<String>,
        actions: Vec<ActionOptionV1>,
    },
    ClassPromotion {
        capability_id: String,
        target_class_id: String,
        actions: Vec<ActionOptionV1>,
    },
    ServiceTransaction {
        capability_id: String,
        transactions: Vec<ServiceTransactionViewV1>,
    },
    Merchant {
        capability_id: String,
        listings: Vec<MerchantListingViewV1>,
        buy_all: ActionOptionV1,
        sales: Vec<ActionOptionV1>,
    },
    ItemService {
        capability_id: String,
        operations: Vec<ItemServiceOperationViewV1>,
    },
    Restoration {
        capability_id: String,
        operations: Vec<RestorationOperationViewV1>,
    },
    Bank {
        capability_id: String,
        bank_id: String,
        balance_gold: i64,
        transaction_cap_gold: i64,
        deposit_actions: Vec<ActionOptionV1>,
        withdrawal_actions: Vec<ActionOptionV1>,
    },
    Locker {
        capability_id: String,
        vault_id: String,
        capacity: u32,
        item_count: u32,
        items: Vec<ItemInstanceViewV1>,
        deposit_actions: Vec<ActionOptionV1>,
        withdrawal_actions: Vec<ActionOptionV1>,
    },
}

fn deserialize_required_nullable_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}
