use crate::spell_effect_support::*;
use crate::spell_support::*;
use tme_rules::*;

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
        player.active_effects[0]
            .last_ticked_at
            .saturating_add_rounds(1),
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
        .apply_actor_intent(
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
        .apply_actor_intent(
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
        .apply_actor_intent(
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

#[test]
fn poison_spell_applies_delayed_tick_damage_and_syncs_character_hp() {
    let mut engine = bs_runtime_spell_engine_mutate(
        &["self_poison"],
        vec!["#####", "#...#", "#####"],
        Coord { x: 3, y: 1 },
        |parts| {
            parts.rules_source_mut()["magic"]["damage_interruption"]["numerator"] =
                serde_json::json!(1);
            parts.rules_source_mut()["magic"]["damage_interruption"]["denominator"] =
                serde_json::json!(100);
        },
    );

    let cast_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "self_poison".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("self poison should cast");

    assert!(!cast_events.iter().any(|event| matches!(
        event,
        Event::SpellCastStubbed { spell_id, .. } if spell_id == "self_poison"
    )));
    assert!(!cast_events.iter().any(|event| matches!(
        event,
        Event::EffectTicked { effect_id, .. } if effect_id == "self_poison"
    )));
    let player_index = engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "player")
        .expect("player index");
    engine.world_mut().actors[player_index].warmed_spell = Some(WarmedSpellState {
        spell_id: "poison_interrupted_spell".to_string(),
        warmed_at: LogicalTime::FIRST,
        ready_at: LogicalTime::new(99),
        status: WarmedSpellStatus::Warming,
    });
    let wait_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("wait should advance poison");

    assert!(wait_events.iter().any(|event| matches!(
        event,
        Event::EffectTicked {
            actor_id,
            effect_id,
            kind,
            potency,
            remaining_rounds,
            ..
        } if actor_id == "player"
            && effect_id == "self_poison"
            && kind == "poison"
            && *potency == 2
            && *remaining_rounds == Some(2)
    )));
    let damaged = wait_events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::EffectDamaged { effect_id, .. } if effect_id == "self_poison"
            )
        })
        .expect("poison damage event");
    let fizzled = wait_events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::SpellFizzled {
                    spell_id,
                    cause: SpellFizzleCause::Damage { .. },
                    ..
                } if spell_id == "poison_interrupted_spell"
            )
        })
        .expect("poison damage fizzle");
    assert!(damaged < fizzled);
    assert!(engine.world().actors[player_index].warmed_spell.is_none());
    assert!(wait_events.iter().any(|event| matches!(
        event,
        Event::EffectDamaged {
            actor_id,
            effect_id,
            tags,
            damage,
            hp,
            ..
        } if actor_id == "player"
            && effect_id == "self_poison"
            && tags.iter().any(|tag| tag == "poison")
            && *damage == 2
            && *hp == 8
    )));
    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    assert_eq!(player.hp, 8);
    assert_eq!(
        player
            .character
            .as_ref()
            .expect("player character")
            .resources
            .hp,
        8
    );
    assert_eq!(
        player.active_effects[0].source_actor_id.as_deref(),
        Some("player")
    );
}

#[test]
fn caster_created_poison_retains_credit_through_lethal_delayed_damage() {
    let mut engine = bs_runtime_spell_engine_mutate(
        &["self_poison"],
        vec!["#####", "#...#", "#####"],
        Coord { x: 3, y: 1 },
        |parts| {
            parts.actor_definition_mut(0)["stats"]["hp"] = serde_json::json!(1);
            parts.actors_mut()[0]["character"]["resources"]["hp"] = serde_json::json!(1);
        },
    );

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "self_poison".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("self poison should cast");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .active_effects[0]
            .source_actor_id
            .as_deref(),
        Some("player")
    );

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("poison tick should defeat the player");
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::ActorDefeated {
                actor_id,
                cause: tme_rules::DeathCause::Poison,
                credited_actor_id: Some(credited_actor_id),
                ..
            } if actor_id == "player" && credited_actor_id == "player"
        )),
        "{events:#?}"
    );
    assert!(matches!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .life_state,
        tme_rules::ActorLifeState::Ghost { .. }
    ));
}

#[test]
fn poison_cure_removes_only_poison_effects() {
    let mut engine = bs_runtime_spell_engine_mutate(
        &["poison_cure"],
        vec!["#####", "#...#", "#####"],
        Coord { x: 3, y: 1 },
        |parts| {
            parts.actors_mut()[1]["active_effects"] = serde_json::json!([
                {
                    "instance_id": "venom_1",
                    "effect_id": "venom",
                    "source": {"kind": "fixture", "id": "bs_runtime_spell_test"},
                    "kind": "poison",
                    "tags": ["poison"],
                    "potency": 2,
                    "remaining_rounds": 3,
                    "stacking": "replace_same_kind",
                    "start_delay_rounds": 0,
                    "tick_interval_rounds": 1,
                    "suppresses_action": false,
                    "resistance_boosts": []
                },
                {
                    "instance_id": "toxin_ward_1",
                    "effect_id": "toxin_ward",
                    "source": {"kind": "fixture", "id": "bs_runtime_spell_test"},
                    "kind": "protection",
                    "tags": ["ward"],
                    "potency": 1,
                    "remaining_rounds": 3,
                    "stacking": "replace_same_kind",
                    "start_delay_rounds": 0,
                    "tick_interval_rounds": 1,
                    "suppresses_action": false,
                    "resistance_boosts": [{"tag": "poison", "bonus_twentieths": 3}]
                },
                {
                    "instance_id": "ward_1",
                    "effect_id": "ward",
                    "source": {"kind": "fixture", "id": "bs_runtime_spell_test"},
                    "kind": "protection",
                    "tags": ["ward"],
                    "potency": 1,
                    "remaining_rounds": 3,
                    "stacking": "replace_same_kind",
                    "start_delay_rounds": 0,
                    "tick_interval_rounds": 1,
                    "suppresses_action": false,
                    "resistance_boosts": []
                }
            ]);
        },
    );

    let cure_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "poison_cure".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("poison cure should cast");

    assert!(!cure_events.iter().any(|event| matches!(
        event,
        Event::SpellCastStubbed { spell_id, .. } if spell_id == "poison_cure"
    )));
    assert!(cure_events.iter().any(|event| matches!(
        event,
        Event::EffectRemoved {
            actor_id,
            instance_id,
            reason,
            ..
        } if actor_id == "target" && instance_id == "venom_1" && reason == "poison_cure"
    )));
    assert!(!cure_events.iter().any(|event| matches!(
        event,
        Event::EffectRemoved {
            actor_id,
            instance_id,
            ..
        } if actor_id == "target" && (instance_id == "toxin_ward_1" || instance_id == "ward_1")
    )));
    let target = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "target")
        .expect("target actor");
    assert_eq!(target.active_effects.len(), 2);
    assert!(
        target
            .active_effects
            .iter()
            .any(|effect| effect.instance_id == "toxin_ward_1")
    );
    assert!(
        target
            .active_effects
            .iter()
            .any(|effect| effect.instance_id == "ward_1")
    );

    let second_cure_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "poison_cure".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("poison cure should still apply when no poison remains");
    assert!(!second_cure_events.iter().any(|event| matches!(
        event,
        Event::SpellCastStubbed { spell_id, .. } if spell_id == "poison_cure"
    )));
    assert!(!second_cure_events.iter().any(|event| matches!(
        event,
        Event::EffectRemoved {
            actor_id,
            reason,
            ..
        } if actor_id == "target" && reason == "poison_cure"
    )));
}

#[test]
fn equipped_item_boost_contributes_to_successful_poison_save() {
    let mut engine = bs_runtime_spell_engine_mutate(
        &["poison"],
        vec!["#####", "#...#", "#####"],
        Coord { x: 3, y: 1 },
        |parts| {
            parts.selected_by_runtime_id_mut("spells", "poison")["effect"]["resistance"] = serde_json::json!({"role": "incoming", "tag": "poison", "mitigation": {"mode": "negate"}});
            parts.actors_mut()[1]["carried"] = serde_json::json!({
                "items": [{"item_instance_id": "antidote_charm", "position": "neck"}],
                "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}
            });
            parts.push_selected(
                "items",
                "item/antidote_charm/spell_actor_effects_test",
                serde_json::json!({
                    "id": "antidote_charm",
                    "kind": "accessory",
                    "name": "Antidote Charm",
                    "valid_placements": ["neck"],
                    "capability": {
                        "resistance_boosts": [{"tag": "poison", "bonus_twentieths": 15}]
                    }
                , "economy": {"unit_burden": 1}}),
            );
            *parts.item_instances_mut() = serde_json::json!({
                "antidote_charm": {
                    "definition_id": "antidote_charm",
                    "binding": {"state": "unrestricted"}
                }
            });
        },
    );

    let poison_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "poison".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("poison should resolve");

    assert!(!poison_events.iter().any(|event| matches!(
        event,
        Event::SpellCastStubbed { spell_id, .. } if spell_id == "poison"
    )));
    assert!(poison_events.iter().any(|event| matches!(
        event,
        Event::SpellSaveResolved {
            actor_id,
            effect_id,
            resistance_tag,
            ..
        } if actor_id == "target" && effect_id == "poison" && resistance_tag == "poison"
    )));
    assert!(!poison_events.iter().any(|event| matches!(
        event,
        Event::EffectApplied {
            actor_id,
            effect_id,
            ..
        } if actor_id == "target" && effect_id == "poison"
    )));
    assert!(!poison_events.iter().any(|event| matches!(
        event,
        Event::SpellSaveResolved {
            actor_id,
            effect_id,
            resistance_tag,
            ..
        } if actor_id == "target" && effect_id == "poison" && resistance_tag == "mind"
    )));
    let target = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "target")
        .expect("target actor");
    assert!(target.active_effects.is_empty());
}

#[test]
fn blind_status_blocks_sight_required_spell_targeting() {
    let mut engine = bs_runtime_spell_engine(
        &["blind_self", "spark"],
        vec!["#####", "#...#", "#####"],
        Coord { x: 3, y: 1 },
    );

    let blind_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "blind_self".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("blind should cast");

    assert!(!blind_events.iter().any(|event| matches!(
        event,
        Event::SpellCastStubbed { spell_id, .. } if spell_id == "blind_self"
    )));

    let err = engine
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
        .expect_err("blind caster should not target visible-only spells");
    assert!(err.to_string().contains("target_not_visible"));
}

#[test]
fn blind_status_excludes_ranged_attack_target_from_context_and_blocks_commit() {
    let mut engine = bs_runtime_spell_engine_mutate(
        &["blind_self"],
        vec!["#####", "#...#", "#####"],
        Coord { x: 3, y: 1 },
        |parts| equip_one_handed_test_weapon(parts, "test_bow", "shoot"),
    );

    let blind_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "blind_self".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("blind should cast");
    assert!(!blind_events.iter().any(|event| matches!(
        event,
        Event::SpellCastStubbed { spell_id, .. } if spell_id == "blind_self"
    )));

    let context = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context");
    assert!(
        context
            .attack_targets
            .iter()
            .all(|target| target.actor_id != "target"),
        "an unseen living actor must be excluded from Action Context 28"
    );

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Shoot,
                target_actor_id: "target".into(),
            },
        )
        .expect("blind ranged attack should resolve as blocked");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::AttackBlockedNoSight {
            attacker_id,
            defender_id,
            ..
        } if attacker_id == "player" && defender_id == "target"
    )));
}

#[test]
fn blind_status_excludes_thrown_attack_target_from_context_and_blocks_commit() {
    let mut engine = bs_runtime_spell_engine_mutate(
        &["blind_self"],
        vec!["#####", "#...#", "#####"],
        Coord { x: 3, y: 1 },
        |parts| equip_one_handed_test_weapon(parts, "test_javelin", "throw"),
    );

    let blind_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "blind_self".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("blind should cast");
    assert!(!blind_events.iter().any(|event| matches!(
        event,
        Event::SpellCastStubbed { spell_id, .. } if spell_id == "blind_self"
    )));

    let context = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context");
    assert!(
        context
            .attack_targets
            .iter()
            .all(|target| target.actor_id != "target"),
        "an unseen living actor must be excluded from Action Context 28"
    );

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "target".into(),
            },
        )
        .expect("blind thrown attack should resolve as blocked");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::AttackBlockedNoSight {
            attacker_id,
            defender_id,
            ..
        } if attacker_id == "player" && defender_id == "target"
    )));
}

#[test]
fn successful_save_halves_direct_spell_damage_with_matching_protection_boost() {
    let mut engine = bs_runtime_spell_engine(
        &["arcane_guard", "spark"],
        vec!["#####", "#...#", "#####"],
        Coord { x: 3, y: 1 },
    );

    let guard_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "arcane_guard".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("arcane guard should cast");

    assert!(!guard_events.iter().any(|event| matches!(
        event,
        Event::SpellCastStubbed { spell_id, .. } if spell_id == "arcane_guard"
    )));

    let spark_events = engine
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
        .expect("spark should resolve");

    assert!(!spark_events.iter().any(
        |event| matches!(event, Event::SpellCastStubbed { spell_id, .. } if spell_id == "spark")
    ));
    assert!(spark_events.iter().any(|event| matches!(
        event,
        Event::SpellSaveResolved {
            actor_id,
            effect_id,
            resistance_tag,
            ..
        } if actor_id == "target" && effect_id == "spark" && resistance_tag == "arcane"
    )));
    assert!(spark_events.iter().any(|event| matches!(
        event,
        Event::SpellDamaged { spell_id, damage: 1, hp: 7, .. } if spell_id == "spark"
    )));
    let target = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "target")
        .expect("target actor");
    assert_eq!(target.hp, 7);
}

#[test]
fn lethal_direct_damage_spell_marks_target_dead_and_removes_active_effects() {
    let mut engine = br_effect_spell_engine_with_player_hp_mutate(&["spark"], 10, |parts| {
        parts.actor_definition_mut(1)["stats"]["hp"] = serde_json::json!(3);
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
                "resistance_boosts": []
            }
        ]);
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
        .expect("lethal damage spell should cast");

    let damaged = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::SpellDamaged {
                    target_id,
                    hp: 0,
                    ..
                } if target_id == "target"
            )
        })
        .expect("lethal spell damage event");
    let removed = events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::EffectRemoved {
                    actor_id,
                    instance_id,
                    reason,
                    ..
                } if actor_id == "target"
                    && instance_id == "ward_1"
                    && reason == "defeat"
            )
        })
        .expect("defeat effect removal");
    let died = events
        .iter()
        .position(
            |event| matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == "target"),
        )
        .expect("target death");
    assert!(damaged < removed);
    assert!(removed < died);
    assert_eq!(
        events
            .iter()
            .filter(|event| {
                matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == "target")
            })
            .count(),
        1
    );

    assert!(events.iter().any(|event| matches!(
        event,
        Event::SpellDamaged {
            target_id,
            damage: 3,
            hp: 0,
            ..
        } if target_id == "target"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::EffectRemoved {
            actor_id,
            instance_id,
            reason,
            ..
        } if actor_id == "target" && instance_id == "ward_1" && reason == "defeat"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ActorDefeated {
            actor_id,
            cause: tme_rules::DeathCause::OtherMagic,
            credited_actor_id: Some(credited_actor_id),
            ..
        } if actor_id == "target" && credited_actor_id == "player"
    )));
    assert!(!events.iter().any(
        |event| matches!(event, Event::SpellCastStubbed { spell_id, .. } if spell_id == "spark")
    ));
    let target = engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "target")
        .expect("target actor");
    assert_eq!(target.hp, 0);
    assert!(!target.is_alive());
    assert!(target.active_effects.is_empty());
}

#[test]
fn lethal_fire_spell_passes_fire_credit_and_suppresses_corpse_creation() {
    let mut engine = br_effect_spell_engine_with_player_hp_mutate(&["spark"], 10, |parts| {
        parts.actor_definition_mut(1)["stats"]["hp"] = serde_json::json!(3);
        parts.selected_by_runtime_id_mut("spells", "spark")["effect"]["damage_kind"] =
            serde_json::json!("fire");
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
        .expect("lethal fire spell should cast");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::ActorDefeated {
            actor_id,
            cause: tme_rules::DeathCause::Fire,
            credited_actor_id: Some(credited_actor_id),
            ..
        } if actor_id == "target" && credited_actor_id == "player"
    )));
    assert!(engine.world().corpses.is_empty());
    assert_eq!(engine.world().next_corpse_sequence, 1);
    assert!(matches!(
        engine
            .world()
            .actors
            .iter()
            .find(|actor| actor.id == "target")
            .unwrap()
            .life_state,
        tme_rules::ActorLifeState::Dead
    ));
}
