use super::*;

#[test]
fn defeat_removes_active_effects() {
    let mut engine = status_engine_with_seed(
        |value| {
            equip_one_handed_ranged_weapon(value, "player", "player_test_bow", 3);
        },
        1_010_580_540,
    );
    let seeded = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .active_effects
        .first()
        .expect("seeded effect")
        .clone();

    {
        let world = engine.world_mut();
        let player_index = world
            .actors
            .iter()
            .position(|actor| actor.id == "player")
            .expect("player index");
        let watcher_index = world
            .actors
            .iter()
            .position(|actor| actor.id == "watcher")
            .expect("watcher index");
        world.actors[player_index].active_effects.clear();
        world.actors[player_index].stats.attack = 20;
        world.actors[watcher_index].hp = 1;
        world.actors[watcher_index].active_effects = vec![seeded];
    }

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "watcher".into(),
            },
        )
        .expect("attack should kill watcher");
    assert!(events.iter().any(|event| {
        matches!(event, Event::EffectRemoved {
            actor_id,
            instance_id,
            reason,
            ..
        } if actor_id == "watcher" && instance_id == "rooted_1" && reason == "defeat")
    }));
    let watcher = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "watcher")
        .expect("watcher");
    assert!(watcher.active_effects.is_empty());
}

#[test]
fn poison_tick_can_defeat_actor_and_remove_active_effects() {
    let mut engine = status_engine_with(|parts| {
        parts.actors_mut()[0]["active_effects"] = serde_json::json!([]);
        parts.actor_definition_mut(1)["stats"]["hp"] = serde_json::json!(2);
        parts.actors_mut()[1]["active_effects"] = serde_json::json!([
        {
            "instance_id": "venom_1",
            "effect_id": "venom",
            "source": {"kind": "fixture", "id": "status_effects"},
            "kind": "poison",
            "tags": ["poison"],
            "potency": 2,
            "remaining_rounds": 2,
            "stacking": "replace_same_kind",
            "start_delay_rounds": 0,
            "tick_interval_rounds": 1,
            "suppresses_action": false,
            "resistance_boosts": []
        },
        {
            "instance_id": "ward_1",
            "effect_id": "ward",
            "source": {"kind": "fixture", "id": "status_effects"},
            "kind": "protection",
            "tags": ["ward"],
            "potency": 1,
            "remaining_rounds": 2,
            "stacking": "replace_same_kind",
            "start_delay_rounds": 0,
            "tick_interval_rounds": 1,
            "suppresses_action": false,
            "resistance_boosts": []
        }
        ]);
    });

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("wait should advance poison");

    let ticked = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::EffectTicked {
                    actor_id,
                    instance_id,
                    ..
                } if actor_id == "watcher" && instance_id == "venom_1"
            )
        })
        .expect("venom tick event");
    let damaged = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::EffectDamaged {
                    actor_id,
                    instance_id,
                    hp: 0,
                    ..
                } if actor_id == "watcher" && instance_id == "venom_1"
            )
        })
        .expect("venom damage event");
    let venom_removed = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::EffectRemoved {
                    actor_id,
                    instance_id,
                    reason,
                    ..
                } if actor_id == "watcher"
                    && instance_id == "venom_1"
                    && reason == "defeat"
            )
        })
        .expect("venom defeat removal");
    let ward_removed = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::EffectRemoved {
                    actor_id,
                    instance_id,
                    reason,
                    ..
                } if actor_id == "watcher"
                    && instance_id == "ward_1"
                    && reason == "defeat"
            )
        })
        .expect("ward defeat removal");
    let died = events
        .iter()
        .position(
            |event| matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == "watcher"),
        )
        .expect("watcher death");
    assert!(ticked < damaged);
    assert!(damaged < venom_removed);
    assert!(venom_removed < ward_removed);
    assert!(ward_removed < died);
    assert_eq!(
        events
            .iter()
            .filter(|event| {
                matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == "watcher")
            })
            .count(),
        1
    );

    assert!(events.iter().any(|event| matches!(
        event,
        Event::EffectDamaged {
            actor_id,
            effect_id,
            tags,
            damage,
            hp,
            ..
        } if actor_id == "watcher"
            && effect_id == "venom"
            && tags.iter().any(|tag| tag == "poison")
            && *damage == 2
            && *hp == 0
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::EffectRemoved {
            actor_id,
            instance_id,
            reason,
            ..
        } if actor_id == "watcher" && instance_id == "venom_1" && reason == "defeat"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::EffectRemoved {
            actor_id,
            instance_id,
            reason,
            ..
        } if actor_id == "watcher" && instance_id == "ward_1" && reason == "defeat"
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::EffectTicked {
            actor_id,
            instance_id,
            ..
        } if actor_id == "watcher" && instance_id == "ward_1"
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::EffectExpired {
            actor_id,
            instance_id,
            ..
        } if actor_id == "watcher" && instance_id == "ward_1"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ActorDefeated {
            actor_id,
            cause: tme_rules::DeathCause::Poison,
            credited_actor_id: None,
            ..
        } if actor_id == "watcher"
    )));
    let watcher = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "watcher")
        .expect("watcher");
    assert_eq!(watcher.hp, 0);
    assert!(!watcher.is_alive());
    assert!(watcher.active_effects.is_empty());
}
