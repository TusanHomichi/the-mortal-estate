//! Weapon-speed readiness tests — per-weapon attack cooldowns.

use crate::support::content_parts::ContentParts;
use tme_rules::{Direction, Engine, Event, PhysicalAttackMode, PlayerIntent};

fn engine_with_cooldown(cooldown_units: u32, seed: u64) -> Engine {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.selected_by_runtime_id_mut("items", "training_knife")["weapon"]["cooldown_units"] =
        serde_json::json!(cooldown_units);
    parts.engine(seed).expect("weapon-speed graph should start")
}

fn engage(engine: &mut Engine) {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("move should succeed");
}

fn attack(engine: &mut Engine) -> Vec<Event> {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack step should resolve")
        .events
}

fn player_attempted_attack(events: &[Event]) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            Event::Attacked { attacker, .. }
                | Event::AttackBlocked { attacker, .. }
                | Event::AttackMissed { attacker, .. }
                if attacker == "Delver"
        )
    })
}

#[test]
fn default_weapon_uses_one_round_cooldown() {
    let mut engine = engine_with_cooldown(1, 1_010_580_540);
    engage(&mut engine);
    assert!(player_attempted_attack(&attack(&mut engine)));
}

#[test]
fn slower_weapon_blocks_attack_until_ready() {
    let mut engine = engine_with_cooldown(2, 7);
    engage(&mut engine);

    assert!(
        player_attempted_attack(&attack(&mut engine)),
        "slow weapon should attempt its first attack"
    );

    let blocked = attack(&mut engine);
    assert!(
        blocked
            .iter()
            .any(|event| matches!(event, Event::AttackNotReady { .. }))
    );
    assert!(!player_attempted_attack(&blocked));

    assert!(
        player_attempted_attack(&attack(&mut engine)),
        "slow weapon should be ready on the following opportunity"
    );
}

#[test]
fn action_context_reports_not_ready_state() {
    let mut engine = engine_with_cooldown(2, 7);
    engage(&mut engine);
    attack(&mut engine);

    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context");
    assert!(context.attack_ready_at > context.logical_time);
}

#[test]
fn tracked_first_room_preserves_one_round_cooldown() {
    let mut engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(1_010_580_540)
        .expect("tracked first-room graph should start");
    engage(&mut engine);
    assert!(player_attempted_attack(&attack(&mut engine)));

    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context");
    assert_eq!(context.attack_ready_at, context.logical_time);
}
