use super::super::visibility::PLAYER_OBSERVATION_RADIUS;
use crate::model::{ActorAwarenessPolicy, Coord, LawZone, WorldPosition};
use crate::view::{
    ActorViewV1, AutomaticActorAwarenessPolicyViewV1, AutomaticActorRememberedHostileViewV1,
    AutomaticActorViewV1, ConcealedTransitionViewV1, EcologyMemberSlotViewV1, EcologySiteViewV1,
    GroundItemViewV1, LawZoneViewV1, LevelSnapshotV1, LevelSnapshotV2, LootClaimViewV1,
    NpcGrudgeRelationViewV1, OBSERVED_SNAPSHOT_CONTRACT_VERSION, ObservedActorViewV1,
    PlayerObservedFrameV1, RealmSnapshotV1, RealmSnapshotV2, SNAPSHOT_CONTRACT_VERSION,
    SelfDefenseRelationViewV1, SnapshotScopeV1, SocialRelationLedgerViewV1,
    SpellSocialCatalogViewV1, SpellSocialViewV1, SpellTownLawViewV1, TileObservationV1,
    TileSnapshotV1, TileSnapshotV2, WorldSnapshotV1, WorldSnapshotV2,
};

use super::super::{Engine, StepError};

impl Engine {
    fn law_zone_view(zone: LawZone) -> LawZoneViewV1 {
        match zone {
            LawZone::None => LawZoneViewV1::None,
            LawZone::Town => LawZoneViewV1::Town,
        }
    }

    fn sorted_spell_social_views(&self) -> Vec<SpellSocialCatalogViewV1> {
        let mut views = self
            .definition
            .catalog
            .spells
            .iter()
            .map(|(spell_id, spell)| SpellSocialCatalogViewV1 {
                spell_id: spell_id.clone(),
                social: SpellSocialViewV1 {
                    hostile_act: spell.social.hostile_act,
                    town_law: match spell.social.town_law {
                        crate::content::TownLawClassificationDef::Permitted => {
                            SpellTownLawViewV1::Permitted
                        }
                        crate::content::TownLawClassificationDef::TerrainAlignmentViolation => {
                            SpellTownLawViewV1::TerrainAlignmentViolation
                        }
                    },
                },
            })
            .collect::<Vec<_>>();
        views.sort_by(|left, right| left.spell_id.cmp(&right.spell_id));
        views
    }

    /// Build a deterministic `WorldSnapshotV1` from current engine state.
    ///
    /// Currently rebuilds all level tile vectors from scratch on every call.
    /// For maps up to prototype size (< 100 tiles) this is negligible. A
    /// future optimization could cache per-level tile snapshots and only
    /// rebuild levels whose terrain or navigation state changed.
    pub fn snapshot(&self) -> WorldSnapshotV1 {
        let world = &self.world;
        let realms_source = &self.definition.world_template.realms;

        let mut controlled_actor_ids = world
            .actors
            .iter()
            .filter(|actor| actor.kind == crate::model::ActorKind::Player)
            .map(|actor| actor.id.clone())
            .collect::<Vec<_>>();
        controlled_actor_ids.sort();

        let rules = self.rules_view();

        let mut realm_ids: Vec<&String> = realms_source.keys().collect();
        realm_ids.sort();
        let mut realms = Vec::with_capacity(realm_ids.len());
        for realm_id in realm_ids {
            let realm = &realms_source[realm_id];
            let mut level_ids: Vec<&String> = realm.levels.keys().collect();
            level_ids.sort();
            let mut levels = Vec::with_capacity(level_ids.len());
            for level_id in level_ids {
                let level = &realm.levels[level_id];
                let tile_count = (level.width as usize).saturating_mul(level.height as usize);
                let mut tiles = Vec::with_capacity(tile_count);
                for y in 0..level.height {
                    for x in 0..level.width {
                        let position = Coord { x, y };
                        let location = WorldPosition::new(realm_id, level_id, position);
                        let (terrain_id, terrain_name, passable, move_cost) = self
                            .effective_tile_at(&location)
                            .map(|tile| {
                                (
                                    tile.terrain_id,
                                    tile.terrain_name,
                                    tile.passable,
                                    tile.move_cost,
                                )
                            })
                            .unwrap_or_else(|| (String::new(), String::new(), false, None));
                        tiles.push(TileSnapshotV1 {
                            position,
                            terrain_id,
                            terrain_name,
                            passable,
                            move_cost,
                            transition: self.transition_view_at(&location),
                        });
                    }
                }
                levels.push(LevelSnapshotV1 {
                    id: level_id.clone(),
                    law_zone: Self::law_zone_view(level.law_zone),
                    width: level.width,
                    height: level.height,
                    tiles,
                });
            }
            realms.push(RealmSnapshotV1 {
                id: realm_id.clone(),
                name: realm.name.clone(),
                levels,
            });
        }

        let mut actor_ids: Vec<(crate::model::ActorId, usize)> = world
            .actors
            .iter()
            .enumerate()
            .map(|(i, actor)| (actor.id.clone(), i))
            .collect();
        actor_ids.sort_by(|(id_a, _), (id_b, _)| id_a.cmp(id_b));

        let actors: Vec<ActorViewV1> = actor_ids
            .iter()
            .map(|(_id, index)| self.actor_view(*index, true))
            .collect();

        // Automatic-actor state is intentionally emitted in actor registration
        // order because that order is also the scheduler's stable tie break.
        let automatic_actors = world
            .actors
            .iter()
            .filter_map(|actor| {
                let ai = actor.ai.as_ref()?;
                let awareness = match ai.awareness.policy {
                    ActorAwarenessPolicy::Unrestricted => {
                        AutomaticActorAwarenessPolicyViewV1::Unrestricted
                    }
                    ActorAwarenessPolicy::LineOfSightMemory {
                        memory_opportunities,
                    } => AutomaticActorAwarenessPolicyViewV1::LineOfSightMemory {
                        memory_opportunities,
                    },
                };
                Some(AutomaticActorViewV1 {
                    actor_id: actor.id.clone(),
                    behavior: ai.behavior,
                    cadence_units: ai.cadence_units,
                    aggro_radius: ai.aggro_radius,
                    leash_range: ai.leash_range,
                    physical_attack_modes: ai.physical_attack_modes.clone(),
                    awareness,
                    remembered: ai.awareness.remembered.as_ref().map(|remembered| {
                        AutomaticActorRememberedHostileViewV1 {
                            actor_id: remembered.actor_id.clone(),
                            last_seen: remembered.last_seen.clone(),
                            remaining_opportunities: remembered.remaining_opportunities,
                        }
                    }),
                    returning_home: ai.returning_home,
                })
            })
            .collect();
        let ecology_sites = world
            .ecology_sites
            .values()
            .map(|site| EcologySiteViewV1 {
                site_id: site.id.clone(),
                spawn_group_id: site.spawn_group_id.clone(),
                generation: site.generation,
                full_clear_due_at: site.full_clear_due_at,
                member_slots: site
                    .member_slots
                    .values()
                    .map(|slot| EcologyMemberSlotViewV1 {
                        member_id: slot.member_id.clone(),
                        location: slot.location.clone(),
                        actor_id: slot.actor_id.clone(),
                        vacant: slot.actor_id.is_none(),
                        due_at: slot.due_at,
                    })
                    .collect(),
            })
            .collect();

        let mut ground_items: Vec<GroundItemViewV1> = self
            .ground_items()
            .iter()
            .map(|item| GroundItemViewV1 {
                item: self
                    .item_instance_view(&item.item_instance_id)
                    .expect("validated ground item instance"),
                location: item.location.clone(),
                loot_claim: item.loot_claim.as_ref().map(LootClaimViewV1::from),
            })
            .collect();
        // Sort by (room, y, x, item_instance_id) for determinism.
        ground_items.sort_by(|a, b| {
            a.location
                .level
                .cmp(&b.location.level)
                .then(a.location.position.y.cmp(&b.location.position.y))
                .then(a.location.position.x.cmp(&b.location.position.x))
                .then(a.item.item_instance_id.cmp(&b.item.item_instance_id))
        });

        WorldSnapshotV1 {
            contract_version: SNAPSHOT_CONTRACT_VERSION,
            scope: SnapshotScopeV1::OmniscientLocal,
            logical_time: world.timing.now,
            controlled_actor_ids,
            rules,
            spell_social: self.sorted_spell_social_views(),
            social_relations: SocialRelationLedgerViewV1 {
                self_defense: world
                    .social_relations
                    .self_defense
                    .values()
                    .map(|relation| SelfDefenseRelationViewV1 {
                        victim_character_id: relation.victim_character_id.clone(),
                        attacker_character_id: relation.attacker_character_id.clone(),
                    })
                    .collect(),
                npc_grudges: world
                    .social_relations
                    .npc_grudges
                    .iter()
                    .map(|relation| NpcGrudgeRelationViewV1 {
                        npc_actor_id: relation.npc_actor_id.clone(),
                        attacker_actor_id: relation.attacker_actor_id.clone(),
                    })
                    .collect(),
            },
            realms,
            actors,
            automatic_actors,
            ecology_sites,
            ground_items,
            corpses: self.sorted_corpse_views(),
            ground_gold: self.sorted_ground_gold_views(),
            tile_effects: self.sorted_tile_effect_views(),
            concealed_transitions: world
                .concealed_transitions
                .iter()
                .map(ConcealedTransitionViewV1::from)
                .collect(),
            quest_states: self.sorted_character_quest_state_views(),
        }
    }

    /// Build a deterministic, player-observed `WorldSnapshotV2` from current
    /// engine state. Tiles, actors, and ground items outside the player's
    /// line of sight are masked (unknown observation, no data) or excluded.
    pub fn actor_observed_snapshot(
        &self,
        actor_id: &crate::model::ActorId,
    ) -> Result<WorldSnapshotV2, StepError> {
        let visible = self.visible_tiles_for_actor_id(actor_id)?;
        self.build_observed_snapshot(actor_id, &visible)
    }

    /// Build an observed snapshot reusing precomputed visible tiles.
    fn build_observed_snapshot(
        &self,
        actor_id: &crate::model::ActorId,
        visible: &std::collections::BTreeSet<WorldPosition>,
    ) -> Result<WorldSnapshotV2, StepError> {
        let world = &self.world;
        let realms_source = &self.definition.world_template.realms;

        let player_index = self.player_actor_index(actor_id)?;
        let player = &world.actors[player_index];
        let observer_actor_id = player.id.clone();
        let observation_center = player.location.clone();
        let rules = self.rules_view();

        let mut realm_ids: Vec<&String> = realms_source.keys().collect();
        realm_ids.sort();
        let mut realms = Vec::with_capacity(realm_ids.len());
        for realm_id in realm_ids {
            let realm = &realms_source[realm_id];
            let mut level_ids: Vec<&String> = realm.levels.keys().collect();
            level_ids.sort();
            let mut levels = Vec::with_capacity(level_ids.len());
            for level_id in level_ids {
                let level = &realm.levels[level_id];
                let mut tiles = Vec::with_capacity(
                    (level.width as usize).saturating_mul(level.height as usize),
                );
                for y in 0..level.height {
                    for x in 0..level.width {
                        let position = Coord { x, y };
                        let location = WorldPosition::new(realm_id, level_id, position);
                        let observed = visible.contains(&location);
                        let (terrain_id, terrain_name, passable, move_cost, transition) =
                            if observed {
                                let transition = self.transition_view_at(&location);
                                let effective = self.effective_tile_at(&location);
                                (
                                    effective.as_ref().map(|tile| tile.terrain_id.clone()),
                                    effective.as_ref().map(|tile| tile.terrain_name.clone()),
                                    effective.as_ref().map(|tile| tile.passable),
                                    effective.and_then(|tile| tile.move_cost),
                                    transition,
                                )
                            } else {
                                (None, None, None, None, None)
                            };
                        tiles.push(TileSnapshotV2 {
                            position,
                            terrain_id,
                            terrain_name,
                            passable,
                            move_cost,
                            transition,
                            observation: if observed {
                                TileObservationV1::Visible
                            } else {
                                TileObservationV1::Unknown
                            },
                        });
                    }
                }
                levels.push(LevelSnapshotV2 {
                    id: level_id.clone(),
                    law_zone: Self::law_zone_view(level.law_zone),
                    width: level.width,
                    height: level.height,
                    tiles,
                });
            }
            realms.push(RealmSnapshotV2 {
                id: realm_id.clone(),
                name: realm.name.clone(),
                levels,
            });
        }

        // Actors: only visible ones
        let mut actor_ids: Vec<(crate::model::ActorId, usize)> = world
            .actors
            .iter()
            .enumerate()
            .filter(|(_, a)| visible.contains(&a.location.clone()))
            .map(|(i, a)| (a.id.clone(), i))
            .collect();
        actor_ids.sort_by(|(id_a, _), (id_b, _)| id_a.cmp(id_b));
        let actors: Vec<ObservedActorViewV1> = actor_ids
            .iter()
            .map(|(_, index)| self.observed_actor_view(player_index, *index))
            .collect::<Result<_, _>>()?;

        // Ground items: only visible ones
        let mut ground_items: Vec<GroundItemViewV1> = self
            .ground_items()
            .iter()
            .filter(|gi| visible.contains(&gi.location.clone()))
            .map(|gi| GroundItemViewV1 {
                item: self
                    .item_instance_view(&gi.item_instance_id)
                    .expect("validated ground item instance"),
                location: gi.location.clone(),
                loot_claim: gi.loot_claim.as_ref().map(LootClaimViewV1::from),
            })
            .collect();
        ground_items.sort_by(|a, b| {
            a.location
                .level
                .cmp(&b.location.level)
                .then(a.location.position.y.cmp(&b.location.position.y))
                .then(a.location.position.x.cmp(&b.location.position.x))
                .then(a.item.item_instance_id.cmp(&b.item.item_instance_id))
        });
        let corpses = self
            .sorted_corpse_views()
            .into_iter()
            .filter(|corpse| visible.contains(&corpse.location.clone()))
            .collect();
        let ground_gold = self
            .sorted_ground_gold_views()
            .into_iter()
            .filter(|pile| visible.contains(&pile.location.clone()))
            .collect();

        Ok(WorldSnapshotV2 {
            contract_version: OBSERVED_SNAPSHOT_CONTRACT_VERSION,
            logical_time: world.timing.now,
            observer_actor_id,
            observation_center,
            observation_radius: PLAYER_OBSERVATION_RADIUS,
            rules,
            realms,
            actors,
            ground_items,
            corpses,
            ground_gold,
            tile_effects: self.visible_tile_effect_views(visible),
        })
    }

    /// Build a combined observed frame from one visibility pass.
    /// Avoids duplicate line-of-sight computation when both the observed
    /// snapshot and action context are needed.
    pub fn actor_observed_frame(
        &self,
        actor_id: &crate::model::ActorId,
    ) -> Result<PlayerObservedFrameV1, StepError> {
        let visible = self.visible_tiles_for_actor_id(actor_id)?;
        Ok(PlayerObservedFrameV1 {
            contract_version: OBSERVED_SNAPSHOT_CONTRACT_VERSION,
            observed_snapshot: self.build_observed_snapshot(actor_id, &visible)?,
            action_context: self.build_observed_action_context(actor_id, &visible)?,
        })
    }
}
