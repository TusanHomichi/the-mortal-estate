//! The first playable slice's story, played on the real map: walk up to the
//! old hall, meet the dun, die into Lately inside the hall as it was, find
//! the way out and over to the arch, and turn up again with a tardy.

use std::path::Path;
use std::sync::Arc;
use tme_sim::world::{Event, Input, Intent, PersonId, Settings};
use tme_sim::{Dir, Side, Vec2, World};

const ME: PersonId = PersonId(1);

fn world() -> World {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let content = tme_content::load(&dir).expect("content/ compiles");
    let mut w = World::new(Arc::new(content.catalogue), 1, Settings::default());
    w.apply(&Input::Join {
        person: ME,
        name: "Me".into(),
    })
    .unwrap();
    w
}

#[test]
fn every_model_the_content_names_is_in_the_assets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let content = tme_content::load(&root.join("content")).expect("content/ compiles");
    assert_eq!(
        tme_content::missing_models(&content, &root.join("assets/models")),
        Vec::<String>::new()
    );
}

fn pos(w: &World) -> Vec2 {
    w.person(ME).unwrap().body.at.pos
}

fn m(x: f64, y: f64) -> Vec2 {
    Vec2::new((x * 1000.0) as i32, (y * 1000.0) as i32)
}

/// Walk through `waypoints` in turn, giving up after `limit` steps. Every
/// event on the way.
fn walk(w: &mut World, waypoints: &[Vec2], limit: u32) -> Vec<Event> {
    let mut events = Vec::new();
    let mut next = 0;
    for _ in 0..limit {
        if next == waypoints.len() {
            break;
        }
        if pos(w).distance_sq(waypoints[next]) < 500 * 500 {
            next += 1;
            continue;
        }
        let _ = w.apply(&Input::Act {
            person: ME,
            intent: Intent::Move(Dir::toward(pos(w), waypoints[next])),
        });
        w.step();
        events.extend(w.take_events());
    }
    let _ = w.apply(&Input::Act {
        person: ME,
        intent: Intent::Move(Dir::NONE),
    });
    assert_eq!(
        next,
        waypoints.len(),
        "stuck at {:?} on the way to {:?}",
        pos(w),
        waypoints.get(next)
    );
    events
}

#[test]
fn die_in_the_hall_walk_out_of_its_past_and_turn_up_at_the_arch() {
    let mut w = world();
    // From the well, up past the rock to the hall's south side, and in
    // through the gap where its wall burned.
    walk(
        &mut w,
        &[m(-6.0, 3.0), m(6.0, 2.0), m(15.0, 2.0), m(15.0, 6.0)],
        2000,
    );
    // Stand and wait: the dun comes.
    let mut died = false;
    for _ in 0..2000 {
        w.step();
        if w.take_events()
            .iter()
            .any(|e| matches!(e, Event::WentToLately { person } if *person == ME))
        {
            died = true;
            break;
        }
    }
    assert!(died, "the dun kills whoever stands in its hall");
    assert_eq!(w.person(ME).unwrap().body.side, Side::Lately);
    // In Lately the hall is whole: out through its door, over to the arch.
    walk(
        &mut w,
        &[m(15.0, 6.0), m(15.0, 2.0), m(6.0, 8.0), m(2.0, 16.6)],
        3000,
    );
    w.apply(&Input::Act {
        person: ME,
        intent: Intent::Use,
    })
    .expect("the arch is a threshold");
    let mut tardy = None;
    for _ in 0..200 {
        w.step();
        for e in w.take_events() {
            if let Event::TurnedUp { tardy: t, .. } = e {
                tardy = Some(t);
            }
        }
    }
    let me = w.person(ME).unwrap();
    assert!(tardy.flatten().is_some(), "turning up leaves a tardy");
    assert_eq!(me.body.side, Side::Living);
    assert_eq!(me.tardies.len(), 1);
}

#[test]
fn the_dun_never_wanders_toward_somewhere_it_cannot_reach() {
    // Seed 35 once sent the dun's first wander into the hall's table, and
    // it pressed against the table's edge for good.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let content = tme_content::load(&dir).expect("content/ compiles");
    for seed in [35, 1, 2, 3] {
        let mut w = World::new(
            Arc::new(content.catalogue.clone()),
            seed,
            Settings::default(),
        );
        let mut held_since = (None, 0);
        for step in 0..(150 * tme_sim::STEPS_PER_SECOND) {
            w.step();
            let Some(dun) = w.state().creatures.values().next() else {
                continue;
            };
            if dun.roam_to != held_since.0 {
                held_since = (dun.roam_to, step);
            }
            assert!(
                dun.roam_to.is_none() || step - held_since.1 < 20 * tme_sim::STEPS_PER_SECOND,
                "seed {seed}: the dun has been going to {:?} for 20 s, and is at {:?}",
                dun.roam_to,
                dun.body.at.pos
            );
        }
    }
}

#[test]
fn the_hamlet_is_out_of_the_duns_reach() {
    let mut w = world();
    for _ in 0..(60 * tme_sim::STEPS_PER_SECOND) {
        w.step();
        assert!(
            !w.take_events()
                .iter()
                .any(|e| matches!(e, Event::Struck { .. })),
            "nobody is struck standing at the well"
        );
    }
}
