//! Real authored expedition inputs; only duel placement, skill and test gear are staged.
use std::sync::{Arc, OnceLock};

use serde_json::{Value, json};
use tme_rules::{
    ActorId, ActorState, BlockSourceKind, CarriedPosition, CatalogProfileKey, CharacterId, Engine,
    Event, GameDefinition, GroundItem, HostilityAuthorization, ItemBindingState, ItemInstanceState,
    ItemKnowledgeState, ItemMoveDestination, PhysicalAttackMode, PhysicalBlockCandidateViewV1,
    PlayerIntent, ValidatedWorldSeed, WorldSeedDef, WorldTemplateV3,
};

pub const PROFILE: &str = "profile/first_expedition";
pub const ACTION: &str = "profession_action/martial_hand_block/first_expedition";
pub const ARMOR: &str = "defense_test_armor";

fn inputs() -> &'static (Value, WorldTemplateV3, WorldSeedDef) {
    static INPUTS: OnceLock<(Value, WorldTemplateV3, WorldSeedDef)> = OnceLock::new();
    INPUTS.get_or_init(|| {
        let root = tme_authoring::repository_root().unwrap();
        let land =
            tme_authoring::load(&root, tme_authoring::land("first_expedition").unwrap()).unwrap();
        let catalog = serde_json::from_slice(
            &std::fs::read(root.join(land.contract().terrain_registry_catalog)).unwrap(),
        )
        .unwrap();
        let mut seed: Value = serde_json::from_slice(
            &std::fs::read(root.join("content/lands/first-expedition/simulation_seed.json"))
                .unwrap(),
        )
        .unwrap();
        for key in ["schema_version", "kind", "id"] {
            seed.as_object_mut().unwrap().remove(key);
        }
        (
            catalog,
            tme_authoring::project(&land).unwrap(),
            serde_json::from_value(seed).unwrap(),
        )
    })
}

pub fn catalog() -> Value {
    inputs().0.clone()
}

pub fn build_definition(catalog: Value) -> Arc<GameDefinition> {
    GameDefinition::from_content(
        serde_json::from_value(catalog).unwrap(),
        CatalogProfileKey::from(PROFILE),
        inputs().1.clone(),
    )
    .unwrap()
}

pub fn definition() -> Arc<GameDefinition> {
    static DEFINITION: OnceLock<Arc<GameDefinition>> = OnceLock::new();
    DEFINITION
        .get_or_init(|| build_definition(catalog()))
        .clone()
}

pub fn armor_definition() -> Arc<GameDefinition> {
    // A labeled fixture addition only. The production expedition has no wearable armor.
    let mut source = catalog();
    source["items"]["item/defense_test_armor"] = json!({
        "id": ARMOR, "name": "Defense Test Armor", "kind": "armor",
        "valid_placements": ["hand", "sack", "outer_armor"],
        "armor": {
            "block_rating": 0, "encumbrance": 5,
            "damage_reduction": {"cutting": 0, "piercing": 0, "crushing": 0}
        },
        "economy": {"unit_burden": 1}
    });
    source["profiles"][PROFILE]["items"]
        .as_array_mut()
        .unwrap()
        .push(json!("item/defense_test_armor"));
    build_definition(source)
}

pub fn defender_id() -> ActorId {
    ActorId::from("created/proof/martial_defender")
}

pub fn defender(engine: &Engine) -> &ActorState {
    engine.world().actor(&defender_id()).unwrap()
}

pub fn defender_mut(engine: &mut Engine) -> &mut ActorState {
    engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id == defender_id())
        .unwrap()
}

pub fn duel(definition: Arc<GameDefinition>, profile_id: &str, level: u8, seed: u64) -> Engine {
    let mut engine = Engine::new(
        ValidatedWorldSeed::new(definition.clone(), inputs().2.clone()).unwrap(),
        seed,
    )
    .unwrap();
    let profile = definition
        .creation_profiles()
        .iter()
        .find(|profile| profile.id == profile_id)
        .unwrap();
    let character_id = CharacterId::new("proof/martial_defender");
    engine = engine
        .prepare_character_creation(
            profile_id,
            character_id.clone(),
            "Defense test character",
            profile.character.attributes.clone(),
        )
        .unwrap();
    engine
        .apply_connection_presence(&character_id, 1, true)
        .unwrap();
    let location = engine
        .world()
        .actor(&ActorId::from("player"))
        .unwrap()
        .location
        .clone();
    let actor = defender_mut(&mut engine);
    actor.location = location;
    // Stage comparable skill and empty hands; preserve every created item's identity.
    for (hand, sack) in [
        (CarriedPosition::LeftHand, CarriedPosition::SackItem19),
        (CarriedPosition::RightHand, CarriedPosition::SackItem20),
    ] {
        if let Some(item) = actor.carried.items.remove(&hand) {
            assert!(actor.carried.items.insert(sack, item).is_none());
        }
    }
    actor
        .character
        .as_mut()
        .unwrap()
        .skill_ledger
        .iter_mut()
        .find(|entry| entry.track_id == "hand")
        .unwrap()
        .level = level;
    engine
}

pub fn candidate(engine: &Engine) -> Option<PhysicalBlockCandidateViewV1> {
    engine
        .snapshot()
        .actors
        .into_iter()
        .find(|actor| actor.id == defender_id())
        .unwrap()
        .physical_weapon?
        .eligible_block_candidates
        .into_iter()
        .find(|entry| entry.source == BlockSourceKind::RightMartialHand)
}

pub fn attack(engine: &mut Engine, mode: PhysicalAttackMode) -> Vec<Event> {
    engine
        .apply_realtime_actor_intent(
            &ActorId::from("player"),
            PlayerIntent::PhysicalAttack {
                mode,
                target_actor_id: defender_id(),
                authorization: HostilityAuthorization::ConfirmedUnsafe,
            },
        )
        .unwrap()
        .events
}

pub fn hand_block(events: &[Event]) -> Option<&Event> {
    events.iter().find(|event| {
        matches!(event,
            Event::AttackBlocked {
                source: BlockSourceKind::RightMartialHand, defender_id: id, ..
            } if *id == defender_id()
        )
    })
}

pub fn place_test_item(engine: &mut Engine, definition_id: &str, instance: &str) {
    let location = defender(engine).location.clone();
    let previous = engine.world_mut().item_instances.insert(
        instance.to_string(),
        ItemInstanceState {
            definition_id: definition_id.to_string(),
            quantity: 1,
            knowledge: ItemKnowledgeState::default(),
            binding: ItemBindingState::Unrestricted,
            bow_readiness: None,
        },
    );
    assert!(previous.is_none());
    engine.world_mut().ground_items.push(GroundItem {
        item_instance_id: instance.to_string(),
        location,
        loot_claim: None,
    });
}

pub fn equip(engine: &mut Engine, instance: &str, position: CarriedPosition) {
    engine
        .apply_realtime_actor_intent(
            &defender_id(),
            PlayerIntent::MoveItem {
                item_instance_id: instance.to_string(),
                destination: ItemMoveDestination::Carried { position },
            },
        )
        .unwrap();
}

/// Deadline advancement may run unrelated actors. Read, never reset, the RNG
/// checkpoint when checking the next single-candidate unarmed defense roll.
pub fn next_block_roll(engine: &Engine) -> u32 {
    let checkpoint = engine.export_checkpoint().unwrap();
    let payload: Value = serde_json::from_slice(checkpoint.as_bytes()).unwrap();
    let state = payload["rng_state"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    tme_rules::DeterministicRng::new(state).roll_d20()
}
