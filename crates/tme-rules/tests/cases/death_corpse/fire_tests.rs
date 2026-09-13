use super::*;
use tme_rules::model::{ActiveEffectSource, TileEffectState};

fn hazard_engine(alignment: &str, hazard: &str) -> Engine {
    let mut engine = super::control_tests::return_engine(alignment, false);
    for actor in engine.world_mut().actors.iter_mut().skip(1) {
        actor.location.position = (4, 1).into();
        actor.timing.ready_at = LogicalTime::from_millis(1_000_000);
    }
    let location = engine.world().actors[0].location.clone();
    let now = engine.world().timing.now;
    engine.world_mut().tile_effects.push(TileEffectState {
        instance_id: "fire:proof".into(),
        effect_id: "fire_proof".into(),
        source: ActiveEffectSource {
            kind: "test".into(),
            id: "fire_proof".into(),
        },
        source_actor_id: None,
        hostile_authority: None,
        location,
        kind: "terrain_overlay".into(),
        tags: vec![hazard.into()],
        potency: 100,
        remaining_rounds: Some(3),
        passability: None,
        sight: None,
        hazard: Some(hazard.into()),
        move_cost: None,
        tick_interval_rounds: 1,
        last_ticked_at: now,
    });
    engine
}

#[test]
fn lethal_fire_returns_at_the_damage_instant_with_drops_and_durable_cooldown() {
    for (alignment, destination) in [("lawful", (2, 1)), ("neutral", (3, 1))] {
        let mut engine = hazard_engine(alignment, "fire");
        let actor = engine.world().actors[0].clone();
        let death_at = engine.world().timing.now.saturating_add_millis(3_000);
        let events = engine.advance_to(death_at).unwrap().events;
        let returned = engine.world().actor(&actor.id).unwrap();
        assert!(returned.is_alive());
        assert_eq!(returned.character_id, actor.character_id);
        assert_eq!(returned.location.position, destination.into());
        assert_eq!(returned.hp, actor.max_hp() - 1);
        assert_eq!(returned.stamina, actor.max_stamina() - 1);
        assert_eq!(
            returned.timing.ready_at,
            death_at.saturating_add_millis(3_000)
        );
        assert_eq!(engine.world().timing.now, death_at);
        assert!(engine.world().corpses.is_empty());
        assert_eq!(engine.world().next_corpse_sequence, 1);
        for item in ["oak_club", "flint"] {
            assert!(
                matches!(engine.item_location(item).unwrap(), ItemLocation::Ground { position, .. }
                if position == actor.location)
            );
        }
        assert_eq!(
            engine
                .world()
                .ground_gold
                .values()
                .map(|pile| pile.amount)
                .sum::<i64>(),
            2
        );
        assert!(
            engine
                .world()
                .ground_gold
                .values()
                .all(|pile| pile.location == actor.location)
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, Event::ActorResurrected { .. }))
                .count(),
            1
        );
        assert!(!events.iter().any(|e| matches!(
            e,
            Event::CorpseCreated { .. }
                | Event::ActorLifeStateChanged {
                    to: ActorLifeState::Ghost { .. },
                    ..
                }
        )));
        let defeated = event_position(&events, |e| {
            matches!(
                e,
                Event::ActorDefeated {
                    cause: DeathCause::Fire,
                    ..
                }
            )
        });
        let returned_event =
            event_position(&events, |e| matches!(e, Event::ActorResurrected { .. }));
        assert!(defeated < returned_event);
        assert!(
            events
                .iter()
                .enumerate()
                .filter(|(_, e)| matches!(
                    e,
                    Event::ItemRelocated { .. } | Event::GoldRelocated { .. }
                ))
                .all(|(i, _)| i < returned_event)
        );
        let checkpoint = engine.export_checkpoint().unwrap();
        let mut recovered =
            Engine::hydrate_checkpoint(engine.definition().clone(), &checkpoint).unwrap();
        for candidate in [&mut engine, &mut recovered] {
            assert!(
                candidate
                    .apply_realtime_actor_intent(&actor.id, PlayerIntent::Wait)
                    .is_err()
            );
            assert!(
                candidate
                    .apply_realtime_actor_intent(&actor.id, PlayerIntent::RequestResurrection)
                    .is_err()
            );
            assert_eq!(candidate.export_checkpoint().unwrap(), checkpoint);
            let next = candidate
                .advance_to(death_at.saturating_add_millis(3_000))
                .unwrap();
            assert!(!next.events.iter().any(|e| matches!(
                e,
                Event::ActorDefeated { .. } | Event::ActorResurrected { .. }
            )));
            candidate
                .apply_realtime_actor_intent(&actor.id, PlayerIntent::Wait)
                .unwrap();
        }
        assert_eq!(
            engine.export_checkpoint().unwrap(),
            recovered.export_checkpoint().unwrap()
        );
    }
}

#[test]
fn fire_return_does_not_borrow_routes_for_other_alignments_or_causes() {
    for alignment in ["evil", "chaotic"] {
        let mut engine = hazard_engine(alignment, "fire");
        let events = engine
            .advance_to(engine.world().timing.now.saturating_add_millis(3_000))
            .unwrap();
        assert!(matches!(
            engine.world().actors[0].life_state,
            ActorLifeState::AwaitingResurrection {
                cause: DeathCause::Fire,
                ..
            }
        ));
        assert!(
            !events
                .events
                .iter()
                .any(|e| matches!(e, Event::ActorResurrected { .. }))
        );
    }
    let mut engine = hazard_engine("lawful", "acid");
    engine
        .advance_to(engine.world().timing.now.saturating_add_millis(3_000))
        .unwrap();
    assert!(matches!(
        engine.world().actors[0].life_state,
        ActorLifeState::Ghost { .. }
    ));
    engine
        .advance_to(engine.world().timing.now.saturating_add_millis(600_000))
        .unwrap();
    assert!(matches!(
        engine.world().actors[0].life_state,
        ActorLifeState::Ghost { .. }
    ));
}
