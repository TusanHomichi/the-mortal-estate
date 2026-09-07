//! Provider-neutral transaction plans and service entry points.

mod commit;
mod plan;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashSet};

use crate::events::{
    Event, TransactionCostReceiptV1, TransactionRewardReceiptV1, TransactionSourceV1,
};
use crate::model::{
    BankId, CarriedPosition, CharacterId, CorpseId, GoldPileId, ItemBindingState,
    ItemInstanceState, ItemKnowledgeState, ItemLocation, ItemOperationSource, MerchantListingState,
    Transaction, TransactionCost, TransactionRequirement, TransactionReward, WorldPosition,
};
use crate::view::ActionBlockedReasonV1;

use super::{Engine, StepError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransactionPlanError {
    reason: ActionBlockedReasonV1,
    message: String,
}

impl TransactionPlanError {
    pub(super) fn new(reason: ActionBlockedReasonV1, message: impl Into<String>) -> Self {
        Self {
            reason,
            message: message.into(),
        }
    }

    pub(super) const fn reason(&self) -> ActionBlockedReasonV1 {
        self.reason
    }

    pub(super) fn message(&self) -> &str {
        &self.message
    }
}

impl From<TransactionPlanError> for StepError {
    fn from(error: TransactionPlanError) -> Self {
        StepError::new(error.message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TransactionSource {
    SkillTraining {
        service_id: String,
        capability_id: String,
        track_id: String,
    },
    SpellLearning {
        service_id: String,
        capability_id: String,
        spell_id: String,
    },
    ClassPromotion {
        service_id: String,
        capability_id: String,
        transaction_id: String,
        target_class_id: String,
    },
    ServiceTransaction {
        service_id: String,
        capability_id: String,
        transaction_id: String,
    },
    MerchantPurchase {
        service_id: String,
        capability_id: String,
        item_instance_ids: Vec<String>,
    },
    MerchantSale {
        service_id: String,
        capability_id: String,
        item_instance_id: String,
    },
    ItemService {
        service_id: String,
        capability_id: String,
        operation: crate::model::ItemServiceOperationKind,
        item_instance_id: String,
    },
    RestorationService {
        service_id: String,
        capability_id: String,
        operation_id: String,
        corpse_id: Option<CorpseId>,
    },
    NpcInteraction {
        npc_actor_id: crate::model::ActorId,
        interaction_id: String,
    },
    BankDeposit {
        service_id: String,
        capability_id: String,
        bank_id: BankId,
        gold_pile_id: GoldPileId,
    },
    BankWithdrawal {
        service_id: String,
        capability_id: String,
        bank_id: BankId,
        amount: i64,
    },
}

impl TransactionSource {
    fn view(&self) -> TransactionSourceV1 {
        match self {
            Self::SkillTraining {
                service_id,
                capability_id,
                track_id,
            } => TransactionSourceV1::SkillTraining {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                track_id: track_id.clone(),
            },
            Self::SpellLearning {
                service_id,
                capability_id,
                spell_id,
            } => TransactionSourceV1::SpellLearning {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                spell_id: spell_id.clone(),
            },
            Self::ClassPromotion {
                service_id,
                capability_id,
                transaction_id,
                target_class_id,
            } => TransactionSourceV1::ClassPromotion {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                transaction_id: transaction_id.clone(),
                target_class_id: target_class_id.clone(),
            },
            Self::ServiceTransaction {
                service_id,
                capability_id,
                transaction_id,
            } => TransactionSourceV1::ServiceTransaction {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                transaction_id: transaction_id.clone(),
            },
            Self::MerchantPurchase {
                service_id,
                capability_id,
                item_instance_ids,
            } => TransactionSourceV1::MerchantPurchase {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                item_instance_ids: item_instance_ids.clone(),
            },
            Self::MerchantSale {
                service_id,
                capability_id,
                item_instance_id,
            } => TransactionSourceV1::MerchantSale {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                item_instance_id: item_instance_id.clone(),
            },
            Self::ItemService {
                service_id,
                capability_id,
                operation,
                item_instance_id,
            } => TransactionSourceV1::ItemService {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                operation: *operation,
                item_instance_id: item_instance_id.clone(),
            },
            Self::RestorationService {
                service_id,
                capability_id,
                operation_id,
                corpse_id,
            } => TransactionSourceV1::RestorationService {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                operation_id: operation_id.clone(),
                corpse_id: corpse_id.clone(),
            },
            Self::NpcInteraction {
                npc_actor_id,
                interaction_id,
            } => TransactionSourceV1::NpcInteraction {
                npc_actor_id: npc_actor_id.clone(),
                interaction_id: interaction_id.clone(),
            },
            Self::BankDeposit {
                service_id,
                capability_id,
                bank_id,
                gold_pile_id,
            } => TransactionSourceV1::BankDeposit {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                bank_id: bank_id.as_str().to_string(),
                gold_pile_id: gold_pile_id.clone(),
            },
            Self::BankWithdrawal {
                service_id,
                capability_id,
                bank_id,
                amount,
            } => TransactionSourceV1::BankWithdrawal {
                service_id: service_id.clone(),
                capability_id: capability_id.clone(),
                bank_id: bank_id.as_str().to_string(),
                amount: *amount,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PlannedCost {
    CarriedGold {
        amount: i64,
    },
    SelectedCarriedItem {
        quantity: u32,
    },
    MerchantItem {
        item_instance_id: String,
        expected: ItemLocation,
        destination: ItemLocation,
        listing: MerchantListingState,
    },
    GroundGoldPile {
        gold_pile_id: GoldPileId,
        amount: i64,
        bank_id: BankId,
        character_id: CharacterId,
    },
    BankBalance {
        bank_id: BankId,
        character_id: CharacterId,
        amount: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PlannedReward {
    LearningRate {
        track_id: String,
        before: u64,
        after: u64,
    },
    Experience {
        amount: i32,
    },
    Item {
        item_instance_id: String,
        item_definition_id: String,
        position: CarriedPosition,
    },
    Class {
        from_class_id: String,
        from_class_display: String,
        to_class_id: String,
        to_class_display: String,
        level: i32,
    },
    Spell {
        spell_id: String,
        lane: String,
        learned_at_level: i32,
    },
    CarriedGold {
        amount: i64,
    },
    MerchantItem {
        item_instance_id: String,
        expected: ItemLocation,
        destination: ItemLocation,
        listing_price_gold: i64,
    },
    ItemAppraisal {
        item_instance_id: String,
        source: ItemOperationSource,
        unit_value_gold: u64,
        total_value_gold: u64,
    },
    ItemIdentification {
        item_instance_id: String,
        source: ItemOperationSource,
        location: String,
    },
    ItemEnchantment {
        item_instance_id: String,
        source: ItemOperationSource,
        enchantment_instance_id: String,
        combat_add_rating_bonus: i32,
        tags: Vec<String>,
        remaining_rounds: Option<u32>,
    },
    Restoration(super::restoration::RestorationRewardPlan),
    NpcInteraction(super::npc_interactions::NpcInteractionRewardPlan),
    QuestStage(super::quests::QuestTransitionPlan),
    BankBalance {
        bank_id: BankId,
        character_id: CharacterId,
        amount: i64,
    },
    GroundGoldPile {
        bank_id: BankId,
        character_id: CharacterId,
        amount: i64,
        location: WorldPosition,
    },
    ReturnedGold {
        amount: i64,
        location: WorldPosition,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransactionPlan {
    pub(super) actor_id: crate::model::ActorId,
    pub(super) actor_name: String,
    pub(super) source: TransactionSource,
    pub(super) costs: Vec<PlannedCost>,
    pub(super) rewards: Vec<PlannedReward>,
    pub(super) selected_item_instance_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransactionCommitReceipt {
    pub(super) source: TransactionSource,
    pub(super) costs: Vec<TransactionCostReceiptV1>,
    pub(super) rewards: Vec<TransactionRewardReceiptV1>,
    pub(super) delegated_events: Vec<Event>,
}

impl TransactionCommitReceipt {
    pub(super) fn committed_event(self, actor_id: crate::model::ActorId, actor: String) -> Event {
        Event::TransactionCommitted {
            actor_id,
            actor,
            source: self.source.view(),
            costs: self.costs,
            rewards: self.rewards,
        }
    }
}

impl Engine {
    pub(super) fn generic_service_transaction_plan(
        &self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        transaction_id: &str,
        selected_item_instance_id: Option<&str>,
    ) -> Result<TransactionPlan, TransactionPlanError> {
        let actor = self.world.actors.get(actor_index).ok_or_else(|| {
            TransactionPlanError::new(ActionBlockedReasonV1::NoSuchTarget, "unknown actor")
        })?;
        let service = self.service_by_id(service_id).ok_or_else(|| {
            TransactionPlanError::new(
                ActionBlockedReasonV1::NoService,
                format!("service {service_id:?} was not found"),
            )
        })?;
        if service.position() != &actor.location {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::ServiceNotHere,
                format!("service {service_id:?} is not at the actor coordinate"),
            ));
        }
        let capability = self
            .service_transaction_capability(service, capability_id)
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::NoService,
                    format!(
                        "service {service_id:?} has no transaction capability {capability_id:?}"
                    ),
                )
            })?;
        let transaction = capability
            .transactions
            .iter()
            .find(|transaction| transaction.id == transaction_id)
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::NoSuchTransaction,
                    format!("transaction {transaction_id:?} was not found"),
                )
            })?;
        let source = TransactionSource::ServiceTransaction {
            service_id: service_id.to_string(),
            capability_id: capability_id.to_string(),
            transaction_id: transaction_id.to_string(),
        };
        self.plan_transaction(
            actor_index,
            source,
            transaction,
            selected_item_instance_id,
            Vec::new(),
        )
    }

    pub(super) fn apply_player_service_transaction(
        &mut self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        transaction_id: &str,
        item_instance_id: Option<&str>,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let plan = self.generic_service_transaction_plan(
            actor_index,
            service_id,
            capability_id,
            transaction_id,
            item_instance_id,
        )?;
        let actor_id = plan.actor_id.clone();
        let actor_name = plan.actor_name.clone();
        let mut receipt = self.commit_transaction(actor_index, plan)?;
        events.append(&mut receipt.delegated_events);
        events.push(receipt.committed_event(actor_id, actor_name));
        Ok(())
    }
}
