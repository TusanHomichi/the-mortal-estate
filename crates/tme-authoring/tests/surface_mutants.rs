//! One mutant per fail-closed surface class (P9).
//!
//! These drive the CANDIDATE entry point, which by contract runs the same
//! compile logic the promoted path runs. That is not a convenience: it means
//! this corpus qualifies the promoted path's checks, and a drifted second
//! validator would show up here as a mutant that survives.

mod support;

use serde_json::{Value, json};
use support::{
    SURFACE_WIDTH, assert_rejects, fixture_land, fixture_surface, object, property,
    surface_document, tile,
};

#[test]
fn the_tracked_master_is_accepted() {
    let report = tme_authoring::validate_candidate(
        fixture_land().id,
        fixture_surface(),
        &surface_document(),
    )
    .unwrap();
    assert!(report.accepted, "diagnostics: {:?}", report.diagnostics);
    let statistics = report
        .statistics
        .expect("an accepted candidate reports statistics");
    assert_eq!(statistics.width, 24);
    assert_eq!(statistics.height, 16);
    assert_eq!(statistics.passable_cells, 281);
    assert_eq!(statistics.route_cells, 38);
    assert_eq!(statistics.structure_footprint_cells, 12);
    assert_eq!(statistics.structures.len(), 3);
    assert_eq!(statistics.landmarks.len(), 2);
}

#[test]
fn a_stretched_envelope_is_rejected() {
    let mut document = surface_document();
    document["width"] = json!(25);
    assert_rejects(&document, "authored map.width must be 24");
}

#[test]
fn an_infinite_map_is_rejected() {
    let mut document = surface_document();
    document["infinite"] = json!(true);
    assert_rejects(&document, "finite and orthogonal");
}

#[test]
fn a_drifted_map_property_is_rejected() {
    let mut document = surface_document();
    let properties = document["properties"].as_array_mut().unwrap();
    let entry = properties
        .iter_mut()
        .find(|row| row["name"] == "content_authority")
        .unwrap();
    entry["value"] = json!("first_land");
    assert_rejects(&document, "accepted map property content_authority differs");
}

#[test]
fn an_extra_map_property_is_rejected() {
    let mut document = surface_document();
    document["properties"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name": "extra_claim", "type": "bool", "value": true}));
    assert_rejects(&document, "the accepted contract declares");
}

#[test]
fn a_renamed_tile_class_is_rejected() {
    let mut document = surface_document();
    document["tilesets"][0]["tiles"][1]["class"] = json!("testland_meadow");
    assert_rejects(&document, "embedded tile vocabulary differs");
}

#[test]
fn a_missing_layer_is_rejected() {
    let mut document = surface_document();
    let layers = document["layers"].as_array_mut().unwrap();
    layers.retain(|layer| layer["name"] != "landmark_marks");
    assert_rejects(&document, "authored layer set differs");
}

#[test]
fn an_extra_layer_is_rejected() {
    let mut document = surface_document();
    document["layers"].as_array_mut().unwrap().push(json!({
        "draworder": "index", "id": 99, "name": "annotations", "objects": [],
        "opacity": 1, "type": "objectgroup", "visible": true, "x": 0, "y": 0,
    }));
    assert_rejects(&document, "authored layer set differs");
}

#[test]
fn an_unknown_tile_id_is_rejected() {
    let mut document = surface_document();
    *tile(&mut document, "base_terrain", SURFACE_WIDTH, 12, 11) = json!(99);
    assert_rejects(&document, "outside the class set");
}

#[test]
fn a_tile_class_used_in_the_wrong_layer_is_rejected() {
    let mut document = surface_document();
    // 9 is the structure footprint class; it has no business in base terrain.
    *tile(&mut document, "base_terrain", SURFACE_WIDTH, 12, 11) = json!(9);
    assert_rejects(&document, "belongs to another layer");
}

#[test]
fn an_unauthored_terrain_cell_is_rejected() {
    let mut document = surface_document();
    *tile(&mut document, "base_terrain", SURFACE_WIDTH, 12, 11) = json!(0);
    assert_rejects(&document, "leaves cell");
}

#[test]
fn a_stale_passability_annotation_is_rejected() {
    let mut document = surface_document();
    // 12 is fixture_blocked; cell 12,11 is open grass.
    *tile(&mut document, "passability", SURFACE_WIDTH, 12, 11) = json!(12);
    assert_rejects(&document, "passability annotation is stale at 12,11");
}

#[test]
fn a_footprint_layer_that_disagrees_with_its_objects_is_rejected() {
    let mut document = surface_document();
    *tile(&mut document, "structure_footprints", SURFACE_WIDTH, 8, 6) = json!(0);
    assert_rejects(&document, "describe different cells");
}

#[test]
fn a_sealed_walkable_pocket_is_rejected() {
    let mut document = surface_document();
    // Wall cell 1,1 off from the rest of the island with deep water.
    for (x, y) in [(2_usize, 1_usize), (1, 2)] {
        *tile(&mut document, "base_terrain", SURFACE_WIDTH, x, y) = json!(1);
        *tile(&mut document, "passability", SURFACE_WIDTH, x, y) = json!(12);
    }
    assert_rejects(&document, "walkable cells no one can reach");
}

#[test]
fn a_door_that_is_not_on_a_footprint_edge_is_rejected() {
    let mut document = surface_document();
    let structure = object(&mut document, "structures", "fixture_structure_north");
    *property(structure, "access_cell_y") = json!(10);
    assert_rejects(&document, "must touch exactly one footprint cell, not 0");
}

#[test]
fn an_ambiguous_door_is_rejected() {
    let mut document = surface_document();
    // An access cell inside the footprint touches two footprint cells.
    let structure = object(&mut document, "structures", "fixture_structure_north");
    *property(structure, "access_cell_y") = json!(6);
    assert_rejects(&document, "must touch exactly one footprint cell, not 2");
}

#[test]
fn a_blocked_structure_access_cell_is_rejected() {
    let mut document = surface_document();
    *tile(&mut document, "base_terrain", SURFACE_WIDTH, 8, 8) = json!(1);
    *tile(&mut document, "passability", SURFACE_WIDTH, 8, 8) = json!(12);
    assert_rejects(&document, "access cell is blocked");
}

#[test]
fn a_structure_detached_from_every_route_is_rejected() {
    let mut document = surface_document();
    for x in 7..17 {
        *tile(&mut document, "routes", SURFACE_WIDTH, x, 9) = json!(0);
    }
    assert_rejects(&document, "detached from every authored route");
}

#[test]
fn a_structure_whose_ground_disagrees_with_its_scope_is_rejected() {
    let mut document = surface_document();
    // Repaint one cell under a clustered building from town ground to grass.
    *tile(&mut document, "base_terrain", SURFACE_WIDTH, 8, 6) = json!(2);
    assert_rejects(&document, "but its footprint at 8,6 disagrees");
}

#[test]
fn overlapping_footprints_are_rejected() {
    let mut document = surface_document();
    let structure = object(&mut document, "structures", "fixture_structure_south");
    structure["x"] = json!(9 * 16);
    assert_rejects(&document, "overlaps another footprint");
}

#[test]
fn a_renamed_structure_is_rejected() {
    let mut document = surface_document();
    object(&mut document, "structures", "fixture_structure_south")["name"] =
        json!("fixture_structure_east");
    assert_rejects(&document, "structure program differs");
}

#[test]
fn a_reclassified_structure_is_rejected() {
    let mut document = surface_document();
    let structure = object(&mut document, "structures", "fixture_structure_outland");
    *property(structure, "scope") = json!("clustered");
    assert_rejects(&document, "scope differs from the accepted contract");
}

#[test]
fn an_unoccupied_structure_is_rejected() {
    let mut document = surface_document();
    let structure = object(&mut document, "structures", "fixture_structure_north");
    *property(structure, "occupied") = json!(false);
    assert_rejects(&document, "must be occupied");
}

#[test]
fn a_purposeless_structure_is_rejected() {
    let mut document = surface_document();
    let structure = object(&mut document, "structures", "fixture_structure_north");
    *property(structure, "purpose") = json!("");
    assert_rejects(&document, "purpose must be a non-empty");
}

#[test]
fn a_miscast_structure_object_is_rejected() {
    let mut document = surface_document();
    object(&mut document, "structures", "fixture_structure_north")["class"] = json!("decoration");
    assert_rejects(&document, "must be a functional_building");
}

#[test]
fn a_renamed_landmark_is_rejected() {
    let mut document = surface_document();
    object(&mut document, "landmarks", "fixture_ruin_marker")["name"] = json!("fixture_shrine");
    assert_rejects(&document, "landmark program differs");
}

#[test]
fn an_unpainted_landmark_marker_is_rejected() {
    // The cell is READ from the landmark rather than written down, because this
    // mutant is about the marker under a landmark and not about a coordinate.
    // Pinning the number instead cost a real failure: the first time the
    // Workbench moved this landmark and the fixture was re-attested, the mutant
    // cleared an already-empty cell and was accepted — a check that had quietly
    // stopped binding anything.
    let mut document = surface_document();
    let landmark = object(&mut document, "landmarks", "fixture_ruin_marker").clone();
    let (x, y) = (
        landmark["x"].as_u64().unwrap() as usize / 16,
        landmark["y"].as_u64().unwrap() as usize / 16,
    );
    *tile(&mut document, "landmark_marks", SURFACE_WIDTH, x, y) = json!(0);
    assert_rejects(
        &document,
        "not painted with its testland_ruin_ground marker",
    );
}

#[test]
fn an_arrival_off_the_route_network_is_rejected() {
    let mut document = surface_document();
    object(&mut document, "landmarks", "fixture_dock_arrival")["x"] = json!(11 * 16);
    assert_rejects(&document, "must stand on an authored route cell");
}

#[test]
fn a_landmark_on_a_blocked_cell_is_rejected() {
    let mut document = surface_document();
    object(&mut document, "landmarks", "fixture_ruin_marker")["x"] = json!(0);
    object(&mut document, "landmarks", "fixture_ruin_marker")["y"] = json!(0);
    assert_rejects(&document, "stands on a blocked cell");
}

#[test]
fn a_rerouted_transition_is_rejected() {
    let mut document = surface_document();
    let transition = object(&mut document, "transitions", "fixture_descent");
    *property(transition, "target_member") = json!("surface");
    assert_rejects(&document, "differs from the accepted contract");
}

#[test]
fn a_transition_access_cell_away_from_its_marker_is_rejected() {
    let mut document = surface_document();
    let transition = object(&mut document, "transitions", "fixture_descent");
    *property(transition, "access_cell_y") = json!(10);
    assert_rejects(&document, "cardinally adjacent to its marker");
}

#[test]
fn an_unpainted_transition_marker_is_rejected() {
    let mut document = surface_document();
    *tile(&mut document, "landmark_marks", SURFACE_WIDTH, 18, 7) = json!(0);
    assert_rejects(&document, "not painted with its testland_shaft marker");
}

#[test]
fn an_off_lattice_object_is_rejected() {
    let mut document = surface_document();
    object(&mut document, "landmarks", "fixture_ruin_marker")["x"] = json!(5 * 16 + 3);
    assert_rejects(&document, "must align to a non-negative 16px authored cell");
}

#[test]
fn a_duplicated_layer_name_is_rejected() {
    let mut document = surface_document();
    let clone: Value = document["layers"][0].clone();
    document["layers"].as_array_mut().unwrap().push(clone);
    assert_rejects(&document, "duplicates layer");
}

#[test]
fn a_second_embedded_tileset_is_rejected() {
    let mut document = surface_document();
    let clone: Value = document["tilesets"][0].clone();
    document["tilesets"].as_array_mut().unwrap().push(clone);
    assert_rejects(&document, "exactly one tileset");
}
