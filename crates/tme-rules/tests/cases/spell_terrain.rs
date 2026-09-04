use crate::spell_effect_support::*;
use tme_rules::*;

fn bt_spell_engine(known_spell_ids: &[&str]) -> Engine {
    bt_spell_engine_with_damage_interruption(known_spell_ids, false)
}

fn bt_spell_engine_with_damage_interruption(
    known_spell_ids: &[&str],
    force_interruption: bool,
) -> Engine {
    bs_runtime_spell_engine_mutate(
        known_spell_ids,
        vec!["#####", "#...#", "#####"],
        Coord { x: 2, y: 1 },
        |parts| {
            let mut push_spell = |mut spell: serde_json::Value| {
                let id = spell["id"].as_str().expect("spell id").to_string();
                if ["web_field", "steady_light", "deep_darkness"].contains(&id.as_str()) {
                    spell["effect"]["duration"]["rounds"] = serde_json::json!(3);
                }
                let canonical_key = match id.as_str() {
                    "web_field" => Some("spell/web_field/area_path_terrain_spells"),
                    "ember_cloud" => Some("spell/ember_cloud/area_path_terrain_spells"),
                    _ => None,
                };
                if let Some(key) = canonical_key {
                    if id == "web_field" {
                        parts.catalog["spells"][key]["effect"]["duration"]["rounds"] =
                            serde_json::json!(3);
                    }
                    parts.profile_value_mut()["spells"]
                        .as_array_mut()
                        .expect("spell selection")
                        .push(serde_json::json!(key));
                } else {
                    parts.push_selected("spells", &format!("spell/{id}/terrain_test"), spell);
                }
            };
            push_spell(serde_json::json!({
                "id": "web_field",
                "name": "Web Field",
                "status": "draft",
                "lane": "wizard_magic",
                "skill_requirement": 1,
                "mp_cost": 2,
                "stamina_cost": 1,
                "social": {"hostile_act": false, "town_law": "permitted"},
                "effect": {
                    "family": "terrain_overlay",
                    "status_kind": "web",
                    "potency": 0,
                    "terrain_overlay": {
                        "passability": "hindered",
                        "sight": "obscured",
                        "move_cost": 2
                    },
                    "duration": {"policy": "rounds", "rounds": 2}
                },
                "target": {
                    "kind": "area",
                    "range": 4,
                    "requires_visible": true,
                    "area": {"shape": "radius", "radius": 1}
                },
                "casting": {"method": "direct", "cast_class": "not_applicable"}
            }));
            push_spell(serde_json::json!({
                "id": "ember_cloud",
                "name": "Ember Cloud",
                "status": "draft",
                "lane": "wizard_magic",
                "skill_requirement": 1,
                "mp_cost": 2,
                "stamina_cost": 1,
                "social": {"hostile_act": false, "town_law": "permitted"},
                "effect": {
                    "family": "terrain_overlay",
                    "status_kind": "ember",
                    "potency": 2,
                    "terrain_overlay": {
                        "hazard": "fire"
                    },
                    "duration": {"policy": "rounds", "rounds": 2, "tick_interval_rounds": 1}
                },
                "target": {
                    "kind": "coordinate",
                    "range": 4,
                    "requires_visible": true
                },
                "casting": {"method": "direct", "cast_class": "not_applicable"}
            }));
            push_spell(serde_json::json!({
                "id": "stone_field",
                "name": "Stone Field",
                "status": "draft",
                "lane": "wizard_magic",
                "skill_requirement": 1,
                "mp_cost": 2,
                "stamina_cost": 1,
                "social": {"hostile_act": false, "town_law": "permitted"},
                "effect": {
                    "family": "terrain_overlay",
                    "status_kind": "stone",
                    "potency": 0,
                    "terrain_overlay": {
                        "passability": "blocked"
                    },
                    "duration": {"policy": "rounds", "rounds": 4}
                },
                "target": {
                    "kind": "area",
                    "range": 4,
                    "requires_visible": true,
                    "area": {"shape": "radius", "radius": 1}
                },
                "casting": {"method": "direct", "cast_class": "not_applicable"}
            }));
            push_spell(serde_json::json!({
                "id": "clear_field",
                "name": "Clear Field",
                "status": "draft",
                "lane": "wizard_magic",
                "skill_requirement": 1,
                "mp_cost": 2,
                "stamina_cost": 1,
                "social": {"hostile_act": false, "town_law": "permitted"},
                "effect": {
                    "family": "terrain_overlay",
                    "status_kind": "clear",
                    "potency": 0,
                    "terrain_overlay": {
                        "passability": "remove_overlay"
                    },
                    "duration": {"policy": "rounds", "rounds": 1}
                },
                "target": {
                    "kind": "area",
                    "range": 4,
                    "requires_visible": true,
                    "area": {"shape": "radius", "radius": 1}
                },
                "casting": {"method": "direct", "cast_class": "not_applicable"}
            }));
            push_spell(serde_json::json!({
                "id": "dark_field",
                "name": "Dark Field",
                "status": "draft",
                "lane": "wizard_magic",
                "skill_requirement": 1,
                "mp_cost": 2,
                "stamina_cost": 1,
                "social": {"hostile_act": false, "town_law": "permitted"},
                "effect": {
                    "family": "terrain_overlay",
                    "status_kind": "darkness",
                    "potency": 0,
                    "terrain_overlay": {
                        "sight": "blocked"
                    },
                    "duration": {"policy": "rounds", "rounds": 4}
                },
                "target": {
                    "kind": "coordinate",
                    "range": 4,
                    "requires_visible": true
                },
                "casting": {"method": "direct", "cast_class": "not_applicable"}
            }));
            push_spell(serde_json::json!({
                "id": "sight_clear_field",
                "name": "Sight Clear Field",
                "status": "draft",
                "lane": "wizard_magic",
                "skill_requirement": 1,
                "mp_cost": 2,
                "stamina_cost": 1,
                "social": {"hostile_act": false, "town_law": "permitted"},
                "effect": {
                    "family": "terrain_overlay",
                    "status_kind": "clear_sight",
                    "potency": 0,
                    "terrain_overlay": {
                        "sight": "remove_overlay"
                    },
                    "duration": {"policy": "rounds", "rounds": 1}
                },
                "target": {
                    "kind": "coordinate",
                    "range": 4,
                    "requires_visible": true
                },
                "casting": {"method": "direct", "cast_class": "not_applicable"}
            }));
            push_spell(serde_json::json!({
                "id": "steady_light",
                "name": "Steady Light",
                "status": "draft",
                "lane": "wizard_magic",
                "skill_requirement": 1,
                "mp_cost": 2,
                "stamina_cost": 1,
                "social": {"hostile_act": false, "town_law": "permitted"},
                "effect": {
                    "family": "light",
                    "status_kind": "illumination",
                    "terrain_overlay": {"sight": "clear"},
                    "duration": {"policy": "rounds", "rounds": 2}
                },
                "target": {
                    "kind": "coordinate",
                    "range": 4,
                    "requires_visible": true
                },
                "casting": {"method": "direct", "cast_class": "not_applicable"}
            }));
            push_spell(serde_json::json!({
                "id": "deep_darkness",
                "name": "Deep Darkness",
                "status": "draft",
                "lane": "wizard_magic",
                "skill_requirement": 1,
                "mp_cost": 2,
                "stamina_cost": 1,
                "social": {"hostile_act": false, "town_law": "permitted"},
                "effect": {
                    "family": "darkness",
                    "status_kind": "darkness",
                    "terrain_overlay": {"sight": "blocked"},
                    "duration": {"policy": "rounds", "rounds": 2}
                },
                "target": {
                    "kind": "coordinate",
                    "range": 4,
                    "requires_visible": true
                },
                "casting": {"method": "direct", "cast_class": "not_applicable"}
            }));
            if force_interruption {
                parts.rules_source_mut()["magic"]["damage_interruption"]["numerator"] =
                    serde_json::json!(1);
                parts.rules_source_mut()["magic"]["damage_interruption"]["denominator"] =
                    serde_json::json!(100);
            }
        },
    )
}

#[test]
fn light_and_darkness_families_use_tile_effect_owner_without_stub() {
    let mut engine = bt_spell_engine(&["steady_light", "deep_darkness"]);
    for (spell_id, x, expected_sight) in [
        ("steady_light", 1, "clear"),
        ("deep_darkness", 3, "blocked"),
    ] {
        let events = engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::CastSpell {
                    spell_id: spell_id.to_string(),
                    target: Some(SpellTarget::Coordinate {
                        position: WorldPosition::new("realm_0", "room_0", Coord { x, y: 1 }),
                    }),
                    authorization: tme_rules::HostilityAuthorization::Safe,
                },
            )
            .unwrap_or_else(|error| panic!("{spell_id} should cast: {error}"));
        assert!(events.iter().any(|event| matches!(
            event,
            Event::TileEffectApplied { effect_id, sight, .. }
                if effect_id == spell_id && sight.as_deref() == Some(expected_sight)
        )));
        assert!(!events.iter().any(|event| matches!(
            event,
            Event::SpellCastStubbed { spell_id: stubbed, .. } if stubbed == spell_id
        )));
    }

    let effects = &engine.world().tile_effects;
    assert!(effects.iter().any(|effect| {
        effect.effect_id == "steady_light"
            && effect.kind == "light"
            && effect.sight.as_deref() == Some("clear")
    }));
    assert!(effects.iter().any(|effect| {
        effect.effect_id == "deep_darkness"
            && effect.kind == "darkness"
            && effect.sight.as_deref() == Some("blocked")
    }));
}

#[test]
fn terrain_overlay_spell_applies_area_tile_effects_without_stub() {
    let mut engine = bt_spell_engine(&["web_field"]);
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "web_field".to_string(),
                target: Some(SpellTarget::Area {
                    center: WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("cast web");

    assert!(!events.iter().any(|event| {
        matches!(event, Event::SpellCastStubbed { spell_id, .. } if spell_id == "web_field")
    }));
    assert_eq!(engine.world().tile_effects.len(), 9);
    let applied_positions: Vec<Coord> = events
        .iter()
        .filter_map(|event| match event {
            Event::TileEffectApplied {
                effect_id,
                location,
                ..
            } if effect_id == "web_field" => Some(location.position),
            _ => None,
        })
        .collect();
    assert_eq!(
        applied_positions,
        vec![
            Coord { x: 1, y: 0 },
            Coord { x: 2, y: 0 },
            Coord { x: 3, y: 0 },
            Coord { x: 1, y: 1 },
            Coord { x: 2, y: 1 },
            Coord { x: 3, y: 1 },
            Coord { x: 1, y: 2 },
            Coord { x: 2, y: 2 },
            Coord { x: 3, y: 2 },
        ]
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Event::TileEffectApplied { effect_id, location, passability, sight, move_cost, .. }
            if effect_id == "web_field"
                && location == &WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 1 })
                && passability.as_deref() == Some("hindered")
                && sight.as_deref() == Some("obscured")
                && *move_cost == Some(2)
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::TileEffectApplied { effect_id, location, passability, .. }
            if effect_id == "web_field"
                && location == &WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 0 })
                && passability.as_deref() == Some("hindered")
    )));
}

#[test]
fn area_remove_overlay_can_clear_blocking_overlay_from_now_impassable_tile() {
    let mut engine = bt_spell_engine(&["stone_field", "clear_field"]);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "stone_field".to_string(),
                target: Some(SpellTarget::Area {
                    center: WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("cast stone field");
    assert!(engine.world().tile_effects.iter().any(|effect| {
        effect.effect_id == "stone_field" && effect.location.position == Coord { x: 2, y: 1 }
    }));

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "clear_field".to_string(),
                target: Some(SpellTarget::Area {
                    center: WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("clear stone field");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::TileEffectRemoved { effect_id, location, reason, .. }
            if effect_id == "stone_field"
                && location == &WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 1 })
                && reason == "clear_field"
    )));
    assert!(!engine.world().tile_effects.iter().any(|effect| {
        effect.effect_id == "stone_field" && effect.location.position == Coord { x: 2, y: 1 }
    }));
}

#[test]
fn remove_overlay_only_clears_matching_categories_at_a_cell() {
    let target = Coord { x: 2, y: 1 };
    let mut engine = bt_spell_engine(&[
        "stone_field",
        "dark_field",
        "clear_field",
        "sight_clear_field",
    ]);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "stone_field".to_string(),
                target: Some(SpellTarget::Area {
                    center: WorldPosition::new("realm_0", "room_0", target),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("cast stone field");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "dark_field".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "room_0", target),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("cast dark field");
    let current_time = engine.world().timing.now;
    engine
        .world_mut()
        .tile_effects
        .push(tme_rules::model::TileEffectState {
            source_actor_id: None,
            instance_id: "tile:manual:hazard".to_string(),
            effect_id: "ember_manual".to_string(),
            source: tme_rules::model::ActiveEffectSource {
                kind: "spell".to_string(),
                id: "ember_manual".to_string(),
            },
            location: WorldPosition::new("realm_0", "room_0", target),
            kind: "terrain_overlay".to_string(),
            tags: vec!["fire".to_string()],
            potency: 2,
            remaining_rounds: Some(10),
            passability: None,
            sight: None,
            hazard: Some("fire".to_string()),
            move_cost: None,
            tick_interval_rounds: 1,
            last_ticked_at: current_time,
            hostile_authority: None,
        });
    assert_eq!(
        engine
            .world()
            .tile_effects
            .iter()
            .filter(|effect| effect.location.position == target)
            .count(),
        3,
        "the target cell should have one passability, one sight, and one hazard overlay"
    );

    let passability_clear_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "clear_field".to_string(),
                target: Some(SpellTarget::Area {
                    center: WorldPosition::new("realm_0", "room_0", target),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("clear passability overlays");

    assert!(passability_clear_events.iter().any(|event| matches!(
        event,
        Event::TileEffectRemoved { effect_id, location, reason, .. }
            if effect_id == "stone_field" && location.position == target && reason == "clear_field"
    )));
    assert!(!passability_clear_events.iter().any(|event| matches!(
        event,
        Event::TileEffectRemoved { effect_id, location, .. }
            if (effect_id == "dark_field" || effect_id == "ember_manual")
                && location.position == target
    )));
    assert!(
        !engine.world().tile_effects.iter().any(|effect| {
            effect.effect_id == "stone_field" && effect.location.position == target
        })
    );
    assert!(
        engine.world().tile_effects.iter().any(|effect| {
            effect.effect_id == "dark_field" && effect.location.position == target
        })
    );
    assert!(engine.world().tile_effects.iter().any(|effect| {
        effect.effect_id == "ember_manual" && effect.location.position == target
    }));

    let sight_clear_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "sight_clear_field".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "room_0", target),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("clear sight overlays");

    assert!(sight_clear_events.iter().any(|event| matches!(
        event,
        Event::TileEffectRemoved { effect_id, location, reason, .. }
            if effect_id == "dark_field"
                && location.position == target
                && reason == "sight_clear_field"
    )));
    assert!(!sight_clear_events.iter().any(|event| matches!(
        event,
        Event::TileEffectRemoved { effect_id, location, .. }
            if effect_id == "ember_manual" && location.position == target
    )));
    assert!(
        !engine.world().tile_effects.iter().any(|effect| {
            effect.effect_id == "dark_field" && effect.location.position == target
        })
    );
    assert!(engine.world().tile_effects.iter().any(|effect| {
        effect.effect_id == "ember_manual" && effect.location.position == target
    }));
}

#[test]
fn remove_overlay_preserves_untargeted_descriptors_on_combined_effects() {
    let target = Coord { x: 2, y: 1 };
    let mut engine = bt_spell_engine(&["web_field", "clear_field"]);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "web_field".to_string(),
                target: Some(SpellTarget::Area {
                    center: WorldPosition::new("realm_0", "room_0", target),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("cast web field");
    assert!(engine.world().tile_effects.iter().any(|effect| {
        effect.effect_id == "web_field"
            && effect.location.position == target
            && effect.passability.as_deref() == Some("hindered")
            && effect.sight.as_deref() == Some("obscured")
    }));

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "clear_field".to_string(),
                target: Some(SpellTarget::Area {
                    center: WorldPosition::new("realm_0", "room_0", target),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("clear passability");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::TileEffectRemoved { effect_id, location, reason, .. }
            if effect_id == "web_field" && location.position == target && reason == "clear_field"
    )));
    assert!(engine.world().tile_effects.iter().any(|effect| {
        effect.effect_id == "web_field"
            && effect.location.position == target
            && effect.passability.is_none()
            && effect.move_cost.is_none()
            && effect.sight.as_deref() == Some("obscured")
    }));
}

#[test]
fn hazard_overlay_ticks_damage_and_expires() {
    let mut engine = bt_spell_engine_with_damage_interruption(&["ember_cloud"], true);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "ember_cloud".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("cast ember");
    let target_index = engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "target")
        .expect("target index");
    engine.world_mut().actors[target_index].warmed_spell = Some(WarmedSpellState {
        spell_id: "hazard_interrupted_spell".to_string(),
        warmed_at: LogicalTime::FIRST,
        ready_at: LogicalTime::new(99),
        status: WarmedSpellStatus::Warming,
    });
    let tick_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("tick");
    assert!(tick_events.iter().any(
        |event| matches!(event, Event::TileEffectTicked { effect_id, .. } if effect_id == "ember_cloud")
    ));
    assert!(tick_events.iter().any(|event| matches!(
        event,
        Event::TileEffectDamaged { effect_id, actor_id, damage: 2, .. }
            if effect_id == "ember_cloud" && actor_id == "target"
    )));
    let damaged = tick_events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::TileEffectDamaged { effect_id, actor_id, .. }
                    if effect_id == "ember_cloud" && actor_id == "target"
            )
        })
        .expect("tile damage event");
    let fizzled = tick_events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::SpellFizzled {
                    spell_id,
                    cause: SpellFizzleCause::Damage { .. },
                    ..
                } if spell_id == "hazard_interrupted_spell"
            )
        })
        .expect("tile damage fizzle");
    assert!(damaged < fizzled);
    assert!(engine.world().actors[target_index].warmed_spell.is_none());

    let expire_events = tick_events;
    assert!(expire_events.iter().any(
        |event| matches!(event, Event::TileEffectExpired { effect_id, .. } if effect_id == "ember_cloud")
    ));
}

#[test]
fn lethal_hazard_overlay_emits_damage_then_one_defeat_before_expiry() {
    let mut engine = bt_spell_engine(&["ember_cloud"]);
    let target_index = engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "target")
        .expect("target actor");
    engine.world_mut().actors[target_index].hp = 2;

    engine
        .apply_realtime_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "ember_cloud".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("cast ember");

    let tick_events = engine.advance_action_interval().expect("lethal tick");
    let ticked = tick_events
        .iter()
        .position(
            |event| matches!(event, Event::TileEffectTicked { effect_id, .. } if effect_id == "ember_cloud"),
        )
        .expect("tile tick");
    let damaged = tick_events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::TileEffectDamaged {
                    effect_id,
                    actor_id,
                    damage: 2,
                    hp: 0,
                    ..
                } if effect_id == "ember_cloud" && actor_id == "target"
            )
        })
        .expect("lethal tile damage");
    let died = tick_events
        .iter()
        .position(
            |event| matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == "target"),
        )
        .expect("target death");
    assert!(ticked < damaged);
    assert!(damaged < died);
    assert_eq!(
        tick_events
            .iter()
            .filter(|event| {
                matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == "target")
            })
            .count(),
        1
    );
    assert!(tick_events.iter().any(|event| matches!(
        event,
        Event::ActorDefeated {
            actor_id,
            cause: tme_rules::DeathCause::Fire,
            credited_actor_id: Some(credited_actor_id),
            ..
        } if actor_id == "target" && credited_actor_id == "player"
    )));
    let target = &engine.world().actors[target_index];
    assert_eq!(target.hp, 0);
    assert!(!target.is_alive());

    let expire_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("expire");
    let final_tick = expire_events
        .iter()
        .position(
            |event| matches!(event, Event::TileEffectTicked { effect_id, .. } if effect_id == "ember_cloud"),
        )
        .expect("final tile tick");
    let expired = expire_events
        .iter()
        .position(
            |event| matches!(event, Event::TileEffectExpired { effect_id, .. } if effect_id == "ember_cloud"),
        )
        .expect("tile expiry");
    assert!(final_tick < expired);
    assert!(!expire_events.iter().any(|event| {
        matches!(
            event,
            Event::TileEffectDamaged { actor_id, .. } | Event::ActorDefeated { actor_id, .. }
                if actor_id == "target"
        )
    }));
}
