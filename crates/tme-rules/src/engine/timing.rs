use crate::events::Event;
use crate::model::{
    ActionCost, ActorKind, ActorTimingState, DurableGameplayEffectV1, LogicalTime, PlayerIntent,
};

use super::{Engine, StepError};

impl Engine {
    pub(super) fn make_npc_ready_now(&mut self, actor_index: usize) -> Result<(), StepError> {
        let now = self.world.timing.now;
        let actor = self
            .world
            .actors
            .get_mut(actor_index)
            .ok_or_else(|| StepError::new("NPC timing actor disappeared"))?;
        if actor.kind != ActorKind::Npc {
            return Err(StepError::new("NPC timing activation requires an NPC"));
        }
        actor.timing.ready_at = now;
        Ok(())
    }

    pub(super) fn make_npc_dormant(&mut self, actor_index: usize) -> Result<(), StepError> {
        let actor = self
            .world
            .actors
            .get_mut(actor_index)
            .ok_or_else(|| StepError::new("NPC timing actor disappeared"))?;
        if actor.kind != ActorKind::Npc {
            return Err(StepError::new("NPC timing dormancy requires an NPC"));
        }
        actor.timing.ready_at = LogicalTime::new(u64::MAX);
        Ok(())
    }

    pub fn apply_actor_intent(
        &mut self,
        actor_id: &crate::model::ActorId,
        intent: PlayerIntent,
    ) -> Result<RulesOutcomeV1, StepError> {
        let before = self.clone();
        self.pending_durable_effects.clear();
        let state_changed = !matches!(intent, PlayerIntent::Inspect | PlayerIntent::ShowSack);
        match self.apply_actor_intent_transaction(actor_id, intent) {
            Ok(events) => Ok(RulesOutcomeV1 {
                events,
                state_changed,
                durable_effects: std::mem::take(&mut self.pending_durable_effects),
            }),
            Err(error) => {
                *self = before;
                Err(error)
            }
        }
    }

    pub fn apply_realtime_actor_intent(
        &mut self,
        actor_id: &crate::model::ActorId,
        intent: PlayerIntent,
    ) -> Result<RulesOutcomeV1, StepError> {
        let before = self.clone();
        self.pending_durable_effects.clear();
        let state_changed = !matches!(intent, PlayerIntent::Inspect | PlayerIntent::ShowSack);
        match self.apply_actor_intent_transaction_mode(actor_id, intent, false) {
            Ok(events) => Ok(RulesOutcomeV1 {
                events,
                state_changed,
                durable_effects: std::mem::take(&mut self.pending_durable_effects),
            }),
            Err(error) => {
                *self = before;
                Err(error)
            }
        }
    }

    pub fn advance_realtime_boundary(&mut self) -> Result<RulesOutcomeV1, StepError> {
        let before = self.clone();
        self.pending_durable_effects.clear();
        let mut events = Vec::new();
        match self.advance_realtime_boundary_transaction(&mut events) {
            Ok(()) => Ok(RulesOutcomeV1 {
                events,
                state_changed: true,
                durable_effects: std::mem::take(&mut self.pending_durable_effects),
            }),
            Err(error) => {
                *self = before;
                Err(error)
            }
        }
    }

    fn apply_actor_intent_transaction(
        &mut self,
        actor_id: &crate::model::ActorId,
        intent: PlayerIntent,
    ) -> Result<Vec<Event>, StepError> {
        self.apply_actor_intent_transaction_mode(actor_id, intent, true)
    }

    fn apply_actor_intent_transaction_mode(
        &mut self,
        actor_id: &crate::model::ActorId,
        intent: PlayerIntent,
        drain_after: bool,
    ) -> Result<Vec<Event>, StepError> {
        let actor_index = self.controlled_actor_index(actor_id)?;
        let actor = &self.world.actors[actor_index];
        if !actor.is_alive() {
            return Err(StepError::new("cannot step after actor death"));
        }
        if actor.kind != ActorKind::Player {
            return Err(StepError::new("addressed actor is not player-controlled"));
        }
        if actor.timing.ready_at > self.world.timing.now {
            return Err(StepError::new(format!(
                "actor {actor_id:?} is not ready by logical time {}",
                self.world.timing.now.value()
            )));
        }

        let actor_name = actor.name.clone();
        let logical_time = self.world.timing.now;
        let intent_label = intent.label();
        let cost = Self::player_intent_cost(&intent);
        let mut events = vec![
            Event::ActorReady {
                actor_id: actor_id.clone(),
                actor: actor_name.clone(),
                kind: ActorKind::Player,
                logical_time,
            },
            Event::PlayerIntent {
                actor_id: actor_id.clone(),
                actor: actor_name,
                logical_time,
                intent: intent_label,
            },
        ];

        self.apply_player_intent(actor_index, intent, &mut events)?;
        let actor_index = self
            .world
            .actors
            .iter()
            .position(|actor| &actor.id == actor_id)
            .ok_or_else(|| StepError::new("controlled actor disappeared during intent"))?;
        if cost == ActionCost::STANDARD {
            super::progression::apply_ready_level_advances(self, actor_index, &mut events)?;
        }
        self.schedule_actor(actor_index, cost, &mut events)?;
        if drain_after {
            self.drain_until_controlled_ready(actor_id, &mut events)?;
        }
        self.reconcile_separated_item_offers(&mut events)?;
        Ok(events)
    }

    fn player_intent_cost(intent: &PlayerIntent) -> ActionCost {
        match intent {
            PlayerIntent::Inspect | PlayerIntent::ShowSack => ActionCost::FREE,
            _ => ActionCost::STANDARD,
        }
    }

    pub(super) fn current_time(&self) -> LogicalTime {
        self.world.timing.now
    }

    pub(super) fn logical_time_after(&self, rounds: u32) -> LogicalTime {
        self.current_time().saturating_add_rounds(rounds)
    }

    pub(super) fn allocate_actor_timing(&mut self, ready_at: LogicalTime) -> ActorTimingState {
        let tie_break_order = self.world.timing.next_tie_break_order;
        self.world.timing.next_tie_break_order = tie_break_order.saturating_add(1);
        ActorTimingState {
            ready_at,
            tie_break_order,
        }
    }

    pub(super) fn actor_can_act(&self, actor_index: usize) -> bool {
        self.world
            .actors
            .get(actor_index)
            .is_some_and(|actor| actor.is_alive() && actor.timing.ready_at <= self.world.timing.now)
    }

    pub(super) fn schedule_actor(
        &mut self,
        actor_index: usize,
        cost: ActionCost,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let ready_at = self.world.timing.now.saturating_add_rounds(cost.units());
        let actor = self
            .world
            .actors
            .get_mut(actor_index)
            .ok_or_else(|| StepError::new("scheduled actor no longer exists"))?;
        actor.timing.ready_at = ready_at;
        events.push(Event::ActorReadinessScheduled {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            cost_units: cost.units(),
            ready_at,
        });
        Ok(())
    }

    pub(super) fn schedule_resurrected_actor(
        &mut self,
        actor_id: &crate::model::ActorId,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let actor_index = self
            .world
            .actors
            .iter()
            .position(|actor| &actor.id == actor_id)
            .ok_or_else(|| StepError::new("resurrected actor no longer exists"))?;
        let ready_at = self
            .world
            .timing
            .now
            .saturating_add_rounds(ActionCost::STANDARD.units());
        let timing = self.allocate_actor_timing(ready_at);
        let actor = &mut self.world.actors[actor_index];
        actor.timing = timing;
        events.push(Event::ActorReadinessScheduled {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            cost_units: ActionCost::STANDARD.units(),
            ready_at,
        });
        Ok(())
    }

    fn drain_until_controlled_ready(
        &mut self,
        controlled_actor_id: &crate::model::ActorId,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        loop {
            let Some(controlled_index) = self
                .world
                .actors
                .iter()
                .position(|actor| &actor.id == controlled_actor_id)
            else {
                return Ok(());
            };
            if !self.world.actors[controlled_index].is_alive() {
                return Ok(());
            }

            if self.world.actors[controlled_index].timing.ready_at <= self.world.timing.now {
                return Ok(());
            }
            self.advance_realtime_boundary_transaction(events)?;
        }
    }

    fn advance_realtime_boundary_transaction(
        &mut self,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        self.complete_current_logical_boundary(events)?;
        let from = self.world.timing.now;
        let to = from.saturating_add_rounds(1);
        if to == from {
            return Err(StepError::new("logical time cannot advance"));
        }
        self.world.timing.now = to;
        events.push(Event::LogicalTimeAdvanced { from, to });
        self.expire_group_state(events)?;

        let mut opportunities = 0_usize;
        let maximum_opportunities = self.world.actors.len().saturating_mul(4).saturating_add(64);
        loop {
            let next = self
                .world
                .actors
                .iter()
                .enumerate()
                .filter(|(_, actor)| {
                    actor.is_alive()
                        && actor.kind != ActorKind::Player
                        && actor.timing.ready_at <= self.world.timing.now
                })
                .min_by_key(|(_, actor)| (actor.timing.ready_at, actor.timing.tie_break_order))
                .map(|(index, _)| index);
            let Some(next_index) = next else {
                break;
            };
            opportunities = opportunities.saturating_add(1);
            if opportunities > maximum_opportunities {
                return Err(StepError::new("automatic actor opportunity bound exceeded"));
            }
            let actor = &self.world.actors[next_index];
            events.push(Event::ActorReady {
                actor_id: actor.id.clone(),
                actor: actor.name.clone(),
                kind: actor.kind,
                logical_time: self.world.timing.now,
            });
            let ready_actor_id = self.world.actors[next_index].id.clone();
            let cost = match self.world.actors[next_index].kind {
                ActorKind::Monster => Some(self.apply_automatic_actor_action(next_index, events)?),
                ActorKind::Npc => self.apply_ready_npc_action(next_index, events)?,
                ActorKind::Player => unreachable!("players were filtered"),
            };
            if let Some(cost) = cost
                && let Some(current_index) = self
                    .world
                    .actors
                    .iter()
                    .position(|actor| actor.id == ready_actor_id)
            {
                self.schedule_actor(current_index, cost, events)?;
            }
        }
        self.process_player_follow_opportunities(events)?;
        Ok(())
    }

    fn complete_current_logical_boundary(
        &mut self,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let boundary_at = self.logical_time_after(1);
        self.apply_active_effect_ticks(events)?;
        self.apply_tile_effect_ticks(events)?;
        let mut controlled_actor_ids = self
            .world
            .actors
            .iter()
            .filter(|actor| actor.kind == ActorKind::Player && actor.is_alive())
            .map(|actor| (actor.timing.tie_break_order, actor.id.clone()))
            .collect::<Vec<_>>();
        controlled_actor_ids.sort();
        for (_, actor_id) in controlled_actor_ids {
            let Some(player_index) = self.actor_index_by_id(&actor_id) else {
                continue;
            };
            if !self.world.actors[player_index].is_alive() {
                continue;
            }
            self.apply_balm_tick(player_index, events)?;
            if !self.world.actors[player_index].is_alive() {
                continue;
            }
            self.apply_actor_resource_recovery(player_index, boundary_at, events)?;
            super::progression::apply_ready_level_advances(self, player_index, events)?;
        }
        self.transition_warmed_spells_ready(boundary_at, events);
        self.process_ecology_resets(boundary_at, events)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RulesOutcomeV1 {
    pub events: Vec<Event>,
    pub state_changed: bool,
    pub durable_effects: Vec<DurableGameplayEffectV1>,
}

impl RulesOutcomeV1 {
    pub fn iter(&self) -> std::slice::Iter<'_, Event> {
        self.events.iter()
    }
}

impl AsRef<[Event]> for RulesOutcomeV1 {
    fn as_ref(&self) -> &[Event] {
        &self.events
    }
}

impl std::ops::Index<usize> for RulesOutcomeV1 {
    type Output = Event;

    fn index(&self, index: usize) -> &Self::Output {
        &self.events[index]
    }
}

impl IntoIterator for RulesOutcomeV1 {
    type Item = Event;
    type IntoIter = std::vec::IntoIter<Event>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.into_iter()
    }
}

impl<'a> IntoIterator for &'a RulesOutcomeV1 {
    type Item = &'a Event;
    type IntoIter = std::slice::Iter<'a, Event>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.iter()
    }
}

#[cfg(test)]
mod tests {
    use crate::events::Event;
    use crate::model::{ActorId, ActorKind, BalmEffectState, LogicalTime};

    #[test]
    fn logical_boundary_applies_actor_local_balm_in_stable_player_order() {
        let mut engine = crate::engine::setup::test_engine("balm_cache");
        let mut second = engine.world.actors[0].clone();
        second.id = ActorId::from("second_player");
        second.name = "Second Player".to_string();
        second.kind = ActorKind::Player;
        second.timing.tie_break_order = 99;
        second.character_id = None;
        second.character = None;
        second.carried.items.clear();
        second.hp = 2;
        second.stats.hp = 10;
        engine.world.actors[0].hp = 2;
        engine.world.actors[0].character_id = None;
        engine.world.actors[0].character = None;
        engine.world.actors[0].balm_effect = Some(BalmEffectState {
            heal_per_round: 2,
            restored: 0,
            budget: 4,
            last_tick_at: LogicalTime::ZERO,
        });
        second.balm_effect = Some(BalmEffectState {
            heal_per_round: 3,
            restored: 0,
            budget: 6,
            last_tick_at: LogicalTime::ZERO,
        });
        engine.world.actors.push(second);

        let mut events = Vec::new();
        engine
            .complete_current_logical_boundary(&mut events)
            .expect("multi-player boundary");

        assert_eq!(engine.world.actors[0].hp, 4);
        assert_eq!(
            engine
                .world
                .actor(&ActorId::from("second_player"))
                .unwrap()
                .hp,
            5
        );
        assert_eq!(
            events
                .iter()
                .filter_map(|event| match event {
                    Event::BalmHealed { actor_id, .. } => Some(actor_id.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            ["player", "second_player"]
        );
    }
}
