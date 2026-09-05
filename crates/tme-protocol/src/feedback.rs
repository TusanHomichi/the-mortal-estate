use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackCue {
    PhysicalCombat {
        #[serde(deserialize_with = "deserialize_required_nullable")]
        source: Option<FeedbackActor>,
        target: FeedbackActor,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        location: Option<Position>,
        mode: PhysicalAttackMode,
        outcome: FeedbackPhysicalOutcome,
    },
    WeaponFumbled {
        actor: FeedbackActor,
        mode: PhysicalAttackMode,
        result: FeedbackWeaponFumbleResult,
    },
    SpellLifecycle {
        actor: FeedbackActor,
        spell_id: WireLabel,
        spell_name: WireLabel,
        state: FeedbackSpellLifecycleState,
    },
    SpellImpact {
        #[serde(deserialize_with = "deserialize_required_nullable")]
        source: Option<FeedbackActor>,
        spell_id: WireLabel,
        spell_name: WireLabel,
        target: FeedbackActor,
        location: Position,
        outcome: FeedbackSpellImpactOutcome,
    },
    ActorEffect {
        actor: FeedbackActor,
        location: Position,
        effect_id: WireLabel,
        effect_kind: WireLabel,
        change: FeedbackEffectChange,
    },
    TileEffect {
        location: Position,
        effect_id: WireLabel,
        effect_kind: WireLabel,
        change: FeedbackEffectChange,
    },
    EffectDamage {
        actor: FeedbackActor,
        location: Position,
        effect_id: WireLabel,
        effect_kind: WireLabel,
        damage: i32,
        actor_hp: i32,
    },
    Resource {
        actor: FeedbackActor,
        resource: ResourceKind,
        reason: FeedbackResourceReason,
        amount: i32,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        current: Option<i32>,
        maximum: i32,
    },
    Transaction {
        actor: FeedbackActor,
        source: FeedbackTransactionSource,
        costs: Vec<FeedbackTransactionCost>,
        rewards: Vec<FeedbackTransactionReward>,
    },
    Quest {
        quest_id: WireLabel,
        quest_title: WireLabel,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        before_stage_id: Option<WireLabel>,
        after_stage_id: WireLabel,
        after_stage_label: WireLabel,
        terminal: bool,
    },
    NpcMessage {
        npc_actor_id: ActorId,
        npc_name: WireLabel,
        interaction_id: WireLabel,
        response: FeedbackText,
    },
    Defeat {
        actor: FeedbackActor,
        location: Position,
        cause: FeedbackDeathCause,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        credited_source: Option<FeedbackActor>,
    },
    Corpse {
        corpse_id: CorpseId,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        origin: Option<FeedbackActor>,
        location: Position,
        change: FeedbackCorpseChange,
    },
    LifeState {
        actor: FeedbackActor,
        from: LifeState,
        to: LifeState,
    },
    Resurrection {
        actor: FeedbackActor,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        corpse_id: Option<CorpseId>,
        method: FeedbackResurrectionMethod,
        destination: Position,
        current_hp: i32,
        current_stamina: i32,
    },
}

impl FeedbackCue {
    pub(super) fn validate(&self) -> Result<(), ProtocolError> {
        if let Self::Transaction {
            costs,
            rewards,
            source,
            ..
        } = self
        {
            if costs.len() > MAX_FEEDBACK_TRANSACTION_COSTS
                || rewards.len() > MAX_FEEDBACK_TRANSACTION_REWARDS
            {
                return Err(ProtocolError::new(
                    "feedback transaction exceeds receipt bound",
                ));
            }
            if let FeedbackTransactionSource::MerchantPurchase {
                item_instance_ids, ..
            } = source
                && item_instance_ids.len() > MAX_MERCHANT_PURCHASE_ITEMS
            {
                return Err(ProtocolError::new(
                    "feedback merchant purchase exceeds item bound",
                ));
            }
        }
        let valid_gold_pile_id =
            |value: &WireLabel| is_canonical_sequence_id(value.as_str(), "gold:");
        let valid_corpse_id =
            |value: &CorpseId| is_canonical_sequence_id(value.as_str(), "corpse:");
        let sequence_ids_are_valid = match self {
            Self::Transaction {
                source,
                costs,
                rewards,
                ..
            } => {
                let source_is_valid = match source {
                    FeedbackTransactionSource::RestorationService { corpse_id, .. } => {
                        corpse_id.as_ref().is_none_or(valid_corpse_id)
                    }
                    FeedbackTransactionSource::BankDeposit { gold_pile_id, .. } => {
                        valid_gold_pile_id(gold_pile_id)
                    }
                    _ => true,
                };
                let costs_are_valid = costs.iter().all(|cost| match cost {
                    FeedbackTransactionCost::GroundGoldPile { gold_pile_id, .. } => {
                        valid_gold_pile_id(gold_pile_id)
                    }
                    _ => true,
                });
                let rewards_are_valid = rewards.iter().all(|reward| match reward {
                    FeedbackTransactionReward::GroundGoldPile { gold_pile_id, .. } => {
                        valid_gold_pile_id(gold_pile_id)
                    }
                    FeedbackTransactionReward::PriestResurrection { corpse_id, .. } => {
                        valid_corpse_id(corpse_id)
                    }
                    _ => true,
                });
                source_is_valid && costs_are_valid && rewards_are_valid
            }
            Self::Corpse { corpse_id, .. } => valid_corpse_id(corpse_id),
            Self::Resurrection { corpse_id, .. } => corpse_id.as_ref().is_none_or(valid_corpse_id),
            _ => true,
        };
        if !sequence_ids_are_valid {
            return Err(ProtocolError::new(
                "feedback sequence identity is not canonical",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedEvent {
    ActorMoved {
        actor_id: ActorId,
        from: Position,
        to: Position,
        navigation: NavigationKind,
    },
    Inspected {
        location: Position,
        tile: WireLabel,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        tile_move_cost: Option<i32>,
        exits: Vec<ObserverInspectExit>,
        nearby_actors: Vec<ObserverInspectActor>,
        ground_items: Vec<ObserverInspectGroundItem>,
    },
    GroupChanged {
        group_id: DecimalU64,
    },
    GroupInvitationChanged {
        invitation_id: DecimalU64,
    },
    GroupPresenceChanged {
        group_id: DecimalU64,
        character_id: CharacterId,
        connected: bool,
    },
    PlayerFollowChanged {
        follower_character_id: CharacterId,
        target_character_id: Option<CharacterId>,
    },
    CommunicationPreferencesChanged,
    ItemOfferChanged {
        item_instance_id: ItemInstanceId,
    },
    DefeatRewardShare {
        character_id: CharacterId,
        amount: i32,
    },
    Feedback {
        cue: FeedbackCue,
    },
}

impl ObservedEvent {
    pub(super) fn validate(&self) -> Result<(), ProtocolError> {
        match self {
            Self::Feedback { cue } => cue.validate(),
            _ => Ok(()),
        }
    }
}
