#![allow(dead_code)]

use super::*;

pub(crate) fn bt_action_context_overlay_engine() -> Engine {
    let mut engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("tracked content should start");
    engine.world_mut().tile_effects.push(TileEffectState {
        source_actor_id: None,
        instance_id: "tile:ember:1".to_string(),
        effect_id: "ember_cloud".to_string(),
        source: ActiveEffectSource {
            kind: "spell".to_string(),
            id: "ember_cloud".to_string(),
        },
        location: WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 }),
        kind: "terrain_overlay".to_string(),
        tags: vec!["fire".to_string()],
        potency: 1,
        remaining_rounds: Some(2),
        passability: None,
        sight: None,
        hazard: Some("fire".to_string()),
        move_cost: None,
        tick_interval_rounds: 1,
        last_ticked_at: tme_rules::LogicalTime::new(0),
        hostile_authority: None,
    });
    engine.world_mut().tile_effects.push(TileEffectState {
        source_actor_id: None,
        instance_id: "tile:web:1".to_string(),
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
        passability: Some("hindered".to_string()),
        sight: None,
        hazard: None,
        move_cost: Some(2),
        tick_interval_rounds: 1,
        last_ticked_at: tme_rules::LogicalTime::new(0),
        hostile_authority: None,
    });
    engine
}

pub(crate) fn hidden_closed_door_engine() -> Engine {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    let visual_manifest_digest = parts.world_template["visual_manifest_digest"].clone();
    parts.world_template = serde_json::json!({
        "schema_version": 3,
        "kind": "world_template",
        "id": "hidden_closed_door",
        "visual_manifest_digest": visual_manifest_digest,
        "realms": {"realm_0": {"name": "Hidden Closed Door", "levels": {
            "start": {
                "law_zone": "none",
                "scene_role": "overworld",
                "presentation_mode": "overworld_town",
                "world_zoom": {"screen_cell_pitch": [156, 104]},
                "maximum_clear_sightline": 2,
                "staged_viewport": null,
                "wall_terrain_ids": [],
                "static_props": [],
                "width": 4, "height": 3,
                "cells": [
                    [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]],
                    [["stone_wall"], ["flagstone"], ["flagstone"], ["stone_wall"]],
                    [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]]
                ]
            },
            "vault": {
                "law_zone": "none",
                "scene_role": "overworld",
                "presentation_mode": "overworld_town",
                "world_zoom": {"screen_cell_pitch": [156, 104]},
                "maximum_clear_sightline": 2,
                "staged_viewport": null,
                "wall_terrain_ids": [],
                "static_props": [],
                "width": 4, "height": 3,
                "cells": [
                    [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]],
                    [["stone_wall"], ["flagstone"], ["flagstone"], ["stone_wall"]],
                    [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]]
                ]
            }
        }}},
        "arrivals": {},
        "topology": {
            "edge/start/2/1": {
                "at": {"realm": "realm_0", "level": "start", "position": {"x": 2, "y": 1}},
                "target": {"kind": "position", "location": {
                    "realm": "realm_0", "level": "vault", "position": {"x": 1, "y": 1}
                }},
                "kind": {
                    "kind": "door",
                    "initial_state": "closed",
                    "binding_id": "hidden_vault",
                    "endpoint_id": "hidden_vault/exterior",
                    "reciprocal_endpoint_id": "hidden_vault/interior"
                },
                "hidden": true
            },
            "edge/vault/1/1": {
                "at": {"realm": "realm_0", "level": "vault", "position": {"x": 1, "y": 1}},
                "target": {"kind": "position", "location": {
                    "realm": "realm_0", "level": "start", "position": {"x": 2, "y": 1}
                }},
                "kind": {
                    "kind": "door",
                    "initial_state": "closed",
                    "binding_id": "hidden_vault",
                    "endpoint_id": "hidden_vault/interior",
                    "reciprocal_endpoint_id": "hidden_vault/exterior"
                },
                "hidden": true
            }
        }
    });
    parts.actors_mut()[0]["location"]["level"] = serde_json::json!("start");
    parts.actors_mut()[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    parts.actors_mut()[1]["location"]["level"] = serde_json::json!("vault");
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 2, "y": 1});
    parts.engine(7).expect("hidden door content should start")
}

pub(crate) fn bw_summon_action_context_engine(alignment: &str) -> Engine {
    let mut parts = ContentParts::tracked(
        "summons_created_creature_lifecycle",
        "profile/summons_created_creature_lifecycle",
    );
    let definition_id = parts.selected_mut("summon_templates", 0)["actor_definition_id"]
        .as_str()
        .expect("summon actor definition id")
        .to_string();
    parts.selected_by_runtime_id_mut("actor_definitions", &definition_id)["social"]["alignment_source"]
        ["alignment"] = serde_json::json!(alignment);
    parts.engine(7).expect("summon content should start")
}
