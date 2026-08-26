use crate::support::content_parts::ContentParts;
use tme_rules::{
    ActorId, AutomaticActorDecisionV1, AutomaticMovementPurposeV1, Coord, Direction,
    EcologyLifecyclePolicyV1, Engine, Event, LogicalTime, PhysicalAttackMode, PlayerIntent,
    RulesOutcomeV1, WorldPosition,
};

fn first_land_engine() -> Engine {
    ContentParts::tracked("first_land_structure", "profile/first_land_structure")
        .engine(7)
        .expect("seed-seven first-land engine")
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

fn suppress_automatic_actors(engine: &mut Engine) {
    for actor in &mut engine.world_mut().actors {
        if actor.id != "player" {
            actor.timing.ready_at = LogicalTime::new(u64::MAX);
            actor.attack_ready_at = LogicalTime::new(u64::MAX);
        }
    }
}

fn force_defeat(engine: &mut Engine, target_id: &str) -> RulesOutcomeV1 {
    suppress_automatic_actors(engine);
    let target_location = engine
        .world()
        .actor(&ActorId::from(target_id))
        .unwrap_or_else(|| panic!("target {target_id}"))
        .location
        .clone();
    {
        let now = engine.world().timing.now;
        let target = engine
            .world_mut()
            .actors
            .iter_mut()
            .find(|actor| actor.id == target_id)
            .unwrap_or_else(|| panic!("mutable target {target_id}"));
        target.hp = 1;
        target.stats.defense = -100;
        target.timing.ready_at = LogicalTime::new(u64::MAX);
        target.attack_ready_at = LogicalTime::new(u64::MAX);

        let player = engine
            .world_mut()
            .actors
            .iter_mut()
            .find(|actor| actor.id == "player")
            .expect("first-land player");
        player.location = target_location;
        player.stats.attack = 100;
        player.timing.ready_at = now;
        player.attack_ready_at = now;
    }
    let outcome = engine
        .apply_actor_intent(
            &ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: PhysicalAttackMode::Fight,
                target_actor_id: target_id.into(),
            },
        )
        .unwrap_or_else(|error| panic!("force defeat {target_id}: {error}"));
    assert!(
        engine
            .world()
            .actor(&ActorId::from(target_id))
            .is_some_and(|actor| !actor.is_alive()),
        "{target_id} remained alive"
    );
    outcome
}

fn move_player_off_level(engine: &mut Engine, observed_level: &str) {
    let (level, position) = if observed_level == "surface" {
        ("upper_halls", Coord { x: 3, y: 9 })
    } else {
        ("surface", Coord { x: 4, y: 41 })
    };
    engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == "player")
        .expect("first-land player")
        .location = WorldPosition::new("testland", level, position);
}

fn advance_to_boundary(engine: &mut Engine, boundary: LogicalTime) -> RulesOutcomeV1 {
    assert!(boundary > LogicalTime::ZERO);
    suppress_automatic_actors(engine);
    let before = LogicalTime::new(boundary.value() - 1);
    engine.world_mut().timing.now = before;
    let player = engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == "player")
        .expect("first-land player");
    player.timing.ready_at = before;
    player.attack_ready_at = before;
    engine
        .apply_actor_intent(&ActorId::from("player"), PlayerIntent::Wait)
        .expect("advance to selected ecology boundary")
}

fn controlled_action_time(outcome: &RulesOutcomeV1) -> LogicalTime {
    outcome
        .events
        .iter()
        .find_map(|event| match event {
            Event::ActorReady {
                actor_id,
                logical_time,
                ..
            } if actor_id == "player" => Some(*logical_time),
            _ => None,
        })
        .expect("controlled action time")
}

fn wait_for_automatic_decision(
    engine: &mut Engine,
    player_id: &ActorId,
    actor_id: &ActorId,
) -> (usize, RulesOutcomeV1) {
    for wait_count in 1..=64 {
        let outcome = engine
            .apply_actor_intent(player_id, PlayerIntent::Wait)
            .expect("advance to automatic decision");
        if outcome.events.iter().any(|event| {
            matches!(
                event,
                Event::AutomaticActorDecision {
                    actor_id: decision_actor_id,
                    ..
                } if decision_actor_id == actor_id
            )
        }) {
            return (wait_count, outcome);
        }
    }
    panic!("automatic actor {actor_id} did not act within 64 player waits");
}

#[test]
fn gate_two_first_land_population_and_actual_loot_results_are_exact() {
    let first_land =
        || ContentParts::tracked("first_land_structure", "profile/first_land_structure");
    let seed_seven = first_land()
        .engine(7)
        .expect("seed-seven first-land engine");
    let ecology_actors = seed_seven
        .world()
        .actors
        .iter()
        .filter(|actor| actor.ecology_origin.is_some())
        .collect::<Vec<_>>();
    let starting_item_count = ecology_actors
        .iter()
        .map(|actor| actor.carried.items.len())
        .sum::<usize>();
    let starting_gold = ecology_actors
        .iter()
        .map(|actor| actor.carried.gold.sack)
        .sum::<i64>();
    let starting_yield_by_level = ecology_actors.iter().fold(
        std::collections::BTreeMap::<&str, (usize, i64)>::new(),
        |mut totals, actor| {
            let total = totals.entry(actor.location.level.as_str()).or_default();
            total.0 += actor.carried.items.len();
            total.1 += actor.carried.gold.sack;
            totals
        },
    );
    assert_eq!(starting_item_count, 61);
    assert_eq!(starting_gold, 418);
    assert_eq!(
        starting_yield_by_level,
        [
            ("lake_level", (9, 68)),
            ("lower_halls", (10, 22)),
            ("old_temple", (16, 222)),
            ("surface", (14, 53)),
            ("upper_halls", (12, 53)),
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(seed_seven.world().ecology_sites.len(), 38);
    assert_eq!(ecology_actors.len(), 92);
    let level_population = ecology_actors.iter().fold(
        std::collections::BTreeMap::<&str, usize>::new(),
        |mut counts, actor| {
            *counts.entry(actor.location.level.as_str()).or_default() += 1;
            counts
        },
    );
    assert_eq!(
        level_population,
        [
            ("lake_level", 16),
            ("lower_halls", 19),
            ("old_temple", 20),
            ("surface", 19),
            ("upper_halls", 18),
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        seed_seven
            .initial_events()
            .into_iter()
            .filter_map(|event| match event {
                Event::EcologyActorSpawned {
                    site_id,
                    member_id,
                    generation,
                    actor_id,
                    location,
                    ..
                } => Some((
                    site_id,
                    member_id,
                    generation,
                    actor_id.to_string(),
                    location.position,
                )),
                _ => None,
            })
            .filter(|(site_id, ..)| {
                matches!(
                    site_id.as_str(),
                    "upper_halls_foragers" | "upper_halls_raiders" | "upper_halls_watch"
                )
            })
            .collect::<Vec<_>>(),
        [
            (
                "upper_halls_foragers".to_string(),
                "forager_a".to_string(),
                0,
                "ecology:upper_halls_foragers:forager_a:0".to_string(),
                Coord { x: 28, y: 6 },
            ),
            (
                "upper_halls_foragers".to_string(),
                "forager_b".to_string(),
                0,
                "ecology:upper_halls_foragers:forager_b:0".to_string(),
                Coord { x: 31, y: 6 },
            ),
            (
                "upper_halls_foragers".to_string(),
                "forager_c".to_string(),
                0,
                "ecology:upper_halls_foragers:forager_c:0".to_string(),
                Coord { x: 31, y: 5 },
            ),
            (
                "upper_halls_raiders".to_string(),
                "raider_a".to_string(),
                0,
                "ecology:upper_halls_raiders:raider_a:0".to_string(),
                Coord { x: 18, y: 23 },
            ),
            (
                "upper_halls_raiders".to_string(),
                "raider_b".to_string(),
                0,
                "ecology:upper_halls_raiders:raider_b:0".to_string(),
                Coord { x: 21, y: 23 },
            ),
            (
                "upper_halls_raiders".to_string(),
                "forager".to_string(),
                0,
                "ecology:upper_halls_raiders:forager:0".to_string(),
                Coord { x: 21, y: 24 },
            ),
            (
                "upper_halls_watch".to_string(),
                "watch_a".to_string(),
                0,
                "ecology:upper_halls_watch:watch_a:0".to_string(),
                Coord { x: 39, y: 14 },
            ),
            (
                "upper_halls_watch".to_string(),
                "watch_b".to_string(),
                0,
                "ecology:upper_halls_watch:watch_b:0".to_string(),
                Coord { x: 44, y: 15 },
            ),
        ]
    );
    assert!(
        ecology_actors
            .iter()
            .all(|actor| actor.carried.items.len() <= 2),
        "every actual Gate Two starting inventory stays bounded"
    );
    for (actor_id, expected_item_suffix, gold_range) in [
        (
            "ecology:surface_great_bear:great_bear:0",
            ":great_bear_hide",
            0..=0,
        ),
        (
            "ecology:surface_east_portal_magus:magus:0",
            ":portal_glass_focus",
            20..=40,
        ),
        (
            "ecology:upper_halls_hidden_lair:lair_troll:0",
            ":holdfast_maul",
            25..=50,
        ),
        (
            "ecology:old_temple_summoning_lair:dragon:0",
            ":cinderheart_scale",
            50..=100,
        ),
        (
            "ecology:old_temple_crypt_lair:crypt_wraith:0",
            ":ashwake_hammer",
            30..=70,
        ),
    ] {
        let actor = seed_seven
            .world()
            .actor(&ActorId::from(actor_id))
            .unwrap_or_else(|| panic!("signature actor {actor_id}"));
        assert!(
            actor
                .carried
                .items
                .values()
                .any(|item_id| item_id.ends_with(expected_item_suffix)),
            "{actor_id} omitted {expected_item_suffix}"
        );
        assert!(gold_range.contains(&actor.carried.gold.sack));
    }

    let inventory_snapshot = |engine: &Engine| {
        engine
            .world()
            .actors
            .iter()
            .filter(|actor| actor.ecology_origin.is_some())
            .map(|actor| {
                (
                    actor.id.clone(),
                    actor.carried.items.values().cloned().collect::<Vec<_>>(),
                    actor.carried.gold.sack,
                )
            })
            .collect::<Vec<_>>()
    };
    let repeated_seed_seven = first_land()
        .engine(7)
        .expect("repeated seed-seven first-land engine");
    assert_eq!(
        inventory_snapshot(&seed_seven),
        inventory_snapshot(&repeated_seed_seven)
    );
}

#[test]
fn gate_one_first_land_forager_notices_remembers_attacks_and_returns_home() {
    let first_land =
        || ContentParts::tracked("first_land_structure", "profile/first_land_structure");
    let target_id = ActorId::from("ecology:upper_halls_foragers:forager_b:0");

    let mut chase = first_land().engine(7).expect("first-land chase engine");
    chase
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == "player")
        .expect("first-land player")
        .location = WorldPosition::new("testland", "upper_halls", Coord { x: 29, y: 6 });
    let notice = chase
        .apply_actor_intent(&ActorId::from("player"), PlayerIntent::Wait)
        .expect("notice opportunity");
    assert!(notice.events.iter().any(|event| matches!(
        event,
        Event::AutomaticActorDecision {
            actor_id,
            decision: AutomaticActorDecisionV1::Move {
                direction: Direction::West,
                purpose: AutomaticMovementPurposeV1::Chase,
            },
            ..
        } if actor_id == &target_id
    )));
    let target = chase.world().actor(&target_id).expect("noticed forager");
    assert_eq!(target.location.position, Coord { x: 30, y: 6 });
    let remembered = target
        .ai
        .as_ref()
        .and_then(|ai| ai.awareness.remembered.as_ref())
        .expect("visible player is remembered");
    assert_eq!(remembered.actor_id, "player");
    assert_eq!(remembered.remaining_opportunities, 2);

    chase
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == "player")
        .expect("first-land player")
        .location = WorldPosition::new("testland", "upper_halls", Coord { x: 21, y: 5 });
    chase
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == target_id)
        .expect("noticed forager")
        .location = WorldPosition::new("testland", "upper_halls", Coord { x: 22, y: 5 });
    let (_, leash) = wait_for_automatic_decision(&mut chase, &ActorId::from("player"), &target_id);
    assert!(leash.events.iter().any(|event| matches!(
        event,
        Event::AutomaticActorDecision {
            actor_id,
            decision: AutomaticActorDecisionV1::Move {
                purpose: AutomaticMovementPurposeV1::ReturnHome,
                ..
            },
            ..
        } if actor_id == &target_id
    )));
    let target = chase.world().actor(&target_id).expect("returning forager");
    assert_ne!(target.location, target.home_location);
    assert!(target.ai.as_ref().expect("forager AI").returning_home);
    assert!(
        chase
            .world()
            .actor(&target_id)
            .expect("returning forager")
            .ai
            .as_ref()
            .expect("forager AI")
            .awareness
            .remembered
            .is_none(),
        "leash break clears pursuit memory"
    );

    let mut attack_engine = first_land().engine(7).expect("first-land attack engine");
    attack_engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == "player")
        .expect("first-land player")
        .location = WorldPosition::new("testland", "upper_halls", Coord { x: 31, y: 6 });
    let attacked = attack_engine
        .apply_actor_intent(&ActorId::from("player"), PlayerIntent::Wait)
        .expect("same-square attack opportunity");
    assert!(attacked.events.iter().any(|event| matches!(
        event,
        Event::AutomaticActorDecision {
            actor_id,
            decision: AutomaticActorDecisionV1::PhysicalAttack {
                target_id: attack_target_id,
                mode: PhysicalAttackMode::Fight,
                ..
            },
            ..
        } if actor_id == &target_id && attack_target_id == "player"
    )));
    assert!(
        attacked.events.iter().any(|event| {
            matches!(
                event,
                Event::Attacked {
                    attacker_id,
                    defender_id,
                    mode: PhysicalAttackMode::Fight,
                    ..
                } if attacker_id == &target_id && defender_id == "player"
            ) || matches!(
                event,
                Event::AttackMissed {
                    attacker_id,
                    defender_id,
                    mode: PhysicalAttackMode::Fight,
                    ..
                } if attacker_id == &target_id && defender_id == "player"
            )
        }),
        "unexpected attack events: {:?}",
        attacked.events
    );
}

#[test]
fn gate_one_leash_return_is_multi_boundary_on_original_testland_topology() {
    let mut engine = ContentParts::tracked("first_land_structure", "profile/first_land_structure")
        .engine(7)
        .expect("seed-seven first-land leash engine");
    let player_id = ActorId::from("player");
    let watch_id = ActorId::from("ecology:upper_halls_watch:watch_a:0");
    let home = engine
        .world()
        .actor(&watch_id)
        .expect("Upper Halls watch skeleton")
        .home_location
        .clone();
    assert_eq!(home.position, Coord { x: 39, y: 14 });

    engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == player_id)
        .expect("first-land player")
        .location = WorldPosition::new("testland", "upper_halls", Coord { x: 32, y: 14 });
    engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == watch_id)
        .expect("Upper Halls watch skeleton")
        .location = WorldPosition::new("testland", "upper_halls", Coord { x: 35, y: 14 });
    engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == watch_id)
        .and_then(|actor| actor.ai.as_mut())
        .expect("Upper Halls watch skeleton AI")
        .leash_range = 2;

    let (_, first_break) = wait_for_automatic_decision(&mut engine, &player_id, &watch_id);
    assert!(
        first_break.events.iter().any(|event| matches!(
            event,
            Event::AutomaticActorDecision {
                actor_id,
                decision: AutomaticActorDecisionV1::Move {
                    purpose: AutomaticMovementPurposeV1::ReturnHome,
                    ..
                },
                ..
            } if actor_id == &watch_id
        )),
        "unexpected first-break events: {:?}",
        first_break.events
    );
    let watch = engine.world().actor(&watch_id).expect("returning skeleton");
    assert!(watch.ai.as_ref().expect("skeleton AI").returning_home);
    assert!(
        watch
            .ai
            .as_ref()
            .expect("skeleton AI")
            .awareness
            .remembered
            .is_none()
    );

    engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == player_id)
        .expect("first-land player")
        .location = WorldPosition::new("testland", "surface", Coord { x: 4, y: 41 });
    let mut return_moves = 1;
    for _ in 0..10 {
        let (_, outcome) = wait_for_automatic_decision(&mut engine, &player_id, &watch_id);
        if outcome.events.iter().any(|event| {
            matches!(
                event,
                Event::AutomaticActorDecision {
                    actor_id,
                    decision: AutomaticActorDecisionV1::Move {
                        purpose: AutomaticMovementPurposeV1::ReturnHome,
                        ..
                    },
                    ..
                } if actor_id == &watch_id
            )
        }) {
            return_moves += 1;
            continue;
        }
        break;
    }
    assert!(return_moves > 1);
    let watch = engine.world().actor(&watch_id).expect("home skeleton");
    assert_eq!(watch.location, home);
    assert!(!watch.ai.as_ref().expect("skeleton AI").returning_home);
}

#[test]
fn gate_two_tracked_partial_replenishment_is_exact_deferred_and_checkpoint_stable() {
    let mut engine = first_land_engine();
    let site_id = "upper_halls_foragers";
    let member_id = "forager_a";
    let actor_id = "ecology:upper_halls_foragers:forager_a:0";
    let corpse_count_before = engine.world().corpses.len();
    let defeated = force_defeat(&mut engine, actor_id);
    let due_at = defeated
        .events
        .iter()
        .find_map(|event| match event {
            Event::EcologyResetScheduled {
                site_id: scheduled_site,
                member_ids,
                due_at,
                policy: EcologyLifecyclePolicyV1::SlotReplenishment,
                ..
            } if scheduled_site == site_id && member_ids == &[member_id.to_string()] => {
                Some(*due_at)
            }
            _ => None,
        })
        .expect("tracked partial replenishment schedule");
    assert_eq!(
        due_at,
        controlled_action_time(&defeated).saturating_add_rounds(60)
    );
    assert_eq!(engine.world().corpses.len(), corpse_count_before + 1);
    assert_eq!(
        engine.world().ecology_sites[site_id].member_slots[member_id].due_at,
        Some(due_at)
    );
    assert!(
        engine.world().ecology_sites[site_id]
            .member_slots
            .values()
            .filter(|slot| slot.actor_id.is_some())
            .count()
            == 2
    );
    engine = checkpoint_round_trip(&engine);

    let observed = advance_to_boundary(&mut engine, due_at);
    assert!(!observed.events.iter().any(|event| matches!(
        event,
        Event::EcologyActorSpawned { site_id: spawned_site, .. } if spawned_site == site_id
    )));
    assert_eq!(
        engine.world().ecology_sites[site_id].member_slots[member_id].due_at,
        Some(due_at),
        "observed partial due time changed"
    );
    engine = checkpoint_round_trip(&engine);

    move_player_off_level(&mut engine, "upper_halls");
    let reset = advance_to_boundary(&mut engine, LogicalTime::new(due_at.value() + 1));
    assert!(reset.events.iter().any(|event| matches!(
        event,
        Event::EcologyReset {
            site_id: reset_site,
            from_generation: 0,
            to_generation: 1,
            member_ids,
            policy: EcologyLifecyclePolicyV1::SlotReplenishment,
        } if reset_site == site_id && member_ids == &[member_id.to_string()]
    )));
    assert_eq!(
        engine.world().ecology_sites[site_id].member_slots[member_id]
            .actor_id
            .as_deref(),
        Some("ecology:upper_halls_foragers:forager_a:1")
    );
    assert_eq!(engine.world().corpses.len(), corpse_count_before + 1);
    checkpoint_round_trip(&engine);
}

#[test]
fn gate_two_tracked_full_clears_use_180_units_and_cap_each_simultaneous_site() {
    let mut engine = first_land_engine();
    let sites = [
        (
            "upper_halls_foragers",
            ["forager_a", "forager_b", "forager_c"],
        ),
        ("upper_halls_raiders", ["raider_a", "raider_b", "forager"]),
    ];
    let mut due_values = Vec::new();
    for (site_id, member_ids) in &sites {
        let mut final_outcome = None;
        for member_id in member_ids {
            final_outcome = Some(force_defeat(
                &mut engine,
                &format!("ecology:{site_id}:{member_id}:0"),
            ));
        }
        let outcome = final_outcome.expect("three-member full clear");
        let due_at = outcome
            .events
            .iter()
            .find_map(|event| match event {
                Event::EcologyResetScheduled {
                    site_id: scheduled_site,
                    member_ids: scheduled_members,
                    due_at,
                    policy: EcologyLifecyclePolicyV1::SlotReplenishment,
                    ..
                } if scheduled_site == site_id
                    && scheduled_members == &member_ids.map(str::to_string) =>
                {
                    Some(*due_at)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("full-clear schedule for {site_id}"));
        assert_eq!(
            due_at,
            controlled_action_time(&outcome).saturating_add_rounds(180)
        );
        assert_eq!(
            engine.world().ecology_sites[*site_id].full_clear_due_at,
            Some(due_at)
        );
        due_values.push(due_at);
    }
    assert_eq!(engine.world().corpses.len(), 6);
    engine = checkpoint_round_trip(&engine);
    move_player_off_level(&mut engine, "upper_halls");

    let boundary = *due_values.iter().max().expect("full-clear due values");
    let first = advance_to_boundary(&mut engine, boundary);
    for (site_id, member_ids) in &sites {
        let spawned = first
            .events
            .iter()
            .filter_map(|event| match event {
                Event::EcologyActorSpawned {
                    site_id: spawned_site,
                    member_id,
                    ..
                } if spawned_site == site_id => Some(member_id.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(spawned, member_ids[..2]);
        assert!(
            engine.world().ecology_sites[*site_id].member_slots[member_ids[2]]
                .due_at
                .is_some()
        );
    }
    assert_eq!(engine.world().corpses.len(), 6);
    engine = checkpoint_round_trip(&engine);

    let second = advance_to_boundary(&mut engine, LogicalTime::new(boundary.value() + 1));
    for (site_id, member_ids) in &sites {
        assert!(second.events.iter().any(|event| matches!(
            event,
            Event::EcologyActorSpawned {
                site_id: spawned_site,
                member_id,
                ..
            } if spawned_site == site_id && member_id == member_ids[2]
        )));
        assert!(
            engine.world().ecology_sites[*site_id]
                .member_slots
                .values()
                .all(|slot| slot.actor_id.is_some() && slot.due_at.is_none())
        );
    }
    assert_eq!(engine.world().corpses.len(), 6);
    checkpoint_round_trip(&engine);
}

#[test]
fn gate_two_all_five_signature_resets_defer_and_refresh_guaranteed_loot() {
    for (site_id, member_id, observed_level, delay, guaranteed_suffix) in [
        (
            "surface_great_bear",
            "great_bear",
            "surface",
            450,
            ":great_bear_hide",
        ),
        (
            "surface_east_portal_magus",
            "magus",
            "surface",
            450,
            ":portal_glass_focus",
        ),
        (
            "upper_halls_hidden_lair",
            "lair_troll",
            "upper_halls",
            900,
            ":holdfast_maul",
        ),
        (
            "old_temple_summoning_lair",
            "dragon",
            "old_temple",
            900,
            ":cinderheart_scale",
        ),
        (
            "old_temple_crypt_lair",
            "crypt_wraith",
            "old_temple",
            900,
            ":ashwake_hammer",
        ),
    ] {
        let mut engine = first_land_engine();
        let generation_zero_id = format!("ecology:{site_id}:{member_id}:0");
        let generation_zero_items = engine
            .world()
            .actor(&ActorId::from(generation_zero_id.as_str()))
            .unwrap_or_else(|| panic!("signature actor {generation_zero_id}"))
            .carried
            .items
            .values()
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            generation_zero_items
                .iter()
                .any(|item_id| item_id.ends_with(guaranteed_suffix))
        );
        let site_member_ids = engine.world().ecology_sites[site_id]
            .member_slots
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let mut final_defeat = None;
        for site_member_id in &site_member_ids {
            final_defeat = Some(force_defeat(
                &mut engine,
                &format!("ecology:{site_id}:{site_member_id}:0"),
            ));
        }
        let defeated = final_defeat.expect("signature full clear");
        let due_at = defeated
            .events
            .iter()
            .find_map(|event| match event {
                Event::EcologyResetScheduled {
                    site_id: scheduled_site,
                    member_ids,
                    due_at,
                    policy: EcologyLifecyclePolicyV1::FullSite,
                    ..
                } if scheduled_site == site_id
                    && member_ids.len() == site_member_ids.len()
                    && member_ids
                        .iter()
                        .all(|member| site_member_ids.contains(member)) =>
                {
                    Some(*due_at)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("signature schedule for {site_id}"));
        assert_eq!(
            due_at,
            controlled_action_time(&defeated).saturating_add_rounds(delay)
        );
        let corpse_count = engine.world().corpses.len();
        assert!(
            generation_zero_items.iter().all(|item_id| {
                engine
                    .world()
                    .corpses
                    .values()
                    .any(|corpse| corpse.contents.values().any(|value| value == item_id))
                    || engine
                        .world()
                        .ground_items
                        .iter()
                        .any(|item| item.item_instance_id == *item_id)
            }),
            "signature remains did not retain generation-zero loot for {site_id}"
        );
        engine = checkpoint_round_trip(&engine);

        let observed = advance_to_boundary(&mut engine, due_at);
        assert!(!observed.events.iter().any(|event| matches!(
            event,
            Event::EcologyActorSpawned { site_id: spawned_site, .. } if spawned_site == site_id
        )));
        assert_eq!(
            engine.world().ecology_sites[site_id].full_clear_due_at,
            Some(due_at)
        );
        engine = checkpoint_round_trip(&engine);

        move_player_off_level(&mut engine, observed_level);
        let mut reset = advance_to_boundary(&mut engine, LogicalTime::new(due_at.value() + 1));
        assert!(reset.events.iter().any(|event| matches!(
            event,
            Event::EcologyReset {
                site_id: reset_site,
                from_generation: 0,
                to_generation: 1,
                policy: EcologyLifecyclePolicyV1::FullSite,
                ..
            } if reset_site == site_id
        )));
        while engine.world().ecology_sites[site_id]
            .member_slots
            .values()
            .any(|slot| slot.actor_id.is_none())
        {
            let next_boundary = LogicalTime::new(engine.world().timing.now.value() + 1);
            reset = advance_to_boundary(&mut engine, next_boundary);
            assert!(reset.events.iter().any(|event| matches!(
                event,
                Event::EcologyActorSpawned { site_id: spawned_site, generation: 1, .. }
                    if spawned_site == site_id
            )));
        }
        assert_eq!(engine.world().ecology_sites[site_id].generation, 1);
        let generation_one_id = format!("ecology:{site_id}:{member_id}:1");
        let generation_one_items = engine
            .world()
            .actor(&ActorId::from(generation_one_id.as_str()))
            .unwrap_or_else(|| panic!("refreshed signature actor {generation_one_id}"))
            .carried
            .items
            .values()
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            generation_one_items
                .iter()
                .any(|item_id| item_id.ends_with(guaranteed_suffix))
        );
        assert!(
            generation_zero_items
                .iter()
                .all(|item_id| engine.world().item_instances.contains_key(item_id))
        );
        assert!(
            generation_one_items
                .iter()
                .all(|item_id| engine.world().item_instances.contains_key(item_id))
        );
        assert_eq!(engine.world().corpses.len(), corpse_count);
        checkpoint_round_trip(&engine);
    }
}
