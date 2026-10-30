//! From files to a checked catalogue and scenery.

use crate::files::{self, Point, Sides};
use crate::{Content, Problem, Problems};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tme_sim::catalogue::{
    Attack, Behaviour, BodyKind, Catalogue, Dodge, Effect, Lair, Mark, Place, Rules, Threshold,
    TurningUp, steps_from_ms,
};
use tme_sim::geom::{Mm, Rect, Shape, Vec2};
use tme_sim::{Id, STEPS_PER_SECOND, Side};

/// How a kind of body is drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub name: String,
    pub description: String,
    /// Under `assets/models/`.
    pub model: String,
    pub scale: f32,
}

/// What stands in a place, for a client to draw.
#[derive(Clone, Debug, PartialEq)]
pub struct Scenery {
    pub name: String,
    pub bounds: Rect,
    /// The ground's colour on each side, red, green and blue from 0 to 1.
    pub ground_living: [f32; 3],
    pub ground_lately: [f32; 3],
    pub props: Vec<Prop>,
}

/// One model standing somewhere.
#[derive(Clone, Debug, PartialEq)]
pub struct Prop {
    pub model: String,
    pub at: Vec2,
    /// Degrees anticlockwise, seen from above.
    pub turn: f32,
    pub scale: f32,
    pub sides: Sides,
    pub glow: bool,
}

/// Problems found so far, and the file they're being found in.
struct Report {
    root: PathBuf,
    problems: Vec<Problem>,
}

impl Report {
    fn add(&mut self, file: &Path, line: Option<usize>, message: impl Into<String>) {
        self.problems.push(Problem {
            file: file.strip_prefix(&self.root).unwrap_or(file).to_owned(),
            line,
            message: message.into(),
        });
    }

    /// Read and parse one file, noting what's wrong with it.
    fn parse<T: DeserializeOwned>(&mut self, path: &Path) -> Option<T> {
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                self.add(path, None, format!("can't be read: {e}"));
                return None;
            }
        };
        match toml::from_str(&text) {
            Ok(v) => Some(v),
            Err(e) => {
                let line = e.span().map(|s| text[..s.start].lines().count().max(1));
                let message = e.message().trim_end().to_owned();
                self.add(
                    path,
                    line,
                    format!("{message} (see the format in crates/content/src/files.rs)"),
                );
                None
            }
        }
    }

    /// Every `*.toml` in a folder, by id, in name order.
    fn folder<T: DeserializeOwned>(&mut self, dir: &Path) -> BTreeMap<String, (PathBuf, T)> {
        let mut out = BTreeMap::new();
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                self.add(dir, None, format!("can't be read: {e}"));
                return out;
            }
        };
        let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        paths.sort();
        for path in paths {
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                self.add(
                    &path,
                    None,
                    "isn't a .toml file: content is one entry per .toml file",
                );
                continue;
            }
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_owned();
            if id.is_empty()
                || !id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                self.add(
                    &path,
                    None,
                    "has a name that isn't lowercase letters, digits and hyphens; rename the file",
                );
                continue;
            }
            if let Some(v) = self.parse(&path) {
                out.insert(id, (path, v));
            }
        }
        out
    }
}

pub fn compile(dir: &Path) -> Result<Content, Problems> {
    let mut r = Report {
        root: dir.to_owned(),
        problems: Vec::new(),
    };
    let rules_path = dir.join("rules.toml");
    let rules: Option<files::Rules> = r.parse(&rules_path);
    let bodies: BTreeMap<String, (PathBuf, files::Body)> = r.folder(&dir.join("bodies"));
    let marks: BTreeMap<String, (PathBuf, files::Mark)> = r.folder(&dir.join("marks"));
    let places: BTreeMap<String, (PathBuf, files::Place)> = r.folder(&dir.join("places"));

    let mut catalogue_bodies = BTreeMap::new();
    let mut looks = BTreeMap::new();
    for (id, (path, b)) in &bodies {
        let (kind, look) = body(&mut r, path, b);
        catalogue_bodies.insert(Id::new(id), kind);
        looks.insert(Id::new(id), look);
    }
    let catalogue_marks = marks
        .iter()
        .map(|(id, (_, m))| (Id::new(id), mark(m)))
        .collect();
    let mut catalogue_places = BTreeMap::new();
    let mut scenery = BTreeMap::new();
    for (id, (path, p)) in &places {
        let (place, look) = place(&mut r, path, p);
        catalogue_places.insert(Id::new(id), place);
        scenery.insert(Id::new(id), look);
    }

    let Some(rules) = rules else {
        return Err(Problems(r.problems));
    };
    let catalogue = Catalogue {
        bodies: catalogue_bodies,
        marks: catalogue_marks,
        places: catalogue_places,
        rules: Rules {
            person: Id::new(&rules.person),
            arrival: Id::new(&rules.arrival),
            turning_up: TurningUp {
                base: steps_from_ms(rules.turning_up.base_ms),
                per_tardy: steps_from_ms(rules.turning_up.per_tardy_ms),
                max_tardies: rules.turning_up.max_tardies,
            },
        },
    };
    if let Err(e) = catalogue.check() {
        let file = if e.entry == "rules" {
            rules_path
        } else {
            dir.join(format!("{}.toml", e.entry))
        };
        r.add(&file, None, e.problem);
    }
    if r.problems.is_empty() {
        Ok(Content {
            catalogue,
            looks,
            scenery,
        })
    } else {
        Err(Problems(r.problems))
    }
}

fn mm(m: f64) -> Mm {
    (m * 1000.0).round() as Mm
}

fn point(p: Point) -> Vec2 {
    Vec2::new(mm(p[0]), mm(p[1]))
}

fn body(r: &mut Report, path: &Path, b: &files::Body) -> (BodyKind, Look) {
    if b.radius_m <= 0.0 {
        r.add(path, None, "radius_m must be above 0");
    }
    if b.health <= 0 {
        r.add(path, None, "health must be above 0");
    }
    if b.attack.wind_up_ms == 0 {
        r.add(
            path,
            None,
            "attack.wind_up_ms must be above 0: every swing winds up where it can be seen",
        );
    }
    if !(b.attack.arc_degrees > 0.0 && b.attack.arc_degrees <= 360.0) {
        r.add(
            path,
            None,
            "attack.arc_degrees must be above 0 and at most 360",
        );
    }
    let half = (b.attack.arc_degrees / 2.0).to_radians();
    let kind = BodyKind {
        name: b.name.clone(),
        radius: mm(b.radius_m),
        speed: mm(b.speed_m_per_s / f64::from(STEPS_PER_SECOND)),
        health: b.health,
        attack: Attack {
            wind_up: steps_from_ms(b.attack.wind_up_ms),
            recover: steps_from_ms(b.attack.recover_ms),
            reach: mm(b.attack.reach_m),
            arc_cos: (half.cos() * 1024.0).round() as i32,
            damage: b.attack.damage,
        },
        dodge: b.dodge.as_ref().map(|d| Dodge {
            distance: mm(d.distance_m),
            duration: steps_from_ms(d.duration_ms),
            recover: steps_from_ms(d.recover_ms),
        }),
        behaviour: b.behaviour.as_ref().map(|b| Behaviour {
            sense: mm(b.sense_m),
            leash: mm(b.leash_m),
            roam: mm(b.roam_m),
            respawn: steps_from_ms(b.respawn_ms),
        }),
    };
    let look = Look {
        name: b.name.clone(),
        description: b.description.clone(),
        model: b.model.clone(),
        scale: b.scale as f32,
    };
    (kind, look)
}

fn mark(m: &files::Mark) -> Mark {
    let e = &m.effects;
    let effects = e
        .max_health
        .map(Effect::MaxHealth)
        .into_iter()
        .chain(e.speed_percent.map(Effect::Speed))
        .chain(e.sees_lately.then_some(Effect::SeesLately))
        .collect();
    Mark {
        name: m.name.clone(),
        description: m.description.clone(),
        effects,
    }
}

fn side(s: files::Side) -> Side {
    match s {
        files::Side::Living => Side::Living,
        files::Side::Lately => Side::Lately,
    }
}

fn colour(r: &mut Report, path: &Path, what: &str, hex: &str) -> [f32; 3] {
    let parsed = hex
        .strip_prefix('#')
        .filter(|h| h.len() == 6)
        .and_then(|h| u32::from_str_radix(h, 16).ok());
    match parsed {
        Some(n) => [(n >> 16) & 0xff, (n >> 8) & 0xff, n & 0xff].map(|c| c as f32 / 255.0),
        None => {
            r.add(
                path,
                None,
                format!("ground.{what} must be a colour written #rrggbb, not {hex:?}"),
            );
            [0.0; 3]
        }
    }
}

fn place(r: &mut Report, path: &Path, p: &files::Place) -> (Place, Scenery) {
    let bounds = Rect {
        min: point(p.bounds[0]),
        max: point(p.bounds[1]),
    };
    if bounds.min.x >= bounds.max.x || bounds.min.y >= bounds.max.y {
        r.add(
            path,
            None,
            "bounds must be [south-west corner, north-east corner]",
        );
    }
    let inside = |r: &mut Report, what: &str, at: Point| {
        if !bounds.contains(point(at)) {
            r.add(
                path,
                None,
                format!("{what} at {at:?} is outside the bounds"),
            );
        }
    };
    inside(r, "arrival", p.arrival);
    for t in &p.thresholds {
        inside(r, "a threshold", t.at);
        inside(r, "a threshold's exit", t.exit);
    }
    for l in &p.lairs {
        inside(r, "a lair", l.at);
    }

    let (mut living, mut lately) = (Vec::new(), Vec::new());
    let mut solid = |sides: Sides, shape: Shape| {
        if sides != Sides::Lately {
            living.push(shape);
        }
        if sides != Sides::Living {
            lately.push(shape);
        }
    };
    let mut props = Vec::new();
    for prop in &p.props {
        let at = point(prop.at);
        match prop.solid {
            Some(files::Solid::Circle { radius_m }) => solid(
                prop.sides,
                Shape::Circle {
                    centre: at,
                    radius: mm(radius_m),
                },
            ),
            Some(files::Solid::Box { size_m }) => {
                let half = Vec2::new(mm(size_m[0] / 2.0), mm(size_m[1] / 2.0));
                solid(
                    prop.sides,
                    Shape::Box(Rect {
                        min: at - half,
                        max: at + half,
                    }),
                );
            }
            None => {}
        }
        props.push(Prop {
            model: prop.model.clone(),
            at,
            turn: prop.turn as f32,
            scale: prop.scale as f32,
            sides: prop.sides,
            glow: prop.glow,
        });
    }
    for run in &p.runs {
        let (from, to) = (point(run.from), point(run.to));
        let along_x = from.y == to.y;
        if !along_x && from.x != to.x {
            r.add(
                path,
                None,
                format!(
                    "a run from {:?} to {:?} isn't due east-west or north-south",
                    run.from, run.to
                ),
            );
            continue;
        }
        if run.every_m <= 0.0 || run.models.is_empty() {
            r.add(path, None, "a run needs models and every_m above 0");
            continue;
        }
        let length = (to - from).length();
        let every = i64::from(mm(run.every_m));
        let count = (length / every).max(1);
        let dir = (to - from).with_length(1000);
        for i in 0..count {
            let centre = every * i + every / 2;
            let offset = Vec2::new(
                (i64::from(dir.x) * centre / 1000) as Mm,
                (i64::from(dir.y) * centre / 1000) as Mm,
            );
            props.push(Prop {
                model: run.models[i as usize % run.models.len()].clone(),
                at: from + offset,
                turn: if along_x { 0.0 } else { 90.0 },
                scale: run.scale as f32,
                sides: run.sides,
                glow: false,
            });
        }
        if let Some(t) = run.thickness_m {
            let half = mm(t / 2.0);
            let (min, max) = (
                Vec2::new(from.x.min(to.x), from.y.min(to.y)),
                Vec2::new(from.x.max(to.x), from.y.max(to.y)),
            );
            let rect = if along_x {
                Rect {
                    min: Vec2::new(min.x, min.y - half),
                    max: Vec2::new(max.x, max.y + half),
                }
            } else {
                Rect {
                    min: Vec2::new(min.x - half, min.y),
                    max: Vec2::new(max.x + half, max.y),
                }
            };
            solid(run.sides, Shape::Box(rect));
        }
    }
    let place = Place {
        name: p.name.clone(),
        bounds,
        living,
        lately,
        arrival: point(p.arrival),
        thresholds: p
            .thresholds
            .iter()
            .map(|t| Threshold {
                at: point(t.at),
                radius: mm(t.radius_m),
                exit: point(t.exit),
            })
            .collect(),
        lairs: p
            .lairs
            .iter()
            .map(|l| Lair {
                kind: Id::new(&l.kind),
                at: point(l.at),
                side: side(l.side),
            })
            .collect(),
    };
    let scenery = Scenery {
        name: p.name.clone(),
        bounds,
        ground_living: colour(r, path, "living", &p.ground.living),
        ground_lately: colour(r, path, "lately", &p.ground.lately),
        props,
    };
    (place, scenery)
}
