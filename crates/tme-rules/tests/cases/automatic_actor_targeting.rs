use crate::ai_support::{
    automatic_actor, decision, engine, engine_from_value, open_room_value, unrestricted, wait,
};
use tme_rules::{AutomaticActorDecisionV1, AutomaticMovementPurposeV1, Coord, Direction};

#[test]
fn chaotic_alignment_creature_simple_chase_targets_lawful_player() {
    let mut engine = engine(vec![automatic_actor(
        "chaser",
        "chaotic",
        Coord { x: 5, y: 2 },
        "simple_chase",
        1,
        unrestricted(),
        &["fight"],
    )]);
    let events = wait(&mut engine);
    assert_eq!(
        decision(&events, "chaser"),
        &AutomaticActorDecisionV1::Move {
            direction: Direction::West,
            purpose: AutomaticMovementPurposeV1::Chase,
        }
    );
}

#[test]
fn lawful_alignment_creature_has_no_legacy_team_hostility() {
    let mut engine = engine(vec![
        automatic_actor(
            "guardian",
            "lawful",
            Coord { x: 3, y: 2 },
            "simple_chase",
            1,
            unrestricted(),
            &["fight"],
        ),
        automatic_actor(
            "raider",
            "chaotic",
            Coord { x: 7, y: 2 },
            "hold_ground",
            1,
            unrestricted(),
            &["fight"],
        ),
    ]);
    let events = wait(&mut engine);
    assert_eq!(
        decision(&events, "guardian"),
        &AutomaticActorDecisionV1::Wait {
            reason: tme_rules::AutomaticWaitReasonV1::Watch,
        }
    );
}

#[test]
fn passive_neutral_observer_does_not_select_targets() {
    let actors = vec![
        automatic_actor(
            "watcher",
            "chaotic",
            Coord { x: 4, y: 2 },
            "simple_chase",
            1,
            unrestricted(),
            &["fight"],
        ),
        automatic_actor(
            "packmate",
            "chaotic",
            Coord { x: 5, y: 2 },
            "hold_ground",
            1,
            unrestricted(),
            &["fight"],
        ),
        automatic_actor(
            "neutral",
            "neutral",
            Coord { x: 3, y: 2 },
            "hold_ground",
            1,
            unrestricted(),
            &["fight"],
        ),
    ];
    let mut value = open_room_value(actors);
    value.actor_definition_mut(1)["social"]["alignment_source"] =
        serde_json::json!({"kind": "inherent", "alignment": "neutral"});
    value.actor_definition_mut(1)["social"]["behavior"] = serde_json::json!("passive");
    let mut engine = engine_from_value(value);
    let events = wait(&mut engine);
    assert_eq!(
        decision(&events, "watcher"),
        &AutomaticActorDecisionV1::Wait {
            reason: tme_rules::AutomaticWaitReasonV1::Watch,
        }
    );
}

#[test]
fn closer_non_player_hostile_wins_over_farther_player() {
    let mut engine = engine(vec![
        automatic_actor(
            "chooser",
            "chaotic",
            Coord { x: 5, y: 2 },
            "simple_chase",
            1,
            unrestricted(),
            &["fight"],
        ),
        automatic_actor(
            "guardian",
            "lawful",
            Coord { x: 6, y: 2 },
            "hold_ground",
            1,
            unrestricted(),
            &["fight"],
        ),
    ]);
    let events = wait(&mut engine);
    assert_eq!(
        decision(&events, "chooser"),
        &AutomaticActorDecisionV1::Move {
            direction: Direction::East,
            purpose: AutomaticMovementPurposeV1::Chase,
        }
    );
}

#[test]
fn same_distance_tie_uses_registration_order() {
    let mut engine = engine(vec![
        automatic_actor(
            "chooser",
            "chaotic",
            Coord { x: 4, y: 2 },
            "simple_chase",
            1,
            unrestricted(),
            &["fight"],
        ),
        automatic_actor(
            "first",
            "lawful",
            Coord { x: 3, y: 1 },
            "hold_ground",
            1,
            unrestricted(),
            &["fight"],
        ),
        automatic_actor(
            "second",
            "lawful",
            Coord { x: 5, y: 1 },
            "hold_ground",
            1,
            unrestricted(),
            &["fight"],
        ),
    ]);
    let events = wait(&mut engine);
    assert_eq!(
        decision(&events, "chooser"),
        &AutomaticActorDecisionV1::Move {
            direction: Direction::Northwest,
            purpose: AutomaticMovementPurposeV1::Chase,
        }
    );
}

#[test]
fn automatic_actor_can_target_hostile_creature_when_player_is_neutral() {
    let actors = vec![
        automatic_actor(
            "hunter",
            "chaotic",
            Coord { x: 6, y: 2 },
            "simple_chase",
            1,
            unrestricted(),
            &["fight"],
        ),
        automatic_actor(
            "guardian",
            "lawful",
            Coord { x: 4, y: 2 },
            "hold_ground",
            1,
            unrestricted(),
            &["fight"],
        ),
    ];
    let mut value = open_room_value(actors);
    value.actor_definition_mut(0)["social"]["alignment_source"] =
        serde_json::json!({"kind": "inherent", "alignment": "neutral"});
    let mut engine = engine_from_value(value);
    let events = wait(&mut engine);
    assert_eq!(
        decision(&events, "hunter"),
        &AutomaticActorDecisionV1::Move {
            direction: Direction::West,
            purpose: AutomaticMovementPurposeV1::Chase,
        }
    );
}
