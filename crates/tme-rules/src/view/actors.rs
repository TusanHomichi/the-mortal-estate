use serde::{Deserialize, Deserializer, Serialize};

use crate::model::{
    ActorAiBehavior, ActorId, ActorKind, ActorLifeState, CharacterId, CorpseId, CreatureTrait,
    DeathCause, GoldPileId, LogicalTime, LootClaim, LootClaimBasis, LootOwnerId,
    PhysicalAttackMode, SummonedActorState, WorldPosition,
};

use super::{
    ActiveEffectViewV1, ArmorProtectionViewV1, BurdenViewV1, CarriedLayoutViewV1,
    CharacterSheetViewV1, MagicResistanceViewV1, PhysicalWeaponViewV1, WarmedSpellViewV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialNatureViewV1 {
    Human,
    Animal,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialBehaviorViewV1 {
    Adventurer,
    Civilian,
    TownEnforcer,
    AlignmentCreature,
    Passive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialOwnerRelationViewV1 {
    None,
    Summoner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicHostilityReasonV1 {
    SameActor,
    OwnerProtected,
    Passive,
    SelfDefense,
    Retaliation,
    LawfulResponse,
    ChaoticOpposition,
    EvilOpposition,
    NoHostility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrueSocialViewV1 {
    pub alignment: crate::model::CharacterAlignment,
    pub nature: SocialNatureViewV1,
    pub behavior: SocialBehaviorViewV1,
    pub owner_relation: SocialOwnerRelationViewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedSocialViewV1 {
    pub apparent_behavior: SocialBehaviorViewV1,
    pub hostile_to_observer: bool,
    pub hostility_reason: PublicHostilityReasonV1,
    pub attack_safety: crate::model::AttackSafety,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActorLifeStateViewV1 {
    Alive,
    Ghost {
        corpse_id: CorpseId,
        defeated_at: LogicalTime,
    },
    AwaitingResurrection {
        cause: DeathCause,
        defeated_at: LogicalTime,
    },
    Dead,
}

impl From<&ActorLifeState> for ActorLifeStateViewV1 {
    fn from(value: &ActorLifeState) -> Self {
        match value {
            ActorLifeState::Alive => Self::Alive,
            ActorLifeState::Ghost {
                corpse_id,
                defeated_at,
            } => Self::Ghost {
                corpse_id: corpse_id.clone(),
                defeated_at: *defeated_at,
            },
            ActorLifeState::AwaitingResurrection { cause, defeated_at } => {
                Self::AwaitingResurrection {
                    cause: *cause,
                    defeated_at: *defeated_at,
                }
            }
            ActorLifeState::Dead => Self::Dead,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LootClaimViewV1 {
    pub owner: LootOwnerId,
    pub basis: LootClaimBasis,
}

impl From<&LootClaim> for LootClaimViewV1 {
    fn from(value: &LootClaim) -> Self {
        Self {
            owner: value.owner.clone(),
            basis: value.basis,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CorpseViewV1 {
    pub corpse_id: CorpseId,
    pub origin_actor_id: ActorId,
    pub origin_character_id: Option<CharacterId>,
    pub origin_kind: ActorKind,
    pub origin_name: String,
    pub location: WorldPosition,
    pub created_at: LogicalTime,
    pub sequence: u64,
    pub searched: bool,
    pub loot_claim: Option<LootClaimViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CorpseActionV1 {
    pub corpse_id: CorpseId,
    pub pile_index: usize,
    pub origin_actor_id: ActorId,
    pub origin_kind: ActorKind,
    pub origin_name: String,
    pub searched: bool,
    pub loot_claim: Option<LootClaimViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GroundGoldPileViewV1 {
    pub gold_pile_id: GoldPileId,
    pub amount: i64,
    pub location: WorldPosition,
    pub loot_claim: Option<LootClaimViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SummonedActorViewV1 {
    pub instance_id: ActorId,
    pub source_spell_id: String,
    pub template_id: String,
    pub remaining_rounds: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ActorViewV1 {
    pub id: ActorId,
    #[serde(deserialize_with = "deserialize_required_nullable_character_id")]
    pub character_id: Option<CharacterId>,
    pub kind: ActorKind,
    #[serde(default)]
    pub creature_traits: Vec<CreatureTrait>,
    pub social: TrueSocialViewV1,
    pub name: String,
    pub location: WorldPosition,
    pub hp: i32,
    pub max_hp: i32,
    pub wound_state: crate::model::WoundState,
    pub armor_protection: ArmorProtectionViewV1,
    pub life_state: ActorLifeStateViewV1,
    pub ready_at: LogicalTime,
    pub last_resource_activity_at: Option<LogicalTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tie_break_order: Option<u64>,
    pub attack_ready_at: LogicalTime,
    pub physical_weapon: Option<PhysicalWeaponViewV1>,
    pub carried: CarriedLayoutViewV1,
    pub burden: BurdenViewV1,
    #[serde(deserialize_with = "deserialize_required_nullable_npc_actor_state")]
    pub npc: Option<super::NpcActorStateViewV1>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character: Option<CharacterSheetViewV1>,
    #[serde(default)]
    pub active_effects: Vec<ActiveEffectViewV1>,
    pub magic_resistance: MagicResistanceViewV1,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warmed_spell: Option<WarmedSpellViewV1>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<ActorId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summoned: Option<SummonedActorViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObservedActorViewV1 {
    pub id: ActorId,
    pub kind: ActorKind,
    #[serde(default)]
    pub creature_traits: Vec<CreatureTrait>,
    pub social: ObservedSocialViewV1,
    pub name: String,
    pub location: WorldPosition,
    pub hp: i32,
    pub max_hp: i32,
    pub wound_state: crate::model::WoundState,
    pub armor_protection: ArmorProtectionViewV1,
    pub life_state: ActorLifeStateViewV1,
    pub ready_at: LogicalTime,
    pub last_resource_activity_at: Option<LogicalTime>,
    pub attack_ready_at: LogicalTime,
    pub physical_weapon: Option<PhysicalWeaponViewV1>,
    pub carried: CarriedLayoutViewV1,
    pub burden: BurdenViewV1,
    #[serde(deserialize_with = "deserialize_required_nullable_npc_actor_state")]
    pub npc: Option<super::NpcActorStateViewV1>,
    #[serde(deserialize_with = "deserialize_required_nullable_character_sheet")]
    pub character: Option<CharacterSheetViewV1>,
    #[serde(default)]
    pub active_effects: Vec<ActiveEffectViewV1>,
    pub magic_resistance: MagicResistanceViewV1,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warmed_spell: Option<WarmedSpellViewV1>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<ActorId>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summoned: Option<SummonedActorViewV1>,
}

fn deserialize_required_nullable_character_sheet<'de, D>(
    deserializer: D,
) -> Result<Option<CharacterSheetViewV1>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<CharacterSheetViewV1>::deserialize(deserializer)
}

fn deserialize_required_nullable_character_id<'de, D>(
    deserializer: D,
) -> Result<Option<CharacterId>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<CharacterId>::deserialize(deserializer)
}

fn deserialize_required_nullable_npc_actor_state<'de, D>(
    deserializer: D,
) -> Result<Option<super::NpcActorStateViewV1>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<super::NpcActorStateViewV1>::deserialize(deserializer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum AutomaticActorAwarenessPolicyViewV1 {
    Unrestricted,
    LineOfSightMemory { memory_opportunities: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomaticActorRememberedHostileViewV1 {
    pub actor_id: ActorId,
    pub last_seen: WorldPosition,
    pub remaining_opportunities: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomaticActorViewV1 {
    pub actor_id: ActorId,
    pub behavior: ActorAiBehavior,
    pub cadence_units: u32,
    pub aggro_radius: u32,
    pub leash_range: u32,
    pub physical_attack_modes: Vec<PhysicalAttackMode>,
    pub awareness: AutomaticActorAwarenessPolicyViewV1,
    pub remembered: Option<AutomaticActorRememberedHostileViewV1>,
    pub returning_home: bool,
}

impl From<&SummonedActorState> for SummonedActorViewV1 {
    fn from(value: &SummonedActorState) -> Self {
        Self {
            instance_id: value.instance_id.clone(),
            source_spell_id: value.source_spell_id.clone(),
            template_id: value.template_id.clone(),
            remaining_rounds: value.remaining_rounds,
        }
    }
}
