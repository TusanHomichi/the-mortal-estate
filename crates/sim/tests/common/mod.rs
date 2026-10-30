//! A small world for scenarios: one place, a person, a beast and a
//! threshold, with numbers picked to make the tests easy to read, not from
//! `content/`.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::Arc;
use tme_sim::catalogue::{
    Attack, Behaviour, BodyKind, Catalogue, Dodge, Effect, Lair, Mark, Place, Rules, Threshold,
    TurningUp,
};
use tme_sim::geom::{Rect, Shape, Vec2};
use tme_sim::world::{Event, Input, Intent, PersonId, Settings};
use tme_sim::{Dir, Id, Side, World};

pub const ALICE: PersonId = PersonId(1);
pub const BOB: PersonId = PersonId(2);

/// Where the beast's lair is.
pub const LAIR: Vec2 = Vec2::new(10_000, 10_000);
/// The threshold, and where it lets you out.
pub const THRESHOLD: Vec2 = Vec2::new(0, -10_000);
pub const EXIT: Vec2 = Vec2::new(0, -12_000);

pub fn person() -> BodyKind {
    BodyKind {
        name: "Person".into(),
        radius: 400,
        speed: 200,
        health: 30,
        attack: Attack {
            wind_up: 6,
            recover: 6,
            reach: 900,
            arc_cos: 500,
            damage: 10,
        },
        dodge: Some(Dodge {
            distance: 3000,
            duration: 6,
            recover: 4,
        }),
        behaviour: None,
    }
}

pub fn beast() -> BodyKind {
    BodyKind {
        name: "Beast".into(),
        radius: 500,
        speed: 150,
        health: 20,
        attack: Attack {
            wind_up: 16,
            recover: 20,
            reach: 800,
            arc_cos: 700,
            damage: 12,
        },
        dodge: None,
        behaviour: Some(Behaviour {
            sense: 8000,
            leash: 15_000,
            roam: 2000,
            respawn: 200,
        }),
    }
}

pub fn catalogue() -> Catalogue {
    let mut bodies = BTreeMap::new();
    bodies.insert(Id::new("person"), person());
    bodies.insert(Id::new("beast"), beast());
    let mark = |name: &str, effects| Mark {
        name: name.into(),
        description: String::new(),
        effects,
    };
    let mut marks = BTreeMap::new();
    marks.insert(Id::new("pale"), mark("Pale", vec![Effect::MaxHealth(-5)]));
    marks.insert(
        Id::new("light-foot"),
        mark("Light Foot", vec![Effect::Speed(10)]),
    );
    marks.insert(
        Id::new("second-sight"),
        mark("Second Sight", vec![Effect::SeesLately]),
    );
    let mut places = BTreeMap::new();
    places.insert(
        Id::new("field"),
        Place {
            name: "Field".into(),
            bounds: Rect {
                min: Vec2::new(-20_000, -20_000),
                max: Vec2::new(20_000, 20_000),
            },
            // A wall to the west that's only in the living land, and one to
            // the east that's only in Lately.
            living: vec![wall(-5000)],
            lately: vec![wall(5000)],
            arrival: Vec2::ZERO,
            thresholds: vec![Threshold {
                at: THRESHOLD,
                radius: 1500,
                exit: EXIT,
            }],
            lairs: vec![Lair {
                kind: Id::new("beast"),
                at: LAIR,
                side: Side::Living,
            }],
        },
    );
    let catalogue = Catalogue {
        bodies,
        marks,
        places,
        rules: Rules {
            person: Id::new("person"),
            arrival: Id::new("field"),
            turning_up: TurningUp {
                base: 40,
                per_tardy: 20,
                max_tardies: 2,
            },
        },
    };
    catalogue.check().expect("the fixture is whole");
    catalogue
}

/// A north-south wall a metre thick and six long, whose east face is at
/// `x`, across the middle of the field.
fn wall(x: i32) -> Shape {
    Shape::Box(Rect {
        min: Vec2::new(x - 1000, -3000),
        max: Vec2::new(x, 3000),
    })
}

/// A world with the beast's lair far from everyone, and Alice in it.
pub fn world() -> World {
    let mut w = World::new(Arc::new(catalogue()), 42, Settings::default());
    join(&mut w, ALICE);
    w
}

pub fn join(w: &mut World, who: PersonId) {
    w.apply(&Input::Join {
        person: who,
        name: format!("{who:?}"),
    })
    .expect("joins");
}

pub fn act(w: &mut World, who: PersonId, intent: Intent) -> Result<(), tme_sim::Refused> {
    w.apply(&Input::Act {
        person: who,
        intent,
    })
}

/// Step `n` times, collecting what happened.
pub fn run(w: &mut World, n: u32) -> Vec<Event> {
    let mut events = Vec::new();
    for _ in 0..n {
        w.step();
        events.extend(w.take_events());
    }
    events
}

/// Step until `done` says an event is the one, at most `limit` steps: the
/// step count it happened on.
pub fn until(w: &mut World, limit: u32, done: impl Fn(&Event) -> bool) -> Option<u64> {
    for _ in 0..limit {
        w.step();
        if w.take_events().iter().any(&done) {
            return Some(w.step_count());
        }
    }
    None
}

pub fn pos(w: &World, who: PersonId) -> Vec2 {
    w.person(who).expect("here").body.at.pos
}

pub fn east() -> Dir {
    Dir::new(i32::from(Dir::UNIT), 0)
}

pub fn west() -> Dir {
    Dir::new(-i32::from(Dir::UNIT), 0)
}

/// Put someone straight onto a spot, as a test world can.
pub fn place(w: &mut World, who: PersonId, at: Vec2) {
    // Walk there: the shortest honest way, no teleporting in the rules.
    let start = pos(w, who);
    let to = at - start;
    let steps = (to.length() / 200 + 1) as u32;
    act(w, who, Intent::Move(Dir::toward(start, at))).expect("walks");
    for _ in 0..steps {
        if pos(w, who).distance_sq(at) <= 200 * 200 {
            break;
        }
        act(w, who, Intent::Move(Dir::toward(pos(w, who), at))).expect("walks");
        w.step();
    }
    act(w, who, Intent::Move(Dir::NONE)).expect("stops");
    w.take_events();
}
