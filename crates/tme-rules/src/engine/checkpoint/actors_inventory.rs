use super::*;

copy_checkpoint!(MagicResistanceCheckpointV2, ActorMagicResistanceState, {
    natural_save_twentieths: u32,
    evidence_state: MagicRuleEvidenceState,
});
copy_checkpoint!(PhysicalAffinityCheckpointV2, PhysicalDamageAffinity, {
    cutting_numerator: u32,
    cutting_denominator: u32,
    piercing_numerator: u32,
    piercing_denominator: u32,
    crushing_numerator: u32,
    crushing_denominator: u32,
});
copy_checkpoint!(ResourceActivityCheckpointV2, ActorResourceActivity, {
    last_active_at: Option<LogicalTime>,
    last_recovered_at: LogicalTime,
});
copy_checkpoint!(ActorTimingCheckpointV2, ActorTimingState, {
    ready_at: LogicalTime,
    tie_break_order: u64,
});
copy_checkpoint!(EcologyOriginCheckpointV2, EcologyActorOrigin, {
    site_id: String,
    member_id: String,
    generation: u32,
});
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ItemKnowledgeCheckpointV2 {
    pub(super) identified: bool,
    pub(super) appraised: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ItemInstanceCheckpointV2 {
    pub(super) definition_id: String,
    pub(super) quantity: u32,
    pub(super) knowledge: ItemKnowledgeCheckpointV2,
    pub(super) binding: ItemBindingState,
    pub(super) bow_readiness: Option<BowReadiness>,
}

impl From<&ItemInstanceState> for ItemInstanceCheckpointV2 {
    fn from(value: &ItemInstanceState) -> Self {
        Self {
            definition_id: value.definition_id.clone(),
            quantity: value.quantity,
            knowledge: ItemKnowledgeCheckpointV2 {
                identified: value.knowledge.identified,
                appraised: value.knowledge.appraised,
            },
            binding: value.binding.clone(),
            bow_readiness: value.bow_readiness,
        }
    }
}

impl From<ItemInstanceCheckpointV2> for ItemInstanceState {
    fn from(value: ItemInstanceCheckpointV2) -> Self {
        Self {
            definition_id: value.definition_id,
            quantity: value.quantity,
            knowledge: ItemKnowledgeState {
                identified: value.knowledge.identified,
                appraised: value.knowledge.appraised,
            },
            binding: value.binding,
            bow_readiness: value.bow_readiness,
        }
    }
}
copy_checkpoint!(ServiceInstanceCheckpointV2, ServiceInstanceState, {
    id: String,
    definition_id: String,
    position: WorldPosition,
});
copy_checkpoint!(BankCheckpointV2, BankState, {
    balances: BTreeMap<CharacterId, i64>,
});
copy_checkpoint!(LockerCheckpointV2, LockerVaultState, {
    lockers: BTreeMap<CharacterId, Vec<String>>,
});
copy_checkpoint!(ItemOfferCheckpointV2, ItemOfferState, {
    sender_character_id: CharacterId,
    recipient_character_id: CharacterId,
    source_position: CarriedPosition,
});
copy_checkpoint!(GroundItemCheckpointV2, GroundItem, {
    item_instance_id: String,
    location: WorldPosition,
    loot_claim: Option<LootClaim>,
});
copy_checkpoint!(CorpseCheckpointV2, CorpseState, {
    id: CorpseId,
    origin_actor_id: ActorId,
    origin_character_id: Option<CharacterId>,
    origin_kind: ActorKind,
    origin_name: String,
    location: WorldPosition,
    created_at: LogicalTime,
    sequence: u64,
    searched: bool,
    loot_claim: Option<LootClaim>,
    contents: BTreeMap<CarriedPosition, String>,
    gold: i64,
});
copy_checkpoint!(GroundGoldCheckpointV2, GroundGoldPile, {
    id: GoldPileId,
    amount: i64,
    location: WorldPosition,
    loot_claim: Option<LootClaim>,
});
copy_checkpoint!(TileEffectCheckpointV3, TileEffectState, {
    instance_id: String,
    effect_id: String,
    source: ActiveEffectSource,
    source_actor_id: Option<ActorId>,
    hostile_authority: Option<HostileEffectAuthority>,
    location: WorldPosition,
    kind: String,
    tags: Vec<String>,
    potency: i32,
    remaining_rounds: Option<u32>,
    passability: Option<String>,
    sight: Option<String>,
    hazard: Option<String>,
    move_cost: Option<i32>,
    tick_interval_rounds: u32,
    last_ticked_at: LogicalTime,
});
copy_checkpoint!(ItemEnchantmentCheckpointV2, ItemEnchantmentState, {
    enchantment_instance_id: String,
    source: ItemOperationSource,
    item_instance_id: String,
    combat_add_rating_bonus: i32,
    tags: Vec<String>,
    remaining_rounds: Option<u32>,
    last_ticked_at: LogicalTime,
});
copy_checkpoint!(PortalCheckpointV2, PortalTransitionState, {
    instance_id: String,
    source_spell_id: String,
    source_actor_id: ActorId,
    location: WorldPosition,
    target: WorldPosition,
    two_way: bool,
    remaining_rounds: Option<u32>,
    last_ticked_at: LogicalTime,
});
copy_checkpoint!(ConcealedCheckpointV2, ConcealedTransitionState, {
    instance_id: String,
    source_spell_id: String,
    source_actor_id: ActorId,
    location: WorldPosition,
    remaining_rounds: u32,
    last_ticked_at: LogicalTime,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BalmCheckpointV2 {
    pub(super) heal_per_round: i32,
    pub(super) restored: i32,
    pub(super) budget: i32,
    pub(super) last_tick_at: LogicalTime,
}

impl From<&BalmEffectState> for BalmCheckpointV2 {
    fn from(value: &BalmEffectState) -> Self {
        Self {
            heal_per_round: value.heal_per_round,
            restored: value.restored,
            budget: value.budget,
            last_tick_at: value.last_tick_at,
        }
    }
}

impl From<BalmCheckpointV2> for BalmEffectState {
    fn from(value: BalmCheckpointV2) -> Self {
        Self {
            heal_per_round: value.heal_per_round,
            restored: value.restored,
            budget: value.budget,
            last_tick_at: value.last_tick_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CarriedCheckpointV2 {
    pub(super) items: BTreeMap<CarriedPosition, String>,
    pub(super) gold: CarriedGold,
}

impl From<&CarriedLayout> for CarriedCheckpointV2 {
    fn from(value: &CarriedLayout) -> Self {
        Self {
            items: value.items.clone(),
            gold: value.gold,
        }
    }
}

impl From<CarriedCheckpointV2> for CarriedLayout {
    fn from(value: CarriedCheckpointV2) -> Self {
        Self {
            items: value.items,
            gold: value.gold,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum AwarenessPolicyCheckpointV2 {
    Unrestricted,
    LineOfSightMemory { memory_opportunities: u32 },
}

impl From<ActorAwarenessPolicy> for AwarenessPolicyCheckpointV2 {
    fn from(value: ActorAwarenessPolicy) -> Self {
        match value {
            ActorAwarenessPolicy::Unrestricted => Self::Unrestricted,
            ActorAwarenessPolicy::LineOfSightMemory {
                memory_opportunities,
            } => Self::LineOfSightMemory {
                memory_opportunities,
            },
        }
    }
}

impl From<AwarenessPolicyCheckpointV2> for ActorAwarenessPolicy {
    fn from(value: AwarenessPolicyCheckpointV2) -> Self {
        match value {
            AwarenessPolicyCheckpointV2::Unrestricted => Self::Unrestricted,
            AwarenessPolicyCheckpointV2::LineOfSightMemory {
                memory_opportunities,
            } => Self::LineOfSightMemory {
                memory_opportunities,
            },
        }
    }
}

copy_checkpoint!(RememberedCheckpointV2, RememberedHostile, {
    actor_id: ActorId,
    last_seen: WorldPosition,
    remaining_opportunities: u32,
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AiCheckpointV3 {
    pub(super) behavior: ActorAiBehavior,
    pub(super) cadence_units: u32,
    pub(super) aggro_radius: u32,
    pub(super) leash_range: u32,
    pub(super) policy: AwarenessPolicyCheckpointV2,
    pub(super) remembered: Option<RememberedCheckpointV2>,
    pub(super) physical_attack_modes: Vec<PhysicalAttackMode>,
    pub(super) returning_home: bool,
}

impl From<&ActorAiState> for AiCheckpointV3 {
    fn from(value: &ActorAiState) -> Self {
        Self {
            behavior: value.behavior,
            cadence_units: value.cadence_units,
            aggro_radius: value.aggro_radius,
            leash_range: value.leash_range,
            policy: value.awareness.policy.into(),
            remembered: value.awareness.remembered.as_ref().map(Into::into),
            physical_attack_modes: value.physical_attack_modes.clone(),
            returning_home: value.returning_home,
        }
    }
}

impl From<AiCheckpointV3> for ActorAiState {
    fn from(value: AiCheckpointV3) -> Self {
        Self {
            behavior: value.behavior,
            cadence_units: value.cadence_units,
            aggro_radius: value.aggro_radius,
            leash_range: value.leash_range,
            awareness: ActorAwarenessState {
                policy: value.policy.into(),
                remembered: value.remembered.map(Into::into),
            },
            physical_attack_modes: value.physical_attack_modes,
            returning_home: value.returning_home,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NpcCheckpointV2 {
    pub(super) follow_cadence_units: u32,
    pub(super) interactions: Vec<NpcInteractionCheckpointV2>,
    pub(super) following_character_id: Option<CharacterId>,
}

impl From<&NpcState> for NpcCheckpointV2 {
    fn from(value: &NpcState) -> Self {
        Self {
            follow_cadence_units: value.follow_cadence_units,
            interactions: value.interactions.iter().map(Into::into).collect(),
            following_character_id: value.following_character_id.clone(),
        }
    }
}

impl From<NpcCheckpointV2> for NpcState {
    fn from(value: NpcCheckpointV2) -> Self {
        Self {
            follow_cadence_units: value.follow_cadence_units,
            interactions: value.interactions.into_iter().map(Into::into).collect(),
            following_character_id: value.following_character_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NpcInteractionCheckpointV2 {
    pub(super) transaction: TransactionCheckpointV2,
    pub(super) response: String,
    pub(super) outcome: NpcInteractionOutcome,
}

impl From<&NpcInteraction> for NpcInteractionCheckpointV2 {
    fn from(value: &NpcInteraction) -> Self {
        Self {
            transaction: (&value.transaction).into(),
            response: value.response.clone(),
            outcome: value.outcome.clone(),
        }
    }
}

impl From<NpcInteractionCheckpointV2> for NpcInteraction {
    fn from(value: NpcInteractionCheckpointV2) -> Self {
        Self {
            transaction: value.transaction.into(),
            response: value.response,
            outcome: value.outcome,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum RequirementCheckpointV2 {
    CurrentClass {
        class_id: String,
    },
    MinimumLevel {
        level: i32,
    },
    ExactKarma {
        karma_points: u32,
    },
    ExactAlignment {
        alignment: CharacterAlignment,
    },
    MinimumSkillLevel {
        track_id: String,
        level: u8,
    },
    MinimumCarriedGold {
        amount: i64,
    },
    CarriedItem {
        item_definition_id: String,
        quantity: u32,
    },
    CarriedPositionEmpty {
        position: CarriedPosition,
    },
    SpellUnknown {
        spell_id: String,
    },
    QuestUnstarted {
        quest_id: QuestId,
    },
    QuestAtStage {
        quest_id: QuestId,
        stage_id: QuestStageId,
    },
    NpcAccompanying {
        npc_actor_id: ActorId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum CostCheckpointV2 {
    CarriedGold { amount: i64 },
    SelectedCarriedItem { quantity: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum RewardCheckpointV2 {
    Experience {
        amount: i32,
    },
    Item {
        item_instance_id: String,
        item_definition_id: String,
        position: CarriedPosition,
    },
    Class {
        to_class_id: String,
        to_class_display: String,
    },
    Spell {
        spell_id: String,
    },
    QuestStage {
        quest_id: QuestId,
        stage_id: QuestStageId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TransactionCheckpointV2 {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) requirements: Vec<RequirementCheckpointV2>,
    pub(super) costs: Vec<CostCheckpointV2>,
    pub(super) rewards: Vec<RewardCheckpointV2>,
}

impl From<&Transaction> for TransactionCheckpointV2 {
    fn from(value: &Transaction) -> Self {
        Self {
            id: value.id.clone(),
            label: value.label.clone(),
            requirements: value
                .requirements
                .iter()
                .map(requirement_to_checkpoint)
                .collect(),
            costs: value.costs.iter().map(cost_to_checkpoint).collect(),
            rewards: value.rewards.iter().map(reward_to_checkpoint).collect(),
        }
    }
}

impl From<TransactionCheckpointV2> for Transaction {
    fn from(value: TransactionCheckpointV2) -> Self {
        Self {
            id: value.id,
            label: value.label,
            requirements: value.requirements.into_iter().map(Into::into).collect(),
            costs: value.costs.into_iter().map(Into::into).collect(),
            rewards: value.rewards.into_iter().map(Into::into).collect(),
        }
    }
}

pub(super) fn requirement_to_checkpoint(value: &TransactionRequirement) -> RequirementCheckpointV2 {
    match value {
        TransactionRequirement::CurrentClass { class_id } => {
            RequirementCheckpointV2::CurrentClass {
                class_id: class_id.clone(),
            }
        }
        TransactionRequirement::MinimumLevel { level } => {
            RequirementCheckpointV2::MinimumLevel { level: *level }
        }
        TransactionRequirement::ExactKarma { karma_points } => {
            RequirementCheckpointV2::ExactKarma {
                karma_points: *karma_points,
            }
        }
        TransactionRequirement::ExactAlignment { alignment } => {
            RequirementCheckpointV2::ExactAlignment {
                alignment: *alignment,
            }
        }
        TransactionRequirement::MinimumSkillLevel { track_id, level } => {
            RequirementCheckpointV2::MinimumSkillLevel {
                track_id: track_id.clone(),
                level: *level,
            }
        }
        TransactionRequirement::MinimumCarriedGold { amount } => {
            RequirementCheckpointV2::MinimumCarriedGold { amount: *amount }
        }
        TransactionRequirement::CarriedItem {
            item_definition_id,
            quantity,
        } => RequirementCheckpointV2::CarriedItem {
            item_definition_id: item_definition_id.clone(),
            quantity: *quantity,
        },
        TransactionRequirement::CarriedPositionEmpty { position } => {
            RequirementCheckpointV2::CarriedPositionEmpty {
                position: *position,
            }
        }
        TransactionRequirement::SpellUnknown { spell_id } => {
            RequirementCheckpointV2::SpellUnknown {
                spell_id: spell_id.clone(),
            }
        }
        TransactionRequirement::QuestUnstarted { quest_id } => {
            RequirementCheckpointV2::QuestUnstarted {
                quest_id: quest_id.clone(),
            }
        }
        TransactionRequirement::QuestAtStage { quest_id, stage_id } => {
            RequirementCheckpointV2::QuestAtStage {
                quest_id: quest_id.clone(),
                stage_id: stage_id.clone(),
            }
        }
        TransactionRequirement::NpcAccompanying { npc_actor_id } => {
            RequirementCheckpointV2::NpcAccompanying {
                npc_actor_id: npc_actor_id.clone(),
            }
        }
    }
}

impl From<RequirementCheckpointV2> for TransactionRequirement {
    fn from(value: RequirementCheckpointV2) -> Self {
        match value {
            RequirementCheckpointV2::CurrentClass { class_id } => Self::CurrentClass { class_id },
            RequirementCheckpointV2::MinimumLevel { level } => Self::MinimumLevel { level },
            RequirementCheckpointV2::ExactKarma { karma_points } => {
                Self::ExactKarma { karma_points }
            }
            RequirementCheckpointV2::ExactAlignment { alignment } => {
                Self::ExactAlignment { alignment }
            }
            RequirementCheckpointV2::MinimumSkillLevel { track_id, level } => {
                Self::MinimumSkillLevel { track_id, level }
            }
            RequirementCheckpointV2::MinimumCarriedGold { amount } => {
                Self::MinimumCarriedGold { amount }
            }
            RequirementCheckpointV2::CarriedItem {
                item_definition_id,
                quantity,
            } => Self::CarriedItem {
                item_definition_id,
                quantity,
            },
            RequirementCheckpointV2::CarriedPositionEmpty { position } => {
                Self::CarriedPositionEmpty { position }
            }
            RequirementCheckpointV2::SpellUnknown { spell_id } => Self::SpellUnknown { spell_id },
            RequirementCheckpointV2::QuestUnstarted { quest_id } => {
                Self::QuestUnstarted { quest_id }
            }
            RequirementCheckpointV2::QuestAtStage { quest_id, stage_id } => {
                Self::QuestAtStage { quest_id, stage_id }
            }
            RequirementCheckpointV2::NpcAccompanying { npc_actor_id } => {
                Self::NpcAccompanying { npc_actor_id }
            }
        }
    }
}

pub(super) fn cost_to_checkpoint(value: &TransactionCost) -> CostCheckpointV2 {
    match value {
        TransactionCost::CarriedGold { amount } => {
            CostCheckpointV2::CarriedGold { amount: *amount }
        }
        TransactionCost::SelectedCarriedItem { quantity } => {
            CostCheckpointV2::SelectedCarriedItem {
                quantity: *quantity,
            }
        }
    }
}

impl From<CostCheckpointV2> for TransactionCost {
    fn from(value: CostCheckpointV2) -> Self {
        match value {
            CostCheckpointV2::CarriedGold { amount } => Self::CarriedGold { amount },
            CostCheckpointV2::SelectedCarriedItem { quantity } => {
                Self::SelectedCarriedItem { quantity }
            }
        }
    }
}

pub(super) fn reward_to_checkpoint(value: &TransactionReward) -> RewardCheckpointV2 {
    match value {
        TransactionReward::Experience { amount } => {
            RewardCheckpointV2::Experience { amount: *amount }
        }
        TransactionReward::Item {
            item_instance_id,
            item_definition_id,
            position,
        } => RewardCheckpointV2::Item {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: item_definition_id.clone(),
            position: *position,
        },
        TransactionReward::Class {
            to_class_id,
            to_class_display,
        } => RewardCheckpointV2::Class {
            to_class_id: to_class_id.clone(),
            to_class_display: to_class_display.clone(),
        },
        TransactionReward::Spell { spell_id } => RewardCheckpointV2::Spell {
            spell_id: spell_id.clone(),
        },
        TransactionReward::QuestStage { quest_id, stage_id } => RewardCheckpointV2::QuestStage {
            quest_id: quest_id.clone(),
            stage_id: stage_id.clone(),
        },
    }
}

impl From<RewardCheckpointV2> for TransactionReward {
    fn from(value: RewardCheckpointV2) -> Self {
        match value {
            RewardCheckpointV2::Experience { amount } => Self::Experience { amount },
            RewardCheckpointV2::Item {
                item_instance_id,
                item_definition_id,
                position,
            } => Self::Item {
                item_instance_id,
                item_definition_id,
                position,
            },
            RewardCheckpointV2::Class {
                to_class_id,
                to_class_display,
            } => Self::Class {
                to_class_id,
                to_class_display,
            },
            RewardCheckpointV2::Spell { spell_id } => Self::Spell { spell_id },
            RewardCheckpointV2::QuestStage { quest_id, stage_id } => {
                Self::QuestStage { quest_id, stage_id }
            }
        }
    }
}
