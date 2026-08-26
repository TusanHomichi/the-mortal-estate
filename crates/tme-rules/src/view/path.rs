use serde::{Deserialize, Serialize};

use crate::model::{
    ActorId, Direction, MovementExertion, MovementPace, MovementStopReason, WorldPosition,
};

use super::{BurdenViewV1, TransitionKindViewV1};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PathPreviewV1 {
    pub contract_version: u32,
    pub actor_id: ActorId,
    pub start: WorldPosition,
    pub pace: MovementPace,
    pub requested_path: Vec<Direction>,
    pub available_path_points: i32,
    pub accepted_steps: usize,
    pub steps: Vec<PathPreviewStepV1>,
    pub stop_reason: MovementStopReason,
    pub final_position: WorldPosition,
    pub remaining_path_points: i32,
    pub burden: BurdenViewV1,
    pub movement_exertion: MovementExertion,
    pub stamina_before: Option<i32>,
    pub stamina_cost: Option<i32>,
    pub stamina_after: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PathPreviewStepV1 {
    pub index: usize,
    pub direction: Direction,
    pub from: WorldPosition,
    pub attempted: WorldPosition,
    pub opens_door: bool,
    pub terrain_name: Option<String>,
    pub cost: Option<i32>,
    pub remaining_points_after: Option<i32>,
    pub outcome: PathPreviewStepOutcomeV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum PathPreviewStepOutcomeV1 {
    Moved {
        kind: TransitionKindViewV1,
    },
    Transitioned {
        kind: TransitionKindViewV1,
        to: WorldPosition,
    },
    Blocked {
        reason: PathPreviewBlockedReasonV1,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathPreviewBlockedReasonV1 {
    SuppressedByStatus,
    OutOfBounds,
    BlockedTerrain,
    InsufficientMovementPoints,
}
