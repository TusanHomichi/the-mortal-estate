use crate::events::{
    AutomaticActorDecisionV1, AutomaticMovementPurposeV1, AutomaticWaitReasonV1, Event,
};
use crate::model::{ActionCost, ActorAiBehavior, ActorAwarenessPolicy, Direction, WorldPosition};

use super::{Engine, StepError};

mod abilities;
mod awareness;
mod behaviors;
mod navigation;
mod physical;
mod scavenging;
mod targeting;

use targeting::AutomaticTarget;

impl Engine {
    pub(super) fn apply_automatic_actor_action(
        &mut self,
        actor_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<ActionCost, StepError> {
        if !self
            .world
            .actors
            .get(actor_index)
            .is_some_and(|actor| actor.is_alive())
        {
            return Err(StepError::new("automatic actor is not alive"));
        }
        let cadence_units = self.world.actors[actor_index]
            .ai
            .as_ref()
            .ok_or_else(|| StepError::new("automatic actor has no AI state"))?
            .cadence_units;
        let cost = ActionCost::from_positive_units(cadence_units)
            .ok_or_else(|| StepError::new("automatic actor cadence must be positive"))?;

        if let Some(status) = self.automatic_suppression_status(actor_index) {
            self.emit_automatic_decision(
                actor_index,
                AutomaticActorDecisionV1::Suppressed { status },
                events,
            );
            return Ok(cost);
        }

        if self.automatic_actor_is_beyond_leash(actor_index) {
            self.begin_automatic_return_home(actor_index, events)?;
            return Ok(cost);
        }

        let returning_home = self.world.actors[actor_index]
            .ai
            .as_ref()
            .is_some_and(|ai| ai.returning_home);
        let target = if returning_home {
            let policy = self.world.actors[actor_index]
                .ai
                .as_ref()
                .expect("automatic actor AI was checked")
                .awareness
                .policy;
            if !matches!(policy, ActorAwarenessPolicy::LineOfSightMemory { .. }) {
                self.act_return_home(actor_index, events)?;
                return Ok(cost);
            }
            let target = self.select_automatic_target(actor_index)?;
            if !matches!(target, AutomaticTarget::Actor(_)) {
                self.act_return_home(actor_index, events)?;
                return Ok(cost);
            }
            self.world.actors[actor_index]
                .ai
                .as_mut()
                .expect("automatic actor AI was checked")
                .returning_home = false;
            target
        } else {
            self.select_automatic_target(actor_index)?
        };

        if self.try_automatic_balm(actor_index, events)? {
            return Ok(cost);
        }

        match target {
            AutomaticTarget::Actor(target_index) => {
                self.act_automatic_behavior(actor_index, target_index, events)?;
            }
            AutomaticTarget::Search(position) => {
                self.act_search(actor_index, &position, events)?;
            }
            AutomaticTarget::None => {
                if !self.try_automatic_scavenging(actor_index, events)? {
                    self.act_without_target(actor_index, events)?;
                }
            }
        }
        Ok(cost)
    }

    fn automatic_suppression_status(&self, actor_index: usize) -> Option<String> {
        let effect = self.suppressing_effect_for_actor(actor_index)?;
        Some(
            effect
                .tags
                .first()
                .filter(|tag| !tag.trim().is_empty())
                .or_else(|| (!effect.kind.trim().is_empty()).then_some(&effect.kind))
                .unwrap_or(&effect.effect_id)
                .clone(),
        )
    }

    pub(super) fn emit_automatic_decision(
        &self,
        actor_index: usize,
        decision: AutomaticActorDecisionV1,
        events: &mut Vec<Event>,
    ) {
        let actor = &self.world.actors[actor_index];
        events.push(Event::AutomaticActorDecision {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            decision,
        });
    }

    pub(super) fn commit_automatic_move(
        &mut self,
        actor_index: usize,
        direction: Direction,
        purpose: AutomaticMovementPurposeV1,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        if purpose != AutomaticMovementPurposeV1::ReturnHome
            && self.automatic_move_would_break_leash(actor_index, direction)?
        {
            return self.begin_automatic_return_home(actor_index, events);
        }
        self.emit_automatic_decision(
            actor_index,
            AutomaticActorDecisionV1::Move { direction, purpose },
            events,
        );
        self.try_actor_move(actor_index, direction, events)
    }

    fn automatic_position_is_beyond_leash(
        &self,
        actor_index: usize,
        position: &WorldPosition,
    ) -> bool {
        let actor = &self.world.actors[actor_index];
        !position.same_site(&actor.home_location)
            || actor
                .home_location
                .position
                .chebyshev_distance(position.position)
                > actor
                    .ai
                    .as_ref()
                    .expect("automatic actor AI was checked")
                    .leash_range as i32
    }

    fn automatic_actor_is_beyond_leash(&self, actor_index: usize) -> bool {
        self.automatic_position_is_beyond_leash(
            actor_index,
            &self.world.actors[actor_index].location,
        )
    }

    fn automatic_move_would_break_leash(
        &self,
        actor_index: usize,
        direction: Direction,
    ) -> Result<bool, StepError> {
        let plan = self.evaluate_actor_path(
            actor_index,
            &[direction],
            self.definition.catalog.rules.movement.automatic_step_points,
        )?;
        Ok(self.automatic_position_is_beyond_leash(actor_index, &plan.final_position))
    }

    fn begin_automatic_return_home(
        &mut self,
        actor_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let ai = self.world.actors[actor_index]
            .ai
            .as_mut()
            .expect("automatic actor AI was checked");
        ai.returning_home = true;
        ai.awareness.remembered = None;
        self.act_return_home(actor_index, events)
    }

    pub(super) fn commit_automatic_wait(
        &self,
        actor_index: usize,
        reason: AutomaticWaitReasonV1,
        events: &mut Vec<Event>,
    ) {
        self.emit_automatic_decision(
            actor_index,
            AutomaticActorDecisionV1::Wait { reason },
            events,
        );
    }

    fn act_without_target(
        &mut self,
        actor_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let behavior = self.world.actors[actor_index]
            .ai
            .as_ref()
            .expect("automatic actor AI was checked")
            .behavior;
        match behavior {
            ActorAiBehavior::HoldGround => {
                self.commit_automatic_wait(actor_index, AutomaticWaitReasonV1::Hold, events);
            }
            ActorAiBehavior::WebAmbush => {
                let actor = &self.world.actors[actor_index];
                if actor.location.level == actor.home_location.level
                    && actor.location.position == actor.home_location.position
                {
                    self.commit_automatic_wait(actor_index, AutomaticWaitReasonV1::Ambush, events);
                } else {
                    self.world.actors[actor_index]
                        .ai
                        .as_mut()
                        .expect("automatic actor AI was checked")
                        .returning_home = true;
                    self.act_return_home(actor_index, events)?;
                }
            }
            ActorAiBehavior::SimpleChase | ActorAiBehavior::PackForager => {
                self.commit_automatic_wait(actor_index, AutomaticWaitReasonV1::Watch, events);
            }
        }
        Ok(())
    }
}
