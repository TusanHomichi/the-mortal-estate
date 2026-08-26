use crate::support::content_parts::ContentParts;
use serde_json::json;
use tme_rules::{
    ActionBlockedReasonV1, COMMAND_CONTRACT_VERSION, CarriedPosition, CharacterAlignment, Coord,
    Engine, Event, GroundItem, ItemMoveDestination, PlayerCommandV1, PlayerIntent,
    PlayerIntentPayloadV1, SpellTarget, WorldPosition,
};

const RING_INSTANCE_ID: &str = "oath_ring:knight_promotion:primary";
const KNIGHT_SPELL_IDS: [&str; 5] = ["blessed_edge", "valor", "cleanse", "beacon", "trail_sense"];

fn fixture_value() -> ContentParts {
    ContentParts::tracked("knight_promotion", "profile/knight_promotion")
}

fn engine_from_value(value: ContentParts) -> Engine {
    value.engine(7).expect("engine should start")
}

fn engine() -> Engine {
    engine_from_value(fixture_value())
}

fn promotion_command(target_class_id: &str) -> PlayerCommandV1 {
    PlayerCommandV1 {
        contract_version: COMMAND_CONTRACT_VERSION,
        actor_id: "player".into(),
        intent: PlayerIntentPayloadV1::PromoteClass {
            target_class_id: target_class_id.to_string(),
        },
    }
}

fn promote(engine: &mut Engine) -> Vec<Event> {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PromoteClass("knight".to_string()),
        )
        .expect("promotion should succeed")
        .events
}

fn move_ring(engine: &mut Engine, position: CarriedPosition) -> Vec<Event> {
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::MoveItem {
                item_instance_id: RING_INSTANCE_ID.to_string(),
                destination: ItemMoveDestination::Carried { position },
            },
        )
        .expect("ring move should succeed")
        .events
}

fn player_character(engine: &Engine) -> &tme_rules::CharacterSheetV1 {
    engine
        .world()
        .actor(&tme_rules::ActorId::from("player"))
        .and_then(|player| player.character.as_ref())
        .expect("player should have a character")
}

#[test]
fn lawful_zero_karma_fighter_promotes_at_exact_coordinate() {
    let mut engine = engine();
    let events = promote(&mut engine);
    let character = player_character(&engine);

    assert_eq!(
        character.alignment_state.alignment,
        CharacterAlignment::Lawful
    );
    assert_eq!(character.alignment_state.karma_points, 0);
    assert_eq!(character.identity.base_class_id, "fighter");
    assert_eq!(character.identity.current_class_id, "knight");
    assert_eq!(character.identity.display_class, "Knight");
    assert_eq!(character.resources.mp, 18, "promotion must not grant MP");
    assert_eq!(
        character.resources.max_mp, 18,
        "promotion must not grant MP"
    );
    assert!(character.skill_ledger.is_empty());
    assert_eq!(character.promotion_history.len(), 1);
    assert_eq!(character.promotion_history[0].from_class_id, "fighter");
    assert_eq!(character.promotion_history[0].to_class_id, "knight");
    assert_eq!(character.promotion_history[0].level, 8);
    assert_eq!(
        character
            .known_spells
            .iter()
            .map(|spell| spell.spell_id.as_str())
            .collect::<Vec<_>>(),
        KNIGHT_SPELL_IDS
    );
    assert!(
        character
            .known_spells
            .iter()
            .all(|spell| spell.lane == "knight_magic" && spell.learned_at_level == 8)
    );

    let ring = &engine.world().item_instances[RING_INSTANCE_ID];
    assert_eq!(ring.definition_id, "oath_ring");
    assert_eq!(ring.quantity, 1);
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .carried
            .items[&CarriedPosition::RightHand],
        RING_INSTANCE_ID
    );

    assert!(matches!(events.first(), Some(Event::ActorReady { .. })));
    assert!(matches!(events.get(1), Some(Event::PlayerIntent { .. })));
    match events.get(2) {
        Some(Event::ClassPromoted {
            actor_id,
            actor,
            from_class,
            to_class,
            granted_item_instance_id,
            granted_item_definition_id,
            granted_item,
            granted_item_position,
            granted_spells,
        }) => {
            assert_eq!(actor_id, "player");
            assert_eq!(actor, "Delver");
            assert_eq!(from_class, "Fighter");
            assert_eq!(to_class, "Knight");
            assert_eq!(granted_item_instance_id, RING_INSTANCE_ID);
            assert_eq!(granted_item_definition_id, "oath_ring");
            assert_eq!(granted_item, "Oath Ring");
            assert_eq!(*granted_item_position, CarriedPosition::RightHand);
            assert_eq!(
                granted_spells
                    .iter()
                    .map(|spell| (
                        spell.spell_id.as_str(),
                        spell.spell_name.as_str(),
                        spell.lane.as_str(),
                    ))
                    .collect::<Vec<_>>(),
                vec![
                    ("blessed_edge", "Blessed Edge", "knight_magic"),
                    ("valor", "Valor", "knight_magic"),
                    ("cleanse", "Cleanse", "knight_magic"),
                    ("beacon", "Beacon", "knight_magic"),
                    ("trail_sense", "Trail Sense", "knight_magic"),
                ]
            );
        }
        event => panic!("expected enriched promotion event at commit position, got {event:?}"),
    }
    assert!(matches!(
        events.get(3),
        Some(Event::TransactionCommitted {
            source: tme_rules::TransactionSourceV1::ClassPromotion {
                transaction_id,
                target_class_id,
                ..
            },
            rewards,
            ..
        }) if transaction_id == "fighter_to_knight"
            && target_class_id == "knight"
            && matches!(
                rewards.as_slice(),
                [
                    tme_rules::TransactionRewardReceiptV1::Class { .. },
                    tme_rules::TransactionRewardReceiptV1::Item { .. },
                    tme_rules::TransactionRewardReceiptV1::Spell { .. },
                    tme_rules::TransactionRewardReceiptV1::Spell { .. },
                    tme_rules::TransactionRewardReceiptV1::Spell { .. },
                    tme_rules::TransactionRewardReceiptV1::Spell { .. },
                    tme_rules::TransactionRewardReceiptV1::Spell { .. }
                ]
            )
    ));
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::SpellLearned { .. } | Event::ItemConsumed { .. } | Event::ItemRelocated { .. }
    )));
}

#[test]
fn neutral_zero_karma_fighter_is_not_blocked_by_an_invented_alignment_gate() {
    let mut value = fixture_value();
    value.actors_mut()[0]["character"]["alignment_state"]["alignment"] = json!("neutral");
    let mut engine = engine_from_value(value);

    promote(&mut engine);

    let character = player_character(&engine);
    assert_eq!(
        character.alignment_state.alignment,
        CharacterAlignment::Neutral
    );
    assert_eq!(character.identity.current_class_id, "knight");
}

#[test]
fn promotion_rejects_below_level_nonzero_karma_and_occupied_right_hand() {
    let mut below_level = fixture_value();
    below_level.actors_mut()[0]["character"]["progression"]["level"] = json!(7);
    let mut engine = engine_from_value(below_level);
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PromoteClass("knight".into()),
        )
        .expect_err("level 7 must be rejected");
    assert!(error.to_string().contains("at least level 8"));

    let mut nonzero_karma = fixture_value();
    nonzero_karma.actors_mut()[0]["character"]["alignment_state"]["karma_points"] = json!(1);
    let mut engine = engine_from_value(nonzero_karma);
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PromoteClass("knight".into()),
        )
        .expect_err("nonzero karma must be rejected");
    assert!(error.to_string().contains("exactly 0 karma points"));

    let mut occupied = fixture_value();
    occupied.item_instances_mut()["blocking_ring"] = json!({
        "definition_id": "oath_ring",
        "binding": {"state": "unrestricted"}
    });
    occupied.actors_mut()[0]["carried"]["items"] = json!([{
        "item_instance_id": "blocking_ring",
        "position": "right_hand"
    }]);
    let mut engine = engine_from_value(occupied);
    let status = engine
        .validate_actor_command(&promotion_command("knight"))
        .expect("command validation should succeed structurally");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::OccupiedCarriedPosition)
    );
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PromoteClass("knight".into()),
        )
        .expect_err("occupied right hand must be rejected");
    assert!(error.to_string().contains("right hand must be empty"));
}

#[test]
fn promotion_requires_current_fighter_class_and_exact_configured_target() {
    let mut wrong_class = fixture_value();
    wrong_class.actors_mut()[0]["character"]["identity"]["current_class_id"] = json!("knight");
    wrong_class.actors_mut()[0]["character"]["identity"]["display_class"] = json!("Knight");
    let mut wrong_class_engine = engine_from_value(wrong_class);
    let error = wrong_class_engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PromoteClass("knight".into()),
        )
        .expect_err("current Knight must not promote from base Fighter identity");
    assert!(error.to_string().contains("current class \"fighter\""));

    let engine = engine();
    let status = engine
        .validate_actor_command(&promotion_command("paladin"))
        .expect("command validation should succeed structurally");
    assert!(!status.accepted);
    assert_eq!(
        status.blocked_reason,
        Some(ActionBlockedReasonV1::NoService)
    );
}

#[test]
fn promotion_requires_the_service_room_and_exact_coordinate() {
    let mut wrong_position = fixture_value();
    wrong_position.service_instances_mut()[0]["location"]["position"] = json!({"x": 1, "y": 2});
    let mut engine = engine_from_value(wrong_position);
    assert!(
        engine
            .actor_action_options(&tme_rules::ActorId::from("player"))
            .expect("action options")
            .iter()
            .all(|option| option.id != "promote_knight")
    );
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PromoteClass("knight".into()),
        )
        .expect_err("misplaced service must be rejected");
    assert!(error.to_string().contains("at the actor coordinate"));

    let mut wrong_room = fixture_value();
    let room = wrong_room.template_levels_source_mut()["room_0"].clone();
    wrong_room.template_levels_source_mut()["other"] = room;
    wrong_room.actors_mut()[0]["location"]["level"] = json!("other");
    let mut engine = engine_from_value(wrong_room);
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PromoteClass("knight".into()),
        )
        .expect_err("service in another room must be rejected");
    assert!(error.to_string().contains("at the actor coordinate"));
}

#[test]
fn action_option_and_direct_command_share_promotion_preflight() {
    let engine = engine();
    let option = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("action options")
        .into_iter()
        .find(|option| option.id == "promote_knight")
        .expect("promotion option should exist at the service coordinate");
    assert!(option.enabled);
    assert_eq!(option.blocked_reason, None);
    let command = option
        .command
        .expect("promotion option should carry a command");
    let status = engine
        .validate_actor_command(&command)
        .expect("command validation");
    assert!(status.accepted);
    assert_eq!(status.blocked_reason, None);

    let mut blocked = fixture_value();
    blocked.actors_mut()[0]["character"]["alignment_state"]["karma_points"] = json!(3);
    let engine = engine_from_value(blocked);
    let option = engine
        .actor_action_options(&tme_rules::ActorId::from("player"))
        .expect("action options")
        .into_iter()
        .find(|option| option.id == "promote_knight")
        .expect("blocked promotion option should remain visible");
    assert!(!option.enabled);
    assert_eq!(option.blocked_reason, Some(ActionBlockedReasonV1::NotReady));
    let status = engine
        .validate_actor_command(&option.command.unwrap())
        .expect("command validation");
    assert!(!status.accepted);
    assert_eq!(status.blocked_reason, option.blocked_reason);
}

#[test]
fn second_promotion_is_rejected_by_current_class_without_boolean_status() {
    let mut engine = engine();
    promote(&mut engine);
    let before = engine.world().clone();

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PromoteClass("knight".into()),
        )
        .expect_err("a current Knight must not promote again");

    assert!(error.to_string().contains("current class \"fighter\""));
    assert_eq!(engine.world(), &before);
    assert_eq!(player_character(&engine).promotion_history.len(), 1);
}

#[test]
fn knight_cast_requires_the_ring_on_a_finger_and_debits_exactly_three_mp() {
    let mut value = fixture_value();
    value.rules_source_mut()["resources"]["recovery_interval_units"] = json!(100);
    let mut engine = engine_from_value(value);
    promote(&mut engine);

    let right_hand_status = engine
        .validate_actor_command(&PlayerCommandV1 {
            contract_version: COMMAND_CONTRACT_VERSION,
            actor_id: "player".into(),
            intent: PlayerIntentPayloadV1::CastSpell {
                spell_id: "valor".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        })
        .expect("cast command validation");
    assert!(!right_hand_status.accepted);
    assert_eq!(
        right_hand_status.blocked_reason,
        Some(ActionBlockedReasonV1::MissingRequiredItem)
    );

    move_ring(&mut engine, CarriedPosition::LeftFinger1);
    let before_mp = player_character(&engine).resources.mp;
    let cast_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "valor".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect("finger-worn ring should power the Knight cast");
    assert_eq!(before_mp, 18);
    assert_eq!(player_character(&engine).resources.mp, 15);
    assert!(cast_events.events.iter().any(|event| matches!(
        event,
        Event::EffectApplied { effect_id, .. } if effect_id == "valor"
    )));
    assert!(
        !cast_events
            .events
            .iter()
            .any(|event| matches!(event, Event::SkillPracticeAwarded { .. }))
    );
    assert!(player_character(&engine).skill_ledger.is_empty());

    move_ring(&mut engine, CarriedPosition::SackItem1);
    let before = engine.world().clone();
    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "valor".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("removing the ring must disable the next cast");
    assert!(error.to_string().contains("missing_required_item"));
    assert_eq!(engine.world(), &before);
}

#[test]
fn insufficient_mp_blocks_a_ring_powered_cast_without_mutation() {
    let mut value = fixture_value();
    value.rules_source_mut()["resources"]["recovery_interval_units"] = json!(100);
    let mut engine = engine_from_value(value);
    promote(&mut engine);
    move_ring(&mut engine, CarriedPosition::RightFinger4);
    engine.world_mut().actors[0]
        .character
        .as_mut()
        .unwrap()
        .resources
        .mp = 2;
    engine.world_mut().actors[0].mp = 2;
    let before = engine.world().clone();

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::CastSpell {
                spell_id: "valor".to_string(),
                target: Some(SpellTarget::SelfTarget),
                authorization: tme_rules::HostilityAuthorization::Safe,
            },
        )
        .expect_err("two MP must not pay a three-MP cost");

    assert!(
        error.to_string().contains("insufficient_magic_points"),
        "unexpected error: {error}"
    );
    assert_eq!(engine.world(), &before);
}

#[test]
fn late_inventory_validation_failure_rolls_back_the_entire_promotion_step() {
    let mut engine = engine();
    let accepted = engine
        .validate_actor_command(&promotion_command("knight"))
        .expect("command validation");
    assert!(
        accepted.accepted,
        "preflight should initially accept promotion"
    );

    engine.world_mut().ground_items.push(GroundItem {
        item_instance_id: "corrupt_unregistered_location".to_string(),
        location: WorldPosition::new("realm_0", "room_0", Coord { x: 1, y: 1 }),
        loot_claim: None,
    });
    let before = engine.world().clone();

    let error = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PromoteClass("knight".into()),
        )
        .expect_err("late inventory validation should reject the transaction");

    assert!(
        error
            .to_string()
            .contains("item location references unknown instance")
    );
    assert_eq!(
        engine.world(),
        &before,
        "step rollback must restore all state"
    );
    assert_eq!(
        player_character(&engine).identity.current_class_id,
        "fighter"
    );
    assert!(player_character(&engine).known_spells.is_empty());
    assert!(player_character(&engine).promotion_history.is_empty());
    assert!(!engine.world().item_instances.contains_key(RING_INSTANCE_ID));
}
