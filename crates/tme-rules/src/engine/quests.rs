use crate::events::{Event, TransactionRewardReceiptV1};
use crate::model::{CharacterId, QuestId, QuestStageId};
use crate::view::ActionBlockedReasonV1;

use super::transactions::TransactionPlanError;
use super::{Engine, StepError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct QuestTransitionPlan {
    pub(super) character_id: CharacterId,
    pub(super) quest_id: QuestId,
    pub(super) before_stage_id: Option<QuestStageId>,
    pub(super) after_stage_id: QuestStageId,
}

impl Engine {
    fn quest_state_view(
        &self,
        quest_id: &QuestId,
        stage_id: &QuestStageId,
    ) -> Option<crate::view::QuestStateViewV1> {
        let quest = self.definition.catalog.quests.get(quest_id)?;
        let stage = quest.stages.get(stage_id)?;
        Some(crate::view::QuestStateViewV1 {
            quest_id: quest.id.as_str().to_string(),
            quest_title: quest.title.clone(),
            stage_id: stage.id.as_str().to_string(),
            stage_label: stage.label.clone(),
            terminal: stage.terminal,
        })
    }

    pub(super) fn sorted_character_quest_state_views(
        &self,
    ) -> Vec<crate::view::CharacterQuestStateViewV1> {
        self.world
            .quest_states
            .iter()
            .flat_map(|(character_id, quests)| {
                quests.iter().filter_map(move |(quest_id, stage_id)| {
                    Some(crate::view::CharacterQuestStateViewV1 {
                        character_id: character_id.clone(),
                        quest: self.quest_state_view(quest_id, stage_id)?,
                    })
                })
            })
            .collect()
    }

    pub(super) fn quest_log_for_actor(
        &self,
        actor_index: usize,
    ) -> Vec<crate::view::QuestStateViewV1> {
        let Some(character_id) = self
            .world
            .actors
            .get(actor_index)
            .and_then(|actor| actor.character_id.as_ref())
        else {
            return Vec::new();
        };
        self.world
            .quest_states
            .get(character_id)
            .into_iter()
            .flat_map(|quests| quests.iter())
            .filter_map(|(quest_id, stage_id)| self.quest_state_view(quest_id, stage_id))
            .collect()
    }

    pub(super) fn quest_stage_for_character(
        &self,
        character_id: &CharacterId,
        quest_id: &QuestId,
    ) -> Option<&QuestStageId> {
        self.world
            .quest_states
            .get(character_id)
            .and_then(|quests| quests.get(quest_id))
    }

    pub(super) fn plan_quest_stage_reward(
        &self,
        actor_index: usize,
        quest_id: &QuestId,
        after_stage_id: &QuestStageId,
    ) -> Result<QuestTransitionPlan, TransactionPlanError> {
        let actor = self.world.actors.get(actor_index).ok_or_else(|| {
            TransactionPlanError::new(ActionBlockedReasonV1::NoSuchTarget, "unknown actor")
        })?;
        let character_id = actor.character_id.clone().ok_or_else(|| {
            TransactionPlanError::new(
                ActionBlockedReasonV1::QuestStateMismatch,
                "quest transition requires stable character identity",
            )
        })?;
        let quest = self
            .definition
            .catalog
            .quests
            .get(quest_id)
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::QuestStateMismatch,
                    format!("quest {:?} is not defined", quest_id.as_str()),
                )
            })?;
        if !quest.stages.contains_key(after_stage_id) {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::QuestStateMismatch,
                format!(
                    "quest {:?} has no stage {:?}",
                    quest_id.as_str(),
                    after_stage_id.as_str()
                ),
            ));
        }
        let before_stage_id = self
            .quest_stage_for_character(&character_id, quest_id)
            .cloned();
        if before_stage_id.as_ref() == Some(after_stage_id) {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::AlreadyComplete,
                "quest is already at the requested stage",
            ));
        }
        Ok(QuestTransitionPlan {
            character_id,
            quest_id: quest_id.clone(),
            before_stage_id,
            after_stage_id: after_stage_id.clone(),
        })
    }

    pub(super) fn apply_quest_transition(
        &mut self,
        plan: &QuestTransitionPlan,
    ) -> Result<(TransactionRewardReceiptV1, Event), StepError> {
        let current = self
            .quest_stage_for_character(&plan.character_id, &plan.quest_id)
            .cloned();
        if current != plan.before_stage_id {
            return Err(StepError::new(
                "captured quest state changed before transaction commit",
            ));
        }
        let quest = self
            .definition
            .catalog
            .quests
            .get(&plan.quest_id)
            .ok_or_else(|| StepError::new("captured quest definition disappeared"))?;
        if !quest.stages.contains_key(&plan.after_stage_id) {
            return Err(StepError::new("captured quest stage disappeared"));
        }
        self.world
            .quest_states
            .entry(plan.character_id.clone())
            .or_default()
            .insert(plan.quest_id.clone(), plan.after_stage_id.clone());

        let quest_id = plan.quest_id.as_str().to_string();
        let before_stage_id = plan
            .before_stage_id
            .as_ref()
            .map(|stage| stage.as_str().to_string());
        let after_stage_id = plan.after_stage_id.as_str().to_string();
        Ok((
            TransactionRewardReceiptV1::QuestStage {
                character_id: plan.character_id.clone(),
                quest_id: quest_id.clone(),
                before_stage_id: before_stage_id.clone(),
                after_stage_id: after_stage_id.clone(),
            },
            Event::QuestStateChanged {
                character_id: plan.character_id.clone(),
                quest_id,
                before_stage_id,
                after_stage_id,
            },
        ))
    }
}
