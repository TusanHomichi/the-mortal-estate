use super::*;
use crate::model::{ActorId, ActorKind, CharacterPresenceState, LogicalTime};

fn social_engine(character_count: usize) -> Engine {
    let mut engine = crate::engine::setup::test_engine("character_sheet");
    engine
        .world
        .actors
        .retain(|actor| actor.kind == ActorKind::Player);
    let original_character_id = engine.world.actors[0]
        .character_id
        .clone()
        .expect("fixture character");
    engine.world.communication_preferences.clear();
    engine.world.character_presence.clear();
    for index in 0..character_count {
        let character_id = CharacterId::new(format!("character:{index}"));
        if index == 0 {
            engine.world.actors[0].character_id = Some(character_id.clone());
            engine.world.actors[0].id = ActorId::new("player0");
            engine.world.actors[0].name = "Player 0".to_string();
        } else {
            let mut actor = engine.world.actors[0].clone();
            actor.id = ActorId::new(format!("player{index}"));
            actor.name = format!("Player {index}");
            actor.character_id = Some(character_id.clone());
            actor.timing.tie_break_order = u64::try_from(index).unwrap() + 10;
            actor.carried.items.clear();
            actor.carried.gold = Default::default();
            engine.world.actors.push(actor);
        }
        engine
            .world
            .communication_preferences
            .insert(character_id.clone(), CommunicationPreferences::default());
        engine.world.character_presence.insert(
            character_id,
            CharacterPresenceState {
                connected: true,
                control_epoch: 1,
                absent_since: None,
            },
        );
    }
    engine.world.quest_states.remove(&original_character_id);
    engine
}

fn group(engine: &mut Engine, count: usize) -> GroupId {
    let first = CharacterId::new("character:0");
    for index in 1..count {
        let target = CharacterId::new(format!("character:{index}"));
        let invite = engine
            .apply_social_intent(
                &ActorId::new("player0"),
                SocialIntent::Invite {
                    target_character_id: target.clone(),
                },
            )
            .expect("invite");
        let invitation_id = invite
            .events
            .iter()
            .find_map(|event| match event {
                Event::GroupInvitationCreated { invitation_id, .. } => Some(*invitation_id),
                _ => None,
            })
            .expect("invitation ID");
        engine
            .apply_social_intent(
                &ActorId::new(format!("player{index}")),
                SocialIntent::AcceptInvite { invitation_id },
            )
            .expect("accept");
    }
    engine.group_id_for_character(&first).expect("group")
}

#[test]
fn group_contract_caps_six_and_social_commands_are_free() {
    let mut engine = social_engine(7);
    let ready_before = engine.world.actors[0].timing.ready_at;
    let now_before = engine.world.timing.now;
    let group_id = group(&mut engine, MAX_GROUP_MEMBERS);
    assert_eq!(engine.world.groups[&group_id].members.len(), 6);
    assert_eq!(engine.world.actors[0].timing.ready_at, ready_before);
    assert_eq!(engine.world.timing.now, now_before);
    let error = engine
        .apply_social_intent(
            &ActorId::new("player0"),
            SocialIntent::Invite {
                target_character_id: CharacterId::new("character:6"),
            },
        )
        .expect_err("full group rejects invitation");
    assert_eq!(error.message(), "group is full");
}

#[test]
fn leader_disconnect_grace_falls_back_once_and_reconnect_does_not_reclaim() {
    let mut engine = social_engine(3);
    let group_id = group(&mut engine, 3);
    let leader = CharacterId::new("character:0");
    engine
        .apply_connection_presence(&leader, 2, false)
        .expect("disconnect");
    let disconnected_at = engine.world.timing.now.value();
    engine.world.timing.now = LogicalTime::new(disconnected_at + GROUP_DISCONNECT_GRACE_UNITS - 1);
    engine.expire_group_state(&mut Vec::new()).unwrap();
    assert_eq!(engine.world.groups[&group_id].leader_character_id, leader);
    engine.world.timing.now = LogicalTime::new(disconnected_at + GROUP_DISCONNECT_GRACE_UNITS);
    let mut events = Vec::new();
    engine.expire_group_state(&mut events).unwrap();
    assert_eq!(
        engine.world.groups[&group_id].leader_character_id,
        CharacterId::new("character:1")
    );
    engine
        .apply_connection_presence(&leader, 3, true)
        .expect("reconnect");
    assert_eq!(
        engine.world.groups[&group_id].leader_character_id,
        CharacterId::new("character:1")
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Event::GroupChanged {
            reason: GroupChangeReasonV1::LeadershipFallback,
            ..
        }
    )));
}

#[test]
fn dormant_group_promotes_the_first_member_who_reconnects() {
    let mut engine = social_engine(3);
    let group_id = group(&mut engine, 3);
    for index in 0..3 {
        engine
            .apply_connection_presence(&CharacterId::new(format!("character:{index}")), 2, false)
            .expect("disconnect");
    }
    let disconnected_at = engine.world.timing.now.value();
    engine.world.timing.now = LogicalTime::new(disconnected_at + GROUP_DISCONNECT_GRACE_UNITS);
    engine.expire_group_state(&mut Vec::new()).unwrap();
    assert_eq!(
        engine.world.groups[&group_id].leader_character_id,
        CharacterId::new("character:0")
    );
    engine
        .apply_connection_presence(&CharacterId::new("character:2"), 3, true)
        .expect("first reconnect");
    assert_eq!(
        engine.world.groups[&group_id].leader_character_id,
        CharacterId::new("character:2")
    );
}

#[test]
fn follow_rejects_cycles_and_target_disconnect_clears_the_edge() {
    let mut engine = social_engine(3);
    group(&mut engine, 3);
    engine
        .apply_social_intent(
            &ActorId::new("player1"),
            SocialIntent::BeginFollow {
                target_character_id: CharacterId::new("character:0"),
            },
        )
        .expect("first follow");
    engine
        .apply_social_intent(
            &ActorId::new("player2"),
            SocialIntent::BeginFollow {
                target_character_id: CharacterId::new("character:1"),
            },
        )
        .expect("second follow");
    assert!(
        engine
            .apply_social_intent(
                &ActorId::new("player0"),
                SocialIntent::BeginFollow {
                    target_character_id: CharacterId::new("character:2"),
                },
            )
            .is_err()
    );
    engine
        .apply_connection_presence(&CharacterId::new("character:0"), 2, false)
        .expect("target disconnect");
    assert!(
        !engine
            .world
            .player_follow_targets
            .contains_key(&CharacterId::new("character:1"))
    );
    assert_eq!(
        engine
            .world
            .player_follow_targets
            .get(&CharacterId::new("character:2")),
        Some(&CharacterId::new("character:1"))
    );
}

#[test]
fn leadership_can_be_transferred_and_voluntary_leave_uses_tenure() {
    let mut engine = social_engine(3);
    let group_id = group(&mut engine, 3);
    engine
        .apply_social_intent(
            &ActorId::new("player0"),
            SocialIntent::TransferLeadership {
                member_character_id: CharacterId::new("character:2"),
            },
        )
        .expect("handoff");
    assert_eq!(
        engine.world.groups[&group_id].leader_character_id,
        CharacterId::new("character:2")
    );
    engine
        .apply_social_intent(&ActorId::new("player2"), SocialIntent::LeaveGroup)
        .expect("leader leaves");
    assert_eq!(
        engine.world.groups[&group_id].leader_character_id,
        CharacterId::new("character:0")
    );
}

#[test]
fn social_checkpoint_round_trip_is_canonical() {
    let mut engine = social_engine(3);
    group(&mut engine, 3);
    engine
        .apply_social_intent(
            &ActorId::new("player1"),
            SocialIntent::BeginFollow {
                target_character_id: CharacterId::new("character:0"),
            },
        )
        .expect("follow");
    let checkpoint = engine.export_checkpoint().expect("checkpoint");
    let hydrated =
        Engine::hydrate_checkpoint(engine.definition.clone(), &checkpoint).expect("hydrate");
    assert_eq!(
        hydrated.export_checkpoint().unwrap().as_bytes(),
        checkpoint.as_bytes()
    );
}

#[test]
fn restart_disconnect_preserves_an_existing_absence_deadline() {
    let mut engine = social_engine(2);
    let absent = CharacterId::new("character:0");
    engine.world.timing.now = LogicalTime::from_millis(4_127);
    engine.apply_connection_presence(&absent, 2, false).unwrap();
    engine.world.timing.now = LogicalTime::from_millis(5_300);
    engine.mark_all_characters_disconnected().unwrap();
    assert_eq!(
        engine.world.character_presence[&absent].absent_since,
        Some(LogicalTime::from_millis(4_127))
    );
    assert_eq!(
        engine.world.character_presence[&CharacterId::new("character:1")].absent_since,
        Some(LogicalTime::from_millis(5_300))
    );
    assert!(
        !engine
            .mark_all_characters_disconnected()
            .unwrap()
            .state_changed
    );
}
