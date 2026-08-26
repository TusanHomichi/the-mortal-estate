use serde::{Deserialize, Serialize};

use crate::model::{ActorId, CarriedPosition, CharacterAlignment};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionRequirementDef {
    CurrentClass {
        class_id: String,
    },
    MinimumLevel {
        level: i32,
    },
    ExactKarma {
        karma_points: u32,
    },
    ExactAlignment {
        alignment: CharacterAlignment,
    },
    MinimumSkillLevel {
        track_id: String,
        level: u8,
    },
    MinimumCarriedGold {
        amount: i64,
    },
    CarriedItem {
        item_definition_id: String,
        quantity: u32,
    },
    CarriedPositionEmpty {
        position: CarriedPosition,
    },
    SpellUnknown {
        spell_id: String,
    },
    QuestUnstarted {
        quest_id: String,
    },
    QuestAtStage {
        quest_id: String,
        stage_id: String,
    },
    NpcAccompanying {
        npc_actor_id: ActorId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionCostDef {
    CarriedGold { amount: i64 },
    SelectedCarriedItem { quantity: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionRewardDef {
    Experience {
        amount: i32,
    },
    Item {
        item_instance_id: String,
        item_definition_id: String,
        position: CarriedPosition,
    },
    Class {
        to_class_id: String,
        to_class_display: String,
    },
    Spell {
        spell_id: String,
    },
    QuestStage {
        quest_id: String,
        stage_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionDef {
    pub id: String,
    pub label: String,
    pub requirements: Vec<TransactionRequirementDef>,
    pub costs: Vec<TransactionCostDef>,
    pub rewards: Vec<TransactionRewardDef>,
}
