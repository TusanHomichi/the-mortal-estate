//! Four-floor geography and runtime traversal through the same authored authority.
use serde_json::Value;
use tme_authoring::{Point, land, load, project, repository_root};
use tme_rules::{
    CatalogProfileKey, CatalogV6, Engine, GameDefinition, ValidatedWorldSeed, WorldSeedDef,
};

#[test]
fn four_envelopes_preserve_materials_and_omit_only_the_storage_cutout() {
    let compiled = load(
        &repository_root().unwrap(),
        land("first_expedition").unwrap(),
    )
    .unwrap();
    for (id, width, passable, doors) in [
        ("d1_entry", 36, 635, 55),
        ("d2", 38, 686, 64),
        ("d3", 36, 765, 45),
        ("d4", 37, 585, 35),
    ] {
        let member = compiled.member(id).unwrap();
        assert_eq!((member.width(), member.height()), (width, 43));
        assert_eq!(member.report().passable_cells, passable);
        assert_eq!(member.doors().len(), doors);
    }
    let fourth = compiled.member("d4").unwrap();
    let inside = |p: Point| (16..=24).contains(&p.x) && (25..=34).contains(&p.y);
    for y in 25..=34 {
        for x in 16..=24 {
            assert_eq!(fourth.cells()[y][x], vec![Some("expedition_wall".into())]);
        }
    }
    assert!(fourth.doors().iter().all(|d| !inside(d.at)));
    assert!(fourth.landmarks().values().all(|l| !inside(l.at)));
    assert!(
        fourth
            .transitions()
            .values()
            .all(|t| !inside(t.access) && !inside(t.landing))
    );
    // The source's shared boundary column contains third-floor lake cells;
    // they do not become a disconnected duplicate lake on the second floor.
    for y in 6..=12 {
        assert_eq!(
            compiled.member("d2").unwrap().cells()[y][37],
            vec![Some("expedition_wall".into())]
        );
        assert_eq!(
            compiled.member("d3").unwrap().cells()[y][0],
            vec![Some("expedition_dungeon_water".into())]
        );
    }
}

#[test]
fn stair_correspondences_and_both_grand_door_faces_have_exact_reciprocals() {
    let compiled = load(
        &repository_root().unwrap(),
        land("first_expedition").unwrap(),
    )
    .unwrap();
    let world = serde_json::to_value(project(&compiled).unwrap()).unwrap();
    let edges = world["topology"].as_object().unwrap();
    for (lower, upper, x, native_y, lower_origin, upper_origin) in [
        ("d1_entry", "d2", 7, 3, 0, 35),
        ("d1_entry", "d2", 15, 39, 0, 35),
        ("d1_entry", "d2", 28, 21, 0, 35),
        ("d2", "d3", 39, 39, 35, 72),
        ("d2", "d3", 53, 19, 35, 72),
        ("d2", "d3", 60, 14, 35, 72),
        ("d2", "d3", 66, 11, 35, 72),
        ("d2", "d3", 69, 27, 35, 72),
        ("d3", "d4", 81, 25, 72, 108),
        ("d3", "d4", 102, 25, 72, 108),
        ("d3", "d4", 104, 3, 72, 108),
    ] {
        let from = compiled.member(lower).unwrap();
        let t = from
            .transitions()
            .values()
            .find(|t| {
                t.access
                    == Point {
                        x: x - lower_origin,
                        y: 42 - native_y,
                    }
            })
            .unwrap();
        assert_eq!(t.direction, "down");
        assert_eq!(t.target_member, upper);
        let paired = &compiled.member(upper).unwrap().transitions()[&t.paired_transition];
        assert_eq!(paired.direction, "up");
        assert_eq!(paired.paired_transition, t.id);
        assert_eq!(
            paired.access,
            Point {
                x: x + 35 - upper_origin,
                y: 42 - native_y
            }
        );
    }
    for y in [33, 34] {
        for (from, x, to, other_x) in [("d2", 37, "d3", 0), ("d3", 0, "d2", 37)] {
            let t = compiled
                .member(from)
                .unwrap()
                .transitions()
                .values()
                .find(|t| t.access == Point { x, y })
                .unwrap();
            let edge = &edges[&format!("route/{}", t.id)];
            assert_eq!(edge["kind"]["kind"], "door");
            assert_eq!(
                edge["kind"]["reciprocal_endpoint_id"],
                format!("route/{}", t.paired_transition)
            );
            assert_eq!(edge["target"]["location"]["level"], to);
            assert_eq!(
                edge["target"]["location"]["position"],
                serde_json::json!({"x":other_x,"y":y})
            );
            assert!(!edges.contains_key(&format!("door/{from}/{x}/{y}")));
        }
    }
    let guild = &compiled.member("d1_entry").unwrap().transitions()["d1_entry_to_d2_guild"];
    assert_eq!(guild.access, Point { x: 35, y: 29 });
    assert_eq!(guild.direction, "passage");
    assert_eq!(guild.target_member, "d2");
}

#[test]
fn every_numbered_stair_round_trip_runs_in_rules_including_the_concealed_alcove() {
    let root = repository_root().unwrap();
    let compiled = load(&root, land("first_expedition").unwrap()).unwrap();
    let catalog: CatalogV6 = serde_json::from_slice(
        &std::fs::read(root.join(compiled.contract().terrain_registry_catalog)).unwrap(),
    )
    .unwrap();
    let template = project(&compiled).unwrap();
    let definition = GameDefinition::from_content(
        catalog,
        CatalogProfileKey::from("profile/first_expedition"),
        template.clone(),
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
    let mut count = 0;
    for (id, edge) in &template.topology {
        if !id.contains("_stair_") {
            continue;
        }
        let tme_rules::TopologyKindDef::Stairs { direction } = edge.kind else {
            panic!("numbered route must be stairs")
        };
        let mut engine = Engine::new(
            ValidatedWorldSeed::new(definition.clone(), seed.clone()).unwrap(),
            7,
        )
        .unwrap();
        let actor = tme_rules::ActorId::from("player");
        engine
            .world_mut()
            .actors
            .iter_mut()
            .find(|a| a.id == actor)
            .unwrap()
            .location = edge.at.clone();
        let traversal = match direction {
            tme_rules::model::VerticalDirection::Down => {
                tme_rules::ExplicitTraversalKind::StairsDown
            }
            tme_rules::model::VerticalDirection::Up => tme_rules::ExplicitTraversalKind::StairsUp,
        };
        engine
            .apply_actor_intent(&actor, tme_rules::PlayerIntent::Traverse(traversal))
            .unwrap();
        let tme_rules::TopologyTargetDef::Position { location } = &edge.target else {
            panic!("stairs must have a position")
        };
        assert_eq!(
            &engine.world().actor(&actor).unwrap().location,
            location,
            "{id}"
        );
        count += 1;
    }
    assert_eq!(count, 22);
    let alcove = &compiled.member("d2").unwrap().transitions()["d2_to_d3_stair_3"];
    assert_eq!(alcove.access, Point { x: 25, y: 28 });
    assert_eq!(alcove.landing, alcove.access);
}
