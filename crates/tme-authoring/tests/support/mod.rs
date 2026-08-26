//! Mutation helpers for the authoring compiler's mutant corpus.
//!
//! Every mutant starts from the tracked fixture, changes exactly one thing,
//! and asserts the compiler dies on it. Starting from the real document rather
//! than a hand-built minimal one is deliberate: a mutant that only proves a
//! check fires against a toy is a check qualified against a toy.

#![allow(dead_code)]

use serde_json::Value;
use tme_authoring::{MemberContract, validate_candidate};

pub const SURFACE_WIDTH: usize = 24;
pub const INTERIOR_WIDTH: usize = 10;

/// The land these mutants are cut from, and the member with the candidate
/// entry point they are judged through.
pub fn fixture_land() -> &'static tme_authoring::LandContract {
    tme_authoring::land("authoring_fixture").expect("the fixture land is carried")
}

pub fn fixture_surface() -> &'static MemberContract {
    fixture_land()
        .candidate_member()
        .expect("the fixture declares one candidate entry point")
}

pub fn surface_document() -> Value {
    serde_json::from_str(include_str!(
        "../../../../content/authoring-fixture/fixture-surface.tmj"
    ))
    .expect("the tracked surface member is valid JSON")
}

pub fn interior_document() -> Value {
    serde_json::from_str(include_str!(
        "../../../../content/authoring-fixture/fixture-interior.tmj"
    ))
    .expect("the tracked interior member is valid JSON")
}

pub fn layer<'a>(document: &'a mut Value, name: &str) -> &'a mut Value {
    document["layers"]
        .as_array_mut()
        .expect("layers is an array")
        .iter_mut()
        .find(|layer| layer["name"] == name)
        .unwrap_or_else(|| panic!("no layer named {name}"))
}

pub fn tile<'a>(
    document: &'a mut Value,
    layer_name: &str,
    width: usize,
    x: usize,
    y: usize,
) -> &'a mut Value {
    &mut layer(document, layer_name)["data"][y * width + x]
}

pub fn object<'a>(document: &'a mut Value, layer_name: &str, name: &str) -> &'a mut Value {
    layer(document, layer_name)["objects"]
        .as_array_mut()
        .expect("objects is an array")
        .iter_mut()
        .find(|object| object["name"] == name)
        .unwrap_or_else(|| panic!("no object named {name}"))
}

pub fn property<'a>(owner: &'a mut Value, name: &str) -> &'a mut Value {
    owner["properties"]
        .as_array_mut()
        .expect("properties is an array")
        .iter_mut()
        .find(|property| property["name"] == name)
        .unwrap_or_else(|| panic!("no property named {name}"))
        .get_mut("value")
        .expect("a property carries a value")
}

/// Run the candidate path and return the single blocking diagnostic. Panics if
/// the mutant was ACCEPTED, which is the failure this corpus exists to catch.
pub fn rejection(document: &Value) -> String {
    let report = validate_candidate(fixture_land().id, fixture_surface(), document)
        .expect("a candidate report is produced");
    assert!(
        !report.accepted,
        "mutant was accepted; the check it targets does not bind"
    );
    assert!(
        report.statistics.is_none(),
        "a rejected candidate has no statistics"
    );
    report
        .diagnostics
        .first()
        .cloned()
        .expect("a rejected candidate carries a diagnostic")
}

pub fn assert_rejects(document: &Value, fragment: &str) {
    let diagnostic = rejection(document);
    assert!(
        diagnostic.contains(fragment),
        "expected a diagnostic containing {fragment:?}, got {diagnostic:?}"
    );
}
