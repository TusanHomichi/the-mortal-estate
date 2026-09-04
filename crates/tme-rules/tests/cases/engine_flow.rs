use crate::support::content_parts::ContentParts;
use tme_rules::{
    ActorKind, AutomaticActorDecisionV1, AutomaticMovementPurposeV1, AutomaticWaitReasonV1, Coord,
    Direction, Engine, Event, ExplicitTraversalKind, InspectActor, InspectExitStatus, LogicalTime,
    NavigationKind, PlayerIntent, VerticalDirection, WorldPosition,
};

fn tracked(case_id: &str) -> ContentParts {
    ContentParts::tracked(case_id, &format!("profile/{case_id}"))
}

fn layered_cells(rows: &[&str], mapping: &[(char, &str)]) -> serde_json::Value {
    serde_json::json!(
        rows.iter()
            .map(|row| {
                row.chars()
                    .map(|glyph| {
                        vec![
                            mapping
                                .iter()
                                .find_map(|(candidate, terrain)| {
                                    (*candidate == glyph).then_some(*terrain)
                                })
                                .unwrap_or_else(|| panic!("unmapped fixture glyph {glyph:?}")),
                        ]
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    )
}

fn has<E: AsRef<[Event]>, F: Fn(&Event) -> bool>(events: &E, f: F) -> bool {
    events.as_ref().iter().any(f)
}

fn hands_instance_id(actor: &tme_rules::ActorState) -> Option<&str> {
    actor
        .carried
        .items
        .get(&tme_rules::CarriedPosition::RightHand)
        .map(String::as_str)
}

fn first_room_engine() -> Engine {
    tracked("first_room")
        .engine(7)
        .expect("first-room content graph should start")
}

fn tied_player_weapon_engine(
    binding: serde_json::Value,
    starts_on_ground: bool,
    monster_x: i32,
) -> Engine {
    let mut parts = tracked("first_room");
    parts.item_instances_mut()["training_knife"]["binding"] = binding;
    let weapon = parts.selected_by_runtime_id_mut("items", "training_knife");
    weapon["weapon"]["default_attack_mode"] = serde_json::json!("poke");
    weapon["weapon"]["attack_modes"] = serde_json::json!([{
        "mode": "poke",
        "maximum_range": 1,
        "damage_kind": "piercing"
    }]);
    weapon["valid_placements"] = serde_json::json!(["hand", "sack"]);
    parts.actors_mut()[0]["character_id"] = serde_json::json!("character:tied_weapon_test:primary");
    parts.actor_definition_mut(0)["social"]["alignment_source"] =
        serde_json::json!({"kind": "character"});
    parts.actors_mut()[0]["character"] = serde_json::json!({
        "identity": {
            "base_class_id": "fighter",
            "current_class_id": "fighter",
            "display_class": "Fighter",
            "nationality_id": "aldland"
        },
        "alignment_state": {"alignment": "lawful", "karma_points": 0},
        "attributes": {
            "strength": 10,
            "dexterity": 10,
            "constitution": 10,
            "intelligence": 10,
            "wisdom": 10,
            "charisma": 10
        },
        "resources": {
            "hp": 12,
            "max_hp": 12,
            "peak_hp": 12,
            "mp": 0,
            "max_mp": 0,
            "stamina": 10,
            "max_stamina": 10
        },
        "progression": {"level": 1, "experience": 0},
        "physical_attribute_adds": {"strength_adds": 0, "dexterity_adds": 0},
        "promotion_history": [],
        "skill_ledger": []
    });
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": monster_x, "y": 1});
    parts.actor_definition_mut(1)["ai"]["behavior"] = serde_json::json!("hold_ground");
    parts.actor_definition_mut(1)["stats"]["attack"] = serde_json::json!(0);
    if starts_on_ground {
        parts.actors_mut()[0]["carried"]["items"] = serde_json::json!([]);
        *parts.ground_items_mut() = serde_json::json!([{
            "item_instance_id": "training_knife",
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 1, "y": 1}
            }
        }]);
    }
    parts.engine(7).expect("tied fixture should start")
}

fn player_attack_roll<E: AsRef<[Event]>>(events: &E) -> Option<i32> {
    events.as_ref().iter().find_map(|event| match event {
        Event::Attacked {
            attacker_id, roll, ..
        } if attacker_id == "player" => Some(*roll as i32),
        Event::AttackMissed {
            attacker_id, roll, ..
        } if attacker_id == "player" => Some(*roll),
        _ => None,
    })
}

fn inspect_edge_room_engine() -> Engine {
    let mut parts = tracked("first_room");
    *parts.template_levels_source_mut() = serde_json::json!({
        "room_0": {
            "law_zone": "none",
            "width": 3,
            "height": 3,
            "cells": layered_cells(
                &["###", "..#", "..#"],
                &[('#', "stone_wall"), ('.', "flagstone")]
            )
        }
    });
    parts.actors_mut()[0]["location"]["position"] = serde_json::json!({"x": 0, "y": 1});
    parts.actors_mut()[0]["carried"]["items"] = serde_json::json!([]);
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    parts.actor_definition_mut(1)["stats"]["attack"] = serde_json::json!(0);
    *parts.item_instances_mut() = serde_json::json!({});
    *parts.ground_items_mut() = serde_json::json!([]);
    parts
        .engine(7)
        .expect("inspect edge-room content should start")
}

fn terrain_engine() -> Engine {
    let mut parts = tracked("terrain_movement");
    *parts.template_levels_source_mut() = serde_json::json!({
        "room_0": {
            "law_zone": "none",
            "width": 6,
            "height": 5,
            "cells": layered_cells(
                &["######", "#..,,#", "#.~..#", "#....#", "######"],
                &[
                    ('#', "stone_wall"),
                    ('.', "flagstone"),
                    (',', "scrub"),
                    ('~', "deep_water")
                ]
            )
        }
    });
    parts.actors_mut()[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    parts.actors_mut()[0]["carried"]["items"] = serde_json::json!([]);
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 4, "y": 3});
    parts.actor_definition_mut(1)["stats"]["attack"] = serde_json::json!(0);
    *parts.item_instances_mut() = serde_json::json!({});
    *parts.ground_items_mut() = serde_json::json!([]);
    parts
        .engine(7)
        .expect("terrain fixture content should start")
}

fn terrain_attack_engine(monster_tile: char) -> Engine {
    let tiles = match monster_tile {
        ',' => ["#####", "#.,,#", "#...#", "#####"],
        '.' => ["#####", "#...#", "#.,.#", "#####"],
        _ => panic!("unsupported monster tile"),
    };
    let mut parts = tracked("terrain_movement");
    *parts.template_levels_source_mut() = serde_json::json!({
        "room_0": {
            "law_zone": "none",
            "width": 5,
            "height": 4,
            "cells": layered_cells(
                &tiles,
                &[('#', "stone_wall"), ('.', "flagstone"), (',', "scrub")]
            )
        }
    });
    parts.actors_mut()[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    parts.actor_definition_mut(0)["stats"]["attack"] = serde_json::json!(6);
    parts.actors_mut()[0]["carried"]["items"] = serde_json::json!([]);
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 3, "y": 1});
    parts.actor_definition_mut(1)["stats"]["attack"] = serde_json::json!(0);
    *parts.item_instances_mut() = serde_json::json!({});
    *parts.ground_items_mut() = serde_json::json!([]);
    parts
        .engine(7)
        .expect("terrain attack content should start")
}

fn reach_attack_engine(player_has_polearm: bool) -> Engine {
    let mut parts = tracked("reach_attack");
    parts.actor_definition_mut(1)["stats"]["attack"] = serde_json::json!(0);
    if !player_has_polearm {
        parts.actors_mut()[0]["carried"]["items"] = serde_json::json!([]);
        *parts.item_instances_mut() = serde_json::json!({});
    }
    parts
        .engine(1_010_580_540)
        .expect("reach attack content should start")
}

fn ranged_attack_engine(maximum_range: u32, monster_x: i32) -> Engine {
    let mut parts = tracked("ranged_attack");
    let attack_modes =
        &mut parts.selected_by_runtime_id_mut("items", "elm_bow")["weapon"]["attack_modes"];
    let shoot_mode = attack_modes
        .as_array_mut()
        .expect("bow attack modes")
        .iter_mut()
        .find(|mode| mode["mode"] == "shoot")
        .expect("bow shoot mode");
    shoot_mode["maximum_range"] = serde_json::json!(maximum_range);
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": monster_x, "y": 1});
    parts.actor_definition_mut(1)["stats"]["attack"] = serde_json::json!(0);
    parts.actor_definition_mut(1)["ai"]["behavior"] = serde_json::json!("hold_ground");
    parts
        .engine(1_010_580_540)
        .expect("ranged attack content should start")
}

fn thrown_attack_engine(maximum_range: u32, monster_x: i32, monster_hp: i32) -> Engine {
    let mut parts = tracked("thrown_attack");
    let attack_modes =
        &mut parts.selected_by_runtime_id_mut("items", "oak_javelin")["weapon"]["attack_modes"];
    let throw_mode = attack_modes
        .as_array_mut()
        .expect("javelin attack modes")
        .iter_mut()
        .find(|mode| mode["mode"] == "throw")
        .expect("javelin throw mode");
    throw_mode["maximum_range"] = serde_json::json!(maximum_range);
    let monster = &mut parts.actors_mut().as_array_mut().expect("seed actors")[1];
    monster["id"] = serde_json::json!("reedling");
    monster["location"]["position"] = serde_json::json!({"x": monster_x, "y": 1});
    let monster_definition = parts.actor_definition_mut(1);
    monster_definition["name"] = serde_json::json!("Reedling");
    monster_definition["stats"]["hp"] = serde_json::json!(monster_hp);
    monster_definition["stats"]["attack"] = serde_json::json!(0);
    parts
        .engine(1_010_580_540)
        .expect("thrown attack content should start")
}

fn non_weapon_hands_with_ground_weapon_engine() -> Engine {
    let mut parts = tracked("equipment_slots");
    parts.actors_mut()[0]["carried"]["items"] = serde_json::json!([{
        "item_instance_id": "leather_jerkin",
        "position": "right_hand"
    }]);
    parts
        .ground_items_mut()
        .as_array_mut()
        .expect("ground items array")
        .push(serde_json::json!({
            "item_instance_id": "training_knife",
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 1, "y": 1}
            }
        }));
    parts
        .engine(7)
        .expect("occupied-hands content should start")
}

fn resource_recovery_engine() -> Engine {
    let mut parts = tracked("resting_hollow");
    *parts.template_levels_source_mut() = serde_json::json!({
        "room_0": {
            "law_zone": "none",
            "width": 5,
            "height": 3,
            "cells": layered_cells(
                &["#####", "#...#", "#####"],
                &[('#', "stone_wall"), ('.', "flagstone")]
            )
        }
    });
    let actors = parts.actors_mut().as_array_mut().expect("seed actors");
    actors[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    actors[0]["character_id"] = serde_json::json!("character:resource_recovery_engine:primary");
    actors[0]["character"]["resources"] = serde_json::json!({
        "hp": 5,
        "max_hp": 12,
        "peak_hp": 12,
        "mp": 1,
        "max_mp": 4,
        "stamina": 3,
        "max_stamina": 6
    });
    actors[1]["location"]["position"] = serde_json::json!({"x": 2, "y": 1});
    parts.actor_definition_mut(0)["stats"]["attack"] = serde_json::json!(2);
    parts.actor_definition_mut(1)["stats"]["attack"] = serde_json::json!(0);
    parts
        .engine(7)
        .expect("resource-recovery content should start")
}

fn balm_engine(mut mutate: impl FnMut(&mut ContentParts)) -> Engine {
    let mut parts = tracked("balm_cache");
    parts.selected_by_runtime_id_mut("items", "spare_balm")["consumable"]["heal_per_round"] =
        serde_json::json!(3);
    parts.push_selected(
        "items",
        "item/weak_balm/engine_flow",
        serde_json::json!({
            "id": "weak_balm",
            "kind": "consumable",
            "valid_placements": ["hand", "sack"],
            "name": "Weak Balm",
            "consumable": {"effect": "healing", "heal_per_round": 1},
            "economy": {"unit_burden": 1}
        }),
    );
    parts.push_selected(
        "items",
        "item/hemp_cord/engine_flow",
        serde_json::json!({
            "id": "hemp_cord",
            "kind": "gear",
            "valid_placements": ["hand", "sack"],
            "name": "Hemp Cord",
            "economy": {"unit_burden": 1}
        }),
    );
    parts.item_instances_mut()["weak_balm"] = serde_json::json!({
        "definition_id": "weak_balm",
        "binding": {"state": "unrestricted"}
    });
    parts.item_instances_mut()["hemp_cord"] = serde_json::json!({
        "definition_id": "hemp_cord",
        "binding": {"state": "unrestricted"}
    });
    parts
        .ground_items_mut()
        .as_array_mut()
        .expect("ground items")
        .extend([
            serde_json::json!({
                "item_instance_id": "weak_balm",
                "location": {
                    "realm": "realm_0",
                    "level": "room_0",
                    "position": {"x": 1, "y": 1}
                }
            }),
            serde_json::json!({
                "item_instance_id": "hemp_cord",
                "location": {
                    "realm": "realm_0",
                    "level": "room_0",
                    "position": {"x": 1, "y": 1}
                }
            }),
        ]);
    parts.actors_mut()[0]["character_id"] = serde_json::json!("character:balm_engine:primary");
    parts.actors_mut()[0]["character"]["identity"]["nationality_id"] = serde_json::json!("test");
    parts.actors_mut()[0]["character"]["resources"] = serde_json::json!({
        "hp": 4,
        "max_hp": 8,
        "peak_hp": 8,
        "mp": 0,
        "max_mp": 0,
        "stamina": 10,
        "max_stamina": 10
    });
    parts.rules_source_mut()["resources"]["recovery_interval_units"] = serde_json::json!(1000);
    mutate(&mut parts);
    parts.engine(7).expect("balm content should start")
}

fn balm_events<E: AsRef<[Event]>>(events: &E) -> Vec<&Event> {
    events
        .as_ref()
        .iter()
        .filter(|event| matches!(event, Event::BalmHealed { .. }))
        .collect()
}

/// Rounds 1-4: take both balms, walk east, engage the warden.
/// Then `hits` wait rounds; the engaged warden deals exactly 1 damage each.
fn share_hex_and_take_hits(engine: &mut Engine, hits: usize) {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "healing_balm".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("round one should take the healing balm");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spare_balm".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem2,
                },
            },
        )
        .expect("round two should take the spare balm");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("round three should approach the warden");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("round four should engage the warden");
    for hit in 0..hits {
        engine
            .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
            .unwrap_or_else(|error| panic!("hit round {hit} should step: {error}"));
    }
}

fn multi_room_door_engine() -> Engine {
    let mut parts = tracked("first_room");
    *parts.template_levels_source_mut() = serde_json::json!({
        "home": {
            "law_zone": "none",
            "width": 5,
            "height": 3,
            "cells": layered_cells(
                &["#####", "#...#", "#####"],
                &[('#', "stone_wall"), ('.', "flagstone")]
            )
        },
        "den": {
            "law_zone": "none",
            "width": 5,
            "height": 3,
            "cells": layered_cells(
                &["#####", "....#", "#####"],
                &[('#', "stone_wall"), ('.', "flagstone")]
            )
        }
    });
    parts.world_template["topology"] = serde_json::json!({
        "edge/home/1/2": {
            "at": {"realm": "realm_0", "level": "home", "position": {"x": 2, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "den", "position": {"x": 1, "y": 1}
            }},
            "kind": {"kind": "door", "initial_state": "closed"},
            "hidden": false
        },
        "edge/den/1/0": {
            "at": {"realm": "realm_0", "level": "den", "position": {"x": 0, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "home", "position": {"x": 1, "y": 1}
            }},
            "kind": {"kind": "door", "initial_state": "closed"},
            "hidden": false
        }
    });
    let actors = parts.actors_mut().as_array_mut().expect("seed actors");
    actors[0]["location"]["level"] = serde_json::json!("home");
    actors[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    actors[0]["carried"]["items"] = serde_json::json!([]);
    actors[1]["id"] = serde_json::json!("warden");
    actors[1]["location"]["level"] = serde_json::json!("den");
    actors[1]["location"]["position"] = serde_json::json!({"x": 3, "y": 1});
    let warden_definition = parts.actor_definition_mut(1);
    warden_definition["name"] = serde_json::json!("Warden");
    warden_definition["stats"] = serde_json::json!({"hp": 7, "attack": 0, "defense": 0});
    warden_definition["ai"]["behavior"] = serde_json::json!("hold_ground");
    *parts.item_instances_mut() = serde_json::json!({});
    *parts.ground_items_mut() = serde_json::json!([]);
    parts
        .engine(7)
        .expect("multi-room door content should start")
}

fn multi_room_chase_engine(monster_ai: &str) -> Engine {
    let mut parts = tracked("first_room");
    *parts.template_levels_source_mut() = serde_json::json!({
        "home": {
            "law_zone": "none",
            "width": 5,
            "height": 3,
            "cells": layered_cells(
                &["#####", "#...#", "#####"],
                &[('#', "stone_wall"), ('.', "flagstone")]
            )
        },
        "den": {
            "law_zone": "none",
            "width": 5,
            "height": 3,
            "cells": layered_cells(
                &["....#", ".....", "#####"],
                &[('#', "stone_wall"), ('.', "flagstone")]
            )
        },
        "escape": {
            "law_zone": "none",
            "width": 3,
            "height": 3,
            "cells": layered_cells(
                &["###", "#.#", "###"],
                &[('#', "stone_wall"), ('.', "flagstone")]
            )
        }
    });
    parts.world_template["topology"] = serde_json::json!({
        "edge/home/1/2": {
            "at": {"realm": "realm_0", "level": "home", "position": {"x": 2, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "den", "position": {"x": 1, "y": 1}
            }},
            "kind": {"kind": "door", "initial_state": "open"},
            "hidden": false
        },
        "edge/den/0/0": {
            "at": {"realm": "realm_0", "level": "den", "position": {"x": 0, "y": 0}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "home", "position": {"x": 1, "y": 1}
            }},
            "kind": {"kind": "door", "initial_state": "open"},
            "hidden": false
        },
        "edge/den/0/3": {
            "at": {"realm": "realm_0", "level": "den", "position": {"x": 3, "y": 0}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "escape", "position": {"x": 1, "y": 1}
            }},
            "kind": {"kind": "stairs", "direction": "down"},
            "hidden": false
        }
    });
    let actors = parts.actors_mut().as_array_mut().expect("seed actors");
    actors[0]["location"]["level"] = serde_json::json!("den");
    actors[0]["location"]["position"] = serde_json::json!({"x": 2, "y": 0});
    actors[0]["carried"]["items"] = serde_json::json!([]);
    actors[1]["location"]["level"] = serde_json::json!("home");
    actors[1]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    parts.actor_definition_mut(0)["stats"]["attack"] = serde_json::json!(4);
    let monster_definition = parts.actor_definition_mut(1);
    monster_definition["stats"]["attack"] = serde_json::json!(0);
    monster_definition["ai"]["behavior"] = serde_json::json!(monster_ai);
    monster_definition["ai"]["leash_range"] = serde_json::json!(2);
    *parts.item_instances_mut() = serde_json::json!({});
    *parts.ground_items_mut() = serde_json::json!([]);
    parts
        .engine(7)
        .expect("multi-room chase content should start")
}

#[path = "engine_flow/tied_weapon_first_stable_character_touch_binds_and_can_attack.rs"]
mod tied_weapon_first_stable_character_touch_binds_and_can_attack;

#[path = "engine_flow/bare_handed_thrower_cannot_attack_at_distance.rs"]
mod bare_handed_thrower_cannot_attack_at_distance;

#[path = "engine_flow/balm_effect_ends_at_max_hp.rs"]
mod balm_effect_ends_at_max_hp;
