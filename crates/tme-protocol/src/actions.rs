use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostilityAuthorization {
    Safe,
    ConfirmedUnsafe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalAttackMode {
    Fight,
    Kick,
    Jumpkick,
    Poke,
    Shoot,
    Throw,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SpellTarget {
    None,
    SelfTarget,
    Actor {
        actor_id: ActorId,
    },
    Path {
        directions: Vec<Direction>,
    },
    Coordinate {
        position: Position,
    },
    Area {
        center: Position,
    },
    Direction {
        direction: Direction,
    },
    Door {
        direction: Direction,
    },
    Item {
        item_instance_id: ItemInstanceId,
        location: SpellItemLocation,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellItemLocation {
    Sack,
    ActiveEquipment,
    GroundHere,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemMoveDestination {
    GroundHere,
    Carried { position: CarriedPosition },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GoldMoveSource {
    Carried { position: CarriedGoldPosition },
    Ground { gold_pile_id: WireLabel },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GoldMoveDestination {
    Carried { position: CarriedGoldPosition },
    GroundHere,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GoldMoveQuantity {
    All,
    Exact { amount: DecimalI64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementPace {
    Walk,
    Run,
    Sprint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BurdenTier {
    LightlyLoaded,
    ModeratelyLoaded,
    HeavilyLoaded,
    VeryHeavilyLoaded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementExertion {
    None,
    Normal,
    Rapid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementStopReason {
    FullPathAccepted,
    Blocked,
    Transitioned,
    ZeroStaminaLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathPreviewBlockedReason {
    SuppressedByStatus,
    OutOfBounds,
    BlockedTerrain,
    InsufficientMovementPoints,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PathPreviewStepOutcome {
    Moved {
        navigation: NavigationKind,
    },
    Transitioned {
        navigation: NavigationKind,
        to: Position,
    },
    Blocked {
        reason: PathPreviewBlockedReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathPreviewStep {
    pub index: DecimalU64,
    pub direction: Direction,
    pub from: Position,
    pub attempted: Position,
    pub opens_door: bool,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub terrain_name: Option<WireLabel>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub cost: Option<i32>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub remaining_points_after: Option<i32>,
    pub outcome: PathPreviewStepOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Burden {
    pub item_burden: DecimalU64,
    pub coin_burden: DecimalU64,
    pub total_burden: DecimalU64,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub lightly_loaded_limit: Option<DecimalU64>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub moderately_loaded_limit: Option<DecimalU64>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub heavily_loaded_limit: Option<DecimalU64>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub tier: Option<BurdenTier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathPreview {
    pub contract_version: u32,
    pub actor_id: ActorId,
    pub start: Position,
    pub pace: MovementPace,
    pub requested_path: Vec<Direction>,
    pub available_path_points: i32,
    pub accepted_steps: DecimalU64,
    pub steps: Vec<PathPreviewStep>,
    pub stop_reason: MovementStopReason,
    pub final_position: Position,
    pub remaining_path_points: i32,
    pub burden: Burden,
    pub movement_exertion: MovementExertion,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub stamina_before: Option<i32>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub stamina_cost: Option<i32>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub stamina_after: Option<i32>,
}

impl PathPreview {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.contract_version != 8 {
            return Err(ProtocolError::new(
                "path preview contract version is not current",
            ));
        }
        if self.requested_path.is_empty() || self.requested_path.len() > MAX_MOVE_PATH_STEPS {
            return Err(ProtocolError::new(
                "path preview must contain 1-3 requested steps",
            ));
        }
        if self.accepted_steps.get() > self.requested_path.len() as u64
            || self.steps.len() > self.requested_path.len()
            || self.accepted_steps.get() > self.steps.len() as u64
        {
            return Err(ProtocolError::new("path preview step counts are invalid"));
        }
        let expected_pace = match self.requested_path.len() {
            1 => MovementPace::Walk,
            2 => MovementPace::Run,
            3 => MovementPace::Sprint,
            _ => unreachable!("requested path length was validated"),
        };
        if self.pace != expected_pace {
            return Err(ProtocolError::new("path preview pace is invalid"));
        }
        if self
            .burden
            .item_burden
            .get()
            .checked_add(self.burden.coin_burden.get())
            != Some(self.burden.total_burden.get())
        {
            return Err(ProtocolError::new("path preview burden total is invalid"));
        }
        let burden_shape = [
            self.burden.lightly_loaded_limit.is_some(),
            self.burden.moderately_loaded_limit.is_some(),
            self.burden.heavily_loaded_limit.is_some(),
            self.burden.tier.is_some(),
        ];
        if !burden_shape.iter().all(|present| *present)
            && burden_shape.iter().any(|present| *present)
        {
            return Err(ProtocolError::new(
                "path preview burden classification is incomplete",
            ));
        }
        let stamina_shape = [
            self.stamina_before.is_some(),
            self.stamina_cost.is_some(),
            self.stamina_after.is_some(),
        ];
        if !stamina_shape.iter().all(|present| *present)
            && stamina_shape.iter().any(|present| *present)
        {
            return Err(ProtocolError::new(
                "path preview stamina classification is incomplete",
            ));
        }

        let mut current = &self.start;
        let mut accepted = 0_u64;
        let mut last_remaining = self.available_path_points;
        for (index, step) in self.steps.iter().enumerate() {
            if step.index.get() != index as u64
                || step.direction != self.requested_path[index]
                || &step.from != current
            {
                return Err(ProtocolError::new("path preview step ordering is invalid"));
            }
            match &step.outcome {
                PathPreviewStepOutcome::Moved { .. } => {
                    if step.terrain_name.is_none()
                        || step.cost.is_none()
                        || step.remaining_points_after.is_none()
                        || step.opens_door
                    {
                        return Err(ProtocolError::new(
                            "path preview moved-step facts are invalid",
                        ));
                    }
                    accepted += 1;
                    current = &step.attempted;
                    last_remaining = step.remaining_points_after.unwrap();
                }
                PathPreviewStepOutcome::Transitioned { navigation, to } => {
                    if step.terrain_name.is_none()
                        || step.cost.is_none()
                        || step.remaining_points_after.is_none()
                        || (step.opens_door && *navigation != NavigationKind::Door)
                    {
                        return Err(ProtocolError::new(
                            "path preview transition-step facts are invalid",
                        ));
                    }
                    accepted += 1;
                    current = to;
                    last_remaining = step.remaining_points_after.unwrap();
                }
                PathPreviewStepOutcome::Blocked { .. } => {
                    if step.terrain_name.is_some()
                        || step.cost.is_some()
                        || step.remaining_points_after.is_some()
                        || step.opens_door
                    {
                        return Err(ProtocolError::new(
                            "path preview blocked-step facts are invalid",
                        ));
                    }
                }
            }
        }
        if self.accepted_steps.get() != accepted
            || &self.final_position != current
            || self.remaining_path_points != last_remaining
        {
            return Err(ProtocolError::new(
                "path preview accepted prefix is inconsistent",
            ));
        }
        let stop_is_valid = match self.stop_reason {
            MovementStopReason::FullPathAccepted => {
                self.steps.len() == self.requested_path.len()
                    && self
                        .steps
                        .iter()
                        .all(|step| !matches!(step.outcome, PathPreviewStepOutcome::Blocked { .. }))
            }
            MovementStopReason::Blocked => {
                self.steps.last().is_some_and(|step| {
                    matches!(step.outcome, PathPreviewStepOutcome::Blocked { .. })
                }) && self
                    .steps
                    .iter()
                    .filter(|step| matches!(step.outcome, PathPreviewStepOutcome::Blocked { .. }))
                    .count()
                    == 1
            }
            MovementStopReason::Transitioned => {
                self.steps.last().is_some_and(|step| {
                    matches!(step.outcome, PathPreviewStepOutcome::Transitioned { .. })
                }) && self
                    .steps
                    .iter()
                    .all(|step| !matches!(step.outcome, PathPreviewStepOutcome::Blocked { .. }))
            }
            MovementStopReason::ZeroStaminaLimit => {
                self.steps.len() < self.requested_path.len()
                    && self
                        .steps
                        .iter()
                        .all(|step| !matches!(step.outcome, PathPreviewStepOutcome::Blocked { .. }))
            }
        };
        if !stop_is_valid {
            return Err(ProtocolError::new("path preview stop reason is invalid"));
        }
        Ok(())
    }
}

pub(super) fn deserialize_required_nullable_spell_target<'de, D>(
    deserializer: D,
) -> Result<Option<SpellTarget>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<SpellTarget>::deserialize(deserializer)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Intent {
    MovePath {
        path: Vec<Direction>,
    },
    Traverse {
        traversal: ExplicitTraversalKind,
    },
    Open {
        direction: Direction,
    },
    Close {
        direction: Direction,
    },
    Inspect,
    Hide,
    ShowSack,
    Wait,
    Rest,
    PhysicalAttack {
        mode: PhysicalAttackMode,
        target_actor_id: ActorId,
        authorization: HostilityAuthorization,
    },
    Nock,
    UnloadBow,
    WarmSpell {
        spell_id: WireLabel,
    },
    CastSpell {
        spell_id: WireLabel,
        #[serde(deserialize_with = "deserialize_required_nullable_spell_target")]
        target: Option<SpellTarget>,
        authorization: HostilityAuthorization,
    },
    CastWarmedSpell {
        #[serde(deserialize_with = "deserialize_required_nullable_spell_target")]
        target: Option<SpellTarget>,
        authorization: HostilityAuthorization,
    },
    FizzleWarmedSpell,
    SearchCorpse {
        corpse_id: CorpseId,
    },
    MoveItem {
        item_instance_id: ItemInstanceId,
        destination: ItemMoveDestination,
    },
    MoveGold {
        source: GoldMoveSource,
        destination: GoldMoveDestination,
        quantity: GoldMoveQuantity,
    },
    DepositBankGold {
        service_id: WireLabel,
        capability_id: WireLabel,
        gold_pile_id: WireLabel,
    },
    WithdrawBankGold {
        service_id: WireLabel,
        capability_id: WireLabel,
        amount: DecimalI64,
    },
    DepositLockerItem {
        service_id: WireLabel,
        capability_id: WireLabel,
        item_instance_id: ItemInstanceId,
    },
    WithdrawLockerItem {
        service_id: WireLabel,
        capability_id: WireLabel,
        item_instance_id: ItemInstanceId,
        destination: CarriedPosition,
    },
    DrinkItem {
        item_instance_id: ItemInstanceId,
    },
    Train {
        service_id: WireLabel,
        offered_gold: DecimalI64,
    },
    Critique {
        service_id: WireLabel,
        track_id: WireLabel,
    },
    PromoteClass {
        target_class_id: WireLabel,
    },
    LearnSpell {
        spell_id: WireLabel,
    },
    CommitServiceTransaction {
        service_id: WireLabel,
        capability_id: WireLabel,
        transaction_id: WireLabel,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        item_instance_id: Option<ItemInstanceId>,
    },
    BuyFromMerchant {
        service_id: WireLabel,
        capability_id: WireLabel,
        item_instance_ids: Vec<ItemInstanceId>,
    },
    SellToMerchant {
        service_id: WireLabel,
        capability_id: WireLabel,
        item_instance_id: ItemInstanceId,
    },
    UseItemService {
        service_id: WireLabel,
        capability_id: WireLabel,
        operation: ItemServiceOperationKind,
        item_instance_id: ItemInstanceId,
    },
    UseRestorationService {
        service_id: WireLabel,
        capability_id: WireLabel,
        operation_id: WireLabel,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        item_instance_id: Option<ItemInstanceId>,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        corpse_id: Option<CorpseId>,
    },
    InteractWithNpc {
        npc_actor_id: ActorId,
        interaction_id: WireLabel,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        item_instance_id: Option<ItemInstanceId>,
    },
    ClearSelfDefense {
        attacker_character_id: CharacterId,
    },
    Invite {
        target_character_id: CharacterId,
    },
    AcceptInvite {
        invitation_id: DecimalU64,
    },
    DeclineInvite {
        invitation_id: DecimalU64,
    },
    CancelInvite {
        invitation_id: DecimalU64,
    },
    LeaveGroup,
    RemoveMember {
        member_character_id: CharacterId,
    },
    DisbandGroup,
    TransferLeadership {
        member_character_id: CharacterId,
    },
    BeginFollow {
        target_character_id: CharacterId,
    },
    EndFollow,
    SetPagesEnabled {
        enabled: bool,
    },
    Block {
        target_character_id: CharacterId,
    },
    Unblock {
        target_character_id: CharacterId,
    },
    OfferItem {
        recipient_character_id: CharacterId,
        item_instance_id: ItemInstanceId,
    },
    AcceptItemOffer {
        item_instance_id: ItemInstanceId,
        destination: CarriedPosition,
    },
    RefuseItemOffer {
        item_instance_id: ItemInstanceId,
    },
    WithdrawItemOffer {
        item_instance_id: ItemInstanceId,
    },
}

impl Intent {
    pub(super) fn validate(&self) -> Result<(), ProtocolError> {
        if let Self::MovePath { path } = self
            && (path.is_empty() || path.len() > MAX_MOVE_PATH_STEPS)
        {
            return Err(ProtocolError::new("move path must contain 1-3 steps"));
        }
        match self {
            Self::CastSpell {
                target: Some(SpellTarget::Path { directions }),
                ..
            }
            | Self::CastWarmedSpell {
                target: Some(SpellTarget::Path { directions }),
                ..
            } if directions.is_empty() || directions.len() > MAX_MOVE_PATH_STEPS => {
                return Err(ProtocolError::new("spell path must contain 1-3 steps"));
            }
            Self::SearchCorpse { corpse_id }
                if !is_canonical_sequence_id(corpse_id.as_str(), "corpse:") =>
            {
                return Err(ProtocolError::new("corpse ID is not canonical"));
            }
            Self::MoveGold {
                source: GoldMoveSource::Ground { gold_pile_id },
                ..
            } if !is_canonical_sequence_id(gold_pile_id.as_str(), "gold:") => {
                return Err(ProtocolError::new("gold pile ID is not canonical"));
            }
            Self::MoveGold {
                quantity: GoldMoveQuantity::Exact { amount },
                ..
            } if amount.get() <= 0 => {
                return Err(ProtocolError::new("gold amount must be positive"));
            }
            Self::WithdrawBankGold { amount, .. } if amount.get() <= 0 => {
                return Err(ProtocolError::new(
                    "bank withdrawal amount must be positive",
                ));
            }
            Self::Train { offered_gold, .. } if offered_gold.get() <= 0 => {
                return Err(ProtocolError::new("training offer must be positive"));
            }
            Self::BuyFromMerchant {
                item_instance_ids, ..
            } => {
                if item_instance_ids.len() > MAX_MERCHANT_PURCHASE_ITEMS {
                    return Err(ProtocolError::new(
                        "merchant purchase contains too many items",
                    ));
                }
                let mut unique = BTreeSet::new();
                if item_instance_ids
                    .iter()
                    .any(|item_instance_id| !unique.insert(item_instance_id.as_str()))
                {
                    return Err(ProtocolError::new(
                        "merchant purchase contains duplicate items",
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }
}
