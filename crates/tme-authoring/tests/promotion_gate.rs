//! Mutants for the promotion gate, and the separation between the promoted
//! path and the candidate path.
//!
//! The gate is double-anchored on purpose, so the corpus includes the mutant
//! that matters most: an attacker (or a well-meaning script) who rewrites the
//! authored bytes AND re-signs the receipt to match. That mutant defeats a
//! receipt-only gate and dies here against the reviewed constant.
//!
//! **Every mutant runs against every authored land.** The gate is one
//! implementation over a table of lands, so a land that joined the table
//! without its receipt being proven would be a land nobody checked.

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};
use tme_authoring::{BuildMode, LandContract, build, repository_root};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn lands() -> Vec<&'static LandContract> {
    ["authoring_fixture", "identity_proof"]
        .into_iter()
        .map(|id| tme_authoring::land(id).expect("the land is carried"))
        .collect()
}

/// A throwaway root holding just one land's authored bytes, so a mutant never
/// touches the tracked tree.
fn staged_root(land: &'static LandContract) -> PathBuf {
    let source = repository_root().expect("the repository root resolves");
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "promotion-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let mut files = vec![land.receipt_path];
    files.extend(land.members.iter().map(|member| member.document));
    for relative in files {
        let destination = root.join(relative);
        fs::create_dir_all(destination.parent().expect("a staged file has a parent"))
            .expect("the staged directory is created");
        fs::copy(source.join(relative), destination).expect("the authored file is staged");
    }
    root
}

fn receipt(root: &Path, land: &'static LandContract) -> Value {
    serde_json::from_slice(
        &fs::read(root.join(land.receipt_path)).expect("the receipt is readable"),
    )
    .expect("the receipt is JSON")
}

fn write_receipt(root: &Path, land: &'static LandContract, value: &Value) {
    fs::write(
        root.join(land.receipt_path),
        serde_json::to_vec_pretty(value).expect("the receipt serializes"),
    )
    .expect("the receipt is written");
}

fn assert_rejects(root: &Path, land: &'static LandContract, fragment: &str) {
    let diagnostic = tme_authoring::load(root, land).expect_err("the mutant must be rejected");
    assert!(
        diagnostic.contains(fragment),
        "{}: expected a diagnostic containing {fragment:?}, got {diagnostic:?}",
        land.id
    );
}

/// Mutate one land's receipt and assert the gate refuses it, for every land.
fn every_land_rejects(fragment: &str, mutate: impl Fn(&mut Value)) {
    for land in lands() {
        let root = staged_root(land);
        let mut value = receipt(&root, land);
        mutate(&mut value);
        write_receipt(&root, land, &value);
        assert_rejects(&root, land, fragment);
    }
}

#[test]
fn every_staged_land_loads() {
    for land in lands() {
        let root = staged_root(land);
        let compiled = tme_authoring::load(&root, land).expect("the unmutated land loads");
        assert_eq!(compiled.members().count(), land.members.len());
        assert_eq!(compiled.master_digest(), land.master_digest);
        assert!(compiled.arrival_member().is_ok());
    }
}

#[test]
fn an_edited_master_is_rejected() {
    for land in lands() {
        let root = staged_root(land);
        let path = root.join(land.master().document);
        let mut bytes = fs::read(&path).unwrap();
        bytes.push(b'\n');
        fs::write(&path, bytes).unwrap();
        assert_rejects(&root, land, "digest mismatch");
    }
}

#[test]
fn an_edited_companion_is_rejected() {
    let land = tme_authoring::land("authoring_fixture").expect("the fixture is carried");
    let companion = land
        .companions()
        .first()
        .expect("the fixture carries a companion");
    let root = staged_root(land);
    let path = root.join(companion.document);
    let mut bytes = fs::read(&path).unwrap();
    bytes.push(b'\n');
    fs::write(&path, bytes).unwrap();
    assert_rejects(&root, land, "digest mismatch");
}

/// The mutant a single-anchor gate cannot survive: the authored bytes change
/// and the receipt is re-signed to agree with them. Only the reviewed constant
/// in source notices.
#[test]
fn a_master_edited_and_resigned_together_is_rejected() {
    for land in lands() {
        let root = staged_root(land);
        let path = root.join(land.master().document);
        let mut bytes = fs::read(&path).unwrap();
        bytes.push(b'\n');
        let resigned = format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(&bytes));
        fs::write(&path, bytes).unwrap();
        let mut value = receipt(&root, land);
        value["master"]["sha256"] = json!(resigned);
        write_receipt(&root, land, &value);
        assert_rejects(&root, land, "differs from the attested contract");
    }
}

#[test]
fn an_over_broad_authority_block_is_rejected() {
    every_land_rejects("incomplete or over-broad", |value| {
        value["authority"]["presentation_art"] = json!(true);
    });
}

#[test]
fn an_authority_block_claiming_canon_is_rejected() {
    every_land_rejects("incomplete or over-broad", |value| {
        value["authority"]["content_canon"] = json!(true);
    });
}

#[test]
fn an_incomplete_authority_block_is_rejected() {
    every_land_rejects("incomplete or over-broad", |value| {
        value["authority"]["coordinates"] = json!(false);
    });
}

/// The authority to be loaded by a runtime is per-land and exact in both
/// directions: the fixture may not claim it, and the identity proof's land may
/// not lose it.
#[test]
fn a_flipped_runtime_loading_authority_is_rejected() {
    every_land_rejects("incomplete or over-broad", |value| {
        let current = value["authority"]["runtime_loads_authoring_source"]
            .as_bool()
            .expect("the authority block declares the flag");
        value["authority"]["runtime_loads_authoring_source"] = json!(!current);
    });
}

#[test]
fn a_promoted_status_the_owner_did_not_grant_is_rejected() {
    every_land_rejects("differs from the attested contract", |value| {
        value["status"] = json!("owner_approved_authoring_source");
    });
}

#[test]
fn an_empty_provenance_chain_is_rejected() {
    every_land_rejects("differs from the attested contract", |value| {
        value["research_boundary"]["review_refs"] = json!([]);
    });
}

/// A receipt must name exactly the members the contract declares. For a land
/// with a companion that means losing one; for a land without, it means
/// growing one. Both are the same defect: a receipt describing a land the
/// contract does not.
#[test]
fn a_member_set_the_contract_does_not_declare_is_rejected() {
    for land in lands() {
        let root = staged_root(land);
        let mut value = receipt(&root, land);
        if land.companions().is_empty() {
            value["companions"] = json!([{
                "path": "content/authoring-fixture/fixture-interior.tmj",
                "sha256": "0".repeat(64),
            }]);
        } else {
            value["companions"] = json!([]);
        }
        write_receipt(&root, land, &value);
        assert_rejects(
            &root,
            land,
            "must name exactly the members the contract declares",
        );
    }
}

#[test]
fn an_unrecognized_receipt_field_is_rejected() {
    every_land_rejects("unknown field", |value| {
        value["also_approves"] = json!("everything else");
    });
}

#[test]
fn a_missing_receipt_is_rejected() {
    for land in lands() {
        let root = staged_root(land);
        fs::remove_file(root.join(land.receipt_path)).unwrap();
        assert_rejects(&root, land, "read ");
    }
}

/// The candidate path must reach a member's semantics without the receipt.
/// Proven by running it against a root that has no receipt at all.
#[test]
fn the_candidate_path_needs_no_receipt_and_writes_nothing() {
    let land = support::fixture_land();
    let root = staged_root(land);
    fs::remove_file(root.join(land.receipt_path)).unwrap();
    let directory = root.join("content/authoring-fixture");
    let before = fs::read_dir(&directory).unwrap().count();

    let accepted = tme_authoring::validate_candidate(
        land.id,
        support::fixture_surface(),
        &support::surface_document(),
    )
    .expect("a candidate report is produced");
    assert!(accepted.accepted);

    let mut broken = support::surface_document();
    broken["width"] = json!(25);
    let rejected = tme_authoring::validate_candidate(land.id, support::fixture_surface(), &broken)
        .expect("a candidate report is produced");
    assert!(!rejected.accepted);
    assert_ne!(accepted.candidate_sha256, rejected.candidate_sha256);

    assert_eq!(
        before,
        fs::read_dir(&directory).unwrap().count(),
        "the candidate path wrote into the authored directory"
    );
}

/// One implementation of member semantics, reached two ways. If these ever
/// disagree, a second validator has appeared.
#[test]
fn the_candidate_and_promoted_paths_report_the_same_member() {
    let land = support::fixture_land();
    let root = staged_root(land);
    let compiled = tme_authoring::load(&root, land).expect("the land loads");
    let candidate = tme_authoring::validate_candidate(
        land.id,
        support::fixture_surface(),
        &support::surface_document(),
    )
    .expect("a candidate report is produced");
    assert_eq!(
        candidate.statistics.as_ref(),
        Some(compiled.member("surface").expect("the surface").report()),
    );
}

/// `--check` proves the tracked projections are exactly what a fresh run
/// writes, and running it twice proves the reports are byte-reproducible.
#[test]
fn the_tracked_projections_are_current_and_the_reports_reproduce() {
    let root = repository_root().expect("the repository root resolves");
    let mode = BuildMode {
        check: true,
        report: true,
    };
    let first = build(&root, mode).expect("the tracked projections are current");
    let second = build(&root, mode).expect("the tracked projections are current");
    assert_eq!(first, second, "the reports are not byte-reproducible");
    assert_eq!(
        first.len(),
        lands().len() * 2,
        "one summary and one report per land"
    );
    for land in lands() {
        assert!(
            first
                .iter()
                .any(|line| line.contains(&format!("authored land {}: PASS", land.id))),
            "{} was not compiled",
            land.id
        );
    }
    assert!(first[1].contains("\"kind\": \"authored_land_compile_report\""));
}
