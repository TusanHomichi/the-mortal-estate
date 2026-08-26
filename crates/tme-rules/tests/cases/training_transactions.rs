use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};
use tme_rules::{
    ActionBlockedReasonV1, COMMAND_CONTRACT_VERSION, Engine, Event, PlayerCommandV1, PlayerIntent,
    PlayerIntentPayloadV1,
};

fn titles() -> Value {
    Value::Array(
        (0_u8..=19)
            .map(|level| json!({"level": level, "title": format!("Measure {level}")}))
            .collect(),
    )
}

fn training_parts() -> ContentParts {
    let mut parts = ContentParts::tracked("gold_training", "profile/gold_training");
    let skill_catalog = parts.skill_catalog_mut().expect("selected skill catalog");
    skill_catalog["ladders"][0]["titles"] = titles();
    skill_catalog["tracks"] = json!([
        {"id": "sword", "display": "Sword", "kind": "weapon", "ladder_id": "measure"},
        {"id": "mace", "display": "Mace", "kind": "weapon", "ladder_id": "measure"},
        {"id": "hand", "display": "Hand", "kind": "martial_arts", "ladder_id": "measure"},
        {"id": "lockcraft", "display": "Lockcraft", "kind": "thievery", "ladder_id": "measure"},
        {"id": "wizard_magic", "display": "Wizard Magic", "kind": "magic", "ladder_id": "measure", "eligible_class_ids": ["wizard"]}
    ]);

    let service = parts.selected_mut("service_definitions", 0);
    service["id"] = json!("trainer");
    service["name"] = json!("Practice Mentor");
    service["capabilities"][0]["offers"] = json!([
        {"track_id": "sword", "eligible_class_ids": ["fighter"], "minimum_category_level": 0, "maximum_category_level": 19},
        {"track_id": "hand", "eligible_class_ids": ["fighter"], "minimum_category_level": 0, "maximum_category_level": 19},
        {"track_id": "lockcraft", "eligible_class_ids": ["fighter"], "minimum_category_level": 0, "maximum_category_level": 19},
        {"track_id": "wizard_magic", "eligible_class_ids": ["wizard"], "minimum_category_level": 0, "maximum_category_level": 19}
    ]);
    parts.service_instances_mut()[0]["id"] = json!("trainer");
    parts.service_instances_mut()[0]["service_definition_id"] = json!("trainer");
    parts.actors_mut()[0]["character"]["skill_ledger"] = json!([{
        "track_id": "sword",
        "level": 0,
        "critique_rank": 0,
        "practice_points": 5,
        "learning_rate": 1
    }]);

    parts.push_selected(
        "items",
        "test/lock_pick",
        json!({
            "id": "lock_pick",
            "kind": "tool",
            "name": "Practice Pick",
            "capability": {"training_focus_for": ["lockcraft"]},
            "valid_placements": ["hand", "sack"],
            "economy": {"unit_burden": 1}
        }),
    );
    parts.push_selected(
        "items",
        "test/spell_book",
        json!({
            "id": "spell_book",
            "kind": "book",
            "name": "Practice Book",
            "capability": {"spell_book_for": ["wizard_magic"]},
            "valid_placements": ["hand", "sack"],
            "economy": {"unit_burden": 1}
        }),
    );
    parts
}

fn ensure_growth_profile(parts: &mut ContentParts) {
    let Some(class_id) = parts.actors_mut()[0]["character"]["identity"]["current_class_id"]
        .as_str()
        .map(str::to_string)
    else {
        return;
    };
    if parts.rules_source_mut()["progression"]["growth_profiles"]
        .as_array()
        .expect("growth profiles")
        .iter()
        .any(|profile| profile["class_id"] == class_id)
    {
        return;
    }
    let mut source = ContentParts::tracked("first_room", "profile/first_room");
    let profile = source.rules_source_mut()["progression"]["growth_profiles"]
        .as_array()
        .expect("source growth profiles")
        .iter()
        .find(|profile| profile["class_id"] == class_id)
        .unwrap_or_else(|| panic!("missing growth profile for {class_id:?}"))
        .clone();
    parts.rules_source_mut()["progression"]["growth_profiles"]
        .as_array_mut()
        .expect("growth profiles")
        .push(profile);
}

fn engine_with(mutate: impl FnOnce(&mut ContentParts)) -> Engine {
    let mut parts = training_parts();
    mutate(&mut parts);
    ensure_growth_profile(&mut parts);
    parts
        .engine(7)
        .unwrap_or_else(|error| panic!("focused training content must start: {error}"))
}

fn train(service_id: &str, offered_gold: i64) -> PlayerIntent {
    PlayerIntent::Train {
        service_id: service_id.to_string(),
        offered_gold,
    }
}

fn command(intent: PlayerIntentPayloadV1) -> PlayerCommandV1 {
    PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent,
    }
}

fn train_command(service_id: &str, offered_gold: i64) -> PlayerCommandV1 {
    command(PlayerIntentPayloadV1::Train {
        service_id: service_id.to_string(),
        offered_gold,
    })
}

fn critique_command(service_id: &str, track_id: &str) -> PlayerCommandV1 {
    command(PlayerIntentPayloadV1::Critique {
        service_id: service_id.to_string(),
        track_id: track_id.to_string(),
    })
}

#[test]
fn training_spends_only_absorbable_gold_awards_xp_and_preserves_position() {
    let mut engine = engine_with(|_| {});
    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), train("trainer", 40))
        .expect("training succeeds");

    let character = engine.world().actors[0].character.as_ref().unwrap();
    let sword = &character.skill_ledger[0];
    assert_eq!((sword.level, sword.critique_rank), (0, 0));
    assert_eq!(sword.practice_points, 5);
    assert_eq!(sword.learning_rate, 2);
    assert_eq!(character.progression.experience, 303);
    assert_eq!(engine.world().actors[0].carried.gold.sack, 493);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::SkillPositionChanged { .. }))
    );

    let gold_index = events
        .iter()
        .position(|event| matches!(event, Event::GoldChanged { .. }))
        .unwrap();
    let training_index = events
        .iter()
        .position(|event| matches!(event, Event::TrainingPurchased { .. }))
        .unwrap();
    let xp_index = events
        .iter()
        .position(|event| matches!(event, Event::ExperienceAwarded { .. }))
        .unwrap();
    let receipt_index = events
        .iter()
        .position(|event| matches!(event, Event::TransactionCommitted { .. }))
        .unwrap();
    assert!(gold_index < training_index && training_index < xp_index && xp_index < receipt_index);
    assert!(matches!(
        &events[receipt_index],
        Event::TransactionCommitted {
            source: tme_rules::TransactionSourceV1::SkillTraining { service_id, track_id, .. },
            ..
        } if service_id == "trainer" && track_id == "sword"
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::TrainingPurchased {
            service_id,
            track_id,
            offered_gold: 40,
            spent_gold: 7,
            unspent_gold: 33,
            previous_learning_rate: 1,
            new_learning_rate: 2,
            ..
        } if service_id == "trainer" && track_id == "sword"
    )));
}

#[test]
fn missing_skill_insertion_and_rate_caps_preserve_skill_position() {
    let mut untrained = engine_with(|parts| {
        parts.actors_mut()[0]["character"]["skill_ledger"] = json!([]);
    });
    untrained
        .apply_actor_intent(&tme_rules::ActorId::from("player"), train("trainer", 7))
        .expect("untrained skill buys one rate");
    let entry = &untrained.world().actors[0]
        .character
        .as_ref()
        .unwrap()
        .skill_ledger[0];
    assert_eq!(
        (
            entry.level,
            entry.critique_rank,
            entry.practice_points,
            entry.learning_rate,
        ),
        (0, 0, 0, 2)
    );

    let before = untrained.snapshot();
    assert_eq!(
        untrained
            .validate_actor_command(&train_command("trainer", 7))
            .unwrap()
            .blocked_reason,
        Some(ActionBlockedReasonV1::TrainingCapReached)
    );
    assert!(
        untrained
            .apply_actor_intent(&tme_rules::ActorId::from("player"), train("trainer", 7))
            .is_err()
    );
    assert_eq!(untrained.snapshot(), before);

    let above_cap = engine_with(|parts| {
        parts.actors_mut()[0]["character"]["skill_ledger"][0]["learning_rate"] = json!(3);
    });
    assert_eq!(
        above_cap
            .validate_actor_command(&train_command("trainer", 7))
            .unwrap()
            .blocked_reason,
        Some(ActionBlockedReasonV1::TrainingCapReached)
    );
}

#[test]
fn critique_is_focusless_read_only_rank_zero_aware_and_independent() {
    let mut engine = engine_with(|parts| {
        parts.actors_mut()[0]["carried"]["items"] = json!([]);
        *parts.item_instances_mut() = json!({});
        parts.actors_mut()[0]["character"]["skill_ledger"] = json!([]);
    });
    let before = engine.world().actors[0].character.clone();
    let before_time = engine.world().timing.now;
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Critique {
                service_id: "trainer".to_string(),
                track_id: "mace".to_string(),
            },
        )
        .expect("focusless critique succeeds");
    assert_eq!(engine.world().actors[0].character, before);
    assert!(engine.world().timing.now > before_time);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SkillCritiqued {
            service_id,
            track_id,
            level: 0,
            critique_rank: None,
            ..
        } if service_id == "trainer" && track_id == "mace"
    )));

    let no_critique = engine_with(|parts| {
        parts.selected_mut("service_definitions", 0)["capabilities"]
            .as_array_mut()
            .expect("capabilities")
            .retain(|capability| capability["kind"] != "skill_critique");
    });
    assert_eq!(
        no_critique
            .validate_actor_command(&critique_command("trainer", "mace"))
            .unwrap()
            .blocked_reason,
        Some(ActionBlockedReasonV1::NoService)
    );

    let mut rank = engine_with(|parts| {
        parts.actors_mut()[0]["character"]["skill_ledger"] = json!([{
            "track_id": "mace", "level": 5, "critique_rank": 3,
            "practice_points": 9, "learning_rate": 4
        }]);
    });
    let before = rank.world().actors[0].character.clone();
    let events = rank
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Critique {
                service_id: "trainer".to_string(),
                track_id: "mace".to_string(),
            },
        )
        .expect("attained rank is reportable");
    assert_eq!(rank.world().actors[0].character, before);
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SkillCritiqued {
            level: 5,
            critique_rank: Some(3),
            level_title: Some(title),
            ..
        } if title == "Measure 5"
    )));
}

#[test]
fn exact_service_coordinate_focus_and_validation_order_failures_are_typed() {
    let engine = engine_with(|_| {});
    assert_eq!(
        engine
            .validate_actor_command(&train_command("missing", 7))
            .unwrap()
            .blocked_reason,
        Some(ActionBlockedReasonV1::NoService)
    );
    assert_eq!(
        engine
            .validate_actor_command(&critique_command("trainer", "missing_track"))
            .unwrap()
            .blocked_reason,
        Some(ActionBlockedReasonV1::InvalidTrainingOffer)
    );

    let not_here = engine_with(|parts| {
        parts.template_levels_source_mut()["room_0"]["cells"] = json!([
            [["stone_wall"], ["stone_wall"], ["stone_wall"]],
            [["stone_wall"], ["flagstone"], ["flagstone"]],
            [["stone_wall"], ["flagstone"], ["stone_wall"]]
        ]);
        parts.service_instances_mut()[0]["location"]["position"] = json!({"x": 2, "y": 1});
    });
    assert_eq!(
        not_here
            .validate_actor_command(&train_command("trainer", 7))
            .unwrap()
            .blocked_reason,
        Some(ActionBlockedReasonV1::ServiceNotHere)
    );

    let missing_focus = engine_with(|parts| {
        parts.actors_mut()[0]["carried"]["items"] = json!([]);
        *parts.item_instances_mut() = json!({});
        parts.selected_mut("service_definitions", 0)["capabilities"][0]["offers"] = json!([{
            "track_id": "sword", "eligible_class_ids": ["fighter"],
            "minimum_category_level": 0, "maximum_category_level": 19
        }]);
    });
    assert_eq!(
        missing_focus
            .validate_actor_command(&train_command("trainer", 7))
            .unwrap()
            .blocked_reason,
        Some(ActionBlockedReasonV1::MissingTrainingFocus)
    );

    let no_sheet = engine_with(|parts| {
        parts.actors_mut()[0]
            .as_object_mut()
            .unwrap()
            .remove("character");
        parts.actors_mut()[0]
            .as_object_mut()
            .unwrap()
            .remove("character_id");
        parts.actor_definition_mut(0)["social"]["alignment_source"] =
            json!({"kind": "inherent", "alignment": "lawful"});
    });
    assert_eq!(
        no_sheet
            .validate_actor_command(&train_command("missing", 7))
            .unwrap()
            .blocked_reason,
        Some(ActionBlockedReasonV1::WrongClass)
    );
}

#[test]
fn explicit_service_id_and_weapon_capability_select_exact_training_facts() {
    let mut addressed = engine_with(|parts| {
        let original_definition = parts.selected_mut("service_definitions", 0).clone();
        parts.selected_mut("service_definitions", 0)["id"] = json!("first_trainer");
        parts.selected_mut("service_definitions", 0)["name"] = json!("First Mentor");
        parts.service_instances_mut()[0]["id"] = json!("first_trainer");
        parts.service_instances_mut()[0]["service_definition_id"] = json!("first_trainer");
        parts.push_selected("service_definitions", "test/trainer", original_definition);
        let mut second = parts.service_instances_mut()[0].clone();
        second["id"] = json!("trainer");
        second["service_definition_id"] = json!("trainer");
        parts
            .service_instances_mut()
            .as_array_mut()
            .unwrap()
            .push(second);
    });
    let events = addressed
        .apply_actor_intent(&tme_rules::ActorId::from("player"), train("trainer", 7))
        .expect("explicit second trainer is selected");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::TrainingPurchased { service_id, .. } if service_id == "trainer"
    )));

    let mut capability = engine_with(|parts| {
        parts.selected_mut("items", 0)["category"] = json!("mace");
        assert_eq!(
            parts.selected_mut("items", 0)["weapon"]["skill_track_id"],
            json!("sword")
        );
    });
    let events = capability
        .apply_actor_intent(&tme_rules::ActorId::from("player"), train("trainer", 7))
        .expect("weapon capability selects Sword");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::TrainingPurchased { track_id, .. } if track_id == "sword"
    )));
}

fn make_wizard(parts: &mut ContentParts) {
    parts.actors_mut()[0]["character"]["identity"]["base_class_id"] = json!("wizard");
    parts.actors_mut()[0]["character"]["identity"]["current_class_id"] = json!("wizard");
    parts.actors_mut()[0]["character"]["identity"]["display_class"] = json!("Wizard");
    parts.selected_mut("service_definitions", 0)["capabilities"][0]["offers"][0]["eligible_class_ids"] =
        json!(["wizard"]);
}

#[test]
fn tool_empty_hand_and_personal_spell_book_select_locked_tracks() {
    let mut lockcraft = engine_with(|parts| {
        parts.actors_mut()[0]["carried"]["items"][0]["item_instance_id"] = json!("lock_pick");
        *parts.item_instances_mut() = json!({
            "lock_pick": {"definition_id": "lock_pick", "binding": {"state": "unrestricted"}}
        });
        parts.actors_mut()[0]["character"]["skill_ledger"] = json!([]);
    });
    let events = lockcraft
        .apply_actor_intent(&tme_rules::ActorId::from("player"), train("trainer", 7))
        .expect("tool focus trains lockcraft");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::TrainingPurchased { track_id, .. } if track_id == "lockcraft"
    )));

    let mut hand = engine_with(|parts| {
        parts.actors_mut()[0]["carried"]["items"] = json!([]);
        *parts.item_instances_mut() = json!({});
        parts.actors_mut()[0]["character"]["skill_ledger"] = json!([]);
    });
    let events = hand
        .apply_actor_intent(&tme_rules::ActorId::from("player"), train("trainer", 7))
        .expect("empty hand trains hand");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::TrainingPurchased { track_id, .. } if track_id == "hand"
    )));

    let mut magic = engine_with(|parts| {
        make_wizard(parts);
        parts.actors_mut()[0]["character"]["skill_ledger"] = json!([]);
        parts.actors_mut()[0]["carried"]["items"][0]["item_instance_id"] = json!("spell_book");
        *parts.item_instances_mut() = json!({
            "spell_book": {
                "definition_id": "spell_book",
                "binding": {"state": "bound", "character_id": "character:gold_training:primary"}
            }
        });
    });
    let events = magic
        .apply_actor_intent(&tme_rules::ActorId::from("player"), train("trainer", 7))
        .expect("personal spell book trains class magic");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::TrainingPurchased { track_id, .. } if track_id == "wizard_magic"
    )));

    let missing_book = engine_with(|parts| {
        make_wizard(parts);
        parts.selected_mut("service_definitions", 0)["capabilities"][0]["offers"] = json!([{
            "track_id": "wizard_magic", "eligible_class_ids": ["wizard"],
            "minimum_category_level": 0, "maximum_category_level": 19
        }]);
    });
    assert_eq!(
        missing_book
            .validate_actor_command(&train_command("trainer", 7))
            .unwrap()
            .blocked_reason,
        Some(ActionBlockedReasonV1::SpellBookRequired)
    );

    let mut wrong_owner = engine_with(|parts| {
        make_wizard(parts);
        parts.actors_mut()[0]["character"]["skill_ledger"] = json!([]);
        parts.actors_mut()[0]["carried"]["items"][0]["item_instance_id"] = json!("spell_book");
        *parts.item_instances_mut() = json!({
            "spell_book": {
                "definition_id": "spell_book",
                "binding": {"state": "bound", "character_id": "character:gold_training:primary"}
            }
        });
    });
    wrong_owner
        .world_mut()
        .item_instances
        .get_mut("spell_book")
        .expect("spell book")
        .binding = tme_rules::ItemBindingState::Bound {
        character_id: serde_json::from_str("\"character:someone_else\"").unwrap(),
    };
    assert_eq!(
        wrong_owner
            .validate_actor_command(&train_command("trainer", 7))
            .unwrap()
            .blocked_reason,
        Some(ActionBlockedReasonV1::SpellBookNotOwned)
    );
}

#[test]
fn ambiguous_wrong_class_and_windowed_focus_fail_with_typed_reasons() {
    let wrong = engine_with(|parts| {
        parts.selected_mut("items", 1)["capability"]["training_focus_for"] = json!(["mace"]);
        parts.actors_mut()[0]["carried"]["items"][0]["item_instance_id"] = json!("lock_pick");
        *parts.item_instances_mut() = json!({
            "lock_pick": {"definition_id": "lock_pick", "binding": {"state": "unrestricted"}}
        });
    });
    assert_eq!(
        wrong
            .validate_actor_command(&train_command("trainer", 7))
            .unwrap()
            .blocked_reason,
        Some(ActionBlockedReasonV1::MissingTrainingFocus)
    );

    let ambiguous = engine_with(|parts| {
        parts.selected_mut("items", 1)["capability"]["training_focus_for"] =
            json!(["sword", "lockcraft"]);
        parts.actors_mut()[0]["carried"]["items"][0]["item_instance_id"] = json!("lock_pick");
        *parts.item_instances_mut() = json!({
            "lock_pick": {"definition_id": "lock_pick", "binding": {"state": "unrestricted"}}
        });
    });
    assert_eq!(
        ambiguous
            .validate_actor_command(&train_command("trainer", 7))
            .unwrap()
            .blocked_reason,
        Some(ActionBlockedReasonV1::InvalidTrainingOffer)
    );

    let outside = engine_with(|parts| {
        parts.actors_mut()[0]["character"]["skill_ledger"] = json!([
            {"track_id": "sword", "level": 0, "critique_rank": 0, "practice_points": 0, "learning_rate": 1},
            {"track_id": "mace", "level": 5, "critique_rank": 0, "practice_points": 0, "learning_rate": 1}
        ]);
        parts.selected_mut("service_definitions", 0)["capabilities"][0]["offers"][0]["maximum_category_level"] =
            json!(4);
    });
    assert_eq!(
        outside
            .validate_actor_command(&train_command("trainer", 7))
            .unwrap()
            .blocked_reason,
        Some(ActionBlockedReasonV1::OutsideTrainerWindow)
    );
}

#[test]
fn trainer_window_endpoints_and_class_specific_ceiling_are_exact() {
    let ledger = json!([
        {"track_id": "sword", "level": 0, "critique_rank": 0, "practice_points": 0, "learning_rate": 1},
        {"track_id": "mace", "level": 5, "critique_rank": 0, "practice_points": 0, "learning_rate": 1}
    ]);
    for boundary in ["minimum_category_level", "maximum_category_level"] {
        let engine = engine_with(|parts| {
            parts.actors_mut()[0]["character"]["skill_ledger"] = ledger.clone();
            parts.selected_mut("service_definitions", 0)["capabilities"][0]["offers"][0]
                [boundary] = json!(5);
        });
        assert!(
            engine
                .validate_actor_command(&train_command("trainer", 7))
                .unwrap()
                .accepted
        );
    }

    let prepare = |parts: &mut ContentParts| {
        parts.actors_mut()[0]["carried"]["items"] = json!([]);
        *parts.item_instances_mut() = json!({});
        parts.actors_mut()[0]["character"]["skill_ledger"] = json!([{
            "track_id": "sword", "level": 7, "critique_rank": 0,
            "practice_points": 0, "learning_rate": 1
        }]);
        parts.selected_mut("service_definitions", 0)["capabilities"][0]["offers"][1]["maximum_category_level"] =
            json!(6);
        parts.selected_mut("service_definitions", 0)["capabilities"][0]["offers"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "track_id": "hand", "eligible_class_ids": ["martial_artist"],
                "minimum_category_level": 0, "maximum_category_level": 19
            }));
    };
    let fighter = engine_with(prepare);
    assert_eq!(
        fighter
            .validate_actor_command(&train_command("trainer", 7))
            .unwrap()
            .blocked_reason,
        Some(ActionBlockedReasonV1::OutsideTrainerWindow)
    );
    let martial = engine_with(|parts| {
        prepare(parts);
        parts.actors_mut()[0]["character"]["identity"]["base_class_id"] = json!("martial_artist");
        parts.actors_mut()[0]["character"]["identity"]["current_class_id"] =
            json!("martial_artist");
        parts.actors_mut()[0]["character"]["identity"]["display_class"] = json!("Martial Artist");
    });
    assert!(
        martial
            .validate_actor_command(&train_command("trainer", 7))
            .unwrap()
            .accepted
    );
}

#[test]
fn insufficient_overflowing_and_exact_boundary_offers_are_atomic() {
    let mut insufficient = engine_with(|_| {});
    let before = insufficient.snapshot();
    assert!(
        insufficient
            .apply_actor_intent(&tme_rules::ActorId::from("player"), train("trainer", 6))
            .is_err()
    );
    assert_eq!(insufficient.snapshot(), before);
    assert_eq!(
        insufficient
            .validate_actor_command(&train_command("trainer", 501))
            .unwrap()
            .blocked_reason,
        Some(ActionBlockedReasonV1::InsufficientGold)
    );

    let mut overflow = engine_with(|parts| {
        parts.rules_source_mut()["skills"]["training"]["experience_per_learning_rate"] =
            json!(i32::MAX);
        parts.rules_source_mut()["skills"]["training"]["maximum_learning_rates"] = json!([
            3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22
        ]);
    });
    let before = overflow.snapshot();
    assert!(
        overflow
            .apply_actor_intent(&tme_rules::ActorId::from("player"), train("trainer", 14))
            .is_err()
    );
    assert_eq!(overflow.snapshot(), before);

    let mut maximum_gold = engine_with(|parts| {
        parts.rules_source_mut()["skills"]["training"]["gold_per_learning_rate"] = json!(i64::MAX);
        parts.actors_mut()[0]["carried"]["gold"]["sack"] = json!(i64::MAX);
    });
    maximum_gold
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            train("trainer", i64::MAX),
        )
        .expect("maximum signed gold buys one exact unit");
    assert_eq!(maximum_gold.world().actors[0].carried.gold.sack, 0);

    let mut maximum_rate = engine_with(|parts| {
        parts.rules_source_mut()["skills"]["training"]["gold_per_learning_rate"] = json!(1);
        parts.rules_source_mut()["skills"]["training"]["experience_per_learning_rate"] = json!(1);
        parts.rules_source_mut()["skills"]["training"]["maximum_learning_rates"] = json!(
            (0_u64..20)
                .map(|offset| u64::MAX - 19 + offset)
                .collect::<Vec<_>>()
        );
        parts.actors_mut()[0]["character"]["skill_ledger"] = json!([{
            "track_id": "sword", "level": 19, "critique_rank": 0,
            "practice_points": 5, "learning_rate": u64::MAX - 1
        }]);
    });
    maximum_rate
        .apply_actor_intent(&tme_rules::ActorId::from("player"), train("trainer", 1))
        .expect("last representable rate is purchasable");
    assert_eq!(
        maximum_rate.world().actors[0]
            .character
            .as_ref()
            .unwrap()
            .skill_ledger[0]
            .learning_rate,
        u64::MAX
    );
}

#[test]
fn command_round_trip_action_options_and_offer_validation_use_current_contracts() {
    assert_eq!(COMMAND_CONTRACT_VERSION, 26);
    let engine = engine_with(|_| {});
    for intent in [
        train("trainer", 7),
        PlayerIntent::Critique {
            service_id: "trainer".to_string(),
            track_id: "sword".to_string(),
        },
    ] {
        let command = engine
            .actor_command_for_intent(&tme_rules::ActorId::from("player"), &intent)
            .unwrap();
        assert_eq!(engine.command_to_actor_intent(&command).unwrap(), intent);
    }
    assert!(
        serde_json::from_value::<PlayerCommandV1>(json!({
            "contract_version": 12,
            "actor_id": "player",
            "intent": {"train": {"skill_id": "sword"}}
        }))
        .is_err()
    );

    let options = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("action options");
    assert!(options.iter().any(|option| matches!(
        option.command.as_ref().map(|command| &command.intent),
        Some(PlayerIntentPayloadV1::Train { service_id, offered_gold: 500 })
            if service_id == "trainer"
    )));
    assert!(options.iter().any(|option| matches!(
        option.command.as_ref().map(|command| &command.intent),
        Some(PlayerIntentPayloadV1::Critique { service_id, track_id })
            if service_id == "trainer" && track_id == "mace"
    )));

    let mut unknown_class = training_parts();
    unknown_class.selected_mut("service_definitions", 0)["capabilities"][0]["offers"][0]["eligible_class_ids"] =
        json!(["invented_class"]);
    let error = unknown_class
        .definition()
        .expect_err("unknown class")
        .to_string();
    assert!(error.contains("references unknown class"), "{error}");

    for field in ["training_offers", "kind"] {
        let mut obsolete = training_parts();
        obsolete.selected_mut("service_definitions", 0)[field] = if field == "kind" {
            json!("trainer")
        } else {
            json!([])
        };
        let error = obsolete
            .definition()
            .expect_err("obsolete service field")
            .to_string();
        assert!(
            error.contains(&format!("unknown field `{field}`")),
            "{error}"
        );
    }
}
