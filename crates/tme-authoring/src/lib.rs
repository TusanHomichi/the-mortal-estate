//! The authoring compiler: typed authored geography in, proven runtime
//! content out.
//!
//! Six properties define this crate, and each one lives in exactly one place:
//!
//! 1. **Typed input** — [`tiled`] turns an authored Tiled document into typed
//!    values or a located error, and nothing downstream touches raw JSON.
//! 2. **Fail-closed validation** — [`compile`] holds every semantic assertion,
//!    in one implementation, with no advisory tier and no partial acceptance.
//! 3. **Deterministic projection** — [`project`] builds the runtime's own
//!    world-template value from ordered maps and serializes it one way.
//! 4. **Exact identity** — [`contract`] declares each land, its members, and
//!    their accepted envelopes, vocabularies, layer sets, properties, and
//!    programs as data; a drift is a rejection, not a merge.
//! 5. **Promotion separation** — [`promotion`] is the only path to a compiled
//!    [`Land`], and [`candidate`] runs the same semantics with no authority at
//!    all.
//! 6. **Reproducible reports** — [`emit`] is the single serializer, so the
//!    report and the projection are byte-identical across runs.
//!
//! [`export`] adds a seventh emission rather than a seventh property: the same
//! compiled land, written as one read-only document the Workbench's logical
//! view renders. It is derived truth under property 3, not a new authority.
//!
//! [`operations`] and [`replay`] add the Workbench's staged edit: a typed verb
//! vocabulary over the same object model, replayed against a COPY of the
//! accepted master to produce a candidate. They add no authority either — a
//! candidate is judged by [`candidate`], which has none — and they live here
//! rather than in the tool because a verb is a statement about the authored
//! object model, and that model is declared exactly once.
//!
//! It compiles the lands [`contract::LANDS`] declares: the synthetic authoring
//! fixture, which carries no content authority
//! (`content/authoring-fixture/README.md`), and the identity proof's land,
//! which is the one a runtime loads
//! (`content/lands/identity-proof/README.md`).

mod candidate;
pub mod cli;
mod compile;
pub mod contract;
mod emit;
mod export;
mod graph;
#[cfg(test)]
mod mutants;
mod operations;
mod project;
mod promotion;
mod replay;
mod tiled;

pub use candidate::{CandidateReport, validate_candidate};
pub use compile::{Landmark, Member, MemberReport, Structure, Transition, compile_member};
pub use contract::{LandContract, MemberContract, land};
pub use export::CANDIDATE_DOCUMENT_KIND;
pub use graph::{Connectivity, Edge};
pub use operations::{OperationSet, StagedOperation, VOCABULARY, vocabulary_document};
pub use project::{BuildMode, build, build_land, project};
pub use promotion::{Land, load};
pub use replay::replay;
pub use tiled::Point;

pub(crate) type Result<T> = std::result::Result<T, String>;

/// Locate the repository root from the crate's own manifest directory.
///
/// The markers are structural rather than name-based: the workspace manifest,
/// the content root, and the checks entry point must all be present. Nothing
/// here reads an environment variable, so a compiler run cannot be pointed at
/// a different tree by accident.
pub fn repository_root() -> Result<std::path::PathBuf> {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|path| {
            path.join("Cargo.toml").is_file()
                && path.join("content").is_dir()
                && path.join("tools/run_checks.py").is_file()
        })
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| "could not locate the repository root".into())
}
