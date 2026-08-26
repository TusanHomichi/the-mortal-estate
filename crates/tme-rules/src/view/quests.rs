use serde::{Deserialize, Serialize};

use crate::model::CharacterId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct QuestStateViewV1 {
    pub quest_id: String,
    pub quest_title: String,
    pub stage_id: String,
    pub stage_label: String,
    pub terminal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CharacterQuestStateViewV1 {
    pub character_id: CharacterId,
    pub quest: QuestStateViewV1,
}
