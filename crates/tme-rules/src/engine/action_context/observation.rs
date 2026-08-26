use super::*;

/// Intermediate exit data shared by V1 and V2 context builders.
struct ExitDraft {
    direction: Direction,
    position: WorldPosition,
    terrain_name: Option<String>,
    move_cost: Option<i32>,
    opens_door: bool,
    blocked: bool,
    blocked_reason_str: Option<String>,
    transition: Option<TransitionViewV1>,
    tile_effects: Vec<TileEffectViewV1>,
}

/// Intermediate attack-target data shared by V1 and V2 context builders.
struct AttackTargetDraft {
    actor_id: crate::model::ActorId,
    actor_name: String,
    actor_kind: ActorKind,
    creature_traits: Vec<CreatureTrait>,
    social: ObservedSocialViewV1,
    position: WorldPosition,
    hp: i32,
    max_hp: i32,
    wound_state: crate::model::WoundState,
    physical_attacks: Vec<PhysicalAttackOptionV1>,
    owner_id: Option<crate::model::ActorId>,
    summoned: Option<SummonedActorViewV1>,
}

impl Engine {
    fn action_law_zone_view(zone: LawZone) -> LawZoneViewV1 {
        match zone {
            LawZone::None => LawZoneViewV1::None,
            LawZone::Town => LawZoneViewV1::Town,
        }
    }

    fn collect_corpse_actions(&self, player_index: usize) -> Vec<CorpseActionV1> {
        let player = &self.world.actors[player_index];
        let mut corpses = self
            .world
            .corpses
            .values()
            .filter(|corpse| {
                corpse.location.level == player.location.level
                    && corpse.location.position == player.location.position
            })
            .collect::<Vec<_>>();
        corpses.sort_by(|left, right| {
            right
                .sequence
                .cmp(&left.sequence)
                .then(left.id.cmp(&right.id))
        });
        corpses
            .into_iter()
            .enumerate()
            .map(|(index, corpse)| CorpseActionV1 {
                corpse_id: corpse.id.clone(),
                pile_index: index + 1,
                origin_actor_id: corpse.origin_actor_id.clone(),
                origin_kind: corpse.origin_kind,
                origin_name: corpse.origin_name.clone(),
                searched: corpse.searched,
                loot_claim: corpse.loot_claim.as_ref().map(LootClaimViewV1::from),
            })
            .collect()
    }

    fn collect_ground_gold_here(&self, player_index: usize) -> Vec<GroundGoldPileViewV1> {
        let player = &self.world.actors[player_index];
        let mut piles = self
            .world
            .ground_gold
            .values()
            .filter(|pile| {
                pile.location.level == player.location.level
                    && pile.location.position == player.location.position
            })
            .map(Self::ground_gold_view)
            .collect::<Vec<_>>();
        piles.sort_by(|left, right| left.gold_pile_id.cmp(&right.gold_pile_id));
        piles
    }

    fn item_offer_views_for_actor(
        &self,
        actor_index: usize,
    ) -> Result<(Vec<ItemOfferViewV1>, Vec<ItemOfferViewV1>), StepError> {
        let actor = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("unknown actor"))?;
        let Some(character_id) = actor.character_id.as_ref() else {
            return Ok((Vec::new(), Vec::new()));
        };
        let mut incoming = Vec::new();
        let mut outgoing = Vec::new();
        for (item_instance_id, offer) in &self.world.item_offers {
            if offer.sender_character_id != *character_id
                && offer.recipient_character_id != *character_id
            {
                continue;
            }
            let item = self.item_instance_view(item_instance_id)?;
            let actions = if offer.recipient_character_id == *character_id {
                let definition = self.item_definition(item_instance_id)?;
                let mut actions = Vec::new();
                for destination in CarriedPosition::all().iter().copied().filter(|position| {
                    definition
                        .valid_placements
                        .contains(&position.placement_kind())
                }) {
                    let command = PlayerCommandV1 {
                        contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                        actor_id: actor.id.clone(),
                        intent: PlayerIntentPayloadV1::AcceptItemOffer {
                            item_instance_id: item_instance_id.clone(),
                            destination,
                        },
                    };
                    let status = self.validate_actor_command(&command)?;
                    actions.push(ActionOptionV1 {
                        id: format!(
                            "accept_offer_{}_to_{}",
                            item_instance_id,
                            destination.label()
                        ),
                        label: format!("Accept {} to {}", item.name, destination.label()),
                        enabled: status.accepted,
                        blocked_reason: status.blocked_reason,
                        command: Some(command),
                    });
                }
                let command = PlayerCommandV1 {
                    contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                    actor_id: actor.id.clone(),
                    intent: PlayerIntentPayloadV1::RefuseItemOffer {
                        item_instance_id: item_instance_id.clone(),
                    },
                };
                let status = self.validate_actor_command(&command)?;
                actions.push(ActionOptionV1 {
                    id: format!("refuse_offer_{item_instance_id}"),
                    label: format!("Refuse {}", item.name),
                    enabled: status.accepted,
                    blocked_reason: status.blocked_reason,
                    command: Some(command),
                });
                actions
            } else {
                let command = PlayerCommandV1 {
                    contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                    actor_id: actor.id.clone(),
                    intent: PlayerIntentPayloadV1::WithdrawItemOffer {
                        item_instance_id: item_instance_id.clone(),
                    },
                };
                let status = self.validate_actor_command(&command)?;
                vec![ActionOptionV1 {
                    id: format!("withdraw_offer_{item_instance_id}"),
                    label: format!("Withdraw {} offer", item.name),
                    enabled: status.accepted,
                    blocked_reason: status.blocked_reason,
                    command: Some(command),
                }]
            };
            let view = ItemOfferViewV1 {
                item,
                sender_character_id: offer.sender_character_id.clone(),
                recipient_character_id: offer.recipient_character_id.clone(),
                source_position: offer.source_position,
                actions,
            };
            if offer.recipient_character_id == *character_id {
                incoming.push(view);
            } else {
                outgoing.push(view);
            }
        }
        Ok((incoming, outgoing))
    }

    fn item_offer_creation_actions(
        &self,
        actor_index: usize,
    ) -> Result<Vec<ActionOptionV1>, StepError> {
        let actor = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("unknown actor"))?;
        if actor.character_id.is_none() {
            return Ok(Vec::new());
        }
        let mut recipients = self
            .world
            .actors
            .iter()
            .enumerate()
            .filter(|(index, recipient)| {
                *index != actor_index
                    && recipient.kind == ActorKind::Player
                    && recipient.is_alive()
                    && recipient.character_id.is_some()
                    && recipient.location.level == actor.location.level
                    && recipient.location.position == actor.location.position
            })
            .collect::<Vec<_>>();
        recipients.sort_by(|(_, left), (_, right)| {
            left.character_id
                .cmp(&right.character_id)
                .then(left.id.cmp(&right.id))
        });
        let mut actions = Vec::new();
        for (position, item_instance_id) in &actor.carried.items {
            if !matches!(
                position,
                CarriedPosition::LeftHand | CarriedPosition::RightHand
            ) {
                continue;
            }
            let item = self.item_instance_view(item_instance_id)?;
            for (_, recipient) in &recipients {
                let recipient_character_id = recipient
                    .character_id
                    .clone()
                    .expect("filtered offer recipient has identity");
                let command = PlayerCommandV1 {
                    contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                    actor_id: actor.id.clone(),
                    intent: PlayerIntentPayloadV1::OfferItem {
                        recipient_character_id: recipient_character_id.clone(),
                        item_instance_id: item_instance_id.clone(),
                    },
                };
                let status = self.validate_actor_command(&command)?;
                actions.push(ActionOptionV1 {
                    id: format!(
                        "offer_{}_to_{}",
                        item_instance_id,
                        recipient_character_id.as_str()
                    ),
                    label: format!("Offer {} to {}", item.name, recipient.name),
                    enabled: status.accepted,
                    blocked_reason: status.blocked_reason,
                    command: Some(command),
                });
            }
        }
        Ok(actions)
    }

    pub fn actor_action_context(
        &self,
        actor_id: &crate::model::ActorId,
    ) -> Result<PlayerActionContextV1, StepError> {
        let player_index = self.player_actor_index(actor_id)?;
        let visible_tiles = self.visible_tiles_for_actor_id(actor_id)?;
        let player = &self.world.actors[player_index];
        let current_location = player.location.clone();

        // --- Exits ---
        let exits: Vec<ActionExitV1> = self
            .build_exit_drafts(player_index)
            .into_iter()
            .map(|d| ActionExitV1 {
                direction: d.direction,
                position: d.position,
                terrain_name: d.terrain_name,
                move_cost: d.move_cost,
                opens_door: d.opens_door,
                blocked: d.blocked,
                blocked_reason: d.blocked_reason_str,
                transition: d.transition,
                tile_effects: d.tile_effects,
            })
            .collect();

        // --- Attack targets ---
        let attack_targets: Vec<ActionTargetV1> = self
            .build_attack_target_drafts(player_index)
            .into_iter()
            .filter(|d| visible_tiles.contains(&d.position))
            .map(|d| ActionTargetV1 {
                actor_id: d.actor_id,
                actor_name: d.actor_name,
                actor_kind: d.actor_kind,
                creature_traits: d.creature_traits,
                social: d.social,
                position: d.position,
                hp: d.hp,
                max_hp: d.max_hp,
                wound_state: d.wound_state,
                physical_attacks: d.physical_attacks,
                owner_id: d.owner_id,
                summoned: d.summoned,
            })
            .collect();

        // --- Ground items at player's position ---
        let ground_items_here = self.collect_ground_items(player_index);
        let corpses_here = self.collect_corpse_actions(player_index);
        let ground_gold_here = self.collect_ground_gold_here(player_index);

        let carried = self.collect_carried_layout(player_index)?;

        // --- Usable (drinkable) items ---
        let usable_items = self.collect_usable_items(player_index);

        // --- Door actions (adjacent doors) ---
        let door_actions = self.collect_door_actions(player_index);
        let traversal_actions = self.collect_traversal_actions(player_index);
        let services_here = self.service_views_for_actor(player_index)?;
        let npcs_here = self.npc_views_for_actor(player_index)?;
        let quest_log = self.quest_log_for_actor(player_index);
        let item_offer_actions = self.item_offer_creation_actions(player_index)?;
        let (incoming_item_offers, outgoing_item_offers) =
            self.item_offer_views_for_actor(player_index)?;

        Ok(PlayerActionContextV1 {
            contract_version: crate::view::ACTION_CONTEXT_CONTRACT_VERSION,
            actor_id: player.id.clone(),
            actor_name: player.name.clone(),
            actor_kind: player.kind,
            position: current_location.clone(),
            law_zone: Self::action_law_zone_view(
                self.level_at(&current_location)
                    .expect("validated player location")
                    .law_zone,
            ),
            logical_time: self.current_time(),
            ready_at: player.timing.ready_at,
            can_act: self.actor_can_act(player_index),
            life_state: ActorLifeStateViewV1::from(&player.life_state),
            controlled_path_points: self
                .definition
                .catalog
                .rules
                .movement
                .controlled_path_points,
            max_path_steps: MAX_CONTROLLED_PATH_STEPS,
            last_resource_activity_at: player.resource_activity.last_active_at,
            attack_ready_at: player.attack_ready_at,
            active_effects: player
                .active_effects
                .iter()
                .map(ActiveEffectViewV1::from)
                .collect(),
            magic_resistance: crate::view::MagicResistanceViewV1 {
                natural_save_twentieths: player.magic_resistance.natural_save_twentieths,
                evidence_state: player.magic_resistance.evidence_state,
                boosts: self
                    .actor_resistance_boosts(&player.id)?
                    .into_iter()
                    .map(|boost| crate::view::MagicResistanceBoostViewV1 {
                        tag: boost.tag,
                        bonus_twentieths: boost.bonus_twentieths,
                        source_kind: boost.source_kind,
                        source_id: boost.source_id,
                    })
                    .collect(),
            },
            warmed_spell: player.warmed_spell.as_ref().map(WarmedSpellViewV1::from),
            spell_actions: self.spell_action_descriptors(player_index)?,
            services_here,
            npcs_here,
            quest_log,
            item_offer_actions,
            incoming_item_offers,
            outgoing_item_offers,
            tile_effects_here: self.tile_effect_views_at(&current_location),
            exits,
            attack_targets,
            ground_items_here,
            corpses_here,
            ground_gold_here,
            carried,
            usable_items,
            door_actions,
            traversal_actions,
            burden: self.burden_view(player_index)?,
        })
    }

    /// Player-observed action context: visibility-filtered, typed block reasons.
    /// Hides same-room actors and items that are not visible to the player.
    pub fn actor_observed_action_context(
        &self,
        actor_id: &crate::model::ActorId,
    ) -> Result<PlayerActionContextV2, StepError> {
        let visible_tiles = self.visible_tiles_for_actor_id(actor_id)?;
        self.build_observed_action_context(actor_id, &visible_tiles)
    }

    /// Build an observed action context reusing precomputed visible tiles.
    pub(in crate::engine) fn build_observed_action_context(
        &self,
        actor_id: &crate::model::ActorId,
        visible_tiles: &std::collections::BTreeSet<WorldPosition>,
    ) -> Result<PlayerActionContextV2, StepError> {
        let player_index = self.player_actor_index(actor_id)?;
        let player = &self.world.actors[player_index];
        let current_location = player.location.clone();

        // --- Exits ---
        let exits: Vec<ActionExitV2> = self
            .build_exit_drafts(player_index)
            .into_iter()
            .map(|d| ActionExitV2 {
                direction: d.direction,
                position: d.position,
                terrain_name: d.terrain_name,
                move_cost: d.move_cost,
                opens_door: d.opens_door,
                blocked: d.blocked,
                blocked_reason: d
                    .blocked_reason_str
                    .as_deref()
                    .map(Self::exit_block_to_typed),
                transition: d.transition,
                tile_effects: d.tile_effects,
            })
            .collect();

        // --- Attack targets (visibility-filtered) ---
        let attack_targets: Vec<ActionTargetV2> = self
            .build_attack_target_drafts(player_index)
            .into_iter()
            .filter(|d| visible_tiles.contains(&d.position))
            .map(|d| ActionTargetV2 {
                actor_id: d.actor_id,
                actor_name: d.actor_name,
                actor_kind: d.actor_kind,
                creature_traits: d.creature_traits,
                social: d.social,
                position: d.position,
                hp: d.hp,
                max_hp: d.max_hp,
                wound_state: d.wound_state,
                physical_attacks: d.physical_attacks,
                owner_id: d.owner_id,
                summoned: d.summoned,
            })
            .collect();

        // --- Ground items at player's position (always visible) ---
        let ground_items_here = self.collect_ground_items(player_index);
        let corpses_here = self.collect_corpse_actions(player_index);
        let ground_gold_here = self.collect_ground_gold_here(player_index);

        let carried = self.collect_carried_layout(player_index)?;

        // --- Usable (drinkable) items ---
        let usable_items = self.collect_usable_items(player_index);

        // --- Door actions (adjacent doors) ---
        let door_actions = self.collect_door_actions(player_index);
        let traversal_actions = self.collect_traversal_actions(player_index);
        let services_here = self.service_views_for_actor(player_index)?;
        let npcs_here = self.npc_views_for_actor(player_index)?;
        let quest_log = self.quest_log_for_actor(player_index);
        let item_offer_actions = self.item_offer_creation_actions(player_index)?;
        let (incoming_item_offers, outgoing_item_offers) =
            self.item_offer_views_for_actor(player_index)?;

        Ok(PlayerActionContextV2 {
            contract_version: crate::view::ACTION_CONTEXT_CONTRACT_VERSION,
            actor_id: player.id.clone(),
            actor_name: player.name.clone(),
            actor_kind: player.kind,
            position: current_location.clone(),
            law_zone: Self::action_law_zone_view(
                self.level_at(&current_location)
                    .expect("validated player location")
                    .law_zone,
            ),
            logical_time: self.current_time(),
            ready_at: player.timing.ready_at,
            can_act: self.actor_can_act(player_index),
            life_state: ActorLifeStateViewV1::from(&player.life_state),
            controlled_path_points: self
                .definition
                .catalog
                .rules
                .movement
                .controlled_path_points,
            max_path_steps: MAX_CONTROLLED_PATH_STEPS,
            last_resource_activity_at: player.resource_activity.last_active_at,
            attack_ready_at: player.attack_ready_at,
            active_effects: player
                .active_effects
                .iter()
                .map(ActiveEffectViewV1::from)
                .collect(),
            magic_resistance: crate::view::MagicResistanceViewV1 {
                natural_save_twentieths: player.magic_resistance.natural_save_twentieths,
                evidence_state: player.magic_resistance.evidence_state,
                boosts: self
                    .actor_resistance_boosts(&player.id)?
                    .into_iter()
                    .map(|boost| crate::view::MagicResistanceBoostViewV1 {
                        tag: boost.tag,
                        bonus_twentieths: boost.bonus_twentieths,
                        source_kind: boost.source_kind,
                        source_id: boost.source_id,
                    })
                    .collect(),
            },
            warmed_spell: player.warmed_spell.as_ref().map(WarmedSpellViewV1::from),
            spell_actions: self.spell_action_descriptors(player_index)?,
            services_here,
            npcs_here,
            quest_log,
            item_offer_actions,
            incoming_item_offers,
            outgoing_item_offers,
            tile_effects_here: self.tile_effect_views_at(&current_location),
            exits,
            attack_targets,
            ground_items_here,
            corpses_here,
            ground_gold_here,
            carried,
            usable_items,
            door_actions,
            traversal_actions,
            burden: self.burden_view(player_index)?,
        })
    }

    fn collect_ground_items(&self, player_index: usize) -> Vec<ItemInstanceViewV1> {
        let player = &self.world.actors[player_index];
        self.ground_items_at(&player.location.clone())
            .into_iter()
            .filter_map(|item| self.item_instance_view(&item.item_instance_id).ok())
            .collect()
    }

    fn collect_carried_layout(
        &self,
        player_index: usize,
    ) -> Result<CarriedLayoutViewV1, StepError> {
        let actor = &self.world.actors[player_index];
        let items = actor
            .carried
            .items
            .iter()
            .map(|(position, item_instance_id)| {
                self.positioned_item_view(item_instance_id, *position)
            })
            .collect::<Result<Vec<_>, StepError>>()?;
        Ok(CarriedLayoutViewV1 {
            items,
            gold: actor.carried.gold,
        })
    }

    pub(super) fn collect_usable_items(&self, player_index: usize) -> Vec<UsableItemActionV1> {
        self.sack_item_ids(player_index)
            .expect("validated player index")
            .iter()
            .filter(|item_instance_id| self.consumable_heal_for_item(item_instance_id).is_some())
            .filter_map(|item_instance_id| {
                Some(UsableItemActionV1 {
                    item: self.item_instance_view(item_instance_id).ok()?,
                    action: "drink".to_string(),
                })
            })
            .collect()
    }

    pub(super) fn collect_door_actions(&self, player_index: usize) -> Vec<DoorActionV1> {
        let player = &self.world.actors[player_index];
        Direction::all()
            .into_iter()
            .filter_map(|direction| {
                let target = WorldPosition::new(
                    &player.location.realm,
                    &player.location.level,
                    player.location.position.step(direction),
                );
                if self.is_navigation_concealed(&target) {
                    return None;
                }
                let transition = self.effective_transition_at(&target)?;
                if transition.kind != NavigationKind::Door {
                    return None;
                }
                let is_open = self.effective_door_state_at(&target).unwrap_or(false);
                let door_state = if is_open {
                    DoorStateViewV1::Open
                } else {
                    DoorStateViewV1::Closed
                };
                let can_close = is_open && self.live_occupants_at(&target).is_empty();
                let can_open = !is_open;
                Some(DoorActionV1 {
                    direction,
                    location: target,
                    door_state,
                    target: transition.target,
                    can_open,
                    can_close,
                })
            })
            .collect()
    }

    fn collect_traversal_actions(&self, player_index: usize) -> Vec<TraversalActionV1> {
        let Some(player) = self.world.actors.get(player_index) else {
            return Vec::new();
        };
        self.effective_navigation_at(&player.location)
            .into_iter()
            .filter_map(|transition| {
                let kind = match transition.kind {
                    NavigationKind::Stairs {
                        direction: VerticalDirection::Up,
                    } => ExplicitTraversalKind::StairsUp,
                    NavigationKind::Stairs {
                        direction: VerticalDirection::Down,
                    } => ExplicitTraversalKind::StairsDown,
                    NavigationKind::Climb {
                        direction: VerticalDirection::Up,
                    } => ExplicitTraversalKind::ClimbUp,
                    NavigationKind::Climb {
                        direction: VerticalDirection::Down,
                    } => ExplicitTraversalKind::ClimbDown,
                    _ => return None,
                };
                self.evaluate_explicit_traversal(player_index, kind)
                    .ok()
                    .map(|plan| TraversalActionV1 {
                        kind,
                        target: plan.to,
                    })
            })
            .collect()
    }

    /// Map an exit block reason string to its typed V2 equivalent.
    fn exit_block_to_typed(reason: &str) -> ActionBlockedReasonV1 {
        match reason {
            "out of bounds" => ActionBlockedReasonV1::OutOfBounds,
            "blocked terrain" => ActionBlockedReasonV1::BlockedTerrain,
            "closed door" => ActionBlockedReasonV1::ClosedDoor,
            "suppressed by status" => ActionBlockedReasonV1::SuppressedByStatus,
            _ => ActionBlockedReasonV1::BlockedTerrain,
        }
    }

    /// Build shared exit drafts. V1 and V2 map these to their respective
    /// exit types (String vs ActionBlockedReasonV1 for blocked_reason).
    fn build_exit_drafts(&self, player_index: usize) -> Vec<ExitDraft> {
        let player = &self.world.actors[player_index];
        let suppressed = self.suppressing_effect_for_actor(player_index).is_some();
        Direction::all()
            .into_iter()
            .map(|direction| {
                let target = WorldPosition::new(
                    &player.location.realm,
                    &player.location.level,
                    player.location.position.step(direction),
                );
                let tile = self.effective_tile_at(&target);
                let terrain_name = tile.as_ref().map(|t| t.terrain_name.clone());
                let move_cost = tile.as_ref().and_then(|t| t.move_cost);
                let tile_effects = tile
                    .as_ref()
                    .map(|t| t.tile_effects.iter().map(TileEffectViewV1::from).collect())
                    .unwrap_or_default();

                let mut blocked = false;
                let mut blocked_reason_str = None;

                if !self.in_bounds(&target) {
                    blocked = true;
                    blocked_reason_str = Some("out of bounds".to_string());
                } else if tile.as_ref().is_none_or(|tile| !tile.passable) {
                    blocked = true;
                    blocked_reason_str = Some("blocked terrain".to_string());
                } else if suppressed {
                    blocked = true;
                    blocked_reason_str = Some("suppressed by status".to_string());
                }

                let transition = self.transition_view_at(&target);
                let opens_door = !blocked
                    && transition.as_ref().is_some_and(|transition| {
                        transition.kind == crate::view::TransitionKindViewV1::Door
                            && transition.door_state == Some(DoorStateViewV1::Closed)
                    });

                ExitDraft {
                    direction,
                    position: target,
                    terrain_name,
                    move_cost,
                    opens_door,
                    blocked,
                    blocked_reason_str,
                    transition,
                    tile_effects,
                }
            })
            .collect()
    }

    fn tile_effect_views_at(&self, location: &WorldPosition) -> Vec<TileEffectViewV1> {
        self.effective_tile_at(location)
            .map(|tile| {
                tile.tile_effects
                    .iter()
                    .map(TileEffectViewV1::from)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Build one canonical mode-option list per living same-room target.
    fn build_attack_target_drafts(&self, player_index: usize) -> Vec<AttackTargetDraft> {
        let player = &self.world.actors[player_index];
        let mut drafts = self
            .world
            .actors
            .iter()
            .enumerate()
            .filter(|(i, a)| {
                *i != player_index && a.is_alive() && a.location.same_site(&player.location)
            })
            .map(|(i, a)| {
                let social = self
                    .observed_social_view(player_index, i)
                    .expect("validated social observation");
                let attack_safety = social.attack_safety;
                let authorization =
                    if matches!(attack_safety, crate::model::AttackSafety::Protected) {
                        crate::model::HostilityAuthorization::ConfirmedUnsafe
                    } else {
                        crate::model::HostilityAuthorization::Safe
                    };
                let physical_attacks = PhysicalAttackMode::ALL
                    .into_iter()
                    .map(|mode| {
                        let (mode_plan, plan) =
                            self.physical_attack_option_plan(player_index, i, mode);
                        let mut blocked_reason =
                            plan.as_ref().err().map(Self::physical_attack_error_reason);
                        if blocked_reason.is_none()
                            && self.current_time() < self.world.actors[player_index].attack_ready_at
                        {
                            blocked_reason = Some(ActionBlockedReasonV1::NotReady);
                        }
                        if self.suppressing_effect_for_actor(player_index).is_some() {
                            blocked_reason = Some(ActionBlockedReasonV1::SuppressedByStatus);
                        }
                        let enabled = blocked_reason.is_none();
                        let command = enabled.then(|| PlayerCommandV1 {
                            contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                            actor_id: player.id.clone(),
                            intent: PlayerIntentPayloadV1::PhysicalAttack {
                                mode,
                                target_actor_id: a.id.clone(),
                                authorization,
                            },
                        });
                        match (mode_plan, plan) {
                            (_, Ok(plan)) => PhysicalAttackOptionV1 {
                                mode,
                                enabled,
                                blocked_reason,
                                maximum_range: Some(plan.maximum_range),
                                damage_kind: Some(plan.damage_kind),
                                skill_track_id: Some(plan.skill_track_id.clone()),
                                skill_level: Some(plan.skill_level),
                                projected_risk: self.physical_combat_risk(&plan).ok().flatten(),
                                selected_item_instance_id: plan.selection.item_instance_id.clone(),
                                selected_item_definition_id: plan
                                    .selection
                                    .item_definition_id
                                    .clone(),
                                full_two_handed_effect: plan.selection.full_two_handed_effect,
                                barefoot_full_effect: plan.barefoot_full_effect,
                                attack_safety,
                                command,
                            },
                            (Some(mode_plan), Err(_)) => PhysicalAttackOptionV1 {
                                mode,
                                enabled,
                                blocked_reason,
                                maximum_range: Some(mode_plan.maximum_range),
                                damage_kind: Some(mode_plan.damage_kind),
                                skill_track_id: Some(mode_plan.skill_track_id),
                                skill_level: Some(mode_plan.skill_level),
                                projected_risk: None,
                                selected_item_instance_id: mode_plan.selection.item_instance_id,
                                selected_item_definition_id: mode_plan.selection.item_definition_id,
                                full_two_handed_effect: mode_plan.selection.full_two_handed_effect,
                                barefoot_full_effect: mode_plan.barefoot_full_effect,
                                attack_safety,
                                command,
                            },
                            (None, Err(_)) => PhysicalAttackOptionV1 {
                                mode,
                                enabled,
                                blocked_reason,
                                maximum_range: None,
                                damage_kind: None,
                                skill_track_id: None,
                                skill_level: None,
                                projected_risk: None,
                                selected_item_instance_id: None,
                                selected_item_definition_id: None,
                                full_two_handed_effect: false,
                                barefoot_full_effect: false,
                                attack_safety,
                                command,
                            },
                        }
                    })
                    .collect();

                AttackTargetDraft {
                    actor_id: a.id.clone(),
                    actor_name: a.name.clone(),
                    actor_kind: a.kind,
                    creature_traits: a.creature_traits.clone(),
                    social,
                    position: a.location.clone(),
                    hp: a.hp,
                    max_hp: a.max_hp(),
                    wound_state: self.wound_state(i),
                    physical_attacks,
                    owner_id: a
                        .summoned
                        .as_ref()
                        .map(|summoned| summoned.owner_id.clone()),
                    summoned: a.summoned.as_ref().map(SummonedActorViewV1::from),
                }
            })
            .collect::<Vec<_>>();
        drafts.sort_by(|left, right| left.actor_id.cmp(&right.actor_id));
        drafts
    }
}
