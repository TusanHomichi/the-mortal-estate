use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};
use tme_rules::{
    Engine, Event, ItemRelocationReason, PhysicalAttackMode, PhysicalAttackOutcome, PlayerIntent,
    model::CombatRisk,
};

const FIRST_ROOM: (&str, &str) = ("first_room", "profile/first_room");
const SKILL_PROGRESSION: (&str, &str) = ("skill_progression", "profile/skill_progression");

fn fixture((case_id, profile): (&str, &str)) -> ContentParts {
    ContentParts::tracked(case_id, profile)
}

fn engine(parts: ContentParts, seed: u64) -> Engine {
    parts.engine(seed).expect("engine should start")
}

fn actor_mut<'a>(parts: &'a mut ContentParts, actor_id: &str) -> &'a mut Value {
    parts
        .actors_mut()
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|actor| actor["id"] == actor_id)
        .unwrap()
}

fn resolve_fight(parts: ContentParts) -> Vec<Event> {
    engine(parts, 7)
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("fight should resolve")
        .events
}

fn event_index(events: &[Event], predicate: impl Fn(&Event) -> bool) -> usize {
    events.iter().position(predicate).expect("expected event")
}

#[test]
fn practice_plan_classifies_all_three_risk_bands_once() {
    for (label, target_xp, player_hp, expected) in [
        ("practice", 4, 20, CombatRisk::Practice),
        ("life_and_death", 5, 20, CombatRisk::LifeAndDeath),
        ("overwhelming", 4, 4, CombatRisk::Overwhelming),
    ] {
        let mut parts = fixture(SKILL_PROGRESSION);
        let player_position = actor_mut(&mut parts, "player")["location"]["position"].clone();
        actor_mut(&mut parts, "mireling")["location"]["position"] = player_position;
        parts.actor_definition_by_actor_id_mut("mireling")["stats"]["defense"] = json!(100);
        parts.actor_definition_by_actor_id_mut("mireling")["xp_value"] = json!(target_xp);
        parts.actor_definition_by_actor_id_mut("player")["stats"]["hp"] = json!(player_hp);
        actor_mut(&mut parts, "player")["character"]["resources"]["hp"] = json!(player_hp);
        let events = resolve_fight(parts);
        let receipts = events
            .iter()
            .filter_map(|event| match event {
                Event::PhysicalPracticeEvaluated { risk, .. } => Some(*risk),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(receipts, [expected], "{label}");
    }
}

#[test]
fn advanced_practice_band_can_evaluate_to_zero_without_mutating_skill_points() {
    let mut parts = fixture(SKILL_PROGRESSION);
    let player_position = actor_mut(&mut parts, "player")["location"]["position"].clone();
    actor_mut(&mut parts, "mireling")["location"]["position"] = player_position;
    parts.actor_definition_by_actor_id_mut("mireling")["stats"]["defense"] = json!(100);
    parts.actor_definition_by_actor_id_mut("mireling")["xp_value"] = json!(1);
    actor_mut(&mut parts, "player")["character"]["skill_ledger"] = json!([{
        "track_id": "sword",
        "level": 8,
        "critique_rank": 0,
        "practice_points": 0,
        "learning_rate": 1
    }]);
    let events = resolve_fight(parts);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::PhysicalPracticeEvaluated {
            risk: CombatRisk::Practice,
            base_raw_points: 0,
            fatal_blow_bonus_raw_points: 0,
            total_raw_points: 0,
            ..
        }
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::SkillPracticeAwarded { actor_id, .. } if actor_id == "player"
    )));
}

#[test]
fn fatal_throw_records_one_contribution_reward_before_post_attack_practice_and_relocation() {
    let mut parts = fixture(SKILL_PROGRESSION);
    actor_mut(&mut parts, "mireling")["location"]["position"] = json!({"x": 3, "y": 1});
    parts.actor_definition_by_actor_id_mut("mireling")["stats"]["hp"] = json!(1);
    parts.actor_definition_by_actor_id_mut("mireling")["xp_value"] = json!(5);
    let weapon = parts.selected_mut("items", 0);
    weapon["weapon"]["default_attack_mode"] = json!("throw");
    weapon["weapon"]["attack_modes"] = json!([{
        "mode": "throw", "maximum_range": 3, "damage_kind": "piercing"
    }]);
    let events = engine(parts, 1_010_580_540)
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: PhysicalAttackMode::Throw,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("fatal throw should resolve");

    let practice = event_index(&events.events, |event| {
        matches!(
            event,
            Event::PhysicalPracticeEvaluated {
                mode: PhysicalAttackMode::Throw,
                outcome: PhysicalAttackOutcome::FatalBlow,
                risk: CombatRisk::LifeAndDeath,
                base_raw_points: 2,
                fatal_blow_bonus_raw_points: 1,
                total_raw_points: 3,
                ..
            }
        )
    });
    let relocation = event_index(&events.events, |event| {
        matches!(
            event,
            Event::ItemRelocated {
                reason: ItemRelocationReason::Thrown,
                ..
            }
        )
    });
    let reward = event_index(&events.events, |event| {
        matches!(
            event,
            Event::DefeatRewardEvaluated {
                awarded_experience: 5,
                reason,
                ..
            } if reason == "contribution_shared"
        )
    });
    let experience = event_index(&events.events, |event| {
        matches!(event, Event::ExperienceAwarded { amount: 5, .. })
    });
    assert!(reward < experience && experience < practice && practice < relocation);
}

#[test]
fn transient_player_actor_gets_an_explicit_zero_contribution_reward() {
    let mut parts = fixture(FIRST_ROOM);
    let player_position = actor_mut(&mut parts, "player")["location"]["position"].clone();
    actor_mut(&mut parts, "mireling")["location"]["position"] = player_position;
    parts.actor_definition_by_actor_id_mut("mireling")["stats"]["hp"] = json!(1);
    parts.actor_definition_by_actor_id_mut("mireling")["xp_value"] = json!(7);
    parts.actor_definition_by_actor_id_mut("player")["stats"]["attack"] = json!(100);
    let events = engine(parts, 1_010_580_540)
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("fatal fight should resolve");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::DefeatRewardEvaluated {
            awarded_experience: 0,
            reason,
            ..
        } if reason == "no_eligible_player_contribution"
    )));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::ExperienceAwarded { .. }))
    );
}
