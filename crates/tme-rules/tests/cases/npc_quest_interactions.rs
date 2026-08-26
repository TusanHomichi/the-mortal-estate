use crate::support::content_parts::ContentParts;
use tme_rules::{
    ActionBlockedReasonV1, COMMAND_CONTRACT_VERSION, CharacterAlignment, CharacterId, Coord,
    Engine, Event, ExplicitTraversalKind, GroundItem, LogicalTime, NavigationKind,
    NpcFollowDecisionV1, NpcFollowWaitReasonV1, NpcInteractionOutcome, PhysicalAttackMode,
    PlayerCommandV1, PlayerIntent, PlayerIntentPayloadV1, QuestId, QuestStageId,
    SocialAlignmentSource, SocialBehavior, TransactionCostReceiptV1, TransactionRewardReceiptV1,
    TransactionSourceV1, VerticalDirection, WorldPosition,
};

fn fixture_value() -> ContentParts {
    ContentParts::tracked("npc_quest_interactions", "profile/npc_quest_interactions")
}

fn engine_from(value: ContentParts) -> Engine {
    value.engine(7).expect("EF graph starts")
}

fn engine() -> Engine {
    engine_from(fixture_value())
}

fn interact(npc_actor_id: &str, interaction_id: &str, item: Option<&str>) -> PlayerIntent {
    PlayerIntent::InteractWithNpc {
        npc_actor_id: npc_actor_id.into(),
        interaction_id: interaction_id.to_string(),
        item_instance_id: item.map(str::to_string),
    }
}

fn status(engine: &Engine, intent: PlayerIntent) -> tme_rules::PlayerCommandStatusV1 {
    let command = engine
        .actor_command_for_intent(&tme_rules::ActorId::from("player"), &intent)
        .expect("intent converts to a command");
    engine
        .validate_actor_command(&command)
        .expect("command status")
}

fn start_quest(engine: &mut Engine) -> Vec<Event> {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            interact("wayfinder", "ask_about_crossing", None),
        )
        .expect("opening interaction commits")
        .events
}

fn begin_follow(engine: &mut Engine) -> Vec<Event> {
    start_quest(engine);
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            interact(
                "wayfinder",
                "offer_signal_token",
                Some("player_signal_token"),
            ),
        )
        .expect("item delivery begins follow")
        .events
}

fn actor_index(engine: &Engine, actor_id: &str) -> usize {
    engine
        .world()
        .actors
        .iter()
        .position(|actor| actor.id == actor_id)
        .expect("actor exists")
}

fn character_id() -> CharacterId {
    serde_json::from_str(r#""character:harbor:primary""#).expect("character ID")
}

fn transaction(events: &[Event]) -> &Event {
    let matches = events
        .iter()
        .filter(|event| matches!(event, Event::TransactionCommitted { .. }))
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1, "one transaction receipt per interaction");
    matches[0]
}

fn event_index(events: &[Event], predicate: impl Fn(&Event) -> bool) -> usize {
    events
        .iter()
        .position(predicate)
        .expect("expected ordered event")
}

#[test]
fn ef_end_to_end_uses_shared_item_quest_follow_movement_stair_and_reward_owners() {
    let mut engine = engine();

    let initial = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("initial action context");
    assert!(initial.quest_log.is_empty());
    assert_eq!(
        initial
            .npcs_here
            .iter()
            .map(|npc| npc.actor_id.as_str())
            .collect::<Vec<_>>(),
        vec!["wayfinder"]
    );

    let opening = start_quest(&mut engine);
    let Event::TransactionCommitted {
        source,
        costs,
        rewards,
        ..
    } = transaction(&opening)
    else {
        unreachable!()
    };
    assert!(matches!(
        source,
        TransactionSourceV1::NpcInteraction {
            npc_actor_id,
            interaction_id,
        } if npc_actor_id == "wayfinder" && interaction_id == "ask_about_crossing"
    ));
    assert!(costs.is_empty());
    assert!(matches!(
        rewards.as_slice(),
        [
            TransactionRewardReceiptV1::NpcInteraction {
                outcome: NpcInteractionOutcome::Speak,
                ..
            },
            TransactionRewardReceiptV1::QuestStage {
                before_stage_id: None,
                after_stage_id,
                ..
            }
        ] if after_stage_id == "awaiting_token"
    ));

    let offered = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            interact(
                "wayfinder",
                "offer_signal_token",
                Some("player_signal_token"),
            ),
        )
        .expect("exact item delivery commits");
    let Event::TransactionCommitted {
        source,
        costs,
        rewards,
        ..
    } = transaction(&offered.events)
    else {
        unreachable!()
    };
    assert!(matches!(
        source,
        TransactionSourceV1::NpcInteraction {
            npc_actor_id,
            interaction_id,
        } if npc_actor_id == "wayfinder" && interaction_id == "offer_signal_token"
    ));
    assert!(matches!(
        costs.as_slice(),
        [TransactionCostReceiptV1::SelectedCarriedItem {
            item_instance_id,
            item_definition_id,
            consumed_quantity: 1,
            remaining_quantity: 0,
        }] if item_instance_id == "player_signal_token" && item_definition_id == "signal_token"
    ));
    assert!(matches!(
        rewards.as_slice(),
        [
            TransactionRewardReceiptV1::NpcInteraction {
                outcome: NpcInteractionOutcome::BeginFollow,
                ..
            },
            TransactionRewardReceiptV1::QuestStage {
                before_stage_id: Some(before),
                after_stage_id,
                ..
            }
        ] if before == "awaiting_token" && after_stage_id == "escorting_guide"
    ));
    assert!(
        !engine
            .world()
            .item_instances
            .contains_key("player_signal_token")
    );
    assert!(engine.world().item_offers.is_empty());

    let movement = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![tme_rules::Direction::East]),
        )
        .expect("player and follower move to stairs");
    let decision = event_index(&movement.events, |event| {
        matches!(
            event,
            Event::NpcFollowDecision {
                npc_actor_id,
                decision: NpcFollowDecisionV1::Move {
                    direction: tme_rules::Direction::East
                },
                ..
            } if npc_actor_id == "wayfinder"
        )
    });
    let follower_move = event_index(
        &movement.events,
        |event| matches!(event, Event::Moved { actor_id, .. } if actor_id == "wayfinder"),
    );
    assert!(decision < follower_move);

    let climb = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            interact("wayfinder", "climb_to_watch", None),
        )
        .expect("directed NPC stair climb commits");
    let speech = event_index(
        &climb.events,
        |event| matches!(event, Event::NpcSpoke { npc_actor_id, .. } if npc_actor_id == "wayfinder"),
    );
    let transition = event_index(&climb.events, |event| {
        matches!(
            event,
            Event::WorldTransition {
                actor_id,
                navigation: NavigationKind::Stairs {
                    direction: VerticalDirection::Up
                },
                ..
            } if actor_id == "wayfinder"
        )
    });
    let committed = event_index(&climb.events, |event| {
        matches!(event, Event::TransactionCommitted { .. })
    });
    assert!(speech < transition && transition < committed);

    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::Traverse(ExplicitTraversalKind::StairsUp),
        )
        .expect("existing player stair path commits");
    let completion = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            interact("watchkeeper", "complete_harbor_escort", None),
        )
        .expect("escort completion commits");
    let speech = event_index(
        &completion.events,
        |event| matches!(event, Event::NpcSpoke { npc_actor_id, .. } if npc_actor_id == "watchkeeper"),
    );
    let follow_clear = event_index(&completion.events, |event| {
        matches!(
            event,
            Event::NpcFollowChanged {
                npc_actor_id,
                to_character_id: None,
                ..
            } if npc_actor_id == "wayfinder"
        )
    });
    let quest_change = event_index(&completion.events, |event| {
        matches!(
            event,
            Event::QuestStateChanged {
                quest_id,
                after_stage_id,
                ..
            } if quest_id == "harbor_escort" && after_stage_id == "completed"
        )
    });
    let xp = event_index(&completion.events, |event| {
        matches!(event, Event::ExperienceAwarded { amount: 7, .. })
    });
    let committed = event_index(&completion.events, |event| {
        matches!(event, Event::TransactionCommitted { .. })
    });
    assert!(
        speech < follow_clear && follow_clear < quest_change && quest_change < xp && xp < committed
    );

    let Event::TransactionCommitted { rewards, .. } = transaction(&completion.events) else {
        unreachable!()
    };
    assert!(matches!(
        rewards.as_slice(),
        [
            TransactionRewardReceiptV1::NpcInteraction {
                outcome: NpcInteractionOutcome::CompleteEscort { npc_actor_id },
                ..
            },
            TransactionRewardReceiptV1::QuestStage { after_stage_id, .. },
            TransactionRewardReceiptV1::Experience {
                amount: 7,
                total_xp: 7,
            }
        ] if npc_actor_id == "wayfinder" && after_stage_id == "completed"
    ));

    let player = &engine.world().actors[actor_index(&engine, "player")];
    assert_eq!(
        player
            .character
            .as_ref()
            .expect("character")
            .progression
            .experience,
        7
    );
    let guide = &engine.world().actors[actor_index(&engine, "wayfinder")];
    assert_eq!(
        guide
            .npc
            .as_ref()
            .expect("NPC state")
            .following_character_id,
        None
    );
    assert!(guide.ai.is_some(), "lawful-human NPC keeps its social AI");
    assert_ne!(
        guide.timing.ready_at,
        LogicalTime::new(u64::MAX),
        "ending follow does not unschedule the NPC's independent social AI"
    );
    assert_eq!(
        engine.world().quest_states[&character_id()][&QuestId::new("harbor_escort")],
        QuestStageId::new("completed")
    );
}

#[test]
fn npc_discovery_is_planner_backed_ordered_private_and_requires_exact_item_selection() {
    let mut engine = engine();
    let before = engine.world().clone();
    let first = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("first context");
    let second = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("second context");
    assert_eq!(first, second);
    assert_eq!(engine.world(), &before);
    let guide = &first.npcs_here[0];
    assert_eq!(
        guide
            .interactions
            .iter()
            .map(|interaction| interaction.interaction_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "ask_about_crossing",
            "offer_signal_token",
            "climb_to_watch",
            "end_escort",
        ]
    );
    assert!(guide.interactions[0].actions[0].enabled);
    assert_eq!(
        guide.interactions[1].actions[0].blocked_reason,
        Some(ActionBlockedReasonV1::QuestStateMismatch)
    );
    let serialized = serde_json::to_string(&first).expect("context serializes");
    assert!(!serialized.contains("A steady hand can help"));
    assert!(!serialized.contains("The signal is clear"));

    start_quest(&mut engine);
    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("post-opening context");
    let offer = context.npcs_here[0]
        .interactions
        .iter()
        .find(|interaction| interaction.interaction_id == "offer_signal_token")
        .expect("offer interaction");
    assert_eq!(offer.actions.len(), 1);
    assert!(offer.actions[0].enabled);
    assert!(matches!(
        offer.actions[0]
            .command
            .as_ref()
            .map(|command| &command.intent),
        Some(PlayerIntentPayloadV1::InteractWithNpc {
            item_instance_id: Some(item_instance_id),
            ..
        }) if item_instance_id == "player_signal_token"
    ));

    let player_index = actor_index(&engine, "player");
    engine.world_mut().actors[player_index]
        .carried
        .items
        .clear();
    let context = engine
        .actor_observed_action_context(&tme_rules::ActorId::from("player"))
        .expect("missing-item context");
    let offer = context.npcs_here[0]
        .interactions
        .iter()
        .find(|interaction| interaction.interaction_id == "offer_signal_token")
        .expect("offer interaction");
    assert_eq!(offer.actions.len(), 1);
    assert!(!offer.actions[0].enabled);
    assert_eq!(
        offer.actions[0].blocked_reason,
        Some(ActionBlockedReasonV1::MissingRequiredItem)
    );
    assert!(matches!(
        offer.actions[0]
            .command
            .as_ref()
            .map(|command| &command.intent),
        Some(PlayerIntentPayloadV1::InteractWithNpc {
            item_instance_id: None,
            ..
        })
    ));
}

#[test]
fn npc_command_and_provider_failures_are_typed_and_read_only() {
    let mut engine = engine();
    let before = engine.world().clone();
    for (intent, expected) in [
        (
            interact("missing", "ask_about_crossing", None),
            ActionBlockedReasonV1::NoSuchNpc,
        ),
        (
            interact("watch_sentinel", "ask_about_crossing", None),
            ActionBlockedReasonV1::NoSuchNpc,
        ),
        (
            interact("watchkeeper", "complete_harbor_escort", None),
            ActionBlockedReasonV1::NpcNotHere,
        ),
        (
            interact("wayfinder", "missing", None),
            ActionBlockedReasonV1::NoSuchInteraction,
        ),
        (
            interact(
                "wayfinder",
                "ask_about_crossing",
                Some("player_signal_token"),
            ),
            ActionBlockedReasonV1::UnexpectedTransactionInput,
        ),
        (
            interact("wayfinder", "climb_to_watch", None),
            ActionBlockedReasonV1::NpcNotFollowing,
        ),
    ] {
        let result = status(&engine, intent);
        assert!(!result.accepted);
        assert_eq!(result.blocked_reason, Some(expected));
        assert_eq!(engine.world(), &before);
    }

    start_quest(&mut engine);
    for (selection, expected) in [
        (None, ActionBlockedReasonV1::MissingRequiredItem),
        (Some("missing"), ActionBlockedReasonV1::MissingRequiredItem),
    ] {
        let result = status(
            &engine,
            interact("wayfinder", "offer_signal_token", selection),
        );
        assert!(!result.accepted);
        assert_eq!(result.blocked_reason, Some(expected));
    }
    assert_eq!(
        status(&engine, interact("wayfinder", "ask_about_crossing", None)).blocked_reason,
        Some(ActionBlockedReasonV1::QuestStateMismatch)
    );

    let explicit_null = format!(
        r#"{{"contract_version":{COMMAND_CONTRACT_VERSION},"actor_id":"player","intent":{{"interact_with_npc":{{"npc_actor_id":"wayfinder","interaction_id":"ask_about_crossing","item_instance_id":null}}}}}}"#
    );
    serde_json::from_str::<PlayerCommandV1>(&explicit_null).expect("explicit null is current");
    let missing_null = explicit_null.replace(",\"item_instance_id\":null", "");
    assert!(serde_json::from_str::<PlayerCommandV1>(&missing_null).is_err());
    let unknown = explicit_null.replace(
        "\"item_instance_id\":null",
        "\"item_instance_id\":null,\"quest_flag\":true",
    );
    assert!(serde_json::from_str::<PlayerCommandV1>(&unknown).is_err());
}

#[test]
fn follow_lifecycle_covers_end_target_loss_blocked_and_non_stair_transition() {
    let mut ended = engine();
    begin_follow(&mut ended);
    let end_events = ended
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            interact("wayfinder", "end_escort", None),
        )
        .expect("end follow commits");
    assert!(end_events.events.iter().any(|event| matches!(
        event,
        Event::NpcFollowChanged {
            npc_actor_id,
            to_character_id: None,
            ..
        } if npc_actor_id == "wayfinder"
    )));
    let guide = &ended.world().actors[actor_index(&ended, "wayfinder")];
    assert!(
        guide
            .npc
            .as_ref()
            .expect("NPC")
            .following_character_id
            .is_none()
    );
    assert!(guide.ai.is_some(), "lawful-human NPC keeps its social AI");
    assert_ne!(
        guide.timing.ready_at,
        LogicalTime::new(u64::MAX),
        "ending follow leaves independent social AI scheduled"
    );

    let mut lost = engine();
    begin_follow(&mut lost);
    let player_index = actor_index(&lost, "player");
    lost.world_mut().actors[player_index].character_id = Some(
        serde_json::from_str(r#""character:harbor_escort:replacement""#)
            .expect("replacement character id"),
    );
    let lost_events = lost
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("target loss clears follow safely");
    assert!(lost_events.events.iter().any(|event| matches!(
        event,
        Event::NpcFollowChanged {
            npc_actor_id,
            to_character_id: None,
            ..
        } if npc_actor_id == "wayfinder"
    )));
    let guide = &lost.world().actors[actor_index(&lost, "wayfinder")];
    assert!(
        guide
            .npc
            .as_ref()
            .expect("NPC")
            .following_character_id
            .is_none()
    );
    assert!(guide.ai.is_some(), "lawful-human NPC keeps its social AI");
    assert_ne!(
        guide.timing.ready_at,
        LogicalTime::new(u64::MAX),
        "target loss clears follow without disabling independent social AI"
    );

    let mut blocked_value = fixture_value();
    blocked_value.template_levels_source_mut()["crossing"]["cells"][1][2] =
        serde_json::json!(["stone_wall"]);
    blocked_value.world_template["topology"]["edge/crossing/1/2"]["at"]["position"] =
        serde_json::json!({"x": 3, "y": 1});
    blocked_value.world_template["topology"]["edge/watch/1/2"]["target"]["location"]["position"] =
        serde_json::json!({"x": 3, "y": 1});
    let mut blocked = engine_from(blocked_value);
    begin_follow(&mut blocked);
    let player_index = actor_index(&blocked, "player");
    blocked.world_mut().actors[player_index].location.position = Coord { x: 3, y: 1 };
    let blocked_events = blocked
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("blocked follower waits without aborting the player step");
    assert!(
        blocked_events.events.iter().any(|event| matches!(
            event,
            Event::NpcFollowDecision {
                npc_actor_id,
                decision: NpcFollowDecisionV1::Wait {
                    reason: NpcFollowWaitReasonV1::Blocked,
                },
                ..
            } if npc_actor_id == "wayfinder"
        )),
        "{blocked_events:#?}"
    );
    let guide = &blocked.world().actors[actor_index(&blocked, "wayfinder")];
    assert_eq!(guide.location.position, Coord { x: 1, y: 1 });
    assert_eq!(
        guide.npc.as_ref().expect("NPC").following_character_id,
        Some(character_id())
    );

    let mut door_value = fixture_value();
    door_value.world_template["topology"]["edge/crossing/1/2"]["kind"] =
        serde_json::json!({"kind": "door", "initial_state": "open"});
    let mut across = engine_from(door_value);
    begin_follow(&mut across);
    let player_index = actor_index(&across, "player");
    across.world_mut().actors[player_index].location.level = "watch".to_string();
    across.world_mut().actors[player_index].location.position = Coord { x: 2, y: 1 };
    let across_events = across
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("follower uses the existing non-stair transition path");
    assert!(across_events.events.iter().any(|event| matches!(
        event,
        Event::WorldTransition {
            actor_id,
            navigation: NavigationKind::Door,
            to,
            ..
        } if actor_id == "wayfinder" && to.level == "watch"
    )));
    let guide = &across.world().actors[actor_index(&across, "wayfinder")];
    assert_eq!(guide.location.level, "watch");
    assert_eq!(guide.location.position, Coord { x: 2, y: 1 });
}

#[test]
fn controlled_target_death_clears_follow_before_automatic_draining_stops() {
    let mut engine = engine();
    begin_follow(&mut engine);
    let player_index = actor_index(&engine, "player");
    let sentinel_index = actor_index(&engine, "watch_sentinel");
    let player_room = engine.world().actors[player_index].location.level.clone();
    let player_position = engine.world().actors[player_index].location.position;
    {
        let player = &mut engine.world_mut().actors[player_index];
        player.hp = 1;
        player
            .character
            .as_mut()
            .expect("player character")
            .resources
            .hp = 1;
    }
    {
        let sentinel = &mut engine.world_mut().actors[sentinel_index];
        sentinel.location.level = player_room;
        sentinel.location.position = player_position;
        sentinel.home_location = sentinel.location.clone();
        sentinel.social.alignment_source = SocialAlignmentSource::Inherent {
            alignment: CharacterAlignment::Chaotic,
        };
        sentinel.social.behavior = SocialBehavior::AlignmentCreature;
        sentinel.stats.attack = 100;
        sentinel.ai.as_mut().expect("sentinel AI").behavior =
            tme_rules::ActorAiBehavior::SimpleChase;
    }

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("fatal automatic action commits");
    assert!(!engine.world().actors[player_index].is_alive());
    assert!(events.events.iter().any(|event| matches!(
        event,
        Event::NpcFollowChanged {
            npc_actor_id,
            from_character_id: Some(from),
            to_character_id: None,
            ..
        } if npc_actor_id == "wayfinder" && from == &character_id()
    )));
    let guide = &engine.world().actors[actor_index(&engine, "wayfinder")];
    assert!(
        guide
            .npc
            .as_ref()
            .expect("NPC")
            .following_character_id
            .is_none()
    );
    assert!(guide.ai.is_some(), "lawful-human NPC keeps its social AI");
    assert_ne!(
        guide.timing.ready_at,
        LogicalTime::new(u64::MAX),
        "target death clears follow without disabling independent social AI"
    );
}

#[test]
fn directed_climb_rejects_no_stair_wrong_direction_and_changed_follow_state() {
    let mut no_stair = engine();
    begin_follow(&mut no_stair);
    no_stair
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![tme_rules::Direction::East]),
        )
        .expect("reach stair");
    for actor_id in ["player", "wayfinder"] {
        let index = actor_index(&no_stair, actor_id);
        no_stair.world_mut().actors[index].location.position = Coord { x: 1, y: 1 };
    }
    assert_eq!(
        status(&no_stair, interact("wayfinder", "climb_to_watch", None)).blocked_reason,
        Some(ActionBlockedReasonV1::NpcCannotClimb)
    );

    let mut wrong_value = fixture_value();
    wrong_value.actors_mut()[1]["npc"]["interactions"][2]["outcome"]["direction"] =
        serde_json::json!("down");
    let mut wrong = engine_from(wrong_value);
    begin_follow(&mut wrong);
    wrong
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MovePath(vec![tme_rules::Direction::East]),
        )
        .expect("reach stair");
    assert_eq!(
        status(&wrong, interact("wayfinder", "climb_to_watch", None)).blocked_reason,
        Some(ActionBlockedReasonV1::NpcCannotClimb)
    );

    let mut changed = engine();
    begin_follow(&mut changed);
    changed
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            interact("wayfinder", "end_escort", None),
        )
        .expect("clear follow");
    assert_eq!(
        status(&changed, interact("wayfinder", "climb_to_watch", None)).blocked_reason,
        Some(ActionBlockedReasonV1::NpcNotFollowing)
    );
}

#[test]
fn post_selected_item_failure_rolls_back_item_quest_follow_time_and_rng() {
    let mut value = fixture_value();
    value.selected_by_runtime_id_mut("items", "signal_token")["valid_placements"] =
        serde_json::json!(["hand", "sack", "belt_side"]);
    value.actors_mut()[1]["npc"]["interactions"][1]["transaction"]["rewards"]
        .as_array_mut()
        .expect("offer rewards")
        .push(serde_json::json!({
            "kind": "item",
            "item_instance_id": "rollback_reward",
            "item_definition_id": "signal_token",
            "position": "belt_1"
        }));
    let mut engine = engine_from(value);
    start_quest(&mut engine);
    engine.world_mut().ground_items.push(GroundItem {
        item_instance_id: "corrupt_unregistered_location".to_string(),
        location: WorldPosition::new("realm_0", "crossing", Coord { x: 3, y: 1 }),
        loot_claim: None,
    });
    let mut control = engine.clone();
    let before = engine.world().clone();
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            interact(
                "wayfinder",
                "offer_signal_token",
                Some("player_signal_token"),
            ),
        )
        .expect_err("late item registration fails after selected-item cost");
    assert!(
        error
            .to_string()
            .contains("item location references unknown instance")
    );
    assert_eq!(engine.world(), &before);
    assert!(
        engine
            .world()
            .item_instances
            .contains_key("player_signal_token")
    );
    assert_eq!(
        engine.world().quest_states[&character_id()][&QuestId::new("harbor_escort")],
        QuestStageId::new("awaiting_token")
    );
    assert!(
        engine.world().actors[actor_index(&engine, "wayfinder")]
            .npc
            .as_ref()
            .expect("NPC")
            .following_character_id
            .is_none()
    );

    for candidate in [&mut engine, &mut control] {
        candidate.world_mut().ground_items.clear();
        let player_position = candidate.world().actors[actor_index(candidate, "player")]
            .location
            .position;
        let monster_index = actor_index(candidate, "watch_sentinel");
        candidate.world_mut().actors[monster_index].location.level = "crossing".to_string();
        candidate.world_mut().actors[monster_index]
            .location
            .position = player_position;
        candidate.world_mut().actors[monster_index]
            .social
            .alignment_source = SocialAlignmentSource::Inherent {
            alignment: CharacterAlignment::Chaotic,
        };
        candidate.world_mut().actors[monster_index].social.behavior =
            SocialBehavior::AlignmentCreature;
    }
    let attack = PlayerIntent::PhysicalAttack {
        authorization: tme_rules::HostilityAuthorization::Safe,
        mode: PhysicalAttackMode::Fight,
        target_actor_id: "watch_sentinel".into(),
    };
    let actual_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), attack.clone())
        .expect("post-rollback RNG probe");
    let control_events = control
        .apply_actor_intent(&tme_rules::ActorId::from("player"), attack)
        .expect("control RNG probe");
    assert_eq!(actual_events, control_events);
    assert_eq!(engine.world(), control.world());
}

#[test]
fn debug_snapshot_has_full_sorted_ledger_while_observed_frame_exposes_only_controlled_log() {
    let mut engine = engine();
    start_quest(&mut engine);
    let foreign_character: CharacterId =
        serde_json::from_str(r#""character:foreign:private""#).expect("foreign ID");
    engine.world_mut().quest_states.insert(
        foreign_character.clone(),
        std::collections::BTreeMap::from([(
            QuestId::new("harbor_escort"),
            QuestStageId::new("completed"),
        )]),
    );

    let debug = engine.snapshot();
    assert_eq!(debug.quest_states.len(), 2);
    assert_eq!(
        debug
            .quest_states
            .iter()
            .map(|row| row.character_id.as_str())
            .collect::<Vec<_>>(),
        vec!["character:foreign:private", "character:harbor:primary"]
    );
    let frame = engine
        .actor_observed_frame(&tme_rules::ActorId::from("player"))
        .expect("observed frame");
    assert_eq!(frame.action_context.quest_log.len(), 1);
    assert_eq!(frame.action_context.quest_log[0].stage_id, "awaiting_token");
    let observed_json = serde_json::to_string(&frame).expect("frame serializes");
    assert!(!observed_json.contains("character:foreign:private"));

    let mut missing_actor_npc = serde_json::to_value(&debug).expect("debug JSON");
    missing_actor_npc["actors"][0]
        .as_object_mut()
        .expect("actor object")
        .remove("npc");
    assert!(serde_json::from_value::<tme_rules::WorldSnapshotV1>(missing_actor_npc).is_err());
    let mut missing_ledger = serde_json::to_value(&debug).expect("debug JSON");
    missing_ledger
        .as_object_mut()
        .expect("debug object")
        .remove("quest_states");
    assert!(serde_json::from_value::<tme_rules::WorldSnapshotV1>(missing_ledger).is_err());

    for field in ["npcs_here", "quest_log"] {
        let mut missing = serde_json::to_value(&frame.action_context).expect("action context JSON");
        missing
            .as_object_mut()
            .expect("action context object")
            .remove(field);
        assert!(serde_json::from_value::<tme_rules::PlayerActionContextV2>(missing).is_err());
    }
}

#[test]
fn event_39_npc_and_quest_payloads_are_strict_with_required_nulls() {
    assert_eq!(tme_rules::EVENT_CONTRACT_VERSION, 40);
    let mut engine = engine();
    let opening = start_quest(&mut engine);
    let offered = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            interact(
                "wayfinder",
                "offer_signal_token",
                Some("player_signal_token"),
            ),
        )
        .expect("follow starts");

    let opening_quest = opening
        .iter()
        .find(|event| matches!(event, Event::QuestStateChanged { .. }))
        .expect("opening quest event");
    let mut missing_before = serde_json::to_value(opening_quest).expect("event JSON");
    missing_before["quest_state_changed"]
        .as_object_mut()
        .expect("quest payload")
        .remove("before_stage_id");
    assert!(serde_json::from_value::<Event>(missing_before).is_err());

    let follow = offered
        .events
        .iter()
        .find(|event| matches!(event, Event::NpcFollowChanged { .. }))
        .expect("follow event");
    for nullable_field in ["from_character_id", "to_character_id"] {
        let mut missing = serde_json::to_value(follow).expect("follow JSON");
        missing["npc_follow_changed"]
            .as_object_mut()
            .expect("follow payload")
            .remove(nullable_field);
        assert!(serde_json::from_value::<Event>(missing).is_err());
    }

    let spoke = offered
        .events
        .iter()
        .find(|event| matches!(event, Event::NpcSpoke { .. }))
        .expect("NPC response event");
    let Event::NpcSpoke {
        recipient_character_id,
        ..
    } = spoke
    else {
        unreachable!("selected NPC response")
    };
    assert_eq!(recipient_character_id.as_str(), "character:harbor:primary");
    let mut missing_recipient = serde_json::to_value(spoke).expect("NPC response JSON");
    missing_recipient["npc_spoke"]
        .as_object_mut()
        .expect("NPC response payload")
        .remove("recipient_character_id");
    assert!(serde_json::from_value::<Event>(missing_recipient).is_err());

    let mut receipt = serde_json::to_value(transaction(&offered.events)).expect("receipt JSON");
    receipt["transaction_committed"]["source"]["legacy_quest_flag"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Event>(receipt).is_err());

    let mut reward_missing_before =
        serde_json::to_value(transaction(&offered.events)).expect("receipt JSON");
    reward_missing_before["transaction_committed"]["rewards"][1]
        .as_object_mut()
        .expect("quest reward")
        .remove("before_stage_id");
    assert!(serde_json::from_value::<Event>(reward_missing_before).is_err());
}

#[test]
fn generic_service_transaction_reuses_alignment_and_quest_gates_and_quest_owner() {
    let mut value = ContentParts::tracked("service_transactions", "profile/service_transactions");
    value.push_selected(
        "quests",
        "quest/service_proof/test",
        serde_json::json!({
            "id": "service_proof",
            "title": "Service Proof",
            "stages": [{"id": "done", "label": "Proof complete", "terminal": true}]
        }),
    );
    let transaction = &mut value
        .selected_by_runtime_id_mut("service_definitions", "waystation_clerk")["capabilities"][0]["transactions"]
        [0];
    transaction["requirements"]
        .as_array_mut()
        .expect("requirements")
        .extend([
            serde_json::json!({"kind": "exact_alignment", "alignment": "lawful"}),
            serde_json::json!({"kind": "quest_unstarted", "quest_id": "service_proof"}),
        ]);
    transaction["rewards"]
        .as_array_mut()
        .expect("rewards")
        .push(serde_json::json!({
            "kind": "quest_stage",
            "quest_id": "service_proof",
            "stage_id": "done"
        }));
    let mut engine = engine_from(value);
    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CommitServiceTransaction {
                service_id: "waystation_clerk".to_string(),
                capability_id: "exchanges".to_string(),
                transaction_id: "token_for_badge".to_string(),
                item_instance_id: Some("etched_token_stack".to_string()),
            },
        )
        .expect("generic service uses the shared gates and quest reward");
    assert!(events.events.iter().any(|event| matches!(
        event,
        Event::QuestStateChanged {
            quest_id,
            before_stage_id: None,
            after_stage_id,
            ..
        } if quest_id == "service_proof" && after_stage_id == "done"
    )));
    assert_eq!(
        engine.world().quest_states[&serde_json::from_str::<CharacterId>(
            r#""character:service_transactions:primary""#
        )
        .expect("service character")][&QuestId::new("service_proof")],
        QuestStageId::new("done")
    );
}

#[test]
fn four_contract_quest_and_npc_shapes_are_required_strict_and_direct() {
    fixture_value()
        .validated_seed()
        .expect("EF content graph validates");

    let mut stale = fixture_value();
    stale.world_seed["schema_version"] = serde_json::json!(24);
    assert!(
        stale
            .validated_seed()
            .expect_err("flat schema field is stale")
            .contains("unknown field `schema_version`")
    );

    let mut missing_quests = fixture_value();
    missing_quests
        .profile_value_mut()
        .as_object_mut()
        .expect("profile object")
        .remove("quests");
    missing_quests
        .validated_seed()
        .expect_err("profile quest selection is required");

    for actor_index in 0..4 {
        let mut missing_npc = fixture_value();
        missing_npc.actors_mut()[actor_index]
            .as_object_mut()
            .expect("actor object")
            .remove("npc");
        missing_npc
            .validated_seed()
            .expect_err("nullable npc key is required on every actor");
    }

    let mut unknown_outcome = fixture_value();
    unknown_outcome.actors_mut()[1]["npc"]["interactions"][0]["outcome"] =
        serde_json::json!({"kind": "set_quest_flag", "flag": "started"});
    unknown_outcome
        .validated_seed()
        .expect_err("outcomes are finite and strict");

    let mut non_npc_role = fixture_value();
    let npc = non_npc_role.actors_mut()[1]["npc"].clone();
    non_npc_role.actors_mut()[3]["npc"] = npc;
    non_npc_role
        .validated_seed()
        .expect_err("monster cannot carry NPC state");
}
