use crate::support::content_parts::ContentParts;
use tme_rules::{
    ACTION_CONTEXT_CONTRACT_VERSION, COMMAND_CONTRACT_VERSION, Direction, EVENT_CONTRACT_VERSION,
    Event, OBSERVED_SNAPSHOT_CONTRACT_VERSION, PlayerIntent, SNAPSHOT_CONTRACT_VERSION, SkillEntry,
};

fn progression_value() -> ContentParts {
    ContentParts::tracked("skill_progression", "profile/skill_progression")
}

fn content_error(value: &ContentParts) -> String {
    match value.validated_seed() {
        Ok(_) => panic!("mutated skill contract must be rejected"),
        Err(error) => error,
    }
}

#[test]
fn skill_position_owner_enforces_every_boundary_transition() {
    let mut untrained = SkillEntry::untrained("blade", 3);
    assert!(untrained.is_valid_position());
    assert!(untrained.has_valid_learning_rate());
    assert_eq!(untrained.learning_rate, 3);
    assert!(untrained.advance_position());
    assert_eq!((untrained.level, untrained.critique_rank), (1, 1));
    assert_eq!(untrained.learning_rate, 3);

    let mut uncritiqued = SkillEntry {
        track_id: "blade".to_string(),
        level: 4,
        critique_rank: 0,
        practice_points: 7,
        learning_rate: 4,
    };
    assert!(uncritiqued.is_valid_position());
    assert!(uncritiqued.advance_position());
    assert_eq!((uncritiqued.level, uncritiqued.critique_rank), (4, 1));
    assert_eq!(uncritiqued.practice_points, 7);
    assert_eq!(uncritiqued.learning_rate, 4);

    let mut within_level = SkillEntry {
        track_id: "blade".to_string(),
        level: 4,
        critique_rank: 9,
        practice_points: 0,
        learning_rate: 5,
    };
    assert!(within_level.advance_position());
    assert_eq!((within_level.level, within_level.critique_rank), (4, 10));

    let mut next_level = SkillEntry {
        track_id: "blade".to_string(),
        level: 4,
        critique_rank: 10,
        practice_points: 0,
        learning_rate: 6,
    };
    assert!(next_level.advance_position());
    assert_eq!((next_level.level, next_level.critique_rank), (5, 1));

    let mut maximum = SkillEntry {
        track_id: "blade".to_string(),
        level: 19,
        critique_rank: 10,
        practice_points: 3,
        learning_rate: 7,
    };
    assert!(maximum.is_maximum());
    assert!(!maximum.advance_position());
    assert_eq!((maximum.level, maximum.critique_rank), (19, 10));

    assert!(
        !SkillEntry {
            track_id: "blade".to_string(),
            level: 0,
            critique_rank: 1,
            practice_points: 0,
            learning_rate: 1,
        }
        .is_valid_position()
    );
    assert!(
        !SkillEntry {
            track_id: "blade".to_string(),
            level: 20,
            critique_rank: 0,
            practice_points: 0,
            learning_rate: 1,
        }
        .is_valid_position()
    );
    let mut invalid = SkillEntry {
        track_id: "blade".to_string(),
        level: 20,
        critique_rank: 10,
        practice_points: 0,
        learning_rate: 1,
    };
    assert!(!invalid.advance_position());
    assert_eq!((invalid.level, invalid.critique_rank), (20, 10));

    let zero_rate = SkillEntry::untrained("blade", 0);
    assert!(!zero_rate.has_valid_learning_rate());
}

#[test]
fn world_seed_rejects_invalid_duplicate_and_wrong_class_skill_state() {
    let mut invalid = progression_value();
    invalid.actors_mut()[0]["character"]["skill_ledger"] = serde_json::json!([{
        "track_id": "sword",
        "level": 0,
        "critique_rank": 1,
        "practice_points": 0,
        "learning_rate": 1
    }]);
    assert!(content_error(&invalid).contains("must use level 0/critique 0"));

    let mut duplicate = progression_value();
    duplicate.actors_mut()[0]["character"]["skill_ledger"] = serde_json::json!([
        {"track_id": "sword", "level": 1, "critique_rank": 0, "practice_points": 0, "learning_rate": 1},
        {"track_id": "sword", "level": 2, "critique_rank": 0, "practice_points": 0, "learning_rate": 1}
    ]);
    assert!(content_error(&duplicate).contains("track_id must be unique"));

    let mut wrong_class = progression_value();
    wrong_class.profile_value_mut()["skill_catalog"] = serde_json::Value::Null;
    wrong_class.actors_mut()[0]["character"]["skill_ledger"] = serde_json::json!([{
        "track_id": "wizard_magic",
        "level": 1,
        "critique_rank": 0,
        "practice_points": 0,
        "learning_rate": 1
    }]);
    assert!(
        content_error(&wrong_class)
            .contains("track_id is not a magic skill track for class \"fighter\"")
    );
}

#[test]
fn catalog_requires_complete_skill_rules_and_explicit_learning_rates() {
    let mut missing_rules = progression_value();
    missing_rules
        .rules_source_mut()
        .as_object_mut()
        .expect("rules object")
        .remove("skills");
    assert!(content_error(&missing_rules).contains("missing field `skills`"));

    let mut missing_thresholds = progression_value();
    missing_thresholds.rules_source_mut()["skills"]
        .as_object_mut()
        .expect("skill rules")
        .remove("practice_thresholds");
    assert!(content_error(&missing_thresholds).contains("missing field `practice_thresholds`"));

    let mut missing_training = progression_value();
    missing_training.rules_source_mut()["skills"]
        .as_object_mut()
        .expect("skill rules")
        .remove("training");
    assert!(content_error(&missing_training).contains("missing field `training`"));

    let mut short_thresholds = progression_value();
    short_thresholds.rules_source_mut()["skills"]["practice_thresholds"]
        .as_array_mut()
        .expect("practice thresholds")
        .pop();
    assert!(content_error(&short_thresholds).contains("exactly 20 level-ordered values"));

    let mut zero_threshold = progression_value();
    zero_threshold.rules_source_mut()["skills"]["practice_thresholds"][7] = serde_json::json!(0);
    assert!(content_error(&zero_threshold).contains("practice_thresholds[7] must be positive"));

    let mut short_caps = progression_value();
    short_caps.rules_source_mut()["skills"]["training"]["maximum_learning_rates"]
        .as_array_mut()
        .expect("learning-rate caps")
        .pop();
    assert!(content_error(&short_caps).contains("exactly 20 level-ordered values"));

    let mut zero_cap = progression_value();
    zero_cap.rules_source_mut()["skills"]["training"]["maximum_learning_rates"][0] =
        serde_json::json!(0);
    assert!(content_error(&zero_cap).contains("maximum_learning_rates[0] must be positive"));

    let mut zero_base = progression_value();
    zero_base.rules_source_mut()["skills"]["base_learning_rate"] = serde_json::json!(0);
    assert!(content_error(&zero_base).contains("base_learning_rate must be positive"));

    let mut zero_gold = progression_value();
    zero_gold.rules_source_mut()["skills"]["training"]["gold_per_learning_rate"] =
        serde_json::json!(0);
    assert!(content_error(&zero_gold).contains("gold_per_learning_rate must be positive"));

    let mut zero_xp = progression_value();
    zero_xp.rules_source_mut()["skills"]["training"]["experience_per_learning_rate"] =
        serde_json::json!(0);
    assert!(content_error(&zero_xp).contains("experience_per_learning_rate must be positive"));

    let mut unordered_caps = progression_value();
    unordered_caps.rules_source_mut()["skills"]["training"]["maximum_learning_rates"][1] =
        serde_json::json!(2);
    assert!(content_error(&unordered_caps).contains("must be greater than the previous level"));

    let mut cap_below_base = progression_value();
    cap_below_base.rules_source_mut()["skills"]["base_learning_rate"] = serde_json::json!(3);
    assert!(
        content_error(&cap_below_base)
            .contains("maximum_learning_rates[0] must be at least base_learning_rate")
    );

    let mut missing_rate = progression_value();
    missing_rate.actors_mut()[0]["character"]["skill_ledger"] = serde_json::json!([{
        "track_id": "sword",
        "level": 1,
        "critique_rank": 0,
        "practice_points": 0
    }]);
    assert!(content_error(&missing_rate).contains("missing field `learning_rate`"));

    let mut below_base = progression_value();
    below_base.rules_source_mut()["skills"]["base_learning_rate"] = serde_json::json!(2);
    below_base.actors_mut()[0]["character"]["skill_ledger"] = serde_json::json!([{
        "track_id": "sword",
        "level": 1,
        "critique_rank": 0,
        "practice_points": 0,
        "learning_rate": 1
    }]);
    assert!(
        content_error(&below_base)
            .contains("learning_rate must be at least rules.skills.base_learning_rate")
    );
}

#[test]
fn obsolete_flat_rank_fields_and_old_catalog_version_are_rejected() {
    let mut obsolete = progression_value();
    obsolete.actors_mut()[0]["character"]["skill_ledger"] = serde_json::json!([{
        "track_id": "sword",
        "rank": 1,
        "skill_xp": 0
    }]);
    let error = content_error(&obsolete);
    assert!(error.contains("unknown field `rank`"), "{error}");

    let mut previous = progression_value();
    previous.catalog["schema_version"] = serde_json::json!(10);
    assert!(content_error(&previous).contains("catalog.schema_version must be 6"));
}

#[test]
fn catalog_rejects_invalid_skill_structure_and_knight_magic_eligibility() {
    let mut short_ladder = progression_value();
    short_ladder.skill_catalog_mut().expect("skill catalog")["ladders"][0]["titles"]
        .as_array_mut()
        .expect("title rows")
        .pop();
    assert!(content_error(&short_ladder).contains("exactly 20 ordered levels"));

    let mut blank_ladder = progression_value();
    blank_ladder.skill_catalog_mut().expect("skill catalog")["ladders"][0]["id"] =
        serde_json::json!(" ");
    assert!(content_error(&blank_ladder).contains("ladders[0].id must be non-empty"));

    let mut duplicate_ladder = progression_value();
    let duplicate =
        duplicate_ladder.skill_catalog_mut().expect("skill catalog")["ladders"][0].clone();
    duplicate_ladder.skill_catalog_mut().expect("skill catalog")["ladders"]
        .as_array_mut()
        .expect("ladder rows")
        .push(duplicate);
    assert!(content_error(&duplicate_ladder).contains("ladders[1].id must be unique"));

    let mut unordered_title = progression_value();
    unordered_title.skill_catalog_mut().expect("skill catalog")["ladders"][0]["titles"][3]["level"] =
        serde_json::json!(4);
    assert!(content_error(&unordered_title).contains("level must equal its ordered index 3"));

    let mut blank_title = progression_value();
    blank_title.skill_catalog_mut().expect("skill catalog")["ladders"][0]["titles"][3]["title"] =
        serde_json::json!(" ");
    assert!(content_error(&blank_title).contains("titles[3].title must be non-empty"));

    let mut duplicate_track = progression_value();
    let duplicate =
        duplicate_track.skill_catalog_mut().expect("skill catalog")["tracks"][0].clone();
    duplicate_track.skill_catalog_mut().expect("skill catalog")["tracks"]
        .as_array_mut()
        .expect("track rows")
        .push(duplicate);
    assert!(content_error(&duplicate_track).contains("tracks[1].id must be unique"));

    let mut blank_track = progression_value();
    blank_track.skill_catalog_mut().expect("skill catalog")["tracks"][0]["id"] =
        serde_json::json!(" ");
    assert!(content_error(&blank_track).contains("tracks[0].id must be non-empty"));

    let mut blank_display = progression_value();
    blank_display.skill_catalog_mut().expect("skill catalog")["tracks"][0]["display"] =
        serde_json::json!(" ");
    assert!(content_error(&blank_display).contains("tracks[0].display must be non-empty"));

    let mut unknown_ladder = progression_value();
    unknown_ladder.skill_catalog_mut().expect("skill catalog")["tracks"][0]["ladder_id"] =
        serde_json::json!("missing");
    assert!(content_error(&unknown_ladder).contains("ladder_id references unknown ladder"));

    let mut unknown_kind = progression_value();
    unknown_kind.skill_catalog_mut().expect("skill catalog")["tracks"][0]["kind"] =
        serde_json::json!("general");
    assert!(content_error(&unknown_kind).contains("unknown variant `general`"));

    let mut empty_magic_classes = progression_value();
    empty_magic_classes
        .skill_catalog_mut()
        .expect("skill catalog")["tracks"][0]["kind"] = serde_json::json!("magic");
    assert!(
        content_error(&empty_magic_classes)
            .contains("eligible_class_ids must be non-empty for a magic track")
    );

    let mut duplicate_magic_class = progression_value();
    duplicate_magic_class
        .skill_catalog_mut()
        .expect("skill catalog")["tracks"][0]["kind"] = serde_json::json!("magic");
    duplicate_magic_class
        .skill_catalog_mut()
        .expect("skill catalog")["tracks"][0]["eligible_class_ids"] =
        serde_json::json!(["wizard", "wizard"]);
    assert!(
        content_error(&duplicate_magic_class)
            .contains("eligible_class_ids must not contain duplicates")
    );

    let mut blank_magic_class = progression_value();
    blank_magic_class
        .skill_catalog_mut()
        .expect("skill catalog")["tracks"][0]["kind"] = serde_json::json!("magic");
    blank_magic_class
        .skill_catalog_mut()
        .expect("skill catalog")["tracks"][0]["eligible_class_ids"] = serde_json::json!([""]);
    assert!(content_error(&blank_magic_class).contains("eligible_class_ids[0] must be non-empty"));

    let mut knight_magic = progression_value();
    knight_magic.skill_catalog_mut().expect("skill catalog")["tracks"][0]["kind"] =
        serde_json::json!("magic");
    knight_magic.skill_catalog_mut().expect("skill catalog")["tracks"][0]["eligible_class_ids"] =
        serde_json::json!(["knight"]);
    assert!(content_error(&knight_magic).contains("must not grant magic skill to knight"));
}

#[test]
fn optional_catalog_titles_flow_through_events_and_snapshot_views() {
    assert_eq!(EVENT_CONTRACT_VERSION, 41);
    assert_eq!(SNAPSHOT_CONTRACT_VERSION, 31);
    assert_eq!(OBSERVED_SNAPSHOT_CONTRACT_VERSION, 30);
    assert_eq!(ACTION_CONTEXT_CONTRACT_VERSION, 32);
    assert_eq!(COMMAND_CONTRACT_VERSION, 26);

    let mut engine = progression_value()
        .engine(7)
        .expect("clean skill fixture starts");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East, Direction::East]),
        )
        .expect("player engages target");
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("attack resolves");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::SkillPracticeAwarded {
            track_id,
            track_display: Some(display),
            level: 0,
            critique_rank: 0,
            ..
        } if track_id == "sword" && display == "Sword"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::SkillPositionChanged {
            track_id,
            track_display: Some(display),
            new_level: 1,
            new_critique_rank: 1,
            level_title: Some(title),
            ..
        } if track_id == "sword" && display == "Sword" && title == "First Measure"
    )));

    let snapshot = engine.snapshot();
    let player = snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("player view");
    let entry = &player
        .character
        .as_ref()
        .expect("character view")
        .skill_ledger[0];
    assert_eq!(entry.track_id, "sword");
    assert_eq!(entry.level, 1);
    assert_eq!(entry.critique_rank, 1);
    assert_eq!(entry.practice_points, 0);
    assert_eq!(entry.learning_rate, 1);
    assert_eq!(entry.track_display.as_deref(), Some("Sword"));
    assert_eq!(entry.level_title.as_deref(), Some("First Measure"));
}

#[test]
fn absent_catalog_preserves_state_without_inventing_labels() {
    let snapshot = ContentParts::tracked(
        "area_path_terrain_spells",
        "profile/area_path_terrain_spells",
    )
    .engine(7)
    .unwrap()
    .snapshot();
    let player = snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .expect("player view");
    let entry = &player
        .character
        .as_ref()
        .expect("character view")
        .skill_ledger[0];
    assert_eq!(entry.track_id, "wizard_magic");
    assert_eq!(entry.track_display, None);
    assert_eq!(entry.level_title, None);
}

#[test]
fn knight_magic_rejects_spell_and_teaching_skill_gates() {
    let mut spell_gate =
        ContentParts::tracked("knight_support_actions", "profile/knight_support_actions");
    spell_gate.selected_mut("spells", 0)["skill_requirement"] = serde_json::json!(1);
    assert!(
        content_error(&spell_gate)
            .contains("spells[0].skill_requirement must be absent for knight_magic")
    );

    let mut teaching_gate =
        ContentParts::tracked("knight_support_actions", "profile/knight_support_actions");
    teaching_gate.push_selected(
        "service_definitions",
        "service/forbidden_knight_teacher/test",
        serde_json::json!({
            "id": "forbidden_knight_teacher",
            "name": "Forbidden Knight Teacher",
            "capabilities": [
                {
                    "id": "knight_training",
                    "kind": "skill_training",
                    "offers": [{
                        "track_id": "knight_magic",
                        "eligible_class_ids": ["knight"],
                        "minimum_category_level": 0,
                        "maximum_category_level": 19
                    }]
                },
                {
                    "id": "knight_teaching",
                    "kind": "spell_teaching",
                    "training_capability_id": "knight_training",
                    "teachings": [{"spell_id": "blessed_edge"}]
                }
            ]
        }),
    );
    teaching_gate
        .service_instances_mut()
        .as_array_mut()
        .expect("service instances")
        .push(serde_json::json!({
            "id": "forbidden_knight_teacher",
            "service_definition_id": "forbidden_knight_teacher",
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 1, "y": 1}
            }
        }));
    assert!(content_error(&teaching_gate).contains(
        "service_definitions[0].capabilities[1].teachings[0] must not teach knight_magic"
    ));
}
