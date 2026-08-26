use crate::support::content_parts::ContentParts;
use tme_rules::{
    ActorId, ActorKind, ActorLifeState, CharacterId, Coord, DamageLabel, Direction, Event,
    ItemMoveDestination, LogicalTime, MAX_OBSERVED_EVENTS, MAX_OBSERVER_ACTORS, NavigationKind,
    OBSERVER_PROJECTION_CONTRACT_VERSION, ObservedEventV1, ObserverFeedbackCueV1,
    ObserverInspectExitStatusV1, ObserverPhysicalOutcomeV1, PhysicalAttackMode, PhysicalDamageKind,
    PlayerIntent, TransactionSourceV1, WorldPosition, WoundState,
};

fn engine() -> tme_rules::Engine {
    let mut character_parts = ContentParts::tracked("character_sheet", "profile/character_sheet");
    let character = character_parts.actors_mut()[0]["character"].clone();
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.actor_definition_by_actor_id_mut("player")["social"]["alignment_source"] =
        serde_json::json!({"kind": "character"});
    parts.actors_mut()[0]["character_id"] = serde_json::json!("character:observer:player");
    parts.actors_mut()[0]["character"] = character;
    parts.engine(7).expect("first-room engine")
}

fn two_player_engine() -> tme_rules::Engine {
    let mut character_parts = ContentParts::tracked("character_sheet", "profile/character_sheet");
    let character = character_parts.actors_mut()[0]["character"].clone();
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.actor_definition_by_actor_id_mut("player")["social"]["alignment_source"] =
        serde_json::json!({"kind": "character"});
    parts.actors_mut()[0]["character_id"] = serde_json::json!("character:observer:player");
    parts.actors_mut()[0]["character"] = character;
    let mut retained_monster = parts.actors_mut()[1].clone();
    retained_monster["id"] = serde_json::json!("observer_monster");
    let mut second = parts.actors_mut()[0].clone();
    second["id"] = serde_json::json!("mireling");
    second["character_id"] = serde_json::json!("character:observer:mireling");
    second["location"]["position"] = serde_json::json!({"x": 3, "y": 1});
    second["carried"]["items"] = serde_json::json!([]);
    parts.actors_mut()[1] = second;
    parts
        .actors_mut()
        .as_array_mut()
        .expect("seed actors")
        .push(retained_monster);
    parts.engine(7).expect("two-player observer engine")
}

fn door_engine() -> tme_rules::Engine {
    let mut character_parts = ContentParts::tracked("character_sheet", "profile/character_sheet");
    let character = character_parts.actors_mut()[0]["character"].clone();
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.actor_definition_by_actor_id_mut("player")["social"]["alignment_source"] =
        serde_json::json!({"kind": "character"});
    parts.actors_mut()[0]["character_id"] = serde_json::json!("character:observer:player");
    parts.actors_mut()[0]["character"] = character;
    parts.world_template["topology"] = serde_json::json!({
        "edge/room_0/1/2": {
            "at": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 2, "y": 1}
            },
            "target": {
                "kind": "position",
                "location": {
                    "realm": "realm_0",
                    "level": "room_0",
                    "position": {"x": 3, "y": 1}
                }
            },
            "kind": {"kind": "door", "initial_state": "closed"},
            "hidden": false
        }
    });
    parts.engine(7).expect("door observer engine")
}

fn moved(actor_id: &str, from: Coord, to: Coord) -> Event {
    Event::Moved {
        actor_id: ActorId::from(actor_id),
        actor: actor_id.to_string(),
        from: WorldPosition::new("realm_0", "room_0", from),
        to: WorldPosition::new("realm_0", "room_0", to),
        navigation: NavigationKind::Walk,
    }
}

#[test]
fn projection_is_bounded_sorted_strict_and_contains_only_safe_fields() {
    let engine = engine();
    let projection = engine
        .observer_projection(&ActorId::from("player"), &[])
        .expect("projection");

    assert_eq!(
        projection.contract_version,
        OBSERVER_PROJECTION_CONTRACT_VERSION
    );
    assert_eq!(
        projection.frame.contract_version,
        OBSERVER_PROJECTION_CONTRACT_VERSION
    );
    assert_eq!(projection.frame.observer_actor_id, "player");
    assert_eq!(projection.frame.observation_radius, 7);
    assert!(projection.frame.tiles.len() <= 49);
    assert!(projection.frame.actors.len() <= MAX_OBSERVER_ACTORS);
    assert!(
        projection
            .frame
            .actors
            .windows(2)
            .all(|pair| pair[0].actor_id < pair[1].actor_id)
    );

    let value = serde_json::to_value(&projection).expect("serialize");
    let actor = value["frame"]["actors"][0]
        .as_object()
        .expect("safe actor object");
    assert_eq!(
        actor.keys().cloned().collect::<Vec<_>>(),
        [
            "actor_id",
            "attack_safety",
            "character_id",
            "hp",
            "kind",
            "life_state",
            "max_hp",
            "name",
            "position"
        ]
    );
    assert_eq!(
        projection
            .frame
            .actors
            .iter()
            .find(|actor| actor.actor_id == "player")
            .and_then(|actor| actor.character_id.as_ref())
            .map(|character_id| character_id.as_str()),
        Some("character:observer:player")
    );
    assert_eq!(
        projection.frame.character.identity.current_class_id,
        "fighter"
    );
    assert_eq!(projection.frame.carried.items.len(), 1);
    assert_eq!(projection.frame.burden.total_burden, 1);
    let wire = value.to_string();
    for forbidden in [
        "storage",
        "rng",
        "trace",
        "social_relations",
        "rules",
        "active_effects",
    ] {
        assert!(!wire.contains(forbidden), "projection leaked {forbidden}");
    }
    serde_json::from_value::<tme_rules::ObserverProjectionV1>(value)
        .expect("strict projection round trip");
}

#[test]
fn frame_reports_rules_owned_readiness_without_deriving_admission() {
    let mut engine = engine();
    engine.world_mut().actors[0].timing.ready_at = LogicalTime::new(4);
    let projection = engine
        .observer_projection(&ActorId::from("player"), &[])
        .expect("projection");

    assert_eq!(projection.frame.logical_time, engine.world().timing.now);
    assert_eq!(projection.frame.ready_at, LogicalTime::new(4));
    assert!(!projection.frame.can_act);
}

#[test]
fn inspect_events_are_private_typed_and_observer_safe() {
    let mut engine = two_player_engine();
    engine.world_mut().actors[1].kind = ActorKind::Player;
    engine.world_mut().actors[1].location.position = (2, 1).into();
    engine
        .apply_actor_intent(
            &ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: "training_knife".to_string(),
                destination: ItemMoveDestination::GroundHere,
            },
        )
        .expect("ground one exact item before inspect");
    let before = engine.snapshot();
    let events = engine
        .apply_actor_intent(&ActorId::from("player"), PlayerIntent::Inspect)
        .expect("inspect");
    assert_eq!(engine.snapshot(), before, "inspect remains a free read");

    let player = engine
        .observer_projection(&ActorId::from("player"), events.as_ref())
        .expect("player projection");
    let mireling = engine
        .observer_projection(&ActorId::from("mireling"), events.as_ref())
        .expect("other projection");

    let inspected = player
        .events
        .iter()
        .find(|event| matches!(event, ObservedEventV1::Inspected { .. }))
        .expect("inspecting actor receives the safe result");
    assert!(
        !mireling
            .events
            .iter()
            .any(|event| matches!(event, ObservedEventV1::Inspected { .. }))
    );

    let ObservedEventV1::Inspected {
        exits,
        nearby_actors,
        ground_items,
        ..
    } = inspected
    else {
        unreachable!("selected event is inspected")
    };
    assert_eq!(
        exits.iter().map(|exit| exit.direction).collect::<Vec<_>>(),
        [
            Direction::North,
            Direction::Northeast,
            Direction::East,
            Direction::Southeast,
            Direction::South,
            Direction::Southwest,
            Direction::West,
            Direction::Northwest,
        ],
        "inspect exits retain deterministic direction order"
    );
    assert!(
        nearby_actors
            .iter()
            .any(|actor| actor.actor_id == "mireling")
    );
    assert_eq!(ground_items.len(), 1);
    assert_eq!(ground_items[0].item.item_instance_id, "training_knife");
    assert_eq!(ground_items[0].direction, None);

    let wire = serde_json::to_string(inspected).expect("serialize inspect event");
    for forbidden in [
        "character_identity",
        "loot_claim",
        "active_effects",
        "storage",
        "service",
        "rules",
    ] {
        assert!(
            !wire.contains(forbidden),
            "inspect event leaked {forbidden}"
        );
    }
}

#[test]
fn inspect_door_status_is_finite_and_typed() {
    let mut engine = door_engine();
    let events = engine
        .apply_actor_intent(&ActorId::from("player"), PlayerIntent::Inspect)
        .expect("inspect");
    let projection = engine
        .observer_projection(&ActorId::from("player"), events.as_ref())
        .expect("projection");
    let exits = projection
        .events
        .iter()
        .find_map(|event| match event {
            ObservedEventV1::Inspected { exits, .. } => Some(exits),
            _ => None,
        })
        .expect("safe inspect result");
    assert!(exits.iter().any(|exit| matches!(
        exit.status,
        ObserverInspectExitStatusV1::Door { open: false, .. }
    )));
}

#[test]
fn hidden_tile_facts_are_absent_instead_of_serialized() {
    let mut engine = engine();
    engine.world_mut().actors[0]
        .active_effects
        .push(tme_rules::model::ActiveEffectState {
            spell_damage_credit: None,
            source_actor_id: None,
            hostile_authority: None,
            instance_id: "blind:test".to_string(),
            effect_id: "blind".to_string(),
            source: tme_rules::model::ActiveEffectSource {
                kind: "test".to_string(),
                id: "blind".to_string(),
            },
            kind: "control_status".to_string(),
            tags: vec!["blind".to_string()],
            potency: 0,
            remaining_rounds: Some(1),
            until_condition: None,
            stacking: tme_rules::model::ActiveEffectStackingPolicy::RefreshDuration,
            start_delay_rounds: 0,
            tick_interval_rounds: 1,
            suppresses_action: false,
            resistance_boosts: vec![],
            last_ticked_at: tme_rules::LogicalTime::ZERO,
        });
    let value = serde_json::to_value(
        engine
            .observer_projection(&ActorId::from("player"), &[])
            .expect("blind projection"),
    )
    .expect("serialize");
    let hidden = value["frame"]["tiles"]
        .as_array()
        .expect("tiles")
        .iter()
        .find(|tile| tile["position"] != serde_json::json!({"x": 1, "y": 1}))
        .expect("hidden tile")
        .as_object()
        .expect("tile object");
    assert_eq!(hidden.keys().cloned().collect::<Vec<_>>(), ["position"]);
}

#[test]
fn projection_rejects_unknown_and_non_player_observers_but_keeps_dead_players_observing() {
    let engine = engine();
    assert!(
        engine
            .observer_projection(&ActorId::from("missing"), &[])
            .is_err()
    );
    assert!(
        engine
            .observer_projection(&ActorId::from("mireling"), &[])
            .is_err()
    );

    let mut dead = engine;
    dead.world_mut().actors[0].life_state = ActorLifeState::Dead;
    let projection = dead
        .observer_projection(&ActorId::from("player"), &[])
        .expect("dead player retains an observer-safe frame");
    assert!(!projection.frame.can_act);
    assert!(
        projection
            .frame
            .action_options
            .iter()
            .all(|option| !option.enabled)
    );
}

#[test]
fn each_observer_gets_a_distinct_frame_and_only_visible_movement() {
    let mut engine = two_player_engine();
    engine.world_mut().actors[1].kind = ActorKind::Player;
    let player_event = moved("player", Coord { x: 1, y: 1 }, Coord { x: 2, y: 1 });
    let mireling_event = moved("mireling", Coord { x: 3, y: 1 }, Coord { x: 2, y: 1 });
    let hidden_event = Event::Moved {
        actor_id: ActorId::from("hidden"),
        actor: "Hidden".to_string(),
        from: WorldPosition::new("realm_0", "elsewhere", Coord { x: 1, y: 1 }),
        to: WorldPosition::new("realm_0", "elsewhere", Coord { x: 2, y: 1 }),
        navigation: NavigationKind::Walk,
    };

    let player = engine
        .observer_projection(
            &ActorId::from("player"),
            &[
                player_event.clone(),
                mireling_event.clone(),
                hidden_event.clone(),
            ],
        )
        .expect("player projection");
    let mireling = engine
        .observer_projection(
            &ActorId::from("mireling"),
            &[player_event, mireling_event, hidden_event],
        )
        .expect("mireling projection");

    assert_ne!(
        player.frame.observation_center,
        mireling.frame.observation_center
    );
    assert_eq!(player.events.len(), 2);
    assert_eq!(mireling.events.len(), 2);
    assert!(player.events.iter().any(|event| matches!(
        event,
        ObservedEventV1::ActorMoved { actor_id, .. } if actor_id == "player"
    )));
    assert!(mireling.events.iter().any(|event| matches!(
        event,
        ObservedEventV1::ActorMoved { actor_id, .. } if actor_id == "mireling"
    )));
}

#[test]
fn movement_event_cap_preserves_order_and_reports_truncation() {
    let engine = engine();
    let events = (0..(MAX_OBSERVED_EVENTS + 2))
        .map(|_| moved("player", Coord { x: 1, y: 1 }, Coord { x: 2, y: 1 }))
        .collect::<Vec<_>>();
    let projection = engine
        .observer_projection(&ActorId::from("player"), &events)
        .expect("projection");
    assert_eq!(projection.events.len(), MAX_OBSERVED_EVENTS);
    assert!(projection.events_truncated);
}

#[test]
fn feedback_routes_private_facts_only_to_the_controlled_character() {
    let mut engine = two_player_engine();
    engine.world_mut().actors[1].kind = ActorKind::Player;
    let player_character: CharacterId =
        serde_json::from_str(r#""character:observer:player""#).expect("player character ID");
    let events = vec![
        Event::PhysicalStaminaSpent {
            actor_id: ActorId::from("player"),
            actor: "Wayfarer".to_string(),
            mode: PhysicalAttackMode::Fight,
            amount: 2,
            stamina: 8,
            max_stamina: 10,
        },
        Event::TransactionCommitted {
            actor_id: ActorId::from("player"),
            actor: "Wayfarer".to_string(),
            source: TransactionSourceV1::NpcInteraction {
                npc_actor_id: ActorId::from("observer_monster"),
                interaction_id: "speak".to_string(),
            },
            costs: vec![],
            rewards: vec![],
        },
        Event::NpcSpoke {
            npc_actor_id: ActorId::from("observer_monster"),
            npc: "Observer Monster".to_string(),
            recipient_character_id: player_character,
            interaction_id: "speak".to_string(),
            response: "Private committed response".to_string(),
        },
    ];

    let player = engine
        .observer_projection(&ActorId::from("player"), &events)
        .expect("player feedback");
    let other = engine
        .observer_projection(&ActorId::from("mireling"), &events)
        .expect("other feedback");
    let player_cues = player
        .events
        .iter()
        .filter_map(|event| match event {
            ObservedEventV1::Feedback { cue } => Some(cue),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(player_cues.len(), 3);
    assert!(matches!(
        player_cues[0],
        ObserverFeedbackCueV1::Resource { .. }
    ));
    assert!(matches!(
        player_cues[1],
        ObserverFeedbackCueV1::Transaction { .. }
    ));
    assert!(matches!(
        player_cues[2],
        ObserverFeedbackCueV1::NpcMessage { .. }
    ));
    assert!(
        !other
            .events
            .iter()
            .any(|event| matches!(event, ObservedEventV1::Feedback { .. }))
    );

    let encoded = serde_json::to_string(&player.events).expect("serialize private feedback");
    for forbidden in [
        "character:observer:player",
        "credential",
        "session",
        "account",
    ] {
        assert!(!encoded.contains(forbidden), "feedback leaked {forbidden}");
    }
}

#[test]
fn visible_combat_feedback_nulls_an_unseen_source_and_drops_private_math() {
    let engine = engine();
    let event = Event::Attacked {
        attacker_id: ActorId::from("hidden-attacker"),
        attacker: "Hidden Attacker".to_string(),
        defender_id: ActorId::from("player"),
        defender: "Wayfarer".to_string(),
        defender_location: WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 }),
        mode: PhysicalAttackMode::Fight,
        damage_kind: PhysicalDamageKind::Cutting,
        effective_combat_add_rating: 999,
        roll: 19,
        damage: 3,
        armor_reduction: 1,
        label: DamageLabel::Light,
        wound_before: WoundState::Unhurt,
        wound_after: WoundState::Wounded,
        defender_hp: 7,
    };
    let projection = engine
        .observer_projection(&ActorId::from("player"), &[event])
        .expect("combat feedback");
    let cue = projection
        .events
        .iter()
        .find_map(|event| match event {
            ObservedEventV1::Feedback { cue } => Some(cue),
            _ => None,
        })
        .expect("combat cue");
    let ObserverFeedbackCueV1::PhysicalCombat {
        source,
        outcome:
            ObserverPhysicalOutcomeV1::Hit {
                damage,
                armor_reduction,
                target_hp,
                ..
            },
        ..
    } = cue
    else {
        panic!("expected hit feedback")
    };
    assert_eq!(source, &None);
    assert_eq!((*damage, *armor_reduction, *target_hp), (3, 1, 7));
    let encoded = serde_json::to_string(cue).expect("serialize combat feedback");
    for forbidden in ["roll", "effective_combat_add_rating", "damage_kind"] {
        assert!(!encoded.contains(forbidden), "feedback leaked {forbidden}");
    }
}

#[test]
fn visible_actor_overflow_fails_instead_of_truncating() {
    let mut engine = engine();
    let template = engine.world().actors[1].clone();
    engine.world_mut().actors.clear();
    for index in 0..=MAX_OBSERVER_ACTORS {
        let mut actor = template.clone();
        actor.id = ActorId::from(format!("actor-{index:03}"));
        actor.kind = ActorKind::Player;
        actor.location = WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 });
        actor.timing.tie_break_order = index as u64;
        engine.world_mut().actors.push(actor);
    }
    let observer = engine.world().actors[0].id.clone();
    assert!(engine.observer_projection(&observer, &[]).is_err());
}
