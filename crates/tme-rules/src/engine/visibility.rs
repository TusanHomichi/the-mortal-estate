use std::collections::BTreeSet;

use crate::model::{ActorKind, Coord, WorldPosition};

use super::tile_effects::TileSightBlocker;
use super::{Engine, StepError};

pub const PLAYER_OBSERVATION_RADIUS: u32 = 7;

impl Engine {
    pub fn has_line_of_sight(&self, from: &WorldPosition, to: &WorldPosition) -> bool {
        from.same_site(to) && self.line_of_sight_in_level(from, to.position, false)
    }

    pub fn visible_tiles_from(
        &self,
        origin: &WorldPosition,
        radius: Option<u32>,
    ) -> BTreeSet<WorldPosition> {
        let mut visible = BTreeSet::from([origin.clone()]);
        let Some(level) = self.level_at(origin) else {
            return visible;
        };
        let max_dist = radius.map_or(i32::MAX, |value| value as i32);
        for y in 0..level.height {
            for x in 0..level.width {
                let position = Coord { x, y };
                if position == origin.position
                    || origin.position.chebyshev_distance(position) > max_dist
                {
                    continue;
                }
                if self.line_of_sight_in_level(origin, position, false) {
                    visible.insert(WorldPosition::new(&origin.realm, &origin.level, position));
                }
            }
        }
        visible
    }

    pub(in crate::engine) fn visible_tiles_for_actor_id(
        &self,
        actor_id: &crate::model::ActorId,
    ) -> Result<BTreeSet<WorldPosition>, StepError> {
        let player_index = self.player_actor_index(actor_id)?;
        Ok(self.visible_tiles_for_actor(player_index, Some(PLAYER_OBSERVATION_RADIUS)))
    }

    pub(super) fn actor_can_see(&self, observer_index: usize, target: &WorldPosition) -> bool {
        let Some(observer) = self.world.actors.get(observer_index) else {
            return false;
        };
        if !observer.is_alive()
            || self.actor_is_blind(observer_index)
            || !observer.location.same_site(target)
        {
            return false;
        }
        if observer.kind == ActorKind::Player
            && observer
                .location
                .position
                .chebyshev_distance(target.position)
                > PLAYER_OBSERVATION_RADIUS as i32
        {
            return false;
        }
        self.line_of_sight_in_level(
            &observer.location,
            target.position,
            self.actor_has_active_tag(observer_index, "night_vision"),
        )
    }

    fn visible_tiles_for_actor(
        &self,
        observer_index: usize,
        radius: Option<u32>,
    ) -> BTreeSet<WorldPosition> {
        let observer = &self.world.actors[observer_index];
        if !observer.is_alive() || self.actor_is_blind(observer_index) {
            return BTreeSet::from([observer.location.clone()]);
        }
        let ignores_darkness = self.actor_has_active_tag(observer_index, "night_vision");
        let mut visible = BTreeSet::from([observer.location.clone()]);
        let Some(level) = self.level_at(&observer.location) else {
            return visible;
        };
        let max_dist = radius.map_or(i32::MAX, |value| value as i32);
        for y in 0..level.height {
            for x in 0..level.width {
                let position = Coord { x, y };
                if position == observer.location.position
                    || observer.location.position.chebyshev_distance(position) > max_dist
                {
                    continue;
                }
                if self.line_of_sight_in_level(&observer.location, position, ignores_darkness) {
                    visible.insert(WorldPosition::new(
                        &observer.location.realm,
                        &observer.location.level,
                        position,
                    ));
                }
            }
        }
        visible
    }

    fn line_of_sight_in_level(
        &self,
        from: &WorldPosition,
        to: Coord,
        ignores_darkness: bool,
    ) -> bool {
        if from.position == to {
            return true;
        }
        let dx = (to.x - from.position.x).abs();
        let dy = (to.y - from.position.y).abs();
        let sx = if from.position.x < to.x { 1 } else { -1 };
        let sy = if from.position.y < to.y { 1 } else { -1 };
        let mut err = dx - dy;
        let mut x = from.position.x;
        let mut y = from.position.y;
        loop {
            if x == to.x && y == to.y {
                return true;
            }
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
            if x == to.x && y == to.y {
                return true;
            }
            let location = WorldPosition::new(&from.realm, &from.level, Coord { x, y });
            if self.tile_blocks_sight(&location, ignores_darkness) {
                return false;
            }
        }
    }

    fn tile_blocks_sight(&self, location: &WorldPosition, ignores_darkness: bool) -> bool {
        match self.tile_sight_blocker(location) {
            TileSightBlocker::None => false,
            TileSightBlocker::DarknessOverlay => !ignores_darkness,
            TileSightBlocker::OutOfBounds
            | TileSightBlocker::Terrain
            | TileSightBlocker::ClosedTransition
            | TileSightBlocker::OtherOverlay => true,
        }
    }
}
