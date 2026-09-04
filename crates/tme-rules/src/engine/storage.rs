//! Session-local gold movement, stable bank/locker ownership, and reserved offers.

use crate::events::{Event, ItemOfferCompletionReasonV1, ItemRelocationReason};
use crate::model::{
    BankId, CarriedPosition, CharacterId, GoldPileId, ItemHolderId, ItemLocation, LockerVaultId,
    ResolvedService,
};
use crate::view::ActionBlockedReasonV1;

use super::transactions::{
    PlannedCost, PlannedReward, TransactionPlan, TransactionPlanError, TransactionSource,
};
use super::{Engine, StepError};

#[derive(Debug, Clone, PartialEq, Eq)]
struct BankAccess {
    bank_id: BankId,
    transaction_cap_gold: i64,
    character_id: CharacterId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LockerAccess {
    vault_id: LockerVaultId,
    character_id: CharacterId,
}

impl Engine {
    fn reachable_service(
        &self,
        actor_index: usize,
        service_id: &str,
    ) -> Result<ResolvedService<'_>, TransactionPlanError> {
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
        Ok(service)
    }

    fn actor_character_id(&self, actor_index: usize) -> Result<CharacterId, TransactionPlanError> {
        self.world
            .actors
            .get(actor_index)
            .and_then(|actor| actor.character_id.clone())
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::NoSuchTarget,
                    "storage access requires stable character identity",
                )
            })
    }

    fn reachable_bank(
        &self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
    ) -> Result<BankAccess, TransactionPlanError> {
        let service = self.reachable_service(actor_index, service_id)?;
        let capability = self
            .bank_capability(service, capability_id)
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::NoService,
                    "bank capability was not found",
                )
            })?;
        self.world.banks.get(&capability.bank_id).ok_or_else(|| {
            TransactionPlanError::new(ActionBlockedReasonV1::NoService, "bank state is missing")
        })?;
        let bank = self
            .definition
            .catalog
            .bank_definitions
            .get(&capability.bank_id)
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::NoService,
                    "bank definition is missing",
                )
            })?;
        Ok(BankAccess {
            bank_id: capability.bank_id.clone(),
            transaction_cap_gold: bank.transaction_cap_gold,
            character_id: self.actor_character_id(actor_index)?,
        })
    }

    fn reachable_locker(
        &self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
    ) -> Result<LockerAccess, TransactionPlanError> {
        let service = self.reachable_service(actor_index, service_id)?;
        let capability = self
            .locker_capability(service, capability_id)
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::NoService,
                    "locker capability was not found",
                )
            })?;
        if !self.world.locker_vaults.contains_key(&capability.vault_id) {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::NoService,
                "locker vault state is missing",
            ));
        }
        Ok(LockerAccess {
            vault_id: capability.vault_id.clone(),
            character_id: self.actor_character_id(actor_index)?,
        })
    }

    pub(super) fn validate_bank_deposit(
        &self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        gold_pile_id: &GoldPileId,
    ) -> Result<(), TransactionPlanError> {
        let access = self.reachable_bank(actor_index, service_id, capability_id)?;
        let actor = &self.world.actors[actor_index];
        let pile = self.world.ground_gold.get(gold_pile_id).ok_or_else(|| {
            TransactionPlanError::new(
                ActionBlockedReasonV1::NoSuchGold,
                "ground gold pile was not found",
            )
        })?;
        if pile.location.level != actor.location.level
            || pile.location.position != actor.location.position
        {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::NoSuchGold,
                "ground gold pile is not at the actor coordinate",
            ));
        }
        if pile.amount <= 0 || pile.amount > access.transaction_cap_gold {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::BankTransactionLimit,
                "deposit exceeds the bank transaction limit",
            ));
        }
        let balance = self.world.banks[&access.bank_id].balance(&access.character_id);
        balance.checked_add(pile.amount).ok_or_else(|| {
            TransactionPlanError::new(
                ActionBlockedReasonV1::InvalidGoldAmount,
                "bank balance overflow",
            )
        })?;
        Ok(())
    }

    pub(super) fn apply_bank_deposit(
        &mut self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        gold_pile_id: &GoldPileId,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let plan = self.bank_deposit_plan(actor_index, service_id, capability_id, gold_pile_id)?;
        let actor_id = plan.actor_id.clone();
        let actor_name = plan.actor_name.clone();
        let mut receipt = self.commit_transaction(actor_index, plan)?;
        events.append(&mut receipt.delegated_events);
        events.push(receipt.committed_event(actor_id, actor_name));
        Ok(())
    }

    fn bank_deposit_plan(
        &self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        gold_pile_id: &GoldPileId,
    ) -> Result<TransactionPlan, TransactionPlanError> {
        self.validate_bank_deposit(actor_index, service_id, capability_id, gold_pile_id)?;
        let access = self.reachable_bank(actor_index, service_id, capability_id)?;
        let actor = &self.world.actors[actor_index];
        let pile = self.world.ground_gold[gold_pile_id].clone();
        Ok(TransactionPlan {
            actor_id: actor.id.clone(),
            actor_name: actor.name.clone(),
            source: TransactionSource::BankDeposit {
                service_id: service_id.to_string(),
                capability_id: capability_id.to_string(),
                bank_id: access.bank_id.clone(),
                gold_pile_id: pile.id.clone(),
            },
            costs: vec![PlannedCost::GroundGoldPile {
                gold_pile_id: pile.id,
                amount: pile.amount,
                bank_id: access.bank_id.clone(),
                character_id: access.character_id.clone(),
            }],
            rewards: vec![PlannedReward::BankBalance {
                bank_id: access.bank_id,
                character_id: access.character_id,
                amount: pile.amount,
            }],
            selected_item_instance_id: None,
        })
    }

    pub(super) fn validate_bank_withdrawal(
        &self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        amount: i64,
    ) -> Result<(), TransactionPlanError> {
        let access = self.reachable_bank(actor_index, service_id, capability_id)?;
        if amount <= 0 || amount > access.transaction_cap_gold {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::BankTransactionLimit,
                "withdrawal exceeds the bank transaction limit",
            ));
        }
        if self.world.banks[&access.bank_id].balance(&access.character_id) < amount {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::InsufficientGold,
                "bank balance cannot cover withdrawal",
            ));
        }
        Ok(())
    }

    pub(super) fn apply_bank_withdrawal(
        &mut self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        amount: i64,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let plan = self.bank_withdrawal_plan(actor_index, service_id, capability_id, amount)?;
        let actor_id = plan.actor_id.clone();
        let actor_name = plan.actor_name.clone();
        let mut receipt = self.commit_transaction(actor_index, plan)?;
        events.append(&mut receipt.delegated_events);
        events.push(receipt.committed_event(actor_id, actor_name));
        Ok(())
    }

    fn bank_withdrawal_plan(
        &self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        amount: i64,
    ) -> Result<TransactionPlan, TransactionPlanError> {
        self.validate_bank_withdrawal(actor_index, service_id, capability_id, amount)?;
        let access = self.reachable_bank(actor_index, service_id, capability_id)?;
        let actor = &self.world.actors[actor_index];
        Ok(TransactionPlan {
            actor_id: actor.id.clone(),
            actor_name: actor.name.clone(),
            source: TransactionSource::BankWithdrawal {
                service_id: service_id.to_string(),
                capability_id: capability_id.to_string(),
                bank_id: access.bank_id.clone(),
                amount,
            },
            costs: vec![PlannedCost::BankBalance {
                bank_id: access.bank_id.clone(),
                character_id: access.character_id.clone(),
                amount,
            }],
            rewards: vec![PlannedReward::GroundGoldPile {
                bank_id: access.bank_id,
                character_id: access.character_id,
                amount,
                location: actor.location.clone(),
            }],
            selected_item_instance_id: None,
        })
    }

    pub(super) fn validate_locker_deposit(
        &self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        item_instance_id: &str,
    ) -> Result<(), TransactionPlanError> {
        let access = self.reachable_locker(actor_index, service_id, capability_id)?;
        let holder = ItemHolderId::Character(access.character_id.clone());
        if !matches!(
            self.item_location(item_instance_id),
            Ok(ItemLocation::Carried { holder: actual, .. }) if actual == holder
        ) {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::NoSuchItem,
                "locker deposit item is not carried by the character",
            ));
        }
        let vault = &self.world.locker_vaults[&access.vault_id];
        let definition = &self.definition.catalog.locker_vault_definitions[&access.vault_id];
        let count = u32::try_from(vault.contents(&access.character_id).len()).map_err(|_| {
            TransactionPlanError::new(ActionBlockedReasonV1::LockerFull, "locker count overflow")
        })?;
        if count >= definition.capacity {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::LockerFull,
                "locker is full",
            ));
        }
        Ok(())
    }

    pub(super) fn apply_locker_deposit(
        &mut self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        item_instance_id: &str,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        self.validate_locker_deposit(actor_index, service_id, capability_id, item_instance_id)?;
        let access = self.reachable_locker(actor_index, service_id, capability_id)?;
        let source = self.item_location(item_instance_id)?;
        self.relocate_item_with_event(
            actor_index,
            item_instance_id,
            source,
            ItemLocation::Locker {
                vault_id: access.vault_id,
                owner_character_id: access.character_id,
            },
            ItemRelocationReason::LockerDeposit,
            events,
        )
    }

    pub(super) fn validate_locker_withdrawal(
        &self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        item_instance_id: &str,
        destination: CarriedPosition,
    ) -> Result<(), TransactionPlanError> {
        let access = self.reachable_locker(actor_index, service_id, capability_id)?;
        let expected = ItemLocation::Locker {
            vault_id: access.vault_id.clone(),
            owner_character_id: access.character_id.clone(),
        };
        if self.item_location(item_instance_id).ok().as_ref() != Some(&expected) {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::NoSuchItem,
                "item is not in the character locker",
            ));
        }
        if self
            .item_at_position(actor_index, destination)
            .map_err(|error| {
                TransactionPlanError::new(ActionBlockedReasonV1::NoSuchItem, error.message())
            })?
            .is_some()
        {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::OccupiedCarriedPosition,
                "locker withdrawal destination is occupied",
            ));
        }
        self.validate_carried_placement(
            item_instance_id,
            actor_index,
            destination,
            &self.world.item_instances,
        )
        .map_err(|error| {
            TransactionPlanError::new(ActionBlockedReasonV1::InvalidItemPlacement, error.message())
        })?;
        Ok(())
    }

    pub(super) fn apply_locker_withdrawal(
        &mut self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        item_instance_id: &str,
        destination: CarriedPosition,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        self.validate_locker_withdrawal(
            actor_index,
            service_id,
            capability_id,
            item_instance_id,
            destination,
        )?;
        let access = self.reachable_locker(actor_index, service_id, capability_id)?;
        self.relocate_item_with_event(
            actor_index,
            item_instance_id,
            ItemLocation::Locker {
                vault_id: access.vault_id,
                owner_character_id: access.character_id,
            },
            ItemLocation::Carried {
                holder: self.item_holder_for_actor_index(actor_index)?,
                position: destination,
            },
            ItemRelocationReason::LockerWithdrawal,
            events,
        )
    }

    fn actor_index_for_character_id(&self, character_id: &CharacterId) -> Result<usize, StepError> {
        self.actor_index_for_item_holder(&ItemHolderId::Character(character_id.clone()))
    }

    fn offer_item_details(&self, item_instance_id: &str) -> Result<(String, String), StepError> {
        let instance = self.item_instance(item_instance_id)?;
        Ok((
            instance.definition_id.clone(),
            self.item_definition(item_instance_id)?.name.clone(),
        ))
    }

    fn offer_actor_index(&self, character_id: &CharacterId) -> Result<usize, TransactionPlanError> {
        self.actor_index_for_character_id(character_id)
            .map_err(|error| {
                TransactionPlanError::new(ActionBlockedReasonV1::NoSuchTarget, error.message())
            })
    }

    pub(super) fn validate_item_offer(
        &self,
        actor_index: usize,
        recipient_character_id: &CharacterId,
        item_instance_id: &str,
    ) -> Result<(), TransactionPlanError> {
        let sender = self.world.actors.get(actor_index).ok_or_else(|| {
            TransactionPlanError::new(ActionBlockedReasonV1::NoSuchTarget, "unknown actor")
        })?;
        let sender_character_id = sender.character_id.as_ref().ok_or_else(|| {
            TransactionPlanError::new(
                ActionBlockedReasonV1::NoSuchTarget,
                "item offer requires stable character identity",
            )
        })?;
        if sender_character_id == recipient_character_id {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::InvalidTarget,
                "item offer recipient must differ from sender",
            ));
        }
        let recipient_index = self.offer_actor_index(recipient_character_id)?;
        let recipient = &self.world.actors[recipient_index];
        if sender.kind != crate::model::ActorKind::Player
            || recipient.kind != crate::model::ActorKind::Player
            || !sender.is_alive()
            || !recipient.is_alive()
        {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::InvalidTarget,
                "item offer parties must be living player characters",
            ));
        }
        if sender.location.level != recipient.location.level
            || sender.location.position != recipient.location.position
        {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::TargetOutOfRange,
                "item offer parties must share one coordinate",
            ));
        }
        match self.item_location(item_instance_id) {
            Ok(ItemLocation::Carried {
                holder: ItemHolderId::Character(owner),
                position: CarriedPosition::LeftHand | CarriedPosition::RightHand,
            }) if &owner == sender_character_id => Ok(()),
            Ok(_) => Err(TransactionPlanError::new(
                ActionBlockedReasonV1::InvalidItemPlacement,
                "item offer source must be a sender hand",
            )),
            Err(error) => Err(TransactionPlanError::new(
                ActionBlockedReasonV1::NoSuchItem,
                error.message(),
            )),
        }
    }

    pub(super) fn validate_accept_item_offer(
        &self,
        actor_index: usize,
        item_instance_id: &str,
        destination: CarriedPosition,
    ) -> Result<(), TransactionPlanError> {
        let offer = self
            .world
            .item_offers
            .get(item_instance_id)
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::NoSuchTransaction,
                    "item offer is not pending",
                )
            })?;
        let recipient_character_id = self.actor_character_id(actor_index)?;
        if recipient_character_id != offer.recipient_character_id {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::InvalidTarget,
                "only the offer recipient may accept",
            ));
        }
        let sender_index = self.offer_actor_index(&offer.sender_character_id)?;
        let sender = &self.world.actors[sender_index];
        let recipient = &self.world.actors[actor_index];
        if !sender.is_alive() || !recipient.is_alive() {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::ActorNotLiving,
                "item offer parties must be living",
            ));
        }
        if sender.location.level != recipient.location.level
            || sender.location.position != recipient.location.position
        {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::TargetOutOfRange,
                "item offer parties are separated",
            ));
        }
        if self
            .item_at_position(actor_index, destination)
            .map_err(|error| {
                TransactionPlanError::new(ActionBlockedReasonV1::NoSuchItem, error.message())
            })?
            .is_some()
        {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::OccupiedCarriedPosition,
                "item offer destination is occupied",
            ));
        }
        self.validate_carried_placement(
            item_instance_id,
            actor_index,
            destination,
            &self.world.item_instances,
        )
        .map_err(|error| {
            TransactionPlanError::new(ActionBlockedReasonV1::InvalidItemPlacement, error.message())
        })
    }

    pub(super) fn validate_refuse_item_offer(
        &self,
        actor_index: usize,
        item_instance_id: &str,
    ) -> Result<(), TransactionPlanError> {
        let offer = self
            .world
            .item_offers
            .get(item_instance_id)
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::NoSuchTransaction,
                    "item offer is not pending",
                )
            })?;
        if self.actor_character_id(actor_index)? != offer.recipient_character_id {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::InvalidTarget,
                "only the offer recipient may refuse",
            ));
        }
        Ok(())
    }

    pub(super) fn validate_withdraw_item_offer(
        &self,
        actor_index: usize,
        item_instance_id: &str,
    ) -> Result<(), TransactionPlanError> {
        let offer = self
            .world
            .item_offers
            .get(item_instance_id)
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::NoSuchTransaction,
                    "item offer is not pending",
                )
            })?;
        if self.actor_character_id(actor_index)? != offer.sender_character_id {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::InvalidTarget,
                "only the offer sender may withdraw",
            ));
        }
        Ok(())
    }

    pub(super) fn apply_item_offer(
        &mut self,
        actor_index: usize,
        recipient_character_id: &CharacterId,
        item_instance_id: &str,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        self.validate_item_offer(actor_index, recipient_character_id, item_instance_id)?;
        let sender = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("unknown actor"))?;
        let sender_character_id = sender
            .character_id
            .clone()
            .ok_or_else(|| StepError::new("item offer requires stable character identity"))?;
        if sender_character_id == *recipient_character_id {
            return Err(StepError::new(
                "item offer recipient must differ from sender",
            ));
        }
        let recipient_index = self.actor_index_for_character_id(recipient_character_id)?;
        let recipient = &self.world.actors[recipient_index];
        if sender.kind != crate::model::ActorKind::Player
            || recipient.kind != crate::model::ActorKind::Player
            || !sender.is_alive()
            || !recipient.is_alive()
        {
            return Err(StepError::new(
                "item offer parties must be living player characters",
            ));
        }
        if sender.location.level != recipient.location.level
            || sender.location.position != recipient.location.position
        {
            return Err(StepError::new(
                "item offer parties must share one coordinate",
            ));
        }
        let source = self.item_location(item_instance_id)?;
        let source_position = match &source {
            ItemLocation::Carried {
                holder: ItemHolderId::Character(owner),
                position: position @ (CarriedPosition::LeftHand | CarriedPosition::RightHand),
            } if owner == &sender_character_id => *position,
            _ => return Err(StepError::new("item offer source must be a sender hand")),
        };
        let sender_id = sender.id.clone();
        let sender_name = sender.name.clone();
        let (item_definition_id, item_name) = self.offer_item_details(item_instance_id)?;
        self.relocate_item_with_event(
            actor_index,
            item_instance_id,
            source,
            ItemLocation::Offered {
                sender_character_id: sender_character_id.clone(),
                recipient_character_id: recipient_character_id.clone(),
                source_position,
            },
            ItemRelocationReason::OfferCreated,
            events,
        )?;
        events.push(Event::ItemOfferCreated {
            actor_id: sender_id,
            actor: sender_name,
            item_instance_id: item_instance_id.to_string(),
            item_definition_id,
            item: item_name,
            sender_character_id,
            recipient_character_id: recipient_character_id.clone(),
            source_position,
        });
        Ok(())
    }

    fn complete_item_offer(
        &mut self,
        actor_index: usize,
        item_instance_id: &str,
        destination_actor_index: usize,
        destination: CarriedPosition,
        reason: ItemOfferCompletionReasonV1,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let offer = self
            .world
            .item_offers
            .get(item_instance_id)
            .cloned()
            .ok_or_else(|| StepError::new("item offer is not pending"))?;
        let expected = ItemLocation::Offered {
            sender_character_id: offer.sender_character_id.clone(),
            recipient_character_id: offer.recipient_character_id.clone(),
            source_position: offer.source_position,
        };
        let destination_holder = self.item_holder_for_actor_index(destination_actor_index)?;
        let (item_definition_id, item_name) = self.offer_item_details(item_instance_id)?;
        let actor = &self.world.actors[actor_index];
        let actor_id = actor.id.clone();
        let actor_name = actor.name.clone();
        self.relocate_item_with_event(
            actor_index,
            item_instance_id,
            expected,
            ItemLocation::Carried {
                holder: destination_holder,
                position: destination,
            },
            if reason == ItemOfferCompletionReasonV1::Accepted {
                ItemRelocationReason::OfferAccepted
            } else {
                ItemRelocationReason::OfferReturned
            },
            events,
        )?;
        events.push(Event::ItemOfferCompleted {
            actor_id,
            actor: actor_name,
            item_instance_id: item_instance_id.to_string(),
            item_definition_id,
            item: item_name,
            sender_character_id: offer.sender_character_id,
            recipient_character_id: offer.recipient_character_id,
            destination,
            reason,
        });
        Ok(())
    }

    pub(super) fn apply_accept_item_offer(
        &mut self,
        actor_index: usize,
        item_instance_id: &str,
        destination: CarriedPosition,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        self.validate_accept_item_offer(actor_index, item_instance_id, destination)?;
        self.complete_item_offer(
            actor_index,
            item_instance_id,
            actor_index,
            destination,
            ItemOfferCompletionReasonV1::Accepted,
            events,
        )
    }

    pub(super) fn apply_refuse_item_offer(
        &mut self,
        actor_index: usize,
        item_instance_id: &str,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        self.validate_refuse_item_offer(actor_index, item_instance_id)?;
        let offer = self
            .world
            .item_offers
            .get(item_instance_id)
            .cloned()
            .ok_or_else(|| StepError::new("item offer is not pending"))?;
        if self.actor_character_id(actor_index)? != offer.recipient_character_id {
            return Err(StepError::new("only the offer recipient may refuse"));
        }
        let sender_index = self.actor_index_for_character_id(&offer.sender_character_id)?;
        self.complete_item_offer(
            actor_index,
            item_instance_id,
            sender_index,
            offer.source_position,
            ItemOfferCompletionReasonV1::Refused,
            events,
        )
    }

    pub(super) fn apply_withdraw_item_offer(
        &mut self,
        actor_index: usize,
        item_instance_id: &str,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        self.validate_withdraw_item_offer(actor_index, item_instance_id)?;
        let offer = self
            .world
            .item_offers
            .get(item_instance_id)
            .cloned()
            .ok_or_else(|| StepError::new("item offer is not pending"))?;
        if self.actor_character_id(actor_index)? != offer.sender_character_id {
            return Err(StepError::new("only the offer sender may withdraw"));
        }
        self.complete_item_offer(
            actor_index,
            item_instance_id,
            actor_index,
            offer.source_position,
            ItemOfferCompletionReasonV1::Withdrawn,
            events,
        )
    }

    pub(super) fn reconcile_separated_item_offers(
        &mut self,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let offers = self
            .world
            .item_offers
            .iter()
            .map(|(item_id, offer)| (item_id.clone(), offer.clone()))
            .collect::<Vec<_>>();
        let mut separated = Vec::new();
        for (item_id, offer) in offers {
            let sender_index = self.actor_index_for_character_id(&offer.sender_character_id)?;
            let recipient_index =
                self.actor_index_for_character_id(&offer.recipient_character_id)?;
            let sender = &self.world.actors[sender_index];
            let recipient = &self.world.actors[recipient_index];
            if sender.location.level != recipient.location.level
                || sender.location.position != recipient.location.position
            {
                separated.push((item_id, sender_index, offer.source_position));
            }
        }
        for (item_id, sender_index, source_position) in separated {
            self.complete_item_offer(
                sender_index,
                &item_id,
                sender_index,
                source_position,
                ItemOfferCompletionReasonV1::Separated,
                events,
            )?;
        }
        Ok(())
    }

    pub(super) fn unwind_item_offers_for_defeat(
        &mut self,
        actor_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let defeated_character_id = self
            .world
            .actors
            .get(actor_index)
            .and_then(|actor| actor.character_id.clone());
        let Some(defeated_character_id) = defeated_character_id else {
            return Ok(());
        };
        let affected = self
            .world
            .item_offers
            .iter()
            .filter_map(|(item_id, offer)| {
                if offer.sender_character_id == defeated_character_id {
                    Some((
                        item_id.clone(),
                        offer.sender_character_id.clone(),
                        offer.source_position,
                        ItemOfferCompletionReasonV1::SenderDefeated,
                    ))
                } else if offer.recipient_character_id == defeated_character_id {
                    Some((
                        item_id.clone(),
                        offer.sender_character_id.clone(),
                        offer.source_position,
                        ItemOfferCompletionReasonV1::RecipientDefeated,
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for (item_id, sender_character_id, source_position, reason) in affected {
            let sender_index = self.actor_index_for_character_id(&sender_character_id)?;
            self.complete_item_offer(
                actor_index,
                &item_id,
                sender_index,
                source_position,
                reason,
                events,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
