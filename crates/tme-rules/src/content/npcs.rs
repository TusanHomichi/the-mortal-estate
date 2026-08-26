use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

use crate::model::{ActorId, VerticalDirection};

use super::TransactionDef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NpcInteractionOutcomeDef {
    Speak,
    BeginFollow,
    EndFollow,
    CompleteEscort { npc_actor_id: ActorId },
    Climb { direction: VerticalDirection },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum StrictNpcInteractionOutcomeDef {
    Speak {},
    BeginFollow {},
    EndFollow {},
    CompleteEscort { npc_actor_id: ActorId },
    Climb { direction: VerticalDirection },
}

impl<'de> Deserialize<'de> for NpcInteractionOutcomeDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let strict =
            StrictNpcInteractionOutcomeDef::deserialize(deserializer).map_err(de::Error::custom)?;
        Ok(match strict {
            StrictNpcInteractionOutcomeDef::Speak {} => Self::Speak,
            StrictNpcInteractionOutcomeDef::BeginFollow {} => Self::BeginFollow,
            StrictNpcInteractionOutcomeDef::EndFollow {} => Self::EndFollow,
            StrictNpcInteractionOutcomeDef::CompleteEscort { npc_actor_id } => {
                Self::CompleteEscort { npc_actor_id }
            }
            StrictNpcInteractionOutcomeDef::Climb { direction } => Self::Climb { direction },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcInteractionDef {
    pub transaction: TransactionDef,
    pub response: String,
    pub outcome: NpcInteractionOutcomeDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcDef {
    pub follow_cadence_units: u32,
    pub interactions: Vec<NpcInteractionDef>,
}
