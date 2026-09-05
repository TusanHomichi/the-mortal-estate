use crate::support::content_parts::ContentParts;
use tme_rules::{
    ActionBlockedReasonV1, AutomaticActorDecisionV1, COMMAND_CONTRACT_VERSION, Coord, Direction,
    Engine, Event, MovementStopReason, PathPreviewBlockedReasonV1, PathPreviewStepOutcomeV1,
    PlayerCommandV1, PlayerIntent, PlayerIntentPayloadV1, SpellTarget, WorldPosition,
    model::{ActiveEffectSource, ActiveEffectStackingPolicy, ActiveEffectState, TileEffectState},
};

fn equip_one_handed_ranged_weapon(
    parts: &mut ContentParts,
    actor_id: &str,
    weapon_instance_id: &str,
    maximum_range: u32,
) {
    parts.push_selected(
        "items",
        &format!("item/{weapon_instance_id}/active_effects"),
        serde_json::json!({
            "id": weapon_instance_id,
            "kind": "weapon",
            "name": "Test Javelin",
            "weapon": {
                "skill_track_id": "staff",
                "default_attack_mode": "throw",
                "attack_modes": [{
                    "mode": "throw",
                    "maximum_range": maximum_range,
                    "damage_kind": "piercing"
                }],
                "cooldown_units": 1,
                "combat_add_rating": 0,
                "handedness": "one_handed",
                "block_value": 0
            },
            "valid_placements": ["hand", "belt_side", "belt_back", "sack"]
        , "economy": {"unit_burden": 1}}),
    );
    parts.item_instances_mut()[weapon_instance_id] = serde_json::json!({
        "definition_id": weapon_instance_id,
        "binding": {"state": "unrestricted"}
    });
    let actor = parts
        .actors_mut()
        .as_array_mut()
        .expect("actors should be an array")
        .iter_mut()
        .find(|actor| actor["id"] == actor_id)
        .expect("weapon owner should exist");
    actor["carried"]["items"]
        .as_array_mut()
        .expect("carried items should be an array")
        .push(serde_json::json!({
            "item_instance_id": weapon_instance_id,
            "position": "right_hand"
        }));
}

fn status_engine_with_seed(mutate: impl FnOnce(&mut ContentParts), seed: u64) -> Engine {
    let mut parts = ContentParts::tracked("status_effects", "profile/status_effects");
    mutate(&mut parts);
    parts.engine(seed).expect("engine should start")
}

fn status_engine_with(mutate: impl FnOnce(&mut ContentParts)) -> Engine {
    status_engine_with_seed(mutate, 7)
}

fn status_engine() -> Engine {
    status_engine_with(|_| {})
}

fn thief_hide_engine_on_dark_tile_with(mutate: impl FnOnce(&mut ContentParts)) -> Engine {
    let mut parts = ContentParts::tracked(
        "profession_specific_actions",
        "profile/profession_specific_actions",
    );
    parts.template_levels_source_mut()["room_0"] = serde_json::json!({
        "law_zone": "none",
        "scene_role": "combat_space",
        "presentation_mode": "combat_space",
        "world_zoom": {"screen_cell_pitch": [156, 104]},
        "maximum_clear_sightline": 5,
        "staged_viewport": null,
        "wall_terrain_ids": [],
        "static_props": [],
        "width": 7,
        "height": 5,
        "cells": [
            [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]],
            [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
            [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
            [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
            [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]]
        ]
    });
    parts.actors_mut()[0]["location"]["position"] = serde_json::json!({"x": 2, "y": 2});
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 5, "y": 2});
    parts.actor_definition_mut(1)["ai"]["behavior"] = serde_json::json!("hold_ground");
    mutate(&mut parts);
    let mut engine = parts.engine(7).expect("hide fixture should start");
    engine.world_mut().tile_effects.push(TileEffectState {
        source_actor_id: None,
        instance_id: "tile:shadow:1".to_string(),
        effect_id: "shadow_veil".to_string(),
        source: ActiveEffectSource {
            kind: "spell".to_string(),
            id: "shadow_veil".to_string(),
        },
        location: WorldPosition::new("realm_0", "room_0", tme_rules::Coord { x: 2, y: 2 }),
        kind: "terrain_overlay".to_string(),
        tags: vec!["shadow".to_string()],
        potency: 0,
        remaining_rounds: Some(3),
        passability: None,
        sight: Some("obscured".to_string()),
        hazard: None,
        move_cost: None,
        tick_interval_rounds: 1,
        last_ticked_at: tme_rules::LogicalTime::new(0),
        hostile_authority: None,
    });
    engine
}
fn thief_hide_engine_on_dark_tile() -> Engine {
    thief_hide_engine_on_dark_tile_with(|_| {})
}

fn suppressed_monster_ability_engine() -> Engine {
    let mut parts = ContentParts::tracked(
        "monster_spellcasting_special_attacks",
        "profile/monster_spellcasting_special_attacks",
    );
    let actors = parts.actors_mut().as_array_mut().expect("seed actors");
    actors.retain(|actor| matches!(actor["id"].as_str(), Some("player" | "ember_imp")));
    actors[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    actors[1]["location"]["position"] = serde_json::json!({"x": 4, "y": 1});
    actors[1]["active_effects"] = serde_json::json!([
        {
            "instance_id": "stun_1",
            "effect_id": "stunning_gaze",
            "source": {"kind": "fixture", "id": "suppressed_monster_ability"},
            "kind": "control_status",
            "tags": ["stun"],
            "potency": 0,
            "remaining_rounds": 2,
            "stacking": "refresh_duration",
            "start_delay_rounds": 0,
            "tick_interval_rounds": 1,
            "suppresses_action": true,
            "resistance_boosts": []
        }
    ]);
    parts.actor_definition_mut(1)["ai"]["awareness"] = serde_json::json!({"mode": "unrestricted"});
    parts.engine(7).expect("suppression fixture should start")
}
fn summon_item_engine(mutate: impl FnOnce(&mut ContentParts)) -> Engine {
    let mut parts = ContentParts::tracked(
        "summons_created_creature_lifecycle",
        "profile/summons_created_creature_lifecycle",
    );
    parts.template_levels_source_mut()["start"]["width"] = serde_json::json!(6);
    parts.template_levels_source_mut()["start"]["cells"] = serde_json::json!([
        [
            ["stone_wall"],
            ["stone_wall"],
            ["stone_wall"],
            ["stone_wall"],
            ["stone_wall"],
            ["stone_wall"]
        ],
        [
            ["stone_wall"],
            ["flagstone"],
            ["flagstone"],
            ["flagstone"],
            ["flagstone"],
            ["stone_wall"]
        ],
        [
            ["stone_wall"],
            ["stone_wall"],
            ["stone_wall"],
            ["stone_wall"],
            ["stone_wall"],
            ["stone_wall"]
        ]
    ]);
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 4, "y": 1});
    parts.push_selected(
        "items",
        "item/echo_focus/active_effects",
        serde_json::json!({
            "id": "echo_focus",
            "kind": "weapon",
            "name": "Echo Focus",
            "category": "weapon",
            "weapon": {
                "skill_track_id": "staff",
                "default_attack_mode": "fight",
                "attack_modes": [{"mode": "fight", "maximum_range": 0, "damage_kind": "cutting"}],
                "cooldown_units": 1,
                "combat_add_rating": 0,
                "handedness": "one_handed",
                "block_value": 0
            },
            "capability": {"taxonomy_id": "echo_focus"},
            "valid_placements": ["hand", "belt_side", "belt_back", "sack"],
            "economy": {"unit_burden": 1}
        }),
    );
    {
        let summon = parts.selected_mut("summon_templates", 0);
        summon["id"] = serde_json::json!("echo_guard");
        summon["item_instances"] = serde_json::json!({
            "focus": {"definition_id": "echo_focus", "binding": {"state": "unrestricted"}}
        });
        summon["carried"] = serde_json::json!({
            "items": [{"item_instance_id": "focus", "position": "right_hand"}],
            "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}
        });
    }
    let summon_definition = parts.summon_actor_definition_mut(0);
    summon_definition["name"] = serde_json::json!("Echo Guard");
    summon_definition["ai"]["behavior"] = serde_json::json!("hold_ground");
    summon_definition["stats"]["attack"] = serde_json::json!(0);
    parts.selected_mut("spells", 0)["effect"]["summon_actor_id"] = serde_json::json!("echo_guard");
    mutate(&mut parts);
    parts
        .engine(1_010_580_540)
        .expect("summon item lifecycle engine should start")
}

#[path = "active_effects/summon_item_instances_use_deterministic_expanded_ids.rs"]
mod summon_item_instances_use_deterministic_expanded_ids;

#[path = "active_effects/defeat_removes_active_effects.rs"]
mod defeat_removes_active_effects;
