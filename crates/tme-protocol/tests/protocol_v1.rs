use tme_protocol::{
    ActorId, ActorKind, AdmissionTicket, AttackSafety, Burden, CONTROL_API_VERSION, CarriedGold,
    CarriedLayout, CharacterAlignment, CharacterAttributes, CharacterId, CharacterIdentity,
    CharacterProgression, CharacterResources, CharacterSummaryV1, ClientCommandEnvelope, CommandId,
    ControlledCharacter, Coord, DecimalI64, DecimalU64, GoldMoveQuantity, Intent, LifeState,
    MAX_INPUT_BYTES, MAX_JSON_NESTING, MAX_SERVER_ENVELOPE_BYTES, MAX_SOCIAL_BODY_BYTES,
    MAX_SOCIAL_BODY_SCALARS, MAX_STATIC_SCENE_TILES, ObserverActor, ObserverFrame,
    ObserverGoldPile, ObserverTile, PROTOCOL_MINOR, PhysicalAttributeAdds, Position,
    PresentationMode, ServerEnvelope, SessionBootstrapV1, SocialBody, SocialScope, SocialView,
    StaticSceneBounds, StaticSceneContext, StaticSceneRole, StaticSceneSite, StaticSceneTile,
    WireLabel, decode_client_command, decode_client_hello, decode_login_request,
    encode_server_envelope,
};

const TICKET: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
const COMMAND: &str = "018f0f9f-9b5a-7c61-8d2d-5ab82b1c3d4e";
const FACET: &str = "018f0f9f-9b5a-7c61-8d2d-5ab82b1c3d4f";
const CHARACTER: &str = "018f0f9f-9b5a-7c61-8d2d-5ab82b1c3d50";

fn position() -> Position {
    Position {
        realm: WireLabel::new("realm_0").unwrap(),
        level: WireLabel::new("room_0").unwrap(),
        position: Coord { x: 1, y: 1 },
    }
}

fn character_id() -> CharacterId {
    serde_json::from_str(&format!(r#""{CHARACTER}""#)).unwrap()
}

fn observer_frame(sack_gold: i64, pile_amounts: &[i64]) -> ObserverFrame {
    ObserverFrame {
        contract_version: 8,
        logical_time: DecimalU64::new(0),
        ready_at: DecimalU64::new(0),
        observer_actor_id: ActorId::new("player").unwrap(),
        observation_center: position(),
        observation_radius: 7,
        can_act: true,
        tiles: Vec::new(),
        actors: Vec::new(),
        corpses: Vec::new(),
        corpses_truncated: false,
        ground_items: Vec::new(),
        ground_items_truncated: false,
        gold_piles: pile_amounts
            .iter()
            .enumerate()
            .map(|(index, amount)| ObserverGoldPile {
                gold_pile_id: WireLabel::new(format!("gold:{}", index + 1)).unwrap(),
                amount: DecimalI64::new(*amount),
                location: position(),
                loot_claim: None,
            })
            .collect(),
        gold_piles_truncated: false,
        character: ControlledCharacter {
            identity: CharacterIdentity {
                base_class_id: WireLabel::new("fighter").unwrap(),
                current_class_id: WireLabel::new("fighter").unwrap(),
                display_class: WireLabel::new("Fighter").unwrap(),
                nationality_id: WireLabel::new("default").unwrap(),
                sex_or_gender_display: None,
            },
            alignment: CharacterAlignment::Neutral,
            karma_points: 0,
            attributes: CharacterAttributes {
                strength: 10,
                dexterity: 10,
                constitution: 10,
                intelligence: 10,
                wisdom: 10,
                charisma: 10,
            },
            resources: CharacterResources {
                hp: 10,
                max_hp: 10,
                peak_hp: 10,
                mp: 0,
                max_mp: 0,
                stamina: 10,
                max_stamina: 10,
            },
            progression: CharacterProgression {
                level: 3,
                experience: DecimalI64::new(1600),
                pending_target_level: None,
            },
            physical_attribute_adds: PhysicalAttributeAdds {
                strength_adds: 0,
                dexterity_adds: 0,
            },
            promotion_history: Vec::new(),
            known_spells: Vec::new(),
            skill_ledger: Vec::new(),
        },
        carried: CarriedLayout {
            items: Vec::new(),
            gold: CarriedGold {
                left_hand: DecimalI64::new(0),
                right_hand: DecimalI64::new(0),
                sack: DecimalI64::new(sack_gold),
            },
        },
        burden: Burden {
            item_burden: DecimalU64::new(0),
            coin_burden: DecimalU64::new(0),
            total_burden: DecimalU64::new(0),
            lightly_loaded_limit: None,
            moderately_loaded_limit: None,
            heavily_loaded_limit: None,
            tier: None,
        },
        warmed_spell: None,
        spell_actions: Vec::new(),
        services_here: Vec::new(),
        npcs_here: Vec::new(),
        quest_log: Vec::new(),
        action_options: Vec::new(),
        action_options_truncated: false,
        social: SocialView {
            character_id: character_id(),
            group: None,
            incoming_invitations: Vec::new(),
            outgoing_invitations: Vec::new(),
            following_character_id: None,
            pages_enabled: true,
            blocked_character_ids: Vec::new(),
        },
        incoming_item_offers: Vec::new(),
        outgoing_item_offers: Vec::new(),
    }
}

fn full_r7_static_scene_context() -> StaticSceneContext {
    let tiles = (-7..=7)
        .flat_map(|y| {
            (-7..=7).map(move |x| StaticSceneTile {
                position: Coord { x, y },
                terrain_ids: vec![WireLabel::new("dungeon_floor").unwrap()],
                walkable: true,
            })
        })
        .collect::<Vec<_>>();
    StaticSceneContext {
        contract_version: 1,
        site: StaticSceneSite {
            realm: WireLabel::new("realm_0").unwrap(),
            level: WireLabel::new("room_0").unwrap(),
        },
        bounds: StaticSceneBounds {
            min: Coord { x: -7, y: -7 },
            max: Coord { x: 7, y: 7 },
        },
        content_digest: WireLabel::new("a".repeat(64)).unwrap(),
        visual_manifest_digest: WireLabel::new("b".repeat(64)).unwrap(),
        scene_role: StaticSceneRole::CombatSpace,
        presentation_mode: PresentationMode::CombatSpace,
        world_zoom: [156, 104],
        walkable_mask: tiles.iter().map(|tile| tile.position.clone()).collect(),
        tiles,
        static_props: Vec::new(),
        transition_apertures: Vec::new(),
    }
}

fn observer_gold_pile_json(amount: &str) -> String {
    format!(
        r#"{{"gold_pile_id":"gold:1","amount":{amount},"location":{{"realm":"realm_0","level":"room_0","position":{{"x":1,"y":1}}}},"loot_claim":null}}"#
    )
}

fn observer_frame_json(sack_gold: &str) -> String {
    format!(
        r#"{{"contract_version":5,"logical_time":"0","ready_at":"0","observer_actor_id":"player","observation_center":{{"realm":"realm_0","level":"room_0","position":{{"x":1,"y":1}}}},"observation_radius":7,"can_act":true,"tiles":[],"actors":[],"corpses":[],"corpses_truncated":false,"ground_items":[],"ground_items_truncated":false,"gold_piles":[],"gold_piles_truncated":false,"character":{{"identity":{{"base_class_id":"fighter","current_class_id":"fighter","display_class":"Fighter","nationality_id":"default","sex_or_gender_display":null}},"alignment":"neutral","karma_points":0,"attributes":{{"strength":10,"dexterity":10,"constitution":10,"intelligence":10,"wisdom":10,"charisma":10}},"resources":{{"hp":10,"max_hp":10,"peak_hp":10,"mp":0,"max_mp":0,"stamina":10,"max_stamina":10}},"progression":{{"level":3,"experience":"1600","pending_target_level":null}},"physical_attribute_adds":{{"strength_adds":0,"dexterity_adds":0}},"promotion_history":[],"known_spells":[],"skill_ledger":[]}},"carried":{{"items":[],"gold":{{"left_hand":"0","right_hand":"0","sack":{sack_gold}}}}},"burden":{{"item_burden":"0","coin_burden":"0","total_burden":"0","lightly_loaded_limit":null,"moderately_loaded_limit":null,"heavily_loaded_limit":null,"tier":null}},"warmed_spell":null,"spell_actions":[],"services_here":[],"npcs_here":[],"quest_log":[],"action_options":[],"action_options_truncated":false,"social":{{"character_id":"{CHARACTER}","group":null,"incoming_invitations":[],"outgoing_invitations":[],"following_character_id":null,"pages_enabled":true,"blocked_character_ids":[]}},"incoming_item_offers":[],"outgoing_item_offers":[]}}"#
    )
}

fn move_gold_command(amount: &str) -> String {
    format!(
        r#"{{"kind":"command","command_id":"{COMMAND}","control_epoch":"1","client_sequence":"1","observed_world_revision":"0","actor_id":"player","intent":{{"kind":"move_gold","source":{{"kind":"carried","position":"sack"}},"destination":{{"kind":"ground_here"}},"quantity":{{"kind":"exact","amount":{amount}}}}}}}"#
    )
}

fn bootstrap_json() -> serde_json::Value {
    serde_json::json!({
        "control_api_version": CONTROL_API_VERSION,
        "account": {
            "account_id": COMMAND,
            "display_name": "Account"
        },
        "session": {
            "session_id": CHARACTER,
            "idle_timeout_seconds": "60",
            "absolute_timeout_seconds": "120"
        },
        "csrf_token": TICKET,
        "characters": [],
        "selected_character_id": null,
        "player_kill_marks": {
            "active_count": 0,
            "gameplay_locked": false,
            "active_marks": [],
            "forgivable_marks": []
        }
    })
}

fn feedback_envelope(cue: serde_json::Value) -> ServerEnvelope {
    serde_json::from_value(serde_json::json!({
        "kind": "command_result",
        "command_id": COMMAND,
        "disposition": {"kind": "accepted"},
        "replay_status": "new",
        "server_sequence": "1",
        "before_revision": "1",
        "after_revision": "2",
        "events": [{"kind": "feedback", "cue": cue}],
        "events_truncated": false
    }))
    .expect("strict feedback envelope")
}

#[test]
fn hello_is_exact_strict_and_never_exposes_ticket_in_debug() {
    let hello = format!(r#"{{"kind":"client_hello","ticket":"{TICKET}","supported_minors":[8]}}"#);
    let decoded = decode_client_hello(hello.as_bytes()).expect("hello");
    assert!(!format!("{decoded:?}").contains(TICKET));
    assert!(decode_client_hello(format!("{hello} ").as_bytes()).is_ok());
    assert!(decode_client_hello(format!("{hello} true").as_bytes()).is_err());
    assert!(decode_client_hello(
        format!(r#"{{"kind":"client_hello","ticket":"{TICKET}","ticket":"{TICKET}","supported_minors":[8]}}"#).as_bytes()
    ).is_err());
    assert!(
        decode_client_hello(
            format!(r#"{{"kind":"client_hello","ticket":"{TICKET}","supported_minors":[8,8]}}"#)
                .as_bytes()
        )
        .is_err()
    );
}

#[test]
fn full_r7_static_context_and_frame_fit_the_strict_wire_budget() {
    let context = full_r7_static_scene_context();
    assert_eq!(context.tiles.len(), MAX_STATIC_SCENE_TILES);
    let mut frame = observer_frame(0, &[]);
    frame.tiles = (-7..=7)
        .flat_map(|y| {
            (-7..=7).map(move |x| ObserverTile {
                position: Coord { x, y },
                terrain_id: Some(WireLabel::new("dungeon_floor").unwrap()),
                terrain_name: Some(WireLabel::new("Dungeon Floor").unwrap()),
                passable: Some(true),
                move_cost: Some(1),
                transition: None,
            })
        })
        .collect();
    frame.actors = (0..32)
        .map(|index| ObserverActor {
            actor_id: ActorId::new(format!("actor:{index}")).unwrap(),
            character_id: None,
            name: WireLabel::new(format!("Observed Actor {index}")).unwrap(),
            kind: ActorKind::Monster,
            position: Position {
                realm: WireLabel::new("realm_0").unwrap(),
                level: WireLabel::new("room_0").unwrap(),
                position: Coord {
                    x: index % 15 - 7,
                    y: index / 15 - 7,
                },
            },
            life_state: LifeState::Alive,
            hp: 20,
            max_hp: 20,
            attack_safety: AttackSafety::OpenHostile,
        })
        .collect();
    let envelope = ServerEnvelope::StateUpdate {
        server_sequence: DecimalU64::new(1),
        world_revision: DecimalU64::new(1),
        events: Vec::new(),
        events_truncated: false,
        static_scene_context: context,
        frame,
    };
    let encoded = encode_server_envelope(&envelope).expect("maximum R7 envelope");
    println!("R7_STATIC_CONTEXT_ENCODED_BYTES={}", encoded.len());
    assert!(encoded.len() < MAX_SERVER_ENVELOPE_BYTES);
}

#[test]
fn static_context_rejects_overflow_and_malformed_walkability() {
    let mut overflow = full_r7_static_scene_context();
    overflow.tiles.push(StaticSceneTile {
        position: Coord { x: 8, y: 7 },
        terrain_ids: vec![WireLabel::new("dungeon_floor").unwrap()],
        walkable: true,
    });
    assert!(overflow.validate().is_err());

    let mut mismatched_mask = full_r7_static_scene_context();
    mismatched_mask.walkable_mask.pop();
    assert!(mismatched_mask.validate().is_err());
}

#[test]
fn command_supports_bounded_move_path_and_protocol_1_8_revision_shape() {
    let command = format!(
        r#"{{"kind":"command","command_id":"{COMMAND}","control_epoch":"1","client_sequence":"1","observed_world_revision":"0","actor_id":"player","intent":{{"kind":"move_path","path":["east"]}}}}"#
    );
    let decoded = decode_client_command(command.as_bytes(), PROTOCOL_MINOR).expect("command");
    assert!(matches!(
        decoded,
        ClientCommandEnvelope::Command {
            intent: Intent::MovePath { .. },
            ..
        }
    ));
    assert!(
        decode_client_command(
            command.replace("[\"east\"]", "[]").as_bytes(),
            PROTOCOL_MINOR
        )
        .is_err()
    );
    assert!(
        decode_client_command(
            command
                .replace("[\"east\"]", "[\"east\",\"east\",\"east\",\"east\"]")
                .as_bytes(),
            PROTOCOL_MINOR
        )
        .is_err()
    );
    assert!(decode_client_command(command.as_bytes(), 3).is_err());
    assert!(decode_client_command(command.as_bytes(), 0).is_err());
}

#[test]
fn protocol_1_8_retains_typed_traversal_door_inspect_and_preview_reads() {
    for intent in [
        r#"{"kind":"traverse","traversal":"stairs_up"}"#,
        r#"{"kind":"open","direction":"east"}"#,
        r#"{"kind":"close","direction":"west"}"#,
        r#"{"kind":"inspect"}"#,
    ] {
        let command = format!(
            r#"{{"kind":"command","command_id":"{COMMAND}","control_epoch":"1","client_sequence":"1","observed_world_revision":"0","actor_id":"player","intent":{intent}}}"#
        );
        assert!(decode_client_command(command.as_bytes(), PROTOCOL_MINOR).is_ok());
        assert!(decode_client_command(command.as_bytes(), 6).is_err());
    }

    let preview = format!(
        r#"{{"kind":"path_preview","preview_id":"{COMMAND}","control_epoch":"1","observed_world_revision":"0","actor_id":"player","path":["north","east"]}}"#
    );
    assert!(matches!(
        decode_client_command(preview.as_bytes(), PROTOCOL_MINOR).unwrap(),
        ClientCommandEnvelope::PathPreview { path, .. } if path.len() == 2
    ));
    assert!(decode_client_command(preview.as_bytes(), 5).is_err());
    assert!(
        decode_client_command(
            preview.replace("[\"north\",\"east\"]", "[]").as_bytes(),
            PROTOCOL_MINOR,
        )
        .is_err()
    );
}

#[test]
fn path_preview_result_requires_a_valid_path_8_payload_exactly_on_success() {
    let preview = serde_json::json!({
        "kind": "path_preview_result",
        "preview_id": COMMAND,
        "disposition": {"kind": "previewed"},
        "control_epoch": "1",
        "actor_id": "player",
        "world_revision": "2",
        "preview": {
            "contract_version": 8,
            "actor_id": "player",
            "start": position(),
            "pace": "walk",
            "requested_path": ["north"],
            "available_path_points": 8,
            "accepted_steps": "1",
            "steps": [{
                "index": "0",
                "direction": "north",
                "from": position(),
                "attempted": {"realm":"realm_0","level":"room_0","position":{"x":1,"y":0}},
                "opens_door": false,
                "terrain_name": "Stone",
                "cost": 1,
                "remaining_points_after": 7,
                "outcome": {"kind":"moved","navigation":"walk"}
            }],
            "stop_reason": "full_path_accepted",
            "final_position": {"realm":"realm_0","level":"room_0","position":{"x":1,"y":0}},
            "remaining_path_points": 7,
            "burden": {
                "item_burden": "0",
                "coin_burden": "0",
                "total_burden": "0",
                "lightly_loaded_limit": null,
                "moderately_loaded_limit": null,
                "heavily_loaded_limit": null,
                "tier": null
            },
            "movement_exertion": "none",
            "stamina_before": null,
            "stamina_cost": null,
            "stamina_after": null
        }
    });
    let envelope: ServerEnvelope = serde_json::from_value(preview.clone()).unwrap();
    envelope.validate().unwrap();

    let mut mismatch = preview.clone();
    mismatch["preview"] = serde_json::Value::Null;
    let mismatch: ServerEnvelope = serde_json::from_value(mismatch).unwrap();
    assert!(mismatch.validate().is_err());

    let mut burden_mismatch = preview.clone();
    burden_mismatch["preview"]["burden"]["total_burden"] = serde_json::json!("1");
    let burden_mismatch: ServerEnvelope = serde_json::from_value(burden_mismatch).unwrap();
    assert!(burden_mismatch.validate().is_err());

    let mut missing_nullable = preview;
    missing_nullable["preview"]
        .as_object_mut()
        .unwrap()
        .remove("stamina_after");
    assert!(serde_json::from_value::<ServerEnvelope>(missing_nullable).is_err());
}

#[test]
fn decimal_i64_accepts_canonical_boundaries_and_round_trips() {
    for (input, expected) in [
        (r#""9007199254740991""#, 9_007_199_254_740_991_i64),
        (r#""9007199254740992""#, 9_007_199_254_740_992_i64),
        (r#""9007199254740993""#, 9_007_199_254_740_993_i64),
        (r#""9223372036854775807""#, i64::MAX),
        (r#""-9223372036854775808""#, i64::MIN),
        (r#""0""#, 0),
        (r#""17""#, 17),
        (r#""-17""#, -17),
    ] {
        let decoded: DecimalI64 = serde_json::from_str(input).unwrap();
        assert_eq!(decoded.get(), expected);
        assert_eq!(serde_json::to_string(&decoded).unwrap(), input);
    }
}

#[test]
fn decimal_i64_rejects_noncanonical_and_out_of_range_values() {
    for bad in [
        r#"""#,
        r#""-""#,
        r#""+1""#,
        r#""05""#,
        r#""-0""#,
        r#""-05""#,
        r#"" 1""#,
        r#""1 ""#,
        r#""1.0""#,
        r#""a""#,
        r#""9223372036854775808""#,
        r#""-9223372036854775809""#,
    ] {
        assert!(serde_json::from_str::<DecimalI64>(bad).is_err(), "{bad}");
    }
    for bad in ["0", "1", "-1"] {
        assert!(serde_json::from_str::<DecimalI64>(bad).is_err(), "{bad}");
    }
}

#[test]
fn gold_wire_fields_accept_canonical_decimal_strings() {
    for (input, expected) in [
        (r#""9007199254740991""#, 9_007_199_254_740_991_i64),
        (r#""9007199254740992""#, 9_007_199_254_740_992_i64),
        (r#""9007199254740993""#, 9_007_199_254_740_993_i64),
        (r#""9223372036854775807""#, i64::MAX),
        (r#""-9223372036854775808""#, i64::MIN),
        (r#""0""#, 0),
        (r#""17""#, 17),
        (r#""-17""#, -17),
    ] {
        let pile: ObserverGoldPile = serde_json::from_str(&observer_gold_pile_json(input)).unwrap();
        assert_eq!(pile.amount.get(), expected);

        let frame: ObserverFrame = serde_json::from_str(&observer_frame_json(input)).unwrap();
        assert_eq!(frame.carried.gold.sack.get(), expected);

        let quantity: GoldMoveQuantity =
            serde_json::from_str(&format!(r#"{{"kind":"exact","amount":{input}}}"#)).unwrap();
        let GoldMoveQuantity::Exact { amount } = quantity else {
            panic!("expected exact gold quantity");
        };
        assert_eq!(amount.get(), expected);
    }
}

#[test]
fn gold_wire_fields_reject_json_numbers() {
    assert!(serde_json::from_str::<ObserverGoldPile>(&observer_gold_pile_json("1")).is_err());
    assert!(serde_json::from_str::<ObserverFrame>(&observer_frame_json("1")).is_err());
    assert!(serde_json::from_str::<GoldMoveQuantity>(r#"{"kind":"exact","amount":1}"#).is_err());
    assert!(decode_client_command(move_gold_command("1").as_bytes(), PROTOCOL_MINOR).is_err());
}

#[test]
fn gold_move_exact_rejects_non_positive_amounts() {
    assert!(decode_client_command(move_gold_command(r#""1""#).as_bytes(), PROTOCOL_MINOR).is_ok());
    for amount in [r#""0""#, r#""-1""#, r#""-9223372036854775808""#] {
        let error = decode_client_command(move_gold_command(amount).as_bytes(), PROTOCOL_MINOR)
            .unwrap_err();
        assert_eq!(error.to_string(), "gold amount must be positive");
    }
    let minimum: DecimalI64 = serde_json::from_str(r#""-9223372036854775808""#).unwrap();
    assert_eq!(minimum.get(), i64::MIN);
}

#[test]
fn observer_frame_validation_rejects_negative_gold() {
    assert!(observer_frame(0, &[]).validate().is_ok());
    assert!(observer_frame(i64::MAX, &[0]).validate().is_ok());
    assert_eq!(
        observer_frame(0, &[-1]).validate().unwrap_err().to_string(),
        "observer gold pile amount must be non-negative"
    );
    assert_eq!(
        observer_frame(0, &[i64::MIN])
            .validate()
            .unwrap_err()
            .to_string(),
        "observer gold pile amount must be non-negative"
    );
    assert_eq!(
        observer_frame(-1, &[]).validate().unwrap_err().to_string(),
        "observer carried gold must be non-negative"
    );
    assert_eq!(
        observer_frame(i64::MIN, &[0])
            .validate()
            .unwrap_err()
            .to_string(),
        "observer carried gold must be non-negative"
    );
}

#[test]
fn control_api_4_dtos_are_strict_and_predecessor_shapes_fail() {
    assert_eq!(CONTROL_API_VERSION, 4);
    assert_eq!(PROTOCOL_MINOR, 8);

    let command = format!(
        r#"{{"kind":"command","command_id":"{COMMAND}","control_epoch":"1","client_sequence":"1","observed_world_revision":"0","actor_id":"player","intent":{{"kind":"wait"}}}}"#
    );
    assert!(decode_client_command(command.as_bytes(), 8).is_ok());
    assert!(decode_client_command(command.as_bytes(), 7).is_err());

    let bootstrap: SessionBootstrapV1 = serde_json::from_value(bootstrap_json()).unwrap();
    assert!(bootstrap.characters.is_empty());

    // D4: the bootstrap advertises no selectable world directory, and the
    // predecessor shape that did is refused outright.
    let mut predecessor_bootstrap = bootstrap_json();
    predecessor_bootstrap.as_object_mut().unwrap().insert(
        "facets".to_string(),
        serde_json::json!([{
            "facet_id": FACET,
            "facet_key": "main",
            "family_key": "world",
            "copy_key": "one"
        }]),
    );
    assert!(serde_json::from_value::<SessionBootstrapV1>(predecessor_bootstrap).is_err());

    // D4: a command names no world. The predecessor shape that did is refused.
    let with_world = format!(
        r#"{{"kind":"command","command_id":"{COMMAND}","control_epoch":"1","client_sequence":"1","observed_world_revision":"0","facet_id":"{FACET}","actor_id":"player","intent":{{"kind":"wait"}}}}"#
    );
    assert!(decode_client_command(with_world.as_bytes(), PROTOCOL_MINOR).is_err());

    // D4: a character summary carries no world identity for the player to read.
    let bound_character = serde_json::json!({
        "character_id": CHARACTER,
        "slot": 1,
        "display_name": "Wayfarer",
        "facet_id": FACET
    });
    assert!(serde_json::from_value::<CharacterSummaryV1>(bound_character).is_err());
}

#[test]
fn feedback_rejects_noncanonical_sequence_ids_and_excess_receipts() {
    let actor = serde_json::json!({
        "actor_id": "player",
        "name": "Wayfarer",
        "kind": "player"
    });
    let malformed_corpse = feedback_envelope(serde_json::json!({
        "kind": "corpse",
        "corpse_id": "corpse:01",
        "origin": actor,
        "location": position(),
        "change": {"kind": "created"}
    }));
    assert!(malformed_corpse.validate().is_err());

    let costs = (0..=tme_protocol::MAX_FEEDBACK_TRANSACTION_COSTS)
        .map(|_| {
            serde_json::json!({
                "kind": "carried_gold",
                "amount": "1",
                "position": "sack",
                "before": "2",
                "after": "1"
            })
        })
        .collect::<Vec<_>>();
    let excessive = feedback_envelope(serde_json::json!({
        "kind": "transaction",
        "actor": {
            "actor_id": "player",
            "name": "Wayfarer",
            "kind": "player"
        },
        "source": {
            "kind": "bank_withdrawal",
            "service_id": "bank",
            "capability_id": "withdraw",
            "bank_id": "bank_1",
            "amount": "1"
        },
        "costs": costs,
        "rewards": []
    }));
    assert!(excessive.validate().is_err());

    let malformed_gold = feedback_envelope(serde_json::json!({
        "kind": "transaction",
        "actor": {
            "actor_id": "player",
            "name": "Wayfarer",
            "kind": "player"
        },
        "source": {
            "kind": "bank_deposit",
            "service_id": "bank",
            "capability_id": "deposit",
            "bank_id": "bank_1",
            "gold_pile_id": "gold:0"
        },
        "costs": [],
        "rewards": []
    }));
    assert!(malformed_gold.validate().is_err());
}

#[test]
fn wire_scalars_are_canonical() {
    assert!(serde_json::from_str::<DecimalU64>(r#""0""#).is_ok());
    for bad in [r#""00""#, r#""+1""#, r#""-1""#, "1"] {
        assert!(serde_json::from_str::<DecimalU64>(bad).is_err(), "{bad}");
    }
    assert!(serde_json::from_str::<CommandId>(&format!(r#""{COMMAND}""#)).is_ok());
    assert!(
        serde_json::from_str::<CommandId>(r#""00000000-0000-0000-0000-000000000000""#).is_err()
    );
    assert!(AdmissionTicket::new(TICKET).is_ok());
    assert!(AdmissionTicket::new(format!("{}B", "A".repeat(42))).is_err());
}

#[test]
fn social_messages_are_strict_bounded_and_body_debug_is_redacted() {
    let message = format!(
        r#"{{"kind":"social_message","message_id":"{COMMAND}","control_epoch":"1","actor_id":"player","scope":{{"kind":"page","target_character_id":"018f0f9f-9b5a-7c61-8d2d-5ab82b1c3d50"}},"body":"meet by the fountain"}}"#
    );
    let decoded = decode_client_command(message.as_bytes(), PROTOCOL_MINOR).expect("social");
    assert!(matches!(
        decoded,
        ClientCommandEnvelope::SocialMessage {
            scope: SocialScope::Page { .. },
            ..
        }
    ));
    assert!(!format!("{decoded:?}").contains("meet by the fountain"));
    assert!(
        decode_client_command(
            message.replace("}", ",\"legacy\":true}").as_bytes(),
            PROTOCOL_MINOR
        )
        .is_err()
    );
    assert!(SocialBody::new("").is_err());
    assert!(SocialBody::new("line\nbreak").is_err());
    assert!(SocialBody::new("x".repeat(MAX_SOCIAL_BODY_SCALARS + 1)).is_err());
    assert!(SocialBody::new("🜃".repeat(MAX_SOCIAL_BODY_BYTES / 4 + 1)).is_err());
    let body = SocialBody::new("private words").expect("valid body");
    assert_eq!(format!("{body:?}"), "SocialBody([REDACTED])");
}

#[test]
fn old_expected_revision_field_is_rejected() {
    let command = format!(
        r#"{{"kind":"command","command_id":"{COMMAND}","control_epoch":"1","client_sequence":"1","expected_facet_revision":"0","actor_id":"player","intent":{{"kind":"wait"}}}}"#
    );
    assert!(decode_client_command(command.as_bytes(), PROTOCOL_MINOR).is_err());
}

#[test]
fn size_nesting_unknown_fields_and_invalid_utf8_fail_closed() {
    assert!(decode_client_hello(&vec![b' '; MAX_INPUT_BYTES + 1]).is_err());
    assert!(decode_client_hello(&[0xff]).is_err());
    let mut nested = String::new();
    for _ in 0..=MAX_JSON_NESTING {
        nested.push('[');
    }
    nested.push('0');
    for _ in 0..=MAX_JSON_NESTING {
        nested.push(']');
    }
    assert!(decode_client_hello(nested.as_bytes()).is_err());
    let unknown = format!(
        r#"{{"kind":"client_hello","ticket":"{TICKET}","supported_minors":[8],"legacy":true}}"#
    );
    assert!(decode_client_hello(unknown.as_bytes()).is_err());
}

#[test]
fn control_login_is_strict_bounded_and_redacted() {
    let request = decode_login_request(
        br#"{"username":"test_user","password":"correct horse battery staple"}"#,
    )
    .expect("login request");
    assert!(!format!("{request:?}").contains("correct horse"));
    assert!(
        decode_login_request(
            br#"{"username":"Test_User","password":"correct horse battery staple"}"#,
        )
        .is_err()
    );
    assert!(decode_login_request(
        br#"{"username":"test_user","password":"correct horse battery staple","password":"duplicate"}"#,
    )
    .is_err());
}
