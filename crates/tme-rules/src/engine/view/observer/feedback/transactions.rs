//! Projection of transaction receipts into bounded observer facts.
use super::*;

pub(super) fn observer_transaction_source(
    value: &TransactionSourceV1,
) -> ObserverTransactionSourceV1 {
    match value {
        TransactionSourceV1::SkillTraining {
            service_id,
            capability_id,
            track_id,
        } => ObserverTransactionSourceV1::SkillTraining {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            track_id: track_id.clone(),
        },
        TransactionSourceV1::SpellLearning {
            service_id,
            capability_id,
            spell_id,
        } => ObserverTransactionSourceV1::SpellLearning {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            spell_id: spell_id.clone(),
        },
        TransactionSourceV1::ClassPromotion {
            service_id,
            capability_id,
            transaction_id,
            target_class_id,
        } => ObserverTransactionSourceV1::ClassPromotion {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            transaction_id: transaction_id.clone(),
            target_class_id: target_class_id.clone(),
        },
        TransactionSourceV1::ServiceTransaction {
            service_id,
            capability_id,
            transaction_id,
        } => ObserverTransactionSourceV1::ServiceTransaction {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            transaction_id: transaction_id.clone(),
        },
        TransactionSourceV1::MerchantPurchase {
            service_id,
            capability_id,
            item_instance_ids,
        } => ObserverTransactionSourceV1::MerchantPurchase {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            item_instance_ids: item_instance_ids.clone(),
        },
        TransactionSourceV1::MerchantSale {
            service_id,
            capability_id,
            item_instance_id,
        } => ObserverTransactionSourceV1::MerchantSale {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            item_instance_id: item_instance_id.clone(),
        },
        TransactionSourceV1::ItemService {
            service_id,
            capability_id,
            operation,
            item_instance_id,
        } => ObserverTransactionSourceV1::ItemService {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            operation: *operation,
            item_instance_id: item_instance_id.clone(),
        },
        TransactionSourceV1::RestorationService {
            service_id,
            capability_id,
            operation_id,
            corpse_id,
        } => ObserverTransactionSourceV1::RestorationService {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            operation_id: operation_id.clone(),
            corpse_id: corpse_id.clone(),
        },
        TransactionSourceV1::NpcInteraction {
            npc_actor_id,
            interaction_id,
        } => ObserverTransactionSourceV1::NpcInteraction {
            npc_actor_id: npc_actor_id.clone(),
            interaction_id: interaction_id.clone(),
        },
        TransactionSourceV1::BankDeposit {
            service_id,
            capability_id,
            bank_id,
            gold_pile_id,
        } => ObserverTransactionSourceV1::BankDeposit {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            bank_id: bank_id.clone(),
            gold_pile_id: gold_pile_id.clone(),
        },
        TransactionSourceV1::BankWithdrawal {
            service_id,
            capability_id,
            bank_id,
            amount,
        } => ObserverTransactionSourceV1::BankWithdrawal {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            bank_id: bank_id.clone(),
            amount: *amount,
        },
    }
}

pub(super) fn observer_transaction_cost(
    value: &TransactionCostReceiptV1,
) -> ObserverTransactionCostV1 {
    match value {
        TransactionCostReceiptV1::CarriedGold {
            amount,
            position,
            before,
            after,
        } => ObserverTransactionCostV1::CarriedGold {
            amount: *amount,
            position: *position,
            before: *before,
            after: *after,
        },
        TransactionCostReceiptV1::GroundGoldPile {
            gold_pile_id,
            amount,
            ..
        } => ObserverTransactionCostV1::GroundGoldPile {
            gold_pile_id: gold_pile_id.clone(),
            amount: *amount,
        },
        TransactionCostReceiptV1::BankBalance {
            bank_id,
            amount,
            before,
            after,
            ..
        } => ObserverTransactionCostV1::BankBalance {
            bank_id: bank_id.clone(),
            amount: *amount,
            before: *before,
            after: *after,
        },
        TransactionCostReceiptV1::SelectedCarriedItem {
            item_instance_id,
            item_definition_id,
            consumed_quantity,
            remaining_quantity,
        } => ObserverTransactionCostV1::SelectedCarriedItem {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: item_definition_id.clone(),
            consumed_quantity: *consumed_quantity,
            remaining_quantity: *remaining_quantity,
        },
        TransactionCostReceiptV1::MerchantItem {
            item_instance_id,
            item_definition_id,
            quantity,
            pawn_listing_price_gold,
            ..
        } => ObserverTransactionCostV1::MerchantItem {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: item_definition_id.clone(),
            quantity: *quantity,
            pawn_listing_price_gold: *pawn_listing_price_gold,
        },
    }
}

pub(super) fn observer_transaction_reward(
    value: &TransactionRewardReceiptV1,
) -> ObserverTransactionRewardV1 {
    match value {
        TransactionRewardReceiptV1::LearningRate {
            track_id,
            before,
            after,
        } => ObserverTransactionRewardV1::LearningRate {
            track_id: track_id.clone(),
            before: *before,
            after: *after,
        },
        TransactionRewardReceiptV1::Experience { amount, total_xp } => {
            ObserverTransactionRewardV1::Experience {
                amount: *amount,
                total_xp: *total_xp,
            }
        }
        TransactionRewardReceiptV1::Item {
            item_instance_id,
            item_definition_id,
            position,
            quantity,
        } => ObserverTransactionRewardV1::Item {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: item_definition_id.clone(),
            position: *position,
            quantity: *quantity,
        },
        TransactionRewardReceiptV1::Class {
            from_class_id,
            from_class_display,
            to_class_id,
            to_class_display,
        } => ObserverTransactionRewardV1::Class {
            from_class_id: from_class_id.clone(),
            from_class_display: from_class_display.clone(),
            to_class_id: to_class_id.clone(),
            to_class_display: to_class_display.clone(),
        },
        TransactionRewardReceiptV1::Spell {
            spell_id,
            learned_at_level,
        } => ObserverTransactionRewardV1::Spell {
            spell_id: spell_id.clone(),
            learned_at_level: *learned_at_level,
        },
        TransactionRewardReceiptV1::CarriedGold {
            amount,
            position,
            before,
            after,
        } => ObserverTransactionRewardV1::CarriedGold {
            amount: *amount,
            position: *position,
            before: *before,
            after: *after,
        },
        TransactionRewardReceiptV1::BankBalance {
            bank_id,
            amount,
            before,
            after,
            ..
        } => ObserverTransactionRewardV1::BankBalance {
            bank_id: bank_id.clone(),
            amount: *amount,
            before: *before,
            after: *after,
        },
        TransactionRewardReceiptV1::GroundGoldPile {
            gold_pile_id,
            amount,
            ..
        } => ObserverTransactionRewardV1::GroundGoldPile {
            gold_pile_id: gold_pile_id.clone(),
            amount: *amount,
        },
        TransactionRewardReceiptV1::MerchantItem {
            item_instance_id,
            item_definition_id,
            quantity,
            listing_price_gold,
            ..
        } => ObserverTransactionRewardV1::MerchantItem {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: item_definition_id.clone(),
            quantity: *quantity,
            listing_price_gold: *listing_price_gold,
        },
        TransactionRewardReceiptV1::ItemAppraised {
            item_instance_id,
            item_definition_id,
            unit_value_gold,
            total_value_gold,
        } => ObserverTransactionRewardV1::ItemAppraised {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: item_definition_id.clone(),
            unit_value_gold: *unit_value_gold,
            total_value_gold: *total_value_gold,
        },
        TransactionRewardReceiptV1::ItemIdentified {
            item_instance_id,
            item_definition_id,
        } => ObserverTransactionRewardV1::ItemIdentified {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: item_definition_id.clone(),
        },
        TransactionRewardReceiptV1::ItemEnchanted {
            item_instance_id,
            item_definition_id,
            enchantment_instance_id,
            combat_add_rating_bonus,
            tags,
            remaining_rounds,
        } => ObserverTransactionRewardV1::ItemEnchanted {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: item_definition_id.clone(),
            enchantment_instance_id: enchantment_instance_id.clone(),
            combat_add_rating_bonus: *combat_add_rating_bonus,
            tags: tags.clone(),
            remaining_rounds: *remaining_rounds,
        },
        TransactionRewardReceiptV1::ResourceRestored {
            resource,
            before,
            after,
            maximum,
            ..
        } => ObserverTransactionRewardV1::ResourceRestored {
            resource: *resource,
            before: *before,
            after: *after,
            maximum: *maximum,
        },
        TransactionRewardReceiptV1::StatusCured {
            status,
            removed_count,
            ..
        } => ObserverTransactionRewardV1::StatusCured {
            status: *status,
            removed_count: *removed_count,
        },
        TransactionRewardReceiptV1::PriestResurrection {
            corpse_id,
            method,
            current_hp,
            current_stamina,
            ..
        } => ObserverTransactionRewardV1::PriestResurrection {
            corpse_id: corpse_id.clone(),
            method: *method,
            current_hp: *current_hp,
            current_stamina: *current_stamina,
        },
        TransactionRewardReceiptV1::NpcInteraction {
            npc_actor_id,
            interaction_id,
            outcome,
        } => ObserverTransactionRewardV1::NpcInteraction {
            npc_actor_id: npc_actor_id.clone(),
            interaction_id: interaction_id.clone(),
            outcome: outcome.clone(),
        },
        TransactionRewardReceiptV1::QuestStage {
            quest_id,
            before_stage_id,
            after_stage_id,
            ..
        } => ObserverTransactionRewardV1::QuestStage {
            quest_id: quest_id.clone(),
            before_stage_id: before_stage_id.clone(),
            after_stage_id: after_stage_id.clone(),
        },
    }
}
