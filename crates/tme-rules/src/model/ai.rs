use serde::{Deserialize, Serialize};

use super::{ActorId, PhysicalAttackMode, WorldPosition};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorAiBehavior {
    SimpleChase,
    PackForager,
    WebAmbush,
    HoldGround,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorAwarenessPolicy {
    Unrestricted,
    LineOfSightMemory { memory_opportunities: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RememberedHostile {
    pub actor_id: ActorId,
    pub last_seen: WorldPosition,
    pub remaining_opportunities: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorAwarenessState {
    pub policy: ActorAwarenessPolicy,
    pub remembered: Option<RememberedHostile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorAiState {
    pub behavior: ActorAiBehavior,
    pub cadence_units: u32,
    pub aggro_radius: u32,
    pub leash_range: u32,
    pub awareness: ActorAwarenessState,
    pub physical_attack_modes: Vec<PhysicalAttackMode>,
    pub returning_home: bool,
}
