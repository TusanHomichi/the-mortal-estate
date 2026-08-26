use super::{ActorId, CarriedPosition, CharacterAlignment, QuestId, QuestStageId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionRequirement {
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
        quest_id: QuestId,
    },
    QuestAtStage {
        quest_id: QuestId,
        stage_id: QuestStageId,
    },
    NpcAccompanying {
        npc_actor_id: ActorId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionCost {
    CarriedGold { amount: i64 },
    SelectedCarriedItem { quantity: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionReward {
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
        quest_id: QuestId,
        stage_id: QuestStageId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub id: String,
    pub label: String,
    pub requirements: Vec<TransactionRequirement>,
    pub costs: Vec<TransactionCost>,
    pub rewards: Vec<TransactionReward>,
}
