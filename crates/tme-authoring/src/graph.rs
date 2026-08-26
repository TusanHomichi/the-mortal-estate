//! The land's one connectivity graph.
//!
//! Members are validated for their own internal reachability by
//! [`crate::compile`]; reachability BETWEEN members is validated here, over
//! every member at once. A stair's reciprocity cannot be checked across a
//! format border, so it is never checked across a file border either.
//!
//! A land of one member has no cross-member edges and therefore an empty
//! graph. That is a fact about the land, not a missing check: every transition
//! the land declares is still resolved, and there are none.

use std::collections::BTreeMap;

use crate::Result;
use crate::compile::Member;
use crate::tiled::Point;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Edge {
    pub id: String,
    pub from_member: String,
    pub from: Point,
    pub to_member: String,
    pub to: Point,
    pub direction: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Connectivity {
    pub edges: Vec<Edge>,
}

fn opposite(direction: &str) -> Option<&'static str> {
    match direction {
        "down" => Some("up"),
        "up" => Some("down"),
        _ => None,
    }
}

/// Resolve every declared transition against the member it names.
///
/// Blocks on: a transition naming a member that does not exist, a pair whose
/// other half is missing, a pair that does not name each other, a pair whose
/// directions are not complements, and an endpoint that lands on a blocked
/// cell. Each of those is a dangling endpoint in some form, and a dangling
/// endpoint is the defect this contract exists to make impossible.
pub fn link(members: &BTreeMap<String, Member>) -> Result<Connectivity> {
    let mut edges = Vec::new();
    for (member_id, member) in members {
        for transition in member.transitions().values() {
            let target = members
                .get(transition.target_member.as_str())
                .ok_or_else(|| {
                    format!(
                        "transition {} names member {:?}, which the land does not carry",
                        transition.id, transition.target_member
                    )
                })?;
            let paired = target
                .transitions()
                .get(&transition.paired_transition)
                .ok_or_else(|| {
                    format!(
                    "transition {} names paired transition {:?}, which member {:?} does not carry",
                    transition.id, transition.paired_transition, transition.target_member
                )
                })?;
            if paired.paired_transition != transition.id || paired.target_member != *member_id {
                return Err(format!(
                    "transitions {} and {} are not exact reciprocals",
                    transition.id, paired.id
                ));
            }
            if opposite(&transition.direction) != Some(paired.direction.as_str()) {
                return Err(format!(
                    "transitions {} and {} declare directions that are not complements",
                    transition.id, paired.id
                ));
            }
            if !member.is_passable(transition.access) || !target.is_passable(paired.access) {
                return Err(format!(
                    "transition {} connects through a blocked endpoint",
                    transition.id
                ));
            }
            edges.push(Edge {
                id: format!("route/{}", transition.id),
                from_member: member_id.clone(),
                from: transition.access,
                to_member: transition.target_member.clone(),
                to: paired.access,
                direction: transition.direction.clone(),
            });
        }
    }
    edges.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(Connectivity { edges })
}
