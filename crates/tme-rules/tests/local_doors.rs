//! Local dungeon-corridor doors: authored `local_door` topology, strict
//! decoding, and runtime self-target movement.

#[path = "support/mod.rs"]
mod support;

use crate::support::content_parts::ContentParts;
use serde_json::json;
use tme_rules::{
    ActorId, Coord, Direction, DoorStateViewV1, Engine, Event, MovementStopReason,
    PathPreviewStepOutcomeV1, PlayerIntent, TransitionKindViewV1,
};

fn local_door_parts(hidden: bool, initial_state: &str) -> ContentParts {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.world_template["id"] = json!("local_door");
    *parts.template_levels_source_mut() = json!({
        "corridor": {
            "law_zone": "none",
            "width": 5,
            "height": 3,
            "cells": [
                [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]],
                [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
                [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]]
            ]
        }
    });
    parts.world_template["arrivals"] = json!({});
    parts.world_template["topology"] = json!({
        "edge/corridor/2/1": {
            "at": {"realm": "realm_0", "level": "corridor", "position": {"x": 2, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "corridor", "position": {"x": 2, "y": 1}
            }},
            "kind": {"kind": "local_door", "initial_state": initial_state},
            "hidden": hidden
        }
    });
    let actors = parts
        .actors_mut()
        .as_array_mut()
        .expect("first-room actors");
    actors[0]["location"]["level"] = json!("corridor");
    actors[0]["location"]["position"] = json!({"x": 1, "y": 1});
    actors[0]["carried"]["items"] = json!([]);
    actors[1]["id"] = json!("walker");
    actors[1]["location"]["level"] = json!("corridor");
    actors[1]["location"]["position"] = json!({"x": 3, "y": 1});
    parts.actor_definition_mut(1)["ai"]["behavior"] = json!("hold_ground");
    *parts.item_instances_mut() = json!({});
    *parts.ground_items_mut() = json!([]);
    parts
}

fn local_door_engine(hidden: bool, initial_state: &str) -> Engine {
    local_door_parts(hidden, initial_state)
        .engine(7)
        .expect("local door content should start")
}

/// Wider corridor (floor `x=1..4`, rows `y=1..3`) so a three-step sprint can
/// finish past the door. The walker sits off the tested row.
fn wide_local_door_parts(door_x: i32, initial_state: &str) -> ContentParts {
    let mut parts = local_door_parts(false, initial_state);
    *parts.template_levels_source_mut() = json!({
        "corridor": {
            "law_zone": "none",
            "width": 6,
            "height": 5,
            "cells": [
                [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]],
                [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
                [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
                [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
                [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]]
            ]
        }
    });
    let mut topology = serde_json::Map::new();
    topology.insert(
        format!("edge/corridor/{door_x}/1"),
        json!({
            "at": {"realm": "realm_0", "level": "corridor", "position": {"x": door_x, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "corridor", "position": {"x": door_x, "y": 1}
            }},
            "kind": {"kind": "local_door", "initial_state": initial_state},
            "hidden": false
        }),
    );
    parts.world_template["topology"] = serde_json::Value::Object(topology);
    let actors = parts
        .actors_mut()
        .as_array_mut()
        .expect("first-room actors");
    actors[1]["location"]["position"] = json!({"x": 1, "y": 2});
    parts
}

fn wide_local_door_engine(door_x: i32, initial_state: &str) -> Engine {
    wide_local_door_parts(door_x, initial_state)
        .engine(7)
        .expect("wide local door content should start")
}

fn movement_started(events: &[Event]) -> (usize, MovementStopReason) {
    events
        .iter()
        .find_map(|event| match event {
            Event::MovementStarted {
                accepted_steps,
                stop_reason,
                ..
            } => Some((*accepted_steps, *stop_reason)),
            _ => None,
        })
        .expect("movement started event")
}

fn local_door_is_open(engine: &Engine, position: Coord) -> bool {
    engine
        .snapshot()
        .realms
        .iter()
        .find(|realm| realm.id == "realm_0")
        .and_then(|realm| realm.levels.iter().find(|level| level.id == "corridor"))
        .and_then(|level| level.tiles.iter().find(|tile| tile.position == position))
        .and_then(|tile| tile.transition.as_ref())
        .is_some_and(|transition| {
            transition.kind == TransitionKindViewV1::Door
                && transition.door_state == Some(DoorStateViewV1::Open)
        })
}

fn player_position(engine: &Engine) -> tme_rules::WorldPosition {
    engine
        .world()
        .actor(&ActorId::from("player"))
        .expect("player")
        .location
        .clone()
}

fn definition_error(parts: &ContentParts) -> String {
    parts
        .definition()
        .expect_err("definition mutation must fail")
        .to_string()
}

fn assert_has(error: &str, expected: &str) {
    assert!(
        error.contains(expected),
        "expected {expected:?} in diagnostic:\n{error}"
    );
}

#[test]
fn local_door_kind_decodes_strictly_and_refuses_foreign_fields() {
    let kind: tme_rules::TopologyKindDef =
        serde_json::from_value(json!({"kind": "local_door", "initial_state": "closed"}))
            .expect("canonical local_door decodes");
    assert_eq!(
        kind,
        tme_rules::TopologyKindDef::LocalDoor {
            initial_state: tme_rules::DoorStateDef::Closed
        }
    );

    let error = serde_json::from_value::<tme_rules::TopologyKindDef>(json!({
        "kind": "local_door",
        "initial_state": "closed",
        "binding_id": "door_binding/foreign"
    }))
    .expect_err("foreign fields must be refused");
    assert!(
        error.to_string().contains("unknown field"),
        "unexpected strict-decoder diagnostic: {error}"
    );

    let error = serde_json::from_value::<tme_rules::TopologyKindDef>(json!({
        "kind": "local_door",
        "initial_state": "ajar"
    }))
    .expect_err("unknown door state must be refused");
    assert!(
        error.to_string().contains("unknown variant"),
        "unexpected door-state diagnostic: {error}"
    );
}

#[test]
fn local_door_self_target_is_accepted() {
    let parts = local_door_parts(false, "closed");
    parts
        .definition()
        .expect("a local door targeting its own edge position is valid");
}

#[test]
fn local_door_rejects_nonlocal_and_arrival_targets() {
    let mut nonlocal = local_door_parts(false, "closed");
    nonlocal.world_template["topology"]["edge/corridor/2/1"]["target"] = json!({
        "kind": "position",
        "location": {
            "realm": "realm_0", "level": "corridor", "position": {"x": 3, "y": 1}
        }
    });
    let error = definition_error(&nonlocal);
    assert_has(&error, "local_door target must be a position exactly equal");

    let mut alias = local_door_parts(false, "closed");
    alias.world_template["arrivals"] = json!({
        "arrival/door": {
            "realm": "realm_0", "level": "corridor", "position": {"x": 2, "y": 1}
        }
    });
    alias.world_template["topology"]["edge/corridor/2/1"]["target"] =
        json!({"kind": "arrival", "arrival_id": "arrival/door"});
    let error = definition_error(&alias);
    assert_has(&error, "never an arrival alias");
}

#[test]
fn local_door_rejects_hidden_open() {
    let parts = local_door_parts(true, "open");
    let error = definition_error(&parts);
    assert_has(
        &error,
        "local_door cannot be hidden with initial_state open",
    );

    local_door_parts(true, "closed")
        .definition()
        .expect("a hidden closed local door is valid");
}

#[test]
fn paired_door_still_requires_two_distinct_reciprocal_endpoints() {
    let mut parts = local_door_parts(false, "closed");
    parts.world_template["topology"] = json!({
        "edge/corridor/2/1": {
            "at": {"realm": "realm_0", "level": "corridor", "position": {"x": 2, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "corridor", "position": {"x": 3, "y": 1}
            }},
            "kind": {
                "kind": "door",
                "binding_id": "binding/paired",
                "endpoint_id": "edge/corridor/2/1",
                "reciprocal_endpoint_id": "edge/corridor/3/1",
                "initial_state": "closed"
            },
            "hidden": false
        },
        "edge/corridor/3/1": {
            "at": {"realm": "realm_0", "level": "corridor", "position": {"x": 3, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "corridor", "position": {"x": 2, "y": 1}
            }},
            "kind": {
                "kind": "door",
                "binding_id": "binding/paired",
                "endpoint_id": "edge/corridor/3/1",
                "reciprocal_endpoint_id": "edge/corridor/2/1",
                "initial_state": "closed"
            },
            "hidden": false
        }
    });
    parts
        .definition()
        .expect("two distinct reciprocal endpoints are valid");

    let mut broken = local_door_parts(false, "closed");
    broken.world_template["topology"] = json!({
        "edge/corridor/2/1": {
            "at": {"realm": "realm_0", "level": "corridor", "position": {"x": 2, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "corridor", "position": {"x": 3, "y": 1}
            }},
            "kind": {
                "kind": "door",
                "binding_id": "binding/broken",
                "endpoint_id": "edge/corridor/2/1",
                "reciprocal_endpoint_id": "edge/corridor/2/1",
                "initial_state": "closed"
            },
            "hidden": false
        },
        "edge/corridor/3/1": {
            "at": {"realm": "realm_0", "level": "corridor", "position": {"x": 3, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "corridor", "position": {"x": 2, "y": 1}
            }},
            "kind": {
                "kind": "door",
                "binding_id": "binding/broken",
                "endpoint_id": "edge/corridor/3/1",
                "reciprocal_endpoint_id": "edge/corridor/3/1",
                "initial_state": "closed"
            },
            "hidden": false
        }
    });
    let error = definition_error(&broken);
    assert_has(&error, "are not exact reciprocals");

    let mut identical = local_door_parts(false, "closed");
    identical.world_template["topology"] = json!({
        "edge/corridor/2/1": {
            "at": {"realm": "realm_0", "level": "corridor", "position": {"x": 2, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "corridor", "position": {"x": 3, "y": 1}
            }},
            "kind": {
                "kind": "door",
                "binding_id": "binding/identical",
                "endpoint_id": "edge/corridor/same",
                "reciprocal_endpoint_id": "edge/corridor/same",
                "initial_state": "closed"
            },
            "hidden": false
        }
    });
    let error = definition_error(&identical);
    assert_has(&error, "distinct endpoint IDs must be non-empty");
}

#[test]
fn closed_local_door_opens_by_movement_and_stays_on_the_door_tile() {
    let mut engine = local_door_engine(false, "closed");
    let door_position = Coord { x: 2, y: 1 };
    assert!(!local_door_is_open(&engine, door_position));

    let events = engine
        .apply_actor_intent(
            &ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("movement onto a closed local door opens it");

    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::DoorOpened { location, .. } if location.position == door_position)),
        "movement should open the local door in place: {events:?}"
    );
    assert_eq!(
        player_position(&engine),
        tme_rules::WorldPosition::new("realm_0", "corridor", door_position),
        "the actor should end on the door tile, not a teleport target"
    );
    assert!(local_door_is_open(&engine, door_position));

    let checkpoint = engine.export_checkpoint().expect("export checkpoint");
    let hydrated = Engine::hydrate_checkpoint(engine.definition().clone(), &checkpoint)
        .expect("hydrate checkpoint");
    assert_eq!(
        checkpoint,
        hydrated.export_checkpoint().expect("re-export checkpoint")
    );
    assert_eq!(
        player_position(&hydrated),
        tme_rules::WorldPosition::new("realm_0", "corridor", door_position)
    );
    assert!(local_door_is_open(&hydrated, door_position));
}

#[test]
fn closed_local_door_first_in_three_step_path_stops_on_the_door_tile() {
    let mut engine = wide_local_door_engine(2, "closed");
    let path = [Direction::East, Direction::East, Direction::East];
    let preview = engine
        .preview_actor_path(&ActorId::from("player"), &path)
        .expect("preview");
    assert_eq!(preview.accepted_steps, 1);
    assert_eq!(preview.stop_reason, MovementStopReason::Transitioned);
    assert_eq!(preview.final_position.position, Coord { x: 2, y: 1 });
    assert_eq!(preview.remaining_path_points, 2);
    assert!(preview.steps[0].opens_door);
    assert!(matches!(
        preview.steps[0].outcome,
        PathPreviewStepOutcomeV1::Transitioned { .. }
    ));

    let events = engine
        .apply_actor_intent(
            &ActorId::from("player"),
            PlayerIntent::MovePath(path.to_vec()),
        )
        .expect("commit");
    assert_eq!(
        movement_started(&events.events),
        (1, MovementStopReason::Transitioned)
    );
    assert_eq!(player_position(&engine), preview.final_position);
    assert!(local_door_is_open(&engine, Coord { x: 2, y: 1 }));
}

#[test]
fn closed_local_door_second_in_three_step_path_stops_on_the_door_tile() {
    let mut engine = wide_local_door_engine(3, "closed");
    let path = [Direction::East, Direction::East, Direction::East];
    let preview = engine
        .preview_actor_path(&ActorId::from("player"), &path)
        .expect("preview");
    assert_eq!(preview.accepted_steps, 2);
    assert_eq!(preview.stop_reason, MovementStopReason::Transitioned);
    assert_eq!(preview.final_position.position, Coord { x: 3, y: 1 });
    assert_eq!(preview.remaining_path_points, 1);
    assert!(!preview.steps[0].opens_door);
    assert!(matches!(
        preview.steps[0].outcome,
        PathPreviewStepOutcomeV1::Moved { .. }
    ));
    assert!(preview.steps[1].opens_door);
    assert!(matches!(
        preview.steps[1].outcome,
        PathPreviewStepOutcomeV1::Transitioned { .. }
    ));

    let events = engine
        .apply_actor_intent(
            &ActorId::from("player"),
            PlayerIntent::MovePath(path.to_vec()),
        )
        .expect("commit");
    assert_eq!(
        movement_started(&events.events),
        (2, MovementStopReason::Transitioned)
    );
    assert_eq!(player_position(&engine), preview.final_position);
    assert!(local_door_is_open(&engine, Coord { x: 3, y: 1 }));
}

#[test]
fn open_local_door_permits_completing_three_step_sprint() {
    let mut engine = wide_local_door_engine(2, "open");
    let path = [Direction::East, Direction::East, Direction::East];
    let preview = engine
        .preview_actor_path(&ActorId::from("player"), &path)
        .expect("preview");
    assert_eq!(preview.accepted_steps, 3);
    assert_eq!(preview.stop_reason, MovementStopReason::FullPathAccepted);
    assert_eq!(preview.final_position.position, Coord { x: 4, y: 1 });
    assert_eq!(preview.remaining_path_points, 0);
    assert!(preview.steps.iter().all(|step| !step.opens_door));
    assert!(matches!(
        &preview.steps[0].outcome,
        PathPreviewStepOutcomeV1::Transitioned { to, .. }
            if to.position == Coord { x: 2, y: 1 }
    ));
    assert!(
        preview.steps[1..]
            .iter()
            .all(|step| matches!(step.outcome, PathPreviewStepOutcomeV1::Moved { .. }))
    );

    let events = engine
        .apply_actor_intent(
            &ActorId::from("player"),
            PlayerIntent::MovePath(path.to_vec()),
        )
        .expect("commit");
    assert_eq!(
        movement_started(&events.events),
        (3, MovementStopReason::FullPathAccepted)
    );
    assert_eq!(player_position(&engine), preview.final_position);
    assert!(local_door_is_open(&engine, Coord { x: 2, y: 1 }));
}

#[test]
fn concealed_local_door_blocks_movement_and_sight_until_revealed() {
    let mut engine = local_door_engine(true, "closed");
    let at = tme_rules::WorldPosition::new("realm_0", "corridor", Coord { x: 2, y: 1 });
    let tile = engine.snapshot().realms[0].levels[0]
        .tiles
        .iter()
        .find(|tile| tile.position == at.position)
        .unwrap()
        .clone();
    assert!(!tile.passable);
    assert!(tile.transition.is_none());
    let before = player_position(&engine);
    let beyond = tme_rules::WorldPosition::new("realm_0", "corridor", Coord { x: 3, y: 1 });
    assert!(!engine.has_line_of_sight(&before, &beyond));
    let _ = engine.apply_actor_intent(
        &ActorId::from("player"),
        PlayerIntent::MovePath(vec![Direction::East]),
    );
    assert_eq!(player_position(&engine), before);
    engine.set_navigation_revealed(&at, true).unwrap();
    // Revelation changes knowledge; opening still goes through normal movement.
    assert!(!local_door_is_open(&engine, at.position));
    engine
        .apply_actor_intent(
            &ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .unwrap();
    assert_eq!(player_position(&engine), at);
    assert!(local_door_is_open(&engine, at.position));
    assert!(engine.has_line_of_sight(&before, &beyond));
}

#[test]
fn scrolling_interiors_keep_single_cell_walls_without_forcing_shadow_geometry() {
    let mut parts = local_door_parts(false, "closed");
    let level = &mut parts.template_levels_source_mut()["corridor"];
    level["scene_role"] = json!("interior");
    level["presentation_mode"] = json!("overworld_town");
    level["world_zoom"] = json!({"screen_cell_pitch": [156,104]});
    level["maximum_clear_sightline"] = json!(3);
    level["wall_terrain_ids"] = json!(["stone_wall"]);
    level["staged_viewport"] = json!({"frame_size": [100,100], "fit_whole_level": false});
    parts.definition().unwrap();
    parts.template_levels_source_mut()["corridor"]["staged_viewport"]["fit_whole_level"] =
        json!(true);
    assert_has(&definition_error(&parts), "whole-level composition");
    parts.template_levels_source_mut()["corridor"]["staged_viewport"] =
        json!({"frame_size": [0,100], "fit_whole_level": false});
    assert_has(
        &definition_error(&parts),
        "frame_size values must be positive",
    );
}
