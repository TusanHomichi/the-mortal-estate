use crate::support::content_parts::ContentParts;
use tme_rules::{Direction, Engine, Event, PlayerIntent};

fn tracked_engine(case_id: &str, profile: &str) -> Engine {
    ContentParts::tracked(case_id, profile)
        .engine(7)
        .expect("tracked content should start")
}

#[test]
fn melee_attack_unchanged_by_los() {
    let mut engine = tracked_engine("first_room", "profile/first_room");
    let _ = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .ok();
    let _ = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .ok();
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("step");
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::AttackBlockedNoSight { .. })),
        "melee attacks should not be sight-gated"
    );
}

#[test]
fn action_context_marks_los_blocked_targets() {
    let engine = tracked_engine("ranged_attack", "profile/ranged_attack");
    let context = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    for target in &context.attack_targets {
        for option in &target.physical_attacks {
            if !option.enabled {
                assert!(option.blocked_reason.is_some(), "blocked must have reason");
            }
        }
    }
}

#[test]
fn ranged_attack_unambiguously_blocked_by_wall() {
    let mut parts = ContentParts::tracked("ranged_attack", "profile/ranged_attack");
    parts.template_levels_source_mut()["room_0"]["cells"][1][2] = serde_json::json!(["stone_wall"]);
    let mut engine = parts.engine(7).expect("wall content should start");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Shoot,
                target_actor_id: "reedling".into(),
            },
        )
        .expect("attack");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::AttackBlockedNoSight { .. })),
        "ranged attack through wall must be blocked by sight"
    );
}
