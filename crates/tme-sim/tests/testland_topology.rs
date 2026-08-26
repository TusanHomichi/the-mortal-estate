use std::collections::BTreeSet;
use std::path::PathBuf;
use tme_rules::{ActorAiBehavior, ActorId, Coord, SocialBehavior, WorldPosition};

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/test-corpus/first_land_structure.json")
}

fn position(level: &str, x: i32, y: i32) -> WorldPosition {
    WorldPosition::new("testland", level, Coord::from((x, y)))
}

#[test]
fn production_content_uses_the_accepted_first_land_surface() {
    let engine = tme_sim::load_engine_from_scenario(&scenario_path(), None)
        .expect("load the tracked first-land Scenario/Catalog/Template/Seed graph");
    let definition = engine.definition();
    let template = definition.world_template();
    let realm = &template.realms()["testland"];
    assert_eq!(
        realm
            .levels
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "surface",
            "bank_interior",
            "upper_halls",
            "lower_halls",
            "lake_level",
            "old_temple",
        ])
    );
    let surface = &realm.levels["surface"];
    assert_eq!((surface.width, surface.height), (160, 64));
    assert_eq!(
        template.arrivals()["south_dock"],
        position("surface", 25, 62)
    );

    for (terrain_id, passable, blocks_sight) in [
        ("testland_deep_water", false, false),
        ("testland_grass", true, false),
        ("testland_forest", true, false),
        ("testland_marsh", true, false),
        ("testland_rock_coast", false, false),
        ("testland_town_ground", true, false),
        ("testland_path", true, false),
        ("testland_bridge", true, false),
        ("testland_graveyard_ground", true, false),
        ("testland_ruin_ground", true, false),
        ("testland_structure_footprint", false, true),
    ] {
        let terrain = definition
            .catalog()
            .terrain(terrain_id)
            .unwrap_or_else(|| panic!("selected first-land terrain {terrain_id}"));
        assert_eq!(terrain.passable, passable, "{terrain_id} passability");
        assert_eq!(
            terrain.blocks_sight, blocks_sight,
            "{terrain_id} sight role"
        );
        assert_eq!(terrain.move_cost, passable.then_some(1));
    }
    for obsolete in [
        "testland_fence",
        "testland_garden_yard",
        "testland_visual_threshold",
        "testland_road_exit",
        "testland_well",
        "testland_plaza",
        "testland_tree_band",
        "town_floor",
        "island_ground",
    ] {
        assert!(
            definition.catalog().terrain(obsolete).is_none(),
            "obsolete surface terrain {obsolete} must not survive the atomic cutover"
        );
    }

    let props = surface
        .static_props
        .iter()
        .map(|prop| (prop.id.as_str(), prop.visual_family.as_str(), prop.anchor))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        props,
        BTreeSet::from([
            (
                "south_dock_reeds",
                "gathering_inert/reeds",
                Coord::from((24, 59))
            ),
            (
                "town_herb_garden",
                "gathering_inert/herbs",
                Coord::from((15, 36))
            ),
            (
                "western_reserve_lumber",
                "gathering_inert/lumber",
                Coord::from((36, 10))
            ),
        ])
    );

    for (source, target, reciprocal) in [
        (
            position("surface", 30, 46),
            position("bank_interior", 5, 6),
            true,
        ),
        (
            position("surface", 17, 43),
            position("upper_halls", 5, 5),
            true,
        ),
        (
            position("surface", 8, 51),
            position("lower_halls", 5, 5),
            true,
        ),
        (
            position("surface", 72, 43),
            position("lake_level", 7, 13),
            false,
        ),
    ] {
        assert!(
            template.navigation()[&source]
                .iter()
                .any(|edge| edge.target == target && !edge.hidden),
            "forward semantic transition {source:?} -> {target:?}"
        );
        assert_eq!(
            template
                .navigation()
                .get(&target)
                .is_some_and(|edges| edges.iter().any(|edge| edge.target == source)),
            reciprocal,
            "reciprocity for {source:?} -> {target:?}"
        );
    }

    for (actor_id, expected) in [
        ("player", position("surface", 25, 62)),
        ("town_watch", position("surface", 23, 51)),
        ("temple_priest", position("surface", 16, 43)),
        ("temple_keeper", position("surface", 18, 43)),
        ("forge_keeper", position("surface", 36, 31)),
        ("shrine_keeper", position("surface", 142, 31)),
        ("bank_keeper", position("bank_interior", 7, 4)),
    ] {
        assert_eq!(
            engine
                .world()
                .actor(&ActorId::from(actor_id))
                .unwrap()
                .location,
            expected,
            "production seed actor {actor_id}"
        );
    }
    let surface_ecology = engine
        .world()
        .ecology_sites
        .values()
        .filter(|site| site.id.starts_with("surface_"))
        .flat_map(|site| site.member_slots.values())
        .collect::<Vec<_>>();
    assert_eq!(surface_ecology.len(), 19);
    for slot in &surface_ecology {
        let actor_id = slot
            .actor_id
            .as_ref()
            .expect("every initial surface ecology slot is occupied");
        let actor = engine
            .world()
            .actor(actor_id)
            .expect("occupied ecology slot resolves");
        assert_eq!(actor.location, slot.location);
        assert_eq!(actor.home_location, slot.location);
    }
    for site_id in ["surface_dock_crocodile", "surface_town_hounds"] {
        for actor in engine.world().actors.iter().filter(|actor| {
            actor
                .ecology_origin
                .as_ref()
                .is_some_and(|origin| origin.site_id == site_id)
        }) {
            assert_eq!(actor.social.behavior, SocialBehavior::Passive);
            assert_eq!(
                actor.ai.as_ref().expect("ecology actor has AI").behavior,
                ActorAiBehavior::HoldGround
            );
        }
    }

    let projection = engine
        .observer_projection(&ActorId::from("player"), &[])
        .expect("South Dock projection");
    assert_eq!(
        projection
            .static_scene_context
            .static_props
            .iter()
            .map(|prop| prop.id.as_str())
            .collect::<Vec<_>>(),
        ["south_dock_reeds"]
    );
    assert!(serde_json::to_vec(&projection).unwrap().len() <= 64 * 1024);
    assert_eq!(
        surface.cells[62][25],
        vec![Some("testland_bridge".into()), Some("south_dock".into())]
    );
    assert!(surface.cells.iter().flatten().all(|cell| {
        !cell
            .iter()
            .flatten()
            .any(|id| id == "testland_plaza" || id == "testland_tree_band")
    }));
}
