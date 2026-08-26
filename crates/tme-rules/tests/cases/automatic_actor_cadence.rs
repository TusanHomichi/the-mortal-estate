use crate::ai_support::{
    automatic_actor, decisions, engine, line_of_sight_memory, set_actor_hidden, unrestricted, wait,
};
use tme_rules::{Coord, Event, LogicalTime};

fn cadence_actor(id: &str, y: i32, cadence_units: u32) -> serde_json::Value {
    automatic_actor(
        id,
        "chaotic",
        Coord { x: 6, y },
        "hold_ground",
        cadence_units,
        unrestricted(),
        &["fight"],
    )
}

fn ready_order(events: &[Event]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::ActorReady { actor_id, .. } if actor_id != "player" => Some(actor_id.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn cadences_one_two_and_three_interleave_on_one_logical_clock() {
    let mut engine = engine(vec![
        cadence_actor("one", 1, 1),
        cadence_actor("two", 2, 2),
        cadence_actor("three", 3, 3),
    ]);

    let initial_boundary = wait(&mut engine);
    assert_eq!(ready_order(&initial_boundary), vec!["one", "two", "three"]);
    let scheduled = initial_boundary
        .iter()
        .filter_map(|event| match event {
            Event::ActorReadinessScheduled {
                actor_id,
                cost_units,
                ready_at,
                ..
            } if actor_id != "player" => Some((actor_id.as_str(), *cost_units, *ready_at)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        scheduled,
        vec![
            ("one", 1, LogicalTime::new(3)),
            ("two", 2, LogicalTime::new(4)),
            ("three", 3, LogicalTime::new(5)),
        ]
    );

    let at_one = wait(&mut engine);
    assert_eq!(ready_order(&at_one), vec!["one"]);

    let at_two = wait(&mut engine);
    assert_eq!(ready_order(&at_two), vec!["one", "two"]);

    let at_three = wait(&mut engine);
    assert_eq!(ready_order(&at_three), vec!["one", "three"]);
}

#[test]
fn every_ready_opportunity_emits_one_decision_and_one_cadence_schedule() {
    let mut engine = engine(vec![
        cadence_actor("one", 1, 1),
        cadence_actor("two", 2, 2),
        cadence_actor("three", 3, 3),
    ]);
    for _ in 0..5 {
        let events = wait(&mut engine);
        for actor_id in ["one", "two", "three"] {
            let ready_count = events
                .iter()
                .filter(|event| {
                    matches!(
                        event,
                        Event::ActorReady { actor_id: candidate, .. } if candidate == actor_id
                    )
                })
                .count();
            let decision_count = decisions(&events, actor_id).count();
            let schedule_count = events
                .iter()
                .filter(|event| {
                    matches!(
                        event,
                        Event::ActorReadinessScheduled { actor_id: candidate, .. }
                            if candidate == actor_id
                    )
                })
                .count();
            assert_eq!(decision_count, ready_count, "decision count for {actor_id}");
            assert_eq!(schedule_count, ready_count, "schedule count for {actor_id}");
        }
    }
}

#[test]
fn stable_registration_order_breaks_repeated_ready_ties() {
    let mut engine = engine(vec![
        cadence_actor("first", 1, 2),
        cadence_actor("second", 2, 2),
        cadence_actor("third", 3, 2),
    ]);
    assert_eq!(
        ready_order(&wait(&mut engine)),
        vec!["first", "second", "third"]
    );
    assert!(ready_order(&wait(&mut engine)).is_empty());
    assert_eq!(
        ready_order(&wait(&mut engine)),
        vec!["first", "second", "third"]
    );
}

#[test]
fn memory_opportunities_count_per_actor_opportunity_not_group_boundary() {
    let mut engine = engine(vec![
        automatic_actor(
            "fast",
            "chaotic",
            Coord { x: 5, y: 1 },
            "simple_chase",
            1,
            line_of_sight_memory(3),
            &["fight"],
        ),
        automatic_actor(
            "slow",
            "chaotic",
            Coord { x: 5, y: 3 },
            "simple_chase",
            2,
            line_of_sight_memory(3),
            &["fight"],
        ),
    ]);
    wait(&mut engine);
    set_actor_hidden(&mut engine, "player", true);
    wait(&mut engine);
    wait(&mut engine);

    let remaining = |actor_id: &str| {
        engine
            .world()
            .actors
            .iter()
            .find(|actor| actor.id == actor_id)
            .and_then(|actor| actor.ai.as_ref())
            .and_then(|ai| ai.awareness.remembered.as_ref())
            .map(|memory| memory.remaining_opportunities)
            .expect("actor retains memory")
    };
    assert_eq!(remaining("fast"), 1);
    assert_eq!(remaining("slow"), 2);
}

#[test]
fn debug_snapshot_contains_only_actor_local_cadence_and_memory_state() {
    let mut engine = engine(vec![
        cadence_actor("one", 1, 1),
        cadence_actor("two", 2, 2),
        cadence_actor("three", 3, 3),
    ]);
    wait(&mut engine);
    let snapshot = engine.snapshot();
    assert_eq!(
        snapshot
            .automatic_actors
            .iter()
            .map(|actor| (actor.actor_id.as_str(), actor.cadence_units))
            .collect::<Vec<_>>(),
        vec![("one", 1), ("two", 2), ("three", 3)]
    );
    let json = serde_json::to_value(snapshot).expect("snapshot serializes");
    let legacy_awareness = ["monster", "awareness"].join("_");
    let legacy_return = ["returning", "monsters"].join("_");
    assert!(json.get(&legacy_awareness).is_none());
    assert!(json.get(&legacy_return).is_none());
    assert!(json.get("pack_timing").is_none());
}
