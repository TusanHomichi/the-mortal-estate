//! A world position is a realm *and* a level *and* coordinates.
//!
//! Issue #79: `hostile_spell_contact_target_indices` selected path-endpoint and
//! area/coordinate contacts by level name plus coordinates only, and
//! `apply_direct_damage_spell` repeated the same comparison in its fallback
//! selection. Two realms may each own a level of the same name, so that
//! comparison is not a location. The pre-fix behaviour was observed here first:
//! a cast aimed at `realm_0/room:(2,1)` damaged the actor standing at
//! `realm_1/room:(2,1)` as well as the local one.
//!
//! These cases drive the ordinary `CastSpell` command through the engine. They
//! assert on the authoritative world and on the checkpoint, not on a helper.

use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};
use tme_rules::*;

/// Room shape shared by both realms: `#####/#...#/#####`.
fn room_cells() -> Value {
    ["#####", "#...#", "#####"]
        .iter()
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
        .collect()
}

fn room() -> Value {
    json!({
        "law_zone": "none",
        "width": 5,
        "height": 3,
        "cells": room_cells()
    })
}

/// `sunder` is a hostile direct-damage spell. Content validation requires
/// `not_applicable` casting for coordinate and area targets, and `path` casting
/// for a walked path, so the path case uses its own authored row.
fn sunder_spell(target: Value, cast_class: &str, spell_id: &str) -> Value {
    json!({
        "social": {"hostile_act": true, "town_law": "permitted"},
        "id": spell_id,
        "name": "Sunder",
        "status": "draft",
        "lane": "wizard_magic",
        "skill_requirement": 1,
        "mp_cost": 2,
        "stamina_cost": 0,
        "effect": {
            "family": "direct_damage",
            "potency": 4,
            "damage_kind": "arcane",
            "resistance": {
                "role": "incoming",
                "tag": "arcane",
                "mitigation": {"mode": "half_damage", "rounding": "down", "minimum_damage": 1}
            }
        },
        "target": target,
        "casting": {"method": "direct", "cast_class": cast_class}
    })
}

/// Two realms, each owning a level called `room`. The caster and `near_twin`
/// stand in `realm_0`; `far_twin` stands at the *same level name and
/// coordinates* in `realm_1`. `far_level` renames the second realm's level so
/// the deliberately-distinct-name control is an additional case rather than the
/// only one.
/// The case's authored spell. Coordinate and area rows cast as
/// `not_applicable`; the walked-path row casts as `path`.
#[derive(Clone, Copy)]
enum Reach {
    Coordinate,
    Area,
    Path,
}

impl Reach {
    fn spell(self) -> Value {
        match self {
            Self::Coordinate => sunder_spell(
                json!({"kind": "coordinate", "range": 3, "requires_visible": true}),
                "not_applicable",
                "sunder",
            ),
            Self::Area => sunder_spell(
                json!({"kind": "area", "area": {"shape": "radius", "radius": 1}}),
                "not_applicable",
                "sunder",
            ),
            Self::Path => sunder_spell(
                json!({"kind": "coordinate", "range": 3, "requires_visible": true}),
                "path",
                "sunder_path",
            ),
        }
    }

    fn spell_id(self) -> &'static str {
        match self {
            Self::Path => "sunder_path",
            _ => "sunder",
        }
    }
}

fn two_realm_parts(reach: Reach, far_level: &str) -> ContentParts {
    let mut parts = ContentParts::tracked("spell_effects", "profile/spell_effects");
    parts.profile_value_mut()["rules_profile"] = json!("rules/first_room");
    parts.profile_value_mut()["spells"] = json!([]);
    parts.profile_value_mut()["items"] = json!([]);
    *parts.item_instances_mut() = json!({});
    parts.push_selected("spells", "spell/sunder/runtime_test", reach.spell());
    let digest = parts.world_template["visual_manifest_digest"].clone();
    parts.world_template = json!({
        "schema_version": 4,
        "kind": "world_template",
        "id": "realm_targeting_probe",
        "visual_manifest_digest": digest,
        "realms": {
            "realm_0": {"name": "Near", "levels": {"room": room()}},
            "realm_1": {"name": "Far", "levels": {far_level: room()}}
        },
        "arrivals": {},
        "resurrection": {},
        "topology": {}
    });

    let base = parts.actors_mut()[0].clone();
    let monster = parts.actors_mut()[1].clone();
    let caster = {
        let mut row = base.clone();
        row["id"] = json!("caster");
        row["location"] =
            json!({"realm": "realm_0", "level": "room", "position": {"x": 1, "y": 1}});
        row["character"]["resources"] = json!({
            "hp": 40, "max_hp": 40, "peak_hp": 40,
            "mp": 40, "max_mp": 40, "stamina": 20, "max_stamina": 20
        });
        row["character"]["known_spells"] =
            json!([{"spell_id": reach.spell_id(), "lane": "wizard_magic", "learned_at_level": 1}]);
        row["character"]["skill_ledger"] = json!([{
            "track_id": "wizard_magic", "level": 10, "critique_rank": 0,
            "practice_points": 0, "learning_rate": 1
        }]);
        row["carried"] = json!({"items": [], "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}});
        row
    };
    let stander = |id: &str, realm: &str, position: (i32, i32)| {
        let mut row = monster.clone();
        row["id"] = json!(id);
        row["location"] = json!({
            "realm": realm,
            "level": if realm == "realm_1" { far_level } else { "room" },
            "position": {"x": position.0, "y": position.1}
        });
        row["character"] = json!(null);
        row["npc"] = json!(null);
        row
    };
    *parts.actors_mut() = Value::Array(vec![
        caster,
        stander("near_twin", "realm_0", (2, 1)),
        stander("far_twin", "realm_1", (2, 1)),
    ]);
    parts.actor_definition_mut(0)["stats"] = json!({"hp": 40, "attack": 1, "defense": 0});
    parts.actor_definition_mut(1)["stats"] = json!({"hp": 40, "attack": 0, "defense": 0});
    parts.actor_definition_mut(1)["magic_resistance"]["natural_save_twentieths"] = json!(0);
    parts.actor_definition_mut(1)["death"] = json!({"remains": "none"});
    parts
}

fn engine(reach: Reach, far_level: &str) -> Engine {
    two_realm_parts(reach, far_level)
        .engine(7)
        .expect("two-realm engine should start")
}

fn cast(engine: &mut Engine, target: SpellTarget) -> Result<Vec<Event>, String> {
    cast_as(engine, "sunder", target)
}

fn cast_as(engine: &mut Engine, spell_id: &str, target: SpellTarget) -> Result<Vec<Event>, String> {
    engine
        .apply_actor_intent(
            &ActorId::from("caster"),
            PlayerIntent::CastSpell {
                spell_id: spell_id.to_string(),
                target: Some(target),
                authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
            },
        )
        .map(|outcome| outcome.events)
        .map_err(|error| error.to_string())
}

fn hp(engine: &Engine, actor_id: &str) -> i32 {
    engine
        .world()
        .actor(&ActorId::from(actor_id))
        .unwrap_or_else(|| panic!("actor {actor_id:?}"))
        .hp
}

fn active_effect_count(engine: &Engine, actor_id: &str) -> usize {
    engine
        .world()
        .actor(&ActorId::from(actor_id))
        .expect("actor")
        .active_effects
        .len()
}

/// The engine's own identity for one actor, as the other cases compare it.
fn location(engine: &Engine, actor_id: &str) -> WorldPosition {
    engine
        .world()
        .actor(&ActorId::from(actor_id))
        .expect("actor")
        .location
        .clone()
}

fn coordinate_target(realm: &str, level: &str, x: i32, y: i32) -> SpellTarget {
    SpellTarget::Coordinate {
        position: WorldPosition::new(realm, level, Coord { x, y }),
    }
}

fn assert_same_realm_fixture(engine: &Engine) {
    let near = location(engine, "near_twin");
    let far = location(engine, "far_twin");
    assert_eq!(
        (near.level.as_str(), near.position),
        (far.level.as_str(), far.position),
        "the two targets must differ only by realm for this fixture to mean anything"
    );
    assert_ne!(near.realm, far.realm);
}

#[test]
fn coordinate_cast_damages_only_the_local_actor_in_a_two_realm_world() {
    let mut engine = engine(Reach::Coordinate, "room");
    assert_same_realm_fixture(&engine);
    let before = engine.world().clone();
    let before_checkpoint = engine.export_checkpoint().expect("checkpoint");

    let events = cast(&mut engine, coordinate_target("realm_0", "room", 2, 1))
        .expect("the local coordinate is an admitted target");

    assert!(hp(&engine, "near_twin") < before.actor(&ActorId::from("near_twin")).unwrap().hp);
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::SpellDamaged { target_id, .. } if target_id.as_str() == "near_twin"
        )),
        "the local target must take the authored damage"
    );
    assert!(
        !events.iter().any(|event| matches!(
            event,
            Event::SpellDamaged { target_id, .. } if target_id.as_str() == "far_twin"
        )),
        "an off-realm actor must not appear as a damage target at all"
    );
    assert_eq!(
        hp(&engine, "far_twin"),
        before.actor(&ActorId::from("far_twin")).unwrap().hp,
        "an actor in another realm must not be damaged by a local cast"
    );
    assert_eq!(
        active_effect_count(&engine, "far_twin"),
        before
            .actor(&ActorId::from("far_twin"))
            .unwrap()
            .active_effects
            .len()
    );
    assert!(
        engine.world().corpses.is_empty(),
        "no off-realm defeat or corpse may be produced"
    );
    assert_eq!(
        location(&engine, "far_twin"),
        before.actor(&ActorId::from("far_twin")).unwrap().location,
        "an off-realm actor does not move"
    );
    // The cast still consumes exactly what a local cast consumes: the caster's
    // action, its MP and its own RNG advancement are unchanged by the presence
    // of an off-realm twin, which is proved by the equivalent control below.
    let _ = before_checkpoint;
}

#[test]
fn area_cast_damages_only_the_local_actor_in_a_two_realm_world() {
    let mut engine = engine(Reach::Area, "room");
    assert_same_realm_fixture(&engine);
    let far_hp = hp(&engine, "far_twin");

    cast(
        &mut engine,
        SpellTarget::Area {
            center: WorldPosition::new("realm_0", "room", Coord { x: 2, y: 1 }),
        },
    )
    .expect("the local area is an admitted target");

    assert!(
        hp(&engine, "near_twin") < 40,
        "the local actor takes damage"
    );
    assert_eq!(
        hp(&engine, "far_twin"),
        far_hp,
        "the off-realm actor takes none"
    );
}

#[test]
fn path_cast_damages_only_the_local_actor_in_a_two_realm_world() {
    let mut engine = engine(Reach::Path, "room");
    assert_same_realm_fixture(&engine);
    let far_hp = hp(&engine, "far_twin");

    cast_as(
        &mut engine,
        "sunder_path",
        SpellTarget::Path {
            directions: vec![Direction::East],
        },
    )
    .expect("the local path is an admitted target");

    assert!(
        hp(&engine, "near_twin") < 40,
        "the local actor takes damage"
    );
    assert_eq!(
        hp(&engine, "far_twin"),
        far_hp,
        "the off-realm actor takes none"
    );
}

#[test]
fn a_distinct_off_realm_level_name_is_an_additional_negative_control() {
    // The same cast with the other realm's level renamed must behave
    // identically: unique level names are not the mechanism under test.
    let mut engine = engine(Reach::Coordinate, "far_room");
    let far_hp = hp(&engine, "far_twin");
    assert_eq!(location(&engine, "far_twin").level, "far_room");

    cast(&mut engine, coordinate_target("realm_0", "room", 2, 1))
        .expect("the local coordinate is an admitted target");

    assert!(
        hp(&engine, "near_twin") < 40,
        "the local actor takes damage"
    );
    assert_eq!(
        hp(&engine, "far_twin"),
        far_hp,
        "the off-realm actor takes none"
    );
}

#[test]
fn a_cross_realm_coordinate_target_is_refused_before_any_state_changes() {
    let mut engine = engine(Reach::Coordinate, "room");
    let before = engine.export_checkpoint().expect("checkpoint");

    let error = cast(&mut engine, coordinate_target("realm_1", "room", 2, 1))
        .expect_err("a cross-realm coordinate is not a local target");
    assert!(error.contains("invalid_target"), "{error}");
    assert_eq!(
        engine.export_checkpoint().expect("checkpoint").as_bytes(),
        before.as_bytes(),
        "a refused cast rolls back completely"
    );
}

#[test]
fn an_off_realm_twin_does_not_change_local_outcomes_or_roll_evolution() {
    // Equivalent worlds, one with the off-realm actor removed. The cast must
    // produce the same events, the same damage and the same resulting engine
    // state, so no off-realm identity can enter target order, RNG or rewards.
    let with_twin = two_realm_parts(Reach::Coordinate, "room");
    let without_twin = {
        let mut parts = two_realm_parts(Reach::Coordinate, "room");
        let mut actors = parts.actors_mut().as_array().expect("actors").clone();
        actors.retain(|actor| actor["id"] != json!("far_twin"));
        *parts.actors_mut() = Value::Array(actors);
        parts
    };
    let mut solitary = without_twin.engine(7).expect("control engine");
    let mut paired = with_twin.engine(7).expect("paired engine");

    let control = cast(&mut solitary, coordinate_target("realm_0", "room", 2, 1))
        .expect("control cast resolves");
    let observed = cast(&mut paired, coordinate_target("realm_0", "room", 2, 1))
        .expect("paired cast resolves");

    // The extra actor legitimately produces its own readiness and decision
    // events when the clock drains, so compare everything the cast itself
    // decided: the resolution, damage, reward, practice and RNG evolution.
    fn resolution_events(events: &[Event]) -> Vec<Event> {
        events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    Event::SpellCastCommitted { .. }
                        | Event::SpellSaveResolved { .. }
                        | Event::SpellDamaged { .. }
                        | Event::DefeatContributionRecorded { .. }
                        | Event::MagicPracticeEvaluated { .. }
                        | Event::SkillPracticeAwarded { .. }
                )
            })
            .cloned()
            .collect()
    }
    assert_eq!(
        resolution_events(&control),
        resolution_events(&observed),
        "an off-realm actor must not change the cast's resolution, order or rewards"
    );
    assert_eq!(
        hp(&solitary, "near_twin"),
        hp(&paired, "near_twin"),
        "the local target takes the same damage"
    );
    let state = |engine: &Engine| {
        let bytes = engine.export_checkpoint().expect("checkpoint");
        let value: Value = serde_json::from_slice(bytes.as_bytes()).expect("checkpoint json");
        (
            value["rng_state"].clone(),
            value["world"]["actors"]
                .as_array()
                .expect("actors")
                .iter()
                .map(|actor| (actor["id"].clone(), actor["hp"].clone()))
                .collect::<Vec<_>>(),
            value["world"]["defeat_contributions"].clone(),
        )
    };
    let (control_rng, control_actors, control_contributions) = state(&solitary);
    let (paired_rng, paired_actors, paired_contributions) = state(&paired);
    assert_eq!(
        control_rng, paired_rng,
        "an off-realm actor must not consume or divert the cast's random rolls"
    );
    // The paired world legitimately holds one more actor; every actor the
    // control world has must end on the same HP, and the extra one must be
    // untouched at its authored maximum.
    let paired_shared = paired_actors
        .iter()
        .filter(|(id, _)| control_actors.iter().any(|(other, _)| other == id))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        control_actors, paired_shared,
        "the same actors end on the same HP"
    );
    let far_hp = paired_actors
        .iter()
        .find(|(id, _)| id == &json!("far_twin"))
        .expect("off-realm actor is retained");
    assert_eq!(
        far_hp.1,
        json!(40),
        "the off-realm actor keeps its authored maximum"
    );
    assert_eq!(
        control_contributions, paired_contributions,
        "an off-realm actor must not acquire target-derived reward credit"
    );
}
