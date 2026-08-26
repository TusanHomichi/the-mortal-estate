use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::CharacterId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QuestId(String);

impl QuestId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QuestStageId(String);

impl QuestStageId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestStage {
    pub id: QuestStageId,
    pub label: String,
    pub terminal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestDefinition {
    pub id: QuestId,
    pub title: String,
    pub stages: BTreeMap<QuestStageId, QuestStage>,
}

pub type QuestStateLedger = BTreeMap<CharacterId, BTreeMap<QuestId, QuestStageId>>;
