//! The repository's own content compiles, and the compiler explains what's
//! wrong with content that doesn't.

use std::fs;
use std::path::{Path, PathBuf};
use tme_content::{Problem, load};
use tme_sim::{Id, Side};

fn repo_content() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

#[test]
fn the_repositorys_content_compiles() {
    let content =
        load(&repo_content()).unwrap_or_else(|p| panic!("content/ doesn't compile:\n{p}"));
    let c = &content.catalogue;
    let place = c.place(&Id::new("lattermead"));
    // Both sides of the same place, each with its own solids: the hall is
    // whole in Lately and broken in the living land.
    assert_ne!(place.living, place.lately);
    assert!(!place.thresholds.is_empty());
    assert!(place.lairs.iter().all(|l| l.side == Side::Living));
    // Every person has a body a dun can kill, and a dun a body a person can.
    let person = c.body(&c.rules.person);
    let dun = c.body(&Id::new("dun"));
    assert!(person.health > 0 && dun.attack.damage > 0);
    // A person can always walk away from a dun.
    assert!(person.speed > dun.speed);
    assert!(content.scenery[&Id::new("lattermead")].props.len() > 20);
}

/// A copy of the repository's content in a scratch folder, to break.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tme-content-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    copy(&repo_content(), &dir);
    dir
}

fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn problems(dir: &Path) -> Vec<Problem> {
    let out = load(dir).expect_err("broken content doesn't compile").0;
    fs::remove_dir_all(dir).unwrap();
    out
}

#[test]
fn a_reference_to_nothing_names_the_file_and_what_is_missing() {
    let dir = scratch("dangling");
    let place = dir.join("places/lattermead.toml");
    let text = fs::read_to_string(&place)
        .unwrap()
        .replace("kind = \"dun\"", "kind = \"wolf\"");
    fs::write(&place, text).unwrap();
    let p = problems(&dir);
    assert_eq!(p.len(), 1, "{p:?}");
    assert_eq!(p[0].file, Path::new("places/lattermead.toml"));
    assert!(p[0].message.contains("bodies/wolf"), "{}", p[0]);
}

#[test]
fn a_malformed_file_names_its_line() {
    let dir = scratch("malformed");
    fs::write(
        dir.join("marks/pale.toml"),
        "name = \"Pale\"\ndescription = \"x\"\n\n[effects]\nmax_health = \"lots\"\n",
    )
    .unwrap();
    let p = problems(&dir);
    assert_eq!(p.len(), 1, "{p:?}");
    assert_eq!(p[0].file, Path::new("marks/pale.toml"));
    assert_eq!(p[0].line, Some(5));
}

#[test]
fn every_problem_is_reported_not_only_the_first() {
    let dir = scratch("several");
    fs::write(dir.join("bodies/notes.txt"), "stray").unwrap();
    fs::write(dir.join("marks/Bad Name.toml"), "").unwrap();
    let body = dir.join("bodies/dun.toml");
    let text = fs::read_to_string(&body)
        .unwrap()
        .replace("wind_up_ms = 900", "wind_up_ms = 0");
    fs::write(&body, text).unwrap();
    let p = problems(&dir);
    assert_eq!(p.len(), 3, "{p:?}");
    assert!(p.iter().any(|p| p.message.contains("winds up")));
}

#[test]
fn the_rules_cannot_ask_for_more_tardies_than_there_are() {
    let dir = scratch("tardies");
    let rules = dir.join("rules.toml");
    let text = fs::read_to_string(&rules)
        .unwrap()
        .replace("max_tardies = 3", "max_tardies = 9");
    fs::write(&rules, text).unwrap();
    let p = problems(&dir);
    assert_eq!(p[0].file, Path::new("rules.toml"));
}
