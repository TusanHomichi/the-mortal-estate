//! Effect families that write tile, door and transition state.

use crate::content::SpellEffectDef;
use crate::events::{Event, TransitionConcealmentRemovalReasonV1};
use crate::model::{
    ActiveEffectSource, ActiveEffectState, HostileEffectAuthority, NavigationKind,
    SpellEffectFamily, SpellTarget, TileEffectState,
};

use super::super::{DoorSecretAction, SpellCommandPlan, SpellEffectOutcome};
use crate::engine::spellcasting::targeting::transition_kind_label;
use crate::engine::{Engine, StepError};

impl Engine {
    pub(super) fn apply_door_secret_spell(
        &mut self,
        player_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(action) = Self::door_secret_action(effect) else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        match action {
            DoorSecretAction::Open | DoorSecretAction::Close => {
                self.apply_door_control_spell(player_index, plan, action, events)
            }
            DoorSecretAction::RevealSecret | DoorSecretAction::HideSecret => {
                self.apply_secret_transition_spell(player_index, plan, effect, action, events)
            }
        }
    }

    pub(super) fn apply_door_control_spell(
        &mut self,
        player_index: usize,
        plan: &SpellCommandPlan,
        action: DoorSecretAction,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        match plan.target.as_ref() {
            Some(SpellTarget::Door { direction }) => match action {
                DoorSecretAction::Open => self.apply_door_open(player_index, *direction, events)?,
                DoorSecretAction::Close => {
                    self.apply_door_close(player_index, *direction, events)?
                }
                _ => return Ok(SpellEffectOutcome::Stubbed),
            },
            Some(SpellTarget::Coordinate { position }) => {
                let player = &self.world.actors[player_index];
                if !player.location.same_site(position) {
                    return Ok(SpellEffectOutcome::Stubbed);
                }
                let transition = self
                    .effective_transition_at(position)
                    .ok_or_else(|| StepError::new("invalid_target"))?;
                if transition.kind != NavigationKind::Door {
                    return Ok(SpellEffectOutcome::Stubbed);
                }
                if action == DoorSecretAction::Close
                    && self
                        .world
                        .actors
                        .iter()
                        .any(|actor| actor.is_alive() && actor.location == *position)
                {
                    return Err(StepError::new("invalid_target"));
                }
                let state = self
                    .world
                    .door_states
                    .get_mut(position)
                    .ok_or_else(|| StepError::new("invalid_target"))?;
                let (actor_id, actor) = {
                    let player = &self.world.actors[player_index];
                    (player.id.clone(), player.name.clone())
                };
                match action {
                    DoorSecretAction::Open => {
                        *state = true;
                        events.push(Event::DoorOpened {
                            actor_id,
                            actor,
                            location: position.clone(),
                        });
                    }
                    DoorSecretAction::Close => {
                        *state = false;
                        events.push(Event::DoorClosed {
                            actor_id,
                            actor,
                            location: position.clone(),
                        });
                    }
                    _ => return Ok(SpellEffectOutcome::Stubbed),
                }
            }
            _ => return Ok(SpellEffectOutcome::Stubbed),
        }
        Ok(SpellEffectOutcome::Applied)
    }

    pub(super) fn apply_secret_transition_spell(
        &mut self,
        player_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        action: DoorSecretAction,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let positions = match plan.target.as_ref() {
            Some(SpellTarget::Coordinate { position }) => vec![position.clone()],
            None | Some(SpellTarget::None) => {
                let range = self
                    .definition
                    .catalog
                    .spells
                    .get(&plan.spell_id)
                    .and_then(|spell| Self::door_secret_range(spell, effect));
                self.matching_secret_transition_targets(player_index, action, range, None)
            }
            _ => return Ok(SpellEffectOutcome::Stubbed),
        };
        if positions.is_empty() {
            return Ok(SpellEffectOutcome::Stubbed);
        }
        let (actor_id, actor) = {
            let player = &self.world.actors[player_index];
            (player.id.clone(), player.name.clone())
        };
        for position in positions {
            let Some(transitions) = self.definition.world_template.navigation.get(&position) else {
                continue;
            };
            let Some(transition) = transitions
                .iter()
                .find(|transition| transition.hidden)
                .or_else(|| {
                    self.is_navigation_concealed(&position)
                        .then(|| transitions.first())
                        .flatten()
                })
            else {
                continue;
            };
            let transition_kind = transition_kind_label(transition.kind).to_string();
            let authored_hidden = transition.hidden;
            match action {
                DoorSecretAction::RevealSecret => {
                    self.remove_navigation_concealment_at(
                        &position,
                        TransitionConcealmentRemovalReasonV1::Revealed,
                        events,
                    );
                    if authored_hidden && !self.is_navigation_revealed(&position) {
                        self.set_navigation_revealed(&position, true)?;
                        events.push(Event::SecretTransitionRevealed {
                            actor_id: actor_id.clone(),
                            actor: actor.clone(),
                            location: position,
                            transition_kind,
                        });
                    }
                }
                DoorSecretAction::HideSecret => {
                    self.set_navigation_revealed(&position, false)?;
                    events.push(Event::SecretTransitionHidden {
                        actor_id: actor_id.clone(),
                        actor: actor.clone(),
                        location: position,
                        transition_kind,
                    });
                }
                _ => return Ok(SpellEffectOutcome::Stubbed),
            }
        }
        Ok(SpellEffectOutcome::Applied)
    }

    pub(super) fn apply_tile_overlay_spell(
        &mut self,
        caster_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(overlay) = effect.terrain_overlay.as_ref() else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        if overlay.passability.is_none()
            && overlay.sight.is_none()
            && overlay.hazard.is_none()
            && overlay.move_cost.is_none()
        {
            return Ok(SpellEffectOutcome::Stubbed);
        }
        let Some(positions) = self.tile_positions_for_spell_target(plan, effect) else {
            return Ok(SpellEffectOutcome::Stubbed);
        };

        if overlay.passability.as_deref() == Some("remove_overlay")
            || overlay.sight.as_deref() == Some("remove_overlay")
        {
            let remove_passability = overlay.passability.as_deref() == Some("remove_overlay");
            let remove_sight = overlay.sight.as_deref() == Some("remove_overlay");
            for position in positions {
                self.remove_tile_effects_at(
                    &position,
                    remove_passability,
                    remove_sight,
                    &plan.spell_id,
                    events,
                );
            }
            return Ok(SpellEffectOutcome::Applied);
        }

        let mut tags = self.spell_effect_tags(effect);
        if let Some(hazard) = overlay.hazard.as_ref()
            && hazard != "unknown"
        {
            tags.push(hazard.clone());
        }
        tags.sort();
        tags.dedup();

        for position in positions {
            let effect_state = TileEffectState {
                instance_id: format!(
                    "tile:{}:{}:{}:{}:{}",
                    plan.spell_id,
                    self.current_time(),
                    position.level,
                    position.position.x,
                    position.position.y
                ),
                effect_id: plan.spell_id.clone(),
                source: ActiveEffectSource {
                    kind: "spell".to_string(),
                    id: plan.spell_id.clone(),
                },
                source_actor_id: Some(self.world.actors[caster_index].id.clone()),
                hostile_authority: plan.hostility_authorization.and_then(|authorization| {
                    self.world.actors[caster_index].character_id.clone().map(
                        |credited_character_id| HostileEffectAuthority {
                            credited_actor_id: self.world.actors[caster_index].id.clone(),
                            credited_character_id,
                            authorization,
                        },
                    )
                }),
                location: position,
                kind: effect.family.label().to_string(),
                tags: tags.clone(),
                potency: effect.potency.unwrap_or(0),
                remaining_rounds: self.spell_effect_remaining_rounds(effect.duration.as_ref()),
                passability: overlay.passability.clone(),
                sight: overlay.sight.clone(),
                hazard: overlay.hazard.clone(),
                move_cost: overlay.move_cost,
                tick_interval_rounds: self
                    .spell_effect_tick_interval_rounds(effect.duration.as_ref()),
                last_ticked_at: self.current_time(),
            };
            self.apply_tile_effect_state(effect_state, events);
        }

        Ok(SpellEffectOutcome::Applied)
    }

    pub(super) fn apply_active_effect_spell(
        &mut self,
        player_index: usize,
        plan: &SpellCommandPlan,
        effect: &SpellEffectDef,
        events: &mut Vec<Event>,
    ) -> Result<SpellEffectOutcome, StepError> {
        let Some(target_index) = self.resolve_spell_effect_target_index(player_index, plan) else {
            return Ok(SpellEffectOutcome::Stubbed);
        };
        if matches!(
            effect.family,
            SpellEffectFamily::ControlStatus | SpellEffectFamily::Poison
        ) && let Some(resolution) =
            self.resolve_spell_resistance(target_index, &plan.spell_id, effect, None, events)
            && resolution.success
        {
            return Ok(SpellEffectOutcome::Applied);
        }
        let target_actor_id = self.world.actors[target_index].id.clone();
        let effect_state = ActiveEffectState {
            instance_id: format!(
                "spell:{}:{}:{}",
                plan.spell_id,
                self.current_time(),
                target_actor_id
            ),
            effect_id: plan.spell_id.clone(),
            source: ActiveEffectSource {
                kind: "spell".to_string(),
                id: plan.spell_id.clone(),
            },
            source_actor_id: Some(self.world.actors[player_index].id.clone()),
            hostile_authority: (target_index != player_index)
                .then_some(plan.hostility_authorization)
                .flatten()
                .and_then(|authorization| {
                    self.world.actors[player_index].character_id.clone().map(
                        |credited_character_id| HostileEffectAuthority {
                            credited_actor_id: self.world.actors[player_index].id.clone(),
                            credited_character_id,
                            authorization,
                        },
                    )
                }),
            spell_damage_credit: if effect.family == SpellEffectFamily::Poison {
                plan.damage_credit(&self.world.actors[player_index].id)
            } else {
                None
            },
            kind: effect.family.label().to_string(),
            tags: self.spell_effect_tags(effect),
            potency: effect.potency.unwrap_or(0),
            remaining_rounds: self.spell_effect_remaining_rounds(effect.duration.as_ref()),
            until_condition: None,
            stacking: self.spell_effect_stacking(effect),
            start_delay_rounds: self.spell_effect_start_delay_rounds(effect),
            tick_interval_rounds: self.spell_effect_tick_interval_rounds(effect.duration.as_ref()),
            suppresses_action: self.spell_effect_suppresses_action(effect),
            resistance_boosts: Self::resistance_boosts_from_effect(effect),
            last_ticked_at: self.current_time(),
        };
        self.apply_spell_active_effect_state(target_index, effect_state, events);
        Ok(SpellEffectOutcome::Applied)
    }
}
