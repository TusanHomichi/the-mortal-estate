//! The ground plane in whole millimetres, so every step works out the same on
//! every machine: the server, a replay of its journal and a client predicting
//! its own movement all get the same answer.

use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

/// A millimetre count. A metre is [`METRE`].
pub type Mm = i32;

/// A metre, in millimetres.
pub const METRE: Mm = 1000;

/// The farthest anything in a place can be from its middle, either way:
/// a hundred kilometres. Far enough for any place, and near enough that
/// distances squared never overflow.
pub const FARTHEST: Mm = 100_000 * METRE;

/// A point or offset on the ground plane: x east, y north, in millimetres.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: Mm,
    pub y: Mm,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0, y: 0 };

    pub const fn new(x: Mm, y: Mm) -> Vec2 {
        Vec2 { x, y }
    }

    pub fn length_sq(self) -> i64 {
        let (x, y) = (i64::from(self.x), i64::from(self.y));
        x * x + y * y
    }

    pub fn length(self) -> i64 {
        isqrt(self.length_sq())
    }

    pub fn dot(self, other: Vec2) -> i64 {
        i64::from(self.x) * i64::from(other.x) + i64::from(self.y) * i64::from(other.y)
    }

    /// This offset at `length` millimetres long, pointing the same way. Zero
    /// stays zero.
    pub fn with_length(self, length: Mm) -> Vec2 {
        let current = self.length();
        if current == 0 {
            return Vec2::ZERO;
        }
        let scale = |v: Mm| (i64::from(v) * i64::from(length) / current) as Mm;
        Vec2::new(scale(self.x), scale(self.y))
    }

    pub fn distance_sq(self, other: Vec2) -> i64 {
        (self - other).length_sq()
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }
}

/// The integer square root: the largest r with r * r <= n.
pub fn isqrt(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    let n = n as u64;
    let mut r = (n as f64).sqrt() as u64;
    // The float guess can be one off either way; settle it in integers.
    while r * r > n {
        r -= 1;
    }
    while (r + 1) * (r + 1) <= n {
        r += 1;
    }
    r as i64
}

/// A direction and how far it's pushed, as a stick or keys give it: a vector
/// no longer than [`Dir::UNIT`]. Full speed is a vector [`Dir::UNIT`] long;
/// a stick half over is half as long. One read from outside (a player's
/// input, a snapshot) is cut to length like any other.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "Pushed")]
pub struct Dir {
    x: i16,
    y: i16,
}

/// A direction as it's written, before it's cut to length.
#[derive(Deserialize)]
struct Pushed {
    x: i16,
    y: i16,
}

impl From<Pushed> for Dir {
    fn from(p: Pushed) -> Dir {
        Dir::new(p.x.into(), p.y.into())
    }
}

impl Dir {
    /// The length of a direction pushed all the way.
    pub const UNIT: i16 = 1024;
    pub const NONE: Dir = Dir { x: 0, y: 0 };
    pub const NORTH: Dir = Dir { x: 0, y: Dir::UNIT };

    /// The direction (x, y) points, no longer than [`Dir::UNIT`]: a longer
    /// one is cut to it.
    pub fn new(x: i32, y: i32) -> Dir {
        let v = Vec2::new(x, y);
        let v = if v.length() > i64::from(Dir::UNIT) {
            v.with_length(Mm::from(Dir::UNIT))
        } else {
            v
        };
        Dir {
            x: v.x as i16,
            y: v.y as i16,
        }
    }

    /// The direction from one point toward another, all the way pushed.
    pub fn toward(from: Vec2, to: Vec2) -> Dir {
        let v = (to - from).with_length(Mm::from(Dir::UNIT));
        Dir::new(v.x, v.y)
    }

    pub fn x(self) -> i16 {
        self.x
    }

    pub fn y(self) -> i16 {
        self.y
    }

    pub fn is_none(self) -> bool {
        self.x == 0 && self.y == 0
    }

    pub fn as_vec(self) -> Vec2 {
        Vec2::new(self.x.into(), self.y.into())
    }

    /// This direction pushed all the way, or `None` for no direction.
    pub fn full(self) -> Option<Dir> {
        (!self.is_none()).then(|| {
            let v = self.as_vec().with_length(Mm::from(Dir::UNIT));
            Dir::new(v.x, v.y)
        })
    }

    /// How far `speed` carries a body pushed this way in one step.
    pub fn travel(self, speed: Mm) -> Vec2 {
        let unit = i64::from(Dir::UNIT);
        let s = i64::from(speed);
        Vec2::new(
            (i64::from(self.x) * s / unit) as Mm,
            (i64::from(self.y) * s / unit) as Mm,
        )
    }
}

/// An axis-aligned rectangle, from its south-west corner to its north-east.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    /// Whether its corners are the right way round and it has some size.
    pub fn is_ordered(&self) -> bool {
        self.min.x < self.max.x && self.min.y < self.max.y
    }

    /// Whether it's all within [`FARTHEST`] of the middle.
    pub fn is_near(&self) -> bool {
        [self.min, self.max]
            .iter()
            .all(|p| p.x.abs() <= FARTHEST && p.y.abs() <= FARTHEST)
    }
}

/// Something solid a body can't pass through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shape {
    Circle { centre: Vec2, radius: Mm },
    Box(Rect),
}

impl Shape {
    /// Whether it's a shape at all: a circle with some radius, a box with
    /// its corners the right way round.
    pub fn is_whole(&self) -> bool {
        match self {
            Shape::Circle { radius, .. } => *radius > 0,
            Shape::Box(rect) => rect.is_ordered(),
        }
    }

    /// Whether the straight line from `a` to `b` passes through its inside.
    /// A line that only grazes its edge or a corner gets past.
    pub fn crosses(&self, a: Vec2, b: Vec2) -> bool {
        let d = b - a;
        match *self {
            Shape::Circle { centre, radius } => {
                let r = i128::from(radius);
                let off = centre - a;
                let along = i128::from(off.dot(d));
                let length = i128::from(d.length_sq());
                if along <= 0 || length == 0 {
                    // The nearest point of the line is `a`.
                    return i128::from(off.length_sq()) < r * r;
                }
                if along >= length {
                    // The nearest point is `b`.
                    return i128::from((centre - b).length_sq()) < r * r;
                }
                // Square of the distance from the centre to the line, times
                // its length squared.
                let cross =
                    i128::from(off.x) * i128::from(d.y) - i128::from(off.y) * i128::from(d.x);
                cross * cross < r * r * length
            }
            Shape::Box(rect) => {
                // Where along the line (a fraction, top over bottom) it's
                // inside the box on both axes at once.
                let (mut enter, mut leave) = ((0_i64, 1_i64), (1_i64, 1_i64));
                for (from, step, low, high) in [
                    (a.x, d.x, rect.min.x, rect.max.x),
                    (a.y, d.y, rect.min.y, rect.max.y),
                ] {
                    let (from, step, low, high) = (
                        i64::from(from),
                        i64::from(step),
                        i64::from(low),
                        i64::from(high),
                    );
                    if step == 0 {
                        if from <= low || from >= high {
                            return false;
                        }
                        continue;
                    }
                    let (near, far) = if step > 0 {
                        ((low - from, step), (high - from, step))
                    } else {
                        ((from - high, -step), (from - low, -step))
                    };
                    if near.0 * enter.1 > enter.0 * near.1 {
                        enter = near;
                    }
                    if far.0 * leave.1 < leave.0 * far.1 {
                        leave = far;
                    }
                }
                enter.0 * leave.1 < leave.0 * enter.1
            }
        }
    }

    /// Where a body of `radius` at `p` ends up once pushed out of this
    /// shape, the shortest way. A body that doesn't overlap it stays put.
    pub fn push_out(&self, p: Vec2, radius: Mm) -> Vec2 {
        match *self {
            Shape::Circle { centre, radius: r } => {
                let reach = i64::from(r + radius);
                let off = p - centre;
                if off.length_sq() >= reach * reach {
                    return p;
                }
                // Dead centre: out to the north, the same way every time.
                let away = if off == Vec2::ZERO {
                    Vec2::new(0, 1)
                } else {
                    off
                };
                centre + away.with_length(r + radius)
            }
            Shape::Box(rect) => {
                let nearest = Vec2::new(
                    p.x.clamp(rect.min.x, rect.max.x),
                    p.y.clamp(rect.min.y, rect.max.y),
                );
                let off = p - nearest;
                if off != Vec2::ZERO {
                    // Outside the box: push away from its nearest point.
                    if off.length_sq() >= i64::from(radius) * i64::from(radius) {
                        return p;
                    }
                    return nearest + off.with_length(radius);
                }
                // Inside the box: out through the nearest side.
                let sides = [
                    (p.x - rect.min.x, Vec2::new(rect.min.x - radius, p.y)),
                    (rect.max.x - p.x, Vec2::new(rect.max.x + radius, p.y)),
                    (p.y - rect.min.y, Vec2::new(p.x, rect.min.y - radius)),
                    (rect.max.y - p.y, Vec2::new(p.x, rect.max.y + radius)),
                ];
                sides
                    .into_iter()
                    .min_by_key(|(depth, _)| *depth)
                    .map(|(_, out)| out)
                    .unwrap_or(p)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isqrt_is_exact() {
        for n in [
            0,
            1,
            2,
            3,
            4,
            15,
            16,
            17,
            1 << 40,
            (1 << 40) + 1,
            i64::from(i32::MAX),
        ] {
            let r = isqrt(n);
            assert!(r * r <= n && (r + 1) * (r + 1) > n, "isqrt({n}) = {r}");
        }
    }

    #[test]
    fn a_direction_is_never_longer_than_a_full_push() {
        let d = Dir::new(5000, 5000);
        assert!(d.as_vec().length() <= i64::from(Dir::UNIT));
        assert!(d.as_vec().length() >= i64::from(Dir::UNIT) - 2);
        assert_eq!(Dir::new(100, 0).as_vec(), Vec2::new(100, 0));
    }

    #[test]
    fn a_direction_read_from_outside_is_cut_to_length() {
        use serde::Deserialize;
        use serde::de::value::{Error, SeqDeserializer};
        let read = |x: i16, y: i16| {
            Dir::deserialize(SeqDeserializer::<_, Error>::new([x, y].into_iter())).unwrap()
        };
        assert!(read(i16::MAX, 0).as_vec().length() <= i64::from(Dir::UNIT));
        assert_eq!(read(i16::MAX, 0).travel(200), Vec2::new(200, 0));
        // A partial push and no push at all come through as they were.
        assert_eq!(read(300, -400), Dir::new(300, -400));
        assert_eq!(read(0, 0), Dir::NONE);
    }

    #[test]
    fn a_line_through_a_solid_is_blocked_and_one_past_it_is_not() {
        let wall = Shape::Box(Rect {
            min: Vec2::new(0, -2000),
            max: Vec2::new(1000, 2000),
        });
        assert!(wall.crosses(Vec2::new(-1000, 0), Vec2::new(2000, 0)));
        assert!(wall.crosses(Vec2::new(2000, 500), Vec2::new(-1000, -500)));
        // Past its end, or stopping short of it.
        assert!(!wall.crosses(Vec2::new(-1000, 2500), Vec2::new(2000, 2500)));
        assert!(!wall.crosses(Vec2::new(-1000, 0), Vec2::new(-10, 0)));
        // Along its face, and through its corner exactly.
        assert!(!wall.crosses(Vec2::new(0, -3000), Vec2::new(0, 3000)));
        assert!(!wall.crosses(Vec2::new(-1000, 1000), Vec2::new(1000, 3000)));
        let post = Shape::Circle {
            centre: Vec2::ZERO,
            radius: 500,
        };
        assert!(post.crosses(Vec2::new(-2000, 0), Vec2::new(2000, 100)));
        assert!(!post.crosses(Vec2::new(-2000, 600), Vec2::new(2000, 600)));
        assert!(!post.crosses(Vec2::new(-2000, 0), Vec2::new(-600, 0)));
    }

    #[test]
    fn a_body_is_pushed_out_of_a_circle() {
        let post = Shape::Circle {
            centre: Vec2::ZERO,
            radius: 500,
        };
        let out = post.push_out(Vec2::new(300, 0), 400);
        assert_eq!(out, Vec2::new(900, 0));
        assert_eq!(post.push_out(Vec2::new(2000, 0), 400), Vec2::new(2000, 0));
    }

    #[test]
    fn a_body_is_pushed_out_of_a_box_by_its_nearest_side() {
        let wall = Shape::Box(Rect {
            min: Vec2::new(0, 0),
            max: Vec2::new(4000, 500),
        });
        // Inside, nearer the south side.
        assert_eq!(
            wall.push_out(Vec2::new(2000, 100), 400),
            Vec2::new(2000, -400)
        );
        // Overlapping its north face from outside.
        assert_eq!(
            wall.push_out(Vec2::new(2000, 700), 400),
            Vec2::new(2000, 900)
        );
        // Clear of it.
        assert_eq!(
            wall.push_out(Vec2::new(2000, 1000), 400),
            Vec2::new(2000, 1000)
        );
    }
}
