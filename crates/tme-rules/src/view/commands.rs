use serde::{Deserialize, Deserializer, Serialize};

use crate::model::{
    ActorId, CarriedPosition, CharacterId, CorpseId, Direction, ExplicitTraversalKind,
    GoldMoveDestination, GoldMoveQuantity, GoldMoveSource, GoldPileId, HostilityAuthorization,
    ItemMoveDestination, SpellCastClass, SpellCastingMethod, SpellTarget, SpellTargetKind,
};

use super::ActionBlockedReasonV1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PlayerCommandV1 {
    pub contract_version: u32,
    pub actor_id: ActorId,
    pub intent: PlayerIntentPayloadV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum PlayerIntentPayloadV1 {
    MovePath {
        path: Vec<Direction>,
    },
    Traverse {
        kind: ExplicitTraversalKind,
    },
    Hide,
    Nock,
    UnloadBow,
    PhysicalAttack {
        mode: crate::model::PhysicalAttackMode,
        target_actor_id: ActorId,
        authorization: HostilityAuthorization,
    },
    SearchCorpse {
        corpse_id: CorpseId,
    },
    MoveItem {
        item_instance_id: String,
        destination: ItemMoveDestination,
    },
    MoveGold {
        source: GoldMoveSource,
        destination: GoldMoveDestination,
        quantity: GoldMoveQuantity,
    },
    DepositBankGold {
        service_id: String,
        capability_id: String,
        gold_pile_id: GoldPileId,
    },
    WithdrawBankGold {
        service_id: String,
        capability_id: String,
        amount: i64,
    },
    DepositLockerItem {
        service_id: String,
        capability_id: String,
        item_instance_id: String,
    },
    WithdrawLockerItem {
        service_id: String,
        capability_id: String,
        item_instance_id: String,
        destination: CarriedPosition,
    },
    OfferItem {
        recipient_character_id: CharacterId,
        item_instance_id: String,
    },
    AcceptItemOffer {
        item_instance_id: String,
        destination: CarriedPosition,
    },
    RefuseItemOffer {
        item_instance_id: String,
    },
    WithdrawItemOffer {
        item_instance_id: String,
    },
    Drink {
        item_instance_id: String,
    },
    Open {
        direction: Direction,
    },
    Close {
        direction: Direction,
    },
    ShowSack,
    Wait,
    Inspect,
    Train {
        service_id: String,
        offered_gold: i64,
    },
    Critique {
        service_id: String,
        track_id: String,
    },
    PromoteClass {
        target_class_id: String,
    },
    LearnSpell {
        spell_id: String,
    },
    CommitServiceTransaction {
        service_id: String,
        capability_id: String,
        transaction_id: String,
        #[serde(deserialize_with = "deserialize_required_nullable_string")]
        item_instance_id: Option<String>,
    },
    BuyFromMerchant {
        service_id: String,
        capability_id: String,
        item_instance_ids: Vec<String>,
    },
    SellToMerchant {
        service_id: String,
        capability_id: String,
        item_instance_id: String,
    },
    UseItemService {
        service_id: String,
        capability_id: String,
        operation: crate::model::ItemServiceOperationKind,
        item_instance_id: String,
    },
    UseRestorationService {
        service_id: String,
        capability_id: String,
        operation_id: String,
        #[serde(deserialize_with = "deserialize_required_nullable_string")]
        item_instance_id: Option<String>,
        #[serde(deserialize_with = "deserialize_required_nullable_corpse_id")]
        corpse_id: Option<CorpseId>,
    },
    InteractWithNpc {
        npc_actor_id: ActorId,
        interaction_id: String,
        #[serde(deserialize_with = "deserialize_required_nullable_string")]
        item_instance_id: Option<String>,
    },
    CastSpell {
        spell_id: String,
        target: Option<SpellTarget>,
        authorization: HostilityAuthorization,
    },
    WarmSpell {
        spell_id: String,
    },
    CastWarmedSpell {
        target: Option<SpellTarget>,
        authorization: HostilityAuthorization,
    },
    ClearSelfDefense {
        attacker_character_id: CharacterId,
    },
    FizzleWarmedSpell,
    Rest,
}

fn deserialize_required_nullable_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
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
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PhysicalAttackOptionV1 {
    pub mode: crate::model::PhysicalAttackMode,
    pub attack_safety: crate::model::AttackSafety,
    pub enabled: bool,
    pub blocked_reason: Option<ActionBlockedReasonV1>,
    pub maximum_range: Option<i32>,
    pub damage_kind: Option<crate::model::PhysicalDamageKind>,
    pub skill_track_id: Option<String>,
    pub skill_level: Option<u8>,
    pub projected_risk: Option<crate::model::CombatRisk>,
    pub selected_item_instance_id: Option<String>,
    pub selected_item_definition_id: Option<String>,
    pub full_two_handed_effect: bool,
    pub barefoot_full_effect: bool,
    pub command: Option<PlayerCommandV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ActionOptionV1 {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub blocked_reason: Option<ActionBlockedReasonV1>,
    /// The command payload for this option. Present for all action options,
    /// including disabled ones (for UI/debug parity). Use
    /// `Engine::validate_actor_command()` to check whether a command is
    /// currently accepted — submitting a disabled command will be rejected
    /// with a typed `ActionBlockedReasonV1`.
    pub command: Option<PlayerCommandV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct SpellActionStateV1 {
    pub enabled: bool,
    pub blocked_reason: Option<ActionBlockedReasonV1>,
    pub requires_target_selection: bool,
    pub command: Option<PlayerCommandV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellTownLawViewV1 {
    Permitted,
    TerrainAlignmentViolation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellSocialViewV1 {
    pub hostile_act: bool,
    pub town_law: SpellTownLawViewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct SpellActionV1 {
    pub spell_id: String,
    pub spell_name: String,
    pub casting_method: SpellCastingMethod,
    pub cast_class: SpellCastClass,
    pub target_kind: Option<SpellTargetKind>,
    pub mp_cost: Option<i32>,
    pub stamina_cost: Option<i32>,
    pub social: SpellSocialViewV1,
    pub warm: SpellActionStateV1,
    pub cast: SpellActionStateV1,
}

/// Result of validating a player command before commit.
/// Gives clients a typed accept/reject answer without parsing StepError strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PlayerCommandStatusV1 {
    pub contract_version: u32,
    pub command: PlayerCommandV1,
    pub accepted: bool,
    pub blocked_reason: Option<ActionBlockedReasonV1>,
}
