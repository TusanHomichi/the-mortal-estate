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

use std::collections::{BTreeMap, BTreeSet, VecDeque};

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
        "passage" => Some("passage"),
        "door" => Some("door"),
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
            if !member.is_passable(transition.access) || !target.is_passable(paired.landing) {
                return Err(format!(
                    "transition {} connects through a blocked endpoint",
                    transition.id
                ));
            }
            let door = member
                .doors()
                .iter()
                .find(|door| door.at == transition.access);
            if transition.direction == "door" {
                let other = target.doors().iter().find(|door| door.at == paired.access);
                if transition.landing != transition.access
                    || paired.landing != paired.access
                    || !matches!((door, other), (Some(a), Some(b)) if !a.hidden && !b.hidden && a.open == b.open)
                {
                    return Err(format!(
                        "transition {} requires matching visible door thresholds",
                        transition.id
                    ));
                }
            } else if door.is_some() {
                return Err(format!(
                    "transition {} conflicts with local door topology",
                    transition.id
                ));
            }
            edges.push(Edge {
                id: format!("route/{}", transition.id),
                from_member: member_id.clone(),
                from: transition.access,
                to_member: transition.target_member.clone(),
                to: paired.landing,
                direction: transition.direction.clone(),
            });
        }
    }
    let roots = members
        .iter()
        .filter(|(_, member)| member.arrival().is_some())
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let [root] = roots.as_slice() else {
        return Err("land connectivity requires exactly one arrival member".into());
    };
    let mut reached = BTreeSet::from([root.clone()]);
    let mut queue = VecDeque::from([root.clone()]);
    while let Some(member) = queue.pop_front() {
        for edge in edges.iter().filter(|edge| edge.from_member == member) {
            if reached.insert(edge.to_member.clone()) {
                queue.push_back(edge.to_member.clone());
            }
        }
    }
    if reached.len() != members.len() {
        return Err("land connectivity contains a member unreachable from arrival".into());
    }
    edges.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(Connectivity { edges })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn grand_door_graph_refuses_a_mismatched_or_concealed_face() {
        let root = crate::repository_root().unwrap();
        let contract = crate::land("first_expedition").unwrap();
        for class in ["expedition_doorway_open", "expedition_masonry"] {
            let members = contract
                .members
                .iter()
                .map(|member| {
                    let mut doc: Value =
                        serde_json::from_slice(&std::fs::read(root.join(member.document)).unwrap())
                            .unwrap();
                    if member.id == "d2" {
                        let gid = doc["tilesets"][0]["tiles"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|tile| tile["class"] == class)
                            .unwrap()["id"]
                            .as_u64()
                            .unwrap()
                            + 1;
                        let layer = doc["layers"]
                            .as_array_mut()
                            .unwrap()
                            .iter_mut()
                            .find(|layer| layer["name"] == "base_terrain")
                            .unwrap();
                        layer["data"][34 * 38 + 37] = json!(gid);
                    }
                    (
                        member.id.to_string(),
                        crate::compile_member(member, &doc).unwrap(),
                    )
                })
                .collect();
            assert!(
                link(&members)
                    .unwrap_err()
                    .contains("matching visible door thresholds")
            );
        }
    }
}
