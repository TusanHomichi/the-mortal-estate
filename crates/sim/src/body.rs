//! Bodies: where they stand, which side they're on, what they're committed
//! to, and how they walk. Walking is the code a client runs to predict its
//! own movement, so it's here, free of everything else in the world.

use crate::catalogue::{BodyKind, Effect, Id, Mark, Place, Side};
use crate::geom::{Dir, Mm, Vec2};
use crate::{Steps, catalogue::Catalogue};
use serde::{Deserialize, Serialize};

/// A full address: the place, the level within it and the spot on that
/// level. Never just a name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    pub place: Id<Place>,
    pub level: i8,
    pub pos: Vec2,
}

/// A body in the world, a person's or a creature's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Body {
    pub kind: Id<BodyKind>,
    pub at: Address,
    pub side: Side,
    /// Which way it faces: always pushed all the way.
    pub facing: Dir,
    pub health: i32,
    /// The way it's being steered: a held stick or keys, or a creature's
    /// mind. It's only walked when nothing is under way.
    pub held: Dir,
    /// What it's committed to, if anything.
    pub action: Option<Action>,
}

/// Something a body has started and is in until it's done.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    pub doing: Doing,
    /// The step it began on.
    pub started: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Doing {
    /// A swing toward `facing`: winds up, lands, recovers.
    Attack,
    /// A dash, the only way out of a wind-up.
    Dodge { dir: Dir },
    /// The rite at a threshold that brings the dead back; it holds them for
    /// `length` steps.
    TurnUp { threshold: usize, length: Steps },
}

/// Where in its action a body is, as anyone watching would see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    WindUp,
    Recover,
    Dashing,
    Rite,
}

impl Body {
    /// Where its action is on step `now`.
    pub fn phase(&self, kind: &BodyKind, now: u64) -> Option<Phase> {
        let action = self.action.as_ref()?;
        let into = now.saturating_sub(action.started);
        Some(match action.doing {
            Doing::Attack if into < u64::from(kind.attack.wind_up) => Phase::WindUp,
            Doing::Attack => Phase::Recover,
            Doing::Dodge { .. } => {
                let duration = kind.dodge.as_ref().map_or(0, |d| d.duration);
                if into < u64::from(duration) {
                    Phase::Dashing
                } else {
                    Phase::Recover
                }
            }
            Doing::TurnUp { .. } => Phase::Rite,
        })
    }
}

/// How far `travel` carries a body of `radius` from `from` on `side` of
/// `place`: blocked by what's solid there and kept inside the bounds. Long
/// moves go in short pieces, so nothing passes through a thin wall.
pub fn walk(place: &Place, side: Side, from: Vec2, radius: Mm, travel: Vec2) -> Vec2 {
    let longest = i64::from((radius / 2).max(1));
    let pieces = (travel.length() / longest + 1) as i32;
    let mut pos = from;
    let mut done = Vec2::ZERO;
    for i in 1..=pieces {
        let target = Vec2::new(travel.x * i / pieces, travel.y * i / pieces);
        pos = pos + (target - done);
        done = target;
        pos = settle(place, side, pos, radius);
    }
    pos
}

/// A body at `pos` pushed clear of everything solid and back inside the
/// bounds. A few passes, so a corner between two solids lets go too.
pub fn settle(place: &Place, side: Side, mut pos: Vec2, radius: Mm) -> Vec2 {
    for _ in 0..3 {
        for solid in place.solids(side) {
            pos = solid.push_out(pos, radius);
        }
        let b = place.bounds;
        pos.x = pos.x.clamp(b.min.x + radius, b.max.x - radius);
        pos.y = pos.y.clamp(b.min.y + radius, b.max.y - radius);
    }
    pos
}

/// A person's most health: their body's, changed by their tardies.
pub fn max_health(catalogue: &Catalogue, kind: &Id<BodyKind>, tardies: &[Id<Mark>]) -> i32 {
    let base = catalogue.body(kind).health;
    let change: i32 = effects(catalogue, tardies)
        .filter_map(|e| match e {
            Effect::MaxHealth(n) => Some(n),
            _ => None,
        })
        .sum();
    (base + change).max(1)
}

/// How far a person walks in a step at a full push: their body's speed,
/// changed by their tardies.
pub fn speed(catalogue: &Catalogue, kind: &Id<BodyKind>, tardies: &[Id<Mark>]) -> Mm {
    let base = catalogue.body(kind).speed;
    let percent: i32 = effects(catalogue, tardies)
        .filter_map(|e| match e {
            Effect::Speed(n) => Some(n),
            _ => None,
        })
        .sum();
    (base * (100 + percent).max(10) / 100).max(1)
}

/// Whether a life's tardies let it see the dead clearly.
pub fn sees_lately(catalogue: &Catalogue, tardies: &[Id<Mark>]) -> bool {
    effects(catalogue, tardies).any(|e| e == Effect::SeesLately)
}

fn effects<'a>(
    catalogue: &'a Catalogue,
    tardies: &'a [Id<Mark>],
) -> impl Iterator<Item = Effect> + 'a {
    tardies
        .iter()
        .filter_map(|t| catalogue.marks.get(t))
        .flat_map(|m| m.effects.iter().copied())
}
