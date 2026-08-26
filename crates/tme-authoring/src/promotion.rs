//! The promotion gate: the only path from authored bytes to a compiled land.
//!
//! The gate is deliberately DOUBLE-ANCHORED. A receipt on disk records what
//! was attested, and the land contract's `master_digest` — a constant in
//! reviewed source — records the same fact where no file-writing process can
//! reach it. Either anchor alone is weak: a receipt can be rewritten by
//! anything that can write files, and a constant alone cannot carry per-file
//! digests or a bounded authority statement. Requiring both is the whole
//! point.
//!
//! **Attestation, honestly.** Each land declares the attestation it actually
//! has. The authoring fixture carries the owner's acceptance from gate G4X; the
//! identity proof's land carries the owner's acceptance of slice S1's
//! geography. Neither started there: both were lane-attested and pending until
//! the owner looked, because fabricating an approval would make the strongest
//! check in this crate a lie. An acceptance moves the status, the attestor and
//! the reviewed digest together, which is exactly the ceremony the arrangement
//! is built to require.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::Result;
use crate::compile::{self, Member, MemberReport};
use crate::contract::{LandContract, ReceiptAuthority};
use crate::emit;
use crate::graph::{self, Connectivity};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema_version: u32,
    kind: String,
    status: String,
    attested_by: String,
    attested_on: String,
    master: AttestedFile,
    companions: Vec<AttestedFile>,
    authority: Authority,
    research_boundary: ResearchBoundary,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AttestedFile {
    path: String,
    sha256: String,
    #[serde(default)]
    byte_identical_to_reviewed_master: bool,
}

/// What the attestation covers, and — just as load-bearing — what it does not.
/// The block must equal the land contract's declaration exactly, so a receipt
/// can never quietly grow into a licence for art, tuning, or canon, nor
/// quietly drop the authority the land depends on.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Authority {
    coordinates: bool,
    terrain_and_passability: bool,
    structures_and_landmarks: bool,
    member_transition_endpoints: bool,
    runtime_loads_authoring_source: bool,
    presentation_art: bool,
    gameplay_tuning: bool,
    content_canon: bool,
}

impl From<Authority> for ReceiptAuthority {
    fn from(value: Authority) -> Self {
        Self {
            coordinates: value.coordinates,
            terrain_and_passability: value.terrain_and_passability,
            structures_and_landmarks: value.structures_and_landmarks,
            member_transition_endpoints: value.member_transition_endpoints,
            runtime_loads_authoring_source: value.runtime_loads_authoring_source,
            presentation_art: value.presentation_art,
            gameplay_tuning: value.gameplay_tuning,
            content_canon: value.content_canon,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResearchBoundary {
    review_refs: Vec<String>,
}

/// The compiled land. Only [`load`] produces one, and only a `Land` reaches
/// the projection.
#[derive(Debug, Clone)]
pub struct Land {
    pub(crate) contract: &'static LandContract,
    pub(crate) members: BTreeMap<String, Member>,
    pub(crate) graph: Connectivity,
    pub(crate) digests: BTreeMap<String, String>,
}

impl Land {
    pub fn contract(&self) -> &'static LandContract {
        self.contract
    }

    pub fn id(&self) -> &'static str {
        self.contract.id
    }

    pub fn master_digest(&self) -> &str {
        &self.digests[self.contract.master().document]
    }

    pub fn digests(&self) -> &BTreeMap<String, String> {
        &self.digests
    }

    /// The members, in the order the contract declares them.
    pub fn members(&self) -> impl Iterator<Item = &Member> {
        self.contract
            .members
            .iter()
            .map(|member| &self.members[member.id])
    }

    pub fn member(&self, id: &str) -> Result<&Member> {
        self.members
            .get(id)
            .ok_or_else(|| format!("land {} carries no member {id:?}", self.contract.id))
    }

    /// The member that declares the land's arrival landmark. Exactly one does;
    /// a land with none has nowhere to put a player and a land with two has no
    /// answer to where the arrival is.
    pub fn arrival_member(&self) -> Result<&Member> {
        let mut found = self.members().filter(|member| member.arrival().is_some());
        let first = found.next().ok_or_else(|| {
            format!(
                "land {} declares no arrival landmark in any member",
                self.contract.id
            )
        })?;
        if found.next().is_some() {
            return Err(format!(
                "land {} declares an arrival landmark in more than one member",
                self.contract.id
            ));
        }
        Ok(first)
    }

    pub fn reports(&self) -> Vec<&MemberReport> {
        self.members().map(Member::report).collect()
    }

    pub fn connectivity(&self) -> &Connectivity {
        &self.graph
    }
}

pub fn load(root: &Path, contract: &'static LandContract) -> Result<Land> {
    let (documents, digests) = validate_promotion(root, contract)?;
    let mut members = BTreeMap::new();
    for member in contract.members {
        let compiled = compile::compile_member(member, &documents[member.document])?;
        members.insert(member.id.to_owned(), compiled);
    }
    let graph = graph::link(&members)?;
    Ok(Land {
        contract,
        members,
        graph,
        digests,
    })
}

fn validate_promotion(
    root: &Path,
    contract: &'static LandContract,
) -> Result<(BTreeMap<String, Value>, BTreeMap<String, String>)> {
    let receipt_path = root.join(contract.receipt_path);
    let receipt: Receipt = serde_json::from_slice(&emit::read(&receipt_path)?)
        .map_err(|error| format!("{}: {error}", receipt_path.display()))?;

    if receipt.schema_version != 1
        || receipt.kind != contract.receipt_kind
        || receipt.status != contract.receipt_status
        || receipt.attested_by != contract.receipt_attested_by
        || receipt.attested_on != contract.receipt_attested_on
        || !receipt.master.byte_identical_to_reviewed_master
        || receipt.master.path != contract.master().document
        || receipt.master.sha256 != contract.master_digest
        || receipt.research_boundary.review_refs.is_empty()
    {
        return Err(format!(
            "the {} promotion receipt differs from the attested contract",
            contract.id
        ));
    }

    if ReceiptAuthority::from(receipt.authority) != contract.authority {
        return Err(format!(
            "the {} promotion authority is incomplete or over-broad",
            contract.id
        ));
    }

    let mut files = vec![receipt.master];
    files.extend(receipt.companions);
    let declared = contract
        .members
        .iter()
        .map(|member| member.document)
        .collect::<Vec<_>>();
    if files.len() != declared.len()
        || files
            .iter()
            .zip(&declared)
            .any(|(file, path)| file.path != *path)
    {
        return Err(format!(
            "the {} promotion receipt must name exactly the members the contract declares, \
             master first: {}",
            contract.id,
            declared.join(", ")
        ));
    }

    let mut documents = BTreeMap::new();
    let mut digests = BTreeMap::new();
    for file in files {
        let path = root.join(&file.path);
        let bytes = emit::read(&path)?;
        let actual = emit::digest(&bytes);
        if actual != file.sha256 {
            return Err(format!(
                "{} digest mismatch: the receipt claims {}, the file is {actual}",
                path.display(),
                file.sha256
            ));
        }
        let document = serde_json::from_slice(&bytes)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        documents.insert(file.path.clone(), document);
        digests.insert(file.path, actual);
    }
    Ok((documents, digests))
}
