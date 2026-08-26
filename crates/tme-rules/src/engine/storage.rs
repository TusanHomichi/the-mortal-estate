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
mod tests {
    use super::super::death::DefeatContext;
    use super::*;
    use crate::events::{BankBalanceChangeReasonV1, TransactionSourceV1};
    use crate::model::{
        ActorTimingState, CarriedGold, DeathCause, GoldMoveDestination, GoldMoveQuantity,
        GoldMoveSource, ItemBindingState, LogicalTime, LootClaim, LootClaimBasis, LootOwnerId,
    };
    use crate::view::{PlayerIntentPayloadV1, ServiceCapabilityViewV1};
    use serde_json::json;

    fn character_id(value: &str) -> CharacterId {
        serde_json::from_value(json!(value)).expect("character ID")
    }

    fn storage_engine() -> Engine {
        use crate::content::{
            BankDef, CatalogRegistryKey, LockerVaultDef, ServiceCapabilityDef,
            ServiceDefinitionDef, ServiceInstanceSeedDef,
        };

        let (mut catalog, profile, template, mut seed) =
            crate::engine::setup::test_parts("gold_training");
        let profile_def = catalog.profiles.get_mut(&profile).expect("test profile");
        for (key, bank) in [
            (
                "bank/test/shared",
                BankDef {
                    id: "shared_bank".to_string(),
                    transaction_cap_gold: 200,
                },
            ),
            (
                "bank/test/isolated",
                BankDef {
                    id: "isolated_bank".to_string(),
                    transaction_cap_gold: 75,
                },
            ),
        ] {
            let key = CatalogRegistryKey::from(key);
            catalog.banks.insert(key.clone(), bank);
            profile_def.banks.push(key);
        }
        for (key, vault) in [
            (
                "vault/test/shared",
                LockerVaultDef {
                    id: "shared_vault".to_string(),
                    capacity: 2,
                },
            ),
            (
                "vault/test/isolated",
                LockerVaultDef {
                    id: "isolated_vault".to_string(),
                    capacity: 1,
                },
            ),
        ] {
            let key = CatalogRegistryKey::from(key);
            catalog.locker_vaults.insert(key.clone(), vault);
            profile_def.locker_vaults.push(key);
        }
        for (suffix, name, bank_id, vault_id) in [
            ("a", "Storage Counter A", "shared_bank", "shared_vault"),
            ("b", "Storage Counter B", "shared_bank", "shared_vault"),
            ("c", "Storage Counter C", "isolated_bank", "isolated_vault"),
        ] {
            let definition_id = format!("storage_counter_{suffix}");
            let key = CatalogRegistryKey::from(format!("service/test/{suffix}"));
            catalog.service_definitions.insert(
                key.clone(),
                ServiceDefinitionDef {
                    id: definition_id.clone(),
                    name: name.to_string(),
                    capabilities: vec![
                        ServiceCapabilityDef::Bank {
                            id: format!("bank_{suffix}"),
                            bank_id: bank_id.to_string(),
                        },
                        ServiceCapabilityDef::Locker {
                            id: format!("locker_{suffix}"),
                            vault_id: vault_id.to_string(),
                        },
                    ],
                },
            );
            profile_def.service_definitions.push(key);
            seed.service_instances.push(ServiceInstanceSeedDef {
                id: definition_id.clone(),
                service_definition_id: definition_id,
                location: crate::model::WorldPosition::new(
                    "realm_0",
                    "room_0",
                    crate::model::Coord { x: 1, y: 1 },
                ),
            });
        }
        crate::engine::setup::test_engine_from_parts(catalog, profile, template, seed)
    }

    fn add_recipient(engine: &mut Engine) -> (usize, CharacterId) {
        let recipient_character_id = character_id("character:storage:recipient");
        let mut recipient = engine.world.actors[0].clone();
        recipient.id = "recipient".into();
        recipient.name = "Recipient".to_string();
        recipient.character_id = Some(recipient_character_id.clone());
        recipient.carried.items.clear();
        recipient.carried.gold = CarriedGold::default();
        recipient.timing = ActorTimingState {
            ready_at: LogicalTime::FIRST,
            tie_break_order: engine.world.timing.next_tie_break_order,
        };
        engine.world.timing.next_tie_break_order += 1;
        engine.world.actors.push(recipient);
        (engine.world.actors.len() - 1, recipient_character_id)
    }

    fn create_offer(engine: &mut Engine) -> (usize, CharacterId, Vec<Event>) {
        let (recipient_index, recipient_character_id) = add_recipient(engine);
        let mut events = Vec::new();
        engine
            .apply_item_offer(0, &recipient_character_id, "training_sword", &mut events)
            .expect("offer creation");
        (recipient_index, recipient_character_id, events)
    }

    #[test]
    fn positioned_gold_split_collect_and_hand_collision_are_atomic() {
        let mut engine = storage_engine();
        let mut events = Vec::new();
        engine
            .apply_player_move_gold(
                0,
                &GoldMoveSource::Carried {
                    position: crate::model::CarriedGoldPosition::Sack,
                },
                &GoldMoveDestination::Carried {
                    position: crate::model::CarriedGoldPosition::LeftHand,
                },
                &GoldMoveQuantity::Exact { amount: 40 },
                &mut events,
            )
            .expect("split into open hand");
        engine
            .apply_player_move_gold(
                0,
                &GoldMoveSource::Carried {
                    position: crate::model::CarriedGoldPosition::LeftHand,
                },
                &GoldMoveDestination::GroundHere,
                &GoldMoveQuantity::Exact { amount: 15 },
                &mut events,
            )
            .expect("split to ground");
        assert_eq!(engine.world.actors[0].carried.gold.left_hand, 25);
        assert_eq!(engine.world.actors[0].carried.gold.sack, 460);
        assert_eq!(engine.world.ground_gold.values().next().unwrap().amount, 15);

        let claim = LootClaim {
            owner: LootOwnerId::Character(
                engine.world.actors[0]
                    .character_id
                    .clone()
                    .expect("stable character"),
            ),
            basis: LootClaimBasis::KillingBlow,
        };
        let claimed = engine
            .create_ground_gold_pile(
                9,
                engine.world.actors[0].location.clone(),
                Some(claim.clone()),
            )
            .expect("claimed pile");
        engine
            .apply_player_move_gold(
                0,
                &GoldMoveSource::Ground {
                    gold_pile_id: claimed.id.clone(),
                },
                &GoldMoveDestination::Carried {
                    position: crate::model::CarriedGoldPosition::LeftHand,
                },
                &GoldMoveQuantity::Exact { amount: 4 },
                &mut events,
            )
            .expect("partial claimed-pile collection");
        assert_eq!(engine.world.ground_gold[&claimed.id].amount, 5);
        assert_eq!(
            engine.world.ground_gold[&claimed.id].loot_claim,
            Some(claim.clone())
        );
        assert!(events.iter().any(|event| matches!(
            event,
            Event::GoldRelocated {
                amount: 4,
                loot_claim: Some(actual),
                ..
            } if actual == &claim
        )));

        let before = engine.world.clone();
        let error = engine
            .apply_player_move_gold(
                0,
                &GoldMoveSource::Carried {
                    position: crate::model::CarriedGoldPosition::Sack,
                },
                &GoldMoveDestination::Carried {
                    position: crate::model::CarriedGoldPosition::RightHand,
                },
                &GoldMoveQuantity::Exact { amount: 1 },
                &mut Vec::new(),
            )
            .expect_err("item and gold cannot share a hand");
        assert!(error.message().contains("occupied by an item"));
        assert_eq!(engine.world, before);
    }

    #[test]
    fn bank_transactions_share_branches_and_emit_atomic_coordinator_receipts() {
        let mut engine = storage_engine();
        let player = engine.world.actors[0].clone();
        let character_id = player.character_id.expect("stable character");
        let pile = engine
            .create_ground_gold_pile(125, player.location.clone(), None)
            .expect("ground pile");
        let mut deposit_events = Vec::new();
        engine
            .apply_bank_deposit(
                0,
                "storage_counter_a",
                "bank_a",
                &pile.id,
                &mut deposit_events,
            )
            .expect("deposit through branch A");
        assert_eq!(
            engine.world.banks[&BankId::new("shared_bank")].balance(&character_id),
            125
        );
        assert!(!engine.world.ground_gold.contains_key(&pile.id));
        assert!(deposit_events.iter().any(|event| matches!(
            event,
            Event::BankBalanceChanged {
                before: 0,
                after: 125,
                reason: BankBalanceChangeReasonV1::Deposit,
                ..
            }
        )));
        assert!(deposit_events.iter().any(|event| matches!(
            event,
            Event::TransactionCommitted {
                source: TransactionSourceV1::BankDeposit { bank_id, .. },
                costs,
                rewards,
                ..
            } if bank_id == "shared_bank" && costs.len() == 1 && rewards.len() == 1
        )));

        let mut withdrawal_events = Vec::new();
        engine
            .apply_bank_withdrawal(0, "storage_counter_b", "bank_b", 40, &mut withdrawal_events)
            .expect("withdrawal through branch B");
        assert_eq!(
            engine.world.banks[&BankId::new("shared_bank")].balance(&character_id),
            85
        );
        assert_eq!(engine.world.ground_gold.values().next().unwrap().amount, 40);
        assert!(withdrawal_events.iter().any(|event| matches!(
            event,
            Event::BankBalanceChanged {
                before: 125,
                after: 85,
                reason: BankBalanceChangeReasonV1::Withdrawal,
                ..
            }
        )));

        let over_cap = engine
            .create_ground_gold_pile(201, player.location.clone(), None)
            .expect("over-cap pile");
        let before = engine.world.clone();
        let error = engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                crate::model::PlayerIntent::DepositBankGold {
                    service_id: "storage_counter_a".to_string(),
                    capability_id: "bank_a".to_string(),
                    gold_pile_id: over_cap.id,
                },
            )
            .expect_err("over-cap deposit");
        assert!(error.message().contains("transaction limit"));
        assert_eq!(engine.world, before);

        let next_sequence = engine.world.next_gold_sequence;
        engine.world.next_gold_sequence = u64::MAX;
        let before = engine.world.clone();
        let error = engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                crate::model::PlayerIntent::WithdrawBankGold {
                    service_id: "storage_counter_b".to_string(),
                    capability_id: "bank_b".to_string(),
                    amount: 1,
                },
            )
            .expect_err("late pile allocation failure must roll back the debit");
        assert!(error.message().contains("ground gold sequence overflow"));
        assert_eq!(engine.world, before);
        engine.world.next_gold_sequence = next_sequence;

        assert_eq!(
            engine.world.banks[&BankId::new("isolated_bank")].balance(&character_id),
            0
        );
        let isolated_pile = engine
            .create_ground_gold_pile(50, player.location.clone(), None)
            .expect("isolated bank pile");
        engine
            .apply_bank_deposit(
                0,
                "storage_counter_c",
                "bank_c",
                &isolated_pile.id,
                &mut Vec::new(),
            )
            .expect("deposit into isolated bank");
        assert_eq!(
            engine.world.banks[&BankId::new("isolated_bank")].balance(&character_id),
            50
        );
        assert_eq!(
            engine.world.banks[&BankId::new("shared_bank")].balance(&character_id),
            85
        );

        let (recipient_index, recipient_character_id) = add_recipient(&mut engine);
        assert_eq!(
            engine.world.banks[&BankId::new("shared_bank")].balance(&recipient_character_id),
            0
        );
        let recipient_pile = engine
            .create_ground_gold_pile(20, player.location.clone(), None)
            .expect("recipient pile");
        engine
            .apply_bank_deposit(
                recipient_index,
                "storage_counter_a",
                "bank_a",
                &recipient_pile.id,
                &mut Vec::new(),
            )
            .expect("recipient deposit");
        assert_eq!(
            engine.world.banks[&BankId::new("shared_bank")].balance(&recipient_character_id),
            20
        );
        assert_eq!(
            engine.world.banks[&BankId::new("shared_bank")].balance(&character_id),
            85
        );
    }

    #[test]
    fn locker_relocation_is_ordered_capacity_bounded_and_shared_by_vault() {
        let mut engine = storage_engine();
        let character_id = engine.world.actors[0]
            .character_id
            .clone()
            .expect("stable character");
        let second = engine.world.item_instances["training_sword"].clone();
        engine
            .world
            .item_instances
            .insert("training_sword_2".to_string(), second);
        let third = engine.world.item_instances["training_sword"].clone();
        engine
            .world
            .item_instances
            .insert("training_sword_3".to_string(), third);
        engine.world.actors[0]
            .carried
            .items
            .insert(CarriedPosition::LeftHand, "training_sword_2".to_string());
        engine.world.actors[0]
            .carried
            .items
            .insert(CarriedPosition::SackItem1, "training_sword_3".to_string());

        let mut events = Vec::new();
        engine
            .apply_locker_deposit(
                0,
                "storage_counter_a",
                "locker_a",
                "training_sword",
                &mut events,
            )
            .expect("locker deposit");
        let vault_id = LockerVaultId::new("shared_vault");
        engine
            .apply_locker_deposit(
                0,
                "storage_counter_b",
                "locker_b",
                "training_sword_2",
                &mut events,
            )
            .expect("second ordered locker deposit");
        assert_eq!(
            engine.world.locker_vaults[&vault_id].contents(&character_id),
            ["training_sword", "training_sword_2"]
        );
        let before = engine.world.clone();
        let error = engine
            .validate_locker_deposit(0, "storage_counter_b", "locker_b", "training_sword_3")
            .expect_err("full locker");
        assert_eq!(error.reason(), ActionBlockedReasonV1::LockerFull);
        assert_eq!(engine.world, before);

        engine
            .apply_locker_withdrawal(
                0,
                "storage_counter_b",
                "locker_b",
                "training_sword",
                CarriedPosition::RightHand,
                &mut events,
            )
            .expect("shared-vault withdrawal");
        assert_eq!(
            engine.world.locker_vaults[&vault_id].contents(&character_id),
            ["training_sword_2"]
        );
        assert!(matches!(
            engine.item_location("training_sword").unwrap(),
            ItemLocation::Carried {
                position: CarriedPosition::RightHand,
                ..
            }
        ));

        engine
            .apply_locker_deposit(
                0,
                "storage_counter_c",
                "locker_c",
                "training_sword_3",
                &mut events,
            )
            .expect("separate vault deposit");
        assert_eq!(
            engine.world.locker_vaults[&LockerVaultId::new("isolated_vault")]
                .contents(&character_id),
            ["training_sword_3"]
        );
        assert_eq!(
            engine.world.locker_vaults[&vault_id].contents(&character_id),
            ["training_sword_2"]
        );

        let (recipient_index, recipient_character_id) = add_recipient(&mut engine);
        let fourth = engine.world.item_instances["training_sword"].clone();
        engine
            .world
            .item_instances
            .insert("training_sword_4".to_string(), fourth);
        engine.world.actors[recipient_index]
            .carried
            .items
            .insert(CarriedPosition::LeftHand, "training_sword_4".to_string());
        engine
            .apply_locker_deposit(
                recipient_index,
                "storage_counter_a",
                "locker_a",
                "training_sword_4",
                &mut events,
            )
            .expect("recipient has an isolated locker within the shared vault");
        assert_eq!(
            engine.world.locker_vaults[&vault_id].contents(&recipient_character_id),
            ["training_sword_4"]
        );
        assert_eq!(
            engine.world.locker_vaults[&vault_id].contents(&character_id),
            ["training_sword_2"]
        );
        engine
            .validate_world_item_locations()
            .expect("locker locations remain authoritative");
    }

    #[test]
    fn action_context_projects_command_ready_bank_locker_and_offer_surfaces() {
        let mut engine = storage_engine();
        let player = engine.world.actors[0].clone();
        let pile = engine
            .create_ground_gold_pile(125, player.location.clone(), None)
            .expect("ground pile");
        let (recipient_index, recipient_character_id) = add_recipient(&mut engine);
        let mut later_recipient = engine.world.actors[recipient_index].clone();
        later_recipient.id = "later_recipient".into();
        later_recipient.name = "Later Recipient".to_string();
        later_recipient.character_id = Some(character_id("character:storage:z_recipient"));
        later_recipient.timing.tie_break_order = engine.world.timing.next_tie_break_order;
        engine.world.timing.next_tie_break_order += 1;
        engine.world.actors.insert(1, later_recipient);

        let context = engine
            .actor_observed_action_context(&tme_rules::ActorId::from("player"))
            .expect("observed storage context");
        assert_eq!(context.contract_version, 31);
        assert_eq!(context.carried.gold.sack, 500);
        assert_eq!(context.item_offer_actions.len(), 2);
        assert!(matches!(
            context.item_offer_actions[0]
                .command
                .as_ref()
                .map(|command| &command.intent),
            Some(PlayerIntentPayloadV1::OfferItem {
                recipient_character_id: recipient,
                item_instance_id,
            }) if recipient == &recipient_character_id && item_instance_id == "training_sword"
        ));

        let counter = context
            .services_here
            .iter()
            .find(|service| service.service_id == "storage_counter_a")
            .expect("storage counter");
        let bank = counter
            .capabilities
            .iter()
            .find_map(|capability| match capability {
                ServiceCapabilityViewV1::Bank {
                    balance_gold,
                    transaction_cap_gold,
                    deposit_actions,
                    withdrawal_actions,
                    ..
                } => Some((
                    *balance_gold,
                    *transaction_cap_gold,
                    deposit_actions,
                    withdrawal_actions,
                )),
                _ => None,
            })
            .expect("bank view");
        assert_eq!((bank.0, bank.1), (0, 200));
        assert_eq!(bank.2.len(), 1);
        assert!(bank.2[0].enabled);
        assert!(matches!(
            bank.2[0].command.as_ref().map(|command| &command.intent),
            Some(PlayerIntentPayloadV1::DepositBankGold { gold_pile_id, .. })
                if gold_pile_id == &pile.id
        ));
        assert_eq!(bank.3.len(), 1);
        assert!(!bank.3[0].enabled);
        assert!(bank.3[0].blocked_reason.is_some());
        assert!(bank.3[0].command.is_none());

        let locker = counter
            .capabilities
            .iter()
            .find_map(|capability| match capability {
                ServiceCapabilityViewV1::Locker {
                    capacity,
                    item_count,
                    deposit_actions,
                    withdrawal_actions,
                    ..
                } => Some((*capacity, *item_count, deposit_actions, withdrawal_actions)),
                _ => None,
            })
            .expect("locker view");
        assert_eq!((locker.0, locker.1), (2, 0));
        assert_eq!(locker.2.len(), 1);
        assert!(locker.2[0].enabled);
        assert!(locker.3.is_empty());

        engine
            .apply_item_offer(
                0,
                &recipient_character_id,
                "training_sword",
                &mut Vec::new(),
            )
            .expect("offer");
        let offered = engine
            .actor_observed_action_context(&tme_rules::ActorId::from("player"))
            .expect("offered context");
        assert!(offered.item_offer_actions.is_empty());
        assert!(offered.incoming_item_offers.is_empty());
        assert_eq!(offered.outgoing_item_offers.len(), 1);
        assert_eq!(offered.outgoing_item_offers[0].actions.len(), 1);
        assert!(matches!(
            offered.outgoing_item_offers[0].actions[0]
                .command
                .as_ref()
                .map(|command| &command.intent),
            Some(PlayerIntentPayloadV1::WithdrawItemOffer { item_instance_id })
                if item_instance_id == "training_sword"
        ));
    }

    #[test]
    fn offers_accept_refuse_withdraw_and_separation_return_the_reserved_hand() {
        let mut accepted = storage_engine();
        let (recipient_index, recipient_character_id, _) = create_offer(&mut accepted);
        assert_eq!(
            accepted
                .item_at_position(0, CarriedPosition::RightHand)
                .unwrap(),
            Some("training_sword")
        );
        assert!(matches!(
            accepted.item_location("training_sword").unwrap(),
            ItemLocation::Offered { .. }
        ));
        let mut events = Vec::new();
        accepted
            .apply_accept_item_offer(
                recipient_index,
                "training_sword",
                CarriedPosition::LeftHand,
                &mut events,
            )
            .expect("offer acceptance");
        assert!(accepted.world.item_offers.is_empty());
        assert!(matches!(
            accepted.item_location("training_sword").unwrap(),
            ItemLocation::Carried {
                holder: ItemHolderId::Character(owner),
                position: CarriedPosition::LeftHand,
            } if owner == recipient_character_id
        ));
        assert!(events.iter().any(|event| matches!(
            event,
            Event::ItemOfferCompleted {
                reason: ItemOfferCompletionReasonV1::Accepted,
                ..
            }
        )));

        let mut refused = storage_engine();
        let (recipient_index, _, _) = create_offer(&mut refused);
        let mut events = Vec::new();
        refused
            .apply_refuse_item_offer(recipient_index, "training_sword", &mut events)
            .expect("offer refusal");
        assert_eq!(
            refused
                .item_at_position(0, CarriedPosition::RightHand)
                .unwrap(),
            Some("training_sword")
        );
        assert!(events.iter().any(|event| matches!(
            event,
            Event::ItemOfferCompleted {
                reason: ItemOfferCompletionReasonV1::Refused,
                ..
            }
        )));

        let mut withdrawn = storage_engine();
        create_offer(&mut withdrawn);
        let mut events = Vec::new();
        withdrawn
            .apply_withdraw_item_offer(0, "training_sword", &mut events)
            .expect("sender withdrawal");
        assert!(events.iter().any(|event| matches!(
            event,
            Event::ItemOfferCompleted {
                reason: ItemOfferCompletionReasonV1::Withdrawn,
                ..
            }
        )));

        let mut separated = storage_engine();
        let (recipient_index, _, _) = create_offer(&mut separated);
        separated.world.actors[recipient_index].location.position.x += 1;
        let mut events = Vec::new();
        separated
            .reconcile_separated_item_offers(&mut events)
            .expect("separation return");
        assert_eq!(
            separated
                .item_at_position(0, CarriedPosition::RightHand)
                .unwrap(),
            Some("training_sword")
        );
        assert!(events.iter().any(|event| matches!(
            event,
            Event::ItemOfferCompleted {
                reason: ItemOfferCompletionReasonV1::Separated,
                ..
            }
        )));
    }

    #[test]
    fn offer_reservations_exclude_ordinary_moves_and_reject_collisions() {
        let mut engine = storage_engine();
        let second = engine.world.item_instances["training_sword"].clone();
        engine
            .world
            .item_instances
            .insert("training_sword_2".to_string(), second);
        engine.world.actors[0]
            .carried
            .items
            .insert(CarriedPosition::LeftHand, "training_sword_2".to_string());
        let (recipient_index, recipient_character_id, _) = create_offer(&mut engine);
        let sender_character_id = engine.world.actors[0]
            .character_id
            .clone()
            .expect("sender identity");
        engine
            .validate_world_item_locations()
            .expect("offer starts with one authoritative location");

        let before = engine.world.clone();
        let error = engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                crate::model::PlayerIntent::MoveItem {
                    item_instance_id: "training_sword".to_string(),
                    destination: crate::model::ItemMoveDestination::GroundHere,
                },
            )
            .expect_err("offered item cannot move through ordinary inventory");
        assert!(error.message().contains("reserved by an item offer"));
        assert_eq!(engine.world, before);

        let error = engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                crate::model::PlayerIntent::MoveGold {
                    source: GoldMoveSource::Carried {
                        position: crate::model::CarriedGoldPosition::Sack,
                    },
                    destination: GoldMoveDestination::Carried {
                        position: crate::model::CarriedGoldPosition::RightHand,
                    },
                    quantity: GoldMoveQuantity::Exact { amount: 1 },
                },
            )
            .expect_err("reserved hand cannot receive gold");
        assert!(error.message().contains("occupied by an item"));
        assert_eq!(engine.world, before);

        let error = engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                crate::model::PlayerIntent::MoveItem {
                    item_instance_id: "training_sword_2".to_string(),
                    destination: crate::model::ItemMoveDestination::Carried {
                        position: CarriedPosition::RightHand,
                    },
                },
            )
            .expect_err("reserved hand cannot receive another item");
        assert!(error.message().contains("occupied"));
        assert_eq!(engine.world, before);

        assert_eq!(
            engine
                .validate_accept_item_offer(0, "training_sword", CarriedPosition::SackItem1)
                .expect_err("sender cannot accept")
                .reason(),
            ActionBlockedReasonV1::InvalidTarget
        );
        assert_eq!(
            engine
                .validate_refuse_item_offer(0, "training_sword")
                .expect_err("sender cannot refuse")
                .reason(),
            ActionBlockedReasonV1::InvalidTarget
        );
        assert_eq!(
            engine
                .validate_withdraw_item_offer(recipient_index, "training_sword")
                .expect_err("recipient cannot withdraw")
                .reason(),
            ActionBlockedReasonV1::InvalidTarget
        );

        let mut duplicate_reservation = engine.clone();
        duplicate_reservation.world.item_offers.insert(
            "training_sword_2".to_string(),
            crate::model::ItemOfferState {
                sender_character_id: sender_character_id.clone(),
                recipient_character_id: recipient_character_id.clone(),
                source_position: CarriedPosition::RightHand,
            },
        );
        let error = duplicate_reservation
            .validate_world_item_locations()
            .expect_err("one sender hand cannot back two offers");
        assert!(
            error
                .message()
                .contains("multiple offers reserve the same sender hand")
        );

        let mut carried_collision = engine.clone();
        carried_collision.world.actors[0]
            .carried
            .items
            .remove(&CarriedPosition::LeftHand);
        carried_collision.world.actors[0]
            .carried
            .items
            .insert(CarriedPosition::RightHand, "training_sword_2".to_string());
        let error = carried_collision
            .validate_world_item_locations()
            .expect_err("reserved hand cannot also contain a carried item");
        assert!(
            error
                .message()
                .contains("offered source hand also contains a carried item")
        );

        let mut gold_collision = engine;
        gold_collision.world.actors[0].carried.gold.right_hand = 1;
        let error = gold_collision
            .validate_world_item_locations()
            .expect_err("reserved hand cannot also contain carried gold");
        assert!(
            error
                .message()
                .contains("offered source hand also contains carried gold")
        );
    }

    #[test]
    fn offers_preserve_binding_unwind_for_death_and_reject_missing_parties() {
        let mut bound = storage_engine();
        let sender_character_id = bound.world.actors[0]
            .character_id
            .clone()
            .expect("sender identity");
        bound
            .world
            .item_instances
            .get_mut("training_sword")
            .expect("training sword")
            .binding = ItemBindingState::Bound {
            character_id: sender_character_id.clone(),
        };
        let (recipient_index, _, _) = create_offer(&mut bound);
        bound
            .apply_accept_item_offer(
                recipient_index,
                "training_sword",
                CarriedPosition::LeftHand,
                &mut Vec::new(),
            )
            .expect("non-owner possession remains legal");
        assert_eq!(
            bound
                .item_at_position(recipient_index, CarriedPosition::LeftHand)
                .unwrap(),
            Some("training_sword")
        );
        assert_eq!(
            bound.world.item_instances["training_sword"].binding,
            ItemBindingState::Bound {
                character_id: sender_character_id,
            }
        );
        assert!(!bound.world.item_offers.contains_key("training_sword"));

        let mut recipient_defeat = storage_engine();
        let (recipient_index, _, _) = create_offer(&mut recipient_defeat);
        let mut events = Vec::new();
        recipient_defeat
            .unwind_item_offers_for_defeat(recipient_index, &mut events)
            .expect("recipient death unwind");
        assert_eq!(
            recipient_defeat
                .item_at_position(0, CarriedPosition::RightHand)
                .unwrap(),
            Some("training_sword")
        );
        assert!(events.iter().any(|event| matches!(
            event,
            Event::ItemOfferCompleted {
                reason: ItemOfferCompletionReasonV1::RecipientDefeated,
                ..
            }
        )));

        let mut sender_defeat = storage_engine();
        create_offer(&mut sender_defeat);
        let mut events = Vec::new();
        sender_defeat
            .unwind_item_offers_for_defeat(0, &mut events)
            .expect("sender death unwind");
        assert_eq!(
            sender_defeat
                .item_at_position(0, CarriedPosition::RightHand)
                .unwrap(),
            Some("training_sword")
        );
        assert!(events.iter().any(|event| matches!(
            event,
            Event::ItemOfferCompleted {
                reason: ItemOfferCompletionReasonV1::SenderDefeated,
                ..
            }
        )));

        let mut missing = storage_engine();
        create_offer(&mut missing);
        missing.world.actors.pop();
        let before = missing.world.clone();
        let error = missing
            .reconcile_separated_item_offers(&mut Vec::new())
            .expect_err("missing offer party is invariant failure");
        assert!(error.message().contains("unknown item holder"));
        assert_eq!(missing.world, before);
    }

    #[test]
    fn death_unwinds_sender_offer_before_inventory_relocation_and_conserves_positioned_gold() {
        let mut offered = storage_engine();
        create_offer(&mut offered);
        let mut events = Vec::new();
        offered
            .resolve_actor_defeat(
                0,
                DefeatContext {
                    cause: DeathCause::Physical,
                    credited_actor_id: None,
                    direct_social_actor_id: None,
                    spell_damage_credit: None,
                    hostile_authority: None,
                },
                &mut events,
            )
            .expect("sender defeat");
        assert!(offered.world.item_offers.is_empty());
        assert!(matches!(
            offered.item_location("training_sword").unwrap(),
            ItemLocation::Ground { .. }
        ));
        let completed = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    Event::ItemOfferCompleted {
                        reason: ItemOfferCompletionReasonV1::SenderDefeated,
                        ..
                    }
                )
            })
            .expect("offer completion event");
        let relocated = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    Event::ItemRelocated {
                        item_instance_id,
                        reason: ItemRelocationReason::DeathDrop,
                        ..
                    } if item_instance_id == "training_sword"
                )
            })
            .expect("ordinary death relocation event");
        assert!(completed < relocated);

        let mut gold = storage_engine();
        let sword = gold.world.actors[0]
            .carried
            .items
            .remove(&CarriedPosition::RightHand)
            .expect("right-hand sword");
        gold.world.actors[0]
            .carried
            .items
            .insert(CarriedPosition::SackItem1, sword);
        gold.world.actors[0].carried.gold = CarriedGold {
            left_hand: 10,
            right_hand: 20,
            sack: 30,
        };
        let mut events = Vec::new();
        gold.resolve_actor_defeat(
            0,
            DefeatContext {
                cause: DeathCause::Physical,
                credited_actor_id: None,
                direct_social_actor_id: None,
                spell_damage_credit: None,
                hostile_authority: None,
            },
            &mut events,
        )
        .expect("positioned-gold defeat");
        assert_eq!(gold.world.actors[0].carried.gold, CarriedGold::default());
        assert_eq!(gold.world.corpses.values().next().expect("corpse").gold, 60);
        let relocated = events
            .iter()
            .filter_map(|event| match event {
                Event::GoldRelocated {
                    amount,
                    from: crate::events::GoldLocationViewV1::Carried { position, .. },
                    reason: crate::events::GoldRelocationReason::CorpseRetention,
                    ..
                } => Some((*position, *amount)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            relocated,
            [
                (crate::model::CarriedGoldPosition::LeftHand, 10),
                (crate::model::CarriedGoldPosition::RightHand, 20),
                (crate::model::CarriedGoldPosition::Sack, 30),
            ]
        );
    }
}
