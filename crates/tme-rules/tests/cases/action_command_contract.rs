use crate::support::content_parts::ContentParts;
use tme_rules::{
    ActionBlockedReasonV1, COMMAND_CONTRACT_VERSION, Direction, Engine, PlayerCommandStatusV1,
    PlayerCommandV1, PlayerIntent, PlayerIntentPayloadV1,
};

#[path = "../action_context_support/commands.rs"]
mod command_support;
#[path = "../action_context_support/common.rs"]
mod common_support;

use command_support::suppressed_options_engine;
use common_support::{first_room_engine, make_command, option_by_id, status_engine};

fn tracked_engine(case_id: &str, profile: &str) -> Engine {
    ContentParts::tracked(case_id, profile)
        .engine(7)
        .expect("tracked content should start")
}

#[test]
fn command_validation_reports_suppressed_status() {
    let engine = status_engine();
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::MovePath {
            path: vec![Direction::East],
        },
    };
    let status = engine.validate_actor_command(&command).expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::SuppressedByStatus)
    );
}

#[test]
fn action_options_generates_move_commands() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");
    let moves: Vec<_> = options
        .iter()
        .filter(|option| {
            matches!(
                option.command.as_ref().map(|command| &command.intent),
                Some(PlayerIntentPayloadV1::MovePath { .. })
            )
        })
        .collect();
    assert_eq!(moves.len(), 8, "must have 8 directional move options");
    let walkable = moves.iter().filter(|o| o.enabled).count();
    assert!(walkable > 0, "at least one move must be walkable");
    // Every enabled move must have a valid command payload
    for m in &moves {
        if m.enabled {
            assert!(m.command.is_some(), "enabled move must have command");
        }
    }
}

#[test]
fn action_options_includes_attack_targets_with_ids() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");
    let attacks: Vec<_> = options
        .iter()
        .filter(|o| o.id.starts_with("physical_attack_"))
        .collect();
    assert!(!attacks.is_empty(), "must have attack targets");
    // Verify commands use actor IDs, not display names
    for a in &attacks {
        if let Some(ref cmd) = a.command {
            match &cmd.intent {
                PlayerIntentPayloadV1::PhysicalAttack {
                    authorization: _,
                    mode: _,
                    target_actor_id,
                } => {
                    assert!(!target_actor_id.is_empty(), "target must use actor_id");
                }
                _ => panic!("attack option must have Attack payload"),
            }
        }
    }
}

#[test]
fn action_options_includes_door_actions() {
    let engine = tracked_engine("undercroft_loop", "profile/undercroft_loop");
    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");
    let doors: Vec<_> = options
        .iter()
        .filter(|o| o.id.starts_with("open_") || o.id.starts_with("close_"))
        .collect();
    assert!(!doors.is_empty(), "must have door actions");
    for d in &doors {
        assert!(d.command.is_some(), "door action must have command");
    }
}

#[test]
fn action_options_includes_item_actions() {
    let engine = tracked_engine("supply_cache", "profile/supply_cache");
    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");
    let moves: Vec<_> = options
        .iter()
        .filter(|option| option.id.starts_with("move_hemp_rope_to_"))
        .collect();
    assert!(
        !moves.is_empty(),
        "must have exact move options for ground items"
    );
    for option in &moves {
        if let Some(ref cmd) = option.command {
            match &cmd.intent {
                PlayerIntentPayloadV1::MoveItem {
                    item_instance_id,
                    destination: tme_rules::ItemMoveDestination::Carried { .. },
                } => {
                    assert!(
                        !item_instance_id.is_empty(),
                        "item move must use item_instance_id"
                    );
                }
                _ => panic!("ground-item option must have a carried MoveItem payload"),
            }
        }
    }
}

#[test]
fn action_options_always_includes_show_sack_wait_inspect() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");
    let ids: Vec<_> = options.iter().map(|o| o.id.clone()).collect();
    assert!(
        ids.contains(&"show_sack".to_string()),
        "must have show_sack"
    );
    assert!(ids.contains(&"wait".to_string()), "must have wait");
    assert!(ids.contains(&"inspect".to_string()), "must have inspect");
    // These should always be enabled
    for id in ["show_sack", "wait", "inspect"] {
        let opt = options.iter().find(|o| o.id == id).unwrap();
        assert!(opt.enabled, "{id} must always be enabled");
        assert!(opt.command.is_some(), "{id} must have command");
    }
}

#[test]
fn command_to_intent_converts_all_variants() {
    let engine = tracked_engine("first_room", "profile/first_room");

    let cmd = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::MovePath {
            path: vec![tme_rules::Direction::East],
        },
    };
    let intent = engine.command_to_actor_intent(&cmd).expect("convert");
    assert_eq!(
        intent,
        PlayerIntent::MovePath(vec![tme_rules::Direction::East])
    );
}

#[test]
fn command_to_intent_rejects_wrong_actor() {
    let engine = tracked_engine("first_room", "profile/first_room");

    let cmd = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "not_the_player".into(),
        intent: PlayerIntentPayloadV1::Wait,
    };
    assert!(engine.command_to_actor_intent(&cmd).is_err());
}

#[test]
fn action_options_is_read_only() {
    let engine = tracked_engine("first_room", "profile/first_room");
    let opts1 = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("first");
    let opts2 = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("second");
    assert_eq!(opts1, opts2, "action options must be deterministic");
}

#[test]
fn command_envelope_serializes_deterministically() {
    let cmd = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::PhysicalAttack {
            authorization: tme_rules::HostilityAuthorization::Safe,
            mode: tme_rules::PhysicalAttackMode::Fight,
            target_actor_id: "mireling".into(),
        },
    };
    let json1 = serde_json::to_string(&cmd).expect("serialize");
    let json2 = serde_json::to_string(&cmd).expect("serialize");
    assert_eq!(json1, json2, "serialization must be deterministic");
    // Verify key fields are present
    assert!(json1.contains(&format!(
        r#""contract_version":{}"#,
        COMMAND_CONTRACT_VERSION
    )));
    assert!(json1.contains(r#""actor_id":"player""#));
    assert!(json1.contains(r#""target_actor_id":"mireling""#));
}

#[test]
fn command_26_service_transaction_requires_explicit_nullable_item_selection() {
    assert_eq!(COMMAND_CONTRACT_VERSION, 26);
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::CommitServiceTransaction {
            service_id: "clerk".to_string(),
            capability_id: "exchanges".to_string(),
            transaction_id: "token_for_badge".to_string(),
            item_instance_id: None,
        },
    };
    let value = serde_json::to_value(&command).expect("command serializes");
    assert_eq!(
        value["intent"]["commit_service_transaction"],
        serde_json::json!({
            "service_id": "clerk",
            "capability_id": "exchanges",
            "transaction_id": "token_for_badge",
            "item_instance_id": null
        })
    );
    serde_json::from_value::<PlayerCommandV1>(value.clone()).expect("command round trips");

    let mut missing = value.clone();
    missing["intent"]["commit_service_transaction"]
        .as_object_mut()
        .expect("payload")
        .remove("item_instance_id");
    assert!(serde_json::from_value::<PlayerCommandV1>(missing).is_err());

    let mut obsolete = value;
    obsolete["intent"]["commit_service_transaction"]["item_id"] = serde_json::json!("token");
    assert!(serde_json::from_value::<PlayerCommandV1>(obsolete).is_err());
}

#[test]
fn command_26_storage_and_offer_payloads_are_exact_strict_and_current() {
    assert_eq!(COMMAND_CONTRACT_VERSION, 26);
    let payloads = [
        serde_json::json!({
            "move_gold": {
                "source": {"kind": "carried", "position": "sack"},
                "destination": {"kind": "ground_here"},
                "quantity": {"kind": "exact", "amount": 25}
            }
        }),
        serde_json::json!({
            "deposit_bank_gold": {
                "service_id": "counter",
                "capability_id": "bank",
                "gold_pile_id": "gold:1"
            }
        }),
        serde_json::json!({
            "withdraw_bank_gold": {
                "service_id": "counter",
                "capability_id": "bank",
                "amount": 40
            }
        }),
        serde_json::json!({
            "deposit_locker_item": {
                "service_id": "counter",
                "capability_id": "locker",
                "item_instance_id": "keepsake"
            }
        }),
        serde_json::json!({
            "withdraw_locker_item": {
                "service_id": "counter",
                "capability_id": "locker",
                "item_instance_id": "keepsake",
                "destination": "sack_item_1"
            }
        }),
        serde_json::json!({
            "offer_item": {
                "recipient_character_id": "character:recipient",
                "item_instance_id": "keepsake"
            }
        }),
        serde_json::json!({
            "accept_item_offer": {
                "item_instance_id": "keepsake",
                "destination": "left_hand"
            }
        }),
        serde_json::json!({"refuse_item_offer": {"item_instance_id": "keepsake"}}),
        serde_json::json!({"withdraw_item_offer": {"item_instance_id": "keepsake"}}),
    ];

    for intent in payloads {
        let value = serde_json::json!({
            "contract_version": 26,
            "actor_id": "player",
            "intent": intent
        });
        let decoded = serde_json::from_value::<PlayerCommandV1>(value.clone())
            .expect("ED command shape should deserialize");
        assert_eq!(
            serde_json::to_value(decoded).expect("ED command should serialize"),
            value
        );

        let mut unknown = value;
        let payload = unknown["intent"]
            .as_object_mut()
            .expect("tagged intent")
            .values_mut()
            .next()
            .expect("payload")
            .as_object_mut()
            .expect("object payload");
        payload.insert("legacy".to_string(), serde_json::json!(true));
        assert!(serde_json::from_value::<PlayerCommandV1>(unknown).is_err());
    }

    let engine = first_room_engine();
    let stale = PlayerCommandV1 {
        contract_version: 18,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::Wait,
    };
    let status = engine
        .validate_actor_command(&stale)
        .expect("old version status");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::OutOfBounds)
    );
    assert!(engine.command_to_actor_intent(&stale).is_err());
}

#[test]
fn command_to_intent_then_step_matches_direct_step() {
    // Prove that going through command->intent->step produces the same result
    // as calling step directly with the equivalent PlayerIntent.
    // Engine A: direct PlayerIntent
    let mut engine_a = tracked_engine("first_room", "profile/first_room");
    let events_a = engine_a
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![tme_rules::Direction::East]),
        )
        .expect("direct step");

    // Engine B: via command -> intent -> step
    let mut engine_b = tracked_engine("first_room", "profile/first_room");
    let cmd = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::MovePath {
            path: vec![tme_rules::Direction::East],
        },
    };
    let intent = engine_b.command_to_actor_intent(&cmd).expect("convert");
    let events_b = engine_b
        .apply_actor_intent(&tme_rules::ActorId::from("player"), intent)
        .expect("command step");

    assert_eq!(
        events_a, events_b,
        "both paths must produce identical events"
    );
}

#[test]
fn bq_reserved_blocked_reasons_serialize_and_display_with_command_codes() {
    let expected = [
        (
            ActionBlockedReasonV1::NoSuchSpell,
            "no_such_spell",
            "no such spell",
        ),
        (
            ActionBlockedReasonV1::SpellNotKnown,
            "spell_not_known",
            "spell not known",
        ),
        (
            ActionBlockedReasonV1::WrongClass,
            "wrong_class",
            "wrong class",
        ),
        (
            ActionBlockedReasonV1::SkillLevelTooLow,
            "skill_level_too_low",
            "skill level too low",
        ),
        (
            ActionBlockedReasonV1::InsufficientMagicPoints,
            "insufficient_magic_points",
            "insufficient magic points",
        ),
        (
            ActionBlockedReasonV1::InsufficientStamina,
            "insufficient_stamina",
            "insufficient stamina",
        ),
        (
            ActionBlockedReasonV1::InvalidTarget,
            "invalid_target",
            "invalid target",
        ),
        (
            ActionBlockedReasonV1::TargetNotVisible,
            "target_not_visible",
            "target not visible",
        ),
        (
            ActionBlockedReasonV1::TargetOutOfRange,
            "target_out_of_range",
            "target out of range",
        ),
        (
            ActionBlockedReasonV1::EffectAlreadyActive,
            "effect_already_active",
            "effect already active",
        ),
        (
            ActionBlockedReasonV1::TargetImmune,
            "target_immune",
            "target immune",
        ),
        (
            ActionBlockedReasonV1::EffectResisted,
            "effect_resisted",
            "effect resisted",
        ),
        (
            ActionBlockedReasonV1::MissingRequiredItem,
            "missing_required_item",
            "missing required item",
        ),
        (
            ActionBlockedReasonV1::SpellAlreadyKnown,
            "spell_already_known",
            "spell already known",
        ),
        (ActionBlockedReasonV1::NoService, "no_service", "no service"),
        (
            ActionBlockedReasonV1::SpellRequiresWarming,
            "spell_requires_warming",
            "spell requires warming",
        ),
        (
            ActionBlockedReasonV1::SpellCastsDirectly,
            "spell_casts_directly",
            "spell casts directly",
        ),
        (
            ActionBlockedReasonV1::NoWarmedSpell,
            "no_warmed_spell",
            "no warmed spell",
        ),
        (
            ActionBlockedReasonV1::SpellStillWarming,
            "spell_still_warming",
            "spell is still warming",
        ),
    ];

    for (reason, code, display) in expected {
        assert_eq!(reason.code(), code);
        assert_eq!(
            serde_json::to_value(reason).unwrap(),
            serde_json::json!(code)
        );
        assert_eq!(reason.to_string(), display);

        let command = PlayerCommandV1 {
            contract_version: COMMAND_CONTRACT_VERSION,
            actor_id: "player".into(),
            intent: PlayerIntentPayloadV1::Wait,
        };
        let status = PlayerCommandStatusV1 {
            contract_version: COMMAND_CONTRACT_VERSION,
            command,
            accepted: false,
            blocked_reason: Some(reason),
        };
        assert_eq!(
            serde_json::to_value(&status).unwrap()["blocked_reason"],
            serde_json::json!(code)
        );
    }
}

#[test]
fn validate_rejects_wrong_contract_version() {
    let engine = first_room_engine();
    let cmd = PlayerCommandV1 {
        contract_version: 999,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::Wait,
    };
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::OutOfBounds)
    );
}

#[test]
fn eu_contract_versions_are_exact_and_trace_envelopes_remain_current() {
    assert_eq!(tme_rules::EVENT_CONTRACT_VERSION, 41);
    assert_eq!(tme_rules::SNAPSHOT_CONTRACT_VERSION, 31);
    assert_eq!(tme_rules::OBSERVED_SNAPSHOT_CONTRACT_VERSION, 30);
    assert_eq!(tme_rules::ACTION_CONTEXT_CONTRACT_VERSION, 32);
    assert_eq!(tme_rules::COMMAND_CONTRACT_VERSION, 26);
    assert_eq!(tme_rules::PATH_PREVIEW_CONTRACT_VERSION, 8);
    assert_eq!(tme_rules::TRACE_V2_CONTRACT_VERSION, 2);
    assert_eq!(tme_rules::TRACE_CONTRACT_VERSION, 1);
}

#[test]
fn validate_rejects_wrong_actor() {
    let engine = first_room_engine();
    let cmd = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "not_the_player".into(),
        intent: PlayerIntentPayloadV1::Wait,
    };
    assert!(engine.validate_actor_command(&cmd).is_err());
}

#[test]
fn validate_rejects_move_into_wall() {
    let engine = first_room_engine();
    // first_room has player at (1,1). North is a Stone Wall (impassable).
    let cmd = make_command(PlayerIntentPayloadV1::MovePath {
        path: vec![Direction::North],
    });
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::BlockedTerrain)
    );
}

#[test]
fn validate_accepts_move_to_open_hex() {
    let engine = first_room_engine();
    let cmd = make_command(PlayerIntentPayloadV1::MovePath {
        path: vec![Direction::East],
    });
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(status.accepted);
    assert_eq!(status.blocked_reason, None);
}

#[test]
fn validate_rejects_attack_nonexistent_target() {
    let engine = first_room_engine();
    let cmd = make_command(PlayerIntentPayloadV1::PhysicalAttack {
        authorization: tme_rules::HostilityAuthorization::Safe,
        mode: tme_rules::PhysicalAttackMode::Fight,
        target_actor_id: "nonexistent".into(),
    });
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::NoSuchTarget)
    );
}

#[test]
fn validate_rejects_attack_not_engaged() {
    let engine = first_room_engine();
    // Mireling is at (2,1), player at (1,1) — not engaged yet
    let cmd = make_command(PlayerIntentPayloadV1::PhysicalAttack {
        authorization: tme_rules::HostilityAuthorization::Safe,
        mode: tme_rules::PhysicalAttackMode::Fight,
        target_actor_id: "mireling".into(),
    });
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::NotEngaged)
    );
}

#[test]
fn validate_accepts_attack_engaged_target() {
    let mut engine = first_room_engine();
    // Move east lets the mireling engage at time one. Both actors' physical
    // attack gates become time two, which is also the returned player time.
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("move");
    let cmd = make_command(PlayerIntentPayloadV1::PhysicalAttack {
        authorization: tme_rules::HostilityAuthorization::Safe,
        mode: tme_rules::PhysicalAttackMode::Fight,
        target_actor_id: "mireling".into(),
    });
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(
        status.accepted,
        "engaged attack should be accepted, got: {:?}",
        status.blocked_reason
    );
}

#[test]
fn validate_accepts_show_sack_wait_inspect() {
    let engine = first_room_engine();
    for intent in [
        PlayerIntentPayloadV1::ShowSack,
        PlayerIntentPayloadV1::Wait,
        PlayerIntentPayloadV1::Inspect,
    ] {
        let cmd = make_command(intent);
        let status = engine.validate_actor_command(&cmd).expect("validate");
        assert!(status.accepted, "always-enabled action should be accepted");
    }
}

#[test]
fn command_to_intent_rejects_stale_contract_version() {
    let engine = first_room_engine();
    let cmd = PlayerCommandV1 {
        contract_version: 0,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::Wait,
    };
    let result = engine.command_to_actor_intent(&cmd);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("contract version"));
}

#[test]
fn path_preview_reports_ordinary_movement_toward_an_occupied_hex() {
    let engine = tracked_engine("first_room", "profile/first_room");

    let preview = engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::East])
        .expect("preview");
    assert_eq!(
        preview.stop_reason,
        tme_rules::MovementStopReason::FullPathAccepted
    );
    assert!(matches!(
        preview.steps[0].outcome,
        tme_rules::PathPreviewStepOutcomeV1::Moved { .. }
    ));
}

#[test]
fn validate_rejects_movepath_into_wall() {
    let engine = first_room_engine();
    let cmd = make_command(PlayerIntentPayloadV1::MovePath {
        path: vec![Direction::North],
    });
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::BlockedTerrain)
    );
}

#[test]
fn validate_rejects_open_nonexistent_door() {
    let engine = first_room_engine();
    let cmd = make_command(PlayerIntentPayloadV1::Open {
        direction: Direction::East,
    });
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(!status.accepted);
    // first_room has no door east of player
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::NoSuchTarget)
    );
}

#[test]
fn validate_rejects_close_nonexistent_door() {
    let engine = first_room_engine();
    let cmd = make_command(PlayerIntentPayloadV1::Close {
        direction: Direction::East,
    });
    let status = engine.validate_actor_command(&cmd).expect("validate");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::NoSuchTarget)
    );
}

#[test]
fn action_options_disable_suppressed_movement_but_keep_passive_actions_enabled() {
    let engine = suppressed_options_engine();
    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");

    let move_east = option_by_id(&options, "move_east");
    assert!(!move_east.enabled, "suppressed movement should be disabled");
    assert_eq!(
        move_east.blocked_reason,
        Some(ActionBlockedReasonV1::SuppressedByStatus)
    );

    let attack = option_by_id(&options, "physical_attack_fight_watcher");
    assert!(!attack.enabled, "suppressed attack should be disabled");
    assert_eq!(
        attack.blocked_reason,
        Some(ActionBlockedReasonV1::SuppressedByStatus)
    );

    let take = option_by_id(&options, "move_healing_balm_to_sack_item_1");
    assert!(!take.enabled, "suppressed item move should be disabled");
    assert_eq!(
        take.blocked_reason,
        Some(ActionBlockedReasonV1::SuppressedByStatus)
    );

    assert!(
        options.iter().all(|option| option.id != "cast_spark"),
        "a disabled spell descriptor must not fabricate a concrete action option"
    );
    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("suppressed action context");
    let cast = &context
        .spell_actions
        .iter()
        .find(|spell| spell.spell_id == "spark")
        .expect("spark descriptor")
        .cast;
    assert!(!cast.enabled, "suppressed cast should be disabled");
    assert_eq!(
        cast.blocked_reason,
        Some(ActionBlockedReasonV1::SuppressedByStatus)
    );
    assert_eq!(cast.command, None);

    let show_sack = option_by_id(&options, "show_sack");
    assert!(
        !show_sack.enabled,
        "suppressed show_sack should be disabled"
    );
    assert_eq!(
        show_sack.blocked_reason,
        Some(ActionBlockedReasonV1::SuppressedByStatus)
    );

    let wait = option_by_id(&options, "wait");
    assert!(wait.enabled, "wait stays passive under suppression");
    assert_eq!(wait.blocked_reason, None);

    let inspect = option_by_id(&options, "inspect");
    assert!(inspect.enabled, "inspect stays passive under suppression");
    assert_eq!(inspect.blocked_reason, None);
}

#[test]
fn validate_rejects_attack_out_of_range() {
    let mut parts = ContentParts::tracked("reach_attack", "profile/reach_attack");
    parts.actors_mut()[1]["location"]["position"] = serde_json::json!({"x": 3, "y": 1});
    let engine = parts.engine(7).expect("reach content should start");
    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::PhysicalAttack {
            authorization: tme_rules::HostilityAuthorization::Safe,
            mode: tme_rules::PhysicalAttackMode::Poke,
            target_actor_id: "reedling".into(),
        },
    };
    let status = engine
        .validate_actor_command(&command)
        .expect("validate command");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::OutOfRange)
    );
}
