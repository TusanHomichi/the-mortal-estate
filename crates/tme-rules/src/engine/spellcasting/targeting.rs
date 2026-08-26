use crate::content::{SpellDef, SpellEffectDef, SpellTargetDef};
use crate::events::SpellPathFailureReason;
use crate::model::{
    NavigationKind, SpellEffectFamily, SpellItemLocation, SpellTarget, SpellTargetKind,
    WorldPosition,
};
use crate::view::ActionBlockedReasonV1;

use super::DoorSecretAction;
use crate::engine::{Engine, StepError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::engine) struct SpellPathPlan {
    pub directions: Vec<crate::model::Direction>,
    pub visited: Vec<WorldPosition>,
    pub final_position: Option<WorldPosition>,
    pub failure: Option<SpellPathFailureReason>,
}

impl Engine {
    pub(in crate::engine) fn spell_command_error(reason: ActionBlockedReasonV1) -> StepError {
        StepError::new(spell_blocked_reason_code(reason))
    }

    pub(super) fn normalize_spell_target(
        &self,
        spell: &SpellDef,
        target: Option<&SpellTarget>,
    ) -> Option<SpellTarget> {
        if let Some(target) = target {
            return Some(target.clone());
        }
        match spell.target.as_ref().map(|target| target.kind) {
            Some(SpellTargetKind::SelfTarget) => Some(SpellTarget::SelfTarget),
            _ => None,
        }
    }

    pub(super) fn evaluate_spell_path(
        &self,
        actor_index: usize,
        spell: &SpellDef,
        directions: &[crate::model::Direction],
    ) -> Result<SpellPathPlan, ActionBlockedReasonV1> {
        if directions.is_empty() {
            return Err(ActionBlockedReasonV1::InvalidTarget);
        }
        let actor = &self.world.actors[actor_index];
        let site = actor.location.site();
        let mut coordinate = actor.location.position;
        let mut visited = Vec::new();
        let maximum_steps = spell.target.as_ref().and_then(|target| target.range);
        for (index, direction) in directions.iter().copied().enumerate() {
            coordinate = coordinate.step(direction);
            let position = WorldPosition::new(&site.realm, &site.level, coordinate);
            let failure = if !self.in_bounds(&position) {
                Some(SpellPathFailureReason::OutOfBounds)
            } else if !self.actor_can_see(actor_index, &position) {
                Some(SpellPathFailureReason::NotVisible)
            } else if maximum_steps.is_some_and(|maximum| {
                i32::try_from(index + 1).map_or(true, |steps| steps > maximum)
            }) {
                Some(SpellPathFailureReason::OutOfRange)
            } else {
                None
            };
            visited.push(position.clone());
            if let Some(failure) = failure {
                return Ok(SpellPathPlan {
                    directions: directions.to_vec(),
                    visited,
                    final_position: Some(position),
                    failure: Some(failure),
                });
            }
        }
        Ok(SpellPathPlan {
            directions: directions.to_vec(),
            final_position: visited.last().cloned(),
            visited,
            failure: None,
        })
    }

    pub(in crate::engine) fn door_secret_action(
        effect: &SpellEffectDef,
    ) -> Option<DoorSecretAction> {
        if effect.family == SpellEffectFamily::SecretDetection && effect.door_control.is_none() {
            return Some(DoorSecretAction::RevealSecret);
        }
        match effect.door_control.as_ref()?.action.as_str() {
            "open" => Some(DoorSecretAction::Open),
            "close" => Some(DoorSecretAction::Close),
            "reveal_secret" => Some(DoorSecretAction::RevealSecret),
            "hide_secret" => Some(DoorSecretAction::HideSecret),
            _ => None,
        }
    }

    pub(in crate::engine) fn door_secret_range(
        spell: &SpellDef,
        effect: &SpellEffectDef,
    ) -> Option<i32> {
        effect
            .door_control
            .as_ref()
            .and_then(|door_control| i32::try_from(door_control.range?).ok())
            .or_else(|| spell.target.as_ref().and_then(|target| target.range))
    }

    pub(in crate::engine) fn matching_secret_transition_targets(
        &self,
        player_index: usize,
        action: DoorSecretAction,
        range: Option<i32>,
        coordinate: Option<&WorldPosition>,
    ) -> Vec<WorldPosition> {
        let player = &self.world.actors[player_index];
        let mut positions: Vec<WorldPosition> = self
            .definition
            .world_template
            .navigation
            .iter()
            .filter_map(|(location, transitions)| {
                if !location.same_site(&player.location) {
                    return None;
                }
                if let Some(coordinate) = coordinate
                    && coordinate != location
                {
                    return None;
                }
                if let Some(range) = range
                    && player
                        .location
                        .position
                        .chebyshev_distance(location.position)
                        > range
                {
                    return None;
                }
                let revealed = self.is_navigation_revealed(location);
                let dynamically_concealed = self.is_navigation_concealed(location);
                let authored_hidden = transitions.iter().any(|transition| transition.hidden);
                match action {
                    DoorSecretAction::RevealSecret
                        if dynamically_concealed || (authored_hidden && !revealed) =>
                    {
                        Some(location.clone())
                    }
                    DoorSecretAction::HideSecret if authored_hidden && revealed => {
                        Some(location.clone())
                    }
                    _ => None,
                }
            })
            .collect();
        positions.sort();
        positions
    }

    fn validate_door_secret_target(
        &self,
        player_index: usize,
        spell: &SpellDef,
        effect: &SpellEffectDef,
        target: Option<&SpellTarget>,
    ) -> Result<(), ActionBlockedReasonV1> {
        let Some(action) = Self::door_secret_action(effect) else {
            return Ok(());
        };
        let player = &self.world.actors[player_index];
        match action {
            DoorSecretAction::Open | DoorSecretAction::Close => match target {
                Some(SpellTarget::Coordinate { position }) => {
                    if !self.coordinate_within_door_secret_range(
                        player_index,
                        spell,
                        effect,
                        position,
                    ) {
                        return Err(ActionBlockedReasonV1::TargetOutOfRange);
                    }
                    let Some(transition) = self.effective_transition_at(position) else {
                        return Err(ActionBlockedReasonV1::InvalidTarget);
                    };
                    if transition.kind != NavigationKind::Door {
                        return Err(ActionBlockedReasonV1::InvalidTarget);
                    }
                    let is_open = self.effective_door_state_at(position).unwrap_or(false);
                    if action == DoorSecretAction::Open && is_open {
                        return Err(ActionBlockedReasonV1::InvalidTarget);
                    }
                    if action == DoorSecretAction::Close {
                        if !is_open {
                            return Err(ActionBlockedReasonV1::InvalidTarget);
                        }
                        if self
                            .world
                            .actors
                            .iter()
                            .any(|actor| actor.is_alive() && actor.location == *position)
                        {
                            return Err(ActionBlockedReasonV1::InvalidTarget);
                        }
                    }
                    Ok(())
                }
                Some(SpellTarget::Door { direction }) => {
                    let position = WorldPosition::new(
                        &player.location.realm,
                        &player.location.level,
                        player.location.position.step(*direction),
                    );
                    let Some(transition) = self.effective_transition_at(&position) else {
                        return Err(ActionBlockedReasonV1::InvalidTarget);
                    };
                    if transition.kind == NavigationKind::Door {
                        Ok(())
                    } else {
                        Err(ActionBlockedReasonV1::InvalidTarget)
                    }
                }
                _ => Err(ActionBlockedReasonV1::InvalidTarget),
            },
            DoorSecretAction::RevealSecret | DoorSecretAction::HideSecret => match target {
                None | Some(SpellTarget::None) => {
                    if self
                        .matching_secret_transition_targets(
                            player_index,
                            action,
                            Self::door_secret_range(spell, effect),
                            None,
                        )
                        .is_empty()
                    {
                        Err(ActionBlockedReasonV1::InvalidTarget)
                    } else {
                        Ok(())
                    }
                }
                Some(SpellTarget::Coordinate { position }) => {
                    if !self.coordinate_within_door_secret_range(
                        player_index,
                        spell,
                        effect,
                        position,
                    ) {
                        return Err(ActionBlockedReasonV1::TargetOutOfRange);
                    }
                    if self
                        .matching_secret_transition_targets(
                            player_index,
                            action,
                            Self::door_secret_range(spell, effect),
                            Some(position),
                        )
                        .is_empty()
                    {
                        Err(ActionBlockedReasonV1::InvalidTarget)
                    } else {
                        Ok(())
                    }
                }
                _ => Err(ActionBlockedReasonV1::InvalidTarget),
            },
        }
    }

    fn coordinate_within_door_secret_range(
        &self,
        player_index: usize,
        spell: &SpellDef,
        effect: &SpellEffectDef,
        position: &WorldPosition,
    ) -> bool {
        let player = &self.world.actors[player_index];
        player.location.same_site(position)
            && Self::door_secret_range(spell, effect).is_none_or(|range| {
                player
                    .location
                    .position
                    .chebyshev_distance(position.position)
                    <= range
            })
    }

    pub(in crate::engine) fn validate_spell_target(
        &self,
        player_index: usize,
        spell: &SpellDef,
        target: Option<&SpellTarget>,
    ) -> Result<(), ActionBlockedReasonV1> {
        let Some(target_def) = spell.target.as_ref() else {
            return if target.is_none() || matches!(target, Some(SpellTarget::None)) {
                Ok(())
            } else {
                Err(ActionBlockedReasonV1::InvalidTarget)
            };
        };

        let result = match target_def.kind {
            SpellTargetKind::None => {
                if target.is_none() || matches!(target, Some(SpellTarget::None)) {
                    Ok(())
                } else {
                    Err(ActionBlockedReasonV1::InvalidTarget)
                }
            }
            SpellTargetKind::SelfTarget => {
                if matches!(target, Some(SpellTarget::SelfTarget)) {
                    Ok(())
                } else {
                    Err(ActionBlockedReasonV1::InvalidTarget)
                }
            }
            SpellTargetKind::Actor => {
                let Some(SpellTarget::Actor { actor_id }) = target else {
                    return Err(ActionBlockedReasonV1::InvalidTarget);
                };
                let Some(target_index) = self
                    .world
                    .actors
                    .iter()
                    .position(|actor| actor.is_alive() && actor.id == *actor_id)
                else {
                    return Err(ActionBlockedReasonV1::InvalidTarget);
                };
                let player = &self.world.actors[player_index];
                let target_actor = &self.world.actors[target_index];
                if !player.location.same_site(&target_actor.location) {
                    return Err(ActionBlockedReasonV1::InvalidTarget);
                }
                let player_rc = player.location.clone();
                let target_rc = target_actor.location.clone();
                self.validate_target_visibility_and_range(
                    player_index,
                    target_def,
                    &player_rc,
                    &target_rc,
                )
            }
            SpellTargetKind::Coordinate => {
                let Some(SpellTarget::Coordinate { position }) = target else {
                    return Err(ActionBlockedReasonV1::InvalidTarget);
                };
                let player = &self.world.actors[player_index];
                let player_rc = player.location.clone();
                if !player.location.same_site(position) {
                    return Err(ActionBlockedReasonV1::InvalidTarget);
                }
                if !self.in_bounds(position) {
                    return Err(ActionBlockedReasonV1::InvalidTarget);
                }
                self.validate_target_visibility_and_range(
                    player_index,
                    target_def,
                    &player_rc,
                    position,
                )
            }
            SpellTargetKind::Area => {
                let Some(SpellTarget::Area { center }) = target else {
                    return Err(ActionBlockedReasonV1::InvalidTarget);
                };
                let player = &self.world.actors[player_index];
                let player_rc = player.location.clone();
                if !player.location.same_site(center) {
                    return Err(ActionBlockedReasonV1::InvalidTarget);
                }
                if !self.in_bounds(center) {
                    return Err(ActionBlockedReasonV1::InvalidTarget);
                }
                self.validate_target_visibility_and_range(
                    player_index,
                    target_def,
                    &player_rc,
                    center,
                )
            }
            SpellTargetKind::Direction => {
                if matches!(target, Some(SpellTarget::Direction { .. })) {
                    Ok(())
                } else {
                    Err(ActionBlockedReasonV1::InvalidTarget)
                }
            }
            SpellTargetKind::Door => {
                let Some(SpellTarget::Door { direction }) = target else {
                    return Err(ActionBlockedReasonV1::InvalidTarget);
                };
                let player = &self.world.actors[player_index];
                let position = WorldPosition::new(
                    &player.location.realm,
                    &player.location.level,
                    player.location.position.step(*direction),
                );
                if self
                    .effective_transition_at(&position)
                    .is_some_and(|transition| transition.kind == crate::model::NavigationKind::Door)
                {
                    Ok(())
                } else {
                    Err(ActionBlockedReasonV1::InvalidTarget)
                }
            }
            SpellTargetKind::Item => {
                let Some(SpellTarget::Item {
                    item_instance_id,
                    location,
                }) = target
                else {
                    return Err(ActionBlockedReasonV1::InvalidTarget);
                };
                if !target_def
                    .item_location
                    .is_none_or(|allowed| allowed == *location)
                {
                    return Err(ActionBlockedReasonV1::InvalidTarget);
                }
                self.resolve_spell_item(player_index, item_instance_id, *location)
                    .map(|_| ())
                    .ok_or(ActionBlockedReasonV1::InvalidTarget)
            }
        };
        result?;

        if let Some(effect) = spell.effect.as_ref()
            && matches!(
                effect.family,
                SpellEffectFamily::DoorControl | SpellEffectFamily::SecretDetection
            )
        {
            self.validate_door_secret_target(player_index, spell, effect, target)?;
        }
        if let Some(effect) = spell.effect.as_ref()
            && matches!(
                effect.family,
                SpellEffectFamily::ItemIdentify
                    | SpellEffectFamily::ItemEnchant
                    | SpellEffectFamily::WeaponEnchant
            )
        {
            self.validate_item_utility_target(player_index, effect, target)?;
        }
        if let Some(effect) = spell.effect.as_ref()
            && effect.family == SpellEffectFamily::Portal
        {
            self.validate_portal_target(player_index, effect, target)?;
        }
        if let Some(effect) = spell.effect.as_ref()
            && effect.family == SpellEffectFamily::Summon
        {
            self.validate_summon_target(target)?;
            self.validate_summon_burden(effect)
                .map_err(|_| ActionBlockedReasonV1::InvalidTarget)?;
        }
        if let Some(effect) = spell.effect.as_ref()
            && effect.family == SpellEffectFamily::Concealment
        {
            self.validate_concealment_target(player_index, target)?;
        }
        Ok(())
    }

    fn validate_concealment_target(
        &self,
        actor_index: usize,
        target: Option<&SpellTarget>,
    ) -> Result<(), ActionBlockedReasonV1> {
        match target {
            Some(SpellTarget::SelfTarget) => {
                let Some((_, config)) = self.hide_action_config_for_actor(actor_index) else {
                    return Err(ActionBlockedReasonV1::NoProfessionAction);
                };
                if !self.actor_has_concealment_cover_or_darkness(actor_index) {
                    return Err(ActionBlockedReasonV1::NoCoverOrDarkness);
                }
                if !self.hide_equipment_allowed(actor_index, config) {
                    return Err(ActionBlockedReasonV1::ForbiddenEquipment);
                }
                Ok(())
            }
            Some(SpellTarget::Door { direction }) => {
                let actor = &self.world.actors[actor_index];
                let position = WorldPosition::new(
                    &actor.location.realm,
                    &actor.location.level,
                    actor.location.position.step(*direction),
                );
                if self
                    .effective_transition_at(&position)
                    .is_some_and(|transition| transition.kind == NavigationKind::Door)
                    && self.effective_door_state_at(&position) == Some(false)
                {
                    Ok(())
                } else {
                    Err(ActionBlockedReasonV1::InvalidTarget)
                }
            }
            _ => Err(ActionBlockedReasonV1::InvalidTarget),
        }
    }

    pub(in crate::engine) fn validate_portal_target(
        &self,
        player_index: usize,
        effect: &SpellEffectDef,
        target: Option<&SpellTarget>,
    ) -> Result<(), ActionBlockedReasonV1> {
        let Some(portal) = effect.portal.as_ref() else {
            return Err(ActionBlockedReasonV1::InvalidTarget);
        };
        let Some(SpellTarget::Coordinate { position }) = target else {
            return Err(ActionBlockedReasonV1::InvalidTarget);
        };
        self.validate_portal_anchor(player_index, position)?;
        if self.portal_target_is_authored_and_passable(&portal.target) {
            Ok(())
        } else {
            Err(ActionBlockedReasonV1::InvalidTarget)
        }
    }

    pub(super) fn validate_portal_anchor(
        &self,
        player_index: usize,
        position: &WorldPosition,
    ) -> Result<(), ActionBlockedReasonV1> {
        let player = &self.world.actors[player_index];
        if !player.location.same_site(position) {
            return Err(ActionBlockedReasonV1::InvalidTarget);
        }
        if !self
            .effective_tile_at(position)
            .is_some_and(|tile| tile.passable)
        {
            return Err(ActionBlockedReasonV1::InvalidTarget);
        }
        Ok(())
    }

    pub(super) fn portal_target_is_authored_and_passable(
        &self,
        target: &crate::content::TopologyTargetDef,
    ) -> bool {
        self.resolve_topology_target(target)
            .is_some_and(|location| {
                self.in_bounds(&location)
                    && self
                        .terrain_at(&location)
                        .is_some_and(|terrain| terrain.passable)
            })
    }

    pub(super) fn resolve_topology_target(
        &self,
        target: &crate::content::TopologyTargetDef,
    ) -> Option<WorldPosition> {
        match target {
            crate::content::TopologyTargetDef::Position { location } => Some(location.clone()),
            crate::content::TopologyTargetDef::Arrival { arrival_id } => self
                .definition
                .world_template
                .arrivals
                .get(arrival_id)
                .cloned(),
        }
    }

    fn validate_item_utility_target(
        &self,
        player_index: usize,
        effect: &SpellEffectDef,
        target: Option<&SpellTarget>,
    ) -> Result<(), ActionBlockedReasonV1> {
        let Some(item_utility) = effect.item_utility.as_ref() else {
            return Ok(());
        };
        let Some(SpellTarget::Item {
            item_instance_id,
            location,
        }) = target
        else {
            return Err(ActionBlockedReasonV1::InvalidTarget);
        };
        let resolved = self
            .resolve_spell_item(player_index, item_instance_id, *location)
            .ok_or(ActionBlockedReasonV1::InvalidTarget)?;
        match item_utility.action.as_str() {
            "identify" => Ok(()),
            "enchant_weapon" => {
                if resolved.is_weapon {
                    Ok(())
                } else {
                    Err(ActionBlockedReasonV1::InvalidTarget)
                }
            }
            "transform_item" => match item_utility.output_item_definition_id.as_deref() {
                Some(output_item_definition_id)
                    if self
                        .definition
                        .catalog
                        .item_catalog
                        .contains_key(output_item_definition_id)
                        && (*location == SpellItemLocation::GroundHere
                            || self.output_item_can_replace_positioned_spell_item(
                                player_index,
                                item_instance_id,
                                output_item_definition_id,
                            ))
                        && self
                            .validate_prospective_transform_metrics(
                                item_instance_id,
                                output_item_definition_id,
                            )
                            .is_ok() =>
                {
                    Ok(())
                }
                _ => Err(ActionBlockedReasonV1::InvalidTarget),
            },
            _ => Err(ActionBlockedReasonV1::InvalidTarget),
        }
    }

    pub(super) fn validate_target_visibility_and_range(
        &self,
        player_index: usize,
        target_def: &SpellTargetDef,
        player: &WorldPosition,
        target: &WorldPosition,
    ) -> Result<(), ActionBlockedReasonV1> {
        if target_def.requires_visible.unwrap_or(false) && !self.actor_can_see(player_index, target)
        {
            return Err(ActionBlockedReasonV1::TargetNotVisible);
        }
        if let Some(range) = target_def.range {
            if !player.same_site(target) {
                return Err(ActionBlockedReasonV1::TargetOutOfRange);
            }
            if player.position.chebyshev_distance(target.position) > range {
                return Err(ActionBlockedReasonV1::TargetOutOfRange);
            }
        }
        Ok(())
    }
}

fn spell_blocked_reason_code(reason: ActionBlockedReasonV1) -> &'static str {
    reason.code()
}

pub(super) fn transition_kind_label(kind: NavigationKind) -> &'static str {
    match kind {
        NavigationKind::Walk => "walk",
        NavigationKind::Swim => "swim",
        NavigationKind::Door => "door",
        NavigationKind::Stairs { .. } => "stairs",
        NavigationKind::Pit => "pit",
        NavigationKind::Climb { .. } => "climb",
        NavigationKind::Passage => "passage",
        NavigationKind::Portal => "portal",
    }
}
