use serde::{Deserialize, Serialize};

use crate::model::{
    ActorId, ActorKind, CarriedPosition, CharacterId, CreatureTrait, Direction,
    ExplicitTraversalKind, LogicalTime, WarmedSpellState, WarmedSpellStatus, WorldPosition,
};

use super::{
    ActiveEffectViewV1, ActorLifeStateViewV1, BurdenViewV1, CarriedLayoutViewV1, CorpseActionV1,
    DoorStateViewV1, GroundGoldPileViewV1, ItemInstanceViewV1, MagicResistanceViewV1,
    ObservedSocialViewV1, PhysicalAttackOptionV1, ServiceViewV1, SpellActionV1,
    SummonedActorViewV1, TileEffectViewV1, TransitionViewV1,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PlayerActionContextV1 {
    pub contract_version: u32,
    pub actor_id: ActorId,
    pub actor_name: String,
    pub actor_kind: ActorKind,
    pub position: WorldPosition,
    pub law_zone: super::LawZoneViewV1,
    pub logical_time: LogicalTime,
    pub ready_at: LogicalTime,
    pub can_act: bool,
    pub life_state: ActorLifeStateViewV1,
    pub controlled_path_points: i32,
    pub max_path_steps: usize,
    pub last_resource_activity_at: Option<LogicalTime>,
    pub attack_ready_at: LogicalTime,
    #[serde(default)]
    pub active_effects: Vec<ActiveEffectViewV1>,
    pub magic_resistance: MagicResistanceViewV1,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warmed_spell: Option<WarmedSpellViewV1>,
    pub spell_actions: Vec<SpellActionV1>,
    pub services_here: Vec<ServiceViewV1>,
    pub npcs_here: Vec<super::NpcViewV1>,
    pub quest_log: Vec<super::QuestStateViewV1>,
    pub item_offer_actions: Vec<super::ActionOptionV1>,
    pub incoming_item_offers: Vec<ItemOfferViewV1>,
    pub outgoing_item_offers: Vec<ItemOfferViewV1>,
    #[serde(default)]
    pub tile_effects_here: Vec<TileEffectViewV1>,
    pub exits: Vec<ActionExitV1>,
    pub attack_targets: Vec<ActionTargetV1>,
    pub ground_items_here: Vec<ItemInstanceViewV1>,
    pub corpses_here: Vec<CorpseActionV1>,
    pub ground_gold_here: Vec<GroundGoldPileViewV1>,
    pub carried: CarriedLayoutViewV1,
    pub usable_items: Vec<UsableItemActionV1>,
    pub door_actions: Vec<DoorActionV1>,
    pub traversal_actions: Vec<TraversalActionV1>,
    pub burden: BurdenViewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ActionExitV1 {
    pub direction: Direction,
    pub position: WorldPosition,
    pub terrain_name: Option<String>,
    pub move_cost: Option<i32>,
    pub opens_door: bool,
    pub blocked: bool,
    pub blocked_reason: Option<String>,
    pub transition: Option<TransitionViewV1>,
    #[serde(default)]
    pub tile_effects: Vec<TileEffectViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ActionTargetV1 {
    pub actor_id: ActorId,
    pub actor_name: String,
    pub actor_kind: ActorKind,
    #[serde(default)]
    pub creature_traits: Vec<CreatureTrait>,
    pub social: ObservedSocialViewV1,
    pub position: WorldPosition,
    pub hp: i32,
    pub max_hp: i32,
    pub wound_state: crate::model::WoundState,
    pub physical_attacks: Vec<PhysicalAttackOptionV1>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<ActorId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summoned: Option<SummonedActorViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct UsableItemActionV1 {
    #[serde(flatten)]
    pub item: ItemInstanceViewV1,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct DoorActionV1 {
    pub direction: Direction,
    pub location: WorldPosition,
    pub door_state: DoorStateViewV1,
    pub target: WorldPosition,
    pub can_open: bool,
    pub can_close: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TraversalActionV1 {
    pub kind: ExplicitTraversalKind,
    pub target: WorldPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct WarmedSpellViewV1 {
    pub spell_id: String,
    pub warmed_at: LogicalTime,
    pub ready_at: LogicalTime,
    pub status: WarmedSpellStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ItemOfferViewV1 {
    pub item: ItemInstanceViewV1,
    pub sender_character_id: CharacterId,
    pub recipient_character_id: CharacterId,
    pub source_position: CarriedPosition,
    pub actions: Vec<super::ActionOptionV1>,
}

impl From<&WarmedSpellState> for WarmedSpellViewV1 {
    fn from(value: &WarmedSpellState) -> Self {
        Self {
            spell_id: value.spell_id.clone(),
            warmed_at: value.warmed_at,
            ready_at: value.ready_at,
            status: value.status,
        }
    }
}

// ---------------------------------------------------------------------------
// V2 action-context types (observed, typed reasons)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionBlockedReasonV1 {
    SuppressedByStatus,
    OutOfBounds,
    BlockedTerrain,
    ClosedDoor,
    Occupied,
    InsufficientMovementPoints,
    NotEngaged,
    OutOfRange,
    BlockedBySight,
    NotReady,
    RightHandNotWeapon,
    PhysicalModeNotSupported,
    BowNotNocked,
    LeftHandOccupied,
    BowAlreadyNocked,
    BowNotNockedForUnload,
    NoSuchItem,
    InvalidItemQuantity,
    NoSuchGold,
    InvalidGoldAmount,
    OccupiedCarriedPosition,
    InvalidItemPlacement,
    NoSuchTarget,
    ProtectedTargetRequiresConfirmation,
    InvalidHostileTarget,
    NoSuchSpell,
    SpellNotKnown,
    WrongClass,
    NoProfessionAction,
    NoCoverOrDarkness,
    ForbiddenEquipment,
    SkillLevelTooLow,
    InsufficientMagicPoints,
    InsufficientStamina,
    EffectAlreadyActive,
    InvalidTarget,
    TargetNotVisible,
    TargetOutOfRange,
    TargetImmune,
    EffectResisted,
    MissingRequiredItem,
    SpellBookRequired,
    SpellBookNotOwned,
    SpellAlreadyKnown,
    NoService,
    NoSuchTransaction,
    UnexpectedTransactionInput,
    AlreadyComplete,
    ServiceNotHere,
    MissingTrainingFocus,
    OutsideTrainerWindow,
    TrainingCapReached,
    InsufficientGold,
    BankTransactionLimit,
    LockerFull,
    InvalidTrainingOffer,
    SpellRequiresWarming,
    SpellCastsDirectly,
    NoWarmedSpell,
    SpellStillWarming,
    NoTraversalHere,
    WrongTraversalKind,
    ActorNotLiving,
    NoSuchCorpse,
    CorpseNotHere,
    CorpseAlreadySearched,
    ItemNotSaleable,
    NoCarriedCapacity,
    UnsupportedItemService,
    NoRestorationNeeded,
    UnsupportedRestoration,
    NoSuchNpc,
    NpcNotHere,
    NoSuchInteraction,
    QuestStateMismatch,
    NpcAlreadyFollowing,
    NpcNotFollowing,
    NpcNotAccompanying,
    NpcCannotClimb,
}

impl ActionBlockedReasonV1 {
    pub fn code(self) -> &'static str {
        match self {
            Self::SuppressedByStatus => "suppressed_by_status",
            Self::OutOfBounds => "out_of_bounds",
            Self::BlockedTerrain => "blocked_terrain",
            Self::ClosedDoor => "closed_door",
            Self::Occupied => "occupied",
            Self::InsufficientMovementPoints => "insufficient_movement_points",
            Self::NotEngaged => "not_engaged",
            Self::OutOfRange => "out_of_range",
            Self::BlockedBySight => "blocked_by_sight",
            Self::NotReady => "not_ready",
            Self::RightHandNotWeapon => "right_hand_not_weapon",
            Self::PhysicalModeNotSupported => "physical_mode_not_supported",
            Self::BowNotNocked => "bow_not_nocked",
            Self::LeftHandOccupied => "left_hand_occupied",
            Self::BowAlreadyNocked => "bow_already_nocked",
            Self::BowNotNockedForUnload => "bow_not_nocked_for_unload",
            Self::NoSuchItem => "no_such_item",
            Self::InvalidItemQuantity => "invalid_item_quantity",
            Self::NoSuchGold => "no_such_gold",
            Self::InvalidGoldAmount => "invalid_gold_amount",
            Self::OccupiedCarriedPosition => "occupied_carried_position",
            Self::InvalidItemPlacement => "invalid_item_placement",
            Self::NoSuchTarget => "no_such_target",
            Self::ProtectedTargetRequiresConfirmation => "protected_target_requires_confirmation",
            Self::InvalidHostileTarget => "invalid_hostile_target",
            Self::NoSuchSpell => "no_such_spell",
            Self::SpellNotKnown => "spell_not_known",
            Self::WrongClass => "wrong_class",
            Self::NoProfessionAction => "no_profession_action",
            Self::NoCoverOrDarkness => "no_cover_or_darkness",
            Self::ForbiddenEquipment => "forbidden_equipment",
            Self::SkillLevelTooLow => "skill_level_too_low",
            Self::InsufficientMagicPoints => "insufficient_magic_points",
            Self::InsufficientStamina => "insufficient_stamina",
            Self::EffectAlreadyActive => "effect_already_active",
            Self::InvalidTarget => "invalid_target",
            Self::TargetNotVisible => "target_not_visible",
            Self::TargetOutOfRange => "target_out_of_range",
            Self::TargetImmune => "target_immune",
            Self::EffectResisted => "effect_resisted",
            Self::MissingRequiredItem => "missing_required_item",
            Self::SpellBookRequired => "spell_book_required",
            Self::SpellBookNotOwned => "spell_book_not_owned",
            Self::SpellAlreadyKnown => "spell_already_known",
            Self::NoService => "no_service",
            Self::NoSuchTransaction => "no_such_transaction",
            Self::UnexpectedTransactionInput => "unexpected_transaction_input",
            Self::AlreadyComplete => "already_complete",
            Self::ServiceNotHere => "service_not_here",
            Self::MissingTrainingFocus => "missing_training_focus",
            Self::OutsideTrainerWindow => "outside_trainer_window",
            Self::TrainingCapReached => "training_cap_reached",
            Self::InsufficientGold => "insufficient_gold",
            Self::BankTransactionLimit => "bank_transaction_limit",
            Self::LockerFull => "locker_full",
            Self::InvalidTrainingOffer => "invalid_training_offer",
            Self::SpellRequiresWarming => "spell_requires_warming",
            Self::SpellCastsDirectly => "spell_casts_directly",
            Self::NoWarmedSpell => "no_warmed_spell",
            Self::SpellStillWarming => "spell_still_warming",
            Self::NoTraversalHere => "no_traversal_here",
            Self::WrongTraversalKind => "wrong_traversal_kind",
            Self::ActorNotLiving => "actor_not_living",
            Self::NoSuchCorpse => "no_such_corpse",
            Self::CorpseNotHere => "corpse_not_here",
            Self::CorpseAlreadySearched => "corpse_already_searched",
            Self::ItemNotSaleable => "item_not_saleable",
            Self::NoCarriedCapacity => "no_carried_capacity",
            Self::UnsupportedItemService => "unsupported_item_service",
            Self::NoRestorationNeeded => "no_restoration_needed",
            Self::UnsupportedRestoration => "unsupported_restoration",
            Self::NoSuchNpc => "no_such_npc",
            Self::NpcNotHere => "npc_not_here",
            Self::NoSuchInteraction => "no_such_interaction",
            Self::QuestStateMismatch => "quest_state_mismatch",
            Self::NpcAlreadyFollowing => "npc_already_following",
            Self::NpcNotFollowing => "npc_not_following",
            Self::NpcNotAccompanying => "npc_not_accompanying",
            Self::NpcCannotClimb => "npc_cannot_climb",
        }
    }
}

impl std::fmt::Display for ActionBlockedReasonV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SuppressedByStatus => write!(f, "suppressed by status"),
            Self::OutOfBounds => write!(f, "out of bounds"),
            Self::BlockedTerrain => write!(f, "blocked terrain"),
            Self::ClosedDoor => write!(f, "closed door"),
            Self::Occupied => write!(f, "occupied"),
            Self::InsufficientMovementPoints => write!(f, "insufficient movement points"),
            Self::NotEngaged => write!(f, "not engaged"),
            Self::OutOfRange => write!(f, "out of range"),
            Self::BlockedBySight => write!(f, "blocked by sight"),
            Self::NotReady => write!(f, "not ready"),
            Self::RightHandNotWeapon => write!(f, "right hand is not holding a bow"),
            Self::PhysicalModeNotSupported => write!(f, "physical mode is not supported"),
            Self::BowNotNocked => write!(f, "bow is not nocked"),
            Self::LeftHandOccupied => write!(f, "left hand is occupied"),
            Self::BowAlreadyNocked => write!(f, "bow is already nocked"),
            Self::BowNotNockedForUnload => write!(f, "bow is not nocked for unload"),
            Self::NoSuchItem => write!(f, "no such item"),
            Self::InvalidItemQuantity => write!(f, "invalid item quantity"),
            Self::NoSuchGold => write!(f, "no such gold"),
            Self::InvalidGoldAmount => write!(f, "invalid gold amount"),
            Self::OccupiedCarriedPosition => write!(f, "occupied carried position"),
            Self::InvalidItemPlacement => write!(f, "invalid item placement"),
            Self::NoSuchTarget => write!(f, "no such target"),
            Self::ProtectedTargetRequiresConfirmation => {
                write!(f, "protected target requires confirmation")
            }
            Self::InvalidHostileTarget => write!(f, "invalid hostile target"),
            Self::NoSuchSpell => write!(f, "no such spell"),
            Self::SpellNotKnown => write!(f, "spell not known"),
            Self::WrongClass => write!(f, "wrong class"),
            Self::NoProfessionAction => write!(f, "no profession action"),
            Self::NoCoverOrDarkness => write!(f, "no cover or darkness"),
            Self::ForbiddenEquipment => write!(f, "forbidden equipment"),
            Self::SkillLevelTooLow => write!(f, "skill level too low"),
            Self::InsufficientMagicPoints => write!(f, "insufficient magic points"),
            Self::InsufficientStamina => write!(f, "insufficient stamina"),
            Self::EffectAlreadyActive => write!(f, "effect already active"),
            Self::InvalidTarget => write!(f, "invalid target"),
            Self::TargetNotVisible => write!(f, "target not visible"),
            Self::TargetOutOfRange => write!(f, "target out of range"),
            Self::TargetImmune => write!(f, "target immune"),
            Self::EffectResisted => write!(f, "effect resisted"),
            Self::MissingRequiredItem => write!(f, "missing required item"),
            Self::SpellBookRequired => write!(f, "Spell Book required in right hand"),
            Self::SpellBookNotOwned => write!(f, "Spell Book belongs to another character"),
            Self::SpellAlreadyKnown => write!(f, "spell already known"),
            Self::NoService => write!(f, "no service"),
            Self::NoSuchTransaction => write!(f, "no such transaction"),
            Self::UnexpectedTransactionInput => write!(f, "unexpected transaction input"),
            Self::AlreadyComplete => write!(f, "already complete"),
            Self::ServiceNotHere => write!(f, "service is not here"),
            Self::MissingTrainingFocus => write!(f, "missing training focus"),
            Self::OutsideTrainerWindow => write!(f, "outside trainer window"),
            Self::TrainingCapReached => write!(f, "training cap reached"),
            Self::InsufficientGold => write!(f, "insufficient gold"),
            Self::BankTransactionLimit => write!(f, "bank transaction limit"),
            Self::LockerFull => write!(f, "locker full"),
            Self::InvalidTrainingOffer => write!(f, "invalid training offer"),
            Self::SpellRequiresWarming => write!(f, "spell requires warming"),
            Self::SpellCastsDirectly => write!(f, "spell casts directly"),
            Self::NoWarmedSpell => write!(f, "no warmed spell"),
            Self::SpellStillWarming => write!(f, "spell is still warming"),
            Self::NoTraversalHere => write!(f, "no traversal here"),
            Self::WrongTraversalKind => write!(f, "wrong traversal kind"),
            Self::ActorNotLiving => write!(f, "actor is not living"),
            Self::NoSuchCorpse => write!(f, "no such corpse"),
            Self::CorpseNotHere => write!(f, "corpse is not here"),
            Self::CorpseAlreadySearched => write!(f, "corpse already searched"),
            Self::ItemNotSaleable => write!(f, "item is not saleable"),
            Self::NoCarriedCapacity => write!(f, "no carried capacity"),
            Self::UnsupportedItemService => write!(f, "unsupported item service"),
            Self::NoRestorationNeeded => write!(f, "no restoration needed"),
            Self::UnsupportedRestoration => write!(f, "unsupported restoration"),
            Self::NoSuchNpc => write!(f, "no such NPC"),
            Self::NpcNotHere => write!(f, "NPC is not here"),
            Self::NoSuchInteraction => write!(f, "no such NPC interaction"),
            Self::QuestStateMismatch => write!(f, "quest state does not match"),
            Self::NpcAlreadyFollowing => write!(f, "NPC is already following"),
            Self::NpcNotFollowing => write!(f, "NPC is not following"),
            Self::NpcNotAccompanying => write!(f, "NPC is not accompanying the actor"),
            Self::NpcCannotClimb => write!(f, "NPC cannot climb here"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ActionExitV2 {
    pub direction: Direction,
    pub position: WorldPosition,
    pub terrain_name: Option<String>,
    pub move_cost: Option<i32>,
    pub opens_door: bool,
    pub blocked: bool,
    pub blocked_reason: Option<ActionBlockedReasonV1>,
    pub transition: Option<TransitionViewV1>,
    #[serde(default)]
    pub tile_effects: Vec<TileEffectViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ActionTargetV2 {
    pub actor_id: ActorId,
    pub actor_name: String,
    pub actor_kind: ActorKind,
    #[serde(default)]
    pub creature_traits: Vec<CreatureTrait>,
    pub social: ObservedSocialViewV1,
    pub position: WorldPosition,
    pub hp: i32,
    pub max_hp: i32,
    pub wound_state: crate::model::WoundState,
    pub physical_attacks: Vec<PhysicalAttackOptionV1>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<ActorId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summoned: Option<SummonedActorViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PlayerActionContextV2 {
    pub contract_version: u32,
    pub actor_id: ActorId,
    pub actor_name: String,
    pub actor_kind: ActorKind,
    pub position: WorldPosition,
    pub law_zone: super::LawZoneViewV1,
    pub logical_time: LogicalTime,
    pub ready_at: LogicalTime,
    pub can_act: bool,
    pub life_state: ActorLifeStateViewV1,
    pub controlled_path_points: i32,
    pub max_path_steps: usize,
    pub last_resource_activity_at: Option<LogicalTime>,
    pub attack_ready_at: LogicalTime,
    #[serde(default)]
    pub active_effects: Vec<ActiveEffectViewV1>,
    pub magic_resistance: MagicResistanceViewV1,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warmed_spell: Option<WarmedSpellViewV1>,
    pub spell_actions: Vec<SpellActionV1>,
    pub services_here: Vec<ServiceViewV1>,
    pub npcs_here: Vec<super::NpcViewV1>,
    pub quest_log: Vec<super::QuestStateViewV1>,
    pub item_offer_actions: Vec<super::ActionOptionV1>,
    pub incoming_item_offers: Vec<ItemOfferViewV1>,
    pub outgoing_item_offers: Vec<ItemOfferViewV1>,
    #[serde(default)]
    pub tile_effects_here: Vec<TileEffectViewV1>,
    pub exits: Vec<ActionExitV2>,
    pub attack_targets: Vec<ActionTargetV2>,
    pub ground_items_here: Vec<ItemInstanceViewV1>,
    pub corpses_here: Vec<CorpseActionV1>,
    pub ground_gold_here: Vec<GroundGoldPileViewV1>,
    pub carried: CarriedLayoutViewV1,
    pub usable_items: Vec<UsableItemActionV1>,
    pub door_actions: Vec<DoorActionV1>,
    pub traversal_actions: Vec<TraversalActionV1>,
    pub burden: BurdenViewV1,
}
