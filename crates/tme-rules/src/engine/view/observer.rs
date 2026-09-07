use std::collections::BTreeSet;

mod feedback;

use crate::events::{
    Event, SpellCastFailure, SpellFizzleCause, TransactionCostReceiptV1,
    TransactionRewardReceiptV1, TransactionSourceV1,
};
use crate::model::{ActorId, Coord, ItemBindingState, WorldPosition};
use crate::view::{
    LootClaimViewV1, MAX_FEEDBACK_TEXT_BYTES, MAX_FEEDBACK_TEXT_SCALARS,
    MAX_FEEDBACK_TRANSACTION_COSTS, MAX_FEEDBACK_TRANSACTION_REWARDS, MAX_OBSERVED_EVENTS,
    MAX_OBSERVER_ACTION_OPTIONS, MAX_OBSERVER_ACTORS, MAX_OBSERVER_CORPSES,
    MAX_OBSERVER_GOLD_PILES, MAX_OBSERVER_GROUND_ITEMS, OBSERVER_PROJECTION_CONTRACT_VERSION,
    ObservedEventV1, ObserverActorV1, ObserverCorpseChangeV1, ObserverCorpseV1,
    ObserverEffectChangeV1, ObserverFeedbackActorV1, ObserverFeedbackCueV1, ObserverFrameV1,
    ObserverGoldPileV1, ObserverGroundItemV1, ObserverGroupInvitationV2, ObserverGroupMemberV2,
    ObserverGroupV2, ObserverInspectActorV1, ObserverInspectExitStatusV1, ObserverInspectExitV1,
    ObserverInspectGroundItemV1, ObserverItemBindingV1, ObserverItemV1, ObserverLifeStateV1,
    ObserverPhysicalOutcomeV1, ObserverProjectionV1, ObserverResourceReasonV1, ObserverSocialV2,
    ObserverSpellFailureReasonV1, ObserverSpellFizzleReasonV1, ObserverSpellImpactOutcomeV1,
    ObserverSpellLifecycleStateV1, ObserverTileV1, ObserverTransactionCostV1,
    ObserverTransactionRewardV1, ObserverTransactionSourceV1,
    STATIC_SCENE_CONTEXT_CONTRACT_VERSION, StaticPresentationModeV1, StaticSceneBoundsV1,
    StaticSceneContextV1, StaticScenePropV1, StaticSceneRoleV1, StaticSceneSiteV1,
    StaticSceneTileV1, StaticTransitionApertureV1,
};

use super::super::{Engine, PLAYER_OBSERVATION_RADIUS, StepError};

impl Engine {
    fn static_scene_context(
        &self,
        center: &WorldPosition,
    ) -> Result<StaticSceneContextV1, StepError> {
        const FRAME_HALF_WIDTH: i32 = 7;
        const FRAME_HALF_HEIGHT: i32 = 6;

        let level = self
            .level_at(center)
            .ok_or_else(|| StepError::new("static scene context level is missing"))?;
        let (min, max) = if level.scene_role == crate::model::SceneRole::Interior {
            (
                Coord { x: 0, y: 0 },
                Coord {
                    x: level.width - 1,
                    y: level.height - 1,
                },
            )
        } else {
            (
                Coord {
                    x: (center.position.x - FRAME_HALF_WIDTH).max(0),
                    y: (center.position.y - FRAME_HALF_HEIGHT).max(0),
                },
                Coord {
                    x: (center.position.x + FRAME_HALF_WIDTH).min(level.width - 1),
                    y: (center.position.y + FRAME_HALF_HEIGHT).min(level.height - 1),
                },
            )
        };
        let mut tiles = Vec::new();
        let mut walkable_mask = Vec::new();
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                let position = Coord { x, y };
                let terrain_ids = level.cells[y as usize][x as usize]
                    .iter()
                    .flatten()
                    .cloned()
                    .collect::<Vec<_>>();
                let mut walkable = false;
                let mut blocked = false;
                for terrain_id in &terrain_ids {
                    match self.definition.catalog.terrains.get(terrain_id) {
                        Some(terrain) if terrain.unresolved || !terrain.passable => blocked = true,
                        Some(_) => walkable = true,
                        None => blocked = true,
                    }
                }
                walkable &= !blocked;
                if walkable {
                    walkable_mask.push(position);
                }
                tiles.push(StaticSceneTileV1 {
                    position,
                    terrain_ids,
                    walkable,
                });
            }
        }
        let mut transition_apertures = self
            .definition
            .world_template
            .navigation
            .iter()
            .filter(|(at, _)| {
                at.same_site(center)
                    && at.position.x >= min.x
                    && at.position.x <= max.x
                    && at.position.y >= min.y
                    && at.position.y <= max.y
            })
            .flat_map(|(at, rows)| {
                rows.iter()
                    .filter(|row| !row.hidden)
                    .map(|row| StaticTransitionApertureV1 {
                        at: at.position,
                        navigation: row.kind,
                        target: row.target.clone(),
                    })
            })
            .collect::<Vec<_>>();
        transition_apertures.sort_by(|left, right| {
            left.at
                .cmp(&right.at)
                .then_with(|| left.target.cmp(&right.target))
        });
        Ok(StaticSceneContextV1 {
            contract_version: STATIC_SCENE_CONTEXT_CONTRACT_VERSION,
            site: StaticSceneSiteV1 {
                realm: center.realm.clone(),
                level: center.level.clone(),
            },
            bounds: StaticSceneBoundsV1 { min, max },
            content_digest: self.definition.content_identity().definition_sha256.clone(),
            visual_manifest_digest: self
                .definition
                .world_template
                .visual_manifest_digest
                .clone(),
            scene_role: match level.scene_role {
                crate::model::SceneRole::Overworld => StaticSceneRoleV1::Overworld,
                crate::model::SceneRole::CombatSpace => StaticSceneRoleV1::CombatSpace,
                crate::model::SceneRole::Interior => StaticSceneRoleV1::Interior,
            },
            presentation_mode: match level.presentation_mode {
                crate::model::PresentationMode::OverworldTown => {
                    StaticPresentationModeV1::OverworldTown
                }
                crate::model::PresentationMode::CombatSpace => {
                    StaticPresentationModeV1::CombatSpace
                }
            },
            world_zoom: level.world_zoom,
            tiles,
            walkable_mask,
            static_props: level
                .static_props
                .iter()
                .filter(|prop| {
                    prop.anchor.x >= min.x
                        && prop.anchor.x <= max.x
                        && prop.anchor.y >= min.y
                        && prop.anchor.y <= max.y
                })
                .map(|prop| StaticScenePropV1 {
                    id: prop.id.clone(),
                    visual_family: prop.visual_family.clone(),
                    anchor: prop.anchor,
                    layer: prop.layer,
                })
                .collect(),
            transition_apertures,
        })
    }

    fn observer_item(&self, item_instance_id: &str) -> Result<ObserverItemV1, StepError> {
        let instance = self.item_instance(item_instance_id)?;
        let definition = self.item_definition(item_instance_id)?;
        Ok(ObserverItemV1 {
            item_instance_id: item_instance_id.to_string(),
            item_definition_id: instance.definition_id.clone(),
            name: definition.name.clone(),
            quantity: instance.quantity,
            binding: match instance.binding {
                ItemBindingState::Unrestricted | ItemBindingState::BindOnFirstCharacterTouch => {
                    ObserverItemBindingV1::Unbound
                }
                ItemBindingState::Bound { .. } => ObserverItemBindingV1::Bound,
            },
        })
    }

    pub fn observer_projection(
        &self,
        observer_actor_id: &ActorId,
        raw_events: &[Event],
    ) -> Result<ObserverProjectionV1, StepError> {
        let observer_index = self.player_actor_index(observer_actor_id)?;
        let observer = &self.world.actors[observer_index];
        let observer_character_id = observer
            .character_id
            .clone()
            .ok_or_else(|| StepError::new("observer has no stable character ID"))?;
        let visible = self.visible_tiles_for_actor_id(observer_actor_id)?;

        let level = self
            .level_at(&observer.location)
            .ok_or_else(|| StepError::new("observer is outside the validated world"))?;
        let center = observer.location.clone();
        let radius = PLAYER_OBSERVATION_RADIUS as i32;
        let mut tiles = Vec::with_capacity(225);
        for y in (center.position.y - radius)..=(center.position.y + radius) {
            for x in (center.position.x - radius)..=(center.position.x + radius) {
                if x < 0 || y < 0 || x >= level.width || y >= level.height {
                    continue;
                }
                let position = Coord { x, y };
                let location = WorldPosition::new(&center.realm, &center.level, position);
                let observed = visible.contains(&location);
                let effective = observed
                    .then(|| self.effective_tile_at(&location))
                    .flatten();
                tiles.push(ObserverTileV1 {
                    position,
                    terrain_id: effective.as_ref().map(|tile| tile.terrain_id.clone()),
                    terrain_name: effective.as_ref().map(|tile| tile.terrain_name.clone()),
                    passable: effective.as_ref().map(|tile| tile.passable),
                    move_cost: effective.as_ref().and_then(|tile| tile.move_cost),
                    transition: observed
                        .then(|| self.transition_view_at(&location))
                        .flatten(),
                });
            }
        }

        let mut visible_actors = self
            .world
            .actors
            .iter()
            .enumerate()
            .filter(|(_, actor)| visible.contains(&actor.location))
            .collect::<Vec<_>>();
        if visible_actors.len() > MAX_OBSERVER_ACTORS {
            return Err(StepError::new(format!(
                "observer projection contains {} visible actors; maximum is {MAX_OBSERVER_ACTORS}",
                visible_actors.len()
            )));
        }
        visible_actors.sort_by(|(_, left), (_, right)| left.id.cmp(&right.id));
        let actors = visible_actors
            .into_iter()
            .map(|(target_index, actor)| {
                Ok(ObserverActorV1 {
                    actor_id: actor.id.clone(),
                    character_id: (actor.kind == crate::model::ActorKind::Player)
                        .then(|| actor.character_id.clone())
                        .flatten(),
                    name: actor.name.clone(),
                    kind: actor.kind,
                    position: actor.location.clone(),
                    life_state: ObserverLifeStateV1::from(&actor.life_state),
                    hp: actor.hp,
                    max_hp: actor.max_hp(),
                    attack_safety: if !actor.is_alive() {
                        crate::model::AttackSafety::Invalid
                    } else {
                        self.attack_safety_assessment(observer_index, target_index)?
                            .safety
                    },
                })
            })
            .collect::<Result<Vec<_>, StepError>>()?;

        let distance =
            |location: &WorldPosition| center.position.chebyshev_distance(location.position);
        let mut visible_corpses = self
            .world
            .corpses
            .values()
            .filter(|corpse| visible.contains(&corpse.location))
            .collect::<Vec<_>>();
        visible_corpses.sort_by(|left, right| {
            distance(&left.location)
                .cmp(&distance(&right.location))
                .then_with(|| left.location.cmp(&right.location))
                .then_with(|| right.sequence.cmp(&left.sequence))
                .then_with(|| left.id.cmp(&right.id))
        });
        let corpses_truncated = visible_corpses.len() > MAX_OBSERVER_CORPSES;
        let corpses = visible_corpses
            .into_iter()
            .take(MAX_OBSERVER_CORPSES)
            .map(|corpse| ObserverCorpseV1 {
                corpse_id: corpse.id.clone(),
                origin_actor_id: corpse.origin_actor_id.clone(),
                origin_kind: corpse.origin_kind,
                origin_name: corpse.origin_name.clone(),
                location: corpse.location.clone(),
                sequence: corpse.sequence,
                searched: corpse.searched,
                loot_claim: corpse.loot_claim.as_ref().map(LootClaimViewV1::from),
            })
            .collect();

        let mut visible_ground_items = self
            .world
            .ground_items
            .iter()
            .filter(|item| visible.contains(&item.location))
            .collect::<Vec<_>>();
        visible_ground_items.sort_by(|left, right| {
            distance(&left.location)
                .cmp(&distance(&right.location))
                .then_with(|| left.location.cmp(&right.location))
                .then_with(|| left.item_instance_id.cmp(&right.item_instance_id))
        });
        let ground_items_truncated = visible_ground_items.len() > MAX_OBSERVER_GROUND_ITEMS;
        let ground_items = visible_ground_items
            .into_iter()
            .take(MAX_OBSERVER_GROUND_ITEMS)
            .map(|item| {
                Ok(ObserverGroundItemV1 {
                    item: self.observer_item(&item.item_instance_id)?,
                    location: item.location.clone(),
                    loot_claim: item.loot_claim.as_ref().map(LootClaimViewV1::from),
                })
            })
            .collect::<Result<Vec<_>, StepError>>()?;

        let mut visible_gold = self
            .world
            .ground_gold
            .values()
            .filter(|pile| visible.contains(&pile.location))
            .collect::<Vec<_>>();
        visible_gold.sort_by(|left, right| {
            distance(&left.location)
                .cmp(&distance(&right.location))
                .then_with(|| left.location.cmp(&right.location))
                .then_with(|| left.id.cmp(&right.id))
        });
        let gold_piles_truncated = visible_gold.len() > MAX_OBSERVER_GOLD_PILES;
        let gold_piles = visible_gold
            .into_iter()
            .take(MAX_OBSERVER_GOLD_PILES)
            .map(|pile| ObserverGoldPileV1 {
                gold_pile_id: pile.id.clone(),
                amount: pile.amount,
                location: pile.location.clone(),
                loot_claim: pile.loot_claim.as_ref().map(LootClaimViewV1::from),
            })
            .collect();

        let controlled_view = self.actor_view(observer_index, false);
        let character = controlled_view
            .character
            .ok_or_else(|| StepError::new("observer has no controlled character sheet"))?;
        let carried = controlled_view.carried;
        let burden = controlled_view.burden;
        let observed_context = self.actor_observed_action_context(observer_actor_id)?;
        let warmed_spell = observed_context.warmed_spell.clone();
        let spell_actions = observed_context.spell_actions.clone();
        let services_here = observed_context.services_here.clone();
        let npcs_here = observed_context.npcs_here.clone();
        let quest_log = observed_context.quest_log.clone();
        let incoming_item_offers = observed_context.incoming_item_offers.clone();
        let outgoing_item_offers = observed_context.outgoing_item_offers.clone();
        let mut action_options = self.actor_action_options(observer_actor_id)?;
        let action_options_truncated = action_options.len() > MAX_OBSERVER_ACTION_OPTIONS;
        action_options.truncate(MAX_OBSERVER_ACTION_OPTIONS);

        let mut events = Vec::new();
        let mut events_truncated = false;
        for event in raw_events {
            let observed = match event {
                Event::Moved {
                    actor_id,
                    from,
                    to,
                    navigation,
                    ..
                } => {
                    let actor_is_visible = actor_id == observer_actor_id
                        || self
                            .world
                            .actor(actor_id)
                            .is_some_and(|actor| visible.contains(&actor.location));
                    (actor_is_visible && (actor_id == observer_actor_id || visible.contains(to)))
                        .then(|| ObservedEventV1::ActorMoved {
                            actor_id: actor_id.clone(),
                            from: from.clone(),
                            to: to.clone(),
                            navigation: *navigation,
                        })
                }
                Event::Inspected {
                    actor_id,
                    location,
                    tile,
                    tile_move_cost,
                    exits,
                    nearby_actors,
                    ground_items,
                    ..
                } if actor_id == observer_actor_id => {
                    let exits = exits
                        .iter()
                        .map(|exit| {
                            let status = match &exit.status {
                                crate::events::InspectExitStatus::Walkable => {
                                    ObserverInspectExitStatusV1::Walkable
                                }
                                crate::events::InspectExitStatus::BlockedTerrain => {
                                    ObserverInspectExitStatusV1::BlockedTerrain
                                }
                                crate::events::InspectExitStatus::Door { state, target } => {
                                    let open = match state.as_str() {
                                        "open" => true,
                                        "closed" => false,
                                        _ => {
                                            return Err(StepError::new(
                                                "inspect event contains invalid door state",
                                            ));
                                        }
                                    };
                                    ObserverInspectExitStatusV1::Door {
                                        open,
                                        target: target.clone(),
                                    }
                                }
                                crate::events::InspectExitStatus::OutOfBounds => {
                                    ObserverInspectExitStatusV1::OutOfBounds
                                }
                            };
                            Ok(ObserverInspectExitV1 {
                                direction: exit.direction,
                                location: exit.location.clone(),
                                terrain: exit.terrain.clone(),
                                move_cost: exit.move_cost,
                                status,
                            })
                        })
                        .collect::<Result<Vec<_>, StepError>>()?;
                    let nearby_actors = nearby_actors
                        .iter()
                        .filter(|actor| visible.contains(&actor.location))
                        .map(|actor| ObserverInspectActorV1 {
                            direction: actor.direction,
                            actor_id: actor.actor_id.clone(),
                            actor: actor.actor.clone(),
                            kind: actor.kind,
                            location: actor.location.clone(),
                            hp: actor.hp,
                        })
                        .collect();
                    let ground_items = ground_items
                        .iter()
                        .filter(|item| visible.contains(&item.location))
                        .map(|item| {
                            Ok(ObserverInspectGroundItemV1 {
                                item: self.observer_item(&item.item.item_instance_id)?,
                                location: item.location.clone(),
                                direction: item.direction,
                            })
                        })
                        .collect::<Result<Vec<_>, StepError>>()?;
                    Some(ObservedEventV1::Inspected {
                        location: location.clone(),
                        tile: tile.clone(),
                        tile_move_cost: *tile_move_cost,
                        exits,
                        nearby_actors,
                        ground_items,
                    })
                }
                Event::GroupChanged {
                    group_id,
                    member_character_ids,
                    subject_character_id,
                    ..
                } => (member_character_ids.contains(&observer_character_id)
                    || subject_character_id.as_ref() == Some(&observer_character_id)
                    || self.group_id_for_character(&observer_character_id) == Some(*group_id))
                .then_some(ObservedEventV1::GroupChanged {
                    group_id: *group_id,
                }),
                Event::GroupInvitationCreated {
                    invitation_id,
                    issuer_character_id,
                    target_character_id,
                    ..
                }
                | Event::GroupInvitationResolved {
                    invitation_id,
                    issuer_character_id,
                    target_character_id,
                    ..
                } => (issuer_character_id == &observer_character_id
                    || target_character_id == &observer_character_id
                    || self
                        .group_id_for_character(issuer_character_id)
                        .is_some_and(|group_id| {
                            self.group_id_for_character(&observer_character_id) == Some(group_id)
                        }))
                .then_some(ObservedEventV1::GroupInvitationChanged {
                    invitation_id: *invitation_id,
                }),
                Event::GroupPresenceChanged {
                    group_id,
                    character_id,
                    connected,
                    ..
                } => (self.group_id_for_character(&observer_character_id) == Some(*group_id))
                    .then_some(ObservedEventV1::GroupPresenceChanged {
                        group_id: *group_id,
                        character_id: character_id.clone(),
                        connected: *connected,
                    }),
                Event::PlayerFollowChanged {
                    follower_character_id,
                    target_character_id,
                    ..
                } => (follower_character_id == &observer_character_id
                    || target_character_id.as_ref() == Some(&observer_character_id))
                .then(|| ObservedEventV1::PlayerFollowChanged {
                    follower_character_id: follower_character_id.clone(),
                    target_character_id: target_character_id.clone(),
                }),
                Event::CommunicationPreferenceChanged { character_id, .. }
                | Event::CharacterBlockChanged { character_id, .. } => (character_id
                    == &observer_character_id)
                    .then_some(ObservedEventV1::CommunicationPreferencesChanged),
                Event::ItemOfferCreated {
                    item_instance_id,
                    sender_character_id,
                    recipient_character_id,
                    ..
                }
                | Event::ItemOfferCompleted {
                    item_instance_id,
                    sender_character_id,
                    recipient_character_id,
                    ..
                } => (sender_character_id == &observer_character_id
                    || recipient_character_id == &observer_character_id)
                    .then(|| ObservedEventV1::ItemOfferChanged {
                        item_instance_id: item_instance_id.clone(),
                    }),
                Event::DefeatRewardShareAwarded {
                    character_id,
                    amount,
                    ..
                } => (character_id == &observer_character_id).then(|| {
                    ObservedEventV1::DefeatRewardShare {
                        character_id: character_id.clone(),
                        amount: *amount,
                    }
                }),
                _ => None,
            };
            if let Some(observed) = observed {
                if events.len() == MAX_OBSERVED_EVENTS {
                    events_truncated = true;
                } else {
                    events.push(observed);
                }
                continue;
            }
            for cue in self.observer_feedback_cues(
                event,
                observer_actor_id,
                &observer_character_id,
                &visible,
            )? {
                if events.len() == MAX_OBSERVED_EVENTS {
                    events_truncated = true;
                    continue;
                }
                events.push(ObservedEventV1::Feedback { cue });
            }
        }

        let group = self
            .group_id_for_character(&observer_character_id)
            .and_then(|group_id| self.world.groups.get(&group_id))
            .map(|group| ObserverGroupV2 {
                group_id: group.id,
                leader_character_id: group.leader_character_id.clone(),
                members: group
                    .members
                    .iter()
                    .map(|member| {
                        let presence = self
                            .world
                            .character_presence
                            .get(&member.character_id)
                            .expect("validated group member presence");
                        ObserverGroupMemberV2 {
                            character_id: member.character_id.clone(),
                            joined_order: member.joined_order,
                            membership_epoch: member.membership_epoch,
                            connected: presence.connected,
                            absent_since: presence.absent_since,
                        }
                    })
                    .collect(),
            });
        let invitations =
            self.world
                .group_invitations
                .values()
                .map(|invitation| ObserverGroupInvitationV2 {
                    invitation_id: invitation.id,
                    issuer_character_id: invitation.issuer_character_id.clone(),
                    target_character_id: invitation.target_character_id.clone(),
                    group_id: invitation.group_id,
                    expires_at: invitation.expires_at,
                });
        let incoming_invitations = invitations
            .clone()
            .filter(|invitation| invitation.target_character_id == observer_character_id)
            .collect();
        let observer_group_id = self.group_id_for_character(&observer_character_id);
        let outgoing_invitations = invitations
            .filter(|invitation| {
                invitation.issuer_character_id == observer_character_id
                    || invitation
                        .group_id
                        .is_some_and(|group_id| observer_group_id == Some(group_id))
            })
            .collect();
        let preferences = self
            .world
            .communication_preferences
            .get(&observer_character_id)
            .ok_or_else(|| StepError::new("observer communication preferences are missing"))?;
        let social = ObserverSocialV2 {
            character_id: observer_character_id.clone(),
            group,
            incoming_invitations,
            outgoing_invitations,
            following_character_id: self
                .world
                .player_follow_targets
                .get(&observer_character_id)
                .cloned(),
            pages_enabled: preferences.pages_enabled,
            blocked_character_ids: preferences.blocked_character_ids.iter().cloned().collect(),
        };
        Ok(ObserverProjectionV1 {
            contract_version: OBSERVER_PROJECTION_CONTRACT_VERSION,
            static_scene_context: self.static_scene_context(&center)?,
            frame: ObserverFrameV1 {
                contract_version: OBSERVER_PROJECTION_CONTRACT_VERSION,
                logical_time: self.world.timing.now,
                ready_at: observer.timing.ready_at,
                observer_actor_id: observer_actor_id.clone(),
                observation_center: center,
                observation_radius: PLAYER_OBSERVATION_RADIUS,
                can_act: self.actor_can_act(observer_index),
                tiles,
                actors,
                corpses,
                corpses_truncated,
                ground_items,
                ground_items_truncated,
                gold_piles,
                gold_piles_truncated,
                character,
                carried,
                burden,
                warmed_spell,
                spell_actions,
                services_here,
                npcs_here,
                quest_log,
                action_options,
                action_options_truncated,
                social,
                incoming_item_offers,
                outgoing_item_offers,
            },
            events,
            events_truncated,
        })
    }
}
