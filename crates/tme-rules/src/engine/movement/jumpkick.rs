//! A closing attack borrows navigation legality and step commitment, not a second
//! movement action. No partial route, door opening, or automatic transition can
//! become a successful approach.

use super::{MovementBlockedReason, MovementPlan, MovementStepOutcome};
use crate::engine::{Engine, StepError};
use crate::events::Event;
use crate::model::{Direction, MovementStopReason, NavigationKind, TerrainTraversal, WorldPosition};
use crate::view::ActionBlockedReasonV1;

impl Engine {
    pub(in crate::engine) fn jumpkick_approach_plan(
        &self,
        actor_index: usize,
        target: &WorldPosition,
    ) -> Result<MovementPlan, StepError> {
        let origin = &self.world.actors[actor_index].location;
        if !origin.same_site(target) {
            return Err(StepError::blocked(
                ActionBlockedReasonV1::InvalidTarget,
                "jumpkick approach must stay in the same room",
            ));
        }
        let distance = origin.position.chebyshev_distance(target.position);
        if !(1..=crate::model::MAX_CONTROLLED_PATH_STEPS as i32).contains(&distance) {
            return Err(StepError::blocked(
                ActionBlockedReasonV1::OutOfRange,
                "jumpkick target is out of range",
            ));
        }
        if !self.is_walkable(origin)
            || !self.effective_tile_at(origin).is_some_and(|tile| {
                tile.traversal == Some(TerrainTraversal::Walk)
            })
        {
            return Err(StepError::blocked(
                ActionBlockedReasonV1::BlockedTerrain,
                "jumpkick requires a walkable takeoff tile",
            ));
        }

        // Deterministic direct approach: diagonal toward the target until one
        // coordinate matches, then cardinal. Never search around an obstacle.
        let mut at = origin.position;
        let mut path = Vec::new();
        while at != target.position {
            let delta = (
                (target.position.x - at.x).signum(),
                (target.position.y - at.y).signum(),
            );
            let direction = Direction::all()
                .into_iter()
                .find(|direction| direction.delta() == delta)
                .ok_or_else(|| StepError::new("jumpkick direction is missing"))?;
            path.push(direction);
            at = at.step(direction);
        }
        let plan = self.evaluate_actor_path(
            actor_index,
            &path,
            self.definition.catalog.rules.movement.controlled_path_points,
        )?;
        if plan.steps.iter().any(|step| step.opens_door) {
            return Err(StepError::blocked(
                ActionBlockedReasonV1::ClosedDoor,
                "jumpkick approach cannot open a door",
            ));
        }
        if plan.stop_reason != MovementStopReason::FullPathAccepted
            || plan.accepted_steps != path.len()
            || plan.final_position != *target
        {
            let reason = match plan.steps.last().map(|step| &step.outcome) {
                Some(MovementStepOutcome::Blocked { reason }) => match reason {
                    MovementBlockedReason::SuppressedByStatus => {
                        ActionBlockedReasonV1::SuppressedByStatus
                    }
                    MovementBlockedReason::OutOfBounds => ActionBlockedReasonV1::OutOfBounds,
                    MovementBlockedReason::BlockedTerrain => ActionBlockedReasonV1::BlockedTerrain,
                    MovementBlockedReason::InsufficientMovementPoints => {
                        ActionBlockedReasonV1::InsufficientMovementPoints
                    }
                },
                _ => ActionBlockedReasonV1::BlockedTerrain,
            };
            return Err(StepError::blocked(reason, "jumpkick approach cannot reach the target"));
        }
        for step in &plan.steps {
            let local_walk = match &step.outcome {
                MovementStepOutcome::Moved { navigation, .. } => {
                    *navigation == NavigationKind::Walk
                }
                MovementStepOutcome::Transitioned { kind, target, .. } => {
                    *kind == NavigationKind::Door && *target == step.attempted
                }
                MovementStepOutcome::Blocked { .. } => false,
            };
            let walkable_ground = self.effective_tile_at(&step.attempted).is_some_and(|tile| {
                tile.passable && tile.traversal == Some(TerrainTraversal::Walk)
            });
            if !local_walk || !walkable_ground {
                return Err(StepError::blocked(
                    ActionBlockedReasonV1::BlockedTerrain,
                    "jumpkick approach requires a local walkable route",
                ));
            }
        }
        Ok(plan)
    }

    pub(in crate::engine) fn commit_jumpkick_approach(
        &mut self,
        actor_index: usize,
        defender_index: usize,
        expected: &MovementPlan,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let defender = self.world.actors.get(defender_index).ok_or_else(|| {
            StepError::new("jumpkick target disappeared before approach commit")
        })?;
        if !defender.is_alive() || defender.location != expected.final_position {
            return Err(StepError::new("jumpkick target changed before approach commit"));
        }
        let current = self.jumpkick_approach_plan(actor_index, &defender.location)?;
        if current != *expected {
            return Err(StepError::new("jumpkick approach changed before commit"));
        }
        self.commit_actor_path_steps(actor_index, expected, events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ActorLifeState;

    fn planned_approach() -> (Engine, MovementPlan) {
        let mut engine = crate::engine::setup::test_engine("character_sheet");
        let target = engine.world.actors[1].location.clone();
        for direction in Direction::all() {
            engine.world.actors[0].location = WorldPosition::new(
                &target.realm,
                &target.level,
                target.position.step(direction),
            );
            if let Ok(plan) = engine.jumpkick_approach_plan(0, &target) {
                return (engine, plan);
            }
        }
        panic!("fixture needs a valid one-tile approach");
    }

    #[test]
    fn commit_refuses_a_moved_dead_or_removed_defender_before_any_step() {
        for change in 0..3 {
            let (mut engine, plan) = planned_approach();
            match change {
                0 => engine.world.actors[1].location = engine.world.actors[0].location.clone(),
                1 => engine.world.actors[1].life_state = ActorLifeState::Dead,
                _ => { engine.world.actors.remove(1); }
            }
            let before = engine.export_checkpoint().unwrap();
            let mut events = Vec::new();
            engine.commit_jumpkick_approach(0, 1, &plan, &mut events).unwrap_err();
            assert!(events.is_empty());
            assert_eq!(engine.export_checkpoint().unwrap(), before);
        }
    }

    #[test]
    fn commit_rechecks_takeoff_and_captured_route_before_any_step() {
        let (mut engine, plan) = planned_approach();
        engine.world.actors[0].location = plan.final_position.clone();
        let before = engine.export_checkpoint().unwrap();
        let mut events = Vec::new();
        engine.commit_jumpkick_approach(0, 1, &plan, &mut events).unwrap_err();
        assert!(events.is_empty());
        assert_eq!(engine.export_checkpoint().unwrap(), before);

        let (mut engine, mut plan) = planned_approach();
        plan.steps[0].from = plan.final_position.clone();
        let before = engine.export_checkpoint().unwrap();
        let error = engine.commit_jumpkick_approach(0, 1, &plan, &mut events).unwrap_err();
        assert_eq!(error.message(), "jumpkick approach changed before commit");
        assert!(events.is_empty());
        assert_eq!(engine.export_checkpoint().unwrap(), before);
    }
}
