use crate::support::content_parts::ContentParts;
use tme_rules::*;

fn layered_cells(rows: &[&str]) -> serde_json::Value {
    serde_json::json!(
        rows.iter()
            .map(|row| {
                row.chars()
                    .map(|glyph| {
                        vec![match glyph {
                            '#' => "stone_wall",
                            '.' | 'H' | 'D' => "flagstone",
                            _ => panic!("unmapped fixture glyph {glyph:?}"),
                        }]
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    )
}

fn bu_door_secret_spell_engine(known_spell_ids: &[&str]) -> Engine {
    let mut parts = ContentParts::tracked(
        "utility_door_secret_item_spells",
        "profile/utility_door_secret_item_spells",
    );
    parts.world_template["id"] = serde_json::json!("bu_door_secret_spell_test");
    *parts.template_levels_source_mut() = serde_json::json!({
        "start": {
            "law_zone": "none",
            "width": 5,
            "height": 3,
            "cells": layered_cells(&["#####", "#.HD#", "#####"])
        },
        "vault": {
            "law_zone": "none",
            "width": 5,
            "height": 3,
            "cells": layered_cells(&["#####", "#...#", "#####"])
        }
    });
    parts.world_template["topology"] = serde_json::json!({
        "edge/start/1/2": {
            "at": {"realm": "realm_0", "level": "start", "position": {"x": 2, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "vault", "position": {"x": 1, "y": 1}
            }},
            "kind": {"kind": "door", "initial_state": "open"},
            "hidden": true
        },
        "edge/start/1/3": {
            "at": {"realm": "realm_0", "level": "start", "position": {"x": 3, "y": 1}},
            "target": {"kind": "position", "location": {
                "realm": "realm_0", "level": "vault", "position": {"x": 2, "y": 1}
            }},
            "kind": {"kind": "door", "initial_state": "closed"},
            "hidden": false
        }
    });
    *parts.item_instances_mut() = serde_json::json!({});
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
    parts.actor_definition_mut(0)["name"] = serde_json::json!("Wiz");
    let actors = parts.actors_mut().as_array_mut().expect("utility actors");
    actors[0]["location"]["level"] = serde_json::json!("start");
    actors[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    actors[0]["character"]["known_spells"] = serde_json::Value::Array(known_spells);
    actors[0]["carried"]["items"] = serde_json::json!([]);
    actors[1]["location"]["level"] = serde_json::json!("vault");
    actors[1]["location"]["position"] = serde_json::json!({"x": 3, "y": 1});
    actors[1]["carried"]["items"] = serde_json::json!([]);
    parts.profile_value_mut()["spells"] = serde_json::json!([
        "spell/open_gate/utility_door_secret_item_spells",
        "spell/close_gate/utility_door_secret_item_spells",
        "spell/sense_secret/utility_door_secret_item_spells",
        "spell/hide_secret/utility_door_secret_item_spells"
    ]);
    let mut knock =
        parts.catalog["spells"]["spell/open_gate/utility_door_secret_item_spells"].clone();
    knock["id"] = serde_json::json!("knock_adjacent");
    knock["name"] = serde_json::json!("Knock Adjacent");
    knock["effect"]["door_control"]["range"] = serde_json::json!(1);
    knock["target"] = serde_json::json!({"kind": "door"});
    parts.push_selected("spells", "spell/knock_adjacent/spell_doors_test", knock);
    parts.engine(7).expect("engine should start")
}

#[test]
fn door_control_coordinate_open_and_close_change_revealed_door_state() {
    let door_position = WorldPosition::new("realm_0", "start", Coord { x: 3, y: 1 });
    let mut engine = bu_door_secret_spell_engine(&["open_gate", "close_gate"]);

    let closed_preview = engine
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::East, Direction::East],
        )
        .expect("closed door preview");
    assert_eq!(
        closed_preview.stop_reason,
        tme_rules::MovementStopReason::FullPathAccepted
    );
    assert!(closed_preview.steps[1].opens_door);

    let opened = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "open_gate".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: door_position.clone(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("coordinate door open spell should execute");
    assert!(opened.events.iter().any(|event| matches!(
        event,
        Event::DoorOpened {
            actor_id,
            actor,
            location
        } if actor_id == "player"
            && actor == "Wiz"
            && location == &door_position
    )));
    assert!(
        !opened.events.iter().any(|event| matches!(
            event,
            Event::SpellCastStubbed { spell_id, .. } if spell_id == "open_gate"
        )),
        "supported door control must not be stubbed"
    );

    let open_preview = engine
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::East, Direction::East],
        )
        .expect("open door preview");
    assert_eq!(
        open_preview.stop_reason,
        tme_rules::MovementStopReason::FullPathAccepted
    );
    assert!(!open_preview.steps[1].opens_door);
    assert_eq!(open_preview.final_position.level, "vault");

    let closed = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "close_gate".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: door_position.clone(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("coordinate door close spell should execute");
    assert!(closed.events.iter().any(|event| matches!(
        event,
        Event::DoorClosed {
            actor_id,
            actor,
            location
        } if actor_id == "player"
            && actor == "Wiz"
            && location == &door_position
    )));

    let closed_again_preview = engine
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::East, Direction::East],
        )
        .expect("closed again preview");
    assert_eq!(
        closed_again_preview.stop_reason,
        tme_rules::MovementStopReason::FullPathAccepted
    );
    assert!(closed_again_preview.steps[1].opens_door);
}

#[test]
fn secret_detection_scan_reveals_and_hide_scan_hides_hidden_transition() {
    let secret_position = Coord { x: 2, y: 1 };
    let mut engine = bu_door_secret_spell_engine(&["sense_secret", "hide_secret"]);

    let hidden_preview = engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .expect("hidden preview");
    assert_eq!(
        hidden_preview.stop_reason,
        tme_rules::MovementStopReason::FullPathAccepted
    );
    assert!(
        engine
            .snapshot()
            .realms
            .iter()
            .find(|realm| realm.id == "realm_0")
            .expect("starter realm")
            .levels
            .iter()
            .find(|level| level.id == "start")
            .expect("start level")
            .tiles
            .iter()
            .find(|tile| tile.position == secret_position)
            .expect("secret tile")
            .transition
            .is_none()
    );

    let revealed = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "sense_secret".to_string(),
                target: None,
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("secret detection scan should execute");
    assert!(revealed.events.iter().any(|event| matches!(
        event,
        Event::SecretTransitionRevealed {
            actor_id,
            actor,
            location,
            transition_kind
        } if actor_id == "player"
            && actor == "Wiz"
            && location == &WorldPosition::new("realm_0", "start", secret_position)
            && transition_kind == "door"
    )));
    let revealed_preview = engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .expect("revealed preview");
    assert_eq!(
        revealed_preview.stop_reason,
        tme_rules::MovementStopReason::FullPathAccepted
    );

    let hidden = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "hide_secret".to_string(),
                target: None,
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("hide secret scan should execute");
    assert!(hidden.events.iter().any(|event| matches!(
        event,
        Event::SecretTransitionHidden {
            actor_id,
            actor,
            location,
            transition_kind
        } if actor_id == "player"
            && actor == "Wiz"
            && location == &WorldPosition::new("realm_0", "start", secret_position)
            && transition_kind == "door"
    )));
    let hidden_again_preview = engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .expect("hidden again preview");
    assert_eq!(
        hidden_again_preview.stop_reason,
        tme_rules::MovementStopReason::FullPathAccepted
    );
}

#[test]
fn adjacent_door_control_rejects_hidden_door_until_revealed() {
    let mut engine = bu_door_secret_spell_engine(&["knock_adjacent", "sense_secret"]);

    let hidden = engine.apply_actor_intent(
        &tme_rules::ActorId::from("player"),
        PlayerIntent::CastSpell {
            spell_id: "knock_adjacent".to_string(),
            target: Some(SpellTarget::Door {
                direction: Direction::East,
            }),
            authorization: tme_rules::HostilityAuthorization::Safe,
        },
    );
    assert_eq!(
        hidden
            .expect_err("hidden adjacent door should not validate")
            .message(),
        ActionBlockedReasonV1::InvalidTarget.code()
    );

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "sense_secret".to_string(),
                target: None,
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("reveal secret");
    let opened = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "knock_adjacent".to_string(),
                target: Some(SpellTarget::Door {
                    direction: Direction::East,
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("revealed adjacent door spell should execute");
    assert!(opened.events.iter().any(|event| matches!(
        event,
        Event::DoorOpened {
            location,
            ..
        } if location == &WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 })
    )));
}
