use super::inventory::{ItemBindingChange, ItemRelocation};
use super::{CatalogItem, Engine, StepError};
use crate::events::{
    Event, ItemConsumptionReason, ItemLocationViewV1, ItemRelocationReason, SpellFizzleCause,
};
use crate::model::{
    BowReadiness, BowReadinessChangeReason, CarriedPosition, ItemEnchantmentState,
    ItemInstanceState, ItemLocation, ItemMoveDestination, ItemOperationSource, SpellItemLocation,
    WeaponHandedness,
};
use crate::view::{BurdenViewV1, ItemCapabilityViewV1, ItemInstanceViewV1, PositionedItemViewV1};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ActorBurden {
    pub item_burden: u64,
    pub coin_burden: u64,
    pub total_burden: u64,
}

#[derive(Debug, Clone)]
pub(super) struct ResolvedSpellItem {
    pub item_instance_id: String,
    pub item_definition_id: String,
    pub quantity: u32,
    pub location: SpellItemLocation,
    pub is_weapon: bool,
}

fn checked_stack_metric(
    unit_amount: Option<u64>,
    quantity: u32,
    overflow_message: &'static str,
) -> Result<Option<u64>, StepError> {
    unit_amount
        .map(|unit_amount| {
            unit_amount
                .checked_mul(u64::from(quantity))
                .ok_or_else(|| StepError::new(overflow_message))
        })
        .transpose()
}

impl Engine {
    pub(super) fn item_instance(&self, instance_id: &str) -> Result<&ItemInstanceState, StepError> {
        self.world
            .item_instances
            .get(instance_id)
            .ok_or_else(|| StepError::new(format!("unknown item instance {instance_id:?}")))
    }

    pub(super) fn item_instance_mut(
        &mut self,
        instance_id: &str,
    ) -> Result<&mut ItemInstanceState, StepError> {
        self.world
            .item_instances
            .get_mut(instance_id)
            .ok_or_else(|| StepError::new(format!("unknown item instance {instance_id:?}")))
    }

    pub(super) fn item_definition(&self, instance_id: &str) -> Result<&CatalogItem, StepError> {
        let definition_id = &self.item_instance(instance_id)?.definition_id;
        self.item_definition_by_id(definition_id).map_err(|_| {
            StepError::new(format!(
                "item instance {instance_id:?} references unknown definition {definition_id:?}"
            ))
        })
    }

    fn item_definition_by_id(&self, definition_id: &str) -> Result<&CatalogItem, StepError> {
        self.definition
            .catalog
            .item_catalog
            .get(definition_id)
            .ok_or_else(|| StepError::new(format!("unknown item definition {definition_id:?}")))
    }

    fn definition_id(&self, instance_id: &str) -> Result<&str, StepError> {
        Ok(self.item_instance(instance_id)?.definition_id.as_str())
    }

    fn item_name(&self, instance_id: &str) -> Result<String, StepError> {
        Ok(self.item_definition(instance_id)?.name.clone())
    }

    pub(super) fn apply_item_appraisal(
        &mut self,
        actor_index: usize,
        item_instance_id: &str,
        source: ItemOperationSource,
        unit_value_gold: u64,
        total_value_gold: u64,
        events: &mut Vec<Event>,
    ) -> Result<(u64, u64), StepError> {
        let instance = self.item_instance(item_instance_id)?.clone();
        let definition = self.item_definition(item_instance_id)?;
        let item_name = definition.name.clone();
        self.item_instance_mut(item_instance_id)?
            .knowledge
            .appraised = true;
        let actor = &self.world.actors[actor_index];
        events.push(Event::ItemAppraised {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            source,
            item_instance_id: item_instance_id.to_string(),
            item_definition_id: instance.definition_id,
            item_name,
            quantity: instance.quantity,
            unit_value_gold,
            total_value_gold,
        });
        Ok((unit_value_gold, total_value_gold))
    }

    pub(super) fn apply_item_identification(
        &mut self,
        actor_index: usize,
        item_instance_id: &str,
        source: ItemOperationSource,
        location: String,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let instance = self.item_instance(item_instance_id)?.clone();
        let definition = self.item_definition(item_instance_id)?;
        let item_name = definition.name.clone();
        let capability = definition.capability.clone();
        self.item_instance_mut(item_instance_id)?
            .knowledge
            .identified = true;
        let actor = &self.world.actors[actor_index];
        events.push(Event::ItemIdentified {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            source,
            item_instance_id: item_instance_id.to_string(),
            item_definition_id: instance.definition_id,
            item_name,
            quantity: instance.quantity,
            location,
            capability,
        });
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply_weapon_enchantment(
        &mut self,
        actor_index: usize,
        item_instance_id: &str,
        source: ItemOperationSource,
        enchantment_instance_id: String,
        combat_add_rating_bonus: i32,
        tags: Vec<String>,
        remaining_rounds: Option<u32>,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let instance = self.item_instance(item_instance_id)?.clone();
        if self.item_definition(item_instance_id)?.weapon.is_none() {
            return Err(StepError::new("enchantment target is not a weapon"));
        }
        let state = ItemEnchantmentState {
            enchantment_instance_id: enchantment_instance_id.clone(),
            source: source.clone(),
            item_instance_id: item_instance_id.to_string(),
            combat_add_rating_bonus,
            tags: tags.clone(),
            remaining_rounds,
            last_ticked_at: self.current_time(),
        };
        self.world
            .item_enchantments
            .retain(|existing| existing.item_instance_id != item_instance_id);
        self.world.item_enchantments.push(state);
        let actor = &self.world.actors[actor_index];
        events.push(Event::ItemEnchanted {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            source,
            item_instance_id: item_instance_id.to_string(),
            item_definition_id: instance.definition_id,
            quantity: instance.quantity,
            enchantment_instance_id,
            combat_add_rating_bonus,
            tags,
            remaining_rounds,
        });
        Ok(())
    }

    fn stack_burden_for_definition(
        &self,
        definition_id: &str,
        quantity: u32,
    ) -> Result<u64, StepError> {
        self.item_definition_by_id(definition_id)?
            .economy
            .unit_burden
            .checked_mul(u64::from(quantity))
            .ok_or_else(|| StepError::new("item stack burden overflow"))
    }

    fn stack_value_for_definition(
        &self,
        definition_id: &str,
        quantity: u32,
    ) -> Result<Option<u64>, StepError> {
        checked_stack_metric(
            self.item_definition_by_id(definition_id)?
                .economy
                .unit_value_gold,
            quantity,
            "item stack value overflow",
        )
    }

    pub(super) fn consumable_heal_for_item(&self, instance_id: &str) -> Option<i32> {
        self.item_definition(instance_id).ok()?;
        self.definition
            .catalog
            .consumable_heals
            .get(self.definition_id(instance_id).ok()?)
            .copied()
    }

    pub(super) fn item_unit_value_gold(&self, instance_id: &str) -> Result<Option<u64>, StepError> {
        Ok(self.item_definition(instance_id)?.economy.unit_value_gold)
    }

    pub(super) fn item_stack_value_gold(
        &self,
        instance_id: &str,
    ) -> Result<Option<u64>, StepError> {
        let instance = self.item_instance(instance_id)?;
        self.stack_value_for_definition(&instance.definition_id, instance.quantity)
    }

    pub(super) fn known_item_value_gold(
        &self,
        instance_id: &str,
    ) -> Result<(Option<u64>, Option<u64>), StepError> {
        if !self.item_instance(instance_id)?.knowledge.appraised {
            return Ok((None, None));
        }
        Ok((
            self.item_unit_value_gold(instance_id)?,
            self.item_stack_value_gold(instance_id)?,
        ))
    }

    pub(super) fn item_unit_burden(&self, instance_id: &str) -> Result<u64, StepError> {
        Ok(self.item_definition(instance_id)?.economy.unit_burden)
    }

    pub(super) fn item_stack_burden(&self, instance_id: &str) -> Result<u64, StepError> {
        let instance = self.item_instance(instance_id)?;
        self.item_unit_burden(instance_id)?
            .checked_mul(u64::from(instance.quantity))
            .ok_or_else(|| StepError::new("item stack burden overflow"))
    }

    pub(super) fn actor_burden(&self, actor_index: usize) -> Result<ActorBurden, StepError> {
        self.world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("unknown actor"))?;
        let mut item_burden = 0_u64;
        for instance_id in self.ordered_actor_item_ids(actor_index)? {
            item_burden = item_burden
                .checked_add(self.item_stack_burden(&instance_id)?)
                .ok_or_else(|| StepError::new("actor item burden overflow"))?;
        }
        let gold = u64::try_from(
            self.carried_gold_at(actor_index, crate::model::CarriedGoldPosition::Sack)?,
        )
        .map_err(|_| StepError::new("carried gold cannot be negative"))?;
        let coin_burden = self
            .definition
            .catalog
            .rules
            .burden
            .coin_burden_per_gold
            .checked_mul(gold)
            .ok_or_else(|| StepError::new("actor coin burden overflow"))?;
        let total_burden = item_burden
            .checked_add(coin_burden)
            .ok_or_else(|| StepError::new("actor total burden overflow"))?;
        Ok(ActorBurden {
            item_burden,
            coin_burden,
            total_burden,
        })
    }

    pub(super) fn item_instance_view(
        &self,
        item_instance_id: &str,
    ) -> Result<ItemInstanceViewV1, StepError> {
        let instance = self.item_instance(item_instance_id)?;
        let definition = self.item_definition(item_instance_id)?;
        let (known_unit_value_gold, known_stack_value_gold) =
            self.known_item_value_gold(item_instance_id)?;
        Ok(ItemInstanceViewV1 {
            item_instance_id: item_instance_id.to_string(),
            item_definition_id: instance.definition_id.clone(),
            name: definition.name.clone(),
            quantity: instance.quantity,
            identified: instance.knowledge.identified,
            appraised: instance.knowledge.appraised,
            known_unit_value_gold,
            known_stack_value_gold,
            unit_burden: self.item_unit_burden(item_instance_id)?,
            stack_burden: self.item_stack_burden(item_instance_id)?,
            binding: (&instance.binding).into(),
            bow_readiness: instance.bow_readiness,
        })
    }

    pub(super) fn positioned_item_view(
        &self,
        item_instance_id: &str,
        position: CarriedPosition,
    ) -> Result<PositionedItemViewV1, StepError> {
        let definition = self.item_definition(item_instance_id)?;
        Ok(PositionedItemViewV1 {
            item: self.item_instance_view(item_instance_id)?,
            position,
            category: definition.category.clone(),
            valid_placements: definition.valid_placements.clone(),
            capability: definition
                .capability
                .as_ref()
                .map(ItemCapabilityViewV1::from),
            armor: definition
                .armor
                .as_ref()
                .map(|armor| crate::view::ArmorDefinitionViewV1 {
                    block_rating: armor.block_rating,
                    encumbrance: armor.encumbrance,
                    cutting_reduction: armor.damage_reduction.cutting,
                    piercing_reduction: armor.damage_reduction.piercing,
                    crushing_reduction: armor.damage_reduction.crushing,
                }),
        })
    }

    pub(super) fn burden_view(&self, actor_index: usize) -> Result<BurdenViewV1, StepError> {
        self.classify_burden(actor_index)
    }

    fn checked_world_burden_total(&self) -> Result<u64, StepError> {
        let mut total = 0_u64;
        for instance_id in self.world.item_instances.keys() {
            total = total
                .checked_add(
                    self.item_stack_burden(instance_id)
                        .map_err(|_| StepError::new("world item burden overflow"))?,
                )
                .ok_or_else(|| StepError::new("world item burden overflow"))?;
        }
        for actor_index in 0..self.world.actors.len() {
            let gold = u64::try_from(
                self.carried_gold_at(actor_index, crate::model::CarriedGoldPosition::Sack)?,
            )
            .map_err(|_| StepError::new("carried gold cannot be negative"))?;
            total = total
                .checked_add(
                    self.definition
                        .catalog
                        .rules
                        .burden
                        .coin_burden_per_gold
                        .checked_mul(gold)
                        .ok_or_else(|| StepError::new("world coin burden overflow"))?,
                )
                .ok_or_else(|| StepError::new("world item burden overflow"))?;
        }
        Ok(total)
    }

    pub(super) fn validate_world_item_burden(&self) -> Result<(), StepError> {
        for actor_index in 0..self.world.actors.len() {
            let burden = self.actor_burden(actor_index)?;
            if burden.item_burden.checked_add(burden.coin_burden) != Some(burden.total_burden) {
                return Err(StepError::new("actor burden invariant failed"));
            }
        }
        self.checked_world_burden_total()?;
        Ok(())
    }

    pub(super) fn validate_prospective_transform_metrics(
        &self,
        item_instance_id: &str,
        output_item_definition_id: &str,
    ) -> Result<(), StepError> {
        let instance = self.item_instance(item_instance_id)?;
        self.stack_value_for_definition(output_item_definition_id, instance.quantity)?;
        let current_total = self.checked_world_burden_total()?;
        let old_stack_burden = self.item_stack_burden(item_instance_id)?;
        let new_stack_burden = self
            .stack_burden_for_definition(output_item_definition_id, instance.quantity)
            .map_err(|_| StepError::new("world item burden overflow"))?;
        current_total
            .checked_sub(old_stack_burden)
            .and_then(|without_old| without_old.checked_add(new_stack_burden))
            .ok_or_else(|| StepError::new("world item burden overflow"))?;
        Ok(())
    }

    pub(super) fn validate_prospective_item_instances_burden(
        &self,
        item_instances: &std::collections::BTreeMap<String, ItemInstanceState>,
    ) -> Result<(), StepError> {
        let mut total = self.checked_world_burden_total()?;
        for instance in item_instances.values() {
            total = total
                .checked_add(
                    self.stack_burden_for_definition(&instance.definition_id, instance.quantity)
                        .map_err(|_| StepError::new("world item burden overflow"))?,
                )
                .ok_or_else(|| StepError::new("world item burden overflow"))?;
        }
        Ok(())
    }

    pub(super) fn consume_one(
        &mut self,
        actor_index: usize,
        instance_id: &str,
    ) -> Result<u32, StepError> {
        self.resolve_sack_instance(actor_index, instance_id)?;
        self.consume_carried_quantity(actor_index, instance_id, 1)
    }

    pub(super) fn consume_carried_quantity(
        &mut self,
        actor_index: usize,
        instance_id: &str,
        consumed: u32,
    ) -> Result<u32, StepError> {
        if consumed == 0 {
            return Err(StepError::new("consumed item quantity must be positive"));
        }
        let holder = self.item_holder_for_actor_index(actor_index)?;
        match self.item_location(instance_id)? {
            crate::model::ItemLocation::Carried {
                holder: actual_holder,
                ..
            } if actual_holder == holder => {}
            _ => return Err(StepError::new("item instance is not carried by actor")),
        }
        let quantity = self.item_instance(instance_id)?.quantity;
        if quantity < consumed {
            return Err(StepError::new(format!(
                "item instance has quantity {quantity}, need {consumed}"
            )));
        }
        if quantity == consumed {
            self.destroy_item_instances(&[instance_id.to_string()])?;
            Ok(0)
        } else {
            let remaining = quantity - consumed;
            self.item_instance_mut(instance_id)?.quantity = remaining;
            Ok(remaining)
        }
    }

    pub(super) fn output_item_can_replace_positioned_spell_item(
        &self,
        actor_index: usize,
        item_instance_id: &str,
        output_item_definition_id: &str,
    ) -> bool {
        let Ok(Some(position)) = self.carried_position_for_item(actor_index, item_instance_id)
        else {
            return false;
        };
        self.definition
            .catalog
            .item_catalog
            .get(output_item_definition_id)
            .is_some_and(|item| item.valid_placements.contains(&position.placement_kind()))
    }

    pub(super) fn resolve_spell_item(
        &self,
        actor_index: usize,
        item_instance_id: &str,
        location: SpellItemLocation,
    ) -> Option<ResolvedSpellItem> {
        let actor = self.world.actors.get(actor_index)?;
        let holder = self.item_holder_for_actor_index(actor_index).ok()?;
        let actual = self.item_location(item_instance_id).ok()?;
        let present = match (location, actual) {
            (
                SpellItemLocation::Sack,
                ItemLocation::Carried {
                    holder: actual,
                    position,
                },
            ) => actual == holder && position.is_sack_item(),
            (
                SpellItemLocation::ActiveEquipment,
                ItemLocation::Carried {
                    holder: actual,
                    position,
                },
            ) => actual == holder && position.is_active_equipment(),
            (SpellItemLocation::GroundHere, ItemLocation::Ground { position }) => {
                position == actor.location.clone()
            }
            _ => false,
        };
        if !present {
            return None;
        }
        let definition = self.item_definition(item_instance_id).ok()?;
        let instance = self.item_instance(item_instance_id).ok()?;
        Some(ResolvedSpellItem {
            item_instance_id: item_instance_id.to_string(),
            item_definition_id: instance.definition_id.clone(),
            quantity: instance.quantity,
            location,
            is_weapon: definition.weapon.is_some(),
        })
    }

    pub(super) fn replace_spell_item(
        &mut self,
        actor_index: usize,
        item_instance_id: &str,
        location: SpellItemLocation,
        output_item_definition_id: &str,
        events: &mut Vec<Event>,
    ) -> Result<ResolvedSpellItem, StepError> {
        let old_item = self
            .resolve_spell_item(actor_index, item_instance_id, location)
            .ok_or_else(|| StepError::new("invalid_target"))?;
        let output = self
            .definition
            .catalog
            .item_catalog
            .get(output_item_definition_id)
            .ok_or_else(|| StepError::new("invalid_target"))?;
        if let ItemLocation::Carried { position, .. } = self.item_location(item_instance_id)?
            && !output.valid_placements.contains(&position.placement_kind())
        {
            return Err(StepError::new("invalid_target"));
        }
        self.validate_prospective_transform_metrics(item_instance_id, output_item_definition_id)?;
        let previous_readiness = self.item_instance(item_instance_id)?.bow_readiness;
        let output_readiness = output.weapon.as_ref().and_then(|weapon| {
            (weapon.handedness == WeaponHandedness::Bow).then_some(BowReadiness::Unnocked)
        });
        let instance = self.item_instance_mut(item_instance_id)?;
        instance.definition_id = output_item_definition_id.to_string();
        instance.knowledge = Default::default();
        instance.bow_readiness = output_readiness;
        self.world
            .item_enchantments
            .retain(|enchantment| enchantment.item_instance_id != item_instance_id);
        if previous_readiness == Some(BowReadiness::Nocked) {
            let actor = &self.world.actors[actor_index];
            events.push(Event::BowReadinessChanged {
                actor_id: actor.id.clone(),
                actor: actor.name.clone(),
                item_instance_id: item_instance_id.to_string(),
                from: BowReadiness::Nocked,
                to: BowReadiness::Unnocked,
                reason: BowReadinessChangeReason::ItemRelocated,
            });
        }
        self.validate_bow_readiness_invariants()?;
        Ok(old_item)
    }

    pub(super) fn location_view(
        &self,
        location: &ItemLocation,
    ) -> Result<ItemLocationViewV1, StepError> {
        match location {
            ItemLocation::Ground { position } => Ok(ItemLocationViewV1::Ground {
                location: position.clone(),
            }),
            ItemLocation::Carried { holder, position } => {
                let actor_index = self.actor_index_for_item_holder(holder)?;
                Ok(ItemLocationViewV1::Carried {
                    actor_id: self.world.actors[actor_index].id.clone(),
                    position: *position,
                })
            }
            ItemLocation::Corpse {
                corpse_id,
                position,
            } => Ok(ItemLocationViewV1::Corpse {
                corpse_id: corpse_id.clone(),
                position: *position,
            }),
            ItemLocation::Merchant { inventory_id } => Ok(ItemLocationViewV1::Merchant {
                service_id: inventory_id.service_id.clone(),
                capability_id: inventory_id.capability_id.clone(),
            }),
            ItemLocation::Locker {
                vault_id,
                owner_character_id,
            } => Ok(ItemLocationViewV1::Locker {
                vault_id: vault_id.as_str().to_string(),
                owner_character_id: owner_character_id.clone(),
            }),
            ItemLocation::Offered {
                sender_character_id,
                recipient_character_id,
                source_position,
            } => Ok(ItemLocationViewV1::Offered {
                sender_character_id: sender_character_id.clone(),
                recipient_character_id: recipient_character_id.clone(),
                source_position: *source_position,
            }),
        }
    }

    fn emit_binding_changes(
        &self,
        changes: Vec<ItemBindingChange>,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        for change in changes {
            let actor_index = self.actor_index_for_item_holder(&change.holder)?;
            let actor = &self.world.actors[actor_index];
            events.push(Event::ItemBound {
                actor_id: actor.id.clone(),
                actor: actor.name.clone(),
                item_instance_id: change.item_instance_id.clone(),
                item_definition_id: self.definition_id(&change.item_instance_id)?.to_string(),
                item: self.item_name(&change.item_instance_id)?,
                state: "bound".to_string(),
            });
        }
        Ok(())
    }

    pub(super) fn relocate_item_with_event(
        &mut self,
        actor_index: usize,
        item_instance_id: &str,
        expected: ItemLocation,
        destination: ItemLocation,
        reason: ItemRelocationReason,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        self.relocate_items_with_events(
            actor_index,
            vec![ItemRelocation {
                item_instance_id: item_instance_id.to_string(),
                expected,
                destination,
                loot_claim: None,
                merchant_listing: None,
            }],
            reason,
            events,
        )
    }

    pub(super) fn relocate_items_with_events(
        &mut self,
        actor_index: usize,
        relocations: Vec<ItemRelocation>,
        reason: ItemRelocationReason,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let readiness_rows = relocations
            .iter()
            .map(|relocation| {
                let source_actor = match &relocation.expected {
                    ItemLocation::Carried {
                        holder,
                        position: CarriedPosition::RightHand,
                    } => Some(self.actor_index_for_item_holder(holder)),
                    _ => None,
                };
                let destination_actor = match &relocation.destination {
                    ItemLocation::Carried {
                        holder,
                        position: CarriedPosition::LeftHand,
                    } => Some(self.actor_index_for_item_holder(holder)),
                    _ => None,
                };
                Ok((
                    relocation.item_instance_id.clone(),
                    source_actor.transpose()?,
                    destination_actor.transpose()?,
                ))
            })
            .collect::<Result<Vec<_>, StepError>>()?;
        let event_rows = relocations
            .iter()
            .map(|relocation| {
                Ok((
                    relocation.item_instance_id.clone(),
                    self.definition_id(&relocation.item_instance_id)?
                        .to_string(),
                    self.item_name(&relocation.item_instance_id)?,
                    self.item_instance(&relocation.item_instance_id)?.quantity,
                    self.location_view(&relocation.expected)?,
                    self.location_view(&relocation.destination)?,
                    relocation.loot_claim.clone(),
                ))
            })
            .collect::<Result<Vec<_>, StepError>>()?;
        let changes = self.relocate_items(&relocations)?;
        let actor = &self.world.actors[actor_index];
        for (item_instance_id, item_definition_id, item, quantity, from, to, loot_claim) in
            event_rows
        {
            events.push(Event::ItemRelocated {
                actor_id: actor.id.clone(),
                actor: actor.name.clone(),
                item_instance_id,
                item_definition_id,
                item,
                quantity,
                from,
                to,
                reason,
                loot_claim,
            });
        }
        for (item_instance_id, source_actor, destination_actor) in readiness_rows {
            if let Some(source_actor) = source_actor {
                self.unload_item_if_nocked(
                    source_actor,
                    &item_instance_id,
                    crate::model::BowReadinessChangeReason::LeftRightHand,
                    events,
                )?;
            }
            if let Some(destination_actor) = destination_actor
                && let Some(right_id) = self
                    .item_at_position(destination_actor, CarriedPosition::RightHand)?
                    .map(str::to_string)
            {
                self.unload_item_if_nocked(
                    destination_actor,
                    &right_id,
                    crate::model::BowReadinessChangeReason::LeftHandOccupied,
                    events,
                )?;
            }
        }
        self.validate_bow_readiness_invariants()?;
        self.emit_binding_changes(changes, events)
    }

    pub(super) fn apply_player_move_item(
        &mut self,
        player_index: usize,
        item_instance_id: &str,
        destination: &ItemMoveDestination,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        self.apply_actor_move_item(
            player_index,
            item_instance_id,
            destination,
            ItemRelocationReason::PlayerMove,
            events,
        )
    }

    pub(super) fn apply_actor_move_item(
        &mut self,
        actor_index: usize,
        item_instance_id: &str,
        destination: &ItemMoveDestination,
        reason: ItemRelocationReason,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let plan = self
            .validate_item_move(actor_index, item_instance_id, destination)
            .map_err(|error| StepError::new(error.message))?;
        self.relocate_item_with_event(
            actor_index,
            item_instance_id,
            plan.source,
            plan.target,
            reason,
            events,
        )?;
        Ok(())
    }

    pub(super) fn apply_player_drink(
        &mut self,
        player_index: usize,
        item_instance_id: &str,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        self.apply_actor_drink(player_index, item_instance_id, events)
    }

    pub(super) fn apply_actor_drink(
        &mut self,
        actor_index: usize,
        item_instance_id: &str,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let heal_per_round = self
            .consumable_heal_for_item(item_instance_id)
            .ok_or_else(|| {
                StepError::new(format!(
                    "drink target {item_instance_id:?} is not drinkable"
                ))
            })?;
        let item_name = self.item_name(item_instance_id)?;
        let item_definition_id = self.definition_id(item_instance_id)?.to_string();
        let remaining_quantity = self
            .consume_one(actor_index, item_instance_id)
            .map_err(|error| StepError::new(format!("drink {}", error.message())))?;
        events.push(Event::ItemConsumed {
            actor_id: self.world.actors[actor_index].id.clone(),
            actor: self.world.actors[actor_index].name.clone(),
            item_instance_id: item_instance_id.to_string(),
            item_definition_id,
            item: item_name,
            quantity_consumed: 1,
            remaining_quantity,
            reason: ItemConsumptionReason::Drink,
            location: self.world.actors[actor_index].location.clone(),
        });
        self.fizzle_warmed_spell(actor_index, SpellFizzleCause::HealingBalm, events);
        self.world.actors[actor_index].balm_effect = Some(crate::model::BalmEffectState {
            heal_per_round,
            restored: 0,
            budget: self.world.actors[actor_index].max_hp(),
            last_tick_at: crate::model::LogicalTime::ZERO,
        });
        self.apply_balm_tick(actor_index, events)?;
        Ok(())
    }

    pub(super) fn apply_player_show_sack(
        &self,
        player_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let gold = self.carried_gold_at(player_index, crate::model::CarriedGoldPosition::Sack)?;
        let actor = &self.world.actors[player_index];
        let items = actor
            .carried
            .items
            .iter()
            .filter(|(position, _)| position.is_sack_item())
            .map(|(position, instance_id)| self.positioned_item_view(instance_id, *position))
            .collect::<Result<Vec<_>, StepError>>()?;
        events.push(Event::SackShown {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            items,
            gold,
        });
        Ok(())
    }
}
