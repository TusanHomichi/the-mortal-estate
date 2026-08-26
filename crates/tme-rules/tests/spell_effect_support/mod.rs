#![allow(dead_code)]

use crate::support::content_parts::ContentParts;
use tme_rules::*;

pub(super) fn br_effect_spell_engine(known_spell_ids: &[&str]) -> Engine {
    br_effect_spell_engine_with_player_hp(known_spell_ids, 10)
}

pub(super) fn br_effect_spell_engine_with_player_hp(
    known_spell_ids: &[&str],
    player_hp: i32,
) -> Engine {
    br_effect_spell_engine_with_player_hp_mutate(known_spell_ids, player_hp, |_| {})
}

pub(super) fn br_effect_spell_engine_with_player_hp_mutate(
    known_spell_ids: &[&str],
    player_hp: i32,
    mutate: impl FnOnce(&mut ContentParts),
) -> Engine {
    let mut parts = spell_parts(
        "spell_effects",
        "profile/spell_effects",
        known_spell_ids,
        vec!["#####", "#...#", "#####"],
        Coord { x: 2, y: 1 },
        br_spells(),
    );
    parts.actor_definition_mut(0)["stats"]["hp"] = serde_json::json!(player_hp);
    parts.actors_mut()[0]["character"]["resources"]["hp"] = serde_json::json!(player_hp);
    mutate(&mut parts);
    parts.engine(7).expect("BR effect engine should start")
}

pub(super) fn br_effect_spell_engine_with_effect_mutate(
    known_spell_ids: &[&str],
    spell_id: &str,
    mutate_effect: impl FnOnce(&mut serde_json::Value),
) -> Engine {
    br_effect_spell_engine_with_player_hp_mutate(known_spell_ids, 10, |parts| {
        let spell = parts.selected_by_runtime_id_mut("spells", spell_id);
        mutate_effect(&mut spell["effect"]);
    })
}

pub(super) fn add_test_tile_passability_effect(
    engine: &mut Engine,
    room: &str,
    position: Coord,
    passability: &str,
) {
    let round = engine.world().timing.now;
    engine
        .world_mut()
        .tile_effects
        .push(tme_rules::model::TileEffectState {
            source_actor_id: None,
            instance_id: format!(
                "test_tile_effect:{room}:{}:{}:{passability}",
                position.x, position.y
            ),
            effect_id: "test_tile_effect".to_string(),
            source: tme_rules::model::ActiveEffectSource {
                kind: "test".to_string(),
                id: "spell_casting_regression".to_string(),
            },
            location: WorldPosition::new("realm_0", room, position),
            kind: "terrain_overlay".to_string(),
            tags: Vec::new(),
            potency: 0,
            remaining_rounds: Some(2),
            passability: Some(passability.to_string()),
            sight: None,
            hazard: None,
            move_cost: None,
            tick_interval_rounds: 1,
            last_ticked_at: round,
            hostile_authority: None,
        });
}

pub(super) fn bs_runtime_spell_engine(
    known_spell_ids: &[&str],
    tiles: Vec<&str>,
    target_position: Coord,
) -> Engine {
    bs_runtime_spell_engine_mutate(known_spell_ids, tiles, target_position, |_| {})
}

pub(super) fn bs_runtime_spell_engine_mutate(
    known_spell_ids: &[&str],
    tiles: Vec<&str>,
    target_position: Coord,
    mutate: impl FnOnce(&mut ContentParts),
) -> Engine {
    let mut parts = spell_parts(
        "control_poison_protection",
        "profile/control_poison_protection",
        known_spell_ids,
        tiles,
        target_position,
        bs_spells(),
    );
    parts.profile_value_mut()["items"] = serde_json::json!([]);
    *parts.item_instances_mut() = serde_json::json!({});
    parts.actors_mut()[0]["carried"]["items"] = serde_json::json!([]);
    mutate(&mut parts);
    parts.engine(7).expect("BS runtime engine should start")
}

fn spell_parts(
    case_id: &str,
    profile: &str,
    known_spell_ids: &[&str],
    tiles: Vec<&str>,
    target_position: Coord,
    spells: Vec<serde_json::Value>,
) -> ContentParts {
    let mut parts = ContentParts::tracked(case_id, profile);
    parts.profile_value_mut()["rules_profile"] = serde_json::json!("rules/first_room");
    parts.profile_value_mut()["spells"] = serde_json::json!([]);
    for spell in spells {
        let id = spell["id"].as_str().expect("spell id").to_string();
        let existing_key = parts.catalog["spells"]
            .as_object()
            .expect("spell registry")
            .iter()
            .find_map(|(key, value)| (value == &spell).then(|| key.clone()));
        if let Some(key) = existing_key {
            parts.profile_value_mut()["spells"]
                .as_array_mut()
                .expect("spell selection")
                .push(serde_json::Value::String(key));
        } else {
            parts.push_selected("spells", &format!("spell/{id}/runtime_test"), spell);
        }
    }
    let width = tiles.first().expect("room tiles").chars().count();
    let height = tiles.len();
    let cells = tiles
        .iter()
        .map(|row| {
            row.chars()
                .map(|glyph| match glyph {
                    '#' => vec!["stone_wall"],
                    '.' => vec!["flagstone"],
                    _ => panic!("unmapped spell fixture glyph {glyph:?}"),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    *parts.template_levels_source_mut() = serde_json::json!({
        "room_0": {
            "law_zone": "none",
            "width": width,
            "height": height,
            "cells": cells
        }
    });
    *parts.ground_items_mut() = serde_json::json!([]);
    let known_spells = known_spell_ids
        .iter()
        .map(|spell_id| {
            serde_json::json!({
                "spell_id": spell_id,
                "lane": "wizard_magic",
                "learned_at_level": 1
            })
        })
        .collect::<Vec<_>>();
    let actors = parts.actors_mut().as_array_mut().expect("spell actors");
    actors[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    actors[0]["character"]["resources"]["hp"] = serde_json::json!(10);
    actors[0]["character"]["resources"]["max_hp"] = serde_json::json!(10);
    actors[0]["character"]["resources"]["peak_hp"] = serde_json::json!(10);
    actors[0]["character"]["resources"]["mp"] = serde_json::json!(30);
    actors[0]["character"]["resources"]["max_mp"] = serde_json::json!(30);
    actors[0]["character"]["known_spells"] = serde_json::Value::Array(known_spells);
    actors[1]["id"] = serde_json::json!("target");
    actors[1]["location"]["position"] =
        serde_json::json!({"x": target_position.x, "y": target_position.y});
    let player_definition = parts.actor_definition_mut(0);
    player_definition["name"] = serde_json::json!("Wiz");
    player_definition["stats"] = serde_json::json!({"hp": 10, "attack": 1, "defense": 0});
    let target_definition = parts.actor_definition_mut(1);
    target_definition["name"] = serde_json::json!("Target");
    target_definition["stats"] = serde_json::json!({"hp": 8, "attack": 0, "defense": 0});
    parts
}

fn br_spells() -> Vec<serde_json::Value> {
    vec![
        spell(
            "spark",
            "Spark",
            true,
            serde_json::json!({
                "family": "direct_damage", "potency": 3, "damage_kind": "arcane",
                "resistance": {"role": "incoming", "tag": "arcane", "mitigation": {
                    "mode": "half_damage", "rounding": "down", "minimum_damage": 1
                }}
            }),
            serde_json::json!({"kind": "actor", "range": 3, "requires_visible": true}),
            "direct",
            "character",
        ),
        spell(
            "mend",
            "Mend",
            false,
            serde_json::json!({"family": "healing", "potency": 4}),
            serde_json::json!({"kind": "self"}),
            "direct",
            "self",
        ),
        spell(
            "strength",
            "Strength",
            false,
            serde_json::json!({
                "family": "attribute_buff", "status_kind": "strength", "potency": 2,
                "stacking": "replace_same_kind", "duration": {"policy": "rounds", "rounds": 2}
            }),
            serde_json::json!({"kind": "self"}),
            "direct",
            "self",
        ),
        spell(
            "hex",
            "Hex",
            true,
            serde_json::json!({
                "family": "curse", "status_kind": "cursed", "potency": 2,
                "stacking": "replace_same_kind", "duration": {"policy": "rounds", "rounds": 2}
            }),
            serde_json::json!({"kind": "actor", "range": 3, "requires_visible": true}),
            "direct",
            "character",
        ),
        spell(
            "charged_spark",
            "Charged Spark",
            true,
            serde_json::json!({
                "family": "direct_damage", "potency": 3, "damage_kind": "arcane",
                "resistance": {"role": "incoming", "tag": "arcane", "mitigation": {
                    "mode": "half_damage", "rounding": "down", "minimum_damage": 1
                }}
            }),
            serde_json::json!({"kind": "actor", "range": 3, "requires_visible": true}),
            "warm_then_cast",
            "character",
        ),
    ]
}

fn bs_spells() -> Vec<serde_json::Value> {
    vec![
        spell(
            "spark",
            "Spark",
            true,
            serde_json::json!({
                "family": "direct_damage", "potency": 3, "damage_kind": "arcane",
                "resistance": {"role": "incoming", "tag": "arcane", "mitigation": {
                    "mode": "half_damage", "rounding": "down", "minimum_damage": 1
                }}
            }),
            serde_json::json!({"kind": "actor", "range": 3, "requires_visible": true}),
            "direct",
            "character",
        ),
        spell(
            "terror",
            "Terror",
            false,
            serde_json::json!({
                "family": "control_status", "status_kind": "fear", "potency": 1,
                "resistance": {"role": "incoming", "tag": "fear", "mitigation": {"mode": "negate"}},
                "duration": {"policy": "rounds", "rounds": 2}
            }),
            serde_json::json!({"kind": "self"}),
            "direct",
            "self",
        ),
        spell(
            "blind_self",
            "Blind Self",
            false,
            serde_json::json!({
                "family": "control_status", "status_kind": "blind",
                "resistance": {"role": "incoming", "tag": "blind", "mitigation": {"mode": "negate"}},
                "duration": {"policy": "rounds", "rounds": 2}
            }),
            serde_json::json!({"kind": "self"}),
            "direct",
            "self",
        ),
        spell(
            "ward_target",
            "Ward Target",
            false,
            serde_json::json!({
                "family": "protection", "status_kind": "ward",
                "duration": {"policy": "rounds", "rounds": 3},
                "resistance": {"role": "boost", "boosts": [{"tag": "mind", "bonus_twentieths": 15}]}
            }),
            serde_json::json!({"kind": "actor", "range": 3, "requires_visible": true}),
            "direct",
            "character",
        ),
        spell(
            "arcane_guard",
            "Arcane Guard",
            false,
            serde_json::json!({
                "family": "protection", "status_kind": "ward",
                "duration": {"policy": "rounds", "rounds": 3},
                "resistance": {"role": "boost", "boosts": [{"tag": "arcane", "bonus_twentieths": 15}]}
            }),
            serde_json::json!({"kind": "actor", "range": 3, "requires_visible": true}),
            "direct",
            "character",
        ),
        spell(
            "hold",
            "Hold",
            true,
            serde_json::json!({
                "family": "control_status", "status_kind": "stun", "potency": 1,
                "duration": {"policy": "rounds", "rounds": 2},
                "resistance": {"role": "incoming", "tag": "mind", "mitigation": {"mode": "negate"}}
            }),
            serde_json::json!({"kind": "actor", "range": 3, "requires_visible": true}),
            "direct",
            "character",
        ),
        spell(
            "poison",
            "Poison",
            true,
            poison_effect(),
            serde_json::json!({"kind": "actor", "range": 3, "requires_visible": true}),
            "direct",
            "character",
        ),
        spell(
            "self_poison",
            "Self Poison",
            true,
            poison_effect(),
            serde_json::json!({"kind": "self"}),
            "direct",
            "self",
        ),
        spell(
            "poison_cure",
            "Poison Cure",
            false,
            serde_json::json!({"family": "poison_cure"}),
            serde_json::json!({"kind": "actor", "range": 3, "requires_visible": true}),
            "direct",
            "character",
        ),
    ]
}

fn poison_effect() -> serde_json::Value {
    serde_json::json!({
        "family": "poison", "status_kind": "poison", "potency": 2,
        "start_delay_rounds": 1,
        "resistance": {"role": "incoming", "tag": "poison", "mitigation": {"mode": "negate"}},
        "duration": {"policy": "rounds", "rounds": 3}
    })
}

fn spell(
    id: &str,
    name: &str,
    hostile: bool,
    effect: serde_json::Value,
    target: serde_json::Value,
    method: &str,
    cast_class: &str,
) -> serde_json::Value {
    serde_json::json!({
        "social": {"hostile_act": hostile, "town_law": "permitted"},
        "id": id,
        "name": name,
        "status": "draft",
        "lane": "wizard_magic",
        "skill_requirement": 1,
        "mp_cost": 2,
        "stamina_cost": 1,
        "effect": effect,
        "target": target,
        "casting": {"method": method, "cast_class": cast_class}
    })
}
