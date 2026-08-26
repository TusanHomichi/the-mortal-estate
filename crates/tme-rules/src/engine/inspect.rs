use super::{Engine, StepError};
use crate::events::{Event, InspectActor, InspectExit, InspectExitStatus, InspectGroundItem};
use crate::model::{Direction, NavigationKind, WorldPosition};

impl Engine {
    pub(super) fn inspect_actor(
        &self,
        actor_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let actor = &self.world.actors[actor_index];
        let location = actor.location.clone();
        let tile = self.tile_label(&location)?;

        let exits = Direction::all()
            .into_iter()
            .map(|direction| {
                let target = WorldPosition::new(
                    &location.realm,
                    &location.level,
                    location.position.step(direction),
                );
                let tile = self.effective_tile_at(&target);
                let terrain = tile.as_ref().map(|tile| tile.terrain_name.clone());
                let move_cost = tile.as_ref().and_then(|tile| tile.move_cost);
                let door_status = self.automatic_navigation_at(&target).and_then(|edge| {
                    (edge.kind == NavigationKind::Door).then(|| InspectExitStatus::Door {
                        state: if self.effective_door_state_at(&target).unwrap_or(false) {
                            "open".to_string()
                        } else {
                            "closed".to_string()
                        },
                        target: edge.target,
                    })
                });
                let status = if let Some(door_status) = door_status {
                    door_status
                } else if !self.in_bounds(&target) {
                    InspectExitStatus::OutOfBounds
                } else if !self.is_walkable(&target) {
                    InspectExitStatus::BlockedTerrain
                } else {
                    InspectExitStatus::Walkable
                };
                InspectExit {
                    direction,
                    location: target,
                    terrain,
                    move_cost,
                    status,
                }
            })
            .collect();

        let mut nearby_actors = Vec::new();
        for direction in Direction::all() {
            let target = WorldPosition::new(
                &location.realm,
                &location.level,
                location.position.step(direction),
            );
            for target_index in self.live_occupants_at(&target) {
                let target_actor = &self.world.actors[target_index];
                nearby_actors.push(InspectActor {
                    direction,
                    actor_id: target_actor.id.clone(),
                    actor: target_actor.name.clone(),
                    kind: target_actor.kind,
                    location: target_actor.location.clone(),
                    hp: target_actor.hp,
                    character_identity: target_actor.character.as_ref().map(|c| c.identity.clone()),
                });
            }
        }

        let ground_items = self
            .ground_items()
            .iter()
            .filter(|item| item.location.same_site(&location))
            .filter_map(|item| {
                let item_view = self.item_instance_view(&item.item_instance_id).ok()?;
                if item.location.position == location.position {
                    Some(InspectGroundItem {
                        item: item_view,
                        location: item.location.clone(),
                        direction: None,
                    })
                } else if location.position.chebyshev_distance(item.location.position) == 1 {
                    self.direction_between(location.position, item.location.position)
                        .ok()
                        .map(|direction| InspectGroundItem {
                            item: item_view,
                            location: item.location.clone(),
                            direction: Some(direction),
                        })
                } else {
                    None
                }
            })
            .collect();

        events.push(Event::Inspected {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            location: location.clone(),
            tile,
            tile_move_cost: self.terrain_cost(&location),
            exits,
            nearby_actors,
            ground_items,
        });
        Ok(())
    }
}
