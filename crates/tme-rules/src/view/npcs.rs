use serde::{Deserialize, Deserializer, Serialize};

use crate::model::{ActorId, CharacterId, NpcInteractionOutcome};

use super::{
    ActionOptionV1, TransactionCostViewV1, TransactionRequirementViewV1, TransactionRewardViewV1,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct NpcActorStateViewV1 {
    pub follow_cadence_units: u32,
    #[serde(deserialize_with = "deserialize_required_nullable_character_id")]
    pub following_character_id: Option<CharacterId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct NpcInteractionViewV1 {
    pub interaction_id: String,
    pub label: String,
    pub requirements: Vec<TransactionRequirementViewV1>,
    pub costs: Vec<TransactionCostViewV1>,
    pub rewards: Vec<TransactionRewardViewV1>,
    pub outcome: NpcInteractionOutcome,
    pub actions: Vec<ActionOptionV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct NpcViewV1 {
    pub actor_id: ActorId,
    pub name: String,
    #[serde(deserialize_with = "deserialize_required_nullable_character_id")]
    pub following_character_id: Option<CharacterId>,
    pub interactions: Vec<NpcInteractionViewV1>,
}

fn deserialize_required_nullable_character_id<'de, D>(
    deserializer: D,
) -> Result<Option<CharacterId>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<CharacterId>::deserialize(deserializer)
}
