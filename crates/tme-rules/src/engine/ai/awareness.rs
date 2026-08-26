use crate::model::{ActorAwarenessPolicy, RememberedHostile};

use super::super::{Engine, StepError};
use super::targeting::AutomaticTarget;

impl Engine {
    pub(super) fn resolve_automatic_awareness(
        &mut self,
        actor_index: usize,
        policy: ActorAwarenessPolicy,
        unrestricted_target: Option<usize>,
        visible_target: Option<usize>,
    ) -> Result<AutomaticTarget, StepError> {
        match policy {
            ActorAwarenessPolicy::Unrestricted => {
                self.world.actors[actor_index]
                    .ai
                    .as_mut()
                    .expect("automatic actor AI was checked")
                    .awareness
                    .remembered = None;
                Ok(unrestricted_target.map_or(AutomaticTarget::None, AutomaticTarget::Actor))
            }
            ActorAwarenessPolicy::LineOfSightMemory {
                memory_opportunities,
            } => {
                self.resolve_line_of_sight_memory(actor_index, visible_target, memory_opportunities)
            }
        }
    }

    fn resolve_line_of_sight_memory(
        &mut self,
        actor_index: usize,
        visible_target: Option<usize>,
        memory_opportunities: u32,
    ) -> Result<AutomaticTarget, StepError> {
        if let Some(target_index) = visible_target {
            let target = &self.world.actors[target_index];
            let remembered = RememberedHostile {
                actor_id: target.id.clone(),
                last_seen: target.location.clone(),
                remaining_opportunities: memory_opportunities,
            };
            self.world.actors[actor_index]
                .ai
                .as_mut()
                .expect("automatic actor AI was checked")
                .awareness
                .remembered = Some(remembered);
            return Ok(AutomaticTarget::Actor(target_index));
        }

        let remembered = self.world.actors[actor_index]
            .ai
            .as_ref()
            .expect("automatic actor AI was checked")
            .awareness
            .remembered
            .clone();
        let Some(mut remembered) = remembered else {
            return Ok(AutomaticTarget::None);
        };
        let still_hostile = self
            .world
            .actors
            .iter()
            .enumerate()
            .find(|(_, actor)| actor.id == remembered.actor_id && actor.is_alive())
            .map(|(target_index, _)| self.hostility_assessment(actor_index, target_index))
            .transpose()?
            .is_some_and(|assessment| assessment.hostile);
        if !still_hostile || remembered.remaining_opportunities == 0 {
            self.world.actors[actor_index]
                .ai
                .as_mut()
                .expect("automatic actor AI was checked")
                .awareness
                .remembered = None;
            return Ok(AutomaticTarget::None);
        }
        remembered.remaining_opportunities -= 1;
        let search = remembered.last_seen.clone();
        self.world.actors[actor_index]
            .ai
            .as_mut()
            .expect("automatic actor AI was checked")
            .awareness
            .remembered = Some(remembered);
        Ok(AutomaticTarget::Search(search))
    }
}
