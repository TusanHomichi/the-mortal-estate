use super::death::DefeatContext;
use super::{Engine, StepError};
use crate::events::{Event, TransitionConcealmentRemovalReasonV1};
use crate::model::DeathCause;

impl Engine {
    pub(super) fn actor_has_active_tag(&self, actor_index: usize, tag: &str) -> bool {
        self.world.actors[actor_index]
            .active_effects
            .iter()
            .any(|effect| {
                effect.start_delay_rounds == 0 && effect.tags.iter().any(|existing| existing == tag)
            })
    }

    pub(super) fn actor_is_blind(&self, actor_index: usize) -> bool {
        self.actor_has_active_tag(actor_index, "blind")
    }

    pub(super) fn actor_has_effect_matching_tag(&self, actor_index: usize, tag: &str) -> bool {
        self.world.actors[actor_index]
            .active_effects
            .iter()
            .any(|effect| effect.kind == tag || effect.tags.iter().any(|existing| existing == tag))
    }

    pub(super) fn apply_active_effect_ticks(
        &mut self,
        events: &mut Vec<Event>,
    ) -> Result<(), super::StepError> {
        self.apply_transition_concealment_ticks(events);
        self.apply_portal_transition_ticks(events);
        self.apply_item_enchantment_ticks(events);
        self.apply_summon_ticks(events)?;
        let now = self.current_time();
        for actor_index in 0..self.world.actors.len() {
            if !self.world.actors[actor_index].is_alive() {
                continue;
            }
            let actor_id = self.world.actors[actor_index].id.clone();
            let actor_name = self.world.actors[actor_index].name.clone();
            let actor_location = self.world.actors[actor_index].location.clone();
            let mut kept = Vec::new();
            let mut defeat_resolved = false;
            let effects = std::mem::take(&mut self.world.actors[actor_index].active_effects);
            let mut remaining_effects = effects.into_iter();
            while let Some(mut effect) = remaining_effects.next() {
                if effect.start_delay_rounds > 0 {
                    if now.elapsed_rounds_since(effect.last_ticked_at) >= 1 {
                        effect.start_delay_rounds -= 1;
                        effect.last_ticked_at = now;
                    }
                    kept.push(effect);
                    continue;
                }
                let should_tick = now.elapsed_rounds_since(effect.last_ticked_at)
                    >= u64::from(effect.tick_interval_rounds);
                if should_tick {
                    effect.last_ticked_at = now;
                    if let Some(remaining) = effect.remaining_rounds.as_mut() {
                        *remaining = remaining.saturating_sub(1);
                    }
                    events.push(Event::EffectTicked {
                        actor_id: actor_id.clone(),
                        actor: actor_name.clone(),
                        location: actor_location.clone(),
                        instance_id: effect.instance_id.clone(),
                        effect_id: effect.effect_id.clone(),
                        kind: effect.kind.clone(),
                        tags: effect.tags.clone(),
                        potency: effect.potency,
                        remaining_rounds: effect.remaining_rounds,
                    });
                    if effect.kind == "poison" {
                        let applies = match effect.hostile_authority.as_ref() {
                            Some(authority) => {
                                self.delayed_hostile_contact_allowed(authority, actor_index)?
                            }
                            None => true,
                        };
                        if applies {
                            let potency = effect.potency;
                            let damaged_actor_id = actor_id.clone();
                            let damaged_actor_name = actor_name.clone();
                            let damaged_actor_location = actor_location.clone();
                            let instance_id = effect.instance_id.clone();
                            let effect_id = effect.effect_id.clone();
                            let kind = effect.kind.clone();
                            let tags = effect.tags.clone();
                            let credited_actor_id = effect.source_actor_id.clone();
                            let spell_damage_credit = effect.spell_damage_credit.clone();
                            let hostile_authority = effect.hostile_authority.clone();
                            let direct_social_actor_id =
                                hostile_authority.as_ref().and_then(|authority| {
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
                            kept.push(effect);
                            let outcome = self.apply_damage_and_resolve_defeat(
                                actor_index,
                                potency,
                                DefeatContext {
                                    cause: DeathCause::Poison,
                                    credited_actor_id,
                                    direct_social_actor_id,
                                    spell_damage_credit,
                                    hostile_authority,
                                },
                                events,
                                move |outcome| Event::EffectDamaged {
                                    actor_id: damaged_actor_id,
                                    actor: damaged_actor_name,
                                    location: damaged_actor_location,
                                    instance_id,
                                    effect_id,
                                    kind,
                                    tags,
                                    damage: outcome.applied,
                                    hp: outcome.hp_after,
                                },
                                |actor| {
                                    kept.extend(remaining_effects.by_ref());
                                    actor.active_effects = std::mem::take(&mut kept);
                                },
                            )?;
                            if outcome.defeated {
                                defeat_resolved = true;
                                break;
                            }
                            if outcome.applied > 0 {
                                kept = self.retain_after_spell_hidden_break(
                                    actor_index,
                                    kept,
                                    "hit",
                                    events,
                                );
                                let remaining = remaining_effects.collect::<Vec<_>>();
                                remaining_effects = self
                                    .retain_after_spell_hidden_break(
                                        actor_index,
                                        remaining,
                                        "hit",
                                        events,
                                    )
                                    .into_iter();
                            }
                            effect = kept
                                .pop()
                                .expect("nonlethal poison effect remains in tick state");
                        }
                    }
                }
                if effect.remaining_rounds == Some(0) {
                    events.push(Event::EffectExpired {
                        actor_id: actor_id.clone(),
                        actor: actor_name.clone(),
                        location: actor_location.clone(),
                        instance_id: effect.instance_id.clone(),
                        effect_id: effect.effect_id.clone(),
                        kind: effect.kind.clone(),
                    });
                } else {
                    kept.push(effect);
                }
            }
            if !defeat_resolved {
                self.world.actors[actor_index].active_effects = kept;
            }
        }
        Ok(())
    }

    fn apply_transition_concealment_ticks(&mut self, events: &mut Vec<Event>) {
        let now = self.current_time();
        let mut kept = Vec::new();
        for mut concealment in std::mem::take(&mut self.world.concealed_transitions) {
            if now.elapsed_rounds_since(concealment.last_ticked_at) >= 1 {
                concealment.last_ticked_at = now;
                concealment.remaining_rounds = concealment.remaining_rounds.saturating_sub(1);
            }
            if concealment.remaining_rounds == 0 {
                events.push(Event::TransitionConcealmentRemoved {
                    instance_id: concealment.instance_id,
                    source_spell_id: concealment.source_spell_id,
                    source_actor_id: concealment.source_actor_id,
                    location: concealment.location,
                    reason: TransitionConcealmentRemovalReasonV1::Expired,
                });
            } else {
                kept.push(concealment);
            }
        }
        self.world.concealed_transitions = kept;
    }

    fn apply_portal_transition_ticks(&mut self, events: &mut Vec<Event>) {
        let now = self.current_time();
        let mut kept = Vec::new();
        for mut portal in std::mem::take(&mut self.world.portal_transitions) {
            if now.elapsed_rounds_since(portal.last_ticked_at) >= 1 {
                portal.last_ticked_at = now;
                if let Some(remaining) = portal.remaining_rounds.as_mut() {
                    *remaining = remaining.saturating_sub(1);
                }
            }
            if portal.remaining_rounds == Some(0) {
                events.push(Event::PortalExpired {
                    instance_id: portal.instance_id,
                    location: portal.location,
                });
            } else {
                kept.push(portal);
            }
        }
        self.world.portal_transitions = kept;
    }

    fn apply_item_enchantment_ticks(&mut self, events: &mut Vec<Event>) {
        let now = self.current_time();
        let mut kept = Vec::new();
        for mut enchantment in std::mem::take(&mut self.world.item_enchantments) {
            if now.elapsed_rounds_since(enchantment.last_ticked_at) >= 1 {
                enchantment.last_ticked_at = now;
                if let Some(remaining) = enchantment.remaining_rounds.as_mut() {
                    *remaining = remaining.saturating_sub(1);
                }
            }
            if enchantment.remaining_rounds == Some(0) {
                let instance = self
                    .world
                    .item_instances
                    .get(&enchantment.item_instance_id)
                    .expect("active enchantment references a validated item instance");
                events.push(Event::ItemEnchantmentExpired {
                    item_instance_id: enchantment.item_instance_id,
                    item_definition_id: instance.definition_id.clone(),
                    quantity: instance.quantity,
                    enchantment_instance_id: enchantment.enchantment_instance_id,
                    source: enchantment.source,
                });
            } else {
                kept.push(enchantment);
            }
        }
        self.world.item_enchantments = kept;
    }

    pub(super) fn suppressing_effect_for_actor(
        &self,
        actor_index: usize,
    ) -> Option<&crate::model::ActiveEffectState> {
        self.world.actors[actor_index]
            .active_effects
            .iter()
            .find(|effect| effect.suppresses_action && effect.start_delay_rounds == 0)
    }

    pub(super) fn remove_active_effects_from_actor(
        &mut self,
        actor_index: usize,
        reason: &str,
        events: &mut Vec<Event>,
    ) {
        let actor_id = self.world.actors[actor_index].id.clone();
        let actor_name = self.world.actors[actor_index].name.clone();
        let actor_location = self.world.actors[actor_index].location.clone();
        let effects = std::mem::take(&mut self.world.actors[actor_index].active_effects);
        for effect in effects {
            events.push(Event::EffectRemoved {
                actor_id: actor_id.clone(),
                actor: actor_name.clone(),
                location: actor_location.clone(),
                instance_id: effect.instance_id,
                effect_id: effect.effect_id,
                kind: effect.kind,
                reason: reason.to_string(),
            });
        }
    }

    pub(super) fn remove_active_effects_matching_tag_from_actor(
        &mut self,
        actor_index: usize,
        tag: &str,
        reason: &str,
        events: &mut Vec<Event>,
    ) -> usize {
        let actor_id = self.world.actors[actor_index].id.clone();
        let actor_name = self.world.actors[actor_index].name.clone();
        let actor_location = self.world.actors[actor_index].location.clone();
        let mut removed = 0;
        let mut kept = Vec::new();
        let effects = std::mem::take(&mut self.world.actors[actor_index].active_effects);
        for effect in effects {
            let matches_tag =
                effect.kind == tag || effect.tags.iter().any(|existing| existing == tag);
            if matches_tag {
                removed += 1;
                events.push(Event::EffectRemoved {
                    actor_id: actor_id.clone(),
                    actor: actor_name.clone(),
                    location: actor_location.clone(),
                    instance_id: effect.instance_id,
                    effect_id: effect.effect_id,
                    kind: effect.kind,
                    reason: reason.to_string(),
                });
            } else {
                kept.push(effect);
            }
        }
        self.world.actors[actor_index].active_effects = kept;
        removed
    }

    pub(super) fn apply_balm_tick(
        &mut self,
        player_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let Some(mut effect) = self.world.actors[player_index].balm_effect else {
            return Ok(());
        };
        if self
            .current_time()
            .elapsed_rounds_since(effect.last_tick_at)
            < 1
        {
            return Ok(());
        }
        if !self.world.actors[player_index].is_alive() {
            self.world.actors[player_index].balm_effect = None;
            return Ok(());
        }
        let missing_hp =
            self.world.actors[player_index].max_hp() - self.world.actors[player_index].hp;
        let amount = effect
            .heal_per_round
            .min(missing_hp)
            .min(effect.budget.saturating_sub(effect.restored));
        if amount <= 0 {
            self.world.actors[player_index].balm_effect = None;
            return Ok(());
        }
        let delta = self.change_hp(player_index, amount)?;
        effect.restored += delta.actual;
        effect.last_tick_at = self.current_time();
        events.push(Event::BalmHealed {
            actor_id: self.world.actors[player_index].id.clone(),
            actor: self.world.actors[player_index].name.clone(),
            location: self.world.actors[player_index].location.clone(),
            amount: delta.actual,
            hp: delta.current,
        });
        if self.world.actors[player_index].hp >= self.world.actors[player_index].max_hp()
            || effect.restored >= effect.budget
        {
            self.world.actors[player_index].balm_effect = None;
        } else {
            self.world.actors[player_index].balm_effect = Some(effect);
        }
        Ok(())
    }
}
