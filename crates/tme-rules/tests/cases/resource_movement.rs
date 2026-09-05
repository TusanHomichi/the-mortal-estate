use crate::support::content_parts::ContentParts;
use tme_rules::{
    ACTION_CONTEXT_CONTRACT_VERSION, BurdenTier, COMMAND_CONTRACT_VERSION, Coord, Direction,
    EVENT_CONTRACT_VERSION, Engine, Event, LogicalTime, MovementExertion, MovementPace,
    MovementStopReason, OBSERVED_SNAPSHOT_CONTRACT_VERSION, PATH_PREVIEW_CONTRACT_VERSION,
    PlayerCommandV1, PlayerIntent, PlayerIntentPayloadV1, ResourceActivity, ResourceKind,
    SNAPSHOT_CONTRACT_VERSION,
};

fn parts() -> ContentParts {
    ContentParts::tracked("resource_movement", "profile/resource_movement")
}

fn engine() -> Engine {
    let mut engine = parts().engine(7).expect("resource movement engine starts");
    // This fixture begins midway through an existing recovery interval.
    engine.world_mut().actors[0]
        .resource_activity
        .last_recovered_at = LogicalTime::ZERO;
    engine
}

fn set_player_resources(engine: &mut Engine, hp: i32, mp: i32, stamina: i32) {
    let player = &mut engine.world_mut().actors[0];
    player.hp = hp;
    player.mp = mp;
    player.stamina = stamina;
    let resources = &mut player.character.as_mut().expect("character").resources;
    resources.hp = hp;
    resources.mp = mp;
    resources.stamina = stamina;
}

fn set_player_gold(engine: &mut Engine, gold: i64) {
    engine.world_mut().actors[0].carried.gold.sack = gold;
}

fn movement_started(events: &[Event]) -> &Event {
    events
        .iter()
        .find(|event| matches!(event, Event::MovementStarted { .. }))
        .expect("movement-started event")
}

#[test]
fn pace_cardinality_mixed_paths_and_read_surfaces_share_one_pure_fact() {
    for (path, expected) in [
        (vec![Direction::South], MovementPace::Walk),
        (vec![Direction::South, Direction::East], MovementPace::Run),
        (
            vec![Direction::South, Direction::East, Direction::East],
            MovementPace::Sprint,
        ),
    ] {
        let engine = engine();
        let preview = engine
            .preview_actor_path(&tme_rules::ActorId::from("player"), &path)
            .expect("valid preview");
        assert_eq!(preview.pace, expected);
        assert_eq!(preview.requested_path, path);
    }

    let mut invalid = engine();
    let before = serde_json::to_value(invalid.snapshot()).expect("snapshot serializes");
    assert!(
        invalid
            .preview_actor_path(&tme_rules::ActorId::from("player"), &[])
            .is_err()
    );
    assert!(
        invalid
            .preview_actor_path(
                &tme_rules::ActorId::from("player"),
                &[
                    Direction::South,
                    Direction::East,
                    Direction::East,
                    Direction::West,
                ]
            )
            .is_err()
    );
    assert!(
        invalid
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::MovePath(Vec::new())
            )
            .is_err()
    );
    assert!(
        invalid
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::MovePath(vec![
                    Direction::South,
                    Direction::East,
                    Direction::East,
                    Direction::West,
                ]),
            )
            .is_err()
    );
    assert_eq!(
        serde_json::to_value(invalid.snapshot()).expect("snapshot serializes"),
        before,
        "invalid cardinalities must not mutate"
    );

    let mut read_engine = engine();
    let mut control_engine = engine();
    let read_before = serde_json::to_value(read_engine.snapshot()).expect("snapshot serializes");
    for _ in 0..3 {
        let _ = read_engine.snapshot();
        let _ = read_engine
            .actor_action_context(&tme_rules::ActorId::from("player"))
            .expect("action context");
        let _ = read_engine
            .actor_observed_frame(&tme_rules::ActorId::from("player"))
            .expect("observed frame");
        let _ = read_engine
            .preview_actor_path(
                &tme_rules::ActorId::from("player"),
                &[Direction::South, Direction::East],
            )
            .expect("preview");
    }
    assert_eq!(
        serde_json::to_value(read_engine.snapshot()).expect("snapshot serializes"),
        read_before
    );
    read_engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("move into engagement after reads");
    control_engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("control move into engagement");
    let after_reads = read_engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "sparring_post".into(),
            },
        )
        .expect("attack after reads");
    let control = control_engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "sparring_post".into(),
            },
        )
        .expect("control attack");
    assert_eq!(after_reads, control, "reads must not consume RNG");
    assert_eq!(read_engine.snapshot(), control_engine.snapshot());
}

#[test]
fn burden_uses_checked_raw_item_coin_totals_and_inclusive_strength_tiers() {
    for (gold, total, tier) in [
        (8, 8, BurdenTier::LightlyLoaded),
        (9, 9, BurdenTier::ModeratelyLoaded),
        (17, 17, BurdenTier::HeavilyLoaded),
        (25, 25, BurdenTier::VeryHeavilyLoaded),
    ] {
        let mut engine = engine();
        set_player_gold(&mut engine, gold);
        let burden = engine
            .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::South])
            .expect("preview")
            .burden;
        assert_eq!(burden.item_burden, 0);
        assert_eq!(burden.coin_burden, total);
        assert_eq!(burden.total_burden, total);
        assert_eq!(burden.lightly_loaded_limit, Some(8));
        assert_eq!(burden.moderately_loaded_limit, Some(16));
        assert_eq!(burden.heavily_loaded_limit, Some(24));
        assert_eq!(burden.tier, Some(tier));
    }

    let mut loaded = engine();
    for (position, instance_id) in [
        (tme_rules::CarriedPosition::SackItem1, "reed_bundle"),
        (tme_rules::CarriedPosition::SackItem2, "clay_weight"),
        (tme_rules::CarriedPosition::SackItem3, "rope_coil"),
    ] {
        loaded.world_mut().actors[0]
            .carried
            .items
            .insert(position, instance_id.to_string());
    }
    let burden = loaded
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::South])
        .expect("loaded preview")
        .burden;
    assert_eq!(
        (burden.item_burden, burden.coin_burden, burden.total_burden),
        (23, 2, 25)
    );
    assert_eq!(burden.tier, Some(BurdenTier::VeryHeavilyLoaded));

    loaded.world_mut().actors[1].carried.gold.sack = 3;
    let monster = loaded
        .snapshot()
        .actors
        .into_iter()
        .find(|actor| actor.id == "sparring_post")
        .expect("monster view");
    assert_eq!(
        (monster.burden.item_burden, monster.burden.coin_burden),
        (0, 3)
    );
    assert_eq!(monster.burden.total_burden, 3);
    assert_eq!(monster.burden.tier, None);
    assert_eq!(monster.burden.lightly_loaded_limit, None);

    let mut overflow_parts = parts();
    overflow_parts.rules_source_mut()["burden"]["lightly_loaded_max_per_strength"] =
        serde_json::json!(u64::MAX / 2);
    overflow_parts.rules_source_mut()["burden"]["moderately_loaded_max_per_strength"] =
        serde_json::json!(u64::MAX / 2 + 1);
    overflow_parts.rules_source_mut()["burden"]["heavily_loaded_max_per_strength"] =
        serde_json::json!(u64::MAX / 2 + 2);
    let overflow = overflow_parts
        .validated_seed()
        .expect_err("overflowing authored burden limits fail before runtime setup");
    assert!(overflow.to_string().contains("must not overflow"));
}

#[test]
fn exertion_table_selects_one_normal_or_rapid_action_charge() {
    let mut light = engine();
    set_player_resources(&mut light, 7, 2, 8);
    let walk = light
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::South])
        .expect("light walk");
    assert_eq!(
        (walk.movement_exertion, walk.stamina_cost),
        (MovementExertion::None, Some(0))
    );
    let run = light
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::South, Direction::East],
        )
        .expect("light run");
    assert_eq!(
        (run.movement_exertion, run.stamina_cost),
        (MovementExertion::None, Some(0))
    );

    let mut moderate = engine();
    set_player_resources(&mut moderate, 7, 2, 8);
    set_player_gold(&mut moderate, 9);
    let run = moderate
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::South, Direction::East],
        )
        .expect("moderate run");
    assert_eq!(
        (run.movement_exertion, run.stamina_cost),
        (MovementExertion::Normal, Some(1))
    );

    let mut very_heavy = engine();
    set_player_resources(&mut very_heavy, 7, 2, 8);
    set_player_gold(&mut very_heavy, 25);
    let walk = very_heavy
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::South])
        .expect("very-heavy walk");
    assert_eq!(
        (walk.movement_exertion, walk.stamina_cost),
        (MovementExertion::Normal, Some(1))
    );

    let mut difficult = engine();
    set_player_resources(&mut difficult, 7, 2, 8);
    let run = difficult
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::East, Direction::East],
        )
        .expect("difficult run");
    assert_eq!(
        (run.movement_exertion, run.stamina_cost),
        (MovementExertion::Rapid, Some(2))
    );

    let mut half_hp = engine();
    set_player_resources(&mut half_hp, 6, 2, 8);
    let run = half_hp
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::South, Direction::East],
        )
        .expect("half-hp run");
    assert_eq!(
        (run.movement_exertion, run.stamina_cost),
        (MovementExertion::Rapid, Some(2))
    );
    set_player_gold(&mut half_hp, 9);
    let overridden = half_hp
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::South, Direction::East],
        )
        .expect("rapid overrides normal");
    assert_eq!(
        (overridden.movement_exertion, overridden.stamina_cost),
        (MovementExertion::Rapid, Some(2))
    );

    let blocked = half_hp
        .preview_actor_path(&tme_rules::ActorId::from("player"), &[Direction::North])
        .expect("blocked preview remains a plan");
    assert_eq!(blocked.accepted_steps, 0);
    assert_eq!(blocked.stop_reason, MovementStopReason::Blocked);
    assert_eq!(
        (blocked.movement_exertion, blocked.stamina_cost),
        (MovementExertion::None, Some(0))
    );

    set_player_resources(&mut half_hp, 6, 2, 1);
    let preview = half_hp
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::South, Direction::East],
        )
        .expect("insufficient positive stamina preview");
    assert_eq!(preview.accepted_steps, 2);
    assert_eq!(
        (
            preview.stamina_before,
            preview.stamina_cost,
            preview.stamina_after
        ),
        (Some(1), Some(2), Some(0))
    );
    let events = half_hp
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::South, Direction::East]),
        )
        .expect("accepted path commits before saturation");
    let spent = events
        .events
        .iter()
        .filter_map(|event| match event {
            Event::MovementStaminaSpent {
                amount, stamina, ..
            } => Some((*amount, *stamina)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(spent, vec![(1, 0)]);
}

#[test]
fn zero_stamina_caps_after_one_legal_step_and_preview_matches_commit() {
    let mut zero_engine = engine();
    set_player_resources(&mut zero_engine, 6, 2, 0);
    let path = vec![Direction::South, Direction::East, Direction::East];
    let preview = zero_engine
        .preview_actor_path(&tme_rules::ActorId::from("player"), &path)
        .expect("zero-stamina preview");
    assert_eq!(preview.pace, MovementPace::Sprint);
    assert_eq!(preview.accepted_steps, 1);
    assert_eq!(preview.stop_reason, MovementStopReason::ZeroStaminaLimit);
    assert_eq!(preview.final_position.position, Coord { x: 1, y: 2 });
    assert_eq!(
        (preview.stamina_before, preview.stamina_after),
        (Some(0), Some(0))
    );

    let events = zero_engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(path),
        )
        .expect("one legal step commits");
    assert_eq!(
        zero_engine.world().actors[0].location.position,
        preview.final_position.position
    );
    assert!(
        !events
            .events
            .iter()
            .any(|event| matches!(event, Event::MovementStaminaSpent { .. }))
    );
    match movement_started(&events.events) {
        Event::MovementStarted {
            pace,
            accepted_steps,
            burden_tier,
            exertion,
            stamina_cost,
            stop_reason,
            ..
        } => {
            assert_eq!(*pace, preview.pace);
            assert_eq!(*accepted_steps, preview.accepted_steps);
            assert_eq!(*burden_tier, preview.burden.tier);
            assert_eq!(*exertion, preview.movement_exertion);
            assert_eq!(*stamina_cost, preview.stamina_cost);
            assert_eq!(*stop_reason, preview.stop_reason);
        }
        _ => unreachable!(),
    }

    let mut blocked = engine();
    set_player_resources(&mut blocked, 6, 2, 0);
    let preview = blocked
        .preview_actor_path(
            &tme_rules::ActorId::from("player"),
            &[Direction::North, Direction::East],
        )
        .expect("ordinary first-step block");
    assert_eq!(preview.accepted_steps, 0);
    assert_eq!(preview.stop_reason, MovementStopReason::Blocked);
}

#[test]
fn activity_classification_wait_attack_and_free_reads_are_direct() {
    let mut inactive = engine();
    let before = inactive.snapshot();
    let inspect = inactive
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Inspect)
        .expect("inspect is free");
    assert!(
        inspect
            .events
            .iter()
            .any(|event| matches!(event, Event::Inspected { .. }))
    );
    assert_eq!(inactive.snapshot(), before);
    inactive
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::ShowSack)
        .expect("show sack is free");
    assert_eq!(inactive.snapshot(), before);

    let wait_events = inactive
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("wait advances without direct mutation");
    assert_eq!(
        inactive.world().actors[0].resource_activity.last_active_at,
        None
    );
    let recovered = wait_events
        .events
        .iter()
        .filter_map(|event| match event {
            Event::ResourceRegenerated {
                resource,
                activity,
                amount,
                ..
            } => Some((*resource, *activity, *amount)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        recovered,
        vec![
            (ResourceKind::Hp, ResourceActivity::Inactive, 2),
            (ResourceKind::Mp, ResourceActivity::Inactive, 1),
        ]
    );

    let mut active = engine();
    let movement = active
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::South]),
        )
        .expect("movement commits");
    assert_eq!(
        active.world().actors[0].resource_activity.last_active_at,
        Some(LogicalTime::new(1))
    );
    let recovered = movement
        .events
        .iter()
        .filter_map(|event| match event {
            Event::ResourceRegenerated {
                resource,
                activity,
                amount,
                ..
            } => Some((*resource, *activity, *amount)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        recovered,
        vec![
            (ResourceKind::Hp, ResourceActivity::Active, 1),
            (ResourceKind::Mp, ResourceActivity::Active, 1),
        ]
    );

    let mut attack = engine();
    attack
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("move into engagement");
    let stamina_before = attack.world().actors[0].stamina;
    attack
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "sparring_post".into(),
            },
        )
        .expect("ordinary attack adjudicates without stamina");
    assert_eq!(attack.world().actors[0].stamina, stamina_before);
    assert_eq!(
        attack.world().actors[0].resource_activity.last_active_at,
        Some(LogicalTime::new(2))
    );

    let mut zero_commit = engine();
    zero_commit
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![Direction::North]),
        )
        .expect("blocked movement is a resolved inactive action");
    assert_eq!(
        zero_commit.world().actors[0]
            .resource_activity
            .last_active_at,
        None
    );
}

#[test]
fn recovery_is_hp_then_mp_then_inactive_full_hp_stamina_with_caps_and_mirrors() {
    let mut recovery_engine = engine();
    set_player_resources(&mut recovery_engine, 11, 7, 10);
    let events = recovery_engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("inactive recovery boundary");
    let recovered = events
        .events
        .iter()
        .filter_map(|event| match event {
            Event::ResourceRegenerated {
                resource,
                activity,
                boundary_at,
                amount,
                current,
                maximum,
                ..
            } => Some((
                *resource,
                *activity,
                *boundary_at,
                *amount,
                *current,
                *maximum,
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        recovered,
        vec![
            (
                ResourceKind::Hp,
                ResourceActivity::Inactive,
                LogicalTime::new(2),
                1,
                12,
                12
            ),
            (
                ResourceKind::Mp,
                ResourceActivity::Inactive,
                LogicalTime::new(2),
                1,
                8,
                8
            ),
            (
                ResourceKind::Stamina,
                ResourceActivity::Inactive,
                LogicalTime::new(2),
                1,
                11,
                12
            ),
        ]
    );
    let player = &recovery_engine.world().actors[0];
    let resources = &player.character.as_ref().expect("character").resources;
    assert_eq!(
        (player.hp, player.mp, player.stamina),
        (resources.hp, resources.mp, resources.stamina)
    );

    let mut capped = engine();
    set_player_resources(&mut capped, 12, 8, 12);
    let events = capped
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("full resources wait");
    assert!(
        !events
            .events
            .iter()
            .any(|event| matches!(event, Event::ResourceRegenerated { .. }))
    );
}

#[test]
fn dn_component_versions_and_direct_shapes_are_exact() {
    assert_eq!(EVENT_CONTRACT_VERSION, 41);
    assert_eq!(SNAPSHOT_CONTRACT_VERSION, 31);
    assert_eq!(OBSERVED_SNAPSHOT_CONTRACT_VERSION, 30);
    assert_eq!(ACTION_CONTEXT_CONTRACT_VERSION, 32);
    assert_eq!(COMMAND_CONTRACT_VERSION, 26);
    assert_eq!(PATH_PREVIEW_CONTRACT_VERSION, 8);

    let engine = engine();
    let snapshot = serde_json::to_value(engine.snapshot()).expect("snapshot serializes");
    assert_eq!(snapshot["contract_version"], 31);
    assert!(snapshot["rules"]["movement"].is_object());
    assert!(snapshot["rules"]["burden"].is_object());
    assert!(snapshot["rules"]["resources"].is_object());
    assert_eq!(
        snapshot["rules"]["combat"]["tuning_status"],
        "original_provisional"
    );
    assert!(snapshot["actors"][0]["last_resource_activity_at"].is_null());
    assert!(snapshot["actors"][0]["burden"]["total_burden"].is_number());

    let observed = serde_json::to_value(
        engine
            .actor_observed_frame(&tme_rules::ActorId::from("player"))
            .expect("observed frame"),
    )
    .expect("observed frame serializes");
    assert_eq!(observed["contract_version"], 30);
    assert_eq!(observed["observed_snapshot"]["contract_version"], 30);
    assert_eq!(observed["action_context"]["contract_version"], 32);

    let preview = serde_json::to_value(
        engine
            .preview_actor_path(
                &tme_rules::ActorId::from("player"),
                &[Direction::South, Direction::East],
            )
            .expect("preview"),
    )
    .expect("preview serializes");
    assert_eq!(preview["contract_version"], 8);
    for key in [
        "pace",
        "accepted_steps",
        "stop_reason",
        "burden",
        "movement_exertion",
        "stamina_before",
        "stamina_cost",
        "stamina_after",
    ] {
        assert!(preview.get(key).is_some(), "missing Path-8 field {key}");
    }

    let command = PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::MovePath {
            path: vec![Direction::South, Direction::East],
        },
    };
    let command = serde_json::to_value(command).expect("command serializes");
    assert_eq!(command["contract_version"], 26);
    assert_eq!(
        command["intent"]["move_path"]["path"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert!(command["intent"].get("move").is_none());
}
