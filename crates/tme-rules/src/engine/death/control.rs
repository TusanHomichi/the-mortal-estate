//! Ordinary death control: derived eligibility and the existing return transaction.

use crate::engine::{Engine, StepError};
use crate::model::{
    ActorLifeState, CharacterAlignment, LogicalTime, ResurrectionMethod, ResurrectionRequest,
};
use crate::view::ActionBlockedReasonV1;

impl Engine {
    pub(in crate::engine) fn resurrection_request_at(
        &self,
        actor_index: usize,
    ) -> Option<LogicalTime> {
        let actor = &self.world.actors[actor_index];
        let ActorLifeState::Ghost { defeated_at, .. } = actor.life_state else {
            return None;
        };
        let policy = self
            .definition
            .world_template
            .resurrection
            .get(&actor.location.realm)?;
        Some(defeated_at.saturating_add_millis(policy.request_delay_ms))
    }

    pub(in crate::engine) fn requested_resurrection_plan(
        &self,
        actor_index: usize,
    ) -> Result<ResurrectionRequest, ActionBlockedReasonV1> {
        let actor = &self.world.actors[actor_index];
        let corpse_id = match &actor.life_state {
            ActorLifeState::Ghost { corpse_id, .. } => corpse_id.clone(),
            _ => return Err(ActionBlockedReasonV1::NoRestorationNeeded),
        };
        let at = self
            .resurrection_request_at(actor_index)
            .ok_or(ActionBlockedReasonV1::NoService)?;
        if self.current_time() < at {
            return Err(ActionBlockedReasonV1::NotReady);
        }
        let policy = &self.definition.world_template.resurrection[&actor.location.realm];
        let destination = match self
            .true_actor_alignment(actor_index)
            .map_err(|_| ActionBlockedReasonV1::UnsupportedRestoration)?
        {
            CharacterAlignment::Lawful => policy.lawful_destination.clone(),
            CharacterAlignment::Neutral => policy.neutral_destination.clone(),
            CharacterAlignment::Evil | CharacterAlignment::Chaotic => {
                return Err(ActionBlockedReasonV1::UnsupportedRestoration);
            }
        };
        let request = ResurrectionRequest {
            actor_id: actor.id.clone(),
            corpse_id: Some(corpse_id),
            method: ResurrectionMethod::Gods,
            destination,
            current_hp: actor
                .max_hp()
                .saturating_sub(policy.hit_points_missing)
                .max(1),
            current_stamina: actor
                .max_stamina()
                .saturating_sub(policy.stamina_missing)
                .max(0),
        };
        self.validate_resurrection_request(&request)
            .map_err(|_| ActionBlockedReasonV1::UnsupportedRestoration)?;
        Ok(request)
    }

    pub(in crate::engine) fn request_resurrection(
        &mut self,
        actor_index: usize,
        events: &mut Vec<crate::events::Event>,
    ) -> Result<(), StepError> {
        let plan = self
            .requested_resurrection_plan(actor_index)
            .map_err(|reason| StepError::new(reason.to_string()))?;
        events.extend(self.apply_resurrection_request(plan)?);
        Ok(())
    }
}
