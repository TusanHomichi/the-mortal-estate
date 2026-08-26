use crate::content::SpellEffectDef;
use crate::events::Event;
use crate::model::{
    Coord, DeathCause, NavigationKind, SpellTarget, TerrainTraversal, TileEffectState,
    WorldPosition,
};

use super::Engine;
use super::death::DefeatContext;
use super::spellcasting::SpellCommandPlan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EffectiveTile {
    pub terrain_id: String,
    pub terrain_name: String,
    pub passable: bool,
    pub move_cost: Option<i32>,
    pub blocks_sight: bool,
    pub traversal: Option<TerrainTraversal>,
    pub tile_effects: Vec<TileEffectState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TileSightBlocker {
    None,
    OutOfBounds,
    Terrain,
    ClosedTransition,
    DarknessOverlay,
    OtherOverlay,
}

impl Engine {
    pub(super) fn tile_sight_blocker(&self, location: &WorldPosition) -> TileSightBlocker {
        if !self.in_bounds(location) {
            return TileSightBlocker::OutOfBounds;
        }
        if self
            .automatic_navigation_at(location)
            .is_some_and(|transition| transition.kind == NavigationKind::Door)
            && self.effective_door_state_at(location) == Some(false)
        {
            return TileSightBlocker::ClosedTransition;
        }
        let Some(terrain) = self.terrain_at(location) else {
            return TileSightBlocker::OutOfBounds;
        };
        let Some(tile) = self.effective_tile_at(location) else {
            return TileSightBlocker::OutOfBounds;
        };
        if !tile.blocks_sight {
            return TileSightBlocker::None;
        }
        let blocking_overlays = tile
            .tile_effects
            .iter()
            .filter(|effect| matches!(effect.sight.as_deref(), Some("blocked" | "obscured")));
        let mut saw_darkness = false;
        let mut saw_other = false;
        for effect in blocking_overlays {
            if effect.tags.iter().any(|tag| tag == "darkness") {
                saw_darkness = true;
            } else {
                saw_other = true;
            }
        }
        if saw_other {
            TileSightBlocker::OtherOverlay
        } else if terrain.blocks_sight {
            TileSightBlocker::Terrain
        } else if saw_darkness {
            TileSightBlocker::DarknessOverlay
        } else {
            TileSightBlocker::OtherOverlay
        }
    }

    pub(super) fn tile_positions_for_spell_target(
        &self,
        plan: &SpellCommandPlan,
        _effect: &SpellEffectDef,
    ) -> Option<Vec<WorldPosition>> {
        match plan.target.as_ref()? {
            SpellTarget::Coordinate { position } => Some(vec![position.clone()]),
            SpellTarget::Area { center } => {
                let radius = self
                    .definition
                    .catalog
                    .spells
                    .get(&plan.spell_id)
                    .and_then(|spell| spell.target.as_ref())
                    .and_then(|target| target.area.as_ref())
                    .and_then(|area| area.radius)
                    .unwrap_or(0);
                let mut positions = Vec::new();
                for y in center.position.y - radius..=center.position.y + radius {
                    for x in center.position.x - radius..=center.position.x + radius {
                        let position = Coord { x, y };
                        if center.position.chebyshev_distance(position) <= radius
                            && self
                                .terrain_at(&WorldPosition::new(
                                    &center.realm,
                                    &center.level,
                                    position,
                                ))
                                .is_some()
                        {
                            positions.push(WorldPosition::new(
                                &center.realm,
                                &center.level,
                                position,
                            ));
                        }
                    }
                }
                positions.sort_by_key(|position| (position.position.y, position.position.x));
                Some(positions)
            }
            _ => None,
        }
    }

    pub(super) fn apply_tile_effect_state(
        &mut self,
        effect_state: TileEffectState,
        events: &mut Vec<Event>,
    ) {
        self.world.tile_effects.push(effect_state.clone());
        events.push(Event::TileEffectApplied {
            location: effect_state.location.clone(),
            instance_id: effect_state.instance_id.clone(),
            effect_id: effect_state.effect_id.clone(),
            source_kind: effect_state.source.kind.clone(),
            source_id: effect_state.source.id.clone(),
            kind: effect_state.kind.clone(),
            tags: effect_state.tags.clone(),
            potency: effect_state.potency,
            remaining_rounds: effect_state.remaining_rounds,
            passability: effect_state.passability.clone(),
            sight: effect_state.sight.clone(),
            hazard: effect_state.hazard.clone(),
            move_cost: effect_state.move_cost,
        });
    }

    pub(super) fn remove_tile_effects_at(
        &mut self,
        location: &WorldPosition,
        remove_passability: bool,
        remove_sight: bool,
        reason: &str,
        events: &mut Vec<Event>,
    ) -> usize {
        let mut removed = 0;
        let mut kept = Vec::new();
        let effects = std::mem::take(&mut self.world.tile_effects);
        for mut effect in effects {
            let remove_movement =
                remove_passability && (effect.passability.is_some() || effect.move_cost.is_some());
            let remove_sight_descriptor = remove_sight && effect.sight.is_some();
            let matches_category = remove_movement || remove_sight_descriptor;
            if effect.location == *location && matches_category {
                removed += 1;
                events.push(Event::TileEffectRemoved {
                    location: effect.location.clone(),
                    instance_id: effect.instance_id.clone(),
                    effect_id: effect.effect_id.clone(),
                    kind: effect.kind.clone(),
                    reason: reason.to_string(),
                });
                if remove_movement {
                    effect.passability = None;
                    effect.move_cost = None;
                }
                if remove_sight_descriptor {
                    effect.sight = None;
                }

                if effect.passability.is_some()
                    || effect.sight.is_some()
                    || effect.hazard.is_some()
                    || effect.move_cost.is_some()
                {
                    kept.push(effect);
                }
            } else {
                kept.push(effect);
            }
        }
        self.world.tile_effects = kept;
        removed
    }

    pub(super) fn apply_tile_effect_ticks(
        &mut self,
        events: &mut Vec<Event>,
    ) -> Result<(), super::StepError> {
        let now = self.current_time();
        let effects = std::mem::take(&mut self.world.tile_effects);
        let mut kept = Vec::new();

        for mut effect in effects {
            let should_tick = now.elapsed_rounds_since(effect.last_ticked_at)
                >= u64::from(effect.tick_interval_rounds);
            if should_tick {
                effect.last_ticked_at = now;
                if let Some(remaining) = effect.remaining_rounds.as_mut() {
                    *remaining = remaining.saturating_sub(1);
                }
                events.push(Event::TileEffectTicked {
                    location: effect.location.clone(),
                    instance_id: effect.instance_id.clone(),
                    effect_id: effect.effect_id.clone(),
                    kind: effect.kind.clone(),
                    tags: effect.tags.clone(),
                    potency: effect.potency,
                    remaining_rounds: effect.remaining_rounds,
                });

                if effect
                    .hazard
                    .as_deref()
                    .is_some_and(|hazard| hazard != "unknown")
                    && effect.potency > 0
                {
                    let actor_indices: Vec<usize> = self
                        .world
                        .actors
                        .iter()
                        .enumerate()
                        .filter(|(_, actor)| actor.is_alive() && actor.location == effect.location)
                        .map(|(index, _)| index)
                        .collect();
                    for actor_index in actor_indices {
                        if !self.world.actors[actor_index].is_alive() {
                            continue;
                        }
                        if let Some(authority) = effect.hostile_authority.as_ref()
                            && !self.delayed_hostile_contact_allowed(authority, actor_index)?
                        {
                            continue;
                        }
                        let (actor_id, actor_name, actor_location) = {
                            let actor = &self.world.actors[actor_index];
                            (actor.id.clone(), actor.name.clone(), actor.location.clone())
                        };
                        let instance_id = effect.instance_id.clone();
                        let effect_id = effect.effect_id.clone();
                        let kind = effect.kind.clone();
                        let tags = effect.tags.clone();
                        let cause = if effect.hazard.as_deref() == Some("fire") {
                            DeathCause::Fire
                        } else {
                            DeathCause::Hazard
                        };
                        let credited_actor_id = effect.source_actor_id.clone();
                        let direct_social_actor_id =
                            effect.hostile_authority.as_ref().and_then(|authority| {
                                self.world
                                    .actors
                                    .iter()
                                    .any(|actor| {
                                        actor.id == authority.credited_actor_id
                                            && actor.character_id.as_ref()
                                                == Some(&authority.credited_character_id)
                                    })
                                    .then(|| authority.credited_actor_id.clone())
                            });
                        self.apply_damage_and_resolve_defeat(
                            actor_index,
                            effect.potency,
                            DefeatContext {
                                cause,
                                credited_actor_id,
                                direct_social_actor_id,
                                spell_damage_credit: None,
                                hostile_authority: effect.hostile_authority.clone(),
                            },
                            events,
                            move |outcome| Event::TileEffectDamaged {
                                actor_id,
                                actor: actor_name,
                                location: actor_location,
                                instance_id,
                                effect_id,
                                kind,
                                tags,
                                damage: outcome.applied,
                                hp: outcome.hp_after,
                            },
                            |_| {},
                        )?;
                    }
                }
            }

            if effect.remaining_rounds == Some(0) {
                events.push(Event::TileEffectExpired {
                    location: effect.location,
                    instance_id: effect.instance_id,
                    effect_id: effect.effect_id,
                    kind: effect.kind,
                });
            } else {
                kept.push(effect);
            }
        }

        self.world.tile_effects = kept;
        Ok(())
    }

    pub(super) fn effective_tile_at(&self, location: &WorldPosition) -> Option<EffectiveTile> {
        let terrain = self.terrain_at(location)?;
        let tile_effects: Vec<TileEffectState> = self
            .world
            .tile_effects
            .iter()
            .filter(|effect| effect.location == *location && effect.remaining_rounds != Some(0))
            .cloned()
            .collect();

        let mut passable = terrain.passable;
        let mut move_cost = terrain.move_cost;
        let mut traversal = terrain.traversal;
        let mut sight_overlay_blocks = None;

        for effect in &tile_effects {
            match effect.passability.as_deref() {
                Some("blocked") => passable = false,
                Some("passable") => {
                    if !terrain.unresolved {
                        passable = true;
                        traversal.get_or_insert(TerrainTraversal::Walk);
                        move_cost = effect.move_cost.or(terrain.move_cost).or(Some(1));
                    }
                }
                Some("hindered") => {
                    if !terrain.unresolved {
                        passable = true;
                        traversal.get_or_insert(TerrainTraversal::Walk);
                        move_cost = Some(effect.move_cost.unwrap_or(2));
                    }
                }
                Some("unknown" | "remove_overlay") | None => {}
                Some(_) => {}
            }

            if let Some(cost) = effect.move_cost {
                move_cost = Some(cost);
            }

            match effect.sight.as_deref() {
                Some("blocked" | "obscured") => sight_overlay_blocks = Some(true),
                Some("clear") => sight_overlay_blocks = Some(false),
                Some("unknown" | "remove_overlay") | None => {}
                Some(_) => {}
            }
        }

        let terrain_name = if tile_effects.is_empty() {
            terrain.name.clone()
        } else {
            format!(
                "{} + {}",
                terrain.name,
                tile_effects
                    .iter()
                    .map(|effect| effect.effect_id.as_str())
                    .collect::<Vec<_>>()
                    .join(" + ")
            )
        };

        Some(EffectiveTile {
            terrain_id: terrain.id.clone(),
            terrain_name,
            passable,
            move_cost,
            blocks_sight: sight_overlay_blocks.unwrap_or(terrain.blocks_sight),
            traversal,
            tile_effects,
        })
    }
}
