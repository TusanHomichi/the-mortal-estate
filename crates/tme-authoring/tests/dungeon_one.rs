//! Full-floor encoding and explicitly deferred traversal retain fail-closed proof.
mod support;
use serde_json::{Value, json};
use tme_authoring::{compile_member, land};

fn document() -> Value {
    serde_json::from_str(include_str!(
        "../../../content/lands/first-expedition/d1_entry.tmj"
    ))
    .unwrap()
}

fn compile(doc: &Value) -> Result<tme_authoring::Member, String> {
    compile_member(
        land("first_expedition")
            .unwrap()
            .member("d1_entry")
            .unwrap(),
        doc,
    )
}

fn paint(doc: &mut Value, layer: &str, x: usize, y: usize, class: &str) {
    let gid = doc["tilesets"][0]["tiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tile| tile["class"] == class)
        .unwrap()["id"]
        .as_u64()
        .unwrap()
        + 1;
    *support::tile(doc, layer, 36, x, y) = json!(gid);
}

#[test]
fn unaccounted_islands_and_accidentally_connected_deferred_regions_are_rejected() {
    let mut orphan = document();
    paint(&mut orphan, "base_terrain", 1, 1, "expedition_floor");
    paint(&mut orphan, "passability", 1, 1, "expedition_walkable");
    assert!(
        compile(&orphan)
            .unwrap_err()
            .contains("walkable cells no one can reach")
    );

    let mut bridge = document();
    paint(&mut bridge, "base_terrain", 15, 29, "expedition_floor");
    paint(&mut bridge, "passability", 15, 29, "expedition_walkable");
    assert!(
        compile(&bridge)
            .unwrap_err()
            .contains("deferred access joins an already reachable component")
    );
}

#[test]
fn an_overlay_cannot_silently_replace_an_operable_door() {
    let mut doc = document();
    paint(&mut doc, "routes", 23, 6, "expedition_path");
    assert!(
        compile(&doc)
            .unwrap_err()
            .contains("door at 23,6 has conflicting layers or state")
    );
}

#[test]
fn all_stair_markers_survive_with_three_lower_routes_and_reserved_surface_exit() {
    let member = compile(&document()).unwrap();
    for (x, y, terrain) in [
        (25, 7, "expedition_stairs_up"),
        (7, 35, "expedition_stairs_up"),
        (15, 3, "expedition_stairs_down"),
        (28, 21, "expedition_stairs_down"),
        (7, 39, "expedition_stairs_down"),
    ] {
        assert_eq!(member.cells()[y][x], vec![Some(terrain.into())]);
    }
    assert_eq!(member.transitions().len(), 5);
    assert_eq!(
        member
            .landmarks()
            .values()
            .filter(|landmark| landmark.role == "deferred_access")
            .count(),
        10
    );
    assert_eq!(
        member.transitions()["d1_entry_to_temple"].access,
        tme_authoring::Point { x: 25, y: 7 }
    );
}
