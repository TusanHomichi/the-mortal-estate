use serde::{Deserialize, Serialize};

use crate::model::{
    ActorId, CarriedPosition, CharacterAlignment, TransactionCost, TransactionRequirement,
    TransactionReward,
};

use super::ActionOptionV1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionRequirementViewV1 {
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

impl From<&TransactionRequirement> for TransactionRequirementViewV1 {
    fn from(value: &TransactionRequirement) -> Self {
        match value {
            TransactionRequirement::CurrentClass { class_id } => Self::CurrentClass {
                class_id: class_id.clone(),
            },
            TransactionRequirement::MinimumLevel { level } => Self::MinimumLevel { level: *level },
            TransactionRequirement::ExactKarma { karma_points } => Self::ExactKarma {
                karma_points: *karma_points,
            },
            TransactionRequirement::ExactAlignment { alignment } => Self::ExactAlignment {
                alignment: *alignment,
            },
            TransactionRequirement::MinimumSkillLevel { track_id, level } => {
                Self::MinimumSkillLevel {
                    track_id: track_id.clone(),
                    level: *level,
                }
            }
            TransactionRequirement::MinimumCarriedGold { amount } => {
                Self::MinimumCarriedGold { amount: *amount }
            }
            TransactionRequirement::CarriedItem {
                item_definition_id,
                quantity,
            } => Self::CarriedItem {
                item_definition_id: item_definition_id.clone(),
                quantity: *quantity,
            },
            TransactionRequirement::CarriedPositionEmpty { position } => {
                Self::CarriedPositionEmpty {
                    position: *position,
                }
            }
            TransactionRequirement::SpellUnknown { spell_id } => Self::SpellUnknown {
                spell_id: spell_id.clone(),
            },
            TransactionRequirement::QuestUnstarted { quest_id } => Self::QuestUnstarted {
                quest_id: quest_id.as_str().to_string(),
            },
            TransactionRequirement::QuestAtStage { quest_id, stage_id } => Self::QuestAtStage {
                quest_id: quest_id.as_str().to_string(),
                stage_id: stage_id.as_str().to_string(),
            },
            TransactionRequirement::NpcAccompanying { npc_actor_id } => Self::NpcAccompanying {
                npc_actor_id: npc_actor_id.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionCostViewV1 {
    CarriedGold { amount: i64 },
    SelectedCarriedItem { quantity: u32 },
}

impl From<&TransactionCost> for TransactionCostViewV1 {
    fn from(value: &TransactionCost) -> Self {
        match value {
            TransactionCost::CarriedGold { amount } => Self::CarriedGold { amount: *amount },
            TransactionCost::SelectedCarriedItem { quantity } => Self::SelectedCarriedItem {
                quantity: *quantity,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionRewardViewV1 {
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

impl From<&TransactionReward> for TransactionRewardViewV1 {
    fn from(value: &TransactionReward) -> Self {
        match value {
            TransactionReward::Experience { amount } => Self::Experience { amount: *amount },
            TransactionReward::Item {
                item_instance_id,
                item_definition_id,
                position,
            } => Self::Item {
                item_instance_id: item_instance_id.clone(),
                item_definition_id: item_definition_id.clone(),
                position: *position,
            },
            TransactionReward::Class {
                to_class_id,
                to_class_display,
            } => Self::Class {
                to_class_id: to_class_id.clone(),
                to_class_display: to_class_display.clone(),
            },
            TransactionReward::Spell { spell_id } => Self::Spell {
                spell_id: spell_id.clone(),
            },
            TransactionReward::QuestStage { quest_id, stage_id } => Self::QuestStage {
                quest_id: quest_id.as_str().to_string(),
                stage_id: stage_id.as_str().to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ServiceTransactionViewV1 {
    pub transaction_id: String,
    pub label: String,
    pub requirements: Vec<TransactionRequirementViewV1>,
    pub costs: Vec<TransactionCostViewV1>,
    pub rewards: Vec<TransactionRewardViewV1>,
    pub actions: Vec<ActionOptionV1>,
}
