//! The formats of the files under `content/`, as they're written: metres,
//! milliseconds and degrees, which a person can read. `compile` turns them
//! into the simulation's whole millimetres and steps.

use serde::de::Error;
use serde::{Deserialize, Deserializer};
use tme_sim::geom::{FARTHEST, METRE};

/// A point on the ground, `[east, north]` in metres.
pub type Point = [Metres; 2];

/// A number of metres (or metres a second): a finite number no bigger
/// than the simulation's coordinates go.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Metres(pub f64);

impl<'de> Deserialize<'de> for Metres {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Metres, D::Error> {
        let m = f64::deserialize(d)?;
        let farthest = f64::from(FARTHEST / METRE);
        if m.is_finite() && m.abs() <= farthest {
            Ok(Metres(m))
        } else {
            Err(D::Error::custom(format!(
                "{m} isn't a number of metres from -{farthest} to {farthest}"
            )))
        }
    }
}

/// `content/rules.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    /// The body every person has, in `bodies/`.
    pub person: String,
    /// The place someone new arrives in, in `places/`.
    pub arrival: String,
    pub turning_up: TurningUp,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurningUp {
    pub base_ms: u32,
    pub per_tardy_ms: u32,
    pub max_tardies: usize,
}

/// `content/bodies/NAME.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Body {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub radius_m: Metres,
    pub speed_m_per_s: Metres,
    pub health: i32,
    /// The model it's drawn with, under `assets/models/`.
    pub model: String,
    /// How much bigger or smaller than the model as it comes.
    #[serde(default = "one")]
    pub scale: f64,
    pub attack: Attack,
    pub dodge: Option<Dodge>,
    pub behaviour: Option<Behaviour>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attack {
    pub wind_up_ms: u32,
    pub recover_ms: u32,
    pub reach_m: Metres,
    /// The whole arc it sweeps in front.
    pub arc_degrees: f64,
    pub damage: i32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dodge {
    pub distance_m: Metres,
    pub duration_ms: u32,
    pub recover_ms: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Behaviour {
    pub sense_m: Metres,
    pub leash_m: Metres,
    pub roam_m: Metres,
    pub respawn_ms: u32,
}

/// `content/marks/NAME.toml`: a tardy.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mark {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub effects: Effects,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effects {
    pub max_health: Option<i32>,
    pub speed_percent: Option<i32>,
    #[serde(default)]
    pub sees_lately: bool,
}

/// `content/places/NAME.toml`: one area, both sides of it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place {
    pub name: String,
    /// The south-west and north-east corners.
    pub bounds: [Point; 2],
    pub arrival: Point,
    #[serde(default)]
    pub ground: Ground,
    #[serde(default, rename = "threshold")]
    pub thresholds: Vec<Threshold>,
    #[serde(default, rename = "lair")]
    pub lairs: Vec<Lair>,
    #[serde(default, rename = "prop")]
    pub props: Vec<Prop>,
    #[serde(default, rename = "run")]
    pub runs: Vec<Run>,
}

/// The colour of the ground on each side, as `#rrggbb`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ground {
    pub living: String,
    pub lately: String,
}

impl Default for Ground {
    fn default() -> Ground {
        Ground {
            living: "#6b7a45".into(),
            lately: "#4c6b6a".into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Threshold {
    pub at: Point,
    pub radius_m: Metres,
    pub exit: Point,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lair {
    /// The kind of body that comes out, in `bodies/`.
    pub kind: String,
    pub at: Point,
    #[serde(default)]
    pub side: Side,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    #[default]
    Living,
    Lately,
}

/// Which sides something stands on. Left out, both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sides {
    #[default]
    Both,
    Living,
    Lately,
}

/// One thing standing in a place.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prop {
    /// Under `assets/models/`.
    pub model: String,
    pub at: Point,
    /// Degrees anticlockwise, seen from above.
    #[serde(default)]
    pub turn: f64,
    #[serde(default = "one")]
    pub scale: f64,
    #[serde(default)]
    pub sides: Sides,
    pub solid: Option<Solid>,
    /// It gives off light, as windows and lanterns do at dusk.
    #[serde(default)]
    pub glow: bool,
}

/// A row of the same models from one point to another, due east-west or
/// north-south: a wall, a fence. Solid along its whole length if it says so.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    /// Used in turn along the run, under `assets/models/`.
    pub models: Vec<String>,
    pub from: Point,
    pub to: Point,
    /// How far apart the models stand.
    pub every_m: Metres,
    #[serde(default = "one")]
    pub scale: f64,
    #[serde(default)]
    pub sides: Sides,
    /// How thick it is to walk into; not solid if left out.
    pub thickness_m: Option<Metres>,
}

/// What's solid about a prop, centred where it stands: a circle, or a box
/// lined up with east and north.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum Solid {
    Circle { radius_m: Metres },
    Box { size_m: Point },
}

fn one() -> f64 {
    1.0
}
