use super::*;
use serde_json::json;
use tme_rules::{ActorId, SocialBroadcastScope};

pub(super) fn return_engine(alignment: &str, witness: bool) -> Engine {
    let mut parts = fixture_value();
    parts.world_template["resurrection"] = json!({"realm_0": {
        "request_delay_ms": 60_000,
        "lawful_destination": WorldPosition::new("realm_0", "room_0", (2, 1).into()),
        "neutral_destination": WorldPosition::new("realm_0", "room_0", (3, 1).into()),
        "hit_points_missing": 1, "stamina_missing": 1
    }});
    parts.actors_mut()[0]["character"]["alignment_state"]["alignment"] = json!(alignment);
    if witness {
        let mut other = parts.actors_mut()[0].clone();
        other["id"] = json!("witness");
        other["character_id"] = json!("character:death_corpse:witness");
        other["location"]["position"] = json!({"x": 1, "y": 1});
        other["carried"] =
            json!({"items": [], "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}});
        for key in ["hp", "max_hp", "peak_hp"] {
            other["character"]["resources"][key] = json!(1000);
        }
        parts.actors_mut().as_array_mut().unwrap().push(other);
    }
    parts.engine(7).unwrap()
}

fn request_at(engine: &Engine) -> LogicalTime {
    let ActorLifeState::Ghost { defeated_at, .. } =
        engine.world().actor(&"player".into()).unwrap().life_state
    else {
        panic!("ordinary death must create a ghost")
    };
    defeated_at.saturating_add_millis(60_000)
}

#[test]
fn ordinary_return_uses_the_death_deadline_across_checkpoint_and_refuses_early_or_repeated_requests()
 {
    for (alignment, destination) in [("lawful", (2, 1)), ("neutral", (3, 1))] {
        let mut engine = return_engine(alignment, false);
        defeat_player(&mut engine);
        let actor = ActorId::from("player");
        let deadline = request_at(&engine);
        engine
            .advance_to(LogicalTime::from_millis(deadline.as_millis() - 1))
            .unwrap();
        let before = engine.export_checkpoint().unwrap();
        let command = engine
            .actor_command_for_intent(&actor, &PlayerIntent::RequestResurrection)
            .unwrap();
        assert_eq!(
            engine
                .validate_actor_command(&command)
                .unwrap()
                .blocked_reason,
            Some(ActionBlockedReasonV1::NotReady)
        );
        assert!(
            engine
                .apply_realtime_actor_intent(&actor, PlayerIntent::RequestResurrection)
                .is_err()
        );
        assert_eq!(engine.export_checkpoint().unwrap(), before);
        assert_eq!(engine.next_deadline(), Some(deadline));
        let mut restored =
            Engine::hydrate_checkpoint(engine.definition().clone(), &before).unwrap();
        for candidate in [&mut engine, &mut restored] {
            candidate.advance_to(deadline).unwrap();
            let frame = candidate.observer_projection(&actor, &[]).unwrap().frame;
            assert!(frame.can_act);
            assert_eq!(frame.ready_at, deadline);
            assert!(candidate.validate_actor_command(&command).unwrap().accepted);
            let result = candidate
                .apply_realtime_actor_intent(&actor, PlayerIntent::RequestResurrection)
                .unwrap();
            assert_eq!(
                result
                    .events
                    .iter()
                    .filter(|event| matches!(event, Event::ActorResurrected { .. }))
                    .count(),
                1
            );
            let player = candidate.world().actor(&actor).unwrap();
            assert!(player.is_alive());
            assert_eq!(player.location.position, destination.into());
            assert_eq!(
                player.timing.ready_at,
                deadline.saturating_add_millis(3_000)
            );
            assert!(!candidate.world().corpses.contains_key(&corpse_id(3)));
            assert!(matches!(
                candidate.item_location("flint").unwrap(),
                ItemLocation::Carried { .. }
            ));
            assert!(matches!(
                candidate.item_location("oak_club").unwrap(),
                ItemLocation::Ground { .. }
            ));
            let returned = candidate.export_checkpoint().unwrap();
            assert!(
                candidate
                    .apply_realtime_actor_intent(&actor, PlayerIntent::RequestResurrection)
                    .is_err()
            );
            assert_eq!(candidate.export_checkpoint().unwrap(), returned);
        }
        assert_eq!(
            engine.export_checkpoint().unwrap(),
            restored.export_checkpoint().unwrap()
        );
    }
}

#[test]
fn an_eligible_ghost_still_cannot_move_fight_wait_or_handle_items() {
    let mut engine = return_engine("lawful", false);
    defeat_player(&mut engine);
    engine.advance_to(request_at(&engine)).unwrap();
    let actor = ActorId::from("player");
    let before = engine.export_checkpoint().unwrap();
    for intent in [
        PlayerIntent::MovePath(vec![tme_rules::Direction::South]),
        PlayerIntent::Open(tme_rules::Direction::South),
        PlayerIntent::Wait,
        PlayerIntent::Hide,
        PlayerIntent::Rest,
        PlayerIntent::ShowSack,
        PlayerIntent::SearchCorpse(corpse_id(3)),
        PlayerIntent::PhysicalAttack {
            mode: tme_rules::PhysicalAttackMode::Fight,
            target_actor_id: "brute".into(),
            authorization: tme_rules::HostilityAuthorization::Safe,
        },
    ] {
        let command = engine.actor_command_for_intent(&actor, &intent).unwrap();
        assert_eq!(
            engine
                .validate_actor_command(&command)
                .unwrap()
                .blocked_reason,
            Some(ActionBlockedReasonV1::ActorNotLiving)
        );
        assert!(engine.apply_realtime_actor_intent(&actor, intent).is_err());
        assert_eq!(engine.export_checkpoint().unwrap(), before);
    }
    let options = engine.actor_action_options(&actor).unwrap();
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].id, "request_resurrection");
    assert!(options[0].enabled);
}

#[test]
fn a_ghost_keeps_awareness_and_two_way_speech_while_living_observers_see_its_corpse() {
    let mut engine = return_engine("lawful", true);
    let player = ActorId::from("player");
    let witness = ActorId::from("witness");
    let player_character = engine
        .world()
        .actor(&player)
        .unwrap()
        .character_id
        .clone()
        .unwrap();
    let witness_character = engine
        .world()
        .actor(&witness)
        .unwrap()
        .character_id
        .clone()
        .unwrap();
    engine
        .apply_connection_presence(&player_character, 1, true)
        .unwrap();
    engine
        .apply_connection_presence(&witness_character, 1, true)
        .unwrap();
    let events = defeat_player(&mut engine);
    engine
        .apply_realtime_actor_intent(
            &witness,
            PlayerIntent::MovePath(vec![tme_rules::Direction::East]),
        )
        .unwrap();
    let ghost = engine.observer_projection(&player, &events).unwrap();
    assert!(
        ghost
            .frame
            .tiles
            .iter()
            .filter(|tile| tile.terrain_id.is_some())
            .count()
            > 1
    );
    assert!(
        ghost
            .frame
            .actors
            .iter()
            .any(|actor| actor.actor_id == witness)
    );
    assert!(
        ghost
            .frame
            .actors
            .iter()
            .any(|actor| actor.actor_id == player)
    );
    let living = engine.observer_projection(&witness, &events).unwrap();
    assert!(
        !living
            .frame
            .actors
            .iter()
            .any(|actor| actor.actor_id == player)
    );
    assert!(
        living
            .frame
            .corpses
            .iter()
            .any(|corpse| corpse.corpse_id == corpse_id(3))
    );
    assert_eq!(
        engine
            .social_broadcast_recipients(&player_character, SocialBroadcastScope::Say)
            .unwrap(),
        vec![witness_character.clone()]
    );
    assert_eq!(
        engine
            .social_broadcast_recipients(&witness_character, SocialBroadcastScope::Say)
            .unwrap(),
        vec![player_character]
    );
}
