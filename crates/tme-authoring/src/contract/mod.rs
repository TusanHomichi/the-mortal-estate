//! The accepted contracts for the authored lands, as data.
//!
//! Everything the compiler asserts EXACTLY — which lands exist, which members
//! each carries, their envelopes, tile vocabularies, layer sets, map
//! properties, authored programs, promotion receipts, and the presentation
//! fields the projection stamps — is declared here and nowhere else. The
//! validators in [`crate::compile`] read these tables; they do not restate
//! them. That is the one-source-of-truth rule applied to the compiler's own
//! expectations, and it is why "what changed?" is answerable by reading a
//! single declaration.
//!
//! **A land is data, and so is its member count.** The compiler compiles the
//! lands in [`LANDS`], each with the members its own contract declares — one,
//! two, or more. Nothing in the crate knows the number, and nothing outside
//! this module names a member by a type.

use serde_json::{Value, json};
use tme_rules::{LawZoneDef, PresentationModeDef, SceneRoleDef, StagedViewportDef, WorldZoomDef};

use crate::Result;

pub mod fixture;
pub mod identity_proof;

/// Which layer a tile class is legal in. A class is legal in exactly one, so
/// a stray footprint id in the terrain layer is a rejection rather than a
/// silently reinterpreted cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileRole {
    /// Ground truth for a cell. Carries a terrain identity and a passability
    /// verdict.
    Base { blocked: bool },
    /// An authored route overlay. A route crossing blocked ground replaces it.
    Route,
    /// A structure footprint cell. Always blocks.
    Footprint,
    /// A landmark or transition marker overlay. Never blocks on its own.
    Mark,
    /// The authored passability annotation, checked against the computed
    /// verdict. Carries no terrain identity.
    Passability { walkable: bool },
}

#[derive(Debug, Clone, Copy)]
pub struct TileClass {
    pub name: &'static str,
    pub role: TileRole,
}

pub(crate) const fn base(name: &'static str, blocked: bool) -> TileClass {
    TileClass {
        name,
        role: TileRole::Base { blocked },
    }
}

pub(crate) const fn class(name: &'static str, role: TileRole) -> TileClass {
    TileClass { name, role }
}

/// A map-level property value. The authored documents use exactly two shapes,
/// and a declaration that cannot express a third is a declaration that cannot
/// grow one by accident.
#[derive(Debug, Clone, Copy)]
pub enum PropertyValue {
    Text(&'static str),
    Flag(bool),
}

impl PropertyValue {
    fn to_json(self) -> Value {
        match self {
            Self::Text(value) => json!(value),
            Self::Flag(value) => json!(value),
        }
    }
}

/// One authored structure and the scope it claims.
#[derive(Debug, Clone, Copy)]
pub struct StructureContract {
    pub id: &'static str,
    pub scope: &'static str,
}

/// One authored landmark, its role, and the marker class it must be painted
/// with. An empty marker class means the landmark carries no mark tile.
#[derive(Debug, Clone, Copy)]
pub struct LandmarkContract {
    pub id: &'static str,
    pub role: &'static str,
    pub marker_class: &'static str,
}

/// One authored transition, owned by the member that declares it.
#[derive(Debug, Clone, Copy)]
pub struct TransitionContract {
    pub id: &'static str,
    pub target_member: &'static str,
    pub paired_transition: &'static str,
    pub direction: &'static str,
    pub marker_class: &'static str,
}

/// Presentation fields the projection stamps on a level. They belong to the
/// runtime world-template contract rather than to the authored document, so
/// they are declared here in the runtime's own types and never read from an
/// authored map.
#[derive(Debug, Clone, Copy)]
pub struct LevelPresentation {
    pub scene_role: SceneRoleDef,
    pub presentation_mode: PresentationModeDef,
    pub law_zone: LawZoneDef,
    pub world_zoom: WorldZoomDef,
    pub staged_viewport: Option<StagedViewportDef>,
}

/// What a promotion receipt's attestation covers, and — just as load-bearing —
/// what it does not. The compiler asserts the receipt's authority block equals
/// this EXACTLY, so a receipt can neither quietly grow into a licence for art,
/// tuning, or canon, nor quietly drop the authority the land depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiptAuthority {
    pub coordinates: bool,
    pub terrain_and_passability: bool,
    pub structures_and_landmarks: bool,
    pub member_transition_endpoints: bool,
    pub runtime_loads_authoring_source: bool,
    pub presentation_art: bool,
    pub gameplay_tuning: bool,
    pub content_canon: bool,
}

/// One authored member of a land.
#[derive(Debug)]
pub struct MemberContract {
    pub id: &'static str,
    /// The authored document, addressed as this repository names it.
    pub document: &'static str,
    pub width: usize,
    pub height: usize,
    pub classes: &'static [TileClass],
    pub tile_layers: &'static [&'static str],
    pub object_layers: &'static [&'static str],
    /// The map-level properties the member must declare, apart from
    /// `member_role`, which is derived from [`MemberContract::id`] so the two
    /// cannot disagree.
    pub map_properties: &'static [(&'static str, PropertyValue)],
    pub structures: &'static [StructureContract],
    pub landmarks: &'static [LandmarkContract],
    pub transitions: &'static [TransitionContract],
    /// The terrain class that makes ground "clustered" for this member. A
    /// member that authors structures must declare one; the scope check has
    /// nothing to compare against otherwise.
    pub clustered_ground_class: Option<&'static str>,
    /// Terrain identities the projection declares as this member's solid mass.
    pub wall_terrain_ids: &'static [&'static str],
    pub presentation: LevelPresentation,
    /// Whether the Workbench may stage truth operations against this member.
    /// A member with no candidate entry point says so rather than failing
    /// later with a confusing diagnostic.
    pub candidate_entry: bool,
}

impl MemberContract {
    pub fn envelope(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// The exact map-level properties an authored member must declare.
    pub fn declared_properties(&self) -> Vec<(&'static str, Value)> {
        let mut declared: Vec<(&'static str, Value)> = self
            .map_properties
            .iter()
            .map(|(name, value)| (*name, value.to_json()))
            .collect();
        declared.push(("member_role", json!(self.id)));
        declared
    }

    pub fn class(&self, name: &str) -> Option<&'static TileClass> {
        self.classes.iter().find(|entry| entry.name == name)
    }

    /// The one class in this member's vocabulary whose role matches, for the
    /// derived layers the replay refreshes. A vocabulary carrying two would be
    /// ambiguous, and ambiguity in a derived layer is a silent wrong answer.
    pub fn sole_class(
        &self,
        admits: impl Fn(TileRole) -> bool,
        what: &str,
    ) -> Result<&'static str> {
        let mut matches = self
            .classes
            .iter()
            .filter(|entry| admits(entry.role))
            .map(|entry| entry.name);
        let first = matches
            .next()
            .ok_or_else(|| format!("member {} declares no {what} class", self.id))?;
        if matches.next().is_some() {
            return Err(format!(
                "member {} declares more than one {what} class",
                self.id
            ));
        }
        Ok(first)
    }

    pub fn landmark(&self, id: &str) -> Option<&'static LandmarkContract> {
        self.landmarks.iter().find(|entry| entry.id == id)
    }

    pub fn transition(&self, id: &str) -> Option<&'static TransitionContract> {
        self.transitions.iter().find(|entry| entry.id == id)
    }
}

/// One authored land: its identity, its members, and the promotion receipt
/// that is the only door from its authored bytes to a compiled land.
#[derive(Debug)]
pub struct LandContract {
    pub id: &'static str,
    pub realm_id: &'static str,
    pub realm_name: &'static str,
    /// The arrival key the runtime world template publishes, bound to the one
    /// member that declares an arrival landmark.
    pub arrival_id: &'static str,
    /// The members, master first. The count is this contract's to declare.
    pub members: &'static [MemberContract],
    pub receipt_path: &'static str,
    pub receipt_kind: &'static str,
    pub receipt_status: &'static str,
    pub receipt_attested_by: &'static str,
    pub receipt_attested_on: &'static str,
    /// The reviewed digest of the accepted master, in reviewed source. The
    /// second of the promotion gate's two anchors.
    pub master_digest: &'static str,
    pub authority: ReceiptAuthority,
    pub world_template_output: &'static str,
    pub report_output: &'static str,
    pub workbench_projection_output: &'static str,
    /// The runtime terrain registry this land's vocabulary must resolve in,
    /// and the profile that selects it.
    pub terrain_registry_catalog: &'static str,
    pub terrain_registry_profile: &'static str,
}

impl LandContract {
    /// The attested master: the first declared member.
    pub fn master(&'static self) -> &'static MemberContract {
        &self.members[0]
    }

    pub fn companions(&'static self) -> &'static [MemberContract] {
        &self.members[1..]
    }

    pub fn member(&'static self, id: &str) -> Result<&'static MemberContract> {
        self.members
            .iter()
            .find(|member| member.id == id)
            .ok_or_else(|| format!("land {} carries no member {id:?}", self.id))
    }

    /// The one member the Workbench may stage truth operations against.
    pub fn candidate_member(&'static self) -> Result<&'static MemberContract> {
        let entries: Vec<&MemberContract> = self
            .members
            .iter()
            .filter(|member| member.candidate_entry)
            .collect();
        match entries.as_slice() {
            [member] => Ok(member),
            [] => Err(format!(
                "land {} declares no candidate entry point",
                self.id
            )),
            _ => Err(format!(
                "land {} declares more than one candidate entry point",
                self.id
            )),
        }
    }
}

/// Every land this compiler compiles.
pub static LANDS: &[&LandContract] = &[&fixture::LAND, &identity_proof::LAND];

pub fn land(id: &str) -> Result<&'static LandContract> {
    LANDS
        .iter()
        .copied()
        .find(|land| land.id == id)
        .ok_or_else(|| {
            format!(
                "no authored land {id:?}; this compiler carries {}",
                LANDS
                    .iter()
                    .map(|land| land.id)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

/// The one layer a tile role is authored in. This is the whole class-to-layer
/// relation: [`crate::compile`] reads it to reject a class in the wrong layer,
/// and [`crate::export`] reads it to tell the Workbench which authored layer a
/// compiled cell's terrain came from. Two statements of this relation would be
/// two chances to disagree.
pub fn layer_of(role: TileRole) -> &'static str {
    match role {
        TileRole::Base { .. } => "base_terrain",
        TileRole::Route => "routes",
        TileRole::Footprint => "structure_footprints",
        TileRole::Mark => "landmark_marks",
        TileRole::Passability { .. } => "passability",
    }
}

pub const STRUCTURE_CLASS: &str = "functional_building";
pub const TRANSITION_CLASS: &str = "member_transition";
pub const LANDMARK_CLASS: &str = "surface_landmark";
pub const ARRIVAL_ROLE: &str = "arrival";
