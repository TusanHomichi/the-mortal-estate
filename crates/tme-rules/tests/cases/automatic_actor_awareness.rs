use crate::ai_support::{
    automatic_actor, decision, engine, engine_from_value, line_of_sight_memory, open_room_value,
    set_actor_hidden, unrestricted, wait,
};
use tme_rules::{
    ActorLifeState, AutomaticActorDecisionV1, AutomaticMovementPurposeV1, AutomaticWaitReasonV1,
    CharacterAlignment, Coord, SocialAlignmentSource,
};

fn remembered<'a>(
    engine: &'a tme_rules::Engine,
    actor_id: &str,
) -> Option<&'a tme_rules::RememberedHostile> {
    engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == actor_id)
        .and_then(|actor| actor.ai.as_ref())
        .and_then(|ai| ai.awareness.remembered.as_ref())
}

fn memory_actor(id: &str, position: Coord, opportunities: u32) -> serde_json::Value {
    automatic_actor(
        id,
        "chaotic",
        position,
        "simple_chase",
        1,
        line_of_sight_memory(opportunities),
        &["fight"],
    )
}

#[test]
fn unrestricted_awareness_still_requires_line_of_sight_for_initial_acquisition() {
    let mut engine = engine(vec![automatic_actor(
        "hunter",
        "chaotic",
        Coord { x: 5, y: 2 },
        "simple_chase",
        1,
        unrestricted(),
        &["fight"],
    )]);
    set_actor_hidden(&mut engine, "player", true);
    let events = wait(&mut engine);
    assert_eq!(
        decision(&events, "hunter"),
        &AutomaticActorDecisionV1::Wait {
            reason: AutomaticWaitReasonV1::Watch,
        }
    );
    assert!(remembered(&engine, "hunter").is_none());
}

#[test]
fn hidden_hostile_is_not_a_fresh_line_of_sight_target() {
    let mut engine = engine(vec![memory_actor("watcher", Coord { x: 5, y: 2 }, 2)]);
    set_actor_hidden(&mut engine, "player", true);
    let events = wait(&mut engine);
    assert_eq!(
        decision(&events, "watcher"),
        &AutomaticActorDecisionV1::Wait {
            reason: AutomaticWaitReasonV1::Watch,
        }
    );
    assert!(remembered(&engine, "watcher").is_none());
}

#[test]
fn hidden_hostile_on_shared_hex_is_still_actionable() {
    let mut engine = engine(vec![memory_actor("watcher", Coord { x: 1, y: 2 }, 2)]);
    set_actor_hidden(&mut engine, "player", true);
    let events = wait(&mut engine);
    assert!(matches!(
        decision(&events, "watcher"),
        AutomaticActorDecisionV1::PhysicalAttack {
            target_id,
            mode: tme_rules::PhysicalAttackMode::Fight,
            ..
        } if target_id == "player"
    ));
    assert_eq!(
        remembered(&engine, "watcher")
            .expect("shared-hex target remembered")
            .actor_id,
        "player"
    );
}

#[test]
fn visible_refresh_then_hidden_search_decrements_once_per_opportunity() {
    let mut engine = engine(vec![memory_actor("watcher", Coord { x: 5, y: 2 }, 2)]);
    wait(&mut engine);
    let first = remembered(&engine, "watcher").expect("visible target remembered");
    assert_eq!(first.actor_id, "player");
    assert_eq!(first.remaining_opportunities, 2);

    set_actor_hidden(&mut engine, "player", true);
    let events = wait(&mut engine);
    assert!(matches!(
        decision(&events, "watcher"),
        AutomaticActorDecisionV1::Move {
            purpose: AutomaticMovementPurposeV1::Search,
            ..
        }
    ));
    assert_eq!(
        remembered(&engine, "watcher")
            .expect("memory retained")
            .remaining_opportunities,
        1
    );
}

#[test]
fn read_only_debug_snapshots_do_not_mutate_memory() {
    let mut engine = engine(vec![memory_actor("watcher", Coord { x: 5, y: 2 }, 3)]);
    wait(&mut engine);
    set_actor_hidden(&mut engine, "player", true);
    wait(&mut engine);
    let before = remembered(&engine, "watcher").cloned();
    let first = engine.snapshot();
    let second = engine.snapshot();
    assert_eq!(first.automatic_actors, second.automatic_actors);
    assert_eq!(remembered(&engine, "watcher").cloned(), before);
}

#[test]
fn awareness_memory_is_separate_per_actor() {
    let mut engine = engine(vec![
        memory_actor("north", Coord { x: 5, y: 1 }, 3),
        memory_actor("south", Coord { x: 5, y: 3 }, 2),
    ]);
    wait(&mut engine);
    set_actor_hidden(&mut engine, "player", true);
    wait(&mut engine);
    assert_eq!(
        remembered(&engine, "north")
            .expect("north memory")
            .remaining_opportunities,
        2
    );
    assert_eq!(
        remembered(&engine, "south")
            .expect("south memory")
            .remaining_opportunities,
        1
    );
}

#[test]
fn visible_hostile_replaces_remembered_target() {
    let mut guardian = automatic_actor(
        "guardian",
        "lawful",
        Coord { x: 6, y: 2 },
        "hold_ground",
        1,
        unrestricted(),
        &["fight"],
    );
    guardian["active_effects"] = serde_json::json!([crate::ai_support::active_effect(
        "guardian_hidden",
        "hidden",
        false,
    )]);
    let mut engine = engine(vec![
        memory_actor("watcher", Coord { x: 5, y: 2 }, 3),
        guardian,
    ]);
    wait(&mut engine);
    assert_eq!(
        remembered(&engine, "watcher")
            .expect("first memory")
            .actor_id,
        "player"
    );

    set_actor_hidden(&mut engine, "player", true);
    set_actor_hidden(&mut engine, "guardian", false);
    wait(&mut engine);
    assert_eq!(
        remembered(&engine, "watcher")
            .expect("replacement memory")
            .actor_id,
        "guardian"
    );
}

#[test]
fn memory_expires_after_authored_opportunities() {
    let mut engine = engine(vec![memory_actor("watcher", Coord { x: 5, y: 2 }, 1)]);
    wait(&mut engine);
    set_actor_hidden(&mut engine, "player", true);
    let search = wait(&mut engine);
    assert!(matches!(
        decision(&search, "watcher"),
        AutomaticActorDecisionV1::Move {
            purpose: AutomaticMovementPurposeV1::Search,
            ..
        }
    ));
    assert_eq!(
        remembered(&engine, "watcher")
            .expect("zero-count memory remains until next opportunity")
            .remaining_opportunities,
        0
    );
    let expired = wait(&mut engine);
    assert_eq!(
        decision(&expired, "watcher"),
        &AutomaticActorDecisionV1::Wait {
            reason: AutomaticWaitReasonV1::Watch,
        }
    );
    assert!(remembered(&engine, "watcher").is_none());
}

fn creature_target_engine() -> tme_rules::Engine {
    let actors = vec![
        memory_actor("watcher", Coord { x: 5, y: 2 }, 2),
        automatic_actor(
            "guardian",
            "lawful",
            Coord { x: 3, y: 2 },
            "hold_ground",
            1,
            unrestricted(),
            &["fight"],
        ),
    ];
    let mut value = open_room_value(actors);
    value.actor_definition_mut(0)["social"]["alignment_source"] =
        serde_json::json!({"kind": "inherent", "alignment": "chaotic"});
    engine_from_value(value)
}

#[test]
fn target_death_invalidates_memory() {
    let mut engine = creature_target_engine();
    wait(&mut engine);
    assert_eq!(
        remembered(&engine, "watcher")
            .expect("guardian remembered")
            .actor_id,
        "guardian"
    );
    let guardian = engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == "guardian")
        .expect("guardian exists");
    guardian.life_state = ActorLifeState::Dead;
    let events = wait(&mut engine);
    assert!(matches!(
        decision(&events, "watcher"),
        AutomaticActorDecisionV1::Wait {
            reason: AutomaticWaitReasonV1::Watch,
        }
    ));
    assert!(remembered(&engine, "watcher").is_none());
}

#[test]
fn social_alignment_change_invalidates_memory() {
    let mut engine = creature_target_engine();
    wait(&mut engine);
    let guardian = engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == "guardian")
        .expect("guardian exists");
    guardian.social.alignment_source = SocialAlignmentSource::Inherent {
        alignment: CharacterAlignment::Chaotic,
    };
    let events = wait(&mut engine);
    assert!(matches!(
        decision(&events, "watcher"),
        AutomaticActorDecisionV1::Wait {
            reason: AutomaticWaitReasonV1::Watch,
        }
    ));
    assert!(remembered(&engine, "watcher").is_none());
}

#[test]
fn remembered_search_keeps_last_seen_room_when_target_changes_rooms() {
    let actors = vec![memory_actor("watcher", Coord { x: 5, y: 2 }, 2)];
    let mut value = open_room_value(actors);
    let room = value.template_levels_source_mut()["room_0"].clone();
    value.template_levels_source_mut()["room_1"] = room;
    let mut engine = engine_from_value(value);
    wait(&mut engine);
    {
        let player = engine
            .world_mut()
            .actors
            .iter_mut()
            .find(|actor| actor.id == "player")
            .expect("player exists");
        player.location.level = "room_1".to_string();
    }
    set_actor_hidden(&mut engine, "player", true);
    let events = wait(&mut engine);
    assert!(matches!(
        decision(&events, "watcher"),
        AutomaticActorDecisionV1::Move {
            purpose: AutomaticMovementPurposeV1::Search,
            ..
        }
    ));
    assert_eq!(
        remembered(&engine, "watcher")
            .expect("room-scoped memory")
            .last_seen
            .level,
        "room_0"
    );
}
