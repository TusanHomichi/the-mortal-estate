use crate::support::content_parts::ContentParts;
use tme_rules::{DamageLabel, Direction, Engine, Event, PlayerIntent};

fn first_room_engine() -> Engine {
    ContentParts::tracked("first_room", "profile/first_room")
        .engine(1_010_580_540)
        .expect("engine should start")
}

fn canonical_label(damage: i32, hp_before: i32, hp_after: i32) -> DamageLabel {
    DamageLabel::for_hit(damage, hp_before, hp_after, 20, 40, 70)
}

#[test]
fn damage_labels_use_pre_hit_current_hp_thresholds() {
    assert_eq!(canonical_label(1, 10, 9), DamageLabel::Light);
    assert_eq!(canonical_label(19, 100, 81), DamageLabel::Light);
    assert_eq!(canonical_label(2, 10, 8), DamageLabel::Moderate);
    assert_eq!(canonical_label(20, 100, 80), DamageLabel::Moderate);
    assert_eq!(canonical_label(39, 100, 61), DamageLabel::Moderate);
    assert_eq!(canonical_label(4, 10, 6), DamageLabel::Heavy);
    assert_eq!(canonical_label(40, 100, 60), DamageLabel::Heavy);
    assert_eq!(canonical_label(69, 100, 31), DamageLabel::Heavy);
    assert_eq!(canonical_label(7, 10, 3), DamageLabel::Severe);
    assert_eq!(canonical_label(70, 100, 30), DamageLabel::Severe);
}

#[test]
fn fatal_label_overrides_percentage_thresholds() {
    assert_eq!(canonical_label(1, 10, 0), DamageLabel::Fatal);
    assert_eq!(canonical_label(10, 10, 0), DamageLabel::Fatal);
}

#[test]
fn damage_labels_use_authored_thresholds() {
    assert_eq!(
        DamageLabel::for_hit(25, 100, 75, 30, 60, 90),
        DamageLabel::Light
    );
    assert_eq!(
        DamageLabel::for_hit(30, 100, 70, 30, 60, 90),
        DamageLabel::Moderate
    );
    assert_eq!(
        DamageLabel::for_hit(60, 100, 40, 30, 60, 90),
        DamageLabel::Heavy
    );
    assert_eq!(
        DamageLabel::for_hit(90, 100, 10, 30, 60, 90),
        DamageLabel::Severe
    );
}

#[test]
fn damage_labels_render_lowercase() {
    assert_eq!(DamageLabel::Light.label(), "light");
    assert_eq!(DamageLabel::Moderate.label(), "moderate");
    assert_eq!(DamageLabel::Heavy.label(), "heavy");
    assert_eq!(DamageLabel::Severe.label(), "severe");
    assert_eq!(DamageLabel::Fatal.label(), "fatal");
}

#[test]
fn attack_events_include_damage_labels() {
    let mut engine = first_room_engine();

    let turn_one = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("turn one should step");
    assert!(!turn_one.iter().any(|event| matches!(
        event,
        Event::Attacked {
            attacker_id,
            defender_id,
            ..
        } if attacker_id == "mireling" && defender_id == "player"
    )));
    assert!(turn_one.events.contains(&Event::Moved {
        actor_id: "mireling".into(),
        actor: "Mireling".to_string(),
        from: tme_rules::WorldPosition::new("realm_0", "room_0", (3, 1).into()),
        to: tme_rules::WorldPosition::new("realm_0", "room_0", (2, 1).into()),
        navigation: tme_rules::NavigationKind::Walk,
    }));

    let turn_two = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("turn two should step");
    assert!(turn_two.iter().any(|event| matches!(
        event,
        Event::Attacked {
            attacker_id,
            defender_id,
            mode: tme_rules::PhysicalAttackMode::Fight,
            damage_kind: tme_rules::PhysicalDamageKind::Cutting,
            label: DamageLabel::Fatal,
            defender_hp: 0,
            ..
        } if attacker_id == "player" && defender_id == "mireling"
    )));
}
