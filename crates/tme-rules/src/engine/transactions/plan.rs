//! Read-only transaction preflight.

use super::*;

impl Engine {
    pub(in crate::engine) fn plan_transaction(
        &self,
        actor_index: usize,
        source: TransactionSource,
        transaction: &Transaction,
        selected_item_instance_id: Option<&str>,
        runtime_rewards: Vec<PlannedReward>,
    ) -> Result<TransactionPlan, TransactionPlanError> {
        let actor = self.world.actors.get(actor_index).ok_or_else(|| {
            TransactionPlanError::new(ActionBlockedReasonV1::NoSuchTarget, "unknown actor")
        })?;
        let character = actor.character.as_ref();
        let carried_requirement = transaction.requirements.iter().find_map(|requirement| {
            if let TransactionRequirement::CarriedItem {
                item_definition_id,
                quantity,
            } = requirement
            {
                Some((item_definition_id.as_str(), *quantity))
            } else {
                None
            }
        });
        if carried_requirement.is_none() && selected_item_instance_id.is_some() {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::UnexpectedTransactionInput,
                "transaction does not accept an item selection",
            ));
        }

        for requirement in &transaction.requirements {
            match requirement {
                TransactionRequirement::CurrentClass { class_id } => {
                    if character
                        .is_none_or(|character| character.identity.current_class_id != *class_id)
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::WrongClass,
                            format!("transaction requires current class {class_id:?}"),
                        ));
                    }
                }
                TransactionRequirement::MinimumLevel { level } => {
                    if character.is_none_or(|character| character.progression.level < *level) {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::NotReady,
                            format!("must be at least level {level} for this transaction"),
                        ));
                    }
                }
                TransactionRequirement::ExactKarma { karma_points } => {
                    if character.is_none_or(|character| {
                        character.alignment_state.karma_points != *karma_points
                    }) {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::NotReady,
                            format!("transaction requires exactly {karma_points} karma points"),
                        ));
                    }
                }
                TransactionRequirement::ExactAlignment { alignment } => {
                    if character
                        .is_none_or(|character| character.alignment_state.alignment != *alignment)
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::NotReady,
                            format!("transaction requires {alignment:?} alignment"),
                        ));
                    }
                }
                TransactionRequirement::MinimumSkillLevel { track_id, level } => {
                    let current = character
                        .and_then(|character| {
                            character
                                .skill_ledger
                                .iter()
                                .find(|entry| entry.track_id == *track_id)
                        })
                        .map_or(0, |entry| entry.level);
                    if current < *level {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::SkillLevelTooLow,
                            format!("transaction requires {track_id:?} level {level}"),
                        ));
                    }
                }
                TransactionRequirement::MinimumCarriedGold { amount } => {
                    if actor.carried.gold.sack < *amount {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::InsufficientGold,
                            format!("transaction requires {amount} carried gold"),
                        ));
                    }
                }
                TransactionRequirement::CarriedItem {
                    item_definition_id,
                    quantity,
                } => {
                    let selected = selected_item_instance_id.ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::MissingRequiredItem,
                            "transaction requires an exact carried item selection",
                        )
                    })?;
                    let holder = actor.item_holder_id();
                    match self.item_location(selected) {
                        Ok(ItemLocation::Carried {
                            holder: actual_holder,
                            ..
                        }) if actual_holder == holder => {}
                        _ => {
                            return Err(TransactionPlanError::new(
                                ActionBlockedReasonV1::MissingRequiredItem,
                                "selected item is not carried by the actor",
                            ));
                        }
                    }
                    let instance = self.world.item_instances.get(selected).ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::MissingRequiredItem,
                            "selected item instance is missing",
                        )
                    })?;
                    if instance.definition_id != *item_definition_id {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::MissingRequiredItem,
                            "selected item has the wrong definition",
                        ));
                    }
                    if instance.quantity < *quantity {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidItemQuantity,
                            "selected item quantity is too small",
                        ));
                    }
                }
                TransactionRequirement::CarriedPositionEmpty { position } => {
                    if self
                        .item_at_position(actor_index, *position)
                        .map_err(|error| {
                            TransactionPlanError::new(
                                ActionBlockedReasonV1::OccupiedCarriedPosition,
                                error.message(),
                            )
                        })?
                        .is_some()
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::OccupiedCarriedPosition,
                            format!(
                                "{} must be empty for transaction",
                                position.label().replace('_', " ")
                            ),
                        ));
                    }
                }
                TransactionRequirement::SpellUnknown { spell_id } => {
                    if character.is_some_and(|character| {
                        character
                            .known_spells
                            .iter()
                            .any(|known| known.spell_id == *spell_id)
                    }) {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::SpellAlreadyKnown,
                            format!("spell {spell_id:?} is already known"),
                        ));
                    }
                }
                TransactionRequirement::QuestUnstarted { quest_id } => {
                    let character_id = actor.character_id.as_ref().ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::QuestStateMismatch,
                            "quest gate requires stable character identity",
                        )
                    })?;
                    if self
                        .quest_stage_for_character(character_id, quest_id)
                        .is_some()
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::QuestStateMismatch,
                            format!("quest {:?} must be unstarted", quest_id.as_str()),
                        ));
                    }
                }
                TransactionRequirement::QuestAtStage { quest_id, stage_id } => {
                    let character_id = actor.character_id.as_ref().ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::QuestStateMismatch,
                            "quest gate requires stable character identity",
                        )
                    })?;
                    if self.quest_stage_for_character(character_id, quest_id) != Some(stage_id) {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::QuestStateMismatch,
                            format!(
                                "quest {:?} must be at stage {:?}",
                                quest_id.as_str(),
                                stage_id.as_str()
                            ),
                        ));
                    }
                }
                TransactionRequirement::NpcAccompanying { npc_actor_id } => {
                    let character_id = actor.character_id.as_ref().ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::NpcNotAccompanying,
                            "NPC accompaniment requires stable character identity",
                        )
                    })?;
                    let accompanying = self.world.actors.iter().any(|candidate| {
                        candidate.id == *npc_actor_id
                            && candidate.kind == crate::model::ActorKind::Npc
                            && candidate.is_alive()
                            && candidate.location.level == actor.location.level
                            && candidate.location.position == actor.location.position
                            && candidate.npc.as_ref().is_some_and(|npc| {
                                npc.following_character_id.as_ref() == Some(character_id)
                            })
                    });
                    if !accompanying {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::NpcNotAccompanying,
                            format!("NPC {npc_actor_id:?} is not accompanying the actor"),
                        ));
                    }
                }
            }
        }

        let mut total_gold = 0_i64;
        for cost in &transaction.costs {
            match cost {
                TransactionCost::CarriedGold { amount } => {
                    total_gold = total_gold.checked_add(*amount).ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::InsufficientGold,
                            "transaction gold cost overflow",
                        )
                    })?;
                }
                TransactionCost::SelectedCarriedItem { quantity } => {
                    let selected = selected_item_instance_id.ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::MissingRequiredItem,
                            "selected-item cost requires an item selection",
                        )
                    })?;
                    if self
                        .world
                        .item_instances
                        .get(selected)
                        .is_none_or(|instance| instance.quantity < *quantity)
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidItemQuantity,
                            "selected item cannot cover the transaction cost",
                        ));
                    }
                }
            }
        }
        if actor.carried.gold.sack < total_gold {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::InsufficientGold,
                "carried gold cannot cover transaction costs",
            ));
        }

        let mut rewards = runtime_rewards;
        let mut planned_positions = HashSet::new();
        for reward in &transaction.rewards {
            match reward {
                TransactionReward::Experience { amount } => {
                    let current = character.map_or(0, |character| character.progression.experience);
                    current.checked_add(i64::from(*amount)).ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidTarget,
                            "experience reward would overflow",
                        )
                    })?;
                    rewards.push(PlannedReward::Experience { amount: *amount });
                }
                TransactionReward::Item {
                    item_instance_id,
                    item_definition_id,
                    position,
                } => {
                    if self.world.item_instances.contains_key(item_instance_id)
                        || self.item_location(item_instance_id).is_ok()
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::AlreadyComplete,
                            format!("grant item {item_instance_id:?} already exists"),
                        ));
                    }
                    let item = self
                        .definition
                        .catalog
                        .item_catalog
                        .get(item_definition_id)
                        .ok_or_else(|| {
                            TransactionPlanError::new(
                                ActionBlockedReasonV1::InvalidTarget,
                                format!("grant item definition {item_definition_id:?} is missing"),
                            )
                        })?;
                    if !item.valid_placements.contains(&position.placement_kind()) {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidTarget,
                            "grant item cannot occupy its authored position",
                        ));
                    }
                    if !planned_positions.insert(*position)
                        || self
                            .item_at_position(actor_index, *position)
                            .map_err(|error| {
                                TransactionPlanError::new(
                                    ActionBlockedReasonV1::OccupiedCarriedPosition,
                                    error.message(),
                                )
                            })?
                            .is_some()
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::OccupiedCarriedPosition,
                            format!("grant position {} is occupied", position.label()),
                        ));
                    }
                    rewards.push(PlannedReward::Item {
                        item_instance_id: item_instance_id.clone(),
                        item_definition_id: item_definition_id.clone(),
                        position: *position,
                    });
                }
                TransactionReward::Class {
                    to_class_id,
                    to_class_display,
                } => {
                    let character = character.ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::WrongClass,
                            "class reward requires a character sheet",
                        )
                    })?;
                    rewards.push(PlannedReward::Class {
                        from_class_id: character.identity.current_class_id.clone(),
                        from_class_display: character.identity.display_class.clone(),
                        to_class_id: to_class_id.clone(),
                        to_class_display: to_class_display.clone(),
                        level: character.progression.level,
                    });
                }
                TransactionReward::Spell { spell_id } => {
                    let character = character.ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidTarget,
                            "spell reward requires a character sheet",
                        )
                    })?;
                    if character
                        .known_spells
                        .iter()
                        .any(|known| known.spell_id == *spell_id)
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::SpellAlreadyKnown,
                            format!("spell {spell_id:?} is already known"),
                        ));
                    }
                    let spell = self
                        .definition
                        .catalog
                        .spells
                        .get(spell_id)
                        .ok_or_else(|| {
                            TransactionPlanError::new(
                                ActionBlockedReasonV1::InvalidTarget,
                                format!("spell reward {spell_id:?} is missing"),
                            )
                        })?;
                    rewards.push(PlannedReward::Spell {
                        spell_id: spell_id.clone(),
                        lane: spell.lane.clone().unwrap_or_default(),
                        learned_at_level: character.progression.level,
                    });
                }
                TransactionReward::QuestStage { quest_id, stage_id } => {
                    rewards.push(PlannedReward::QuestStage(self.plan_quest_stage_reward(
                        actor_index,
                        quest_id,
                        stage_id,
                    )?));
                }
            }
        }

        let mut returned_gold = 0_i64;
        for reward in &rewards {
            match reward {
                PlannedReward::ReturnedGold { amount, location } => {
                    returned_gold = returned_gold.checked_add(*amount).ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidGoldAmount,
                            "transaction gold return overflow",
                        )
                    })?;
                    if *amount <= 0 || returned_gold > total_gold || *location != actor.location {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidGoldAmount,
                            "gold return must be covered by payment and stay at the actor square",
                        ));
                    }
                }
                PlannedReward::LearningRate {
                    track_id,
                    before,
                    after,
                } => {
                    let base = self.definition.catalog.rules.skills.base_learning_rate;
                    let current = character
                        .and_then(|character| {
                            character
                                .skill_ledger
                                .iter()
                                .find(|entry| entry.track_id == *track_id)
                        })
                        .map_or(base, |entry| entry.learning_rate);
                    if !self.skill_track_is_allowed_for_actor(actor_index, track_id)
                        || current != *before
                        || *after < base
                        || *after <= *before
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidTrainingOffer,
                            "planned learning-rate reward is not currently applicable",
                        ));
                    }
                }
                PlannedReward::Experience { amount } => {
                    let current = character.map_or(0, |character| character.progression.experience);
                    if *amount <= 0 || current.checked_add(i64::from(*amount)).is_none() {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidTarget,
                            "planned experience reward is invalid or would overflow",
                        ));
                    }
                }
                PlannedReward::Item {
                    item_instance_id,
                    item_definition_id,
                    position,
                } => {
                    let definition = self
                        .definition
                        .catalog
                        .item_catalog
                        .get(item_definition_id)
                        .ok_or_else(|| {
                            TransactionPlanError::new(
                                ActionBlockedReasonV1::InvalidTarget,
                                "planned item reward definition is missing",
                            )
                        })?;
                    if self.world.item_instances.contains_key(item_instance_id)
                        || self.item_location(item_instance_id).is_ok()
                        || !definition
                            .valid_placements
                            .contains(&position.placement_kind())
                        || self
                            .item_at_position(actor_index, *position)
                            .map_err(|error| {
                                TransactionPlanError::new(
                                    ActionBlockedReasonV1::OccupiedCarriedPosition,
                                    error.message(),
                                )
                            })?
                            .is_some()
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::AlreadyComplete,
                            "planned item reward is no longer available",
                        ));
                    }
                }
                PlannedReward::Class {
                    from_class_id,
                    from_class_display,
                    to_class_id,
                    to_class_display,
                    ..
                } => {
                    let character = character.ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::WrongClass,
                            "planned class reward requires a character sheet",
                        )
                    })?;
                    if character.identity.current_class_id != *from_class_id
                        || character.identity.display_class != *from_class_display
                        || to_class_id.trim().is_empty()
                        || to_class_display.trim().is_empty()
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::WrongClass,
                            "planned class reward is not currently applicable",
                        ));
                    }
                }
                PlannedReward::Spell {
                    spell_id,
                    lane,
                    learned_at_level,
                } => {
                    let character = character.ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidTarget,
                            "planned spell reward requires a character sheet",
                        )
                    })?;
                    let spell = self
                        .definition
                        .catalog
                        .spells
                        .get(spell_id)
                        .ok_or_else(|| {
                            TransactionPlanError::new(
                                ActionBlockedReasonV1::InvalidTarget,
                                "planned spell reward is missing",
                            )
                        })?;
                    if character
                        .known_spells
                        .iter()
                        .any(|known| known.spell_id == *spell_id)
                        || spell.lane.as_deref().unwrap_or_default() != lane
                        || *learned_at_level <= 0
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::SpellAlreadyKnown,
                            "planned spell reward is not currently applicable",
                        ));
                    }
                }
                PlannedReward::CarriedGold { amount } => {
                    if *amount <= 0 || actor.carried.gold.sack.checked_add(*amount).is_none() {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidTarget,
                            "planned carried-gold reward is invalid or would overflow",
                        ));
                    }
                }
                PlannedReward::MerchantItem {
                    item_instance_id,
                    expected,
                    destination,
                    listing_price_gold,
                } => {
                    let ItemLocation::Carried { holder, position } = destination else {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidTarget,
                            "planned merchant item destination must be carried",
                        ));
                    };
                    if *listing_price_gold <= 0
                        || self.item_location(item_instance_id).as_ref() != Ok(expected)
                        || holder != &actor.item_holder_id()
                        || self
                            .item_at_position(actor_index, *position)
                            .map_err(|error| {
                                TransactionPlanError::new(
                                    ActionBlockedReasonV1::InvalidTarget,
                                    error.message(),
                                )
                            })?
                            .is_some()
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidTarget,
                            "planned merchant item reward is not currently applicable",
                        ));
                    }
                    self.validate_carried_placement(
                        item_instance_id,
                        actor_index,
                        *position,
                        &self.world.item_instances,
                    )
                    .map_err(|error| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidItemPlacement,
                            error.message(),
                        )
                    })?;
                }
                PlannedReward::BankBalance {
                    bank_id,
                    character_id,
                    amount,
                } => {
                    let bank = self.world.banks.get(bank_id).ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::NoService,
                            "planned bank reward references a missing bank",
                        )
                    })?;
                    if *amount <= 0 || bank.balance(character_id).checked_add(*amount).is_none() {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidGoldAmount,
                            "planned bank reward is invalid or would overflow",
                        ));
                    }
                }
                PlannedReward::GroundGoldPile {
                    bank_id, amount, ..
                } => {
                    if *amount <= 0 || !self.world.banks.contains_key(bank_id) {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidGoldAmount,
                            "planned ground-gold reward is invalid",
                        ));
                    }
                }
                PlannedReward::ItemAppraisal {
                    item_instance_id,
                    unit_value_gold,
                    total_value_gold,
                    ..
                } => {
                    let instance = self.item_instance(item_instance_id).map_err(|error| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::NoSuchItem,
                            error.message(),
                        )
                    })?;
                    if instance.knowledge.appraised
                        || unit_value_gold.checked_mul(u64::from(instance.quantity))
                            != Some(*total_value_gold)
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::AlreadyComplete,
                            "planned appraisal is not currently applicable",
                        ));
                    }
                }
                PlannedReward::ItemIdentification {
                    item_instance_id, ..
                } => {
                    if self
                        .item_instance(item_instance_id)
                        .map_err(|error| {
                            TransactionPlanError::new(
                                ActionBlockedReasonV1::NoSuchItem,
                                error.message(),
                            )
                        })?
                        .knowledge
                        .identified
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::AlreadyComplete,
                            "planned identification is already complete",
                        ));
                    }
                }
                PlannedReward::ItemEnchantment {
                    item_instance_id,
                    enchantment_instance_id,
                    tags,
                    remaining_rounds,
                    ..
                } => {
                    if enchantment_instance_id.trim().is_empty()
                        || tags.is_empty()
                        || remaining_rounds.is_some_and(|rounds| rounds == 0)
                        || self
                            .item_definition(item_instance_id)
                            .map_err(|error| {
                                TransactionPlanError::new(
                                    ActionBlockedReasonV1::NoSuchItem,
                                    error.message(),
                                )
                            })?
                            .weapon
                            .is_none()
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::InvalidTarget,
                            "planned weapon enchantment is invalid",
                        ));
                    }
                }
                PlannedReward::Restoration(_) => {}
                PlannedReward::NpcInteraction(_) => {}
                PlannedReward::QuestStage(quest) => {
                    let current = self
                        .quest_stage_for_character(&quest.character_id, &quest.quest_id)
                        .cloned();
                    if current != quest.before_stage_id
                        || !self
                            .definition
                            .catalog
                            .quests
                            .get(&quest.quest_id)
                            .is_some_and(|definition| {
                                definition.stages.contains_key(&quest.after_stage_id)
                            })
                    {
                        return Err(TransactionPlanError::new(
                            ActionBlockedReasonV1::QuestStateMismatch,
                            "planned quest transition is no longer applicable",
                        ));
                    }
                }
            }
        }

        Ok(TransactionPlan {
            actor_id: actor.id.clone(),
            actor_name: actor.name.clone(),
            source,
            costs: transaction
                .costs
                .iter()
                .map(|cost| match cost {
                    TransactionCost::CarriedGold { amount } => {
                        PlannedCost::CarriedGold { amount: *amount }
                    }
                    TransactionCost::SelectedCarriedItem { quantity } => {
                        PlannedCost::SelectedCarriedItem {
                            quantity: *quantity,
                        }
                    }
                })
                .collect(),
            rewards,
            selected_item_instance_id: selected_item_instance_id.map(str::to_string),
        })
    }
}
