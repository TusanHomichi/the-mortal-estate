//! Finding a way: each side of each place laid out as a grid of half-metre
//! cells, free or blocked for a body of a given size, and the shortest way
//! across it. Creatures use it to come round walls instead of pressing
//! against them. Integer costs and fixed tie-breaks keep it deterministic.

use crate::catalogue::{Catalogue, Id, Place, Side};
use crate::geom::{Mm, Vec2};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

/// The side of a cell.
const CELL: Mm = 500;

/// The most cells a search looks at before it gives up.
const SEARCH_LIMIT: usize = 40_000;

/// One side of one place, for bodies of one radius.
pub struct Grid {
    origin: Vec2,
    width: i32,
    height: i32,
    blocked: Vec<bool>,
}

impl Grid {
    pub fn build(place: &Place, side: Side, radius: Mm) -> Grid {
        let b = place.bounds;
        let width = (b.max.x - b.min.x) / CELL + 1;
        let height = (b.max.y - b.min.y) / CELL + 1;
        let mut blocked = Vec::with_capacity((width * height) as usize);
        for cy in 0..height {
            for cx in 0..width {
                let c = Vec2::new(
                    b.min.x + cx * CELL + CELL / 2,
                    b.min.y + cy * CELL + CELL / 2,
                );
                let outside = c.x < b.min.x + radius
                    || c.x > b.max.x - radius
                    || c.y < b.min.y + radius
                    || c.y > b.max.y - radius;
                let solid = place
                    .solids(side)
                    .iter()
                    .any(|s| s.push_out(c, radius) != c);
                blocked.push(outside || solid);
            }
        }
        Grid {
            origin: b.min,
            width,
            height,
            blocked,
        }
    }

    fn cell(&self, p: Vec2) -> (i32, i32) {
        (
            ((p.x - self.origin.x) / CELL).clamp(0, self.width - 1),
            ((p.y - self.origin.y) / CELL).clamp(0, self.height - 1),
        )
    }

    fn centre(&self, (cx, cy): (i32, i32)) -> Vec2 {
        Vec2::new(
            self.origin.x + cx * CELL + CELL / 2,
            self.origin.y + cy * CELL + CELL / 2,
        )
    }

    fn free(&self, (cx, cy): (i32, i32)) -> bool {
        cx >= 0
            && cy >= 0
            && cx < self.width
            && cy < self.height
            && !self.blocked[(cy * self.width + cx) as usize]
    }

    /// Whether a body can walk straight from `a` to `b` without meeting a
    /// blocked cell.
    pub fn clear(&self, a: Vec2, b: Vec2) -> bool {
        let length = (b - a).length();
        let samples = (length / i64::from(CELL / 2)).max(1);
        (0..=samples).all(|i| {
            let p = a + Vec2::new(
                ((i64::from(b.x - a.x) * i) / samples) as Mm,
                ((i64::from(b.y - a.y) * i) / samples) as Mm,
            );
            self.free(self.cell(p))
        })
    }

    /// The nearest free cell to `p`, looking outward a few cells.
    fn nearest_free(&self, p: Vec2) -> Option<(i32, i32)> {
        let (cx, cy) = self.cell(p);
        (0..4).find_map(|r| {
            (-r..=r)
                .flat_map(|dy| (-r..=r).map(move |dx| (cx + dx, cy + dy)))
                .filter(|c| self.free(*c))
                .min_by_key(|c| (self.centre(*c).distance_sq(p), c.1, c.0))
        })
    }

    /// The shortest way from `from` to `to`, as points to walk through in
    /// turn, ending at `to`; `None` if there isn't one.
    pub fn path(&self, from: Vec2, to: Vec2) -> Option<Vec<Vec2>> {
        let start = self.nearest_free(from)?;
        let goal = self.nearest_free(to)?;
        let index = |(x, y): (i32, i32)| (y * self.width + x) as usize;
        let h = |(x, y): (i32, i32)| {
            let (dx, dy) = ((x - goal.0).abs(), (y - goal.1).abs());
            10 * (dx + dy) - 6 * dx.min(dy)
        };
        let mut cost = vec![i32::MAX; self.blocked.len()];
        let mut came: Vec<Option<(i32, i32)>> = vec![None; self.blocked.len()];
        let mut open = BinaryHeap::new();
        cost[index(start)] = 0;
        open.push(Reverse((h(start), 0, start.1, start.0)));
        let mut looked = 0;
        while let Some(Reverse((_, g, y, x))) = open.pop() {
            let c = (x, y);
            if c == goal {
                break;
            }
            if g > cost[index(c)] {
                continue;
            }
            looked += 1;
            if looked > SEARCH_LIMIT {
                return None;
            }
            for (dx, dy) in [
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ] {
                let n = (x + dx, y + dy);
                // Diagonals only where both sides are open: no cutting corners.
                if !self.free(n)
                    || (dx != 0 && dy != 0 && !(self.free((x + dx, y)) && self.free((x, y + dy))))
                {
                    continue;
                }
                let step = if dx != 0 && dy != 0 { 14 } else { 10 };
                let ng = g + step;
                if ng < cost[index(n)] {
                    cost[index(n)] = ng;
                    came[index(n)] = Some(c);
                    open.push(Reverse((ng + h(n), ng, n.1, n.0)));
                }
            }
        }
        if cost[index(goal)] == i32::MAX {
            return None;
        }
        let mut cells = vec![goal];
        while let Some(prev) = came[index(*cells.last().expect("not empty"))] {
            cells.push(prev);
        }
        cells.reverse();
        let mut points: Vec<Vec2> = cells.into_iter().skip(1).map(|c| self.centre(c)).collect();
        if let Some(last) = points.last_mut() {
            *last = to;
        } else {
            points.push(to);
        }
        Some(points)
    }
}

/// Grids for every side of every place a body of some radius has needed,
/// built when first asked for. Derived from the catalogue alone, so never
/// saved.
#[derive(Default)]
pub struct Nav {
    grids: BTreeMap<(Id<Place>, Side, Mm), Grid>,
}

impl Nav {
    pub fn grid(
        &mut self,
        catalogue: &Catalogue,
        place: &Id<Place>,
        side: Side,
        radius: Mm,
    ) -> &Grid {
        self.grids
            .entry((place.clone(), side, radius))
            .or_insert_with(|| Grid::build(catalogue.place(place), side, radius))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{Rect, Shape};

    fn field() -> Place {
        Place {
            name: "Field".into(),
            bounds: Rect {
                min: Vec2::new(-10_000, -10_000),
                max: Vec2::new(10_000, 10_000),
            },
            // A wall from the south edge most of the way north.
            living: vec![Shape::Box(Rect {
                min: Vec2::new(-500, -10_000),
                max: Vec2::new(500, 6000),
            })],
            lately: vec![],
            arrival: Vec2::ZERO,
            thresholds: vec![],
            lairs: vec![],
        }
    }

    #[test]
    fn the_way_round_a_wall_goes_round_its_end() {
        let grid = Grid::build(&field(), Side::Living, 400);
        let (a, b) = (Vec2::new(-3000, 0), Vec2::new(3000, 0));
        assert!(!grid.clear(a, b));
        let path = grid.path(a, b).expect("a way round");
        assert_eq!(*path.last().unwrap(), b);
        assert!(
            path.iter().any(|p| p.y > 6000),
            "it goes round the north end"
        );
        for pair in path.windows(2) {
            assert!(grid.clear(pair[0], pair[1]));
        }
        // The other side of the same place has no wall.
        let lately = Grid::build(&field(), Side::Lately, 400);
        assert!(lately.clear(a, b));
    }

    #[test]
    fn there_is_no_way_into_a_closed_room() {
        let mut place = field();
        place.living = vec![Shape::Box(Rect {
            min: Vec2::new(-10_000, -500),
            max: Vec2::new(10_000, 500),
        })];
        let grid = Grid::build(&place, Side::Living, 400);
        assert_eq!(grid.path(Vec2::new(0, -5000), Vec2::new(0, 5000)), None);
    }
}
