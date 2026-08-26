use crate::support::content_parts::ContentParts;
use serde_json::json;
use tme_rules::{Direction, Engine, Event, PhysicalAttackMode, PlayerIntent, SpellTarget};

fn has<F: Fn(&Event) -> bool>(events: &[Event], predicate: F) -> bool {
    events.iter().any(predicate)
}

fn count_skill_practice(events: &[Event], track_id: &str) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                event,
                Event::SkillPracticeAwarded {
                    track_id: event_track_id,
                    ..
                } if event_track_id == track_id
            )
        })
        .count()
}

fn actor_index(engine: &Engine, actor_id: &str) -> usize {
    engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == actor_id)
        .expect("actor")
}

fn skill_parts() -> ContentParts {
    ContentParts::tracked("skill_progression", "profile/skill_progression")
}

fn skill_engine() -> Engine {
    skill_parts().engine(7).expect("skill progression graph")
}

fn engage(engine: &mut Engine) {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("engage");
}

fn fight(engine: &mut Engine) -> Vec<Event> {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack")
        .events
}

fn magic_skill_practice_engine(warmed: bool) -> Engine {
    let mut parts = ContentParts::tracked("spell_readiness", "profile/spell_readiness");
    let spell_id = if warmed { "charged_spark" } else { "spark" };
    parts.selected_by_runtime_id_mut("spells", spell_id)["mp_cost"] = json!(3);
    parts.actors_mut()[1]["id"] = json!("target");
    parts.actor_definition_mut(1)["name"] = json!("Target");
    parts.engine(7).expect("spell-practice graph")
}

fn add_skill_track(parts: &mut ContentParts, track_id: &str, display: &str) {
    let catalog = parts.skill_catalog_mut().expect("skill catalog");
    let mut track = catalog["tracks"][0].clone();
    track["id"] = json!(track_id);
    track["display"] = json!(display);
    catalog["tracks"]
        .as_array_mut()
        .expect("tracks")
        .push(track);
}

#[test]
fn applied_spell_increments_wizard_magic_practice() {
    let mut engine = magic_skill_practice_engine(false);
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "spark".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("direct spell");
    assert_eq!(count_skill_practice(&events.events, "wizard_magic"), 1);
    assert!(events.events.iter().any(|event| matches!(
        event,
        Event::MagicPracticeEvaluated {
            base_raw_points: 3,
            primary_attribute_bonus_raw_points: 1,
            total_raw_points: 4,
            risk_applied: false,
            ..
        }
    )));
    assert!(events.events.iter().any(|event| matches!(
        event,
        Event::SkillPracticeAwarded {
            track_id,
            raw_amount: 4,
            learning_rate: 1,
            credited_amount: 4,
            ..
        } if track_id == "wizard_magic"
    )));

    let player = actor_index(&engine, "player");
    let wizard_magic = engine.world().actors[player]
        .character
        .as_ref()
        .expect("character")
        .skill_ledger
        .iter()
        .find(|entry| entry.track_id == "wizard_magic")
        .expect("wizard magic");
    assert_eq!(wizard_magic.practice_points, 1);
    assert_eq!(wizard_magic.critique_rank, 1);
}

#[test]
fn magic_skill_practice_warming_leaves_ledger_unchanged_until_cast() {
    let mut engine = magic_skill_practice_engine(true);
    let warmed = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("warm");
    assert_eq!(count_skill_practice(&warmed.events, "wizard_magic"), 0);
    let player = actor_index(&engine, "player");
    assert_eq!(
        engine.world().actors[player]
            .character
            .as_ref()
            .expect("character")
            .skill_ledger[0]
            .practice_points,
        0
    );

    let cast = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastWarmedSpell {
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("warmed cast");
    assert_eq!(count_skill_practice(&cast.events, "wizard_magic"), 1);
    assert!(cast.events.iter().any(|event| matches!(
        event,
        Event::MagicPracticeEvaluated {
            base_raw_points: 3,
            primary_attribute_bonus_raw_points: 1,
            total_raw_points: 4,
            risk_applied: false,
            ..
        }
    )));
    let wizard_magic = &engine.world().actors[player]
        .character
        .as_ref()
        .expect("character")
        .skill_ledger[0];
    assert_eq!(wizard_magic.practice_points, 1);
    assert_eq!(wizard_magic.critique_rank, 1);
}

#[test]
fn attack_grants_skill_practice() {
    let mut engine = skill_engine();
    let mut events = engine.initial_events();
    engage(&mut engine);
    events.extend(fight(&mut engine));
    events.extend(engine.final_events());
    assert!(has(&events, |event| matches!(
        event,
        Event::SkillPracticeAwarded { .. }
    )));
    assert!(has(&events, |event| matches!(
        event,
        Event::SkillPositionChanged { .. }
    )));
}

#[test]
fn untrained_skill_reaches_level_one_critique_one() {
    let mut engine = skill_engine();
    engage(&mut engine);
    fight(&mut engine);
    let player = actor_index(&engine, "player");
    let sword = engine.world().actors[player]
        .character
        .as_ref()
        .expect("character")
        .skill_ledger
        .iter()
        .find(|entry| entry.track_id == "sword")
        .expect("sword");
    assert_eq!(sword.level, 1);
    assert_eq!(sword.critique_rank, 1);
    assert_eq!(sword.practice_points, 0);
    assert_eq!(sword.learning_rate, 1);
}

#[test]
fn permanent_learning_rate_credits_practice_and_survives_position_change() {
    let mut parts = skill_parts();
    parts.actors_mut()[0]["character"]["skill_ledger"] = json!([{
        "track_id": "sword",
        "level": 0,
        "critique_rank": 0,
        "practice_points": 0,
        "learning_rate": 2
    }]);
    let mut engine = parts.engine(7).expect("learning-rate graph");
    engage(&mut engine);
    let events = fight(&mut engine);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SkillPracticeAwarded {
            track_id,
            raw_amount: 1,
            learning_rate: 2,
            credited_amount: 2,
            practice_points: 2,
            level: 0,
            critique_rank: 0,
            ..
        } if track_id == "sword"
    )));
    let player = actor_index(&engine, "player");
    let sword = &engine.world().actors[player]
        .character
        .as_ref()
        .expect("character")
        .skill_ledger[0];
    assert_eq!((sword.level, sword.critique_rank), (1, 1));
    assert_eq!(sword.practice_points, 1);
    assert_eq!(sword.learning_rate, 2);
}

#[test]
fn practice_overflow_rolls_back_world_events_and_rng() {
    let mut parts = skill_parts();
    parts.actors_mut()[0]["character"]["skill_ledger"] = json!([{
        "track_id": "sword",
        "level": 0,
        "critique_rank": 0,
        "practice_points": 1,
        "learning_rate": u64::MAX
    }]);
    let mut engine = parts.engine(7).expect("overflow graph");
    engage(&mut engine);
    let mut expected = engine.clone();
    let world_before = engine.world().clone();
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect_err("practice overflow");
    assert!(error.message().contains("practice pool must not overflow"));
    assert_eq!(engine.world(), &world_before);

    for candidate in [&mut engine, &mut expected] {
        let player = actor_index(candidate, "player");
        candidate.world_mut().actors[player]
            .character
            .as_mut()
            .expect("character")
            .skill_ledger[0]
            .learning_rate = 1;
    }
    let actual_events = fight(&mut engine);
    let expected_events = fight(&mut expected);
    assert_eq!(actual_events, expected_events);
    assert_eq!(engine.world(), expected.world());
}

#[test]
fn non_progression_fixture_has_empty_skill_ledger() {
    let mut engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("first-room graph");
    engage(&mut engine);
    let events = fight(&mut engine);
    assert!(!has(&events, |event| matches!(
        event,
        Event::SkillPracticeAwarded { .. }
    )));
}

#[test]
fn multiple_attacks_accumulate_skill_practice() {
    let mut engine = skill_engine();
    engage(&mut engine);
    fight(&mut engine);
    fight(&mut engine);
    let player = actor_index(&engine, "player");
    let sword = engine.world().actors[player]
        .character
        .as_ref()
        .expect("character")
        .skill_ledger
        .iter()
        .find(|entry| entry.track_id == "sword")
        .expect("sword");
    assert_eq!(sword.practice_points, 1);
    assert_eq!(sword.level, 1);
    assert_eq!(sword.critique_rank, 1);
}

#[test]
fn sword_category_awards_sword_practice() {
    let mut engine = skill_engine();
    let mut events = engine.initial_events();
    engage(&mut engine);
    events.extend(fight(&mut engine));
    events.extend(engine.final_events());
    assert!(has(&events, |event| matches!(
        event,
        Event::SkillPracticeAwarded { track_id, .. } if track_id == "sword"
    )));
}

#[test]
fn capability_skill_track_overrides_category() {
    let mut parts = skill_parts();
    add_skill_track(&mut parts, "rapier", "Rapier");
    parts.selected_by_runtime_id_mut("items", "training_sword")["weapon"]["skill_track_id"] =
        json!("rapier");
    let mut engine = parts.engine(7).expect("track override graph");
    let mut events = engine.initial_events();
    engage(&mut engine);
    events.extend(fight(&mut engine));
    events.extend(engine.final_events());
    assert!(has(&events, |event| matches!(
        event,
        Event::SkillPracticeAwarded { track_id, .. } if track_id == "rapier"
    )));
    assert!(!has(&events, |event| matches!(
        event,
        Event::SkillPracticeAwarded { track_id, .. } if track_id == "sword"
    )));
}

#[test]
fn unarmed_attack_awards_hand_practice() {
    let mut parts = skill_parts();
    add_skill_track(&mut parts, "hand", "Hand");
    parts.actors_mut()[0]["carried"]["items"] = json!([]);
    parts
        .item_instances_mut()
        .as_object_mut()
        .expect("items")
        .remove("training_sword");
    let mut engine = parts.engine(7).expect("unarmed graph");
    let mut events = engine.initial_events();
    engage(&mut engine);
    events.extend(fight(&mut engine));
    events.extend(engine.final_events());
    assert!(has(&events, |event| matches!(
        event,
        Event::SkillPracticeAwarded { track_id, .. } if track_id == "hand"
    )));
}

#[test]
fn no_character_sheet_means_no_skill_practice() {
    let mut engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("first-room graph");
    let mut events = engine.initial_events();
    engage(&mut engine);
    events.extend(fight(&mut engine));
    events.extend(engine.final_events());
    assert!(!has(&events, |event| matches!(
        event,
        Event::SkillPracticeAwarded { .. }
    )));
    assert!(!has(&events, |event| matches!(
        event,
        Event::SkillPositionChanged { .. }
    )));
}

#[test]
fn skill_caps_at_level_19_critique_10() {
    let mut parts = skill_parts();
    parts.actors_mut()[0]["character"]["skill_ledger"] = json!([{
        "track_id": "sword",
        "level": 19,
        "critique_rank": 10,
        "practice_points": 0,
        "learning_rate": 1
    }]);
    let mut engine = parts.engine(7).expect("maximum skill graph");
    let mut events = engine.initial_events();
    engage(&mut engine);
    events.extend(fight(&mut engine));
    events.extend(engine.final_events());
    assert!(!has(&events, |event| matches!(
        event,
        Event::SkillPracticeAwarded { .. }
    )));
    assert!(!has(&events, |event| matches!(
        event,
        Event::SkillPositionChanged { .. }
    )));
    let player = actor_index(&engine, "player");
    let sword = &engine.world().actors[player]
        .character
        .as_ref()
        .expect("character")
        .skill_ledger[0];
    assert_eq!(sword.level, 19);
    assert_eq!(sword.critique_rank, 10);
}
