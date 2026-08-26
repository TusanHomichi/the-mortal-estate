use std::collections::BTreeSet;

use super::resources::MovementResourcePlan;
use super::{Engine, StepError};
use crate::events::Event;
use crate::model::{
    Direction, MovementPace, MovementStopReason, NavigationKind, TerrainTraversal, WorldPosition,
};

#[derive(Debug, Clone)]
pub(super) enum MovementStepOutcome {
    Moved {
        navigation: NavigationKind,
        terrain_name: String,
        cost: i32,
        remaining_after: i32,
    },
    Transitioned {
        kind: NavigationKind,
        target: WorldPosition,
        terrain_name: String,
        cost: i32,
        remaining_after: i32,
    },
    Blocked {
        reason: MovementBlockedReason,
    },
}

#[derive(Debug, Clone, Copy)]
pub(super) enum MovementBlockedReason {
    SuppressedByStatus,
    OutOfBounds,
    BlockedTerrain,
    InsufficientMovementPoints,
}

#[derive(Debug, Clone)]
pub(super) struct MovementStep {
    pub(super) direction: Direction,
    pub(super) from: WorldPosition,
    pub(super) attempted: WorldPosition,
    pub(super) opens_door: bool,
    pub(super) outcome: MovementStepOutcome,
}

#[derive(Debug, Clone)]
pub(super) struct MovementPlan {
    pub(super) pace: MovementPace,
    pub(super) requested_steps: usize,
    pub(super) available_path_points: i32,
    pub(super) steps: Vec<MovementStep>,
    pub(super) accepted_steps: usize,
    pub(super) stop_reason: MovementStopReason,
    pub(super) final_position: WorldPosition,
    pub(super) remaining_path_points: i32,
    pub(super) resources: MovementResourcePlan,
}

struct MovementPlanDraft {
    pace: MovementPace,
    requested_steps: usize,
    available_path_points: i32,
    steps: Vec<MovementStep>,
    stop_reason: MovementStopReason,
    final_position: WorldPosition,
    remaining_path_points: i32,
}

struct BlockedMovementPlan {
    pace: MovementPace,
    requested_steps: usize,
    available_path_points: i32,
    steps: Vec<MovementStep>,
    direction: Direction,
    from: WorldPosition,
    attempted: WorldPosition,
    reason: MovementBlockedReason,
    remaining_path_points: i32,
}

impl Engine {
    fn finish_movement_plan(
        &self,
        actor_index: usize,
        draft: MovementPlanDraft,
    ) -> Result<MovementPlan, StepError> {
        let accepted_steps = draft
            .steps
            .iter()
            .filter(|step| !matches!(step.outcome, MovementStepOutcome::Blocked { .. }))
            .count();
        let difficult_terrain_committed = draft.steps.iter().any(|step| match step.outcome {
            MovementStepOutcome::Moved { cost, .. }
            | MovementStepOutcome::Transitioned { cost, .. } => cost > 1,
            MovementStepOutcome::Blocked { .. } => false,
        });
        let resources = self.movement_resource_plan(
            actor_index,
            draft.pace,
            accepted_steps,
            difficult_terrain_committed,
        )?;
        Ok(MovementPlan {
            pace: draft.pace,
            requested_steps: draft.requested_steps,
            available_path_points: draft.available_path_points,
            steps: draft.steps,
            accepted_steps,
            stop_reason: draft.stop_reason,
            final_position: draft.final_position,
            remaining_path_points: draft.remaining_path_points,
            resources,
        })
    }

    fn blocked_plan(
        &self,
        actor_index: usize,
        mut blocked: BlockedMovementPlan,
    ) -> Result<MovementPlan, StepError> {
        blocked.steps.push(MovementStep {
            direction: blocked.direction,
            from: blocked.from.clone(),
            attempted: blocked.attempted,
            opens_door: false,
            outcome: MovementStepOutcome::Blocked {
                reason: blocked.reason,
            },
        });
        self.finish_movement_plan(
            actor_index,
            MovementPlanDraft {
                pace: blocked.pace,
                requested_steps: blocked.requested_steps,
                available_path_points: blocked.available_path_points,
                steps: blocked.steps,
                stop_reason: MovementStopReason::Blocked,
                final_position: blocked.from,
                remaining_path_points: blocked.remaining_path_points,
            },
        )
    }

    pub(super) fn evaluate_actor_path(
        &self,
        actor_index: usize,
        path: &[Direction],
        available_path_points: i32,
    ) -> Result<MovementPlan, StepError> {
        let pace = MovementPace::from_step_count(path.len()).ok_or_else(|| {
            StepError::new("movement path must contain between one and three directions")
        })?;
        let actor = &self.world.actors[actor_index];
        let mut current = actor.location.clone();
        let mut remaining = available_path_points;
        let mut steps = Vec::new();
        let mut opened_doors = BTreeSet::new();
        let zero_stamina_limit = actor.character.is_some() && actor.stamina == 0;

        if self.suppressing_effect_for_actor(actor_index).is_some() {
            let direction = path[0];
            let attempted = WorldPosition::new(
                &current.realm,
                &current.level,
                current.position.step(direction),
            );
            return self.blocked_plan(
                actor_index,
                BlockedMovementPlan {
                    pace,
                    requested_steps: path.len(),
                    available_path_points,
                    steps,
                    direction,
                    from: current,
                    attempted,
                    reason: MovementBlockedReason::SuppressedByStatus,
                    remaining_path_points: remaining,
                },
            );
        }

        for (step_index, direction) in path.iter().copied().enumerate() {
            let from = current.clone();
            let (delta_x, delta_y) = direction.delta();
            let attempted = WorldPosition::new(
                &current.realm,
                &current.level,
                current.position.step(direction),
            );
            if !self.in_bounds(&attempted) {
                return self.blocked_plan(
                    actor_index,
                    BlockedMovementPlan {
                        pace,
                        requested_steps: path.len(),
                        available_path_points,
                        steps,
                        direction,
                        from,
                        attempted,
                        reason: MovementBlockedReason::OutOfBounds,
                        remaining_path_points: remaining,
                    },
                );
            }
            if delta_x != 0 && delta_y != 0 {
                let horizontal = WorldPosition::new(
                    &current.realm,
                    &current.level,
                    crate::model::Coord {
                        x: current.position.x + delta_x,
                        y: current.position.y,
                    },
                );
                let vertical = WorldPosition::new(
                    &current.realm,
                    &current.level,
                    crate::model::Coord {
                        x: current.position.x,
                        y: current.position.y + delta_y,
                    },
                );
                if !self.is_walkable(&horizontal) || !self.is_walkable(&vertical) {
                    return self.blocked_plan(
                        actor_index,
                        BlockedMovementPlan {
                            pace,
                            requested_steps: path.len(),
                            available_path_points,
                            steps,
                            direction,
                            from,
                            attempted,
                            reason: MovementBlockedReason::BlockedTerrain,
                            remaining_path_points: remaining,
                        },
                    );
                }
            }
            let Some(tile) = self.effective_tile_at(&attempted) else {
                return self.blocked_plan(
                    actor_index,
                    BlockedMovementPlan {
                        pace,
                        requested_steps: path.len(),
                        available_path_points,
                        steps,
                        direction,
                        from,
                        attempted,
                        reason: MovementBlockedReason::BlockedTerrain,
                        remaining_path_points: remaining,
                    },
                );
            };
            if !tile.passable {
                return self.blocked_plan(
                    actor_index,
                    BlockedMovementPlan {
                        pace,
                        requested_steps: path.len(),
                        available_path_points,
                        steps,
                        direction,
                        from,
                        attempted,
                        reason: MovementBlockedReason::BlockedTerrain,
                        remaining_path_points: remaining,
                    },
                );
            }
            let cost = tile
                .move_cost
                .ok_or_else(|| StepError::new("passable terrain is missing move_cost"))?;
            if cost > remaining {
                return self.blocked_plan(
                    actor_index,
                    BlockedMovementPlan {
                        pace,
                        requested_steps: path.len(),
                        available_path_points,
                        steps,
                        direction,
                        from,
                        attempted,
                        reason: MovementBlockedReason::InsufficientMovementPoints,
                        remaining_path_points: remaining,
                    },
                );
            }

            let automatic = self.automatic_navigation_at(&attempted);
            let opens_door = automatic
                .as_ref()
                .is_some_and(|edge| edge.kind == NavigationKind::Door)
                && !opened_doors.contains(&attempted)
                && !self.effective_door_state_at(&attempted).unwrap_or(false);
            if opens_door {
                opened_doors.insert(attempted.clone());
            }
            remaining -= cost;
            if let Some(edge) = automatic {
                current = edge.target.clone();
                steps.push(MovementStep {
                    direction,
                    from,
                    attempted,
                    opens_door,
                    outcome: MovementStepOutcome::Transitioned {
                        kind: edge.kind,
                        target: edge.target,
                        terrain_name: tile.terrain_name,
                        cost,
                        remaining_after: remaining,
                    },
                });
                if edge.kind != NavigationKind::Door {
                    return self.finish_movement_plan(
                        actor_index,
                        MovementPlanDraft {
                            pace,
                            requested_steps: path.len(),
                            available_path_points,
                            steps,
                            stop_reason: MovementStopReason::Transitioned,
                            final_position: current,
                            remaining_path_points: remaining,
                        },
                    );
                }
            } else {
                current = attempted.clone();
                let navigation = match tile.traversal {
                    Some(TerrainTraversal::Swim) => NavigationKind::Swim,
                    _ => NavigationKind::Walk,
                };
                steps.push(MovementStep {
                    direction,
                    from,
                    attempted,
                    opens_door: false,
                    outcome: MovementStepOutcome::Moved {
                        navigation,
                        terrain_name: tile.terrain_name,
                        cost,
                        remaining_after: remaining,
                    },
                });
            }
            if zero_stamina_limit && step_index + 1 < path.len() {
                return self.finish_movement_plan(
                    actor_index,
                    MovementPlanDraft {
                        pace,
                        requested_steps: path.len(),
                        available_path_points,
                        steps,
                        stop_reason: MovementStopReason::ZeroStaminaLimit,
                        final_position: current,
                        remaining_path_points: remaining,
                    },
                );
            }
        }

        self.finish_movement_plan(
            actor_index,
            MovementPlanDraft {
                pace,
                requested_steps: path.len(),
                available_path_points,
                steps,
                stop_reason: MovementStopReason::FullPathAccepted,
                final_position: current,
                remaining_path_points: remaining,
            },
        )
    }

    pub(super) fn commit_actor_path(
        &mut self,
        actor_index: usize,
        plan: &MovementPlan,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let actor_id = self.world.actors[actor_index].id.clone();
        let actor_name = self.world.actors[actor_index].name.clone();
        events.push(Event::MovementStarted {
            actor_id: actor_id.clone(),
            actor: actor_name.clone(),
            pace: plan.pace,
            requested_steps: plan.requested_steps,
            accepted_steps: plan.accepted_steps,
            available_path_points: plan.available_path_points,
            burden_tier: plan.resources.burden.tier,
            exertion: plan.resources.exertion,
            stamina_cost: plan.resources.stamina_cost,
            stop_reason: plan.stop_reason,
        });

        for step in &plan.steps {
            let (navigation, terrain_name, cost, remaining_after) = match &step.outcome {
                MovementStepOutcome::Moved {
                    navigation,
                    terrain_name,
                    cost,
                    remaining_after,
                } => (*navigation, terrain_name, *cost, *remaining_after),
                MovementStepOutcome::Transitioned {
                    kind,
                    terrain_name,
                    cost,
                    remaining_after,
                    ..
                } => (*kind, terrain_name, *cost, *remaining_after),
                MovementStepOutcome::Blocked { reason } => {
                    let reason = match reason {
                        MovementBlockedReason::SuppressedByStatus => "suppressed by status",
                        MovementBlockedReason::OutOfBounds => "out of bounds",
                        MovementBlockedReason::BlockedTerrain => "blocked terrain",
                        MovementBlockedReason::InsufficientMovementPoints => {
                            "insufficient movement points"
                        }
                    };
                    events.push(Event::MovementBlocked {
                        actor_id: actor_id.clone(),
                        actor: actor_name.clone(),
                        from: step.from.clone(),
                        attempted: step.attempted.clone(),
                        reason: reason.to_string(),
                    });
                    continue;
                }
            };
            if step.opens_door {
                self.open_door_at(actor_index, &step.attempted, events)?;
            }
            events.push(Event::MovementCostPaid {
                actor_id: actor_id.clone(),
                actor: actor_name.clone(),
                site: step.from.site(),
                direction: step.direction,
                navigation,
                terrain: terrain_name.clone(),
                cost,
                remaining_points: remaining_after,
                destination: step.attempted.clone(),
            });
            match &step.outcome {
                MovementStepOutcome::Moved { navigation, .. } => {
                    self.world.actors[actor_index].location = step.attempted.clone();
                    events.push(Event::Moved {
                        actor_id: actor_id.clone(),
                        actor: actor_name.clone(),
                        from: step.from.clone(),
                        to: step.attempted.clone(),
                        navigation: *navigation,
                    });
                }
                MovementStepOutcome::Transitioned { kind, target, .. } => {
                    self.world.actors[actor_index].location = target.clone();
                    events.push(Event::WorldTransition {
                        actor_id: actor_id.clone(),
                        actor: actor_name.clone(),
                        from: step.from.clone(),
                        to: target.clone(),
                        navigation: *kind,
                    });
                }
                MovementStepOutcome::Blocked { .. } => unreachable!("handled above"),
            }
        }
        if plan.accepted_steps > 0 {
            self.break_spell_hidden_after_uncovered_move(actor_index, events);
            self.unload_actor_bow_after_movement(actor_index, events)?;
        }
        self.commit_movement_stamina(actor_index, plan.pace, &plan.resources, events)?;
        Ok(())
    }

    pub(super) fn resolve_player_path(
        &mut self,
        player_index: usize,
        path: &[Direction],
        events: &mut Vec<Event>,
    ) -> Result<bool, StepError> {
        let plan = self.evaluate_actor_path(
            player_index,
            path,
            self.definition
                .catalog
                .rules
                .movement
                .controlled_path_points,
        )?;
        let active = plan.accepted_steps > 0;
        self.commit_actor_path(player_index, &plan, events)?;
        Ok(active)
    }

    pub(super) fn try_actor_move(
        &mut self,
        actor_index: usize,
        direction: Direction,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let plan = self.evaluate_actor_path(
            actor_index,
            &[direction],
            self.definition.catalog.rules.movement.automatic_step_points,
        )?;
        self.commit_actor_path(actor_index, &plan, events)
    }
}
