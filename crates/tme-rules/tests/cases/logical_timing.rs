use crate::support::content_parts::ContentParts;
use tme_rules::model::{
    ActiveEffectSource, ItemEnchantmentState, ItemOperationSource, PortalTransitionState,
    SummonedActorState, TileEffectState,
};
use tme_rules::{
    ActorKind, AutomaticActorDecisionV1, Coord, Direction, Engine, Event, LogicalTime,
    PlayerIntent, SpellTarget, WorldPosition,
};

fn first_room_engine() -> Engine {
    ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("engine should start")
}

fn timing_effects_engine() -> Engine {
    let mut parts = ContentParts::tracked("balm_cache", "profile/balm_cache");
    let player = &mut parts.actors_mut()[0];
    player["character"]["resources"]["hp"] = serde_json::json!(2);
    player["active_effects"] = serde_json::json!([
        {
            "instance_id": "ward_1",
            "effect_id": "steady_ward",
            "source": {"kind": "fixture", "id": "timing_effects"},
            "kind": "protection",
            "tags": ["ward"],
            "potency": 1,
            "remaining_rounds": 3,
            "stacking": "replace_same_kind",
            "start_delay_rounds": 0,
            "tick_interval_rounds": 1,
            "suppresses_action": false,
            "resistance_boosts": []
        }
    ]);
    parts.engine(7).expect("engine should start")
}

fn monster_ready_pairs(events: &[Event]) -> Vec<(String, LogicalTime)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::ActorReady {
                actor_id,
                kind: ActorKind::Monster,
                logical_time,
                ..
            } => Some((actor_id.to_string(), *logical_time)),
            _ => None,
        })
        .collect()
}

#[test]
fn setup_assigns_first_ready_time_and_stable_authored_order() {
    let engine = first_room_engine();

    assert_eq!(engine.world().timing.now, LogicalTime::FIRST);
    assert_eq!(engine.world().timing.next_tie_break_order, 2);
    assert_eq!(engine.world().actors[0].timing.ready_at, LogicalTime::FIRST);
    assert_eq!(engine.world().actors[0].timing.tie_break_order, 0);
    assert_eq!(
        engine.world().actors[1].timing.ready_at,
        LogicalTime::FIRST.saturating_add_rounds(1)
    );
    assert_eq!(engine.world().actors[1].timing.tie_break_order, 1);
}

#[test]
fn standard_action_schedules_actor_and_drains_automatic_actor() {
    let mut engine = first_room_engine();

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("logical action should resolve");

    assert_eq!(
        events[0],
        Event::ActorReady {
            actor_id: "player".into(),
            actor: "Delver".to_string(),
            kind: ActorKind::Player,
            logical_time: LogicalTime::FIRST,
        }
    );
    assert_eq!(
        events[1],
        Event::PlayerIntent {
            actor_id: "player".into(),
            actor: "Delver".to_string(),
            logical_time: LogicalTime::FIRST,
            intent: "walk east".to_string(),
        }
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ActorReadinessScheduled {
            actor_id,
            cost_units: 1,
            ready_at,
            ..
        } if actor_id == "player" && *ready_at == LogicalTime::new(2)
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ActorReady {
            actor_id,
            kind: ActorKind::Monster,
            logical_time,
            ..
        } if actor_id == "mireling" && *logical_time == LogicalTime::new(2)
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::LogicalTimeAdvanced { from, to }
            if *from == LogicalTime::FIRST && *to == LogicalTime::new(2)
    )));
    assert_eq!(engine.world().timing.now, LogicalTime::new(2));
}

#[test]
fn inspect_and_show_sack_are_repeatable_free_actions() {
    for intent in [PlayerIntent::Inspect, PlayerIntent::ShowSack] {
        let mut engine = first_room_engine();
        let mut control = engine.clone();
        let before = engine.snapshot();
        let events = engine
            .apply_actor_intent(&tme_rules::ActorId::from("player"), intent.clone())
            .expect("free action should resolve");

        assert_eq!(engine.world().timing.now, LogicalTime::FIRST);
        assert_eq!(engine.world().actors[0].timing.ready_at, LogicalTime::FIRST);
        assert_eq!(
            engine.world().actors[1].timing.ready_at,
            LogicalTime::FIRST.saturating_add_rounds(1)
        );
        assert!(!events.iter().any(|event| matches!(
            event,
            Event::AutomaticActorDecision { .. } | Event::LogicalTimeAdvanced { .. }
        )));
        assert!(!events.iter().any(|event| matches!(
            event,
            Event::ActorReadinessScheduled {
                actor_id,
                cost_units: 0,
                ready_at,
                ..
            } if actor_id == "player" && *ready_at == LogicalTime::FIRST
        )));

        engine
            .apply_actor_intent(&tme_rules::ActorId::from("player"), intent)
            .expect("the same free action should remain ready");
        let after = engine.snapshot();
        assert_eq!(before, after);

        let actual_setup = engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
            )
            .expect("setup after free reads should resolve");
        let control_setup = control
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
            )
            .expect("control setup should resolve");
        assert_eq!(actual_setup, control_setup);

        let actual_attack = engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::PhysicalAttack {
                    authorization: tme_rules::HostilityAuthorization::Safe,
                    mode: tme_rules::PhysicalAttackMode::Fight,
                    target_actor_id: "mireling".into(),
                },
            )
            .expect("attack after free reads should resolve");
        let control_attack = control
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::PhysicalAttack {
                    authorization: tme_rules::HostilityAuthorization::Safe,
                    mode: tme_rules::PhysicalAttackMode::Fight,
                    target_actor_id: "mireling".into(),
                },
            )
            .expect("control attack should resolve");
        assert_eq!(actual_attack, control_attack);
        assert_eq!(engine.snapshot(), control.snapshot());
    }
}

#[test]
fn ready_monsters_use_authored_order_instead_of_actor_id_order() {
    let mut engine = ContentParts::tracked("kobold_warren", "profile/kobold_warren")
        .engine(7)
        .expect("engine should start");

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("logical action should resolve");
    let monster_order = events
        .iter()
        .filter_map(|event| match event {
            Event::ActorReady {
                actor_id,
                kind: ActorKind::Monster,
                ..
            } => Some(actor_id.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        monster_order,
        ["kobold_scrounger", "kobold_skulker", "kobold_runt"]
    );
}

#[test]
fn different_actor_readiness_values_interleave_independently() {
    let mut engine = ContentParts::tracked("kobold_warren", "profile/kobold_warren")
        .engine(7)
        .expect("engine should start");
    {
        let actors = &mut engine.world_mut().actors;
        actors[1].timing.ready_at = LogicalTime::new(3);
        actors[2].timing.ready_at = LogicalTime::FIRST;
        actors[3].timing.ready_at = LogicalTime::new(2);
    }

    let at_one = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("time one should resolve");
    assert_eq!(
        monster_ready_pairs(&at_one.events),
        [
            ("kobold_skulker".to_string(), LogicalTime::FIRST),
            ("kobold_skulker".to_string(), LogicalTime::new(2)),
            ("kobold_runt".to_string(), LogicalTime::new(2)),
        ]
    );

    let at_two = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("time two should resolve");
    assert_eq!(
        monster_ready_pairs(&at_two.events),
        [
            ("kobold_scrounger".to_string(), LogicalTime::new(3)),
            ("kobold_skulker".to_string(), LogicalTime::new(3)),
            ("kobold_runt".to_string(), LogicalTime::new(3)),
        ]
    );

    let at_three = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("time three should resolve");
    assert_eq!(
        monster_ready_pairs(&at_three.events),
        [
            ("kobold_scrounger".to_string(), LogicalTime::new(4)),
            ("kobold_skulker".to_string(), LogicalTime::new(4)),
            ("kobold_runt".to_string(), LogicalTime::new(4)),
        ]
    );
}

#[test]
fn invalid_addressed_actor_states_are_atomic_and_ready_peers_do_not_block() {
    let mut unknown = first_room_engine();
    let before = unknown.snapshot();
    let error = unknown
        .apply_actor_intent(&tme_rules::ActorId::from("missing"), PlayerIntent::Wait)
        .expect_err("unknown actor should fail");
    assert!(error.to_string().contains("unknown actor"));
    assert_eq!(unknown.snapshot(), before);

    let mut dead = first_room_engine();
    dead.world_mut().actors[0].life_state = tme_rules::ActorLifeState::Dead;
    let before = dead.snapshot();
    let error = dead
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect_err("dead actor should fail");
    assert!(error.to_string().contains("actor death"));
    assert_eq!(dead.snapshot(), before);

    let mut not_ready = first_room_engine();
    not_ready.world_mut().actors[0].timing.ready_at = LogicalTime::new(2);
    let before = not_ready.snapshot();
    let error = not_ready
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect_err("future actor should fail");
    assert!(error.to_string().contains("not ready"));
    assert_eq!(not_ready.snapshot(), before);

    let mut not_next = first_room_engine();
    {
        let actors = &mut not_next.world_mut().actors;
        actors[0].timing.tie_break_order = 1;
        actors[1].kind = ActorKind::Player;
        actors[1].timing.ready_at = LogicalTime::FIRST;
        actors[1].timing.tie_break_order = 0;
    }
    let outcome = not_next
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("another ready controlled actor must not block this actor");
    assert!(outcome.events.iter().any(|event| matches!(
        event,
        Event::ActorReady { actor_id, kind: ActorKind::Player, .. }
            if actor_id == "player"
    )));
}

#[test]
fn ready_controlled_actors_act_independently() {
    let mut engine = ContentParts::tracked("character_sheet", "profile/character_sheet")
        .engine(7)
        .expect("character timing engine should start");
    {
        let actors = &mut engine.world_mut().actors;
        let mut peer_character = actors[0]
            .character
            .clone()
            .expect("timing player has a character sheet");
        peer_character.resources.hp = 100;
        peer_character.resources.max_hp = 100;
        peer_character.resources.peak_hp = peer_character.resources.peak_hp.max(100);
        actors[0].location.position = Coord { x: 3, y: 1 };
        actors[1].kind = ActorKind::Player;
        actors[1].timing.ready_at = LogicalTime::FIRST;
        actors[1].character_id = Some(tme_rules::CharacterId::new("character:timing:peer"));
        actors[1].character = Some(peer_character);
        actors[1].social.alignment_source = tme_rules::SocialAlignmentSource::Character {};
        actors[1].hp = 100;
        actors[1].stats.hp = 100;
    }
    let mut control = engine.clone();
    let before = engine.snapshot();

    let actual = engine
        .apply_realtime_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("ready controlled actor should act");
    let expected = control
        .apply_realtime_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("control attack should resolve");
    assert_eq!(actual, expected);
    assert_ne!(engine.snapshot(), before);
    assert_eq!(engine.snapshot(), control.snapshot());
    assert!(!actual.iter().any(|event| matches!(
        event,
        Event::ActorReady { actor_id, .. } if actor_id == "mireling"
    )));

    let error = engine
        .apply_realtime_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect_err("scheduled actor must wait for its own readiness");
    assert!(error.to_string().contains("not ready"));
    let peer = engine
        .apply_realtime_actor_intent(&tme_rules::ActorId::from("mireling"), PlayerIntent::Wait)
        .expect("independently ready peer should act");
    assert!(peer.events.iter().any(|event| matches!(
        event,
        Event::ActorReady { actor_id, kind: ActorKind::Player, .. }
            if actor_id == "mireling"
    )));
}

#[test]
fn every_successive_logical_boundary_runs_lifecycle_once() {
    let mut engine = timing_effects_engine();

    for from_value in 1..=3 {
        let events = engine
            .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
            .expect("standard action should complete one boundary");
        let from = LogicalTime::new(from_value);
        let to = LogicalTime::new(from_value + 1);
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::EffectTicked { .. }))
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| {
                    matches!(event, Event::LogicalTimeAdvanced { from: event_from, to: event_to }
                        if *event_from == from && *event_to == to)
                })
                .count(),
            1
        );
    }
    assert_eq!(engine.world().timing.now, LogicalTime::new(4));
}

#[test]
fn combat_spell_and_monster_ability_gates_share_logical_time() {
    let mut combat = first_room_engine();
    let movement_events = combat
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("movement should resolve");
    assert!(movement_events.iter().any(|event| matches!(
        event,
        Event::AttackMissed { attacker_id, .. } if attacker_id == "mireling"
    )));
    let attack_events = combat
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack should be ready at time two");
    assert!(attack_events.iter().any(|event| matches!(
        event,
        Event::Attacked { attacker_id, .. } | Event::AttackMissed { attacker_id, .. }
            if attacker_id == "player"
    )));

    let mut spell = ContentParts::tracked("spell_readiness", "profile/spell_readiness")
        .engine(7)
        .expect("spell engine should start");
    spell
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("warmed spell should resolve");
    let warmed = spell.world().actors[0]
        .warmed_spell
        .as_ref()
        .expect("warmed spell state");
    assert_eq!(warmed.warmed_at, LogicalTime::FIRST);
    assert_eq!(warmed.ready_at, LogicalTime::new(2));
    assert_eq!(warmed.status, tme_rules::WarmedSpellStatus::Ready);

    let mut ability = ContentParts::tracked(
        "monster_spellcasting_special_attacks",
        "profile/monster_spellcasting_special_attacks",
    )
    .engine(7)
    .expect("ability engine should start");
    let ability_events = ability
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("monster ability opportunity should resolve");
    assert!(ability_events.iter().any(|event| matches!(
        event,
        Event::AutomaticActorDecision {
            actor_id,
            decision: AutomaticActorDecisionV1::UseAbility { spell_name, .. },
            ..
        } if actor_id == "ember_imp" && spell_name == "Ember Spit"
    )));
    let ember_spit = ability.world().actors[1]
        .monster_abilities
        .iter()
        .find(|candidate| candidate.id == "ember_spit")
        .expect("ember spit ability state");
    assert_eq!(ember_spit.ready_at, LogicalTime::new(4));
}

#[test]
fn rejected_addressed_actor_rolls_back_the_complete_engine() {
    let mut engine = first_room_engine();
    let mut control = engine.clone();
    let before = engine.snapshot();

    let error = engine
        .apply_actor_intent(&tme_rules::ActorId::from("mireling"), PlayerIntent::Wait)
        .expect_err("automatic actor must not accept a controlled intent");
    assert!(error.to_string().contains("not player-controlled"));
    assert_eq!(engine.snapshot(), before);

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("engage after rollback");
    control
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("control engage");
    let actual = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("valid RNG action after rollback");
    let expected = control
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("control RNG action");
    assert_eq!(actual, expected);
    assert_eq!(engine.snapshot(), control.snapshot());
}

#[test]
fn summoned_actor_gets_fresh_order_and_no_immediate_physical_attack() {
    let mut engine = ContentParts::tracked(
        "summons_created_creature_lifecycle",
        "profile/summons_created_creature_lifecycle",
    )
    .engine(7)
    .expect("summon engine should start");
    let initial_next_order = engine.world().timing.next_tie_break_order;

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("summon should resolve");
    let summoned_id = events
        .iter()
        .find_map(|event| match event {
            Event::ActorSummoned { actor_id, .. } => Some(actor_id.clone()),
            _ => None,
        })
        .expect("summon event should expose actor id");
    let summoned = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == summoned_id)
        .expect("summoned actor should remain active");

    assert_eq!(summoned.timing.tie_break_order, initial_next_order);
    assert_eq!(
        engine.world().timing.next_tie_break_order,
        initial_next_order + 1
    );
    assert_eq!(summoned.timing.ready_at, LogicalTime::new(3));
    assert_eq!(summoned.attack_ready_at, LogicalTime::new(2));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ActorReady {
            actor_id,
            logical_time,
            ..
        } if actor_id == &summoned_id && *logical_time == LogicalTime::new(2)
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::Attacked { attacker_id, .. } if attacker_id == &summoned_id
    )));
}

#[test]
fn defeated_and_expired_actors_never_act_again_or_renumber_survivors() {
    let mut defeated = ContentParts::tracked("first_room", "profile/first_room")
        .engine(1_010_580_540)
        .expect("first-room engine should start");
    defeated.world_mut().actors[1].attack_ready_at = LogicalTime::new(99);
    defeated
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("engage should resolve");
    let defeat_events = defeated
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("defeating attack should resolve");
    assert!(defeat_events.iter().any(|event| matches!(
        event,
        Event::ActorDefeated { actor_id, .. } if actor_id == "mireling"
    )));
    assert!(!defeat_events.iter().any(|event| matches!(
        event,
        Event::ActorReady { actor_id, .. } if actor_id == "mireling"
    )));
    let after_defeat = defeated
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Inspect)
        .expect("free read after defeat should resolve");
    assert!(!after_defeat.iter().any(|event| matches!(
        event,
        Event::ActorReady { actor_id, .. } if actor_id == "mireling"
    )));
    assert_eq!(defeated.world().actors[0].timing.tie_break_order, 0);
    assert_eq!(defeated.world().timing.next_tie_break_order, 2);

    let mut expired = ContentParts::tracked(
        "summons_created_creature_lifecycle",
        "profile/summons_created_creature_lifecycle",
    )
    .engine(7)
    .expect("summon engine should start");
    expired.world_mut().actors[1].hp = 100;
    expired.world_mut().actors[1].stats.hp = 100;
    let cast_events = expired
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("summon should resolve");
    let summoned_id = cast_events
        .iter()
        .find_map(|event| match event {
            Event::ActorSummoned { actor_id, .. } => Some(actor_id.clone()),
            _ => None,
        })
        .expect("summoned actor id");
    let expiry_events = expired
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("expiry boundary should resolve");
    assert!(expiry_events.iter().any(|event| matches!(
        event,
        Event::SummonExpired { actor_id, .. } if actor_id == &summoned_id
    )));
    let after_expiry = expired
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Inspect)
        .expect("free read after expiry should resolve");
    assert!(!after_expiry.iter().any(|event| matches!(
        event,
        Event::ActorReady { actor_id, .. } if actor_id == &summoned_id
    )));
    assert_eq!(
        expired
            .world()
            .actors
            .iter()
            .map(|actor| (actor.id.as_str(), actor.timing.tie_break_order))
            .collect::<Vec<_>>(),
        [("player", 0), ("ash_imp", 1)]
    );
    assert_eq!(expired.world().timing.next_tie_break_order, 3);
}

#[test]
fn lifecycle_ticks_before_due_monsters_in_complete_domain_order() {
    let mut engine = timing_effects_engine();
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
        .expect("setup action should take the balm");
    let enchantment_target = engine
        .world()
        .item_instances
        .get("healing_balm")
        .expect("balm item state")
        .clone();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Drink("healing_balm".to_string()),
        )
        .expect("setup action should drink the balm");
    {
        let world = engine.world_mut();
        world.actors[0].resource_activity.last_recovered_at =
            LogicalTime::from_millis(world.timing.now.as_millis().saturating_sub(3000));
        world
            .item_instances
            .insert("enchantment_target".to_string(), enchantment_target);
        world.ground_items.push(tme_rules::GroundItem {
            loot_claim: None,
            item_instance_id: "enchantment_target".to_string(),
            location: WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 }),
        });
        world.portal_transitions.push(PortalTransitionState {
            instance_id: "portal:test:1".to_string(),
            source_spell_id: "test_portal".to_string(),
            source_actor_id: "player".into(),
            location: WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 }),
            target: WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 1 }),
            two_way: false,
            remaining_rounds: Some(1),
            last_ticked_at: LogicalTime::new(2),
        });
        world.item_enchantments.push(ItemEnchantmentState {
            enchantment_instance_id: "enchantment:test:1".to_string(),
            source: ItemOperationSource::Spell {
                spell_id: "test_enchantment".to_string(),
                actor_id: "player".into(),
            },
            item_instance_id: "enchantment_target".to_string(),
            combat_add_rating_bonus: 1,
            tags: vec!["test".to_string()],
            remaining_rounds: Some(1),
            last_ticked_at: LogicalTime::new(2),
        });
        world.actors[1].summoned = Some(SummonedActorState {
            instance_id: "summon:test:1".into(),
            owner_id: "player".into(),
            source_spell_id: "test_summon".to_string(),
            template_id: "test_watcher".to_string(),
            remaining_rounds: Some(1),
            last_ticked_at: LogicalTime::new(2),
        });
        world.tile_effects.push(TileEffectState {
            source_actor_id: None,
            instance_id: "tile:test:1".to_string(),
            effect_id: "test_tile".to_string(),
            source: ActiveEffectSource {
                kind: "fixture".to_string(),
                id: "logical_timing".to_string(),
            },
            location: WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 }),
            kind: "terrain_overlay".to_string(),
            tags: vec!["test".to_string()],
            potency: 0,
            remaining_rounds: Some(1),
            passability: None,
            sight: None,
            hazard: None,
            move_cost: None,
            tick_interval_rounds: 1,
            last_ticked_at: LogicalTime::new(2),
            hostile_authority: None,
        });
        world.actors[0].resource_activity.last_active_at = None;
        world.actors[0].hp = 4;
        world.actors[0].character.as_mut().unwrap().resources.hp = 4;
        world.actors[0].stamina = 8;
        world.actors[0]
            .character
            .as_mut()
            .expect("player character")
            .resources
            .stamina = 8;
    }
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("action should run every lifecycle domain");
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::AutomaticActorDecision { .. }))
    );
    let portal = events
        .iter()
        .position(|event| matches!(event, Event::PortalExpired { .. }))
        .expect("portal expiry");
    let enchantment = events
        .iter()
        .position(|event| matches!(event, Event::ItemEnchantmentExpired { .. }))
        .expect("enchantment expiry");
    let summon = events
        .iter()
        .position(|event| matches!(event, Event::SummonExpired { .. }))
        .expect("summon expiry");
    let actor_effect = events
        .iter()
        .position(|event| matches!(event, Event::EffectTicked { .. }))
        .expect("effect tick");
    let tile = events
        .iter()
        .position(|event| matches!(event, Event::TileEffectTicked { .. }))
        .expect("tile effect tick");
    let balm = events
        .iter()
        .position(|event| matches!(event, Event::BalmHealed { .. }))
        .expect("balm tick");
    let hp_recovery = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::ResourceRegenerated {
                    resource: tme_rules::ResourceKind::Hp,
                    ..
                }
            )
        })
        .expect("hp recovery");
    let stamina = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::ResourceRegenerated {
                    resource: tme_rules::ResourceKind::Stamina,
                    ..
                }
            )
        })
        .expect("stamina recovery");
    assert!(portal < enchantment);
    assert!(enchantment < summon);
    assert!(summon < actor_effect);
    assert!(actor_effect < tile);
    assert!(tile < balm);
    assert!(balm < hp_recovery);
    assert!(hp_recovery < stamina);
}

#[test]
fn serialized_timing_contract_has_no_global_round_or_phase_scaffold() {
    let mut engine = first_room_engine();
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Inspect)
        .expect("free inspect should resolve");
    let event_json = serde_json::to_value(events.events).expect("events should serialize");
    let event_entries = event_json.as_array().expect("event array");
    assert!(
        event_entries
            .iter()
            .any(|entry| entry.get("actor_ready").is_some())
    );
    assert!(
        event_entries
            .iter()
            .all(|entry| entry.get("actor_readiness_scheduled").is_none())
    );
    assert!(
        event_entries
            .iter()
            .all(|entry| entry.get("round_started").is_none())
    );
    assert!(
        event_entries
            .iter()
            .all(|entry| entry.get("phase_started").is_none())
    );

    let snapshot = serde_json::to_value(engine.snapshot()).expect("snapshot should serialize");
    assert_eq!(
        snapshot.get("logical_time"),
        Some(&serde_json::json!({"milliseconds": 3000}))
    );
    assert!(snapshot.get("round").is_none());
    let actor = &snapshot["actors"][0];
    assert!(actor.get("ready_at").is_some());
    assert!(actor.get("attack_ready_at").is_some());
    assert!(actor.get("attack_ready_round").is_none());
}
