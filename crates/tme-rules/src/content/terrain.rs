use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
pub enum TerrainNavigationDef {
    Walk {
        move_cost: i32,
        blocks_sight: bool,
    },
    Swim {
        move_cost: i32,
        blocks_sight: bool,
    },
    Blocked {
        blocks_sight: bool,
    },
    Unresolved {
        source_code: u16,
        question_id: String,
    },
}

impl<'de> Deserialize<'de> for TerrainNavigationDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
        enum Raw {
            Walk {
                move_cost: i32,
                blocks_sight: bool,
            },
            Swim {
                move_cost: i32,
                blocks_sight: bool,
            },
            Blocked {
                blocks_sight: bool,
            },
            Unresolved {
                source_code: u16,
                question_id: String,
            },
        }

        Ok(match Raw::deserialize(deserializer)? {
            Raw::Walk {
                move_cost,
                blocks_sight,
            } => Self::Walk {
                move_cost,
                blocks_sight,
            },
            Raw::Swim {
                move_cost,
                blocks_sight,
            } => Self::Swim {
                move_cost,
                blocks_sight,
            },
            Raw::Blocked { blocks_sight } => Self::Blocked { blocks_sight },
            Raw::Unresolved {
                source_code,
                question_id,
            } => Self::Unresolved {
                source_code,
                question_id,
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainDef {
    pub id: String,
    pub name: String,
    pub navigation: TerrainNavigationDef,
}
