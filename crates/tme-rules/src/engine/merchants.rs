//! Session-local merchant planning built on the shared transaction coordinator.

use std::collections::HashSet;

use crate::events::Event;
use crate::model::{
    CarriedPosition, ItemBindingState, ItemLocation, ItemOperationSource, ItemPlacementKind,
    ItemServiceCapability, ItemServiceOperation, ItemServiceOperationKind, MerchantCapability,
    MerchantInventoryId, MerchantListingOrigin, MerchantListingState,
};
use crate::view::ActionBlockedReasonV1;

use super::transactions::{
    PlannedCost, PlannedReward, TransactionPlan, TransactionPlanError, TransactionSource,
};
use super::{Engine, StepError};

impl Engine {
    fn reachable_item_service(
        &self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
    ) -> Result<ItemServiceCapability, TransactionPlanError> {
        let actor = self.world.actors.get(actor_index).ok_or_else(|| {
            TransactionPlanError::new(ActionBlockedReasonV1::NoSuchTarget, "unknown actor")
        })?;
        let service = self.service_by_id(service_id).ok_or_else(|| {
            TransactionPlanError::new(ActionBlockedReasonV1::NoService, "service was not found")
        })?;
        if service.position().level != actor.location.level
            || service.position().position != actor.location.position
        {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::ServiceNotHere,
                "service is not at the actor coordinate",
            ));
        }
        let capability = self
            .item_service_capability(service, capability_id)
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::NoService,
                    "item-service capability was not found",
                )
            })?;
        Ok(capability.clone())
    }

    fn reachable_merchant(
        &self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
    ) -> Result<(MerchantCapability, MerchantInventoryId), TransactionPlanError> {
        let actor = self.world.actors.get(actor_index).ok_or_else(|| {
            TransactionPlanError::new(ActionBlockedReasonV1::NoSuchTarget, "unknown actor")
        })?;
        let service = self.service_by_id(service_id).ok_or_else(|| {
            TransactionPlanError::new(
                ActionBlockedReasonV1::NoService,
                format!("service {service_id:?} was not found"),
            )
        })?;
        if service.position().level != actor.location.level
            || service.position().position != actor.location.position
        {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::ServiceNotHere,
                format!("service {service_id:?} is not at the actor coordinate"),
            ));
        }
        let capability = self
            .merchant_capability(service, capability_id)
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::NoService,
                    format!("service {service_id:?} has no merchant capability {capability_id:?}"),
                )
            })?;
        let inventory_id = MerchantInventoryId::new(service_id, capability_id);
        if !self.world.merchant_inventories.contains_key(&inventory_id) {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::NoService,
                "merchant inventory is missing",
            ));
        }
        Ok((capability.clone(), inventory_id))
    }

    pub(super) fn merchant_purchase_plan(
        &self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        item_instance_ids: &[String],
    ) -> Result<TransactionPlan, TransactionPlanError> {
        let (_capability, inventory_id) =
            self.reachable_merchant(actor_index, service_id, capability_id)?;
        let actor = &self.world.actors[actor_index];
        let inventory = &self.world.merchant_inventories[&inventory_id];
        if item_instance_ids.is_empty() {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::NoSuchItem,
                "merchant purchase requires at least one item",
            ));
        }
        let unique = item_instance_ids.iter().collect::<HashSet<_>>();
        if unique.len() != item_instance_ids.len() {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::NoSuchItem,
                "merchant purchase item IDs must be unique",
            ));
        }
        let current_ids = inventory
            .listings
            .iter()
            .map(|listing| listing.item_instance_id.as_str())
            .collect::<Vec<_>>();
        if item_instance_ids.len() > 1
            && item_instance_ids
                .iter()
                .map(String::as_str)
                .ne(current_ids.iter().copied())
        {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::NoSuchItem,
                "multi-item purchase must equal the complete ordered inventory",
            ));
        }

        let mut captured = Vec::with_capacity(item_instance_ids.len());
        let mut total_gold = 0_i64;
        for item_instance_id in item_instance_ids {
            let listing = inventory
                .listings
                .iter()
                .find(|listing| listing.item_instance_id == *item_instance_id)
                .ok_or_else(|| {
                    TransactionPlanError::new(
                        ActionBlockedReasonV1::NoSuchItem,
                        format!("merchant item {item_instance_id:?} was not found"),
                    )
                })?;
            total_gold = total_gold.checked_add(listing.price_gold).ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::InsufficientGold,
                    "merchant purchase total overflow",
                )
            })?;
            captured.push(listing.clone());
        }
        if actor.carried.gold.sack < total_gold {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::InsufficientGold,
                "carried gold cannot cover merchant purchase",
            ));
        }

        let empty_positions = CarriedPosition::all()
            .iter()
            .copied()
            .filter(|position| position.is_sack_item())
            .filter(|position| !actor.carried.items.contains_key(position))
            .collect::<Vec<_>>();
        if empty_positions.len() < captured.len() {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::NoCarriedCapacity,
                "merchant purchase requires more empty sack positions",
            ));
        }

        let holder = actor.item_holder_id();
        let mut rewards = Vec::with_capacity(captured.len());
        for (listing, position) in captured.iter().zip(empty_positions) {
            let instance = self
                .item_instance(&listing.item_instance_id)
                .map_err(|error| {
                    TransactionPlanError::new(ActionBlockedReasonV1::NoSuchItem, error.message())
                })?;
            let definition = self
                .item_definition(&listing.item_instance_id)
                .map_err(|error| {
                    TransactionPlanError::new(
                        ActionBlockedReasonV1::InvalidItemPlacement,
                        error.message(),
                    )
                })?;
            if !definition
                .valid_placements
                .contains(&ItemPlacementKind::Sack)
                || !matches!(instance.binding, ItemBindingState::Unrestricted)
            {
                return Err(TransactionPlanError::new(
                    ActionBlockedReasonV1::InvalidItemPlacement,
                    "merchant item cannot enter the carried sack",
                ));
            }
            rewards.push(PlannedReward::MerchantItem {
                item_instance_id: listing.item_instance_id.clone(),
                expected: ItemLocation::Merchant {
                    inventory_id: inventory_id.clone(),
                },
                destination: ItemLocation::Carried {
                    holder: holder.clone(),
                    position,
                },
                listing_price_gold: listing.price_gold,
            });
        }

        Ok(TransactionPlan {
            actor_id: actor.id.clone(),
            actor_name: actor.name.clone(),
            source: TransactionSource::MerchantPurchase {
                service_id: service_id.to_string(),
                capability_id: capability_id.to_string(),
                item_instance_ids: item_instance_ids.to_vec(),
            },
            costs: vec![PlannedCost::CarriedGold { amount: total_gold }],
            rewards,
            selected_item_instance_id: None,
        })
    }

    pub(super) fn merchant_sale_plan(
        &self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        item_instance_id: &str,
    ) -> Result<TransactionPlan, TransactionPlanError> {
        let (capability, inventory_id) =
            self.reachable_merchant(actor_index, service_id, capability_id)?;
        let policy = capability.player_sales.ok_or_else(|| {
            TransactionPlanError::new(
                ActionBlockedReasonV1::ItemNotSaleable,
                "merchant does not accept player sales",
            )
        })?;
        let actor = &self.world.actors[actor_index];
        let holder = actor.item_holder_id();
        let expected = self.item_location(item_instance_id).map_err(|error| {
            TransactionPlanError::new(ActionBlockedReasonV1::NoSuchItem, error.message())
        })?;
        if !matches!(&expected, ItemLocation::Carried { holder: actual, .. } if actual == &holder) {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::ItemNotSaleable,
                "sale item is not carried by the actor",
            ));
        }
        let instance = self.item_instance(item_instance_id).map_err(|error| {
            TransactionPlanError::new(ActionBlockedReasonV1::NoSuchItem, error.message())
        })?;
        let definition = self.item_definition(item_instance_id).map_err(|error| {
            TransactionPlanError::new(ActionBlockedReasonV1::ItemNotSaleable, error.message())
        })?;
        let unit_value = definition.economy.unit_value_gold.ok_or_else(|| {
            TransactionPlanError::new(
                ActionBlockedReasonV1::ItemNotSaleable,
                "sale item has no authored value",
            )
        })?;
        if !matches!(instance.binding, ItemBindingState::Unrestricted)
            || !definition
                .valid_placements
                .contains(&ItemPlacementKind::Sack)
        {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::ItemNotSaleable,
                "sale item is tied or cannot occupy a sack",
            ));
        }
        let payout_u64 = unit_value
            .checked_mul(u64::from(instance.quantity))
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::ItemNotSaleable,
                    "sale value is zero or overflowed",
                )
            })?;
        let pawn_price_u64 = payout_u64
            .checked_mul(u64::from(policy.pawn_listing_multiplier))
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::ItemNotSaleable,
                    "pawn listing price overflow",
                )
            })?;
        let payout = i64::try_from(payout_u64).map_err(|_| {
            TransactionPlanError::new(
                ActionBlockedReasonV1::ItemNotSaleable,
                "sale value does not fit carried gold",
            )
        })?;
        let pawn_price = i64::try_from(pawn_price_u64).map_err(|_| {
            TransactionPlanError::new(
                ActionBlockedReasonV1::ItemNotSaleable,
                "pawn listing price does not fit carried gold",
            )
        })?;
        if actor.carried.gold.sack.checked_add(payout).is_none() {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::ItemNotSaleable,
                "sale credit would overflow carried gold",
            ));
        }
        let listing = MerchantListingState {
            item_instance_id: item_instance_id.to_string(),
            origin: MerchantListingOrigin::PawnPool,
            price_gold: pawn_price,
        };

        Ok(TransactionPlan {
            actor_id: actor.id.clone(),
            actor_name: actor.name.clone(),
            source: TransactionSource::MerchantSale {
                service_id: service_id.to_string(),
                capability_id: capability_id.to_string(),
                item_instance_id: item_instance_id.to_string(),
            },
            costs: vec![PlannedCost::MerchantItem {
                item_instance_id: item_instance_id.to_string(),
                expected,
                destination: ItemLocation::Merchant {
                    inventory_id: inventory_id.clone(),
                },
                listing,
            }],
            rewards: vec![PlannedReward::CarriedGold { amount: payout }],
            selected_item_instance_id: None,
        })
    }

    pub(super) fn apply_player_merchant_purchase(
        &mut self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        item_instance_ids: &[String],
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let plan =
            self.merchant_purchase_plan(actor_index, service_id, capability_id, item_instance_ids)?;
        let actor_id = plan.actor_id.clone();
        let actor_name = plan.actor_name.clone();
        let mut receipt = self.commit_transaction(actor_index, plan)?;
        events.append(&mut receipt.delegated_events);
        events.push(receipt.committed_event(actor_id, actor_name));
        Ok(())
    }

    pub(super) fn apply_player_merchant_sale(
        &mut self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        item_instance_id: &str,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let plan =
            self.merchant_sale_plan(actor_index, service_id, capability_id, item_instance_id)?;
        let actor_id = plan.actor_id.clone();
        let actor_name = plan.actor_name.clone();
        let mut receipt = self.commit_transaction(actor_index, plan)?;
        events.append(&mut receipt.delegated_events);
        events.push(receipt.committed_event(actor_id, actor_name));
        Ok(())
    }

    pub(super) fn item_service_plan(
        &self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        operation_kind: ItemServiceOperationKind,
        item_instance_id: &str,
    ) -> Result<TransactionPlan, TransactionPlanError> {
        let capability = self.reachable_item_service(actor_index, service_id, capability_id)?;
        let operation = capability
            .operations
            .iter()
            .find(|operation| operation.kind() == operation_kind)
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::UnsupportedItemService,
                    "item-service operation is not offered",
                )
            })?;
        let actor = &self.world.actors[actor_index];
        let holder = actor.item_holder_id();
        let location = self.item_location(item_instance_id).map_err(|error| {
            TransactionPlanError::new(ActionBlockedReasonV1::NoSuchItem, error.message())
        })?;
        let ItemLocation::Carried {
            holder: actual_holder,
            position,
        } = &location
        else {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::NoSuchItem,
                "item-service target is not carried",
            ));
        };
        if actual_holder != &holder {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::NoSuchItem,
                "item-service target belongs to another actor",
            ));
        }
        let instance = self.item_instance(item_instance_id).map_err(|error| {
            TransactionPlanError::new(ActionBlockedReasonV1::NoSuchItem, error.message())
        })?;
        let definition = self.item_definition(item_instance_id).map_err(|error| {
            TransactionPlanError::new(ActionBlockedReasonV1::InvalidTarget, error.message())
        })?;
        let source = ItemOperationSource::Service {
            service_id: service_id.to_string(),
            capability_id: capability_id.to_string(),
        };
        let (gold_cost, reward) = match operation {
            ItemServiceOperation::Appraise => {
                if instance.knowledge.appraised {
                    return Err(TransactionPlanError::new(
                        ActionBlockedReasonV1::AlreadyComplete,
                        "item is already appraised",
                    ));
                }
                let unit_value_gold = definition.economy.unit_value_gold.ok_or_else(|| {
                    TransactionPlanError::new(
                        ActionBlockedReasonV1::InvalidTarget,
                        "item has no authored value",
                    )
                })?;
                let total_value_gold = unit_value_gold
                    .checked_mul(u64::from(instance.quantity))
                    .ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidTarget,
                            "appraised stack value overflow",
                        )
                    })?;
                (
                    0,
                    PlannedReward::ItemAppraisal {
                        item_instance_id: item_instance_id.to_string(),
                        source,
                        unit_value_gold,
                        total_value_gold,
                    },
                )
            }
            ItemServiceOperation::Identify { gold_cost } => {
                if instance.knowledge.identified {
                    return Err(TransactionPlanError::new(
                        ActionBlockedReasonV1::AlreadyComplete,
                        "item is already identified",
                    ));
                }
                (
                    *gold_cost,
                    PlannedReward::ItemIdentification {
                        item_instance_id: item_instance_id.to_string(),
                        source,
                        location: position.label().to_string(),
                    },
                )
            }
            ItemServiceOperation::EnchantWeapon {
                gold_cost,
                combat_add_rating_bonus,
                tags,
                remaining_rounds,
            } => {
                if definition.weapon.is_none() {
                    return Err(TransactionPlanError::new(
                        ActionBlockedReasonV1::InvalidTarget,
                        "enchantment target is not a weapon",
                    ));
                }
                (
                    *gold_cost,
                    PlannedReward::ItemEnchantment {
                        item_instance_id: item_instance_id.to_string(),
                        source,
                        enchantment_instance_id: format!(
                            "service:{service_id}:{capability_id}:{}:{item_instance_id}",
                            self.current_time()
                        ),
                        combat_add_rating_bonus: *combat_add_rating_bonus,
                        tags: tags.clone(),
                        remaining_rounds: *remaining_rounds,
                    },
                )
            }
        };
        if actor.carried.gold.sack < gold_cost {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::InsufficientGold,
                "carried gold cannot cover item service",
            ));
        }
        let costs = (gold_cost > 0)
            .then_some(PlannedCost::CarriedGold { amount: gold_cost })
            .into_iter()
            .collect();
        Ok(TransactionPlan {
            actor_id: actor.id.clone(),
            actor_name: actor.name.clone(),
            source: TransactionSource::ItemService {
                service_id: service_id.to_string(),
                capability_id: capability_id.to_string(),
                operation: operation_kind,
                item_instance_id: item_instance_id.to_string(),
            },
            costs,
            rewards: vec![reward],
            selected_item_instance_id: None,
        })
    }

    pub(super) fn apply_player_item_service(
        &mut self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        operation: ItemServiceOperationKind,
        item_instance_id: &str,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let plan = self.item_service_plan(
            actor_index,
            service_id,
            capability_id,
            operation,
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
