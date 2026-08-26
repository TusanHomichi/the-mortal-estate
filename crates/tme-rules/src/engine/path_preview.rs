use crate::model::Direction;
use crate::view::{
    PathPreviewBlockedReasonV1, PathPreviewStepOutcomeV1, PathPreviewStepV1, PathPreviewV1,
    TransitionKindViewV1,
};

use super::movement::{MovementBlockedReason, MovementStepOutcome};
use super::{Engine, StepError};

impl Engine {
    /// Project the shared read-only movement plan into the public Path V4 view.
    pub fn preview_actor_path(
        &self,
        actor_id: &crate::model::ActorId,
        path: &[Direction],
    ) -> Result<PathPreviewV1, StepError> {
        let player_index = self.controlled_actor_index(actor_id)?;
        let player = &self.world.actors[player_index];
        let start = player.location.clone();
        let plan = self.evaluate_actor_path(
            player_index,
            path,
            self.definition
                .catalog
                .rules
                .movement
                .controlled_path_points,
        )?;

        let steps = plan
            .steps
            .iter()
            .enumerate()
            .map(|(index, step)| {
                let (outcome, terrain_name, cost, remaining_after) = match &step.outcome {
                    MovementStepOutcome::Moved {
                        navigation,
                        terrain_name,
                        cost,
                        remaining_after,
                    } => (
                        PathPreviewStepOutcomeV1::Moved {
                            kind: TransitionKindViewV1::from(*navigation),
                        },
                        Some(terrain_name.clone()),
                        Some(*cost),
                        Some(*remaining_after),
                    ),
                    MovementStepOutcome::Transitioned {
                        kind,
                        target,
                        terrain_name,
                        cost,
                        remaining_after,
                    } => (
                        PathPreviewStepOutcomeV1::Transitioned {
                            kind: TransitionKindViewV1::from(*kind),
                            to: target.clone(),
                        },
                        Some(terrain_name.clone()),
                        Some(*cost),
                        Some(*remaining_after),
                    ),
                    MovementStepOutcome::Blocked { reason } => (
                        PathPreviewStepOutcomeV1::Blocked {
                            reason: match reason {
                                MovementBlockedReason::SuppressedByStatus => {
                                    PathPreviewBlockedReasonV1::SuppressedByStatus
                                }
                                MovementBlockedReason::InsufficientMovementPoints => {
                                    PathPreviewBlockedReasonV1::InsufficientMovementPoints
                                }
                                MovementBlockedReason::OutOfBounds => {
                                    PathPreviewBlockedReasonV1::OutOfBounds
                                }
                                MovementBlockedReason::BlockedTerrain => {
                                    PathPreviewBlockedReasonV1::BlockedTerrain
                                }
                            },
                        },
                        None,
                        None,
                        None,
                    ),
                };
                PathPreviewStepV1 {
                    index,
                    direction: step.direction,
                    from: step.from.clone(),
                    attempted: step.attempted.clone(),
                    opens_door: step.opens_door,
                    terrain_name,
                    cost,
                    remaining_points_after: remaining_after,
                    outcome,
                }
            })
            .collect();

        Ok(PathPreviewV1 {
            contract_version: crate::view::PATH_PREVIEW_CONTRACT_VERSION,
            actor_id: player.id.clone(),
            start,
            pace: plan.pace,
            requested_path: path.to_vec(),
            available_path_points: plan.available_path_points,
            accepted_steps: plan.accepted_steps,
            steps,
            stop_reason: plan.stop_reason,
            final_position: plan.final_position,
            remaining_path_points: plan.remaining_path_points,
            burden: plan.resources.burden,
            movement_exertion: plan.resources.exertion,
            stamina_before: plan.resources.stamina_before,
            stamina_cost: plan.resources.stamina_cost,
            stamina_after: plan.resources.stamina_after,
        })
    }
}
