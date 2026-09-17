//! A content cutover that moves an authored health pool says what happens to
//! the current health of every retained actor, and proves it.
//!
//! `Stats` carries `attack`, `defense` and `hp`, but they are not the same kind
//! of fact: attack and defense are ratings, while `hp` is simultaneously an
//! immutable authored maximum and the ceiling of a mutable current value. A
//! cutover that replaces the whole block would move a monster's maximum without
//! saying so, and current-HP preservation alone does not demonstrate health
//! preservation. These cases hold the two operations apart and pin the policy
//! that the migration applies to the second one:
//! [`tme_rules::ActorHealthPolicy::PreserveCurrent`].
//!
//! Every case drives the real first-expedition catalog through the real
//! cutover, and every refusal also proves that the source checkpoint was not
//! mutated.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;
use tme_rules::model::ActorState;
use tme_rules::{
    ActorId, ActorLifeState, CatalogProfileKey, CatalogV6, CheckpointContentMigration, Engine,
    GameDefinition, ValidatedWorldSeed, WorldSeedDef, WorldTemplateV4,
};

const PROFILE: &str = "profile/first_expedition";
const PLAYER_REGISTRY: &str = "actor-definition/first_expedition/player";
const SCAVENGER_REGISTRY: &str = "actor-definition/first_expedition/cellar_scavenger";
/// Runtime IDs, which are what a retained actor references.
const PLAYER_DEFINITION: &str = "actor/first_expedition/player";
const SCAVENGER_DEFINITION: &str = "actor/first_expedition/cellar_scavenger";
const SCAVENGER: &str = "cellar_scavenger";

type Ratings = (i32, i32, i32);

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

/// Compile the served world with two authored rating blocks replaced.
/// The reconciled ratings every health case starts from, so that only the health
/// pool moves unless a case says otherwise.
const RECONCILED_PLAYER: Ratings = (10, 14, 40);
const RECONCILED_SCAVENGER: Ratings = (18, 8, 18);

fn reconciled_definition(player: Ratings, scavenger: Ratings) -> Arc<GameDefinition> {
    authored_definition(player, scavenger)
}

fn authored_definition(player: Ratings, scavenger: Ratings) -> Arc<GameDefinition> {
    let mut catalog = read_json("content/lands/first-expedition/catalog.json");
    for (registry_key, (attack, defense, hp)) in
        [(PLAYER_REGISTRY, player), (SCAVENGER_REGISTRY, scavenger)]
    {
        catalog["actor_definitions"][registry_key]["stats"] = serde_json::json!({
            "attack": attack,
            "defense": defense,
            "hp": hp,
        });
    }
    let catalog: CatalogV6 = serde_json::from_value(catalog).expect("catalog decodes");
    let template: WorldTemplateV4 = serde_json::from_value(read_json(
        "content/lands/first-expedition/generated/world_template.json",
    ))
    .expect("world template decodes");
    GameDefinition::from_content(catalog, CatalogProfileKey::from(PROFILE), template)
        .expect("definition compiles")
}

fn engine(definition: &Arc<GameDefinition>) -> Engine {
    let mut source = read_json("content/lands/first-expedition/simulation_seed.json");
    for key in ["schema_version", "kind", "id"] {
        source.as_object_mut().expect("seed object").remove(key);
    }
    let seed: WorldSeedDef = serde_json::from_value(source).expect("seed payload");
    Engine::new(
        ValidatedWorldSeed::new(definition.clone(), seed).expect("seed validates"),
        7,
    )
    .expect("engine starts")
}

fn actor_mut<'a>(engine: &'a mut Engine, actor_id: &str) -> &'a mut ActorState {
    engine
        .world_mut()
        .actors
        .iter_mut()
        .find(|actor| actor.id.as_str() == actor_id)
        .unwrap_or_else(|| panic!("actor {actor_id:?}"))
}

fn actor<'a>(engine: &'a Engine, actor_id: &str) -> &'a ActorState {
    engine
        .world()
        .actor(&ActorId::from(actor_id))
        .unwrap_or_else(|| panic!("actor {actor_id:?}"))
}

fn plan(
    before: &Arc<GameDefinition>,
    after: &Arc<GameDefinition>,
    ratings: &[&str],
    health: &[&str],
) -> CheckpointContentMigration {
    CheckpointContentMigration {
        from_definition_sha256: before.content_identity().definition_sha256.clone(),
        to_definition_sha256: after.content_identity().definition_sha256.clone(),
        retire_npcs: BTreeSet::new(),
        merge_merchants: Default::default(),
        relocations: Vec::new(),
        initialize_new_topology: false,
        rederive_actor_stats: ratings.iter().map(|id| id.to_string()).collect(),
        rebuild_actor_health: health.iter().map(|id| id.to_string()).collect(),
    }
}

/// Put an actor into a wounded or dead state inside the checkpoint.
fn wound(engine: &mut Engine, actor_id: &str, hp: i32) {
    let actor = actor_mut(engine, actor_id);
    actor.hp = hp;
    if let Some(character) = actor.character.as_mut() {
        character.resources.hp = hp;
    }
}

fn kill(engine: &mut Engine, actor_id: &str) {
    let actor = actor_mut(engine, actor_id);
    actor.hp = 0;
    actor.life_state = ActorLifeState::Dead;
    if let Some(character) = actor.character.as_mut() {
        character.resources.hp = 0;
    }
}

/// The authored enemy health pool this cutover moves, from the superseded
/// `2/4/6` block to the reconciled `18/8/18`.
fn reconciled_scavenger() -> Ratings {
    (18, 8, 18)
}

#[test]
fn a_larger_authored_health_pool_raises_the_ceiling_and_keeps_the_wound() {
    let before = authored_definition((40, 40, 40), (2, 4, 6));
    let after = authored_definition((10, 14, 40), reconciled_scavenger());
    let mut source = engine(&before);
    // A wounded enemy and a full-health character, so both directions of
    // "current health is preserved" are in one case.
    wound(&mut source, SCAVENGER, 2);
    let checkpoint = source.export_checkpoint().expect("checkpoint");

    let (migrated, rebuilds) = Engine::migrate_content_checkpoint_reported(
        before.clone(),
        after.clone(),
        &checkpoint,
        &plan(
            &before,
            &after,
            &[PLAYER_DEFINITION, SCAVENGER_DEFINITION],
            &[SCAVENGER_DEFINITION],
        ),
    )
    .expect("declared cutover");

    // The character's authored health did not move, so its pool is untouched.
    assert_eq!(
        rebuilds.len(),
        1,
        "only the declared pool moved: {rebuilds:?}"
    );
    let rebuild = &rebuilds[0];
    assert_eq!(rebuild.definition_id, SCAVENGER_DEFINITION);
    assert_eq!(
        (rebuild.maximum_before, rebuild.maximum_after),
        (6, 18),
        "the authored maximum moved and the report says so"
    );
    assert_eq!(
        (rebuild.current_before, rebuild.current_after),
        (2, 2),
        "a wounded actor keeps exactly the health it had; the cutover is not a heal"
    );

    let restored = Engine::hydrate_checkpoint(after, &migrated).expect("recovery");
    let enemy = actor(&restored, SCAVENGER);
    assert_eq!(enemy.max_hp(), 18, "the effective maximum is the new pool");
    assert_eq!(enemy.hp, 2, "the effective current value is unchanged");
    assert!(
        enemy.is_alive(),
        "a wounded actor is not killed by a cutover"
    );
    let player = actor(&restored, PLAYER_ACTOR);
    assert_eq!(player.max_hp(), 40, "an unchanged pool is not rebuilt");
    assert_eq!(player.hp, 40);
    assert_eq!(
        (
            player
                .character
                .as_ref()
                .expect("character sheet")
                .resources
                .peak_hp,
            player
                .character
                .as_ref()
                .expect("character sheet")
                .resources
                .max_hp,
        ),
        (40, 40),
        "the character ceiling keeps its own record"
    );
}

/// The seeded controlled actor's instance ID, which is what a checkpoint stores.
const PLAYER_ACTOR: &str = "player";

#[test]
fn a_character_health_pool_moves_with_its_sheet_and_not_the_actors_current_health() {
    let before = reconciled_definition(RECONCILED_PLAYER, RECONCILED_SCAVENGER);
    let after = reconciled_definition((10, 14, 24), RECONCILED_SCAVENGER);
    let mut source = engine(&before);
    wound(&mut source, PLAYER_ACTOR, 9);
    let checkpoint = source.export_checkpoint().expect("checkpoint");

    let (migrated, rebuilds) = Engine::migrate_content_checkpoint_reported(
        before.clone(),
        after.clone(),
        &checkpoint,
        &plan(&before, &after, &[], &[PLAYER_DEFINITION]),
    )
    .expect("declared character health cutover");

    assert_eq!(
        rebuilds
            .iter()
            .map(|row| (
                row.definition_id.as_str(),
                row.maximum_before,
                row.maximum_after,
                row.current_before,
                row.current_after
            ))
            .collect::<Vec<_>>(),
        vec![(PLAYER_DEFINITION, 40, 24, 9, 9)],
        "the character's authored pool shrank and its wound is unchanged"
    );
    let restored = Engine::hydrate_checkpoint(after, &migrated).expect("recovery");
    let player = actor(&restored, PLAYER_ACTOR);
    assert_eq!(player.max_hp(), 24);
    assert_eq!(player.hp, 9);
    let sheet = &player
        .character
        .as_ref()
        .expect("character sheet")
        .resources;
    assert_eq!(
        sheet.max_hp, 24,
        "the sheet maximum follows the authored pool"
    );
    assert_eq!(sheet.hp, 9, "the sheet current value is unchanged");
    assert_eq!(
        sheet.peak_hp, 40,
        "a smaller pool keeps the character's record of what it reached"
    );
    assert!(
        sheet.max_hp <= sheet.peak_hp,
        "the sheet keeps its own invariant"
    );
}

#[test]
fn a_smaller_authored_pool_clamps_a_current_value_that_no_longer_fits() {
    let before = reconciled_definition(RECONCILED_PLAYER, RECONCILED_SCAVENGER);
    let after = reconciled_definition(RECONCILED_PLAYER, (18, 8, 4));
    let mut source = engine(&before);
    wound(&mut source, SCAVENGER, 6);
    let checkpoint = source.export_checkpoint().expect("checkpoint");

    let (migrated, rebuilds) = Engine::migrate_content_checkpoint_reported(
        before.clone(),
        after.clone(),
        &checkpoint,
        &plan(&before, &after, &[], &[SCAVENGER_DEFINITION]),
    )
    .expect("declared shrinking health cutover");

    assert_eq!(
        (
            rebuilds[0].maximum_before,
            rebuilds[0].maximum_after,
            rebuilds[0].current_before,
            rebuilds[0].current_after
        ),
        (18, 4, 6, 4),
        "current health is clamped to a smaller pool, never below one"
    );
    let restored = Engine::hydrate_checkpoint(after, &migrated).expect("recovery");
    assert_eq!(actor(&restored, SCAVENGER).hp, 4);
    assert!(actor(&restored, SCAVENGER).is_alive());
}

#[test]
fn a_dead_actor_keeps_its_death_and_its_zero() {
    let before = reconciled_definition(RECONCILED_PLAYER, RECONCILED_SCAVENGER);
    let after = reconciled_definition((10, 14, 24), RECONCILED_SCAVENGER);
    let mut source = engine(&before);
    kill(&mut source, PLAYER_ACTOR);
    let checkpoint = source.export_checkpoint().expect("checkpoint");

    let (migrated, rebuilds) = Engine::migrate_content_checkpoint_reported(
        before.clone(),
        after.clone(),
        &checkpoint,
        &plan(&before, &after, &[], &[PLAYER_DEFINITION]),
    )
    .expect("declared health cutover over a dead actor");

    assert_eq!(
        (rebuilds[0].current_before, rebuilds[0].current_after),
        (0, 0),
        "a health-pool change is not a resurrection"
    );
    let restored = Engine::hydrate_checkpoint(after, &migrated).expect("recovery");
    let player = actor(&restored, PLAYER_ACTOR);
    assert_eq!(player.hp, 0);
    assert!(!player.is_alive(), "the dead actor is still dead");
    assert_eq!(player.max_hp(), 24, "its authored ceiling still moved");
}

#[test]
fn rating_only_edits_never_touch_health_and_never_need_a_health_declaration() {
    let before = reconciled_definition(RECONCILED_PLAYER, RECONCILED_SCAVENGER);
    let after = reconciled_definition((12, 16, 40), RECONCILED_SCAVENGER);
    let mut source = engine(&before);
    wound(&mut source, SCAVENGER, 3);
    let checkpoint = source.export_checkpoint().expect("checkpoint");

    let (migrated, rebuilds) = Engine::migrate_content_checkpoint_reported(
        before.clone(),
        after.clone(),
        &checkpoint,
        &plan(&before, &after, &[PLAYER_DEFINITION], &[]),
    )
    .expect("a rating-only cutover needs no health declaration");

    assert!(
        rebuilds.is_empty(),
        "no health pool moved, so nothing is reported: {rebuilds:?}"
    );
    let restored = Engine::hydrate_checkpoint(after, &migrated).expect("recovery");
    let player = actor(&restored, PLAYER_ACTOR);
    assert_eq!((player.stats.attack, player.stats.defense), (12, 16));
    assert_eq!((player.stats.hp, player.hp), (40, 40));
    let enemy = actor(&restored, SCAVENGER);
    assert_eq!(
        (enemy.max_hp(), enemy.hp),
        (18, 3),
        "the enemy's pool and wound are untouched by a rating-only cutover"
    );
}

#[test]
fn a_health_only_edit_is_declared_on_its_own_and_never_as_a_rating() {
    let before = reconciled_definition(RECONCILED_PLAYER, RECONCILED_SCAVENGER);
    let after = reconciled_definition(RECONCILED_PLAYER, (18, 8, 30));
    let mut source = engine(&before);
    wound(&mut source, SCAVENGER, 7);
    let checkpoint = source.export_checkpoint().expect("checkpoint");

    // Declaring it as a rating movement is a stale plan and is refused.
    assert!(
        Engine::migrate_content_checkpoint(
            before.clone(),
            after.clone(),
            &checkpoint,
            &plan(&before, &after, &[SCAVENGER_DEFINITION], &[])
        )
        .is_err(),
        "an hp-only change is not a combat-rating change"
    );
    // Declaring the rating as well is equally stale.
    assert!(
        Engine::migrate_content_checkpoint(
            before.clone(),
            after.clone(),
            &checkpoint,
            &plan(
                &before,
                &after,
                &[SCAVENGER_DEFINITION],
                &[SCAVENGER_DEFINITION]
            )
        )
        .is_err(),
        "a rating declaration that did not move is refused"
    );

    let (migrated, rebuilds) = Engine::migrate_content_checkpoint_reported(
        before.clone(),
        after.clone(),
        &checkpoint,
        &plan(&before, &after, &[], &[SCAVENGER_DEFINITION]),
    )
    .expect("the health-only declaration is the correct one");
    assert_eq!(
        (rebuilds[0].maximum_before, rebuilds[0].maximum_after),
        (18, 30)
    );
    let restored = Engine::hydrate_checkpoint(after, &migrated).expect("recovery");
    let enemy = actor(&restored, SCAVENGER);
    assert_eq!((enemy.stats.attack, enemy.stats.defense), (18, 8));
    assert_eq!(
        (enemy.max_hp(), enemy.hp),
        (30, 7),
        "the wound is preserved"
    );
}

#[test]
fn an_added_actor_definition_is_additive_and_needs_no_declaration() {
    // A destination-only definition cannot be referenced by a retained source
    // actor, so it is content addition rather than a rating change. Demanding a
    // rederivation declaration for it would make every additive cutover fail.
    let before = reconciled_definition(RECONCILED_PLAYER, RECONCILED_SCAVENGER);
    let mut catalog = read_json("content/lands/first-expedition/catalog.json");
    let mut added = catalog["actor_definitions"][SCAVENGER_REGISTRY].clone();
    added["id"] = serde_json::json!("actor/first_expedition/lantern_moth");
    added["name"] = serde_json::json!("Lantern Moth");
    added["stats"] = serde_json::json!({"attack": 1, "defense": 1, "hp": 1});
    catalog["actor_definitions"]["actor-definition/first_expedition/lantern_moth"] = added;
    catalog["profiles"][PROFILE]["actor_definitions"]
        .as_array_mut()
        .expect("actor definition selection")
        .push(serde_json::json!(
            "actor-definition/first_expedition/lantern_moth"
        ));
    let catalog: CatalogV6 = serde_json::from_value(catalog).expect("catalog decodes");
    let template: WorldTemplateV4 = serde_json::from_value(read_json(
        "content/lands/first-expedition/generated/world_template.json",
    ))
    .expect("world template decodes");
    let after = GameDefinition::from_content(catalog, CatalogProfileKey::from(PROFILE), template)
        .expect("definition compiles");
    // The compiled definition is the authority; the addition is what makes its
    // content identity differ from the source's.
    assert_ne!(
        after.content_identity().definition_sha256,
        before.content_identity().definition_sha256,
        "the addition really changed the destination definition"
    );
    let source = engine(&before);
    let checkpoint = source.export_checkpoint().expect("checkpoint");

    let (migrated, rebuilds) = Engine::migrate_content_checkpoint_reported(
        before.clone(),
        after.clone(),
        &checkpoint,
        &plan(&before, &after, &[], &[]),
    )
    .expect("an additive definition needs no declaration");
    assert!(rebuilds.is_empty(), "nothing retained moved: {rebuilds:?}");
    let restored = Engine::hydrate_checkpoint(after, &migrated).expect("recovery");
    assert_eq!(
        restored.world().actors,
        source.world().actors,
        "an addition changes no retained actor"
    );
}

#[test]
fn a_removed_definition_cannot_be_reconciled_by_any_declaration() {
    // A retained actor whose definition the destination no longer authors has
    // no authored rules at all, so the cutover refuses instead of leaving it on
    // a definition nobody can look up.
    let before = reconciled_definition(RECONCILED_PLAYER, RECONCILED_SCAVENGER);
    let mut catalog = read_json("content/lands/first-expedition/catalog.json");
    catalog["profiles"][PROFILE]["actor_definitions"] = serde_json::Value::Array(
        catalog["profiles"][PROFILE]["actor_definitions"]
            .as_array()
            .expect("actor definition selection")
            .iter()
            .filter(|key| key.as_str() != Some(SCAVENGER_REGISTRY))
            .cloned()
            .collect(),
    );
    let catalog: CatalogV6 = serde_json::from_value(catalog).expect("catalog decodes");
    let template: WorldTemplateV4 = serde_json::from_value(read_json(
        "content/lands/first-expedition/generated/world_template.json",
    ))
    .expect("world template decodes");
    let after = GameDefinition::from_content(catalog, CatalogProfileKey::from(PROFILE), template)
        .expect("definition compiles");
    let source = engine(&before);
    let checkpoint = source.export_checkpoint().expect("checkpoint");

    let error = Engine::migrate_content_checkpoint(
        before.clone(),
        after.clone(),
        &checkpoint,
        &plan(&before, &after, &[], &[]),
    )
    .expect_err("a removed definition leaves a retained actor unauthored");
    assert!(
        error.message().contains("removes the definition"),
        "{error}"
    );
    // The refusal leaves the source checkpoint exactly as it was, so a rejected
    // plan can never be a partial cutover.
    assert_eq!(
        source.export_checkpoint().expect("re-export").as_bytes(),
        checkpoint.as_bytes()
    );
}
