use super::{Engine, StepError};
use crate::model::{ActorKind, Direction, WorldPosition};

impl Engine {
    pub(super) fn direction_between(
        &self,
        from: crate::model::Coord,
        to: crate::model::Coord,
    ) -> Result<Direction, StepError> {
        let dx = (to.x - from.x).signum();
        let dy = (to.y - from.y).signum();
        match (dx, dy) {
            (0, -1) => Ok(Direction::North),
            (1, -1) => Ok(Direction::Northeast),
            (1, 0) => Ok(Direction::East),
            (1, 1) => Ok(Direction::Southeast),
            (0, 1) => Ok(Direction::South),
            (-1, 1) => Ok(Direction::Southwest),
            (-1, 0) => Ok(Direction::West),
            (-1, -1) => Ok(Direction::Northwest),
            _ => Err(StepError::new("positions are not adjacent")),
        }
    }

    pub(super) fn actor_index_by_id(&self, actor_id: &crate::model::ActorId) -> Option<usize> {
        self.world
            .actors
            .iter()
            .position(|actor| &actor.id == actor_id)
    }

    pub(super) fn player_actor_index(
        &self,
        actor_id: &crate::model::ActorId,
    ) -> Result<usize, StepError> {
        let index = self
            .actor_index_by_id(actor_id)
            .ok_or_else(|| StepError::new(format!("unknown actor: {actor_id}")))?;
        let actor = &self.world.actors[index];
        if actor.kind != ActorKind::Player {
            return Err(StepError::new("addressed actor is not player-controlled"));
        }
        Ok(index)
    }

    pub(super) fn controlled_actor_index(
        &self,
        actor_id: &crate::model::ActorId,
    ) -> Result<usize, StepError> {
        let index = self.player_actor_index(actor_id)?;
        let actor = &self.world.actors[index];
        if !actor.is_alive() {
            return Err(StepError::new("cannot apply intent after actor death"));
        }
        Ok(index)
    }

    pub(super) fn live_actor_by_id(&self, id: &crate::model::ActorId) -> Option<usize> {
        self.world
            .actors
            .iter()
            .position(|actor| actor.is_alive() && &actor.id == id)
    }

    pub(super) fn live_occupants_at(&self, location: &WorldPosition) -> Vec<usize> {
        self.world
            .actors
            .iter()
            .enumerate()
            .filter(|(_, actor)| actor.is_alive() && actor.location == *location)
            .map(|(index, _)| index)
            .collect()
    }

    pub(super) fn level_at(&self, location: &WorldPosition) -> Option<&crate::model::LevelState> {
        self.definition
            .world_template
            .realms
            .get(&location.realm)?
            .levels
            .get(&location.level)
    }

    pub(super) fn in_bounds(&self, location: &WorldPosition) -> bool {
        self.level_at(location).is_some_and(|level| {
            location.position.x >= 0
                && location.position.y >= 0
                && location.position.x < level.width
                && location.position.y < level.height
        })
    }

    pub(super) fn terrain_cost(&self, location: &WorldPosition) -> Option<i32> {
        self.effective_tile_at(location)
            .and_then(|tile| tile.move_cost)
    }

    pub(super) fn tile_label(&self, location: &WorldPosition) -> Result<String, StepError> {
        self.effective_tile_at(location)
            .map(|tile| tile.terrain_name)
            .ok_or_else(|| StepError::new("actor position has no world cell"))
    }
}
