//! The accepted town and first-floor section, including deliberately offset landings.
mod support;
use serde_json::{Value, json};
use tme_authoring::{Point, compile_member, land, load, project, repository_root};

fn document(member: &str) -> Value {
    let root = repository_root().unwrap();
    serde_json::from_slice(
        &std::fs::read(
            root.join(
                land("first_expedition")
                    .unwrap()
                    .member(member)
                    .unwrap()
                    .document,
            ),
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn accepted_geography_projects_exact_passages_stairs_and_dock_arrival() {
    let land = load(
        &repository_root().unwrap(),
        land("first_expedition").unwrap(),
    )
    .unwrap();
    assert_eq!(land.members().count(), 9);
    assert_eq!(land.member("arrival").unwrap().report().passable_cells, 680);
    for retired in ["bakery", "herbalist", "chandler"] {
        assert!(land.member(retired).is_err());
        assert!(
            tme_authoring::land("first_expedition")
                .unwrap()
                .member(retired)
                .is_err()
        );
    }
    assert_eq!(land.member("d1_entry").unwrap().report().passable_cells, 17);
    assert_eq!(
        land.arrival_member().unwrap().arrival(),
        Some(Point { x: 8, y: 34 })
    );
    let world = serde_json::to_value(project(&land).unwrap()).unwrap();
    let edges = world["topology"].as_object().unwrap();
    assert_eq!(edges.len(), 16);
    assert_eq!(edges["route/arrival_to_market"]["kind"]["kind"], "passage");
    assert_eq!(
        edges["route/arrival_to_market"]["target"]["location"]["position"],
        json!({"x":0,"y":1})
    );
    assert_eq!(
        edges["route/temple_to_d1_entry"]["kind"],
        json!({"kind":"stairs","direction":"down"})
    );
    assert_eq!(
        edges["route/temple_to_d1_entry"]["target"]["location"]["position"],
        json!({"x":5,"y":4})
    );
    assert_eq!(
        edges["route/d1_entry_to_temple"]["target"]["location"]["position"],
        json!({"x":0,"y":5})
    );
}

#[test]
fn threshold_landings_are_required_open_and_inside_the_member() {
    let contract = land("first_expedition").unwrap().member("market").unwrap();
    for (name, value) in [("landing_cell_x", 4), ("landing_cell_y", 0)] {
        let mut doc = document("market");
        *support::property(
            support::object(&mut doc, "transitions", "market_to_arrival"),
            name,
        ) = json!(value);
        assert!(
            compile_member(contract, &doc)
                .unwrap_err()
                .contains("blocked or outside")
        );
    }
    let mut doc = document("market");
    support::object(&mut doc, "transitions", "market_to_arrival")["properties"]
        .as_array_mut()
        .unwrap()
        .retain(|p| p["name"] != "landing_cell_x");
    assert!(compile_member(contract, &doc).is_err());
    let mut doc = document("market");
    support::object(&mut doc, "transitions", "market_to_arrival")["x"] = json!(16);
    assert!(
        compile_member(contract, &doc)
            .unwrap_err()
            .contains("threshold marker must equal")
    );
}

#[test]
fn authored_cast_and_all_five_creation_choices_validate_through_runtime_rules() {
    use tme_rules::model::CharacterId;
    use tme_rules::{
        CatalogProfileKey, CatalogV6, Engine, GameDefinition, ValidatedWorldSeed, WorldSeedDef,
    };
    let root = repository_root().unwrap();
    let land = load(&root, land("first_expedition").unwrap()).unwrap();
    let catalog: CatalogV6 = serde_json::from_slice(
        &std::fs::read(root.join(land.contract().terrain_registry_catalog)).unwrap(),
    )
    .unwrap();
    let definition = GameDefinition::from_content(
        catalog,
        CatalogProfileKey::from("profile/first_expedition"),
        project(&land).unwrap(),
    )
    .unwrap();
    let mut source: Value = serde_json::from_slice(
        &std::fs::read(root.join("content/lands/first-expedition/simulation_seed.json")).unwrap(),
    )
    .unwrap();
    for key in ["schema_version", "kind", "id"] {
        source.as_object_mut().unwrap().remove(key);
    }
    let seed: WorldSeedDef = serde_json::from_value(source).unwrap();
    let engine = Engine::new(
        ValidatedWorldSeed::new(definition.clone(), seed).unwrap(),
        7,
    )
    .unwrap();
    assert_eq!(engine.world().service_instances.len(), 11);
    for retired in ["bakery_keeper", "herbalist_keeper", "chandler_keeper"] {
        assert!(
            engine
                .world()
                .actors
                .iter()
                .all(|actor| actor.id.as_str() != retired)
        );
        assert!(
            engine
                .world()
                .service_instances
                .iter()
                .all(|service| service.id != retired)
        );
    }
    let provisions = &engine.world().merchant_inventories
        [&tme_rules::model::MerchantInventoryId::new("provisioner", "trail_wares")];
    assert_eq!(provisions.listings.len(), 12);
    assert!(
        provisions
            .listings
            .iter()
            .all(|listing| listing.price_gold == 5)
    );
    assert_eq!(definition.creation_profiles().len(), 5);
    let mut created = engine;
    for profile in definition.creation_profiles() {
        created = created
            .prepare_character_creation(
                &profile.id,
                CharacterId::new(format!("proof/{}", profile.id)),
                &profile.character.identity.display_class,
                profile.character.attributes.clone(),
            )
            .unwrap();
    }
}
