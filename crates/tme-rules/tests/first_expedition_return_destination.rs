//! Both authored return destinations are reachable, and creation reaches one.
//!
//! The composed death journey in `tools/run_first_expedition_proof.py` creates
//! its character through the ordinary UI, so the alignment it returns with is
//! whatever an authored creation profile selects — and every shipped profile
//! selects `lawful`. The neutral branch is therefore not reachable by creation,
//! and the journey must not fake it by assigning an alignment no supported
//! choice produces.
//!
//! What is proved here is the actual rule that chooses between the two authored
//! destinations, and the actual supported way a character becomes neutral: the
//! unjust kill of an authored lawful NPC. A lawful character returns at the
//! lawful destination; after that ordinary kill it returns at the neutral one.
//! The destinations compared against are the template's own authored values,
//! which are the same ones the browser journey asserts.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;
use tme_rules::model::ActorState;
use tme_rules::{
    ActorId, ActorLifeState, CatalogProfileKey, CatalogV6, CharacterAlignment, CharacterId, Engine,
    GameDefinition, HostilityAuthorization, LogicalTime, PhysicalAttackMode, PlayerIntent,
    ValidatedWorldSeed, WorldPosition, WorldSeedDef, WorldTemplateV4,
};

const PROFILE: &str = "profile/first_expedition";
const SCAVENGER: &str = "cellar_scavenger";
/// An authored lawful human who stands in the open and is not hostile to anyone.
const VICTIM: &str = "balm_seller";

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_json(relative: &str) -> Value {
    let path = repository_root().join(relative);
    serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|error| {
        panic!("{}: {error}", path.display());
    }))
    .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn definition() -> Arc<GameDefinition> {
    let catalog: CatalogV6 =
        serde_json::from_value(read_json("content/lands/first-expedition/catalog.json"))
            .expect("catalog decodes");
    let template: WorldTemplateV4 = serde_json::from_value(read_json(
        "content/lands/first-expedition/generated/world_template.json",
    ))
    .expect("world template decodes");
    GameDefinition::from_content(catalog, CatalogProfileKey::from(PROFILE), template)
        .expect("definition compiles")
}

fn seed_source() -> WorldSeedDef {
    let mut source = read_json("content/lands/first-expedition/simulation_seed.json");
    for key in ["schema_version", "kind", "id"] {
        source.as_object_mut().expect("seed object").remove(key);
    }
    serde_json::from_value(source).expect("simulation seed payload")
}

/// The authored destination policy, read from the served world template.
fn authored_destinations() -> (WorldPosition, WorldPosition) {
    let template: WorldTemplateV4 = serde_json::from_value(read_json(
        "content/lands/first-expedition/generated/world_template.json",
    ))
    .expect("world template decodes");
    let policy = template
        .resurrection
        .get("first_expedition")
        .expect("the first expedition authors a return policy");
    (
        policy.lawful_destination.clone(),
        policy.neutral_destination.clone(),
    )
}

fn create(definition: &Arc<GameDefinition>, rng_seed: u64) -> (Engine, ActorId) {
    let engine = Engine::new(
        ValidatedWorldSeed::new(definition.clone(), seed_source()).expect("seed validates"),
        rng_seed,
    )
    .expect("engine starts");
    let profile_id = "creation/wizard";
    let profile = engine
        .definition()
        .creation_profiles()
        .iter()
        .find(|profile| profile.id == profile_id)
        .unwrap_or_else(|| panic!("creation profile {profile_id:?}"))
        .clone();
    let character_id = CharacterId::new("proof/creation/wizard");
    let engine = engine
        .prepare_character_creation(
            &profile.id,
            character_id.clone(),
            "Probe",
            profile.character.attributes.clone(),
        )
        .expect("ordinary character creation");
    (
        engine,
        ActorId::new(format!("created/{}", character_id.as_str())),
    )
}

fn actor<'a>(engine: &'a Engine, actor_id: &ActorId) -> &'a ActorState {
    engine
        .world()
        .actor(actor_id)
        .unwrap_or_else(|| panic!("actor {actor_id:?}"))
}

fn actor_mut<'a>(engine: &'a mut Engine, actor_id: &ActorId) -> &'a mut ActorState {
    engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| &actor.id == actor_id)
        .unwrap_or_else(|| panic!("actor {actor_id:?}"))
}

/// Stand the character on another actor's square, which ordinary movement
/// reaches; every later fact comes from the engine.
fn stand_on(engine: &mut Engine, actor_id: &ActorId, other: &str) {
    let location = actor(engine, &ActorId::from(other)).location.clone();
    actor_mut(engine, actor_id).location = location;
}

fn attack(engine: &mut Engine, actor_id: &ActorId, target: &str) -> Vec<tme_rules::Event> {
    engine
        .apply_actor_intent(
            actor_id,
            PlayerIntent::PhysicalAttack {
                mode: PhysicalAttackMode::Fight,
                target_actor_id: ActorId::from(target),
                authorization: HostilityAuthorization::ConfirmedUnsafe,
            },
        )
        .unwrap_or_else(|error| panic!("ordinary fight command against {target}: {error}"))
        .events
}

fn wait(engine: &mut Engine, actor_id: &ActorId) {
    engine
        .apply_actor_intent(actor_id, PlayerIntent::Wait)
        .expect("ordinary wait command");
}

fn alignment(engine: &Engine, actor_id: &ActorId) -> CharacterAlignment {
    actor(engine, actor_id)
        .character
        .as_ref()
        .expect("character sheet")
        .alignment_state
        .alignment
}

/// Kill the character with the authored opponent, then take the ordinary return.
fn die_and_return(engine: &mut Engine, actor_id: &ActorId) -> WorldPosition {
    stand_on(engine, actor_id, SCAVENGER);
    let mut rounds = 0;
    while actor(engine, actor_id).is_alive() {
        rounds += 1;
        assert!(rounds < 500, "the authored opponent could not defeat the character");
        wait(engine, actor_id);
    }
    let deadline = match actor(engine, actor_id).life_state {
        ActorLifeState::Ghost { defeated_at, .. } => defeated_at.saturating_add_millis(60_000),
        ref other => panic!("ordinary death must create a ghost, not {other:?}"),
    };
    engine
        .advance_to(deadline)
        .expect("advancing to the authored return deadline");
    engine
        .apply_actor_intent(actor_id, PlayerIntent::RequestResurrection)
        .expect("the authored return request");
    let actor = actor(engine, actor_id);
    assert!(actor.is_alive(), "the return left the character dead");
    actor.location.clone()
}

#[test]
fn every_authored_creation_profile_selects_one_alignment() {
    // This is why the composed journey runs one destination: ordinary creation
    // offers no choice of alignment, so the neutral branch cannot be exercised
    // by a created character at all.
    let catalog = read_json("content/lands/first-expedition/catalog.json");
    let profiles = catalog["creation_profiles"]
        .as_object()
        .expect("creation profiles");
    assert!(!profiles.is_empty(), "the world authors creation profiles");
    let selected = profiles
        .values()
        .map(|profile| {
            profile["character"]["alignment_state"]["alignment"]
                .as_str()
                .expect("authored alignment")
                .to_string()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        selected,
        BTreeSet::from(["lawful".to_string()]),
        "a creation choice that selects another alignment changes which destination is reachable"
    );
}

#[test]
fn a_lawful_created_character_returns_at_the_authored_lawful_destination() {
    let (lawful, neutral) = authored_destinations();
    let definition = definition();
    let (mut engine, actor_id) = create(&definition, 13);
    assert_eq!(alignment(&engine, &actor_id), CharacterAlignment::Lawful);
    let landed = die_and_return(&mut engine, &actor_id);
    assert_eq!(landed, lawful, "a lawful character returns at the lawful destination");
    assert_ne!(landed, neutral, "the two authored branches are distinct places");
}

#[test]
fn an_unjust_kill_makes_the_created_character_neutral_and_moves_its_return() {
    // The supported way to the neutral branch: the ordinary social law, applied
    // to an authored lawful human the character attacks without provocation.
    let (lawful, neutral) = authored_destinations();
    let definition = definition();
    let (mut engine, actor_id) = create(&definition, 13);
    stand_on(&mut engine, &actor_id, VICTIM);
    let victim = ActorId::from(VICTIM);
    let mut rounds = 0;
    while actor(&engine, &victim).is_alive() {
        rounds += 1;
        assert!(rounds < 500, "the authored NPC could not be killed");
        attack(&mut engine, &actor_id, VICTIM);
    }
    assert_eq!(
        alignment(&engine, &actor_id),
        CharacterAlignment::Neutral,
        "an unjust lawful kill is the authored way to become neutral"
    );
    assert_ne!(lawful, neutral, "the two authored branches are distinct places");
    let landed = die_and_return(&mut engine, &actor_id);
    assert_eq!(
        landed, neutral,
        "a neutral character returns at the authored neutral destination"
    );
    assert_ne!(landed, lawful, "the neutral branch did not fall back to the lawful one");
}

#[test]
fn the_opponent_is_chaotic_so_defeating_it_is_never_an_unjust_lawful_kill() {
    // The death journey kills nothing but the authored opponent. If that
    // opponent were a lawful animal or human, the encounter itself would move
    // the character's alignment and the journey's destination would depend on
    // how the fight ended.
    let catalog = read_json("content/lands/first-expedition/catalog.json");
    let opponent = &catalog["actor_definitions"]["actor-definition/first_expedition/cellar_scavenger"];
    assert_eq!(opponent["social"]["nature"].as_str(), Some("other"));
    assert_eq!(
        opponent["social"]["alignment_source"]["alignment"].as_str(),
        Some("chaotic")
    );
    let definition = definition();
    // Seed 23 is a real fight the created wizard wins; the point is what winning
    // does to alignment, so a lost encounter is a failure to observe it.
    let (mut engine, actor_id) = create(&definition, 23);
    stand_on(&mut engine, &actor_id, SCAVENGER);
    let scavenger = ActorId::from(SCAVENGER);
    let mut rounds = 0;
    while actor(&engine, &scavenger).is_alive() {
        rounds += 1;
        assert!(rounds < 500, "the encounter did not resolve");
        assert!(
            actor(&engine, &actor_id).is_alive(),
            "the created character lost the encounter, so defeating the opponent was never observed"
        );
        attack(&mut engine, &actor_id, SCAVENGER);
    }
    assert_eq!(
        alignment(&engine, &actor_id),
        CharacterAlignment::Lawful,
        "defeating the authored opponent leaves the character's alignment alone"
    );
}

#[test]
fn the_return_deadline_is_the_authored_delay_from_the_defeat() {
    let definition = definition();
    let (mut engine, actor_id) = create(&definition, 13);
    stand_on(&mut engine, &actor_id, SCAVENGER);
    while actor(&engine, &actor_id).is_alive() {
        wait(&mut engine, &actor_id);
    }
    let delay = authored_return_delay();
    let defeated_at = match actor(&engine, &actor_id).life_state {
        ActorLifeState::Ghost { defeated_at, .. } => defeated_at,
        ref other => panic!("ordinary death must create a ghost, not {other:?}"),
    };
    // Eligibility is derived from the defeat and the authored delay, not from
    // the actor's scheduling deadline: the character's own `ready_at` is a
    // different fact and may sit later in the same step.
    let deadline = defeated_at.saturating_add_millis(delay);
    engine
        .advance_to(LogicalTime::from_millis(deadline.as_millis() - 1))
        .expect("advancing to just before the authored deadline");
    assert!(
        engine
            .apply_actor_intent(&actor_id, PlayerIntent::RequestResurrection)
            .is_err(),
        "the return must not be available before the authored deadline"
    );
    engine
        .advance_to(deadline)
        .expect("advancing to the authored deadline");
    engine
        .apply_actor_intent(&actor_id, PlayerIntent::RequestResurrection)
        .expect("the return is available at the authored deadline");
    assert!(actor(&engine, &actor_id).is_alive());
}

/// The authored return delay, read from the served template rather than restated.
fn authored_return_delay() -> u64 {
    let template: WorldTemplateV4 = serde_json::from_value(read_json(
        "content/lands/first-expedition/generated/world_template.json",
    ))
    .expect("world template decodes");
    template
        .resurrection
        .get("first_expedition")
        .expect("the first expedition authors a return policy")
        .request_delay_ms
}
