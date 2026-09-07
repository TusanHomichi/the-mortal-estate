use crate::events::{Event, NpcFollowDecisionV1, NpcFollowWaitReasonV1};
use crate::model::{ActionCost, ActorKind, CharacterId};

use super::{Engine, StepError};

#[cfg(test)]
mod tests;

impl Engine {
    pub(super) fn clear_npc_followers_of_character(
        &mut self,
        character_id: &CharacterId,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let follower_ids = self
            .world
            .actors
            .iter()
            .filter(|actor| {
                actor.kind == ActorKind::Npc
                    && actor.npc.as_ref().is_some_and(|npc| {
                        npc.following_character_id.as_ref() == Some(character_id)
                    })
            })
            .map(|actor| actor.id.clone())
            .collect::<Vec<_>>();
        for npc_actor_id in follower_ids {
            self.set_npc_follow_target(&npc_actor_id, None, events)?;
        }
        Ok(())
    }

    pub(super) fn set_npc_follow_target(
        &mut self,
        npc_actor_id: &crate::model::ActorId,
        target: Option<CharacterId>,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let npc_index = self
            .world
            .actors
            .iter()
            .position(|actor| &actor.id == npc_actor_id && actor.kind == ActorKind::Npc)
            .ok_or_else(|| StepError::new("NPC follow target actor disappeared"))?;
        let (npc_name, before) = {
            let actor = &self.world.actors[npc_index];
            let npc = actor
                .npc
                .as_ref()
                .ok_or_else(|| StepError::new("NPC follow state disappeared"))?;
            (actor.name.clone(), npc.following_character_id.clone())
        };
        if before == target {
            return Err(StepError::new("NPC follow target did not change"));
        }
        self.world.actors[npc_index]
            .npc
            .as_mut()
            .ok_or_else(|| StepError::new("NPC follow state disappeared"))?
            .following_character_id = target.clone();
        if target.is_some()
            || self.world.actors[npc_index].ai.is_some()
            || self.world.actors[npc_index]
                .npc
                .as_ref()
                .is_some_and(|npc| !npc.patrol.is_empty())
        {
            self.make_npc_ready_now(npc_index)?;
        } else {
            self.make_npc_dormant(npc_index)?;
        }
        events.push(Event::NpcFollowChanged {
            npc_actor_id: npc_actor_id.clone(),
            npc: npc_name,
            from_character_id: before,
            to_character_id: target,
        });
        Ok(())
    }

    pub(super) fn apply_ready_npc_action(
        &mut self,
        npc_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<Option<ActionCost>, StepError> {
        let actor = self
            .world
            .actors
            .get(npc_index)
            .ok_or_else(|| StepError::new("ready NPC disappeared"))?;
        if actor.kind != ActorKind::Npc || !actor.is_alive() {
            return Err(StepError::new("ready NPC is not a living NPC"));
        }
        let has_ai = actor.ai.is_some();
        let has_patrol = actor.npc.as_ref().is_some_and(|npc| !npc.patrol.is_empty());
        let following_character_id = actor
            .npc
            .as_ref()
            .ok_or_else(|| StepError::new("ready NPC has no NPC state"))?
            .following_character_id
            .clone();

        if let Some(character_id) = following_character_id.as_ref()
            && !self.world.actors.iter().any(|candidate| {
                candidate.is_alive() && candidate.character_id.as_ref() == Some(character_id)
            })
        {
            let npc_actor_id = self.world.actors[npc_index].id.clone();
            self.set_npc_follow_target(&npc_actor_id, None, events)?;
            if has_ai && (!has_patrol || self.has_automatic_combat_priority(npc_index)?) {
                return self
                    .apply_automatic_actor_action(npc_index, events)
                    .map(Some);
            }
            return self.apply_npc_patrol_action(npc_index, events);
        }

        if has_ai
            && ((following_character_id.is_none() && !has_patrol)
                || self.has_automatic_combat_priority(npc_index)?)
        {
            return self
                .apply_automatic_actor_action(npc_index, events)
                .map(Some);
        }
        if following_character_id.is_some() {
            self.apply_npc_follow_action(npc_index, events)
        } else {
            self.apply_npc_patrol_action(npc_index, events)
        }
    }

    fn apply_npc_patrol_action(
        &mut self,
        npc_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<Option<ActionCost>, StepError> {
        let actor = &self.world.actors[npc_index];
        let npc = actor
            .npc
            .as_ref()
            .ok_or_else(|| StepError::new("NPC patrol state missing"))?;
        if npc.patrol.is_empty() || actor.location.site() != actor.home_location.site() {
            self.make_npc_dormant(npc_index)?;
            return Ok(None);
        }
        let cost = ActionCost::from_positive_units(npc.follow_cadence_units)
            .ok_or_else(|| StepError::new("NPC cadence must be positive"))?;
        // Attend to someone approaching the provider. This grants no service range:
        // the same-square rules still decide every transaction.
        if self.world.actors.iter().any(|visitor| {
            visitor.kind == ActorKind::Player
                && visitor.is_alive()
                && visitor.location.site() == actor.location.site()
                && visitor
                    .location
                    .position
                    .x
                    .abs_diff(actor.location.position.x)
                    + visitor
                        .location
                        .position
                        .y
                        .abs_diff(actor.location.position.y)
                    <= 1
        }) {
            return Ok(Some(cost));
        }
        let mut next = npc.patrol_next;
        if next >= npc.patrol.len() {
            return Err(StepError::new("NPC patrol cursor out of bounds"));
        }
        if actor.location.position == npc.patrol[next] {
            next = (next + 1) % npc.patrol.len();
        }
        let target = npc.patrol[next];
        self.world.actors[npc_index]
            .npc
            .as_mut()
            .expect("validated NPC")
            .patrol_next = next;
        if let Some(direction) = self.step_toward(npc_index, target) {
            self.try_actor_move(npc_index, direction, events)?;
        }
        Ok(Some(cost))
    }

    pub(super) fn apply_npc_follow_action(
        &mut self,
        npc_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<Option<ActionCost>, StepError> {
        let (npc_actor_id, npc_name, target_character_id, cadence_units) = {
            let actor = self
                .world
                .actors
                .get(npc_index)
                .ok_or_else(|| StepError::new("ready NPC disappeared"))?;
            if actor.kind != ActorKind::Npc || !actor.is_alive() {
                return Err(StepError::new("ready NPC is not a living NPC"));
            }
            let npc = actor
                .npc
                .as_ref()
                .ok_or_else(|| StepError::new("ready NPC has no NPC state"))?;
            (
                actor.id.clone(),
                actor.name.clone(),
                npc.following_character_id.clone(),
                npc.follow_cadence_units,
            )
        };
        let Some(target_character_id) = target_character_id else {
            self.make_npc_dormant(npc_index)?;
            return Ok(None);
        };
        let Some(target_index) = self.world.actors.iter().position(|actor| {
            actor.is_alive() && actor.character_id.as_ref() == Some(&target_character_id)
        }) else {
            self.set_npc_follow_target(&npc_actor_id, None, events)?;
            return Ok(None);
        };
        let cost = ActionCost::from_positive_units(cadence_units)
            .ok_or_else(|| StepError::new("NPC follow cadence must be positive"))?;

        let npc = &self.world.actors[npc_index];
        let target = &self.world.actors[target_index];
        if npc.location.level == target.location.level
            && npc.location.position == target.location.position
        {
            events.push(Event::NpcFollowDecision {
                npc_actor_id,
                npc: npc_name,
                character_id: target_character_id,
                decision: NpcFollowDecisionV1::Wait {
                    reason: NpcFollowWaitReasonV1::AtTarget,
                },
            });
            return Ok(Some(cost));
        }

        let same_room = npc.location.level == target.location.level;
        let direction = if same_room {
            self.step_toward(npc_index, target.location.position)
        } else {
            self.navigation_direction_toward_site(npc_index, &target.location.site())
        };
        let Some(direction) = direction else {
            events.push(Event::NpcFollowDecision {
                npc_actor_id,
                npc: npc_name,
                character_id: target_character_id,
                decision: NpcFollowDecisionV1::Wait {
                    reason: if same_room {
                        NpcFollowWaitReasonV1::Blocked
                    } else {
                        NpcFollowWaitReasonV1::RouteUnavailable
                    },
                },
            });
            return Ok(Some(cost));
        };

        events.push(Event::NpcFollowDecision {
            npc_actor_id,
            npc: npc_name,
            character_id: target_character_id,
            decision: NpcFollowDecisionV1::Move { direction },
        });
        self.try_actor_move(npc_index, direction, events)?;
        Ok(Some(cost))
    }
}
