//! Passive defense must work in the played catalog, not just a profession fixture.
#[path = "support/martial_defense.rs"]
mod support;

use serde_json::{Value, json};
use tme_rules::{
    ActorId, BlockSourceKind, CarriedPosition, CheckpointContentMigration, DeterministicRng,
    Engine, Event, ObservedEventV1, ObserverFeedbackCueV1, ObserverPhysicalOutcomeV1,
    PhysicalAttackMode,
};

use support::{
    ACTION, ARMOR, PROFILE, armor_definition, attack, build_definition, candidate, catalog,
    defender, defender_id, definition, duel, equip, hand_block, next_block_roll, place_test_item,
};

#[test]
fn played_profile_selects_one_class_specific_passive_without_raising_starting_skill() {
    let source = catalog();
    assert_eq!(
        source["profiles"][PROFILE]["profession_actions"],
        json!([ACTION])
    );
    assert_eq!(source["profession_actions"].as_object().unwrap().len(), 1);
    assert_eq!(
        source["profession_actions"][ACTION]["class_ids"],
        json!(["martial_artist"])
    );
    assert_eq!(
        source["profession_actions"][ACTION]["martial_hand_block"],
        json!({"min_hand_level": 2, "level_divisor": 20, "max_chance_percent": 95})
    );
    let definition = definition();
    assert_eq!(definition.creation_profiles().len(), 5);
    let profile = definition
        .creation_profiles()
        .iter()
        .find(|profile| profile.id == "creation/martial_artist")
        .unwrap();
    assert_eq!(
        profile
            .character
            .skill_ledger
            .iter()
            .find(|entry| entry.track_id == "hand")
            .unwrap()
            .level,
        1
    );
}

#[test]
fn current_class_and_minimum_skill_gate_the_actual_block_not_only_the_snapshot() {
    for class in [
        "fighter",
        "martial_artist",
        "thief",
        "wizard",
        "thaumaturge",
    ] {
        for level in [0, 1, 2, 19] {
            let mut engine = duel(definition(), &format!("creation/{class}"), level, 17);
            let eligible = class == "martial_artist" && level >= 2;
            assert_eq!(candidate(&engine).is_some(), eligible, "{class}, {level}");
            let events = attack(&mut engine, PhysicalAttackMode::Kick);
            assert_eq!(hand_block(&events).is_some(), eligible, "{class}, {level}");
        }
    }
}

#[test]
fn original_curve_has_exact_effective_odds_across_every_d20_face() {
    let cases = [(2, 10, 1), (8, 40, 7), (12, 60, 11), (19, 95, 18)];
    for (level, threshold, expected_successes) in cases {
        let template = duel(definition(), "creation/martial_artist", level, 0);
        assert_eq!(candidate(&template).unwrap().chance_percent, threshold);
        let mut faces = std::collections::BTreeSet::new();
        let mut successes = 0;
        for seed in 0..20 {
            let roll = DeterministicRng::new(seed).roll_d20();
            faces.insert(roll);
            let mut engine = duel(definition(), "creation/martial_artist", level, seed);
            let hp = defender(&engine).hp;
            let events = attack(&mut engine, PhysicalAttackMode::Kick);
            let blocked = hand_block(&events);
            assert_eq!(blocked.is_some(), roll * 5 < threshold, "{level}, {roll}");
            if let Some(Event::AttackBlocked {
                chance_percent,
                roll: actual,
                ..
            }) = blocked
            {
                assert_eq!((*chance_percent, *actual), (threshold, roll));
                assert_eq!(defender(&engine).hp, hp);
                successes += 1;
            }
        }
        assert_eq!(faces, (1..=20).collect());
        assert_eq!(successes, expected_successes, "hand level {level}");
    }
}

#[test]
fn passive_block_is_free_for_a_busy_defender_and_projects_a_single_blocked_outcome() {
    let mut engine = duel(definition(), "creation/martial_artist", 8, 17);
    place_test_item(&mut engine, "trade_charm", "defense_test/charm");
    equip(&mut engine, "defense_test/charm", CarriedPosition::LeftHand);
    let before = defender(&engine).clone();
    let now = engine.world().timing.now;
    assert!(
        before.timing.ready_at > now,
        "fixture defender is still busy"
    );
    let events = attack(&mut engine, PhysicalAttackMode::Kick);
    match hand_block(&events).unwrap() {
        Event::AttackBlocked {
            source,
            carried_position,
            item_instance_id,
            skill_track_id,
            skill_level,
            chance_percent,
            roll,
            effective_combat_add_rating,
            armor_encumbrance,
            ..
        } => {
            assert_eq!(*source, BlockSourceKind::RightMartialHand);
            assert_eq!(*carried_position, Some(CarriedPosition::RightHand));
            assert!(item_instance_id.is_none());
            assert_eq!(skill_track_id.as_deref(), Some("hand"));
            assert_eq!((*skill_level, *chance_percent, *roll), (Some(8), 40, 1));
            assert_eq!((*effective_combat_add_rating, *armor_encumbrance), (0, 0));
        }
        other => panic!("unexpected outcome: {other:?}"),
    }
    let after = defender(&engine);
    assert_eq!(
        (after.hp, after.mp, after.stamina),
        (before.hp, before.mp, before.stamina)
    );
    assert_eq!(after.timing, before.timing);
    assert_eq!(after.attack_ready_at, before.attack_ready_at);
    assert_eq!(after.carried, before.carried);
    assert_eq!(
        after.character.as_ref().unwrap().skill_ledger,
        before.character.unwrap().skill_ledger
    );
    assert_eq!(engine.world().timing.now, now);
    let attacker = engine.world().actor(&ActorId::from("player")).unwrap();
    assert!(attacker.timing.ready_at > now);
    assert!(!events.iter().any(|event| {
        matches!(event, Event::Attacked { defender_id: id, .. } if *id == defender_id())
    }));
    let projection = engine.observer_projection(&defender_id(), &events).unwrap();
    let outcomes = projection
        .events
        .iter()
        .filter(|event| {
            matches!(event,
                ObservedEventV1::Feedback {
                    cue: ObserverFeedbackCueV1::PhysicalCombat {
                        source: Some(source),
                        target,
                        mode: PhysicalAttackMode::Kick,
                        outcome: ObserverPhysicalOutcomeV1::Blocked,
                        ..
                    }
                } if source.actor_id == "player" && target.actor_id == defender_id()
            )
        })
        .count();
    assert_eq!(outcomes, 1);
}

#[test]
fn actual_item_moves_remove_right_hand_defense_and_stowing_restores_it() {
    for item in ["weathered_staff", "trade_charm"] {
        let mut engine = duel(definition(), "creation/martial_artist", 8, 17);
        place_test_item(&mut engine, item, "defense_test/held");
        equip(&mut engine, "defense_test/held", CarriedPosition::RightHand);
        assert!(candidate(&engine).is_none());
        let mut attacked = engine.clone();
        assert!(hand_block(&attack(&mut attacked, PhysicalAttackMode::Kick)).is_none());
        // Advance the real equipment action deadline; do not reset it by hand.
        let ready_at = defender(&engine).timing.ready_at;
        engine.advance_to(ready_at).unwrap();
        equip(&mut engine, "defense_test/held", CarriedPosition::SackItem1);
        assert_eq!(candidate(&engine).unwrap().chance_percent, 40);
        let roll = next_block_roll(&engine);
        assert_eq!(
            hand_block(&attack(&mut engine, PhysicalAttackMode::Kick)).is_some(),
            roll * 5 < 40
        );
        let carried = &defender(&engine).carried.items;
        assert_eq!(
            carried
                .values()
                .filter(|id| *id == "defense_test/held")
                .count(),
            1
        );
    }
    let mut left = duel(definition(), "creation/martial_artist", 8, 17);
    place_test_item(&mut left, "weathered_staff", "defense_test/left");
    equip(&mut left, "defense_test/left", CarriedPosition::LeftHand);
    assert_eq!(candidate(&left).unwrap().chance_percent, 40);
    assert!(hand_block(&attack(&mut left, PhysicalAttackMode::Kick)).is_some());
}

#[test]
fn test_only_worn_armor_reduces_blocking_and_stowing_restores_the_same_item() {
    let definition = armor_definition();
    let mut successes = 0;
    for seed in 0..20 {
        let mut engine = duel(definition.clone(), "creation/martial_artist", 8, seed);
        place_test_item(&mut engine, ARMOR, "defense_test/armor");
        equip(
            &mut engine,
            "defense_test/armor",
            CarriedPosition::OuterArmor,
        );
        assert_eq!(candidate(&engine).unwrap().chance_percent, 30);
        let events = attack(&mut engine, PhysicalAttackMode::Kick);
        if let Some(Event::AttackBlocked {
            armor_encumbrance,
            chance_percent,
            ..
        }) = hand_block(&events)
        {
            assert_eq!((*armor_encumbrance, *chance_percent), (5, 30));
            successes += 1;
        }
    }
    assert_eq!(successes, 5);
    let mut engine = duel(definition, "creation/martial_artist", 8, 17);
    place_test_item(&mut engine, ARMOR, "defense_test/armor");
    equip(
        &mut engine,
        "defense_test/armor",
        CarriedPosition::OuterArmor,
    );
    let ready_at = defender(&engine).timing.ready_at;
    engine.advance_to(ready_at).unwrap();
    equip(
        &mut engine,
        "defense_test/armor",
        CarriedPosition::SackItem1,
    );
    assert_eq!(candidate(&engine).unwrap().chance_percent, 40);
    let roll = next_block_roll(&engine);
    assert_eq!(
        hand_block(&attack(&mut engine, PhysicalAttackMode::Kick)).is_some(),
        roll * 5 < 40
    );
    let carried = &defender(&engine).carried.items;
    assert!(!carried.contains_key(&CarriedPosition::OuterArmor));
    assert_eq!(carried[&CarriedPosition::SackItem1], "defense_test/armor");
    let checkpoint = engine.export_checkpoint().unwrap();
    Engine::hydrate_checkpoint(engine.definition().clone(), &checkpoint).unwrap();
}

#[test]
fn an_actual_incoming_staff_uses_shared_combat_add_penetration() {
    // The seed's attacker holds the production +1 staff. Seed 3 survives its
    // fumble roll, then rolls six for the hand block. Neither rule is overridden.
    let mut engine = duel(definition(), "creation/martial_artist", 8, 3);
    assert_eq!(candidate(&engine).unwrap().chance_percent, 40);
    let events = attack(&mut engine, PhysicalAttackMode::Fight);
    match hand_block(&events).unwrap() {
        Event::AttackBlocked {
            chance_percent,
            effective_combat_add_rating,
            roll,
            ..
        } => {
            assert_eq!(
                (*chance_percent, *effective_combat_add_rating, *roll),
                (38, 1, 6)
            );
        }
        other => panic!("unexpected outcome: {other:?}"),
    }
}

#[test]
fn explicit_content_cutover_preserves_owned_state_and_replays_new_defense() {
    let mut old_source = catalog();
    old_source["profession_actions"]
        .as_object_mut()
        .unwrap()
        .remove(ACTION)
        .unwrap();
    old_source["profiles"][PROFILE]["profession_actions"] = json!([]);
    let old = build_definition(old_source);
    let new = definition();
    let old_engine = duel(old.clone(), "creation/martial_artist", 8, 17);
    assert!(candidate(&old_engine).is_none());
    let before = old_engine.export_checkpoint().unwrap();
    assert_eq!(
        Engine::hydrate_checkpoint(new.clone(), &before)
            .err()
            .unwrap()
            .message(),
        "checkpoint content identity mismatch"
    );
    let migrated = Engine::migrate_content_checkpoint(
        old.clone(),
        new.clone(),
        &before,
        &CheckpointContentMigration {
            from_definition_sha256: old.content_identity().definition_sha256.clone(),
            to_definition_sha256: new.content_identity().definition_sha256.clone(),
            retire_npcs: Default::default(),
            merge_merchants: Default::default(),
            relocations: Vec::new(),
            initialize_new_topology: false,
        },
    )
    .unwrap();
    let mut live = Engine::hydrate_checkpoint(new.clone(), &migrated).unwrap();
    assert_eq!(live.world(), old_engine.world());
    assert_eq!(candidate(&live).unwrap().chance_percent, 40);
    let before_json: Value = serde_json::from_slice(before.as_bytes()).unwrap();
    let migrated_json: Value = serde_json::from_slice(migrated.as_bytes()).unwrap();
    for key in [
        "world",
        "rng_state",
        "initial_events",
        "schema_version",
        "kind",
    ] {
        assert_eq!(before_json[key], migrated_json[key], "preserve {key}");
    }
    assert_ne!(before_json["content"], migrated_json["content"]);
    let mut replay = Engine::hydrate_checkpoint(new, &migrated).unwrap();
    let events = attack(&mut live, PhysicalAttackMode::Kick);
    assert!(hand_block(&events).is_some());
    assert_eq!(events, attack(&mut replay, PhysicalAttackMode::Kick));
    assert_eq!(
        live.export_checkpoint().unwrap(),
        replay.export_checkpoint().unwrap()
    );
}
