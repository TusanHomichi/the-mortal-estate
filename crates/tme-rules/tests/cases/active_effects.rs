use crate::support::content_parts::ContentParts;
use tme_rules::{
    ActionBlockedReasonV1, AutomaticActorDecisionV1, COMMAND_CONTRACT_VERSION, Coord, Direction,
    Engine, Event, MovementStopReason, PathPreviewBlockedReasonV1, PathPreviewStepOutcomeV1,
    PlayerCommandV1, PlayerIntent, PlayerIntentPayloadV1, SpellTarget, WorldPosition,
    model::{ActiveEffectSource, ActiveEffectStackingPolicy, ActiveEffectState, TileEffectState},
};

fn equip_one_handed_ranged_weapon(
    parts: &mut ContentParts,
    actor_id: &str,
    weapon_instance_id: &str,
    maximum_range: u32,
) {
    parts.push_selected(
        "items",
        &format!("item/{weapon_instance_id}/active_effects"),
        serde_json::json!({
            "id": weapon_instance_id,
            "kind": "weapon",
            "name": "Test Javelin",
            "weapon": {
                "skill_track_id": "staff",
                "default_attack_mode": "throw",
                "attack_modes": [{
                    "mode": "throw",
                    "maximum_range": maximum_range,
                    "damage_kind": "piercing"
                }],
                "cooldown_units": 1,
                "combat_add_rating": 0,
                "handedness": "one_handed",
                "block_value": 0
            },
            "valid_placements": ["hand", "belt_side", "belt_back", "sack"]
        , "economy": {"unit_burden": 1}}),
    );
    parts.item_instances_mut()[weapon_instance_id] = serde_json::json!({
        "definition_id": weapon_instance_id,
        "binding": {"state": "unrestricted"}
    });
    let actor = parts
        .actors_mut()
        .as_array_mut()
        .expect("actors should be an array")
        .iter_mut()
        .find(|actor| actor["id"] == actor_id)
        .expect("weapon owner should exist");
    actor["carried"]["items"]
        .as_array_mut()
        .expect("carried items should be an array")
        .push(serde_json::json!({
            "item_instance_id": weapon_instance_id,
            "position": "right_hand"
        }));
}

fn status_engine_with_seed(mutate: impl FnOnce(&mut ContentParts), seed: u64) -> Engine {
    let mut parts = ContentParts::tracked("status_effects", "profile/status_effects");
    mutate(&mut parts);
    parts.engine(seed).expect("engine should start")
}

fn status_engine_with(mutate: impl FnOnce(&mut ContentParts)) -> Engine {
    status_engine_with_seed(mutate, 7)
}

fn status_engine() -> Engine {
    status_engine_with(|_| {})
}

fn thief_hide_engine_on_dark_tile_with(mutate: impl FnOnce(&mut ContentParts)) -> Engine {
    let mut parts = ContentParts::tracked(
        "profession_specific_actions",
        "profile/profession_specific_actions",
    );
    parts.template_levels_source_mut()["room_0"] = serde_json::json!({
        "law_zone": "none",
        "scene_role": "combat_space",
        "presentation_mode": "combat_space",
        "world_zoom": {"screen_cell_pitch": [156, 104]},
        "maximum_clear_sightline": 5,
        "staged_viewport": null,
        "wall_terrain_ids": [],
        "static_props": [],
        "width": 7,
        "height": 5,
        "cells": [
            [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]],
            [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
            [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
            [["stone_wall"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["flagstone"], ["stone_wall"]],
            [["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"], ["stone_wall"]]
        ]
    });
    parts.actors_mut()[0]["location"]["position"] = serde_json::json!({"x": 2, "y": 2});
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 5, "y": 2});
    parts.actor_definition_mut(1)["ai"]["behavior"] = serde_json::json!("hold_ground");
    mutate(&mut parts);
    let mut engine = parts.engine(7).expect("hide fixture should start");
    engine.world_mut().tile_effects.push(TileEffectState {
        source_actor_id: None,
        instance_id: "tile:shadow:1".to_string(),
        effect_id: "shadow_veil".to_string(),
        source: ActiveEffectSource {
            kind: "spell".to_string(),
            id: "shadow_veil".to_string(),
        },
        location: WorldPosition::new("realm_0", "room_0", tme_rules::Coord { x: 2, y: 2 }),
        kind: "terrain_overlay".to_string(),
        tags: vec!["shadow".to_string()],
        potency: 0,
        remaining_rounds: Some(3),
        passability: None,
        sight: Some("obscured".to_string()),
        hazard: None,
        move_cost: None,
        tick_interval_rounds: 1,
        last_ticked_at: tme_rules::LogicalTime::new(0),
        hostile_authority: None,
    });
    engine
}
fn thief_hide_engine_on_dark_tile() -> Engine {
    thief_hide_engine_on_dark_tile_with(|_| {})
}

fn suppressed_monster_ability_engine() -> Engine {
    let mut parts = ContentParts::tracked(
        "monster_spellcasting_special_attacks",
        "profile/monster_spellcasting_special_attacks",
    );
    let actors = parts.actors_mut().as_array_mut().expect("seed actors");
    actors.retain(|actor| matches!(actor["id"].as_str(), Some("player" | "ember_imp")));
    actors[0]["location"]["position"] = serde_json::json!({"x": 1, "y": 1});
    actors[1]["location"]["position"] = serde_json::json!({"x": 4, "y": 1});
    actors[1]["active_effects"] = serde_json::json!([
        {
            "instance_id": "stun_1",
            "effect_id": "stunning_gaze",
            "source": {"kind": "fixture", "id": "suppressed_monster_ability"},
            "kind": "control_status",
            "tags": ["stun"],
            "potency": 0,
            "remaining_rounds": 2,
            "stacking": "refresh_duration",
            "start_delay_rounds": 0,
            "tick_interval_rounds": 1,
            "suppresses_action": true,
            "resistance_boosts": []
        }
    ]);
    parts.actor_definition_mut(1)["ai"]["awareness"] = serde_json::json!({"mode": "unrestricted"});
    parts.engine(7).expect("suppression fixture should start")
}
fn summon_item_engine(mutate: impl FnOnce(&mut ContentParts)) -> Engine {
    let mut parts = ContentParts::tracked(
        "summons_created_creature_lifecycle",
        "profile/summons_created_creature_lifecycle",
    );
    parts.template_levels_source_mut()["start"]["width"] = serde_json::json!(6);
    parts.template_levels_source_mut()["start"]["cells"] = serde_json::json!([
        [
            ["stone_wall"],
            ["stone_wall"],
            ["stone_wall"],
            ["stone_wall"],
            ["stone_wall"],
            ["stone_wall"]
        ],
        [
            ["stone_wall"],
            ["flagstone"],
            ["flagstone"],
            ["flagstone"],
            ["flagstone"],
            ["stone_wall"]
        ],
        [
            ["stone_wall"],
            ["stone_wall"],
            ["stone_wall"],
            ["stone_wall"],
            ["stone_wall"],
            ["stone_wall"]
        ]
    ]);
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 4, "y": 1});
    parts.push_selected(
        "items",
        "item/echo_focus/active_effects",
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
            "capability": {"taxonomy_id": "echo_focus"},
            "valid_placements": ["hand", "belt_side", "belt_back", "sack"],
            "economy": {"unit_burden": 1}
        }),
    );
    {
        let summon = parts.selected_mut("summon_templates", 0);
        summon["id"] = serde_json::json!("echo_guard");
        summon["item_instances"] = serde_json::json!({
            "focus": {"definition_id": "echo_focus", "binding": {"state": "unrestricted"}}
        });
        summon["carried"] = serde_json::json!({
            "items": [{"item_instance_id": "focus", "position": "right_hand"}],
            "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}
        });
    }
    let summon_definition = parts.summon_actor_definition_mut(0);
    summon_definition["name"] = serde_json::json!("Echo Guard");
    summon_definition["ai"]["behavior"] = serde_json::json!("hold_ground");
    summon_definition["stats"]["attack"] = serde_json::json!(0);
    parts.selected_mut("spells", 0)["effect"]["summon_actor_id"] = serde_json::json!("echo_guard");
    mutate(&mut parts);
    parts
        .engine(1_010_580_540)
        .expect("summon item lifecycle engine should start")
}
#[test]
fn summon_item_instances_use_deterministic_expanded_ids() {
    let mut engine = summon_item_engine(|value| {
        value.selected_mut("spells", 0)["effect"]["duration"]["rounds"] = serde_json::json!(4);
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
        .expect("first summon should succeed");
    engine
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
        .expect("second summon should succeed");

    let world = engine.world();
    let first_item_instance_id = "summon:call_echo:1:echo_guard:item:focus";
    let second_item_instance_id = "summon:call_echo:2:echo_guard:item:focus";
    assert!(world.item_instances.contains_key(first_item_instance_id));
    assert!(world.item_instances.contains_key(second_item_instance_id));
    for (actor_id, item_instance_id) in [
        ("summon:call_echo:1:echo_guard", first_item_instance_id),
        ("summon:call_echo:2:echo_guard", second_item_instance_id),
    ] {
        let actor = world
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .expect("summoned actor should exist");
        assert!(actor.character_id.is_none());
        assert_eq!(actor.carried.items.len(), 1);
        assert_eq!(
            actor
                .carried
                .items
                .get(&tme_rules::CarriedPosition::RightHand)
                .map(String::as_str),
            Some(item_instance_id)
        );
    }
}

#[test]
fn summoned_bow_instances_are_initialized_from_the_weapon_definition() {
    let mut engine = summon_item_engine(|value| {
        value.selected_mut("items", 0)["weapon"]["skill_track_id"] = serde_json::json!("bow");
        value.selected_mut("items", 0)["weapon"]["default_attack_mode"] =
            serde_json::json!("shoot");
        value.selected_mut("items", 0)["weapon"]["attack_modes"] = serde_json::json!([{
            "mode": "shoot",
            "maximum_range": 3,
            "damage_kind": "piercing"
        }]);
        value.selected_mut("items", 0)["weapon"]["handedness"] = serde_json::json!("bow");
        value.selected_mut("items", 0)["weapon"]["nocking"] =
            serde_json::json!({"unloads_on_movement": true});
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
        .expect("bow-bearing summon should spawn");
    let item_instance_id = "summon:call_echo:1:echo_guard:item:focus";
    let readiness = engine.world().item_instances[item_instance_id]
        .bow_readiness
        .expect("summoned bow must have readiness state");
    if readiness == tme_rules::BowReadiness::Nocked {
        assert!(events.iter().any(|event| matches!(
            event,
            Event::BowReadinessChanged {
                item_instance_id: changed_id,
                from: tme_rules::BowReadiness::Unnocked,
                to: tme_rules::BowReadiness::Nocked,
                ..
            } if changed_id == item_instance_id
        )));
    }
}

#[test]
fn summon_item_expiry_destroys_still_owned_instance() {
    let mut engine = summon_item_engine(|_| {});
    let actor_id = "summon:call_echo:1:echo_guard";
    let item_instance_id = "summon:call_echo:1:echo_guard:item:focus";

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
        .expect("summon should succeed");
    assert!(engine.world().item_instances.contains_key(item_instance_id));

    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("first wait should keep summon alive");
    let expiry_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("second wait should expire summon");

    assert!(expiry_events.iter().any(
        |event| matches!(event, Event::SummonExpired { actor_id: expired_id, .. } if expired_id == actor_id)
    ));
    assert!(!engine.world().item_instances.contains_key(item_instance_id));
    assert!(
        !engine
            .world()
            .ground_items
            .iter()
            .any(|item| item.item_instance_id == item_instance_id)
    );
}

#[test]
fn summon_item_defeat_drop_survives_later_expiry() {
    let mut engine = summon_item_engine(|value| {
        value.summon_actor_definition_mut(0)["social"]["alignment_source"] =
            serde_json::json!({"kind": "inherent", "alignment": "chaotic"});
        value.summon_actor_definition_mut(0)["stats"]["hp"] = serde_json::json!(1);
        value.selected_mut("spells", 0)["effect"]["duration"]["rounds"] = serde_json::json!(3);
        equip_one_handed_ranged_weapon(value, "player", "player_test_bow", 3);
    });
    let actor_id = "summon:call_echo:1:echo_guard";
    let item_instance_id = "summon:call_echo:1:echo_guard:item:focus";
    let defeat_position = Coord { x: 2, y: 1 };

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", defeat_position),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("hostile summon should succeed");
    {
        engine
            .world_mut()
            .actors
            .iter_mut()
            .find(|actor| actor.id == actor_id)
            .and_then(|actor| actor.summoned.as_mut())
            .expect("summon authority")
            .owner_id = "external_owner".into();
        let player = engine
            .world_mut()
            .actors
            .iter_mut()
            .find(|actor| actor.id == "player")
            .expect("player");
        player.stats.attack = 20;
    }

    let defeat_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: actor_id.into(),
            },
        )
        .expect("player attack should defeat summon");

    assert!(defeat_events.iter().any(
        |event| matches!(event, Event::ActorDefeated { actor_id: defeated_id, .. } if defeated_id == actor_id)
    ));
    assert!(defeat_events.iter().any(|event| matches!(
        event,
        Event::ItemRelocated {
            actor_id: defeated_id,
            item_instance_id: dropped_item_instance_id,
            to: tme_rules::ItemLocationViewV1::Ground { location },
            reason: tme_rules::ItemRelocationReason::DeathDrop,
            ..
        } if defeated_id == actor_id
            && dropped_item_instance_id == item_instance_id
            && location == &WorldPosition::new("realm_0", "start", defeat_position)
    )));
    let defeated = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == actor_id)
        .expect("defeated summon remains until expiry");
    assert!(defeated.carried.items.is_empty());
    assert!(engine.world().item_instances.contains_key(item_instance_id));
    assert!(engine.world().ground_items.iter().any(|item| {
        item.item_instance_id == item_instance_id
            && item.location == WorldPosition::new("realm_0", "start", defeat_position)
    }));

    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("first post-defeat wait should advance summon duration");
    let expiry_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("second post-defeat wait should expire summon actor");

    assert!(expiry_events.iter().any(
        |event| matches!(event, Event::SummonExpired { actor_id: expired_id, .. } if expired_id == actor_id)
    ));
    assert!(engine.world().item_instances.contains_key(item_instance_id));
    assert!(engine.world().ground_items.iter().any(|item| {
        item.item_instance_id == item_instance_id
            && item.location == WorldPosition::new("realm_0", "start", defeat_position)
    }));
}

#[test]
fn summon_item_same_round_defeat_and_expiry_preserve_all_drops() {
    let mut engine = summon_item_engine(|value| {
        value.summon_actor_definition_mut(0)["social"]["alignment_source"] =
            serde_json::json!({"kind": "inherent", "alignment": "chaotic"});
        value.summon_actor_definition_mut(0)["stats"]["hp"] = serde_json::json!(1);
        value.selected_mut("summon_templates", 0)["item_instances"]["token"] = serde_json::json!({
            "definition_id": "echo_focus",
            "binding": {"state": "unrestricted"}
        });
        value.selected_mut("summon_templates", 0)["carried"]["items"]
            .as_array_mut()
            .expect("summon carried items")
            .push(serde_json::json!({
                "item_instance_id": "token",
                "position": "sack_item_1"
            }));
        value.selected_mut("spells", 0)["effect"]["duration"]["rounds"] = serde_json::json!(1);
        equip_one_handed_ranged_weapon(value, "player", "player_test_bow", 3);
    });
    let actor_id = "summon:call_echo:1:echo_guard";
    let inventory_item_instance_id = "summon:call_echo:1:echo_guard:item:token";
    let equipment_item_instance_id = "summon:call_echo:1:echo_guard:item:focus";
    let defeat_position = Coord { x: 2, y: 1 };

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "call_echo".to_string(),
                target: Some(SpellTarget::Coordinate {
                    position: WorldPosition::new("realm_0", "start", defeat_position),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("hostile summon should succeed");
    {
        engine
            .world_mut()
            .actors
            .iter_mut()
            .find(|actor| actor.id == actor_id)
            .and_then(|actor| actor.summoned.as_mut())
            .expect("summon authority")
            .owner_id = "external_owner".into();
        let player = engine
            .world_mut()
            .actors
            .iter_mut()
            .find(|actor| actor.id == "player")
            .expect("player");
        player.stats.attack = 20;
    }

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: actor_id.into(),
            },
        )
        .expect("defeat and expiry should resolve in one round");

    let died_index = events
        .iter()
        .position(|event| matches!(event, Event::ActorDefeated { actor_id: defeated_id, .. } if defeated_id == actor_id))
        .expect("summon defeat event");
    let dropped = events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| match event {
            Event::ItemRelocated {
                actor_id: defeated_id,
                item_instance_id,
                reason: tme_rules::ItemRelocationReason::DeathDrop,
                ..
            } if defeated_id == actor_id => Some((index, item_instance_id.as_str())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(dropped.len(), 2);
    assert_eq!(dropped[0].1, equipment_item_instance_id);
    assert_eq!(dropped[1].1, inventory_item_instance_id);
    let expired_index = events
        .iter()
        .position(|event| matches!(event, Event::SummonExpired { actor_id: expired_id, .. } if expired_id == actor_id))
        .expect("same-round summon expiry event");
    assert!(died_index < dropped[0].0);
    assert!(dropped[0].0 < dropped[1].0);
    assert!(dropped[1].0 < expired_index);

    assert!(
        !engine
            .world()
            .actors
            .iter()
            .any(|actor| actor.id == actor_id)
    );
    for item_instance_id in [inventory_item_instance_id, equipment_item_instance_id] {
        assert!(engine.world().item_instances.contains_key(item_instance_id));
        assert!(engine.world().ground_items.iter().any(|item| {
            item.item_instance_id == item_instance_id
                && item.location == WorldPosition::new("realm_0", "start", defeat_position)
        }));
    }
}

#[test]
fn seeded_effect_emits_applied_initial_event() {
    let engine = status_engine();
    assert!(engine.initial_events().iter().any(|event| {
        matches!(event, Event::EffectApplied {
            actor_id,
            instance_id,
            effect_id,
            kind,
            ..
        } if actor_id == "player"
            && instance_id == "rooted_1"
            && effect_id == "generic_root"
            && kind == "control_status")
    }));
}

#[test]
fn active_effect_suppresses_non_passive_action_and_then_expires() {
    let mut engine = status_engine();

    let first = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("suppressed action should consume a round and emit an event");
    assert!(first.iter().any(|event| {
        matches!(event, Event::ActionSuppressedByStatus {
            actor_id,
            intent,
            instance_id,
            ..
        } if actor_id == "player" && intent == "walk east" && instance_id == "rooted_1")
    }));
    assert!(first.iter().any(|event| {
        matches!(event, Event::EffectTicked {
            actor_id,
            instance_id,
            remaining_rounds,
            ..
        } if actor_id == "player" && instance_id == "rooted_1" && *remaining_rounds == Some(1))
    }));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .location
            .position
            .x,
        1
    );

    let second = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("wait should be allowed");
    assert!(second.iter().any(|event| {
        matches!(event, Event::EffectExpired {
            actor_id,
            instance_id,
            ..
        } if actor_id == "player" && instance_id == "rooted_1")
    }));
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .active_effects
            .is_empty()
    );

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("movement should work after expiration");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .location
            .position
            .x,
        2
    );
}

#[test]
fn suppressing_effect_blocks_monster_ability_and_movement() {
    let mut engine = suppressed_monster_ability_engine();

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("suppressed monster round should resolve");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::AutomaticActorDecision {
            actor_id,
            actor,
            decision: AutomaticActorDecisionV1::Suppressed { status },
        } if actor_id == "ember_imp" && actor == "Ember Imp" && status == "stun"
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::SpellDamaged { caster_id, .. } if caster_id == "ember_imp"
    )));
    assert!(!events.iter().any(|event| {
        matches!(
            event,
            Event::Moved { actor, .. } | Event::Attacked { attacker: actor, .. }
                if actor == "Ember Imp"
        )
    }));
    let monster = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "ember_imp")
        .expect("monster");
    assert_eq!(monster.location.position, tme_rules::Coord { x: 4, y: 1 });
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .hp,
        12
    );
}

#[test]
fn command_validation_reports_suppressed_status() {
    let engine = status_engine();
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::MovePath {
            path: vec![Direction::East],
        },
    };
    let status = engine.validate_actor_command(&command).expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::SuppressedByStatus)
    );
}

#[test]
fn path_preview_reports_suppressed_status() {
    let engine = status_engine();
    let preview = engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .expect("preview");
    assert_eq!(preview.stop_reason, MovementStopReason::Blocked);
    let first = preview.steps.first().expect("blocked step");
    assert!(matches!(
        first.outcome,
        PathPreviewStepOutcomeV1::Blocked {
            reason: PathPreviewBlockedReasonV1::SuppressedByStatus
        }
    ));
    assert_eq!(preview.final_position.position.x, 1);
}

#[test]
fn thief_hide_applies_hidden_effect_and_breaks_on_move() {
    let mut engine = thief_hide_engine_on_dark_tile();

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player0"), PlayerIntent::Hide)
        .expect("hide succeeds");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ActorHidden {
            actor_id, effect_id, ..
        } if actor_id == "player0" && effect_id == "hidden"
    )));
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player0"))
            .expect("player")
            .active_effects
            .iter()
            .any(|effect| effect.kind == "hidden" && effect.tags.iter().any(|tag| tag == "hidden"))
    );

    let move_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player0"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("move succeeds");
    assert!(move_events.iter().any(|event| matches!(
        event,
        Event::HideBroken {
            actor_id, reason, ..
        } if actor_id == "player0" && reason == "move"
    )));
    assert!(
        !engine
            .world()
            .actor(&tme_rules::ActorId::from("player0"))
            .expect("player")
            .active_effects
            .iter()
            .any(|effect| effect.kind == "hidden")
    );
}

#[test]
fn thief_hide_survives_blocked_movement_attempt() {
    let mut engine = thief_hide_engine_on_dark_tile();
    engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == "player0")
        .expect("player")
        .location
        .position = tme_rules::Coord { x: 1, y: 1 };

    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player0"), PlayerIntent::Hide)
        .expect("hide succeeds");

    let move_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player0"),
            PlayerIntent::MovePath(vec![Direction::West]),
        )
        .expect("blocked movement returns events");
    assert!(move_events.iter().any(|event| matches!(
        event,
        Event::MovementBlocked {
            actor_id, reason, ..
        } if actor_id == "player0" && reason == "blocked terrain"
    )));
    assert!(!move_events.iter().any(|event| {
        matches!(event, Event::HideBroken { actor_id, .. } if actor_id == "player0")
    }));
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player0"))
            .expect("player")
            .active_effects
            .iter()
            .any(|effect| effect.kind == "hidden")
    );
}

#[test]
fn thief_hide_survives_not_ready_and_no_sight_attack_attempts() {
    let mut not_ready = thief_hide_engine_on_dark_tile_with(|value| {
        equip_one_handed_ranged_weapon(value, "player0", "player_test_bow", 5);
    });
    {
        let player = not_ready
            .world_mut()
            .actors
            .iter_mut()
            .find(|actor| actor.id == "player0")
            .expect("player");
        player.attack_ready_at = tme_rules::LogicalTime::new(99);
    }
    not_ready
        .apply_actor_intent(&tme_rules::ActorId::from("player0"), PlayerIntent::Hide)
        .expect("hide succeeds");

    let attack_events = not_ready
        .apply_actor_intent(
            &tme_rules::ActorId::from("player0"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "watcher".into(),
            },
        )
        .expect("not-ready attack returns events");
    assert!(attack_events.iter().any(|event| {
        matches!(event, Event::AttackNotReady { actor_id, .. } if actor_id == "player0")
    }));
    assert!(!attack_events.iter().any(|event| {
        matches!(event, Event::HideBroken { actor_id, .. } if actor_id == "player0")
    }));
    assert!(
        not_ready
            .world()
            .actor(&tme_rules::ActorId::from("player0"))
            .expect("player")
            .active_effects
            .iter()
            .any(|effect| effect.kind == "hidden")
    );

    let mut no_sight = thief_hide_engine_on_dark_tile_with(|value| {
        equip_one_handed_ranged_weapon(value, "player0", "player_test_bow", 5);
    });
    no_sight
        .apply_actor_intent(&tme_rules::ActorId::from("player0"), PlayerIntent::Hide)
        .expect("hide succeeds");
    no_sight
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == "player0")
        .expect("player")
        .active_effects
        .push(ActiveEffectState {
            spell_damage_credit: None,
            source_actor_id: None,
            hostile_authority: None,
            instance_id: "test:blind:player0".to_string(),
            effect_id: "test_blind".to_string(),
            source: ActiveEffectSource {
                kind: "test".to_string(),
                id: "blind".to_string(),
            },
            kind: "control_status".to_string(),
            tags: vec!["blind".to_string()],
            potency: 0,
            remaining_rounds: Some(3),
            until_condition: None,
            stacking: ActiveEffectStackingPolicy::RefreshDuration,
            start_delay_rounds: 0,
            tick_interval_rounds: 1,
            suppresses_action: false,
            resistance_boosts: vec![],
            last_ticked_at: tme_rules::LogicalTime::new(0),
        });

    let attack_events = no_sight
        .apply_actor_intent(
            &tme_rules::ActorId::from("player0"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "watcher".into(),
            },
        )
        .expect("no-sight attack returns events");
    assert!(attack_events.iter().any(|event| {
        matches!(event, Event::AttackBlockedNoSight { attacker_id, .. } if attacker_id == "player0")
    }));
    assert!(!attack_events.iter().any(|event| {
        matches!(event, Event::HideBroken { actor_id, .. } if actor_id == "player0")
    }));
    assert!(
        no_sight
            .world()
            .actor(&tme_rules::ActorId::from("player0"))
            .expect("player")
            .active_effects
            .iter()
            .any(|effect| effect.kind == "hidden")
    );
}

#[test]
fn actor_resistance_boosts_include_active_effects_and_equipped_items() {
    let mut engine = status_engine();
    engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == "player")
        .expect("player")
        .active_effects
        .first_mut()
        .expect("seeded effect")
        .resistance_boosts = vec![
        tme_rules::SpellResistanceBoost {
            tag: "poison".to_string(),
            bonus_twentieths: 3,
        },
        tme_rules::SpellResistanceBoost {
            tag: "stun".to_string(),
            bonus_twentieths: 4,
        },
        tme_rules::SpellResistanceBoost {
            tag: "poison".to_string(),
            bonus_twentieths: 2,
        },
    ];
    let boosts = engine
        .actor_resistance_boosts(&"player".into())
        .expect("resistance boosts");
    assert_eq!(
        boosts
            .iter()
            .map(|boost| (boost.tag.as_str(), boost.bonus_twentieths))
            .collect::<Vec<_>>(),
        vec![("poison", 3), ("poison", 2), ("stun", 4), ("stun", 3)]
    );
}

#[test]
fn defeat_removes_active_effects() {
    let mut engine = status_engine_with_seed(
        |value| {
            equip_one_handed_ranged_weapon(value, "player", "player_test_bow", 3);
        },
        1_010_580_540,
    );
    let seeded = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .active_effects
        .first()
        .expect("seeded effect")
        .clone();

    {
        let world = engine.world_mut();
        let player_index = world
            .actors
            .iter()
            .position(|actor| actor.id == "player")
            .expect("player index");
        let watcher_index = world
            .actors
            .iter()
            .position(|actor| actor.id == "watcher")
            .expect("watcher index");
        world.actors[player_index].active_effects.clear();
        world.actors[player_index].stats.attack = 20;
        world.actors[watcher_index].hp = 1;
        world.actors[watcher_index].active_effects = vec![seeded];
    }

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "watcher".into(),
            },
        )
        .expect("attack should kill watcher");
    assert!(events.iter().any(|event| {
        matches!(event, Event::EffectRemoved {
            actor_id,
            instance_id,
            reason,
            ..
        } if actor_id == "watcher" && instance_id == "rooted_1" && reason == "defeat")
    }));
    let watcher = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "watcher")
        .expect("watcher");
    assert!(watcher.active_effects.is_empty());
}

#[test]
fn poison_tick_can_defeat_actor_and_remove_active_effects() {
    let mut engine = status_engine_with(|parts| {
        parts.actors_mut()[0]["active_effects"] = serde_json::json!([]);
        parts.actor_definition_mut(1)["stats"]["hp"] = serde_json::json!(2);
        parts.actors_mut()[1]["active_effects"] = serde_json::json!([
        {
            "instance_id": "venom_1",
            "effect_id": "venom",
            "source": {"kind": "fixture", "id": "status_effects"},
            "kind": "poison",
            "tags": ["poison"],
            "potency": 2,
            "remaining_rounds": 2,
            "stacking": "replace_same_kind",
            "start_delay_rounds": 0,
            "tick_interval_rounds": 1,
            "suppresses_action": false,
            "resistance_boosts": []
        },
        {
            "instance_id": "ward_1",
            "effect_id": "ward",
            "source": {"kind": "fixture", "id": "status_effects"},
            "kind": "protection",
            "tags": ["ward"],
            "potency": 1,
            "remaining_rounds": 2,
            "stacking": "replace_same_kind",
            "start_delay_rounds": 0,
            "tick_interval_rounds": 1,
            "suppresses_action": false,
            "resistance_boosts": []
        }
        ]);
    });

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("wait should advance poison");

    let ticked = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::EffectTicked {
                    actor_id,
                    instance_id,
                    ..
                } if actor_id == "watcher" && instance_id == "venom_1"
            )
        })
        .expect("venom tick event");
    let damaged = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::EffectDamaged {
                    actor_id,
                    instance_id,
                    hp: 0,
                    ..
                } if actor_id == "watcher" && instance_id == "venom_1"
            )
        })
        .expect("venom damage event");
    let venom_removed = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::EffectRemoved {
                    actor_id,
                    instance_id,
                    reason,
                    ..
                } if actor_id == "watcher"
                    && instance_id == "venom_1"
                    && reason == "defeat"
            )
        })
        .expect("venom defeat removal");
    let ward_removed = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::EffectRemoved {
                    actor_id,
                    instance_id,
                    reason,
                    ..
                } if actor_id == "watcher"
                    && instance_id == "ward_1"
                    && reason == "defeat"
            )
        })
        .expect("ward defeat removal");
    let died = events
        .iter()
        .position(
            |event| matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == "watcher"),
        )
        .expect("watcher death");
    assert!(ticked < damaged);
    assert!(damaged < venom_removed);
    assert!(venom_removed < ward_removed);
    assert!(ward_removed < died);
    assert_eq!(
        events
            .iter()
            .filter(|event| {
                matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == "watcher")
            })
            .count(),
        1
    );

    assert!(events.iter().any(|event| matches!(
        event,
        Event::EffectDamaged {
            actor_id,
            effect_id,
            tags,
            damage,
            hp,
            ..
        } if actor_id == "watcher"
            && effect_id == "venom"
            && tags.iter().any(|tag| tag == "poison")
            && *damage == 2
            && *hp == 0
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::EffectRemoved {
            actor_id,
            instance_id,
            reason,
            ..
        } if actor_id == "watcher" && instance_id == "venom_1" && reason == "defeat"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::EffectRemoved {
            actor_id,
            instance_id,
            reason,
            ..
        } if actor_id == "watcher" && instance_id == "ward_1" && reason == "defeat"
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::EffectTicked {
            actor_id,
            instance_id,
            ..
        } if actor_id == "watcher" && instance_id == "ward_1"
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::EffectExpired {
            actor_id,
            instance_id,
            ..
        } if actor_id == "watcher" && instance_id == "ward_1"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ActorDefeated {
            actor_id,
            cause: tme_rules::DeathCause::Poison,
            credited_actor_id: None,
            ..
        } if actor_id == "watcher"
    )));
    let watcher = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "watcher")
        .expect("watcher");
    assert_eq!(watcher.hp, 0);
    assert!(!watcher.is_alive());
    assert!(watcher.active_effects.is_empty());
}
