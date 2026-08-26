use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::model::{
    CarriedLayout, CarriedPosition, CorpseId, GoldPileId, GroundGoldPile, GroundItem,
    ItemBindingState, ItemHolderId, ItemInstanceState, ItemLocation, ItemMoveDestination,
    ItemOfferState, ItemPlacementKind, LockerVaultId, LockerVaultState, LootClaim,
    MerchantInventoryId, MerchantInventoryState, MerchantListingState, WorldPosition,
};
use crate::view::ActionBlockedReasonV1;

use super::{Engine, StepError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MpRecoverySelection {
    pub(super) position: CarriedPosition,
    pub(super) item_instance_id: String,
    pub(super) item_definition_id: String,
    pub(super) item_name: String,
    pub(super) numerator: u32,
    pub(super) denominator: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ItemRelocation {
    pub(super) item_instance_id: String,
    pub(super) expected: ItemLocation,
    pub(super) destination: ItemLocation,
    pub(super) loot_claim: Option<LootClaim>,
    pub(super) merchant_listing: Option<MerchantListingState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ItemBindingChange {
    pub(super) item_instance_id: String,
    pub(super) holder: ItemHolderId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ValidatedItemMove {
    pub(super) source: ItemLocation,
    pub(super) target: ItemLocation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SpellBookReceipt {
    pub(super) item_instance_id: String,
    pub(super) item_definition_id: String,
    pub(super) item_name: String,
    pub(super) character_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ItemMoveValidationError {
    pub(super) reason: ActionBlockedReasonV1,
    pub(super) message: String,
}

impl ItemMoveValidationError {
    fn new(reason: ActionBlockedReasonV1, message: impl Into<String>) -> Self {
        Self {
            reason,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone)]
struct LocationCollections {
    ground_items: Vec<GroundItem>,
    carried: Vec<CarriedLayout>,
    corpses: BTreeMap<CorpseId, BTreeMap<CarriedPosition, String>>,
    merchant_inventories: BTreeMap<MerchantInventoryId, MerchantInventoryState>,
    locker_vaults: BTreeMap<LockerVaultId, LockerVaultState>,
    item_offers: BTreeMap<String, ItemOfferState>,
}

fn require_single_location(
    item_instance_id: &str,
    mut locations: Vec<ItemLocation>,
) -> Result<ItemLocation, StepError> {
    match locations.len() {
        0 => Err(StepError::new(format!(
            "item instance {item_instance_id:?} has no location"
        ))),
        1 => Ok(locations.remove(0)),
        count => Err(StepError::new(format!(
            "item instance {item_instance_id:?} has {count} locations"
        ))),
    }
}

impl Engine {
    pub(super) fn item_holder_for_actor_index(
        &self,
        actor_index: usize,
    ) -> Result<ItemHolderId, StepError> {
        self.world
            .actors
            .get(actor_index)
            .map(|actor| actor.item_holder_id())
            .ok_or_else(|| StepError::new("unknown actor"))
    }

    pub(super) fn actor_index_for_item_holder(
        &self,
        holder: &ItemHolderId,
    ) -> Result<usize, StepError> {
        let mut matches = self
            .world
            .actors
            .iter()
            .enumerate()
            .filter(|(_, actor)| match holder {
                ItemHolderId::Character(character_id) => {
                    actor.character_id.as_ref() == Some(character_id)
                }
                ItemHolderId::TransientActor(actor_id) => &actor.id == actor_id,
            })
            .map(|(index, _)| index);
        let index = matches
            .next()
            .ok_or_else(|| StepError::new(format!("unknown item holder {holder:?}")))?;
        if matches.next().is_some() {
            return Err(StepError::new(format!(
                "item holder {holder:?} resolves to multiple actors"
            )));
        }
        Ok(index)
    }

    pub fn item_location(&self, item_instance_id: &str) -> Result<ItemLocation, StepError> {
        if !self.world.item_instances.contains_key(item_instance_id) {
            return Err(StepError::new(format!(
                "unknown item instance {item_instance_id:?}"
            )));
        }
        self.item_location_in(&self.location_collections(), item_instance_id)
    }

    pub(super) fn carried_items(
        &self,
        actor_index: usize,
    ) -> Result<&BTreeMap<CarriedPosition, String>, StepError> {
        self.world
            .actors
            .get(actor_index)
            .map(|actor| &actor.carried.items)
            .ok_or_else(|| StepError::new("unknown actor"))
    }

    pub(super) fn carried_item_ids(&self, actor_index: usize) -> Result<Vec<String>, StepError> {
        Ok(self.carried_items(actor_index)?.values().cloned().collect())
    }

    pub(super) fn sack_item_ids(&self, actor_index: usize) -> Result<Vec<String>, StepError> {
        Ok(self
            .carried_items(actor_index)?
            .iter()
            .filter(|(position, _)| position.is_sack_item())
            .map(|(_, item_instance_id)| item_instance_id.clone())
            .collect())
    }

    pub(super) fn active_item_ids(&self, actor_index: usize) -> Result<Vec<String>, StepError> {
        Ok(self
            .carried_items(actor_index)?
            .iter()
            .filter(|(position, _)| position.is_active_equipment())
            .map(|(_, item_instance_id)| item_instance_id.clone())
            .collect())
    }

    pub(super) fn highest_worn_mp_recovery_multiplier(
        &self,
        actor_index: usize,
    ) -> Result<Option<MpRecoverySelection>, StepError> {
        let mut selected: Option<MpRecoverySelection> = None;
        for (position, item_instance_id) in self.carried_items(actor_index)? {
            if !position.is_worn() {
                continue;
            }
            let instance = self.item_instance(item_instance_id)?;
            let definition = self
                .definition
                .catalog
                .item_catalog
                .get(&instance.definition_id)
                .ok_or_else(|| StepError::new("MP recovery item references unknown definition"))?;
            let Some(multiplier) = definition
                .capability
                .as_ref()
                .and_then(|capability| capability.mp_recovery_multiplier.as_ref())
            else {
                continue;
            };
            let candidate = MpRecoverySelection {
                position: *position,
                item_instance_id: item_instance_id.clone(),
                item_definition_id: instance.definition_id.clone(),
                item_name: definition.name.clone(),
                numerator: multiplier.numerator,
                denominator: multiplier.denominator,
            };
            let replace = selected.as_ref().is_none_or(|current| {
                let candidate_scaled =
                    u64::from(candidate.numerator) * u64::from(current.denominator);
                let current_scaled =
                    u64::from(current.numerator) * u64::from(candidate.denominator);
                candidate_scaled > current_scaled
                    || (candidate_scaled == current_scaled
                        && (candidate.position, candidate.item_instance_id.as_str())
                            < (current.position, current.item_instance_id.as_str()))
            });
            if replace {
                selected = Some(candidate);
            }
        }
        Ok(selected)
    }

    pub(super) fn has_worn_spell_focus(
        &self,
        actor_index: usize,
        lane: &str,
    ) -> Result<bool, StepError> {
        for (position, item_instance_id) in self.carried_items(actor_index)? {
            if position.placement_kind() != ItemPlacementKind::RingFinger {
                continue;
            }
            let instance = self
                .world
                .item_instances
                .get(item_instance_id)
                .ok_or_else(|| {
                    StepError::new(format!(
                        "carried spell focus references unknown item instance {item_instance_id:?}"
                    ))
                })?;
            let definition = self.definition.catalog.item_catalog
                .get(&instance.definition_id)
                .ok_or_else(|| {
                    StepError::new(format!(
                        "spell focus item instance {item_instance_id:?} references unknown definition {:?}",
                        instance.definition_id
                    ))
                })?;
            if definition
                .capability
                .as_ref()
                .and_then(|capability| capability.spell_focus_for.as_ref())
                .is_some_and(|lanes| lanes.iter().any(|candidate| candidate == lane))
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(super) fn right_hand_spell_book(
        &self,
        actor_index: usize,
        lane: &str,
    ) -> Result<SpellBookReceipt, ActionBlockedReasonV1> {
        let actor = self
            .world
            .actors
            .get(actor_index)
            .ok_or(ActionBlockedReasonV1::NoSuchTarget)?;
        let character_id = actor
            .character_id
            .as_ref()
            .ok_or(ActionBlockedReasonV1::SpellBookRequired)?;
        let item_instance_id = actor
            .carried
            .items
            .get(&CarriedPosition::RightHand)
            .ok_or(ActionBlockedReasonV1::SpellBookRequired)?;
        let instance = self
            .world
            .item_instances
            .get(item_instance_id)
            .ok_or(ActionBlockedReasonV1::SpellBookRequired)?;
        let definition = self
            .definition
            .catalog
            .item_catalog
            .get(&instance.definition_id)
            .ok_or(ActionBlockedReasonV1::SpellBookRequired)?;
        if instance.quantity != 1
            || !definition
                .capability
                .as_ref()
                .and_then(|capability| capability.spell_book_for.as_ref())
                .is_some_and(|lanes| lanes.iter().any(|candidate| candidate == lane))
        {
            return Err(ActionBlockedReasonV1::SpellBookRequired);
        }
        match &instance.binding {
            ItemBindingState::Bound {
                character_id: owner,
            } if owner == character_id => {}
            ItemBindingState::Bound { .. }
            | ItemBindingState::Unrestricted
            | ItemBindingState::BindOnFirstCharacterTouch => {
                return Err(ActionBlockedReasonV1::SpellBookNotOwned);
            }
        }
        Ok(SpellBookReceipt {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: instance.definition_id.clone(),
            item_name: definition.name.clone(),
            character_id: character_id.as_str().to_string(),
        })
    }

    pub(super) fn ordered_actor_item_ids(
        &self,
        actor_index: usize,
    ) -> Result<Vec<String>, StepError> {
        self.carried_item_ids(actor_index)
    }

    pub(super) fn item_at_position(
        &self,
        actor_index: usize,
        position: CarriedPosition,
    ) -> Result<Option<&str>, StepError> {
        if let Some(item) = self.carried_items(actor_index)?.get(&position) {
            return Ok(Some(item.as_str()));
        }
        if !matches!(
            position,
            CarriedPosition::LeftHand | CarriedPosition::RightHand
        ) {
            return Ok(None);
        }
        let character_id = self
            .world
            .actors
            .get(actor_index)
            .and_then(|actor| actor.character_id.as_ref());
        Ok(character_id.and_then(|character_id| {
            self.world.item_offers.iter().find_map(|(item_id, offer)| {
                (offer.sender_character_id == *character_id && offer.source_position == position)
                    .then_some(item_id.as_str())
            })
        }))
    }

    pub(super) fn carried_position_for_item(
        &self,
        actor_index: usize,
        item_instance_id: &str,
    ) -> Result<Option<CarriedPosition>, StepError> {
        Ok(self
            .carried_items(actor_index)?
            .iter()
            .find_map(|(position, existing)| (existing == item_instance_id).then_some(*position)))
    }

    pub(super) fn ground_items(&self) -> &[GroundItem] {
        &self.world.ground_items
    }

    pub(super) fn ground_items_at(&self, position: &WorldPosition) -> Vec<&GroundItem> {
        self.world
            .ground_items
            .iter()
            .filter(|item| item.location == *position)
            .collect()
    }

    pub(super) fn resolve_carried_instance(
        &self,
        actor_index: usize,
        instance_id: &str,
    ) -> Result<CarriedPosition, StepError> {
        self.carried_position_for_item(actor_index, instance_id)?
            .ok_or_else(|| StepError::new(format!("target {instance_id:?} is not carried")))
    }

    pub(super) fn resolve_sack_instance(
        &self,
        actor_index: usize,
        instance_id: &str,
    ) -> Result<CarriedPosition, StepError> {
        let position = self.resolve_carried_instance(actor_index, instance_id)?;
        if !position.is_sack_item() {
            return Err(StepError::new(format!(
                "target {instance_id:?} is not in the sack"
            )));
        }
        Ok(position)
    }

    pub(super) fn carried_gold_at(
        &self,
        actor_index: usize,
        position: crate::model::CarriedGoldPosition,
    ) -> Result<i64, StepError> {
        self.world
            .actors
            .get(actor_index)
            .map(|actor| actor.carried.gold.amount(position))
            .ok_or_else(|| StepError::new("unknown actor"))
    }

    pub(super) fn carried_gold_total(&self, actor_index: usize) -> Result<i64, StepError> {
        self.world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("unknown actor"))?
            .carried
            .gold
            .checked_total()
            .ok_or_else(|| StepError::new("carried gold total overflow"))
    }

    pub(super) fn apply_initial_item_bindings(&mut self) -> Result<(), StepError> {
        let placements = self
            .world
            .actors
            .iter()
            .flat_map(|actor| {
                let holder = actor.item_holder_id();
                actor
                    .carried
                    .items
                    .iter()
                    .map(move |(position, item_instance_id)| {
                        (
                            item_instance_id.clone(),
                            ItemLocation::Carried {
                                holder: holder.clone(),
                                position: *position,
                            },
                        )
                    })
            })
            .collect::<Vec<_>>();
        let mut instances = self.world.item_instances.clone();
        for (item_instance_id, destination) in placements {
            let _ = self.apply_binding_for_destination(
                &mut instances,
                &item_instance_id,
                &destination,
            )?;
        }
        self.world.item_instances = instances;
        Ok(())
    }

    pub(super) fn change_carried_gold_at(
        &mut self,
        actor_index: usize,
        position: crate::model::CarriedGoldPosition,
        amount: i64,
    ) -> Result<i64, StepError> {
        let actor = self
            .world
            .actors
            .get_mut(actor_index)
            .ok_or_else(|| StepError::new("unknown actor"))?;
        let next = actor
            .carried
            .gold
            .amount(position)
            .checked_add(amount)
            .ok_or_else(|| StepError::new("carried gold overflow"))?;
        if next < 0 {
            return Err(StepError::new(format!(
                "insufficient gold: need {}, have {}",
                amount.saturating_neg(),
                actor.carried.gold.amount(position)
            )));
        }
        *actor.carried.gold.amount_mut(position) = next;
        Ok(next)
    }

    fn next_gold_pile_id(&self) -> Result<(GoldPileId, u64), StepError> {
        let sequence = self.world.next_gold_sequence;
        let next = sequence
            .checked_add(1)
            .ok_or_else(|| StepError::new("ground gold sequence overflow"))?;
        let id = GoldPileId::from_sequence(sequence);
        if self.world.ground_gold.contains_key(&id) {
            return Err(StepError::new(format!("duplicate ground gold ID {id}")));
        }
        Ok((id, next))
    }

    pub(super) fn create_ground_gold_pile(
        &mut self,
        amount: i64,
        location: WorldPosition,
        loot_claim: Option<LootClaim>,
    ) -> Result<GroundGoldPile, StepError> {
        if amount <= 0 {
            return Err(StepError::new("ground gold amount must be positive"));
        }
        let (id, next_sequence) = self.next_gold_pile_id()?;
        let pile = GroundGoldPile {
            id: id.clone(),
            amount,
            location,
            loot_claim,
        };
        self.world.ground_gold.insert(id, pile.clone());
        self.world.next_gold_sequence = next_sequence;
        Ok(pile)
    }

    pub(super) fn consume_ground_gold_pile(
        &mut self,
        gold_pile_id: &GoldPileId,
    ) -> Result<GroundGoldPile, StepError> {
        self.world
            .ground_gold
            .remove(gold_pile_id)
            .ok_or_else(|| StepError::new(format!("unknown ground gold pile {gold_pile_id}")))
    }

    pub(super) fn validate_player_move_gold(
        &self,
        actor_index: usize,
        source: &crate::model::GoldMoveSource,
        destination: &crate::model::GoldMoveDestination,
        quantity: &crate::model::GoldMoveQuantity,
    ) -> Result<(), super::transactions::TransactionPlanError> {
        use super::transactions::TransactionPlanError;
        use crate::model::{GoldMoveDestination, GoldMoveQuantity, GoldMoveSource};
        use crate::view::ActionBlockedReasonV1;

        let blocked =
            |reason, message: &str| TransactionPlanError::new(reason, message.to_string());
        let actor = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| blocked(ActionBlockedReasonV1::NoSuchTarget, "unknown actor"))?;
        let available = match source {
            GoldMoveSource::Carried { position } => self
                .carried_gold_at(actor_index, *position)
                .map_err(|error| blocked(ActionBlockedReasonV1::NoSuchGold, error.message()))?,
            GoldMoveSource::Ground { gold_pile_id } => {
                let pile = self.world.ground_gold.get(gold_pile_id).ok_or_else(|| {
                    blocked(
                        ActionBlockedReasonV1::NoSuchGold,
                        "ground gold pile was not found",
                    )
                })?;
                if pile.location.level != actor.location.level
                    || pile.location.position != actor.location.position
                {
                    return Err(blocked(
                        ActionBlockedReasonV1::NoSuchGold,
                        "ground gold pile is not at the actor coordinate",
                    ));
                }
                pile.amount
            }
        };
        if available <= 0 {
            return Err(blocked(
                ActionBlockedReasonV1::NoSuchGold,
                "gold source is empty",
            ));
        }
        let amount = match quantity {
            GoldMoveQuantity::All => available,
            GoldMoveQuantity::Exact { amount } if *amount > 0 && *amount <= available => *amount,
            GoldMoveQuantity::Exact { amount } if *amount <= 0 => {
                return Err(blocked(
                    ActionBlockedReasonV1::InvalidGoldAmount,
                    "gold move amount must be positive",
                ));
            }
            GoldMoveQuantity::Exact { .. } => {
                return Err(blocked(
                    ActionBlockedReasonV1::InsufficientGold,
                    "gold move exceeds source amount",
                ));
            }
        };
        if matches!(
            (source, destination),
            (
                GoldMoveSource::Ground { .. },
                GoldMoveDestination::GroundHere
            )
        ) {
            return Err(blocked(
                ActionBlockedReasonV1::InvalidGoldAmount,
                "ground gold is already at ground_here",
            ));
        }
        if let (
            GoldMoveSource::Carried {
                position: source_position,
            },
            GoldMoveDestination::Carried {
                position: destination_position,
            },
        ) = (source, destination)
            && source_position == destination_position
        {
            return Err(blocked(
                ActionBlockedReasonV1::InvalidGoldAmount,
                "gold is already at the requested destination",
            ));
        }
        if let GoldMoveDestination::Carried { position } = destination {
            if let Some(hand) = position.hand_position()
                && self
                    .item_at_position(actor_index, hand)
                    .map_err(|error| {
                        blocked(ActionBlockedReasonV1::InvalidItemPlacement, error.message())
                    })?
                    .is_some()
            {
                return Err(blocked(
                    ActionBlockedReasonV1::OccupiedCarriedPosition,
                    "carried gold destination is occupied by an item",
                ));
            }
            self.carried_gold_at(actor_index, *position)
                .map_err(|error| blocked(ActionBlockedReasonV1::NoSuchGold, error.message()))?
                .checked_add(amount)
                .ok_or_else(|| {
                    blocked(
                        ActionBlockedReasonV1::InvalidGoldAmount,
                        "carried gold overflow",
                    )
                })?;
        }
        Ok(())
    }

    pub(super) fn apply_player_move_gold(
        &mut self,
        actor_index: usize,
        source: &crate::model::GoldMoveSource,
        destination: &crate::model::GoldMoveDestination,
        quantity: &crate::model::GoldMoveQuantity,
        events: &mut Vec<crate::events::Event>,
    ) -> Result<(), StepError> {
        self.apply_actor_move_gold(
            actor_index,
            source,
            destination,
            quantity,
            crate::events::GoldRelocationReason::PlayerMove,
            events,
        )
    }

    pub(super) fn apply_actor_move_gold(
        &mut self,
        actor_index: usize,
        source: &crate::model::GoldMoveSource,
        destination: &crate::model::GoldMoveDestination,
        quantity: &crate::model::GoldMoveQuantity,
        reason: crate::events::GoldRelocationReason,
        events: &mut Vec<crate::events::Event>,
    ) -> Result<(), StepError> {
        use crate::events::GoldLocationViewV1;
        use crate::model::{GoldMoveDestination, GoldMoveQuantity, GoldMoveSource};

        self.validate_player_move_gold(actor_index, source, destination, quantity)?;
        let actor = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("unknown actor"))?;
        let actor_id = actor.id.clone();
        let actor_name = actor.name.clone();
        let actor_room = actor.location.level.clone();
        let actor_position = actor.location.position;

        let (available, from, source_claim) = match source {
            GoldMoveSource::Carried { position } => (
                self.carried_gold_at(actor_index, *position)?,
                GoldLocationViewV1::Carried {
                    actor_id: actor_id.clone(),
                    position: *position,
                },
                None,
            ),
            GoldMoveSource::Ground { gold_pile_id } => {
                let pile = self.world.ground_gold.get(gold_pile_id).ok_or_else(|| {
                    StepError::new(format!("unknown ground gold pile {gold_pile_id}"))
                })?;
                if pile.location.level != actor_room || pile.location.position != actor_position {
                    return Err(StepError::new("ground gold pile is not at the actor"));
                }
                (
                    pile.amount,
                    GoldLocationViewV1::Ground {
                        gold_pile_id: pile.id.clone(),
                        location: pile.location.clone(),
                    },
                    pile.loot_claim.clone(),
                )
            }
        };
        if available <= 0 {
            return Err(StepError::new("gold source is empty"));
        }
        let amount = match quantity {
            GoldMoveQuantity::All => available,
            GoldMoveQuantity::Exact { amount } if *amount > 0 && *amount <= available => *amount,
            GoldMoveQuantity::Exact { amount } if *amount <= 0 => {
                return Err(StepError::new("gold move amount must be positive"));
            }
            GoldMoveQuantity::Exact { .. } => {
                return Err(StepError::new("gold move exceeds source amount"));
            }
        };

        if matches!(
            (source, destination),
            (
                GoldMoveSource::Ground { .. },
                GoldMoveDestination::GroundHere
            )
        ) {
            return Err(StepError::new("ground gold is already at ground_here"));
        }
        if let (
            GoldMoveSource::Carried {
                position: source_position,
            },
            GoldMoveDestination::Carried {
                position: destination_position,
            },
        ) = (source, destination)
            && source_position == destination_position
        {
            return Err(StepError::new(
                "gold is already at the requested destination",
            ));
        }
        if let GoldMoveDestination::Carried { position } = destination {
            if let Some(hand) = position.hand_position()
                && self.item_at_position(actor_index, hand)?.is_some()
            {
                return Err(StepError::new(format!(
                    "carried gold destination {} is occupied by an item",
                    position.label()
                )));
            }
            self.carried_gold_at(actor_index, *position)?
                .checked_add(amount)
                .ok_or_else(|| StepError::new("carried gold overflow"))?;
        }

        match source {
            GoldMoveSource::Carried { position } => {
                self.change_carried_gold_at(actor_index, *position, -amount)?;
            }
            GoldMoveSource::Ground { gold_pile_id } => {
                let remove = {
                    let pile = self
                        .world
                        .ground_gold
                        .get_mut(gold_pile_id)
                        .ok_or_else(|| StepError::new("ground gold source disappeared"))?;
                    pile.amount -= amount;
                    pile.amount == 0
                };
                if remove {
                    self.world.ground_gold.remove(gold_pile_id);
                }
            }
        }

        let to = match destination {
            GoldMoveDestination::Carried { position } => {
                self.change_carried_gold_at(actor_index, *position, amount)?;
                GoldLocationViewV1::Carried {
                    actor_id: actor_id.clone(),
                    position: *position,
                }
            }
            GoldMoveDestination::GroundHere => {
                let pile = self.create_ground_gold_pile(
                    amount,
                    self.world.actors[actor_index].location.clone(),
                    source_claim.clone(),
                )?;
                GoldLocationViewV1::Ground {
                    gold_pile_id: pile.id,
                    location: pile.location,
                }
            }
        };
        events.push(crate::events::Event::GoldRelocated {
            actor_id,
            actor: actor_name,
            amount,
            from,
            to,
            reason,
            loot_claim: source_claim,
        });
        Ok(())
    }

    pub(super) fn move_actor_gold_to_corpse(
        &mut self,
        actor_index: usize,
        corpse_id: &CorpseId,
    ) -> Result<i64, StepError> {
        let amount = self.carried_gold_total(actor_index)?;
        if amount == 0 {
            return Ok(0);
        }
        let corpse_gold = self
            .world
            .corpses
            .get(corpse_id)
            .ok_or_else(|| StepError::new(format!("unknown corpse {corpse_id}")))?
            .gold;
        let next = corpse_gold
            .checked_add(amount)
            .ok_or_else(|| StepError::new("corpse gold overflow"))?;
        self.world.actors[actor_index].carried.gold = Default::default();
        self.world
            .corpses
            .get_mut(corpse_id)
            .expect("validated corpse must remain present")
            .gold = next;
        Ok(amount)
    }

    pub(super) fn move_actor_gold_to_ground(
        &mut self,
        actor_index: usize,
        location: WorldPosition,
        loot_claim: Option<LootClaim>,
    ) -> Result<Option<GroundGoldPile>, StepError> {
        let amount = self.carried_gold_total(actor_index)?;
        if amount == 0 {
            return Ok(None);
        }
        let (id, next_sequence) = self.next_gold_pile_id()?;
        let pile = GroundGoldPile {
            id: id.clone(),
            amount,
            location,
            loot_claim,
        };
        self.world.actors[actor_index].carried.gold = Default::default();
        self.world.ground_gold.insert(id, pile.clone());
        self.world.next_gold_sequence = next_sequence;
        Ok(Some(pile))
    }

    pub(super) fn move_corpse_gold_to_ground(
        &mut self,
        corpse_id: &CorpseId,
    ) -> Result<Option<GroundGoldPile>, StepError> {
        let corpse = self
            .world
            .corpses
            .get(corpse_id)
            .ok_or_else(|| StepError::new(format!("unknown corpse {corpse_id}")))?;
        let amount = corpse.gold;
        if amount == 0 {
            return Ok(None);
        }
        let location = corpse.location.clone();
        let loot_claim = corpse.loot_claim.clone();
        let (id, next_sequence) = self.next_gold_pile_id()?;
        let pile = GroundGoldPile {
            id: id.clone(),
            amount,
            location,
            loot_claim,
        };
        self.world
            .corpses
            .get_mut(corpse_id)
            .expect("validated corpse must remain present")
            .gold = 0;
        self.world.ground_gold.insert(id, pile.clone());
        self.world.next_gold_sequence = next_sequence;
        Ok(Some(pile))
    }

    pub(super) fn move_corpse_gold_to_actor(
        &mut self,
        corpse_id: &CorpseId,
        actor_index: usize,
    ) -> Result<i64, StepError> {
        let amount = self
            .world
            .corpses
            .get(corpse_id)
            .ok_or_else(|| StepError::new(format!("unknown corpse {corpse_id}")))?
            .gold;
        if amount == 0 {
            return Ok(0);
        }
        let actor_gold =
            self.carried_gold_at(actor_index, crate::model::CarriedGoldPosition::Sack)?;
        let next = actor_gold
            .checked_add(amount)
            .ok_or_else(|| StepError::new("carried gold overflow"))?;
        self.world
            .corpses
            .get_mut(corpse_id)
            .expect("validated corpse must remain present")
            .gold = 0;
        self.world.actors[actor_index].carried.gold.sack = next;
        Ok(amount)
    }

    pub(super) fn validate_item_move(
        &self,
        actor_index: usize,
        item_instance_id: &str,
        destination: &ItemMoveDestination,
    ) -> Result<ValidatedItemMove, ItemMoveValidationError> {
        let actor = self.world.actors.get(actor_index).ok_or_else(|| {
            ItemMoveValidationError::new(ActionBlockedReasonV1::NoSuchItem, "unknown actor")
        })?;
        let holder = actor.item_holder_id();
        let here = actor.location.clone();
        let source = self.item_location(item_instance_id).map_err(|error| {
            ItemMoveValidationError::new(ActionBlockedReasonV1::NoSuchItem, error.message())
        })?;
        match &source {
            ItemLocation::Ground { position } if position == &here => {}
            ItemLocation::Carried { holder: actual, .. } if actual == &holder => {}
            ItemLocation::Ground { .. } => {
                return Err(ItemMoveValidationError::new(
                    ActionBlockedReasonV1::NoSuchItem,
                    format!("target {item_instance_id:?} is not in reach"),
                ));
            }
            ItemLocation::Carried { .. } => {
                return Err(ItemMoveValidationError::new(
                    ActionBlockedReasonV1::NoSuchItem,
                    format!("target {item_instance_id:?} is not carried by the actor"),
                ));
            }
            ItemLocation::Corpse { .. } => {
                return Err(ItemMoveValidationError::new(
                    ActionBlockedReasonV1::NoSuchItem,
                    format!("target {item_instance_id:?} is held by a corpse"),
                ));
            }
            ItemLocation::Merchant { .. } => {
                return Err(ItemMoveValidationError::new(
                    ActionBlockedReasonV1::NoSuchItem,
                    format!("target {item_instance_id:?} is held by a merchant"),
                ));
            }
            ItemLocation::Locker { .. } => {
                return Err(ItemMoveValidationError::new(
                    ActionBlockedReasonV1::NoSuchItem,
                    format!("target {item_instance_id:?} is held in a locker"),
                ));
            }
            ItemLocation::Offered { .. } => {
                return Err(ItemMoveValidationError::new(
                    ActionBlockedReasonV1::NoSuchItem,
                    format!("target {item_instance_id:?} is reserved by an item offer"),
                ));
            }
        }
        let target = match destination {
            ItemMoveDestination::GroundHere => ItemLocation::Ground { position: here },
            ItemMoveDestination::Carried { position } => ItemLocation::Carried {
                holder,
                position: *position,
            },
        };
        if source == target {
            return Err(ItemMoveValidationError::new(
                ActionBlockedReasonV1::InvalidItemPlacement,
                "item is already at the requested destination",
            ));
        }
        if let ItemMoveDestination::Carried { position } = destination {
            if self
                .item_at_position(actor_index, *position)
                .map_err(|error| {
                    ItemMoveValidationError::new(ActionBlockedReasonV1::NoSuchItem, error.message())
                })?
                .is_some()
            {
                return Err(ItemMoveValidationError::new(
                    ActionBlockedReasonV1::OccupiedCarriedPosition,
                    format!("carried position {:?} is occupied", position.label()),
                ));
            }
            let instance = self.item_instance(item_instance_id).map_err(|error| {
                ItemMoveValidationError::new(ActionBlockedReasonV1::NoSuchItem, error.message())
            })?;
            let definition = self.item_definition(item_instance_id).map_err(|error| {
                ItemMoveValidationError::new(ActionBlockedReasonV1::NoSuchItem, error.message())
            })?;
            if !definition
                .valid_placements
                .contains(&position.placement_kind())
            {
                return Err(ItemMoveValidationError::new(
                    ActionBlockedReasonV1::InvalidItemPlacement,
                    format!(
                        "item instance {item_instance_id:?} cannot occupy {:?}",
                        position.label()
                    ),
                ));
            }
            if !position.is_sack_item() && instance.quantity != 1 {
                return Err(ItemMoveValidationError::new(
                    ActionBlockedReasonV1::InvalidItemQuantity,
                    format!(
                        "item instance {item_instance_id:?} must have quantity 1 outside the sack"
                    ),
                ));
            }
        }
        Ok(ValidatedItemMove { source, target })
    }

    pub(super) fn validate_world_item_locations(&self) -> Result<(), StepError> {
        let registry_ids = self.world.item_instances.keys().cloned().collect();
        self.validate_location_collections(
            &self.location_collections(),
            &self.world.item_instances,
            &registry_ids,
        )
    }

    pub(super) fn relocate_items(
        &mut self,
        relocations: &[ItemRelocation],
    ) -> Result<Vec<ItemBindingChange>, StepError> {
        let mut collections = self.location_collections();
        let mut instances = self.world.item_instances.clone();
        let mut operation_ids = HashSet::new();
        let mut binding_changes = Vec::new();

        for relocation in relocations {
            if !operation_ids.insert(relocation.item_instance_id.as_str()) {
                return Err(StepError::new(format!(
                    "duplicate item relocation {:?}",
                    relocation.item_instance_id
                )));
            }
            if !instances.contains_key(&relocation.item_instance_id) {
                return Err(StepError::new(format!(
                    "unknown item instance {:?}",
                    relocation.item_instance_id
                )));
            }
            let actual = self.item_location_in(&collections, &relocation.item_instance_id)?;
            if actual != relocation.expected {
                return Err(StepError::new(format!(
                    "item instance {:?} expected at {:?}, found {:?}",
                    relocation.item_instance_id, relocation.expected, actual
                )));
            }
            self.remove_item_location(
                &mut collections,
                &relocation.item_instance_id,
                &relocation.expected,
            )?;
            self.append_item_location(
                &mut collections,
                &instances,
                &relocation.item_instance_id,
                &relocation.destination,
                relocation.loot_claim.clone(),
                relocation.merchant_listing.clone(),
            )?;
            if let Some(change) = self.apply_binding_for_destination(
                &mut instances,
                &relocation.item_instance_id,
                &relocation.destination,
            )? {
                binding_changes.push(change);
            }
        }

        let registry_ids = instances.keys().cloned().collect();
        self.validate_location_collections(&collections, &instances, &registry_ids)?;
        self.commit_location_collections(collections);
        self.world.item_instances = instances;
        Ok(binding_changes)
    }

    pub(super) fn register_item_instances(
        &mut self,
        mut new_instances: BTreeMap<String, ItemInstanceState>,
        placements: &[(String, ItemLocation)],
    ) -> Result<(), StepError> {
        let mut instances = self.world.item_instances.clone();
        for (instance_id, instance) in &new_instances {
            if instances.contains_key(instance_id) {
                return Err(StepError::new(format!(
                    "item instance {instance_id:?} is already registered"
                )));
            }
            self.validate_binding(instance_id, instance)?;
        }

        let mut placement_ids = HashSet::new();
        let mut collections = self.location_collections();
        for (instance_id, location) in placements {
            if !new_instances.contains_key(instance_id) {
                return Err(StepError::new(format!(
                    "placement references unregistered new item instance {instance_id:?}"
                )));
            }
            if !placement_ids.insert(instance_id.as_str()) {
                return Err(StepError::new(format!(
                    "duplicate item placement {instance_id:?}"
                )));
            }
            self.append_item_location(
                &mut collections,
                &new_instances,
                instance_id,
                location,
                None,
                None,
            )?;
            let _ =
                self.apply_binding_for_destination(&mut new_instances, instance_id, location)?;
        }
        for instance_id in new_instances.keys() {
            if !placement_ids.contains(instance_id.as_str()) {
                return Err(StepError::new(format!(
                    "item instance {instance_id:?} has no initial placement"
                )));
            }
        }
        instances.extend(new_instances);
        let registry_ids = instances.keys().cloned().collect();
        self.validate_location_collections(&collections, &instances, &registry_ids)?;
        self.commit_location_collections(collections);
        self.world.item_instances = instances;
        Ok(())
    }

    pub(super) fn destroy_item_instances(
        &mut self,
        instance_ids: &[String],
    ) -> Result<(), StepError> {
        let mut instances = self.world.item_instances.clone();
        let mut operation_ids = HashSet::new();
        let mut collections = self.location_collections();

        for instance_id in instance_ids {
            if !operation_ids.insert(instance_id.as_str()) {
                return Err(StepError::new(format!(
                    "duplicate item destruction {instance_id:?}"
                )));
            }
            if instances.remove(instance_id).is_none() {
                return Err(StepError::new(format!(
                    "unknown item instance {instance_id:?}"
                )));
            }
            let location = self.item_location_in(&collections, instance_id)?;
            self.remove_item_location(&mut collections, instance_id, &location)?;
        }

        let registry_ids = instances.keys().cloned().collect();
        self.validate_location_collections(&collections, &instances, &registry_ids)?;
        self.commit_location_collections(collections);
        self.world.item_instances = instances;
        self.world
            .item_enchantments
            .retain(|enchantment| !operation_ids.contains(enchantment.item_instance_id.as_str()));
        Ok(())
    }

    fn location_collections(&self) -> LocationCollections {
        LocationCollections {
            ground_items: self.world.ground_items.clone(),
            carried: self
                .world
                .actors
                .iter()
                .map(|actor| actor.carried.clone())
                .collect(),
            corpses: self
                .world
                .corpses
                .iter()
                .map(|(corpse_id, corpse)| (corpse_id.clone(), corpse.contents.clone()))
                .collect(),
            merchant_inventories: self.world.merchant_inventories.clone(),
            locker_vaults: self.world.locker_vaults.clone(),
            item_offers: self.world.item_offers.clone(),
        }
    }

    fn commit_location_collections(&mut self, collections: LocationCollections) {
        debug_assert_eq!(collections.carried.len(), self.world.actors.len());
        self.world.ground_items = collections.ground_items;
        for (actor, carried) in self.world.actors.iter_mut().zip(collections.carried) {
            actor.carried = carried;
        }
        for (corpse_id, contents) in collections.corpses {
            self.world
                .corpses
                .get_mut(&corpse_id)
                .expect("validated corpse collection must exist")
                .contents = contents;
        }
        self.world.merchant_inventories = collections.merchant_inventories;
        self.world.locker_vaults = collections.locker_vaults;
        self.world.item_offers = collections.item_offers;
    }

    fn item_location_in(
        &self,
        collections: &LocationCollections,
        item_instance_id: &str,
    ) -> Result<ItemLocation, StepError> {
        let mut locations = Vec::new();
        locations.extend(
            collections
                .ground_items
                .iter()
                .filter(|item| item.item_instance_id == item_instance_id)
                .map(|item| ItemLocation::Ground {
                    position: item.location.clone(),
                }),
        );
        for (actor_index, carried) in collections.carried.iter().enumerate() {
            locations.extend(
                carried
                    .items
                    .iter()
                    .filter(|(_, existing)| existing.as_str() == item_instance_id)
                    .map(|(position, _)| ItemLocation::Carried {
                        holder: self.world.actors[actor_index].item_holder_id(),
                        position: *position,
                    }),
            );
        }
        for (corpse_id, contents) in &collections.corpses {
            locations.extend(
                contents
                    .iter()
                    .filter(|(_, existing)| existing.as_str() == item_instance_id)
                    .map(|(position, _)| ItemLocation::Corpse {
                        corpse_id: corpse_id.clone(),
                        position: *position,
                    }),
            );
        }
        for (inventory_id, inventory) in &collections.merchant_inventories {
            locations.extend(
                inventory
                    .listings
                    .iter()
                    .filter(|listing| listing.item_instance_id == item_instance_id)
                    .map(|_| ItemLocation::Merchant {
                        inventory_id: inventory_id.clone(),
                    }),
            );
        }
        for (vault_id, vault) in &collections.locker_vaults {
            for (owner_character_id, contents) in &vault.lockers {
                locations.extend(
                    contents
                        .iter()
                        .filter(|existing| existing.as_str() == item_instance_id)
                        .map(|_| ItemLocation::Locker {
                            vault_id: vault_id.clone(),
                            owner_character_id: owner_character_id.clone(),
                        }),
                );
            }
        }
        if let Some(offer) = collections.item_offers.get(item_instance_id) {
            locations.push(ItemLocation::Offered {
                sender_character_id: offer.sender_character_id.clone(),
                recipient_character_id: offer.recipient_character_id.clone(),
                source_position: offer.source_position,
            });
        }
        let location = require_single_location(item_instance_id, locations)?;
        if let ItemLocation::Carried { holder, .. } = &location {
            self.actor_index_for_item_holder(holder)?;
        }
        Ok(location)
    }

    fn remove_item_location(
        &self,
        collections: &mut LocationCollections,
        item_instance_id: &str,
        location: &ItemLocation,
    ) -> Result<(), StepError> {
        match location {
            ItemLocation::Ground { position } => {
                let index = collections
                    .ground_items
                    .iter()
                    .position(|item| {
                        item.item_instance_id == item_instance_id && item.location == *position
                    })
                    .ok_or_else(|| StepError::new("item relocation source disappeared"))?;
                collections.ground_items.remove(index);
            }
            ItemLocation::Carried { holder, position } => {
                let actor_index = self.actor_index_for_item_holder(holder)?;
                match collections.carried[actor_index].items.remove(position) {
                    Some(existing) if existing == item_instance_id => {}
                    Some(existing) => {
                        collections.carried[actor_index]
                            .items
                            .insert(*position, existing);
                        return Err(StepError::new("item relocation source changed"));
                    }
                    None => return Err(StepError::new("item relocation source disappeared")),
                }
            }
            ItemLocation::Corpse {
                corpse_id,
                position,
            } => {
                let contents = collections
                    .corpses
                    .get_mut(corpse_id)
                    .ok_or_else(|| StepError::new("item corpse source disappeared"))?;
                match contents.remove(position) {
                    Some(existing) if existing == item_instance_id => {}
                    Some(existing) => {
                        contents.insert(*position, existing);
                        return Err(StepError::new("item relocation source changed"));
                    }
                    None => return Err(StepError::new("item relocation source disappeared")),
                }
            }
            ItemLocation::Merchant { inventory_id } => {
                let inventory = collections
                    .merchant_inventories
                    .get_mut(inventory_id)
                    .ok_or_else(|| StepError::new("item merchant source disappeared"))?;
                let index = inventory
                    .listings
                    .iter()
                    .position(|listing| listing.item_instance_id == item_instance_id)
                    .ok_or_else(|| StepError::new("item merchant source disappeared"))?;
                inventory.listings.remove(index);
            }
            ItemLocation::Locker {
                vault_id,
                owner_character_id,
            } => {
                let vault = collections
                    .locker_vaults
                    .get_mut(vault_id)
                    .ok_or_else(|| StepError::new("item locker source disappeared"))?;
                let contents = vault
                    .lockers
                    .get_mut(owner_character_id)
                    .ok_or_else(|| StepError::new("item locker owner source disappeared"))?;
                let index = contents
                    .iter()
                    .position(|existing| existing == item_instance_id)
                    .ok_or_else(|| StepError::new("item locker source disappeared"))?;
                contents.remove(index);
                if contents.is_empty() {
                    vault.lockers.remove(owner_character_id);
                }
            }
            ItemLocation::Offered {
                sender_character_id,
                recipient_character_id,
                source_position,
            } => {
                let offer = collections
                    .item_offers
                    .remove(item_instance_id)
                    .ok_or_else(|| StepError::new("item offer source disappeared"))?;
                if offer.sender_character_id != *sender_character_id
                    || offer.recipient_character_id != *recipient_character_id
                    || offer.source_position != *source_position
                {
                    return Err(StepError::new("item offer source changed"));
                }
            }
        }
        Ok(())
    }

    fn append_item_location(
        &self,
        collections: &mut LocationCollections,
        instances: &BTreeMap<String, ItemInstanceState>,
        item_instance_id: &str,
        location: &ItemLocation,
        loot_claim: Option<LootClaim>,
        merchant_listing: Option<MerchantListingState>,
    ) -> Result<(), StepError> {
        if !matches!(location, ItemLocation::Merchant { .. }) && merchant_listing.is_some() {
            return Err(StepError::new(
                "merchant listing metadata requires a merchant destination",
            ));
        }
        match location {
            ItemLocation::Ground { position } => collections.ground_items.push(GroundItem {
                item_instance_id: item_instance_id.to_string(),
                location: position.clone(),
                loot_claim,
            }),
            ItemLocation::Carried { holder, position } => {
                if loot_claim.is_some() {
                    return Err(StepError::new("carried items cannot retain a loot claim"));
                }
                let actor_index = self.actor_index_for_item_holder(holder)?;
                self.validate_carried_placement(
                    item_instance_id,
                    actor_index,
                    *position,
                    instances,
                )?;
                if collections.carried[actor_index]
                    .items
                    .contains_key(position)
                {
                    return Err(StepError::new(format!(
                        "carried position {:?} is occupied",
                        position.label()
                    )));
                }
                collections.carried[actor_index]
                    .items
                    .insert(*position, item_instance_id.to_string());
            }
            ItemLocation::Corpse {
                corpse_id,
                position,
            } => {
                let corpse = self
                    .world
                    .corpses
                    .get(corpse_id)
                    .ok_or_else(|| StepError::new(format!("unknown corpse {corpse_id}")))?;
                if loot_claim != corpse.loot_claim {
                    return Err(StepError::new(
                        "corpse item claim does not match corpse claim",
                    ));
                }
                let contents = collections
                    .corpses
                    .get_mut(corpse_id)
                    .ok_or_else(|| StepError::new(format!("unknown corpse {corpse_id}")))?;
                if contents.contains_key(position) {
                    return Err(StepError::new(format!(
                        "corpse position {:?} is occupied",
                        position.label()
                    )));
                }
                contents.insert(*position, item_instance_id.to_string());
            }
            ItemLocation::Merchant { inventory_id } => {
                if loot_claim.is_some() {
                    return Err(StepError::new("merchant items cannot retain a loot claim"));
                }
                let listing = merchant_listing.ok_or_else(|| {
                    StepError::new("merchant destination requires listing metadata")
                })?;
                if listing.item_instance_id != item_instance_id || listing.price_gold <= 0 {
                    return Err(StepError::new("merchant listing metadata is invalid"));
                }
                let inventory = collections
                    .merchant_inventories
                    .get_mut(inventory_id)
                    .ok_or_else(|| StepError::new("unknown merchant inventory"))?;
                if inventory
                    .listings
                    .iter()
                    .any(|existing| existing.item_instance_id == item_instance_id)
                {
                    return Err(StepError::new("merchant inventory already contains item"));
                }
                inventory.listings.push(listing);
            }
            ItemLocation::Locker {
                vault_id,
                owner_character_id,
            } => {
                if loot_claim.is_some() || merchant_listing.is_some() {
                    return Err(StepError::new(
                        "locker items cannot retain claim or merchant metadata",
                    ));
                }
                let vault = collections
                    .locker_vaults
                    .get_mut(vault_id)
                    .ok_or_else(|| StepError::new("unknown locker vault"))?;
                let contents = vault.lockers.entry(owner_character_id.clone()).or_default();
                let count = u32::try_from(contents.len())
                    .map_err(|_| StepError::new("locker item count overflow"))?;
                let capacity = self
                    .definition
                    .catalog
                    .locker_vault_definitions
                    .get(vault_id)
                    .ok_or_else(|| StepError::new("unknown locker vault definition"))?
                    .capacity;
                if count >= capacity {
                    return Err(StepError::new("locker is full"));
                }
                if contents.iter().any(|existing| existing == item_instance_id) {
                    return Err(StepError::new("locker already contains item"));
                }
                contents.push(item_instance_id.to_string());
            }
            ItemLocation::Offered {
                sender_character_id,
                recipient_character_id,
                source_position,
            } => {
                if loot_claim.is_some() || merchant_listing.is_some() {
                    return Err(StepError::new(
                        "offered items cannot retain claim or merchant metadata",
                    ));
                }
                if collections.item_offers.contains_key(item_instance_id) {
                    return Err(StepError::new("item already has an offer"));
                }
                collections.item_offers.insert(
                    item_instance_id.to_string(),
                    ItemOfferState {
                        sender_character_id: sender_character_id.clone(),
                        recipient_character_id: recipient_character_id.clone(),
                        source_position: *source_position,
                    },
                );
            }
        }
        Ok(())
    }

    pub(super) fn validate_carried_placement(
        &self,
        item_instance_id: &str,
        actor_index: usize,
        position: CarriedPosition,
        instances: &BTreeMap<String, ItemInstanceState>,
    ) -> Result<(), StepError> {
        let instance = instances
            .get(item_instance_id)
            .ok_or_else(|| StepError::new(format!("unknown item instance {item_instance_id:?}")))?;
        self.validate_binding(item_instance_id, instance)?;
        let definition = self
            .definition
            .catalog
            .item_catalog
            .get(&instance.definition_id)
            .ok_or_else(|| {
                StepError::new(format!(
                    "unknown item definition {:?}",
                    instance.definition_id
                ))
            })?;
        let placement_kind = position.placement_kind();
        if !definition.valid_placements.contains(&placement_kind) {
            return Err(StepError::new(format!(
                "item instance {item_instance_id:?} cannot occupy {:?}",
                position.label()
            )));
        }
        if !position.is_sack_item() && instance.quantity != 1 {
            return Err(StepError::new(format!(
                "item instance {item_instance_id:?} must have quantity 1 outside the sack"
            )));
        }
        let hand_gold = match position {
            CarriedPosition::LeftHand => Some(crate::model::CarriedGoldPosition::LeftHand),
            CarriedPosition::RightHand => Some(crate::model::CarriedGoldPosition::RightHand),
            _ => None,
        };
        if let Some(gold_position) = hand_gold
            && self.carried_gold_at(actor_index, gold_position)? > 0
        {
            return Err(StepError::new(format!(
                "carried position {:?} contains gold",
                position.label()
            )));
        }
        Ok(())
    }

    fn validate_binding(
        &self,
        item_instance_id: &str,
        instance: &ItemInstanceState,
    ) -> Result<(), StepError> {
        if !matches!(instance.binding, ItemBindingState::Unrestricted) && instance.quantity != 1 {
            return Err(StepError::new(format!(
                "tied item instance {item_instance_id:?} must have quantity 1"
            )));
        }
        if let ItemBindingState::Bound { character_id } = &instance.binding
            && character_id.as_str().trim().is_empty()
        {
            return Err(StepError::new(format!(
                "tied item instance {item_instance_id:?} has an empty character_id"
            )));
        }
        Ok(())
    }

    fn apply_binding_for_destination(
        &self,
        instances: &mut BTreeMap<String, ItemInstanceState>,
        item_instance_id: &str,
        destination: &ItemLocation,
    ) -> Result<Option<ItemBindingChange>, StepError> {
        let ItemLocation::Carried { holder, .. } = destination else {
            return Ok(None);
        };
        let ItemHolderId::Character(character_id) = holder else {
            return Ok(None);
        };
        let instance = instances
            .get_mut(item_instance_id)
            .ok_or_else(|| StepError::new(format!("unknown item instance {item_instance_id:?}")))?;
        if matches!(
            instance.binding,
            ItemBindingState::BindOnFirstCharacterTouch
        ) {
            instance.binding = ItemBindingState::Bound {
                character_id: character_id.clone(),
            };
            return Ok(Some(ItemBindingChange {
                item_instance_id: item_instance_id.to_string(),
                holder: holder.clone(),
            }));
        }
        Ok(None)
    }

    fn validate_location_collections(
        &self,
        collections: &LocationCollections,
        instances: &BTreeMap<String, ItemInstanceState>,
        registry_ids: &BTreeSet<String>,
    ) -> Result<(), StepError> {
        if collections.carried.len() != self.world.actors.len() {
            return Err(StepError::new(
                "item location actor collections are misaligned",
            ));
        }
        if collections.corpses.len() != self.world.corpses.len()
            || collections
                .corpses
                .keys()
                .any(|corpse_id| !self.world.corpses.contains_key(corpse_id))
        {
            return Err(StepError::new("item corpse collections are misaligned"));
        }
        if collections
            .merchant_inventories
            .keys()
            .ne(self.world.merchant_inventories.keys())
        {
            return Err(StepError::new(
                "merchant inventory collections are misaligned",
            ));
        }
        if collections
            .locker_vaults
            .keys()
            .ne(self.world.locker_vaults.keys())
        {
            return Err(StepError::new("locker vault collections are misaligned"));
        }

        let mut reference_counts = HashMap::<&str, usize>::new();
        for ground_item in &collections.ground_items {
            if !registry_ids.contains(&ground_item.item_instance_id) {
                return Err(StepError::new(format!(
                    "item location references unknown instance {:?}",
                    ground_item.item_instance_id
                )));
            }
            *reference_counts
                .entry(ground_item.item_instance_id.as_str())
                .or_default() += 1;
        }
        for (actor_index, carried) in collections.carried.iter().enumerate() {
            self.actor_index_for_item_holder(&self.world.actors[actor_index].item_holder_id())?;
            if carried.gold.left_hand < 0
                || carried.gold.right_hand < 0
                || carried.gold.sack < 0
                || carried.gold.checked_total().is_none()
            {
                return Err(StepError::new(format!(
                    "actor {:?} carried gold cannot be negative",
                    self.world.actors[actor_index].id
                )));
            }
            for (position, instance_id) in &carried.items {
                if !registry_ids.contains(instance_id) {
                    return Err(StepError::new(format!(
                        "item location references unknown instance {instance_id:?}"
                    )));
                }
                self.validate_carried_placement(instance_id, actor_index, *position, instances)?;
                *reference_counts.entry(instance_id.as_str()).or_default() += 1;
            }
            if carried.gold.left_hand > 0 && carried.items.contains_key(&CarriedPosition::LeftHand)
            {
                return Err(StepError::new("left hand contains both item and gold"));
            }
            if carried.gold.right_hand > 0
                && carried.items.contains_key(&CarriedPosition::RightHand)
            {
                return Err(StepError::new("right hand contains both item and gold"));
            }
        }
        for (corpse_id, contents) in &collections.corpses {
            let corpse = self
                .world
                .corpses
                .get(corpse_id)
                .ok_or_else(|| StepError::new(format!("unknown corpse {corpse_id}")))?;
            if corpse.gold < 0 {
                return Err(StepError::new(format!(
                    "corpse {corpse_id} gold cannot be negative"
                )));
            }
            for instance_id in contents.values() {
                if !registry_ids.contains(instance_id) {
                    return Err(StepError::new(format!(
                        "corpse item location references unknown instance {instance_id:?}"
                    )));
                }
                *reference_counts.entry(instance_id.as_str()).or_default() += 1;
            }
        }
        for (inventory_id, inventory) in &collections.merchant_inventories {
            let mut inventory_ids = HashSet::new();
            for listing in &inventory.listings {
                if listing.price_gold <= 0 {
                    return Err(StepError::new(format!(
                        "merchant inventory {inventory_id:?} has a non-positive price"
                    )));
                }
                if !inventory_ids.insert(listing.item_instance_id.as_str()) {
                    return Err(StepError::new(format!(
                        "merchant inventory {inventory_id:?} contains duplicate item {:?}",
                        listing.item_instance_id
                    )));
                }
                if !registry_ids.contains(&listing.item_instance_id) {
                    return Err(StepError::new(format!(
                        "merchant item location references unknown instance {:?}",
                        listing.item_instance_id
                    )));
                }
                *reference_counts
                    .entry(listing.item_instance_id.as_str())
                    .or_default() += 1;
            }
        }
        for (vault_id, vault) in &collections.locker_vaults {
            let capacity = self
                .definition
                .catalog
                .locker_vault_definitions
                .get(vault_id)
                .ok_or_else(|| StepError::new("unknown locker vault definition"))?
                .capacity;
            for (owner_character_id, contents) in &vault.lockers {
                self.actor_index_for_item_holder(&ItemHolderId::Character(
                    owner_character_id.clone(),
                ))?;
                let count = u32::try_from(contents.len())
                    .map_err(|_| StepError::new("locker item count overflow"))?;
                if count > capacity {
                    return Err(StepError::new(format!(
                        "locker vault {:?} exceeds capacity",
                        vault_id.as_str()
                    )));
                }
                let mut locker_ids = HashSet::new();
                for item_instance_id in contents {
                    if !locker_ids.insert(item_instance_id.as_str()) {
                        return Err(StepError::new("locker contains a duplicate item"));
                    }
                    if !registry_ids.contains(item_instance_id) {
                        return Err(StepError::new(format!(
                            "locker references unknown instance {item_instance_id:?}"
                        )));
                    }
                    *reference_counts
                        .entry(item_instance_id.as_str())
                        .or_default() += 1;
                }
            }
        }
        let mut reserved_offer_hands = HashSet::new();
        for (item_instance_id, offer) in &collections.item_offers {
            if !registry_ids.contains(item_instance_id) {
                return Err(StepError::new(format!(
                    "offer references unknown instance {item_instance_id:?}"
                )));
            }
            if offer.sender_character_id == offer.recipient_character_id {
                return Err(StepError::new("offer sender and recipient must differ"));
            }
            if !matches!(
                offer.source_position,
                CarriedPosition::LeftHand | CarriedPosition::RightHand
            ) {
                return Err(StepError::new("offer source must be a hand"));
            }
            let sender_index = self.actor_index_for_item_holder(&ItemHolderId::Character(
                offer.sender_character_id.clone(),
            ))?;
            self.actor_index_for_item_holder(&ItemHolderId::Character(
                offer.recipient_character_id.clone(),
            ))?;
            if !reserved_offer_hands
                .insert((offer.sender_character_id.clone(), offer.source_position))
            {
                return Err(StepError::new(
                    "multiple offers reserve the same sender hand",
                ));
            }
            let carried = &collections.carried[sender_index];
            if carried.items.contains_key(&offer.source_position) {
                return Err(StepError::new(
                    "offered source hand also contains a carried item",
                ));
            }
            let reserved_gold = match offer.source_position {
                CarriedPosition::LeftHand => carried.gold.left_hand,
                CarriedPosition::RightHand => carried.gold.right_hand,
                _ => unreachable!("offer source hand was validated above"),
            };
            if reserved_gold > 0 {
                return Err(StepError::new(
                    "offered source hand also contains carried gold",
                ));
            }
            *reference_counts
                .entry(item_instance_id.as_str())
                .or_default() += 1;
        }

        for (instance_id, instance) in instances {
            self.validate_binding(instance_id, instance)?;
        }
        for instance_id in registry_ids {
            match reference_counts
                .get(instance_id.as_str())
                .copied()
                .unwrap_or(0)
            {
                1 => {}
                0 => {
                    return Err(StepError::new(format!(
                        "item instance {instance_id:?} has no location"
                    )));
                }
                count => {
                    return Err(StepError::new(format!(
                        "item instance {instance_id:?} has {count} locations"
                    )));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_engine() -> Engine {
        crate::engine::setup::test_engine("equipment_slots")
    }

    #[test]
    fn require_single_location_rejects_missing_and_duplicate_locations() {
        assert_eq!(
            require_single_location("missing", Vec::new())
                .expect_err("missing location should fail")
                .message(),
            "item instance \"missing\" has no location"
        );
        let location = ItemLocation::Ground {
            position: WorldPosition::new("realm_0", "equipment_hall", crate::Coord { x: 1, y: 1 }),
        };
        assert_eq!(
            require_single_location("duplicate", vec![location.clone(), location])
                .expect_err("duplicate locations should fail")
                .message(),
            "item instance \"duplicate\" has 2 locations"
        );
    }

    #[test]
    fn canonical_carried_queries_and_gold_mutation_use_one_authority() {
        let mut engine = fixture_engine();
        assert_eq!(
            engine.carried_item_ids(0).expect("player carried items"),
            ["training_knife", "leather_jerkin"]
        );
        assert_eq!(
            engine.active_item_ids(0).expect("player active items"),
            ["training_knife", "leather_jerkin"]
        );
        assert!(
            engine
                .sack_item_ids(0)
                .expect("player sack items")
                .is_empty()
        );
        assert_eq!(
            engine
                .item_at_position(0, CarriedPosition::RightHand)
                .expect("right hand query"),
            Some("training_knife")
        );
        let sack = crate::model::CarriedGoldPosition::Sack;
        assert_eq!(engine.carried_gold_at(0, sack).expect("gold query"), 0);
        assert_eq!(
            engine
                .change_carried_gold_at(0, sack, 12)
                .expect("gold credit"),
            12
        );
        assert_eq!(
            engine
                .change_carried_gold_at(0, sack, -13)
                .expect_err("overdraft should fail")
                .message(),
            "insufficient gold: need 13, have 12"
        );
        assert_eq!(engine.carried_gold_at(0, sack).expect("gold rollback"), 12);
        assert_eq!(
            engine
                .change_carried_gold_at(0, sack, i64::MAX - 12)
                .expect("credit to maximum"),
            i64::MAX
        );
        assert_eq!(
            engine
                .change_carried_gold_at(0, sack, 1)
                .expect_err("overflow should fail")
                .message(),
            "carried gold overflow"
        );
        assert_eq!(
            engine.carried_gold_at(0, sack).expect("overflow rollback"),
            i64::MAX
        );
    }

    #[test]
    fn late_multi_item_failure_rolls_back_locations_and_first_touch_binding() {
        let mut engine = fixture_engine();
        let pledge_before = engine
            .item_location("pledge_blade")
            .expect("pledge blade location");
        let balm_before = engine
            .item_location("healing_balm")
            .expect("healing balm location");
        let binding_before = engine.world.item_instances["pledge_blade"].binding.clone();
        let holder = engine
            .item_holder_for_actor_index(0)
            .expect("stable player holder");

        let error = engine
            .relocate_items(&[
                ItemRelocation {
                    item_instance_id: "pledge_blade".to_string(),
                    expected: pledge_before.clone(),
                    destination: ItemLocation::Carried {
                        holder: holder.clone(),
                        position: CarriedPosition::SackItem1,
                    },
                    loot_claim: None,
                    merchant_listing: None,
                },
                ItemRelocation {
                    item_instance_id: "healing_balm".to_string(),
                    expected: balm_before.clone(),
                    destination: ItemLocation::Carried {
                        holder,
                        position: CarriedPosition::RightHand,
                    },
                    loot_claim: None,
                    merchant_listing: None,
                },
            ])
            .expect_err("occupied late destination should fail atomically");

        assert_eq!(
            error.message(),
            "carried position \"right_hand\" is occupied"
        );
        assert_eq!(
            engine
                .item_location("pledge_blade")
                .expect("pledge rollback"),
            pledge_before
        );
        assert_eq!(
            engine.item_location("healing_balm").expect("balm rollback"),
            balm_before
        );
        assert_eq!(
            engine.world.item_instances["pledge_blade"].binding,
            binding_before
        );
    }
}
