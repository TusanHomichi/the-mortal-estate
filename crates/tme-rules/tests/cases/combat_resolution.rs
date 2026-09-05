use std::collections::{BTreeMap, BTreeSet};

use crate::support::content_parts::ContentParts;
use serde_json::{Value, json};
use tme_rules::model::SummonedActorState;
use tme_rules::{DeterministicRng, Engine, Event, ItemRelocationReason, LogicalTime, PlayerIntent};

fn first_room() -> ContentParts {
    ContentParts::tracked("first_room", "profile/first_room")
}

fn skill_progression() -> ContentParts {
    ContentParts::tracked("skill_progression", "profile/skill_progression")
}

fn actor_mut<'a>(value: &'a mut ContentParts, actor_id: &str) -> &'a mut Value {
    value
        .actors_mut()
        .as_array_mut()
        .expect("actors should be an array")
        .iter_mut()
        .find(|actor| actor["id"] == actor_id)
        .unwrap_or_else(|| panic!("actor {actor_id:?} should exist"))
}

fn content_error(value: &ContentParts) -> String {
    value
        .validated_seed()
        .expect_err("content should fail")
        .to_string()
}

fn engine_from_value(value: ContentParts, seed: u64) -> Engine {
    value.engine(seed).expect("engine should start")
}

fn same_hex_first_room() -> ContentParts {
    let mut value = first_room();
    let player_position = actor_mut(&mut value, "player")["location"]["position"].clone();
    actor_mut(&mut value, "mireling")["location"]["position"] = player_position;
    let defender = value.actor_definition_by_actor_id_mut("mireling");
    defender["ai"] = json!({
        "behavior": "hold_ground",
        "cadence_units": 1,
        "aggro_radius": 7,
        "leash_range": 12,
        "awareness": {"mode": "unrestricted"},
        "physical_attack_modes": ["fight"]
    });
    defender["stats"]["hp"] = json!(1_000);
    defender["stats"]["defense"] = json!(100);
    value
}

fn protected_first_room(block_field: &str, value_per_item: i32) -> ContentParts {
    let mut value = same_hex_first_room();
    let position = if block_field == "block_value" {
        "left_hand"
    } else {
        "outer_armor"
    };
    let valid_placements = if block_field == "block_value" {
        json!(["hand", "sack"])
    } else {
        json!(["hand", "sack", "outer_armor"])
    };
    let mut capability = json!({"taxonomy_id": "test_protection"});
    if block_field == "block_value" {
        capability
            .as_object_mut()
            .expect("capability should be an object")
            .insert(block_field.to_string(), json!(value_per_item));
    }
    let armor = (block_field == "armor").then(|| {
        json!({
            "block_rating": value_per_item,
            "encumbrance": 0,
            "damage_reduction": {"cutting": 0, "piercing": 0, "crushing": 0}
        })
    });
    value.push_selected(
        "items",
        "item/test_protection/combat_resolution",
        json!({
            "id": "test_protection",
            "kind": "armor",
            "name": "Test Protection",
            "category": "armor",
            "armor": armor,
            "capability": capability,
            "valid_placements": valid_placements,
            "economy": {"unit_burden": 1}
        }),
    );
    value
        .item_instances_mut()
        .as_object_mut()
        .expect("item_instances should be an object")
        .insert(
            "test_protection".to_string(),
            json!({
                "definition_id": "test_protection",
                "binding": {"state": "unrestricted"}
            }),
        );
    actor_mut(&mut value, "mireling")["carried"]["items"] = json!([{
        "item_instance_id": "test_protection",
        "position": position
    }]);
    value
}

fn post_fumble_rolls_by_seed() -> BTreeMap<u32, u64> {
    let mut result = BTreeMap::new();
    for seed in 0_u64..10_000 {
        let mut rng = DeterministicRng::new(seed);
        if rng.roll_percent() <= 5 {
            continue;
        }
        result.entry(rng.roll_d20()).or_insert(seed);
        if result.len() == 20 {
            break;
        }
    }
    assert_eq!(
        result.keys().copied().collect::<BTreeSet<_>>(),
        (1..=20).collect()
    );
    result
}

fn player_attack_events(value: ContentParts, seed: u64, target_id: &str) -> Vec<Event> {
    player_attack_events_with_mode(value, seed, target_id, tme_rules::PhysicalAttackMode::Fight)
}

fn player_attack_events_with_mode(
    value: ContentParts,
    seed: u64,
    target_id: &str,
    mode: tme_rules::PhysicalAttackMode,
) -> Vec<Event> {
    engine_from_value(value, seed)
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode,
                target_actor_id: target_id.into(),
            },
        )
        .expect("attack should resolve")
        .events
}

fn thrown_kill_value() -> ContentParts {
    let mut value = skill_progression();
    actor_mut(&mut value, "mireling")["location"]["position"] = json!({"x": 3, "y": 1});
    value.actor_definition_by_actor_id_mut("mireling")["stats"]["hp"] = json!(1);
    value.actor_definition_by_actor_id_mut("mireling")["xp_value"] = json!(5);
    value.selected_mut("items", 0)["weapon"]["default_attack_mode"] = json!("throw");
    value.selected_mut("items", 0)["weapon"]["attack_modes"] = json!([{
        "mode": "throw",
        "maximum_range": 3,
        "damage_kind": "piercing"
    }]);
    value
}

fn event_index(events: &[Event], predicate: impl Fn(&Event) -> bool) -> usize {
    events
        .iter()
        .position(predicate)
        .expect("expected event should exist")
}

#[test]
fn catalog_six_combat_contract_is_required_strict_and_projected() {
    let mut canonical = first_room();
    let engine = engine_from_value(canonical.clone(), 7);
    let snapshot = serde_json::to_value(engine.snapshot()).expect("snapshot should serialize");
    assert_eq!(snapshot["contract_version"], 31);
    assert_eq!(
        snapshot["rules"]["combat"],
        canonical.rules_source_mut()["combat"]
    );

    let mut previous = canonical.clone();
    previous.catalog["schema_version"] = json!(3);
    assert!(content_error(&previous).contains("catalog.schema_version must be 6"));

    let mut missing = canonical.clone();
    missing
        .rules_source_mut()
        .as_object_mut()
        .expect("rules should be an object")
        .remove("combat");
    assert!(content_error(&missing).contains("missing field `combat`"));

    let mut unknown = canonical.clone();
    unknown.rules_source_mut()["combat"]["legacy_formula"] = json!(true);
    assert!(content_error(&unknown).contains("unknown field `legacy_formula`"));

    let mut zero = canonical;
    zero.rules_source_mut()["combat"]["hit"]["attacker_attack_stat_divisor"] = json!(0);
    assert!(
        content_error(&zero)
            .contains("rules.combat.hit.attacker_attack_stat_divisor must be positive")
    );

    let mut bad_cap = first_room();
    bad_cap.rules_source_mut()["combat"]["block"]["shield_percent_cap"] = json!(9);
    assert!(
        content_error(&bad_cap)
            .contains("shield_percent_per_point must not exceed shield_percent_cap")
    );

    let mut bad_thresholds = first_room();
    bad_thresholds.rules_source_mut()["combat"]["damage"]["heavy_label_min_percent"] = json!(20);
    assert!(content_error(&bad_thresholds).contains("label thresholds must satisfy"));

    let mut bad_status = first_room();
    bad_status.rules_source_mut()["combat"]["tuning_status"] = json!("matched");
    assert!(content_error(&bad_status).contains("unknown variant `matched`"));
}

#[test]
fn canonical_shield_and_armor_rules_keep_complete_d20_distributions() {
    let seeds = post_fumble_rolls_by_seed();

    let shield_blocks = seeds
        .values()
        .filter(|seed| {
            player_attack_events(protected_first_room("block_value", 1), **seed, "mireling")
                .iter()
                .any(|event| {
                    matches!(
                        event,
                        Event::AttackBlocked {
                            attacker_id,
                            source,
                            ..
                        } if attacker_id == "player" && *source == tme_rules::model::BlockSourceKind::LeftShield
                    )
                })
        })
        .count();
    assert_eq!(shield_blocks, 1, "only d20 roll 1 must shield-block");

    let armor_blocks = seeds
        .values()
        .filter(|seed| {
            player_attack_events(protected_first_room("armor", 5), **seed, "mireling")
                .iter()
                .any(|event| {
                    matches!(
                        event,
                        Event::AttackBlocked {
                            attacker_id,
                            source,
                            ..
                        } if attacker_id == "player" && *source == tme_rules::model::BlockSourceKind::Armor
                    )
                })
        })
        .count();
    assert_eq!(armor_blocks, 7, "d20 rolls 1 through 7 must armor-block");
}

#[test]
fn hit_roll_is_reused_for_authored_damage_variation() {
    let seeds = post_fumble_rolls_by_seed();
    let mut damage_counts = BTreeMap::new();
    for (expected_roll, seed) in seeds {
        let mut value = same_hex_first_room();
        value.actor_definition_by_actor_id_mut("player")["stats"]["attack"] = json!(100);
        value.actor_definition_by_actor_id_mut("mireling")["stats"]["defense"] = json!(0);
        let events = player_attack_events(value, seed, "mireling");
        let (roll, damage) = events
            .iter()
            .find_map(|event| match event {
                Event::Attacked {
                    attacker_id,
                    roll,
                    damage,
                    ..
                } if attacker_id == "player" => Some((*roll, *damage)),
                _ => None,
            })
            .expect("player should land a hit");
        assert_eq!(roll, expected_roll);
        assert_eq!(damage, 101 + i32::try_from(expected_roll % 3).unwrap());
        *damage_counts.entry(damage).or_insert(0_usize) += 1;
    }
    assert_eq!(
        damage_counts,
        BTreeMap::from([(101, 6), (102, 7), (103, 7)])
    );
}

#[test]
fn authored_practice_amount_routes_once_through_the_skill_owner() {
    let mut value = skill_progression();
    let player_position = actor_mut(&mut value, "player")["location"]["position"].clone();
    actor_mut(&mut value, "mireling")["location"]["position"] = player_position;
    value.actor_definition_by_actor_id_mut("mireling")["stats"]["defense"] = json!(100);
    value.rules_source_mut()["combat"]["practice"]["practice_raw_points"] = json!(2);
    let events = player_attack_events(value, 7, "mireling");
    let practice = events
        .iter()
        .filter_map(|event| match event {
            Event::SkillPracticeAwarded {
                actor_id,
                track_id,
                raw_amount,
                learning_rate,
                credited_amount,
                ..
            } if actor_id == "player" => Some((
                track_id.as_str(),
                *raw_amount,
                *learning_rate,
                *credited_amount,
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(practice, vec![("sword", 2, 1, 2)]);
}

#[test]
fn thrown_kill_orders_defeat_xp_practice_then_relocation() {
    let events = player_attack_events_with_mode(
        thrown_kill_value(),
        1_010_580_540,
        "mireling",
        tme_rules::PhysicalAttackMode::Throw,
    );
    let attacked = event_index(
        &events,
        |event| matches!(event, Event::Attacked { attacker_id, .. } if attacker_id == "player"),
    );
    let defeated = event_index(
        &events,
        |event| matches!(event, Event::ActorDefeated { actor_id, .. } if actor_id == "mireling"),
    );
    let practice = event_index(
        &events,
        |event| matches!(event, Event::SkillPracticeAwarded { actor_id, .. } if actor_id == "player"),
    );
    let relocated = event_index(&events, |event| {
        matches!(
            event,
            Event::ItemRelocated {
                reason: ItemRelocationReason::Thrown,
                ..
            }
        )
    });
    let experience = event_index(
        &events,
        |event| matches!(event, Event::ExperienceAwarded { actor_id, amount: 5, .. } if actor_id == "player"),
    );
    assert!(attacked < defeated);
    assert!(defeated < experience);
    assert!(experience < practice);
    assert!(practice < relocated);
}

#[test]
fn ineligible_physical_outcomes_produce_no_kill_xp() {
    let mut nonlethal = thrown_kill_value();
    nonlethal.actor_definition_by_actor_id_mut("mireling")["stats"]["hp"] = json!(100);
    assert!(
        !player_attack_events_with_mode(
            nonlethal,
            1_010_580_540,
            "mireling",
            tme_rules::PhysicalAttackMode::Throw,
        )
        .iter()
        .any(|event| matches!(event, Event::ExperienceAwarded { .. }))
    );

    let mut zero_xp = thrown_kill_value();
    zero_xp.actor_definition_by_actor_id_mut("mireling")["xp_value"] = json!(0);
    assert!(
        !player_attack_events_with_mode(
            zero_xp,
            1_010_580_540,
            "mireling",
            tme_rules::PhysicalAttackMode::Throw,
        )
        .iter()
        .any(|event| matches!(event, Event::ExperienceAwarded { .. }))
    );

    let mut owned = engine_from_value(thrown_kill_value(), 1_010_580_540);
    let player_id = owned.world().actors[0].id.clone();
    owned.world_mut().actors[1].summoned = Some(SummonedActorState {
        instance_id: "owned_target".into(),
        owner_id: player_id,
        source_spell_id: "test_spell".to_string(),
        template_id: "test_template".to_string(),
        remaining_rounds: None,
        last_ticked_at: LogicalTime::ZERO,
    });
    let owned_before = owned.world().clone();
    let owned_error = owned
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "mireling".into(),
            },
        )
        .expect_err("owned target attack must be non-overridable");
    assert!(owned_error.message().contains("invalid_hostile_target"));
    assert_eq!(owned.world(), &owned_before);

    let mut automatic = same_hex_first_room();
    automatic.actor_definition_by_actor_id_mut("player")["stats"]["hp"] = json!(1);
    automatic.actor_definition_by_actor_id_mut("mireling")["stats"]["attack"] = json!(100);
    automatic.actor_definition_by_actor_id_mut("mireling")["ai"]["behavior"] =
        json!("simple_chase");
    let automatic_events = engine_from_value(automatic, 7)
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("automatic attack should resolve");
    assert!(automatic_events.events.iter().any(
        |event| matches!(event, Event::Attacked { attacker_id, .. } if attacker_id == "mireling")
    ));
    assert!(
        !automatic_events
            .events
            .iter()
            .any(|event| matches!(event, Event::ExperienceAwarded { .. }))
    );
}

#[test]
fn late_thrown_kill_xp_overflow_restores_world_item_timing_and_rng() {
    let value = thrown_kill_value();
    let mut engine = engine_from_value(value.clone(), 1_010_580_540);
    engine.world_mut().actors[0]
        .character
        .as_mut()
        .expect("player should be character-backed")
        .progression
        .experience = i64::MAX;
    let before_snapshot = engine.snapshot();
    let before_location = engine
        .item_location("training_sword")
        .expect("thrown weapon should have a location");

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "mireling".into(),
            },
        )
        .expect_err("XP addition should overflow after the thrown kill");
    assert!(error.to_string().contains("experience overflow"));
    assert_eq!(engine.snapshot(), before_snapshot);
    assert_eq!(
        engine
            .item_location("training_sword")
            .expect("rolled-back weapon should have a location"),
        before_location
    );

    engine.world_mut().actors[0]
        .character
        .as_mut()
        .expect("player should be character-backed")
        .progression
        .experience = 0;
    let retried = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("retry should succeed after clearing overflow state");
    let fresh = engine_from_value(value, 1_010_580_540)
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Throw,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("fresh attack should succeed");
    assert_eq!(retried, fresh, "retry must replay the restored RNG exactly");
}
