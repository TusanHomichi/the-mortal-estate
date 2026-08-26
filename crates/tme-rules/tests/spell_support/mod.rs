#![allow(dead_code)]

use crate::support::content_parts::ContentParts;
use tme_rules::*;

pub(super) fn equip_one_handed_test_weapon(
    parts: &mut ContentParts,
    weapon_instance_id: &str,
    mode: &str,
) {
    let handedness = if mode == "shoot" { "bow" } else { "one_handed" };
    let nocking = (mode == "shoot").then(|| serde_json::json!({"unloads_on_movement": true}));
    parts.push_selected(
        "items",
        &format!("item/{weapon_instance_id}/spell_test"),
        serde_json::json!({
            "id": weapon_instance_id,
            "kind": "weapon",
            "name": "Sight Test Weapon",
            "weapon": {
                "skill_track_id": "staff",
                "default_attack_mode": mode,
                "attack_modes": [{
                    "mode": mode,
                    "maximum_range": 3,
                    "damage_kind": "piercing"
                }],
                "cooldown_units": 1,
                "combat_add_rating": 0,
                "handedness": handedness,
                "block_value": 0,
                "nocking": nocking
            },
            "valid_placements": ["hand", "belt_side", "belt_back", "sack"],
            "economy": {"unit_burden": 1}
        }),
    );
    parts.item_instances_mut()[weapon_instance_id] = serde_json::json!({
        "definition_id": weapon_instance_id,
        "binding": {"state": "unrestricted"}
    });
    parts.actors_mut()[0]["carried"] = serde_json::json!({
        "items": [{
            "item_instance_id": weapon_instance_id,
            "position": "right_hand"
        }],
        "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}
    });
}

pub(super) fn count_skill_practice(events: &[Event], track_id: &str) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                event,
                Event::SkillPracticeAwarded {
                    track_id: event_track_id,
                    ..
                } if event_track_id == track_id
            )
        })
        .count()
}

pub(super) fn count_combat_xp(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::ExperienceAwarded { .. }))
        .count()
}

pub(super) fn count_stubbed_casts(events: &[Event], spell_id: &str) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                event,
                Event::SpellCastStubbed {
                    spell_id: event_spell_id,
                    ..
                } if event_spell_id == spell_id
            )
        })
        .count()
}

pub(super) fn spell_lane_engine(class_id: &str, lane: &str, spell_id: &str) -> Engine {
    let mut spell = serde_json::json!({
        "id": spell_id,
        "name": "Lane Spell",
        "status": "stub",
        "lane": lane,
        "mp_cost": 3,
        "stamina_cost": 1,
        "target": {"kind": "actor", "range": 3, "requires_visible": true},
        "casting": {"method": "direct", "cast_class": "character"}
    });
    if class_id != "knight" {
        spell["skill_requirement"] = serde_json::json!(1);
    }
    spell_lane_engine_with_spell(class_id, lane, spell)
}

pub(super) fn spell_lane_engine_with_spell(
    class_id: &str,
    lane: &str,
    spell: serde_json::Value,
) -> Engine {
    spell_lane_engine_with_spell_and_seed(class_id, lane, spell, 7)
}

pub(super) fn spell_lane_engine_with_spell_and_seed(
    class_id: &str,
    lane: &str,
    spell: serde_json::Value,
    seed: u64,
) -> Engine {
    spell_lane_engine_with_spell_and_seed_mutate(class_id, lane, spell, seed, |_| {})
}

pub(super) fn spell_lane_engine_with_spell_and_seed_mutate(
    class_id: &str,
    lane: &str,
    mut spell: serde_json::Value,
    seed: u64,
    mutate: impl FnOnce(&mut ContentParts),
) -> Engine {
    if spell.get("social").is_none() {
        spell["social"] = serde_json::json!({"hostile_act": false, "town_law": "permitted"});
    }
    let spell_id = spell["id"].as_str().expect("spell id").to_string();
    let mut parts = ContentParts::tracked("spell_readiness", "profile/spell_readiness");
    parts.profile_value_mut()["rules_profile"] = serde_json::json!("rules/first_room");
    parts.profile_value_mut()["spells"] = serde_json::json!([]);
    parts.profile_value_mut()["items"] = serde_json::json!([]);
    parts.push_selected("spells", "spell/lane_test", spell);
    *parts.template_levels_source_mut() = basic_rooms(vec!["####", "#..#", "####"]);
    *parts.item_instances_mut() = serde_json::json!({});
    *parts.ground_items_mut() = serde_json::json!([]);
    parts.actor_definition_mut(0)["name"] = serde_json::json!("Caster");
    parts.actor_definition_mut(1)["name"] = serde_json::json!("Target");
    parts.actor_definition_mut(1)["stats"] =
        serde_json::json!({"hp": 5, "attack": 0, "defense": 0});

    let actors = parts.actors_mut().as_array_mut().expect("spell actors");
    let player = &mut actors[0];
    player["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    player["character"]["identity"]["base_class_id"] = serde_json::json!(class_id);
    player["character"]["identity"]["current_class_id"] = serde_json::json!(class_id);
    player["character"]["identity"]["display_class"] = serde_json::json!(class_id);
    player["character"]["skill_ledger"] = if class_id == "knight" {
        serde_json::json!([])
    } else {
        serde_json::json!([{
            "track_id": lane,
            "level": 1,
            "critique_rank": 0,
            "practice_points": 0,
            "learning_rate": 1
        }])
    };
    player["character"]["known_spells"] = serde_json::json!([{
        "spell_id": spell_id,
        "lane": lane,
        "learned_at_level": 1
    }]);
    player["carried"] = serde_json::json!({
        "items": [],
        "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}
    });
    actors[1]["id"] = serde_json::json!("target");
    actors[1]["location"]["position"] = serde_json::json!({"x": 2, "y": 1});

    if class_id == "knight" {
        parts.profile_value_mut()["items"] = serde_json::json!(["item/oath_ring/knight_promotion"]);
        parts.item_instances_mut()["oath_ring"] = serde_json::json!({
            "definition_id": "oath_ring",
            "binding": {"state": "unrestricted"}
        });
        parts.actors_mut()[0]["carried"]["items"] = serde_json::json!([{
            "item_instance_id": "oath_ring",
            "position": "left_finger_1"
        }]);
    }
    mutate(&mut parts);
    parts.engine(seed).expect("spell lane engine should start")
}

pub(super) fn wizard_spell_engine(known_spell_id: Option<&str>, skill_rank: i32) -> Engine {
    wizard_spell_engine_with_content_mutate(known_spell_id, skill_rank, |_| {})
}

pub(super) fn wizard_spell_engine_with_content_mutate(
    known_spell_id: Option<&str>,
    skill_rank: i32,
    mutate: impl FnOnce(&mut ContentParts),
) -> Engine {
    let known_spells: Vec<&str> = known_spell_id.into_iter().collect();
    wizard_multi_spell_engine_with_content_mutate(
        &known_spells,
        skill_rank,
        vec!["####", "#..#", "####"],
        Coord { x: 2, y: 1 },
        mutate,
    )
}

pub(super) fn wizard_multi_spell_engine(
    known_spell_ids: &[&str],
    skill_rank: i32,
    tiles: Vec<&str>,
    target_position: Coord,
) -> Engine {
    wizard_multi_spell_engine_with_layout(known_spell_ids, skill_rank, tiles, target_position)
}

pub(super) fn wizard_multi_spell_engine_with_layout(
    known_spell_ids: &[&str],
    skill_rank: i32,
    tiles: Vec<&str>,
    target_position: Coord,
) -> Engine {
    wizard_multi_spell_engine_with_content_mutate(
        known_spell_ids,
        skill_rank,
        tiles,
        target_position,
        |_| {},
    )
}

pub(super) fn wizard_multi_spell_engine_with_content_mutate(
    known_spell_ids: &[&str],
    skill_rank: i32,
    tiles: Vec<&str>,
    target_position: Coord,
    mutate: impl FnOnce(&mut ContentParts),
) -> Engine {
    let known_spells = serde_json::Value::Array(
        known_spell_ids
            .iter()
            .map(|spell_id| {
                serde_json::json!({
                    "spell_id": spell_id,
                    "lane": "wizard_magic",
                    "learned_at_level": 1
                })
            })
            .collect(),
    );
    wizard_spell_engine_with_layout_known_spells_mutate(
        known_spells,
        skill_rank,
        tiles,
        target_position,
        mutate,
    )
}

pub(super) fn wizard_spell_engine_with_layout_known_spells(
    known_spells: serde_json::Value,
    skill_rank: i32,
    tiles: Vec<&str>,
    target_position: Coord,
) -> Engine {
    wizard_spell_engine_with_layout_known_spells_mutate(
        known_spells,
        skill_rank,
        tiles,
        target_position,
        |_| {},
    )
}

fn wizard_spell_engine_with_layout_known_spells_mutate(
    known_spells: serde_json::Value,
    skill_rank: i32,
    tiles: Vec<&str>,
    target_position: Coord,
    mutate: impl FnOnce(&mut ContentParts),
) -> Engine {
    let mut parts = wizard_spell_parts(known_spells, skill_rank, tiles, target_position);
    mutate(&mut parts);
    parts.engine(7).expect("wizard spell engine should start")
}

fn wizard_spell_parts(
    known_spells: serde_json::Value,
    skill_rank: i32,
    tiles: Vec<&str>,
    target_position: Coord,
) -> ContentParts {
    let mut parts = ContentParts::tracked("spell_readiness", "profile/spell_readiness");
    parts.profile_value_mut()["rules_profile"] = serde_json::json!("rules/first_room");
    parts.profile_value_mut()["spells"] = serde_json::json!([]);
    parts.profile_value_mut()["items"] = serde_json::json!([]);
    for spell in wizard_test_spells() {
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
            parts.push_selected("spells", &format!("spell/{id}/spell_test"), spell);
        }
    }
    *parts.template_levels_source_mut() = basic_rooms(tiles);
    *parts.item_instances_mut() = serde_json::json!({});
    *parts.ground_items_mut() = serde_json::json!([]);
    parts.actor_definition_mut(0)["name"] = serde_json::json!("Wiz");
    parts.actor_definition_mut(1)["name"] = serde_json::json!("Target");
    parts.actor_definition_mut(1)["stats"] =
        serde_json::json!({"hp": 5, "attack": 0, "defense": 0});
    let actors = parts.actors_mut().as_array_mut().expect("spell actors");
    actors[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    actors[0]["character"]["skill_ledger"] = serde_json::json!([{
        "track_id": "wizard_magic",
        "level": skill_rank,
        "critique_rank": 0,
        "practice_points": 0,
        "learning_rate": 1
    }]);
    actors[0]["character"]["known_spells"] = known_spells;
    actors[0]["carried"] = serde_json::json!({
        "items": [],
        "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}
    });
    actors[1]["id"] = serde_json::json!("target");
    actors[1]["location"]["position"] =
        serde_json::json!({"x": target_position.x, "y": target_position.y});
    parts
}

fn basic_rooms(tiles: Vec<&str>) -> serde_json::Value {
    let width = tiles.first().expect("room tiles").chars().count();
    let height = tiles.len();
    let cells = tiles
        .iter()
        .map(|row| {
            row.chars()
                .map(|glyph| {
                    vec![match glyph {
                        '#' => "stone_wall",
                        '.' => "flagstone",
                        _ => panic!("unmapped fixture glyph {glyph:?}"),
                    }]
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "room_0": {
            "law_zone": "none",
            "width": width,
            "height": height,
            "cells": cells
        }
    })
}

fn wizard_test_spells() -> Vec<serde_json::Value> {
    vec![
        stub_spell(
            "spark",
            "Spark",
            1,
            (3, 1),
            serde_json::json!({
                "kind": "actor", "range": 3, "requires_visible": true
            }),
            "direct",
            "character",
        ),
        stub_spell(
            "mend",
            "Mend",
            2,
            (5, 2),
            serde_json::json!({"kind": "self"}),
            "direct",
            "self",
        ),
        stub_spell(
            "charged_spark",
            "Charged Spark",
            1,
            (4, 1),
            serde_json::json!({
                "kind": "actor", "range": 3, "requires_visible": true
            }),
            "warm_then_cast",
            "character",
        ),
        stub_spell(
            "charged_mend",
            "Charged Mend",
            1,
            (2, 1),
            serde_json::json!({"kind": "self"}),
            "warm_then_cast",
            "self",
        ),
        stub_spell(
            "charged_ward",
            "Charged Ward",
            1,
            (2, 1),
            serde_json::json!({"kind": "none"}),
            "warm_then_cast",
            "not_applicable",
        ),
        stub_spell(
            "soft_mark",
            "Soft Mark",
            1,
            (1, 0),
            serde_json::json!({"kind": "actor"}),
            "direct",
            "character",
        ),
        stub_spell(
            "mark_coordinate",
            "Mark Coordinate",
            1,
            (1, 0),
            serde_json::json!({"kind": "coordinate"}),
            "direct",
            "not_applicable",
        ),
        stub_spell(
            "mark_area",
            "Mark Area",
            1,
            (1, 0),
            serde_json::json!({
                "kind": "area", "area": {"shape": "radius", "radius": 1}
            }),
            "direct",
            "not_applicable",
        ),
        stub_spell(
            "path_mark",
            "Path Mark",
            1,
            (3, 1),
            serde_json::json!({
                "kind": "coordinate", "range": 2, "requires_visible": true
            }),
            "direct",
            "path",
        ),
        stub_spell(
            "charged_path",
            "Charged Path",
            1,
            (4, 2),
            serde_json::json!({
                "kind": "coordinate", "range": 2, "requires_visible": true
            }),
            "warm_then_cast",
            "path",
        ),
    ]
}

fn stub_spell(
    id: &str,
    name: &str,
    skill_requirement: i32,
    costs: (i32, i32),
    target: serde_json::Value,
    method: &str,
    cast_class: &str,
) -> serde_json::Value {
    serde_json::json!({
        "social": {"hostile_act": false, "town_law": "permitted"},
        "id": id,
        "name": name,
        "status": "stub",
        "lane": "wizard_magic",
        "skill_requirement": skill_requirement,
        "mp_cost": costs.0,
        "stamina_cost": costs.1,
        "target": target,
        "casting": {"method": method, "cast_class": cast_class}
    })
}

pub(super) fn wizard_spell_engine_with_layout(
    known_spell_id: Option<&str>,
    skill_rank: i32,
    tiles: Vec<&str>,
    target_position: Coord,
) -> Engine {
    let known_spells: Vec<&str> = known_spell_id.into_iter().collect();
    wizard_multi_spell_engine_with_layout(&known_spells, skill_rank, tiles, target_position)
}

pub(super) fn attack_damage(events: &[Event]) -> i32 {
    events
        .iter()
        .find_map(|event| match event {
            Event::Attacked { damage, .. } => Some(*damage),
            _ => None,
        })
        .expect("attack should hit")
}
