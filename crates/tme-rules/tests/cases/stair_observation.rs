use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};
use tme_rules::{
    ACTION_CONTEXT_CONTRACT_VERSION, ActionBlockedReasonV1, AutomaticActorDecisionV1,
    AutomaticMovementPurposeV1, Coord, Direction, Engine, Event, ExplicitTraversalKind,
    MovementStopReason, NavigationKind, OBSERVED_SNAPSHOT_CONTRACT_VERSION,
    PATH_PREVIEW_CONTRACT_VERSION, PLAYER_OBSERVATION_RADIUS, PathPreviewStepOutcomeV1,
    PlayerCommandV1, PlayerIntent, PlayerIntentPayloadV1, TileObservationV1, VerticalDirection,
    WorldPosition,
};

fn monster(source: &Value, id: &str, definition_id: &str, room: &str, position: Coord) -> Value {
    let mut actor = source.clone();
    actor["id"] = json!(id);
    actor["actor_definition_id"] = json!(definition_id);
    actor["location"] = json!({
        "realm": "realm_0", "level": room, "position": position
    });
    actor["carried"] = json!({"items": [], "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}});
    actor
}

fn base_parts() -> ContentParts {
    let mut parts = ContentParts::tracked("character_sheet", "profile/character_sheet");
    let mut source = ContentParts::tracked("first_room", "profile/first_room");
    let monster_source = source.actors_mut()[1].clone();
    let monster_definition_source = source.actor_definition_mut(1).clone();

    let cells = |rows: &[&str]| {
        rows.iter()
            .map(|row| {
                row.chars()
                    .map(|glyph| {
                        json!([match glyph {
                            '#' => "stone_wall",
                            _ => "flagstone",
                        }])
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    let visual_manifest_digest = parts.world_template["visual_manifest_digest"].clone();
    parts.world_template = json!({
        "schema_version": 3,
        "kind": "world_template",
        "id": "stair_observation",
        "visual_manifest_digest": visual_manifest_digest,
        "realms": {
            "realm_0": {"name": "Stair Observation", "levels": {
                "gallery": {
                    "law_zone": "none", "width": 11, "height": 9,
                    "cells": cells(&[
                    "...........", "...........", ".....#.....", "...........",
                    "...........", "...........", "...........", "...........", "..........."
                    ])
                },
                "lower_gallery": {
                    "law_zone": "none", "width": 5, "height": 5,
                    "cells": cells(&[".....", ".....", ".....", ".....", "....."])
                }
            }}
        },
        "arrivals": {},
        "topology": {
            "edge/gallery/6/4": {
                "at": {"realm": "realm_0", "level": "gallery", "position": {"x": 6, "y": 4}},
                "target": {"kind": "position", "location": {
                    "realm": "realm_0", "level": "lower_gallery", "position": {"x": 2, "y": 2}
                }},
                "kind": {"kind": "stairs", "direction": "down"},
                "hidden": false
            }
        }
    });
    parts.rules_source_mut()["movement"]["controlled_path_points"] = json!(4);
    parts.profile_value_mut()["items"] = json!([]);
    parts.push_selected(
        "items",
        "item/long_bow/stair_observation",
        json!({
            "id": "long_bow", "kind": "weapon", "name": "Long Bow",
            "weapon": {
                "skill_track_id": "bow", "default_attack_mode": "shoot",
                "attack_modes": [{"mode": "shoot", "maximum_range": 10, "damage_kind": "piercing"}],
                "cooldown_units": 1, "combat_add_rating": 0, "handedness": "bow",
                "block_value": 0, "nocking": {"unloads_on_movement": true}
            },
            "valid_placements": ["hand", "belt_side", "belt_back", "sack"],
            "economy": {"unit_burden": 1}
        }),
    );

    parts.actor_definition_mut(0)["name"] = json!("Observer");
    parts.actor_definition_mut(0)["stats"] = json!({"hp": 50, "attack": 5, "defense": 10});
    for (id, name, behavior) in [
        ("near", "Near Watcher", "hold_ground"),
        ("far", "Far Watcher", "simple_chase"),
        ("occluded", "Occluded Watcher", "hold_ground"),
        ("occupant", "Lower Occupant", "hold_ground"),
    ] {
        let definition_id = format!("actor/test/stair_observation/{id}");
        let mut definition = monster_definition_source.clone();
        definition["id"] = json!(definition_id);
        definition["name"] = json!(name);
        definition["stats"] = json!({"hp": 20, "attack": 0, "defense": 0});
        definition["ai"] = json!({
            "behavior": behavior,
            "cadence_units": 1,
            "aggro_radius": 7,
            "leash_range": 12,
            "awareness": {"mode": "line_of_sight_memory", "memory_opportunities": 2},
            "physical_attack_modes": ["fight"]
        });
        parts.push_selected(
            "actor_definitions",
            &format!("actor/test/stair_observation/{id}"),
            definition,
        );
    }

    let mut player = parts.actors_mut()[0].clone();
    player["id"] = json!("player");
    player["location"] = json!({
        "realm": "realm_0", "level": "gallery", "position": {"x": 5, "y": 4}
    });
    player["character"]["resources"]["hp"] = json!(50);
    player["character"]["resources"]["max_hp"] = json!(50);
    player["character"]["resources"]["peak_hp"] = json!(50);
    player["character"]["resources"]["stamina"] = json!(10);
    player["character"]["resources"]["max_stamina"] = json!(10);
    player["carried"] = json!({
        "items": [{"item_instance_id": "long_bow", "position": "right_hand"}],
        "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}
    });

    *parts.actors_mut() = Value::Array(vec![
        player,
        monster(
            &monster_source,
            "near",
            "actor/test/stair_observation/near",
            "gallery",
            Coord { x: 8, y: 4 },
        ),
        monster(
            &monster_source,
            "far",
            "actor/test/stair_observation/far",
            "gallery",
            Coord { x: 9, y: 4 },
        ),
        monster(
            &monster_source,
            "occluded",
            "actor/test/stair_observation/occluded",
            "gallery",
            Coord { x: 5, y: 1 },
        ),
        monster(
            &monster_source,
            "occupant",
            "actor/test/stair_observation/occupant",
            "lower_gallery",
            Coord { x: 2, y: 2 },
        ),
    ]);
    *parts.item_instances_mut() = json!({
        "long_bow": {"definition_id": "long_bow", "binding": {"state": "unrestricted"}}
    });
    *parts.ground_items_mut() = json!([]);
    *parts.service_instances_mut() = json!([]);
    *parts.merchant_inventories_mut() = json!([]);
    parts
}

fn engine_from(parts: ContentParts) -> Engine {
    parts.engine(7).expect("focused graph starts")
}

fn engine() -> Engine {
    engine_from(base_parts())
}

fn player_position(engine: &Engine) -> WorldPosition {
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player exists");
    player.location.clone()
}

#[test]
fn horizontal_path_moves_onto_and_across_stairs_without_transition() {
    let mut engine = engine();
    let path = [Direction::East, Direction::East];
    let preview = engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &path)
        .expect("preview");
    assert_eq!(preview.contract_version, PATH_PREVIEW_CONTRACT_VERSION);
    assert_eq!(preview.stop_reason, MovementStopReason::FullPathAccepted);
    assert_eq!(preview.steps.len(), 2);
    assert!(preview.steps.iter().all(|step| {
        !step.opens_door
            && step.outcome
                == PathPreviewStepOutcomeV1::Moved {
                    kind: NavigationKind::Walk.into(),
                }
    }));
    assert_eq!(
        preview.final_position,
        WorldPosition::new("realm_0", "gallery", Coord { x: 7, y: 4 })
    );
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(path.to_vec()),
        )
        .expect("commit");
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::Moved { actor_id, .. } if actor_id == "player"))
            .count(),
        2
    );
    assert!(!events.iter().any(
        |event| matches!(event, Event::WorldTransition { actor_id, .. } if actor_id == "player")
    ));
    assert_eq!(player_position(&engine), preview.final_position);
}

#[test]
fn no_stair_and_wrong_direction_are_transactional_typed_failures() {
    let mut engine = engine();
    let before = engine.snapshot();
    assert_eq!(
        engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::Traverse(ExplicitTraversalKind::StairsDown),
            )
            .expect_err("no stair")
            .message(),
        "no traversal here"
    );
    assert_eq!(engine.snapshot(), before);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .unwrap();
    let before_wrong = engine.snapshot();
    assert_eq!(
        engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::Traverse(ExplicitTraversalKind::StairsUp),
            )
            .expect_err("wrong direction")
            .message(),
        "wrong traversal kind"
    );
    assert_eq!(engine.snapshot(), before_wrong);
    let command = |kind| PlayerCommandV1 {
        contract_version: tme_rules::COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::Traverse { kind },
    };
    assert_eq!(
        engine
            .validate_actor_command(&command(ExplicitTraversalKind::StairsUp))
            .unwrap()
            .blocked_reason,
        Some(ActionBlockedReasonV1::WrongTraversalKind)
    );
    assert!(
        engine
            .validate_actor_command(&command(ExplicitTraversalKind::StairsDown))
            .unwrap()
            .accepted
    );
}

#[test]
fn correct_stair_use_is_one_standard_action_and_allows_occupied_target() {
    let mut engine = engine();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .unwrap();
    let stamina = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .unwrap()
        .stamina;
    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .unwrap();
    assert_eq!(context.contract_version, ACTION_CONTEXT_CONTRACT_VERSION);
    assert_eq!(context.traversal_actions.len(), 1);
    assert_eq!(
        context.traversal_actions[0].kind,
        ExplicitTraversalKind::StairsDown
    );
    assert_eq!(
        context.traversal_actions[0].target,
        WorldPosition::new("realm_0", "lower_gallery", Coord { x: 2, y: 2 })
    );
    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .unwrap();
    let up = options
        .iter()
        .find(|option| option.id == "stairs_up")
        .unwrap();
    let down = options
        .iter()
        .find(|option| option.id == "stairs_down")
        .unwrap();
    assert_eq!(
        up.blocked_reason,
        Some(ActionBlockedReasonV1::WrongTraversalKind)
    );
    assert!(down.enabled);
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Traverse(ExplicitTraversalKind::StairsDown),
        )
        .unwrap();
    assert!(matches!(
        events.events.get(2),
        Some(Event::WorldTransition {
            actor_id,
            from,
            to,
            navigation: NavigationKind::Stairs { direction: VerticalDirection::Down },
            ..
        }) if actor_id == "player"
            && from.level == "gallery"
            && to.level == "lower_gallery"
    ));
    assert_eq!(
        player_position(&engine),
        WorldPosition::new("realm_0", "lower_gallery", Coord { x: 2, y: 2 })
    );
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .stamina,
        stamina
    );
    assert_eq!(
        engine
            .world()
            .actors
            .iter()
            .filter(|actor| actor.is_alive()
                && actor.location
                    == WorldPosition::new("realm_0", "lower_gallery", Coord { x: 2, y: 2 }))
            .count(),
        2
    );
}

#[test]
fn observed_projection_is_centered_radius_seven_occluded_and_read_only() {
    let engine = engine();
    let before = engine.snapshot();
    let frame = engine
        .actor_observed_frame(&tme_rules::ActorId::from("player"))
        .unwrap();
    let observed = &frame.observed_snapshot;
    let tile_observation = |x, y| {
        observed
            .realms
            .iter()
            .find(|realm| realm.id == "realm_0")
            .and_then(|realm| realm.levels.iter().find(|level| level.id == "gallery"))
            .and_then(|level| {
                level
                    .tiles
                    .iter()
                    .find(|tile| tile.position == Coord { x, y })
            })
            .map(|tile| tile.observation)
            .expect("gallery tile")
    };
    assert_eq!(PLAYER_OBSERVATION_RADIUS, 7);
    assert_eq!(
        observed.contract_version,
        OBSERVED_SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        observed.observation_center,
        WorldPosition::new("realm_0", "gallery", Coord { x: 5, y: 4 })
    );
    assert_eq!(observed.observation_radius, 7);
    assert_eq!(tile_observation(8, 4), TileObservationV1::Visible);
    assert_eq!(tile_observation(9, 4), TileObservationV1::Visible);
    assert_eq!(tile_observation(5, 1), TileObservationV1::Unknown);
    let actor_ids = observed
        .actors
        .iter()
        .map(|actor| actor.id.as_str())
        .collect::<Vec<_>>();
    assert!(actor_ids.contains(&"near"));
    assert!(actor_ids.contains(&"far"));
    assert!(!actor_ids.contains(&"occluded"));
    let far_tile = observed
        .realms
        .iter()
        .find(|realm| realm.id == "realm_0")
        .and_then(|realm| realm.levels.iter().find(|level| level.id == "gallery"))
        .and_then(|level| {
            level
                .tiles
                .iter()
                .find(|tile| tile.position == Coord { x: 9, y: 4 })
        })
        .expect("shape-preserving tile");
    assert_eq!(far_tile.observation, TileObservationV1::Visible);
    assert!(far_tile.terrain_id.is_some());
    assert_eq!(frame.action_context.attack_targets.len(), 2);
    assert_eq!(
        frame
            .action_context
            .attack_targets
            .iter()
            .map(|target| target.actor_id.as_str())
            .collect::<Vec<_>>(),
        ["far", "near"]
    );
    assert_eq!(engine.snapshot(), before);
}

#[test]
fn player_targeting_uses_radius_seven_while_monster_acquisition_uses_aggro() {
    let mut engine = engine();
    engine
        .world_mut()
        .item_instances
        .get_mut("long_bow")
        .unwrap()
        .bow_readiness = Some(tme_rules::BowReadiness::Nocked);
    let command = |target_actor_id: &str| PlayerCommandV1 {
        contract_version: tme_rules::COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::PhysicalAttack {
            authorization: tme_rules::HostilityAuthorization::Safe,
            mode: tme_rules::PhysicalAttackMode::Shoot,
            target_actor_id: target_actor_id.into(),
        },
    };
    assert!(
        engine
            .validate_actor_command(&command("far"))
            .unwrap()
            .accepted
    );
    assert!(
        engine
            .validate_actor_command(&command("near"))
            .unwrap()
            .accepted
    );
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .unwrap();
    assert!(events.iter().any(|event| matches!(
        event,
        Event::AutomaticActorDecision {
            actor_id,
            decision: AutomaticActorDecisionV1::Move {
                direction: Direction::West,
                purpose: AutomaticMovementPurposeV1::Chase,
            },
            ..
        } if actor_id == "far"
    )));
}

#[test]
fn automatic_actor_walks_onto_stairs_but_stair_only_cross_room_route_is_absent() {
    let mut same_room = base_parts();
    let player = same_room.actors_mut()[0].clone();
    let mut far = same_room.actors_mut()[2].clone();
    far["location"]["position"] = json!({"x": 7, "y": 4});
    same_room.actor_definition_by_actor_id_mut("far")["ai"]["awareness"] =
        json!({"mode": "unrestricted"});
    *same_room.actors_mut() = Value::Array(vec![player, far]);
    let mut engine = engine_from(same_room);
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .unwrap();
    assert!(events.iter().any(|event| matches!(
        event,
        Event::Moved { actor_id, to, .. }
            if actor_id == "far"
                && *to == WorldPosition::new("realm_0", "gallery", Coord { x: 6, y: 4 })
    )));
    assert!(!events.iter().any(
        |event| matches!(event, Event::WorldTransition { actor_id, .. } if actor_id == "far")
    ));

    let mut cross_room = base_parts();
    let mut player = cross_room.actors_mut()[0].clone();
    player["location"] = json!({
        "realm": "realm_0", "level": "lower_gallery", "position": {"x": 2, "y": 2}
    });
    let mut far = cross_room.actors_mut()[2].clone();
    far["location"]["position"] = json!({"x": 6, "y": 4});
    cross_room.actor_definition_by_actor_id_mut("far")["ai"]["awareness"] =
        json!({"mode": "unrestricted"});
    *cross_room.actors_mut() = Value::Array(vec![player, far]);
    let mut engine = engine_from(cross_room);
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .unwrap();
    assert!(!events.iter().any(
        |event| matches!(event, Event::WorldTransition { actor_id, .. } if actor_id == "far")
    ));
    let monster = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "far")
        .unwrap();
    assert_eq!(
        monster.location,
        WorldPosition::new("realm_0", "gallery", Coord { x: 6, y: 4 })
    );
}
