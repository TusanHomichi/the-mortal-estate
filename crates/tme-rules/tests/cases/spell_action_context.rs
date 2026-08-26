use tme_rules::{
    ACTION_CONTEXT_CONTRACT_VERSION, ActionBlockedReasonV1, COMMAND_CONTRACT_VERSION, Engine,
    Event, PlayerIntent, PlayerIntentPayloadV1, SpellCastClass, SpellCastingMethod, SpellTarget,
    WarmedSpellStatus,
};

use crate::action_context_support::common::{make_command, option_by_id};
use crate::action_context_support::spell_learning::learn_spell_context_engine;
use crate::action_context_support::spell_readiness::spell_readiness_options_engine;
use crate::action_context_support::spell_warmed::bt_warmed_spell_engine;
use crate::action_context_support::spell_wizard::wizard_spell_context_engine;

#[test]
fn warm_and_cast_warmed_payloads_round_trip_truthfully() {
    let mut engine = spell_readiness_options_engine();

    let warm_payload = Engine::player_intent_to_payload(&PlayerIntent::WarmSpell {
        spell_id: "charged_spark".to_string(),
    });
    assert_eq!(
        warm_payload,
        PlayerIntentPayloadV1::WarmSpell {
            spell_id: "charged_spark".to_string(),
        }
    );
    let warm_command = make_command(warm_payload);
    assert!(matches!(
        engine
            .command_to_actor_intent(&warm_command)
            .expect("warm command converts"),
        PlayerIntent::WarmSpell { ref spell_id } if spell_id == "charged_spark"
    ));
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("warm");

    let cast_payload = Engine::player_intent_to_payload(&PlayerIntent::CastWarmedSpell {
        target: Some(tme_rules::SpellTarget::Actor {
            actor_id: "watcher".into(),
        }),
        authorization: tme_rules::HostilityAuthorization::Safe,
    });
    assert_eq!(
        cast_payload,
        PlayerIntentPayloadV1::CastWarmedSpell {
            target: Some(tme_rules::SpellTarget::Actor {
                actor_id: "watcher".into(),
            }),
            authorization: tme_rules::HostilityAuthorization::Safe,
        }
    );
    let cast_command = make_command(cast_payload);
    assert!(matches!(
        engine
            .command_to_actor_intent(&cast_command)
            .expect("warmed cast command converts"),
        PlayerIntent::CastWarmedSpell {
            target: Some(tme_rules::SpellTarget::Actor { ref actor_id }),
            authorization: tme_rules::HostilityAuthorization::Safe,
        } if actor_id == "watcher"
    ));
}

#[test]
fn spell_command_payloads_serialize_with_typed_targets() {
    assert_eq!(COMMAND_CONTRACT_VERSION, 26, "EU uses command contract v26");

    let cast = make_command(PlayerIntentPayloadV1::CastSpell {
        spell_id: "spark".to_string(),
        target: Some(tme_rules::SpellTarget::Actor {
            actor_id: "mireling".into(),
        }),
        authorization: tme_rules::HostilityAuthorization::Safe,
    });
    let value = serde_json::to_value(&cast).expect("command serializes");

    assert_eq!(
        value["contract_version"],
        serde_json::json!(COMMAND_CONTRACT_VERSION)
    );
    assert_eq!(value["intent"]["cast_spell"]["spell_id"], "spark");
    assert_eq!(
        value["intent"]["cast_spell"]["target"]["actor"]["actor_id"],
        "mireling"
    );

    let warm = make_command(PlayerIntentPayloadV1::WarmSpell {
        spell_id: "charged_spark".to_string(),
    });
    let cast_warmed = make_command(PlayerIntentPayloadV1::CastWarmedSpell {
        target: Some(tme_rules::SpellTarget::SelfTarget),
        authorization: tme_rules::HostilityAuthorization::Safe,
    });
    assert!(
        serde_json::to_value(warm).unwrap()["intent"]
            .get("warm_spell")
            .is_some()
    );
    assert!(
        serde_json::to_value(cast_warmed).unwrap()["intent"]
            .get("cast_warmed_spell")
            .is_some()
    );
}

#[test]
fn spell_item_target_json_uses_item_instance_id_and_rejects_item_id() {
    let target: SpellTarget = serde_json::from_value(serde_json::json!({
        "item": {
            "item_instance_id": "tonic_a",
            "location": "ground_here"
        }
    }))
    .expect("explicit item-instance spell target should deserialize");
    let serialized = serde_json::to_value(&target).expect("spell target should serialize");
    assert_eq!(serialized["item"]["item_instance_id"], "tonic_a");
    assert!(serialized["item"].get("item_id").is_none());

    assert!(
        serde_json::from_value::<SpellTarget>(serde_json::json!({
            "item": {"item_id": "tonic_a", "location": "ground_here"}
        }))
        .is_err(),
        "the obsolete item_id spell target key must not be accepted"
    );
}

#[test]
fn learn_spell_command_contract_round_trips_with_version_16() {
    assert_eq!(COMMAND_CONTRACT_VERSION, 26);

    let mut engine = learn_spell_context_engine(|_| {});
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spell_book".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("should take personal Spell Book");

    let intent = PlayerIntent::LearnSpell("find_target".to_string());
    let payload = Engine::player_intent_to_payload(&intent);
    assert!(matches!(
        payload,
        PlayerIntentPayloadV1::LearnSpell { ref spell_id } if spell_id == "find_target"
    ));

    let command = make_command(payload);
    let status = engine.validate_actor_command(&command).expect("validate");
    assert!(status.accepted);
    assert_eq!(
        engine
            .command_to_actor_intent(&command)
            .expect("command should round-trip"),
        intent
    );
}

#[test]
fn cast_command_reports_magic_specific_blocked_reasons() {
    let mut spark_engine = wizard_spell_context_engine(Some("spark"), 1);

    let unknown = make_command(PlayerIntentPayloadV1::CastSpell {
        spell_id: "unknown_spell".to_string(),
        target: None,
        authorization: tme_rules::HostilityAuthorization::Safe,
    });
    assert_eq!(
        spark_engine
            .validate_actor_command(&unknown)
            .expect("status")
            .blocked_reason,
        Some(ActionBlockedReasonV1::NoSuchSpell)
    );

    let unlearned = make_command(PlayerIntentPayloadV1::CastSpell {
        spell_id: "mend".to_string(),
        target: Some(tme_rules::SpellTarget::SelfTarget),
        authorization: tme_rules::HostilityAuthorization::Safe,
    });
    assert_eq!(
        spark_engine
            .validate_actor_command(&unlearned)
            .expect("status")
            .blocked_reason,
        Some(ActionBlockedReasonV1::SpellNotKnown)
    );

    let charged_engine = wizard_spell_context_engine(Some("charged_spark"), 1);
    let warm_required_cast = make_command(PlayerIntentPayloadV1::CastSpell {
        spell_id: "charged_spark".to_string(),
        target: Some(tme_rules::SpellTarget::Actor {
            actor_id: "target".into(),
        }),
        authorization: tme_rules::HostilityAuthorization::Safe,
    });
    assert_eq!(
        charged_engine
            .validate_actor_command(&warm_required_cast)
            .expect("status")
            .blocked_reason,
        Some(ActionBlockedReasonV1::SpellRequiresWarming)
    );

    let direct_warm = make_command(PlayerIntentPayloadV1::WarmSpell {
        spell_id: "spark".to_string(),
    });
    assert_eq!(
        spark_engine
            .validate_actor_command(&direct_warm)
            .expect("status")
            .blocked_reason,
        Some(ActionBlockedReasonV1::SpellCastsDirectly)
    );

    let events = spark_engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "spark".to_string(),
                target: Some(tme_rules::SpellTarget::Actor {
                    actor_id: "target".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("typed actor cast succeeds");
    assert!(events.iter().any(
        |event| matches!(event, Event::SpellCastStubbed { spell_id, .. } if spell_id == "spark")
    ));
}

#[test]
fn learn_spell_validate_accepts_local_teacher_and_rejects_away_room() {
    let mut local_engine = learn_spell_context_engine(|_| {});
    local_engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spell_book".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("should take personal Spell Book");
    let command = make_command(PlayerIntentPayloadV1::LearnSpell {
        spell_id: "find_target".to_string(),
    });
    let local_status = local_engine
        .validate_actor_command(&command)
        .expect("validate local teacher");
    assert!(local_status.accepted);
    assert_eq!(local_status.blocked_reason, None);

    let mut away_engine = learn_spell_context_engine(|parts| {
        parts.service_instances_mut()[0]["location"]["level"] = serde_json::json!("hall");
    });
    away_engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spell_book".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("should take personal Spell Book");
    let away_status = away_engine
        .validate_actor_command(&command)
        .expect("validate away teacher");
    assert!(!away_status.accepted);
    assert_eq!(
        away_status.blocked_reason,
        Some(ActionBlockedReasonV1::ServiceNotHere)
    );
}

#[test]
fn learn_spell_validate_reports_specific_blocked_reasons() {
    let mut wrong_class = learn_spell_context_engine(|parts| {
        parts.actors_mut()[0]["character"]["identity"]["base_class_id"] =
            serde_json::json!("fighter");
        parts.actors_mut()[0]["character"]["identity"]["current_class_id"] =
            serde_json::json!("fighter");
        parts.actors_mut()[0]["character"]["identity"]["display_class"] =
            serde_json::json!("Fighter");
        parts.actors_mut()[0]["character"]["skill_ledger"] = serde_json::json!([]);
    });
    wrong_class
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spell_book".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("should take personal Spell Book");
    let wrong_class_status = wrong_class
        .validate_actor_command(&make_command(PlayerIntentPayloadV1::LearnSpell {
            spell_id: "find_target".to_string(),
        }))
        .expect("validate wrong class");
    assert!(!wrong_class_status.accepted);
    assert_eq!(
        wrong_class_status.blocked_reason,
        Some(ActionBlockedReasonV1::WrongClass)
    );

    let mut low_skill_level = learn_spell_context_engine(|parts| {
        parts.actors_mut()[0]["character"]["skill_ledger"][0]["level"] = serde_json::json!(0);
    });
    low_skill_level
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spell_book".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("should take personal Spell Book");
    let low_skill_level_status = low_skill_level
        .validate_actor_command(&make_command(PlayerIntentPayloadV1::LearnSpell {
            spell_id: "find_target".to_string(),
        }))
        .expect("validate low skill level");
    assert!(!low_skill_level_status.accepted);
    assert_eq!(
        low_skill_level_status.blocked_reason,
        Some(ActionBlockedReasonV1::SkillLevelTooLow)
    );

    let mut low_level = learn_spell_context_engine(|parts| {
        parts.actors_mut()[0]["character"]["progression"]["level"] = serde_json::json!(1);
        parts.actors_mut()[0]["character"]["progression"]["experience"] = serde_json::json!(0);
    });
    low_level
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spell_book".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("should take personal Spell Book");
    let low_level_status = low_level
        .validate_actor_command(&make_command(PlayerIntentPayloadV1::LearnSpell {
            spell_id: "find_target".to_string(),
        }))
        .expect("validate low level");
    assert!(
        low_level_status.accepted,
        "learning has no duplicated character-level gate"
    );
    assert_eq!(low_level_status.blocked_reason, None);

    let mut low_gold = learn_spell_context_engine(|parts| {
        parts.actors_mut()[0]["carried"]["gold"]["sack"] = serde_json::json!(24);
    });
    low_gold
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spell_book".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("should take personal Spell Book");
    let low_gold_status = low_gold
        .validate_actor_command(&make_command(PlayerIntentPayloadV1::LearnSpell {
            spell_id: "find_target".to_string(),
        }))
        .expect("validate low gold");
    assert!(!low_gold_status.accepted);
    assert_eq!(
        low_gold_status.blocked_reason,
        Some(ActionBlockedReasonV1::InsufficientGold)
    );

    let missing_item = learn_spell_context_engine(|_| {});
    let missing_item_status = missing_item
        .validate_actor_command(&make_command(PlayerIntentPayloadV1::LearnSpell {
            spell_id: "find_target".to_string(),
        }))
        .expect("validate missing item");
    assert!(!missing_item_status.accepted);
    assert_eq!(
        missing_item_status.blocked_reason,
        Some(ActionBlockedReasonV1::SpellBookRequired)
    );

    let mut already_known = learn_spell_context_engine(|parts| {
        parts.actors_mut()[0]["character"]["known_spells"] = serde_json::json!([
            {
                "spell_id": "find_target",
                "lane": "wizard_magic",
                "learned_at_level": 1
            }
        ]);
    });
    already_known
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spell_book".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("should take personal Spell Book");
    let already_known_status = already_known
        .validate_actor_command(&make_command(PlayerIntentPayloadV1::LearnSpell {
            spell_id: "find_target".to_string(),
        }))
        .expect("validate already known");
    assert!(!already_known_status.accepted);
    assert_eq!(
        already_known_status.blocked_reason,
        Some(ActionBlockedReasonV1::SpellAlreadyKnown)
    );
}

#[test]
fn learn_spell_validate_rejects_wrong_owner_book() {
    let mut engine = learn_spell_context_engine(|_| {});
    engine
        .world_mut()
        .item_instances
        .get_mut("spell_book")
        .unwrap()
        .binding = tme_rules::ItemBindingState::Bound {
        character_id: serde_json::from_str("\"character:someone_else\"").unwrap(),
    };
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spell_book".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("ordinary movement permits carrying another character's book");

    let status = engine
        .validate_actor_command(&make_command(PlayerIntentPayloadV1::LearnSpell {
            spell_id: "find_target".to_string(),
        }))
        .expect("validate wrong owner");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::SpellBookNotOwned)
    );
}

#[test]
fn spell_action_descriptors_expose_method_class_and_target_selection() {
    let engine = spell_readiness_options_engine();
    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context");
    assert_eq!(context.contract_version, ACTION_CONTEXT_CONTRACT_VERSION);
    assert_eq!(context.warmed_spell, None);
    let spark = context
        .spell_actions
        .iter()
        .find(|row| row.spell_id == "spark")
        .expect("spark descriptor");
    assert_eq!(spark.casting_method, SpellCastingMethod::Direct);
    assert_eq!(spark.cast_class, SpellCastClass::Character);
    assert!(!spark.warm.enabled);
    assert_eq!(
        spark.warm.blocked_reason,
        Some(ActionBlockedReasonV1::SpellCastsDirectly)
    );
    assert!(spark.cast.enabled);
    assert!(spark.cast.requires_target_selection);
    assert_eq!(spark.cast.command, None);

    let charged = context
        .spell_actions
        .iter()
        .find(|row| row.spell_id == "charged_spark")
        .expect("charged descriptor");
    assert_eq!(charged.casting_method, SpellCastingMethod::WarmThenCast);
    assert!(charged.warm.enabled);
    assert!(matches!(
        charged.warm.command.as_ref().map(|command| &command.intent),
        Some(PlayerIntentPayloadV1::WarmSpell { spell_id }) if spell_id == "charged_spark"
    ));
    assert!(!charged.cast.enabled);
    assert_eq!(
        charged.cast.blocked_reason,
        Some(ActionBlockedReasonV1::NoWarmedSpell)
    );
}

#[test]
fn warmed_descriptor_changes_to_ready_without_fabricating_target() {
    let mut engine = spell_readiness_options_engine();
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("warm");

    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context");
    let warmed = context.warmed_spell.as_ref().expect("warmed view");
    assert_eq!(warmed.spell_id, "charged_spark");
    assert_eq!(warmed.status, WarmedSpellStatus::Ready);
    let charged = context
        .spell_actions
        .iter()
        .find(|row| row.spell_id == "charged_spark")
        .expect("charged descriptor");
    assert!(charged.cast.enabled);
    assert!(charged.cast.requires_target_selection);
    assert_eq!(charged.cast.command, None);
}

#[test]
fn warm_and_warmed_cast_commands_validate_and_round_trip_without_repeated_id() {
    let mut engine = spell_readiness_options_engine();
    let warm = make_command(PlayerIntentPayloadV1::WarmSpell {
        spell_id: "charged_spark".to_string(),
    });
    assert!(
        engine
            .validate_actor_command(&warm)
            .expect("status")
            .accepted
    );
    assert!(matches!(
        engine.command_to_actor_intent(&warm).expect("intent"),
        PlayerIntent::WarmSpell { ref spell_id } if spell_id == "charged_spark"
    ));
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("warm");

    let cast = make_command(PlayerIntentPayloadV1::CastWarmedSpell {
        target: Some(SpellTarget::Actor {
            actor_id: "watcher".into(),
        }),
        authorization: tme_rules::HostilityAuthorization::Safe,
    });
    assert!(
        engine
            .validate_actor_command(&cast)
            .expect("status")
            .accepted
    );
    assert!(matches!(
        engine.command_to_actor_intent(&cast).expect("intent"),
        PlayerIntent::CastWarmedSpell {
            target: Some(SpellTarget::Actor { ref actor_id }),
            authorization: tme_rules::HostilityAuthorization::Safe,
        } if actor_id == "watcher"
    ));
    let value = serde_json::to_value(&cast).expect("serialize");
    assert!(
        value["intent"]["cast_warmed_spell"]
            .get("spell_id")
            .is_none()
    );

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastWarmedSpell {
                target: Some(SpellTarget::Actor {
                    actor_id: "watcher".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("cast");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::WarmedSpellCast { spell_id, .. } if spell_id == "charged_spark"
    )));
    assert_eq!(
        engine
            .actor_observed_action_context(&tme_rules::ActorId::from("player"))
            .expect("context")
            .warmed_spell,
        None
    );
}

#[test]
fn action_options_keep_rest_top_level_and_offer_fizzle_only_with_slot() {
    let mut engine = spell_readiness_options_engine();
    let initial = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");
    assert!(initial.iter().any(|option| option.id == "rest"));
    assert!(
        !initial
            .iter()
            .any(|option| option.id == "fizzle_warmed_spell")
    );

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            },
        )
        .expect("warm");
    let warmed = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");
    let fizzle = warmed
        .iter()
        .find(|option| option.id == "fizzle_warmed_spell")
        .expect("fizzle option");
    assert!(fizzle.enabled);
    assert!(matches!(
        fizzle.command.as_ref().map(|command| &command.intent),
        Some(PlayerIntentPayloadV1::FizzleWarmedSpell)
    ));
}

#[test]
fn path_cast_descriptors_require_player_selection() {
    let engine = bt_warmed_spell_engine(&["web_field", "ember_cloud"]);
    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context");
    for spell_id in ["web_field", "ember_cloud"] {
        let row = context
            .spell_actions
            .iter()
            .find(|row| row.spell_id == spell_id)
            .expect("path row");
        assert_eq!(row.cast_class, SpellCastClass::Path);
        assert!(row.warm.enabled);
        assert!(!row.cast.enabled);
        assert_eq!(
            row.cast.blocked_reason,
            Some(ActionBlockedReasonV1::NoWarmedSpell)
        );
    }
}

#[test]
fn learn_spell_action_options_include_typed_command_and_blocked_reason() {
    let mut enabled_engine = learn_spell_context_engine(|_| {});
    enabled_engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spell_book".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("should take personal Spell Book");
    let enabled_options = enabled_engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");
    let enabled = option_by_id(&enabled_options, "learn_spell_find_target");
    assert!(
        enabled.enabled,
        "learn option should be enabled near teacher"
    );
    assert_eq!(enabled.blocked_reason, None);
    assert!(matches!(
        enabled.command.as_ref().map(|command| &command.intent),
        Some(PlayerIntentPayloadV1::LearnSpell { spell_id }) if spell_id == "find_target"
    ));

    let mut known_engine = learn_spell_context_engine(|parts| {
        parts.actors_mut()[0]["character"]["known_spells"] = serde_json::json!([
            {
                "spell_id": "find_target",
                "lane": "wizard_magic",
                "learned_at_level": 1
            }
        ]);
    });
    known_engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "spell_book".to_string(),
                destination: tme_rules::ItemMoveDestination::Carried {
                    position: tme_rules::CarriedPosition::RightHand,
                },
            },
        )
        .expect("should take personal Spell Book");
    let known_options = known_engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("options");
    let known = option_by_id(&known_options, "learn_spell_find_target");
    assert!(!known.enabled, "known spell should disable learn option");
    assert_eq!(
        known.blocked_reason,
        Some(ActionBlockedReasonV1::SpellAlreadyKnown)
    );
}
