use super::death::DefeatContext;
use super::{Engine, StepError};
use crate::events::Event;
use crate::model::ActorState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DamageOutcome {
    pub(super) requested: i32,
    pub(super) applied: i32,
    pub(super) hp_before: i32,
    pub(super) hp_after: i32,
    pub(super) defeated: bool,
}

impl Engine {
    /// Apply already-adjudicated damage, emit its domain event, then resolve
    /// the current defeat transition exactly once.
    ///
    /// `prepare_defeat` runs only for a lethal result, after the source event
    /// and before defeat cleanup. Poison ticking uses it to restore the
    /// temporarily detached active-effect vector.
    pub(super) fn apply_damage_and_resolve_defeat<MakeEvent, PrepareDefeat>(
        &mut self,
        target_index: usize,
        requested_damage: i32,
        context: DefeatContext,
        events: &mut Vec<Event>,
        make_event: MakeEvent,
        prepare_defeat: PrepareDefeat,
    ) -> Result<DamageOutcome, StepError>
    where
        MakeEvent: FnOnce(DamageOutcome) -> Event,
        PrepareDefeat: FnOnce(&mut ActorState),
    {
        let delta = self.change_hp(target_index, -requested_damage)?;
        let outcome = DamageOutcome {
            requested: requested_damage,
            applied: -delta.actual,
            hp_before: delta.before,
            hp_after: delta.current,
            defeated: self.world.actors[target_index].is_alive() && delta.current == 0,
        };

        events.push(make_event(outcome));
        if outcome.applied > 0 {
            self.record_defeat_contribution(target_index, outcome.applied, &context, events)?;
            self.break_spell_hidden_after_hit(target_index, events);
        }
        if outcome.defeated {
            prepare_defeat(&mut self.world.actors[target_index]);
            self.resolve_actor_defeat(target_index, context, events)?;
        } else {
            self.fizzle_warmed_spell_for_damage(
                target_index,
                outcome.applied,
                outcome.hp_before,
                events,
            );
        }
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::SpellFizzleCause;
    use crate::model::{LogicalTime, WarmedSpellState, WarmedSpellStatus};
    use std::cell::Cell;

    fn engine_from(case_id: &str) -> Engine {
        crate::engine::setup::test_engine(case_id)
    }

    fn actor_index(engine: &Engine, actor_id: &str) -> usize {
        engine
            .world
            .actors
            .iter()
            .position(|actor| actor.id == actor_id)
            .unwrap_or_else(|| panic!("actor {actor_id:?} should exist"))
    }

    fn apply_test_damage<PrepareDefeat>(
        engine: &mut Engine,
        target_index: usize,
        requested_damage: i32,
        events: &mut Vec<Event>,
        prepare_defeat: PrepareDefeat,
    ) -> Result<DamageOutcome, StepError>
    where
        PrepareDefeat: FnOnce(&mut ActorState),
    {
        let (actor_id, actor_name, location) = {
            let actor = &engine.world.actors[target_index];
            (actor.id.clone(), actor.name.clone(), actor.location.clone())
        };
        engine.apply_damage_and_resolve_defeat(
            target_index,
            requested_damage,
            DefeatContext {
                cause: crate::model::DeathCause::Hazard,
                credited_actor_id: None,
                direct_social_actor_id: None,
                spell_damage_credit: None,
                hostile_authority: None,
            },
            events,
            move |outcome| Event::EffectDamaged {
                actor_id,
                actor: actor_name,
                location,
                instance_id: "damage:test".to_string(),
                effect_id: "damage_test".to_string(),
                kind: "test_damage".to_string(),
                tags: vec![],
                damage: outcome.applied,
                hp: outcome.hp_after,
            },
            prepare_defeat,
        )
    }

    fn event_position(events: &[Event], predicate: impl Fn(&Event) -> bool) -> usize {
        events
            .iter()
            .position(predicate)
            .unwrap_or_else(|| panic!("event not found in {events:?}"))
    }

    fn install_warmed_spell(engine: &mut Engine, actor_index: usize) {
        engine.world.actors[actor_index].warmed_spell = Some(WarmedSpellState {
            spell_id: "damage_test_spell".to_string(),
            warmed_at: LogicalTime::FIRST,
            ready_at: LogicalTime::new(2),
            status: WarmedSpellStatus::Warming,
        });
    }

    fn set_actor_hp(engine: &mut Engine, actor_index: usize, hp: i32) {
        let current = engine.world.actors[actor_index].hp;
        engine
            .change_hp(actor_index, hp - current)
            .expect("test HP change");
    }

    #[test]
    fn nonlethal_damage_reports_actual_amount_and_syncs_character_hp() {
        let mut engine = engine_from("character_sheet");
        let player_index = actor_index(&engine, "player");
        let hp_before = engine.world.actors[player_index].hp;
        let mut events = Vec::new();

        let outcome = apply_test_damage(&mut engine, player_index, 3, &mut events, |_| {})
            .expect("nonlethal damage should resolve");

        assert_eq!(
            outcome,
            DamageOutcome {
                requested: 3,
                applied: 3,
                hp_before,
                hp_after: hp_before - 3,
                defeated: false,
            }
        );
        let player = &engine.world.actors[player_index];
        assert_eq!(player.hp, hp_before - 3);
        assert_eq!(
            player
                .character
                .as_ref()
                .expect("character-backed fixture")
                .resources
                .hp,
            player.hp
        );
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            Event::EffectDamaged {
                damage: 3,
                hp,
                ..
            } if hp == hp_before - 3
        ));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::ActorDefeated { .. }))
        );
    }

    #[test]
    fn strict_fraction_fizzles_only_above_the_authored_threshold() {
        for (hp_before, damage, should_fizzle) in [
            (10, 1, false),
            (10, 2, false),
            (10, 3, true),
            (11, 2, false),
            (11, 3, true),
        ] {
            let mut engine = engine_from("character_sheet");
            let player_index = actor_index(&engine, "player");
            set_actor_hp(&mut engine, player_index, hp_before);
            install_warmed_spell(&mut engine, player_index);
            let mut events = Vec::new();

            let outcome = apply_test_damage(&mut engine, player_index, damage, &mut events, |_| {})
                .expect("fractional test damage");

            assert_eq!(outcome.applied, damage);
            assert_eq!(outcome.hp_before, hp_before);
            assert_eq!(
                engine.world.actors[player_index].warmed_spell.is_none(),
                should_fizzle,
                "hp_before={hp_before} damage={damage}"
            );
            let fizzles = events
                .iter()
                .filter(|event| matches!(event, Event::SpellFizzled { .. }))
                .count();
            assert_eq!(fizzles, usize::from(should_fizzle));
            if should_fizzle {
                assert!(matches!(
                    events.iter().find(|event| matches!(event, Event::SpellFizzled { .. })),
                    Some(Event::SpellFizzled {
                        cause: SpellFizzleCause::Damage {
                            applied_damage,
                            hp_before: event_hp_before,
                        },
                        ..
                    }) if *applied_damage == damage && *event_hp_before == hp_before
                ));
            }
        }
    }

    #[test]
    fn zero_damage_preserves_a_warmed_spell() {
        let mut engine = engine_from("character_sheet");
        let player_index = actor_index(&engine, "player");
        install_warmed_spell(&mut engine, player_index);
        let mut events = Vec::new();

        let outcome = apply_test_damage(&mut engine, player_index, 0, &mut events, |_| {})
            .expect("zero damage");

        assert_eq!(outcome.applied, 0);
        assert!(engine.world.actors[player_index].warmed_spell.is_some());
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::SpellFizzled { .. }))
        );
    }

    #[test]
    fn overkill_keeps_requested_damage_and_clamps_applied_damage() {
        let mut engine = engine_from("first_room");
        let target_index = actor_index(&engine, "mireling");
        let hp_before = engine.world.actors[target_index].hp;
        let requested = hp_before + 20;
        let mut events = Vec::new();

        let outcome = apply_test_damage(&mut engine, target_index, requested, &mut events, |_| {})
            .expect("overkill damage should resolve");

        assert_eq!(outcome.requested, requested);
        assert_eq!(outcome.applied, hp_before);
        assert_eq!(outcome.hp_before, hp_before);
        assert_eq!(outcome.hp_after, 0);
        assert!(outcome.defeated);
    }

    #[test]
    fn lethal_overkill_uses_actual_damage_and_defeat_fizzle_exactly_once() {
        let mut engine = engine_from("character_sheet");
        let player_index = actor_index(&engine, "player");
        install_warmed_spell(&mut engine, player_index);
        let hp_before = engine.world.actors[player_index].hp;
        let requested = hp_before + 50;
        let mut events = Vec::new();

        let outcome = apply_test_damage(&mut engine, player_index, requested, &mut events, |_| {})
            .expect("lethal overkill");

        assert_eq!(outcome.requested, requested);
        assert_eq!(outcome.applied, hp_before);
        let source = event_position(&events, |event| {
            matches!(event, Event::EffectDamaged { hp: 0, .. })
        });
        let fizzle = event_position(&events, |event| {
            matches!(
                event,
                Event::SpellFizzled {
                    cause: SpellFizzleCause::Defeat,
                    ..
                }
            )
        });
        let defeat = event_position(
            &events,
            |event| matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == "player"),
        );
        assert!(source < fizzle && fizzle < defeat);
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::SpellFizzled { .. }))
                .count(),
            1
        );
        assert!(!events.iter().any(|event| matches!(
            event,
            Event::SpellFizzled {
                cause: SpellFizzleCause::Damage { .. },
                ..
            }
        )));
    }

    #[test]
    fn lethal_damage_emits_source_event_before_exactly_one_defeat() {
        let mut engine = engine_from("first_room");
        let target_index = actor_index(&engine, "mireling");
        let mut events = Vec::new();

        apply_test_damage(&mut engine, target_index, 100, &mut events, |_| {})
            .expect("lethal damage should resolve");

        let damaged = event_position(
            &events,
            |event| matches!(event, Event::EffectDamaged { actor_id, hp: 0, .. } if actor_id == "mireling"),
        );
        let defeated = event_position(
            &events,
            |event| matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == "mireling"),
        );
        assert!(damaged < defeated);
        assert_eq!(
            events
                .iter()
                .filter(|event| {
                    matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == "mireling")
                })
                .count(),
            1
        );
        assert_eq!(engine.world.actors[target_index].hp, 0);
        assert!(!engine.world.actors[target_index].is_alive());
    }

    #[test]
    fn prepare_defeat_runs_only_for_lethal_damage_before_effect_cleanup() {
        let mut engine = engine_from("status_effects");
        let player_index = actor_index(&engine, "player");
        let nonlethal_prepared = Cell::new(false);
        let mut events = Vec::new();

        apply_test_damage(&mut engine, player_index, 1, &mut events, |_| {
            nonlethal_prepared.set(true);
        })
        .expect("nonlethal damage should resolve");
        assert!(!nonlethal_prepared.get());

        let detached = std::mem::take(&mut engine.world.actors[player_index].active_effects);
        assert_eq!(detached.len(), 1);
        let lethal_prepared = Cell::new(false);
        apply_test_damage(&mut engine, player_index, 100, &mut events, |actor| {
            lethal_prepared.set(true);
            actor.active_effects = detached;
        })
        .expect("lethal damage should resolve");

        assert!(lethal_prepared.get());
        let damaged = events
            .iter()
            .rposition(|event| matches!(event, Event::EffectDamaged { hp: 0, .. }))
            .expect("lethal source event");
        let removed = event_position(&events, |event| {
            matches!(
                event,
                Event::EffectRemoved {
                    actor_id,
                    instance_id,
                    reason,
                    ..
                } if actor_id == "player"
                    && instance_id == "rooted_1"
                    && reason == "defeat"
            )
        });
        let defeated = event_position(
            &events,
            |event| matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == "player"),
        );
        assert!(damaged < removed);
        assert!(removed < defeated);
        assert!(engine.world.actors[player_index].active_effects.is_empty());
    }

    #[test]
    fn damage_to_dead_actor_does_not_trigger_second_defeat() {
        let mut engine = engine_from("first_room");
        let target_index = actor_index(&engine, "mireling");
        let mut events = Vec::new();

        apply_test_damage(&mut engine, target_index, 100, &mut events, |_| {})
            .expect("first lethal damage should resolve");
        let second = apply_test_damage(&mut engine, target_index, 1, &mut events, |_| {})
            .expect("dead-actor damage should not retrigger defeat");

        assert_eq!(second.hp_before, 0);
        assert_eq!(second.hp_after, 0);
        assert_eq!(second.applied, 0);
        assert!(!second.defeated);
        assert_eq!(
            events
                .iter()
                .filter(|event| {
                    matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == "mireling")
                })
                .count(),
            1
        );
    }

    #[test]
    fn nonfire_player_lethal_damage_creates_a_ghost_and_corpse_without_recovery() {
        let mut engine = engine_from("death_corpse");
        let player_index = actor_index(&engine, "player");
        let death_position = engine.world.actors[player_index].location.position;
        let mut events = Vec::new();

        let outcome = apply_test_damage(&mut engine, player_index, 100, &mut events, |_| {})
            .expect("player defeat should resolve");

        let damaged = event_position(
            &events,
            |event| matches!(event, Event::EffectDamaged { actor_id, hp: 0, .. } if actor_id == "player"),
        );
        let defeated = event_position(
            &events,
            |event| matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == "player"),
        );
        let created = event_position(
            &events,
            |event| matches!(event, Event::CorpseCreated { origin_actor_id, .. } if origin_actor_id == "player"),
        );
        assert!(damaged < defeated);
        assert!(defeated < created);
        assert!(outcome.defeated);

        let player = &engine.world.actors[player_index];
        assert!(!player.is_alive());
        assert_eq!(player.hp, 0);
        assert_eq!(
            player
                .character
                .as_ref()
                .expect("character-backed player")
                .resources
                .hp,
            player.hp
        );
        assert_eq!(player.location.level, "room_0");
        assert_eq!(player.location.position, death_position);
        assert!(player.carried.items.is_empty());
        assert!(player.active_effects.is_empty());
        assert_eq!(engine.world.ground_items.len(), 1);
        assert_eq!(engine.world.ground_items[0].item_instance_id, "oak_club");
        assert_eq!(engine.world.corpses.len(), 1);
        let corpse = engine.world.corpses.values().next().unwrap();
        assert_eq!(corpse.origin_actor_id, "player");
        assert_eq!(
            corpse.contents[&crate::model::CarriedPosition::SackItem1],
            "flint"
        );
        assert_eq!(corpse.gold, 2);
    }

    #[test]
    fn fire_player_lethal_damage_drops_everything_and_requests_gods_resurrection() {
        let mut engine = engine_from("death_corpse");
        let player_index = actor_index(&engine, "player");
        let (actor_id, actor_name, location) = {
            let actor = &engine.world.actors[player_index];
            (actor.id.clone(), actor.name.clone(), actor.location.clone())
        };
        let mut events = Vec::new();

        engine
            .apply_damage_and_resolve_defeat(
                player_index,
                100,
                DefeatContext {
                    cause: crate::model::DeathCause::Fire,
                    credited_actor_id: Some("brute".into()),
                    direct_social_actor_id: Some("brute".into()),
                    spell_damage_credit: None,
                    hostile_authority: None,
                },
                &mut events,
                move |outcome| Event::EffectDamaged {
                    actor_id,
                    actor: actor_name,
                    location,
                    instance_id: "fire:test".to_string(),
                    effect_id: "fire_test".to_string(),
                    kind: "fire".to_string(),
                    tags: vec!["fire".to_string()],
                    damage: outcome.applied,
                    hp: outcome.hp_after,
                },
                |_| {},
            )
            .expect("fire defeat should resolve");

        let player = &engine.world.actors[player_index];
        assert!(matches!(
            player.life_state,
            crate::model::ActorLifeState::AwaitingResurrection {
                cause: crate::model::DeathCause::Fire,
                ..
            }
        ));
        assert!(engine.world.corpses.is_empty());
        assert_eq!(engine.world.next_corpse_sequence, 1);
        assert_eq!(engine.world.ground_items.len(), 2);
        assert_eq!(engine.world.ground_gold.len(), 1);
        assert_eq!(engine.world.ground_gold.values().next().unwrap().amount, 2);
        assert_eq!(engine.world.next_gold_sequence, 2);
        assert!(events.iter().any(|event| matches!(
            event,
            Event::ActorDefeated {
                actor_id,
                cause: crate::model::DeathCause::Fire,
                credited_actor_id: Some(credited),
                loot_claim: Some(claim),
                ..
            } if actor_id == "player"
                && credited == "brute"
                && claim.basis == crate::model::LootClaimBasis::CharacterDeathPile
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            Event::ResurrectionRequested {
                actor_id,
                cause: crate::model::DeathCause::Fire,
                method: crate::model::ResurrectionMethod::Gods,
                ..
            } if actor_id == "player"
        )));

        let before_invalid = engine.world.clone();
        let invalid = crate::model::ResurrectionRequest {
            actor_id: "player".into(),
            corpse_id: None,
            method: crate::model::ResurrectionMethod::Priest,
            destination: crate::model::WorldPosition::new("realm_0", "room_0", (2, 1).into()),
            current_hp: 4,
            current_stamina: 5,
        };
        assert!(engine.resurrect(invalid).is_err());
        assert_eq!(engine.world, before_invalid);

        let valid = crate::model::ResurrectionRequest {
            actor_id: "player".into(),
            corpse_id: None,
            method: crate::model::ResurrectionMethod::Gods,
            destination: crate::model::WorldPosition::new("realm_0", "room_0", (2, 1).into()),
            current_hp: 4,
            current_stamina: 5,
        };
        let resurrection = engine.resurrect(valid).expect("gods resurrection");
        assert_eq!(
            engine.world.actors[player_index].life_state,
            crate::model::ActorLifeState::Alive
        );
        assert_eq!(engine.world.actors[player_index].carried.gold.sack, 0);
        assert!(engine.world.actors[player_index].carried.items.is_empty());
        assert_eq!(engine.world.ground_items.len(), 2);
        assert_eq!(engine.world.ground_gold.len(), 1);
        assert!(resurrection.iter().any(|event| matches!(
            event,
            Event::ActorResurrected {
                corpse_id: None,
                method: crate::model::ResurrectionMethod::Gods,
                ..
            }
        )));
    }

    #[test]
    fn no_remains_monster_drops_all_items_and_gold_without_allocating_a_corpse() {
        let (mut catalog, profile, template, seed) =
            crate::engine::setup::test_parts("death_corpse");
        let definition_id = seed
            .actors
            .iter()
            .find(|actor| actor.id == "scavenger")
            .expect("fixture scavenger")
            .actor_definition_id
            .clone();
        catalog
            .actor_definitions
            .values_mut()
            .find(|definition| definition.id == definition_id)
            .expect("fixture scavenger definition")
            .death
            .remains = crate::model::CorpseDisposition::None;
        let mut engine =
            crate::engine::setup::test_engine_from_parts(catalog, profile, template, seed);
        let target_index = actor_index(&engine, "scavenger");
        let mut events = Vec::new();

        apply_test_damage(&mut engine, target_index, 100, &mut events, |_| {})
            .expect("no-remains defeat should resolve");

        assert!(engine.world.corpses.is_empty());
        assert_eq!(engine.world.next_corpse_sequence, 1);
        assert_eq!(engine.world.ground_items.len(), 2);
        assert_eq!(engine.world.ground_gold.len(), 1);
        assert_eq!(engine.world.ground_gold.values().next().unwrap().amount, 3);
        assert!(matches!(
            engine.world.actors[target_index].life_state,
            crate::model::ActorLifeState::Dead
        ));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::CorpseCreated { .. }))
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::ResurrectionRequested { .. }))
        );
    }
}
