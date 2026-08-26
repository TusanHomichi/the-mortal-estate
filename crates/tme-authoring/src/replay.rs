//! Deterministic replay: a log of staged operations against a copy of the
//! accepted master, producing a candidate document and nothing else.
//!
//! **Why replay is here and not in the tool that stages.** Applying a verb
//! means knowing which tile layers are authored, which are derived, and what
//! each derivation is. That knowledge already exists exactly once, in
//! [`crate::contract`] and [`crate::compile`]. A second implementation in
//! another language would be a second opinion about what an authored member is,
//! and the project's own standard names that class of duplication as the
//! false-green failure it exists to kill. So the session, the log, the
//! ordering, and the digests are the Workbench's; the document is the
//! compiler's.
//!
//! **Deterministic** means what it says: the same operations against the same
//! base produce byte-identical output, because the mutation is a pure function
//! of the two and the serializer is [`crate::emit`]'s single one.
//!
//! **No authority.** Replay reads the accepted master as bytes, writes a
//! candidate to a path its caller names, and grants nothing. It does not read
//! the promotion receipt, does not consult the reviewed digest, and produces no
//! value that [`crate::promotion::load`] would accept. The candidate is judged
//! by [`crate::candidate::validate_candidate`], which has no authority either.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::Result;
use crate::compile;
use crate::contract::{self, MemberContract, TileRole};
use crate::operations::{
    Cell, MoveLandmark, MoveStructure, SetRoute, SetStructureAccess, SetTerrain,
    SetTransitionEndpoint, StagedOperation, TruthEdit,
};
use crate::tiled::{self, TILE};

/// Replay the whole operation set against `document`, in log order.
///
/// The document is mutated in place, so a caller that wants the base preserved
/// hands over a clone — which the CLI does, because the base is the tracked
/// master and nothing here may write it.
pub fn replay(
    member: &'static MemberContract,
    document: &mut Value,
    operations: &[StagedOperation],
) -> Result<()> {
    for operation in operations {
        let edit = crate::operations::parse(member, operation)?;
        apply(member, document, &edit)
            .map_err(|error| format!("{} ({}): {error}", operation.record_id, operation.verb))?;
    }
    refresh_derived(member, document)
}

fn apply(member: &'static MemberContract, document: &mut Value, edit: &TruthEdit) -> Result<()> {
    match edit {
        TruthEdit::SetTerrain(parameters) => set_terrain(member, document, parameters),
        TruthEdit::SetRoute(parameters) => set_route(member, document, parameters),
        TruthEdit::MoveStructure(parameters) => move_structure(document, parameters),
        TruthEdit::SetStructureAccess(parameters) => set_structure_access(document, parameters),
        TruthEdit::MoveLandmark(parameters) => move_landmark(member, document, parameters),
        TruthEdit::SetTransitionEndpoint(parameters) => {
            set_transition_endpoint(member, document, parameters)
        }
    }
}

// ---------------------------------------------------------------------------
// The verbs
// ---------------------------------------------------------------------------

fn set_terrain(
    member: &'static MemberContract,
    document: &mut Value,
    parameters: &SetTerrain,
) -> Result<()> {
    let gid = gid_of(member, document, &parameters.class, |role| {
        matches!(role, TileRole::Base { .. })
    })?;
    let (width, height) = envelope(document)?;
    for cell in &parameters.cells {
        let index = index_of(*cell, width, height)?;
        layer_data(document, "base_terrain")?[index] = json!(gid);
    }
    Ok(())
}

fn set_route(
    member: &'static MemberContract,
    document: &mut Value,
    parameters: &SetRoute,
) -> Result<()> {
    let gid = match &parameters.class {
        Some(class) => gid_of(member, document, class, |role| {
            matches!(role, TileRole::Route)
        })?,
        None => 0,
    };
    let (width, height) = envelope(document)?;
    for cell in &parameters.cells {
        let index = index_of(*cell, width, height)?;
        layer_data(document, "routes")?[index] = json!(gid);
    }
    Ok(())
}

fn move_structure(document: &mut Value, parameters: &MoveStructure) -> Result<()> {
    let (width, height) = envelope(document)?;
    let structure = object(document, "structures", &parameters.structure_id)?;
    let from = origin_of(structure)?;
    structure["x"] = json!(pixels(parameters.to.x));
    structure["y"] = json!(pixels(parameters.to.y));
    let access = access_cell(structure)?;
    // The door travels with the building. Moving a footprint out from under
    // its own access cell would leave a door standing in a field, and the
    // compiler would reject it — correctly, and unhelpfully.
    let moved = Cell {
        x: shift(access.x, from.x, parameters.to.x)?,
        y: shift(access.y, from.y, parameters.to.y)?,
    };
    bounds_check(moved, width, height)?;
    set_int_property(structure, "access_cell_x", moved.x)?;
    set_int_property(structure, "access_cell_y", moved.y)?;
    Ok(())
}

fn set_structure_access(document: &mut Value, parameters: &SetStructureAccess) -> Result<()> {
    let (width, height) = envelope(document)?;
    bounds_check(parameters.cell, width, height)?;
    let structure = object(document, "structures", &parameters.structure_id)?;
    set_int_property(structure, "access_cell_x", parameters.cell.x)?;
    set_int_property(structure, "access_cell_y", parameters.cell.y)?;
    Ok(())
}

fn move_landmark(
    member: &'static MemberContract,
    document: &mut Value,
    parameters: &MoveLandmark,
) -> Result<()> {
    let (width, height) = envelope(document)?;
    bounds_check(parameters.to, width, height)?;
    let marker_class = member
        .landmark(&parameters.landmark_id)
        .map(|landmark| landmark.marker_class)
        .ok_or_else(|| {
            format!(
                "landmark {:?} is not authored in this member",
                parameters.landmark_id
            )
        })?;
    let landmark = object(document, "landmarks", &parameters.landmark_id)?;
    let from = origin_of(landmark)?;
    landmark["x"] = json!(pixels(parameters.to.x));
    landmark["y"] = json!(pixels(parameters.to.y));
    move_marker(member, document, from, parameters.to, marker_class)
}

fn set_transition_endpoint(
    member: &'static MemberContract,
    document: &mut Value,
    parameters: &SetTransitionEndpoint,
) -> Result<()> {
    let (width, height) = envelope(document)?;
    let marker_class = member
        .transition(&parameters.transition_id)
        .map(|transition| transition.marker_class)
        .ok_or_else(|| {
            format!(
                "transition {:?} is not authored in this member",
                parameters.transition_id
            )
        })?;
    let transition = object(document, "transitions", &parameters.transition_id)?;
    let mut moved_marker = None;
    if let Some(cell) = parameters.marker {
        bounds_check(cell, width, height)?;
        let from = origin_of(transition)?;
        transition["x"] = json!(pixels(cell.x));
        transition["y"] = json!(pixels(cell.y));
        moved_marker = Some((from, cell));
    }
    if let Some(cell) = parameters.access {
        bounds_check(cell, width, height)?;
        set_int_property(transition, "access_cell_x", cell.x)?;
        set_int_property(transition, "access_cell_y", cell.y)?;
    }
    if parameters.marker.is_none() && parameters.access.is_none() {
        return Err("names neither a marker nor an access cell to move".into());
    }
    match moved_marker {
        Some((from, to)) => move_marker(member, document, from, to, marker_class),
        None => Ok(()),
    }
}

/// Carry a feature's authored marker tile from one cell to another.
///
/// The old tile is cleared only when it is the marker this feature owns, so a
/// mark that belongs to something else is never quietly erased. The programs
/// pin which class each feature carries, so there is no guessing.
fn move_marker(
    member: &'static MemberContract,
    document: &mut Value,
    from: Cell,
    to: Cell,
    marker_class: &str,
) -> Result<()> {
    if marker_class.is_empty() {
        return Ok(());
    }
    let gid = gid_of(member, document, marker_class, |role| {
        matches!(role, TileRole::Mark)
    })?;
    let (width, height) = envelope(document)?;
    let source = index_of(from, width, height)?;
    let destination = index_of(to, width, height)?;
    let marks = layer_data(document, "landmark_marks")?;
    if marks[source] == json!(gid) {
        marks[source] = json!(0);
    }
    marks[destination] = json!(gid);
    Ok(())
}

// ---------------------------------------------------------------------------
// The derived layers
// ---------------------------------------------------------------------------

/// Rebuild every layer the authored objects and terrain already decide.
///
/// `structure_footprints` is the union of the structure objects' rectangles —
/// that is the equality [`compile`] asserts. `passability` is
/// [`compile::cell_is_passable`] over the three layers that decide it, called rather
/// than restated. Neither is a judgment; both are arithmetic the compiler then
/// independently checks, so a wrong refresh is a rejected candidate.
fn refresh_derived(member: &'static MemberContract, document: &mut Value) -> Result<()> {
    let (width, height) = envelope(document)?;
    // The derived classes are found by ROLE, never by name: a member declares
    // its own vocabulary, and a name written here would be a second opinion
    // about what this member calls its footprint.
    let footprint_class =
        member.sole_class(|role| matches!(role, TileRole::Footprint), "footprint")?;
    let blocked_class = member.sole_class(
        |role| matches!(role, TileRole::Passability { walkable: false }),
        "blocked passability",
    )?;
    let walkable_class = member.sole_class(
        |role| matches!(role, TileRole::Passability { walkable: true }),
        "walkable passability",
    )?;
    let footprint_gid = gid_of(member, document, footprint_class, |role| {
        matches!(role, TileRole::Footprint)
    })?;
    let blocked_gid = gid_of(member, document, blocked_class, |role| {
        matches!(role, TileRole::Passability { walkable: false })
    })?;
    let walkable_gid = gid_of(member, document, walkable_class, |role| {
        matches!(role, TileRole::Passability { walkable: true })
    })?;
    let blocked_bases = blocked_base_gids(member, document)?;

    let mut footprints = vec![0_u64; width * height];
    for structure in objects(document, "structures")? {
        let origin = origin_of(&structure)?;
        let size = size_of(&structure)?;
        for y in origin.y..origin.y + size.1 {
            for x in origin.x..origin.x + size.0 {
                let index = index_of(Cell { x, y }, width, height)?;
                footprints[index] = footprint_gid as u64;
            }
        }
    }
    let base = read_layer(document, "base_terrain")?;
    let routes = read_layer(document, "routes")?;
    let passability = (0..width * height)
        .map(|index| {
            let walkable = compile::cell_is_passable(
                blocked_bases.contains(&base[index]),
                routes[index] as u32,
                footprints[index] as u32,
            );
            json!(if walkable { walkable_gid } else { blocked_gid })
        })
        .collect::<Vec<_>>();

    *layer_data(document, "structure_footprints")? =
        footprints.into_iter().map(|gid| json!(gid)).collect();
    *layer_data(document, "passability")? = passability;
    Ok(())
}

/// Every tile id whose class blocks movement as base terrain.
fn blocked_base_gids(member: &'static MemberContract, document: &Value) -> Result<Vec<u64>> {
    let classes = tiled::tileset_classes(document)?;
    let accepted = member
        .classes
        .iter()
        .map(|class| (class.name, class.role))
        .collect::<BTreeMap<_, _>>();
    Ok(classes
        .iter()
        .enumerate()
        .filter(|(_, name)| {
            matches!(
                accepted.get(name.as_str()),
                Some(TileRole::Base { blocked: true })
            )
        })
        .map(|(index, _)| index as u64 + 1)
        .collect())
}

// ---------------------------------------------------------------------------
// Document access
// ---------------------------------------------------------------------------

fn envelope(document: &Value) -> Result<(usize, usize)> {
    let width = tiled::integer(document.get("width"), "authored map.width")?;
    let height = tiled::integer(document.get("height"), "authored map.height")?;
    if width <= 0 || height <= 0 {
        return Err("the authored envelope is empty".into());
    }
    Ok((width as usize, height as usize))
}

fn index_of(cell: Cell, width: usize, height: usize) -> Result<usize> {
    bounds_check(cell, width, height)?;
    Ok(cell.y * width + cell.x)
}

fn bounds_check(cell: Cell, width: usize, height: usize) -> Result<()> {
    if cell.x >= width || cell.y >= height {
        return Err(format!(
            "cell {},{} is outside the {width}x{height} envelope",
            cell.x, cell.y
        ));
    }
    Ok(())
}

fn pixels(cell: usize) -> i64 {
    cell as i64 * TILE
}

fn shift(value: usize, from: usize, to: usize) -> Result<usize> {
    let moved = value as i64 + to as i64 - from as i64;
    if moved < 0 {
        return Err("the move would carry an access cell off the envelope".into());
    }
    Ok(moved as usize)
}

fn layer_data<'a>(document: &'a mut Value, name: &str) -> Result<&'a mut Vec<Value>> {
    document
        .get_mut("layers")
        .and_then(Value::as_array_mut)
        .ok_or("authored map.layers is missing")?
        .iter_mut()
        .find(|layer| layer.get("name").and_then(Value::as_str) == Some(name))
        .ok_or_else(|| format!("the authored map carries no layer {name:?}"))?
        .get_mut("data")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| format!("layer {name} carries no tile data"))
}

fn read_layer(document: &Value, name: &str) -> Result<Vec<u64>> {
    let layers = tiled::layers_by_name(document)?;
    let layer = layers
        .get(name)
        .ok_or_else(|| format!("the authored map carries no layer {name:?}"))?;
    tiled::array(
        layer
            .get("data")
            .ok_or_else(|| format!("layer {name} carries no tile data"))?,
        &format!("layer {name}.data"),
    )?
    .iter()
    .map(|value| {
        value
            .as_u64()
            .ok_or_else(|| format!("layer {name} carries a tile id that is not an integer"))
    })
    .collect()
}

fn object<'a>(document: &'a mut Value, layer: &str, name: &str) -> Result<&'a mut Value> {
    document
        .get_mut("layers")
        .and_then(Value::as_array_mut)
        .ok_or("authored map.layers is missing")?
        .iter_mut()
        .find(|row| row.get("name").and_then(Value::as_str) == Some(layer))
        .ok_or_else(|| format!("the authored map carries no layer {layer:?}"))?
        .get_mut("objects")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| format!("layer {layer} carries no objects"))?
        .iter_mut()
        .find(|row| row.get("name").and_then(Value::as_str) == Some(name))
        .ok_or_else(|| format!("{name:?} is not authored in the {layer} layer"))
}

fn objects(document: &Value, layer: &str) -> Result<Vec<Value>> {
    let layers = tiled::layers_by_name(document)?;
    let rows = layers
        .get(layer)
        .ok_or_else(|| format!("the authored map carries no layer {layer:?}"))?
        .get("objects")
        .ok_or_else(|| format!("layer {layer} carries no objects"))?;
    Ok(tiled::array(rows, &format!("layer {layer}.objects"))?.clone())
}

fn origin_of(object: &Value) -> Result<Cell> {
    let x = tiled::integer(object.get("x"), "object.x")?;
    let y = tiled::integer(object.get("y"), "object.y")?;
    if x < 0 || y < 0 || x % TILE != 0 || y % TILE != 0 {
        return Err("the object does not stand on the authored cell lattice".into());
    }
    Ok(Cell {
        x: (x / TILE) as usize,
        y: (y / TILE) as usize,
    })
}

fn size_of(object: &Value) -> Result<(usize, usize)> {
    let width = tiled::integer(object.get("width"), "object.width")?;
    let height = tiled::integer(object.get("height"), "object.height")?;
    if width <= 0 || height <= 0 || width % TILE != 0 || height % TILE != 0 {
        return Err("the object's size is not a whole number of authored cells".into());
    }
    Ok(((width / TILE) as usize, (height / TILE) as usize))
}

fn access_cell(object: &Value) -> Result<Cell> {
    let properties = tiled::properties(object, "object")?;
    Ok(Cell {
        x: tiled::property_int(&properties, "access_cell_x", "object")?,
        y: tiled::property_int(&properties, "access_cell_y", "object")?,
    })
}

/// Write an existing integer property. Existing, deliberately: a verb changes
/// what an authored object says, and inventing a property it never declared is
/// how a document grows a field the accepted contract does not know about.
fn set_int_property(object: &mut Value, name: &str, value: usize) -> Result<()> {
    let row = object
        .get_mut("properties")
        .and_then(Value::as_array_mut)
        .ok_or("the object carries no properties")?
        .iter_mut()
        .find(|row| row.get("name").and_then(Value::as_str) == Some(name))
        .ok_or_else(|| format!("the object declares no {name} property"))?;
    row["value"] = json!(value);
    Ok(())
}

/// The tile id of a class, checked against the role the contract gives it.
///
/// The role check is what stops a verb writing a footprint class into the
/// terrain layer: the compiler would reject it, and refusing here says which
/// verb did it.
fn gid_of(
    member: &'static MemberContract,
    document: &Value,
    class: &str,
    admits: impl Fn(TileRole) -> bool,
) -> Result<u64> {
    let accepted = member
        .class(class)
        .ok_or_else(|| format!("{class:?} is not a class of the accepted vocabulary"))?;
    if !admits(accepted.role) {
        return Err(format!(
            "{class:?} belongs to the {} layer and cannot be written here",
            contract::layer_of(accepted.role)
        ));
    }
    let index = tiled::tileset_classes(document)?
        .iter()
        .position(|name| name == class)
        .ok_or_else(|| format!("the authored tileset declares no class {class:?}"))?;
    Ok(index as u64 + 1)
}

#[cfg(test)]
mod tests {
    //! The mutant that qualifies the derived refresh (P9).
    //!
    //! Three layers are derived rather than authored, and the whole safety
    //! argument for deriving them in the replay is that the compiler checks
    //! them independently. That argument is worth exactly as much as the proof
    //! that a wrong derivation dies — so here it is, planted twice: apply a
    //! verb, deliberately skip [`refresh_derived`], and watch the candidate be
    //! rejected in the compiler's own words.

    use super::*;
    use crate::candidate::validate_candidate;
    use crate::contract::fixture;
    use crate::operations::StagedOperation;

    fn editable() -> &'static MemberContract {
        fixture::LAND.candidate_member().unwrap()
    }

    fn surface_document() -> Value {
        serde_json::from_str(include_str!(
            "../../../content/authoring-fixture/fixture-surface.tmj"
        ))
        .unwrap()
    }

    /// Replay WITHOUT refreshing the derived layers. This is the mutant.
    fn replay_without_refresh(document: &mut Value, operation: &StagedOperation) {
        let edit = crate::operations::parse(editable(), operation).unwrap();
        apply(editable(), document, &edit).unwrap();
    }

    fn staged(verb: &str, parameters: Value) -> StagedOperation {
        serde_json::from_value(json!({
            "record_id": "op-mutant",
            "author": "test",
            "class": "truth",
            "member": "surface",
            "verb": verb,
            "parameters": parameters,
        }))
        .unwrap()
    }

    fn diagnostic(document: &Value) -> String {
        let report = validate_candidate(fixture::LAND.id, editable(), document).unwrap();
        assert!(!report.accepted, "the mutant was accepted");
        report.diagnostics.first().cloned().unwrap()
    }

    #[test]
    fn a_terrain_edit_without_the_passability_refresh_is_rejected() {
        let mut document = surface_document();
        replay_without_refresh(
            &mut document,
            &staged(
                "set_terrain",
                json!({"cells": [{"x": 2, "y": 1}], "class": "testland_deep_water"}),
            ),
        );
        assert!(
            diagnostic(&document).contains("the passability annotation is stale at 2,1"),
            "the compiler did not catch a stale derived layer"
        );
    }

    #[test]
    fn a_structure_move_without_the_footprint_refresh_is_rejected() {
        let mut document = surface_document();
        replay_without_refresh(
            &mut document,
            &staged(
                "move_structure",
                json!({"structure_id": "fixture_structure_outland", "to": {"x": 20, "y": 5}}),
            ),
        );
        assert!(
            diagnostic(&document).contains(
                "structure objects and the structure_footprints layer describe different cells"
            ),
            "the compiler did not catch a stale footprint layer"
        );
    }

    #[test]
    fn the_refresh_is_what_makes_those_same_edits_pass() {
        for operation in [
            staged(
                "set_terrain",
                json!({"cells": [{"x": 2, "y": 1}], "class": "testland_forest"}),
            ),
            staged(
                "move_structure",
                json!({"structure_id": "fixture_structure_outland", "to": {"x": 20, "y": 5}}),
            ),
        ] {
            let mut document = surface_document();
            replay(editable(), &mut document, std::slice::from_ref(&operation)).unwrap();
            let report = validate_candidate(fixture::LAND.id, editable(), &document).unwrap();
            assert!(report.accepted, "{:?}", report.diagnostics);
        }
    }
}
