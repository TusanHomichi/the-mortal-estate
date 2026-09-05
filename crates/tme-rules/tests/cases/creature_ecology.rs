use crate::support::content_parts::ContentParts;
use serde_json::json;
use tme_rules::{
    ActorId, Coord, EcologyLifecyclePolicyV1, Engine, Event, PhysicalAttackMode, PlayerIntent,
    WorldPosition,
};

fn parts() -> ContentParts {
    ContentParts::tracked(
        "creature_ecology_gallery",
        "profile/creature_ecology_gallery",
    )
}

fn parts_with_remote_level() -> ContentParts {
    let mut parts = parts();
    let remote = parts.world_template["realms"]["realm_0"]["levels"]["room_0"].clone();
    parts.world_template["realms"]["realm_0"]["levels"]["remote"] = remote;
    parts
}

fn move_player_to_level(engine: &mut Engine, level: &str) {
    engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == "player")
        .expect("gallery player")
        .location = WorldPosition::new("realm_0", level, Coord { x: 2, y: 1 });
}

fn checkpoint_round_trip(engine: &Engine) -> Engine {
    let checkpoint = engine.export_checkpoint().expect("export checkpoint");
    let hydrated =
        Engine::hydrate_checkpoint(engine.definition().clone(), &checkpoint).expect("hydrate");
    assert_eq!(
        checkpoint,
        hydrated.export_checkpoint().expect("re-export checkpoint")
    );
    hydrated
}

fn attack(target_actor_id: &str) -> PlayerIntent {
    PlayerIntent::PhysicalAttack {
        authorization: tme_rules::HostilityAuthorization::Safe,
        mode: PhysicalAttackMode::Fight,
        target_actor_id: target_actor_id.into(),
    }
}

fn loot_probe_parts(
    family: &str,
    maximum_non_gold_drops: Option<u8>,
    entries: serde_json::Value,
) -> ContentParts {
    let mut parts = parts();
    let mut table = json!({
        "family": family,
        "id": "gallery_pack_loot",
        "entries": entries,
    });
    if let Some(cap) = maximum_non_gold_drops {
        table["maximum_non_gold_drops"] = json!(cap);
    }
    *parts.selected_mut("loot_tables", 0) = table;
    parts
}

fn runner_item_ids(engine: &Engine) -> Vec<String> {
    engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "ecology:gallery_pack:runner:0")
        .expect("gallery runner")
        .carried
        .items
        .values()
        .cloned()
        .collect()
}

fn checkpoint_rng_state(engine: &Engine) -> String {
    let checkpoint = engine.export_checkpoint().expect("checkpoint");
    let value: serde_json::Value =
        serde_json::from_slice(checkpoint.as_bytes()).expect("checkpoint JSON");
    value["rng_state"]
        .as_str()
        .expect("decimal RNG state")
        .to_string()
}

#[test]
fn full_site_reset_defers_while_observed_then_recreates_stably_and_retains_remains() {
    let mut engine = parts_with_remote_level().engine(7).expect("gallery engine");
    let initial_spawns = engine
        .initial_events()
        .into_iter()
        .filter_map(|event| match event {
            Event::EcologyActorSpawned {
                site_id,
                member_id,
                generation,
                actor_id,
                ..
            } => Some((site_id, member_id, generation, actor_id)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        initial_spawns,
        [
            (
                "gallery_lair".to_string(),
                "burrower".to_string(),
                0,
                "ecology:gallery_lair:burrower:0".into(),
            ),
            (
                "gallery_pack".to_string(),
                "runner".to_string(),
                0,
                "ecology:gallery_pack:runner:0".into(),
            ),
            (
                "gallery_pack".to_string(),
                "keeper".to_string(),
                0,
                "ecology:gallery_pack:keeper:0".into(),
            ),
        ]
    );
    assert!(engine.world().actors.iter().all(|actor| {
        actor.id == "player"
            || (actor
                .definition_id
                .starts_with("actor/creature_ecology_gallery/")
                && actor.ecology_origin.is_some())
    }));

    let first = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            attack("ecology:gallery_pack:runner:0"),
        )
        .expect("first pack defeat");
    let affinity_index = first
        .events
        .iter()
        .position(|event| matches!(event, Event::PhysicalDamageAffinityApplied { .. }))
        .expect("vulnerability event");
    let attack_index = first
        .events
        .iter()
        .position(|event| matches!(event, Event::Attacked { .. }))
        .expect("attack event");
    assert!(affinity_index < attack_index);
    assert!(
        matches!(
            first[affinity_index],
            Event::PhysicalDamageAffinityApplied {
                input_damage: 43,
                numerator: 3,
                denominator: 2,
                adjusted_damage: 64,
                ..
            }
        ),
        "unexpected affinity event: {:?}",
        first[affinity_index]
    );
    assert_eq!(
        engine.world().ecology_sites["gallery_pack"].full_clear_due_at,
        None
    );

    let second = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            attack("ecology:gallery_pack:keeper:0"),
        )
        .expect("second pack defeat");
    assert_eq!(
        second
            .events
            .iter()
            .filter(|event| matches!(event, Event::EcologyResetScheduled { site_id, .. } if site_id == "gallery_pack"))
            .count(),
        1
    );
    assert!(
        engine.world().ecology_sites["gallery_pack"]
            .full_clear_due_at
            .is_some()
    );
    let scheduled_due = engine.world().ecology_sites["gallery_pack"].full_clear_due_at;

    let third = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            attack("ecology:gallery_lair:burrower:0"),
        )
        .expect("lair defeat while both sites remain observed");
    assert!(
        !third
            .events
            .iter()
            .any(|event| matches!(event, Event::EcologyReset { .. }))
    );
    assert_eq!(
        engine.world().ecology_sites["gallery_pack"].full_clear_due_at,
        scheduled_due,
        "an observed due site retains its exact due time"
    );

    move_player_to_level(&mut engine, "remote");
    let fourth = engine
        .apply_actor_intent(&ActorId::from("player"), PlayerIntent::Wait)
        .expect("unobserved reset boundary");
    let reset_index = fourth
        .events
        .iter()
        .position(|event| matches!(event, Event::EcologyReset { site_id, .. } if site_id == "gallery_pack"))
        .expect("pack reset");
    let spawned_indices = fourth
        .events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| {
            matches!(event, Event::EcologyActorSpawned { site_id, generation: 1, .. } if site_id == "gallery_pack")
                .then_some(index)
        })
        .collect::<Vec<_>>();
    assert_eq!(spawned_indices.len(), 2);
    assert!(spawned_indices.into_iter().all(|index| reset_index < index));
    assert_eq!(engine.world().ecology_sites["gallery_pack"].generation, 1);
    assert!(engine.world().actors.iter().all(|actor| !matches!(
        actor.id.as_str(),
        "ecology:gallery_pack:runner:0" | "ecology:gallery_pack:keeper:0"
    )));
    assert!(engine.world().actors.iter().any(|actor| {
        actor.id == "ecology:gallery_pack:runner:1"
            && actor
                .ecology_origin
                .as_ref()
                .is_some_and(|origin| origin.generation == 1)
    }));
    assert_eq!(engine.world().corpses.len(), 3);
    assert!(
        engine
            .world()
            .item_instances
            .contains_key("ecology:gallery_pack:runner:0:loot:gallery_pack_loot:flint")
    );
    assert!(
        engine
            .world()
            .item_instances
            .contains_key("ecology:gallery_pack:runner:1:loot:gallery_pack_loot:flint")
    );

    assert!(fourth.events.iter().any(|event| matches!(
        event,
        Event::EcologyReset {
            site_id,
            from_generation: 0,
            to_generation: 1,
            member_ids,
            policy: EcologyLifecyclePolicyV1::FullSite,
        } if site_id == "gallery_lair" && member_ids == &["burrower".to_string()]
    )));
    assert_eq!(engine.world().corpses.len(), 3);
}

#[test]
fn physical_affinity_immunity_identity_and_overflow_are_transactional() {
    let mut immune_parts = parts();
    immune_parts.selected_mut("actor_definitions", 1)["physical_damage_affinity_profile_id"] =
        json!("immune");
    let mut immune = immune_parts.engine(7).expect("immune gallery engine");
    let events = immune
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            attack("ecology:gallery_pack:runner:0"),
        )
        .expect("immune attack resolves");
    assert!(events.events.iter().any(|event| matches!(
        event,
        Event::PhysicalDamageAffinityApplied {
            numerator: 0,
            denominator: 1,
            adjusted_damage: 0,
            ..
        }
    )));
    assert_eq!(
        immune
            .world()
            .actors
            .iter()
            .find(|actor| actor.id == "ecology:gallery_pack:runner:0")
            .expect("immune runner")
            .hp,
        2
    );

    let mut ordinary = parts().engine(7).expect("ordinary gallery engine");
    let events = ordinary
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            attack("ecology:gallery_pack:keeper:0"),
        )
        .expect("ordinary attack resolves");
    assert!(
        !events
            .events
            .iter()
            .any(|event| matches!(event, Event::PhysicalDamageAffinityApplied { .. }))
    );

    let mut overflow_parts = parts();
    overflow_parts.selected_mut("physical_damage_affinity_profiles", 2)["responses"][2]["numerator"] =
        json!(u32::MAX);
    overflow_parts.selected_mut("physical_damage_affinity_profiles", 2)["responses"][2]["denominator"] =
        json!(1);
    let mut overflow = overflow_parts.engine(7).expect("overflow gallery engine");
    let before = serde_json::to_value(overflow.snapshot()).expect("snapshot before overflow");
    let mut control = overflow.clone();
    let error = overflow
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            attack("ecology:gallery_pack:runner:0"),
        )
        .expect_err("out-of-range adjusted damage must fail");
    assert_eq!(
        error.message(),
        "physical affinity adjusted damage exceeds i32"
    );
    assert_eq!(
        serde_json::to_value(overflow.snapshot()).expect("snapshot after overflow"),
        before
    );
    assert_eq!(
        serde_json::to_value(
            overflow
                .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
                .expect("post-error wait")
                .events
        )
        .expect("post-error events"),
        serde_json::to_value(
            control
                .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
                .expect("control wait")
                .events
        )
        .expect("control events"),
        "failed affinity transaction must restore RNG as well as world state"
    );
}

#[test]
fn loot_chance_choice_range_cap_and_replay_use_the_one_authored_rng_order() {
    let direct = loot_probe_parts(
        "ordinary",
        Some(2),
        json!([
            {
                "kind": "item",
                "id": "fails_at_thirty",
                "chance_numerator": 30,
                "chance_denominator": 100,
                "item_definition_id": "flint",
                "quantity": 1,
                "position": "sack_item_1"
            },
            {
                "kind": "item",
                "id": "succeeds_at_thirty_one",
                "chance_numerator": 31,
                "chance_denominator": 100,
                "item_definition_id": "cloth_bundle",
                "quantity": 1,
                "position": "sack_item_2"
            }
        ]),
    );
    let direct_engine = direct.engine(7).expect("direct chance probe");
    assert_eq!(
        runner_item_ids(&direct_engine),
        ["ecology:gallery_pack:runner:0:loot:gallery_pack_loot:succeeds_at_thirty_one"]
    );

    let failed_group = loot_probe_parts(
        "ordinary",
        Some(2),
        json!([{
            "kind": "item_choice",
            "id": "choice",
            "chance_numerator": 30,
            "chance_denominator": 100,
            "members": [
                {
                    "member_id": "knife",
                    "item_definition_id": "rusted_knife",
                    "quantity": 1,
                    "position": "belt_1"
                },
                {
                    "member_id": "cloth",
                    "item_definition_id": "cloth_bundle",
                    "quantity": 1,
                    "position": "belt_1"
                }
            ]
        }]),
    );
    assert!(runner_item_ids(&failed_group.engine(7).expect("failed group probe")).is_empty());

    let successful_group = loot_probe_parts(
        "ordinary",
        Some(2),
        json!([{
            "kind": "item_choice",
            "id": "choice",
            "chance_numerator": 31,
            "chance_denominator": 100,
            "members": [
                {
                    "member_id": "knife",
                    "item_definition_id": "rusted_knife",
                    "quantity": 1,
                    "position": "belt_1"
                },
                {
                    "member_id": "cloth",
                    "item_definition_id": "cloth_bundle",
                    "quantity": 1,
                    "position": "belt_1"
                }
            ]
        }]),
    );
    assert_eq!(
        runner_item_ids(&successful_group.engine(7).expect("successful group probe")),
        ["ecology:gallery_pack:runner:0:loot:gallery_pack_loot:choice:cloth"]
    );

    let guaranteed_group = loot_probe_parts(
        "ordinary",
        Some(2),
        json!([{
            "kind": "item_choice",
            "id": "choice",
            "chance_numerator": 1,
            "chance_denominator": 1,
            "members": [
                {
                    "member_id": "knife",
                    "item_definition_id": "rusted_knife",
                    "quantity": 1,
                    "position": "belt_1"
                },
                {
                    "member_id": "cloth",
                    "item_definition_id": "cloth_bundle",
                    "quantity": 1,
                    "position": "belt_1"
                }
            ]
        }]),
    );
    assert_eq!(
        runner_item_ids(&guaranteed_group.engine(7).expect("first uniform member")),
        ["ecology:gallery_pack:runner:0:loot:gallery_pack_loot:choice:knife"]
    );
    assert_eq!(
        runner_item_ids(&guaranteed_group.engine(8).expect("second uniform member")),
        ["ecology:gallery_pack:runner:0:loot:gallery_pack_loot:choice:cloth"]
    );

    let ranged_gold = loot_probe_parts(
        "ordinary",
        Some(2),
        json!([{
            "kind": "gold",
            "id": "coins",
            "chance_numerator": 1,
            "chance_denominator": 1,
            "minimum_amount": 2,
            "maximum_amount": 3,
            "position": "sack"
        }]),
    );
    for (seed, expected) in [(7, 2), (8, 3)] {
        let engine = ranged_gold.engine(seed).expect("ranged-gold boundary");
        let runner = engine
            .world()
            .actors
            .iter()
            .find(|actor| actor.id == "ecology:gallery_pack:runner:0")
            .expect("runner");
        assert_eq!(runner.carried.gold.sack, expected, "seed {seed}");
    }

    let overflow = loot_probe_parts(
        "ordinary",
        Some(2),
        json!([
            {
                "kind": "item",
                "id": "first",
                "chance_numerator": 1,
                "chance_denominator": 1,
                "item_definition_id": "flint",
                "quantity": 1,
                "position": "sack_item_1"
            },
            {
                "kind": "item_choice",
                "id": "choice",
                "chance_numerator": 1,
                "chance_denominator": 1,
                "members": [
                    {
                        "member_id": "knife",
                        "item_definition_id": "rusted_knife",
                        "quantity": 1,
                        "position": "belt_1"
                    },
                    {
                        "member_id": "cloth",
                        "item_definition_id": "cloth_bundle",
                        "quantity": 1,
                        "position": "belt_1"
                    }
                ]
            },
            {
                "kind": "item",
                "id": "overflow",
                "chance_numerator": 1,
                "chance_denominator": 2,
                "item_definition_id": "oak_club",
                "quantity": 1,
                "position": "belt_back"
            },
            {
                "kind": "gold",
                "id": "coins",
                "chance_numerator": 1,
                "chance_denominator": 1,
                "minimum_amount": 2,
                "maximum_amount": 3,
                "position": "sack"
            }
        ]),
    );
    let first = overflow.engine(8).expect("overflow probe");
    let second = overflow.engine(8).expect("overflow replay");
    assert_eq!(first.snapshot(), second.snapshot());
    assert_eq!(
        first.export_checkpoint().expect("first checkpoint"),
        second.export_checkpoint().expect("second checkpoint")
    );
    assert_eq!(
        runner_item_ids(&first),
        [
            "ecology:gallery_pack:runner:0:loot:gallery_pack_loot:choice:cloth",
            "ecology:gallery_pack:runner:0:loot:gallery_pack_loot:first",
        ]
    );
    assert!(
        !first
            .world()
            .item_instances
            .contains_key("ecology:gallery_pack:runner:0:loot:gallery_pack_loot:overflow"),
        "overflow creates no item instance"
    );
    let runner = first
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "ecology:gallery_pack:runner:0")
        .expect("runner");
    assert_eq!(
        runner.carried.gold.sack, 3,
        "gold survives ordinary overflow"
    );
    assert_eq!(
        checkpoint_rng_state(&first),
        "42681",
        "choice, post-cap chance, and ranged-gold amount each consumed one transition"
    );
}

#[test]
fn signature_loot_bypasses_ordinary_truncation_without_a_parallel_owner() {
    let signature = loot_probe_parts(
        "signature",
        None,
        json!([
            {
                "kind": "item",
                "id": "first",
                "chance_numerator": 1,
                "chance_denominator": 1,
                "item_definition_id": "flint",
                "quantity": 1,
                "position": "sack_item_1"
            },
            {
                "kind": "item",
                "id": "second",
                "chance_numerator": 1,
                "chance_denominator": 1,
                "item_definition_id": "cloth_bundle",
                "quantity": 1,
                "position": "sack_item_2"
            },
            {
                "kind": "item",
                "id": "third",
                "chance_numerator": 1,
                "chance_denominator": 1,
                "item_definition_id": "rusted_knife",
                "quantity": 1,
                "position": "belt_1"
            }
        ]),
    );
    assert_eq!(
        runner_item_ids(&signature.engine(7).expect("signature probe")),
        [
            "ecology:gallery_pack:runner:0:loot:gallery_pack_loot:third",
            "ecology:gallery_pack:runner:0:loot:gallery_pack_loot:first",
            "ecology:gallery_pack:runner:0:loot:gallery_pack_loot:second",
        ]
    );
}

#[test]
fn slot_replenishment_preserves_partial_and_full_clear_state_across_checkpoints() {
    let mut parts = parts_with_remote_level();
    parts.selected_mut("spawn_groups", 0)["reset"] = json!({
        "policy": "slot_replenishment",
        "slot_delay_units": 1,
        "full_clear_delay_units": 3
    });
    parts.world_seed["actors"][0]["carried"]["items"] = json!([]);
    parts.world_seed["item_instances"] = json!({});
    let mut engine = parts.engine(7).expect("slot-replenishment engine");
    engine = checkpoint_round_trip(&engine);

    let partial = engine
        .apply_actor_intent(
            &ActorId::from("player"),
            attack("ecology:gallery_pack:runner:0"),
        )
        .expect("partial vacancy");
    assert!(partial.events.iter().any(|event| matches!(
        event,
        Event::EcologyResetScheduled {
            site_id,
            member_ids,
            policy: EcologyLifecyclePolicyV1::SlotReplenishment,
            ..
        } if site_id == "gallery_pack" && member_ids == &["runner".to_string()]
    )));
    let site = &engine.world().ecology_sites["gallery_pack"];
    assert!(site.member_slots["runner"].actor_id.is_none());
    assert!(site.member_slots["runner"].due_at.is_some());
    assert!(site.member_slots["keeper"].actor_id.is_some());
    assert!(site.full_clear_due_at.is_none());
    engine = checkpoint_round_trip(&engine);

    let observed = engine
        .apply_actor_intent(&ActorId::from("player"), PlayerIntent::Wait)
        .expect("observed due boundary");
    assert!(!observed.events.iter().any(|event| matches!(
        event,
        Event::EcologyActorSpawned { site_id, .. } if site_id == "gallery_pack"
    )));
    let retained_due = engine.world().ecology_sites["gallery_pack"].member_slots["runner"].due_at;
    engine = checkpoint_round_trip(&engine);

    move_player_to_level(&mut engine, "remote");
    let replenished = engine
        .apply_actor_intent(&ActorId::from("player"), PlayerIntent::Wait)
        .expect("unobserved partial replenishment");
    assert!(replenished.events.iter().any(|event| matches!(
        event,
        Event::EcologyReset {
            site_id,
            from_generation: 0,
            to_generation: 1,
            member_ids,
            policy: EcologyLifecyclePolicyV1::SlotReplenishment,
        } if site_id == "gallery_pack" && member_ids == &["runner".to_string()]
    )));
    let site = &engine.world().ecology_sites["gallery_pack"];
    assert_eq!(
        site.member_slots["runner"].actor_id.as_deref(),
        Some("ecology:gallery_pack:runner:1")
    );
    assert!(site.member_slots["runner"].due_at.is_none());
    assert!(retained_due.is_some());

    move_player_to_level(&mut engine, "room_0");
    engine
        .apply_actor_intent(
            &ActorId::from("player"),
            attack("ecology:gallery_pack:runner:1"),
        )
        .expect("second runner defeat");
    let full_clear = engine
        .apply_actor_intent(
            &ActorId::from("player"),
            attack("ecology:gallery_pack:keeper:0"),
        )
        .expect("full clear supersedes partial due");
    assert!(
        full_clear.events.iter().any(|event| matches!(
            event,
            Event::EcologyResetScheduled {
                site_id,
                member_ids,
                policy: EcologyLifecyclePolicyV1::SlotReplenishment,
                ..
            } if site_id == "gallery_pack"
                && member_ids == &["runner".to_string(), "keeper".to_string()]
        )),
        "unexpected full-clear events: {:?}",
        full_clear.events
    );
    let site = &engine.world().ecology_sites["gallery_pack"];
    assert!(site.full_clear_due_at.is_some());
    assert!(
        site.member_slots
            .values()
            .all(|slot| slot.actor_id.is_none())
    );
    assert!(site.member_slots.values().all(|slot| slot.due_at.is_none()));
    engine = checkpoint_round_trip(&engine);

    for _ in 0..3 {
        let deferred = engine
            .apply_actor_intent(&ActorId::from("player"), PlayerIntent::Wait)
            .expect("observed full-clear boundary");
        assert!(!deferred.events.iter().any(|event| matches!(
            event,
            Event::EcologyActorSpawned { site_id, .. } if site_id == "gallery_pack"
        )));
    }
    engine = checkpoint_round_trip(&engine);
    move_player_to_level(&mut engine, "remote");
    let reset = engine
        .apply_actor_intent(&ActorId::from("player"), PlayerIntent::Wait)
        .expect("unobserved full-clear materialization");
    assert_eq!(
        reset
            .events
            .iter()
            .filter(|event| matches!(
                event,
                Event::EcologyActorSpawned { site_id, .. } if site_id == "gallery_pack"
            ))
            .count(),
        2
    );
    assert_eq!(engine.world().ecology_sites["gallery_pack"].generation, 2);
    checkpoint_round_trip(&engine);
}

#[test]
fn full_site_materializes_all_due_members_in_stable_order() {
    let mut parts = parts_with_remote_level();
    let extra_member = json!({
        "member_id": "extra",
        "actor_definition_id": "actor/creature_ecology_gallery/pack_b",
        "loot_table_id": null
    });
    parts.selected_mut("spawn_groups", 0)["members"]
        .as_array_mut()
        .unwrap()
        .push(extra_member);
    parts.world_seed["ecology_sites"][0]["member_locations"]["extra"] =
        parts.world_seed["ecology_sites"][0]["member_locations"]["keeper"].clone();
    let mut engine = parts.engine(7).expect("three-member full-site engine");
    for member in ["runner", "keeper", "extra"] {
        let id = format!("ecology:gallery_pack:{member}:0");
        // Combat can miss. Establish the full-clear precondition explicitly.
        for _ in 0..10 {
            if !engine
                .world()
                .actors
                .iter()
                .any(|actor| actor.id == id && actor.is_alive())
            {
                break;
            }
            engine
                .apply_actor_intent(
                    &ActorId::from("player"),
                    attack(&format!("ecology:gallery_pack:{member}:0")),
                )
                .unwrap_or_else(|error| panic!("{member} defeat: {error}"));
        }
        assert!(
            !engine
                .world()
                .actors
                .iter()
                .any(|actor| actor.id == id && actor.is_alive())
        );
    }

    move_player_to_level(&mut engine, "remote");
    let first = engine
        .apply_actor_intent(&ActorId::from("player"), PlayerIntent::Wait)
        .expect("first capped materialization");
    let first_members = first
        .events
        .iter()
        .filter_map(|event| match event {
            Event::EcologyActorSpawned {
                site_id, member_id, ..
            } if site_id == "gallery_pack" => Some(member_id.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(first_members, ["runner", "keeper", "extra"]);
    let site = &engine.world().ecology_sites["gallery_pack"];
    assert_eq!(site.generation, 1);
    assert!(site.member_slots["extra"].actor_id.is_some());
    assert!(site.member_slots["extra"].due_at.is_none());
    engine = checkpoint_round_trip(&engine);

    let second = engine
        .apply_actor_intent(&ActorId::from("player"), PlayerIntent::Wait)
        .expect("second capped materialization");
    let second_members = second
        .events
        .iter()
        .filter_map(|event| match event {
            Event::EcologyActorSpawned {
                site_id,
                member_id,
                generation,
                ..
            } if site_id == "gallery_pack" => Some((member_id.clone(), *generation)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(second_members.is_empty());
    assert_eq!(engine.world().ecology_sites["gallery_pack"].generation, 1);
    checkpoint_round_trip(&engine);
}
