use super::*;

#[test]
fn hide_events_serialize_with_contract_fields() {
    let mut engine = ContentParts::tracked(
        "profession_specific_actions",
        "profile/profession_specific_actions",
    )
    .engine(7)
    .expect("start");
    engine
        .world_mut()
        .tile_effects
        .push(tme_rules::model::TileEffectState {
            source_actor_id: None,
            instance_id: "tile:shadow:1".to_string(),
            effect_id: "shadow_veil".to_string(),
            source: tme_rules::model::ActiveEffectSource {
                kind: "spell".to_string(),
                id: "shadow_veil".to_string(),
            },
            location: WorldPosition::new("realm_0", "room_0", Coord { x: 2, y: 2 }),
            kind: "terrain_overlay".to_string(),
            tags: vec!["shadow".to_string()],
            potency: 0,
            remaining_rounds: Some(3),
            passability: None,
            sight: Some("obscured".to_string()),
            hazard: None,
            move_cost: None,
            tick_interval_rounds: 1,
            last_ticked_at: tme_rules::LogicalTime::new(0),
            hostile_authority: None,
        });

    let hide_events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player0"), PlayerIntent::Hide)
        .expect("hide");
    let move_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player0"),
            PlayerIntent::MovePath(vec![Direction::East]),
        )
        .expect("move");
    let hidden_serialized = serde_json::to_value(&hide_events.events).expect("serialize hide");
    let hidden_entries = hidden_serialized.as_array().expect("hide event array");

    let hidden = hidden_entries
        .iter()
        .find(|entry| entry.get("actor_hidden").is_some())
        .expect("actor_hidden event");
    let hidden_obj = &hidden["actor_hidden"];
    assert!(hidden_obj["actor_id"].is_string());
    assert!(hidden_obj["actor"].is_string());
    assert!(hidden_obj["location"].is_object());
    assert!(hidden_obj["instance_id"].is_string());
    assert!(hidden_obj["effect_id"].is_string());
    assert!(hidden_obj["remaining_rounds"].is_number() || hidden_obj["remaining_rounds"].is_null());

    let broken_serialized = serde_json::to_value(&move_events.events).expect("serialize move");
    let broken_entries = broken_serialized.as_array().expect("move event array");
    let broken = broken_entries
        .iter()
        .find(|entry| entry.get("hide_broken").is_some())
        .expect("hide_broken event");
    let broken_obj = &broken["hide_broken"];
    assert!(broken_obj["actor_id"].is_string());
    assert!(broken_obj["actor"].is_string());
    assert!(broken_obj["location"].is_object());
    assert!(broken_obj["instance_id"].is_string());
    assert!(broken_obj["effect_id"].is_string());
    assert!(broken_obj["reason"].is_string());
}

#[test]
fn martial_hand_block_events_serialize_with_contract_fields() {
    let mut engine = ContentParts::tracked(
        "martial_hand_block_actions",
        "profile/martial_hand_block_actions",
    )
    .engine(7)
    .expect("start");
    engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player0"),
            PlayerIntent::MovePath(vec![Direction::South]),
        )
        .expect("engage");

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player0"), PlayerIntent::Wait)
        .expect("wait");
    let serialized = serde_json::to_value(&events.events).expect("serialize");
    let entries = serialized.as_array().expect("event array");
    let blocked = entries
        .iter()
        .find(|entry| entry.get("attack_blocked").is_some())
        .expect("typed attack_blocked event");
    let blocked_obj = &blocked["attack_blocked"];
    assert!(blocked_obj["attacker_id"].is_string());
    assert!(blocked_obj["attacker"].is_string());
    assert!(blocked_obj["defender_id"].is_string());
    assert!(blocked_obj["defender"].is_string());
    assert!(blocked_obj["defender_location"].is_object());
    assert_eq!(blocked_obj["source"], "right_martial_hand");
    assert_eq!(blocked_obj["carried_position"], "right_hand");
    assert!(blocked_obj["item_instance_id"].is_null());
    assert_eq!(blocked_obj["block_value"], 0);
    assert_eq!(blocked_obj["skill_track_id"], "hand");
    assert_eq!(blocked_obj["skill_level"], 19);
    assert!(blocked_obj["roll"].is_number());
    assert!(blocked_obj["chance_percent"].is_number());
}

#[test]
fn duplicate_display_names_use_ids_for_disambiguation() {
    let mut parts = ContentParts::tracked("first_room", "profile/first_room");
    parts.actor_definition_mut(1)["name"] = json!("Guard");
    let actors = parts
        .actors_mut()
        .as_array_mut()
        .expect("first-room actors");
    let mut guard_a = actors[1].clone();
    guard_a["id"] = json!("guard_a");
    guard_a["location"]["position"] = json!({"x": 1, "y": 2});
    let mut guard_b = guard_a.clone();
    guard_b["id"] = json!("guard_b");
    guard_b["location"]["position"] = json!({"x": 3, "y": 1});
    actors[1] = guard_a;
    actors.push(guard_b);
    let mut engine = parts.engine(7).expect("start");

    let events = engine
        .apply_actor_intent(&tme_rules::ActorId::from("player"), PlayerIntent::Wait)
        .expect("wait");
    // Both Guards appear in events with distinct IDs
    let guard_events: Vec<_> = events
        .events
        .iter()
        .filter(|e| matches!(e, Event::AutomaticActorDecision { actor, .. } if actor == "Guard"))
        .collect();
    assert!(!guard_events.is_empty(), "at least one Guard should act");
    // Action context should list both with distinct actor_ids
    let ctx = engine
        .actor_action_context(&tme_rules::ActorId::from("player"))
        .expect("ctx");
    let guards: Vec<_> = ctx
        .attack_targets
        .iter()
        .filter(|t| t.actor_name == "Guard")
        .collect();
    assert_eq!(guards.len(), 2, "both Guards should appear");
    assert_ne!(
        guards[0].actor_id, guards[1].actor_id,
        "Guards must have distinct IDs"
    );
}

#[test]
fn event_34_preserves_death_corpse_claim_and_search_payloads() {
    assert_eq!(EVENT_CONTRACT_VERSION, 41);
    let mut engine = ContentParts::tracked("death_corpse", "profile/death_corpse")
        .engine(7)
        .expect("death gallery starts");
    let defeat_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                authorization: tme_rules::HostilityAuthorization::Safe,
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "scavenger".into(),
            },
        )
        .expect("monster defeat");

    let defeated = event_payload(&defeat_events.events, "actor_defeated");
    assert_eq!(
        defeated,
        json!({
            "actor_id": "scavenger",
            "actor": "Courtyard Scavenger",
            "kind": "monster",
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 1, "y": 1}
            },
            "cause": "physical",
            "credited_actor_id": "player",
            "loot_claim": {
                "owner": {
                    "kind": "character",
                    "id": "character:death_corpse:primary"
                },
                "basis": "killing_blow"
            }
        })
    );
    let created = event_payload(&defeat_events.events, "corpse_created");
    assert_eq!(created["corpse_id"], "corpse:1");
    assert_eq!(created["origin_actor_id"], "scavenger");
    assert_eq!(created["origin_character_id"], Value::Null);
    assert_eq!(created["origin_kind"], "monster");
    assert_eq!(created["origin_name"], "Courtyard Scavenger");
    assert_eq!(created["sequence"], 1);
    assert_eq!(created["created_at"], json!({"milliseconds": 3000}));
    assert!(created.get("contents").is_none());
    assert!(created.get("sack_gold").is_none());

    let retained = defeat_events
        .events
        .iter()
        .find(|event| {
            matches!(
                event,
                Event::ItemRelocated {
                    item_instance_id,
                    reason: tme_rules::ItemRelocationReason::CorpseRetention,
                    ..
                } if item_instance_id == "cloth_bundle"
            )
        })
        .expect("corpse retention event");
    let retained = serde_json::to_value(retained).unwrap();
    assert_eq!(retained["item_relocated"]["to"]["kind"], "corpse");
    assert_eq!(retained["item_relocated"]["to"]["corpse_id"], "corpse:1");
    assert_eq!(
        retained["item_relocated"]["loot_claim"]["basis"],
        "killing_blow"
    );

    let search_events = engine
        .apply_actor_intent(
            &tme_rules::ActorId::from("player"),
            PlayerIntent::SearchCorpse(CorpseId::parse("corpse:1").unwrap()),
        )
        .expect("corpse search");
    let searched = event_payload(&search_events.events, "corpse_searched");
    assert_eq!(
        searched,
        json!({
            "corpse_id": "corpse:1",
            "actor_id": "player",
            "actor": "Wayfarer",
            "location": {
                "realm": "realm_0",
                "level": "room_0",
                "position": {"x": 1, "y": 1}
            },
            "items_released": 1,
            "gold_released": 3
        })
    );
    let gold = event_payload(&search_events.events, "gold_relocated");
    assert_eq!(gold["amount"], 3);
    assert_eq!(
        gold["from"],
        json!({"kind": "corpse", "corpse_id": "corpse:1"})
    );
    assert_eq!(gold["to"]["kind"], "ground");
    assert_eq!(gold["to"]["gold_pile_id"], "gold:1");
    assert_eq!(gold["reason"], "corpse_search");

    assert!(
        serde_json::from_value::<Event>(json!({
            "died": {"actor_id": "obsolete", "actor": "Obsolete"}
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<Event>(json!({
            "player_recovered": {"actor_id": "obsolete", "actor": "Obsolete"}
        }))
        .is_err()
    );
}

#[test]
fn event_25_automatic_decisions_reject_unknown_fields_and_variants() {
    let valid = json!({
        "kind": "move",
        "direction": "west",
        "purpose": "chase"
    });
    serde_json::from_value::<tme_rules::AutomaticActorDecisionV1>(valid.clone())
        .expect("current decision parses");

    let mut extra = valid.clone();
    extra["summary"] = json!("legacy prose");
    assert!(serde_json::from_value::<tme_rules::AutomaticActorDecisionV1>(extra).is_err());

    let mut unknown = valid;
    unknown["kind"] = json!("wander");
    assert!(serde_json::from_value::<tme_rules::AutomaticActorDecisionV1>(unknown).is_err());
}
