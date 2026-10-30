//! A world's folder keeps its credentials, the invite code and the world's
//! secret key, from everyone but its host.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use tme_server::folder::{Folder, NewWorld};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tme-folder-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn create(dir: &Path) {
    Folder::create(
        dir,
        NewWorld {
            name: "Lattermead",
            content: &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
            bind: "127.0.0.1:0",
            address: "unused:0",
        },
    )
    .expect("a new world");
}

fn others_can_read(path: &Path) -> bool {
    fs::metadata(path).unwrap().permissions().mode() & 0o077 != 0
}

#[test]
fn a_new_world_folder_is_its_hosts_alone() {
    let dir = scratch("new");
    create(&dir);
    assert!(!others_can_read(&dir), "the folder");
    for file in ["world.toml", "world.key"] {
        assert!(!others_can_read(&dir.join(file)), "{file}");
    }
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_world_made_in_a_folder_anyone_can_open_keeps_its_credentials() {
    let dir = scratch("open");
    fs::DirBuilder::new().mode(0o755).create(&dir).unwrap();
    create(&dir);
    for file in ["world.toml", "world.key"] {
        assert!(!others_can_read(&dir.join(file)), "{file}");
    }
    fs::remove_dir_all(&dir).unwrap();
}
