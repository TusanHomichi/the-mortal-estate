use crate::support::content_parts::ContentParts;
use tme_rules::{ActorKind, DeterministicRng, Direction, Engine, Event, LogicalTime, PlayerIntent};

fn has<F: Fn(&Event) -> bool>(events: &[Event], f: F) -> bool {
    events.iter().any(f)
}

fn xp_fixture_value() -> ContentParts {
    ContentParts::tracked("xp_progression", "profile/xp_progression")
}

fn engine_from_value(value: &ContentParts, seed: u64) -> Engine {
    value.engine(seed).expect("engine should start")
}

fn move_to_mireling(engine: &mut Engine) {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("move should work");
}

fn attack_mireling(engine: &mut Engine) -> Vec<Event> {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack should work")
        .events
}

fn set_runtime_hp(engine: &mut Engine, hp: i32) {
    let actor = &mut engine.world_mut().actors[0];
    actor.hp = hp;
    actor
        .character
        .as_mut()
        .expect("player character")
        .resources
        .hp = hp;
}

fn set_runtime_stamina(engine: &mut Engine, stamina: i32) {
    let actor = &mut engine.world_mut().actors[0];
    actor.stamina = stamina;
    actor
        .character
        .as_mut()
        .expect("player character")
        .resources
        .stamina = stamina;
}

fn pending_value(level: i32, experience: i64) -> ContentParts {
    let mut value = xp_fixture_value();
    value.actors_mut()[0]["character"]["progression"]["level"] = level.into();
    value.actors_mut()[0]["character"]["progression"]["experience"] = experience.into();
    value.actor_definition_mut(1)["ai"]["behavior"] = "hold_ground".into();
    value
}

fn profile_from_first_room(class_id: &str) -> serde_json::Value {
    let mut value = ContentParts::tracked("first_room", "profile/first_room");
    value.rules_source_mut()["progression"]["growth_profiles"]
        .as_array()
        .expect("growth profiles")
        .iter()
        .find(|profile| profile["class_id"] == class_id)
        .expect("requested clean profile")
        .clone()
}

fn level_growth(events: &[Event]) -> Vec<(i32, i32, i32, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::LevelGained {
                new_level,
                hp_growth,
                mp_growth,
                stamina_growth,
                ..
            } => Some((*new_level, *hp_growth, *mp_growth, *stamina_growth)),
            _ => None,
        })
        .collect()
}

#[test]
fn kill_awards_xp() {
    let mut engine = xp_fixture_value()
        .engine(1_010_580_540)
        .expect("engine should start");
    let mut all_events = engine.initial_events();
    // Move to engage
    let step1 = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            tme_rules::PlayerIntent::MovePath(vec![
                tme_rules::Direction::East,
                tme_rules::Direction::East,
            ]),
        )
        .expect("move should work");
    all_events.extend(step1.events);
    // Attack
    let step2 = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            tme_rules::PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack should work");
    all_events.extend(step2.events);
    all_events.extend(engine.final_events());

    assert!(
        has(&all_events, |e| matches!(
            e,
            Event::ExperienceAwarded { .. }
        )),
        "should have XP awarded event"
    );
    assert!(
        has(&all_events, |e| matches!(e, Event::LevelGained { .. })),
        "should have level-up event"
    );
    assert!(
        has(&all_events, |e| matches!(
            e,
            Event::PhysicalAttributeAddsChanged { .. }
        )),
        "should have combat-adds event"
    );
    // Player should now have XP and higher level
    let player = &engine.world().actors[0];
    let cs = player.character.as_ref().unwrap();
    assert_eq!(cs.progression.experience, 600);
    assert_eq!(cs.progression.level, 4);
    assert_eq!(cs.resources.hp, 47);
    assert_eq!(cs.resources.max_hp, 47);
    assert_eq!(cs.resources.peak_hp, 47);
    assert_eq!(cs.resources.stamina, 28);
    assert_eq!(cs.resources.max_stamina, 28);
    assert_eq!(cs.physical_attribute_adds.strength_adds, 2);
    assert_eq!(cs.physical_attribute_adds.dexterity_adds, 2);

    let receipts = all_events
        .iter()
        .filter_map(|event| match event {
            Event::ExperienceAwarded { .. } => Some("xp".to_string()),
            Event::LevelGained {
                new_level,
                hp_growth,
                stamina_growth,
                ..
            } => {
                assert_eq!((*hp_growth, *stamina_growth), (9, 6));
                Some(format!("level:{new_level}"))
            }
            Event::PhysicalAttributeAddsChanged { .. } => {
                Some("physical_attribute_adds".to_string())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        receipts,
        [
            "xp",
            "level:2",
            "level:3",
            "physical_attribute_adds",
            "level:4",
        ]
    );
}

#[test]
fn non_progression_fixture_has_no_xp_events() {
    let mut engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("engine should start");
    engine.world_mut().actors[1].attack_ready_at = LogicalTime::new(99);
    let mut all_events = engine.initial_events();
    let step1 = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            tme_rules::PlayerIntent::MovePath(vec![
                tme_rules::Direction::East,
                tme_rules::Direction::East,
            ]),
        )
        .expect("move");
    all_events.extend(step1.events);
    let step2 = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            tme_rules::PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack");
    all_events.extend(step2.events);
    all_events.extend(engine.final_events());
    // This fixture has no xp_value on the monster, so no XP events.
    assert!(!has(&all_events, |e| matches!(
        e,
        Event::ExperienceAwarded { .. }
    )));
}

#[test]
fn negative_xp_value_fails_validation() {
    let mut value = xp_fixture_value();
    value.actor_definition_mut(1)["xp_value"] = serde_json::Value::from(-1);
    let result = value.validated_seed();
    match result {
        Err(e) => {
            assert!(e.to_string().contains("xp_value must be non-negative"));
        }
        Ok(_) => panic!("negative xp_value should fail"),
    }
}

#[test]
fn xp_value_on_player_fails_validation() {
    let mut value = xp_fixture_value();
    value.actor_definition_mut(0)["xp_value"] = serde_json::Value::from(100);
    let result = value.validated_seed();
    match result {
        Err(e) => {
            assert!(
                e.to_string()
                    .contains("xp_value is only valid for monsters")
            );
        }
        Ok(_) => panic!("xp_value on player should fail"),
    }
}

#[test]
fn xp_value_defaults_to_zero() {
    let engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("engine should start");
    let monster = &engine.world().actors[1];
    assert_eq!(monster.kind, ActorKind::Monster);
    assert_eq!(monster.xp_value, 0);
}

#[test]
fn physical_attribute_adds_follow_authored_current_class_rows() {
    let mut engine = xp_fixture_value().engine(7).expect("engine should start");
    // Move + attack to trigger XP
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            tme_rules::PlayerIntent::MovePath(vec![
                tme_rules::Direction::East,
                tme_rules::Direction::East,
            ]),
        )
        .ok();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            tme_rules::PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .ok();
    let player = &engine.world().actors[0];
    let cs = player.character.as_ref().unwrap();
    assert_eq!(
        cs.physical_attribute_adds.strength_adds, 2,
        "fighter at level 4 should retain the baseline-progression +2 STR total"
    );
    assert_eq!(
        cs.physical_attribute_adds.dexterity_adds, 2,
        "fighter at level 4 should retain the baseline-progression +2 DEX total"
    );
    assert_eq!(cs.progression.level, 4);
}

#[test]
fn clean_combat_add_rows_encode_the_selected_two_speed_schedule_as_deltas() {
    let fast = serde_json::json!([
        {"level": 3, "strength_adds": 2, "dexterity_adds": 2},
        {"level": 6, "strength_adds": 1, "dexterity_adds": 1},
        {"level": 9, "strength_adds": 1, "dexterity_adds": 1}
    ]);
    let slow = serde_json::json!([
        {"level": 3, "strength_adds": 1, "dexterity_adds": 1},
        {"level": 4, "strength_adds": 1, "dexterity_adds": 1},
        {"level": 8, "strength_adds": 1, "dexterity_adds": 1}
    ]);

    for class_id in ["fighter", "knight", "martial_artist"] {
        assert_eq!(
            profile_from_first_room(class_id)["physical_attribute_adds_by_level"],
            fast,
            "{class_id} must use the selected fast physical lane"
        );
    }
    for class_id in ["wizard", "thaumaturge", "thief"] {
        assert_eq!(
            profile_from_first_room(class_id)["physical_attribute_adds_by_level"],
            slow,
            "{class_id} must use the selected slower lane"
        );
    }
}

#[test]
fn seeded_level_growth_updates_all_owned_resource_fields() {
    let mut engine = xp_fixture_value()
        .engine(1_010_580_540)
        .expect("engine should start");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            tme_rules::PlayerIntent::MovePath(vec![
                tme_rules::Direction::East,
                tme_rules::Direction::East,
            ]),
        )
        .ok();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            tme_rules::PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .ok();
    let player = &engine.world().actors[0];
    let cs = player.character.as_ref().unwrap();
    assert_eq!(cs.resources.max_hp, 47);
    assert_eq!(cs.resources.hp, 47);
    assert_eq!(cs.resources.peak_hp, 47);
    assert_eq!(cs.resources.max_mp, 0);
    assert_eq!(cs.resources.mp, 0);
    assert_eq!(cs.resources.stamina, 28);
    assert_eq!(cs.resources.max_stamina, 28);
    assert_eq!(player.hp, 47);
    assert_eq!(player.stamina, 28);

    let mut engine2 = xp_fixture_value().engine(7).expect("engine should start");
    let mut all_events = engine2.initial_events();
    all_events.extend(
        engine2
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                tme_rules::PlayerIntent::MovePath(vec![
                    tme_rules::Direction::East,
                    tme_rules::Direction::East,
                ]),
            )
            .expect("move should work")
            .events,
    );
    all_events.extend(
        engine2
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                tme_rules::PlayerIntent::PhysicalAttack {
                    authorization: tme_rules::HostilityAuthorization::Safe,
                    mode: tme_rules::PhysicalAttackMode::Fight,
                    target_actor_id: "mireling".into(),
                },
            )
            .expect("attack should work")
            .events,
    );
    all_events.extend(engine2.final_events());
    let level_receipts = all_events
        .iter()
        .filter(|event| matches!(event, Event::LevelGained { .. }))
        .count();
    assert_eq!(level_receipts, 3);
}

#[test]
fn damaged_hp_banks_xp_and_exposes_derived_pending_target() {
    let mut engine = engine_from_value(&xp_fixture_value(), 7);
    move_to_mireling(&mut engine);
    set_runtime_hp(&mut engine, 10);
    let events = attack_mireling(&mut engine);

    let character = engine.world().actors[0].character.as_ref().unwrap();
    assert_eq!(character.progression.experience, 600);
    assert_eq!(character.progression.level, 1);
    assert!(level_growth(&events).is_empty());
    assert_eq!(
        engine
            .snapshot()
            .actors
            .iter()
            .find(|actor| actor.id == "player")
            .unwrap()
            .character
            .as_ref()
            .unwrap()
            .progression
            .pending_target_level,
        Some(4)
    );
}

#[test]
fn spent_stamina_banks_xp_while_mp_deficit_does_not_gate() {
    let mut stamina_blocked = engine_from_value(&xp_fixture_value(), 7);
    move_to_mireling(&mut stamina_blocked);
    set_runtime_stamina(&mut stamina_blocked, 0);
    let events = attack_mireling(&mut stamina_blocked);
    assert!(level_growth(&events).is_empty());
    assert_eq!(
        stamina_blocked.world().actors[0]
            .character
            .as_ref()
            .unwrap()
            .progression
            .level,
        1
    );

    let mut value = xp_fixture_value();
    value.actors_mut()[0]["character"]["resources"]["mp"] = 2.into();
    value.actors_mut()[0]["character"]["resources"]["max_mp"] = 10.into();
    value.rules_source_mut()["resources"]["recovery_interval_units"] = 100.into();
    let mut mp_spent = engine_from_value(&value, 7);
    move_to_mireling(&mut mp_spent);
    let events = attack_mireling(&mut mp_spent);
    assert_eq!(level_growth(&events).len(), 3);
    let resources = &mp_spent.world().actors[0]
        .character
        .as_ref()
        .unwrap()
        .resources;
    assert_eq!(resources.max_mp - resources.mp, 8);
}

#[test]
fn free_reads_do_not_apply_pending_levels_or_consume_growth_rng() {
    let value = pending_value(1, 100);
    let mut after_reads = engine_from_value(&value, 19);
    let mut direct = engine_from_value(&value, 19);

    after_reads
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Inspect)
        .expect("inspect should work");
    after_reads
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::ShowSack)
        .expect("show sack should work");
    assert_eq!(
        after_reads.world().actors[0]
            .character
            .as_ref()
            .unwrap()
            .progression
            .level,
        1
    );

    let after_read_events = after_reads
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("wait should apply pending level");
    let direct_events = direct
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("wait should apply pending level");
    assert_eq!(
        level_growth(&after_read_events.events),
        level_growth(&direct_events.events)
    );
    assert_eq!(after_reads.snapshot(), direct.snapshot());
}

#[test]
fn ordered_recovery_opens_gate_only_after_hp_and_stamina_are_full() {
    let mut value = pending_value(1, 100);
    value.actors_mut()[0]["character"]["resources"]["hp"] = 18.into();
    value.actors_mut()[0]["character"]["resources"]["stamina"] = 9.into();
    let mut engine = engine_from_value(&value, 7);
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("wait and recovery should work");

    let stamina_recovery = events
        .events
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
        .expect("stamina should recover");
    let level = events
        .events
        .iter()
        .position(|event| matches!(event, Event::LevelGained { .. }))
        .expect("recovery should open the level gate");
    assert!(stamina_recovery < level);
}

#[test]
fn authored_ceiling_retains_excess_xp_without_extrapolation() {
    let value = pending_value(10, 50_000);
    let mut engine = engine_from_value(&value, 7);
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("ceiling wait should work");
    let character = engine.world().actors[0].character.as_ref().unwrap();
    assert_eq!(character.progression.level, 10);
    assert_eq!(character.progression.experience, 50_000);
    assert!(level_growth(&events.events).is_empty());
}

#[test]
fn current_knight_profile_controls_growth_after_promotion() {
    let mut value = pending_value(1, 100);
    value.profile_value_mut()["rules_profile"] = "rules/knight_promotion".into();
    value.actors_mut()[0]["character"]["identity"]["current_class_id"] = "knight".into();
    value.actors_mut()[0]["character"]["identity"]["display_class"] = "Knight".into();
    let mut engine = engine_from_value(&value, 7);
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .unwrap();
    assert_eq!(level_growth(&events.events), [(2, 10, 4, 6)]);
    assert!(events.events.iter().any(|event| matches!(
        event,
        Event::LevelGained { current_class_id, .. } if current_class_id == "knight"
    )));
}

#[test]
fn mp_and_peak_hp_deficits_are_preserved_by_growth() {
    let mut value = pending_value(1, 100);
    value.rules_source_mut()["progression"]["growth_profiles"] =
        serde_json::json!([profile_from_first_room("wizard")]);
    value.actors_mut()[0]["character"]["identity"]["base_class_id"] = "wizard".into();
    value.actors_mut()[0]["character"]["identity"]["current_class_id"] = "wizard".into();
    value.actors_mut()[0]["character"]["resources"]["mp"] = 2.into();
    value.actors_mut()[0]["character"]["resources"]["max_mp"] = 10.into();
    value.actors_mut()[0]["character"]["resources"]["peak_hp"] = 30.into();
    value.rules_source_mut()["resources"]["recovery_interval_units"] = 100.into();
    let mut engine = engine_from_value(&value, 7);
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .unwrap();
    assert_eq!(level_growth(&events.events), [(2, 6, 7, 5)]);
    let resources = &engine.world().actors[0]
        .character
        .as_ref()
        .unwrap()
        .resources;
    assert_eq!(resources.max_mp - resources.mp, 8);
    assert_eq!(resources.peak_hp - resources.max_hp, 10);
}

#[test]
fn checked_growth_overflow_rolls_back_world_events_and_rng() {
    let mut value = pending_value(1, 100);
    for field in ["hp", "max_hp", "peak_hp"] {
        value.actors_mut()[0]["character"]["resources"][field] = i32::MAX.into();
    }
    let mut rolled_back = engine_from_value(&value, 31);
    let mut untouched = rolled_back.clone();
    assert!(
        rolled_back
            .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
            .is_err()
    );
    assert_eq!(rolled_back.snapshot(), untouched.snapshot());

    for engine in [&mut rolled_back, &mut untouched] {
        let actor = &mut engine.world_mut().actors[0];
        actor.hp = 20;
        let resources = &mut actor.character.as_mut().unwrap().resources;
        resources.hp = 20;
        resources.max_hp = 20;
        resources.peak_hp = 20;
    }
    let rolled_back_events = rolled_back
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .unwrap();
    let untouched_events = untouched
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .unwrap();
    assert_eq!(
        level_growth(&rolled_back_events.events),
        level_growth(&untouched_events.events)
    );
    assert_eq!(rolled_back.snapshot(), untouched.snapshot());
}

#[test]
fn seeded_growth_replays_and_varies_without_singleton_draws() {
    let value = pending_value(1, 100);
    let mut seen = std::collections::BTreeSet::new();
    for seed in 1..=16 {
        let mut engine = engine_from_value(&value, seed);
        let events = engine
            .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
            .unwrap();
        seen.insert(level_growth(&events.events)[0]);
    }
    assert!(seen.len() > 1);

    let mut selected = DeterministicRng::new(77);
    let untouched = selected.clone();
    assert_eq!(selected.weighted_index(&[5]), Ok(0));
    assert_eq!(selected, untouched);
}

#[test]
fn unknown_current_class_has_no_fallback_profile() {
    let mut value = xp_fixture_value();
    value.actors_mut()[0]["character"]["identity"]["current_class_id"] = "unknown".into();
    let error = value.validated_seed().expect_err("profile is required");
    assert!(
        error
            .to_string()
            .contains("growth_profiles must contain class_id \"unknown\"")
    );
}
