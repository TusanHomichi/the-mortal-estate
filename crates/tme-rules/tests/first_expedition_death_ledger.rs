//! Nothing is created or destroyed at the first expedition's death boundary.
//!
//! Issue #80's composed journey proves the returned character's inventory and
//! purse with read-only checkpoint evidence, but the reviewed browser oracle
//! only asserted that nothing *increased*: an empty inventory and an empty purse
//! passed it, and so did deleting items, substituting one identity for another
//! with the same count, or duplicating a stack. This file is the shared owner of
//! the real audit. It reads the engine's own typed [`ItemLocation`] for every
//! item instance the world owns — which is exactly one location per instance, so
//! "single owner" is checked rather than assumed — and the whole world's gold,
//! and it names every way the ledger can fail.
//!
//! The cases drive the shipped first-expedition catalog and seed through
//! ordinary character creation and ordinary combat. The opponent's production
//! scavenging stays enabled: an item legitimately taken by the monster is
//! expected to be *with the monster*, and no case may demand it come back to the
//! player. What may never happen is an item or a coin that stops existing, is
//! duplicated, changes identity, or moves without a recorded reason.
//!
//! Every assertion about a real death is paired with a deliberate negative
//! control over the same ledger, so the oracle itself is proven to reject loss,
//! duplication, identity replacement, quantity changes and currency creation or
//! destruction rather than merely being present.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;
use tme_rules::model::ActorState;
use tme_rules::{
    ActorId, CatalogProfileKey, CatalogV6, CharacterId, Engine, Event, GameDefinition, ItemHolderId,
    ItemLocation, PhysicalAttackMode, PlayerIntent, ValidatedWorldSeed, WorldSeedDef,
    WorldTemplateV4,
};

const PROFILE: &str = "profile/first_expedition";
const SCAVENGER: &str = "cellar_scavenger";
const PLAYER_DEFINITION: &str = "actor/first_expedition/player";

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

/// A normally created wizard, which is the starting profile that carries two
/// items and a purse rather than one item.
fn create(definition: &Arc<GameDefinition>, rng_seed: u64) -> (Engine, ActorId, CharacterId) {
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
        character_id,
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

fn join_encounter(engine: &mut Engine, actor_id: &ActorId) {
    let encounter = actor(engine, &ActorId::from(SCAVENGER)).location.clone();
    actor_mut(engine, actor_id).location = encounter;
}

fn wait(engine: &mut Engine, actor_id: &ActorId) -> Vec<Event> {
    engine
        .apply_actor_intent(actor_id, PlayerIntent::Wait)
        .expect("ordinary wait command")
        .events
}

/// One item instance's immutable identity and its one authoritative location.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Holding {
    definition_id: String,
    quantity: u32,
    location: ItemLocation,
}

/// Every item the world owns, by instance ID, and every coin it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ledger {
    items: BTreeMap<String, Holding>,
    gold: BTreeMap<String, i64>,
}

impl Ledger {
    /// Every item instance's single typed location, read from the engine.
    ///
    /// `Engine::item_location` refuses an instance that is in no collection or
    /// in more than one, so this loop is the single-ownership check: a lost item
    /// or a duplicated one cannot be read as a healthy ledger at all.
    fn read(engine: &Engine) -> Self {
        let mut items = BTreeMap::new();
        for (item_instance_id, instance) in &engine.world().item_instances {
            let location = engine
                .item_location(item_instance_id)
                .unwrap_or_else(|error| {
                    panic!("item {item_instance_id:?} has no single owner: {error}")
                });
            items.insert(
                item_instance_id.clone(),
                Holding {
                    definition_id: instance.definition_id.clone(),
                    quantity: instance.quantity,
                    location,
                },
            );
        }
        let mut gold = BTreeMap::new();
        for actor in &engine.world().actors {
            for (label, amount) in [
                ("left_hand", actor.carried.gold.left_hand),
                ("right_hand", actor.carried.gold.right_hand),
                ("sack", actor.carried.gold.sack),
            ] {
                if amount != 0 {
                    gold.insert(format!("actor:{}:{label}", actor.id.as_str()), amount);
                }
            }
        }
        for (corpse_id, corpse) in &engine.world().corpses {
            if corpse.gold != 0 {
                gold.insert(format!("corpse:{}", corpse_id.as_str()), corpse.gold);
            }
        }
        for (pile_id, pile) in &engine.world().ground_gold {
            if pile.amount != 0 {
                gold.insert(format!("ground:{}", pile_id.as_str()), pile.amount);
            }
        }
        for (bank_id, bank) in &engine.world().banks {
            for (character_id, balance) in &bank.balances {
                if *balance != 0 {
                    gold.insert(
                        format!("bank:{}:{}", bank_id.as_str(), character_id.as_str()),
                        *balance,
                    );
                }
            }
        }
        Self { items, gold }
    }

    fn gold_total(&self) -> i64 {
        self.gold.values().sum()
    }

    fn carried_by(&self, holder: &ItemHolderId, item_instance_id: &str) -> bool {
        matches!(
            self.items.get(item_instance_id),
            Some(Holding { location: ItemLocation::Carried { holder: found, .. }, .. })
                if found == holder
        )
    }

    fn location_label(location: &ItemLocation) -> String {
        match location {
            ItemLocation::Ground { position } => format!("ground:{}", position_label(position)),
            ItemLocation::Carried { holder, position } => {
                format!("carried:{holder:?}:{position:?}")
            }
            ItemLocation::Corpse { corpse_id, .. } => format!("corpse:{}", corpse_id.as_str()),
            ItemLocation::Merchant { inventory_id } => format!(
                "merchant:{}:{}",
                inventory_id.service_id, inventory_id.capability_id
            ),
            ItemLocation::Locker {
                vault_id,
                owner_character_id,
            } => format!("locker:{}:{}", vault_id.as_str(), owner_character_id.as_str()),
            ItemLocation::Offered {
                sender_character_id,
                recipient_character_id,
                ..
            } => format!(
                "offered:{}:{}",
                sender_character_id.as_str(),
                recipient_character_id.as_str()
            ),
        }
    }
}

fn position_label(position: &tme_rules::WorldPosition) -> String {
    format!(
        "{}/{}/{},{}",
        position.realm, position.level, position.position.x, position.position.y
    )
}

/// Every way the ledger can fail to be the same ledger, named.
///
/// This is the oracle the browser half's weaker count comparison was missing: it
/// fails on deliberate loss, duplication, identity replacement and quantity
/// change, and it fails on currency created or destroyed anywhere in the world.
fn audit(before: &Ledger, after: &Ledger) -> Vec<String> {
    let mut defects = Vec::new();
    let lost = before
        .items
        .keys()
        .filter(|id| !after.items.contains_key(*id))
        .cloned()
        .collect::<Vec<_>>();
    if !lost.is_empty() {
        defects.push(format!("item instances stopped existing: {lost:?}"));
    }
    let created = after
        .items
        .keys()
        .filter(|id| !before.items.contains_key(*id))
        .cloned()
        .collect::<Vec<_>>();
    if !created.is_empty() {
        defects.push(format!("item instances appeared from nowhere: {created:?}"));
    }
    for (item_instance_id, source) in &before.items {
        let Some(destination) = after.items.get(item_instance_id) else {
            continue;
        };
        if source.definition_id != destination.definition_id {
            defects.push(format!(
                "item instance {item_instance_id:?} changed identity: {} -> {}",
                source.definition_id, destination.definition_id
            ));
        }
        if source.quantity != destination.quantity {
            defects.push(format!(
                "item instance {item_instance_id:?} changed quantity: {} -> {}",
                source.quantity, destination.quantity
            ));
        }
    }
    if before.gold_total() != after.gold_total() {
        defects.push(format!(
            "gold was created or destroyed: {} -> {}",
            before.gold_total(),
            after.gold_total()
        ));
    }
    defects
}

/// The item instances one actor carries, in any position.
fn carried_items(ledger: &Ledger, holder: &ItemHolderId) -> BTreeSet<String> {
    ledger
        .items
        .iter()
        .filter(|(_, holding)| {
            matches!(&holding.location, ItemLocation::Carried { holder: found, .. } if found == holder)
        })
        .map(|(id, _)| id.clone())
        .collect()
}

#[test]
fn an_ordinary_death_conserves_every_item_and_coin_and_accounts_for_each_one() {
    let definition = definition();
    let (mut engine, actor_id, _) = create(&definition, 5);
    join_encounter(&mut engine, &actor_id);
    let scavenger = ActorId::from(SCAVENGER);
    let scavenger_holder = actor(&engine, &scavenger).item_holder_id();
    let player_holder = actor(&engine, &actor_id).item_holder_id();
    let before = Ledger::read(&engine);
    let carried_before = carried_items(&before, &player_holder);
    assert!(
        carried_before.len() >= 2,
        "the wizard starts with equipment to conserve: {carried_before:?}"
    );
    assert!(
        before.gold_total() > 0,
        "the created character holds a purse to conserve"
    );
    let death_location = actor(&engine, &actor_id).location.clone();

    let mut rounds = 0;
    while actor(&engine, &actor_id).is_alive() {
        rounds += 1;
        assert!(rounds < 500, "the authored opponent never won");
        wait(&mut engine, &actor_id);
    }
    let after = Ledger::read(&engine);

    // The whole world's ledger, before and after: nothing lost, nothing created,
    // nothing renamed, no stack resized, no coin minted or burned.
    assert_eq!(audit(&before, &after), Vec::<String>::new());
    assert_eq!(
        before.gold_total(),
        after.gold_total(),
        "every coin still exists somewhere in the world"
    );

    // The ghost holds nothing: death moved its equipment out of its hands.
    assert!(
        carried_items(&after, &player_holder).is_empty(),
        "a dead character still carries items: {:?}",
        carried_items(&after, &player_holder)
    );
    // Every item the character carried is now in a real place: the corpse it
    // left, the ground beneath it, or the monster that took it. Production
    // scavenging is enabled, so the monster taking something is the expected
    // outcome and is asserted as such — never as an item that vanished.
    let mut with_monster = BTreeSet::new();
    for item_instance_id in &carried_before {
        let holding = after
            .items
            .get(item_instance_id)
            .unwrap_or_else(|| panic!("{item_instance_id:?} stopped existing"));
        if after.carried_by(&scavenger_holder, item_instance_id) {
            with_monster.insert(item_instance_id.clone());
            continue;
        }
        let label = Ledger::location_label(&holding.location);
        assert!(
            label.starts_with("corpse:") || label.starts_with("ground:"),
            "{item_instance_id:?} left the character for {label}, which is neither its corpse, \
             the ground beneath it, nor the opponent"
        );
    }
    // Every item the character carried is accounted for exactly once: with the
    // monster that took it, or at the death site. The two sets together are the
    // whole of what the character held, which is the accounting the count
    // comparison could not make.
    let at_death_site = carried_before
        .iter()
        .filter(|item_instance_id| !with_monster.contains(*item_instance_id))
        .count();
    assert_eq!(
        with_monster.len() + at_death_site,
        carried_before.len(),
        "each carried item has exactly one of the two destinations"
    );
    assert!(
        !engine.world().corpses.is_empty(),
        "an ordinary character death leaves a corpse to hold what was retained"
    );
    assert_eq!(
        death_location,
        actor(&engine, &scavenger).location,
        "the exchange happened where the character stood"
    );
}

#[test]
fn the_death_relocations_are_the_recorded_ones_and_nothing_else_moves() {
    // The audit above says the ledger balances. This says *why* it moved: every
    // relocation the engine reported is a relocation the ledger shows, and no
    // other item changed hands.
    let definition = definition();
    let (mut engine, actor_id, _) = create(&definition, 5);
    join_encounter(&mut engine, &actor_id);
    let before = Ledger::read(&engine);
    let mut relocations = BTreeMap::new();
    let mut rounds = 0;
    while actor(&engine, &actor_id).is_alive() {
        rounds += 1;
        assert!(rounds < 500, "the authored opponent never won");
        for event in wait(&mut engine, &actor_id) {
            if let Event::ItemRelocated {
                item_instance_id,
                from,
                to,
                ..
            } = event
            {
                relocations.insert(item_instance_id, (format!("{from:?}"), format!("{to:?}")));
            }
        }
    }
    let after = Ledger::read(&engine);
    assert_eq!(audit(&before, &after), Vec::<String>::new());
    for (item_instance_id, holding) in &after.items {
        let source = before.items.get(item_instance_id).expect("audited above");
        if source.location == holding.location {
            assert!(
                !relocations.contains_key(item_instance_id),
                "{item_instance_id:?} was reported as relocated but never moved"
            );
        } else {
            assert!(
                relocations.contains_key(item_instance_id),
                "{item_instance_id:?} moved to {} with no recorded relocation",
                Ledger::location_label(&holding.location)
            );
        }
    }
}

#[test]
fn the_ledger_oracle_refuses_loss_duplication_replacement_and_minted_gold() {
    // Negative controls over a real death boundary: the same two ledgers, each
    // tampered one way. If any of these passes, the audit above proves nothing.
    let definition = definition();
    let (mut engine, actor_id, _) = create(&definition, 5);
    join_encounter(&mut engine, &actor_id);
    let before = Ledger::read(&engine);
    let mut rounds = 0;
    while actor(&engine, &actor_id).is_alive() {
        rounds += 1;
        assert!(rounds < 500, "the authored opponent never won");
        wait(&mut engine, &actor_id);
    }
    let after = Ledger::read(&engine);
    assert_eq!(
        audit(&before, &after),
        Vec::<String>::new(),
        "the untampered boundary must be clean before the controls are meaningful"
    );

    let victim = after.items.keys().next().cloned().expect("an item exists");

    let mut loss = after.clone();
    loss.items.remove(&victim);
    assert!(
        audit(&before, &loss)
            .iter()
            .any(|defect| defect.contains("stopped existing")),
        "an item that stops existing must fail the oracle"
    );

    let mut duplication = after.clone();
    duplication
        .items
        .insert("duplicate/proof".to_string(), after.items[&victim].clone());
    assert!(
        audit(&before, &duplication)
            .iter()
            .any(|defect| defect.contains("appeared from nowhere")),
        "a duplicated instance must fail the oracle"
    );

    let mut replacement = after.clone();
    replacement
        .items
        .get_mut(&victim)
        .expect("the victim exists")
        .definition_id = "item/substituted".to_string();
    assert!(
        audit(&before, &replacement)
            .iter()
            .any(|defect| defect.contains("changed identity")),
        "a same-count identity substitution must fail the oracle"
    );

    let mut resized = after.clone();
    resized.items.get_mut(&victim).expect("the victim exists").quantity += 1;
    assert!(
        audit(&before, &resized)
            .iter()
            .any(|defect| defect.contains("changed quantity")),
        "a stack that grows must fail the oracle"
    );

    let mut minted = after.clone();
    minted
        .gold
        .insert("ground:forged".to_string(), after.gold_total() + 1);
    assert!(
        audit(&before, &minted)
            .iter()
            .any(|defect| defect.contains("created or destroyed")),
        "minted currency must fail the oracle"
    );

    let mut burned = after.clone();
    *burned.gold.values_mut().next().expect("gold exists") -= 1;
    assert!(
        audit(&before, &burned)
            .iter()
            .any(|defect| defect.contains("created or destroyed")),
        "destroyed currency must fail the oracle"
    );
}

#[test]
fn an_empty_purse_and_an_empty_inventory_do_not_pass_the_oracle() {
    // The reviewed browser guard accepted `items.length <= items.length` and
    // `gold <= gold`, so an empty inventory and an empty purse passed it. The
    // audit is anchored to the instances themselves, so emptying both fails.
    let definition = definition();
    let (mut engine, actor_id, _) = create(&definition, 5);
    join_encounter(&mut engine, &actor_id);
    let before = Ledger::read(&engine);
    assert!(!before.items.is_empty());
    let emptied = Ledger {
        items: BTreeMap::new(),
        gold: BTreeMap::new(),
    };
    let defects = audit(&before, &emptied);
    assert!(
        defects.iter().any(|defect| defect.contains("stopped existing")),
        "an emptied world must fail the oracle: {defects:?}"
    );
    assert!(
        defects
            .iter()
            .any(|defect| defect.contains("created or destroyed")),
        "an emptied purse must fail the oracle: {defects:?}"
    );
}

#[test]
fn a_never_created_character_has_no_ledger_to_conserve() {
    // The audit is anchored to what exists, not to a count, so it can also state
    // the precondition the composed journey relies on: before ordinary creation
    // the world is untouched, and after it the new character's own items and
    // purse are the only additions.
    let definition = definition();
    let seeded = Engine::new(
        ValidatedWorldSeed::new(definition.clone(), seed_source()).expect("seed validates"),
        5,
    )
    .expect("engine starts");
    let before = Ledger::read(&seeded);
    let (engine, actor_id, character_id) = create(&definition, 5);
    let after = Ledger::read(&engine);
    let holder = ItemHolderId::Character(character_id);
    let carried = carried_items(&after, &holder);
    assert_eq!(
        carried.len(),
        2,
        "the wizard's creation profile grants exactly its authored equipment"
    );
    for item_instance_id in &carried {
        assert!(
            !before.items.contains_key(item_instance_id),
            "{item_instance_id:?} existed before the character did"
        );
    }
    assert_eq!(
        after.gold_total(),
        before.gold_total() + actor(&engine, &actor_id).carried.gold.sack,
        "ordinary creation is the only coin source in this window"
    );
    assert!(
        actor(&engine, &actor_id).definition_id == PLAYER_DEFINITION,
        "the created character uses the authored player definition"
    );
}

#[test]
fn a_ghost_may_not_hold_an_item_after_its_death() {
    // Single ownership is read from the engine, so this also documents the
    // location a ghost's equipment may *not* have.
    let definition = definition();
    let (mut engine, actor_id, character_id) = create(&definition, 5);
    join_encounter(&mut engine, &actor_id);
    let holder = ItemHolderId::Character(character_id);
    let mut rounds = 0;
    while actor(&engine, &actor_id).is_alive() {
        rounds += 1;
        assert!(rounds < 500, "the authored opponent never won");
        wait(&mut engine, &actor_id);
    }
    let ledger = Ledger::read(&engine);
    assert!(
        carried_items(&ledger, &holder).is_empty(),
        "the ghost is still carrying {:?}",
        carried_items(&ledger, &holder)
    );
    assert_eq!(
        actor(&engine, &actor_id).carried.gold.sack, 0,
        "the ghost's own purse is empty; the coins are in the world's ledger"
    );
    let corpse = engine
        .world()
        .corpses
        .values()
        .find(|corpse| corpse.origin_actor_id == actor_id)
        .expect("the character's corpse is the holder of what the death retained");
    match &actor(&engine, &actor_id).life_state {
        tme_rules::ActorLifeState::Ghost { corpse_id, .. } => assert_eq!(
            corpse_id, &corpse.id,
            "the ghost is bound to the corpse that holds its retained items"
        ),
        other => panic!("an ordinary character death leaves a ghost, not {other:?}"),
    }
}

#[test]
fn the_encounter_can_also_be_won_without_touching_the_ledger() {
    // The same audit over a victory: production scavenging must not mint or
    // destroy anything when the monster dies instead.
    let definition = definition();
    let (mut engine, actor_id, _) = create(&definition, 23);
    join_encounter(&mut engine, &actor_id);
    let before = Ledger::read(&engine);
    let scavenger = ActorId::from(SCAVENGER);
    let mut rounds = 0;
    while actor(&engine, &scavenger).is_alive() {
        rounds += 1;
        assert!(rounds < 500, "the encounter did not resolve");
        engine
            .apply_actor_intent(
                &actor_id,
                PlayerIntent::PhysicalAttack {
                    mode: PhysicalAttackMode::Fight,
                    target_actor_id: scavenger.clone(),
                    authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
                },
            )
            .expect("ordinary fight command");
    }
    let after = Ledger::read(&engine);
    assert_eq!(audit(&before, &after), Vec::<String>::new());
    assert_eq!(
        before.gold_total(),
        after.gold_total(),
        "a victory mints no coins either"
    );
}
