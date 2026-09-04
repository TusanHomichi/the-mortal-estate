use super::*;

#[test]
fn typed_capabilities_apply_without_stub_or_invented_consumers() {
    let lane = "wizard_magic";
    let spells = vec![
        active_spell(
            "feather_fall",
            lane,
            "fall_protection",
            "fall_protection",
            "self",
        ),
        active_spell("haste", lane, "speed", "speed", "self"),
        active_spell("night_sight", lane, "vision", "night_vision", "self"),
        active_spell(
            "breathe_water",
            lane,
            "water_breathing",
            "water_breathing",
            "actor",
        ),
    ];
    let mut engine = family_engine("wizard", lane, spells, 7, |parts| {
        parts.actor_definition_mut(1)["social"]["alignment_source"] =
            json!({"kind": "inherent", "alignment": "neutral"});
        parts.actor_definition_mut(1)["social"]["behavior"] = json!("passive");
    });

    for (spell_id, target) in [
        ("feather_fall", SpellTarget::SelfTarget),
        ("haste", SpellTarget::SelfTarget),
        ("night_sight", SpellTarget::SelfTarget),
        (
            "breathe_water",
            SpellTarget::Actor {
                actor_id: "player".into(),
            },
        ),
    ] {
        let events = cast(&mut engine, spell_id, Some(target));
        assert!(events.iter().any(|event| matches!(
            event,
            Event::EffectApplied { effect_id, .. } if effect_id == spell_id
        )));
        assert!(!events.iter().any(|event| matches!(
            event,
            Event::SpellCastStubbed { spell_id: stubbed, .. } if stubbed == spell_id
        )));
    }

    let player = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player");
    for tag in [
        "fall_protection",
        "speed",
        "night_vision",
        "water_breathing",
    ] {
        assert!(
            player
                .active_effects
                .iter()
                .any(|effect| { effect.tags.iter().any(|candidate| candidate == tag) })
        );
    }
    assert_eq!(
        engine
            .definition()
            .catalog()
            .rules()
            .movement
            .controlled_path_points,
        3
    );
    assert_eq!(
        engine
            .definition()
            .catalog()
            .rules()
            .movement
            .automatic_step_points,
        1
    );
}

#[test]
fn night_vision_bypasses_only_darkness_for_actor_addressed_sight() {
    let lane = "wizard_magic";
    let night = active_spell("night_sight", lane, "vision", "night_vision", "self");
    let mut engine = family_engine("wizard", lane, vec![night], 7, |parts| {
        parts.actor_definition_mut(1)["social"]["alignment_source"] =
            json!({"kind": "inherent", "alignment": "neutral"});
        parts.actor_definition_mut(1)["social"]["behavior"] = json!("passive");
    });
    let from = WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 });
    let to = WorldPosition::new("realm_0", "room_0", Coord { x: 3, y: 1 });
    let darkness = darkness_effect(&engine, "darkness");
    engine.world_mut().tile_effects.push(darkness);
    assert!(!engine.has_line_of_sight(&from, &to));
    assert!(
        !engine
            .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
            .expect("snapshot")
            .actors
            .iter()
            .any(|actor| actor.id == "target")
    );

    cast(&mut engine, "night_sight", Some(SpellTarget::SelfTarget));
    assert!(!engine.has_line_of_sight(&from, &to));
    assert!(
        engine
            .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
            .expect("night snapshot")
            .actors
            .iter()
            .any(|actor| actor.id == "target")
    );

    engine.world_mut().tile_effects.clear();
    let smoke = darkness_effect(&engine, "smoke");
    engine.world_mut().tile_effects.push(smoke);
    assert!(
        !engine
            .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
            .expect("smoke snapshot")
            .actors
            .iter()
            .any(|actor| actor.id == "target")
    );

    engine.world_mut().tile_effects.clear();
    let darkness = darkness_effect(&engine, "darkness");
    engine.world_mut().tile_effects.push(darkness);
    let now = engine.world().timing.now;
    engine.world_mut().actors[0]
        .active_effects
        .push(ActiveEffectState {
            instance_id: "blind:test".to_string(),
            effect_id: "blind_test".to_string(),
            source: ActiveEffectSource {
                kind: "fixture".to_string(),
                id: "blind_test".to_string(),
            },
            source_actor_id: None,
            hostile_authority: None,
            spell_damage_credit: None,
            kind: "blind".to_string(),
            tags: vec!["blind".to_string()],
            potency: 0,
            remaining_rounds: None,
            until_condition: None,
            stacking: ActiveEffectStackingPolicy::ReplaceSameKind,
            start_delay_rounds: 0,
            tick_interval_rounds: 1,
            suppresses_action: false,
            resistance_boosts: vec![],
            last_ticked_at: now,
        });
    let blind_snapshot = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("blind snapshot");
    assert_eq!(blind_snapshot.actors.len(), 1);
    assert_eq!(blind_snapshot.actors[0].id, "player");
}

#[test]
fn spell_concealment_breaks_on_uncovered_move_and_damage_but_not_cast_itself() {
    let mut engine = thief_concealment_engine();
    let events = cast(
        &mut engine,
        "hide_in_shadows",
        Some(SpellTarget::SelfTarget),
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Event::ActorHidden { effect_id, .. } if effect_id == "hide_in_shadows"
    )));
    assert!(
        engine.world().actors[0]
            .active_effects
            .iter()
            .any(|effect| effect.source.kind == "spell"
                && effect.tags.contains(&"hidden".to_string()))
    );

    let moved = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("uncovered move");
    assert!(moved.events.iter().any(|event| matches!(
        event,
        Event::HideBroken { reason, .. } if reason == "move"
    )));

    let mut engine = thief_concealment_engine();
    cast(
        &mut engine,
        "hide_in_shadows",
        Some(SpellTarget::SelfTarget),
    );
    let now = engine.world().timing.now.saturating_add_rounds(0);
    engine.world_mut().actors[0]
        .active_effects
        .push(ActiveEffectState {
            instance_id: "poison:hit".to_string(),
            effect_id: "poison_hit".to_string(),
            source: ActiveEffectSource {
                kind: "fixture".to_string(),
                id: "poison_hit".to_string(),
            },
            source_actor_id: None,
            hostile_authority: None,
            spell_damage_credit: None,
            kind: "poison".to_string(),
            tags: vec!["poison".to_string()],
            potency: 1,
            remaining_rounds: Some(2),
            until_condition: None,
            stacking: ActiveEffectStackingPolicy::StackInstance,
            start_delay_rounds: 0,
            tick_interval_rounds: 1,
            suppresses_action: false,
            resistance_boosts: vec![],
            last_ticked_at: LogicalTime::new(now.value().saturating_sub(1)),
        });
    let hit = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("poison boundary");
    assert!(hit.events.iter().any(|event| matches!(
        event,
        Event::HideBroken { reason, .. } if reason == "hit"
    )));
    assert!(
        !engine.world().actors[0]
            .active_effects
            .iter()
            .any(|effect| effect.source.kind == "spell"
                && effect.tags.contains(&"hidden".to_string()))
    );
}

#[test]
fn door_concealment_hides_observed_state_and_removes_on_reveal_open_and_expiry() {
    let door = Coord { x: 2, y: 1 };
    let mut engine = door_engine(3);
    let concealed = cast(
        &mut engine,
        "hide_door",
        Some(SpellTarget::Door {
            direction: Direction::East,
        }),
    );
    assert!(concealed.iter().any(|event| matches!(
        event,
        Event::TransitionConcealed { location, .. } if location.position == door
    )));
    let snapshot = engine.snapshot();
    assert_eq!(snapshot.concealed_transitions.len(), 1);
    let concealment = &snapshot.concealed_transitions[0];
    assert_eq!(
        serde_json::to_value(concealment).expect("concealment view serializes"),
        json!({
            "instance_id": concealment.instance_id,
            "source_spell_id": "hide_door",
            "source_actor_id": "player",
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 2, "y": 1}
            },
            "remaining_rounds": 2,
            "last_ticked_at": concealment.last_ticked_at
        })
    );
    let observed = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed");
    assert!(
        observed.realms[0].levels[0]
            .tiles
            .iter()
            .find(|tile| tile.position == door)
            .is_some_and(|tile| tile.transition.is_none())
    );
    let observed_json = serde_json::to_string(&observed).expect("observed snapshot serializes");
    assert!(!observed_json.contains("concealed_transitions"));
    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("context");
    assert!(context.door_actions.is_empty());
    let east_exit = context
        .exits
        .iter()
        .find(|exit| exit.direction == Direction::East)
        .expect("east exit");
    assert!(east_exit.transition.is_none());
    assert!(
        !serde_json::to_string(east_exit)
            .expect("east exit serializes")
            .contains("target_room")
    );

    let revealed = cast(&mut engine, "sense_secret", None);
    assert!(revealed.iter().any(|event| matches!(
        event,
        Event::TransitionConcealmentRemoved {
            reason: TransitionConcealmentRemovalReasonV1::Revealed,
            ..
        }
    )));
    assert!(engine.world().concealed_transitions.is_empty());

    cast(
        &mut engine,
        "hide_door",
        Some(SpellTarget::Door {
            direction: Direction::East,
        }),
    );
    let opened = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Open(Direction::East),
        )
        .expect("underlying door opens explicitly");
    let removed = opened
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                Event::TransitionConcealmentRemoved {
                    reason: TransitionConcealmentRemovalReasonV1::Opened,
                    ..
                }
            )
        })
        .expect("open removal");
    let door_opened = opened
        .events
        .iter()
        .position(|event| matches!(event, Event::DoorOpened { .. }))
        .expect("door opened");
    assert!(removed < door_opened);

    let mut engine = door_engine(2);
    cast(
        &mut engine,
        "hide_door",
        Some(SpellTarget::Door {
            direction: Direction::East,
        }),
    );
    let expired = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("expiry wait");
    assert!(expired.events.iter().any(|event| matches!(
        event,
        Event::TransitionConcealmentRemoved {
            reason: TransitionConcealmentRemovalReasonV1::Expired,
            ..
        }
    )));
}

#[test]
fn banish_removes_only_owned_summoned_demon_and_preserves_failed_targets() {
    let mut engine = banish_engine();
    let debug_target = engine
        .snapshot()
        .actors
        .into_iter()
        .find(|actor| actor.id == "target")
        .expect("debug target");
    assert_eq!(debug_target.creature_traits, vec![CreatureTrait::Demon]);
    let observed_target = engine
        .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
        .expect("observed snapshot")
        .actors
        .into_iter()
        .find(|actor| actor.id == "target")
        .expect("observed target");
    assert_eq!(observed_target.creature_traits, vec![CreatureTrait::Demon]);
    let target_index = engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == "target")
        .expect("target index");
    engine.world_mut().actors[target_index]
        .social
        .alignment_source = SocialAlignmentSource::Inherent {
        alignment: CharacterAlignment::Chaotic,
    };
    engine.world_mut().actors[target_index].social.behavior = SocialBehavior::AlignmentCreature;
    let action_target = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("observed action context")
        .attack_targets
        .into_iter()
        .find(|actor| actor.actor_id == "target")
        .expect("action target");
    assert_eq!(action_target.creature_traits, vec![CreatureTrait::Demon]);
    engine.world_mut().actors[target_index]
        .social
        .alignment_source = SocialAlignmentSource::Inherent {
        alignment: CharacterAlignment::Neutral,
    };
    engine.world_mut().actors[target_index].social.behavior = SocialBehavior::Passive;

    let summoned = cast(
        &mut engine,
        "call_demon",
        Some(SpellTarget::Coordinate {
            position: WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 1 }),
        }),
    );
    let summoned_id = summoned
        .iter()
        .find_map(|event| match event {
            Event::ActorSummoned { actor_id, .. } => Some(actor_id.clone()),
            _ => None,
        })
        .expect("summoned id");
    let item_id = format!("{summoned_id}:item:claw");
    assert!(engine.world().item_instances.contains_key(&item_id));

    let summon_index = engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == summoned_id)
        .expect("summon index");
    let summon_actor = engine.world_mut().actors.remove(summon_index);
    engine.world_mut().actors.insert(0, summon_actor);
    let banished = cast(
        &mut engine,
        "banish",
        Some(SpellTarget::Actor {
            actor_id: summoned_id.clone(),
        }),
    );
    assert!(banished.iter().any(|event| matches!(
        event,
        Event::BanishEvaluated {
            reason: BanishResultReasonV1::Banished,
            success: true,
            ..
        }
    )));
    assert!(banished.iter().any(|event| matches!(
        event,
        Event::ActorBanished { actor_id, .. } if actor_id == &summoned_id
    )));
    assert!(
        !engine
            .world()
            .actors
            .iter()
            .any(|actor| actor.id == summoned_id)
    );
    assert!(!engine.world().item_instances.contains_key(&item_id));
    assert!(banished.iter().any(|event| matches!(
        event,
        Event::ActorReadinessScheduled { actor_id, .. } if actor_id == "player"
    )));

    let mp_before = engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .expect("player")
        .mp;
    let failed = cast(
        &mut engine,
        "banish",
        Some(SpellTarget::Actor {
            actor_id: "target".into(),
        }),
    );
    assert!(failed.iter().any(|event| matches!(
        event,
        Event::BanishEvaluated {
            target_id,
            reason: BanishResultReasonV1::WillpowerFormulaOpen,
            success: false,
            ..
        } if target_id == "target"
    )));
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .expect("player")
            .mp,
        mp_before - 1
    );
    assert!(!failed.iter().any(|event| matches!(
        event,
        Event::MagicPracticeEvaluated { spell_id, .. } if spell_id == "banish"
    )));
    assert!(
        engine
            .world()
            .actors
            .iter()
            .any(|actor| actor.id == "target")
    );
}

#[test]
fn instant_death_uses_level_multiplier_shared_save_and_single_defeat_path() {
    let lane = "thaumaturge_magic";
    let death = spell(
        "death",
        lane,
        json!({
            "family": "instant_death",
            "instant_death": {"damage_per_magic_level": 10},
            "resistance": {"role": "incoming", "tag": "death", "mitigation": {"mode": "half_damage", "rounding": "down", "minimum_damage": 1}}
        }),
        json!({"kind": "actor", "range": 3, "requires_visible": true}),
        "character",
    );
    let mut saved = family_engine("thaumaturge", lane, vec![death.clone()], 7, |parts| {
        parts.actor_definition_mut(1)["stats"]["hp"] = json!(60);
        parts.actor_definition_mut(1)["magic_resistance"]["natural_save_twentieths"] = json!(20);
    });
    let saved_events = cast(
        &mut saved,
        "death",
        Some(SpellTarget::Actor {
            actor_id: "target".into(),
        }),
    );
    assert!(saved_events.iter().any(|event| matches!(
        event,
        Event::SpellSaveResolved {
            requested_damage: Some(50),
            resolved_damage: Some(25),
            success: true,
            ..
        }
    )));
    assert!(saved_events.iter().any(|event| matches!(
        event,
        Event::SpellDamaged {
            damage: 25,
            hp: 35,
            ..
        }
    )));

    let mut lethal = family_engine("thaumaturge", lane, vec![death], 7, |parts| {
        parts.actor_definition_mut(1)["stats"]["hp"] = json!(40);
        parts.actor_definition_mut(1)["xp_value"] = json!(25);
        parts.actor_definition_mut(1)["magic_resistance"]["natural_save_twentieths"] = json!(0);
    });
    let lethal_events = cast(
        &mut lethal,
        "death",
        Some(SpellTarget::Actor {
            actor_id: "target".into(),
        }),
    );
    assert_eq!(
        lethal_events
            .iter()
            .filter(|event| matches!(event, Event::ActorDefeated { actor_id, cause: DeathCause::OtherMagic, .. } if actor_id == "target"))
            .count(),
        1
    );
    assert_eq!(lethal.world().corpses.len(), 1);
    assert_eq!(
        lethal_events
            .iter()
            .filter(|event| matches!(event, Event::DefeatRewardEvaluated { target_id, .. } if target_id == "target"))
            .count(),
        1
    );
}

#[test]
fn raise_dead_selects_newest_player_corpse_and_rolls_only_when_eligible() {
    let mut empty = raise_dead_engine(7);
    let debug_rules =
        serde_json::to_value(&empty.snapshot().rules.magic.effect_families.raise_dead)
            .expect("debug Raise Dead rules serialize");
    let observed_rules = serde_json::to_value(
        &empty
            .actor_observed_snapshot(&tme_rules::ActorId::from("player"))
            .expect("observed snapshot")
            .rules
            .magic
            .effect_families
            .raise_dead,
    )
    .expect("observed Raise Dead rules serialize");
    assert_eq!(
        debug_rules,
        json!({
            "roll_denominator": 20,
            "success_threshold_per_magic_level": 1,
            "minimum_success_threshold": 1,
            "evidence_state": "original_provisional"
        })
    );
    assert_eq!(observed_rules, debug_rules);
    let no_corpse = cast(&mut empty, "raise_dead", None);
    assert!(no_corpse.iter().any(|event| matches!(
        event,
        Event::RaiseDeadEvaluated {
            reason: RaiseDeadResultReasonV1::NoCorpse,
            roll: None,
            ..
        }
    )));

    let mut failed = raise_dead_engine(7);
    install_player_corpse(&mut failed, "corpse:1", 1);
    let failed_events = cast(&mut failed, "raise_dead", None);
    assert!(failed_events.iter().any(|event| matches!(
        event,
        Event::RaiseDeadEvaluated {
            corpse_id: Some(id),
            success_threshold: 5,
            roll: Some(11),
            reason: RaiseDeadResultReasonV1::RollFailed,
            ..
        } if id.as_str() == "corpse:1"
    )));
    assert!(matches!(
        failed
            .world()
            .actors
            .iter()
            .find(|actor| actor.id == "target")
            .expect("target")
            .life_state,
        ActorLifeState::Ghost { .. }
    ));

    let mut succeeded = raise_dead_engine(3);
    install_player_corpse(&mut succeeded, "corpse:2", 2);
    succeeded.world_mut().corpses.insert(
        CorpseId::parse("corpse:1").expect("old corpse"),
        CorpseState {
            id: CorpseId::parse("corpse:1").expect("old corpse"),
            origin_actor_id: "old_npc".into(),
            origin_character_id: None,
            origin_kind: ActorKind::Monster,
            origin_name: "Old NPC".to_string(),
            location: WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 }),
            created_at: LogicalTime::FIRST,
            sequence: 1,
            searched: false,
            loot_claim: None,
            contents: BTreeMap::new(),
            gold: 0,
        },
    );
    let success_events = cast(&mut succeeded, "raise_dead", None);
    assert!(success_events.iter().any(|event| matches!(
        event,
        Event::RaiseDeadEvaluated {
            corpse_id: Some(id),
            roll: Some(3),
            reason: RaiseDeadResultReasonV1::Resurrected,
            ..
        } if id.as_str() == "corpse:2"
    )));
    let target = succeeded
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == "target")
        .expect("raised target");
    assert!(target.is_alive());
    assert_eq!(target.hp, 4);
    assert_eq!(target.stamina, 0);
    assert!(
        !succeeded
            .world()
            .corpses
            .contains_key(&CorpseId::parse("corpse:2").expect("new corpse"))
    );
    assert!(
        succeeded
            .world()
            .corpses
            .contains_key(&CorpseId::parse("corpse:1").expect("old corpse"))
    );
}
