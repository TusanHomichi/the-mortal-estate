#![allow(dead_code)]

pub use crate::support::content_parts::ContentParts;
use tme_rules::model::{
    ActiveEffectSource, ActiveEffectStackingPolicy, ActiveEffectState, LogicalTime,
};
use tme_rules::{AutomaticActorDecisionV1, Coord, Engine, Event, PlayerIntent};

pub fn ai(
    behavior: &str,
    cadence_units: u32,
    leash_range: u32,
    awareness: serde_json::Value,
    physical_attack_modes: &[&str],
) -> serde_json::Value {
    serde_json::json!({
        "behavior": behavior,
        "cadence_units": cadence_units,
        "aggro_radius": leash_range,
        "leash_range": leash_range,
        "awareness": awareness,
        "physical_attack_modes": physical_attack_modes,
    })
}

pub fn unrestricted() -> serde_json::Value {
    serde_json::json!({"mode": "unrestricted"})
}

pub fn line_of_sight_memory(opportunities: u32) -> serde_json::Value {
    serde_json::json!({
        "mode": "line_of_sight_memory",
        "memory_opportunities": opportunities,
    })
}

pub fn automatic_actor(
    id: &str,
    alignment: &str,
    position: Coord,
    behavior: &str,
    cadence_units: u32,
    awareness: serde_json::Value,
    physical_attack_modes: &[&str],
) -> serde_json::Value {
    let social_behavior = if alignment == "neutral" {
        "passive"
    } else {
        "alignment_creature"
    };
    serde_json::json!({
        "id": id,
        "kind": "monster",
        "npc": null,
        "social": {
            "alignment_source": {"kind": "inherent", "alignment": alignment},
            "nature": "other",
            "behavior": social_behavior,
            "owner_relation": "none"
        },
        "magic_resistance": {"natural_save_twentieths": 5, "evidence_state": "original_provisional"},
        "death": {"remains": "searchable_corpse"},
        "name": id,
        "location": {"realm": "realm_0", "level": "room_0", "position": position},
        "stats": {"hp": 20, "attack": 0, "defense": 0},
        "ai": ai(
            behavior,
            cadence_units,
            12,
            awareness,
            physical_attack_modes,
        ),
        "carried": {"items": [], "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}},
    })
}

pub fn active_effect(id: &str, tag: &str, suppresses_action: bool) -> serde_json::Value {
    serde_json::json!({
        "instance_id": id,
        "effect_id": id,
        "source": {"kind": "fixture", "id": "automatic_actor_tests"},
        "kind": "control_status",
        "tags": [tag],
        "potency": 0,
        "remaining_rounds": 20,
        "stacking": "refresh_duration",
        "start_delay_rounds": 0,
        "tick_interval_rounds": 1,
        "suppresses_action": suppresses_action,
        "resistance_boosts": [],
    })
}

pub fn open_room_value(automatic_actors: Vec<serde_json::Value>) -> ContentParts {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.template_levels_source_mut()["room_0"] = serde_json::json!({
        "law_zone": "none",
        "scene_role": "combat_space",
        "presentation_mode": "combat_space",
        "world_zoom": {"screen_cell_pitch": [156, 104]},
        "maximum_clear_sightline": 7,
        "staged_viewport": null,
        "wall_terrain_ids": [],
        "static_props": [],
        "width": 9,
        "height": 5,
        "cells": [
            [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]],
            [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
            [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
            [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
            [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]]
        ]
    });
    parts.actors_mut()[0]["location"]["level"] = serde_json::json!("room_0");
    parts.actors_mut()[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 2});
    let player = parts.world_seed["actors"][0].clone();
    *parts.actors_mut() = serde_json::Value::Array(vec![player]);
    for actor in automatic_actors {
        push_automatic_actor(&mut parts, actor);
    }
    parts
}

pub fn push_automatic_actor(parts: &mut ContentParts, mut actor: serde_json::Value) {
    let object = actor.as_object_mut().expect("automatic actor test row");
    let id = object["id"]
        .as_str()
        .expect("automatic actor id")
        .to_string();
    let definition_id = format!("actor/test/{id}");
    let definition = serde_json::json!({
        "id": definition_id.clone(),
        "kind": object.remove("kind").expect("automatic actor kind"),
        "name": object.remove("name").expect("automatic actor name"),
        "creature_traits": object.remove("creature_traits").unwrap_or_else(|| serde_json::json!([])),
        "stats": object.remove("stats").expect("automatic actor stats"),
        "magic_resistance": object.remove("magic_resistance").expect("automatic actor magic resistance"),
        "death": object.remove("death").expect("automatic actor death"),
        "social": object.remove("social").expect("automatic actor social profile"),
        "ai": object.remove("ai").expect("automatic actor AI"),
        "xp_value": object.remove("xp_value").unwrap_or(serde_json::Value::Null),
        "physical_damage_affinity_profile_id": "ordinary",
        "monster_abilities": object.remove("monster_abilities").unwrap_or_else(|| serde_json::json!([])),
    });
    object.insert(
        "actor_definition_id".to_string(),
        serde_json::Value::String(definition_id),
    );
    parts.push_selected(
        "actor_definitions",
        &format!("actor/test/{id}/ai_support"),
        definition,
    );
    parts
        .actors_mut()
        .as_array_mut()
        .expect("seed actors")
        .push(actor);
}

pub fn engine(automatic_actors: Vec<serde_json::Value>) -> Engine {
    open_room_value(automatic_actors)
        .engine(7)
        .expect("focused AI engine starts")
}

pub fn engine_from_value(value: ContentParts) -> Engine {
    value.engine(7).expect("focused engine starts")
}

pub fn wait(engine: &mut Engine) -> Vec<Event> {
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("controlled wait succeeds")
        .events
}

pub fn decision<'a>(events: &'a [Event], actor_id: &str) -> &'a AutomaticActorDecisionV1 {
    events
        .iter()
        .find_map(|event| match event {
            Event::AutomaticActorDecision {
                actor_id: candidate,
                decision,
                ..
            } if candidate == actor_id => Some(decision),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no automatic decision for {actor_id}"))
}

pub fn decisions<'a>(
    events: &'a [Event],
    actor_id: &'a str,
) -> impl Iterator<Item = &'a AutomaticActorDecisionV1> + 'a {
    events.iter().filter_map(move |event| match event {
        Event::AutomaticActorDecision {
            actor_id: candidate,
            decision,
            ..
        } if candidate == actor_id => Some(decision),
        _ => None,
    })
}

pub fn actor_position(engine: &Engine, actor_id: &str) -> Coord {
    engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == actor_id)
        .unwrap_or_else(|| panic!("missing actor {actor_id}"))
        .location
        .position
}

pub fn set_actor_hidden(engine: &mut Engine, actor_id: &str, hidden: bool) {
    let actor = engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == actor_id)
        .unwrap_or_else(|| panic!("missing actor {actor_id}"));
    actor
        .active_effects
        .retain(|effect| !effect.tags.iter().any(|tag| tag == "hidden"));
    if hidden {
        actor.active_effects.push(ActiveEffectState {
            spell_damage_credit: None,
            instance_id: format!("hidden:{actor_id}"),
            effect_id: "hidden".to_string(),
            source: ActiveEffectSource {
                kind: "fixture".to_string(),
                id: "automatic_actor_tests".to_string(),
            },
            source_actor_id: None,
            hostile_authority: None,
            kind: "control_status".to_string(),
            tags: vec!["hidden".to_string()],
            potency: 0,
            remaining_rounds: Some(20),
            until_condition: None,
            stacking: ActiveEffectStackingPolicy::RefreshDuration,
            start_delay_rounds: 0,
            tick_interval_rounds: 1,
            suppresses_action: false,
            resistance_boosts: Vec::new(),
            last_ticked_at: LogicalTime::ZERO,
        });
    }
}
