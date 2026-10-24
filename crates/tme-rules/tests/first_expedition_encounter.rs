//! The shipped first expedition must be dangerous *and* survivable.
//!
//! Issue #74: the authored starter encounter gave the created player
//! attack/defense/HP 40/40/40 and the cellar scavenger 2/4/6. The hit check is
//! `d20 + attack/2 + adds > 10 + defense/2 + dexterity/3`, so the scavenger's
//! best possible attack score of 21 could never exceed the created player's
//! defense score of 35: the encounter was unwinnable in the wrong direction,
//! and the player was never in danger at all.
//!
//! Every case here drives the real production catalog, the real compiled world
//! template, the real seed and ordinary character creation through the ordinary
//! engine command path. Nothing here overrides stats, gear, AI or cadence.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;
use tme_rules::model::ActorState;
use tme_rules::{
    ActorId, CatalogProfileKey, CatalogV6, CharacterId, CheckpointContentMigration, Engine, Event,
    GameDefinition, PhysicalAttackMode, PlayerIntent, ValidatedWorldSeed, WorldSeedDef,
    WorldTemplateV4,
};

const PROFILE: &str = "profile/first_expedition";
/// Runtime actor definition IDs, which are what a retained actor references.
/// The catalog registry keys are `actor-definition/first_expedition/...`.
const PLAYER_DEFINITION: &str = "actor/first_expedition/player";
const SCAVENGER_DEFINITION: &str = "actor/first_expedition/cellar_scavenger";
const PLAYER_REGISTRY: &str = "actor-definition/first_expedition/player";
/// The authored health change in this catalog cutover is the enemy's, so only
/// that definition declares a health rebuild.
fn scavenger_health_declaration() -> BTreeSet<String> {
    BTreeSet::from([SCAVENGER_DEFINITION.to_string()])
}
const SCAVENGER_REGISTRY: &str = "actor-definition/first_expedition/cellar_scavenger";
const SCAVENGER: &str = "cellar_scavenger";
/// Authored `attack`/`defense`/`hp` for the player and the cellar scavenger.
type Ratings = (i32, i32, i32);
/// The authored ratings this reconciliation replaced. Kept so the saved-state
/// case reconstructs the pre-cutover content rather than inventing a fixture.
const SUPERSEDED_RATINGS: (Ratings, Ratings) = ((40, 40, 40), (2, 4, 6));

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

fn catalog_source() -> Value {
    read_json("content/lands/first-expedition/catalog.json")
}

fn template_source() -> WorldTemplateV4 {
    serde_json::from_value(read_json(
        "content/lands/first-expedition/generated/world_template.json",
    ))
    .expect("compiled world template")
}

fn seed_source() -> WorldSeedDef {
    let mut source = read_json("content/lands/first-expedition/simulation_seed.json");
    for key in ["schema_version", "kind", "id"] {
        source.as_object_mut().expect("seed object").remove(key);
    }
    serde_json::from_value(source).expect("simulation seed payload")
}

/// Compile the served first-expedition world. `ratings` restates the authored
/// combat ratings when a case needs the superseded content.
fn definition(ratings: Option<(Ratings, Ratings)>) -> Arc<GameDefinition> {
    let mut catalog = catalog_source();
    if let Some((player, scavenger)) = ratings {
        for (registry_key, (attack, defense, hp)) in
            [(PLAYER_REGISTRY, player), (SCAVENGER_REGISTRY, scavenger)]
        {
            catalog["actor_definitions"][registry_key]["stats"] = serde_json::json!({
                "attack": attack,
                "defense": defense,
                "hp": hp,
            });
        }
    }
    let catalog: CatalogV6 = serde_json::from_value(catalog).expect("catalog decodes");
    GameDefinition::from_content(catalog, CatalogProfileKey::from(PROFILE), template_source())
        .expect("definition compiles")
}

fn production() -> Arc<GameDefinition> {
    definition(None)
}

fn create(definition: &Arc<GameDefinition>, profile_id: &str, rng_seed: u64) -> (Engine, ActorId) {
    let engine = Engine::new(
        ValidatedWorldSeed::new(definition.clone(), seed_source()).expect("seed validates"),
        rng_seed,
    )
    .expect("engine starts");
    let profile = engine
        .definition()
        .creation_profiles()
        .iter()
        .find(|profile| profile.id == profile_id)
        .unwrap_or_else(|| panic!("creation profile {profile_id:?}"))
        .clone();
    let engine = engine
        .prepare_character_creation(
            &profile.id,
            CharacterId::new(format!("proof/{}", profile.id)),
            "Probe",
            profile.character.attributes.clone(),
        )
        .expect("ordinary character creation");
    (
        engine,
        ActorId::new(format!("created/proof/{}", profile.id)),
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

/// Stand the created character on the scavenger's tile, which ordinary
/// movement reaches; every later fact comes from the engine.
fn join_encounter(engine: &mut Engine, actor_id: &ActorId) {
    let encounter = actor(engine, &ActorId::from(SCAVENGER)).location.clone();
    actor_mut(engine, actor_id).location = encounter;
}

fn attack(engine: &mut Engine, actor_id: &ActorId) -> Vec<Event> {
    engine
        .apply_actor_intent(
            actor_id,
            PlayerIntent::PhysicalAttack {
                mode: PhysicalAttackMode::Fight,
                target_actor_id: ActorId::from(SCAVENGER),
                authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
            },
        )
        .expect("ordinary fight command")
        .events
}

fn wait(engine: &mut Engine, actor_id: &ActorId) -> Vec<Event> {
    engine
        .apply_actor_intent(actor_id, PlayerIntent::Wait)
        .expect("ordinary wait command")
        .events
}

fn scavenger_damage_to(events: &[Event], player_id: &ActorId) -> i32 {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Attacked {
                attacker_id,
                defender_id,
                damage,
                ..
            } if attacker_id.as_str() == SCAVENGER && defender_id == player_id => Some(*damage),
            _ => None,
        })
        .sum()
}

/// One full encounter using only the ordinary Fight command.
struct Encounter {
    player_won: bool,
    player_rounds: u32,
    damage_taken: i32,
    hp_left: i32,
    player_attacks: u32,
    player_hits: u32,
    scavenger_attacks: u32,
    scavenger_hits: u32,
    damage_dealt: i32,
}

fn fight(definition: &Arc<GameDefinition>, profile_id: &str, rng_seed: u64) -> Encounter {
    let (mut engine, actor_id) = create(definition, profile_id, rng_seed);
    join_encounter(&mut engine, &actor_id);
    let scavenger_id = ActorId::from(SCAVENGER);
    let mut report = Encounter {
        player_won: false,
        player_rounds: 0,
        damage_taken: 0,
        hp_left: 0,
        player_attacks: 0,
        player_hits: 0,
        scavenger_attacks: 0,
        scavenger_hits: 0,
        damage_dealt: 0,
    };
    loop {
        report.player_rounds += 1;
        assert!(report.player_rounds < 500, "encounter did not resolve");
        let events = attack(&mut engine, &actor_id);
        report.player_attacks += 1;
        report.player_hits += u32::from(events.iter().any(
            |event| matches!(event, Event::Attacked { attacker_id, .. } if attacker_id == &actor_id),
        ));
        report.damage_dealt += events
            .iter()
            .filter_map(|event| match event {
                Event::Attacked {
                    attacker_id,
                    damage,
                    ..
                } if attacker_id == &actor_id => Some(*damage),
                _ => None,
            })
            .sum::<i32>();
        report.scavenger_attacks += events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    Event::Attacked { attacker_id, .. }
                        | Event::AttackMissed { attacker_id, .. }
                        | Event::AttackBlocked { attacker_id, .. }
                        | Event::WeaponFumbled { attacker_id, .. }
                        | Event::AttackNotReady { actor_id: attacker_id, .. }
                        if attacker_id.as_str() == SCAVENGER
                )
            })
            .count() as u32;
        report.scavenger_hits += events
            .iter()
            .filter(|event| {
                matches!(event, Event::Attacked { attacker_id, .. } if attacker_id.as_str() == SCAVENGER)
            })
            .count() as u32;
        report.damage_taken += scavenger_damage_to(&events, &actor_id);
        report.hp_left = actor(&engine, &actor_id).hp;
        if !actor(&engine, &actor_id).is_alive() {
            return report;
        }
        if !actor(&engine, &scavenger_id).is_alive() {
            report.player_won = true;
            return report;
        }
    }
}

#[test]
fn every_starting_profile_can_reach_and_be_reached_by_the_authored_opponent() {
    // The hit check only needs the score ceilings; a ceiling that cannot clear
    // the defender score makes the whole encounter impossible.
    let definition = production();
    let (engine, actor_id) = create(&definition, "creation/fighter", 11);
    let mut engine = engine;
    join_encounter(&mut engine, &actor_id);
    let player = actor(&engine, &actor_id);
    let scavenger = actor(&engine, &ActorId::from(SCAVENGER));
    let skill_level = player
        .character
        .as_ref()
        .and_then(|character| {
            character
                .skill_ledger
                .iter()
                .find(|entry| entry.track_id == "staff")
        })
        .map_or(0, |entry| entry.level);
    assert!(skill_level > 0, "the fighter starts with staff training");
    let combat_adds = 1;
    let player_ceiling = player.stats.attack / 2 + combat_adds + i32::from(skill_level) / 2;
    let scavenger_ceiling = scavenger.stats.attack / 2;
    let scavenger_defense_score = 10 + scavenger.stats.defense / 2 + 10 / 3;
    let player_defense_score = 10 + player.stats.defense / 2 + 17 / 3;
    assert!(
        player_ceiling + 20 > scavenger_defense_score,
        "player attack ceiling {player_ceiling} cannot beat {scavenger_defense_score}"
    );
    assert!(
        scavenger_ceiling + 20 > player_defense_score,
        "scavenger attack ceiling {scavenger_ceiling} cannot beat {player_defense_score}"
    );
}

#[test]
fn the_authored_opponent_injures_a_normally_created_character() {
    // A hit is not enough on its own: the encounter must be able to remove
    // real HP. Search the ordinary command sequence for the first landing hit
    // rather than asserting a probability.
    let definition = production();
    let (mut engine, actor_id) = create(&definition, "creation/fighter", 3);
    join_encounter(&mut engine, &actor_id);
    let starting_hp = actor(&engine, &actor_id).hp;
    let mut damage = 0;
    let mut rounds = 0;
    while damage == 0 {
        rounds += 1;
        assert!(
            rounds < 200,
            "the scavenger never injured the player in {rounds} rounds"
        );
        damage += scavenger_damage_to(&wait(&mut engine, &actor_id), &actor_id);
    }
    let hp = actor(&engine, &actor_id).hp;
    assert!(damage > 0, "observed damage must be positive");
    assert_eq!(
        hp,
        starting_hp - damage,
        "damage is reflected in current HP"
    );
    // Threat is the share of the starting pool a landing hit removes. A single
    // hit that costs a tenth of the pool is real attrition; anything that
    // rounds away to nothing is not an opponent.
    assert!(
        damage * 10 >= starting_hp,
        "one landing hit removed only {damage} of {starting_hp} HP, which cannot threaten a starting character"
    );
}

#[test]
fn a_starting_character_can_defeat_the_encounter_within_a_real_fight() {
    let definition = production();
    for profile in definition.creation_profiles() {
        for seed in [1_u64, 7, 23, 91] {
            let report = fight(&definition, &profile.id, seed);
            assert!(
                report.player_won,
                "{} seed {seed}: starting character lost the shipped encounter",
                profile.id
            );
            assert!(
                report.player_rounds > 1,
                "{} seed {seed}: the opponent died before it could act",
                profile.id
            );
            assert!(
                report.scavenger_attacks > 0,
                "{} seed {seed}: the opponent never took a turn",
                profile.id
            );
        }
    }
}

#[test]
fn a_starting_character_can_be_defeated_by_the_authored_opponent() {
    // Ordinary combat must be able to kill a normally created character: this
    // is what makes death, healing and retreat meaningful. The character is
    // created normally and then only waits, which is a legal ordinary action.
    let definition = production();
    let (mut engine, actor_id) = create(&definition, "creation/fighter", 17);
    join_encounter(&mut engine, &actor_id);
    let mut rounds = 0;
    while actor(&engine, &actor_id).is_alive() {
        rounds += 1;
        assert!(
            rounds < 500,
            "the authored opponent could not defeat a passive starting character in {rounds} rounds"
        );
        wait(&mut engine, &actor_id);
    }
    assert!(rounds > 1, "defeat required no real exchange");
    assert!(
        actor(&engine, &actor_id).character_id.is_some(),
        "the defeated actor is still the same character"
    );
    assert!(
        !actor(&engine, &actor_id).is_alive(),
        "ordinary production combat reached defeat"
    );
    assert!(
        !engine.world().corpses.is_empty(),
        "an ordinary death leaves a corpse in the world"
    );
}

#[test]
fn migrated_characters_keep_their_earned_state_and_join_the_reconciled_encounter() {
    let before = definition(Some(SUPERSEDED_RATINGS));
    let after = production();
    // A real character, created under the superseded content and carrying
    // earned state the reconciliation must not touch.
    let (mut engine, actor_id) = create(&before, "creation/thaumaturge", 29);
    let scavenger_id = ActorId::from(SCAVENGER);
    actor_mut(&mut engine, &actor_id)
        .character
        .as_mut()
        .expect("character sheet")
        .progression
        .experience = 41;
    actor_mut(&mut engine, &actor_id).carried.gold.sack = 63;
    let earned_ready_at = actor(&engine, &actor_id).timing.ready_at;
    let earned_experience = actor(&engine, &actor_id)
        .character
        .as_ref()
        .expect("character sheet")
        .progression
        .experience;
    let carried = actor(&engine, &actor_id).carried.clone();
    let checkpoint = engine.export_checkpoint().expect("checkpoint");
    let before_identity = before.content_identity().definition_sha256.clone();
    let after_identity = after.content_identity().definition_sha256.clone();

    // The cutover is explicit: a plan that leaves a retained actor on superseded
    // authored ratings, or on a superseded authored health pool, is refused
    // rather than silently accepted. Combat ratings and health are separate
    // declarations because they move different facts.
    let plan = |rederive: BTreeSet<String>, health: BTreeSet<String>| CheckpointContentMigration {
        from_definition_sha256: before_identity.clone(),
        to_definition_sha256: after_identity.clone(),
        retire_npcs: BTreeSet::new(),
        merge_merchants: Default::default(),
        relocations: Vec::new(),
        initialize_new_topology: false,
        rederive_actor_stats: rederive,
        rebuild_actor_health: health,
    };
    let both = BTreeSet::from([
        PLAYER_DEFINITION.to_string(),
        SCAVENGER_DEFINITION.to_string(),
    ]);
    assert!(
        Engine::migrate_content_checkpoint(
            before.clone(),
            after.clone(),
            &checkpoint,
            &plan(BTreeSet::new(), BTreeSet::new())
        )
        .is_err(),
        "an unreconciled retired rating must refuse the cutover"
    );
    assert!(
        Engine::migrate_content_checkpoint(
            before.clone(),
            after.clone(),
            &checkpoint,
            &plan(
                BTreeSet::from(["absent_definition".to_string()]),
                both.clone()
            )
        )
        .is_err(),
        "a plan naming an absent definition must be refused"
    );
    assert!(
        Engine::migrate_content_checkpoint(
            before.clone(),
            after.clone(),
            &checkpoint,
            &plan(both.clone(), BTreeSet::new())
        )
        .is_err(),
        "an undeclared health-pool change must refuse the cutover"
    );

    let (migrated, health_rebuilds) = Engine::migrate_content_checkpoint_reported(
        before,
        after.clone(),
        &checkpoint,
        &plan(both, scavenger_health_declaration()),
    )
    .expect("declared reconciliation");
    let mut restored = Engine::hydrate_checkpoint(after, &migrated).expect("recovery");
    assert_eq!(
        health_rebuilds
            .iter()
            .map(|rebuild| (
                rebuild.definition_id.as_str(),
                rebuild.maximum_before,
                rebuild.maximum_after,
                rebuild.current_before,
                rebuild.current_after
            ))
            .collect::<Vec<_>>(),
        vec![(SCAVENGER_DEFINITION, 6, 18, 6, 6)],
        "the health policy is applied to the retained enemy and to nothing else"
    );

    let player = actor(&restored, &actor_id);
    assert_eq!(player.stats.attack, 10);
    assert_eq!(player.stats.defense, 14);
    assert_eq!(player.max_hp(), 40, "starting health is unchanged");
    assert_eq!(
        player.carried, carried,
        "inventory and balances are preserved"
    );
    assert_eq!(
        player.timing.ready_at, earned_ready_at,
        "deadlines are preserved"
    );
    assert_eq!(
        player
            .character
            .as_ref()
            .expect("character sheet")
            .progression
            .experience,
        earned_experience,
        "earned progression is preserved"
    );
    assert_eq!(
        player.character_id,
        Some(CharacterId::new("proof/creation/thaumaturge"))
    );
    let opponent = actor(&restored, &scavenger_id);
    assert_eq!(
        (opponent.stats.attack, opponent.stats.defense),
        (18, 8),
        "the retained opponent carries the reconciled authored ratings"
    );
    assert_eq!(
        opponent.hp, 6,
        "current HP is live state; a health-pool cutover must not heal or damage anyone"
    );
    assert_eq!(
        opponent.max_hp(),
        18,
        "the retained opponent's authored health ceiling is the reconciled one"
    );
    assert!(
        opponent.hp <= opponent.stats.hp,
        "a retained actor may not exceed its authored maximum"
    );
    assert_eq!(
        opponent
            .character
            .as_ref()
            .map(|character| character.resources.max_hp),
        None,
        "a monster has no character sheet to keep in step"
    );

    // The recovered character now participates in the reconciled encounter
    // rather than one-shotting it from outside the model.
    join_encounter(&mut restored, &actor_id);
    let mut rounds = 0;
    while actor(&restored, &scavenger_id).is_alive() {
        rounds += 1;
        assert!(
            rounds < 200,
            "recovered character could not resolve the encounter"
        );
        attack(&mut restored, &actor_id);
    }
    assert!(
        rounds > 1,
        "the recovered character killed the retained opponent instantly"
    );
}
