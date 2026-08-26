//! The identity proof's land: the first authored land a runtime loads.
//!
//! The compiler's fail-closed classes are qualified against the authoring
//! fixture, member by member, in the other test files. What this file proves is
//! the land itself — that the geography the proof needs is the geography the
//! compiler accepted, and that the emitted runtime template says so.
//!
//! Owner rulings this file pins: **R1** (the land is authoring-compiled and the
//! runtime loads the compiler's output) and **R3** (the 48 x 32 envelope).

use std::fs;

use serde_json::{Value, json};
use tme_authoring::{LandContract, MemberContract, repository_root};

fn land() -> &'static LandContract {
    tme_authoring::land("identity_proof").expect("the identity proof's land is carried")
}

fn settlement() -> &'static MemberContract {
    land().member("settlement").expect("the settlement member")
}

fn document() -> Value {
    serde_json::from_str(include_str!(
        "../../../content/lands/identity-proof/settlement.tmj"
    ))
    .expect("the authored settlement is valid JSON")
}

fn emitted_template() -> Value {
    let root = repository_root().expect("the repository root resolves");
    serde_json::from_slice(
        &fs::read(root.join(land().world_template_output)).expect("the template is readable"),
    )
    .expect("the emitted template is JSON")
}

/// R3: one envelope, 48 x 32, and the slice that adds the layer beneath adds a
/// member rather than editing a type.
#[test]
fn the_settlement_is_the_ruled_envelope_and_the_land_declares_its_own_member_count() {
    assert_eq!(settlement().envelope(), (48, 32));
    assert_eq!(land().members.len(), 1);
    assert_eq!(land().master().id, "settlement");
    assert!(land().companions().is_empty());
}

#[test]
fn the_authored_settlement_compiles_with_the_cast_s_geography() {
    let compiled = tme_authoring::compile_member(settlement(), &document())
        .expect("the tracked settlement compiles");
    let report = compiled.report();
    assert_eq!(report.member, "settlement");
    assert_eq!((report.width, report.height), (48, 32));
    assert_eq!(report.structures.len(), 3, "{:?}", report.structures);
    assert_eq!(
        report.landmarks,
        ["settlement_arrival", "settlement_ruin_mouth"],
    );
    assert!(report.transitions.is_empty(), "S1 authors one member");
    // The dangerous route: the arrival and the ruin mouth are joined by
    // authored route cells, which is what makes "walk out to the lair" a
    // journey rather than a teleport.
    let arrival = report.arrival.expect("the settlement declares the arrival");
    let ruin = compiled.landmarks()["settlement_ruin_mouth"].at;
    assert!(compiled.route_cells().contains(&arrival));
    assert!(compiled.route_cells().contains(&ruin));
    assert!(
        ruin.x - arrival.x >= 20,
        "the route out is {} cells long; the lair should not be next door",
        ruin.x - arrival.x
    );
}

/// R1: the emitted runtime template is what the server loads, so its shape is
/// part of this slice's contract rather than an implementation detail.
#[test]
fn the_emitted_template_is_one_realm_one_level_and_one_arrival() {
    let template = emitted_template();
    assert_eq!(template["id"], "identity_proof");
    assert_eq!(template["schema_version"], 3);
    let realms = template["realms"].as_object().expect("realms is an object");
    assert_eq!(realms.len(), 1);
    let levels = realms["identity_proof"]["levels"]
        .as_object()
        .expect("levels is an object");
    assert_eq!(levels.keys().collect::<Vec<_>>(), ["settlement"]);
    assert_eq!(levels["settlement"]["width"], 48);
    assert_eq!(levels["settlement"]["height"], 32);
    assert_eq!(
        template["arrivals"]["settlement_arrival"],
        json!({"realm": "identity_proof", "level": "settlement", "position": {"x": 3, "y": 16}})
    );
    assert_eq!(
        template["topology"].as_object().expect("an object").len(),
        0,
        "a one-member land has no cross-member topology"
    );
}

/// R1 again, at the anchor that carries it: this land's receipt is the only one
/// in the tree that authorizes a runtime to load authoring output, and the
/// fixture's still refuses it.
#[test]
fn only_this_land_carries_the_runtime_loading_authority() {
    assert!(land().authority.runtime_loads_authoring_source);
    assert!(!land().authority.content_canon);
    assert!(!land().authority.gameplay_tuning);
    assert!(!land().authority.presentation_art);
    let fixture = tme_authoring::land("authoring_fixture").expect("the fixture land is carried");
    assert!(!fixture.authority.runtime_loads_authoring_source);
}

/// The owner accepted this geography on 2026-08-21, after sending the first
/// authored version back for a shape pass. What that acceptance is worth is
/// bounded by the authority block above it — art, tuning and canon stay
/// outside it — and the receipt on disk must agree with this contract exactly,
/// which `tests/promotion_gate.rs` proves by mutating each field in turn.
#[test]
fn the_geography_carries_the_owners_acceptance_and_not_more_than_it() {
    assert_eq!(land().receipt_status, "owner_accepted_at_s1");
    assert_eq!(land().receipt_attested_by, "peter");
    assert_eq!(land().receipt_attested_on, "2026-08-21");
    let receipt: Value = {
        let root = repository_root().expect("the repository root resolves");
        serde_json::from_slice(&fs::read(root.join(land().receipt_path)).expect("readable"))
            .expect("the receipt is JSON")
    };
    assert_eq!(receipt["status"], land().receipt_status);
    assert_eq!(receipt["attested_by"], land().receipt_attested_by);
    assert_eq!(receipt["master"]["sha256"], land().master_digest);
    assert_eq!(receipt["authority"]["content_canon"], json!(false));
    assert_eq!(receipt["authority"]["presentation_art"], json!(false));
    assert_eq!(receipt["authority"]["gameplay_tuning"], json!(false));
}

/// The candidate path reaches this land's member too — which is what item 9's
/// map edit will need, and what a land declaring no entry point would refuse.
#[test]
fn the_settlement_is_the_lands_candidate_entry_point_and_refuses_a_drifted_document() {
    assert_eq!(
        land().candidate_member().expect("a candidate member").id,
        "settlement"
    );
    let accepted = tme_authoring::validate_candidate(land().id, settlement(), &document())
        .expect("a candidate report is produced");
    assert!(accepted.accepted, "{:?}", accepted.diagnostics);
    assert_eq!(accepted.land, "identity_proof");
    assert_eq!(accepted.member, "settlement");

    let mut drifted = document();
    drifted["width"] = json!(49);
    let rejected = tme_authoring::validate_candidate(land().id, settlement(), &drifted)
        .expect("a candidate report is produced");
    assert!(!rejected.accepted);
    assert!(
        rejected.diagnostics[0].contains("authored map.width must be 48"),
        "{:?}",
        rejected.diagnostics
    );
}

/// The land's terrain vocabulary is the runtime registry's. A class the
/// registry does not carry fails the compile, not the running world.
#[test]
fn every_terrain_class_the_settlement_paints_resolves_in_the_bound_registry() {
    let root = repository_root().expect("the repository root resolves");
    let catalog: Value = serde_json::from_slice(
        &fs::read(root.join(land().terrain_registry_catalog)).expect("the registry is readable"),
    )
    .expect("the registry is JSON");
    let profile = &catalog["profiles"][land().terrain_registry_profile]["terrains"];
    let registered = profile
        .as_array()
        .expect("the profile selects terrains")
        .iter()
        .map(|key| {
            catalog["terrains"][key.as_str().expect("a key")]["id"]
                .as_str()
                .expect("a terrain id")
                .to_owned()
        })
        .collect::<Vec<_>>();
    for class in settlement().classes {
        let annotation = matches!(
            class.role,
            tme_authoring::contract::TileRole::Passability { .. }
        );
        assert_eq!(
            !annotation,
            registered.contains(&class.name.to_owned()),
            "{} is {}registered, which is not what its role implies",
            class.name,
            if annotation { "" } else { "not " }
        );
    }
}
