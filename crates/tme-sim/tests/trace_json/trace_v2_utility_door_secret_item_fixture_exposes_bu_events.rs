use super::*;

#[test]
fn trace_v2_utility_door_secret_item_fixture_exposes_bu_events() {
    let trace = run_trace_v2("utility_door_secret_item_spells.json", 7);

    assert_eq!(trace.header.contract_version, TRACE_V2_CONTRACT_VERSION);
    assert_eq!(trace.header.event_contract_version, EVENT_CONTRACT_VERSION);
    assert_eq!(
        trace.header.snapshot_contract_version,
        SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.observed_snapshot_contract_version,
        OBSERVED_SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.action_context_contract_version,
        ACTION_CONTEXT_CONTRACT_VERSION
    );
    assert!(trace.steps.iter().any(|step| matches!(
        &step.command.intent,
        tme_rules::PlayerIntentPayloadV1::CastSpell {
            spell_id,
            target: Some(tme_rules::SpellTarget::None),
            ..
        } if spell_id == "workroom_glimpse"
    )));

    let events: Vec<&tme_rules::Event> = trace
        .steps
        .iter()
        .flat_map(|step| step.events.iter())
        .collect();

    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::SecretTransitionRevealed { location, transition_kind, .. }
            if location.level == "workroom"
                && location.position == tme_rules::Coord { x: 3, y: 1 }
                && transition_kind == "stairs"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::SecretTransitionHidden { location, transition_kind, .. }
            if location.level == "workroom"
                && location.position == tme_rules::Coord { x: 3, y: 1 }
                && transition_kind == "stairs"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::ItemIdentified {
            item_instance_id,
            item_definition_id,
            location,
            capability,
            ..
        }
            if item_instance_id == "ground_charm"
                && item_definition_id == "ground_charm"
                && location == "ground_here"
                && capability.as_ref().and_then(|capability| capability.taxonomy_id.as_deref()) == Some("trinket")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::ItemEnchanted {
            item_instance_id,
            combat_add_rating_bonus,
            tags,
            remaining_rounds,
            ..
        } if item_instance_id == "utility_blade"
            && *combat_add_rating_bonus == 5
            && tags == &vec!["keen".to_string()]
            && *remaining_rounds == Some(1)
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::ItemEnchantmentExpired {
            item_instance_id,
            enchantment_instance_id,
            ..
        }
            if item_instance_id == "utility_blade"
                && enchantment_instance_id.starts_with("spell:keen_edge:")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::ItemTransformed {
            item_instance_id,
            old_item_definition_id,
            new_item_definition_id,
            location,
            ..
        }
            if item_instance_id == "raw_relic"
                && old_item_definition_id == "raw_relic"
                && new_item_definition_id == "recall_token"
                && location == "sack"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::Located { subject, id, location, hint, .. }
            if subject == "item"
                && id == "ground_charm"
                && location.as_ref().is_some_and(|position| position.level == "workroom" && position.position == tme_rules::Coord { x: 1, y: 1 })
                && hint == "item ground_charm located in workroom at 1,1"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::Located { subject, id, location, hint, .. }
            if subject == "item"
                && id == "veiled_charm"
                && location.is_none()
                && hint == "item veiled_charm is hidden or unobserved"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::Located { subject, id, location, hint, .. }
            if subject == "scry"
                && id == "workroom_glimpse"
                && location.as_ref().is_some_and(|position| position.level == "workroom" && position.position == tme_rules::Coord { x: 1, y: 1 })
                && hint == "scry workroom_glimpse located in workroom at 1,1"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::PortalCreated {
            instance_id,
            location,
            target,
            remaining_rounds,
            two_way,
            ..
        } if instance_id.starts_with("portal:blue_gate:")
            && location.level == "workroom"
            && location.position == tme_rules::Coord { x: 2, y: 1 }
            && target.level == "vault"
            && target.position == tme_rules::Coord { x: 1, y: 1 }
            && *remaining_rounds == Some(2)
            && *two_way
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        tme_rules::Event::PortalExpired { instance_id, location }
            if instance_id.starts_with("portal:blue_gate:")
                && location.level == "workroom"
                && location.position == tme_rules::Coord { x: 2, y: 1 }
    )));

    let portal_creation_step = trace
        .steps
        .iter()
        .find(|step| {
            step.events
                .iter()
                .any(|event| matches!(event, tme_rules::Event::PortalCreated { .. }))
        })
        .expect("portal creation step should exist");
    assert!(
        portal_creation_step
            .after_observed_snapshot
            .realms
            .iter()
            .find(|realm| realm.id == "realm_0")
            .expect("realm_0")
            .levels
            .iter()
            .find(|level| level.id == "workroom")
            .expect("workroom in observed snapshot")
            .tiles
            .iter()
            .any(|tile| {
                tile.position == tme_rules::Coord { x: 2, y: 1 }
                    && tile.transition.as_ref().is_some_and(|transition| {
                        transition.kind == tme_rules::TransitionKindViewV1::Portal
                    })
            }),
        "portal transition should be visible while active"
    );
    let portal_expiration_step = trace
        .steps
        .iter()
        .find(|step| {
            step.events
                .iter()
                .any(|event| matches!(event, tme_rules::Event::PortalExpired { .. }))
        })
        .expect("portal expiration step should exist");
    let expired_tile = portal_expiration_step
        .after_observed_snapshot
        .realms
        .iter()
        .find(|realm| realm.id == "realm_0")
        .expect("realm_0")
        .levels
        .iter()
        .find(|level| level.id == "vault")
        .expect("vault in observed snapshot")
        .tiles
        .iter()
        .find(|tile| tile.position == tme_rules::Coord { x: 1, y: 1 })
        .expect("vault-side portal tile should be present");
    assert_eq!(
        expired_tile.observation,
        tme_rules::TileObservationV1::Visible
    );
    assert!(
        !expired_tile
            .transition
            .as_ref()
            .is_some_and(|transition| transition.kind == tme_rules::TransitionKindViewV1::Portal),
        "portal transition should be absent from the observed tile after expiration"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, tme_rules::Event::SpellCastStubbed { .. })),
        "utility gallery should not emit SpellCastStubbed"
    );
}

#[test]
fn scripted_spell_readiness_matches_golden() {
    let output = tme_sim::run_from_args(vec![
        "tme-sim".to_string(),
        "--scenario".to_string(),
        scenario_path("spell_readiness.json"),
        "--seed".to_string(),
        "7".to_string(),
    ])
    .expect("scripted run succeeds");
    let expected = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("golden")
            .join("spell_readiness_seed_7.txt"),
    )
    .expect("golden should read")
    .replace("\r\n", "\n");

    assert_eq!(output, expected);
}

#[test]
fn trace_v2_golden_deserializes_with_correct_contract_versions() {
    let golden_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/trace_v2_first_room_seed_7.json");
    let json = std::fs::read_to_string(&golden_path).expect("read golden");
    let trace: TraceV2 = serde_json::from_str(&json).expect("deserialize");

    // Header contract versions must match current constants
    assert_eq!(trace.header.contract_version, TRACE_V2_CONTRACT_VERSION);
    assert_eq!(
        trace.header.event_contract_version, EVENT_CONTRACT_VERSION,
        "event contract version mismatch"
    );
    assert_eq!(
        trace.header.snapshot_contract_version, SNAPSHOT_CONTRACT_VERSION,
        "snapshot contract version mismatch"
    );
    assert_eq!(
        trace.header.observed_snapshot_contract_version, OBSERVED_SNAPSHOT_CONTRACT_VERSION,
        "observed snapshot contract version mismatch"
    );
    assert_eq!(
        trace.header.action_context_contract_version, ACTION_CONTEXT_CONTRACT_VERSION,
        "action context contract version mismatch"
    );
    assert_eq!(
        trace.header.intent_contract_version, COMMAND_CONTRACT_VERSION,
        "intent contract version mismatch"
    );
    assert_eq!(
        trace.header.initial_debug_snapshot.contract_version,
        SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.initial_observed_snapshot.contract_version,
        OBSERVED_SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.initial_action_context.contract_version,
        ACTION_CONTEXT_CONTRACT_VERSION
    );
    let debug_combat = &trace.header.initial_debug_snapshot.rules.combat;
    let observed_combat = &trace.header.initial_observed_snapshot.rules.combat;
    assert_eq!(debug_combat, observed_combat);
    assert_eq!(
        debug_combat.tuning_status,
        tme_rules::CombatTuningStatusViewV1::OriginalProvisional
    );
    assert_eq!(debug_combat.hit.base_defender_score, 10);
    assert_eq!(debug_combat.hit.attacker_attack_stat_divisor, 2);
    assert_eq!(debug_combat.hit.attacker_skill_level_divisor, 2);
    assert_eq!(debug_combat.hit.defender_defense_stat_divisor, 2);
    assert_eq!(debug_combat.hit.defender_dexterity_divisor, 3);
    assert_eq!(debug_combat.hit.non_character_defender_dexterity, 10);
    assert_eq!(debug_combat.block.shield_percent_per_point, 10);
    assert_eq!(debug_combat.block.shield_percent_cap, 90);
    assert_eq!(debug_combat.block.armor_percent_per_point, 8);
    assert_eq!(debug_combat.block.armor_percent_cap, 80);
    assert_eq!(debug_combat.block.strength_penetration_percent_per_add, 2);
    assert_eq!(debug_combat.block.armor_encumbrance_percent_per_point, 2);
    assert_eq!(
        debug_combat.block.combat_add_penetration_percent_per_rating,
        2
    );
    assert_eq!(debug_combat.damage.minimum_damage, 1);
    assert_eq!(debug_combat.damage.roll_variation_modulus, 3);
    assert_eq!(debug_combat.damage.moderate_label_min_percent, 20);
    assert_eq!(debug_combat.damage.heavy_label_min_percent, 40);
    assert_eq!(debug_combat.damage.severe_label_min_percent, 70);
    assert_eq!(debug_combat.wounds.near_death_max_percent, 20);
    assert_eq!(debug_combat.wounds.badly_wounded_max_percent, 50);
    assert_eq!(debug_combat.wounds.wounded_max_percent, 99);
    assert_eq!(debug_combat.practice.practice_raw_points, 1);
    assert_eq!(debug_combat.practice.life_and_death_raw_points, 2);
    assert_eq!(debug_combat.practice.overwhelming_raw_points, 1);

    // Every step must have a command and after_observed_snapshot
    for step in &trace.steps {
        assert!(!step.command.actor_id.is_empty());
        assert_eq!(step.command.contract_version, COMMAND_CONTRACT_VERSION);
        assert!(!step.intent_label.is_empty());
        assert_eq!(
            step.after_debug_snapshot.contract_version,
            SNAPSHOT_CONTRACT_VERSION
        );
        assert_eq!(
            step.after_observed_snapshot.contract_version,
            OBSERVED_SNAPSHOT_CONTRACT_VERSION
        );
        assert_eq!(
            step.after_action_context.contract_version,
            ACTION_CONTEXT_CONTRACT_VERSION
        );
        assert!(step.after_observed_snapshot.logical_time.value() > 0);
    }
    assert_eq!(trace.r#final.contract_version, TRACE_V2_CONTRACT_VERSION);
    assert_eq!(
        trace.r#final.final_debug_snapshot.contract_version,
        SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.r#final.final_observed_snapshot.contract_version,
        OBSERVED_SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.r#final.final_action_context.contract_version,
        ACTION_CONTEXT_CONTRACT_VERSION
    );
}

#[test]
fn trace_v2_status_effect_fixture_exposes_effect_contracts() {
    let trace = run_trace_v2("status_effects.json", 7);
    assert_eq!(trace.header.event_contract_version, EVENT_CONTRACT_VERSION);
    assert_eq!(
        trace.header.snapshot_contract_version,
        SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.observed_snapshot_contract_version,
        OBSERVED_SNAPSHOT_CONTRACT_VERSION
    );
    assert_eq!(
        trace.header.action_context_contract_version,
        ACTION_CONTEXT_CONTRACT_VERSION
    );
    let player = trace
        .header
        .initial_debug_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("player actor");
    assert_eq!(player.active_effects.len(), 1);
    assert!(trace.steps.iter().flat_map(|step| &step.events).any(|event| {
        matches!(event, tme_rules::Event::EffectExpired { instance_id, .. } if instance_id == "rooted_1")
    }));
}

#[test]
fn trace_v2_control_poison_protection_fixture_exposes_bs_effects() {
    let trace = run_trace_v2("control_poison_protection.json", 7);

    let mut saw_applied = false;
    let mut saw_resistance_applied = false;
    let mut saw_ticked = false;
    let mut saw_spell_damage = false;
    let mut saw_failed_save = false;
    let mut saw_successful_negate = false;
    let mut saw_action_suppressed = false;

    for event in trace.steps.iter().flat_map(|step| step.events.iter()) {
        match event {
            tme_rules::Event::EffectApplied { .. } => saw_applied = true,
            tme_rules::Event::EffectTicked { .. } => saw_ticked = true,
            tme_rules::Event::SpellDamaged { spell_id, .. } if spell_id == "flame" => {
                saw_spell_damage = true;
            }
            tme_rules::Event::SpellSaveResolved {
                effect_id,
                resistance_tag,
                natural_save_twentieths: 5,
                matching_bonus_twentieths: 3,
                denominator: 20,
                save_twentieths: 8,
                roll: 11,
                success: false,
                mitigation_mode: None,
                requested_damage: Some(3),
                resolved_damage: Some(3),
                ..
            } if effect_id == "flame" && resistance_tag == "fire" => {
                saw_failed_save = true;
            }
            tme_rules::Event::SpellSaveResolved {
                effect_id,
                resistance_tag,
                natural_save_twentieths: 5,
                matching_bonus_twentieths: 0,
                denominator: 20,
                save_twentieths: 5,
                roll: 2,
                success: true,
                mitigation_mode: Some(tme_rules::SpellResistanceMitigationMode::Negate),
                requested_damage: None,
                resolved_damage: None,
                ..
            } if effect_id == "venom" && resistance_tag == "poison" => {
                saw_successful_negate = true;
            }
            tme_rules::Event::ActionSuppressedByStatus { .. } => saw_action_suppressed = true,
            _ => {}
        }

        if matches!(
            event,
            tme_rules::Event::EffectApplied {
                actor_id,
                effect_id,
                kind,
                ..
            } if actor_id == "target" && effect_id == "ember_skin" && kind == "resistance"
        ) {
            saw_resistance_applied = true;
        }
    }

    assert!(saw_applied, "fixture should emit effect_applied");
    assert!(
        saw_resistance_applied,
        "fixture should apply a concrete resistance-family effect"
    );
    assert!(saw_ticked, "fixture should emit effect_ticked");
    assert!(saw_spell_damage, "failed save should preserve spell damage");
    assert!(
        saw_failed_save,
        "fixture should expose the exact failed save"
    );
    assert!(
        saw_successful_negate,
        "fixture should expose the exact successful poison negation"
    );
    assert!(
        saw_action_suppressed,
        "fixture should emit ActionSuppressedByStatus"
    );

    assert!(
        trace
            .steps
            .iter()
            .any(|step| step
                .after_observed_snapshot
                .actors
                .iter()
                .any(|actor| actor.id == "target"
                    && actor
                        .magic_resistance
                        .boosts
                        .iter()
                        .any(|boost| boost.tag == "poison"))),
        "the active poison ward must be visible before expiry"
    );
    let final_target = trace
        .r#final
        .final_observed_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "target")
        .expect("visible target");
    assert!(
        !final_target
            .magic_resistance
            .boosts
            .iter()
            .any(|boost| boost.tag == "poison"),
        "the ward expires after its full authored duration"
    );

    assert!(
        trace
            .r#final
            .final_action_context
            .magic_resistance
            .boosts
            .iter()
            .any(|boost| boost.tag == "fire"),
        "player action context should expose the final fire resistance boost"
    );
}

#[test]
fn trace_v2_summon_fixture_exposes_created_creature_lifecycle() {
    let trace = run_trace_v2("summons_created_creature_lifecycle.json", 7);

    assert!(
        !trace
            .steps
            .iter()
            .flat_map(|step| step.events.iter())
            .any(|event| matches!(event, tme_rules::Event::SpellCastStubbed { .. })),
        "summon fixture should not emit SpellCastStubbed"
    );

    let summoned_event = trace
        .steps
        .iter()
        .flat_map(|step| step.events.iter())
        .find_map(|event| match event {
            tme_rules::Event::ActorSummoned {
                actor_id,
                owner_id,
                template_id,
                location,
                ..
            } if actor_id == "summon:call_echo:1:echo_guardian" => {
                Some((owner_id, template_id, location))
            }
            _ => None,
        })
        .expect("trace should include actor_summoned for echo_guardian");
    assert_eq!(summoned_event.0, "player");
    assert_eq!(summoned_event.1, "echo_guardian");
    assert_eq!(summoned_event.2.level, "start");
    assert_eq!(summoned_event.2.position, tme_rules::Coord { x: 2, y: 1 });

    let summon_step = trace
        .steps
        .iter()
        .find(|step| {
            step.events.iter().any(|event| {
                matches!(
                    event,
                    tme_rules::Event::ActorSummoned { actor_id, .. }
                        if actor_id == "summon:call_echo:1:echo_guardian"
                )
            })
        })
        .expect("trace should include a summon step");
    let summoned_actor = summon_step
        .after_debug_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "summon:call_echo:1:echo_guardian")
        .expect("summoned actor should appear in step snapshot");
    assert_eq!(summoned_actor.owner_id.as_deref(), Some("player"));
    let summoned_meta = summoned_actor
        .summoned
        .as_ref()
        .expect("summoned actor should expose summon metadata");
    assert_eq!(summoned_meta.template_id, "echo_guardian");
    let summoned_actor_observed = summon_step
        .after_observed_snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "summon:call_echo:1:echo_guardian")
        .expect("summoned actor should appear in observed snapshot after summon");
    assert_eq!(summoned_actor_observed.owner_id.as_deref(), Some("player"));
    let summoned_meta_observed = summoned_actor_observed
        .summoned
        .as_ref()
        .expect("summoned actor should expose summon metadata in observed snapshot");
    assert_eq!(summoned_meta_observed.template_id, "echo_guardian");

    assert!(trace.steps.iter().flat_map(|step| step.events.iter()).any(
        |event| matches!(
            event,
            tme_rules::Event::SummonExpired {
                actor_id,
                template_id,
                ..
            } if actor_id == "summon:call_echo:1:echo_guardian" && template_id == "echo_guardian"
        )
    ));
    assert!(
        !trace
            .r#final
            .final_debug_snapshot
            .actors
            .iter()
            .any(|actor| actor.id == "summon:call_echo:1:echo_guardian"),
        "expired summon should be absent from final snapshot"
    );
    assert!(
        !trace
            .r#final
            .final_observed_snapshot
            .actors
            .iter()
            .any(|actor| actor.id == "summon:call_echo:1:echo_guardian"),
        "expired summon should be absent from final observed snapshot"
    );
}

#[test]
fn trace_v2_profession_actions_fixture_exposes_bx_events_and_contract() {
    let thief_trace = run_trace_v2("profession_specific_actions.json", 7);
    let martial_trace = run_trace_v2("martial_hand_block_actions.json", 7);
    let knight_trace = run_trace_v2("knight_support_actions.json", 7);

    assert_eq!(
        thief_trace.header.intent_contract_version,
        COMMAND_CONTRACT_VERSION
    );
    assert_eq!(
        martial_trace.header.intent_contract_version,
        COMMAND_CONTRACT_VERSION
    );
    assert_eq!(
        knight_trace.header.intent_contract_version,
        COMMAND_CONTRACT_VERSION
    );

    let event_names: std::collections::BTreeSet<_> = thief_trace
        .steps
        .iter()
        .chain(martial_trace.steps.iter())
        .chain(knight_trace.steps.iter())
        .flat_map(|step| step.events.iter())
        .filter_map(|event| {
            let value = serde_json::to_value(event).expect("event serializes");
            value
                .as_object()
                .and_then(|object| object.keys().next())
                .cloned()
        })
        .collect();

    assert!(event_names.contains("actor_hidden"));
    assert!(event_names.contains("hide_broken"));
    assert!(event_names.contains("attack_blocked"));
    assert!(event_names.contains("item_enchanted"));
    assert!(event_names.contains("effect_applied"));
    assert!(event_names.contains("effect_removed"));
    assert!(event_names.contains("tile_effect_applied"));
    assert!(!event_names.contains("spell_cast_stubbed"));

    let hide_step = thief_trace
        .steps
        .iter()
        .find(|step| step.intent_label == "hide")
        .expect("hide step");
    assert!(matches!(
        hide_step.command.intent,
        tme_rules::PlayerIntentPayloadV1::Hide
    ));
    assert!(
        hide_step
            .after_observed_snapshot
            .actors
            .iter()
            .find(|actor| actor.id == "player0" || actor.id == "player")
            .expect("player observed")
            .active_effects
            .iter()
            .any(|effect| effect.kind == "hidden")
    );
}

#[test]
fn trace_v2_monster_ability_fixture_exposes_by_events() {
    let trace = run_trace_v2("monster_spellcasting_special_attacks.json", 7);

    let mut saw_monster_intent = false;
    let mut saw_spell_damage = false;
    let mut saw_poison_resisted = false;
    let mut saw_effect_tick = false;

    for event in trace.steps.iter().flat_map(|step| step.events.iter()) {
        match event {
            tme_rules::Event::AutomaticActorDecision {
                actor_id,
                decision: tme_rules::AutomaticActorDecisionV1::UseAbility { spell_name, .. },
                ..
            } if actor_id == "ember_imp" && spell_name == "Ember Spit" => {
                saw_monster_intent = true;
            }
            tme_rules::Event::SpellDamaged {
                caster_id,
                spell_id,
                target_id,
                damage_kind,
                damage,
                ..
            } if caster_id == "ember_imp"
                && spell_id == "ember_spit"
                && target_id == "player"
                && damage_kind.as_deref() == Some("fire")
                && *damage > 0 =>
            {
                saw_spell_damage = true;
            }
            tme_rules::Event::SpellSaveResolved {
                actor_id,
                effect_id,
                resistance_tag,
                ..
            } if actor_id == "player"
                && effect_id == "venom_bite"
                && resistance_tag == "poison" =>
            {
                saw_poison_resisted = true;
            }
            tme_rules::Event::EffectTicked {
                actor_id,
                effect_id,
                kind,
                ..
            } if actor_id == "player" && effect_id == "poison_ward" && kind == "protection" => {
                saw_effect_tick = true;
            }
            _ => {}
        }
    }

    assert!(
        saw_monster_intent,
        "fixture should emit monster ability intent"
    );
    assert!(saw_spell_damage, "fixture should emit monster spell damage");
    assert!(
        saw_poison_resisted,
        "fixture should emit poison resistance mitigation"
    );
    assert!(
        saw_effect_tick,
        "fixture should tick the seeded protection effect"
    );
}
