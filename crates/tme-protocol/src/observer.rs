use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    North,
    Northeast,
    East,
    Southeast,
    South,
    Southwest,
    West,
    Northwest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplicitTraversalKind {
    StairsUp,
    StairsDown,
    ClimbUp,
    ClimbDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerticalDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavigationKind {
    Walk,
    Swim,
    Door,
    Stairs { direction: VerticalDirection },
    Pit,
    Climb { direction: VerticalDirection },
    Passage,
    Portal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorKind {
    Player,
    Monster,
    Npc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifeState {
    Alive,
    Ghost,
    AwaitingResurrection,
    Dead,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coord {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub realm: WireLabel,
    pub level: WireLabel,
    pub position: Coord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub navigation: NavigationKind,
    pub target: Position,
    pub door_open: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverTile {
    pub position: Coord,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terrain_id: Option<WireLabel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terrain_name: Option<WireLabel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub move_cost: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<Transition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverActor {
    pub actor_id: ActorId,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub character_id: Option<CharacterId>,
    pub name: WireLabel,
    pub kind: ActorKind,
    pub position: Position,
    pub life_state: LifeState,
    pub hp: i32,
    pub max_hp: i32,
    pub attack_safety: AttackSafety,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttackSafety {
    Invalid,
    Protected,
    OpenSelfDefense,
    OpenEvilPlayer,
    OpenHostile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObserverItemBinding {
    Unbound,
    Bound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverItem {
    pub item_instance_id: ItemInstanceId,
    pub item_definition_id: WireLabel,
    pub name: WireLabel,
    pub quantity: u32,
    pub binding: ObserverItemBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum LootOwner {
    Character(CharacterId),
    TransientActor(ActorId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LootClaimBasis {
    KillingBlow,
    CharacterDeathPile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LootClaim {
    pub owner: LootOwner,
    pub basis: LootClaimBasis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverCorpse {
    pub corpse_id: CorpseId,
    pub origin_actor_id: ActorId,
    pub origin_kind: ActorKind,
    pub origin_name: WireLabel,
    pub location: Position,
    pub sequence: DecimalU64,
    pub searched: bool,
    pub loot_claim: Option<LootClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverGroundItem {
    #[serde(flatten)]
    pub item: ObserverItem,
    pub location: Position,
    pub loot_claim: Option<LootClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverGoldPile {
    pub gold_pile_id: WireLabel,
    pub amount: DecimalI64,
    pub location: Position,
    pub loot_claim: Option<LootClaim>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarriedPosition {
    LeftHand,
    RightHand,
    #[serde(rename = "left_finger_1")]
    LeftFinger1,
    #[serde(rename = "left_finger_2")]
    LeftFinger2,
    #[serde(rename = "left_finger_3")]
    LeftFinger3,
    #[serde(rename = "left_finger_4")]
    LeftFinger4,
    #[serde(rename = "right_finger_1")]
    RightFinger1,
    #[serde(rename = "right_finger_2")]
    RightFinger2,
    #[serde(rename = "right_finger_3")]
    RightFinger3,
    #[serde(rename = "right_finger_4")]
    RightFinger4,
    #[serde(rename = "belt_1")]
    Belt1,
    #[serde(rename = "belt_2")]
    Belt2,
    #[serde(rename = "belt_3")]
    Belt3,
    #[serde(rename = "belt_4")]
    Belt4,
    BeltBack,
    #[serde(rename = "sack_item_1")]
    SackItem1,
    #[serde(rename = "sack_item_2")]
    SackItem2,
    #[serde(rename = "sack_item_3")]
    SackItem3,
    #[serde(rename = "sack_item_4")]
    SackItem4,
    #[serde(rename = "sack_item_5")]
    SackItem5,
    #[serde(rename = "sack_item_6")]
    SackItem6,
    #[serde(rename = "sack_item_7")]
    SackItem7,
    #[serde(rename = "sack_item_8")]
    SackItem8,
    #[serde(rename = "sack_item_9")]
    SackItem9,
    #[serde(rename = "sack_item_10")]
    SackItem10,
    #[serde(rename = "sack_item_11")]
    SackItem11,
    #[serde(rename = "sack_item_12")]
    SackItem12,
    #[serde(rename = "sack_item_13")]
    SackItem13,
    #[serde(rename = "sack_item_14")]
    SackItem14,
    #[serde(rename = "sack_item_15")]
    SackItem15,
    #[serde(rename = "sack_item_16")]
    SackItem16,
    #[serde(rename = "sack_item_17")]
    SackItem17,
    #[serde(rename = "sack_item_18")]
    SackItem18,
    #[serde(rename = "sack_item_19")]
    SackItem19,
    #[serde(rename = "sack_item_20")]
    SackItem20,
    Head,
    Neck,
    LeftArm,
    RightArm,
    Gloves,
    InnerArmor,
    OuterArmor,
    Boots,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarriedGoldPosition {
    LeftHand,
    RightHand,
    Sack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupMember {
    pub character_id: CharacterId,
    pub joined_order: DecimalU64,
    pub membership_epoch: DecimalU64,
    pub connected: bool,
    pub absent_since: Option<DecimalU64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupView {
    pub group_id: DecimalU64,
    pub leader_character_id: CharacterId,
    pub members: Vec<GroupMember>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupInvitation {
    pub invitation_id: DecimalU64,
    pub issuer_character_id: CharacterId,
    pub target_character_id: CharacterId,
    pub group_id: Option<DecimalU64>,
    pub expires_at: DecimalU64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocialView {
    pub character_id: CharacterId,
    pub group: Option<GroupView>,
    pub incoming_invitations: Vec<GroupInvitation>,
    pub outgoing_invitations: Vec<GroupInvitation>,
    pub following_character_id: Option<CharacterId>,
    pub pages_enabled: bool,
    pub blocked_character_ids: Vec<CharacterId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemOffer {
    pub item: OwnedItem,
    pub sender_character_id: CharacterId,
    pub recipient_character_id: CharacterId,
    pub source_position: CarriedPosition,
    pub actions: Vec<ObserverActionOption>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterAlignment {
    Lawful,
    Neutral,
    Chaotic,
    Evil,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterIdentity {
    pub base_class_id: WireLabel,
    pub current_class_id: WireLabel,
    pub display_class: WireLabel,
    pub nationality_id: WireLabel,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub sex_or_gender_display: Option<WireLabel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterAttributes {
    pub strength: i32,
    pub dexterity: i32,
    pub constitution: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub charisma: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterResources {
    pub hp: i32,
    pub max_hp: i32,
    pub peak_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub stamina: i32,
    pub max_stamina: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterProgression {
    pub level: i32,
    pub experience: DecimalI64,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub pending_target_level: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalAttributeAdds {
    pub strength_adds: i32,
    pub dexterity_adds: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionEntry {
    pub from_class_id: WireLabel,
    pub to_class_id: WireLabel,
    pub level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownSpell {
    pub spell_id: WireLabel,
    pub lane: WireLabel,
    pub learned_at_level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillEntry {
    pub track_id: WireLabel,
    pub level: u8,
    pub critique_rank: u8,
    pub practice_points: DecimalU64,
    pub learning_rate: DecimalU64,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub track_display: Option<WireLabel>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub level_title: Option<WireLabel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlledCharacter {
    pub identity: CharacterIdentity,
    pub alignment: CharacterAlignment,
    pub karma_points: u32,
    pub attributes: CharacterAttributes,
    pub resources: CharacterResources,
    pub progression: CharacterProgression,
    pub physical_attribute_adds: PhysicalAttributeAdds,
    pub promotion_history: Vec<PromotionEntry>,
    pub known_spells: Vec<KnownSpell>,
    pub skill_ledger: Vec<SkillEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnedItemBinding {
    Unrestricted,
    BindOnFirstCharacterTouch,
    Bound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BowReadiness {
    Unnocked,
    Nocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemPlacementKind {
    Hand,
    RingFinger,
    BeltSide,
    BeltBack,
    Sack,
    Head,
    Neck,
    Arm,
    Gloves,
    InnerArmor,
    OuterArmor,
    Boots,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedItem {
    pub item_instance_id: ItemInstanceId,
    pub item_definition_id: WireLabel,
    pub name: WireLabel,
    pub quantity: u32,
    pub identified: bool,
    pub appraised: bool,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub known_unit_value_gold: Option<DecimalU64>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub known_stack_value_gold: Option<DecimalU64>,
    pub unit_burden: DecimalU64,
    pub stack_burden: DecimalU64,
    pub binding: OwnedItemBinding,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub bow_readiness: Option<BowReadiness>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionedItem {
    pub position: CarriedPosition,
    pub item: OwnedItem,
    pub valid_placements: Vec<ItemPlacementKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarriedGold {
    pub left_hand: DecimalI64,
    pub right_hand: DecimalI64,
    pub sack: DecimalI64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarriedLayout {
    pub items: Vec<PositionedItem>,
    pub gold: CarriedGold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarmedSpellStatus {
    Warming,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarmedSpell {
    pub spell_id: WireLabel,
    pub warmed_at: DecimalU64,
    pub ready_at: DecimalU64,
    pub status: WarmedSpellStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellCastingMethod {
    Direct,
    WarmThenCast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellCastClass {
    Character,
    Path,
    PathOrCharacter,
    #[serde(rename = "self")]
    SelfTarget,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellTargetKind {
    Actor,
    Area,
    Coordinate,
    Direction,
    Door,
    Item,
    None,
    #[serde(rename = "self")]
    SelfTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellActionState {
    pub enabled: bool,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub blocked_reason: Option<WireLabel>,
    pub requires_target_selection: bool,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub intent: Option<Intent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellAction {
    pub spell_id: WireLabel,
    pub spell_name: WireLabel,
    pub casting_method: SpellCastingMethod,
    pub cast_class: SpellCastClass,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub target_kind: Option<SpellTargetKind>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub mp_cost: Option<i32>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub stamina_cost: Option<i32>,
    pub hostile_act: bool,
    pub town_law_violation: bool,
    pub warm: SpellActionState,
    pub cast: SpellActionState,
}
