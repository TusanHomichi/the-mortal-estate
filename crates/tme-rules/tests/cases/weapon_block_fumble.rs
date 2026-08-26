use crate::support::content_parts::ContentParts;
use serde_json::json;
use tme_rules::{
    BlockSourceKind, CarriedPosition, DeterministicRng, Engine, Event, ItemRelocationReason,
    PlayerIntent, WeaponFumbleReason,
};

fn base() -> ContentParts {
    let mut value = ContentParts::tracked("first_room", "profile/first_room");
    let position = value.actors_mut()[0]["location"]["position"].clone();
    value.actors_mut()[1]["location"]["position"] = position;
    value
}

fn engine(value: ContentParts, seed: u64) -> Engine {
    value.engine(seed).expect("engine")
}

fn character_backed_base() -> ContentParts {
    let mut value = base();
    let mut progression = ContentParts::tracked("skill_progression", "profile/skill_progression");
    value.actors_mut()[0]["character_id"] = json!("character:fumbler");
    value.actor_definition_mut(0)["social"]["alignment_source"] = json!({"kind": "character"});
    value.actors_mut()[0]["character"] = progression.actors_mut()[0]["character"].clone();
    value
}

fn two_block_sources() -> ContentParts {
    let mut value = base();
    value.rules_source_mut()["combat"]["block"]["shield_percent_cap"] = json!(100);
    value.actors_mut()[0]["carried"]["items"] = json!([]);
    *value.ground_items_mut() = json!([{
        "item_instance_id":"training_knife",
        "location":{"realm":"realm_0","level":"room_0","position":{"x":1,"y":1}}
    }]);
    value.push_selected(
        "items",
        "item/guard/weapon_block_fumble",
        json!({
            "id":"guard","kind":"shield","name":"Guard","valid_placements":["hand"],
            "capability":{"block_value":10},"economy":{"unit_burden":0}
        }),
    );
    value.push_selected(
        "items",
        "item/parry_blade/weapon_block_fumble",
        json!({
            "id":"parry_blade","kind":"weapon","name":"Parry Blade","valid_placements":["hand"],
            "weapon":{"skill_track_id":"sword","default_attack_mode": "fight","attack_modes": [{"mode": "fight", "maximum_range": 0, "damage_kind": "cutting"}],"cooldown_units":1,"combat_add_rating":0,"handedness":"one_handed","block_value":10},
            "economy":{"unit_burden":0}
        }),
    );
    value.item_instances_mut()["guard"] =
        json!({"definition_id":"guard","binding":{"state":"unrestricted"}});
    value.item_instances_mut()["parry_blade"] =
        json!({"definition_id":"parry_blade","binding":{"state":"unrestricted"}});
    value.actors_mut()[1]["carried"]["items"] = json!([
        {"item_instance_id":"guard","position":"left_hand"},
        {"item_instance_id":"parry_blade","position":"right_hand"}
    ]);
    value
}

#[test]
fn left_right_selection_is_exactly_authored_seventy_five_twenty_five() {
    let snapshot = engine(two_block_sources(), 0).snapshot();
    let candidates = &snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "mireling")
        .unwrap()
        .physical_weapon
        .as_ref()
        .unwrap()
        .eligible_block_candidates;
    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates[0].carried_position,
        Some(CarriedPosition::LeftHand)
    );
    assert_eq!(candidates[0].block_value, 10);
    assert_eq!(candidates[0].skill_track_id, None);
    assert_eq!(candidates[0].skill_level, None);
    assert_eq!(
        candidates[1].carried_position,
        Some(CarriedPosition::RightHand)
    );
    assert_eq!(candidates[1].block_value, 10);
    assert_eq!(candidates[1].skill_track_id.as_deref(), Some("sword"));
    assert_eq!(candidates[1].skill_level, Some(0));

    let mut left = 0;
    let mut right = 0;
    for seed in 0..100 {
        let mut expected_rng = DeterministicRng::new(seed);
        let expected_source = match expected_rng.weighted_index(&[75, 25]).unwrap() {
            0 => {
                left += 1;
                BlockSourceKind::LeftShield
            }
            1 => {
                right += 1;
                BlockSourceKind::RightWeapon
            }
            other => panic!("unexpected selected index {other}"),
        };
        let expected_roll = expected_rng.roll_d20();
        let events = engine(two_block_sources(), seed)
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::PhysicalAttack {
                    authorization: tme_rules::HostilityAuthorization::Safe,
                    mode: tme_rules::PhysicalAttackMode::Fight,
                    target_actor_id: "mireling".into(),
                },
            )
            .expect("attack");
        let block = events.iter().find_map(|event| match event {
            Event::AttackBlocked { source, .. } => Some(*source),
            _ => None,
        });
        if expected_roll < 20 {
            assert_eq!(block, Some(expected_source));
        } else {
            assert_eq!(
                block, None,
                "strict 100-percent boundary excludes d20 roll 20"
            );
        }
    }
    assert_eq!((left, right), (75, 25));
}

#[test]
fn singleton_block_selection_does_not_consume_an_extra_rng_transition() {
    let mut value = two_block_sources();
    value.actors_mut()[1]["carried"]["items"]
        .as_array_mut()
        .unwrap()
        .retain(|row| row["position"] == "left_hand");
    value
        .ground_items_mut()
        .as_array_mut()
        .unwrap()
        .push(json!({
            "item_instance_id":"parry_blade",
            "location":{"realm":"realm_0","level":"room_0","position":{"x":1,"y":1}}
        }));
    let seed = 7;
    let mut expected = DeterministicRng::new(seed);
    let expected_roll = expected.roll_d20();
    let events = engine(value, seed)
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::AttackBlocked {
            source: BlockSourceKind::LeftShield,
            roll,
            ..
        } if *roll == expected_roll
    )));
}

#[test]
fn general_nonbow_fumble_drops_and_practices_the_captured_track() {
    let value = character_backed_base();
    let events = engine(value, 17)
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("fumble");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::WeaponFumbled {
            reason: WeaponFumbleReason::General,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ItemRelocated {
            reason: ItemRelocationReason::WeaponFumble,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SkillPracticeAwarded { track_id, .. } if track_id == "dagger"
    )));
}

#[test]
fn general_bow_fumble_unloads_without_relocating() {
    let value = ContentParts::tracked("ranged_attack", "profile/ranged_attack");
    let mut engine = engine(value, 17);
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Nock)
        .expect("nock");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Shoot,
                target_actor_id: "reedling".into(),
            },
        )
        .expect("bow fumble");
    let fumble_index = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::WeaponFumbled {
                    reason: WeaponFumbleReason::General,
                    result: tme_rules::WeaponFumbleResult::BowUnnocked,
                    ..
                }
            )
        })
        .expect("bow fumble event");
    let readiness_index = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::BowReadinessChanged {
                    reason: tme_rules::BowReadinessChangeReason::Fumble,
                    to: tme_rules::BowReadiness::Unnocked,
                    ..
                }
            )
        })
        .expect("bow readiness event");
    assert!(fumble_index < readiness_index);
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::ItemRelocated {
            reason: ItemRelocationReason::WeaponFumble,
            ..
        }
    )));
    assert_eq!(
        engine.world().item_instances["elm_bow"].bow_readiness,
        Some(tme_rules::BowReadiness::Unnocked)
    );
}

#[test]
fn deterministic_binding_fumble_uses_no_general_rng_and_awards_no_practice() {
    let mut value = base();
    let mut progression = ContentParts::tracked("skill_progression", "profile/skill_progression");
    value.actors_mut()[0]["character_id"] = json!("character:other");
    value.actor_definition_mut(0)["social"]["alignment_source"] = json!({"kind": "character"});
    value.actors_mut()[0]["character"] = progression.actors_mut()[0]["character"].clone();
    value.item_instances_mut()["training_knife"]["binding"] =
        json!({"state":"bound","character_id":"character:owner"});
    let events = engine(value, 7)
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("restriction fumble");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::WeaponFumbled {
            reason: WeaponFumbleReason::TiedToOtherCharacter,
            ..
        }
    )));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::SkillPracticeAwarded { .. }))
    );
}

#[test]
fn deterministic_alignment_fumble_uses_the_shared_owner_without_practice() {
    let mut value = character_backed_base();
    value.selected_mut("items", 0)["weapon"]["required_alignment"] = json!("chaotic");
    let events = engine(value, 17)
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("alignment restriction fumble");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::WeaponFumbled {
            reason: WeaponFumbleReason::AlignmentMismatch,
            result: tme_rules::WeaponFumbleResult::Dropped,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ItemRelocated {
            reason: ItemRelocationReason::WeaponFumble,
            ..
        }
    )));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::SkillPracticeAwarded { .. }))
    );
}

#[test]
fn late_practice_failure_rolls_back_fumble_location_time_state_and_rng() {
    let value = character_backed_base();
    let mut rolled_back = engine(value.clone(), 17);
    rolled_back.world_mut().actors[0]
        .character
        .as_mut()
        .expect("character")
        .skill_ledger
        .push(tme_rules::SkillEntry {
            track_id: "dagger".to_string(),
            level: 0,
            critique_rank: 0,
            practice_points: u64::MAX,
            learning_rate: 1,
        });
    let before_snapshot = rolled_back.snapshot();
    let before_location = rolled_back
        .item_location("training_knife")
        .expect("weapon location before fumble");

    let error = rolled_back
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect_err("practice overflow should fail after fumble mutation");
    assert!(error.message().contains("practice"), "{error:?}");
    assert_eq!(rolled_back.snapshot(), before_snapshot);
    assert_eq!(
        rolled_back
            .item_location("training_knife")
            .expect("rolled-back weapon location"),
        before_location
    );

    rolled_back.world_mut().actors[0]
        .character
        .as_mut()
        .expect("character")
        .skill_ledger[0]
        .practice_points = 0;
    let retried = rolled_back
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("retry after clearing overflow");

    let mut fresh = engine(value, 17);
    fresh.world_mut().actors[0]
        .character
        .as_mut()
        .expect("character")
        .skill_ledger
        .push(tme_rules::SkillEntry::untrained("dagger", 1));
    let fresh_events = fresh
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("fresh fumble");
    assert_eq!(retried, fresh_events, "retry must replay the restored RNG");
    assert_eq!(rolled_back.snapshot(), fresh.snapshot());
}
