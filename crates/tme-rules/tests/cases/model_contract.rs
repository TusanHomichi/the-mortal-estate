use crate::support::content_parts::ContentParts;
use tme_rules::{
    ActorLifeState, CharacterId, Coord, CorpseId, Direction, GoldPileId, ItemHolderId,
    ItemLocation, LogicalTime, LootClaim, LootClaimBasis, LootOwnerId, PlayerIntent,
    SpellItemLocation, SpellTarget, WorldPosition,
};

#[test]
fn actor_id_is_a_transparent_ordered_display_type_not_a_string_alias() {
    let mut ids = [
        tme_rules::ActorId::from("zeta"),
        tme_rules::ActorId::from("alpha"),
    ];
    ids.sort();
    assert_eq!(ids[0].as_str(), "alpha");
    assert_eq!(ids[0].to_string(), "alpha");
    assert_eq!(serde_json::to_string(&ids[0]).unwrap(), r#""alpha""#);
    assert_eq!(
        serde_json::from_str::<tme_rules::ActorId>(r#""alpha""#).unwrap(),
        ids[0]
    );
}

fn assert_same_type<T>(_: &T, _: &T) {}
fn assert_type<T>() {}

#[test]
fn provider_neutral_transaction_model_is_public() {
    assert_type::<tme_rules::Transaction>();
    assert_type::<tme_rules::TransactionRequirement>();
    assert_type::<tme_rules::TransactionCost>();
    assert_type::<tme_rules::TransactionReward>();
    assert_type::<tme_rules::model::transactions::Transaction>();
}

#[test]
fn player_item_move_intent_labels_are_exact() {
    assert_eq!(
        PlayerIntent::MoveItem {
            item_instance_id: "hemp_rope".to_string(),
            destination: tme_rules::ItemMoveDestination::Carried {
                position: tme_rules::CarriedPosition::SackItem1
            }
        }
        .label(),
        "move_item hemp_rope to sack_item_1"
    );
    assert_eq!(
        PlayerIntent::MoveItem {
            item_instance_id: "hemp_rope".to_string(),
            destination: tme_rules::ItemMoveDestination::GroundHere
        }
        .label(),
        "move_item hemp_rope to ground_here"
    );
    assert_eq!(PlayerIntent::ShowSack.label(), "show_sack");
}

#[test]
fn typed_spell_target_intent_labels_are_stable() {
    assert_eq!(
        PlayerIntent::CastSpell {
            spell_id: "spark".to_string(),
            target: Some(SpellTarget::None),
            authorization: tme_rules::HostilityAuthorization::Safe,
        }
        .label(),
        "cast spark on none"
    );
    assert_eq!(
        PlayerIntent::CastSpell {
            spell_id: "spark".to_string(),
            target: Some(SpellTarget::SelfTarget),
            authorization: tme_rules::HostilityAuthorization::Safe,
        }
        .label(),
        "cast spark on self"
    );
    assert_eq!(
        PlayerIntent::CastSpell {
            spell_id: "spark".to_string(),
            target: Some(SpellTarget::Actor {
                actor_id: "mireling".into(),
            }),
            authorization: tme_rules::HostilityAuthorization::Safe,
        }
        .label(),
        "cast spark on mireling"
    );
    assert_eq!(
        PlayerIntent::CastSpell {
            spell_id: "spark".to_string(),
            target: Some(SpellTarget::Coordinate {
                position: WorldPosition::new("realm_0", "gloom_cellar", Coord { x: 3, y: 4 }),
            }),
            authorization: tme_rules::HostilityAuthorization::Safe,
        }
        .label(),
        "cast spark on realm_0/gloom_cellar:3,4"
    );
    assert_eq!(
        PlayerIntent::CastSpell {
            spell_id: "spark".to_string(),
            target: Some(SpellTarget::Area {
                center: WorldPosition::new("realm_0", "gloom_cellar", Coord { x: 3, y: 4 }),
            }),
            authorization: tme_rules::HostilityAuthorization::Safe,
        }
        .label(),
        "cast spark on area realm_0/gloom_cellar:3,4"
    );
    assert_eq!(
        PlayerIntent::CastSpell {
            spell_id: "spark".to_string(),
            target: Some(SpellTarget::Direction {
                direction: Direction::Northwest,
            }),
            authorization: tme_rules::HostilityAuthorization::Safe,
        }
        .label(),
        "cast spark on northwest"
    );
    assert_eq!(
        PlayerIntent::CastSpell {
            spell_id: "spark".to_string(),
            target: Some(SpellTarget::Door {
                direction: Direction::East,
            }),
            authorization: tme_rules::HostilityAuthorization::Safe,
        }
        .label(),
        "cast spark on door east"
    );
    assert_eq!(
        PlayerIntent::CastWarmedSpell {
            target: Some(SpellTarget::Item {
                item_instance_id: "iron_key".to_string(),
                location: SpellItemLocation::GroundHere,
            }),
            authorization: tme_rules::HostilityAuthorization::Safe,
        }
        .label(),
        "cast_warmed_spell on ground_here:iron_key"
    );
    assert_eq!(
        PlayerIntent::CastSpell {
            spell_id: "path_mark".to_string(),
            target: Some(SpellTarget::Path {
                directions: vec![Direction::East, Direction::Northeast],
            }),
            authorization: tme_rules::HostilityAuthorization::Safe,
        }
        .label(),
        "cast path_mark on path east,northeast"
    );
    assert_eq!(
        PlayerIntent::WarmSpell {
            spell_id: "charged_path".to_string()
        }
        .label(),
        "warm_spell charged_path"
    );
    assert_eq!(
        PlayerIntent::FizzleWarmedSpell.label(),
        "fizzle_warmed_spell"
    );
    assert_eq!(PlayerIntent::Rest.label(), "rest");
}

#[test]
fn spell_item_target_serializes_only_explicit_instance_identity() {
    let target = SpellTarget::Item {
        item_instance_id: "iron_key_a".to_string(),
        location: SpellItemLocation::GroundHere,
    };
    let value = serde_json::to_value(&target).expect("target should serialize");

    assert_eq!(value["item"]["item_instance_id"], "iron_key_a");
    assert!(value["item"].get("item_id").is_none());
    assert!(
        serde_json::from_value::<SpellTarget>(serde_json::json!({
            "item": {"item_id": "iron_key_a", "location": "ground_here"}
        }))
        .is_err()
    );
}

#[test]
fn spell_item_target_rejects_mixed_current_and_obsolete_identity_keys() {
    let error = serde_json::from_value::<SpellTarget>(serde_json::json!({
        "item": {
            "item_instance_id": "iron_key_a",
            "item_id": "obsolete_alias",
            "location": "ground_here"
        }
    }))
    .expect_err("a current item target must reject the obsolete item_id key");

    assert!(
        error.to_string().contains("unknown field `item_id`"),
        "unexpected strict-target error: {error}"
    );
}

#[test]
fn room_coord_equality_and_cloning() {
    use tme_rules::WorldPosition;
    let a = WorldPosition::new("realm_0", "entrance_hall", tme_rules::Coord { x: 1, y: 2 });
    let b = WorldPosition::new("realm_0", "entrance_hall", tme_rules::Coord { x: 1, y: 2 });
    let c = WorldPosition::new("realm_0", "guard_post", tme_rules::Coord { x: 1, y: 2 });
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(a.level, "entrance_hall");
    assert_eq!(a.position, tme_rules::Coord { x: 1, y: 2 });
}

#[test]
fn engine_starts_actor_with_authored_exact_carried_layout() {
    let engine = ContentParts::tracked("first_room", "profile/first_room")
        .engine(7)
        .expect("engine should start");
    assert_eq!(
        engine
            .world()
            .actor(&tme_rules::ActorId::from("player"))
            .unwrap()
            .carried
            .items
            .get(&tme_rules::CarriedPosition::RightHand)
            .map(String::as_str),
        Some("training_knife")
    );
}

#[test]
fn item_instance_definition_parses_required_binding_state() {
    let definition: tme_rules::content::items::ItemInstanceSeedDef =
        serde_json::from_value(serde_json::json!({
            "definition_id": "rope",
            "quantity": 2,
            "knowledge": {
                "identified": true,
                "appraised": false
            },
            "binding": {"state": "unrestricted"}
        }))
        .expect("simulation-seed item instance should parse");

    assert_eq!(definition.definition_id, "rope");
    assert_eq!(definition.quantity, 2);
    assert!(definition.knowledge.identified);
    assert!(!definition.knowledge.appraised);
}

#[test]
fn item_model_root_and_nested_exports_name_the_same_types() {
    let root: tme_rules::ItemHolderId = ItemHolderId::TransientActor("player".into());
    let nested: tme_rules::model::items::ItemHolderId = root.clone();
    assert_same_type(&root, &nested);

    let root_location: tme_rules::ItemLocation = ItemLocation::Carried {
        holder: root,
        position: tme_rules::CarriedPosition::SackItem1,
    };
    let nested_location: tme_rules::model::items::ItemLocation = root_location.clone();
    assert_same_type(&root_location, &nested_location);
}

#[test]
fn stable_and_transient_item_holders_are_type_distinct() {
    let character_id: CharacterId = serde_json::from_str("\"character:model_contract:primary\"")
        .expect("character id should deserialize");
    let stable = ItemHolderId::Character(character_id);
    let transient = ItemHolderId::TransientActor("character:model_contract:primary".into());

    assert_ne!(stable, transient);
}

#[test]
fn debug_snapshot_exposes_eg_character_identity_but_not_item_location_internals() {
    let engine = ContentParts::tracked("character_sheet", "profile/character_sheet")
        .engine(7)
        .expect("engine should start");
    let snapshot = serde_json::to_value(engine.snapshot()).expect("snapshot should serialize");
    let actors = snapshot["actors"].as_array().expect("debug actors");

    assert_eq!(
        actors
            .iter()
            .find(|actor| actor["id"] == "player")
            .expect("player actor")["character_id"],
        "character:character_sheet:primary"
    );
    assert!(
        actors
            .iter()
            .find(|actor| actor["id"] == "mireling")
            .expect("inherent actor")["character_id"]
            .is_null()
    );
    let serialized = snapshot.to_string();
    assert!(!serialized.contains("item_holder"));
    assert!(!serialized.contains("item_location"));
}

#[test]
fn death_strong_ids_accept_only_canonical_positive_sequences() {
    assert_eq!(
        serde_json::to_string(&CorpseId::parse("corpse:1").unwrap()).unwrap(),
        "\"corpse:1\""
    );
    assert_eq!(
        serde_json::to_string(&GoldPileId::parse("gold:42").unwrap()).unwrap(),
        "\"gold:42\""
    );
    for invalid in [
        "",
        "corpse",
        "corpse:",
        "corpse:0",
        "corpse:01",
        "corpse:-1",
        "corpse:+1",
        "corpse:1 ",
        "corpse:1x",
    ] {
        assert!(CorpseId::parse(invalid).is_err(), "{invalid:?}");
    }
    for invalid in ["", "gold", "gold:0", "gold:01", "gold:-1", "gold: 1"] {
        assert!(GoldPileId::parse(invalid).is_err(), "{invalid:?}");
    }
}

#[test]
fn life_state_and_loot_claim_json_shapes_are_exact() {
    let ghost = ActorLifeState::Ghost {
        corpse_id: CorpseId::parse("corpse:3").unwrap(),
        defeated_at: LogicalTime::new(7),
    };
    assert_eq!(
        serde_json::to_value(&ghost).unwrap(),
        serde_json::json!({
            "kind": "ghost",
            "corpse_id": "corpse:3",
            "defeated_at": 7
        })
    );
    assert_eq!(
        serde_json::to_value(ActorLifeState::AwaitingResurrection {
            cause: tme_rules::DeathCause::Fire,
            defeated_at: LogicalTime::new(8),
        })
        .unwrap(),
        serde_json::json!({
            "kind": "awaiting_resurrection",
            "cause": "fire",
            "defeated_at": 8
        })
    );

    let character_id: CharacterId =
        serde_json::from_str("\"character:model_contract:primary\"").unwrap();
    let claim = LootClaim {
        owner: LootOwnerId::Character(character_id),
        basis: LootClaimBasis::CharacterDeathPile,
    };
    assert_eq!(
        serde_json::to_value(claim).unwrap(),
        serde_json::json!({
            "owner": {
                "kind": "character",
                "id": "character:model_contract:primary"
            },
            "basis": "character_death_pile"
        })
    );
    assert_eq!(
        PlayerIntent::SearchCorpse(CorpseId::parse("corpse:3").unwrap()).label(),
        "search corpse corpse:3"
    );
}
