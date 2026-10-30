//! `content/` compiled into the catalogue the simulation runs on, and the
//! scenery a client draws: every file read, every unit turned into the
//! simulation's own, every reference checked. One entry per file; a file's
//! name is its entry's id.
//!
//! Anything wrong comes back as a list of problems, each naming its file
//! and line, what's wrong and what would fix it.

mod compile;
mod files;

pub use compile::{Look, Prop, Scenery};
pub use files::Sides;

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use tme_sim::Catalogue;
use tme_sim::catalogue::{BodyKind, Id, Place};

/// Everything compiled from a content folder.
#[derive(Clone, Debug)]
pub struct Content {
    pub catalogue: Catalogue,
    /// How each kind of body is drawn.
    pub looks: BTreeMap<Id<BodyKind>, Look>,
    /// What stands in each place.
    pub scenery: BTreeMap<Id<Place>, Scenery>,
}

impl Content {
    /// Every model the content names, under `assets/models/`.
    pub fn models(&self) -> impl Iterator<Item = &str> {
        self.looks.values().map(|l| l.model.as_str()).chain(
            self.scenery
                .values()
                .flat_map(|s| s.props.iter().map(|p| p.model.as_str())),
        )
    }
}

/// One thing wrong with the content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// The file, relative to the content folder.
    pub file: PathBuf,
    /// The line in it, where there's one to point at.
    pub line: Option<usize>,
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.file.display())?;
        if let Some(line) = self.line {
            write!(f, ":{line}")?;
        }
        write!(f, ": {}", self.message)
    }
}

/// Everything wrong with the content, in file order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problems(pub Vec<Problem>);

impl fmt::Display for Problems {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, p) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{p}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Problems {}

/// Read and compile the content folder at `dir`.
pub fn load(dir: &Path) -> Result<Content, Problems> {
    compile::compile(dir)
}

/// Models the content names that aren't under `models` (the
/// `assets/models/` folder).
pub fn missing_models(content: &Content, models: &Path) -> Vec<String> {
    let mut missing: Vec<String> = content
        .models()
        .filter(|m| !models.join(m).is_file())
        .map(str::to_owned)
        .collect();
    missing.sort();
    missing.dedup();
    missing
}
