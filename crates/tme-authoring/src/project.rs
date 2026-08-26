//! Deterministic projection of a compiled land into tracked runtime content.
//!
//! The projection is built as the runtime's OWN `WorldTemplateV3` value rather
//! than as free-form JSON, so the compiler cannot drift from the contract it
//! feeds: a change to the runtime shape is a compile error here, not a
//! discovery at load time. It is then validated against the runtime's own
//! content validator before it is written, which is what makes
//! "runtime-consumable" a checked claim instead of a hopeful adjective.
//!
//! The authored documents themselves stay out of the runtime's reach (P8): the
//! runtime loads this compiled output and never the authored member.

use std::collections::BTreeMap;
use std::path::Path;

use tme_rules::model::{Coord, VerticalDirection, WorldPosition};
use tme_rules::{
    CatalogProfileKey, CatalogV6, LevelDef, RealmDef, TopologyEdgeDef, TopologyKindDef,
    TopologyTargetDef, WORLD_TEMPLATE_KIND, WORLD_TEMPLATE_SCHEMA_VERSION, WorldTemplateV3,
};

use crate::Result;
use crate::compile::{Grid, Member, MemberReport};
use crate::contract::{self, LandContract};
use crate::emit;
use crate::export;
use crate::graph::Connectivity;
use crate::promotion::Land;
use crate::tiled::Point;

const REPORT_KIND: &str = "authored_land_compile_report";

/// The runtime contract requires a 64-hex `visual_manifest_digest`, and this
/// project has no visual manifest: the predecessor's was retired and no
/// successor presentation boundary has been decided. The projection therefore
/// pins the field to the authored master's digest, so it carries a real,
/// verifiable identity rather than a placeholder. Revisit the field's NAME
/// when the presentation boundary lands; do not revisit its presence.
fn visual_manifest_digest(land: &Land) -> String {
    land.master_digest().to_owned()
}

#[derive(Debug, serde::Serialize)]
struct LandReport<'a> {
    schema_version: u32,
    kind: &'static str,
    land_id: &'static str,
    member_digests: &'a BTreeMap<String, String>,
    members: Vec<&'a MemberReport>,
    connectivity: &'a Connectivity,
    world_template_sha256: String,
    terrain_registry_catalog: &'static str,
    terrain_registry_profile: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct BuildMode {
    pub check: bool,
    pub report: bool,
}

/// Compile every authored land, project each one, prove the projections, and
/// either write them or assert the tracked bytes already match.
pub fn build(root: &Path, mode: BuildMode) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    for contract in contract::LANDS {
        lines.extend(build_land(root, contract, mode)?);
    }
    Ok(lines)
}

pub fn build_land(
    root: &Path,
    contract: &'static LandContract,
    mode: BuildMode,
) -> Result<Vec<String>> {
    let land = crate::promotion::load(root, contract)?;
    let template = project(&land)?;
    validate_against_registry(root, contract, &template)?;

    let template_bytes = emit::json(&template)?;
    let report = LandReport {
        schema_version: 1,
        kind: REPORT_KIND,
        land_id: contract.id,
        member_digests: land.digests(),
        members: land.reports(),
        connectivity: land.connectivity(),
        world_template_sha256: emit::digest(&template_bytes),
        terrain_registry_catalog: contract.terrain_registry_catalog,
        terrain_registry_profile: contract.terrain_registry_profile,
    };
    let report_bytes = emit::json(&report)?;
    let workbench_bytes = emit::json(&export::document(
        root,
        &land,
        &report.world_template_sha256,
    )?)?;

    emit::write_or_check(
        &root.join(contract.world_template_output),
        &template_bytes,
        mode.check,
    )?;
    emit::write_or_check(
        &root.join(contract.report_output),
        &report_bytes,
        mode.check,
    )?;
    emit::write_or_check(
        &root.join(contract.workbench_projection_output),
        &workbench_bytes,
        mode.check,
    )?;

    let members = land
        .reports()
        .iter()
        .map(|report| {
            format!(
                "{}={}x{} passable={}",
                report.member, report.width, report.height, report.passable_cells
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut lines = vec![format!(
        "authored land {}: PASS {members} edges={} master={}",
        contract.id,
        land.connectivity().edges.len(),
        land.master_digest(),
    )];
    if mode.report {
        lines.push(
            String::from_utf8(report_bytes)
                .map_err(|error| error.to_string())?
                .trim_end()
                .into(),
        );
    }
    Ok(lines)
}

pub fn project(land: &Land) -> Result<WorldTemplateV3> {
    let contract = land.contract();
    let realm = RealmDef {
        name: contract.realm_name.into(),
        levels: land
            .members()
            .map(|member| (member.id().to_owned(), level(member)))
            .collect(),
    };
    let arrival_member = land.arrival_member()?;
    let arrival = arrival_member
        .arrival()
        .ok_or("the arrival member carries no arrival")?;
    Ok(WorldTemplateV3 {
        schema_version: WORLD_TEMPLATE_SCHEMA_VERSION,
        kind: WORLD_TEMPLATE_KIND.into(),
        id: contract.id.into(),
        visual_manifest_digest: visual_manifest_digest(land),
        realms: BTreeMap::from([(contract.realm_id.to_owned(), realm)]),
        arrivals: BTreeMap::from([(
            contract.arrival_id.to_owned(),
            position(contract, arrival_member.id(), arrival),
        )]),
        topology: topology(contract, land.connectivity())?,
    })
}

fn level(member: &Member) -> LevelDef {
    let contract = member.contract;
    let presentation = contract.presentation;
    LevelDef {
        law_zone: presentation.law_zone,
        scene_role: presentation.scene_role,
        presentation_mode: presentation.presentation_mode,
        world_zoom: presentation.world_zoom,
        maximum_clear_sightline: longest_clear_run(&member.grid).clamp(1, 12),
        staged_viewport: presentation.staged_viewport,
        wall_terrain_ids: contract
            .wall_terrain_ids
            .iter()
            .map(|id| (*id).to_owned())
            .collect(),
        static_props: Vec::new(),
        width: member.grid.width as i32,
        height: member.grid.height as i32,
        cells: member.grid.cells.clone(),
    }
}

fn longest_clear_run(grid: &Grid) -> u32 {
    let mut longest = 0;
    for y in 0..grid.height {
        let mut run = 0;
        for x in 0..grid.width {
            run = if grid.passable.contains(&Point { x, y }) {
                run + 1
            } else {
                0
            };
            longest = longest.max(run);
        }
    }
    for x in 0..grid.width {
        let mut run = 0;
        for y in 0..grid.height {
            run = if grid.passable.contains(&Point { x, y }) {
                run + 1
            } else {
                0
            };
            longest = longest.max(run);
        }
    }
    longest
}

fn position(contract: &'static LandContract, member: &str, point: Point) -> WorldPosition {
    WorldPosition::new(
        contract.realm_id,
        member,
        Coord {
            x: point.x as i32,
            y: point.y as i32,
        },
    )
}

fn topology(
    contract: &'static LandContract,
    graph: &Connectivity,
) -> Result<BTreeMap<String, TopologyEdgeDef>> {
    graph
        .edges
        .iter()
        .map(|edge| {
            let direction = match edge.direction.as_str() {
                "down" => VerticalDirection::Down,
                "up" => VerticalDirection::Up,
                other => {
                    return Err(format!(
                        "connectivity edge {} declares unknown direction {other:?}",
                        edge.id
                    ));
                }
            };
            Ok((
                edge.id.clone(),
                TopologyEdgeDef {
                    at: WorldPosition::new(
                        contract.realm_id,
                        edge.from_member.clone(),
                        Coord {
                            x: edge.from.x as i32,
                            y: edge.from.y as i32,
                        },
                    ),
                    target: TopologyTargetDef::Position {
                        location: WorldPosition::new(
                            contract.realm_id,
                            edge.to_member.clone(),
                            Coord {
                                x: edge.to.x as i32,
                                y: edge.to.y as i32,
                            },
                        ),
                    },
                    kind: TopologyKindDef::Stairs { direction },
                    hidden: false,
                },
            ))
        })
        .collect()
}

/// Prove the projection against the runtime's own content validator, using the
/// terrain registry the land's vocabulary claims membership in.
///
/// This is what makes the terrain-class vocabulary a contract rather than a
/// naming convention: a class with no registry mapping fails here, at compile
/// time, instead of becoming a missing-terrain surprise in a running world.
fn validate_against_registry(
    root: &Path,
    contract: &'static LandContract,
    template: &WorldTemplateV3,
) -> Result<()> {
    let path = root.join(contract.terrain_registry_catalog);
    let catalog: CatalogV6 = serde_json::from_slice(&emit::read(&path)?)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let selected = catalog
        .select(&CatalogProfileKey::from(contract.terrain_registry_profile))
        .map_err(|error| format!("terrain registry selection failed: {error}"))?;
    template
        .validate_with(&selected)
        .map_err(|error| format!("projected world template is not runtime-valid: {error}"))
}
