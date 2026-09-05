use super::*;

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
    assert_eq!(player.hp, 9);
    assert_eq!(
        player
            .character
            .as_ref()
            .expect("player character")
            .resources
            .hp,
        9
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
