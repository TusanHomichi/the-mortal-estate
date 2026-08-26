use crate::events::{Event, TransactionRewardReceiptV1};
use crate::model::{
    ActorKind, CharacterId, ExplicitTraversalKind, NpcInteractionOutcome, VerticalDirection,
};
use crate::view::ActionBlockedReasonV1;

use super::navigation::{ExplicitTraversalBlockedReason, ExplicitTraversalPlan};
use super::transactions::{
    PlannedReward, TransactionPlan, TransactionPlanError, TransactionSource,
};
use super::{Engine, StepError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NpcInteractionRewardPlan {
    npc_actor_id: crate::model::ActorId,
    interaction_id: String,
    response: String,
    outcome: NpcInteractionOutcome,
    character_id: CharacterId,
    traversal_plan: Option<ExplicitTraversalPlan>,
}

impl Engine {
    pub(super) fn npc_interaction_transaction_plan(
        &self,
        actor_index: usize,
        npc_actor_id: &crate::model::ActorId,
        interaction_id: &str,
        selected_item_instance_id: Option<&str>,
    ) -> Result<TransactionPlan, TransactionPlanError> {
        let actor = self.world.actors.get(actor_index).ok_or_else(|| {
            TransactionPlanError::new(ActionBlockedReasonV1::NoSuchTarget, "unknown actor")
        })?;
        if !actor.is_alive() {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::ActorNotLiving,
                "NPC interaction actor must be living",
            ));
        }
        let character_id = actor.character_id.clone().ok_or_else(|| {
            TransactionPlanError::new(
                ActionBlockedReasonV1::QuestStateMismatch,
                "NPC interaction requires stable character identity",
            )
        })?;
        let npc_index = self
            .world
            .actors
            .iter()
            .position(|candidate| &candidate.id == npc_actor_id)
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::NoSuchNpc,
                    format!("NPC {npc_actor_id:?} was not found"),
                )
            })?;
        let npc_actor = &self.world.actors[npc_index];
        if npc_actor.kind != ActorKind::Npc || npc_actor.npc.is_none() || !npc_actor.is_alive() {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::NoSuchNpc,
                format!("actor {npc_actor_id:?} is not a living NPC"),
            ));
        }
        if npc_actor.location.level != actor.location.level
            || npc_actor.location.position != actor.location.position
        {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::NpcNotHere,
                format!("NPC {npc_actor_id:?} is not at the actor coordinate"),
            ));
        }
        let npc = npc_actor.npc.as_ref().expect("NPC state was checked");
        let interaction = npc
            .interactions
            .iter()
            .find(|interaction| interaction.transaction.id == interaction_id)
            .cloned()
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::NoSuchInteraction,
                    format!("NPC interaction {interaction_id:?} was not found"),
                )
            })?;

        let mut traversal_plan = None;
        match &interaction.outcome {
            NpcInteractionOutcome::Speak => {}
            NpcInteractionOutcome::BeginFollow => {
                if npc.following_character_id.is_some() {
                    return Err(TransactionPlanError::new(
                        ActionBlockedReasonV1::NpcAlreadyFollowing,
                        "NPC is already following a character",
                    ));
                }
            }
            NpcInteractionOutcome::EndFollow => {
                if npc.following_character_id.as_ref() != Some(&character_id) {
                    return Err(TransactionPlanError::new(
                        ActionBlockedReasonV1::NpcNotFollowing,
                        "NPC is not following the interacting character",
                    ));
                }
            }
            NpcInteractionOutcome::CompleteEscort {
                npc_actor_id: escorted_id,
            } => {
                let escorted = self
                    .world
                    .actors
                    .iter()
                    .find(|candidate| candidate.id == *escorted_id)
                    .ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::NpcNotAccompanying,
                            "escorted NPC was not found",
                        )
                    })?;
                if escorted.kind != ActorKind::Npc
                    || !escorted.is_alive()
                    || escorted.location.level != actor.location.level
                    || escorted.location.position != actor.location.position
                    || escorted.npc.as_ref().is_none_or(|state| {
                        state.following_character_id.as_ref() != Some(&character_id)
                    })
                {
                    return Err(TransactionPlanError::new(
                        ActionBlockedReasonV1::NpcNotAccompanying,
                        "escorted NPC is not accompanying the interacting character",
                    ));
                }
            }
            NpcInteractionOutcome::Climb { direction } => {
                if npc.following_character_id.as_ref() != Some(&character_id) {
                    return Err(TransactionPlanError::new(
                        ActionBlockedReasonV1::NpcNotFollowing,
                        "NPC must be following the interacting character before climbing",
                    ));
                }
                let traversal = match direction {
                    VerticalDirection::Up => ExplicitTraversalKind::StairsUp,
                    VerticalDirection::Down => ExplicitTraversalKind::StairsDown,
                };
                traversal_plan = Some(
                    self.evaluate_explicit_traversal(npc_index, traversal)
                        .map_err(|reason| {
                            let message = match reason {
                                ExplicitTraversalBlockedReason::NoTraversalHere => {
                                    "NPC has no traversal here"
                                }
                                ExplicitTraversalBlockedReason::WrongDirection => {
                                    "NPC traversal direction does not match"
                                }
                            };
                            TransactionPlanError::new(
                                ActionBlockedReasonV1::NpcCannotClimb,
                                message,
                            )
                        })?,
                );
            }
        }

        let reward = NpcInteractionRewardPlan {
            npc_actor_id: npc_actor_id.clone(),
            interaction_id: interaction_id.to_string(),
            response: interaction.response,
            outcome: interaction.outcome,
            character_id,
            traversal_plan,
        };
        self.plan_transaction(
            actor_index,
            TransactionSource::NpcInteraction {
                npc_actor_id: npc_actor_id.clone(),
                interaction_id: interaction_id.to_string(),
            },
            &interaction.transaction,
            selected_item_instance_id,
            vec![PlannedReward::NpcInteraction(reward)],
        )
    }

    pub(super) fn apply_npc_interaction_reward(
        &mut self,
        plan: &NpcInteractionRewardPlan,
    ) -> Result<(TransactionRewardReceiptV1, Vec<Event>), StepError> {
        let npc_index = self
            .world
            .actors
            .iter()
            .position(|actor| actor.id == plan.npc_actor_id && actor.kind == ActorKind::Npc)
            .ok_or_else(|| StepError::new("captured NPC disappeared before commit"))?;
        let npc_name = self.world.actors[npc_index].name.clone();
        let mut events = vec![Event::NpcSpoke {
            npc_actor_id: plan.npc_actor_id.clone(),
            npc: npc_name,
            recipient_character_id: plan.character_id.clone(),
            interaction_id: plan.interaction_id.clone(),
            response: plan.response.clone(),
        }];

        match &plan.outcome {
            NpcInteractionOutcome::Speak => {}
            NpcInteractionOutcome::BeginFollow => {
                if self.world.actors[npc_index]
                    .npc
                    .as_ref()
                    .and_then(|npc| npc.following_character_id.as_ref())
                    .is_some()
                {
                    return Err(StepError::new("captured NPC began following before commit"));
                }
                self.set_npc_follow_target(
                    &plan.npc_actor_id,
                    Some(plan.character_id.clone()),
                    &mut events,
                )?;
            }
            NpcInteractionOutcome::EndFollow => {
                if self.world.actors[npc_index]
                    .npc
                    .as_ref()
                    .and_then(|npc| npc.following_character_id.as_ref())
                    != Some(&plan.character_id)
                {
                    return Err(StepError::new(
                        "captured NPC follow state changed before commit",
                    ));
                }
                self.set_npc_follow_target(&plan.npc_actor_id, None, &mut events)?;
            }
            NpcInteractionOutcome::CompleteEscort { npc_actor_id } => {
                let escorted = self
                    .world
                    .actors
                    .iter()
                    .find(|actor| actor.id == *npc_actor_id)
                    .ok_or_else(|| StepError::new("captured escorted NPC disappeared"))?;
                let provider = &self.world.actors[npc_index];
                if escorted.kind != ActorKind::Npc
                    || escorted.location.level != provider.location.level
                    || escorted.location.position != provider.location.position
                    || escorted.npc.as_ref().is_none_or(|npc| {
                        npc.following_character_id.as_ref() != Some(&plan.character_id)
                    })
                {
                    return Err(StepError::new(
                        "captured escort state changed before commit",
                    ));
                }
                self.set_npc_follow_target(npc_actor_id, None, &mut events)?;
            }
            NpcInteractionOutcome::Climb { .. } => {
                let traversal_plan = plan
                    .traversal_plan
                    .as_ref()
                    .ok_or_else(|| StepError::new("captured NPC climb has no stair plan"))?;
                if self.world.actors[npc_index]
                    .npc
                    .as_ref()
                    .and_then(|npc| npc.following_character_id.as_ref())
                    != Some(&plan.character_id)
                {
                    return Err(StepError::new(
                        "captured NPC follow state changed before climb",
                    ));
                }
                self.commit_explicit_traversal(npc_index, traversal_plan, &mut events)?;
            }
        }

        Ok((
            TransactionRewardReceiptV1::NpcInteraction {
                npc_actor_id: plan.npc_actor_id.clone(),
                interaction_id: plan.interaction_id.clone(),
                outcome: plan.outcome.clone(),
            },
            events,
        ))
    }

    pub(super) fn apply_player_npc_interaction(
        &mut self,
        actor_index: usize,
        npc_actor_id: &crate::model::ActorId,
        interaction_id: &str,
        selected_item_instance_id: Option<&str>,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let plan = self.npc_interaction_transaction_plan(
            actor_index,
            npc_actor_id,
            interaction_id,
            selected_item_instance_id,
        )?;
        let actor_id = plan.actor_id.clone();
        let actor_name = plan.actor_name.clone();
        let mut receipt = self.commit_transaction(actor_index, plan)?;
        events.append(&mut receipt.delegated_events);
        events.push(receipt.committed_event(actor_id, actor_name));
        Ok(())
    }
}
