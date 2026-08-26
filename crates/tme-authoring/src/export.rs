//! The Workbench's logical projection — the compiled land, in one read-only
//! document a non-Rust tool can render.
//!
//! **Why this exists.** The runtime projection ([`crate::project`]) carries
//! what the runtime needs: cells, topology, arrivals. It does not carry
//! structure identities, access cells, façade doors, landmark positions,
//! per-cell passability, or which authored layer a cell's terrain came from —
//! and a logical view that recomputed any of those would be a second authority
//! on geography, which is exactly what the Workbench may never become. So the
//! compiler emits them itself, once, deterministically.
//!
//! **What it is not.** It is not a second content ledger and not an authoring
//! input. Nothing reads it back into a [`crate::promotion::Land`], the runtime
//! never loads it, and it grants no authority of any kind. It is a derived view
//! of the same compiled land the runtime projection is derived from, written
//! through the same single serializer so it drifts from neither.
//!
//! **Its own source binding.** The document names the exact bytes it was
//! derived from — the attested master, every companion, the promotion receipt,
//! and the runtime projection — with their digests. A consumer that recomputes
//! those digests knows whether the view it is holding still describes the tree
//! on disk. That is the fail-closed staleness contract, served from the
//! compiler's side of the boundary.

use std::collections::BTreeMap;
use std::path::Path;

use crate::Result;
use crate::compile::{Grid, Landmark, Member, Structure, Transition};
use crate::contract::{self, MemberContract};
use crate::emit;
use crate::graph::Connectivity;
use crate::promotion::Land;
use crate::tiled::Point;

const DOCUMENT_KIND: &str = "workbench_logical_projection";
pub const CANDIDATE_DOCUMENT_KIND: &str = "workbench_candidate_projection";

/// One addressed source file and the bytes it held when this view was built.
#[derive(Debug, serde::Serialize)]
pub struct SourceFile {
    pub role: &'static str,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, serde::Serialize)]
struct TerrainView {
    class: String,
    layer: &'static str,
}

#[derive(Debug, serde::Serialize)]
struct CellView {
    x: usize,
    y: usize,
    passable: bool,
    terrain: Vec<TerrainView>,
}

#[derive(Debug, serde::Serialize)]
struct MemberView<'a> {
    member: &'static str,
    width: usize,
    height: usize,
    cells: Vec<CellView>,
    routes: Vec<Point>,
    structures: &'a [Structure],
    landmarks: Vec<&'a Landmark>,
    transitions: Vec<&'a Transition>,
}

/// A candidate's logical view: the same member, bound to the candidate's own
/// bytes and carrying no authority of any kind.
#[derive(Debug, serde::Serialize)]
pub struct CandidateProjection<'a> {
    schema_version: u32,
    kind: &'static str,
    authority: &'static str,
    land_id: &'static str,
    realm_id: &'static str,
    candidate_member: &'static str,
    tile_size_px: i64,
    sources: Vec<SourceFile>,
    members: Vec<MemberView<'a>>,
}

#[derive(Debug, serde::Serialize)]
pub struct WorkbenchProjection<'a> {
    schema_version: u32,
    kind: &'static str,
    land_id: &'static str,
    realm_id: &'static str,
    /// The one member the Workbench may stage truth operations against. The
    /// contract decides it; the view carries it so no consumer has to hold a
    /// second opinion about which member is editable.
    candidate_member: &'static str,
    /// The authored pixel lattice one cell occupies. Carried so the view can
    /// draw at authored scale without restating the compiler's constant.
    tile_size_px: i64,
    sources: Vec<SourceFile>,
    members: Vec<MemberView<'a>>,
    connectivity: &'a Connectivity,
}

/// Build the logical projection for a compiled land.
///
/// `world_template_sha256` is the digest of the runtime projection produced by
/// the same run, passed in rather than re-read so that the view can never name
/// a digest for bytes this run did not itself produce.
pub fn document<'a>(
    root: &Path,
    land: &'a Land,
    world_template_sha256: &str,
) -> Result<WorkbenchProjection<'a>> {
    let contract = land.contract();
    let receipt_digest = emit::digest(&emit::read(&root.join(contract.receipt_path))?);
    let mut sources = vec![source(
        "master",
        contract.master().document,
        &land.digests()[contract.master().document],
    )];
    for companion in contract.companions() {
        sources.push(source(
            "companion",
            companion.document,
            &land.digests()[companion.document],
        ));
    }
    sources.push(source("receipt", contract.receipt_path, &receipt_digest));
    sources.push(source(
        "runtime_projection",
        contract.world_template_output,
        world_template_sha256,
    ));
    Ok(WorkbenchProjection {
        schema_version: 1,
        kind: DOCUMENT_KIND,
        land_id: contract.id,
        realm_id: contract.realm_id,
        candidate_member: contract.candidate_member()?.id,
        tile_size_px: crate::tiled::TILE,
        sources,
        members: land.members().map(member_view).collect(),
        connectivity: land.connectivity(),
    })
}

/// One member as the logical view renders it.
///
/// One builder, two callers: the accepted land's projection and a candidate's.
/// A candidate that were described by a second, slightly different view would
/// be previewed as something other than what Apply would produce.
fn member_view(member: &Member) -> MemberView<'_> {
    MemberView {
        member: member.id(),
        width: member.width(),
        height: member.height(),
        cells: cells(member.contract, &member.grid),
        routes: member.route_cells().iter().copied().collect(),
        structures: member.structures(),
        landmarks: member.landmarks().values().collect(),
        transitions: member.transitions().values().collect(),
    }
}

/// The same view, for a candidate document that no one has attested.
///
/// It carries a different `kind` from the accepted projection, and the
/// Workbench's loader refuses a document whose kind is not the accepted one —
/// so a candidate preview cannot be served as the accepted view by accident.
/// It binds one source: the candidate itself. It names no receipt and no
/// reviewed digest, because a candidate has neither.
pub fn candidate_document<'a>(
    land: &'static contract::LandContract,
    member: &'a Member,
    candidate_path: &str,
    candidate_sha256: &str,
) -> CandidateProjection<'a> {
    CandidateProjection {
        schema_version: 1,
        kind: CANDIDATE_DOCUMENT_KIND,
        authority: "none",
        land_id: land.id,
        realm_id: land.realm_id,
        candidate_member: member.id(),
        tile_size_px: crate::tiled::TILE,
        sources: vec![source("candidate", candidate_path, candidate_sha256)],
        members: vec![member_view(member)],
    }
}

fn source(role: &'static str, path: &str, sha256: &str) -> SourceFile {
    SourceFile {
        role,
        path: path.to_owned(),
        sha256: sha256.to_owned(),
    }
}

/// Row-major cells, each carrying the compiler's own passability verdict and
/// its terrain stack attributed back to the authored layer each class belongs
/// to. The attribution is a lookup in [`contract::layer_of`], not a second
/// reading of the authored document.
fn cells(member: &'static MemberContract, grid: &Grid) -> Vec<CellView> {
    let layers = member
        .classes
        .iter()
        .map(|class| (class.name, contract::layer_of(class.role)))
        .collect::<BTreeMap<_, _>>();
    let mut rows = Vec::with_capacity(grid.width * grid.height);
    for y in 0..grid.height {
        for x in 0..grid.width {
            rows.push(CellView {
                x,
                y,
                passable: grid.passable.contains(&Point { x, y }),
                terrain: grid.cells[y][x]
                    .iter()
                    .flatten()
                    .map(|class| TerrainView {
                        class: class.clone(),
                        layer: layers[class.as_str()],
                    })
                    .collect(),
            });
        }
    }
    rows
}
