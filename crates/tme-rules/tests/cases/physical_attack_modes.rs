use crate::support::content_parts::ContentParts;
use serde_json::json;
use tme_rules::{
    ActionBlockedReasonV1, Engine, Event, PhysicalAttackMode, PhysicalDamageKind, PlayerCommandV1,
    PlayerIntent,
};

const FIRST_ROOM: (&str, &str) = ("first_room", "profile/first_room");
const SKILL_PROGRESSION: (&str, &str) = ("skill_progression", "profile/skill_progression");

fn fixture((case_id, profile): (&str, &str)) -> ContentParts {
    ContentParts::tracked(case_id, profile)
}

fn engine(parts: ContentParts, seed: u64) -> Engine {
    parts.engine(seed).expect("engine should start")
}

fn target_options(engine: &Engine) -> Vec<tme_rules::view::PhysicalAttackOptionV1> {
    engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("action context")
        .attack_targets
        .into_iter()
        .find(|target| target.actor_id == "mireling")
        .expect("visible hostile target")
        .physical_attacks
}

#[test]
fn command_26_round_trips_each_explicit_mode_and_rejects_generic_attack() {
    let engine = engine(fixture(FIRST_ROOM), 7);
    for mode in PhysicalAttackMode::ALL {
        let intent = PlayerIntent::PhysicalAttack {
            authorization: tme_rules::HostilityAuthorization::Safe,
            mode,
            target_actor_id: "mireling".into(),
        };
        let command = engine
            .actor_command_for_intent(&tme_rules::ActorId::from("player"), &intent)
            .expect("command should project");
        let json = serde_json::to_value(&command).expect("command should serialize");
        assert_eq!(json["contract_version"], 26);
        assert_eq!(json["intent"]["physical_attack"]["mode"], mode.label());
        assert_eq!(
            json["intent"]["physical_attack"]["target_actor_id"],
            "mireling"
        );
        assert_eq!(
            serde_json::from_value::<PlayerCommandV1>(json)
                .expect("command JSON should round trip"),
            command
        );
    }

    let old_action = ["att", "ack"].concat();
    let stale = json!({
        "contract_version": 15,
        "actor_id": "player",
        "intent": {old_action: "mireling"}
    });
    assert!(serde_json::from_value::<PlayerCommandV1>(stale).is_err());
}

#[test]
fn blocked_resolved_modes_keep_metadata_in_canonical_action_order() {
    let engine = engine(fixture(FIRST_ROOM), 7);
    let before = engine.snapshot();
    let options = target_options(&engine);
    assert_eq!(
        options.iter().map(|option| option.mode).collect::<Vec<_>>(),
        PhysicalAttackMode::ALL
    );

    let fight = &options[0];
    assert_eq!(
        fight.blocked_reason,
        Some(ActionBlockedReasonV1::NotEngaged)
    );
    assert_eq!(fight.maximum_range, Some(0));
    assert_eq!(fight.damage_kind, Some(PhysicalDamageKind::Cutting));
    assert_eq!(fight.skill_track_id.as_deref(), Some("dagger"));

    let jumpkick = &options[2];
    assert_eq!(
        jumpkick.blocked_reason,
        Some(ActionBlockedReasonV1::OutOfRange)
    );
    assert_eq!(jumpkick.maximum_range, Some(1));
    assert_eq!(jumpkick.damage_kind, Some(PhysicalDamageKind::Crushing));
    assert_eq!(jumpkick.skill_track_id.as_deref(), Some("hand"));
    assert_eq!(jumpkick.selected_item_instance_id, None);

    assert_eq!(
        engine.snapshot(),
        before,
        "action planning must be read-only"
    );
}

#[test]
fn kick_uses_hand_independently_of_the_selected_right_hand_weapon() {
    let mut parts = fixture(FIRST_ROOM);
    let player_position = parts.actors_mut()[0]["location"]["position"].clone();
    parts.actors_mut()[1]["location"]["position"] = player_position;
    parts.actor_definition_mut(1)["ai"]["behavior"] = json!("hold_ground");
    let mut engine = engine(parts, 7);
    let kick = target_options(&engine)
        .into_iter()
        .find(|option| option.mode == PhysicalAttackMode::Kick)
        .unwrap();
    assert!(kick.enabled);
    assert_eq!(kick.skill_track_id.as_deref(), Some("hand"));
    assert_eq!(kick.selected_item_instance_id, None);
    assert!(kick.barefoot_full_effect);

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: PhysicalAttackMode::Kick,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("kick should resolve");
    assert!(events.events.iter().any(|event| matches!(
        event,
        Event::Attacked {
            mode: PhysicalAttackMode::Kick,
            damage_kind: PhysicalDamageKind::Crushing,
            ..
        } | Event::AttackMissed {
            mode: PhysicalAttackMode::Kick,
            damage_kind: PhysicalDamageKind::Crushing,
            ..
        }
    )));
}

#[test]
fn jumpkick_spends_authored_stamina_once_and_preserves_weapon_selection() {
    let mut parts = fixture(SKILL_PROGRESSION);
    parts.actors_mut()[1]["location"]["position"] = json!({"x": 2, "y": 1});
    parts.actor_definition_mut(1)["ai"]["behavior"] = json!("hold_ground");
    let mut engine = engine(parts, 7);
    let stamina_before = engine.world().actors[0].stamina;
    let right_hand_before = engine
        .snapshot()
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .unwrap()
        .physical_weapon
        .as_ref()
        .unwrap()
        .item_instance_id
        .clone();

    let events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: PhysicalAttackMode::Jumpkick,
                target_actor_id: "mireling".into(),
            },
        )
        .expect("jumpkick should resolve");
    assert_eq!(engine.world().actors[0].stamina, stamina_before - 1);
    assert_eq!(
        events
            .events
            .iter()
            .filter(|event| matches!(event, Event::PhysicalStaminaSpent { .. }))
            .count(),
        1
    );
    let right_hand_after = engine
        .snapshot()
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .unwrap()
        .physical_weapon
        .as_ref()
        .unwrap()
        .item_instance_id
        .clone();
    assert_eq!(right_hand_after, right_hand_before);
}
