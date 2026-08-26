use serde::{Deserialize, Deserializer, Serialize};

use crate::combat::DamageLabel;
use crate::model::{
    ActorId, ActorKind, ActorLifeState, BlockSourceKind, BowReadiness, BowReadinessChangeReason,
    BurdenTier, CarriedGoldPosition, CarriedPosition, CharacterAlignment, CharacterId,
    CharacterIdentity, CombatRisk, CorpseId, CreatureTrait, DeathCause, Direction, GoldPileId,
    GroupId, GroupInviteId, ItemCapability, LogicalTime, LootClaim, MovementExertion, MovementPace,
    MovementStopReason, NavigationKind, NpcInteractionOutcome, PhysicalAttackMode,
    PhysicalAttackOutcome, PhysicalDamageKind, ResourceActivity, ResourceKind,
    RestorationStatusKind, ResurrectionMethod, SocialProfile, SpellCastClass, SpellCastingMethod,
    SpellTarget, WeaponFumbleReason, WeaponFumbleResult, WorldPosition, WoundState,
};
use crate::view::{ActorLifeStateViewV1, ItemInstanceViewV1, PositionedItemViewV1};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelfDefenseChangeReasonV1 {
    Established,
    Replaced,
    Cleared,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SelfDefenseChangedEventV1Raw")]
pub struct SelfDefenseChangedEventV1 {
    pub victim_actor_id: ActorId,
    pub victim_character_id: CharacterId,
    pub before_attacker_character_id: Option<CharacterId>,
    pub after_attacker_character_id: Option<CharacterId>,
    pub reason: SelfDefenseChangeReasonV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelfDefenseChangedEventV1Raw {
    victim_actor_id: ActorId,
    victim_character_id: CharacterId,
    #[serde(deserialize_with = "deserialize_required_nullable_character_id")]
    before_attacker_character_id: Option<CharacterId>,
    #[serde(deserialize_with = "deserialize_required_nullable_character_id")]
    after_attacker_character_id: Option<CharacterId>,
    reason: SelfDefenseChangeReasonV1,
}

impl TryFrom<SelfDefenseChangedEventV1Raw> for SelfDefenseChangedEventV1 {
    type Error = String;

    fn try_from(raw: SelfDefenseChangedEventV1Raw) -> Result<Self, Self::Error> {
        let before_present = raw.before_attacker_character_id.is_some();
        match raw.reason {
            SelfDefenseChangeReasonV1::Established if before_present => {
                return Err("established self-defense has a before identity".to_string());
            }
            SelfDefenseChangeReasonV1::Replaced if !before_present => {
                return Err("replaced self-defense has no before identity".to_string());
            }
            SelfDefenseChangeReasonV1::Established | SelfDefenseChangeReasonV1::Replaced
                if raw.after_attacker_character_id.is_none() =>
            {
                return Err("established self-defense has no attacker".to_string());
            }
            SelfDefenseChangeReasonV1::Cleared
                if !before_present || raw.after_attacker_character_id.is_some() =>
            {
                return Err("cleared self-defense identity shape is invalid".to_string());
            }
            _ => {}
        }
        Ok(Self {
            victim_actor_id: raw.victim_actor_id,
            victim_character_id: raw.victim_character_id,
            before_attacker_character_id: raw.before_attacker_character_id,
            after_attacker_character_id: raw.after_attacker_character_id,
            reason: raw.reason,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcGrudgeReasonV1 {
    PhysicalAttack,
    HostileSpellContact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentChangeReasonV1 {
    UnjustLawfulHumanKill,
    UnjustLawfulAnimalKill,
    KarmaThreshold,
    TownTerrainCast,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SocialConsequenceSourceV1 {
    LawfulVictimDeath {
        victim_actor_id: ActorId,
    },
    TownTerrainCast {
        spell_id: String,
        site: crate::model::WorldSite,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KarmaChangeReasonV1 {
    UnjustLawfulHumanKill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountMarkAssessmentReasonV1 {
    AddForPlayerKill,
    ExemptSelfDefense,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassDemotionReasonV1 {
    UnjustLawfulHumanKill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemConsumptionReason {
    Drink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellPathFailureReason {
    OutOfBounds,
    NotVisible,
    OutOfRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SpellCastFailure {
    InvalidPath { reason: SpellPathFailureReason },
    AboveSkillAttempt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SpellFizzleCause {
    Replaced {
        replacing_spell_id: String,
        replacing_spell_name: String,
    },
    Canceled,
    Rest,
    HealingBalm,
    Damage {
        applied_damage: i32,
        hp_before: i32,
    },
    Defeat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PromotionSpellGrantViewV1 {
    pub spell_id: String,
    pub spell_name: String,
    pub lane: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionSourceV1 {
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
        #[serde(deserialize_with = "deserialize_required_nullable_corpse_id")]
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

fn deserialize_required_nullable_corpse_id<'de, D>(
    deserializer: D,
) -> Result<Option<CorpseId>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<CorpseId>::deserialize(deserializer)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionCostReceiptV1 {
    CarriedGold {
        amount: i64,
        position: CarriedGoldPosition,
        before: i64,
        after: i64,
    },
    GroundGoldPile {
        gold_pile_id: GoldPileId,
        amount: i64,
        from: GoldLocationViewV1,
    },
    BankBalance {
        bank_id: String,
        character_id: CharacterId,
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
        from: ItemLocationViewV1,
        to: ItemLocationViewV1,
        pawn_listing_price_gold: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionRewardReceiptV1 {
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
        character_id: CharacterId,
        amount: i64,
        before: i64,
        after: i64,
    },
    GroundGoldPile {
        gold_pile_id: GoldPileId,
        amount: i64,
        to: GoldLocationViewV1,
    },
    MerchantItem {
        item_instance_id: String,
        item_definition_id: String,
        quantity: u32,
        from: ItemLocationViewV1,
        to: ItemLocationViewV1,
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
        target_actor_id: ActorId,
        resource: ResourceKind,
        before: i32,
        after: i32,
        maximum: i32,
    },
    StatusCured {
        target_actor_id: ActorId,
        status: RestorationStatusKind,
        removed_count: u32,
    },
    PriestResurrection {
        target_actor_id: ActorId,
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
        character_id: CharacterId,
        quest_id: String,
        #[serde(deserialize_with = "deserialize_required_nullable_string")]
        before_stage_id: Option<String>,
        after_stage_id: String,
    },
}

fn deserialize_required_nullable_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}

fn deserialize_required_nullable_character_id<'de, D>(
    deserializer: D,
) -> Result<Option<CharacterId>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<CharacterId>::deserialize(deserializer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcFollowWaitReasonV1 {
    AtTarget,
    Blocked,
    RouteUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NpcFollowDecisionV1 {
    Move { direction: Direction },
    Wait { reason: NpcFollowWaitReasonV1 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemRelocationReason {
    PlayerMove,
    Scavenging,
    Thrown,
    DeathDrop,
    CorpseRetention,
    CorpseSearch,
    ResurrectionReturn,
    WeaponFumble,
    MerchantPurchase,
    MerchantSale,
    LockerDeposit,
    LockerWithdrawal,
    OfferCreated,
    OfferAccepted,
    OfferReturned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoldRelocationReason {
    PlayerMove,
    Scavenging,
    BankDeposit,
    BankWithdrawal,
    DeathDrop,
    CorpseRetention,
    CorpseSearch,
    ResurrectionReturn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BankBalanceChangeReasonV1 {
    Deposit,
    Withdrawal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemOfferCompletionReasonV1 {
    Accepted,
    Refused,
    Withdrawn,
    Separated,
    SenderDefeated,
    RecipientDefeated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupChangeReasonV1 {
    Created,
    Joined,
    Left,
    Removed,
    LeadershipTransferred,
    LeadershipFallback,
    Disbanded,
    Dissolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupInvitationResolutionV1 {
    Accepted,
    Declined,
    Cancelled,
    Expired,
    Invalidated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerFollowChangeReasonV1 {
    Began,
    Ended,
    ManualAction,
    MembershipLost,
    TargetLost,
    ObservationLost,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmorProtectionSourceEventV1 {
    pub carried_position: CarriedPosition,
    pub item_instance_id: String,
    pub item_definition_id: String,
    pub block_rating: i32,
    pub encumbrance: i32,
    pub cutting_reduction: i32,
    pub piercing_reduction: i32,
    pub crushing_reduction: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomaticMovementPurposeV1 {
    Chase,
    Flee,
    Turned,
    Search,
    Scavenge,
    ReturnHome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BanishResultReasonV1 {
    Banished,
    InvalidTarget,
    IneligibleTrait,
    WillpowerFormulaOpen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RaiseDeadResultReasonV1 {
    Resurrected,
    NoCorpse,
    NonPlayerCorpse,
    RollFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionConcealmentRemovalReasonV1 {
    Revealed,
    Opened,
    Expired,
    Replaced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomaticWaitReasonV1 {
    Watch,
    Hold,
    Blocked,
    ReturnBlocked,
    Home,
    Ambush,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AutomaticActorDecisionV1 {
    Suppressed {
        status: String,
    },
    UseAbility {
        ability_id: String,
        spell_id: String,
        spell_name: String,
        target_id: Option<ActorId>,
        target: Option<String>,
    },
    PhysicalAttack {
        target_id: ActorId,
        target: String,
        mode: PhysicalAttackMode,
    },
    Nock {
        item_instance_id: String,
        item_definition_id: String,
        item: String,
    },
    DrinkBalm {
        item_instance_id: String,
    },
    SearchCorpse {
        corpse_id: CorpseId,
    },
    CollectItem {
        item_instance_id: String,
        destination: CarriedPosition,
    },
    CollectGold {
        gold_pile_id: GoldPileId,
        amount: i64,
    },
    Move {
        direction: Direction,
        purpose: AutomaticMovementPurposeV1,
    },
    Wait {
        reason: AutomaticWaitReasonV1,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemLocationViewV1 {
    Ground {
        location: WorldPosition,
    },
    Carried {
        actor_id: ActorId,
        position: CarriedPosition,
    },
    Corpse {
        corpse_id: CorpseId,
        position: CarriedPosition,
    },
    Merchant {
        service_id: String,
        capability_id: String,
    },
    Locker {
        vault_id: String,
        owner_character_id: CharacterId,
    },
    Offered {
        sender_character_id: CharacterId,
        recipient_character_id: CharacterId,
        source_position: CarriedPosition,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GoldLocationViewV1 {
    Carried {
        actor_id: ActorId,
        position: crate::model::CarriedGoldPosition,
    },
    Corpse {
        corpse_id: CorpseId,
    },
    Ground {
        gold_pile_id: GoldPileId,
        location: WorldPosition,
    },
    Bank {
        bank_id: String,
        character_id: CharacterId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EcologyLifecyclePolicyV1 {
    FullSite,
    SlotReplenishment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    ScenarioLoaded {
        id: String,
        name: String,
        realms: Vec<String>,
        levels: Vec<crate::model::WorldSite>,
    },
    ActorStatus {
        actor_id: ActorId,
        actor: String,
        kind: ActorKind,
        location: WorldPosition,
        hp: i32,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        character_identity: Option<CharacterIdentity>,
    },
    ActorReady {
        actor_id: ActorId,
        actor: String,
        kind: ActorKind,
        logical_time: LogicalTime,
    },
    PlayerIntent {
        actor_id: ActorId,
        actor: String,
        logical_time: LogicalTime,
        intent: String,
    },
    GroupInvitationCreated {
        invitation_id: GroupInviteId,
        issuer_character_id: CharacterId,
        target_character_id: CharacterId,
        group_id: Option<GroupId>,
        expires_at: LogicalTime,
    },
    GroupInvitationResolved {
        invitation_id: GroupInviteId,
        issuer_character_id: CharacterId,
        target_character_id: CharacterId,
        group_id: Option<GroupId>,
        resolution: GroupInvitationResolutionV1,
    },
    GroupChanged {
        group_id: GroupId,
        reason: GroupChangeReasonV1,
        leader_character_id: Option<CharacterId>,
        member_character_ids: Vec<CharacterId>,
        subject_character_id: Option<CharacterId>,
    },
    GroupPresenceChanged {
        group_id: GroupId,
        character_id: CharacterId,
        connected: bool,
        absent_since: Option<LogicalTime>,
    },
    PlayerFollowChanged {
        follower_character_id: CharacterId,
        target_character_id: Option<CharacterId>,
        reason: PlayerFollowChangeReasonV1,
    },
    CommunicationPreferenceChanged {
        character_id: CharacterId,
        pages_enabled: bool,
    },
    CharacterBlockChanged {
        character_id: CharacterId,
        target_character_id: CharacterId,
        blocked: bool,
    },
    ActorReadinessScheduled {
        actor_id: ActorId,
        actor: String,
        cost_units: u32,
        ready_at: LogicalTime,
    },
    LogicalTimeAdvanced {
        from: LogicalTime,
        to: LogicalTime,
    },
    Inspected {
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
        tile: String,
        tile_move_cost: Option<i32>,
        exits: Vec<InspectExit>,
        nearby_actors: Vec<InspectActor>,
        ground_items: Vec<InspectGroundItem>,
    },
    AutomaticActorDecision {
        actor_id: ActorId,
        actor: String,
        decision: AutomaticActorDecisionV1,
    },
    NpcSpoke {
        npc_actor_id: ActorId,
        npc: String,
        recipient_character_id: CharacterId,
        interaction_id: String,
        response: String,
    },
    NpcFollowChanged {
        npc_actor_id: ActorId,
        npc: String,
        #[serde(deserialize_with = "deserialize_required_nullable_character_id")]
        from_character_id: Option<CharacterId>,
        #[serde(deserialize_with = "deserialize_required_nullable_character_id")]
        to_character_id: Option<CharacterId>,
    },
    NpcFollowDecision {
        npc_actor_id: ActorId,
        npc: String,
        character_id: CharacterId,
        decision: NpcFollowDecisionV1,
    },
    SelfDefenseChanged(SelfDefenseChangedEventV1),
    NpcGrudgeEstablished {
        npc_actor_id: ActorId,
        attacker_actor_id: ActorId,
        reason: NpcGrudgeReasonV1,
    },
    AlignmentChanged {
        actor_id: ActorId,
        character_id: CharacterId,
        before: CharacterAlignment,
        after: CharacterAlignment,
        reason: AlignmentChangeReasonV1,
        source: SocialConsequenceSourceV1,
    },
    KarmaChanged {
        actor_id: ActorId,
        character_id: CharacterId,
        before: u32,
        after: u32,
        delta: i32,
        reason: KarmaChangeReasonV1,
        victim_actor_id: ActorId,
    },
    AccountMarkAssessed {
        killer_actor_id: ActorId,
        killer_character_id: CharacterId,
        victim_actor_id: ActorId,
        victim_character_id: CharacterId,
        credited_source_actor_id: ActorId,
        assessed: bool,
        reason: AccountMarkAssessmentReasonV1,
    },
    ClassDemoted {
        actor_id: ActorId,
        character_id: CharacterId,
        from_class_id: String,
        to_class_id: String,
        reason: ClassDemotionReasonV1,
        victim_actor_id: ActorId,
    },
    QuestStateChanged {
        character_id: CharacterId,
        quest_id: String,
        #[serde(deserialize_with = "deserialize_required_nullable_string")]
        before_stage_id: Option<String>,
        after_stage_id: String,
    },
    Moved {
        actor_id: ActorId,
        actor: String,
        from: WorldPosition,
        to: WorldPosition,
        navigation: NavigationKind,
    },
    MovementStarted {
        actor_id: ActorId,
        actor: String,
        pace: MovementPace,
        requested_steps: usize,
        accepted_steps: usize,
        available_path_points: i32,
        burden_tier: Option<BurdenTier>,
        exertion: MovementExertion,
        stamina_cost: Option<i32>,
        stop_reason: MovementStopReason,
    },
    MovementCostPaid {
        actor_id: ActorId,
        actor: String,
        site: crate::model::WorldSite,
        direction: Direction,
        navigation: NavigationKind,
        terrain: String,
        cost: i32,
        remaining_points: i32,
        destination: WorldPosition,
    },
    MovementStaminaSpent {
        actor_id: ActorId,
        actor: String,
        pace: MovementPace,
        exertion: MovementExertion,
        amount: i32,
        stamina: i32,
        max_stamina: i32,
    },
    MovementBlocked {
        actor_id: ActorId,
        actor: String,
        from: WorldPosition,
        attempted: WorldPosition,
        reason: String,
    },
    AttackBlockedNoSight {
        attacker_id: ActorId,
        attacker: String,
        attacker_site: crate::model::WorldSite,
        defender_id: ActorId,
        defender: String,
        mode: PhysicalAttackMode,
    },
    AttackNotReady {
        actor_id: ActorId,
        actor: String,
        target_id: ActorId,
        target: String,
        current_time: LogicalTime,
        ready_at: LogicalTime,
        mode: PhysicalAttackMode,
    },
    Attacked {
        attacker_id: ActorId,
        attacker: String,
        defender_id: ActorId,
        defender: String,
        defender_location: WorldPosition,
        mode: PhysicalAttackMode,
        damage_kind: PhysicalDamageKind,
        effective_combat_add_rating: i32,
        roll: u32,
        damage: i32,
        armor_reduction: i32,
        label: DamageLabel,
        wound_before: WoundState,
        wound_after: WoundState,
        defender_hp: i32,
    },
    AttackBlocked {
        attacker_id: ActorId,
        attacker: String,
        defender_id: ActorId,
        defender: String,
        defender_location: WorldPosition,
        mode: PhysicalAttackMode,
        damage_kind: PhysicalDamageKind,
        effective_combat_add_rating: i32,
        armor_encumbrance: i32,
        source: BlockSourceKind,
        carried_position: Option<CarriedPosition>,
        item_instance_id: Option<String>,
        block_value: i32,
        skill_track_id: Option<String>,
        skill_level: Option<u8>,
        roll: u32,
        chance_percent: u32,
        armor_sources: Vec<ArmorProtectionSourceEventV1>,
    },
    BowReadinessChanged {
        actor_id: ActorId,
        actor: String,
        item_instance_id: String,
        from: BowReadiness,
        to: BowReadiness,
        reason: BowReadinessChangeReason,
    },
    WeaponFumbled {
        attacker_id: ActorId,
        attacker: String,
        item_instance_id: String,
        mode: PhysicalAttackMode,
        reason: WeaponFumbleReason,
        result: WeaponFumbleResult,
    },
    AttackMissed {
        attacker_id: ActorId,
        attacker: String,
        defender_id: ActorId,
        defender: String,
        defender_location: WorldPosition,
        mode: PhysicalAttackMode,
        damage_kind: PhysicalDamageKind,
        effective_combat_add_rating: i32,
        attacker_score: i32,
        defender_score: i32,
        roll: i32,
    },
    ProtectionApplied {
        attacker_id: ActorId,
        attacker: String,
        defender_id: ActorId,
        defender: String,
        damage_kind: PhysicalDamageKind,
        amount: i32,
        armor_sources: Vec<ArmorProtectionSourceEventV1>,
    },
    PhysicalDamageAffinityApplied {
        defender_id: ActorId,
        defender: String,
        damage_kind: PhysicalDamageKind,
        input_damage: i32,
        numerator: u32,
        denominator: u32,
        adjusted_damage: i32,
    },
    EcologyResetScheduled {
        site_id: String,
        generation: u32,
        member_ids: Vec<String>,
        due_at: LogicalTime,
        policy: EcologyLifecyclePolicyV1,
    },
    EcologyReset {
        site_id: String,
        from_generation: u32,
        to_generation: u32,
        member_ids: Vec<String>,
        policy: EcologyLifecyclePolicyV1,
    },
    EcologyActorSpawned {
        site_id: String,
        member_id: String,
        generation: u32,
        actor_id: ActorId,
        actor_definition_id: String,
        location: WorldPosition,
    },
    PhysicalStaminaSpent {
        actor_id: ActorId,
        actor: String,
        mode: PhysicalAttackMode,
        amount: i32,
        stamina: i32,
        max_stamina: i32,
    },
    PhysicalPracticeEvaluated {
        actor_id: ActorId,
        actor: String,
        track_id: String,
        mode: PhysicalAttackMode,
        outcome: PhysicalAttackOutcome,
        risk: CombatRisk,
        base_raw_points: u64,
        fatal_blow_bonus_raw_points: u64,
        total_raw_points: u64,
    },
    DefeatContributionRecorded {
        contributor_character_id: Option<CharacterId>,
        target_id: ActorId,
        reward_unit_id: Option<crate::model::DefeatRewardUnitId>,
        reward_class: Option<crate::model::DefeatRewardClass>,
        applied_damage: u64,
        total_actual_damage: u64,
    },
    DefeatRewardEvaluated {
        target_id: ActorId,
        target: String,
        authored_experience: i32,
        actual_damage: u64,
        weighted_damage_numerator: u64,
        weighted_damage_denominator: u64,
        available_experience: i32,
        awarded_experience: i32,
        reason: String,
    },
    DefeatRewardShareAwarded {
        character_id: CharacterId,
        actor_id: ActorId,
        actor: String,
        reward_unit_id: crate::model::DefeatRewardUnitId,
        amount: i32,
    },
    ThaumAboveSkillEvaluated {
        actor_id: ActorId,
        actor: String,
        spell_id: String,
        spell_name: String,
        track_id: String,
        current_skill_level: u8,
        skill_requirement: u8,
        gap: u8,
        roll_denominator: u32,
        success_threshold: u32,
        roll: u32,
        success: bool,
    },
    MagicPracticeEvaluated {
        actor_id: ActorId,
        actor: String,
        current_class_id: String,
        spell_id: String,
        spell_name: String,
        track_id: String,
        mp_cost: i32,
        cast_class: SpellCastClass,
        primary_attribute: Option<crate::model::MagicPrimaryAttribute>,
        primary_attribute_value: Option<i32>,
        base_raw_points: u64,
        primary_attribute_bonus_raw_points: u64,
        total_raw_points: u64,
        risk_applied: bool,
        reason: String,
    },
    ActorDefeated {
        actor_id: ActorId,
        actor: String,
        kind: ActorKind,
        location: WorldPosition,
        cause: DeathCause,
        credited_actor_id: Option<ActorId>,
        loot_claim: Option<LootClaim>,
    },
    CorpseCreated {
        corpse_id: CorpseId,
        origin_actor_id: ActorId,
        origin_character_id: Option<CharacterId>,
        origin_kind: ActorKind,
        origin_name: String,
        location: WorldPosition,
        created_at: LogicalTime,
        sequence: u64,
        loot_claim: Option<LootClaim>,
    },
    CorpseSearched {
        corpse_id: CorpseId,
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
        items_released: usize,
        gold_released: i64,
    },
    CorpseRemoved {
        corpse_id: CorpseId,
        origin_actor_id: ActorId,
        location: WorldPosition,
        method: ResurrectionMethod,
    },
    ActorLifeStateChanged {
        actor_id: ActorId,
        actor: String,
        from: ActorLifeState,
        to: ActorLifeState,
    },
    ResurrectionRequested {
        actor_id: ActorId,
        actor: String,
        cause: DeathCause,
        method: ResurrectionMethod,
    },
    ActorResurrected {
        actor_id: ActorId,
        actor: String,
        corpse_id: Option<CorpseId>,
        method: ResurrectionMethod,
        destination: WorldPosition,
        current_hp: i32,
        current_stamina: i32,
    },
    GoldRelocated {
        actor_id: ActorId,
        actor: String,
        amount: i64,
        from: GoldLocationViewV1,
        to: GoldLocationViewV1,
        reason: GoldRelocationReason,
        loot_claim: Option<LootClaim>,
    },
    BankBalanceChanged {
        actor_id: ActorId,
        actor: String,
        bank_id: String,
        character_id: CharacterId,
        amount: i64,
        before: i64,
        after: i64,
        reason: BankBalanceChangeReasonV1,
    },
    ItemOfferCreated {
        actor_id: ActorId,
        actor: String,
        item_instance_id: String,
        item_definition_id: String,
        item: String,
        sender_character_id: CharacterId,
        recipient_character_id: CharacterId,
        source_position: CarriedPosition,
    },
    ItemOfferCompleted {
        actor_id: ActorId,
        actor: String,
        item_instance_id: String,
        item_definition_id: String,
        item: String,
        sender_character_id: CharacterId,
        recipient_character_id: CharacterId,
        destination: CarriedPosition,
        reason: ItemOfferCompletionReasonV1,
    },
    ActorHidden {
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
        instance_id: String,
        effect_id: String,
        remaining_rounds: Option<u32>,
    },
    HideBroken {
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
        instance_id: String,
        effect_id: String,
        reason: String,
    },
    ResourceRegenerated {
        actor_id: ActorId,
        actor: String,
        resource: ResourceKind,
        activity: ResourceActivity,
        boundary_at: LogicalTime,
        base_amount: i32,
        multiplier_numerator: u32,
        multiplier_denominator: u32,
        rounding: crate::model::MagicArithmeticRounding,
        modifier_item_instance_id: Option<String>,
        modifier_item_definition_id: Option<String>,
        modifier_item: Option<String>,
        modifier_item_position: Option<CarriedPosition>,
        amount: i32,
        current: i32,
        maximum: i32,
    },
    ResourceRestored {
        actor_id: ActorId,
        actor: String,
        resource: ResourceKind,
        before: i32,
        after: i32,
        maximum: i32,
    },
    DoorOpened {
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
    },
    DoorClosed {
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
    },
    SecretTransitionRevealed {
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
        transition_kind: String,
    },
    SecretTransitionHidden {
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
        transition_kind: String,
    },
    TransitionConcealed {
        actor_id: ActorId,
        actor: String,
        spell_id: String,
        spell_name: String,
        instance_id: String,
        location: WorldPosition,
        remaining_rounds: u32,
    },
    TransitionConcealmentRemoved {
        instance_id: String,
        source_spell_id: String,
        source_actor_id: ActorId,
        location: WorldPosition,
        reason: TransitionConcealmentRemovalReasonV1,
    },
    WorldTransition {
        actor_id: ActorId,
        actor: String,
        from: WorldPosition,
        to: WorldPosition,
        navigation: NavigationKind,
    },
    ItemConsumed {
        actor_id: ActorId,
        actor: String,
        item_instance_id: String,
        item_definition_id: String,
        item: String,
        quantity_consumed: u32,
        remaining_quantity: u32,
        reason: ItemConsumptionReason,
        location: WorldPosition,
    },
    BalmHealed {
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
        amount: i32,
        hp: i32,
    },
    ItemRelocated {
        actor_id: ActorId,
        actor: String,
        item_instance_id: String,
        item_definition_id: String,
        item: String,
        quantity: u32,
        from: ItemLocationViewV1,
        to: ItemLocationViewV1,
        reason: ItemRelocationReason,
        loot_claim: Option<LootClaim>,
    },
    ItemBound {
        actor_id: ActorId,
        actor: String,
        item_instance_id: String,
        item_definition_id: String,
        item: String,
        state: String,
    },
    SackShown {
        actor_id: ActorId,
        actor: String,
        items: Vec<PositionedItemViewV1>,
        gold: i64,
    },
    ItemIdentified {
        actor_id: ActorId,
        actor: String,
        source: crate::model::ItemOperationSource,
        item_instance_id: String,
        item_definition_id: String,
        item_name: String,
        quantity: u32,
        location: String,
        capability: Option<ItemCapability>,
    },
    ItemAppraised {
        actor_id: ActorId,
        actor: String,
        source: crate::model::ItemOperationSource,
        item_instance_id: String,
        item_definition_id: String,
        item_name: String,
        quantity: u32,
        unit_value_gold: u64,
        total_value_gold: u64,
    },
    ItemEnchanted {
        actor_id: ActorId,
        actor: String,
        source: crate::model::ItemOperationSource,
        item_instance_id: String,
        item_definition_id: String,
        quantity: u32,
        enchantment_instance_id: String,
        combat_add_rating_bonus: i32,
        tags: Vec<String>,
        remaining_rounds: Option<u32>,
    },
    ItemEnchantmentExpired {
        item_instance_id: String,
        item_definition_id: String,
        quantity: u32,
        enchantment_instance_id: String,
        source: crate::model::ItemOperationSource,
    },
    ItemTransformed {
        actor_id: ActorId,
        actor: String,
        item_instance_id: String,
        old_item_definition_id: String,
        new_item_definition_id: String,
        quantity: u32,
        location: String,
    },
    Located {
        actor_id: ActorId,
        actor: String,
        subject: String,
        id: String,
        site: Option<crate::model::WorldSite>,
        location: Option<WorldPosition>,
        hint: String,
    },
    PortalCreated {
        actor_id: ActorId,
        actor: String,
        instance_id: String,
        location: WorldPosition,
        target: WorldPosition,
        remaining_rounds: Option<u32>,
        two_way: bool,
    },
    PortalExpired {
        instance_id: String,
        location: WorldPosition,
    },
    ExperienceAwarded {
        actor_id: ActorId,
        actor: String,
        amount: i32,
        total_xp: i64,
    },
    LevelGained {
        actor_id: ActorId,
        actor: String,
        current_class_id: String,
        new_level: i32,
        total_xp: i64,
        hp_growth: i32,
        hp: i32,
        max_hp: i32,
        peak_hp: i32,
        mp_growth: i32,
        mp: i32,
        max_mp: i32,
        stamina_growth: i32,
        stamina: i32,
        max_stamina: i32,
    },
    PhysicalAttributeAddsChanged {
        actor_id: ActorId,
        actor: String,
        strength_adds: i32,
        dexterity_adds: i32,
    },
    SkillPracticeAwarded {
        actor_id: ActorId,
        actor: String,
        track_id: String,
        track_display: Option<String>,
        raw_amount: u64,
        learning_rate: u64,
        credited_amount: u64,
        practice_points: u64,
        level: u8,
        critique_rank: u8,
    },
    SkillPositionChanged {
        actor_id: ActorId,
        actor: String,
        track_id: String,
        track_display: Option<String>,
        new_level: u8,
        new_critique_rank: u8,
        level_title: Option<String>,
    },
    GoldChanged {
        actor_id: ActorId,
        actor: String,
        amount: i64,
        new_total: i64,
    },
    TrainingPurchased {
        actor_id: ActorId,
        actor: String,
        service_id: String,
        track_id: String,
        offered_gold: i64,
        spent_gold: i64,
        unspent_gold: i64,
        previous_learning_rate: u64,
        new_learning_rate: u64,
    },
    SkillCritiqued {
        actor_id: ActorId,
        actor: String,
        service_id: String,
        track_id: String,
        track_display: Option<String>,
        level: u8,
        critique_rank: Option<u8>,
        level_title: Option<String>,
    },
    SpellLearned {
        actor_id: ActorId,
        actor: String,
        spell_id: String,
        spell_name: String,
        lane: String,
        skill_requirement: i32,
        learned_at_level: i32,
        gold_cost: i64,
        trainer_service_id: String,
        trainer: String,
        spell_book_item_instance_id: String,
        spell_book_item_definition_id: String,
        spell_book: String,
        spell_book_character_id: String,
    },
    ClassPromoted {
        actor_id: ActorId,
        actor: String,
        from_class: String,
        to_class: String,
        granted_item_instance_id: String,
        granted_item_definition_id: String,
        granted_item: String,
        granted_item_position: CarriedPosition,
        granted_spells: Vec<PromotionSpellGrantViewV1>,
    },
    TransactionCommitted {
        actor_id: ActorId,
        actor: String,
        source: TransactionSourceV1,
        costs: Vec<TransactionCostReceiptV1>,
        rewards: Vec<TransactionRewardReceiptV1>,
    },
    SpellCastCommitted {
        actor_id: ActorId,
        actor: String,
        spell_id: String,
        spell_name: String,
        target: Option<SpellTarget>,
        casting_method: SpellCastingMethod,
        mp_cost: Option<i32>,
        stamina_cost: Option<i32>,
    },
    SpellCastStubbed {
        actor_id: ActorId,
        actor: String,
        spell_id: String,
        spell_name: String,
        target: Option<SpellTarget>,
        casting_method: SpellCastingMethod,
        lane: String,
        mp_cost: Option<i32>,
        stamina_cost: Option<i32>,
    },
    ActorSummoned {
        caster_id: ActorId,
        caster: String,
        spell_id: String,
        spell_name: String,
        actor_id: ActorId,
        actor: String,
        template_id: String,
        owner_id: ActorId,
        social: SocialProfile,
        location: WorldPosition,
        remaining_rounds: Option<u32>,
    },
    SummonExpired {
        actor_id: ActorId,
        actor: String,
        instance_id: ActorId,
        owner_id: ActorId,
        source_spell_id: String,
        template_id: String,
        location: WorldPosition,
    },
    BanishEvaluated {
        caster_id: ActorId,
        caster: String,
        spell_id: String,
        spell_name: String,
        target_id: ActorId,
        target: String,
        eligible_trait: Option<CreatureTrait>,
        owned_by_caster: bool,
        success: bool,
        reason: BanishResultReasonV1,
    },
    ActorBanished {
        caster_id: ActorId,
        caster: String,
        spell_id: String,
        spell_name: String,
        actor_id: ActorId,
        actor: String,
        instance_id: ActorId,
        owner_id: ActorId,
        template_id: String,
        location: WorldPosition,
    },
    TurnUndeadResolved {
        caster_id: ActorId,
        caster: String,
        spell_id: String,
        spell_name: String,
        considered_actor_ids: Vec<ActorId>,
        moved_actor_ids: Vec<ActorId>,
        blocked_actor_ids: Vec<ActorId>,
    },
    RaiseDeadEvaluated {
        caster_id: ActorId,
        caster: String,
        spell_id: String,
        spell_name: String,
        corpse_id: Option<CorpseId>,
        target_actor_id: Option<ActorId>,
        magic_level: u8,
        roll_denominator: u32,
        success_threshold: u32,
        roll: Option<u32>,
        success: bool,
        reason: RaiseDeadResultReasonV1,
    },
    SpellDamaged {
        caster_id: ActorId,
        caster: String,
        spell_id: String,
        spell_name: String,
        target_id: ActorId,
        target: String,
        location: WorldPosition,
        damage_kind: Option<String>,
        damage: i32,
        hp: i32,
    },
    SpellHealed {
        caster_id: ActorId,
        caster: String,
        spell_id: String,
        spell_name: String,
        target_id: ActorId,
        target: String,
        location: WorldPosition,
        amount: i32,
        hp: i32,
    },
    SpellWarmed {
        actor_id: ActorId,
        actor: String,
        spell_id: String,
        spell_name: String,
        warmed_at: LogicalTime,
        ready_at: LogicalTime,
    },
    WarmedSpellReady {
        actor_id: ActorId,
        actor: String,
        spell_id: String,
        spell_name: String,
        ready_at: LogicalTime,
    },
    WarmedSpellCast {
        actor_id: ActorId,
        actor: String,
        spell_id: String,
        spell_name: String,
        target: Option<SpellTarget>,
    },
    SpellFizzled {
        actor_id: ActorId,
        actor: String,
        spell_id: String,
        spell_name: String,
        cause: SpellFizzleCause,
    },
    SpellCastFailed {
        actor_id: ActorId,
        actor: String,
        spell_id: String,
        spell_name: String,
        target: Option<SpellTarget>,
        failure: SpellCastFailure,
        mp_cost: Option<i32>,
        stamina_cost: Option<i32>,
    },
    EffectApplied {
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
        instance_id: String,
        effect_id: String,
        source_kind: String,
        source_id: String,
        kind: String,
        tags: Vec<String>,
        potency: i32,
        remaining_rounds: Option<u32>,
    },
    TileEffectApplied {
        location: WorldPosition,
        instance_id: String,
        effect_id: String,
        source_kind: String,
        source_id: String,
        kind: String,
        tags: Vec<String>,
        potency: i32,
        remaining_rounds: Option<u32>,
        passability: Option<String>,
        sight: Option<String>,
        hazard: Option<String>,
        move_cost: Option<i32>,
    },
    SpellSaveResolved {
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
        effect_id: String,
        resistance_tag: String,
        natural_save_twentieths: u32,
        matching_bonus_twentieths: u32,
        selected_boost_source_kind: Option<crate::model::ResistanceBoostSourceKind>,
        selected_boost_source_id: Option<String>,
        denominator: u32,
        save_twentieths: u32,
        roll: u32,
        success: bool,
        mitigation_mode: Option<crate::model::SpellResistanceMitigationMode>,
        requested_damage: Option<i32>,
        resolved_damage: Option<i32>,
    },
    EffectTicked {
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
        instance_id: String,
        effect_id: String,
        kind: String,
        tags: Vec<String>,
        potency: i32,
        remaining_rounds: Option<u32>,
    },
    TileEffectTicked {
        location: WorldPosition,
        instance_id: String,
        effect_id: String,
        kind: String,
        tags: Vec<String>,
        potency: i32,
        remaining_rounds: Option<u32>,
    },
    EffectDamaged {
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
        instance_id: String,
        effect_id: String,
        kind: String,
        tags: Vec<String>,
        damage: i32,
        hp: i32,
    },
    TileEffectDamaged {
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
        instance_id: String,
        effect_id: String,
        kind: String,
        tags: Vec<String>,
        damage: i32,
        hp: i32,
    },
    EffectExpired {
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
        instance_id: String,
        effect_id: String,
        kind: String,
    },
    TileEffectExpired {
        location: WorldPosition,
        instance_id: String,
        effect_id: String,
        kind: String,
    },
    EffectRemoved {
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
        instance_id: String,
        effect_id: String,
        kind: String,
        reason: String,
    },
    TileEffectRemoved {
        location: WorldPosition,
        instance_id: String,
        effect_id: String,
        kind: String,
        reason: String,
    },
    ActionSuppressedByStatus {
        actor_id: ActorId,
        actor: String,
        location: WorldPosition,
        intent: String,
        instance_id: String,
        effect_id: String,
        kind: String,
    },
    FinalState {
        actors: Vec<ActorSummary>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActorSummary {
    pub id: ActorId,
    pub name: String,
    pub location: WorldPosition,
    pub hp: i32,
    pub life_state: ActorLifeStateViewV1,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_identity: Option<CharacterIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InspectExit {
    pub direction: Direction,
    pub location: WorldPosition,
    pub terrain: Option<String>,
    pub move_cost: Option<i32>,
    pub status: InspectExitStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectExitStatus {
    Walkable,
    BlockedTerrain,
    Door {
        state: String,
        target: WorldPosition,
    },
    OutOfBounds,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InspectActor {
    pub direction: Direction,
    pub actor_id: ActorId,
    pub actor: String,
    pub kind: ActorKind,
    pub location: WorldPosition,
    pub hp: i32,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_identity: Option<CharacterIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InspectGroundItem {
    #[serde(flatten)]
    pub item: ItemInstanceViewV1,
    pub location: WorldPosition,
    pub direction: Option<Direction>,
}
