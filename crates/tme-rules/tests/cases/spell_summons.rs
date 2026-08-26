use crate::spell_support::*;
use crate::support::content_parts::ContentParts;
use tme_rules::*;

fn layered_cells(rows: &[&str]) -> serde_json::Value {
    serde_json::json!(
        rows.iter()
            .map(|row| {
                row.chars()
                    .map(|glyph| {
                        vec![match glyph {
                            '.' => "flagstone",
                            '#' | 'x' => "stone_wall",
                            _ => panic!("unmapped fixture glyph {glyph:?}"),
                        }]
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    )
}

fn push_test_actor(parts: &mut ContentParts, mut actor: serde_json::Value) {
    let actor = actor.as_object_mut().expect("test actor object");
    let actor_id = actor["id"].as_str().expect("test actor id").to_string();
    let actor_definition_id = format!("actor/test/spell_summons/{actor_id}");
    let definition = serde_json::json!({
        "id": actor_definition_id,
        "kind": actor.remove("kind").expect("test actor kind"),
        "name": actor.remove("name").expect("test actor name"),
        "creature_traits": actor.remove("creature_traits").unwrap_or_else(|| serde_json::json!([])),
        "stats": actor.remove("stats").expect("test actor stats"),
        "magic_resistance": actor.remove("magic_resistance").expect("test actor magic resistance"),
        "death": actor.remove("death").expect("test actor death"),
        "social": actor.remove("social").expect("test actor social state"),
        "ai": actor.remove("ai").unwrap_or(serde_json::Value::Null),
        "xp_value": actor.remove("xp_value").unwrap_or(serde_json::Value::Null),
        "monster_abilities": actor.remove("monster_abilities").unwrap_or_else(|| serde_json::json!([])),
        "physical_damage_affinity_profile_id": actor
            .remove("physical_damage_affinity_profile_id")
            .unwrap_or_else(|| serde_json::json!("ordinary")),
    });
    actor.insert(
        "actor_definition_id".to_string(),
        serde_json::Value::String(actor_definition_id),
    );

    parts.push_selected(
        "actor_definitions",
        &format!("actor/test/spell_summons/{actor_id}"),
        definition,
    );
    parts
        .actors_mut()
        .as_array_mut()
        .expect("actors should be an array")
        .push(serde_json::Value::Object(actor.clone()));
}

fn bw_summon_engine(mutate: impl FnOnce(&mut ContentParts)) -> Engine {
    let mut parts = ContentParts::tracked(
        "summons_created_creature_lifecycle",
        "profile/summons_created_creature_lifecycle",
    );
    parts.profile_value_mut()["rules_profile"] = serde_json::json!("rules/first_room");
    parts.selected_mut("spells", 0)["target"]["range"] = serde_json::json!(3);
    parts.summon_actor_definition_mut(0)["stats"] =
        serde_json::json!({"hp": 6, "attack": 0, "defense": 1});
    parts.summon_actor_definition_mut(0)["ai"]["behavior"] = serde_json::json!("hold_ground");
    parts.summon_actor_definition_mut(0)["ai"]["leash_range"] = serde_json::json!(4);
    parts.selected_mut("summon_templates", 0)["item_instances"] = serde_json::json!({});

    parts.template_levels_source_mut()["holding"] = serde_json::json!({
        "law_zone": "none", "width": 3, "height": 3,
        "cells": layered_cells(&["###", "#.#", "###"])
    });
    parts.actor_definition_mut(0)["name"] = serde_json::json!("Wiz");
    parts.actors_mut()[0]["location"]["level"] = serde_json::json!("start");
    parts.actors_mut()[0]["character"]["resources"] = serde_json::json!({
        "hp": 10, "max_hp": 10, "peak_hp": 10,
        "mp": 20, "max_mp": 20, "stamina": 20, "max_stamina": 20
    });
    parts.actors_mut()[1]["id"] = serde_json::json!("sentinel");
    parts.actor_definition_mut(1)["name"] = serde_json::json!("Sentinel");
    parts.actors_mut()[1]["location"]["level"] = serde_json::json!("holding");
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    parts.actor_definition_mut(1)["stats"] =
        serde_json::json!({"hp": 4, "attack": 0, "defense": 0});
    mutate(&mut parts);
    parts.engine(7).expect("summon engine should start")
}

#[test]
fn summon_spell_spawns_template_actor_without_stub_and_awards_magic_practice() {
    let mut engine = bw_summon_engine(|_| {});

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("summon cast should succeed");

    assert!(events.events.iter().any(|event| matches!(
        event,
        Event::ActorSummoned {
            spell_id,
            actor_id,
            template_id,
            owner_id,
            social,
            location,
            remaining_rounds,
            ..
        } if spell_id == "call_echo"
            && actor_id == "summon:call_echo:1:echo_guardian"
            && template_id == "echo_guardian"
            && owner_id == "player"
            && social.alignment_source
                == SocialAlignmentSource::Inherent {
                    alignment: CharacterAlignment::Lawful,
                }
            && social.nature == SocialNature::Other
            && social.behavior == SocialBehavior::AlignmentCreature
            && social.owner_relation == SocialOwnerRelation::Summoner
            && location.level == "start"
            && location.position == Coord { x: 2, y: 1 }
            && *remaining_rounds == Some(2)
    )));
    assert!(!events.events.iter().any(
        |event| matches!(event, Event::SpellCastStubbed { spell_id, .. } if spell_id == "call_echo")
    ));
    assert_eq!(count_skill_practice(&events.events, "wizard_magic"), 1);

    let snapshot = engine.snapshot();
    let summoned = snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "summon:call_echo:1:echo_guardian")
        .expect("summoned actor visible in snapshot");
    assert_eq!(summoned.name, "Echo Guardian");
}

#[test]
fn summon_spell_rejects_occupied_target_without_spending_resources() {
    let mut engine = bw_summon_engine(|parts| {
        push_test_actor(
            parts,
            serde_json::json!({
                "id": "blocker",
                "kind": "monster", "npc": null, "social": {"alignment_source":{"kind":"inherent","alignment":"chaotic"},"nature":"other","behavior":"alignment_creature","owner_relation":"none"},
                "magic_resistance": {"natural_save_twentieths": 5, "evidence_state": "original_provisional"},
                "death": {"remains": "searchable_corpse"},
                "name": "Blocker",
                "location": {"realm": "realm_0", "level": "start", "position": {"x": 2, "y": 1}},
                "stats": {"hp": 4, "attack": 0, "defense": 0},
                "ai": {"behavior": "hold_ground", "cadence_units": 1, "aggro_radius": 7, "leash_range": 12, "awareness": {"mode": "unrestricted"}, "physical_attack_modes": ["fight"]},
                "carried": {"items": [], "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}}
            }),
        );
    });
    let player_before = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    let (mp_before, stamina_before) = (player_before.mp, player_before.stamina);

    let err = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("occupied summon target should be rejected");

    assert_eq!(err.message(), "invalid_target");
    let player_after = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    assert_eq!(player_after.mp, mp_before);
    assert_eq!(player_after.stamina, stamina_before);
    assert_eq!(engine.world().timing.now, tme_rules::LogicalTime::FIRST);
}

#[test]
fn summon_spell_rejects_impassable_and_out_of_bounds_targets_without_spending_resources() {
    for target in [
        WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
        WorldPosition::new("realm_0", "start", Coord { x: 99, y: 99 }),
    ] {
        let mut engine = bw_summon_engine(|parts| {
            parts.template_levels_source_mut()["start"]["cells"] =
                layered_cells(&["#####", "#.x.#", "#####"]);
        });
        let player_before = engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player");
        let (mp_before, stamina_before) = (player_before.mp, player_before.stamina);

        let err = engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::CastSpell {
                    spell_id: "call_echo".to_string(),
                    target: Some(SpellTarget::Coordinate {
                        position: target.clone(),
                    }),
                    authorization: tme_rules::HostilityAuthorization::Safe,
                },
            )
            .expect_err("invalid summon coordinate should be rejected");

        assert_eq!(err.message(), "invalid_target");
        let player_after = engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player");
        assert_eq!(player_after.mp, mp_before);
        assert_eq!(player_after.stamina, stamina_before);
        assert_eq!(engine.world().timing.now, tme_rules::LogicalTime::FIRST);
    }
}

#[test]
fn summon_spell_expires_after_duration_ticks_and_leaves_snapshot() {
    let mut engine = bw_summon_engine(|_| {});

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("summon cast should succeed");

    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("first waiting round should keep summon alive");
    assert!(
        engine
            .snapshot()
            .actors
            .iter()
            .any(|actor| actor.id == "summon:call_echo:1:echo_guardian")
    );

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("second waiting round should expire summon");

    assert!(events.events.iter().any(|event| matches!(
        event,
        Event::SummonExpired {
            actor_id,
            actor,
            instance_id,
            owner_id,
            source_spell_id,
            template_id,
            location,
        } if actor_id == "summon:call_echo:1:echo_guardian"
            && actor == "Echo Guardian"
            && instance_id == "summon:call_echo:1:echo_guardian"
            && owner_id == "player"
            && source_spell_id == "call_echo"
            && template_id == "echo_guardian"
            && location.level == "start"
            && location.position == Coord { x: 2, y: 1 }
    )));
    assert!(
        !engine
            .snapshot()
            .actors
            .iter()
            .any(|actor| actor.id == "summon:call_echo:1:echo_guardian")
    );
}

#[test]
fn summon_spell_uses_deterministic_incrementing_instance_ids() {
    let mut engine = bw_summon_engine(|parts| {
        parts.selected_mut("spells", 0)["effect"]["duration"]["rounds"] = serde_json::json!(3);
        parts.push_selected(
            "items",
            "item/echo_focus/summon_test",
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
              "capability": {
                "taxonomy_id": "echo_focus"
              },
              "valid_placements": [
                "hand",
                "belt_side",
                "belt_back",
                "sack"
              ],
              "economy": {
                "unit_burden": 1
              }
            }),
        );
        parts.selected_mut("summon_templates", 0)["item_instances"] = serde_json::json!({
            "focus": {
                "definition_id": "echo_focus",
                "binding": {"state": "unrestricted"}
            }
        });
        parts.selected_mut("summon_templates", 0)["carried"] = serde_json::json!({
            "items": [{"item_instance_id": "focus", "position": "right_hand"}],
            "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}
        });
    });

    let first_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("first summon cast should succeed");
    let second_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 3, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("second summon cast should succeed");

    assert!(first_events.events.iter().any(|event| matches!(
        event,
        Event::ActorSummoned { actor_id, .. } if actor_id == "summon:call_echo:1:echo_guardian"
    )));
    assert!(second_events.events.iter().any(|event| matches!(
        event,
        Event::ActorSummoned { actor_id, .. } if actor_id == "summon:call_echo:2:echo_guardian"
    )));
    assert!(
        engine
            .world()
            .item_instances
            .contains_key("summon:call_echo:1:echo_guardian:item:focus")
    );
    assert!(
        engine
            .world()
            .item_instances
            .contains_key("summon:call_echo:2:echo_guardian:item:focus")
    );
}

#[test]
fn player_owned_chaotic_summon_targets_neutral_monster_not_owner() {
    let mut engine = bw_summon_engine(|parts| {
        push_test_actor(
            parts,
            serde_json::json!({
                "id": "raider",
                "kind": "monster", "npc": null, "social": {"alignment_source":{"kind":"inherent","alignment":"neutral"},"nature":"other","behavior":"passive","owner_relation":"none"},
                "magic_resistance": {"natural_save_twentieths": 5, "evidence_state": "original_provisional"},
                "death": {"remains": "searchable_corpse"},
                "name": "Raider",
                "location": {"realm": "realm_0", "level": "start", "position": {"x": 3, "y": 1}},
                "stats": {"hp": 4, "attack": 0, "defense": 0},
                "ai": {"behavior": "hold_ground", "cadence_units": 1, "aggro_radius": 7, "leash_range": 12, "awareness": {"mode": "unrestricted"}, "physical_attack_modes": ["fight"]},
                "carried": {"items": [], "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}}
            }),
        );
        parts.summon_actor_definition_mut(0)["social"]["alignment_source"] =
            serde_json::json!({"kind": "inherent", "alignment": "chaotic"});
        parts.summon_actor_definition_mut(0)["ai"]["behavior"] = serde_json::json!("simple_chase");
        parts.summon_actor_definition_mut(0)["stats"]["attack"] = serde_json::json!(1);
    });

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("summon cast should succeed");

    assert!(events.events.iter().any(|event| matches!(
        event,
        Event::AutomaticActorDecision {
            actor_id,
            decision: AutomaticActorDecisionV1::Move {
                direction: Direction::East,
                purpose: AutomaticMovementPurposeV1::Chase,
            },
            ..
        } if actor_id == "summon:call_echo:1:echo_guardian"
    )));
    assert!(!events.events.iter().any(|event| matches!(
        event,
        Event::AutomaticActorDecision {
            actor_id,
            decision: AutomaticActorDecisionV1::PhysicalAttack { target, .. },
            ..
        } if actor_id == "summon:call_echo:1:echo_guardian" && target == "Wiz"
    )));
}

#[test]
fn player_owned_chaotic_summon_never_targets_owner_or_same_alignment() {
    let mut engine = bw_summon_engine(|parts| {
        push_test_actor(
            parts,
            serde_json::json!({
                "id": "raider",
                "kind": "monster", "npc": null, "social": {"alignment_source":{"kind":"inherent","alignment":"chaotic"},"nature":"other","behavior":"alignment_creature","owner_relation":"none"},
                "magic_resistance": {"natural_save_twentieths": 5, "evidence_state": "original_provisional"},
                "death": {"remains": "searchable_corpse"},
                "name": "Raider",
                "location": {"realm": "realm_0", "level": "start", "position": {"x": 3, "y": 1}},
                "stats": {"hp": 4, "attack": 0, "defense": 0},
                "ai": {"behavior": "hold_ground", "cadence_units": 1, "aggro_radius": 7, "leash_range": 12, "awareness": {"mode": "unrestricted"}, "physical_attack_modes": ["fight"]},
                "carried": {"items": [], "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}}
            }),
        );
        parts.summon_actor_definition_mut(0)["social"]["alignment_source"] =
            serde_json::json!({"kind": "inherent", "alignment": "chaotic"});
        parts.summon_actor_definition_mut(0)["ai"]["behavior"] = serde_json::json!("simple_chase");
    });

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("summon cast should succeed");

    assert!(events.events.iter().any(|event| matches!(
        event,
        Event::AutomaticActorDecision {
            actor_id,
            decision: AutomaticActorDecisionV1::Wait {
                reason: tme_rules::AutomaticWaitReasonV1::Watch,
            },
            ..
        } if actor_id == "summon:call_echo:1:echo_guardian"
    )));
    assert!(!events.events.iter().any(|event| matches!(
        event,
        Event::AutomaticActorDecision {
            actor_id,
            decision: AutomaticActorDecisionV1::PhysicalAttack { target, .. },
            ..
        } if actor_id == "summon:call_echo:1:echo_guardian"
            && matches!(target.as_str(), "Raider" | "Wiz")
    )));
}

#[test]
fn summon_dead_later_actor_gets_no_turn() {
    let mut engine = bw_summon_engine(|parts| {
        parts.selected_mut("spells", 0)["effect"]["duration"]["rounds"] = serde_json::json!(4);
        parts.template_levels_source_mut()["start"]["width"] = serde_json::json!(6);
        parts.template_levels_source_mut()["start"]["height"] = serde_json::json!(4);
        parts.template_levels_source_mut()["start"]["cells"] =
            layered_cells(&["######", "#....#", "#....#", "######"]);
        parts.push_selected(
            "items",
            "item/echo_bolt/summon_test",
            serde_json::json!({
              "id": "echo_bolt",
              "kind": "weapon",
              "name": "Echo Bolt",
              "weapon": {
                "skill_track_id": "staff",
                "default_attack_mode": "throw",
                "attack_modes": [{"mode": "throw", "maximum_range": 2, "damage_kind": "piercing"}],
                "cooldown_units": 1,
                "combat_add_rating": 0,
                "handedness": "one_handed",
                "block_value": 0
              },
              "valid_placements": [
                "hand",
                "belt_side",
                "belt_back",
                "sack"
              ],
              "economy": {
                "unit_burden": 1
              }
            }),
        );
        parts.summon_actor_definition_mut(0)["ai"]["behavior"] = serde_json::json!("simple_chase");
        parts.summon_actor_definition_mut(0)["social"]["alignment_source"] =
            serde_json::json!({"kind": "inherent", "alignment": "evil"});
        parts.summon_actor_definition_mut(0)["ai"]["physical_attack_modes"] =
            serde_json::json!(["throw"]);
        parts.selected_mut("summon_templates", 0)["item_instances"] = serde_json::json!({
            "bolt": {
                "definition_id": "echo_bolt",
                "binding": {"state": "unrestricted"}
            }
        });
        parts.selected_mut("summon_templates", 0)["carried"] = serde_json::json!({
            "items": [{"item_instance_id": "bolt", "position": "right_hand"}],
            "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}
        });
        parts.summon_actor_definition_mut(0)["stats"]["attack"] = serde_json::json!(30);
        parts.push_selected(
            "actor_definitions",
            "actor/echo_invader/summon_test",
            serde_json::json!({
                "id": "actor/summon/echo_invader",
                "name": "Echo Invader",
                "kind": "monster", "social": {"alignment_source":{"kind":"inherent","alignment":"chaotic"},"nature":"other","behavior":"alignment_creature","owner_relation":"summoner"},
                "magic_resistance": {"natural_save_twentieths": 5, "evidence_state": "original_provisional"},
                "death": {"remains": "none"},
                "creature_traits": [],
                "stats": {"hp": 1, "attack": 0, "defense": 0},
                "ai": {"behavior": "simple_chase", "cadence_units": 1, "aggro_radius": 4, "leash_range": 4, "awareness": {"mode": "unrestricted"}, "physical_attack_modes": ["fight"]},
                "xp_value": 0,
                "monster_abilities": [],
                "physical_damage_affinity_profile_id": "ordinary"
            }),
        );
        parts.push_selected(
            "summon_templates",
            "summon/echo_invader/summon_test",
            serde_json::json!({
                "id": "echo_invader",
                "actor_definition_id": "actor/summon/echo_invader",
                "item_instances": {},
                "carried": {"items": [], "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}},
                "active_effects": []
            }),
        );
        parts.push_selected(
            "spells",
            "spell/call_invader/summon_test",
            serde_json::json!({
                "id": "call_invader",
                "name": "Call Invader",
                "status": "draft",
                "lane": "wizard_magic",
                "skill_requirement": 1,
                "mp_cost": 3,
                "stamina_cost": 1,
                "social": {"hostile_act": false, "town_law": "permitted"},
                "effect": {
                    "family": "summon",
                    "summon_actor_id": "echo_invader",
                    "duration": {"policy": "rounds", "rounds": 2}
                },
                "target": {
                    "kind": "coordinate",
                    "range": 3,
                    "requires_visible": false
                },
                "casting": {"method": "direct", "cast_class": "not_applicable"}
            }),
        );
        parts.actors_mut()[0]["character"]["known_spells"]
            .as_array_mut()
            .expect("known spells should be an array")
            .push(serde_json::json!({
                "spell_id": "call_invader",
                "lane": "wizard_magic",
                "learned_at_level": 1
            }));
    });

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("ally summon should succeed");

    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("ally should ready");

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_invader".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 4, y: 2 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("hostile summon should succeed");

    let hostile_id = "summon:call_invader:2:echo_invader";
    let death_index = events
        .events
        .iter()
        .position(|event| matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == hostile_id))
        .expect("hostile summon should die during automatic actor scheduling");

    assert!(
        !events
            .events
            .iter()
            .skip(death_index + 1)
            .any(|event| matches!(
                event,
                Event::ActorReady { actor_id, .. } if actor_id == hostile_id
            ))
    );
    assert!(
        !events
            .events
            .iter()
            .skip(death_index + 1)
            .any(|event| matches!(
                event,
                Event::AutomaticActorDecision { actor_id, .. } if actor_id == hostile_id
            ))
    );
    assert!(!events.events.iter().skip(death_index + 1).any(|event| matches!(
        event,
        Event::Moved { actor_id, actor, .. } if actor_id == hostile_id || actor == "Echo Invader"
    )));
    assert!(!events.events.iter().skip(death_index + 1).any(|event| matches!(
        event,
        Event::Attacked { attacker_id, attacker, .. } if attacker_id == hostile_id || attacker == "Echo Invader"
    )));
}

#[test]
fn player_cannot_attack_owned_hostile_summon_or_gain_xp() {
    let mut engine = bw_summon_engine(|parts| {
        parts.actor_definition_mut(0)["stats"]["attack"] = serde_json::json!(10);
        parts.summon_actor_definition_mut(0)["social"]["alignment_source"] =
            serde_json::json!({"kind": "inherent", "alignment": "chaotic"});
        parts.summon_actor_definition_mut(0)["ai"]["behavior"] = serde_json::json!("simple_chase");
        parts.summon_actor_definition_mut(0)["xp_value"] = serde_json::json!(9);
        parts.summon_actor_definition_mut(0)["stats"]["hp"] = serde_json::json!(1);
        parts.summon_actor_definition_mut(0)["stats"]["attack"] = serde_json::json!(0);
        parts.summon_actor_definition_mut(0)["stats"]["defense"] = serde_json::json!(0);
    });

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", Coord { x: 2, y: 1 }),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("summon cast should succeed");

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("player should share the summon hex before attacking");

    let before_attack = engine.world().clone();
    let attack_error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "summon:call_echo:1:echo_guardian".into(),
            },
        )
        .expect_err("player attack against an owned summon must be rejected");
    assert!(attack_error.message().contains("invalid_hostile_target"));
    assert_eq!(engine.world(), &before_attack);

    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    let character = player.character.as_ref().expect("player character");
    assert_eq!(character.progression.experience, 0);
}
