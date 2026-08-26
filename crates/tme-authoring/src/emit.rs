use std::fs;
use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::Result;

pub fn read(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))
}

/// The one serializer every projected artifact goes through. Pretty-printed
/// with a trailing newline so a projection is reviewable as a text diff, and
/// deterministic because every map it receives is ordered.
pub fn json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Write a projected artifact, or in check mode assert the tracked bytes are
/// exactly what this run would have written. A hand edit and a stale
/// projection fail identically, which is the point.
pub fn write_or_check(path: &Path, expected: &[u8], check: bool) -> Result<()> {
    if check {
        let actual = read(path)?;
        if actual != expected {
            return Err(format!(
                "{} is stale or hand-edited: rerun without --check",
                path.display()
            ));
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    fs::write(path, expected).map_err(|error| format!("write {}: {error}", path.display()))
}
