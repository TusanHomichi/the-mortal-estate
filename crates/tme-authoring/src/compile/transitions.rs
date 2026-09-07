//! Transition document decoding and local endpoint validation.

use super::{Grid, TileLayers, Transition, require_marker};
use crate::contract::MemberContract;
use crate::tiled::Point;
use crate::{Result, contract, tiled};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn read(
    document: &Value,
    contract: &'static MemberContract,
    layers: &TileLayers,
    grid: &Grid,
) -> Result<BTreeMap<String, Transition>> {
    let program = contract
        .transitions
        .iter()
        .map(|entry| {
            (
                entry.id,
                (
                    entry.target_member,
                    entry.paired_transition,
                    entry.direction,
                    entry.marker_class,
                    entry.layout,
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let objects = match tiled::layers_by_name(document)?.get("transitions") {
        Some(layer) => tiled::named_objects(layer, "transitions")?,
        None => BTreeMap::new(),
    };
    if objects.keys().map(String::as_str).collect::<BTreeSet<_>>()
        != program.keys().copied().collect()
    {
        return Err(format!(
            "the {} transition program differs from the accepted contract",
            contract.id
        ));
    }
    let mut transitions = BTreeMap::new();
    for (id, value) in &objects {
        let label = format!("transition {id}");
        let row = tiled::object(value, &label)?;
        if row.get("class").and_then(Value::as_str) != Some(contract::TRANSITION_CLASS) {
            return Err(format!("{label} must be a {}", contract::TRANSITION_CLASS));
        }
        let marker = tiled::point(value, layers.width, layers.height, &label)?;
        let props = tiled::properties(value, &label)?;
        let (target, pair, direction, marker_class, layout) = program[id.as_str()];
        if tiled::property_string(&props, "target_member", &label)? != target
            || tiled::property_string(&props, "paired_transition", &label)? != pair
            || tiled::property_string(&props, "direction", &label)? != direction
        {
            return Err(format!("{label} differs from the accepted contract"));
        }
        let access = Point {
            x: tiled::property_int(&props, "access_cell_x", &label)?,
            y: tiled::property_int(&props, "access_cell_y", &label)?,
        };
        if access.x >= layers.width || access.y >= layers.height {
            return Err(format!("{label} access cell leaves the envelope"));
        }
        let landing = match layout {
            contract::TransitionLayout::AdjacentMarker => {
                if props.contains_key("landing_cell_x") || props.contains_key("landing_cell_y") {
                    return Err(format!("{label} does not author a separate landing"));
                }
                if marker.x.abs_diff(access.x) + marker.y.abs_diff(access.y) != 1 {
                    return Err(format!(
                        "{label} access cell must be cardinally adjacent to its marker"
                    ));
                }
                access
            }
            contract::TransitionLayout::Threshold => {
                if marker != access {
                    return Err(format!(
                        "{label} threshold marker must equal its access cell"
                    ));
                }
                Point {
                    x: tiled::property_int(&props, "landing_cell_x", &label)?,
                    y: tiled::property_int(&props, "landing_cell_y", &label)?,
                }
            }
        };
        if !grid.passable.contains(&marker)
            || !grid.passable.contains(&access)
            || !grid.passable.contains(&landing)
        {
            return Err(format!(
                "{label} marker, access or landing cell is blocked or outside the envelope"
            ));
        }
        require_marker(grid, marker, marker_class, &label)?;
        transitions.insert(
            id.clone(),
            Transition {
                id: id.clone(),
                member: contract.id.into(),
                target_member: target.into(),
                paired_transition: pair.into(),
                direction: direction.into(),
                marker,
                access,
                landing,
            },
        );
    }
    Ok(transitions)
}
