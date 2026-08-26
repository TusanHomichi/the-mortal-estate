use serde::{Deserialize, Serialize};

use crate::model::{
    ActorId, ActorKind, AttackSafety, CarriedGoldPosition, CarriedPosition, CharacterId, Coord,
    CorpseId, DeathCause, Direction, GoldPileId, GroupId, GroupInviteId, LogicalTime,
    NavigationKind, NpcInteractionOutcome, PhysicalAttackMode, ResourceKind, RestorationStatusKind,
    ResurrectionMethod, WeaponFumbleResult, WorldPosition, WoundState,
};

use super::{
    ActionOptionV1, BurdenViewV1, CarriedLayoutViewV1, CharacterSheetViewV1, ItemOfferViewV1,
    LootClaimViewV1, NpcViewV1, QuestStateViewV1, ServiceViewV1, SpellActionV1, TransitionViewV1,
    WarmedSpellViewV1,
};

pub const OBSERVER_PROJECTION_CONTRACT_VERSION: u32 = 7;
pub const STATIC_SCENE_CONTEXT_CONTRACT_VERSION: u32 = 1;
pub const MAX_STATIC_SCENE_TILES: usize = 225;
pub const MAX_STATIC_SCENE_PROPS: usize = 128;
pub const MAX_STATIC_TRANSITION_APERTURES: usize = 64;
pub const MAX_OBSERVER_ACTORS: usize = 128;
pub const MAX_OBSERVED_EVENTS: usize = 64;
pub const MAX_FEEDBACK_TRANSACTION_COSTS: usize = 64;
pub const MAX_FEEDBACK_TRANSACTION_REWARDS: usize = 64;
pub const MAX_FEEDBACK_TEXT_SCALARS: usize = 280;
pub const MAX_FEEDBACK_TEXT_BYTES: usize = 1024;
pub const MAX_OBSERVER_CORPSES: usize = 64;
pub const MAX_OBSERVER_GROUND_ITEMS: usize = 128;
pub const MAX_OBSERVER_GOLD_PILES: usize = 64;
pub const MAX_OBSERVER_ACTION_OPTIONS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObserverLifeStateV1 {
    Alive,
    Ghost,
    AwaitingResurrection,
    Dead,
}

impl From<&crate::model::ActorLifeState> for ObserverLifeStateV1 {
    fn from(value: &crate::model::ActorLifeState) -> Self {
        match value {
            crate::model::ActorLifeState::Alive => Self::Alive,
            crate::model::ActorLifeState::Ghost { .. } => Self::Ghost,
            crate::model::ActorLifeState::AwaitingResurrection { .. } => Self::AwaitingResurrection,
            crate::model::ActorLifeState::Dead => Self::Dead,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObserverFeedbackActorV1 {
    pub actor_id: ActorId,
    pub name: String,
    pub kind: ActorKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObserverPhysicalOutcomeV1 {
    Hit {
        damage: i32,
        armor_reduction: i32,
        wound_before: WoundState,
        wound_after: WoundState,
        target_hp: i32,
    },
    Missed,
    Blocked,
    NoSight,
    NotReady {
        current_time: LogicalTime,
        ready_at: LogicalTime,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObserverSpellFizzleReasonV1 {
    Replaced,
    Canceled,
    Rest,
    HealingBalm,
    Damage,
    Defeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObserverSpellFailureReasonV1 {
    InvalidPath,
    AboveSkillAttempt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObserverSpellLifecycleStateV1 {
    Warmed {
        warmed_at: LogicalTime,
        ready_at: LogicalTime,
    },
    Ready {
        ready_at: LogicalTime,
    },
    Cast {
        mp_cost: Option<i32>,
        stamina_cost: Option<i32>,
    },
    Fizzled {
        reason: ObserverSpellFizzleReasonV1,
    },
    Failed {
        reason: ObserverSpellFailureReasonV1,
        mp_cost: Option<i32>,
        stamina_cost: Option<i32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObserverSpellImpactOutcomeV1 {
    Damaged { damage: i32, target_hp: i32 },
    Healed { amount: i32, target_hp: i32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObserverEffectChangeV1 {
    Applied { remaining_rounds: Option<u32> },
    Ticked { remaining_rounds: Option<u32> },
    Expired,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObserverResourceReasonV1 {
    MovementSpend,
    PhysicalSpend,
    SpellCost,
    Regenerated,
    Restored,
    Balm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObserverTransactionSourceV1 {
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
        npc_actor_id: ActorId,
        interaction_id: String,
    },
    BankDeposit {
        service_id: String,
        capability_id: String,
        bank_id: String,
        gold_pile_id: GoldPileId,
    },
    BankWithdrawal {
        service_id: String,
        capability_id: String,
        bank_id: String,
        amount: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObserverTransactionCostV1 {
    CarriedGold {
        amount: i64,
        position: CarriedGoldPosition,
        before: i64,
        after: i64,
    },
    GroundGoldPile {
        gold_pile_id: GoldPileId,
        amount: i64,
    },
    BankBalance {
        bank_id: String,
        amount: i64,
        before: i64,
        after: i64,
    },
    SelectedCarriedItem {
        item_instance_id: String,
        item_definition_id: String,
        consumed_quantity: u32,
        remaining_quantity: u32,
    },
    MerchantItem {
        item_instance_id: String,
        item_definition_id: String,
        quantity: u32,
        pawn_listing_price_gold: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObserverTransactionRewardV1 {
    LearningRate {
        track_id: String,
        before: u64,
        after: u64,
    },
    Experience {
        amount: i32,
        total_xp: i64,
    },
    Item {
        item_instance_id: String,
        item_definition_id: String,
        position: CarriedPosition,
        quantity: u32,
    },
    Class {
        from_class_id: String,
        from_class_display: String,
        to_class_id: String,
        to_class_display: String,
    },
    Spell {
        spell_id: String,
        learned_at_level: i32,
    },
    CarriedGold {
        amount: i64,
        position: CarriedGoldPosition,
        before: i64,
        after: i64,
    },
    BankBalance {
        bank_id: String,
        amount: i64,
        before: i64,
        after: i64,
    },
    GroundGoldPile {
        gold_pile_id: GoldPileId,
        amount: i64,
    },
    MerchantItem {
        item_instance_id: String,
        item_definition_id: String,
        quantity: u32,
        listing_price_gold: i64,
    },
    ItemAppraised {
        item_instance_id: String,
        item_definition_id: String,
        unit_value_gold: u64,
        total_value_gold: u64,
    },
    ItemIdentified {
        item_instance_id: String,
        item_definition_id: String,
    },
    ItemEnchanted {
        item_instance_id: String,
        item_definition_id: String,
        enchantment_instance_id: String,
        combat_add_rating_bonus: i32,
        tags: Vec<String>,
        remaining_rounds: Option<u32>,
    },
    ResourceRestored {
        resource: ResourceKind,
        before: i32,
        after: i32,
        maximum: i32,
    },
    StatusCured {
        status: RestorationStatusKind,
        removed_count: u32,
    },
    PriestResurrection {
        corpse_id: CorpseId,
        method: ResurrectionMethod,
        current_hp: i32,
        current_stamina: i32,
    },
    NpcInteraction {
        npc_actor_id: ActorId,
        interaction_id: String,
        outcome: NpcInteractionOutcome,
    },
    QuestStage {
        quest_id: String,
        before_stage_id: Option<String>,
        after_stage_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObserverFeedbackCueV1 {
    PhysicalCombat {
        source: Option<ObserverFeedbackActorV1>,
        target: ObserverFeedbackActorV1,
        location: Option<WorldPosition>,
        mode: PhysicalAttackMode,
        outcome: ObserverPhysicalOutcomeV1,
    },
    WeaponFumbled {
        actor: ObserverFeedbackActorV1,
        mode: PhysicalAttackMode,
        result: WeaponFumbleResult,
    },
    SpellLifecycle {
        actor: ObserverFeedbackActorV1,
        spell_id: String,
        spell_name: String,
        state: ObserverSpellLifecycleStateV1,
    },
    SpellImpact {
        source: Option<ObserverFeedbackActorV1>,
        spell_id: String,
        spell_name: String,
        target: ObserverFeedbackActorV1,
        location: WorldPosition,
        outcome: ObserverSpellImpactOutcomeV1,
    },
    ActorEffect {
        actor: ObserverFeedbackActorV1,
        location: WorldPosition,
        effect_id: String,
        effect_kind: String,
        change: ObserverEffectChangeV1,
    },
    TileEffect {
        location: WorldPosition,
        effect_id: String,
        effect_kind: String,
        change: ObserverEffectChangeV1,
    },
    EffectDamage {
        actor: ObserverFeedbackActorV1,
        location: WorldPosition,
        effect_id: String,
        effect_kind: String,
        damage: i32,
        actor_hp: i32,
    },
    Resource {
        actor: ObserverFeedbackActorV1,
        resource: ResourceKind,
        reason: ObserverResourceReasonV1,
        amount: i32,
        current: Option<i32>,
        maximum: i32,
    },
    Transaction {
        actor: ObserverFeedbackActorV1,
        source: ObserverTransactionSourceV1,
        costs: Vec<ObserverTransactionCostV1>,
        rewards: Vec<ObserverTransactionRewardV1>,
    },
    Quest {
        quest_id: String,
        quest_title: String,
        before_stage_id: Option<String>,
        after_stage_id: String,
        after_stage_label: String,
        terminal: bool,
    },
    NpcMessage {
        npc_actor_id: ActorId,
        npc_name: String,
        interaction_id: String,
        response: String,
    },
    Defeat {
        actor: ObserverFeedbackActorV1,
        location: WorldPosition,
        cause: DeathCause,
        credited_source: Option<ObserverFeedbackActorV1>,
    },
    Corpse {
        corpse_id: CorpseId,
        origin: Option<ObserverFeedbackActorV1>,
        location: WorldPosition,
        change: ObserverCorpseChangeV1,
    },
    LifeState {
        actor: ObserverFeedbackActorV1,
        from: ObserverLifeStateV1,
        to: ObserverLifeStateV1,
    },
    Resurrection {
        actor: ObserverFeedbackActorV1,
        corpse_id: Option<CorpseId>,
        method: ResurrectionMethod,
        destination: WorldPosition,
        current_hp: i32,
        current_stamina: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObserverCorpseChangeV1 {
    Created,
    Removed { method: ResurrectionMethod },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObserverTileV1 {
    pub position: Coord,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terrain_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terrain_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub move_cost: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<TransitionViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObserverActorV1 {
    pub actor_id: ActorId,
    pub character_id: Option<CharacterId>,
    pub name: String,
    pub kind: ActorKind,
    pub position: WorldPosition,
    pub life_state: ObserverLifeStateV1,
    pub hp: i32,
    pub max_hp: i32,
    pub attack_safety: AttackSafety,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObserverItemBindingV1 {
    Unbound,
    Bound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObserverItemV1 {
    pub item_instance_id: String,
    pub item_definition_id: String,
    pub name: String,
    pub quantity: u32,
    pub binding: ObserverItemBindingV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObserverCorpseV1 {
    pub corpse_id: CorpseId,
    pub origin_actor_id: ActorId,
    pub origin_kind: ActorKind,
    pub origin_name: String,
    pub location: WorldPosition,
    pub sequence: u64,
    pub searched: bool,
    pub loot_claim: Option<LootClaimViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObserverGroundItemV1 {
    #[serde(flatten)]
    pub item: ObserverItemV1,
    pub location: WorldPosition,
    pub loot_claim: Option<LootClaimViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObserverGoldPileV1 {
    pub gold_pile_id: GoldPileId,
    pub amount: i64,
    pub location: WorldPosition,
    pub loot_claim: Option<LootClaimViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObserverFrameV1 {
    pub contract_version: u32,
    pub logical_time: LogicalTime,
    pub ready_at: LogicalTime,
    pub observer_actor_id: ActorId,
    pub observation_center: WorldPosition,
    pub observation_radius: u32,
    pub can_act: bool,
    pub tiles: Vec<ObserverTileV1>,
    pub actors: Vec<ObserverActorV1>,
    pub corpses: Vec<ObserverCorpseV1>,
    pub corpses_truncated: bool,
    pub ground_items: Vec<ObserverGroundItemV1>,
    pub ground_items_truncated: bool,
    pub gold_piles: Vec<ObserverGoldPileV1>,
    pub gold_piles_truncated: bool,
    pub character: CharacterSheetViewV1,
    pub carried: CarriedLayoutViewV1,
    pub burden: BurdenViewV1,
    pub warmed_spell: Option<WarmedSpellViewV1>,
    pub spell_actions: Vec<SpellActionV1>,
    pub services_here: Vec<ServiceViewV1>,
    pub npcs_here: Vec<NpcViewV1>,
    pub quest_log: Vec<QuestStateViewV1>,
    pub action_options: Vec<ActionOptionV1>,
    pub action_options_truncated: bool,
    pub social: ObserverSocialV2,
    pub incoming_item_offers: Vec<ItemOfferViewV1>,
    pub outgoing_item_offers: Vec<ItemOfferViewV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaticSceneRoleV1 {
    Overworld,
    CombatSpace,
    Interior,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaticPresentationModeV1 {
    OverworldTown,
    CombatSpace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StaticSceneBoundsV1 {
    pub min: Coord,
    pub max: Coord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StaticSceneSiteV1 {
    pub realm: String,
    pub level: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StaticSceneTileV1 {
    pub position: Coord,
    pub terrain_ids: Vec<String>,
    pub walkable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StaticScenePropV1 {
    pub id: String,
    pub visual_family: String,
    pub anchor: Coord,
    pub layer: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StaticTransitionApertureV1 {
    pub at: Coord,
    pub navigation: NavigationKind,
    pub target: WorldPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StaticSceneContextV1 {
    pub contract_version: u32,
    pub site: StaticSceneSiteV1,
    pub bounds: StaticSceneBoundsV1,
    pub content_digest: String,
    pub visual_manifest_digest: String,
    pub scene_role: StaticSceneRoleV1,
    pub presentation_mode: StaticPresentationModeV1,
    pub world_zoom: [u32; 2],
    pub tiles: Vec<StaticSceneTileV1>,
    pub walkable_mask: Vec<Coord>,
    pub static_props: Vec<StaticScenePropV1>,
    pub transition_apertures: Vec<StaticTransitionApertureV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObserverInspectExitStatusV1 {
    Walkable,
    BlockedTerrain,
    Door { open: bool, target: WorldPosition },
    OutOfBounds,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObserverInspectExitV1 {
    pub direction: Direction,
    pub location: WorldPosition,
    pub terrain: Option<String>,
    pub move_cost: Option<i32>,
    pub status: ObserverInspectExitStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObserverInspectActorV1 {
    pub direction: Direction,
    pub actor_id: ActorId,
    pub actor: String,
    pub kind: ActorKind,
    pub location: WorldPosition,
    pub hp: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObserverInspectGroundItemV1 {
    #[serde(flatten)]
    pub item: ObserverItemV1,
    pub location: WorldPosition,
    pub direction: Option<Direction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObserverGroupMemberV2 {
    pub character_id: CharacterId,
    pub joined_order: u64,
    pub membership_epoch: u64,
    pub connected: bool,
    pub absent_since: Option<LogicalTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObserverGroupV2 {
    pub group_id: GroupId,
    pub leader_character_id: CharacterId,
    pub members: Vec<ObserverGroupMemberV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObserverGroupInvitationV2 {
    pub invitation_id: GroupInviteId,
    pub issuer_character_id: CharacterId,
    pub target_character_id: CharacterId,
    pub group_id: Option<GroupId>,
    pub expires_at: LogicalTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObserverSocialV2 {
    pub character_id: CharacterId,
    pub group: Option<ObserverGroupV2>,
    pub incoming_invitations: Vec<ObserverGroupInvitationV2>,
    pub outgoing_invitations: Vec<ObserverGroupInvitationV2>,
    pub following_character_id: Option<CharacterId>,
    pub pages_enabled: bool,
    pub blocked_character_ids: Vec<CharacterId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedEventV1 {
    ActorMoved {
        actor_id: ActorId,
        from: WorldPosition,
        to: WorldPosition,
        navigation: NavigationKind,
    },
    Inspected {
        location: WorldPosition,
        tile: String,
        tile_move_cost: Option<i32>,
        exits: Vec<ObserverInspectExitV1>,
        nearby_actors: Vec<ObserverInspectActorV1>,
        ground_items: Vec<ObserverInspectGroundItemV1>,
    },
    GroupChanged {
        group_id: GroupId,
    },
    GroupInvitationChanged {
        invitation_id: GroupInviteId,
    },
    GroupPresenceChanged {
        group_id: GroupId,
        character_id: CharacterId,
        connected: bool,
    },
    PlayerFollowChanged {
        follower_character_id: CharacterId,
        target_character_id: Option<CharacterId>,
    },
    CommunicationPreferencesChanged,
    ItemOfferChanged {
        item_instance_id: String,
    },
    DefeatRewardShare {
        character_id: CharacterId,
        amount: i32,
    },
    Feedback {
        cue: ObserverFeedbackCueV1,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObserverProjectionV1 {
    pub contract_version: u32,
    pub static_scene_context: StaticSceneContextV1,
    pub frame: ObserverFrameV1,
    pub events: Vec<ObservedEventV1>,
    pub events_truncated: bool,
}
