use crate::spell_support::*;
use crate::support::content_parts::ContentParts;
use tme_rules::*;

fn wizard_room_spell_engine(known_spell_ids: &[&str]) -> Engine {
    wizard_multi_spell_engine_with_content_mutate(
        known_spell_ids,
        1,
        vec!["####", "#..#", "####"],
        Coord { x: 1, y: 1 },
        |parts| {
            parts.template_levels_source_mut()["side"] = serde_json::json!({
                "law_zone": "none",
                "width": 4,
                "height": 3,
                "cells": [
                    [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]],
                    [["stone_wall"], ["flagstone"], ["flagstone"], ["stone_wall"]],
                    [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]]
                ]
            });
            parts.actors_mut()[0]["location"]["level"] = serde_json::json!("room_0");
            parts.actors_mut()[1]["location"]["level"] = serde_json::json!("side");
        },
    )
}

#[test]
fn actor_spell_rejects_cross_room_target_without_range_or_visibility_gate() {
    let mut engine = wizard_room_spell_engine(&["soft_mark"]);
    let err = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "soft_mark".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("cross-room actor target should be rejected");

    assert!(err.to_string().contains("invalid_target"));
}

#[test]
fn coordinate_spell_rejects_out_of_bounds_target_before_range_or_visibility_gate() {
    let mut engine = wizard_spell_engine(Some("mark_coordinate"), 1);
    let err = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "mark_coordinate".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: tme_rules::WorldPosition::new(
                        "realm_0",
                        "room_0",
                        Coord { x: 99, y: 99 },
                    ),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("out-of-bounds coordinate target should be rejected");

    assert!(err.to_string().contains("invalid_target"));
}

#[test]
fn area_spell_rejects_out_of_bounds_center_before_range_or_visibility_gate() {
    let mut engine = wizard_spell_engine(Some("mark_area"), 1);
    let err = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "mark_area".to_string(),
                target: Some(SpellTarget::Area {
                    center: tme_rules::WorldPosition::new(
                        "realm_0",
                        "room_0",
                        Coord { x: -1, y: 1 },
                    ),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("out-of-bounds area center should be rejected");

    assert!(err.to_string().contains("invalid_target"));
}

#[test]
fn coordinate_spell_rejects_cross_room_target_without_range_or_visibility_gate() {
    let mut engine = wizard_room_spell_engine(&["mark_coordinate"]);
    let err = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "mark_coordinate".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: tme_rules::WorldPosition::new(
                        "realm_0",
                        "side",
                        Coord { x: 1, y: 1 },
                    ),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("cross-room coordinate target should be rejected");

    assert!(err.to_string().contains("invalid_target"));
}

#[test]
fn area_spell_rejects_cross_room_center_without_range_or_visibility_gate() {
    let mut engine = wizard_room_spell_engine(&["mark_area"]);
    let err = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "mark_area".to_string(),
                target: Some(SpellTarget::Area {
                    center: tme_rules::WorldPosition::new("realm_0", "side", Coord { x: 1, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("cross-room area center should be rejected");

    assert!(err.to_string().contains("invalid_target"));
}

#[test]
fn cast_unknown_spell_is_rejected() {
    let mut engine = wizard_spell_engine(Some("spark"), 1);
    let result = engine.apply_actor_intent(
        &tme_rules::ActorId::from("player"),
        PlayerIntent::CastSpell {
            spell_id: "unknown_spell".to_string(),
            target: None,
            authorization: tme_rules::HostilityAuthorization::Safe,
        },
    );
    assert!(result.is_err(), "unknown spell should be rejected");
}

#[test]
fn cast_spell_not_in_spellbook_is_rejected() {
    // Player knows "spark" but tries to cast "mend" which is not in known_spells
    let mut engine = wizard_spell_engine(Some("spark"), 1);
    let result = engine.apply_actor_intent(
        &tme_rules::ActorId::from("player"),
        PlayerIntent::CastSpell {
            spell_id: "mend".to_string(),
            target: None,
            authorization: tme_rules::HostilityAuthorization::Safe,
        },
    );
    assert!(result.is_err(), "spell not in spellbook should be rejected");
}

#[test]
fn actor_spell_rejects_invalid_and_missing_targets() {
    let mut missing = wizard_spell_engine(Some("spark"), 1);
    let err = missing
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "spark".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "ghost".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("missing target rejected");
    assert!(err.to_string().contains("invalid_target"));

    let mut targetless = wizard_spell_engine(Some("spark"), 1);
    let err = targetless
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "spark".to_string(),
                target: None,
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("actor spell without an explicit target is blocked");
    assert!(err.to_string().contains("invalid_target"));

    let mut invisible = wizard_spell_engine_with_layout(
        Some("spark"),
        1,
        vec!["#####", "#.#.#", "#####"],
        Coord { x: 3, y: 1 },
    );
    let err = invisible
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
        .expect_err("wall-blocked target rejected");
    assert!(err.to_string().contains("target_not_visible"));

    let mut outside_observation = wizard_spell_engine_with_layout(
        Some("spark"),
        1,
        vec!["###########", "#.........#", "###########"],
        Coord { x: 9, y: 1 },
    );
    let err = outside_observation
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
        .expect_err("target beyond the player observation window is rejected");
    assert!(err.to_string().contains("target_not_visible"));
}

#[test]
fn self_spell_accepts_self_target_and_rejects_actor_target() {
    let mut engine = wizard_spell_engine(Some("mend"), 2);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "mend".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("self target accepted");

    let mut rejected = wizard_spell_engine(Some("mend"), 2);
    let err = rejected
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "mend".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("actor target rejected for self spell");
    assert!(err.to_string().contains("invalid_target"));
}

fn path_command(spell_id: &str, directions: Vec<Direction>) -> PlayerCommandV1 {
    PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::CastSpell {
            spell_id: spell_id.to_string(),
            target: Some(SpellTarget::Path { directions }),
            authorization: tme_rules::HostilityAuthorization::Safe,
        },
    }
}

fn disable_recovery(parts: &mut ContentParts) {
    parts.rules_source_mut()["resources"]["recovery_interval_units"] = serde_json::json!(u32::MAX);
}

#[test]
fn direct_visible_path_cast_spends_once_without_moving_and_awards_practice() {
    let mut engine =
        wizard_spell_engine_with_content_mutate(Some("path_mark"), 1, disable_recovery);
    let player_before = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .clone();

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "path_mark".to_string(),
                target: Some(SpellTarget::Path {
                    directions: vec![Direction::East],
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("visible path should cast");

    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    assert_eq!(player.location.position, player_before.location.position);
    assert_eq!(player.mp, player_before.mp - 3);
    assert_eq!(player.stamina, player_before.stamina - 1);
    assert_eq!(count_skill_practice(&events.events, "wizard_magic"), 1);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellCastStubbed {
            spell_id,
            target: Some(SpellTarget::Path { directions }),
            ..
        } if spell_id == "path_mark" && directions == &[Direction::East]
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::SpellCastFailed { .. } | Event::SpellDamaged { .. }
    )));
}

#[test]
fn empty_missing_and_wrong_shape_paths_reject_before_commit() {
    for target in [
        None,
        Some(SpellTarget::Path {
            directions: Vec::new(),
        }),
        Some(SpellTarget::Coordinate {
            position: WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 1 }),
        }),
    ] {
        let mut engine =
            wizard_spell_engine_with_content_mutate(Some("path_mark"), 1, disable_recovery);
        let before = engine.snapshot();
        let error = engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::CastSpell {
                    spell_id: "path_mark".to_string(),
                    target,
                    authorization: tme_rules::HostilityAuthorization::Safe,
                },
            )
            .expect_err("incomplete path must reject");
        assert_eq!(error.to_string(), "invalid_target");
        assert_eq!(engine.snapshot(), before);
    }
}

fn assert_committed_path_failure(
    mut engine: Engine,
    directions: Vec<Direction>,
    expected_reason: SpellPathFailureReason,
) {
    let before = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .clone();
    let command = path_command("path_mark", directions.clone());
    let status = engine
        .validate_actor_command(&command)
        .expect("path command status");
    assert!(status.accepted, "semantic path failure is a valid command");

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "path_mark".to_string(),
                target: Some(SpellTarget::Path {
                    directions: directions.clone(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("semantic path failure commits");
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    assert_eq!(player.location.position, before.location.position);
    assert_eq!(player.mp, before.mp - 3);
    assert_eq!(player.stamina, before.stamina - 1);
    assert_eq!(count_skill_practice(&events.events, "wizard_magic"), 0);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellCastFailed {
            spell_id,
            target: Some(SpellTarget::Path { directions: actual }),
            failure: SpellCastFailure::InvalidPath { reason },
            mp_cost: Some(3),
            stamina_cost: Some(1),
            ..
        } if spell_id == "path_mark" && actual == &directions && *reason == expected_reason
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::SpellCastStubbed { .. }
            | Event::SpellDamaged { .. }
            | Event::SkillPracticeAwarded { .. }
    )));
}

#[test]
fn nonempty_path_failures_commit_with_typed_first_failure() {
    assert_committed_path_failure(
        wizard_spell_engine_with_content_mutate(Some("path_mark"), 1, disable_recovery),
        vec![Direction::West, Direction::West],
        SpellPathFailureReason::OutOfBounds,
    );
    assert_committed_path_failure(
        wizard_multi_spell_engine_with_content_mutate(
            &["path_mark"],
            1,
            vec!["#####", "#.#.#", "#####"],
            Coord { x: 3, y: 1 },
            disable_recovery,
        ),
        vec![Direction::East, Direction::East],
        SpellPathFailureReason::NotVisible,
    );
    assert_committed_path_failure(
        wizard_multi_spell_engine_with_content_mutate(
            &["path_mark"],
            1,
            vec!["#######", "#.....#", "#######"],
            Coord { x: 5, y: 1 },
            disable_recovery,
        ),
        vec![Direction::East, Direction::East, Direction::East],
        SpellPathFailureReason::OutOfRange,
    );
}

#[test]
fn failed_warmed_path_consumes_slot_and_cast_cost_once() {
    let mut engine =
        wizard_spell_engine_with_content_mutate(Some("charged_path"), 1, disable_recovery);
    let initial_mp = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .mp;
    let initial_stamina = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .stamina;
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_path".to_string(),
            },
        )
        .expect("warm path spell");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .mp,
        initial_mp
    );

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastWarmedSpell {
                target: Some(SpellTarget::Path {
                    directions: vec![Direction::West, Direction::West],
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("invalid warmed path is committed");
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    assert_eq!(player.mp, initial_mp - 4);
    assert_eq!(player.stamina, initial_stamina - 2);
    assert!(player.warmed_spell.is_none());
    assert_eq!(count_skill_practice(&events.events, "wizard_magic"), 0);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellCastFailed {
            failure: SpellCastFailure::InvalidPath {
                reason: SpellPathFailureReason::OutOfBounds
            },
            ..
        }
    )));
}

#[test]
fn committed_path_failure_does_not_consume_rng() {
    let mut failed = wizard_multi_spell_engine_with_content_mutate(
        &["path_mark"],
        1,
        vec!["####", "#..#", "####"],
        Coord { x: 1, y: 1 },
        disable_recovery,
    );
    let mut control = failed.clone();

    failed
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "path_mark".to_string(),
                target: Some(SpellTarget::Path {
                    directions: vec![Direction::West, Direction::West],
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("path failure");
    control
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("control wait");
    *control.world_mut() = failed.world().clone();

    let intent = PlayerIntent::PhysicalAttack {
        authorization: tme_rules::HostilityAuthorization::Safe,
        mode: PhysicalAttackMode::Fight,
        target_actor_id: "target".into(),
    };
    let actual = failed
        .apply_actor_intent(&tme_rules::ActorId::from("player"), intent.clone())
        .expect("attack after path");
    let expected = control
        .apply_actor_intent(&tme_rules::ActorId::from("player"), intent)
        .expect("control attack");
    assert_eq!(actual, expected, "path evaluation must not advance RNG");
}
