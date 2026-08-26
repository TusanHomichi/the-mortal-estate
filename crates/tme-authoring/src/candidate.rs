//! Candidate validation: the compiler's semantics without its authority.
//!
//! A taste round needs to ask "would this pass?" long before anyone is willing
//! to attest to it. That question is answered by the SAME compile logic the
//! promoted path uses — a candidate judged by a gentler copy would learn the
//! wrong rule and arrive at the gate surprised.
//!
//! What the candidate path skips is promotion, and only promotion: it reads no
//! receipt, consults no reviewed digest, asserts no attestation, and writes
//! nothing at all. Its only output is its return value.

use serde_json::Value;

use crate::Result;
use crate::compile::{self, MemberReport};
use crate::contract::MemberContract;
use crate::emit;

/// The result of running a member's semantics over an unattested document.
///
/// This type carries statistics and diagnostics and nothing else. It exposes
/// no conversion into [`crate::Member`], no constructor for one, and no
/// accessor that yields one — so a candidate cannot reach the projection by
/// any route the type system permits. That is deliberate: a convention would
/// hold only until someone was in a hurry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CandidateReport {
    /// The candidate document's own digest, over its canonical serialization.
    /// It is never compared against the reviewed digest; it exists so a
    /// verdict can be pinned to the bytes that produced it.
    pub candidate_sha256: String,
    /// Which land and member the candidate was judged as.
    pub land: &'static str,
    pub member: &'static str,
    /// Whether every blocking assertion held.
    pub accepted: bool,
    /// Derived member statistics, present only for an accepted candidate.
    pub statistics: Option<MemberReport>,
    /// The blocking diagnostics, in the promoted path's own wording.
    pub diagnostics: Vec<String>,
}

/// Run an accepted member's semantic validation against a candidate document.
///
/// Grants no promotion authority, reads no promotion receipt, and writes no
/// tracked projection. The returned report cannot become a
/// [`crate::Member`]:
///
/// ```compile_fail
/// fn takes_member(_: tme_authoring::Member) {}
/// fn from_report(report: tme_authoring::CandidateReport) {
///     takes_member(report.into());
/// }
/// ```
///
/// The `Err` arm is reserved for a document that cannot be canonicalized at
/// all. A document that merely fails the rules is a perfectly good answer, and
/// comes back as an unaccepted report carrying the reason.
pub fn validate_candidate(
    land: &'static str,
    member: &'static MemberContract,
    document: &Value,
) -> Result<CandidateReport> {
    let candidate_sha256 = emit::digest(&emit::json(document)?);
    Ok(match compile::compile_member(member, document) {
        Ok(compiled) => CandidateReport {
            candidate_sha256,
            land,
            member: member.id,
            accepted: true,
            statistics: Some(compiled.report),
            diagnostics: Vec::new(),
        },
        Err(diagnostic) => CandidateReport {
            candidate_sha256,
            land,
            member: member.id,
            accepted: false,
            statistics: None,
            diagnostics: vec![diagnostic],
        },
    })
}
