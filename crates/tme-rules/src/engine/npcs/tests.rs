use super::*;
use crate::content::{CatalogProfileKey, CatalogV6, WorldSeedDef, WorldTemplateV3};
use crate::engine::{GameDefinition, ValidatedWorldSeed};
use crate::model::{ActorLifeState, Coord, ServicePlacement};

fn temple_parts() -> (CatalogV6, CatalogProfileKey, WorldTemplateV3, WorldSeedDef) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/lands/first-expedition");
    let read = |name: &str| std::fs::read_to_string(root.join(name)).unwrap();
    let mut seed: serde_json::Value = serde_json::from_str(&read("simulation_seed.json")).unwrap();
    for key in ["schema_version", "kind", "id"] {
        seed.as_object_mut().unwrap().remove(key);
    }
    (
        serde_json::from_str(&read("catalog.json")).unwrap(),
        CatalogProfileKey::from("profile/first_expedition"),
        serde_json::from_str(&read("generated/world_template.json")).unwrap(),
        serde_json::from_value(seed).unwrap(),
    )
}

fn temple_engine() -> Engine {
    let (catalog, profile, template, seed) = temple_parts();
    crate::engine::setup::test_engine_from_parts(catalog, profile, template, seed)
}

#[test]
fn temple_residents_patrol_deadlines_and_cursor_survive_checkpoint() {
    let mut engine = temple_engine();
    let index = engine
        .world
        .actors
        .iter()
        .position(|actor| actor.id == "tomas")
        .unwrap();
    let start = engine.world.actors[index].location.clone();
    engine.advance_action_interval().unwrap();
    assert_ne!(engine.world.actors[index].location, start);
    assert_eq!(
        engine.world.actors[index].location.position,
        Coord { x: 3, y: 4 }
    );
    let checkpoint = engine.export_checkpoint().unwrap();
    let mut restored =
        Engine::hydrate_checkpoint(engine.definition().clone(), &checkpoint).unwrap();
    assert_eq!(restored.export_checkpoint().unwrap(), checkpoint);
    let mut visited = std::collections::BTreeSet::new();
    for _ in 0..20 {
        visited.insert(engine.world.actors[index].location.position);
        assert_eq!(
            engine.advance_action_interval().unwrap(),
            restored.advance_action_interval().unwrap()
        );
    }
    assert!(
        visited.len() >= 4,
        "the restored patrol must keep moving through its circuit"
    );
    assert_eq!(
        engine.export_checkpoint().unwrap(),
        restored.export_checkpoint().unwrap()
    );
    assert_eq!(engine.world.actors[index].location.site(), start.site());
}

#[test]
fn temple_residents_service_follows_provider_and_attends_approaching_player() {
    let mut engine = temple_engine();
    let priest = engine
        .world
        .actors
        .iter()
        .position(|actor| actor.id == "tomas")
        .unwrap();
    let player = engine
        .world
        .actors
        .iter()
        .position(|actor| actor.kind == ActorKind::Player)
        .unwrap();
    let old = engine.world.actors[priest].location.clone();
    engine.advance_action_interval().unwrap();
    let new = engine.world.actors[priest].location.clone();
    assert_ne!(old, new);
    assert_eq!(engine.service_by_id("tomas").unwrap().position(), &new);
    assert_eq!(
        engine
            .service_by_id("tomas")
            .unwrap()
            .actor_id()
            .unwrap()
            .as_str(),
        "tomas"
    );
    engine.world.actors[player].location = old;
    assert!(
        !engine
            .services_at_actor(player)
            .iter()
            .any(|service| service.id() == "tomas")
    );
    for _ in 0..4 {
        engine.advance_action_interval().unwrap();
    }
    assert_eq!(
        engine.world.actors[priest].location, new,
        "approaching visitor holds the priest's attention"
    );
    engine.world.actors[player].location = new;
    assert!(
        engine
            .services_at_actor(player)
            .iter()
            .any(|service| service.id() == "tomas")
    );
    engine.world.actors[priest].life_state = ActorLifeState::Dead;
    assert!(engine.service_by_id("tomas").is_none());
}

#[test]
fn temple_residents_refuse_bad_routes_unknown_providers_and_retired_placements() {
    let (catalog, profile, template, seed) = temple_parts();
    let definition = GameDefinition::from_content(catalog, profile, template).unwrap();
    let mut bad = seed.clone();
    bad.actors
        .iter_mut()
        .find(|actor| actor.id == "tomas")
        .unwrap()
        .npc
        .as_mut()
        .unwrap()
        .patrol[0] = Coord { x: 0, y: 1 }; // The open stairwell is never patrol ground.
    assert!(ValidatedWorldSeed::new(definition.clone(), bad).is_err());
    let mut bad = seed.clone();
    bad.service_instances[0].placement = ServicePlacement::Actor {
        actor_id: "absent".into(),
    };
    assert!(ValidatedWorldSeed::new(definition, bad).is_err());
    let legacy = serde_json::json!({"id":"legacy", "service_definition_id":"tomas", "location":seed.actors[0].location});
    assert!(serde_json::from_value::<crate::content::ServiceInstanceSeedDef>(legacy).is_err());
    let mut missing = serde_json::to_value(
        seed.actors
            .iter()
            .find(|actor| actor.id == "tomas")
            .unwrap()
            .npc
            .as_ref()
            .unwrap(),
    )
    .unwrap();
    missing.as_object_mut().unwrap().remove("patrol");
    assert!(serde_json::from_value::<crate::content::NpcDef>(missing).is_err());
}

#[test]
fn temple_residents_purchase_requires_the_providers_complete_world_position() {
    let mut engine = temple_engine();
    let seller = engine
        .world
        .actors
        .iter()
        .find(|actor| actor.id == "balm_seller")
        .unwrap()
        .location
        .clone();
    let player = engine
        .world
        .actors
        .iter()
        .position(|actor| actor.kind == ActorKind::Player)
        .unwrap();
    engine.world.actors[player].location = seller;
    engine.world.actors[player].location.realm = "other_realm".into();
    let error = engine
        .merchant_purchase_plan(player, "balm_seller", "trail_wares", &[])
        .unwrap_err();
    assert_eq!(
        error.reason(),
        crate::view::ActionBlockedReasonV1::ServiceNotHere
    );
}
