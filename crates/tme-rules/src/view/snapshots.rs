use serde::{Deserialize, Serialize};

use crate::model::{ActorId, Coord, LogicalTime, WorldPosition};

use super::{
    ActorViewV1, AutomaticActorViewV1, CharacterQuestStateViewV1, ConcealedTransitionViewV1,
    CorpseViewV1, GroundGoldPileViewV1, GroundItemViewV1, ObservedActorViewV1,
    PlayerActionContextV2, RealmSnapshotV1, RulesViewV1, TileEffectViewV1, TransitionViewV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotScopeV1 {
    OmniscientLocal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellSocialCatalogViewV1 {
    pub spell_id: String,
    pub social: super::SpellSocialViewV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfDefenseRelationViewV1 {
    pub victim_character_id: crate::model::CharacterId,
    pub attacker_character_id: crate::model::CharacterId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcGrudgeRelationViewV1 {
    pub npc_actor_id: ActorId,
    pub attacker_actor_id: ActorId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocialRelationLedgerViewV1 {
    pub self_defense: Vec<SelfDefenseRelationViewV1>,
    pub npc_grudges: Vec<NpcGrudgeRelationViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EcologyMemberSlotViewV1 {
    pub member_id: String,
    pub location: WorldPosition,
    #[serde(deserialize_with = "deserialize_required_nullable_actor_id")]
    pub actor_id: Option<ActorId>,
    pub vacant: bool,
    #[serde(deserialize_with = "deserialize_required_nullable_logical_time")]
    pub due_at: Option<LogicalTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EcologySiteViewV1 {
    pub site_id: String,
    pub spawn_group_id: String,
    pub generation: u32,
    #[serde(deserialize_with = "deserialize_required_nullable_logical_time")]
    pub full_clear_due_at: Option<LogicalTime>,
    pub member_slots: Vec<EcologyMemberSlotViewV1>,
}

fn deserialize_required_nullable_actor_id<'de, D>(
    deserializer: D,
) -> Result<Option<ActorId>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<ActorId>::deserialize(deserializer)
}

fn deserialize_required_nullable_logical_time<'de, D>(
    deserializer: D,
) -> Result<Option<LogicalTime>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<LogicalTime>::deserialize(deserializer)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct WorldSnapshotV1 {
    pub contract_version: u32,
    pub scope: SnapshotScopeV1,
    pub logical_time: LogicalTime,
    pub controlled_actor_ids: Vec<ActorId>,
    pub rules: RulesViewV1,
    pub spell_social: Vec<SpellSocialCatalogViewV1>,
    pub social_relations: SocialRelationLedgerViewV1,
    pub realms: Vec<RealmSnapshotV1>,
    pub actors: Vec<ActorViewV1>,
    pub automatic_actors: Vec<AutomaticActorViewV1>,
    pub ecology_sites: Vec<EcologySiteViewV1>,
    pub ground_items: Vec<GroundItemViewV1>,
    pub corpses: Vec<CorpseViewV1>,
    pub ground_gold: Vec<GroundGoldPileViewV1>,
    pub tile_effects: Vec<TileEffectViewV1>,
    pub concealed_transitions: Vec<ConcealedTransitionViewV1>,
    pub quest_states: Vec<CharacterQuestStateViewV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TileObservationV1 {
    Visible,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TileSnapshotV2 {
    pub position: Coord,
    pub terrain_id: Option<String>,
    pub terrain_name: Option<String>,
    pub passable: Option<bool>,
    pub move_cost: Option<i32>,
    pub transition: Option<TransitionViewV1>,
    pub observation: TileObservationV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LevelSnapshotV2 {
    pub id: String,
    pub law_zone: super::LawZoneViewV1,
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<TileSnapshotV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RealmSnapshotV2 {
    pub id: String,
    pub name: String,
    pub levels: Vec<LevelSnapshotV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct WorldSnapshotV2 {
    pub contract_version: u32,
    pub logical_time: LogicalTime,
    pub observer_actor_id: ActorId,
    pub observation_center: WorldPosition,
    pub observation_radius: u32,
    pub rules: RulesViewV1,
    pub realms: Vec<RealmSnapshotV2>,
    pub actors: Vec<ObservedActorViewV1>,
    pub ground_items: Vec<GroundItemViewV1>,
    pub corpses: Vec<CorpseViewV1>,
    pub ground_gold: Vec<GroundGoldPileViewV1>,
    pub tile_effects: Vec<TileEffectViewV1>,
}

/// A combined observed frame: snapshot + action context built from one
/// visibility pass. Avoids duplicate line-of-sight computation when both
/// surfaces are needed (e.g. Trace V2 emission).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PlayerObservedFrameV1 {
    pub contract_version: u32,
    pub observed_snapshot: WorldSnapshotV2,
    pub action_context: PlayerActionContextV2,
}
