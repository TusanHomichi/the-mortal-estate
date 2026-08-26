use serde::{Deserialize, Serialize};

use super::{ActorId, CharacterId, Transaction, VerticalDirection};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NpcInteractionOutcome {
    Speak,
    BeginFollow,
    EndFollow,
    CompleteEscort { npc_actor_id: ActorId },
    Climb { direction: VerticalDirection },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcInteraction {
    pub transaction: Transaction,
    pub response: String,
    pub outcome: NpcInteractionOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcState {
    pub follow_cadence_units: u32,
    pub interactions: Vec<NpcInteraction>,
    pub following_character_id: Option<CharacterId>,
}
