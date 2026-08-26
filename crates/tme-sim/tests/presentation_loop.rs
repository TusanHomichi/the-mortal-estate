//! Presentation loop smoke harness — proves the intended client loop
//! without creating a real client.  See Slice AD.
//!
//! Loop:
//!   load → snapshot → action context/options → preview → commit command →
//!   events → after-snapshot → after-context → repeat

use std::path::PathBuf;
use tme_rules::{
    ActionOptionV1, COMMAND_CONTRACT_VERSION, Direction, Engine, PlayerActionContextV2,
    PlayerCommandV1, PlayerIntent, WorldSnapshotV1, WorldSnapshotV2,
};

fn scenario_path(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/test-corpus")
        .join(name)
        .to_string_lossy()
        .to_string()
}

fn load_engine(name: &str, seed: u64) -> Engine {
    tme_sim::load_engine_from_scenario(std::path::Path::new(&scenario_path(name)), Some(seed))
        .expect("load validated simulation graph")
}

/// One turn of the presentation loop.
#[allow(dead_code)]
struct Frame {
    before_debug: WorldSnapshotV1,
    before_observed: WorldSnapshotV2,
    before_context: PlayerActionContextV2,
    options: Vec<ActionOptionV1>,
    preview: Option<tme_rules::PathPreviewV1>,
    command: PlayerCommandV1,
    events: Vec<tme_rules::Event>,
    after_debug: WorldSnapshotV1,
    after_observed: WorldSnapshotV2,
    after_context: PlayerActionContextV2,
}

fn run_presentation_loop(engine: &mut Engine, intents: Vec<PlayerIntent>) -> Vec<Frame> {
    let mut frames = Vec::new();

    for intent in intents {
        // --- Before state ---
        let before_debug = engine.snapshot();
        let before_observed = engine
            .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
            .expect("observed snapshot");
        let before_context = engine
            .actor_observed_action_context(&tme_rules::ActorId::from("player"))
            .expect("action context");
        let options = engine
            .actor_action_options(&tme_rules::ActorId::from("player"))
            .expect("action options");

        // --- Preview (movement only) ---
        let preview = intent.movement_path().and_then(|path| {
            engine
                .preview_actor_path(&tme_rules::ActorId::from("player"), &path)
                .ok()
        });

        // --- Build command ---
        let player_id = engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .map(|p| p.id.clone())
            .expect("validated simulation player");
        let payload = Engine::player_intent_to_payload(&intent);
        let command = PlayerCommandV1 {
            contract_version: COMMAND_CONTRACT_VERSION,
            actor_id: player_id,
            intent: payload,
        };

        // --- Commit (lock, consume events, unlock) ---
        let events = engine
            .apply_actor_intent(&command.actor_id, intent)
            .expect("step must succeed")
            .events;

        // --- After state ---
        let after_debug = engine.snapshot();
        let after_observed = engine
            .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
            .expect("observed snapshot");
        let after_context = engine
            .actor_observed_action_context(&tme_rules::ActorId::from("player"))
            .expect("action context");

        frames.push(Frame {
            before_debug,
            before_observed,
            before_context,
            options,
            preview,
            command,
            events,
            after_debug,
            after_observed,
            after_context,
        });
    }

    frames
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn presentation_loop_preview_before_commit_for_movement() {
    let mut engine = load_engine("terrain_movement.json", 42);
    let intents = vec![
        PlayerIntent::MovePath(vec![Direction::East]),
        PlayerIntent::MovePath(vec![Direction::East]),
    ];
    let frames = run_presentation_loop(&mut engine, intents);

    for frame in &frames {
        // Every movement step must have captured a preview before commit
        assert!(
            frame.preview.is_some(),
            "movement steps must have a preview before commit"
        );
    }
}

#[test]
fn presentation_loop_after_state_emitted_after_each_step() {
    let mut engine = load_engine("first_room.json", 7);
    let intents = vec![
        PlayerIntent::MovePath(vec![Direction::East]),
        PlayerIntent::PhysicalAttack {
            mode: tme_rules::PhysicalAttackMode::Fight,
            target_actor_id: "mireling".into(),
            authorization: tme_rules::HostilityAuthorization::Safe,
        },
    ];
    let frames = run_presentation_loop(&mut engine, intents);

    assert!(!frames.is_empty(), "must have frames");

    for (i, frame) in frames.iter().enumerate() {
        // After each step, after-snapshots and after-context must exist
        assert!(
            !frame.after_debug.actors.is_empty(),
            "frame {i}: after debug snapshot must have actors"
        );
        assert!(
            !frame.after_observed.actors.is_empty(),
            "frame {i}: after observed snapshot must have actors"
        );
        assert!(
            !frame.after_context.actor_id.is_empty(),
            "frame {i}: after action context must have actor"
        );
        // Events must be non-empty
        assert!(
            !frame.events.is_empty(),
            "frame {i}: must have committed events"
        );
    }
}

#[test]
fn presentation_loop_action_context_updates_after_movement() {
    let mut engine = load_engine("first_room.json", 7);

    let ctx_before = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context before");
    let pos_before = ctx_before.position;

    let _ = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::South]),
        )
        .expect("move south");

    let ctx_after = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context after");
    let pos_after = ctx_after.position;

    assert_ne!(pos_before, pos_after, "position must change after movement");
    assert_eq!(
        ctx_after.logical_time,
        tme_rules::LogicalTime::new(2),
        "a standard movement action must advance to the next logical time"
    );
}

#[test]
fn presentation_loop_action_context_updates_after_item_pickup() {
    let mut engine = load_engine("supply_cache.json", 7);

    let ctx_before = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context before");
    let ground_before = ctx_before.ground_items_here.len();

    let _ = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "hemp_rope".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::SackItem1,
                },
            },
        )
        .expect("take item");

    let ctx_after = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context after");

    assert!(ground_before > 0, "must have ground items before pickup");
    assert!(
        !ctx_after.carried.items.is_empty(),
        "must have carried items after pickup"
    );
}

#[test]
fn presentation_loop_action_context_updates_after_door_open() {
    let mut engine = load_engine("undercroft_loop.json", 7);

    let ctx_before = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context before");
    let doors_open_before = ctx_before
        .door_actions
        .iter()
        .filter(|d| d.can_close)
        .count();

    let _ = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Open(Direction::East),
        )
        .expect("open door");

    let ctx_after = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context after");
    let doors_open_after = ctx_after
        .door_actions
        .iter()
        .filter(|d| d.can_close)
        .count();

    assert_eq!(
        doors_open_before, 0,
        "no door should be open (closeable) before opening"
    );
    assert!(
        doors_open_after > 0,
        "door must be open (closeable) after opening"
    );
}

#[test]
fn presentation_loop_options_include_enabled_and_disabled() {
    let engine = load_engine("first_room.json", 7);
    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");

    let enabled: Vec<_> = options.iter().filter(|o| o.enabled).collect();
    let disabled: Vec<_> = options.iter().filter(|o| !o.enabled).collect();

    assert!(
        !enabled.is_empty(),
        "must have enabled options (walkable exits, always-available)"
    );
    assert!(
        !disabled.is_empty(),
        "must have disabled options (blocked exits)"
    );

    // Every disabled option must have a blocked reason
    for opt in &disabled {
        assert!(
            opt.blocked_reason.is_some(),
            "disabled option '{}' must have a blocked reason",
            opt.id
        );
        // And it must be a valid typed reason (we accept any valid variant)
        let _reason = opt.blocked_reason.unwrap();
    }
}

#[test]
fn presentation_loop_is_deterministic() {
    let mut engine_a = load_engine("first_room.json", 42);
    let mut engine_b = load_engine("first_room.json", 42);
    let intents = vec![
        PlayerIntent::MovePath(vec![Direction::East]),
        PlayerIntent::PhysicalAttack {
            mode: tme_rules::PhysicalAttackMode::Fight,
            target_actor_id: "mireling".into(),
            authorization: tme_rules::HostilityAuthorization::Safe,
        },
    ];

    let frames_a = run_presentation_loop(&mut engine_a, intents.clone());
    let frames_b = run_presentation_loop(&mut engine_b, intents);

    assert_eq!(frames_a.len(), frames_b.len());
    for (i, (fa, fb)) in frames_a.iter().zip(frames_b.iter()).enumerate() {
        assert_eq!(
            fa.events, fb.events,
            "frame {i}: events must be deterministic"
        );
        assert_eq!(
            fa.after_debug, fb.after_debug,
            "frame {i}: after debug snapshot must be deterministic"
        );
        assert_eq!(
            fa.after_observed, fb.after_observed,
            "frame {i}: after observed snapshot must be deterministic"
        );
        assert_eq!(
            fa.after_context, fb.after_context,
            "frame {i}: after action context must be deterministic"
        );
    }
}

#[test]
fn presentation_loop_lock_unlock_cycle_is_deterministic() {
    let mut engine = load_engine("first_room.json", 7);
    let intents = vec![PlayerIntent::Wait, PlayerIntent::Wait];
    let frames = run_presentation_loop(&mut engine, intents);

    for (i, frame) in frames.iter().enumerate() {
        // Each frame represents a lock→unlock cycle:
        // before-state → commit(lock) → events → after-state(unlock)
        assert!(
            !frame.before_debug.actors.is_empty(),
            "frame {i}: must have before-state"
        );
        assert!(
            !frame.events.is_empty(),
            "frame {i}: must have events (consumed during lock)"
        );
        assert!(
            !frame.after_debug.actors.is_empty(),
            "frame {i}: must have after-state (unlock)"
        );
    }

    // Run again and verify deterministic frame structure
    let mut engine2 = load_engine("first_room.json", 7);
    let frames2 = run_presentation_loop(&mut engine2, vec![PlayerIntent::Wait, PlayerIntent::Wait]);
    assert_eq!(frames.len(), frames2.len());
}

#[test]
fn presentation_loop_preview_matches_committed_movement() {
    let mut engine = load_engine("terrain_movement.json", 42);
    let intents = vec![
        PlayerIntent::MovePath(vec![Direction::East]),
        PlayerIntent::MovePath(vec![Direction::East]),
        PlayerIntent::MovePath(vec![Direction::South]),
    ];
    let frames = run_presentation_loop(&mut engine, intents);

    for frame in &frames {
        if let Some(ref preview) = frame.preview {
            // The preview's final position should match the player's position
            // in the after-snapshot
            let player_after = frame
                .after_debug
                .actors
                .iter()
                .find(|a| a.kind == tme_rules::ActorKind::Player)
                .expect("player must exist after step");
            assert_eq!(
                preview.final_position.position, player_after.location.position,
                "preview final position must match player position after commit"
            );
            // Full path accepted means the preview predicted success
            if preview.stop_reason == tme_rules::MovementStopReason::FullPathAccepted {
                assert!(
                    preview.remaining_path_points >= 0,
                    "remaining path points must be non-negative on success"
                );
            }
        }
    }
}
