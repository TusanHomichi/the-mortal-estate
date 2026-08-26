use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::{Coord, VerticalDirection, WorldPosition};

use super::LawZoneDef;

pub const WORLD_TEMPLATE_SCHEMA_VERSION: u32 = 3;
pub const WORLD_TEMPLATE_KIND: &str = "world_template";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldTemplateV3 {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub visual_manifest_digest: String,
    pub realms: BTreeMap<String, RealmDef>,
    pub arrivals: BTreeMap<String, WorldPosition>,
    pub topology: BTreeMap<String, TopologyEdgeDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealmDef {
    pub name: String,
    pub levels: BTreeMap<String, LevelDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelDef {
    pub law_zone: LawZoneDef,
    pub scene_role: SceneRoleDef,
    pub presentation_mode: PresentationModeDef,
    pub world_zoom: WorldZoomDef,
    pub maximum_clear_sightline: u32,
    #[serde(deserialize_with = "deserialize_required_nullable_staged_viewport")]
    pub staged_viewport: Option<StagedViewportDef>,
    pub wall_terrain_ids: Vec<String>,
    pub static_props: Vec<StaticPropDef>,
    pub width: i32,
    pub height: i32,
    pub cells: Vec<Vec<Vec<Option<String>>>>,
}

fn deserialize_required_nullable_staged_viewport<'de, D>(
    deserializer: D,
) -> Result<Option<StagedViewportDef>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<StagedViewportDef>::deserialize(deserializer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneRoleDef {
    Overworld,
    CombatSpace,
    Interior,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationModeDef {
    OverworldTown,
    CombatSpace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldZoomDef {
    pub screen_cell_pitch: [u32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagedViewportDef {
    pub frame_size: [u32; 2],
    pub fit_whole_level: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticPropDef {
    pub id: String,
    pub visual_family: String,
    pub anchor: Coord,
    pub layer: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyEdgeDef {
    pub at: WorldPosition,
    pub target: TopologyTargetDef,
    pub kind: TopologyKindDef,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
pub enum TopologyTargetDef {
    Position { location: WorldPosition },
    Arrival { arrival_id: String },
}

impl<'de> Deserialize<'de> for TopologyTargetDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
        enum Raw {
            Position { location: WorldPosition },
            Arrival { arrival_id: String },
        }
        Ok(match Raw::deserialize(deserializer)? {
            Raw::Position { location } => Self::Position { location },
            Raw::Arrival { arrival_id } => Self::Arrival { arrival_id },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
pub enum TopologyKindDef {
    Door {
        binding_id: String,
        endpoint_id: String,
        reciprocal_endpoint_id: String,
        initial_state: DoorStateDef,
    },
    Stairs {
        direction: VerticalDirection,
    },
    Pit,
    Climb {
        direction: VerticalDirection,
    },
    Passage,
    Portal,
}

impl<'de> Deserialize<'de> for TopologyKindDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
        enum Raw {
            Door {
                binding_id: String,
                endpoint_id: String,
                reciprocal_endpoint_id: String,
                initial_state: DoorStateDef,
            },
            Stairs {
                direction: VerticalDirection,
            },
            Pit,
            Climb {
                direction: VerticalDirection,
            },
            Passage,
            Portal,
        }
        Ok(match Raw::deserialize(deserializer)? {
            Raw::Door {
                binding_id,
                endpoint_id,
                reciprocal_endpoint_id,
                initial_state,
            } => Self::Door {
                binding_id,
                endpoint_id,
                reciprocal_endpoint_id,
                initial_state,
            },
            Raw::Stairs { direction } => Self::Stairs { direction },
            Raw::Pit => Self::Pit,
            Raw::Climb { direction } => Self::Climb { direction },
            Raw::Passage => Self::Passage,
            Raw::Portal => Self::Portal,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoorStateDef {
    Open,
    Closed,
}
