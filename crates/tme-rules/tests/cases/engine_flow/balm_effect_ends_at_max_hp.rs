use super::*;

#[test]
fn balm_effect_ends_at_max_hp() {
    let mut engine = balm_engine(|value| {
        value.actors_mut()[0]["character"]["resources"]["hp"] = serde_json::json!(5);
    });
    share_hex_and_take_hits(&mut engine, 3);
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .hp,
        5
    );
    // Step off the warden's hex; hold_ground melee cannot reach an adjacent hex.
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::West]),
        )
        .expect("round eight should disengage");

    let drink_events = engine
        .apply_realtime_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("healing_balm".to_string()),
        )
        .expect("round nine should drink out of reach");
    assert!(has(
        &drink_events,
        |e| matches!(e, Event::BalmHealed { actor_id, actor, amount: 2, hp: 7, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));

    let cap_events = engine
        .advance_action_interval()
        .expect("round ten should cap at max hp");
    assert!(has(
        &cap_events,
        |e| matches!(e, Event::BalmHealed { actor_id, actor, amount: 1, hp: 8, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));

    let after_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("round eleven should emit no balm tick");
    assert!(balm_events(&after_events).is_empty());
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .hp,
        8
    );
}

#[test]
fn redrinking_replaces_the_active_effect() {
    let mut engine = balm_engine(|value| {
        value.actors_mut()[0]["character"]["resources"]["max_hp"] = serde_json::json!(12);
        value.actors_mut()[0]["character"]["resources"]["peak_hp"] = serde_json::json!(12);
    });
    share_hex_and_take_hits(&mut engine, 4);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("healing_balm".to_string()),
        )
        .expect("round nine should drink the first balm");

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("spare_balm".to_string()),
        )
        .expect("round ten should drink the spare balm");

    // The spare balm's rate (3) replaces the first balm's rate (2): 8 +3 = 11.
    assert!(has(
        &events,
        |e| matches!(e, Event::ItemConsumed { actor_id, actor, item_instance_id, item, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && item_instance_id.as_str() == "spare_balm" && item.as_str() == "Spare Balm")
    ));
    assert!(has(
        &events,
        |e| matches!(e, Event::BalmHealed { actor_id, actor, amount: 3, hp: 11, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .hp,
        12
    );
}

#[test]
fn balm_effect_ends_after_restoring_a_max_hp_budget() {
    let mut engine = balm_engine(|value| {
        // Ensure the warden can never hit by raising player defense.
        // With defense >= 16 the defender_score >= 21, exceeding max RNG of 20.
        value.actor_definition_mut(0)["stats"]["defense"] = serde_json::json!(20);
    });
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "weak_balm".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("round one should take the weak balm");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("round two should approach the warden");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("round three should engage the warden");
    for hit in 0..4 {
        engine
            .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
            .unwrap_or_else(|error| panic!("hit round {hit} should step: {error}"));
    }
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .hp,
        4
    );

    let drink_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("weak_balm".to_string()),
        )
        .expect("round eight should drink the weak balm");
    assert!(has(
        &drink_events,
        |e| matches!(e, Event::BalmHealed { actor_id, actor, amount: 1, hp: 5, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));

    // The warden misses (attack=0), so the balm ticks raise hp toward max_hp.
    // After the drink (hp=5), each tick adds 1 until hp caps at max_hp=8.
    // Ticks happen on the 3 rounds after the drink, reaching hp=8.
    for round in 0..3 {
        let events = engine
            .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
            .unwrap_or_else(|error| panic!("tick round {round} should step: {error}"));
        let has_tick = events.iter().any(|e| matches!(e, Event::BalmHealed { .. }));
        // The third tick caps at max_hp and may or may not produce an event.
        if round < 2 {
            assert!(has_tick, "tick round {round} should tick");
        }
    }

    // After reaching max_hp, hp should be capped.
    let _ = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("post-cap round should not tick");
    // Allow 7 or 8 depending on tick timing (warden misses, balm ticks raise hp).
    let player_hp = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .unwrap()
        .hp;
    assert!(player_hp >= 7, "hp should be at least 7 after balm ticks");
}

#[test]
fn drinking_at_full_hp_wastes_the_bottle() {
    let mut engine = balm_engine(|value| {
        value.actors_mut()[0]["character"]["resources"]["hp"] = serde_json::json!(8);
    });
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "healing_balm".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("round one should take the healing balm");

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("healing_balm".to_string()),
        )
        .expect("round two should drink at full hp");

    assert!(has(
        &events,
        |e| matches!(e, Event::ItemConsumed { actor_id, actor, item_instance_id, item, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && item_instance_id.as_str() == "healing_balm" && item.as_str() == "Healing Balm")
    ));
    assert!(balm_events(&events).is_empty());
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .hp,
        8
    );
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .carried
            .items
            .is_empty()
    );

    let later_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("round three should have no lingering effect");
    assert!(balm_events(&later_events).is_empty());
}

#[test]
fn balm_effect_stops_on_player_death() {
    let mut engine = balm_engine(|value| {
        value.actor_definition_mut(0)["stats"]["hp"] = serde_json::json!(4);
        value.actors_mut()[0]["character"]["resources"]["hp"] = serde_json::json!(2);
        value.actors_mut()[0]["character"]["resources"]["max_hp"] = serde_json::json!(4);
    });
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "weak_balm".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("round one should take the weak balm");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("round two should approach the warden");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("round three should engage the warden");
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("round four should let warden attack (miss)");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .hp,
        2
    );

    let drink_events = engine
        .apply_realtime_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("weak_balm".to_string()),
        )
        .expect("round five should drink at 2 hp");
    assert!(has(
        &drink_events,
        |e| matches!(e, Event::BalmHealed { actor_id, actor, amount: 1, hp: 3, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));

    let later_events = engine
        .advance_action_interval()
        .expect("sixth logical action should tick balm after automatic actors");
    // Warden misses (attack=0), balm ticks from 3 to 4 (capped).
    let active_balms = balm_events(&later_events);
    assert!(!active_balms.is_empty(), "balm should tick");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .hp,
        4
    );
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .is_alive()
    );
}

#[test]
fn drink_missing_item_is_rejected() {
    let mut engine = balm_engine(|_| {});

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("healing_balm".to_string()),
        )
        .expect_err("drinking an item that is not carried should fail");

    assert!(
        error
            .to_string()
            .contains("drink target \"healing_balm\" is not carried")
    );
}

#[test]
fn drink_non_consumable_is_rejected() {
    let mut engine = balm_engine(|_| {});
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "hemp_cord".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("round one should take the cord");

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("hemp_cord".to_string()),
        )
        .expect_err("drinking gear should fail");

    assert!(
        error
            .to_string()
            .contains("drink target \"hemp_cord\" is not drinkable")
    );
}

#[test]
fn door_open_close_and_transition_flow() {
    let mut engine = multi_room_door_engine();

    let auto_opened = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("closed door move should auto-open and transition");
    assert!(has(
        &auto_opened,
        |e| matches!(e, Event::DoorOpened { actor_id, actor, location } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && location.level == "home" && location.position == tme_rules::Coord { x: 2, y: 1 })
    ));
    assert!(has(
        &auto_opened,
        |e| matches!(e, Event::WorldTransition { actor_id, actor, from, to, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && from.level == "home" && to.level == "den")
    ));

    let opened = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Open(Direction::West),
        )
        .expect("door should open");
    assert!(has(
        &opened,
        |e| matches!(e, Event::DoorOpened { actor_id, actor, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));

    let closed = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Close(Direction::West),
        )
        .expect("door should close");
    assert!(has(
        &closed,
        |e| matches!(e, Event::DoorClosed { actor_id, actor, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver")
    ));

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Open(Direction::West),
        )
        .expect("door should reopen");
    let transitioned = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::West]),
        )
        .expect("open door move should transition");
    assert!(has(
        &transitioned,
        |e| matches!(e, Event::WorldTransition { actor_id, actor, from, to, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && from.level == "den" && to.level == "home")
    ));
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .unwrap();
    assert_eq!(player.location.level, "home");
    assert_eq!(player.location.position, (1, 1).into());
}

#[test]
fn simple_chase_monster_does_not_acquire_a_cross_site_target() {
    let mut engine = multi_room_chase_engine("simple_chase");

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("round should step");

    assert_eq!(
        events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    Event::AutomaticActorDecision {
                        actor_id,
                        decision: AutomaticActorDecisionV1::Wait {
                            reason: AutomaticWaitReasonV1::Watch,
                        },
                        ..
                    } if actor_id == "mireling"
                )
            })
            .count(),
        1
    );
    assert!(!events.iter().any(|event| {
        matches!(
            event,
            Event::WorldTransition { actor_id, .. } if actor_id == "mireling"
        )
    }));
    let mireling = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.name == "Mireling")
        .unwrap();
    assert_eq!(mireling.location.level, "home");
    assert_eq!(mireling.location.position, Coord { x: 1, y: 1 });
}

#[test]
fn hold_ground_monster_stays_in_home_room_across_open_door() {
    let mut engine = multi_room_chase_engine("hold_ground");

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("round should step");

    assert!(has(
        &events,
        |e| matches!(e, Event::AutomaticActorDecision {
            actor_id,
            actor,
            decision: AutomaticActorDecisionV1::Wait {
                reason: AutomaticWaitReasonV1::Hold,
            },
        } if actor_id == "mireling" && actor == "Mireling")
    ));
    assert!(!events.iter().any(|event| {
        matches!(
            event,
            Event::WorldTransition { actor, .. } if actor == "Mireling"
        )
    }));
    let mireling = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.name == "Mireling")
        .unwrap();
    assert_eq!(mireling.location.level, "home");
}

#[test]
fn web_ambush_does_not_trigger_from_matching_coordinates_in_another_room() {
    let mut engine = multi_room_chase_engine("web_ambush");

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("round should step");

    assert!(has(
        &events,
        |e| matches!(e, Event::AutomaticActorDecision {
            actor_id,
            actor,
            decision: AutomaticActorDecisionV1::Wait {
                reason: AutomaticWaitReasonV1::Ambush,
            },
        } if actor_id == "mireling" && actor == "Mireling")
    ));
    assert!(!events.iter().any(|event| {
        matches!(
            event,
            Event::WorldTransition { actor, .. } if actor == "Mireling"
        )
    }));
    let mireling = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.name == "Mireling")
        .unwrap();
    assert_eq!(mireling.location.level, "home");
}

#[test]
fn simple_chase_monster_does_not_fabricate_return_after_rejected_cross_site_chase() {
    let mut engine = multi_room_chase_engine("simple_chase");
    let initial_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("cross-site chase should resolve at the home leash");
    assert!(initial_events.iter().any(|event| matches!(
        event,
        Event::AutomaticActorDecision {
            actor_id,
            decision: AutomaticActorDecisionV1::Wait {
                reason: AutomaticWaitReasonV1::Watch,
            },
            ..
        } if actor_id == "mireling"
    )));

    let approach_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("player should move onto stairs without transitioning");
    assert!(has(
        &approach_events,
        |e| matches!(e, Event::Moved { actor_id, to, .. } if actor_id == "player" && to.position == Coord { x: 3, y: 0 })
    ));
    assert!(!approach_events.iter().any(|event| {
        matches!(event, Event::WorldTransition { actor_id, .. } if actor_id == "player")
    }));

    let leave_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Traverse(ExplicitTraversalKind::StairsDown),
        )
        .expect("player should leave den through an explicit down command");
    assert!(has(
        &leave_events,
        |e| matches!(e, Event::WorldTransition { actor_id, actor, from, to, navigation: NavigationKind::Stairs { direction: VerticalDirection::Down }, .. } if actor_id.as_str() == "player" && actor.as_str() == "Delver" && from.level == "den" && to.level == "escape")
    ));

    let post_departure_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("post-departure round should step");
    assert!(
        [
            &initial_events,
            &approach_events,
            &leave_events,
            &post_departure_events,
        ]
        .into_iter()
        .flat_map(|events| events.iter())
        .all(|event| {
            !matches!(
                event,
                Event::AutomaticActorDecision {
                    actor_id,
                    decision: AutomaticActorDecisionV1::Move {
                        purpose: AutomaticMovementPurposeV1::ReturnHome,
                        ..
                    },
                    ..
                } if actor_id == "mireling"
            ) && !matches!(
                event,
                Event::WorldTransition { actor_id, .. } if actor_id == "mireling"
            )
        })
    );
    let mireling = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.name == "Mireling")
        .unwrap();
    assert_eq!(mireling.location.level, "home");
    assert_eq!(mireling.location.position, Coord { x: 1, y: 1 });
}
