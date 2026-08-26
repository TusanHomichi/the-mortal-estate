use crate::events::Event;
use crate::model::{
    ActiveEffectSource, ActiveEffectStackingPolicy, ActiveEffectState, Direction, HideActionConfig,
    HideBreakTrigger, MartialHandBlockConfig, ProfessionActionConfig, WeaponHandedness,
};
use crate::view::ActionBlockedReasonV1;

use super::{Engine, StepError};

impl Engine {
    pub(super) fn hide_action_config_for_actor(
        &self,
        actor_index: usize,
    ) -> Option<(&ProfessionActionConfig, &HideActionConfig)> {
        let class_id = self.actor_current_class_id(actor_index)?;
        self.definition
            .catalog
            .profession_actions
            .iter()
            .find_map(|action| {
                action
                    .hide
                    .as_ref()
                    .filter(|_| {
                        action
                            .class_ids
                            .iter()
                            .any(|candidate| candidate == class_id)
                    })
                    .map(|hide| (action, hide))
            })
    }

    pub(super) fn martial_hand_block_config_for_actor(
        &self,
        actor_index: usize,
    ) -> Option<&MartialHandBlockConfig> {
        let class_id = self.actor_current_class_id(actor_index)?;
        self.definition
            .catalog
            .profession_actions
            .iter()
            .find_map(|action| {
                action.martial_hand_block.as_ref().filter(|_| {
                    action
                        .class_ids
                        .iter()
                        .any(|candidate| candidate == class_id)
                })
            })
    }

    pub(super) fn actor_current_class_id(&self, actor_index: usize) -> Option<&str> {
        self.world.actors[actor_index]
            .character
            .as_ref()
            .map(|character| character.identity.current_class_id.as_str())
    }

    pub(super) fn actor_hand_level(&self, actor_index: usize) -> u8 {
        self.skill_level_for_actor(actor_index, "hand")
    }

    pub(super) fn martial_hand_block_chance_percent(&self, defender_index: usize) -> Option<i32> {
        if self.actor_current_class_id(defender_index) != Some("martial_artist") {
            return None;
        }
        if self
            .item_at_position(defender_index, crate::model::CarriedPosition::RightHand)
            .ok()
            .flatten()
            .is_some()
        {
            return None;
        }
        let config = self.martial_hand_block_config_for_actor(defender_index)?;
        let hand_level = self.actor_hand_level(defender_index);
        if i32::from(hand_level) < config.min_hand_level {
            return None;
        }
        let chance_percent =
            (i32::from(hand_level) * 100 / config.level_divisor).min(config.max_chance_percent);
        (chance_percent > 0).then_some(chance_percent)
    }

    pub(super) fn actor_is_hidden(&self, actor_index: usize) -> bool {
        self.actor_has_active_tag(actor_index, "hidden")
    }

    pub(crate) fn validate_hide_action(
        &self,
        actor_index: usize,
    ) -> Result<(), ActionBlockedReasonV1> {
        if !self
            .definition
            .catalog
            .profession_actions
            .iter()
            .any(|action| action.hide.is_some())
        {
            return Err(ActionBlockedReasonV1::NoProfessionAction);
        }
        if self.actor_current_class_id(actor_index) != Some("thief") {
            return Err(ActionBlockedReasonV1::WrongClass);
        }
        let Some((_, hide_config)) = self.hide_action_config_for_actor(actor_index) else {
            return Err(ActionBlockedReasonV1::WrongClass);
        };
        if hide_config.requires_cover_or_darkness
            && !self.hide_has_cover_or_darkness(actor_index, hide_config)
        {
            return Err(ActionBlockedReasonV1::NoCoverOrDarkness);
        }
        if !self.hide_equipment_allowed(actor_index, hide_config) {
            return Err(ActionBlockedReasonV1::ForbiddenEquipment);
        }
        Ok(())
    }

    pub(super) fn hide_has_cover_or_darkness(
        &self,
        actor_index: usize,
        _config: &HideActionConfig,
    ) -> bool {
        self.actor_has_concealment_cover_or_darkness(actor_index)
    }

    pub(super) fn actor_has_concealment_cover_or_darkness(&self, actor_index: usize) -> bool {
        let actor = &self.world.actors[actor_index];
        std::iter::once(actor.location.clone())
            .chain(Direction::all().into_iter().map(|direction| {
                crate::model::WorldPosition::new(
                    &actor.location.realm,
                    &actor.location.level,
                    actor.location.position.step(direction),
                )
            }))
            .filter(|location| self.in_bounds(location))
            .any(|location| self.hide_tile_has_cover_or_darkness(&location))
    }

    fn hide_tile_has_cover_or_darkness(&self, location: &crate::model::WorldPosition) -> bool {
        let Some(tile) = self.effective_tile_at(location) else {
            return false;
        };
        tile.blocks_sight
            || tile
                .tile_effects
                .iter()
                .any(|effect| matches!(effect.sight.as_deref(), Some("obscured" | "blocked")))
    }

    pub(super) fn hide_equipment_allowed(
        &self,
        actor_index: usize,
        config: &HideActionConfig,
    ) -> bool {
        self.active_item_ids(actor_index).is_ok_and(|items| {
            !items.iter().any(|item_instance_id| {
                config.disallow_two_handed
                    && self
                        .item_definition(item_instance_id)
                        .ok()
                        .and_then(|item| item.weapon.as_ref())
                        .is_some_and(|weapon| weapon.handedness == WeaponHandedness::TwoHanded)
            })
        })
    }

    pub(super) fn apply_player_hide(
        &mut self,
        actor_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        self.validate_hide_action(actor_index)
            .map_err(|reason| StepError::new(Self::command_blocked_reason_code(reason)))?;
        let (config_id, hide_config) = {
            let (action, hide) = self
                .hide_action_config_for_actor(actor_index)
                .expect("validated hide config must exist");
            (action.id.clone(), hide.clone())
        };
        let actor_id = self.world.actors[actor_index].id.clone();
        let instance_id = format!("profession:hide:{}:{actor_id}", self.current_time());
        let effect_state = ActiveEffectState {
            instance_id,
            effect_id: hide_config.effect_id.clone(),
            source: ActiveEffectSource {
                kind: "profession".to_string(),
                id: config_id,
            },
            source_actor_id: Some(actor_id),
            hostile_authority: None,
            spell_damage_credit: None,
            kind: "hidden".to_string(),
            tags: vec!["hidden".to_string()],
            potency: 0,
            remaining_rounds: Some(hide_config.duration_rounds),
            until_condition: Some("breaks_on_action".to_string()),
            stacking: ActiveEffectStackingPolicy::RefreshDuration,
            start_delay_rounds: 0,
            tick_interval_rounds: 1,
            suppresses_action: false,
            resistance_boosts: vec![],
            last_ticked_at: self.current_time(),
        };
        let applied = self.apply_profession_active_effect_state(actor_index, effect_state);
        let actor = &self.world.actors[actor_index];
        events.push(Event::ActorHidden {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            location: actor.location.clone(),
            instance_id: applied.instance_id.clone(),
            effect_id: applied.effect_id.clone(),
            remaining_rounds: applied.remaining_rounds,
        });
        Ok(())
    }

    fn apply_profession_active_effect_state(
        &mut self,
        actor_index: usize,
        effect_state: ActiveEffectState,
    ) -> ActiveEffectState {
        let actor = &mut self.world.actors[actor_index];
        let matching_index = actor.active_effects.iter().position(|existing| {
            existing.source.kind == effect_state.source.kind
                && existing.source.id == effect_state.source.id
        });
        match effect_state.stacking {
            ActiveEffectStackingPolicy::ReplaceSameKind => {
                actor.active_effects.retain(|existing| {
                    !((existing.source.kind == effect_state.source.kind
                        && existing.source.id == effect_state.source.id)
                        || existing.effect_id == effect_state.effect_id)
                });
                actor.active_effects.push(effect_state.clone());
                effect_state
            }
            ActiveEffectStackingPolicy::RefreshDuration => {
                if let Some(index) = matching_index {
                    let mut refreshed = effect_state.clone();
                    refreshed.instance_id = actor.active_effects[index].instance_id.clone();
                    actor.active_effects[index] = refreshed.clone();
                    refreshed
                } else {
                    actor.active_effects.push(effect_state.clone());
                    effect_state
                }
            }
            ActiveEffectStackingPolicy::StackInstance => {
                actor.active_effects.push(effect_state.clone());
                effect_state
            }
        }
    }

    pub(super) fn break_hidden_if_needed(
        &mut self,
        actor_index: usize,
        trigger: HideBreakTrigger,
        events: &mut Vec<Event>,
    ) {
        let Some((action, hide_config)) = self.hide_action_config_for_actor(actor_index) else {
            return;
        };
        if !hide_config.break_on.contains(&trigger) {
            return;
        }
        let reason = match trigger {
            HideBreakTrigger::Move => "move",
            HideBreakTrigger::Attack => "attack",
            HideBreakTrigger::ActiveItemMove => "active_item_move",
            HideBreakTrigger::Cast => "cast",
            HideBreakTrigger::Warm => "warm",
        };
        let actor_id = self.world.actors[actor_index].id.clone();
        let actor_name = self.world.actors[actor_index].name.clone();
        let actor_location = self.world.actors[actor_index].location.clone();
        let action_id = action.id.clone();
        let mut kept = Vec::new();
        for effect in std::mem::take(&mut self.world.actors[actor_index].active_effects) {
            let is_matching_hidden = effect.source.kind == "profession"
                && effect.source.id == action_id
                && (effect.kind == "hidden" || effect.tags.iter().any(|tag| tag == "hidden"));
            if is_matching_hidden {
                events.push(Event::HideBroken {
                    actor_id: actor_id.clone(),
                    actor: actor_name.clone(),
                    location: actor_location.clone(),
                    instance_id: effect.instance_id,
                    effect_id: effect.effect_id,
                    reason: reason.to_string(),
                });
            } else {
                kept.push(effect);
            }
        }
        self.world.actors[actor_index].active_effects = kept;
    }

    pub(super) fn break_spell_hidden_after_hit(
        &mut self,
        actor_index: usize,
        events: &mut Vec<Event>,
    ) {
        self.remove_spell_hidden(actor_index, "hit", events);
    }

    pub(super) fn break_spell_hidden_after_uncovered_move(
        &mut self,
        actor_index: usize,
        events: &mut Vec<Event>,
    ) {
        if self.actor_has_concealment_cover_or_darkness(actor_index) {
            return;
        }
        self.remove_spell_hidden(actor_index, "move", events);
    }

    fn remove_spell_hidden(&mut self, actor_index: usize, reason: &str, events: &mut Vec<Event>) {
        let effects = std::mem::take(&mut self.world.actors[actor_index].active_effects);
        self.world.actors[actor_index].active_effects =
            self.retain_after_spell_hidden_break(actor_index, effects, reason, events);
    }

    pub(super) fn retain_after_spell_hidden_break(
        &self,
        actor_index: usize,
        effects: Vec<ActiveEffectState>,
        reason: &str,
        events: &mut Vec<Event>,
    ) -> Vec<ActiveEffectState> {
        let actor_id = self.world.actors[actor_index].id.clone();
        let actor_name = self.world.actors[actor_index].name.clone();
        let actor_location = self.world.actors[actor_index].location.clone();
        let mut kept = Vec::new();
        for effect in effects {
            let is_spell_hidden = effect.source.kind == "spell"
                && (effect.kind == "hidden" || effect.tags.iter().any(|tag| tag == "hidden"));
            if is_spell_hidden {
                events.push(Event::HideBroken {
                    actor_id: actor_id.clone(),
                    actor: actor_name.clone(),
                    location: actor_location.clone(),
                    instance_id: effect.instance_id,
                    effect_id: effect.effect_id,
                    reason: reason.to_string(),
                });
            } else {
                kept.push(effect);
            }
        }
        kept
    }
}
