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

#[path = "npc_quest_interactions/ef_end_to_end_uses_shared_item_quest_follow_movement_stair_and_reward_owner.rs"]
mod ef_end_to_end_uses_shared_item_quest_follow_movement_stair_and_reward_owner;

#[path = "npc_quest_interactions/debug_snapshot_has_full_sorted_ledger_while_observed_frame_exposes_only_con.rs"]
mod debug_snapshot_has_full_sorted_ledger_while_observed_frame_exposes_only_con;
