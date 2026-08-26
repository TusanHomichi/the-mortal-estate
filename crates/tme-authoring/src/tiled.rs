//! Typed accessors over a Tiled JSON map document.
//!
//! Every reader here fails closed with a located message. Nothing in this
//! module knows what the fixture means; it only turns untyped JSON into typed
//! values or an error, so that [`crate::compile`] contains semantics and
//! nothing else.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::Result;

/// Tiled authors in pixels. One authored cell is this many pixels on a side,
/// and every coordinate the compiler accepts must land on that lattice.
pub const TILE: i64 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}

pub fn object<'a>(value: &'a Value, label: &str) -> Result<&'a serde_json::Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| format!("{label} must be an object"))
}

pub fn array<'a>(value: &'a Value, label: &str) -> Result<&'a Vec<Value>> {
    value
        .as_array()
        .ok_or_else(|| format!("{label} must be an array"))
}

pub fn string(value: Option<&Value>, label: &str) -> Result<String> {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty() && value.trim() == *value)
        .map(str::to_owned)
        .ok_or_else(|| format!("{label} must be a non-empty trimmed string"))
}

pub fn integer(value: Option<&Value>, label: &str) -> Result<i64> {
    value
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{label} must be an integer"))
}

/// Tiled custom properties, flattened to a name-keyed map. A duplicated name
/// is rejected rather than resolved, because there is no correct winner.
pub fn properties(value: &Value, label: &str) -> Result<BTreeMap<String, Value>> {
    let owner = object(value, label)?;
    let rows = owner
        .get("properties")
        .map(|value| array(value, &format!("{label}.properties")))
        .transpose()?
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut result = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        let row = object(row, &format!("{label}.properties[{index}]"))?;
        let name = string(
            row.get("name"),
            &format!("{label}.properties[{index}].name"),
        )?;
        let entry = row.get("value").cloned().unwrap_or(Value::Null);
        if result.insert(name.clone(), entry).is_some() {
            return Err(format!("{label} duplicates property {name:?}"));
        }
    }
    Ok(result)
}

pub fn property_string(
    properties: &BTreeMap<String, Value>,
    name: &str,
    label: &str,
) -> Result<String> {
    string(properties.get(name), &format!("{label}.{name}"))
}

pub fn property_int(
    properties: &BTreeMap<String, Value>,
    name: &str,
    label: &str,
) -> Result<usize> {
    let value = integer(properties.get(name), &format!("{label}.{name}"))?;
    if value < 0 {
        return Err(format!("{label}.{name} must be non-negative"));
    }
    Ok(value as usize)
}

pub fn property_bool(
    properties: &BTreeMap<String, Value>,
    name: &str,
    label: &str,
) -> Result<bool> {
    properties
        .get(name)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("{label}.{name} must be a boolean"))
}

/// An object's pixel origin, converted to a cell coordinate inside a
/// `width` x `height` envelope.
pub fn point(value: &Value, width: usize, height: usize, label: &str) -> Result<Point> {
    let row = object(value, label)?;
    let x = integer(row.get("x"), &format!("{label}.x"))?;
    let y = integer(row.get("y"), &format!("{label}.y"))?;
    if x < 0 || y < 0 || x % TILE != 0 || y % TILE != 0 {
        return Err(format!(
            "{label} must align to a non-negative {TILE}px authored cell"
        ));
    }
    let point = Point {
        x: (x / TILE) as usize,
        y: (y / TILE) as usize,
    };
    if point.x >= width || point.y >= height {
        return Err(format!("{label} is outside the {width}x{height} envelope"));
    }
    Ok(point)
}

pub fn layers_by_name(document: &Value) -> Result<BTreeMap<String, &Value>> {
    let root = object(document, "authored map")?;
    let rows = array(
        root.get("layers").ok_or("authored map.layers is missing")?,
        "authored map.layers",
    )?;
    let mut result = BTreeMap::new();
    for (index, layer) in rows.iter().enumerate() {
        let row = object(layer, &format!("authored map.layers[{index}]"))?;
        let name = string(
            row.get("name"),
            &format!("authored map.layers[{index}].name"),
        )?;
        if result.insert(name.clone(), layer).is_some() {
            return Err(format!("authored map duplicates layer {name:?}"));
        }
    }
    Ok(result)
}

pub fn tile_data(layer: &Value, name: &str, width: usize, height: usize) -> Result<Vec<u32>> {
    let row = object(layer, &format!("layer {name}"))?;
    if row.get("type").and_then(Value::as_str) != Some("tilelayer") {
        return Err(format!("layer {name} must be a tilelayer"));
    }
    for (field, expected) in [("width", width as i64), ("height", height as i64)] {
        if integer(row.get(field), &format!("layer {name}.{field}"))? != expected {
            return Err(format!("layer {name}.{field} must be {expected}"));
        }
    }
    let values = array(
        row.get("data")
            .ok_or_else(|| format!("layer {name}.data is missing"))?,
        &format!("layer {name}.data"),
    )?;
    if values.len() != width * height {
        return Err(format!(
            "layer {name} must contain exactly {} cells",
            width * height
        ));
    }
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value
                .as_u64()
                .and_then(|value| u32::try_from(value).ok())
                .ok_or_else(|| format!("layer {name}.data[{index}] must be a u32 tile id"))
        })
        .collect()
}

pub fn named_objects<'a>(layer: &'a Value, name: &str) -> Result<BTreeMap<String, &'a Value>> {
    let row = object(layer, &format!("layer {name}"))?;
    if row.get("type").and_then(Value::as_str) != Some("objectgroup") {
        return Err(format!("layer {name} must be an objectgroup"));
    }
    let rows = array(
        row.get("objects")
            .ok_or_else(|| format!("layer {name}.objects is missing"))?,
        &format!("layer {name}.objects"),
    )?;
    let mut result = BTreeMap::new();
    for (index, value) in rows.iter().enumerate() {
        let row = object(value, &format!("layer {name}.objects[{index}]"))?;
        let id = string(
            row.get("name"),
            &format!("layer {name}.objects[{index}].name"),
        )?;
        if result.insert(id.clone(), value).is_some() {
            return Err(format!("layer {name} duplicates object {id:?}"));
        }
    }
    Ok(result)
}

/// The embedded tileset's declared tile classes, in tile-id order.
///
/// The fixture carries no tileset image: the successor has no accepted visual
/// vocabulary, so a class here is a NAME and nothing else. Reading it back is
/// what lets the compiler prove the authored document and its compiled-in
/// vocabulary agree instead of trusting bare integers.
pub fn tileset_classes(document: &Value) -> Result<Vec<String>> {
    let root = object(document, "authored map")?;
    let sets = array(
        root.get("tilesets")
            .ok_or("authored map.tilesets is missing")?,
        "authored map.tilesets",
    )?;
    if sets.len() != 1 {
        return Err("authored map must embed exactly one tileset".into());
    }
    let set = object(&sets[0], "authored map.tilesets[0]")?;
    if integer(set.get("firstgid"), "tileset.firstgid")? != 1 {
        return Err("tileset.firstgid must be 1".into());
    }
    for field in ["tilewidth", "tileheight"] {
        if integer(set.get(field), &format!("tileset.{field}"))? != TILE {
            return Err(format!("tileset.{field} must be {TILE}"));
        }
    }
    let tiles = array(
        set.get("tiles").ok_or("tileset.tiles is missing")?,
        "tileset.tiles",
    )?;
    if integer(set.get("tilecount"), "tileset.tilecount")? != tiles.len() as i64 {
        return Err("tileset.tilecount disagrees with tileset.tiles".into());
    }
    tiles
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let row = object(value, &format!("tileset.tiles[{index}]"))?;
            if integer(row.get("id"), &format!("tileset.tiles[{index}].id"))? != index as i64 {
                return Err(format!("tileset.tiles[{index}].id must be {index}"));
            }
            string(row.get("class"), &format!("tileset.tiles[{index}].class"))
        })
        .collect()
}
