use serde::{Deserialize, Deserializer, Serialize};

use crate::events::Event;

mod action_context;
mod actors;
mod characters;
mod combat;
mod commands;
mod contract_versions;
mod effects;
mod items;
mod npcs;
mod observer;
mod path;
mod quests;
mod rules;
mod services;
mod snapshots;
mod transactions;
mod world;

pub use action_context::*;
pub use actors::*;
pub use characters::*;
pub use combat::*;
pub use commands::*;
pub use contract_versions::{
    ACTION_CONTEXT_CONTRACT_VERSION, COMMAND_CONTRACT_VERSION, EVENT_CONTRACT_VERSION,
    OBSERVED_SNAPSHOT_CONTRACT_VERSION, PATH_PREVIEW_CONTRACT_VERSION, SNAPSHOT_CONTRACT_VERSION,
    TRACE_CONTRACT_VERSION, TRACE_V2_CONTRACT_VERSION,
};
pub use effects::*;
pub use items::*;
pub use npcs::*;
pub use observer::*;
pub use path::*;
pub use quests::*;
pub use rules::*;
pub use services::*;
pub use snapshots::*;
pub use transactions::*;
pub use world::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TraceHeaderV1 {
    pub contract_version: u32,
    pub scenario_id: String,
    pub seed: u64,
    pub initial_snapshot: WorldSnapshotV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TraceStepV1 {
    pub contract_version: u32,
    pub step_index: usize,
    pub intent_label: String,
    #[serde(deserialize_with = "deserialize_required_nullable_path_preview")]
    pub preview: Option<PathPreviewV1>,
    pub events: Vec<Event>,
    pub after_snapshot: WorldSnapshotV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TraceFinalV1 {
    pub contract_version: u32,
    pub final_snapshot: WorldSnapshotV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TraceV1 {
    pub header: TraceHeaderV1,
    pub steps: Vec<TraceStepV1>,
    pub r#final: TraceFinalV1,
}

// ---------------------------------------------------------------------------
// Trace V2 — full presentation-contract envelope
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TraceHeaderV2 {
    pub contract_version: u32,
    pub scenario_id: String,
    pub seed: u64,
    pub event_contract_version: u32,
    pub snapshot_contract_version: u32,
    pub observed_snapshot_contract_version: u32,
    pub action_context_contract_version: u32,
    pub intent_contract_version: u32,
    pub initial_debug_snapshot: WorldSnapshotV1,
    pub initial_observed_snapshot: WorldSnapshotV2,
    pub initial_action_context: PlayerActionContextV2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TraceStepV2 {
    pub step_index: usize,
    pub command: PlayerCommandV1,
    pub intent_label: String,
    #[serde(deserialize_with = "deserialize_required_nullable_path_preview")]
    pub preview: Option<PathPreviewV1>,
    pub events: Vec<Event>,
    pub after_debug_snapshot: WorldSnapshotV1,
    pub after_observed_snapshot: WorldSnapshotV2,
    pub after_action_context: PlayerActionContextV2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TraceFinalV2 {
    pub contract_version: u32,
    pub final_debug_snapshot: WorldSnapshotV1,
    pub final_observed_snapshot: WorldSnapshotV2,
    pub final_action_context: PlayerActionContextV2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TraceV2 {
    pub header: TraceHeaderV2,
    pub steps: Vec<TraceStepV2>,
    pub r#final: TraceFinalV2,
}

fn deserialize_required_nullable_path_preview<'de, D>(
    deserializer: D,
) -> Result<Option<PathPreviewV1>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<PathPreviewV1>::deserialize(deserializer)
}
