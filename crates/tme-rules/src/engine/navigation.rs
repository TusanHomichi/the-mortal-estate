use std::collections::{BTreeSet, HashSet, VecDeque};

use super::{Engine, StepError};
use crate::events::{Event, TransitionConcealmentRemovalReasonV1};
use crate::model::{
    Direction, ExplicitTraversalKind, NavigationDef, NavigationKind, TerrainState,
    TerrainTraversal, WorldPosition, WorldSite,
};
use crate::view::{DoorStateViewV1, TransitionKindViewV1, TransitionViewV1};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExplicitTraversalPlan {
    pub(super) from: WorldPosition,
    pub(super) to: WorldPosition,
    pub(super) kind: NavigationKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExplicitTraversalBlockedReason {
    NoTraversalHere,
    WrongDirection,
}

fn is_automatic(kind: NavigationKind) -> bool {
    matches!(
        kind,
        NavigationKind::Door
            | NavigationKind::Pit
            | NavigationKind::Passage
            | NavigationKind::Portal
    )
}

impl Engine {
    pub(super) fn terrain_at(&self, location: &WorldPosition) -> Option<TerrainState> {
        let level = self.level_at(location)?;
        if !self.in_bounds(location) {
            return None;
        }
        let layers = &level.cells[location.position.y as usize][location.position.x as usize];
        let selected = layers
            .iter()
            .flatten()
            .map(|id| self.definition.catalog.terrains.get(id))
            .collect::<Option<Vec<_>>>()?;
        if selected.is_empty() {
            return None;
        }

        let unresolved = selected.iter().any(|terrain| terrain.unresolved);
        let blocked = selected.iter().any(|terrain| !terrain.passable);
        let traversal = if unresolved || blocked {
            None
        } else if selected
            .iter()
            .any(|terrain| terrain.traversal == Some(TerrainTraversal::Swim))
        {
            Some(TerrainTraversal::Swim)
        } else {
            Some(TerrainTraversal::Walk)
        };
        let move_cost = traversal.map(|_| {
            selected
                .iter()
                .filter_map(|terrain| terrain.move_cost)
                .max()
                .unwrap_or(1)
        });
        Some(TerrainState {
            id: selected
                .iter()
                .map(|terrain| terrain.id.as_str())
                .collect::<Vec<_>>()
                .join("+"),
            name: selected
                .iter()
                .map(|terrain| terrain.name.as_str())
                .collect::<Vec<_>>()
                .join(" + "),
            passable: traversal.is_some(),
            move_cost,
            blocks_sight: unresolved || selected.iter().any(|terrain| terrain.blocks_sight),
            traversal,
            unresolved,
        })
    }

    pub(super) fn is_walkable(&self, location: &WorldPosition) -> bool {
        if self
            .effective_transition_at(location)
            .is_some_and(|transition| transition.kind == NavigationKind::Door)
            && !self.effective_door_state_at(location).unwrap_or(false)
        {
            return false;
        }
        self.effective_tile_at(location)
            .is_some_and(|tile| tile.passable)
    }

    pub fn is_navigation_revealed(&self, location: &WorldPosition) -> bool {
        let Some(edges) = self.definition.world_template.navigation.get(location) else {
            return false;
        };
        edges.iter().any(|edge| {
            !edge.hidden
                || self
                    .world
                    .hidden_transition_revealed
                    .get(location)
                    .copied()
                    .unwrap_or(false)
        })
    }

    pub(super) fn is_navigation_concealed(&self, location: &WorldPosition) -> bool {
        self.world
            .concealed_transitions
            .iter()
            .any(|concealment| concealment.location == *location)
    }

    pub(super) fn remove_navigation_concealment_at(
        &mut self,
        location: &WorldPosition,
        reason: TransitionConcealmentRemovalReasonV1,
        events: &mut Vec<Event>,
    ) -> usize {
        let mut removed = 0;
        let mut kept = Vec::new();
        for concealment in std::mem::take(&mut self.world.concealed_transitions) {
            if concealment.location == *location {
                removed += 1;
                events.push(Event::TransitionConcealmentRemoved {
                    instance_id: concealment.instance_id,
                    source_spell_id: concealment.source_spell_id,
                    source_actor_id: concealment.source_actor_id,
                    location: concealment.location,
                    reason,
                });
            } else {
                kept.push(concealment);
            }
        }
        self.world.concealed_transitions = kept;
        removed
    }

    fn dynamic_portals_at(&self, location: &WorldPosition) -> Vec<NavigationDef> {
        let mut edges = Vec::new();
        for portal in &self.world.portal_transitions {
            if portal.remaining_rounds == Some(0) {
                continue;
            }
            if portal.location == *location {
                edges.push(NavigationDef {
                    kind: NavigationKind::Portal,
                    target: portal.target.clone(),
                    initial_state: None,
                    hidden: false,
                });
            }
            if portal.two_way && portal.target == *location {
                edges.push(NavigationDef {
                    kind: NavigationKind::Portal,
                    target: portal.location.clone(),
                    initial_state: None,
                    hidden: false,
                });
            }
        }
        edges
    }

    pub(super) fn effective_navigation_at(&self, location: &WorldPosition) -> Vec<NavigationDef> {
        if self.is_navigation_concealed(location) {
            return Vec::new();
        }
        let mut edges = self.dynamic_portals_at(location);
        if let Some(authored) = self.definition.world_template.navigation.get(location) {
            edges.extend(
                authored
                    .iter()
                    .filter(|edge| {
                        !edge.hidden
                            || self
                                .world
                                .hidden_transition_revealed
                                .get(location)
                                .copied()
                                .unwrap_or(false)
                    })
                    .cloned(),
            );
        }
        edges
    }

    pub(super) fn effective_transition_at(
        &self,
        location: &WorldPosition,
    ) -> Option<NavigationDef> {
        let edges = self.effective_navigation_at(location);
        edges
            .iter()
            .find(|edge| is_automatic(edge.kind))
            .cloned()
            .or_else(|| edges.into_iter().next())
    }

    pub(super) fn automatic_navigation_at(
        &self,
        location: &WorldPosition,
    ) -> Option<NavigationDef> {
        self.effective_navigation_at(location)
            .into_iter()
            .find(|edge| is_automatic(edge.kind))
    }

    pub(super) fn effective_door_state_at(&self, location: &WorldPosition) -> Option<bool> {
        self.effective_transition_at(location)
            .filter(|edge| edge.kind == NavigationKind::Door)?;
        self.world.door_states.get(location).copied()
    }

    pub fn set_navigation_revealed(
        &mut self,
        location: &WorldPosition,
        revealed: bool,
    ) -> Result<(), StepError> {
        let edges = self
            .definition
            .world_template
            .navigation
            .get(location)
            .ok_or_else(|| StepError::new("no navigation edge at position"))?;
        if edges.iter().any(|edge| edge.hidden) {
            self.world
                .hidden_transition_revealed
                .insert(location.clone(), revealed);
        }
        Ok(())
    }

    pub(super) fn transition_view_at(&self, location: &WorldPosition) -> Option<TransitionViewV1> {
        let transition = self.effective_transition_at(location)?;
        let door_state = (transition.kind == NavigationKind::Door)
            .then(|| self.effective_door_state_at(location))
            .flatten()
            .map(|open| {
                if open {
                    DoorStateViewV1::Open
                } else {
                    DoorStateViewV1::Closed
                }
            });
        Some(TransitionViewV1 {
            kind: TransitionKindViewV1::from(transition.kind),
            target: transition.target,
            door_state,
        })
    }

    pub(super) fn evaluate_explicit_traversal(
        &self,
        actor_index: usize,
        requested: ExplicitTraversalKind,
    ) -> Result<ExplicitTraversalPlan, ExplicitTraversalBlockedReason> {
        let actor = self
            .world
            .actors
            .get(actor_index)
            .ok_or(ExplicitTraversalBlockedReason::NoTraversalHere)?;
        let edges = self.effective_navigation_at(&actor.location);
        let mut saw_explicit = false;
        for edge in edges {
            let matches = match edge.kind {
                NavigationKind::Stairs { .. } | NavigationKind::Climb { .. } => {
                    saw_explicit = true;
                    edge.kind == requested.navigation_kind()
                }
                _ => false,
            };
            if matches && self.in_bounds(&edge.target) && self.is_walkable(&edge.target) {
                return Ok(ExplicitTraversalPlan {
                    from: actor.location.clone(),
                    to: edge.target,
                    kind: edge.kind,
                });
            }
        }
        Err(if saw_explicit {
            ExplicitTraversalBlockedReason::WrongDirection
        } else {
            ExplicitTraversalBlockedReason::NoTraversalHere
        })
    }

    pub(super) fn commit_explicit_traversal(
        &mut self,
        actor_index: usize,
        plan: &ExplicitTraversalPlan,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let actor = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("traversal actor no longer exists"))?;
        if actor.location != plan.from {
            return Err(StepError::new("traversal actor moved before commit"));
        }
        let still_exists = self
            .effective_navigation_at(&plan.from)
            .iter()
            .any(|edge| edge.kind == plan.kind && edge.target == plan.to);
        if !still_exists {
            return Err(StepError::new("navigation edge changed before commit"));
        }
        let actor_id = actor.id.clone();
        let actor_name = actor.name.clone();
        self.world.actors[actor_index].location = plan.to.clone();
        events.push(Event::WorldTransition {
            actor_id,
            actor: actor_name,
            from: plan.from.clone(),
            to: plan.to.clone(),
            navigation: plan.kind,
        });
        Ok(())
    }

    pub(super) fn apply_explicit_traversal(
        &mut self,
        actor_index: usize,
        requested: ExplicitTraversalKind,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let plan = self
            .evaluate_explicit_traversal(actor_index, requested)
            .map_err(|reason| match reason {
                ExplicitTraversalBlockedReason::NoTraversalHere => {
                    StepError::new("no traversal here")
                }
                ExplicitTraversalBlockedReason::WrongDirection => {
                    StepError::new("wrong traversal kind")
                }
            })?;
        self.commit_explicit_traversal(actor_index, &plan, events)
    }

    pub(super) fn open_door_at(
        &mut self,
        actor_index: usize,
        location: &WorldPosition,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let transition = if self.is_navigation_concealed(location) {
            self.definition
                .world_template
                .navigation
                .get(location)
                .and_then(|edges| {
                    edges
                        .iter()
                        .find(|edge| edge.kind == NavigationKind::Door)
                        .cloned()
                })
        } else {
            self.automatic_navigation_at(location)
        }
        .ok_or_else(|| StepError::new("no door at position"))?;
        if transition.kind != NavigationKind::Door {
            return Err(StepError::new("no door at position"));
        }
        self.remove_navigation_concealment_at(
            location,
            TransitionConcealmentRemovalReasonV1::Opened,
            events,
        );
        *self
            .world
            .door_states
            .get_mut(location)
            .ok_or_else(|| StepError::new("no door at position"))? = true;
        events.push(Event::DoorOpened {
            actor_id: self.world.actors[actor_index].id.clone(),
            actor: self.world.actors[actor_index].name.clone(),
            location: location.clone(),
        });
        Ok(())
    }

    pub(super) fn apply_door_open(
        &mut self,
        actor_index: usize,
        direction: Direction,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let actor = &self.world.actors[actor_index];
        let location = WorldPosition::new(
            &actor.location.realm,
            &actor.location.level,
            actor.location.position.step(direction),
        );
        self.open_door_at(actor_index, &location, events)
    }

    pub(super) fn apply_door_close(
        &mut self,
        actor_index: usize,
        direction: Direction,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let actor = &self.world.actors[actor_index];
        let location = WorldPosition::new(
            &actor.location.realm,
            &actor.location.level,
            actor.location.position.step(direction),
        );
        if self
            .automatic_navigation_at(&location)
            .is_none_or(|edge| edge.kind != NavigationKind::Door)
        {
            return Err(StepError::new("no door in that direction"));
        }
        if !self.live_occupants_at(&location).is_empty() {
            return Err(StepError::new("cannot close door: occupied"));
        }
        *self
            .world
            .door_states
            .get_mut(&location)
            .ok_or_else(|| StepError::new("no door in that direction"))? = false;
        events.push(Event::DoorClosed {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            location,
        });
        Ok(())
    }

    pub(super) fn automatic_navigation_edges_from(
        &self,
        site: &WorldSite,
    ) -> Vec<(WorldPosition, NavigationDef)> {
        let mut locations = self
            .definition
            .world_template
            .navigation
            .keys()
            .filter(|location| location.site() == *site)
            .cloned()
            .collect::<BTreeSet<_>>();
        for portal in &self.world.portal_transitions {
            if portal.remaining_rounds != Some(0) {
                if portal.location.site() == *site {
                    locations.insert(portal.location.clone());
                }
                if portal.two_way && portal.target.site() == *site {
                    locations.insert(portal.target.clone());
                }
            }
        }
        locations
            .into_iter()
            .filter_map(|location| {
                let edge = self.automatic_navigation_at(&location)?;
                Some((location, edge))
            })
            .collect()
    }

    pub(super) fn next_site_toward(
        &self,
        start: &WorldSite,
        target: &WorldSite,
    ) -> Option<WorldSite> {
        if start == target {
            return None;
        }
        let mut visited = HashSet::from([start.clone()]);
        let mut queue = VecDeque::from([(start.clone(), None)]);
        while let Some((site, first_step)) = queue.pop_front() {
            for (_, edge) in self.automatic_navigation_edges_from(&site) {
                let next = edge.target.site();
                if !visited.insert(next.clone()) {
                    continue;
                }
                let first = first_step.clone().unwrap_or_else(|| next.clone());
                if next == *target {
                    return Some(first);
                }
                queue.push_back((next, Some(first)));
            }
        }
        None
    }

    pub(super) fn navigation_direction_toward_site(
        &self,
        actor_index: usize,
        target: &WorldSite,
    ) -> Option<Direction> {
        let actor = &self.world.actors[actor_index];
        let next = self.next_site_toward(&actor.location.site(), target)?;
        self.automatic_navigation_edges_from(&actor.location.site())
            .into_iter()
            .filter(|(_, edge)| edge.target.site() == next)
            .filter_map(|(location, _)| {
                self.step_toward(actor_index, location.position)
                    .map(|direction| {
                        (
                            direction,
                            actor
                                .location
                                .position
                                .chebyshev_distance(location.position),
                        )
                    })
            })
            .min_by_key(|(_, distance)| *distance)
            .map(|(direction, _)| direction)
    }
}
