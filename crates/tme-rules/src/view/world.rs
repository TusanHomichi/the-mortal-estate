use serde::{Deserialize, Serialize};

use crate::model::{
    ActorId, ConcealedTransitionState, Coord, LogicalTime, VerticalDirection, WorldPosition,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionKindViewV1 {
    Walk,
    Swim,
    Door,
    Stairs { direction: VerticalDirection },
    Pit,
    Climb { direction: VerticalDirection },
    Passage,
    Portal,
}

impl From<crate::model::NavigationKind> for TransitionKindViewV1 {
    fn from(kind: crate::model::NavigationKind) -> Self {
        match kind {
            crate::model::NavigationKind::Walk => Self::Walk,
            crate::model::NavigationKind::Swim => Self::Swim,
            crate::model::NavigationKind::Door => Self::Door,
            crate::model::NavigationKind::Stairs { direction } => Self::Stairs { direction },
            crate::model::NavigationKind::Pit => Self::Pit,
            crate::model::NavigationKind::Climb { direction } => Self::Climb { direction },
            crate::model::NavigationKind::Passage => Self::Passage,
            crate::model::NavigationKind::Portal => Self::Portal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoorStateViewV1 {
    Open,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LawZoneViewV1 {
    None,
    Town,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TransitionViewV1 {
    pub kind: TransitionKindViewV1,
    pub target: WorldPosition,
    pub door_state: Option<DoorStateViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConcealedTransitionViewV1 {
    pub instance_id: String,
    pub source_spell_id: String,
    pub source_actor_id: ActorId,
    pub location: WorldPosition,
    pub remaining_rounds: u32,
    pub last_ticked_at: LogicalTime,
}

impl From<&ConcealedTransitionState> for ConcealedTransitionViewV1 {
    fn from(state: &ConcealedTransitionState) -> Self {
        Self {
            instance_id: state.instance_id.clone(),
            source_spell_id: state.source_spell_id.clone(),
            source_actor_id: state.source_actor_id.clone(),
            location: state.location.clone(),
            remaining_rounds: state.remaining_rounds,
            last_ticked_at: state.last_ticked_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TileSnapshotV1 {
    pub position: Coord,
    pub terrain_id: String,
    pub terrain_name: String,
    pub passable: bool,
    pub move_cost: Option<i32>,
    pub transition: Option<TransitionViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LevelSnapshotV1 {
    pub id: String,
    pub law_zone: LawZoneViewV1,
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<TileSnapshotV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RealmSnapshotV1 {
    pub id: String,
    pub name: String,
    pub levels: Vec<LevelSnapshotV1>,
}
