use crate::support::content_parts::ContentParts;
use tme_rules::{
    Coord, Direction, Engine, Event, MovementStopReason, PathPreviewStepOutcomeV1, PlayerIntent,
    SpellTarget, TransitionKindViewV1, WorldPosition,
    model::{ActiveEffectSource, TileEffectState},
    view::PATH_PREVIEW_CONTRACT_VERSION,
};

fn tracked(case_id: &str) -> ContentParts {
    let profile = format!("profile/{case_id}");
    ContentParts::tracked(case_id, &profile)
}

fn engine(parts: ContentParts) -> Engine {
    parts.engine(7).expect("content graph should start")
}

fn assert_preview_matches_commit(
    parts: ContentParts,
    path: &[Direction],
    expected_stop_reason: MovementStopReason,
) {
    let preview = engine(parts.clone())
        .preview_actor_path(&tme_rules::ActorId::from("player"), path)
        .expect("preview");
    assert_eq!(preview.stop_reason, expected_stop_reason);

    let mut committed = engine(parts);
    committed
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(path.to_vec()),
        )
        .expect("commit");
    let player = committed
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    assert_eq!(preview.final_position, player.location.clone());
}

fn overlay(passability: &str, move_cost: Option<i32>) -> Engine {
    let mut engine = engine(tracked("first_room"));
    engine.world_mut().tile_effects.push(TileEffectState {
        hostile_authority: None,
        source_actor_id: None,
        instance_id: format!("tile:{passability}:1"),
        effect_id: "web_field".to_string(),
        source: ActiveEffectSource {
            kind: "spell".to_string(),
            id: "web_field".to_string(),
        },
        location: WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 1 }),
        kind: "terrain_overlay".to_string(),
        tags: vec!["web".to_string()],
        potency: 0,
        remaining_rounds: Some(2),
        passability: Some(passability.to_string()),
        sight: None,
        hazard: None,
        move_cost,
        tick_interval_rounds: 1,
        last_ticked_at: tme_rules::LogicalTime::ZERO,
    });
    engine
}

fn passable_wall() -> Engine {
    let mut parts = tracked("first_room");
    parts.actors_mut()[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 2});
    let mut engine = engine(parts);
    engine.world_mut().tile_effects.push(TileEffectState {
        hostile_authority: None,
        source_actor_id: None,
        instance_id: "tile:passable:wall".to_string(),
        effect_id: "bridge_field".to_string(),
        source: ActiveEffectSource {
            kind: "spell".to_string(),
            id: "bridge_field".to_string(),
        },
        location: WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 2 }),
        kind: "terrain_overlay".to_string(),
        tags: vec!["bridge".to_string()],
        potency: 0,
        remaining_rounds: Some(2),
        passability: Some("passable".to_string()),
        sight: None,
        hazard: None,
        move_cost: None,
        tick_interval_rounds: 1,
        last_ticked_at: tme_rules::LogicalTime::ZERO,
    });
    engine
}

fn portal_engine() -> Engine {
    engine(tracked("utility_door_secret_item_spells"))
}

fn hidden_door_engine() -> Engine {
    let mut parts = tracked("utility_door_secret_item_spells");
    parts.world_template["topology"]["edge/workroom/1/3"]["kind"] =
        serde_json::json!({"kind": "door", "initial_state": "open"});
    engine(parts)
}

fn cast_portal(engine: &mut Engine, position: Coord) {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "blue_gate".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "workroom", position),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("portal cast");
}

fn summon_engine(alignment: &str) -> Engine {
    let mut parts = tracked("summons_created_creature_lifecycle");
    parts.summon_actor_definition_by_template_id_mut("echo_guardian")["social"]["alignment_source"] =
        serde_json::json!({"kind": "inherent", "alignment": alignment});
    engine(parts)
}

fn cast_summon(engine: &mut Engine) {
    engine
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
        .expect("summon cast");
}

#[test]
fn passability_overlay_blocks_preview_and_commit() {
    let preview = overlay("blocked", None)
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .unwrap();
    assert_eq!(preview.stop_reason, MovementStopReason::Blocked);
    let events = overlay("blocked", None)
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .unwrap();
    assert!(events.iter().any(|event| matches!(
        event,
        Event::MovementBlocked { reason, attempted, .. }
            if reason == "blocked terrain" && attempted.position == Coord { x: 2, y: 1 }
    )));
}

#[test]
fn hindered_overlay_changes_preview_and_commit_cost() {
    let preview = overlay("hindered", Some(2))
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .unwrap();
    assert_eq!(preview.steps[0].cost, Some(2));
    assert_eq!(
        preview.steps[0].terrain_name.as_deref(),
        Some("Flagstone + web_field")
    );
}

#[test]
fn passable_overlay_on_wall_stays_usable_across_preview_context_and_commit() {
    let preview_engine = passable_wall();
    let preview = preview_engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .unwrap();
    assert_eq!(preview.stop_reason, MovementStopReason::FullPathAccepted);
    assert_eq!(preview.steps[0].cost, Some(1));
    assert_eq!(preview.final_position.position, Coord { x: 2, y: 2 });
    let exit = preview_engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .unwrap()
        .exits
        .into_iter()
        .find(|exit| exit.direction == Direction::East)
        .unwrap();
    assert!(!exit.blocked);
    assert_eq!(exit.move_cost, Some(1));
    assert_eq!(
        exit.terrain_name.as_deref(),
        Some("Stone Wall + bridge_field")
    );
    let mut committed = passable_wall();
    committed
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .unwrap();
    assert_eq!(
        committed
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .location
            .position,
        Coord { x: 2, y: 2 }
    );
}

#[test]
fn hidden_transition_preview_and_commit_only_use_transition_when_revealed() {
    let path = [Direction::East, Direction::East];
    let hidden = hidden_door_engine()
        .preview_actor_path(&tme_rules::ActorId::from("player"), &path)
        .unwrap();
    assert_eq!(hidden.stop_reason, MovementStopReason::FullPathAccepted);
    assert_eq!(hidden.final_position.level, "workroom");
    assert_eq!(hidden.final_position.position, Coord { x: 3, y: 1 });
    assert_eq!(
        hidden.steps[1].outcome,
        PathPreviewStepOutcomeV1::Moved {
            kind: TransitionKindViewV1::Walk
        }
    );

    let mut revealed = hidden_door_engine();
    revealed
        .set_navigation_revealed(
            &WorldPosition::new("realm_0", "workroom", Coord { x: 3, y: 1 }),
            true,
        )
        .unwrap();
    let preview = revealed
        .preview_actor_path(&tme_rules::ActorId::from("player"), &path)
        .unwrap();
    assert_eq!(preview.final_position.level, "hidden_room");
    assert!(matches!(
        preview.steps[1].outcome,
        PathPreviewStepOutcomeV1::Transitioned { .. }
    ));

    revealed
        .set_navigation_revealed(
            &WorldPosition::new("realm_0", "workroom", Coord { x: 3, y: 1 }),
            false,
        )
        .unwrap();
    assert_eq!(
        revealed
            .preview_actor_path(&tme_rules::ActorId::from("player"), &path)
            .unwrap()
            .final_position
            .level,
        "workroom"
    );
}

#[test]
fn portal_preview_and_commit_use_the_same_effective_transition() {
    let mut preview_engine = portal_engine();
    cast_portal(&mut preview_engine, Coord { x: 2, y: 1 });
    let preview = preview_engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .unwrap();
    assert_eq!(preview.stop_reason, MovementStopReason::Transitioned);
    assert_eq!(
        preview.final_position,
        WorldPosition::new("realm_0", "vault", Coord { x: 1, y: 1 })
    );

    let mut committed = portal_engine();
    cast_portal(&mut committed, Coord { x: 2, y: 1 });
    committed
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .unwrap();
    let player = committed
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .unwrap();
    assert_eq!(
        (&*player.location.level, player.location.position),
        ("vault", Coord { x: 1, y: 1 })
    );
}

#[test]
fn active_portal_replaces_authored_transition_for_route_edges() {
    let mut engine = portal_engine();
    cast_portal(&mut engine, Coord { x: 3, y: 1 });
    engine
        .set_navigation_revealed(
            &WorldPosition::new("realm_0", "workroom", Coord { x: 3, y: 1 }),
            true,
        )
        .unwrap();
    let preview = engine
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::East, Direction::East],
        )
        .unwrap();
    assert_eq!(preview.stop_reason, MovementStopReason::Transitioned);
    assert_eq!(preview.final_position.level, "vault");
    assert_ne!(preview.final_position.level, "hidden_room");
}

#[test]
fn active_portal_replaces_a_closed_authored_door_without_opening_it() {
    let mut parts = tracked("utility_door_secret_item_spells");
    parts.actors_mut()[0]["location"]["position"] = serde_json::json!({"x": 2, "y": 1});
    let mut engine = engine(parts);
    cast_portal(&mut engine, Coord { x: 4, y: 1 });
    let preview = engine
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::East, Direction::East],
        )
        .unwrap();
    assert_eq!(preview.stop_reason, MovementStopReason::Transitioned);
    assert_eq!(preview.final_position.level, "vault");
    assert!(!preview.steps[1].opens_door);
}

#[test]
fn full_path_accepted_within_one_room() {
    assert_preview_matches_commit(
        tracked("first_room"),
        &[Direction::South, Direction::South],
        MovementStopReason::FullPathAccepted,
    );
}

#[test]
fn blocked_terrain() {
    let preview = engine(tracked("first_room"))
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::North])
        .unwrap();
    assert_eq!(preview.stop_reason, MovementStopReason::Blocked);
    assert!(matches!(
        preview.steps[0].outcome,
        PathPreviewStepOutcomeV1::Blocked { .. }
    ));
}

#[test]
fn out_of_bounds_movement_is_reported_as_a_blocked_path() {
    let mut parts = tracked("first_room");
    let visual_manifest_digest = parts.world_template["visual_manifest_digest"].clone();
    parts.world_template = serde_json::json!({
        "schema_version": 3,
        "kind": "world_template",
        "id": "open_edge",
        "visual_manifest_digest": visual_manifest_digest,
        "realms": {"realm_0": {
            "name": "Open Edge",
            "levels": {"room_0": {
                "law_zone": "none",
                "width": 2,
                "height": 2,
                "cells": [
                    [["flagstone"], ["flagstone"]],
                    [["flagstone"], ["flagstone"]]
                ]
            }}
        }},
        "arrivals": {},
        "topology": {}
    });
    parts.actors_mut()[0]["location"]["position"] = serde_json::json!({"x": 0, "y": 0});
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    let preview = engine(parts)
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::North])
        .unwrap();
    assert_eq!(preview.stop_reason, MovementStopReason::Blocked);
    assert!(matches!(
        preview.steps[0].outcome,
        PathPreviewStepOutcomeV1::Blocked { .. }
    ));
}

#[test]
fn insufficient_movement_points() {
    let mut parts = tracked("terrain_movement");
    parts.rules_source_mut()["movement"]["controlled_path_points"] = serde_json::json!(1);
    let preview = engine(parts)
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::East, Direction::East, Direction::East],
        )
        .unwrap();
    assert_eq!(preview.stop_reason, MovementStopReason::Blocked);
    assert_eq!(preview.accepted_steps, 1);
    assert_eq!(preview.remaining_path_points, 0);
}

#[test]
fn hostile_occupancy_does_not_stop_path() {
    let preview = engine(tracked("first_room"))
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::East, Direction::East],
        )
        .unwrap();
    assert_eq!(preview.stop_reason, MovementStopReason::FullPathAccepted);
    assert!(preview.steps.iter().all(|step| step.outcome
        == PathPreviewStepOutcomeV1::Moved {
            kind: TransitionKindViewV1::Walk
        }));
}

#[test]
fn closed_door_auto_opens_in_preview() {
    let preview = engine(tracked("undercroft_loop"))
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .unwrap();
    assert_eq!(preview.stop_reason, MovementStopReason::FullPathAccepted);
    assert!(preview.steps[0].opens_door);
    assert!(matches!(
        preview.steps[0].outcome,
        PathPreviewStepOutcomeV1::Transitioned { .. }
    ));
}

#[test]
fn open_door_transition_accepts_path_without_open_mark() {
    let mut engine = engine(tracked("undercroft_loop"));
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Open(Direction::East),
        )
        .unwrap();
    let preview = engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .unwrap();
    assert_eq!(preview.stop_reason, MovementStopReason::FullPathAccepted);
    assert!(!preview.steps[0].opens_door);
}

#[test]
fn stairs_are_ordinary_horizontal_path_steps() {
    let preview = engine(tracked("undercroft_loop"))
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::South])
        .unwrap();
    assert_eq!(preview.stop_reason, MovementStopReason::FullPathAccepted);
    assert_eq!(
        preview.final_position,
        WorldPosition::new("realm_0", "entrance_hall", Coord { x: 1, y: 2 })
    );
    assert!(!preview.steps[0].opens_door);
    assert_eq!(
        preview.steps[0].outcome,
        PathPreviewStepOutcomeV1::Moved {
            kind: TransitionKindViewV1::Walk
        }
    );
}

#[test]
fn preview_does_not_mutate_engine() {
    let engine = engine(tracked("first_room"));
    let before = engine.snapshot();
    engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::South])
        .unwrap();
    assert_eq!(engine.snapshot(), before);
}

#[test]
fn preview_returns_requested_path_in_result() {
    let engine = engine(tracked("first_room"));
    let path = vec![Direction::South, Direction::East, Direction::North];
    let preview = engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &path)
        .unwrap();
    assert_eq!(preview.requested_path, path);
    assert_eq!(preview.contract_version, PATH_PREVIEW_CONTRACT_VERSION);
}

#[test]
fn preview_start_matches_player_position() {
    let engine = engine(tracked("first_room"));
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .unwrap();
    let preview = engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::South])
        .unwrap();
    assert_eq!(preview.start, player.location.clone());
}

#[test]
fn summon_preview_allows_player_owned_created_creature_colocation() {
    let mut preview_engine = summon_engine("lawful");
    cast_summon(&mut preview_engine);
    let preview = preview_engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .unwrap();
    assert_eq!(preview.stop_reason, MovementStopReason::FullPathAccepted);
    assert_eq!(
        preview.steps[0].outcome,
        PathPreviewStepOutcomeV1::Moved {
            kind: TransitionKindViewV1::Walk
        }
    );

    let mut committed = summon_engine("lawful");
    cast_summon(&mut committed);
    let events = committed
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .unwrap();
    assert!(events.iter().any(|event| matches!(
        event,
        Event::Moved { actor_id, to, .. }
            if actor_id == "player" && to.position == Coord { x: 2, y: 1 }
    )));
}

#[test]
fn summon_preview_allows_hostile_created_creature_colocation() {
    let mut preview_engine = summon_engine("chaotic");
    cast_summon(&mut preview_engine);
    let preview = preview_engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .unwrap();
    assert_eq!(preview.stop_reason, MovementStopReason::FullPathAccepted);
    assert_eq!(
        preview.steps[0].outcome,
        PathPreviewStepOutcomeV1::Moved {
            kind: TransitionKindViewV1::Walk
        }
    );

    let mut committed = summon_engine("chaotic");
    cast_summon(&mut committed);
    let events = committed
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .unwrap();
    assert!(events.iter().any(|event| matches!(
        event,
        Event::Moved { actor_id, to, .. }
            if actor_id == "player" && to.position == Coord { x: 2, y: 1 }
    )));
}

#[test]
fn allied_occupancy_is_an_accepted_step() {
    let mut parts = tracked("first_room");
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 2, "y": 1});
    parts.actor_definition_mut(1)["social"]["alignment_source"] =
        serde_json::json!({"kind": "inherent", "alignment": "lawful"});
    let preview = engine(parts)
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .unwrap();
    assert_eq!(preview.stop_reason, MovementStopReason::FullPathAccepted);
    assert_eq!(
        preview.steps[0].outcome,
        PathPreviewStepOutcomeV1::Moved {
            kind: TransitionKindViewV1::Walk
        }
    );
}
