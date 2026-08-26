use crate::support::content_parts::ContentParts;
use serde_json::json;
use tme_rules::{Engine, Event, PlayerIntent};

fn engine(parts: ContentParts, seed: u64) -> Engine {
    parts.engine(seed).expect("engine")
}

#[test]
fn snapshot_exposes_the_exact_right_hand_weapon_track_and_empty_hand_fallback() {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    let armed = engine(parts.clone(), 7).snapshot();
    let selected = armed
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .unwrap()
        .physical_weapon
        .as_ref()
        .expect("selected weapon view");
    assert_eq!(selected.item_instance_id.as_deref(), Some("training_knife"));
    assert_eq!(
        selected.item_definition_id.as_deref(),
        Some("training_knife")
    );
    assert_eq!(selected.skill_track_id, "dagger");
    assert_eq!(selected.skill_level, 0);
    assert_eq!(selected.combat_add_rating, 1);
    assert_eq!(selected.block_value, 0);
    assert_eq!(selected.nocking_unloads_on_movement, None);
    assert_eq!(selected.required_alignment, None);
    assert!(selected.binding_usable);
    assert!(selected.alignment_usable);
    assert!(selected.restriction_usable);

    parts.actors_mut()[0]["carried"]["items"] = json!([]);
    *parts.ground_items_mut() = json!([{
        "item_instance_id": "training_knife",
        "location": {
            "realm": "realm_0", "level": "room_0", "position": {"x": 1, "y": 1}
        }
    }]);
    let unarmed = engine(parts, 7).snapshot();
    let selected = unarmed
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .unwrap()
        .physical_weapon
        .as_ref()
        .expect("unarmed selection view");
    assert_eq!(selected.item_instance_id, None);
    assert_eq!(selected.item_definition_id, None);
    assert_eq!(selected.skill_track_id, "hand");
    assert_eq!(selected.skill_level, 0);
}

#[test]
fn unrelated_high_skill_does_not_change_selected_weapon_hit_score() {
    fn score(hand_level: u8) -> i32 {
        let mut parts = ContentParts::tracked("skill_progression", "profile/skill_progression");
        let player_position = parts.actors_mut()[0]["location"]["position"].clone();
        parts.actors_mut()[1]["location"]["position"] = player_position;
        parts.actor_definition_mut(1)["stats"]["defense"] = json!(100);
        let skill_catalog = parts.skill_catalog_mut().expect("skill catalog");
        let mut hand_track = skill_catalog["tracks"][0].clone();
        hand_track["id"] = json!("hand");
        hand_track["display"] = json!("Hand");
        hand_track["kind"] = json!("martial_arts");
        skill_catalog["tracks"]
            .as_array_mut()
            .unwrap()
            .push(hand_track);
        parts.actors_mut()[0]["character"]["skill_ledger"] = json!([
            {"track_id":"sword","level":0,"critique_rank":0,"practice_points":0,"learning_rate":1},
            {"track_id":"hand","level":hand_level,"critique_rank":0,"practice_points":0,"learning_rate":1}
        ]);
        let events = engine(parts, 7)
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::PhysicalAttack {
                    authorization: tme_rules::HostilityAuthorization::Safe,
                    mode: tme_rules::PhysicalAttackMode::Fight,
                    target_actor_id: "mireling".into(),
                },
            )
            .expect("attack");
        events
            .events
            .iter()
            .find_map(|event| match event {
                Event::AttackMissed { attacker_score, .. } => Some(*attacker_score),
                _ => None,
            })
            .expect("forced miss")
    }
    assert_eq!(score(0), score(19));
}

#[test]
fn two_handed_selection_reports_offhand_loss_without_inventing_a_penalty() {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.selected_mut("items", 0)["weapon"]["handedness"] = json!("two_handed");
    parts.push_selected(
        "items",
        "item/offhand_token/test",
        json!({
            "id":"offhand_token","kind":"gear","name":"Offhand Token",
            "valid_placements":["hand"],"economy":{"unit_burden":0}
        }),
    );
    parts.item_instances_mut()["offhand_token"] = json!({
        "definition_id":"offhand_token","binding":{"state":"unrestricted"}
    });
    parts.actors_mut()[0]["carried"]["items"]
        .as_array_mut()
        .unwrap()
        .push(json!({"item_instance_id":"offhand_token","position":"left_hand"}));
    let snapshot = engine(parts, 7).snapshot();
    let selected = snapshot
        .actors
        .iter()
        .find(|actor| actor.id == "player")
        .unwrap()
        .physical_weapon
        .as_ref()
        .unwrap();
    assert!(selected.offhand_occupied);
    assert!(!selected.full_two_handed_effect);
}

#[test]
fn right_hand_nonweapon_rejects_physical_attack() {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    let item = parts.selected_mut("items", 0);
    item["kind"] = json!("gear");
    item.as_object_mut().unwrap().remove("weapon");
    let player_position = parts.actors_mut()[0]["location"]["position"].clone();
    parts.actors_mut()[1]["location"]["position"] = player_position;
    let error = engine(parts, 7)
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
            },
        )
        .expect_err("nonweapon right hand must reject");
    assert!(error.message().contains("is not a weapon"));
}
