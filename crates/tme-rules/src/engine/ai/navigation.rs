use crate::events::{AutomaticMovementPurposeV1, AutomaticWaitReasonV1, Event};
use crate::model::{Coord, Direction, WorldPosition};

use super::super::movement::MovementStepOutcome;
use super::super::{Engine, StepError};

fn direction_candidates(dx: i32, dy: i32) -> Vec<Direction> {
    let mut candidates = Vec::new();
    match (dx, dy) {
        (-1, -1) => candidates.push(Direction::Northwest),
        (1, -1) => candidates.push(Direction::Northeast),
        (-1, 1) => candidates.push(Direction::Southwest),
        (1, 1) => candidates.push(Direction::Southeast),
        _ => {}
    }
    if dx < 0 {
        candidates.push(Direction::West);
    } else if dx > 0 {
        candidates.push(Direction::East);
    }
    if dy < 0 {
        candidates.push(Direction::North);
    } else if dy > 0 {
        candidates.push(Direction::South);
    }
    candidates
}

impl Engine {
    pub(in crate::engine) fn step_toward(
        &self,
        actor_index: usize,
        target: Coord,
    ) -> Option<Direction> {
        let actor = self.world.actors[actor_index].location.position;
        let dx = (target.x - actor.x).signum();
        let dy = (target.y - actor.y).signum();
        direction_candidates(dx, dy)
            .into_iter()
            .find(|direction| self.automatic_direction_is_legal(actor_index, *direction))
    }

    fn automatic_direction_is_legal(&self, actor_index: usize, direction: Direction) -> bool {
        self.evaluate_actor_path(
            actor_index,
            &[direction],
            self.definition.catalog.rules.movement.automatic_step_points,
        )
        .is_ok_and(|plan| {
            plan.steps
                .first()
                .is_some_and(|step| !matches!(step.outcome, MovementStepOutcome::Blocked { .. }))
        })
    }

    pub(super) fn chase_direction_toward(
        &self,
        actor_index: usize,
        target_index: usize,
    ) -> Option<Direction> {
        let target = &self.world.actors[target_index];
        let actor = &self.world.actors[actor_index];
        if actor.location.same_site(&target.location) {
            self.step_toward(actor_index, target.location.position)
        } else {
            self.navigation_direction_toward_site(actor_index, &target.location.site())
        }
    }

    pub(in crate::engine) fn flee_direction_from(
        &self,
        actor_index: usize,
        target_index: usize,
    ) -> Option<Direction> {
        let actor = self.world.actors[actor_index].location.position;
        let target = self.world.actors[target_index].location.position;
        let dx = (actor.x - target.x).signum();
        let dy = (actor.y - target.y).signum();
        if dx == 0 && dy == 0 {
            return Direction::all()
                .into_iter()
                .find(|direction| self.automatic_direction_is_legal(actor_index, *direction));
        }
        direction_candidates(dx, dy)
            .into_iter()
            .find(|direction| self.automatic_direction_is_legal(actor_index, *direction))
    }

    pub(super) fn act_search(
        &mut self,
        actor_index: usize,
        target: &WorldPosition,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let actor = &self.world.actors[actor_index];
        let direction = if actor.location.same_site(target) {
            self.step_toward(actor_index, target.position)
        } else {
            self.navigation_direction_toward_site(actor_index, &target.site())
        };
        if let Some(direction) = direction {
            self.commit_automatic_move(
                actor_index,
                direction,
                AutomaticMovementPurposeV1::Search,
                events,
            )
        } else {
            self.commit_automatic_wait(actor_index, AutomaticWaitReasonV1::Watch, events);
            Ok(())
        }
    }

    pub(super) fn act_return_home(
        &mut self,
        actor_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let home = self.world.actors[actor_index].home_location.clone();
        let direction = if !self.world.actors[actor_index].location.same_site(&home) {
            self.navigation_direction_toward_site(actor_index, &home.site())
        } else if self.world.actors[actor_index].location.position != home.position {
            self.step_toward(actor_index, home.position)
        } else {
            self.world.actors[actor_index]
                .ai
                .as_mut()
                .expect("automatic actor AI was checked")
                .returning_home = false;
            self.commit_automatic_wait(actor_index, AutomaticWaitReasonV1::Home, events);
            return Ok(());
        };
        if let Some(direction) = direction {
            self.commit_automatic_move(
                actor_index,
                direction,
                AutomaticMovementPurposeV1::ReturnHome,
                events,
            )
        } else {
            self.commit_automatic_wait(actor_index, AutomaticWaitReasonV1::ReturnBlocked, events);
            Ok(())
        }
    }
}
