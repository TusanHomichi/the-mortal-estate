use crate::spell_effect_support::*;
use crate::spell_support::*;
use crate::support::content_parts::ContentParts;
use tme_rules::*;

fn disable_recovery(parts: &mut ContentParts) {
    parts.rules_source_mut()["resources"]["recovery_interval_units"] = serde_json::json!(100);
}

fn larger_warmup_without_recovery(parts: &mut ContentParts) {
    parts.rules_source_mut()["magic"]["warmup"]["units"] = serde_json::json!(2);
    disable_recovery(parts);
}

fn force_damage_interruption(parts: &mut ContentParts) {
    parts.rules_source_mut()["magic"]["damage_interruption"]["numerator"] = serde_json::json!(1);
    parts.rules_source_mut()["magic"]["damage_interruption"]["denominator"] =
        serde_json::json!(100);
}

fn set_player_mp(engine: &mut Engine, mp: i32) {
    let player_index = engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.kind == ActorKind::Player)
        .expect("player");
    let player = &mut engine.world_mut().actors[player_index];
    player.mp = mp;
    player.character.as_mut().expect("character").resources.mp = mp;
}

fn warmed_spell(engine: &Engine) -> &WarmedSpellState {
    engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .and_then(|player| player.warmed_spell.as_ref())
        .expect("warmed spell")
}

#[test]
fn direct_stub_cast_spends_resources_and_awards_lane_practice() {
    let mut engine = wizard_spell_engine_with_content_mutate(Some("spark"), 1, disable_recovery);
    let mp_before = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .mp;
    let stamina_before = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .stamina;
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "spark".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("direct cast");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellCastCommitted {
            spell_id,
            casting_method: SpellCastingMethod::Direct,
            mp_cost: Some(3),
            stamina_cost: Some(1),
            ..
        } if spell_id == "spark"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellCastStubbed {
            spell_id,
            casting_method: SpellCastingMethod::Direct,
            ..
        } if spell_id == "spark"
    )));
    assert_eq!(count_skill_practice(&events.events, "wizard_magic"), 1);
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .mp,
        mp_before - 3
    );
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .stamina,
        stamina_before - 1
    );
}

#[test]
fn direct_damage_spell_mutates_target_without_stub() {
    let mut engine = br_effect_spell_engine(&["spark"]);
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "spark".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("damage spell");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellCastCommitted {
            spell_id,
            casting_method: SpellCastingMethod::Direct,
            ..
        } if spell_id == "spark"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellDamaged {
            spell_id,
            target_id,
            damage: 3,
            hp: 5,
            ..
        } if spell_id == "spark" && target_id == "target"
    )));
    assert_eq!(count_stubbed_casts(&events.events, "spark"), 0);
}

#[test]
fn warming_costs_no_resources_and_is_ready_at_next_returned_opportunity() {
    let mut engine =
        wizard_spell_engine_with_content_mutate(Some("charged_spark"), 1, disable_recovery);
    set_player_mp(&mut engine, 0);
    let stamina_before = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .stamina;

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("warming does not require cast resources");

    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .mp,
        0
    );
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .stamina,
        stamina_before
    );
    assert_eq!(warmed_spell(&engine).status, WarmedSpellStatus::Ready);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellWarmed { spell_id, .. } if spell_id == "charged_spark"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::WarmedSpellReady { spell_id, .. } if spell_id == "charged_spark"
    )));
}

#[test]
fn larger_warmup_remains_warming_until_its_boundary() {
    let mut engine = wizard_spell_engine_with_content_mutate(
        Some("charged_spark"),
        1,
        larger_warmup_without_recovery,
    );

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("warm");
    assert_eq!(warmed_spell(&engine).status, WarmedSpellStatus::Warming);
    assert_eq!(warmed_spell(&engine).ready_at, LogicalTime::new(3));

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("wait");
    assert_eq!(warmed_spell(&engine).status, WarmedSpellStatus::Ready);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::WarmedSpellReady { .. }))
            .count(),
        1
    );
}

#[test]
fn commands_enforce_direct_and_warm_then_cast_methods() {
    let mut direct = wizard_spell_engine(Some("spark"), 1);
    let error = direct
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "spark".to_string(),
            },
        )
        .expect_err("direct spell cannot warm");
    assert_eq!(error.to_string(), "spell_casts_directly");

    let mut warmed = wizard_spell_engine(Some("charged_spark"), 1);
    let error = warmed
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "charged_spark".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("warm spell cannot cast directly");
    assert_eq!(error.to_string(), "spell_requires_warming");
}

#[test]
fn replacement_fizzles_before_new_warm_without_cost() {
    let mut engine = wizard_multi_spell_engine_with_content_mutate(
        &["charged_spark", "charged_mend"],
        1,
        vec!["####", "#..#", "####"],
        Coord { x: 2, y: 1 },
        disable_recovery,
    );
    let mp_before = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .mp;
    let stamina_before = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .stamina;
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("first warm");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_mend".to_string(),
            },
        )
        .expect("replacement warm");

    let fizzle = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::SpellFizzled {
                    spell_id,
                    cause: SpellFizzleCause::Replaced { replacing_spell_id, .. },
                    ..
                } if spell_id == "charged_spark" && replacing_spell_id == "charged_mend"
            )
        })
        .expect("replacement fizzle");
    let warm = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::SpellWarmed { spell_id, .. } if spell_id == "charged_mend"
            )
        })
        .expect("new warm");
    assert!(fizzle < warm);
    assert_eq!(warmed_spell(&engine).spell_id, "charged_mend");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .mp,
        mp_before
    );
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .stamina,
        stamina_before
    );
}

#[test]
fn explicit_fizzle_and_rest_cost_no_mp_and_rest_accepts_empty_slot() {
    let mut engine =
        wizard_spell_engine_with_content_mutate(Some("charged_spark"), 1, disable_recovery);
    let mp_before = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .mp;
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("warm");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::FizzleWarmedSpell,
        )
        .expect("explicit fizzle");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellFizzled {
            cause: SpellFizzleCause::Canceled,
            ..
        }
    )));
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .warmed_spell
            .is_none()
    );
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .mp,
        mp_before
    );

    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Rest)
        .expect("empty rest");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("warm again");
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Rest)
        .expect("rest");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellFizzled {
            cause: SpellFizzleCause::Rest,
            ..
        }
    )));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .mp,
        mp_before
    );
}

#[test]
fn movement_wait_inspect_show_sack_and_physical_attack_preserve_slot() {
    let mut engine = wizard_multi_spell_engine(
        &["charged_spark"],
        1,
        vec!["#####", "#...#", "#####"],
        Coord { x: 3, y: 1 },
    );
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("warm");
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Inspect)
        .expect("inspect");
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::ShowSack)
        .expect("show sack");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("move");
    engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("wait");
    assert_eq!(warmed_spell(&engine).spell_id, "charged_spark");

    let mut physical = wizard_multi_spell_engine(
        &["charged_spark"],
        1,
        vec!["####", "#..#", "####"],
        Coord { x: 1, y: 1 },
    );
    physical
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("warm for physical action");
    physical
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: PhysicalAttackMode::Fight,
                target_actor_id: "target".into(),
            },
        )
        .expect("physical attack");
    assert_eq!(warmed_spell(&physical).spell_id, "charged_spark");
}

#[test]
fn insufficient_resources_leave_ready_warmed_spell_intact() {
    let mut engine =
        wizard_spell_engine_with_content_mutate(Some("charged_spark"), 1, disable_recovery);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("warm");
    set_player_mp(&mut engine, 0);
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastWarmedSpell {
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("insufficient MP");
    assert_eq!(error.to_string(), "insufficient_magic_points");
    assert_eq!(warmed_spell(&engine).status, WarmedSpellStatus::Ready);
}

#[test]
fn successful_warmed_cast_spends_once_clears_slot_and_awards_practice() {
    let mut engine =
        wizard_spell_engine_with_content_mutate(Some("charged_spark"), 1, disable_recovery);
    let mp_before = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .mp;
    let stamina_before = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .stamina;
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("warm");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastWarmedSpell {
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("warmed cast");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellCastCommitted {
            spell_id,
            casting_method: SpellCastingMethod::WarmThenCast,
            mp_cost: Some(4),
            stamina_cost: Some(1),
            ..
        } if spell_id == "charged_spark"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::WarmedSpellCast { spell_id, .. } if spell_id == "charged_spark"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellCastStubbed {
            spell_id,
            casting_method: SpellCastingMethod::WarmThenCast,
            ..
        } if spell_id == "charged_spark"
    )));
    assert_eq!(count_skill_practice(&events.events, "wizard_magic"), 1);
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .warmed_spell
            .is_none()
    );
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .mp,
        mp_before - 4
    );
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .stamina,
        stamina_before - 1
    );
}

#[test]
fn failed_unrelated_intent_rolls_back_and_preserves_slot() {
    let mut engine = wizard_spell_engine(Some("charged_spark"), 1);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("warm");
    let before = warmed_spell(&engine).clone();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "missing".to_string(),
                destination: ItemMoveDestination::GroundHere,
            },
        )
        .expect_err("missing item");
    assert_eq!(warmed_spell(&engine), &before);
}

#[test]
fn warmed_self_and_no_target_spells_use_slot_identity() {
    for (spell_id, target) in [
        ("charged_mend", Some(SpellTarget::SelfTarget)),
        ("charged_ward", None),
    ] {
        let mut engine = wizard_spell_engine(Some(spell_id), 1);
        engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::WarmSpell {
                    spell_id: spell_id.to_string(),
                },
            )
            .expect("warm");
        let events = engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::CastWarmedSpell {
                    target,
                    authorization: tme_rules::HostilityAuthorization::Safe,
                },
            )
            .expect("cast warmed");
        assert!(events.iter().any(|event| matches!(
            event,
            Event::WarmedSpellCast {
                spell_id: event_spell_id,
                ..
            } if event_spell_id == spell_id
        )));
    }
}

#[test]
fn lane_practice_routes_and_knight_direct_cast_skips_magic_practice() {
    for (class_id, lane, spell_id) in [
        ("wizard", "wizard_magic", "wizard_bolt"),
        ("thaumaturge", "thaumaturge_magic", "thaum_bolt"),
        ("thief", "thief_magic", "thief_bolt"),
    ] {
        let mut engine = spell_lane_engine(class_id, lane, spell_id);
        let events = engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::CastSpell {
                    spell_id: spell_id.to_string(),
                    target: Some(SpellTarget::Actor {
                        actor_id: "target".into(),
                    }),
                    authorization: tme_rules::HostilityAuthorization::Safe,
                },
            )
            .expect("lane cast");
        assert_eq!(count_skill_practice(&events.events, lane), 1);
    }

    let mut knight = spell_lane_engine("knight", "knight_magic", "knight_bolt");
    let events = knight
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "knight_bolt".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("knight direct cast");
    assert_eq!(count_skill_practice(&events.events, "knight_magic"), 0);
}

#[test]
fn automatic_physical_damage_after_ready_transition_fizzles_before_player_opportunity() {
    let mut engine = wizard_multi_spell_engine_with_content_mutate(
        &["charged_spark"],
        1,
        vec!["####", "#..#", "####"],
        Coord { x: 1, y: 1 },
        force_damage_interruption,
    );
    let player_index = engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "player")
        .expect("player index");
    let target_index = engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "target")
        .expect("target index");
    {
        let player = &mut engine.world_mut().actors[player_index];
        player.hp = 100;
        player.stats.hp = 100;
        player.stats.defense = 0;
        let resources = &mut player
            .character
            .as_mut()
            .expect("player character")
            .resources;
        resources.hp = 100;
        resources.max_hp = 100;
        resources.peak_hp = 100;
    }
    engine.world_mut().actors[target_index].stats.attack = 30;
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("warming action with automatic attack");

    let damaged = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::Attacked {
                    defender_id,
                    damage,
                    ..
                } if defender_id == "player" && *damage > 0
            )
        })
        .expect("automatic physical damage");
    let ready = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::WarmedSpellReady { spell_id, .. } if spell_id == "charged_spark"
            )
        })
        .expect("spell reaches ready at the boundary");
    let fizzled = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::SpellFizzled {
                    spell_id,
                    cause: SpellFizzleCause::Damage { .. },
                    ..
                } if spell_id == "charged_spark"
            )
        })
        .expect("damage fizzle");
    assert!(ready < damaged && damaged < fizzled);
    assert!(engine.world().actors[player_index].warmed_spell.is_none());
}

#[test]
fn wrong_class_skill_and_mp_failures_do_not_mutate_state() {
    let mut fighter = wizard_spell_engine(Some("spark"), 1);
    fighter.world_mut().actors[0]
        .character
        .as_mut()
        .expect("character")
        .identity
        .current_class_id = "fighter".to_string();
    assert_eq!(
        fighter
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::CastSpell {
                    spell_id: "spark".to_string(),
                    target: Some(SpellTarget::Actor {
                        actor_id: "target".into(),
                    }),
                    authorization: tme_rules::HostilityAuthorization::Safe,
                },
            )
            .expect_err("fighter lane")
            .to_string(),
        "wrong_class"
    );

    let mut low_skill = wizard_spell_engine(Some("mend"), 1);
    assert_eq!(
        low_skill
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::CastSpell {
                    spell_id: "mend".to_string(),
                    target: Some(SpellTarget::SelfTarget),
                    authorization: tme_rules::HostilityAuthorization::Safe,
                },
            )
            .expect_err("skill")
            .to_string(),
        "skill_level_too_low"
    );

    let mut no_mp = wizard_spell_engine(Some("spark"), 1);
    set_player_mp(&mut no_mp, 0);
    assert_eq!(
        no_mp
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::CastSpell {
                    spell_id: "spark".to_string(),
                    target: Some(SpellTarget::Actor {
                        actor_id: "target".into(),
                    }),
                    authorization: tme_rules::HostilityAuthorization::Safe,
                },
            )
            .expect_err("MP")
            .to_string(),
        "insufficient_magic_points"
    );
}

fn thaum_spell(method: &str, requirement: i32, mp_cost: i32) -> serde_json::Value {
    serde_json::json!({
        "id": "thaum_test",
        "name": "Thaum Test",
        "status": "stub",
        "lane": "thaumaturge_magic",
        "skill_requirement": requirement,
        "mp_cost": mp_cost,
        "stamina_cost": 1,
        "target": {"kind": "actor", "range": 3, "requires_visible": true},
        "casting": {"method": method, "cast_class": "character"}
    })
}

fn cast_thaum_direct(engine: &mut Engine) -> Vec<Event> {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "thaum_test".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("committed Thaum attempt")
        .events
}

#[test]
fn thaum_above_skill_direct_attempt_commits_resources_then_succeeds_or_fails_once() {
    let mut failure = spell_lane_engine_with_spell_and_seed_mutate(
        "thaumaturge",
        "thaumaturge_magic",
        thaum_spell("direct", 3, 4),
        4,
        disable_recovery,
    );
    let mp_before = failure
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .mp;
    let stamina_before = failure
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .stamina;
    let events = cast_thaum_direct(&mut failure);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::ThaumAboveSkillEvaluated { .. }))
            .count(),
        1
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ThaumAboveSkillEvaluated {
            current_skill_level: 1,
            skill_requirement: 3,
            gap: 2,
            roll_denominator: 20,
            success_threshold: 18,
            roll: 20,
            success: false,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellCastFailed {
            target: Some(SpellTarget::Actor { actor_id }),
            failure: SpellCastFailure::AboveSkillAttempt,
            mp_cost: Some(4),
            stamina_cost: Some(1),
            ..
        } if actor_id == "target"
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::SpellCastStubbed { .. }
            | Event::MagicPracticeEvaluated { .. }
            | Event::SkillPracticeAwarded { .. }
            | Event::SpellSaveResolved { .. }
    )));
    assert_eq!(
        failure
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .mp,
        mp_before - 4
    );
    assert_eq!(
        failure
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .stamina,
        stamina_before - 1
    );

    let mut success = spell_lane_engine_with_spell_and_seed_mutate(
        "thaumaturge",
        "thaumaturge_magic",
        thaum_spell("direct", 3, 4),
        5,
        disable_recovery,
    );
    let events = cast_thaum_direct(&mut success);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ThaumAboveSkillEvaluated {
            roll: 17,
            success: true,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellCastStubbed { spell_id, .. } if spell_id == "thaum_test"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::MagicPracticeEvaluated {
            primary_attribute: Some(MagicPrimaryAttribute::Wisdom),
            mp_cost: 4,
            total_raw_points: 5,
            risk_applied: false,
            reason,
            ..
        } if reason == "eligible_successful_cast"
    )));
}

#[test]
fn thaum_above_skill_reads_do_not_roll_or_mutate_the_future_attempt() {
    let mut baseline = spell_lane_engine_with_spell_and_seed(
        "thaumaturge",
        "thaumaturge_magic",
        thaum_spell("direct", 3, 4),
        4,
    );
    let mut after_reads = spell_lane_engine_with_spell_and_seed(
        "thaumaturge",
        "thaumaturge_magic",
        thaum_spell("direct", 3, 4),
        4,
    );
    let actor_id = after_reads
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .id
        .clone();
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id,
        intent: PlayerIntentPayloadV1::CastSpell {
            spell_id: "thaum_test".to_string(),
            target: Some(SpellTarget::Actor {
                actor_id: "target".into(),
            }),
            authorization: tme_rules::HostilityAuthorization::Safe,
        },
    };
    assert!(
        after_reads
            .validate_actor_command(&command)
            .expect("validation read")
            .accepted
    );
    assert!(
        !after_reads
            .actor_action_options(&tme_rules::ActorId::from("player"))
            .expect("option read")
            .is_empty()
    );
    after_reads
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("observed context read");

    let baseline_events = cast_thaum_direct(&mut baseline);
    let events_after_reads = cast_thaum_direct(&mut after_reads);
    assert_eq!(events_after_reads, baseline_events);
    assert_eq!(after_reads.snapshot(), baseline.snapshot());
}

#[test]
fn warmed_thaum_failure_clears_slot_after_spending_and_applies_no_effect() {
    let mut engine = spell_lane_engine_with_spell_and_seed_mutate(
        "thaumaturge",
        "thaumaturge_magic",
        thaum_spell("warm_then_cast", 3, 4),
        4,
        disable_recovery,
    );
    let mp_before = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .mp;
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "thaum_test".to_string(),
            },
        )
        .expect("warm has no attempt roll");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastWarmedSpell {
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("committed warmed attempt failure");
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::WarmedSpellCast { .. }))
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ThaumAboveSkillEvaluated {
            roll: 20,
            success: false,
            ..
        }
    )));
    assert!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .warmed_spell
            .is_none()
    );
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .mp,
        mp_before - 4
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::SpellCastStubbed { .. }
            | Event::MagicPracticeEvaluated { .. }
            | Event::SkillPracticeAwarded { .. }
    )));
}

#[test]
fn thaum_attempt_threshold_decreases_by_gap_and_at_gate_uses_no_attempt() {
    for (requirement, expected_gap, expected_threshold) in [(2, 1, 19), (4, 3, 17)] {
        let mut engine = spell_lane_engine_with_spell_and_seed(
            "thaumaturge",
            "thaumaturge_magic",
            thaum_spell("direct", requirement, 2),
            5,
        );
        let events = cast_thaum_direct(&mut engine);
        assert!(events.iter().any(|event| matches!(
            event,
            Event::ThaumAboveSkillEvaluated { gap, success_threshold, .. }
                if *gap == expected_gap && *success_threshold == expected_threshold
        )));
    }

    let mut at_gate = spell_lane_engine_with_spell_and_seed(
        "thaumaturge",
        "thaumaturge_magic",
        thaum_spell("direct", 1, 2),
        4,
    );
    let events = cast_thaum_direct(&mut at_gate);
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::ThaumAboveSkillEvaluated { .. } | Event::SpellCastFailed { .. }
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::SpellCastStubbed { .. }))
    );
}

#[test]
fn magic_practice_uses_mp_and_primary_attribute_without_combat_risk() {
    for (class_id, lane, attribute) in [
        (
            "wizard",
            "wizard_magic",
            MagicPrimaryAttribute::Intelligence,
        ),
        ("thief", "thief_magic", MagicPrimaryAttribute::Intelligence),
        (
            "thaumaturge",
            "thaumaturge_magic",
            MagicPrimaryAttribute::Wisdom,
        ),
    ] {
        let spell = serde_json::json!({
            "id": "practice_spell",
            "name": "Practice Spell",
            "status": "stub",
            "lane": lane,
            "skill_requirement": 1,
            "mp_cost": 5,
            "target": {"kind": "actor", "range": 3, "requires_visible": true},
            "casting": {"method": "direct", "cast_class": "character"}
        });
        let mut engine = spell_lane_engine_with_spell(class_id, lane, spell);
        let events = engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::CastSpell {
                    spell_id: "practice_spell".to_string(),
                    target: Some(SpellTarget::Actor {
                        actor_id: "target".into(),
                    }),
                    authorization: tme_rules::HostilityAuthorization::Safe,
                },
            )
            .expect("practice cast");
        assert!(events.iter().any(|event| matches!(
            event,
            Event::MagicPracticeEvaluated {
                primary_attribute: Some(event_attribute),
                base_raw_points: 5,
                primary_attribute_bonus_raw_points: 1,
                total_raw_points: 6,
                risk_applied: false,
                reason,
                ..
            } if *event_attribute == attribute && reason == "eligible_successful_cast"
        )));
    }

    let mut knight = spell_lane_engine("knight", "knight_magic", "knight_bolt");
    let events = knight
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "knight_bolt".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("Knight spell");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::MagicPracticeEvaluated {
            primary_attribute: None,
            base_raw_points: 0,
            primary_attribute_bonus_raw_points: 0,
            total_raw_points: 0,
            risk_applied: false,
            reason,
            ..
        } if reason == "class_has_no_magic_practice"
    )));
    assert_eq!(count_skill_practice(&events.events, "knight_magic"), 0);
}
