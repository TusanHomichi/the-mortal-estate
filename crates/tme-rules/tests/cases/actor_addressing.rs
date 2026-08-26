use crate::support::content_parts::ContentParts;
use tme_rules::{ActorId, ActorKind, Event, PlayerIntent};

fn validated_two_player_engine() -> tme_rules::Engine {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    let mut second = parts.actors_mut()[0].clone();
    second["id"] = serde_json::json!("player_two");
    second["location"]["position"] = serde_json::json!({"x": 2, "y": 3});
    second["carried"]["items"] = serde_json::json!([]);
    parts
        .actors_mut()
        .as_array_mut()
        .expect("seed actors")
        .push(second);
    parts.engine(7).expect("two-player rules graph")
}

#[test]
fn rules_content_accepts_multiple_players_and_snapshot_sorts_their_ids() {
    let engine = validated_two_player_engine();
    assert_eq!(
        engine.snapshot().controlled_actor_ids,
        [ActorId::from("player"), ActorId::from("player_two")]
    );
}

#[test]
fn addressed_turns_handoff_without_an_implicit_first_player() {
    let mut engine = validated_two_player_engine();
    let first = engine
        .apply_actor_intent(&ActorId::from("player"), PlayerIntent::Wait)
        .expect("first addressed turn");
    assert!(first.state_changed);
    assert!(first.events.iter().any(|event| matches!(
        event,
        Event::ActorReady { actor_id, kind: ActorKind::Player, .. }
            if actor_id == "player"
    )));

    let second = engine
        .apply_actor_intent(&ActorId::from("player_two"), PlayerIntent::Wait)
        .expect("handed-off addressed turn");
    assert!(second.state_changed);
    assert!(second.events.iter().any(|event| matches!(
        event,
        Event::ActorReady { actor_id, kind: ActorKind::Player, .. }
            if actor_id == "player_two"
    )));
}

#[test]
fn free_reads_report_no_state_change_for_the_addressed_actor() {
    let mut engine = validated_two_player_engine();
    let before = engine.snapshot();
    let outcome = engine
        .apply_actor_intent(&ActorId::from("player"), PlayerIntent::Inspect)
        .expect("addressed inspect");
    assert!(!outcome.state_changed);
    assert_eq!(engine.snapshot(), before);
}
