use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionRequirement {
    CurrentClass {
        class_id: WireLabel,
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
        track_id: WireLabel,
        level: u8,
    },
    MinimumCarriedGold {
        amount: DecimalI64,
    },
    CarriedItem {
        item_definition_id: WireLabel,
        quantity: u32,
    },
    CarriedPositionEmpty {
        position: CarriedPosition,
    },
    SpellUnknown {
        spell_id: WireLabel,
    },
    QuestUnstarted {
        quest_id: WireLabel,
    },
    QuestAtStage {
        quest_id: WireLabel,
        stage_id: WireLabel,
    },
    NpcAccompanying {
        npc_actor_id: ActorId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionCost {
    CarriedGold { amount: DecimalI64 },
    SelectedCarriedItem { quantity: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionReward {
    Experience {
        amount: i32,
    },
    Item {
        item_instance_id: ItemInstanceId,
        item_definition_id: WireLabel,
        position: CarriedPosition,
    },
    Class {
        to_class_id: WireLabel,
        to_class_display: WireLabel,
    },
    Spell {
        spell_id: WireLabel,
    },
    QuestStage {
        quest_id: WireLabel,
        stage_id: WireLabel,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceTransaction {
    pub transaction_id: WireLabel,
    pub label: WireLabel,
    pub requirements: Vec<TransactionRequirement>,
    pub costs: Vec<TransactionCost>,
    pub rewards: Vec<TransactionReward>,
    pub actions: Vec<ObserverActionOption>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MerchantListingOrigin {
    AuthoredStock,
    PawnPool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantListing {
    pub item: OwnedItem,
    pub origin: MerchantListingOrigin,
    pub price_gold: DecimalI64,
    pub purchase: ObserverActionOption,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemServiceOperationKind {
    Appraise,
    Identify,
    EnchantWeapon,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemServiceOperation {
    pub operation: ItemServiceOperationKind,
    pub actions: Vec<ObserverActionOption>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Hp,
    Mp,
    Stamina,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestorationStatusKind {
    Blindness,
    Poison,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RestorationOutcome {
    RestoreResource { resource: ResourceKind },
    CureStatus { status: RestorationStatusKind },
    PriestResurrection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestorationOperation {
    pub operation_id: WireLabel,
    pub label: WireLabel,
    pub requirements: Vec<TransactionRequirement>,
    pub costs: Vec<TransactionCost>,
    pub outcome: RestorationOutcome,
    pub actions: Vec<ObserverActionOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ServiceCapability {
    SkillTraining {
        capability_id: WireLabel,
        offered_track_ids: Vec<WireLabel>,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        selected_track_id: Option<WireLabel>,
        actions: Vec<ObserverActionOption>,
    },
    SkillCritique {
        capability_id: WireLabel,
        actions: Vec<ObserverActionOption>,
    },
    SpellTeaching {
        capability_id: WireLabel,
        spell_ids: Vec<WireLabel>,
        actions: Vec<ObserverActionOption>,
    },
    ClassPromotion {
        capability_id: WireLabel,
        target_class_id: WireLabel,
        actions: Vec<ObserverActionOption>,
    },
    ServiceTransaction {
        capability_id: WireLabel,
        transactions: Vec<ServiceTransaction>,
    },
    Merchant {
        capability_id: WireLabel,
        listings: Vec<MerchantListing>,
        buy_all: ObserverActionOption,
        sales: Vec<ObserverActionOption>,
    },
    ItemService {
        capability_id: WireLabel,
        operations: Vec<ItemServiceOperation>,
    },
    Restoration {
        capability_id: WireLabel,
        operations: Vec<RestorationOperation>,
    },
    Bank {
        capability_id: WireLabel,
        bank_id: WireLabel,
        balance_gold: DecimalI64,
        transaction_cap_gold: DecimalI64,
        deposit_actions: Vec<ObserverActionOption>,
        withdrawal_actions: Vec<ObserverActionOption>,
    },
    Locker {
        capability_id: WireLabel,
        vault_id: WireLabel,
        capacity: u32,
        item_count: u32,
        items: Vec<OwnedItem>,
        deposit_actions: Vec<ObserverActionOption>,
        withdrawal_actions: Vec<ObserverActionOption>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub service_id: WireLabel,
    pub name: WireLabel,
    pub position: Position,
    pub capabilities: Vec<ServiceCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NpcInteractionOutcome {
    Speak,
    BeginFollow,
    EndFollow,
    CompleteEscort { npc_actor_id: ActorId },
    Climb { direction: VerticalDirection },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcInteraction {
    pub interaction_id: WireLabel,
    pub label: WireLabel,
    pub requirements: Vec<TransactionRequirement>,
    pub costs: Vec<TransactionCost>,
    pub rewards: Vec<TransactionReward>,
    pub outcome: NpcInteractionOutcome,
    pub actions: Vec<ObserverActionOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Npc {
    pub actor_id: ActorId,
    pub name: WireLabel,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub following_character_id: Option<CharacterId>,
    pub interactions: Vec<NpcInteraction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestState {
    pub quest_id: WireLabel,
    pub quest_title: WireLabel,
    pub stage_id: WireLabel,
    pub stage_label: WireLabel,
    pub terminal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverFrame {
    pub contract_version: u32,
    pub logical_time: DecimalU64,
    pub ready_at: DecimalU64,
    pub observer_actor_id: ActorId,
    pub observation_center: Position,
    pub observation_radius: u32,
    pub can_act: bool,
    pub tiles: Vec<ObserverTile>,
    pub actors: Vec<ObserverActor>,
    pub corpses: Vec<ObserverCorpse>,
    pub corpses_truncated: bool,
    pub ground_items: Vec<ObserverGroundItem>,
    pub ground_items_truncated: bool,
    pub gold_piles: Vec<ObserverGoldPile>,
    pub gold_piles_truncated: bool,
    pub character: ControlledCharacter,
    pub carried: CarriedLayout,
    pub burden: Burden,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub warmed_spell: Option<WarmedSpell>,
    pub spell_actions: Vec<SpellAction>,
    pub services_here: Vec<Service>,
    pub npcs_here: Vec<Npc>,
    pub quest_log: Vec<QuestState>,
    pub action_options: Vec<ObserverActionOption>,
    pub action_options_truncated: bool,
    pub social: SocialView,
    pub incoming_item_offers: Vec<ItemOffer>,
    pub outgoing_item_offers: Vec<ItemOffer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaticSceneRole {
    Overworld,
    CombatSpace,
    Interior,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationMode {
    OverworldTown,
    CombatSpace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticSceneSite {
    pub realm: WireLabel,
    pub level: WireLabel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticSceneBounds {
    pub min: Coord,
    pub max: Coord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticSceneTile {
    pub position: Coord,
    pub terrain_ids: Vec<WireLabel>,
    pub walkable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticSceneProp {
    pub id: WireLabel,
    pub visual_family: WireLabel,
    pub anchor: Coord,
    pub layer: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticTransitionAperture {
    pub at: Coord,
    pub navigation: NavigationKind,
    pub target: Position,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticSceneContext {
    pub contract_version: u32,
    pub site: StaticSceneSite,
    pub bounds: StaticSceneBounds,
    pub content_digest: WireLabel,
    pub visual_manifest_digest: WireLabel,
    pub scene_role: StaticSceneRole,
    pub presentation_mode: PresentationMode,
    pub world_zoom: [u32; 2],
    pub tiles: Vec<StaticSceneTile>,
    pub walkable_mask: Vec<Coord>,
    pub static_props: Vec<StaticSceneProp>,
    pub transition_apertures: Vec<StaticTransitionAperture>,
}

impl StaticSceneContext {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.contract_version != 1 {
            return Err(ProtocolError::new(
                "static scene context contract version is not current",
            ));
        }
        if self.bounds.min.x > self.bounds.max.x || self.bounds.min.y > self.bounds.max.y {
            return Err(ProtocolError::new(
                "static scene context bounds are inverted",
            ));
        }
        let width = i64::from(self.bounds.max.x) - i64::from(self.bounds.min.x) + 1;
        let height = i64::from(self.bounds.max.y) - i64::from(self.bounds.min.y) + 1;
        let area = width
            .checked_mul(height)
            .ok_or_else(|| ProtocolError::new("static scene context area overflows"))?;
        if area <= 0
            || usize::try_from(area).ok() != Some(self.tiles.len())
            || self.tiles.len() > MAX_STATIC_SCENE_TILES
        {
            return Err(ProtocolError::new(
                "static scene context tile rectangle is invalid or exceeds its bound",
            ));
        }
        if self.walkable_mask.len() > self.tiles.len()
            || self.static_props.len() > MAX_STATIC_SCENE_PROPS
            || self.transition_apertures.len() > MAX_STATIC_TRANSITION_APERTURES
        {
            return Err(ProtocolError::new(
                "static scene context vector exceeds its bound",
            ));
        }
        if self.world_zoom[0] == 0 || self.world_zoom[1] == 0 {
            return Err(ProtocolError::new(
                "static scene context world zoom must be positive",
            ));
        }
        if !matches!(
            (self.scene_role, self.presentation_mode),
            (StaticSceneRole::Overworld, PresentationMode::OverworldTown)
                | (StaticSceneRole::CombatSpace, PresentationMode::CombatSpace)
                | (StaticSceneRole::Interior, _)
        ) {
            return Err(ProtocolError::new(
                "static scene role and presentation mode disagree",
            ));
        }
        for digest in [&self.content_digest, &self.visual_manifest_digest] {
            if digest.as_str().len() != 64
                || !digest
                    .as_str()
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(ProtocolError::new(
                    "static scene context digest must be lowercase SHA-256",
                ));
            }
        }
        let expected_positions = self
            .tiles
            .iter()
            .map(|tile| (tile.position.x, tile.position.y))
            .collect::<std::collections::BTreeSet<_>>();
        let position_in_bounds = |position: &Coord| {
            position.x >= self.bounds.min.x
                && position.x <= self.bounds.max.x
                && position.y >= self.bounds.min.y
                && position.y <= self.bounds.max.y
        };
        if expected_positions.len() != self.tiles.len()
            || self.tiles.iter().any(|tile| {
                !position_in_bounds(&tile.position)
                    || tile.terrain_ids.is_empty()
                    || tile.terrain_ids.len() > MAX_STATIC_TERRAINS_PER_TILE
            })
            || self.walkable_mask.iter().any(|position| {
                !expected_positions.contains(&(position.x, position.y))
                    || !self
                        .tiles
                        .iter()
                        .any(|tile| tile.position == *position && tile.walkable)
            })
            || self
                .tiles
                .iter()
                .any(|tile| tile.walkable != self.walkable_mask.contains(&tile.position))
        {
            return Err(ProtocolError::new(
                "static scene context walkable mask differs from tile walkability",
            ));
        }
        let mut prop_ids = std::collections::BTreeSet::new();
        if self
            .static_props
            .iter()
            .any(|prop| !position_in_bounds(&prop.anchor) || !prop_ids.insert(prop.id.as_str()))
            || self
                .transition_apertures
                .iter()
                .any(|aperture| !position_in_bounds(&aperture.at))
        {
            return Err(ProtocolError::new(
                "static scene context prop or aperture is invalid",
            ));
        }
        Ok(())
    }
}

impl ObserverFrame {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.contract_version != 8 {
            return Err(ProtocolError::new(
                "observer frame contract version is not current",
            ));
        }
        if self.observation_radius != 7
            || self.tiles.len() > MAX_OBSERVER_TILES
            || self.actors.len() > MAX_OBSERVER_ACTORS
            || self.corpses.len() > MAX_OBSERVER_CORPSES
            || self.ground_items.len() > MAX_OBSERVER_GROUND_ITEMS
            || self.gold_piles.len() > MAX_OBSERVER_GOLD_PILES
            || self.action_options.len() > MAX_OBSERVER_ACTION_OPTIONS
        {
            return Err(ProtocolError::new(
                "observer frame exceeds R7 storage bounds",
            ));
        }
        if self.gold_piles.iter().any(|pile| pile.amount.get() < 0) {
            return Err(ProtocolError::new(
                "observer gold pile amount must be non-negative",
            ));
        }
        if self.carried.gold.left_hand.get() < 0
            || self.carried.gold.right_hand.get() < 0
            || self.carried.gold.sack.get() < 0
        {
            return Err(ProtocolError::new(
                "observer carried gold must be non-negative",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverActionOption {
    pub id: ActionId,
    pub label: ActionLabel,
    pub enabled: bool,
    pub blocked_reason: Option<WireLabel>,
    pub intent: Option<Intent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObserverInspectExitStatus {
    Walkable,
    BlockedTerrain,
    Door { open: bool, target: Position },
    OutOfBounds,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverInspectExit {
    pub direction: Direction,
    pub location: Position,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub terrain: Option<WireLabel>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub move_cost: Option<i32>,
    pub status: ObserverInspectExitStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverInspectActor {
    pub direction: Direction,
    pub actor_id: ActorId,
    pub actor: WireLabel,
    pub kind: ActorKind,
    pub location: Position,
    pub hp: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverInspectGroundItem {
    #[serde(flatten)]
    pub item: ObserverItem,
    pub location: Position,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub direction: Option<Direction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedbackActor {
    pub actor_id: ActorId,
    pub name: WireLabel,
    pub kind: ActorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackWoundState {
    Unhurt,
    Wounded,
    BadlyWounded,
    NearDeath,
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackWeaponFumbleResult {
    Dropped,
    BowUnnocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackDeathCause {
    Physical,
    Poison,
    Fire,
    OtherMagic,
    Hazard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackResurrectionMethod {
    Gods,
    Priest,
    Thaumaturge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackPhysicalOutcome {
    Hit {
        damage: i32,
        armor_reduction: i32,
        wound_before: FeedbackWoundState,
        wound_after: FeedbackWoundState,
        target_hp: i32,
    },
    Missed {},
    Blocked {},
    NoSight {},
    NotReady {
        current_time: DecimalU64,
        ready_at: DecimalU64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackSpellFizzleReason {
    Replaced,
    Canceled,
    Rest,
    HealingBalm,
    Damage,
    Defeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackSpellFailureReason {
    InvalidPath,
    AboveSkillAttempt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackSpellLifecycleState {
    Warmed {
        warmed_at: DecimalU64,
        ready_at: DecimalU64,
    },
    Ready {
        ready_at: DecimalU64,
    },
    Cast {
        #[serde(deserialize_with = "deserialize_required_nullable")]
        mp_cost: Option<i32>,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        stamina_cost: Option<i32>,
    },
    Fizzled {
        reason: FeedbackSpellFizzleReason,
    },
    Failed {
        reason: FeedbackSpellFailureReason,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        mp_cost: Option<i32>,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        stamina_cost: Option<i32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackSpellImpactOutcome {
    Damaged { damage: i32, target_hp: i32 },
    Healed { amount: i32, target_hp: i32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackEffectChange {
    Applied {
        #[serde(deserialize_with = "deserialize_required_nullable")]
        remaining_rounds: Option<u32>,
    },
    Ticked {
        #[serde(deserialize_with = "deserialize_required_nullable")]
        remaining_rounds: Option<u32>,
    },
    Expired {},
    Removed {},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackResourceReason {
    MovementSpend,
    PhysicalSpend,
    SpellCost,
    Regenerated,
    Restored,
    Balm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackTransactionSource {
    SkillTraining {
        service_id: WireLabel,
        capability_id: WireLabel,
        track_id: WireLabel,
    },
    SpellLearning {
        service_id: WireLabel,
        capability_id: WireLabel,
        spell_id: WireLabel,
    },
    ClassPromotion {
        service_id: WireLabel,
        capability_id: WireLabel,
        transaction_id: WireLabel,
        target_class_id: WireLabel,
    },
    ServiceTransaction {
        service_id: WireLabel,
        capability_id: WireLabel,
        transaction_id: WireLabel,
    },
    MerchantPurchase {
        service_id: WireLabel,
        capability_id: WireLabel,
        item_instance_ids: Vec<ItemInstanceId>,
    },
    MerchantSale {
        service_id: WireLabel,
        capability_id: WireLabel,
        item_instance_id: ItemInstanceId,
    },
    ItemService {
        service_id: WireLabel,
        capability_id: WireLabel,
        operation: ItemServiceOperationKind,
        item_instance_id: ItemInstanceId,
    },
    RestorationService {
        service_id: WireLabel,
        capability_id: WireLabel,
        operation_id: WireLabel,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        corpse_id: Option<CorpseId>,
    },
    NpcInteraction {
        npc_actor_id: ActorId,
        interaction_id: WireLabel,
    },
    BankDeposit {
        service_id: WireLabel,
        capability_id: WireLabel,
        bank_id: WireLabel,
        gold_pile_id: WireLabel,
    },
    BankWithdrawal {
        service_id: WireLabel,
        capability_id: WireLabel,
        bank_id: WireLabel,
        amount: DecimalI64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackTransactionCost {
    CarriedGold {
        amount: DecimalI64,
        position: CarriedGoldPosition,
        before: DecimalI64,
        after: DecimalI64,
    },
    GroundGoldPile {
        gold_pile_id: WireLabel,
        amount: DecimalI64,
    },
    BankBalance {
        bank_id: WireLabel,
        amount: DecimalI64,
        before: DecimalI64,
        after: DecimalI64,
    },
    SelectedCarriedItem {
        item_instance_id: ItemInstanceId,
        item_definition_id: WireLabel,
        consumed_quantity: u32,
        remaining_quantity: u32,
    },
    MerchantItem {
        item_instance_id: ItemInstanceId,
        item_definition_id: WireLabel,
        quantity: u32,
        pawn_listing_price_gold: DecimalI64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackTransactionReward {
    LearningRate {
        track_id: WireLabel,
        before: DecimalU64,
        after: DecimalU64,
    },
    Experience {
        amount: i32,
        total_xp: DecimalI64,
    },
    Item {
        item_instance_id: ItemInstanceId,
        item_definition_id: WireLabel,
        position: CarriedPosition,
        quantity: u32,
    },
    Class {
        from_class_id: WireLabel,
        from_class_display: WireLabel,
        to_class_id: WireLabel,
        to_class_display: WireLabel,
    },
    Spell {
        spell_id: WireLabel,
        learned_at_level: i32,
    },
    CarriedGold {
        amount: DecimalI64,
        position: CarriedGoldPosition,
        before: DecimalI64,
        after: DecimalI64,
    },
    BankBalance {
        bank_id: WireLabel,
        amount: DecimalI64,
        before: DecimalI64,
        after: DecimalI64,
    },
    GroundGoldPile {
        gold_pile_id: WireLabel,
        amount: DecimalI64,
    },
    MerchantItem {
        item_instance_id: ItemInstanceId,
        item_definition_id: WireLabel,
        quantity: u32,
        listing_price_gold: DecimalI64,
    },
    ItemAppraised {
        item_instance_id: ItemInstanceId,
        item_definition_id: WireLabel,
        unit_value_gold: DecimalU64,
        total_value_gold: DecimalU64,
    },
    ItemIdentified {
        item_instance_id: ItemInstanceId,
        item_definition_id: WireLabel,
    },
    ItemEnchanted {
        item_instance_id: ItemInstanceId,
        item_definition_id: WireLabel,
        enchantment_instance_id: WireLabel,
        combat_add_rating_bonus: i32,
        tags: Vec<WireLabel>,
        #[serde(deserialize_with = "deserialize_required_nullable")]
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
        method: FeedbackResurrectionMethod,
        current_hp: i32,
        current_stamina: i32,
    },
    NpcInteraction {
        npc_actor_id: ActorId,
        interaction_id: WireLabel,
        outcome: NpcInteractionOutcome,
    },
    QuestStage {
        quest_id: WireLabel,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        before_stage_id: Option<WireLabel>,
        after_stage_id: WireLabel,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedbackCorpseChange {
    Created {},
    Removed { method: FeedbackResurrectionMethod },
}
