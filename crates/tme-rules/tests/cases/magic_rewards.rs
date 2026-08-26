use crate::spell_effect_support::{
    br_effect_spell_engine_with_player_hp_mutate, bs_runtime_spell_engine_mutate,
};
use tme_rules::*;

fn direct_damage_engine(cast_class: &str, target_hp: i32, target_xp: i32) -> Engine {
    br_effect_spell_engine_with_player_hp_mutate(&["spark"], 10, |parts| {
        parts.actor_definition_mut(1)["stats"]["hp"] = serde_json::json!(target_hp);
        parts.actor_definition_mut(1)["xp_value"] = serde_json::json!(target_xp);
        parts.actor_definition_mut(1)["magic_resistance"]["natural_save_twentieths"] =
            serde_json::json!(0);
        let spark = parts.selected_by_runtime_id_mut("spells", "spark");
        spark["casting"]["cast_class"] = serde_json::json!(cast_class);
    })
}

fn actor_target() -> Option<SpellTarget> {
    Some(SpellTarget::Actor {
        actor_id: "target".into(),
    })
}

fn path_target() -> Option<SpellTarget> {
    Some(SpellTarget::Path {
        directions: vec![Direction::East],
    })
}

fn cast_spark(engine: &mut Engine, target: Option<SpellTarget>) -> Vec<Event> {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "spark".to_string(),
                target,
                authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
            },
        )
        .expect("spark should resolve")
        .events
}

fn event_index(events: &[Event], predicate: impl Fn(&Event) -> bool) -> usize {
    events
        .iter()
        .position(predicate)
        .expect("expected event should exist")
}

#[test]
fn directed_spell_kill_awards_full_xp_once_after_defeat() {
    let mut engine = direct_damage_engine("character", 3, 25);
    let events = cast_spark(&mut engine, actor_target());

    assert!(events.iter().any(|event| matches!(
        event,
        Event::DefeatRewardEvaluated {
            target_id,
            authored_experience: 25,
            weighted_damage_denominator: 5,
            available_experience: 25,
            awarded_experience: 25,
            reason,
            ..
        } if target_id == "target" && reason == "contribution_shared"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::DefeatContributionRecorded {
            contributor_character_id: Some(_),
            reward_class: Some(DefeatRewardClass::DirectedSpell),
            ..
        }
    )));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::ActorDefeated { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::DefeatRewardEvaluated { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::ExperienceAwarded { amount: 25, .. }))
            .count(),
        1
    );
    let defeated = event_index(&events, |event| {
        matches!(event, Event::ActorDefeated { .. })
    });
    let reward = event_index(&events, |event| {
        matches!(event, Event::DefeatRewardEvaluated { .. })
    });
    let experience = event_index(&events, |event| {
        matches!(event, Event::ExperienceAwarded { amount: 25, .. })
    });
    let practice = event_index(&events, |event| {
        matches!(event, Event::MagicPracticeEvaluated { .. })
    });
    assert!(defeated < reward && reward < experience && experience < practice);
}

#[test]
fn path_and_path_or_character_targets_select_exact_reward_classes() {
    let mut path = direct_damage_engine("path", 3, 13);
    let path_events = cast_spark(&mut path, path_target());
    assert!(path_events.iter().any(|event| matches!(
        event,
        Event::DefeatRewardEvaluated {
            authored_experience: 13,
            available_experience: 5,
            awarded_experience: 5,
            reason,
            ..
        } if reason == "contribution_shared"
    )));
    assert!(path_events.iter().any(|event| matches!(
        event,
        Event::DefeatContributionRecorded {
            reward_class: Some(DefeatRewardClass::AreaOrIllusionSpell),
            ..
        }
    )));

    let mut actor = direct_damage_engine("path_or_character", 3, 13);
    let actor_events = cast_spark(&mut actor, actor_target());
    assert!(actor_events.iter().any(|event| matches!(
        event,
        Event::DefeatRewardEvaluated {
            awarded_experience: 13,
            ..
        }
    )));

    let mut path_choice = direct_damage_engine("path_or_character", 3, 13);
    let path_choice_events = cast_spark(&mut path_choice, path_target());
    assert!(
        path_choice_events.iter().any(|event| matches!(
            event,
            Event::DefeatRewardEvaluated {
                awarded_experience: 5,
                ..
            }
        )),
        "path-or-character path events: {path_choice_events:#?}"
    );
}

#[test]
fn nonlethal_spell_damage_emits_no_kill_reward() {
    let mut engine = direct_damage_engine("character", 8, 25);
    let events = cast_spark(&mut engine, actor_target());
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellDamaged { target_id, hp: 5, .. } if target_id == "target"
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::DefeatRewardEvaluated { .. } | Event::ExperienceAwarded { .. }
    )));
}

fn poison_engine(target_xp: i32) -> Engine {
    bs_runtime_spell_engine_mutate(
        &["poison"],
        vec!["#####", "#...#", "#####"],
        Coord { x: 2, y: 1 },
        |parts| {
            parts.actor_definition_mut(1)["stats"]["hp"] = serde_json::json!(1);
            parts.actor_definition_mut(1)["xp_value"] = serde_json::json!(target_xp);
            parts.actor_definition_mut(1)["magic_resistance"]["natural_save_twentieths"] =
                serde_json::json!(0);
        },
    )
}

fn apply_poison(engine: &mut Engine) {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "poison".to_string(),
                target: actor_target(),
                authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
            },
        )
        .expect("poison should apply");
}

#[test]
fn delayed_poison_retains_directed_and_area_credit() {
    let mut directed = poison_engine(13);
    apply_poison(&mut directed);
    let events = directed
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("directed poison should tick");
    assert!(events.events.iter().any(|event| matches!(
        event,
        Event::DefeatRewardEvaluated {
            awarded_experience: 13,
            ..
        }
    )));

    let mut area = poison_engine(13);
    apply_poison(&mut area);
    let target_index = area
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "target")
        .expect("target");
    area.world_mut().actors[target_index].active_effects[0]
        .spell_damage_credit
        .as_mut()
        .expect("spell credit")
        .reward_class = SpellDamageRewardClass::AreaOrIllusion;
    let events = area
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("area poison should tick");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::DefeatRewardEvaluated {
            awarded_experience: 5,
            ..
        }
    )));
}

#[test]
fn spell_kill_zero_reasons_do_not_award_experience() {
    let mut player_target = direct_damage_engine("character", 3, 9);
    let mut target_character = player_target.world().actors[0]
        .character
        .clone()
        .expect("source player character");
    target_character.resources.hp = 3;
    let target_social = player_target.world().actors[0].social.clone();
    let target_index = player_target
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "target")
        .expect("target");
    let target = &mut player_target.world_mut().actors[target_index];
    target.kind = ActorKind::Player;
    target.social = target_social;
    target.corpse_disposition = CorpseDisposition::SearchableCorpse;
    target.ai = None;
    target.xp_value = 0;
    target.character_id = Some(
        serde_json::from_value(serde_json::json!("character:magic_rewards:target"))
            .expect("target character id"),
    );
    target.character = Some(target_character);
    let player_target_events = cast_spark(&mut player_target, actor_target());
    assert!(player_target_events.iter().any(|event| matches!(
        event,
        Event::DefeatRewardEvaluated { awarded_experience: 0, reason, .. }
            if reason == "player_target"
    )));

    let mut owned = direct_damage_engine("character", 3, 9);
    let target_index = owned
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "target")
        .expect("target");
    let now = owned.world().timing.now;
    owned.world_mut().actors[target_index].summoned = Some(tme_rules::model::SummonedActorState {
        instance_id: "summon:test".into(),
        owner_id: "player".into(),
        source_spell_id: "summon_test".to_string(),
        template_id: "test".to_string(),
        remaining_rounds: None,
        last_ticked_at: now,
    });
    let owned_before = owned.world().clone();
    let owned_error = owned
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "spark".to_string(),
                target: actor_target(),
                authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
            },
        )
        .expect_err("owned summon is a non-overridable hostile target");
    assert!(owned_error.message().contains("invalid_hostile_target"));
    assert_eq!(owned.world(), &owned_before);

    let mut zero = direct_damage_engine("character", 3, 0);
    let zero_events = cast_spark(&mut zero, actor_target());
    assert!(zero_events.iter().any(|event| matches!(
        event,
        Event::DefeatRewardEvaluated { awarded_experience: 0, reason, .. }
            if reason == "zero_authored_experience"
    )));

    for events in [&player_target_events, &zero_events] {
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::ExperienceAwarded { .. }))
        );
    }
}

#[test]
fn delayed_credit_handles_noncharacter_and_stale_casters_without_xp() {
    let mut noncharacter = poison_engine(9);
    apply_poison(&mut noncharacter);
    let caster = &mut noncharacter.world_mut().actors[0];
    caster.character = None;
    caster.character_id = None;
    caster.social.alignment_source = SocialAlignmentSource::Inherent {
        alignment: CharacterAlignment::Lawful,
    };
    let events = noncharacter
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("noncharacter poison should tick");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::DefeatRewardEvaluated { awarded_experience: 0, reason, .. }
            if reason == "no_eligible_player_contribution"
    )));
    assert!(
        !events
            .events
            .iter()
            .any(|event| matches!(event, Event::ExperienceAwarded { .. }))
    );

    let mut stale = poison_engine(9);
    apply_poison(&mut stale);
    let target_index = stale
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "target")
        .expect("target");
    stale.world_mut().actors[target_index].active_effects[0]
        .spell_damage_credit
        .as_mut()
        .expect("spell credit")
        .caster_actor_id = "missing_caster".into();
    let events = stale
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("stale-caster poison should tick");
    assert!(events.events.iter().any(|event| matches!(
        event,
        Event::DefeatRewardEvaluated { awarded_experience: 0, reason, .. }
            if reason == "no_eligible_player_contribution"
    )));
    assert!(
        !events
            .events
            .iter()
            .any(|event| matches!(event, Event::ExperienceAwarded { .. }))
    );
}
