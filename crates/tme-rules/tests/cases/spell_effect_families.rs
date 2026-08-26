use std::collections::BTreeMap;

use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};
use tme_rules::model::{
    ActiveEffectSource, ActiveEffectStackingPolicy, ActiveEffectState, TileEffectState,
};
use tme_rules::{
    ActorKind, ActorLifeState, AutomaticActorDecisionV1, AutomaticMovementPurposeV1,
    BanishResultReasonV1, CharacterAlignment, Coord, CorpseId, CorpseState, CreatureTrait,
    DeathCause, Direction, Engine, Event, LogicalTime, PlayerIntent, RaiseDeadResultReasonV1,
    SocialAlignmentSource, SocialBehavior, SocialNature, SpellTarget,
    TransitionConcealmentRemovalReasonV1, WorldPosition,
};

fn layered_cells(rows: &[&str]) -> Value {
    json!(
        rows.iter()
            .map(|row| {
                row.chars()
                    .map(|glyph| {
                        vec![match glyph {
                            '#' => "stone_wall",
                            '.' | 'D' => "flagstone",
                            _ => panic!("unmapped fixture glyph {glyph:?}"),
                        }]
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    )
}

fn spell(id: &str, lane: &str, effect: Value, target: Value, cast_class: &str) -> Value {
    let family = effect["family"].as_str().expect("spell effect family");
    let target_kind = target["kind"].as_str().expect("spell target kind");
    let hostile_act = matches!(
        family,
        "banish" | "curse" | "direct_damage" | "instant_death" | "poison" | "turn_undead"
    ) || (family == "control_status" && target_kind == "actor");
    json!({
        "id": id,
        "name": id.replace('_', " "),
        "status": "draft",
        "lane": lane,
        "skill_requirement": 1,
        "mp_cost": 1,
        "social": {"hostile_act": hostile_act, "town_law": "permitted"},
        "effect": effect,
        "target": target,
        "casting": {"method": "direct", "cast_class": cast_class}
    })
}

fn family_engine(
    class_id: &str,
    lane: &str,
    spells: Vec<Value>,
    seed: u64,
    mutate: impl FnOnce(&mut ContentParts),
) -> Engine {
    let mut parts = ContentParts::tracked("spell_effects", "profile/spell_effects");
    parts.profile_value_mut()["rules_profile"] = json!("rules/first_room");
    parts.profile_value_mut()["spells"] = json!([]);
    for spell in &spells {
        let id = spell["id"].as_str().expect("spell id");
        parts.push_selected(
            "spells",
            &format!("spell/{id}/effect_family_test"),
            spell.clone(),
        );
    }
    parts.rules_source_mut()["resources"]["recovery_interval_units"] = json!(1_000);

    let player = &mut parts.actors_mut()[0];
    player["character"]["identity"]["base_class_id"] = json!(class_id);
    player["character"]["identity"]["current_class_id"] = json!(class_id);
    player["character"]["identity"]["display_class"] = json!(class_id.replace('_', " "));
    player["character"]["resources"]["mp"] = json!(100);
    player["character"]["resources"]["max_mp"] = json!(100);
    player["character"]["skill_ledger"] = json!([{
        "track_id": lane,
        "level": 5,
        "critique_rank": 0,
        "practice_points": 0,
        "learning_rate": 1
    }]);
    player["character"]["known_spells"] = Value::Array(
        spells
            .iter()
            .map(|spell| {
                json!({
                    "spell_id": spell["id"].as_str().expect("spell id"),
                    "lane": lane,
                    "learned_at_level": 1
                })
            })
            .collect(),
    );
    mutate(&mut parts);
    parts.engine(seed).expect("DY test engine starts")
}

fn cast(engine: &mut Engine, spell_id: &str, target: Option<SpellTarget>) -> Vec<Event> {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: spell_id.to_string(),
                target,
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .unwrap_or_else(|error| panic!("{spell_id} should cast: {error}"))
        .events
}

fn active_spell(id: &str, lane: &str, family: &str, status_kind: &str, target_kind: &str) -> Value {
    let cast_class = if target_kind == "self" {
        "self"
    } else {
        "character"
    };
    spell(
        id,
        lane,
        json!({
            "family": family,
            "status_kind": status_kind,
            "stacking": "refresh_duration",
            "duration": {"policy": "rounds", "rounds": 20}
        }),
        json!({"kind": target_kind}),
        cast_class,
    )
}

#[test]
fn typed_capabilities_apply_without_stub_or_invented_consumers() {
    let lane = "wizard_magic";
    let spells = vec![
        active_spell(
            "feather_fall",
            lane,
            "fall_protection",
            "fall_protection",
            "self",
        ),
        active_spell("haste", lane, "speed", "speed", "self"),
        active_spell("night_sight", lane, "vision", "night_vision", "self"),
        active_spell(
            "breathe_water",
            lane,
            "water_breathing",
            "water_breathing",
            "actor",
        ),
    ];
    let mut engine = family_engine("wizard", lane, spells, 7, |parts| {
        parts.actor_definition_mut(1)["social"]["alignment_source"] =
            json!({"kind": "inherent", "alignment": "neutral"});
        parts.actor_definition_mut(1)["social"]["behavior"] = json!("passive");
    });

    for (spell_id, target) in [
        ("feather_fall", SpellTarget::SelfTarget),
        ("haste", SpellTarget::SelfTarget),
        ("night_sight", SpellTarget::SelfTarget),
        (
            "breathe_water",
            SpellTarget::Actor {
                actor_id: "player".into(),
            },
        ),
    ] {
        let events = cast(&mut engine, spell_id, Some(target));
        assert!(events.iter().any(|event| matches!(
            event,
            Event::EffectApplied { effect_id, .. } if effect_id == spell_id
        )));
        assert!(!events.iter().any(|event| matches!(
            event,
            Event::SpellCastStubbed { spell_id: stubbed, .. } if stubbed == spell_id
        )));
    }

    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    for tag in [
        "fall_protection",
        "speed",
        "night_vision",
        "water_breathing",
    ] {
        assert!(
            player
                .active_effects
                .iter()
                .any(|effect| { effect.tags.iter().any(|candidate| candidate == tag) })
        );
    }
    assert_eq!(
        engine
            .definition()
            .catalog()
            .rules()
            .movement
            .controlled_path_points,
        3
    );
    assert_eq!(
        engine
            .definition()
            .catalog()
            .rules()
            .movement
            .automatic_step_points,
        1
    );
}

fn darkness_effect(engine: &Engine, tag: &str) -> TileEffectState {
    TileEffectState {
        instance_id: format!("sight:{tag}"),
        effect_id: tag.to_string(),
        source: ActiveEffectSource {
            kind: "spell".to_string(),
            id: tag.to_string(),
        },
        source_actor_id: None,
        location: WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 1 }),
        kind: "darkness".to_string(),
        tags: vec![tag.to_string()],
        potency: 0,
        remaining_rounds: None,
        passability: None,
        sight: Some("blocked".to_string()),
        hazard: None,
        move_cost: None,
        tick_interval_rounds: 1,
        last_ticked_at: engine.world().timing.now,
        hostile_authority: None,
    }
}

#[test]
fn night_vision_bypasses_only_darkness_for_actor_addressed_sight() {
    let lane = "wizard_magic";
    let night = active_spell("night_sight", lane, "vision", "night_vision", "self");
    let mut engine = family_engine("wizard", lane, vec![night], 7, |parts| {
        parts.actor_definition_mut(1)["social"]["alignment_source"] =
            json!({"kind": "inherent", "alignment": "neutral"});
        parts.actor_definition_mut(1)["social"]["behavior"] = json!("passive");
    });
    let from = WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 });
    let to = WorldPosition::new("realm_0", "room_0", Coord { x: 3, y: 1 });
    let darkness = darkness_effect(&engine, "darkness");
    engine.world_mut().tile_effects.push(darkness);
    assert!(!engine.has_line_of_sight(&from, &to));
    assert!(
        !engine
            .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
            .expect("snapshot")
            .actors
            .iter()
            .any(|actor| actor.id == "target")
    );

    cast(&mut engine, "night_sight", Some(SpellTarget::SelfTarget));
    assert!(!engine.has_line_of_sight(&from, &to));
    assert!(
        engine
            .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
            .expect("night snapshot")
            .actors
            .iter()
            .any(|actor| actor.id == "target")
    );

    engine.world_mut().tile_effects.clear();
    let smoke = darkness_effect(&engine, "smoke");
    engine.world_mut().tile_effects.push(smoke);
    assert!(
        !engine
            .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
            .expect("smoke snapshot")
            .actors
            .iter()
            .any(|actor| actor.id == "target")
    );

    engine.world_mut().tile_effects.clear();
    let darkness = darkness_effect(&engine, "darkness");
    engine.world_mut().tile_effects.push(darkness);
    let now = engine.world().timing.now;
    engine.world_mut().actors[0]
        .active_effects
        .push(ActiveEffectState {
            instance_id: "blind:test".to_string(),
            effect_id: "blind_test".to_string(),
            source: ActiveEffectSource {
                kind: "fixture".to_string(),
                id: "blind_test".to_string(),
            },
            source_actor_id: None,
            hostile_authority: None,
            spell_damage_credit: None,
            kind: "blind".to_string(),
            tags: vec!["blind".to_string()],
            potency: 0,
            remaining_rounds: None,
            until_condition: None,
            stacking: ActiveEffectStackingPolicy::ReplaceSameKind,
            start_delay_rounds: 0,
            tick_interval_rounds: 1,
            suppresses_action: false,
            resistance_boosts: vec![],
            last_ticked_at: now,
        });
    let blind_snapshot = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("blind snapshot");
    assert_eq!(blind_snapshot.actors.len(), 1);
    assert_eq!(blind_snapshot.actors[0].id, "player");
}

fn thief_concealment_engine() -> Engine {
    let lane = "thief_magic";
    let hide = active_spell("hide_in_shadows", lane, "concealment", "hidden", "self");
    family_engine("thief", lane, vec![hide], 7, |parts| {
        parts.template_levels_source_mut()["room_0"]["width"] = json!(7);
        parts.template_levels_source_mut()["room_0"]["height"] = json!(5);
        parts.template_levels_source_mut()["room_0"]["cells"] =
            layered_cells(&["#######", "#.....#", "#.....#", "#.....#", "#######"]);
        parts.actors_mut()[0]["location"]["position"] = json!({"x": 1, "y": 2});
        parts.actors_mut()[1]["location"]["position"] = json!({"x": 5, "y": 2});
        parts.actor_definition_mut(1)["social"]["alignment_source"] =
            json!({"kind": "inherent", "alignment": "neutral"});
        parts.actor_definition_mut(1)["social"]["behavior"] = json!("passive");
        parts.profile_value_mut()["profession_actions"] =
            json!(["profession/thief_hide/magic_profession_gallery"]);
    })
}

#[test]
fn spell_concealment_breaks_on_uncovered_move_and_damage_but_not_cast_itself() {
    let mut engine = thief_concealment_engine();
    let events = cast(
        &mut engine,
        "hide_in_shadows",
        Some(SpellTarget::SelfTarget),
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ActorHidden { effect_id, .. } if effect_id == "hide_in_shadows"
    )));
    assert!(
        engine.world().actors[0]
            .active_effects
            .iter()
            .any(|effect| effect.source.kind == "spell"
                && effect.tags.contains(&"hidden".to_string()))
    );

    let moved = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("uncovered move");
    assert!(moved.events.iter().any(|event| matches!(
        event,
        Event::HideBroken { reason, .. } if reason == "move"
    )));

    let mut engine = thief_concealment_engine();
    cast(
        &mut engine,
        "hide_in_shadows",
        Some(SpellTarget::SelfTarget),
    );
    let now = engine.world().timing.now.saturating_add_rounds(0);
    engine.world_mut().actors[0]
        .active_effects
        .push(ActiveEffectState {
            instance_id: "poison:hit".to_string(),
            effect_id: "poison_hit".to_string(),
            source: ActiveEffectSource {
                kind: "fixture".to_string(),
                id: "poison_hit".to_string(),
            },
            source_actor_id: None,
            hostile_authority: None,
            spell_damage_credit: None,
            kind: "poison".to_string(),
            tags: vec!["poison".to_string()],
            potency: 1,
            remaining_rounds: Some(2),
            until_condition: None,
            stacking: ActiveEffectStackingPolicy::StackInstance,
            start_delay_rounds: 0,
            tick_interval_rounds: 1,
            suppresses_action: false,
            resistance_boosts: vec![],
            last_ticked_at: LogicalTime::new(now.value().saturating_sub(1)),
        });
    let hit = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("poison boundary");
    assert!(hit.events.iter().any(|event| matches!(
        event,
        Event::HideBroken { reason, .. } if reason == "hit"
    )));
    assert!(
        !engine.world().actors[0]
            .active_effects
            .iter()
            .any(|effect| effect.source.kind == "spell"
                && effect.tags.contains(&"hidden".to_string()))
    );
}

fn door_engine(duration: u32) -> Engine {
    let lane = "wizard_magic";
    let hide = spell(
        "hide_door",
        lane,
        json!({
            "family": "concealment",
            "duration": {"policy": "rounds", "rounds": duration},
            "door_control": {"action": "hide_secret"}
        }),
        json!({"kind": "door"}),
        "not_applicable",
    );
    let reveal = spell(
        "sense_secret",
        lane,
        json!({"family": "secret_detection", "potency": 1}),
        json!({"kind": "none", "range": 2}),
        "not_applicable",
    );
    family_engine("wizard", lane, vec![hide, reveal], 7, |parts| {
        parts.template_levels_source_mut()["room_0"] = json!({
            "law_zone": "none",
            "width": 5,
            "height": 3,
            "cells": layered_cells(&["#####", "#.D.#", "#####"])
        });
        parts.template_levels_source_mut()["vault"] = json!({
            "law_zone": "none",
            "width": 3, "height": 3, "cells": layered_cells(&["###", "#.#", "###"])
        });
        parts.world_template["topology"] = json!({
            "edge/room_0/1/2": {
                "at": {"realm": "realm_0", "level": "room_0", "position": {"x": 2, "y": 1}},
                "target": {"kind": "position", "location": {
                    "realm": "realm_0", "level": "vault", "position": {"x": 1, "y": 1}
                }},
                "kind": {"kind": "door", "initial_state": "closed"},
                "hidden": false
            }
        });
        parts.actors_mut()[0]["location"]["position"] = json!({"x": 1, "y": 1});
        parts.actors_mut()[1]["location"]["position"] = json!({"x": 3, "y": 1});
        parts.actor_definition_mut(1)["social"]["alignment_source"] =
            json!({"kind": "inherent", "alignment": "neutral"});
        parts.actor_definition_mut(1)["social"]["behavior"] = json!("passive");
    })
}

#[test]
fn door_concealment_hides_observed_state_and_removes_on_reveal_open_and_expiry() {
    let door = Coord { x: 2, y: 1 };
    let mut engine = door_engine(3);
    let concealed = cast(
        &mut engine,
        "hide_door",
        Some(SpellTarget::Door {
            direction: Direction::East,
        }),
    );
    assert!(concealed.iter().any(|event| matches!(
        event,
        Event::TransitionConcealed { location, .. } if location.position == door
    )));
    let snapshot = engine.snapshot();
    assert_eq!(snapshot.concealed_transitions.len(), 1);
    let concealment = &snapshot.concealed_transitions[0];
    assert_eq!(
        serde_json::to_value(concealment).expect("concealment view serializes"),
        json!({
            "instance_id": concealment.instance_id,
            "source_spell_id": "hide_door",
            "source_actor_id": "player",
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 2, "y": 1}
            },
            "remaining_rounds": 3,
            "last_ticked_at": concealment.last_ticked_at
        })
    );
    let observed = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed");
    assert!(
        observed.realms[0].levels[0]
            .tiles
            .iter()
            .find(|tile| tile.position == door)
            .is_some_and(|tile| tile.transition.is_none())
    );
    let observed_json = serde_json::to_string(&observed).expect("observed snapshot serializes");
    assert!(!observed_json.contains("concealed_transitions"));
    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    assert!(context.door_actions.is_empty());
    let east_exit = context
        .exits
        .iter()
        .find(|exit| exit.direction == Direction::East)
        .expect("east exit");
    assert!(east_exit.transition.is_none());
    assert!(
        !serde_json::to_string(east_exit)
            .expect("east exit serializes")
            .contains("target_room")
    );

    let revealed = cast(&mut engine, "sense_secret", None);
    assert!(revealed.iter().any(|event| matches!(
        event,
        Event::TransitionConcealmentRemoved {
            reason: TransitionConcealmentRemovalReasonV1::Revealed,
            ..
        }
    )));
    assert!(engine.world().concealed_transitions.is_empty());

    cast(
        &mut engine,
        "hide_door",
        Some(SpellTarget::Door {
            direction: Direction::East,
        }),
    );
    let opened = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Open(Direction::East),
        )
        .expect("underlying door opens explicitly");
    let removed = opened
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::TransitionConcealmentRemoved {
                    reason: TransitionConcealmentRemovalReasonV1::Opened,
                    ..
                }
            )
        })
        .expect("open removal");
    let door_opened = opened
        .events
        .iter()
        .position(|event| matches!(event, Event::DoorOpened { .. }))
        .expect("door opened");
    assert!(removed < door_opened);

    let mut engine = door_engine(1);
    cast(
        &mut engine,
        "hide_door",
        Some(SpellTarget::Door {
            direction: Direction::East,
        }),
    );
    let expired = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("expiry wait");
    assert!(expired.events.iter().any(|event| matches!(
        event,
        Event::TransitionConcealmentRemoved {
            reason: TransitionConcealmentRemovalReasonV1::Expired,
            ..
        }
    )));
}

fn banish_engine() -> Engine {
    let lane = "thaumaturge_magic";
    let summon = spell(
        "call_demon",
        lane,
        json!({
            "family": "summon",
            "summon_actor_id": "bound_demon",
            "duration": {"policy": "rounds", "rounds": 10}
        }),
        json!({"kind": "coordinate", "range": 3}),
        "not_applicable",
    );
    let banish = spell(
        "banish",
        lane,
        json!({
            "family": "banish",
            "banish": {"eligible_traits": ["demon", "phantasm"]}
        }),
        json!({"kind": "actor", "range": 3, "requires_visible": true}),
        "character",
    );
    family_engine("thaumaturge", lane, vec![summon, banish], 7, |parts| {
        parts.actor_definition_mut(1)["creature_traits"] = json!(["demon"]);
        parts.actor_definition_mut(1)["social"]["alignment_source"] =
            json!({"kind": "inherent", "alignment": "neutral"});
        parts.actor_definition_mut(1)["social"]["behavior"] = json!("passive");
        parts.push_selected(
            "items",
            "item/demon_claw/effect_family_test",
            json!({
                "id": "demon_claw", "kind": "weapon", "name": "Demon Claw",
                "weapon": {
                    "skill_track_id": "hand", "default_attack_mode": "fight",
                    "attack_modes": [{"mode": "fight", "maximum_range": 0, "damage_kind": "cutting"}],
                    "cooldown_units": 1, "combat_add_rating": 0, "handedness": "one_handed",
                    "block_value": 0
                },
                "valid_placements": ["hand"], "economy": {"unit_burden": 1}
            }),
        );
        parts.profile_value_mut()["summon_templates"] = json!([]);
        parts.push_selected(
            "actor_definitions",
            "actor/bound_demon/effect_family_test",
            json!({
                "id": "actor/bound_demon",
                "name": "Bound Demon",
                "kind": "monster",
                "creature_traits": ["demon"],
                "social": {"alignment_source":{"kind":"inherent","alignment":"lawful"},"nature":"other","behavior":"alignment_creature","owner_relation":"summoner"},
                "stats": {"hp": 4, "attack": 1, "defense": 0},
                "magic_resistance": {"natural_save_twentieths": 0, "evidence_state": "original_provisional"},
                "death": {"remains": "none"},
                "ai": {"behavior": "hold_ground", "cadence_units": 1, "aggro_radius": 7, "leash_range": 12, "awareness": {"mode": "unrestricted"}, "physical_attack_modes": ["fight"]},
                "xp_value": 0,
                "physical_damage_affinity_profile_id": "ordinary",
                "monster_abilities": []
            }),
        );
        parts.push_selected(
            "summon_templates",
            "summon/bound_demon/effect_family_test",
            json!({
                "id": "bound_demon",
                "actor_definition_id": "actor/bound_demon",
                "item_instances": {"claw": {"definition_id": "demon_claw", "binding": {"state": "unrestricted"}}},
                "carried": {"items": [{"item_instance_id": "claw", "position": "right_hand"}], "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}},
                "active_effects": []
            }),
        );
    })
}

#[test]
fn banish_removes_only_owned_summoned_demon_and_preserves_failed_targets() {
    let mut engine = banish_engine();
    let debug_target = engine
        .snapshot()
        .actors
        .into_iter()
        .find(|actor| actor.id == "target")
        .expect("debug target");
    assert_eq!(debug_target.creature_traits, vec![CreatureTrait::Demon]);
    let observed_target = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot")
        .actors
        .into_iter()
        .find(|actor| actor.id == "target")
        .expect("observed target");
    assert_eq!(observed_target.creature_traits, vec![CreatureTrait::Demon]);
    let target_index = engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "target")
        .expect("target index");
    engine.world_mut().actors[target_index]
        .social
        .alignment_source = SocialAlignmentSource::Inherent {
        alignment: CharacterAlignment::Chaotic,
    };
    engine.world_mut().actors[target_index].social.behavior = SocialBehavior::AlignmentCreature;
    let action_target = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("observed action context")
        .attack_targets
        .into_iter()
        .find(|actor| actor.actor_id == "target")
        .expect("action target");
    assert_eq!(action_target.creature_traits, vec![CreatureTrait::Demon]);
    engine.world_mut().actors[target_index]
        .social
        .alignment_source = SocialAlignmentSource::Inherent {
        alignment: CharacterAlignment::Neutral,
    };
    engine.world_mut().actors[target_index].social.behavior = SocialBehavior::Passive;

    let summoned = cast(
        &mut engine,
        "call_demon",
        Some(SpellTarget::Coordinate {
            position: WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 1 }),
        }),
    );
    let summoned_id = summoned
        .iter()
        .find_map(|event| match event {
            Event::ActorSummoned { actor_id, .. } => Some(actor_id.clone()),
            _ => None,
        })
        .expect("summoned id");
    let item_id = format!("{summoned_id}:item:claw");
    assert!(engine.world().item_instances.contains_key(&item_id));

    let summon_index = engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == summoned_id)
        .expect("summon index");
    let summon_actor = engine.world_mut().actors.remove(summon_index);
    engine.world_mut().actors.insert(0, summon_actor);
    let banished = cast(
        &mut engine,
        "banish",
        Some(SpellTarget::Actor {
            actor_id: summoned_id.clone(),
        }),
    );
    assert!(banished.iter().any(|event| matches!(
        event,
        Event::BanishEvaluated {
            reason: BanishResultReasonV1::Banished,
            success: true,
            ..
        }
    )));
    assert!(banished.iter().any(|event| matches!(
        event,
        Event::ActorBanished { actor_id, .. } if actor_id == &summoned_id
    )));
    assert!(
        !engine
            .world()
            .actors
            .iter()
            .any(|actor| actor.id == summoned_id)
    );
    assert!(!engine.world().item_instances.contains_key(&item_id));
    assert!(banished.iter().any(|event| matches!(
        event,
        Event::ActorReadinessScheduled { actor_id, .. } if actor_id == "player"
    )));

    let mp_before = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .mp;
    let failed = cast(
        &mut engine,
        "banish",
        Some(SpellTarget::Actor {
            actor_id: "target".into(),
        }),
    );
    assert!(failed.iter().any(|event| matches!(
        event,
        Event::BanishEvaluated {
            target_id,
            reason: BanishResultReasonV1::WillpowerFormulaOpen,
            success: false,
            ..
        } if target_id == "target"
    )));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .mp,
        mp_before - 1
    );
    assert!(!failed.iter().any(|event| matches!(
        event,
        Event::MagicPracticeEvaluated { spell_id, .. } if spell_id == "banish"
    )));
    assert!(
        engine
            .world()
            .actors
            .iter()
            .any(|actor| actor.id == "target")
    );
}

#[test]
fn instant_death_uses_level_multiplier_shared_save_and_single_defeat_path() {
    let lane = "thaumaturge_magic";
    let death = spell(
        "death",
        lane,
        json!({
            "family": "instant_death",
            "instant_death": {"damage_per_magic_level": 10},
            "resistance": {"role": "incoming", "tag": "death", "mitigation": {"mode": "half_damage", "rounding": "down", "minimum_damage": 1}}
        }),
        json!({"kind": "actor", "range": 3, "requires_visible": true}),
        "character",
    );
    let mut saved = family_engine("thaumaturge", lane, vec![death.clone()], 7, |parts| {
        parts.actor_definition_mut(1)["stats"]["hp"] = json!(60);
        parts.actor_definition_mut(1)["magic_resistance"]["natural_save_twentieths"] = json!(20);
    });
    let saved_events = cast(
        &mut saved,
        "death",
        Some(SpellTarget::Actor {
            actor_id: "target".into(),
        }),
    );
    assert!(saved_events.iter().any(|event| matches!(
        event,
        Event::SpellSaveResolved {
            requested_damage: Some(50),
            resolved_damage: Some(25),
            success: true,
            ..
        }
    )));
    assert!(saved_events.iter().any(|event| matches!(
        event,
        Event::SpellDamaged {
            damage: 25,
            hp: 35,
            ..
        }
    )));

    let mut lethal = family_engine("thaumaturge", lane, vec![death], 7, |parts| {
        parts.actor_definition_mut(1)["stats"]["hp"] = json!(40);
        parts.actor_definition_mut(1)["xp_value"] = json!(25);
        parts.actor_definition_mut(1)["magic_resistance"]["natural_save_twentieths"] = json!(0);
    });
    let lethal_events = cast(
        &mut lethal,
        "death",
        Some(SpellTarget::Actor {
            actor_id: "target".into(),
        }),
    );
    assert_eq!(
        lethal_events
            .iter()
            .filter(|event| matches!(event, Event::ActorDefeated { actor_id, cause: DeathCause::OtherMagic, .. } if actor_id == "target"))
            .count(),
        1
    );
    assert_eq!(lethal.world().corpses.len(), 1);
    assert_eq!(
        lethal_events
            .iter()
            .filter(|event| matches!(event, Event::DefeatRewardEvaluated { target_id, .. } if target_id == "target"))
            .count(),
        1
    );
}

fn raise_dead_engine(seed: u64) -> Engine {
    let lane = "thaumaturge_magic";
    let raise = spell(
        "raise_dead",
        lane,
        json!({"family": "raise_dead", "raise_dead": {"method": "thaumaturge"}}),
        json!({"kind": "none"}),
        "not_applicable",
    );
    family_engine("thaumaturge", lane, vec![raise], seed, |parts| {
        parts.actor_definition_mut(1)["social"]["alignment_source"] =
            json!({"kind": "inherent", "alignment": "neutral"});
        parts.actor_definition_mut(1)["social"]["behavior"] = json!("passive");
    })
}

fn install_player_corpse(engine: &mut Engine, corpse_id: &str, sequence: u64) {
    let corpse_id = CorpseId::parse(corpse_id).expect("corpse id");
    let target_index = engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "target")
        .expect("target");
    let location = engine.world().actors[0].location.clone();
    engine.world_mut().actors[target_index].kind = ActorKind::Player;
    engine.world_mut().actors[target_index]
        .social
        .alignment_source = SocialAlignmentSource::Inherent {
        alignment: CharacterAlignment::Neutral,
    };
    engine.world_mut().actors[target_index].social.nature = SocialNature::Human;
    engine.world_mut().actors[target_index].social.behavior = SocialBehavior::Adventurer;
    engine.world_mut().actors[target_index].corpse_disposition =
        tme_rules::CorpseDisposition::SearchableCorpse;
    engine.world_mut().actors[target_index].ai = None;
    engine.world_mut().actors[target_index].xp_value = 0;
    engine.world_mut().actors[target_index].hp = 0;
    engine.world_mut().actors[target_index].stamina = 0;
    engine.world_mut().actors[target_index].life_state = ActorLifeState::Ghost {
        corpse_id: corpse_id.clone(),
        defeated_at: LogicalTime::FIRST,
    };
    engine.world_mut().corpses.insert(
        corpse_id.clone(),
        CorpseState {
            id: corpse_id,
            origin_actor_id: "target".into(),
            origin_character_id: None,
            origin_kind: ActorKind::Player,
            origin_name: "Target".to_string(),
            location,
            created_at: LogicalTime::FIRST,
            sequence,
            searched: false,
            loot_claim: None,
            contents: BTreeMap::new(),
            gold: 0,
        },
    );
}

#[test]
fn raise_dead_selects_newest_player_corpse_and_rolls_only_when_eligible() {
    let mut empty = raise_dead_engine(7);
    let debug_rules =
        serde_json::to_value(&empty.snapshot().rules.magic.effect_families.raise_dead)
            .expect("debug Raise Dead rules serialize");
    let observed_rules = serde_json::to_value(
        &empty
            .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
            .expect("observed snapshot")
            .rules
            .magic
            .effect_families
            .raise_dead,
    )
    .expect("observed Raise Dead rules serialize");
    assert_eq!(
        debug_rules,
        json!({
            "roll_denominator": 20,
            "success_threshold_per_magic_level": 1,
            "minimum_success_threshold": 1,
            "evidence_state": "original_provisional"
        })
    );
    assert_eq!(observed_rules, debug_rules);
    let no_corpse = cast(&mut empty, "raise_dead", None);
    assert!(no_corpse.iter().any(|event| matches!(
        event,
        Event::RaiseDeadEvaluated {
            reason: RaiseDeadResultReasonV1::NoCorpse,
            roll: None,
            ..
        }
    )));

    let mut failed = raise_dead_engine(7);
    install_player_corpse(&mut failed, "corpse:1", 1);
    let failed_events = cast(&mut failed, "raise_dead", None);
    assert!(failed_events.iter().any(|event| matches!(
        event,
        Event::RaiseDeadEvaluated {
            corpse_id: Some(id),
            success_threshold: 5,
            roll: Some(11),
            reason: RaiseDeadResultReasonV1::RollFailed,
            ..
        } if id.as_str() == "corpse:1"
    )));
    assert!(matches!(
        failed
            .world()
            .actors
            .iter()
            .find(|actor| actor.id == "target")
            .expect("target")
            .life_state,
        ActorLifeState::Ghost { .. }
    ));

    let mut succeeded = raise_dead_engine(3);
    install_player_corpse(&mut succeeded, "corpse:2", 2);
    succeeded.world_mut().corpses.insert(
        CorpseId::parse("corpse:1").expect("old corpse"),
        CorpseState {
            id: CorpseId::parse("corpse:1").expect("old corpse"),
            origin_actor_id: "old_npc".into(),
            origin_character_id: None,
            origin_kind: ActorKind::Monster,
            origin_name: "Old NPC".to_string(),
            location: WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 }),
            created_at: LogicalTime::FIRST,
            sequence: 1,
            searched: false,
            loot_claim: None,
            contents: BTreeMap::new(),
            gold: 0,
        },
    );
    let success_events = cast(&mut succeeded, "raise_dead", None);
    assert!(success_events.iter().any(|event| matches!(
        event,
        Event::RaiseDeadEvaluated {
            corpse_id: Some(id),
            roll: Some(3),
            reason: RaiseDeadResultReasonV1::Resurrected,
            ..
        } if id.as_str() == "corpse:2"
    )));
    let target = succeeded
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "target")
        .expect("raised target");
    assert!(target.is_alive());
    assert_eq!(target.hp, 4);
    assert_eq!(target.stamina, 0);
    assert!(
        !succeeded
            .world()
            .corpses
            .contains_key(&CorpseId::parse("corpse:2").expect("new corpse"))
    );
    assert!(
        succeeded
            .world()
            .corpses
            .contains_key(&CorpseId::parse("corpse:1").expect("old corpse"))
    );
}

#[test]
fn turn_undead_uses_visible_stable_actor_order_and_shared_flee_without_damage() {
    let lane = "thaumaturge_magic";
    let turn = spell(
        "turn_undead",
        lane,
        json!({"family": "turn_undead", "turn_undead": {"eligible_trait": "undead"}}),
        json!({"kind": "none"}),
        "not_applicable",
    );
    let mut engine = family_engine("thaumaturge", lane, vec![turn], 7, |parts| {
        parts.template_levels_source_mut()["room_0"]["width"] = json!(7);
        parts.template_levels_source_mut()["room_0"]["cells"] =
            layered_cells(&["#######", "#.....#", "#######"]);
        parts.actors_mut()[0]["location"]["position"] = json!({"x": 1, "y": 1});
        let mut actors = parts.actors_mut().as_array().expect("actors").clone();
        actors[1]["id"] = json!("z_undead");
        actors[1]["location"]["position"] = json!({"x": 3, "y": 1});
        let mut second = actors[1].clone();
        second["id"] = json!("a_undead");
        second["location"]["position"] = json!({"x": 4, "y": 1});
        actors.push(second);
        let mut living = actors[1].clone();
        living["id"] = json!("living");
        living["actor_definition_id"] = json!("actor/test/living");
        living["location"]["position"] = json!({"x": 2, "y": 1});
        actors.push(living);
        *parts.actors_mut() = Value::Array(actors);
        let mut living_definition = parts.actor_definition_mut(1).clone();
        living_definition["id"] = json!("actor/test/living");
        living_definition["name"] = json!("Living");
        living_definition["creature_traits"] = json!([]);
        parts.actor_definition_mut(1)["name"] = json!("Undead");
        parts.actor_definition_mut(1)["creature_traits"] = json!(["undead"]);
        parts.actor_definition_mut(1)["stats"]["hp"] = json!(12);
        parts.push_selected(
            "actor_definitions",
            "actor/test/living/turn_undead",
            living_definition,
        );
    });
    let hp_before = engine
        .world()
        .actors
        .iter()
        .filter(|actor| actor.creature_traits.contains(&CreatureTrait::Undead))
        .map(|actor| (actor.id.clone(), actor.hp))
        .collect::<BTreeMap<_, _>>();
    let events = cast(&mut engine, "turn_undead", None);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::TurnUndeadResolved {
            considered_actor_ids,
            moved_actor_ids,
            blocked_actor_ids,
            ..
        } if considered_actor_ids == &vec!["a_undead".to_string(), "z_undead".to_string()]
            && moved_actor_ids == considered_actor_ids
            && blocked_actor_ids.is_empty()
    )));
    assert!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                Event::AutomaticActorDecision {
                    decision: AutomaticActorDecisionV1::Move {
                        purpose: AutomaticMovementPurposeV1::Turned,
                        ..
                    },
                    ..
                }
            ))
            .count()
            >= 2
    );
    for actor in engine
        .world()
        .actors
        .iter()
        .filter(|actor| actor.creature_traits.contains(&CreatureTrait::Undead))
    {
        assert_eq!(Some(&actor.hp), hp_before.get(&actor.id));
    }
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::ActorDefeated { .. }
            | Event::SpellDamaged { .. }
            | Event::DefeatRewardEvaluated { .. }
    )));
}
