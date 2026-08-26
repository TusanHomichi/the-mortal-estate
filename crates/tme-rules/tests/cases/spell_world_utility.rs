use crate::spell_effect_support::*;
use crate::support::content_parts::ContentParts;
use tme_rules::*;

fn layered_cells(rows: &[&str], floor: &str) -> serde_json::Value {
    serde_json::json!(
        rows.iter()
            .map(|row| {
                row.chars()
                    .map(|glyph| {
                        vec![match glyph {
                            '#' => "stone_wall",
                            '.' | 'S' => floor,
                            _ => panic!("unmapped fixture glyph {glyph:?}"),
                        }]
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    )
}

fn bu_locate_portal_spell_engine(known_spell_ids: &[&str]) -> Engine {
    bu_locate_portal_spell_engine_mutate(known_spell_ids, |_| {})
}

fn bu_locate_portal_spell_engine_mutate(
    known_spell_ids: &[&str],
    mutate: impl FnOnce(&mut ContentParts),
) -> Engine {
    let mut parts = bu_locate_portal_spell_parts(known_spell_ids);
    mutate(&mut parts);
    parts.engine(7).expect("world utility engine should start")
}

fn bu_locate_portal_spell_parts(known_spell_ids: &[&str]) -> ContentParts {
    let mut parts = ContentParts::tracked(
        "utility_door_secret_item_spells",
        "profile/utility_door_secret_item_spells",
    );
    parts.profile_value_mut()["rules_profile"] = serde_json::json!("rules/first_room");
    parts.profile_value_mut()["items"] = serde_json::json!([]);
    for item in utility_items() {
        let id = item["id"].as_str().expect("item id").to_string();
        select_or_push(
            &mut parts,
            "items",
            &format!("item/{id}/world_utility_test"),
            item,
        );
    }
    parts.profile_value_mut()["spells"] = serde_json::json!([]);
    for spell in utility_spells() {
        let id = spell["id"].as_str().expect("spell id").to_string();
        if id == "blue_gate" {
            parts.profile_value_mut()["spells"]
                .as_array_mut()
                .expect("spell selection")
                .push(serde_json::json!(
                    "spell/blue_gate/utility_door_secret_item_spells"
                ));
            continue;
        }
        select_or_push(
            &mut parts,
            "spells",
            &format!("spell/{id}/world_utility_test"),
            spell,
        );
    }
    *parts.template_levels_source_mut() = serde_json::json!({
        "start": {
            "law_zone": "none", "width": 5, "height": 3,
            "cells": layered_cells(&["#####", "#..S#", "#####"], "flagstone")
        },
        "vault": {
            "law_zone": "none", "width": 4, "height": 3,
            "cells": layered_cells(&["####", "#..#", "####"], "silver_floor")
        },
        "hidden_room": {
            "law_zone": "none", "width": 3, "height": 3,
            "cells": layered_cells(&["###", "#.#", "###"], "quiet_floor")
        }
    });
    parts.world_template["topology"] = serde_json::json!({
        "edge/start/1/3": {
            "at": {"realm": "realm_0", "level": "start", "position": {"x": 3, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "hidden_room", "position": {"x": 1, "y": 1}
            }},
            "kind": {"kind": "stairs", "direction": "down"},
            "hidden": true
        }
    });
    *parts.item_instances_mut() = serde_json::json!({
        "seeing_gem": {"definition_id": "seeing_gem", "binding": {"state": "unrestricted"}},
        "carried_gem": {"definition_id": "carried_gem", "binding": {"state": "unrestricted"}},
        "utility_blade": {"definition_id": "utility_blade", "binding": {"state": "unrestricted"}}
    });
    *parts.ground_items_mut() = serde_json::json!([{
        "item_instance_id": "seeing_gem",
        "location": {"realm": "realm_0", "level": "vault", "position": {"x": 1, "y": 1}}
    }]);
    let known = known_spell_ids
        .iter()
        .map(|spell_id| {
            serde_json::json!({
                "spell_id": spell_id, "lane": "wizard_magic", "learned_at_level": 1
            })
        })
        .collect::<Vec<_>>();
    parts.actor_definition_mut(0)["name"] = serde_json::json!("Wiz");
    parts.actor_definition_mut(0)["stats"] =
        serde_json::json!({"hp": 10, "attack": 1, "defense": 0});
    parts.actor_definition_mut(1)["name"] = serde_json::json!("Sentry");
    parts.actor_definition_mut(1)["stats"] =
        serde_json::json!({"hp": 4, "attack": 0, "defense": 0});
    let actors = parts.actors_mut().as_array_mut().expect("utility actors");
    actors[0]["location"]["level"] = serde_json::json!("start");
    actors[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    actors[0]["character"]["resources"] = serde_json::json!({
        "hp": 10, "max_hp": 10, "peak_hp": 10,
        "mp": 40, "max_mp": 40, "stamina": 20, "max_stamina": 20
    });
    actors[0]["character"]["known_spells"] = serde_json::Value::Array(known);
    actors[0]["carried"]["items"] = serde_json::json!([
        {"item_instance_id": "utility_blade", "position": "right_hand"},
        {"item_instance_id": "carried_gem", "position": "sack_item_1"}
    ]);
    actors[1]["id"] = serde_json::json!("sentry");
    actors[1]["location"]["level"] = serde_json::json!("vault");
    actors[1]["location"]["position"] = serde_json::json!({"x": 2, "y": 1});
    parts
}

fn select_or_push(parts: &mut ContentParts, registry: &str, key: &str, value: serde_json::Value) {
    let existing_key = parts.catalog[registry]
        .as_object()
        .unwrap_or_else(|| panic!("{registry} registry"))
        .iter()
        .find_map(|(candidate, existing)| (existing == &value).then(|| candidate.clone()));
    if let Some(existing_key) = existing_key {
        parts.profile_value_mut()[registry]
            .as_array_mut()
            .unwrap_or_else(|| panic!("{registry} selection"))
            .push(serde_json::Value::String(existing_key));
    } else {
        parts.push_selected(registry, key, value);
    }
}

fn utility_items() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "id": "seeing_gem", "kind": "trinket", "name": "Seeing Gem", "category": "tool",
            "valid_placements": ["hand", "sack"], "economy": {"unit_burden": 1}
        }),
        serde_json::json!({
            "id": "carried_gem", "kind": "trinket", "name": "Carried Gem", "category": "tool",
            "valid_placements": ["hand", "sack"], "economy": {"unit_burden": 1}
        }),
        serde_json::json!({
            "id": "utility_blade", "kind": "weapon", "name": "Utility Blade", "category": "sword",
            "weapon": {
                "skill_track_id": "sword", "default_attack_mode": "fight",
                "attack_modes": [{"mode": "fight", "maximum_range": 0, "damage_kind": "cutting"}],
                "cooldown_units": 1, "combat_add_rating": 0, "handedness": "one_handed", "block_value": 0
            },
            "valid_placements": ["hand", "belt_side", "belt_back", "sack"],
            "economy": {"unit_burden": 1}
        }),
    ]
}

fn utility_spells() -> Vec<serde_json::Value> {
    vec![
        utility_spell(
            "find_sentry",
            "Find Sentry",
            serde_json::json!({
                "family": "locate", "locate": {"subject": "actor", "id": "sentry"}
            }),
            serde_json::json!({"kind": "none"}),
        ),
        utility_spell(
            "find_secret_arch",
            "Find Secret Arch",
            serde_json::json!({
                "family": "locate", "locate": {"subject": "level", "id": "hidden_room"}
            }),
            serde_json::json!({"kind": "none"}),
        ),
        utility_spell(
            "find_carried_gem",
            "Find Carried Gem",
            serde_json::json!({
                "family": "locate", "locate": {"subject": "item", "id": "carried_gem", "observed_only": true}
            }),
            serde_json::json!({"kind": "none"}),
        ),
        utility_spell(
            "find_unobserved_gem",
            "Find Unobserved Gem",
            serde_json::json!({
                "family": "locate", "locate": {"subject": "item", "id": "seeing_gem"}
            }),
            serde_json::json!({"kind": "none"}),
        ),
        utility_spell(
            "find_utility_blade",
            "Find Utility Blade",
            serde_json::json!({
                "family": "locate", "locate": {"subject": "item", "id": "utility_blade", "observed_only": true}
            }),
            serde_json::json!({"kind": "none"}),
        ),
        utility_spell(
            "peek_vault",
            "Peek Vault",
            serde_json::json!({
                "family": "scry", "scry": {
                    "scope": "coordinate",
                    "site": {"realm": "realm_0", "level": "start"},
                    "position": {"x": 2, "y": 1}
                }
            }),
            serde_json::json!({"kind": "none"}),
        ),
        utility_spell(
            "peek_hidden_room",
            "Peek Hidden Room",
            serde_json::json!({
                "family": "scry", "scry": {
                    "scope": "level",
                    "site": {"realm": "realm_0", "level": "hidden_room"}
                }
            }),
            serde_json::json!({"kind": "none"}),
        ),
        utility_spell(
            "blue_gate",
            "Blue Gate",
            serde_json::json!({
                "family": "portal", "duration": {"policy": "rounds", "rounds": 1},
                "portal": {"target": {"kind": "position", "location": {
                    "realm": "realm_0", "level": "vault", "position": {"x": 1, "y": 1}
                }}, "two_way": true}
            }),
            serde_json::json!({"kind": "coordinate", "range": 2}),
        ),
    ]
}

fn utility_spell(
    id: &str,
    name: &str,
    effect: serde_json::Value,
    target: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "social": {"hostile_act": false, "town_law": "permitted"},
        "id": id, "name": name, "status": "draft", "lane": "wizard_magic",
        "skill_requirement": 1, "mp_cost": 1, "stamina_cost": 0,
        "effect": effect, "target": target,
        "casting": {"method": "direct", "cast_class": "not_applicable"}
    })
}

fn event_payload_value(event: &Event, key: &str) -> Option<serde_json::Value> {
    serde_json::to_value(event)
        .expect("event serializes")
        .get(key)
        .cloned()
}

#[test]
fn locate_and_scry_emit_bounded_observed_hints() {
    let mut engine = bu_locate_portal_spell_engine(&[
        "find_sentry",
        "find_secret_arch",
        "peek_vault",
        "peek_hidden_room",
    ]);

    let locate_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "find_sentry".to_string(),
                target: None,
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("locate actor should cast");
    let located = locate_events
        .iter()
        .find_map(|event| event_payload_value(event, "located"))
        .expect("locate should emit a bounded located event");
    assert_eq!(located["subject"], "actor");
    assert_eq!(located["id"], "sentry");
    assert_eq!(located["site"], serde_json::Value::Null);
    assert_eq!(located["location"], serde_json::Value::Null);
    assert_eq!(located["hint"], "actor sentry is hidden or unobserved");
    assert!(located.get("rooms").is_none(), "locate must not dump maps");
    assert!(
        !locate_events
            .iter()
            .any(|event| event_payload_value(event, "spell_cast_stubbed").is_some()),
        "supported locate must not be stubbed"
    );

    let hidden_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "find_secret_arch".to_string(),
                target: None,
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("locate hidden room should cast");
    let hidden_hint = hidden_events
        .iter()
        .find_map(|event| event_payload_value(event, "located"))
        .expect("hidden room locate should emit bounded hint");
    assert_eq!(hidden_hint["subject"], "level");
    assert_eq!(hidden_hint["id"], "hidden_room");
    assert_eq!(hidden_hint["site"], serde_json::Value::Null);
    assert_eq!(hidden_hint["location"], serde_json::Value::Null);
    assert_eq!(
        hidden_hint["hint"],
        "level hidden_room is hidden or unobserved"
    );

    let scry_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "peek_vault".to_string(),
                target: None,
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("scry should cast");
    let scry_hint = scry_events
        .iter()
        .find_map(|event| event_payload_value(event, "located"))
        .expect("scry should emit a bounded observed hint");
    assert_eq!(scry_hint["subject"], "scry");
    assert_eq!(scry_hint["id"], "peek_vault");
    assert_eq!(
        scry_hint["site"],
        serde_json::json!({"realm": "realm_0", "level": "start"})
    );
    assert_eq!(
        scry_hint["location"],
        serde_json::json!({
            "realm": "realm_0", "level": "start", "position": {"x": 2, "y": 1}
        })
    );
    assert_eq!(scry_hint["hint"], "scry peek_vault located in start at 2,1");

    let hidden_scry_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "peek_hidden_room".to_string(),
                target: None,
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("hidden room scry should cast");
    let hidden_scry_hint = hidden_scry_events
        .iter()
        .find_map(|event| event_payload_value(event, "located"))
        .expect("hidden room scry should emit bounded hint");
    assert_eq!(hidden_scry_hint["subject"], "scry");
    assert_eq!(hidden_scry_hint["id"], "peek_hidden_room");
    assert_eq!(hidden_scry_hint["site"], serde_json::Value::Null);
    assert_eq!(hidden_scry_hint["location"], serde_json::Value::Null);
    assert_eq!(
        hidden_scry_hint["hint"],
        "scry peek_hidden_room is hidden or unobserved"
    );
}

#[test]
fn locate_item_without_observed_only_does_not_disclose_unobserved_ground_item() {
    let mut engine = bu_locate_portal_spell_engine(&["find_unobserved_gem"]);

    let locate_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "find_unobserved_gem".to_string(),
                target: None,
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("locate unobserved item should cast");
    let located = locate_events
        .iter()
        .find_map(|event| event_payload_value(event, "located"))
        .expect("locate should emit a bounded unobserved-item hint");

    assert_eq!(located["subject"], "item");
    assert_eq!(located["id"], "seeing_gem");
    assert_eq!(located["site"], serde_json::Value::Null);
    assert_eq!(located["location"], serde_json::Value::Null);
    assert_eq!(located["hint"], "item seeing_gem is hidden or unobserved");
    assert!(located.get("rooms").is_none(), "locate must not dump maps");
    assert!(located.get("ground_items").is_none());
}

#[test]
fn locate_item_reports_observed_carried_item_holder_position() {
    let mut engine = bu_locate_portal_spell_engine(&["find_carried_gem"]);

    let locate_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "find_carried_gem".to_string(),
                target: None,
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("locate carried item should cast");
    let located = locate_events
        .iter()
        .find_map(|event| event_payload_value(event, "located"))
        .expect("locate should emit a bounded carried-item hint");

    assert_eq!(located["subject"], "item");
    assert_eq!(located["id"], "carried_gem");
    assert_eq!(
        located["site"],
        serde_json::json!({"realm": "realm_0", "level": "start"})
    );
    assert_eq!(
        located["location"],
        serde_json::json!({
            "realm": "realm_0", "level": "start", "position": {"x": 1, "y": 1}
        })
    );
    assert_eq!(located["hint"], "item carried_gem located in start at 1,1");
    assert!(located.get("inventory").is_none());
    assert!(located.get("equipment").is_none());
}

#[test]
fn locate_item_reports_observed_equipped_item_holder_position() {
    let mut engine = bu_locate_portal_spell_engine(&["find_utility_blade"]);

    let locate_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "find_utility_blade".to_string(),
                target: None,
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("locate equipped item should cast");
    let located = locate_events
        .iter()
        .find_map(|event| event_payload_value(event, "located"))
        .expect("locate should emit a bounded equipped-item hint");

    assert_eq!(located["subject"], "item");
    assert_eq!(located["id"], "utility_blade");
    assert_eq!(
        located["site"],
        serde_json::json!({"realm": "realm_0", "level": "start"})
    );
    assert_eq!(
        located["location"],
        serde_json::json!({
            "realm": "realm_0", "level": "start", "position": {"x": 1, "y": 1}
        })
    );
    assert_eq!(
        located["hint"],
        "item utility_blade located in start at 1,1"
    );
    assert!(located.get("inventory").is_none());
    assert!(located.get("equipment").is_none());
}

#[test]
fn passable_overlay_on_authored_wall_does_not_make_portal_destination_legal() {
    let mut parts = bu_locate_portal_spell_parts(&["blue_gate"]);
    parts.template_levels_source_mut()["vault"]["cells"] =
        layered_cells(&["####", "##.#", "####"], "silver_floor");
    let error = match parts.engine(7) {
        Ok(_) => {
            panic!("authored wall portal destination must reject before runtime overlays exist")
        }
        Err(error) => error,
    };
    assert!(
        error.contains("effect.portal.target is not traversable at realm_0/vault:1,1"),
        "unexpected portal validation error: {error}"
    );
}

#[test]
fn blocked_overlay_on_authored_floor_does_not_make_portal_destination_illegal() {
    let anchor = WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 });
    let mut engine = bu_locate_portal_spell_engine(&["blue_gate"]);
    add_test_tile_passability_effect(&mut engine, "vault", Coord { x: 1, y: 1 }, "blocked");

    let creation = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "blue_gate".to_string(),
                target: Some(SpellTarget::Coordinate { position: anchor }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("portal destination should use authored floor despite blocked overlay");

    assert!(
        creation
            .iter()
            .any(|event| event_payload_value(event, "portal_created").is_some()),
        "portal should be created for authored floor destination"
    );
}

#[test]
fn portal_creation_transition_movement_and_expiration_use_effective_edges() {
    let anchor = WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 });
    let mut engine = bu_locate_portal_spell_engine(&["blue_gate"]);

    let creation = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "blue_gate".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: anchor.clone(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("portal spell should cast");
    let created = creation
        .iter()
        .find_map(|event| event_payload_value(event, "portal_created"))
        .expect("portal creation event should be emitted");
    assert_eq!(created["actor_id"], "player");
    assert_eq!(created["instance_id"], "portal:blue_gate:1:start:2:1");
    assert_eq!(
        created["location"],
        serde_json::json!({
            "realm": "realm_0", "level": "start", "position": {"x": 2, "y": 1}
        })
    );
    assert_eq!(
        created["target"],
        serde_json::json!({
            "realm": "realm_0", "level": "vault", "position": {"x": 1, "y": 1}
        })
    );
    assert_eq!(created["remaining_rounds"], 1);
    assert_eq!(created["two_way"], true);

    let preview = engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .expect("portal preview should build");
    assert_eq!(
        preview.stop_reason,
        tme_rules::MovementStopReason::Transitioned
    );
    assert!(matches!(
        preview.steps[0].outcome,
        tme_rules::PathPreviewStepOutcomeV1::Transitioned { kind, ref to }
            if serde_json::to_value(kind).expect("kind serializes") == "portal"
                && *to == WorldPosition::new("realm_0", "vault", Coord { x: 1, y: 1 })
    ));

    let movement = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("movement through portal should succeed");
    assert!(movement.iter().any(|event| matches!(
        event,
        Event::WorldTransition {
            from,
            to,
            ..
        } if from.level == "start"
            && to.level == "vault"
            && to.position == Coord { x: 1, y: 1 }
    )));
    assert!(
        movement
            .iter()
            .any(|event| event_payload_value(event, "portal_expired").is_some()),
        "round tick should emit portal expiration"
    );

    let mut fresh = bu_locate_portal_spell_engine(&["blue_gate"]);
    fresh
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "blue_gate".to_string(),
                target: Some(SpellTarget::Coordinate { position: anchor }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("portal spell should cast");
    fresh
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("portal should expire");
    let expired_preview = fresh
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .expect("expired portal preview should build");
    assert_eq!(
        expired_preview.stop_reason,
        tme_rules::MovementStopReason::FullPathAccepted
    );
    assert!(matches!(
        expired_preview.steps[0].outcome,
        tme_rules::PathPreviewStepOutcomeV1::Moved { .. }
    ));
}

#[test]
fn locate_and_scry_do_not_reveal_hidden_transitions() {
    let secret_position = Coord { x: 3, y: 1 };
    let mut engine =
        bu_locate_portal_spell_engine(&["find_secret_arch", "peek_hidden_room", "blue_gate"]);

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "find_secret_arch".to_string(),
                target: None,
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("locate hidden room should cast");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "peek_hidden_room".to_string(),
                target: None,
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("scry should cast");

    let snapshot = engine.snapshot();
    let transition_after_info_spells = snapshot
        .realms
        .iter()
        .find(|realm| realm.id == "realm_0")
        .and_then(|realm| realm.levels.iter().find(|level| level.id == "start"))
        .and_then(|level| {
            level
                .tiles
                .iter()
                .find(|tile| tile.position == secret_position)
        })
        .and_then(|tile| tile.transition.as_ref());
    assert_eq!(
        transition_after_info_spells, None,
        "locate and scry must not reveal hidden transitions"
    );
}
