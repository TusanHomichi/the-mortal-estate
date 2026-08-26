//! Mutants for the classes that are not reachable through the public API.
//!
//! Surface semantics are qualified through the candidate entry point in
//! `tests/surface_mutants.rs`, and the promotion gate through
//! `tests/promotion_gate.rs`. Two families remain:
//!
//! - the interior member's own compile, which has no candidate entry point
//!   because the fixture declares one and it is the surface's;
//! - the connectivity graph, whose failure modes are unreachable by document
//!   mutation precisely BECAUSE the member compile enforces an exact
//!   transition program first. They are qualified here against compiled
//!   members whose transitions are mutated directly, which is the only honest
//!   way to prove a check that a stricter check stands in front of.

use serde_json::{Value, json};

use std::collections::BTreeMap;

use crate::compile::{Member, compile_member};
use crate::contract::{MemberContract, fixture};
use crate::graph::link;
use crate::tiled::Point;

const INTERIOR_WIDTH: usize = 10;

fn surface_contract() -> &'static MemberContract {
    fixture::LAND.member("surface").unwrap()
}

fn interior_contract() -> &'static MemberContract {
    fixture::LAND.member("interior").unwrap()
}

fn surface_document() -> Value {
    serde_json::from_str(include_str!(
        "../../../content/authoring-fixture/fixture-surface.tmj"
    ))
    .unwrap()
}

fn interior_document() -> Value {
    serde_json::from_str(include_str!(
        "../../../content/authoring-fixture/fixture-interior.tmj"
    ))
    .unwrap()
}

fn layer<'a>(document: &'a mut Value, name: &str) -> &'a mut Value {
    document["layers"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|layer| layer["name"] == name)
        .unwrap()
}

fn tile<'a>(document: &'a mut Value, name: &str, x: usize, y: usize) -> &'a mut Value {
    &mut layer(document, name)["data"][y * INTERIOR_WIDTH + x]
}

fn interior_rejection(document: &Value, fragment: &str) {
    let diagnostic =
        compile_member(interior_contract(), document).expect_err("the mutant must be rejected");
    assert!(
        diagnostic.contains(fragment),
        "expected {fragment:?}, got {diagnostic:?}"
    );
}

#[test]
fn the_tracked_interior_compiles() {
    let interior = compile_member(interior_contract(), &interior_document()).unwrap();
    assert_eq!(interior.report().width, 10);
    assert_eq!(interior.report().height, 8);
    assert_eq!(interior.report().passable_cells, 38);
    assert_eq!(interior.transitions().len(), 1);
}

#[test]
fn a_stretched_interior_envelope_is_rejected() {
    let mut document = interior_document();
    document["width"] = json!(11);
    interior_rejection(&document, "authored map.width must be 10");
}

#[test]
fn an_interior_declaring_the_surface_role_is_rejected() {
    let mut document = interior_document();
    let properties = document["properties"].as_array_mut().unwrap();
    properties
        .iter_mut()
        .find(|row| row["name"] == "member_role")
        .unwrap()["value"] = json!("surface");
    interior_rejection(&document, "accepted map property member_role differs");
}

#[test]
fn an_interior_carrying_a_surface_layer_is_rejected() {
    let mut document = interior_document();
    document["layers"].as_array_mut().unwrap().push(json!({
        "data": vec![0; 80], "height": 8, "id": 9, "name": "routes", "opacity": 1,
        "type": "tilelayer", "visible": true, "width": 10, "x": 0, "y": 0,
    }));
    interior_rejection(&document, "authored layer set differs");
}

#[test]
fn an_interior_tile_id_outside_the_class_set_is_rejected() {
    let mut document = interior_document();
    *tile(&mut document, "base_terrain", 3, 3) = json!(9);
    interior_rejection(&document, "outside the class set");
}

#[test]
fn an_interior_annotation_class_in_the_terrain_layer_is_rejected() {
    let mut document = interior_document();
    *tile(&mut document, "base_terrain", 3, 3) = json!(4);
    interior_rejection(&document, "belongs to another layer");
}

#[test]
fn a_stale_interior_passability_annotation_is_rejected() {
    let mut document = interior_document();
    *tile(&mut document, "passability", 1, 2) = json!(4);
    interior_rejection(&document, "passability annotation is stale at 1,2");
}

#[test]
fn a_sealed_interior_pocket_is_rejected() {
    let mut document = interior_document();
    for (x, y) in [(2_usize, 2_usize), (1, 3)] {
        *tile(&mut document, "base_terrain", x, y) = json!(1);
        *tile(&mut document, "passability", x, y) = json!(4);
    }
    interior_rejection(&document, "walkable cells no one can reach");
}

#[test]
fn a_renamed_interior_transition_is_rejected() {
    let mut document = interior_document();
    layer(&mut document, "transitions")["objects"][0]["name"] = json!("fixture_hatch");
    interior_rejection(&document, "interior transition program differs");
}

#[test]
fn an_unpainted_interior_stair_is_rejected() {
    let mut document = interior_document();
    *tile(&mut document, "base_terrain", 7, 2) = json!(2);
    interior_rejection(&document, "not painted with its testland_stairs_up marker");
}

fn compiled() -> BTreeMap<String, Member> {
    BTreeMap::from([
        (
            "surface".to_owned(),
            compile_member(surface_contract(), &surface_document()).unwrap(),
        ),
        (
            "interior".to_owned(),
            compile_member(interior_contract(), &interior_document()).unwrap(),
        ),
    ])
}

fn transition<'a>(
    members: &'a mut BTreeMap<String, Member>,
    member: &str,
    id: &str,
) -> &'a mut crate::compile::Transition {
    members
        .get_mut(member)
        .unwrap()
        .transitions
        .get_mut(id)
        .unwrap()
}

fn link_rejection(members: &BTreeMap<String, Member>, fragment: &str) {
    let diagnostic = link(members).expect_err("the mutant must be rejected");
    assert!(
        diagnostic.contains(fragment),
        "expected {fragment:?}, got {diagnostic:?}"
    );
}

#[test]
fn the_tracked_land_links() {
    let graph = link(&compiled()).unwrap();
    assert_eq!(graph.edges.len(), 2);
    assert_eq!(graph.edges[0].id, "route/fixture_ascent");
    assert_eq!(graph.edges[1].id, "route/fixture_descent");
}

// Members are resolved in name order, so a mutant is targeted at the member
// whose own check reaches the branch under test. Mutating the other half kills
// the same class through the reciprocity branch instead, which proves the pair
// is checked from both directions but not the branch these tests name.

#[test]
fn a_transition_to_a_member_the_land_does_not_carry_is_rejected() {
    let mut members = compiled();
    transition(&mut members, "interior", "fixture_ascent").target_member = "cellar".into();
    link_rejection(&members, "which the land does not carry");
}

#[test]
fn a_dangling_transition_pair_is_rejected() {
    let mut members = compiled();
    transition(&mut members, "interior", "fixture_ascent").paired_transition =
        "fixture_hatch".into();
    link_rejection(&members, "which member \"surface\" does not carry");
}

#[test]
fn a_non_reciprocal_transition_pair_is_rejected() {
    let mut members = compiled();
    transition(&mut members, "surface", "fixture_descent").target_member = "surface".into();
    link_rejection(&members, "not exact reciprocals");
}

#[test]
fn a_transition_pair_that_both_go_down_is_rejected() {
    let mut members = compiled();
    transition(&mut members, "interior", "fixture_ascent").direction = "down".into();
    link_rejection(&members, "not complements");
}

#[test]
fn a_transition_through_a_blocked_endpoint_is_rejected() {
    let mut members = compiled();
    transition(&mut members, "surface", "fixture_descent").access = Point { x: 0, y: 0 };
    link_rejection(&members, "blocked endpoint");
}
