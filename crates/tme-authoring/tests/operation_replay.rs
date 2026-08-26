//! The staged-operation vocabulary, verb by verb: what each one can do, and
//! the blocking assertion each one provably trips.
//!
//! The Workbench spec binds the verb set with two constraints (§6.3): every
//! verb must be expressible against the current master without inventing a
//! parallel format, and **every verb must have a validator failure it provably
//! triggers**. The first is proven by the acceptance case; the second by the
//! rejection case beside it. A verb with no rejection here is a verb nobody has
//! shown the compiler can refuse, and it does not belong in the vocabulary.
//!
//! Both halves run through the CANDIDATE path, which by contract is the same
//! compile logic the promoted path runs. Nothing here writes a file, reads a
//! receipt, or touches the reviewed digest.

mod support;

use serde_json::{Value, json};
use support::{fixture_land, fixture_surface, surface_document};
use tme_authoring::{StagedOperation, replay};

fn operation(verb: &str, parameters: Value) -> StagedOperation {
    serde_json::from_value(json!({
        "record_id": "op-0001",
        "author": "owner",
        "class": "truth",
        "member": "surface",
        "verb": verb,
        "parameters": parameters,
    }))
    .expect("the staged operation is well formed")
}

/// Replay one verb against a fresh copy of the accepted master.
fn replayed(verb: &str, parameters: Value) -> Value {
    let mut document = surface_document();
    replay(
        fixture_surface(),
        &mut document,
        &[operation(verb, parameters)],
    )
    .expect("the replay applies");
    document
}

fn accept(verb: &str, parameters: Value) -> Value {
    let document = replayed(verb, parameters);
    let report = tme_authoring::validate_candidate(fixture_land().id, fixture_surface(), &document)
        .expect("a report");
    assert!(
        report.accepted,
        "{verb} was expected to produce an accepted candidate: {:?}",
        report.diagnostics
    );
    document
}

fn reject(verb: &str, parameters: Value, fragment: &str) {
    let document = replayed(verb, parameters);
    support::assert_rejects(&document, fragment);
}

/// The refusal a replay produces on its own, before any candidate exists.
fn refuse(verb: &str, parameters: Value) -> String {
    let mut document = surface_document();
    replay(
        fixture_surface(),
        &mut document,
        &[operation(verb, parameters)],
    )
    .expect_err("the replay refuses")
}

// ---------------------------------------------------------------------------
// Replay itself
// ---------------------------------------------------------------------------

#[test]
fn an_empty_log_reproduces_the_accepted_master_exactly() {
    let mut document = surface_document();
    replay(fixture_surface(), &mut document, &[]).expect("an empty replay applies");
    assert_eq!(
        document,
        surface_document(),
        "replaying nothing changed the document; the derived refresh is not idempotent"
    );
}

#[test]
fn the_same_log_against_the_same_base_produces_the_same_document() {
    let parameters = json!({"landmark_id": "fixture_ruin_marker", "to": {"x": 6, "y": 11}});
    assert_eq!(
        replayed("move_landmark", parameters.clone()),
        replayed("move_landmark", parameters),
    );
}

#[test]
fn operations_replay_in_log_order() {
    // Two edits to one cell: the later one is what the candidate carries.
    let mut document = surface_document();
    replay(
        fixture_surface(),
        &mut document,
        &[
            operation(
                "set_terrain",
                json!({"cells": [{"x": 2, "y": 1}], "class": "testland_forest"}),
            ),
            operation(
                "set_terrain",
                json!({"cells": [{"x": 2, "y": 1}], "class": "testland_marsh"}),
            ),
        ],
    )
    .expect("both edits apply");
    let report = tme_authoring::validate_candidate(fixture_land().id, fixture_surface(), &document)
        .expect("a report");
    assert!(report.accepted, "{:?}", report.diagnostics);
    let terrain = terrain_at(&document, 2, 1);
    assert_eq!(terrain, 4, "the marsh class is the last word");
}

fn terrain_at(document: &Value, x: usize, y: usize) -> u64 {
    let layer = document["layers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|layer| layer["name"] == "base_terrain")
        .unwrap();
    layer["data"][y * 24 + x].as_u64().unwrap()
}

// ---------------------------------------------------------------------------
// set_terrain
// ---------------------------------------------------------------------------

#[test]
fn set_terrain_repaints_the_base_layer() {
    let document = accept(
        "set_terrain",
        json!({"cells": [{"x": 2, "y": 1}, {"x": 2, "y": 4}], "class": "testland_forest"}),
    );
    assert_eq!(terrain_at(&document, 2, 1), 3);
    assert_eq!(terrain_at(&document, 2, 4), 3);
}

#[test]
fn set_terrain_blocking_a_structures_access_cell_is_rejected() {
    reject(
        "set_terrain",
        json!({"cells": [{"x": 8, "y": 8}], "class": "testland_deep_water"}),
        "structure fixture_structure_north access cell is blocked",
    );
}

#[test]
fn set_terrain_sealing_a_walkable_pocket_is_rejected() {
    reject(
        "set_terrain",
        json!({
            "cells": [{"x": 2, "y": 1}, {"x": 1, "y": 2}],
            "class": "testland_deep_water",
        }),
        "walkable cells no one can reach",
    );
}

#[test]
fn set_terrain_may_not_write_a_class_that_belongs_to_another_layer() {
    let refusal = refuse(
        "set_terrain",
        json!({"cells": [{"x": 2, "y": 1}], "class": "testland_path"}),
    );
    assert!(refusal.contains("belongs to the routes layer"), "{refusal}");
}

// ---------------------------------------------------------------------------
// set_route
// ---------------------------------------------------------------------------

#[test]
fn set_route_paints_and_clears_the_route_overlay() {
    let document = accept(
        "set_route",
        json!({"cells": [{"x": 12, "y": 3}], "class": "testland_path"}),
    );
    let routes = document["layers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|layer| layer["name"] == "routes")
        .unwrap();
    assert_eq!(routes["data"][3 * 24 + 12].as_u64().unwrap(), 7);
    accept(
        "set_route",
        json!({"cells": [{"x": 12, "y": 3}], "class": null}),
    );
}

#[test]
fn set_route_clearing_the_arrival_cell_is_rejected() {
    reject(
        "set_route",
        json!({"cells": [{"x": 12, "y": 14}], "class": null}),
        "the arrival landmark must stand on an authored route cell",
    );
}

// ---------------------------------------------------------------------------
// move_structure
// ---------------------------------------------------------------------------

#[test]
fn move_structure_carries_the_footprint_and_its_door() {
    let document = accept(
        "move_structure",
        json!({"structure_id": "fixture_structure_outland", "to": {"x": 20, "y": 5}}),
    );
    let report =
        tme_authoring::validate_candidate(fixture_land().id, fixture_surface(), &document).unwrap();
    let statistics = report
        .statistics
        .expect("an accepted candidate reports statistics");
    assert_eq!(
        statistics.structure_footprint_cells, 12,
        "a move changes where a footprint is, never how big it is"
    );
}

#[test]
fn move_structure_onto_the_wrong_ground_for_its_scope_is_rejected() {
    reject(
        "move_structure",
        json!({"structure_id": "fixture_structure_outland", "to": {"x": 8, "y": 9}}),
        "is scoped \"isolated\" but its footprint",
    );
}

#[test]
fn move_structure_naming_a_building_the_member_does_not_carry_is_refused() {
    let refusal = refuse(
        "move_structure",
        json!({"structure_id": "fixture_structure_west", "to": {"x": 2, "y": 2}}),
    );
    assert!(
        refusal.contains("is not authored in the structures layer"),
        "{refusal}"
    );
}

// ---------------------------------------------------------------------------
// set_structure_access
// ---------------------------------------------------------------------------

#[test]
fn set_structure_access_moves_the_door_it_derives() {
    let document = accept(
        "set_structure_access",
        json!({"structure_id": "fixture_structure_north", "cell": {"x": 9, "y": 8}}),
    );
    let report =
        tme_authoring::validate_candidate(fixture_land().id, fixture_surface(), &document).unwrap();
    assert!(report.accepted, "{:?}", report.diagnostics);
}

#[test]
fn set_structure_access_inside_the_footprint_is_rejected() {
    reject(
        "set_structure_access",
        json!({"structure_id": "fixture_structure_north", "cell": {"x": 8, "y": 7}}),
        "access cell must touch exactly one footprint cell, not 2",
    );
}

#[test]
fn set_structure_access_away_from_the_building_is_rejected() {
    reject(
        "set_structure_access",
        json!({"structure_id": "fixture_structure_north", "cell": {"x": 12, "y": 9}}),
        "access cell must touch exactly one footprint cell, not 0",
    );
}

// ---------------------------------------------------------------------------
// move_landmark
// ---------------------------------------------------------------------------

#[test]
fn move_landmark_carries_its_authored_marker_tile() {
    let document = accept(
        "move_landmark",
        json!({"landmark_id": "fixture_ruin_marker", "to": {"x": 6, "y": 11}}),
    );
    let marks = document["layers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|layer| layer["name"] == "landmark_marks")
        .unwrap();
    assert_eq!(
        marks["data"][11 * 24 + 5].as_u64().unwrap(),
        0,
        "the marker left the cell it stood on"
    );
    assert_eq!(marks["data"][11 * 24 + 6].as_u64().unwrap(), 10);
}

#[test]
fn move_landmark_onto_blocked_ground_is_rejected() {
    reject(
        "move_landmark",
        json!({"landmark_id": "fixture_ruin_marker", "to": {"x": 0, "y": 0}}),
        "landmark fixture_ruin_marker stands on a blocked cell",
    );
}

#[test]
fn move_landmark_naming_a_landmark_the_contract_does_not_carry_is_refused() {
    let refusal = refuse(
        "move_landmark",
        json!({"landmark_id": "fixture_shrine", "to": {"x": 6, "y": 11}}),
    );
    assert!(
        refusal.contains("is not authored in this member"),
        "{refusal}"
    );
}

// ---------------------------------------------------------------------------
// set_transition_endpoint
// ---------------------------------------------------------------------------

#[test]
fn set_transition_endpoint_moves_the_marker_and_its_tile() {
    let document = accept(
        "set_transition_endpoint",
        json!({
            "transition_id": "fixture_descent",
            "marker": {"x": 18, "y": 6},
            "access": {"x": 18, "y": 7},
        }),
    );
    let marks = document["layers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|layer| layer["name"] == "landmark_marks")
        .unwrap();
    assert_eq!(marks["data"][7 * 24 + 18].as_u64().unwrap(), 0);
    assert_eq!(marks["data"][6 * 24 + 18].as_u64().unwrap(), 11);
}

#[test]
fn set_transition_endpoint_separating_the_access_cell_is_rejected() {
    reject(
        "set_transition_endpoint",
        json!({"transition_id": "fixture_descent", "marker": null, "access": {"x": 18, "y": 10}}),
        "access cell must be cardinally adjacent to its marker",
    );
}

#[test]
fn set_transition_endpoint_that_moves_nothing_is_refused() {
    let refusal = refuse(
        "set_transition_endpoint",
        json!({"transition_id": "fixture_descent", "marker": null, "access": null}),
    );
    assert!(
        refusal.contains("neither a marker nor an access cell"),
        "{refusal}"
    );
}

// ---------------------------------------------------------------------------
// The envelope of the vocabulary itself
// ---------------------------------------------------------------------------

#[test]
fn an_unknown_verb_is_refused_and_names_the_vocabulary() {
    let refusal = refuse("paint_pretty", json!({}));
    assert!(refusal.contains("unknown verb"), "{refusal}");
    assert!(refusal.contains("set_terrain"), "{refusal}");
}

#[test]
fn a_cell_outside_the_envelope_is_refused() {
    let refusal = refuse(
        "set_terrain",
        json!({"cells": [{"x": 24, "y": 0}], "class": "testland_grass"}),
    );
    assert!(refusal.contains("outside the 24x16 envelope"), "{refusal}");
}

#[test]
fn an_unknown_parameter_is_refused_rather_than_ignored() {
    let refusal = refuse(
        "move_landmark",
        json!({"landmark_id": "fixture_ruin_marker", "to": {"x": 6, "y": 11}, "why": "because"}),
    );
    assert!(refusal.contains("parameters are malformed"), "{refusal}");
}

#[test]
fn a_dressing_or_asset_operation_is_not_replayed_here() {
    let mut document = surface_document();
    let operation: StagedOperation = serde_json::from_value(json!({
        "record_id": "op-0007",
        "author": "owner",
        "class": "dressing",
        "member": "surface",
        "verb": "set_terrain",
        "parameters": {"cells": [], "class": "testland_grass"},
    }))
    .unwrap();
    let refusal =
        replay(fixture_surface(), &mut document, &[operation]).expect_err("a class refusal");
    assert!(refusal.contains("replays truth operations"), "{refusal}");
}

#[test]
fn an_operation_against_another_member_is_refused() {
    let mut document = surface_document();
    let operation: StagedOperation = serde_json::from_value(json!({
        "record_id": "op-0008",
        "author": "owner",
        "class": "truth",
        "member": "interior",
        "verb": "set_terrain",
        "parameters": {"cells": [], "class": "testland_grass"},
    }))
    .unwrap();
    let refusal =
        replay(fixture_surface(), &mut document, &[operation]).expect_err("a member refusal");
    assert!(refusal.contains("has no candidate path"), "{refusal}");
}

/// Every verb the published table names has both halves above. The count is
/// asserted so that adding a verb without adding its rejection turns this red.
#[test]
fn every_published_verb_has_an_acceptance_and_a_rejection_here() {
    let source = include_str!("operation_replay.rs");
    for spec in tme_authoring::VOCABULARY {
        let mentions = source.matches(&format!("\"{}\"", spec.verb)).count();
        assert!(
            mentions >= 2,
            "{} appears {mentions} time(s) in this file; it needs an acceptance and a rejection",
            spec.verb
        );
    }
}
