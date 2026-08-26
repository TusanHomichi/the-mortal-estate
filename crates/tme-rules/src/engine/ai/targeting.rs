use crate::model::{ActorAwarenessPolicy, HostilityAssessment, WorldPosition};

use super::super::{Engine, StepError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AutomaticTarget {
    Actor(usize),
    Search(WorldPosition),
    None,
}

impl Engine {
    fn automatic_hostile_key(
        &self,
        actor_index: usize,
        target_index: usize,
        assessment: &HostilityAssessment,
    ) -> (u8, i32, u64) {
        let actor = &self.world.actors[actor_index];
        let target = &self.world.actors[target_index];
        (
            assessment.reason.target_priority(),
            if actor.location.same_site(&target.location) {
                actor
                    .location
                    .position
                    .chebyshev_distance(target.location.position)
            } else {
                i32::MAX
            },
            target.timing.tie_break_order,
        )
    }

    fn nearest_automatic_hostile(
        &self,
        actor_index: usize,
        acquisition: bool,
    ) -> Result<Option<usize>, StepError> {
        let actor = &self.world.actors[actor_index];
        let aggro_radius = actor
            .ai
            .as_ref()
            .expect("automatic actor AI was checked")
            .aggro_radius as i32;
        let player_owned = actor.summoned.as_ref().is_some_and(|summoned| {
            self.world.actors.iter().any(|owner| {
                owner.id == summoned.owner_id && owner.kind == crate::model::ActorKind::Player
            })
        });
        let mut selected: Option<((u8, i32, u64), usize)> = None;
        for (target_index, target) in self.world.actors.iter().enumerate() {
            if target_index == actor_index || !target.is_alive() {
                continue;
            }
            if player_owned && target.kind == crate::model::ActorKind::Player {
                continue;
            }
            let assessment = self.hostility_assessment(actor_index, target_index)?;
            if !assessment.hostile {
                continue;
            }
            if acquisition {
                let same_tile = actor.location == target.location;
                let hidden_outside_shared_tile = self.actor_is_hidden(target_index) && !same_tile;
                if !actor.location.same_site(&target.location)
                    || actor
                        .location
                        .position
                        .chebyshev_distance(target.location.position)
                        > aggro_radius
                    || hidden_outside_shared_tile
                    || !self.actor_can_see(actor_index, &target.location.clone())
                {
                    continue;
                }
            }
            let key = self.automatic_hostile_key(actor_index, target_index, &assessment);
            if selected.as_ref().is_none_or(|(best, _)| key < *best) {
                selected = Some((key, target_index));
            }
        }
        Ok(selected.map(|(_, target_index)| target_index))
    }

    pub(super) fn select_automatic_target(
        &mut self,
        actor_index: usize,
    ) -> Result<AutomaticTarget, StepError> {
        let policy = self.world.actors[actor_index]
            .ai
            .as_ref()
            .expect("automatic actor AI was checked")
            .awareness
            .policy;
        let unrestricted_target = self.nearest_automatic_hostile(actor_index, true)?;
        let visible_target = self.nearest_automatic_hostile(actor_index, true)?;
        self.resolve_automatic_awareness(actor_index, policy, unrestricted_target, visible_target)
    }

    pub(in crate::engine) fn has_automatic_combat_priority(
        &mut self,
        actor_index: usize,
    ) -> Result<bool, StepError> {
        let policy = self.world.actors[actor_index]
            .ai
            .as_ref()
            .ok_or_else(|| StepError::new("automatic actor has no AI state"))?
            .awareness
            .policy;
        match policy {
            ActorAwarenessPolicy::Unrestricted => {
                Ok(self.nearest_automatic_hostile(actor_index, true)?.is_some())
            }
            ActorAwarenessPolicy::LineOfSightMemory { .. } => {
                if self.nearest_automatic_hostile(actor_index, true)?.is_some() {
                    return Ok(true);
                }
                let remembered = self.world.actors[actor_index]
                    .ai
                    .as_ref()
                    .expect("automatic actor AI was checked")
                    .awareness
                    .remembered
                    .clone();
                let Some(remembered) = remembered else {
                    return Ok(false);
                };
                let remembered_index = self
                    .world
                    .actors
                    .iter()
                    .position(|actor| actor.id == remembered.actor_id && actor.is_alive());
                let still_hostile = match remembered_index {
                    Some(target_index) => {
                        self.hostility_assessment(actor_index, target_index)?
                            .hostile
                    }
                    None => false,
                };
                if still_hostile && remembered.remaining_opportunities > 0 {
                    Ok(true)
                } else {
                    self.world.actors[actor_index]
                        .ai
                        .as_mut()
                        .expect("automatic actor AI was checked")
                        .awareness
                        .remembered = None;
                    Ok(false)
                }
            }
        }
    }
}
