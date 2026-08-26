use crate::support::content_parts::ContentParts;
use tme_rules::{
    ActionBlockedReasonV1, AutomaticActorDecisionV1, AutomaticWaitReasonV1, Coord, Direction,
    DoorStateViewV1, Engine, Event, InspectExitStatus, LogicalTime, MovementStopReason,
    PathPreviewBlockedReasonV1, PathPreviewStepOutcomeV1, PlayerIntent, TransitionKindViewV1,
};

fn occupancy_engine() -> Engine {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.world_template["id"] = serde_json::json!("dh_occupancy");
    *parts.template_levels_source_mut() = serde_json::json!({
        "start": {
            "law_zone": "none",
            "width": 6,
            "height": 3,
            "cells": [
                [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]],
                [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
                [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]]
            ]
        }
    });
    *parts.item_instances_mut() = serde_json::json!({});
    *parts.ground_items_mut() = serde_json::json!([]);
    parts.actor_definition_mut(0)["stats"] =
        serde_json::json!({"hp": 20, "attack": 4, "defense": 2});
    let ally_definition_id = parts.actors_mut()[1]["actor_definition_id"].clone();
    let ally_definition = parts.actor_definition_mut(1);
    ally_definition["name"] = serde_json::json!("Ally");
    ally_definition["stats"] = serde_json::json!({"hp": 8, "attack": 0, "defense": 0});
    ally_definition["social"]["alignment_source"] =
        serde_json::json!({"kind": "inherent", "alignment": "lawful"});
    ally_definition["social"]["behavior"] = serde_json::json!("alignment_creature");
    ally_definition["ai"]["behavior"] = serde_json::json!("hold_ground");
    let mut neutral_definition = ally_definition.clone();
    neutral_definition["id"] = serde_json::json!("actor/test/movement_pathing/neutral");
    neutral_definition["name"] = serde_json::json!("Neutral");
    neutral_definition["social"]["alignment_source"]["alignment"] = serde_json::json!("neutral");
    neutral_definition["social"]["behavior"] = serde_json::json!("passive");
    let mut hostile_definition = ally_definition.clone();
    hostile_definition["id"] = serde_json::json!("actor/test/movement_pathing/hostile");
    hostile_definition["name"] = serde_json::json!("Hostile");
    hostile_definition["social"]["alignment_source"]["alignment"] = serde_json::json!("chaotic");
    let mut sentinel_definition = neutral_definition.clone();
    sentinel_definition["id"] = serde_json::json!("actor/test/movement_pathing/sentinel");
    sentinel_definition["name"] = serde_json::json!("Sentinel");
    parts.push_selected(
        "actor_definitions",
        "actor/test/movement_pathing/neutral",
        neutral_definition,
    );
    parts.push_selected(
        "actor_definitions",
        "actor/test/movement_pathing/hostile",
        hostile_definition,
    );
    parts.push_selected(
        "actor_definitions",
        "actor/test/movement_pathing/sentinel",
        sentinel_definition,
    );
    let actors = parts
        .actors_mut()
        .as_array_mut()
        .expect("first-room actors");
    let mut player = actors[0].clone();
    player["location"]["level"] = serde_json::json!("start");
    player["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    player["carried"]["items"] = serde_json::json!([]);
    let mut ally = actors[1].clone();
    ally["id"] = serde_json::json!("ally");
    ally["actor_definition_id"] = ally_definition_id;
    ally["location"]["level"] = serde_json::json!("start");
    ally["location"]["position"] = serde_json::json!({"x": 2, "y": 1});
    ally["carried"]["items"] = serde_json::json!([]);
    let mut neutral = ally.clone();
    neutral["id"] = serde_json::json!("neutral");
    neutral["actor_definition_id"] = serde_json::json!("actor/test/movement_pathing/neutral");
    let mut hostile = ally.clone();
    hostile["id"] = serde_json::json!("hostile");
    hostile["actor_definition_id"] = serde_json::json!("actor/test/movement_pathing/hostile");
    let mut sentinel = neutral.clone();
    sentinel["id"] = serde_json::json!("sentinel");
    sentinel["actor_definition_id"] = serde_json::json!("actor/test/movement_pathing/sentinel");
    sentinel["location"]["position"] = serde_json::json!({"x": 3, "y": 1});
    *actors = vec![player, ally, neutral, hostile, sentinel];
    parts.engine(7).expect("occupancy engine starts")
}

fn door_parts(
    player_room: &str,
    player_position: Coord,
    monster_room: &str,
    monster_ai: &str,
) -> ContentParts {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.world_template["id"] = serde_json::json!("dh_doors");
    *parts.template_levels_source_mut() = serde_json::json!({
        "start": {
            "law_zone": "none",
            "width": 5, "height": 3,
            "cells": [
                [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]],
                [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
                [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]]
            ]
        },
        "hall": {
            "law_zone": "none",
            "width": 5, "height": 3,
            "cells": [
                [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]],
                [["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
                [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]]
            ]
        }
    });
    parts.world_template["topology"] = serde_json::json!({
        "edge/start/1/2": {
            "at": {"realm": "realm_0", "level": "start", "position": {"x": 2, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "hall", "position": {"x": 1, "y": 1}
            }},
            "kind": {"kind": "door", "initial_state": "closed"},
            "hidden": false
        },
        "edge/hall/1/0": {
            "at": {"realm": "realm_0", "level": "hall", "position": {"x": 0, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "start", "position": {"x": 1, "y": 1}
            }},
            "kind": {"kind": "door", "initial_state": "closed"},
            "hidden": false
        }
    });
    parts.actor_definition_mut(0)["stats"] =
        serde_json::json!({"hp": 20, "attack": 4, "defense": 2});
    parts.actor_definition_mut(1)["name"] = serde_json::json!("Walker");
    parts.actor_definition_mut(1)["stats"] =
        serde_json::json!({"hp": 8, "attack": 0, "defense": 0});
    parts.actor_definition_mut(1)["ai"]["behavior"] = serde_json::json!(monster_ai);
    let actors = parts
        .actors_mut()
        .as_array_mut()
        .expect("first-room actors");
    actors[0]["location"]["level"] = serde_json::json!(player_room);
    actors[0]["location"]["position"] = serde_json::json!(player_position);
    actors[1]["id"] = serde_json::json!("walker");
    actors[1]["location"]["level"] = serde_json::json!(monster_room);
    actors[1]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    parts
}

fn door_engine(
    player_room: &str,
    player_position: Coord,
    monster_room: &str,
    monster_ai: &str,
) -> Engine {
    door_parts(player_room, player_position, monster_room, monster_ai)
        .engine(7)
        .expect("door engine starts")
}

fn door_is_open(engine: &Engine, room: &str, position: Coord) -> bool {
    engine
        .snapshot()
        .realms
        .iter()
        .find(|candidate| candidate.id == "realm_0")
        .and_then(|realm| realm.levels.iter().find(|candidate| candidate.id == room))
        .and_then(|level| level.tiles.iter().find(|tile| tile.position == position))
        .and_then(|tile| tile.transition.as_ref())
        .is_some_and(|transition| {
            transition.kind == TransitionKindViewV1::Door
                && transition.door_state == Some(DoorStateViewV1::Open)
        })
}

#[test]
fn authored_mixed_occupancy_is_valid_and_paths_cross_or_finish_on_it() {
    let mut crossing = occupancy_engine();
    crossing.world_mut().actors[3].attack_ready_at = LogicalTime::new(99);
    let preview = crossing
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::East, Direction::East, Direction::East],
        )
        .expect("preview succeeds");
    assert_eq!(preview.stop_reason, MovementStopReason::FullPathAccepted);
    assert_eq!(preview.final_position.position, Coord { x: 4, y: 1 });
    assert!(
        preview
            .steps
            .iter()
            .all(|step| matches!(step.outcome, PathPreviewStepOutcomeV1::Moved { .. }))
    );
    let events = crossing
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East, Direction::East]),
        )
        .expect("crossing path commits");
    assert_eq!(
        crossing
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .position,
        Coord { x: 4, y: 1 }
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::MovementBlocked { reason, .. } if reason == "occupied"
    )));

    let mut ending = occupancy_engine();
    ending.world_mut().actors[3].attack_ready_at = LogicalTime::new(99);
    let player_attack_ready = ending.world().actors[0].attack_ready_at;
    let hostile_attack_ready = ending.world().actors[3].attack_ready_at;
    ending
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("co-location commits");
    assert_eq!(
        ending
            .world()
            .actors
            .iter()
            .filter(|actor| { actor.is_alive() && actor.location.position == Coord { x: 2, y: 1 } })
            .count(),
        4
    );
    assert_eq!(
        ending.world().actors[0].attack_ready_at,
        player_attack_ready
    );
    assert_eq!(
        ending.world().actors[3].attack_ready_at,
        hostile_attack_ready
    );
}

#[test]
fn inspect_keeps_occupied_exit_walkable_and_lists_every_adjacent_actor() {
    let mut engine = occupancy_engine();
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Inspect)
        .expect("inspection succeeds");
    let inspected = events
        .iter()
        .find_map(|event| match event {
            Event::Inspected {
                exits,
                nearby_actors,
                ..
            } => Some((exits, nearby_actors)),
            _ => None,
        })
        .expect("inspection event exists");
    let east = inspected
        .0
        .iter()
        .find(|exit| exit.direction == Direction::East)
        .expect("east exit exists");
    assert_eq!(east.status, InspectExitStatus::Walkable);
    assert_eq!(
        inspected
            .1
            .iter()
            .filter(|actor| actor.direction == Direction::East)
            .map(|actor| actor.actor_id.as_str())
            .collect::<Vec<_>>(),
        vec!["ally", "neutral", "hostile"]
    );
}

#[test]
fn closed_door_preview_commit_and_action_context_share_one_path() {
    let mut engine = door_engine("start", Coord { x: 1, y: 1 }, "hall", "hold_ground");
    engine.world_mut().actors[1].attack_ready_at = LogicalTime::new(99);
    let context = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context");
    let east = context
        .exits
        .iter()
        .find(|exit| exit.direction == Direction::East)
        .expect("east exit");
    assert!(!east.blocked);
    assert!(east.opens_door);

    let preview = engine
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::East, Direction::East],
        )
        .expect("preview succeeds");
    assert_eq!(preview.stop_reason, MovementStopReason::FullPathAccepted);
    assert!(preview.steps[0].opens_door);
    assert!(matches!(
        preview.steps[0].outcome,
        PathPreviewStepOutcomeV1::Transitioned { .. }
    ));
    assert!(matches!(
        preview.steps[1].outcome,
        PathPreviewStepOutcomeV1::Moved { .. }
    ));
    assert_eq!(preview.final_position.level, "hall");
    assert_eq!(preview.final_position.position, Coord { x: 2, y: 1 });
    assert_eq!(preview.remaining_path_points, 1);

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("path commits");
    let door_index = events
        .iter()
        .position(
            |event| matches!(event, Event::DoorOpened { actor_id, .. } if actor_id == "player"),
        )
        .expect("door-open event");
    let cost_index = events
        .iter()
        .position(|event| matches!(event, Event::MovementCostPaid { destination, .. } if destination.position == Coord { x: 2, y: 1 }))
        .expect("door-step cost event");
    let transition_index = events
        .iter()
        .position(
            |event| matches!(event, Event::WorldTransition { actor_id, .. } if actor_id == "player"),
        )
        .expect("transition event");
    assert!(door_index < cost_index && cost_index < transition_index);
    assert!(door_is_open(&engine, "start", Coord { x: 2, y: 1 }));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .level,
        "hall"
    );
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .position,
        Coord { x: 2, y: 1 }
    );
}

#[test]
fn occupied_transition_target_allows_colocation_and_explicit_close_uses_living_query() {
    let mut transition = door_engine("start", Coord { x: 1, y: 1 }, "hall", "hold_ground");
    transition.world_mut().actors[1].attack_ready_at = LogicalTime::new(99);
    transition
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("occupied transition target accepts movement");
    assert_eq!(
        transition
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .level,
        "hall"
    );
    assert_eq!(
        transition
            .world()
            .actors
            .iter()
            .filter(|actor| actor.is_alive()
                && actor.location.level == "hall"
                && actor.location.position == Coord { x: 1, y: 1 })
            .count(),
        2
    );

    let mut explicit = door_engine("start", Coord { x: 1, y: 1 }, "hall", "hold_ground");
    explicit
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Open(Direction::East),
        )
        .expect("explicit open remains supported");
    explicit.world_mut().actors[1].location.level = "start".to_string();
    explicit.world_mut().actors[1].location.position = Coord { x: 2, y: 1 };
    let error = explicit
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Close(Direction::East),
        )
        .expect_err("living door occupant blocks close");
    assert_eq!(error.message(), "cannot close door: occupied");
    assert!(door_is_open(&explicit, "start", Coord { x: 2, y: 1 }));

    explicit.world_mut().actors[1].life_state = tme_rules::ActorLifeState::Dead;
    explicit
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Close(Direction::East),
        )
        .expect("dead actor does not block close");
    assert!(!door_is_open(&explicit, "start", Coord { x: 2, y: 1 }));
}

#[test]
fn insufficient_budget_leaves_closed_door_unchanged() {
    let mut parts = door_parts("start", Coord { x: 1, y: 1 }, "hall", "hold_ground");
    parts.selected_by_runtime_id_mut("terrains", "flagstone")["navigation"]["move_cost"] =
        serde_json::json!(4);
    let mut engine = parts.engine(7).expect("zero-budget door engine starts");
    let preview = engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .expect("preview succeeds");
    assert_eq!(preview.stop_reason, MovementStopReason::Blocked);
    assert!(!preview.steps[0].opens_door);
    assert!(matches!(
        preview.steps[0].outcome,
        PathPreviewStepOutcomeV1::Blocked {
            reason: PathPreviewBlockedReasonV1::InsufficientMovementPoints
        }
    ));
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("blocked move is a committed action");
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::DoorOpened { .. }))
    );
    assert!(!door_is_open(&engine, "start", Coord { x: 2, y: 1 }));
}

#[test]
fn automatic_actor_does_not_acquire_a_cross_site_door_target() {
    let mut engine = door_engine("hall", Coord { x: 2, y: 1 }, "start", "simple_chase");
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("automatic actor resolves");
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
                    } if actor_id == "walker"
                )
            })
            .count(),
        1
    );
    assert!(
        !events.iter().any(
            |event| matches!(event, Event::DoorOpened { actor_id, .. } if actor_id == "walker")
        )
    );
    assert!(!events.iter().any(
        |event| matches!(event, Event::WorldTransition { actor_id, .. } if actor_id == "walker")
    ));
    assert!(!door_is_open(&engine, "start", Coord { x: 2, y: 1 }));
    let walker = engine
        .world()
        .actor(&tme_rules::ActorId::from("walker"))
        .expect("walker remains present");
    assert_eq!(walker.location.level, "start");
    assert_eq!(walker.location.position, Coord { x: 1, y: 1 });
}

#[test]
fn repeated_preview_changes_neither_state_nor_rng_visible_outcome() {
    let mut previewed = door_engine("start", Coord { x: 1, y: 1 }, "hall", "hold_ground");
    previewed.world_mut().actors[1].attack_ready_at = LogicalTime::new(99);
    let mut control = previewed.clone();
    let before = previewed.snapshot();
    let first = previewed
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::East, Direction::East],
        )
        .expect("first preview");
    let second = previewed
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::East, Direction::East],
        )
        .expect("second preview");
    assert_eq!(first, second);
    assert_eq!(previewed.snapshot(), before);

    let previewed_events = previewed
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("previewed engine commits");
    let control_events = control
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("control engine commits");
    assert_eq!(previewed_events, control_events);
    assert_eq!(previewed.snapshot(), control.snapshot());
}

#[test]
fn required_contract_fields_have_direct_typed_values() {
    let engine = door_engine("start", Coord { x: 1, y: 1 }, "hall", "hold_ground");
    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("observed context");
    let east = context
        .exits
        .iter()
        .find(|exit| exit.direction == Direction::East)
        .expect("east exit");
    assert!(east.opens_door);
    assert_eq!(east.blocked_reason, None::<ActionBlockedReasonV1>);
}

#[test]
fn positive_stamina_does_not_truncate_an_ordinary_run() {
    let mut parts = ContentParts::tracked("character_sheet", "profile/character_sheet");
    parts.actors_mut()[0]["character"]["resources"]["stamina"] = serde_json::json!(1);
    let mut engine = parts.engine(7).expect("character engine starts");
    engine.world_mut().actors[1].attack_ready_at = LogicalTime::new(99);

    let preview = engine
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::East, Direction::East],
        )
        .expect("preview succeeds");
    assert_eq!(preview.stop_reason, MovementStopReason::FullPathAccepted);
    assert_eq!(preview.steps.len(), 2);
    assert_eq!(preview.final_position.position, Coord { x: 3, y: 1 });

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("full run commits");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .position,
        Coord { x: 3, y: 1 }
    );
}
