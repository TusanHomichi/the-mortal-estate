//! What the world is made of: kinds of bodies, the marks a return leaves,
//! the places and the rules, as `tme-content` compiles them from `content/`.
//! The simulation only reads it.

use crate::geom::{Mm, Rect, Shape, Vec2};
use crate::{STEP_MS, Steps};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// The name of an entry of kind `T`, as content files name it: `dun`,
/// `lattermead`. Typed, so a mark's name can't stand in for a place's.
#[derive(Serialize, Deserialize)]
#[serde(transparent, bound = "")]
pub struct Id<T> {
    name: String,
    #[serde(skip)]
    of: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    pub fn new(name: impl Into<String>) -> Id<T> {
        Id {
            name: name.into(),
            of: PhantomData,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.name
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Id::new(self.name.clone())
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl<T> Eq for Id<T> {}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.name)
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

/// A duration in milliseconds, as content writes it, turned into whole steps
/// (rounded up, so nothing is ever shorter than written).
pub fn steps_from_ms(ms: u32) -> Steps {
    ms.div_ceil(STEP_MS)
}

/// A kind of body: a person, or a kind of creature.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyKind {
    pub name: String,
    pub radius: Mm,
    /// How far it walks in a step at a full push.
    pub speed: Mm,
    pub health: i32,
    pub attack: Attack,
    pub dodge: Option<Dodge>,
    /// How a creature of this kind behaves; `None` for a person, whom a
    /// player moves.
    pub behaviour: Option<Behaviour>,
}

/// A committed swing: it winds up, lands on one step, and recovers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attack {
    pub wind_up: Steps,
    pub recover: Steps,
    /// How far past the attacker's own edge it reaches.
    pub reach: Mm,
    /// The cosine of half the arc it sweeps, in 1024ths: 0 is a half
    /// circle in front, 1024 a straight line.
    pub arc_cos: i32,
    pub damage: i32,
}

impl Attack {
    /// Steps from the start of the swing to the end of its recovery.
    pub fn length(&self) -> Steps {
        self.wind_up + self.recover
    }
}

/// A committed dash: the only way out of a wind-up.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dodge {
    /// How far it carries you, spread evenly over `duration`.
    pub distance: Mm,
    pub duration: Steps,
    pub recover: Steps,
}

/// How a creature hunts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Behaviour {
    /// How near someone has to be for it to notice them.
    pub sense: Mm,
    /// How far from its lair it will chase before giving up and going home.
    pub leash: Mm,
    /// How far from its lair it wanders when there's nobody to hunt.
    pub roam: Mm,
    /// How long after it falls before another takes its place.
    pub respawn: Steps,
}

/// A mark, which the land calls a tardy: what a return from Lately leaves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mark {
    pub name: String,
    pub description: String,
    pub effects: Vec<Effect>,
}

/// What a mark does to a life.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    /// Health added to (or, negative, taken from) the body's most.
    MaxHealth(i32),
    /// Walking speed changed by this many percent.
    Speed(i32),
    /// The dead show clearly to this life, rather than faintly.
    SeesLately,
}

/// Which side of the land a body is on.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Side {
    /// The land as it is now.
    #[default]
    Living,
    /// The land as it was, where the dead are: they've gone to Lately.
    Lately,
}

/// One area of the land, both sides of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub name: String,
    /// Nobody walks out of these.
    pub bounds: Rect,
    /// What's solid on each side: the hall whole in Lately, its rubble in
    /// the living land.
    pub living: Vec<Shape>,
    pub lately: Vec<Shape>,
    /// Where someone new to the world first stands.
    pub arrival: Vec2,
    pub thresholds: Vec<Threshold>,
    pub lairs: Vec<Lair>,
}

impl Place {
    pub fn solids(&self, side: Side) -> &[Shape] {
        match side {
            Side::Living => &self.living,
            Side::Lately => &self.lately,
        }
    }

    /// Whether nothing solid on `side` stands between `a` and `b`: what a
    /// swing needs to reach.
    pub fn unobstructed(&self, side: Side, a: Vec2, b: Vec2) -> bool {
        !self.solids(side).iter().any(|s| s.crosses(a, b))
    }
}

/// A place where the two sides touch: the dead turn up here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Threshold {
    pub at: Vec2,
    /// How near the dead must stand to use it.
    pub radius: Mm,
    /// Where someone who turns up here stands in the living land.
    pub exit: Vec2,
}

/// Where creatures of a kind keep coming from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lair {
    pub kind: Id<BodyKind>,
    pub at: Vec2,
    pub side: Side,
}

/// What turning up costs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurningUp {
    /// How long the rite holds you, with no tardies yet.
    pub base: Steps,
    /// How much longer each tardy you already carry makes it.
    pub per_tardy: Steps,
    /// A life carrying this many tardies can't turn up again; it stays in
    /// Lately.
    pub max_tardies: usize,
}

/// The rules that aren't any one entry's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    /// The kind of body every person has.
    pub person: Id<BodyKind>,
    /// The place someone new arrives in.
    pub arrival: Id<Place>,
    pub turning_up: TurningUp,
}

/// Everything the simulation runs on, every reference in it resolved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Catalogue {
    pub bodies: BTreeMap<Id<BodyKind>, BodyKind>,
    pub marks: BTreeMap<Id<Mark>, Mark>,
    pub places: BTreeMap<Id<Place>, Place>,
    pub rules: Rules,
}

/// What's wrong with a catalogue: which entry, and how.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unsound {
    /// The entry at fault: `rules`, or `places/lattermead`.
    pub entry: String,
    pub problem: String,
}

impl fmt::Display for Unsound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.entry, self.problem)
    }
}

impl std::error::Error for Unsound {}

impl Catalogue {
    /// Every reference between entries resolved, every shape whole and the
    /// rules satisfiable: everything wrong, if anything is.
    pub fn check(&self) -> Result<(), Vec<Unsound>> {
        let mut wrong = Vec::new();
        let mut unsound = |entry: String, problem: String| {
            wrong.push(Unsound { entry, problem });
        };
        let missing = |what: String| format!("names {what}, which doesn't exist");
        if !self.bodies.contains_key(&self.rules.person) {
            unsound(
                "rules".into(),
                missing(format!("bodies/{}", self.rules.person)),
            );
        }
        if !self.places.contains_key(&self.rules.arrival) {
            unsound(
                "rules".into(),
                missing(format!("places/{}", self.rules.arrival)),
            );
        }
        if self.rules.turning_up.max_tardies > self.marks.len() {
            unsound(
                "rules".into(),
                format!(
                    "lets a life carry {} tardies, and there are only {} kinds in marks/",
                    self.rules.turning_up.max_tardies,
                    self.marks.len()
                ),
            );
        }
        for (id, kind) in &self.bodies {
            let entry = format!("bodies/{id}");
            if kind.radius <= 0 {
                unsound(entry.clone(), "has no radius".into());
            }
            if kind.speed < 0 {
                unsound(entry.clone(), "has a speed below 0".into());
            }
            if let Some(b) = &kind.behaviour
                && (b.sense < 0 || b.leash < 0 || b.roam < 0)
            {
                unsound(
                    entry.clone(),
                    "has a behaviour with a distance below 0".into(),
                );
            }
            if *id == self.rules.person && kind.behaviour.is_some() {
                unsound(
                    entry,
                    "is the people's body, and has a creature's behaviour".into(),
                );
            }
        }
        // The widest body has to fit in every place: anyone may go anywhere.
        let widest = self.bodies.values().map(|k| k.radius).max().unwrap_or(0);
        for (id, place) in &self.places {
            let entry = format!("places/{id}");
            let b = place.bounds;
            if !b.is_ordered() || !b.is_near() {
                unsound(
                    entry.clone(),
                    format!(
                        "has bounds that aren't a south-west corner then a north-east one, \
                         within {} km of the middle",
                        crate::geom::FARTHEST / 1_000_000
                    ),
                );
            } else if b.max.x - b.min.x < 2 * widest || b.max.y - b.min.y < 2 * widest {
                unsound(
                    entry.clone(),
                    format!("is too small to hold a body {widest} mm across"),
                );
            }
            let mut inside = |what: &str, at: Vec2| {
                if !b.contains(at) {
                    unsound(
                        entry.clone(),
                        format!("has {what} at {at:?}, outside its bounds"),
                    );
                }
            };
            inside("its arrival", place.arrival);
            for t in &place.thresholds {
                inside("a threshold", t.at);
                inside("a threshold's exit", t.exit);
            }
            for lair in &place.lairs {
                inside("a lair", lair.at);
            }
            for t in &place.thresholds {
                if t.radius <= 0 {
                    unsound(entry.clone(), "has a threshold with no radius".into());
                }
            }
            let mut broken: Vec<&Shape> = Vec::new();
            for s in place.living.iter().chain(&place.lately) {
                if !s.is_whole() && !broken.contains(&s) {
                    broken.push(s);
                }
            }
            for s in broken {
                unsound(
                    entry.clone(),
                    format!(
                        "has a solid with no size, or a box whose corners are the wrong way \
                         round: {s:?}"
                    ),
                );
            }
            for lair in &place.lairs {
                match self.bodies.get(&lair.kind) {
                    None => unsound(entry.clone(), missing(format!("bodies/{}", lair.kind))),
                    Some(kind) if kind.behaviour.is_none() => unsound(
                        entry.clone(),
                        format!(
                            "has a lair of bodies/{}, which has no behaviour to hunt with",
                            lair.kind
                        ),
                    ),
                    Some(_) => {}
                }
            }
        }
        if wrong.is_empty() { Ok(()) } else { Err(wrong) }
    }

    pub fn body(&self, id: &Id<BodyKind>) -> &BodyKind {
        // Checked when the catalogue was built and when the world was loaded.
        &self.bodies[id]
    }

    pub fn place(&self, id: &Id<Place>) -> &Place {
        &self.places[id]
    }
}
