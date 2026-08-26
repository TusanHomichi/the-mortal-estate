use crate::events::{AutomaticMovementPurposeV1, AutomaticWaitReasonV1, Event};
use crate::model::ActorAiBehavior;

use super::super::{Engine, StepError};

impl Engine {
    pub(super) fn act_automatic_behavior(
        &mut self,
        actor_index: usize,
        target_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let behavior = self.world.actors[actor_index]
            .ai
            .as_ref()
            .expect("automatic actor AI was checked")
            .behavior;
        match behavior {
            ActorAiBehavior::SimpleChase => {
                self.act_simple_chase(actor_index, target_index, events, true)
            }
            ActorAiBehavior::PackForager => {
                self.act_pack_forager(actor_index, target_index, events)
            }
            ActorAiBehavior::WebAmbush => self.act_web_ambush(actor_index, target_index, events),
            ActorAiBehavior::HoldGround => self.act_hold_ground(actor_index, target_index, events),
        }
    }

    fn act_simple_chase(
        &mut self,
        actor_index: usize,
        target_index: usize,
        events: &mut Vec<Event>,
        allow_abilities: bool,
    ) -> Result<(), StepError> {
        if allow_abilities && self.try_automatic_ability(actor_index, target_index, events)? {
            return Ok(());
        }
        if self.try_automatic_physical_action(actor_index, target_index, events)? {
            return Ok(());
        }
        if let Some(direction) = self.chase_direction_toward(actor_index, target_index) {
            self.commit_automatic_move(
                actor_index,
                direction,
                AutomaticMovementPurposeV1::Chase,
                events,
            )
        } else {
            self.commit_automatic_wait(actor_index, AutomaticWaitReasonV1::Blocked, events);
            Ok(())
        }
    }

    fn act_pack_forager(
        &mut self,
        actor_index: usize,
        target_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let actor = &self.world.actors[actor_index];
        if actor.hp * 2 >= actor.max_hp() {
            return self.act_simple_chase(actor_index, target_index, events, false);
        }
        if let Some(direction) = self.flee_direction_from(actor_index, target_index) {
            self.commit_automatic_move(
                actor_index,
                direction,
                AutomaticMovementPurposeV1::Flee,
                events,
            )
        } else {
            self.commit_automatic_wait(actor_index, AutomaticWaitReasonV1::Blocked, events);
            Ok(())
        }
    }

    fn act_web_ambush(
        &mut self,
        actor_index: usize,
        target_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let actor = &self.world.actors[actor_index];
        let target = &self.world.actors[target_index];
        let can_engage = actor.location.same_site(&actor.home_location)
            && target.location.same_site(&actor.home_location)
            && actor
                .home_location
                .position
                .chebyshev_distance(target.location.position)
                <= 2;
        if can_engage {
            return self.act_simple_chase(actor_index, target_index, events, false);
        }
        if actor.location == actor.home_location {
            self.commit_automatic_wait(actor_index, AutomaticWaitReasonV1::Ambush, events);
            return Ok(());
        }
        self.world.actors[actor_index]
            .ai
            .as_mut()
            .expect("automatic actor AI was checked")
            .returning_home = true;
        self.act_return_home(actor_index, events)
    }

    fn act_hold_ground(
        &mut self,
        actor_index: usize,
        target_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        if self.try_automatic_ability(actor_index, target_index, events)?
            || self.try_automatic_physical_action(actor_index, target_index, events)?
        {
            return Ok(());
        }
        self.commit_automatic_wait(actor_index, AutomaticWaitReasonV1::Hold, events);
        Ok(())
    }
}
