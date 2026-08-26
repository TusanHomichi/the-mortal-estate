//! Provider-neutral transaction planning and coordinated commit.

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
        if service.position().level != actor.location.level
            || service.position().position != actor.location.position
        {
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

    pub(super) fn plan_transaction(
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

        for reward in &rewards {
            match reward {
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

    pub(super) fn commit_transaction(
        &mut self,
        actor_index: usize,
        plan: TransactionPlan,
    ) -> Result<TransactionCommitReceipt, StepError> {
        let mut costs = Vec::new();
        let mut rewards = Vec::new();
        let mut delegated_events = Vec::new();

        for cost in &plan.costs {
            match cost {
                PlannedCost::CarriedGold { amount } => {
                    let before =
                        self.carried_gold_at(actor_index, crate::model::CarriedGoldPosition::Sack)?;
                    let after = self.change_carried_gold_at(
                        actor_index,
                        crate::model::CarriedGoldPosition::Sack,
                        -*amount,
                    )?;
                    costs.push(TransactionCostReceiptV1::CarriedGold {
                        amount: *amount,
                        position: crate::model::CarriedGoldPosition::Sack,
                        before,
                        after,
                    });
                    delegated_events.push(Event::GoldChanged {
                        actor_id: plan.actor_id.clone(),
                        actor: plan.actor_name.clone(),
                        amount: -*amount,
                        new_total: after,
                    });
                }
                PlannedCost::SelectedCarriedItem { quantity } => {
                    let instance_id =
                        plan.selected_item_instance_id.as_deref().ok_or_else(|| {
                            StepError::new("captured selected-item cost has no item instance")
                        })?;
                    let definition_id = self.item_instance(instance_id)?.definition_id.clone();
                    let remaining =
                        self.consume_carried_quantity(actor_index, instance_id, *quantity)?;
                    costs.push(TransactionCostReceiptV1::SelectedCarriedItem {
                        item_instance_id: instance_id.to_string(),
                        item_definition_id: definition_id,
                        consumed_quantity: *quantity,
                        remaining_quantity: remaining,
                    });
                }
                PlannedCost::MerchantItem {
                    item_instance_id,
                    expected,
                    destination,
                    listing,
                } => {
                    let instance = self.item_instance(item_instance_id)?.clone();
                    let from = self.location_view(expected)?;
                    let to = self.location_view(destination)?;
                    self.relocate_items_with_events(
                        actor_index,
                        vec![super::inventory::ItemRelocation {
                            item_instance_id: item_instance_id.clone(),
                            expected: expected.clone(),
                            destination: destination.clone(),
                            loot_claim: None,
                            merchant_listing: Some(listing.clone()),
                        }],
                        crate::events::ItemRelocationReason::MerchantSale,
                        &mut delegated_events,
                    )?;
                    costs.push(TransactionCostReceiptV1::MerchantItem {
                        item_instance_id: item_instance_id.clone(),
                        item_definition_id: instance.definition_id,
                        quantity: instance.quantity,
                        from,
                        to,
                        pawn_listing_price_gold: listing.price_gold,
                    });
                }
                PlannedCost::GroundGoldPile {
                    gold_pile_id,
                    amount,
                    bank_id,
                    character_id,
                } => {
                    let pile = self.consume_ground_gold_pile(gold_pile_id)?;
                    if pile.amount != *amount {
                        return Err(StepError::new(
                            "captured bank deposit pile amount changed before commit",
                        ));
                    }
                    let from = crate::events::GoldLocationViewV1::Ground {
                        gold_pile_id: pile.id.clone(),
                        location: pile.location.clone(),
                    };
                    delegated_events.push(Event::GoldRelocated {
                        actor_id: plan.actor_id.clone(),
                        actor: plan.actor_name.clone(),
                        amount: *amount,
                        from: from.clone(),
                        to: crate::events::GoldLocationViewV1::Bank {
                            bank_id: bank_id.as_str().to_string(),
                            character_id: character_id.clone(),
                        },
                        reason: crate::events::GoldRelocationReason::BankDeposit,
                        loot_claim: pile.loot_claim,
                    });
                    costs.push(TransactionCostReceiptV1::GroundGoldPile {
                        gold_pile_id: pile.id,
                        amount: *amount,
                        from,
                    });
                }
                PlannedCost::BankBalance {
                    bank_id,
                    character_id,
                    amount,
                } => {
                    let bank = self
                        .world
                        .banks
                        .get_mut(bank_id)
                        .ok_or_else(|| StepError::new("captured bank state is missing"))?;
                    let before = bank.balance(character_id);
                    let after = before
                        .checked_sub(*amount)
                        .filter(|after| *after >= 0)
                        .ok_or_else(|| StepError::new("captured bank balance cannot cover cost"))?;
                    bank.balances.insert(character_id.clone(), after);
                    costs.push(TransactionCostReceiptV1::BankBalance {
                        bank_id: bank_id.as_str().to_string(),
                        character_id: character_id.clone(),
                        amount: *amount,
                        before,
                        after,
                    });
                    delegated_events.push(Event::BankBalanceChanged {
                        actor_id: plan.actor_id.clone(),
                        actor: plan.actor_name.clone(),
                        bank_id: bank_id.as_str().to_string(),
                        character_id: character_id.clone(),
                        amount: *amount,
                        before,
                        after,
                        reason: crate::events::BankBalanceChangeReasonV1::Withdrawal,
                    });
                }
            }
        }

        for reward in &plan.rewards {
            match reward {
                PlannedReward::LearningRate {
                    track_id,
                    before,
                    after,
                } => {
                    self.set_skill_learning_rate(actor_index, track_id, *after)?;
                    rewards.push(TransactionRewardReceiptV1::LearningRate {
                        track_id: track_id.clone(),
                        before: *before,
                        after: *after,
                    });
                }
                PlannedReward::Experience { amount } => {
                    let events =
                        super::progression::award_character_experience(self, actor_index, *amount)?;
                    let total_xp = self.world.actors[actor_index]
                        .character
                        .as_ref()
                        .map_or(0, |character| character.progression.experience);
                    delegated_events.extend(events);
                    rewards.push(TransactionRewardReceiptV1::Experience {
                        amount: *amount,
                        total_xp,
                    });
                }
                PlannedReward::Item {
                    item_instance_id,
                    item_definition_id,
                    position,
                } => {
                    let holder = self.item_holder_for_actor_index(actor_index)?;
                    let mut instances = BTreeMap::new();
                    instances.insert(
                        item_instance_id.clone(),
                        ItemInstanceState {
                            definition_id: item_definition_id.clone(),
                            quantity: 1,
                            knowledge: ItemKnowledgeState::default(),
                            binding: ItemBindingState::Unrestricted,
                            bow_readiness: None,
                        },
                    );
                    self.register_item_instances(
                        instances,
                        &[(
                            item_instance_id.clone(),
                            ItemLocation::Carried {
                                holder,
                                position: *position,
                            },
                        )],
                    )?;
                    rewards.push(TransactionRewardReceiptV1::Item {
                        item_instance_id: item_instance_id.clone(),
                        item_definition_id: item_definition_id.clone(),
                        position: *position,
                        quantity: 1,
                    });
                }
                PlannedReward::Class {
                    from_class_id,
                    from_class_display,
                    to_class_id,
                    to_class_display,
                    level,
                } => {
                    self.apply_transaction_class_reward(
                        actor_index,
                        from_class_id,
                        to_class_id,
                        to_class_display,
                        *level,
                    )?;
                    rewards.push(TransactionRewardReceiptV1::Class {
                        from_class_id: from_class_id.clone(),
                        from_class_display: from_class_display.clone(),
                        to_class_id: to_class_id.clone(),
                        to_class_display: to_class_display.clone(),
                    });
                }
                PlannedReward::Spell {
                    spell_id,
                    lane,
                    learned_at_level,
                } => {
                    self.apply_transaction_spell_reward(
                        actor_index,
                        spell_id,
                        lane,
                        *learned_at_level,
                    )?;
                    rewards.push(TransactionRewardReceiptV1::Spell {
                        spell_id: spell_id.clone(),
                        learned_at_level: *learned_at_level,
                    });
                }
                PlannedReward::CarriedGold { amount } => {
                    let before =
                        self.carried_gold_at(actor_index, crate::model::CarriedGoldPosition::Sack)?;
                    let after = self.change_carried_gold_at(
                        actor_index,
                        crate::model::CarriedGoldPosition::Sack,
                        *amount,
                    )?;
                    delegated_events.push(Event::GoldChanged {
                        actor_id: plan.actor_id.clone(),
                        actor: plan.actor_name.clone(),
                        amount: *amount,
                        new_total: after,
                    });
                    rewards.push(TransactionRewardReceiptV1::CarriedGold {
                        amount: *amount,
                        position: crate::model::CarriedGoldPosition::Sack,
                        before,
                        after,
                    });
                }
                PlannedReward::MerchantItem {
                    item_instance_id,
                    expected,
                    destination,
                    listing_price_gold,
                } => {
                    let instance = self.item_instance(item_instance_id)?.clone();
                    let from = self.location_view(expected)?;
                    let to = self.location_view(destination)?;
                    self.relocate_items_with_events(
                        actor_index,
                        vec![super::inventory::ItemRelocation {
                            item_instance_id: item_instance_id.clone(),
                            expected: expected.clone(),
                            destination: destination.clone(),
                            loot_claim: None,
                            merchant_listing: None,
                        }],
                        crate::events::ItemRelocationReason::MerchantPurchase,
                        &mut delegated_events,
                    )?;
                    rewards.push(TransactionRewardReceiptV1::MerchantItem {
                        item_instance_id: item_instance_id.clone(),
                        item_definition_id: instance.definition_id,
                        quantity: instance.quantity,
                        from,
                        to,
                        listing_price_gold: *listing_price_gold,
                    });
                }
                PlannedReward::ItemAppraisal {
                    item_instance_id,
                    source,
                    unit_value_gold,
                    total_value_gold,
                } => {
                    let definition_id = self.item_instance(item_instance_id)?.definition_id.clone();
                    self.apply_item_appraisal(
                        actor_index,
                        item_instance_id,
                        source.clone(),
                        *unit_value_gold,
                        *total_value_gold,
                        &mut delegated_events,
                    )?;
                    rewards.push(TransactionRewardReceiptV1::ItemAppraised {
                        item_instance_id: item_instance_id.clone(),
                        item_definition_id: definition_id,
                        unit_value_gold: *unit_value_gold,
                        total_value_gold: *total_value_gold,
                    });
                }
                PlannedReward::ItemIdentification {
                    item_instance_id,
                    source,
                    location,
                } => {
                    let definition_id = self.item_instance(item_instance_id)?.definition_id.clone();
                    self.apply_item_identification(
                        actor_index,
                        item_instance_id,
                        source.clone(),
                        location.clone(),
                        &mut delegated_events,
                    )?;
                    rewards.push(TransactionRewardReceiptV1::ItemIdentified {
                        item_instance_id: item_instance_id.clone(),
                        item_definition_id: definition_id,
                    });
                }
                PlannedReward::ItemEnchantment {
                    item_instance_id,
                    source,
                    enchantment_instance_id,
                    combat_add_rating_bonus,
                    tags,
                    remaining_rounds,
                } => {
                    let definition_id = self.item_instance(item_instance_id)?.definition_id.clone();
                    self.apply_weapon_enchantment(
                        actor_index,
                        item_instance_id,
                        source.clone(),
                        enchantment_instance_id.clone(),
                        *combat_add_rating_bonus,
                        tags.clone(),
                        *remaining_rounds,
                        &mut delegated_events,
                    )?;
                    rewards.push(TransactionRewardReceiptV1::ItemEnchanted {
                        item_instance_id: item_instance_id.clone(),
                        item_definition_id: definition_id,
                        enchantment_instance_id: enchantment_instance_id.clone(),
                        combat_add_rating_bonus: *combat_add_rating_bonus,
                        tags: tags.clone(),
                        remaining_rounds: *remaining_rounds,
                    });
                }
                PlannedReward::Restoration(restoration) => {
                    let (reward, mut events) =
                        self.apply_restoration_reward(restoration.clone())?;
                    delegated_events.append(&mut events);
                    rewards.push(reward);
                }
                PlannedReward::NpcInteraction(interaction) => {
                    let (reward, mut events) = self.apply_npc_interaction_reward(interaction)?;
                    delegated_events.append(&mut events);
                    rewards.push(reward);
                }
                PlannedReward::QuestStage(quest) => {
                    let (reward, event) = self.apply_quest_transition(quest)?;
                    delegated_events.push(event);
                    rewards.push(reward);
                }
                PlannedReward::BankBalance {
                    bank_id,
                    character_id,
                    amount,
                } => {
                    let bank = self
                        .world
                        .banks
                        .get_mut(bank_id)
                        .ok_or_else(|| StepError::new("captured bank state is missing"))?;
                    let before = bank.balance(character_id);
                    let after = before
                        .checked_add(*amount)
                        .ok_or_else(|| StepError::new("bank balance overflow"))?;
                    bank.balances.insert(character_id.clone(), after);
                    rewards.push(TransactionRewardReceiptV1::BankBalance {
                        bank_id: bank_id.as_str().to_string(),
                        character_id: character_id.clone(),
                        amount: *amount,
                        before,
                        after,
                    });
                    delegated_events.push(Event::BankBalanceChanged {
                        actor_id: plan.actor_id.clone(),
                        actor: plan.actor_name.clone(),
                        bank_id: bank_id.as_str().to_string(),
                        character_id: character_id.clone(),
                        amount: *amount,
                        before,
                        after,
                        reason: crate::events::BankBalanceChangeReasonV1::Deposit,
                    });
                }
                PlannedReward::GroundGoldPile {
                    bank_id,
                    character_id,
                    amount,
                    location,
                } => {
                    let pile = self.create_ground_gold_pile(*amount, location.clone(), None)?;
                    let to = crate::events::GoldLocationViewV1::Ground {
                        gold_pile_id: pile.id.clone(),
                        location: pile.location.clone(),
                    };
                    delegated_events.push(Event::GoldRelocated {
                        actor_id: plan.actor_id.clone(),
                        actor: plan.actor_name.clone(),
                        amount: *amount,
                        from: crate::events::GoldLocationViewV1::Bank {
                            bank_id: bank_id.as_str().to_string(),
                            character_id: character_id.clone(),
                        },
                        to: to.clone(),
                        reason: crate::events::GoldRelocationReason::BankWithdrawal,
                        loot_claim: None,
                    });
                    rewards.push(TransactionRewardReceiptV1::GroundGoldPile {
                        gold_pile_id: pile.id,
                        amount: *amount,
                        to,
                    });
                }
            }
        }

        Ok(TransactionCommitReceipt {
            source: plan.source,
            costs,
            rewards,
            delegated_events,
        })
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
