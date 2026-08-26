//! The logical projection the Workbench renders.
//!
//! Two properties matter and neither is checkable by reading the emitter. It
//! must be **deterministic**, because the Workbench binds selections to its
//! digest and a document that varied between runs would make every packet
//! stale on the next build. And it must be **the compiled land**, because a
//! view that quietly disagreed with the compiler would put a second geography
//! authority into the tool that is forbidden from having one.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;
use sha2::{Digest, Sha256};
use tme_authoring::{BuildMode, LandContract, build_land, repository_root};

/// This corpus proves the projection over the authoring fixture, which is
/// the land that exists to be a logical target. Every other land goes
/// through the same emitter.
fn fixture() -> &'static LandContract {
    tme_authoring::land("authoring_fixture").expect("the fixture land is carried")
}

fn projection_output() -> &'static str {
    fixture().workbench_projection_output
}

const FIXTURE_FILES: [&str; 3] = [
    "content/authoring-fixture/promotion.json",
    "content/authoring-fixture/fixture-surface.tmj",
    "content/authoring-fixture/fixture-interior.tmj",
];

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn staged_root() -> PathBuf {
    let source = repository_root().expect("the repository root resolves");
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "workbench-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(root.join("content/authoring-fixture"))
        .expect("the staged fixture directory is created");
    for relative in FIXTURE_FILES {
        fs::copy(source.join(relative), root.join(relative)).expect("the fixture is staged");
    }
    // The projection is proven against the runtime's own terrain registry, so
    // the staged root needs the registry too.
    let registry = "content/test-corpus/catalogs/prototype_catalog_v6.json";
    fs::create_dir_all(root.join("content/test-corpus/catalogs"))
        .expect("the staged registry directory is created");
    fs::copy(source.join(registry), root.join(registry)).expect("the registry is staged");
    root
}

fn build(root: &Path) {
    build_land(
        root,
        fixture(),
        BuildMode {
            check: false,
            report: false,
        },
    )
    .expect("the staged fixture compiles");
}

fn tracked_document() -> Value {
    let root = repository_root().expect("the repository root resolves");
    serde_json::from_slice(
        &fs::read(root.join(projection_output())).expect("the projection is readable"),
    )
    .expect("the projection is JSON")
}

fn member<'a>(document: &'a Value, name: &str) -> &'a Value {
    document["members"]
        .as_array()
        .expect("members is an array")
        .iter()
        .find(|member| member["member"] == name)
        .unwrap_or_else(|| panic!("no member named {name}"))
}

fn cell(member: &Value, x: u64, y: u64) -> &Value {
    member["cells"]
        .as_array()
        .expect("cells is an array")
        .iter()
        .find(|cell| cell["x"] == x && cell["y"] == y)
        .unwrap_or_else(|| panic!("no cell at {x},{y}"))
}

fn classes(cell: &Value) -> Vec<(String, String)> {
    cell["terrain"]
        .as_array()
        .expect("terrain is an array")
        .iter()
        .map(|entry| {
            (
                entry["class"].as_str().expect("a class name").to_owned(),
                entry["layer"].as_str().expect("a layer name").to_owned(),
            )
        })
        .collect()
}

#[test]
fn the_tracked_projection_is_exactly_what_a_fresh_build_writes() {
    let root = repository_root().expect("the repository root resolves");
    build_land(
        &root,
        fixture(),
        BuildMode {
            check: true,
            report: false,
        },
    )
    .expect("the tracked projection is current");
}

#[test]
fn two_builds_of_the_same_fixture_are_byte_identical() {
    let first = staged_root();
    let second = staged_root();
    build(&first);
    build(&second);
    let left = fs::read(first.join(projection_output())).expect("the first run wrote");
    let right = fs::read(second.join(projection_output())).expect("the second run wrote");
    assert_eq!(left, right, "the logical projection is not deterministic");
}

#[test]
fn a_rebuild_over_an_existing_projection_reproduces_it() {
    let root = staged_root();
    build(&root);
    let first = fs::read(root.join(projection_output())).expect("the first run wrote");
    build(&root);
    let second = fs::read(root.join(projection_output())).expect("the second run wrote");
    assert_eq!(first, second);
}

#[test]
fn every_named_source_digest_is_the_file_on_disk() {
    let root = repository_root().expect("the repository root resolves");
    let document = tracked_document();
    let sources = document["sources"].as_array().expect("sources is an array");
    let roles = sources
        .iter()
        .map(|source| source["role"].as_str().expect("a role").to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        roles,
        ["master", "companion", "receipt", "runtime_projection"],
        "the source binding lost or reordered a role"
    );
    for source in sources {
        let path = source["path"].as_str().expect("a path");
        let claimed = source["sha256"].as_str().expect("a digest");
        let bytes = fs::read(root.join(path)).expect("the named source is readable");
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            claimed,
            "{path} digest does not match the bytes on disk"
        );
    }
}

#[test]
fn the_projection_carries_every_cell_of_every_member_exactly_once() {
    let document = tracked_document();
    for (name, width, height) in [("surface", 24_u64, 16_u64), ("interior", 10, 8)] {
        let member = member(&document, name);
        assert_eq!(member["width"], width);
        assert_eq!(member["height"], height);
        let cells = member["cells"].as_array().expect("cells is an array");
        assert_eq!(cells.len() as u64, width * height);
        let seen = cells
            .iter()
            .map(|cell| {
                (
                    cell["x"].as_u64().expect("an x"),
                    cell["y"].as_u64().expect("a y"),
                )
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(seen.len() as u64, width * height, "{name} repeats a cell");
        let ordered = cells
            .iter()
            .map(|cell| {
                cell["y"].as_u64().expect("a y") * width + cell["x"].as_u64().expect("an x")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            ordered,
            (0..width * height).collect::<Vec<_>>(),
            "{name} cells are not in row-major order"
        );
    }
}

#[test]
fn passability_matches_the_compilers_own_count() {
    let document = tracked_document();
    for (name, expected) in [("surface", 281), ("interior", 38)] {
        let passable = member(&document, name)["cells"]
            .as_array()
            .expect("cells is an array")
            .iter()
            .filter(|cell| cell["passable"] == true)
            .count();
        assert_eq!(passable, expected, "{name} passable-cell count moved");
    }
}

#[test]
fn terrain_carries_the_authored_layer_each_class_belongs_to() {
    let document = tracked_document();
    let surface = member(&document, "surface");
    // A structure footprint cell: town ground with a footprint stacked on it.
    assert_eq!(
        classes(cell(surface, 8, 6)),
        [
            ("testland_town_ground".into(), "base_terrain".into()),
            (
                "testland_structure_footprint".into(),
                "structure_footprints".into()
            ),
        ]
    );
    // A route over grass: both survive, each attributed to its own layer.
    assert_eq!(
        classes(cell(surface, 6, 4)),
        [
            ("testland_grass".into(), "base_terrain".into()),
            ("testland_path".into(), "routes".into()),
        ]
    );
    // A bridge over deep water REPLACES it, which is the compiler's rule and
    // must not be re-decided by the view.
    assert_eq!(
        classes(cell(surface, 3, 4)),
        [("testland_bridge".into(), "routes".into())]
    );
    // A landmark mark stacks on its ground. The cell is READ from the landmark
    // the mark belongs to, because the claim is about the stack over a marked
    // cell and not about a coordinate — and a coordinate written down here is a
    // coordinate that goes stale the first time the landmark is moved.
    let ruin = surface["landmarks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "fixture_ruin_marker")
        .expect("the surface carries the ruin landmark");
    assert_eq!(
        classes(cell(
            surface,
            ruin["at"]["x"].as_u64().unwrap(),
            ruin["at"]["y"].as_u64().unwrap(),
        )),
        [
            ("testland_grass".into(), "base_terrain".into()),
            ("testland_ruin_ground".into(), "landmark_marks".into()),
        ]
    );
}

#[test]
fn the_authored_features_arrive_with_their_geometry() {
    let document = tracked_document();
    let surface = member(&document, "surface");

    let structures = surface["structures"].as_array().expect("an array");
    assert_eq!(structures.len(), 3);
    let north = structures
        .iter()
        .find(|row| row["id"] == "fixture_structure_north")
        .expect("the north structure");
    assert_eq!(north["x"], 8);
    assert_eq!(north["y"], 6);
    assert_eq!(north["width"], 2);
    assert_eq!(north["height"], 2);
    assert_eq!(north["access"]["x"], 8);
    assert_eq!(north["access"]["y"], 8);
    assert_eq!(north["facade_door"]["x"], 8);
    assert_eq!(north["facade_door"]["y"], 7);

    let landmarks = surface["landmarks"].as_array().expect("an array");
    assert_eq!(landmarks.len(), 2);
    assert_eq!(landmarks[0]["id"], "fixture_dock_arrival");
    assert_eq!(landmarks[0]["at"]["x"], 12);
    assert_eq!(landmarks[0]["at"]["y"], 14);

    let transitions = surface["transitions"].as_array().expect("an array");
    assert_eq!(transitions.len(), 1);
    assert_eq!(transitions[0]["id"], "fixture_descent");
    assert_eq!(transitions[0]["marker"]["x"], 18);
    assert_eq!(transitions[0]["marker"]["y"], 7);
    assert_eq!(transitions[0]["access"]["x"], 18);
    assert_eq!(transitions[0]["access"]["y"], 8);

    assert_eq!(surface["routes"].as_array().expect("an array").len(), 38);

    let interior = member(&document, "interior");
    assert!(
        interior["structures"]
            .as_array()
            .expect("an array")
            .is_empty()
    );
    assert!(
        interior["landmarks"]
            .as_array()
            .expect("an array")
            .is_empty()
    );
    assert_eq!(
        interior["transitions"].as_array().expect("an array")[0]["id"],
        "fixture_ascent"
    );
}

#[test]
fn every_route_cell_is_a_cell_the_grid_agrees_is_routed() {
    let document = tracked_document();
    let surface = member(&document, "surface");
    for route in surface["routes"].as_array().expect("an array") {
        let x = route["x"].as_u64().expect("an x");
        let y = route["y"].as_u64().expect("a y");
        assert!(
            classes(cell(surface, x, y))
                .iter()
                .any(|(_, layer)| layer == "routes"),
            "route cell {x},{y} carries no routes-layer terrain"
        );
    }
}

#[test]
fn a_moved_fixture_moves_the_projections_source_digest() {
    let root = staged_root();
    build(&root);
    let before = tracked_source_digest(&root, "master");

    let master = root.join("content/authoring-fixture/fixture-surface.tmj");
    let mut document: Value =
        serde_json::from_slice(&fs::read(&master).expect("readable")).expect("JSON");
    document["properties"] = document["properties"].clone();
    let bytes = serde_json::to_vec_pretty(&document).expect("serializes");
    fs::write(&master, bytes).expect("the mutant is written");

    // The gate rejects the mutant outright, which is the promotion contract
    // doing its job; the point here is that the view can never be rebuilt to
    // silently describe different bytes under the same digest.
    let error = build_land(
        &root,
        fixture(),
        BuildMode {
            check: false,
            report: false,
        },
    )
    .expect_err("a moved master is rejected");
    assert!(error.contains("digest mismatch"), "{error}");
    assert_eq!(before, tracked_source_digest(&root, "master"));
}

fn tracked_source_digest(root: &Path, role: &str) -> String {
    let document: Value = serde_json::from_slice(
        &fs::read(root.join(projection_output())).expect("the projection is readable"),
    )
    .expect("the projection is JSON");
    document["sources"]
        .as_array()
        .expect("an array")
        .iter()
        .find(|source| source["role"] == role)
        .expect("the role is bound")["sha256"]
        .as_str()
        .expect("a digest")
        .to_owned()
}
