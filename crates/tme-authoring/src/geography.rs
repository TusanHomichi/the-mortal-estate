//! Canonical geographic identity for an accepted visual packet's encoding.
//!
//! The independent review comparison uses the same neutral shape: cell terrain
//! and passability, structures, landmarks, arrival and directed endpoints.
//! Source formatting, authoring metadata and presentation assets are excluded.

use crate::{Result, compile::Member, emit, graph::Connectivity};
use serde_json::json;
use std::collections::BTreeMap;

pub(crate) fn digest(members: &BTreeMap<String, Member>, graph: &Connectivity) -> Result<String> {
    let members = members
        .iter()
        .map(|(id, member)| {
            let cells = (0..member.height())
                .flat_map(|y| {
                    (0..member.width()).map(move |x| {
            json!({"x": x, "y": y, "passable": member.is_passable(crate::Point { x, y }),
                "terrain": member.cells()[y][x]})
        })
                })
                .collect::<Vec<_>>();
            (
                id.clone(),
                json!({
                    "width": member.width(), "height": member.height(), "cells": cells,
                    "structures": member.structures(), "landmarks": member.landmarks(),
                    "arrival": member.arrival(),
                }),
            )
        })
        .collect::<BTreeMap<_, _>>();
    Ok(emit::digest(&emit::json(
        &json!({"members": members, "connectivity": graph}),
    )?))
}
