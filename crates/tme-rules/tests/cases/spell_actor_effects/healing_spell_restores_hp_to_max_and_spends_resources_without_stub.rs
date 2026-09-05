use super::*;

#[test]
fn healing_spell_restores_hp_to_max_and_spends_resources_without_stub() {
    let mut engine = br_effect_spell_engine_with_player_hp(&["mend"], 6);

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "mend".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("healing spell should cast");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellHealed {
            caster_id,
            spell_id,
            target_id,
            location,
            amount: 4,
            hp: 10,
            ..
        } if caster_id == "player"
            && spell_id == "mend"
            && target_id == "player"
            && location.level == "room_0"
            && location.position == Coord { x: 1, y: 1 }
    )));
    assert!(!events.iter().any(
        |event| matches!(event, Event::SpellCastStubbed { spell_id, .. } if spell_id == "mend")
    ));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .hp,
        10
    );
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .character
            .as_ref()
            .unwrap()
            .resources
            .hp,
        10
    );
}

#[test]
fn direct_spell_damage_reaches_the_shared_warmed_spell_fizzle_hook() {
    let mut engine = br_effect_spell_engine_with_player_hp_mutate(&["spark"], 10, |parts| {
        parts.rules_source_mut()["magic"]["damage_interruption"]["numerator"] =
            serde_json::json!(1);
        parts.rules_source_mut()["magic"]["damage_interruption"]["denominator"] =
            serde_json::json!(100);
    });
    let target_index = engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "target")
        .expect("target index");
    engine.world_mut().actors[target_index].warmed_spell = Some(WarmedSpellState {
        spell_id: "target_warmed_spell".to_string(),
        warmed_at: LogicalTime::FIRST,
        ready_at: LogicalTime::new(99),
        status: WarmedSpellStatus::Warming,
    });
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

    let damaged = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::SpellDamaged { target_id, .. } if target_id == "target"
            )
        })
        .expect("spell damage event");
    let fizzled = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::SpellFizzled {
                    spell_id,
                    cause: SpellFizzleCause::Damage { .. },
                    ..
                } if spell_id == "target_warmed_spell"
            )
        })
        .expect("damage fizzle event");
    assert!(damaged < fizzled);
    assert!(engine.world().actors[target_index].warmed_spell.is_none());
}

#[test]
fn successful_negate_save_emits_before_dispatch_and_preserves_warmed_spell() {
    let mut engine = br_effect_spell_engine_with_player_hp_mutate(&["spark"], 10, |parts| {
        parts.actor_definition_mut(1)["magic_resistance"]["natural_save_twentieths"] =
            serde_json::json!(20);
        let spark = parts.selected_by_runtime_id_mut("spells", "spark");
        spark["effect"]["resistance"] = serde_json::json!({
            "role": "incoming",
            "tag": "arcane",
            "mitigation": {"mode": "negate"}
        });
        parts.rules_source_mut()["magic"]["damage_interruption"]["numerator"] =
            serde_json::json!(1);
        parts.rules_source_mut()["magic"]["damage_interruption"]["denominator"] =
            serde_json::json!(100);
    });
    let target_index = engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "target")
        .expect("target index");
    let hp_before = engine.world().actors[target_index].hp;
    engine.world_mut().actors[target_index].warmed_spell = Some(WarmedSpellState {
        spell_id: "target_warmed_spell".to_string(),
        warmed_at: LogicalTime::FIRST,
        ready_at: LogicalTime::new(99),
        status: WarmedSpellStatus::Warming,
    });
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
        .expect("negated spark resolves");

    let save_index = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::SpellSaveResolved {
                    actor_id,
                    effect_id,
                    save_twentieths: 20,
                    success: true,
                    mitigation_mode: Some(SpellResistanceMitigationMode::Negate),
                    requested_damage: Some(3),
                    resolved_damage: Some(0),
                    ..
                } if actor_id == "target" && effect_id == "spark"
            )
        })
        .expect("exact successful save receipt");
    let practice_index = events
        .iter()
        .position(|event| matches!(event, Event::SkillPracticeAwarded { .. }))
        .expect("casting practice remains current");
    assert!(save_index < practice_index);
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::SpellDamaged { target_id, .. } if target_id == "target"
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::SpellFizzled { spell_id, .. } if spell_id == "target_warmed_spell"
    )));
    assert_eq!(engine.world().actors[target_index].hp, hp_before);
    assert_eq!(
        engine.world().actors[target_index]
            .warmed_spell
            .as_ref()
            .map(|slot| slot.spell_id.as_str()),
        Some("target_warmed_spell")
    );
}

#[test]
fn attribute_buff_spell_applies_spell_sourced_active_effect() {
    let mut engine = br_effect_spell_engine(&["strength"]);

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "strength".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("strength should cast");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::EffectApplied {
            actor_id,
            source_kind,
            source_id,
            effect_id,
            kind,
            potency: 2,
            remaining_rounds: Some(2),
            ..
        } if actor_id == "player"
            && source_kind == "spell"
            && source_id == "strength"
            && effect_id == "strength"
            && kind == "attribute_buff"
    )));
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    assert_eq!(player.active_effects.len(), 1);
    assert_eq!(
        player.active_effects[0].last_ticked_at,
        engine.world().timing.now
    );
}

#[test]
fn curse_spell_applies_spell_sourced_active_effect_to_actor_target() {
    let mut engine = br_effect_spell_engine(&["hex"]);

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "hex".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("hex should cast");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::EffectApplied {
            actor_id,
            source_kind,
            source_id,
            effect_id,
            kind,
            tags,
            potency: 2,
            remaining_rounds: Some(2),
            ..
        } if actor_id == "target"
            && source_kind == "spell"
            && source_id == "hex"
            && effect_id == "hex"
            && kind == "curse"
            && tags == &vec!["cursed".to_string()]
    )));
    let target = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "target")
        .expect("target actor");
    assert_eq!(target.active_effects.len(), 1);
    assert_eq!(target.active_effects[0].source.kind, "spell");
    assert_eq!(target.active_effects[0].source.id, "hex");
    assert_eq!(target.active_effects[0].kind, "curse");
}

#[test]
fn curse_without_resistance_policy_is_not_resisted_by_matching_tag() {
    let mut engine = br_effect_spell_engine_with_player_hp_mutate(&["hex"], 10, |parts| {
        parts.actors_mut()[1]["active_effects"] = serde_json::json!([
            {
                "instance_id": "ward_1",
                "effect_id": "steady_ward",
                "source": {"kind": "fixture", "id": "br_effect_spell_test"},
                "kind": "protection",
                "tags": ["ward"],
                "potency": 1,
                "remaining_rounds": 3,
                "stacking": "replace_same_kind",
                "start_delay_rounds": 0,
                "tick_interval_rounds": 1,
                "suppresses_action": false,
                "resistance_boosts": [{"tag": "cursed", "bonus_twentieths": 3}]
            }
        ]);
    });

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "hex".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("hex should still apply");

    assert!(!events.iter().any(
        |event| matches!(event, Event::SpellSaveResolved { effect_id, .. } if effect_id == "hex")
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::EffectApplied {
            actor_id,
            effect_id,
            kind,
            ..
        } if actor_id == "target" && effect_id == "hex" && kind == "curse"
    )));
    let target = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "target")
        .expect("target actor");
    assert!(
        target
            .active_effects
            .iter()
            .any(|effect| effect.effect_id == "hex")
    );
}

#[test]
fn attribute_buff_without_resistance_policy_is_not_resisted_by_matching_tag() {
    let mut engine = br_effect_spell_engine_with_player_hp_mutate(&["strength"], 10, |parts| {
        parts.actors_mut()[0]["active_effects"] = serde_json::json!([
            {
                "instance_id": "ward_1",
                "effect_id": "steady_ward",
                "source": {"kind": "fixture", "id": "br_effect_spell_test"},
                "kind": "protection",
                "tags": ["ward"],
                "potency": 1,
                "remaining_rounds": 3,
                "stacking": "replace_same_kind",
                "start_delay_rounds": 0,
                "tick_interval_rounds": 1,
                "suppresses_action": false,
                "resistance_boosts": [{"tag": "strength", "bonus_twentieths": 3}]
            }
        ]);
    });

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "strength".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("strength should still apply");

    assert!(!events.iter().any(
        |event| matches!(event, Event::SpellSaveResolved { effect_id, .. } if effect_id == "strength")
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::EffectApplied {
            actor_id,
            effect_id,
            kind,
            ..
        } if actor_id == "player" && effect_id == "strength" && kind == "attribute_buff"
    )));
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    assert!(
        player
            .active_effects
            .iter()
            .any(|effect| effect.effect_id == "strength")
    );
}

#[test]
fn replace_same_kind_spell_effect_replaces_existing_spell_effect() {
    let mut engine = br_effect_spell_engine(&["strength"]);

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "strength".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("first strength should cast");
    let first_instance_id = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .active_effects[0]
        .instance_id
        .clone();

    let events = engine
        .apply_realtime_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "strength".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("second strength should cast");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::EffectApplied {
            actor_id,
            source_id,
            effect_id,
            kind,
            remaining_rounds: Some(2),
            ..
        } if actor_id == "player"
            && source_id == "strength"
            && effect_id == "strength"
            && kind == "attribute_buff"
    )));
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    assert_eq!(player.active_effects.len(), 1);
    assert_ne!(player.active_effects[0].instance_id, first_instance_id);
    assert_eq!(player.active_effects[0].remaining_rounds, Some(2));
}

#[test]
fn refresh_duration_spell_effect_updates_existing_effect_without_new_stack() {
    let mut engine =
        br_effect_spell_engine_with_effect_mutate(&["strength"], "strength", |effect| {
            effect["stacking"] = serde_json::json!("refresh_duration")
        });

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "strength".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("first strength should cast");
    let first_instance_id = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .active_effects[0]
        .instance_id
        .clone();
    engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == "player")
        .expect("player")
        .active_effects[0]
        .remaining_rounds = Some(1);

    let events = engine
        .apply_realtime_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "strength".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("second strength should cast");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::EffectApplied {
            actor_id,
            source_id,
            effect_id,
            remaining_rounds: Some(2),
            ..
        } if actor_id == "player" && source_id == "strength" && effect_id == "strength"
    )));
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    assert_eq!(player.active_effects.len(), 1);
    assert_eq!(player.active_effects[0].instance_id, first_instance_id);
    assert_eq!(player.active_effects[0].remaining_rounds, Some(2));
}

#[test]
fn stack_instance_spell_effect_keeps_existing_effect_and_adds_new_instance() {
    let mut engine =
        br_effect_spell_engine_with_effect_mutate(&["strength"], "strength", |effect| {
            effect["stacking"] = serde_json::json!("stack_instance")
        });

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "strength".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("first strength should cast");
    let first_instance_id = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .active_effects[0]
        .instance_id
        .clone();

    engine
        .apply_realtime_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "strength".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("second strength should cast");

    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    assert_eq!(player.active_effects.len(), 2);
    assert_eq!(player.active_effects[0].instance_id, first_instance_id);
    assert_ne!(
        player.active_effects[0].instance_id,
        player.active_effects[1].instance_id
    );
    assert_eq!(player.active_effects[0].remaining_rounds, Some(1));
    assert_eq!(player.active_effects[1].remaining_rounds, Some(2));
}

#[test]
fn control_status_spell_applies_suppressing_effect_and_blocks_non_passive_actions() {
    let mut engine = bs_runtime_spell_engine(
        &["terror", "spark"],
        vec!["####", "#..#", "####"],
        Coord { x: 2, y: 1 },
    );

    let cast_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "terror".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("terror should cast");

    assert!(!cast_events.iter().any(
        |event| matches!(event, Event::SpellCastStubbed { spell_id, .. } if spell_id == "terror")
    ));
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    assert!(player.active_effects.iter().any(|effect| {
        effect.effect_id == "terror"
            && effect.kind == "control_status"
            && effect.tags == vec!["fear".to_string()]
            && effect.suppresses_action
    }));

    let move_status = engine
        .validate_actor_command(&PlayerCommandV1 {
            contract_version: COMMAND_CONTRACT_VERSION,
            actor_id: "player".into(),
            intent: PlayerIntentPayloadV1::MovePath {
                path: vec![tme_rules::Direction::East],
            },
        })
        .expect("move validation should succeed");
    assert!(!move_status.accepted);
    assert_eq!(
        move_status.blocked_reason,
        Some(ActionBlockedReasonV1::SuppressedByStatus)
    );

    let cast_status = engine
        .validate_actor_command(&PlayerCommandV1 {
            contract_version: COMMAND_CONTRACT_VERSION,
            actor_id: "player".into(),
            intent: PlayerIntentPayloadV1::CastSpell {
                spell_id: "spark".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        })
        .expect("cast validation should succeed");
    assert!(!cast_status.accepted);
    assert_eq!(
        cast_status.blocked_reason,
        Some(ActionBlockedReasonV1::SuppressedByStatus)
    );

    let wait_status = engine
        .validate_actor_command(&PlayerCommandV1 {
            contract_version: COMMAND_CONTRACT_VERSION,
            actor_id: "player".into(),
            intent: PlayerIntentPayloadV1::Wait,
        })
        .expect("wait validation should succeed");
    assert!(wait_status.accepted);
    assert_eq!(wait_status.blocked_reason, None);
}

#[test]
fn protection_spell_grants_resistance_boost_and_successful_save_negates_control() {
    let mut engine = bs_runtime_spell_engine(
        &["ward_target", "hold"],
        vec!["#####", "#...#", "#####"],
        Coord { x: 3, y: 1 },
    );

    let ward_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "ward_target".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("ward should cast");

    assert!(!ward_events.iter().any(|event| matches!(
        event,
        Event::SpellCastStubbed { spell_id, .. } if spell_id == "ward_target"
    )));
    let target = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "target")
        .expect("target actor");
    assert_eq!(target.active_effects.len(), 1);
    assert_eq!(target.active_effects[0].resistance_boosts.len(), 1);
    assert_eq!(target.active_effects[0].resistance_boosts[0].tag, "mind");

    let hold_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "hold".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("hold should resolve");

    assert!(!hold_events.iter().any(
        |event| matches!(event, Event::SpellCastStubbed { spell_id, .. } if spell_id == "hold")
    ));
    assert!(hold_events.iter().any(|event| matches!(
        event,
        Event::SpellSaveResolved {
            actor_id,
            effect_id,
            resistance_tag,
            ..
        } if actor_id == "target" && effect_id == "hold" && resistance_tag == "mind"
    )));
    let target = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "target")
        .expect("target actor");
    assert_eq!(target.active_effects.len(), 1);
    assert!(
        !target
            .active_effects
            .iter()
            .any(|effect| effect.effect_id == "hold")
    );
}
