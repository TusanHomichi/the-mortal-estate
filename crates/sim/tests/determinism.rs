//! A world is a function of its seed and its inputs, so a snapshot and the
//! inputs since restore it exactly.

mod common;

use common::*;
use std::sync::Arc;
use tme_sim::body::{Action, Doing};
use tme_sim::catalogue::Lair;
use tme_sim::world::{Input, Intent, Mismatch, Settings};
use tme_sim::{Dir, Id, Side, Vec2, World};

/// A busy few hundred steps: two people wandering and fighting, the beast
/// hunting, deaths, a return.
fn script() -> Vec<(u64, Input)> {
    let mut inputs = vec![
        (
            0,
            Input::Join {
                person: ALICE,
                name: "Alice".into(),
            },
        ),
        (
            3,
            Input::Join {
                person: BOB,
                name: "Bob".into(),
            },
        ),
    ];
    for i in 0..60u64 {
        let who = if i % 2 == 0 { ALICE } else { BOB };
        let dir = Dir::new(
            (i as i32 * 37 % 200) - 100 + 400,
            (i as i32 * 91 % 200) - 100 + 400,
        );
        let intent = match i % 5 {
            0 => Intent::Attack(dir),
            1 => Intent::Dodge(dir),
            _ => Intent::Move(dir),
        };
        inputs.push((
            10 + i * 9,
            Input::Act {
                person: who,
                intent,
            },
        ));
    }
    inputs.push((300, Input::Leave { person: BOB }));
    inputs
}

/// Run `inputs` from `world` up to step `end`, ignoring refusals as the
/// server does (a refused input changes nothing and isn't journalled).
fn play(world: &mut World, inputs: &[(u64, Input)], end: u64) {
    for (at, input) in inputs {
        while world.step_count() < *at {
            world.step();
        }
        let _ = world.apply(input);
    }
    while world.step_count() < end {
        world.step();
    }
}

#[test]
fn the_same_seed_and_inputs_make_the_same_world() {
    let new = || World::new(Arc::new(catalogue()), 7, Settings::default());
    let (mut a, mut b) = (new(), new());
    play(&mut a, &script(), 800);
    play(&mut b, &script(), 800);
    assert_eq!(a.state(), b.state());
    assert!(a.state().creatures.len() + a.state().people.len() >= 3);
}

#[test]
fn a_snapshot_plus_the_inputs_since_restores_the_world() {
    let script = script();
    let mut whole = World::new(Arc::new(catalogue()), 7, Settings::default());
    play(&mut whole, &script, 800);

    let mut first = World::new(Arc::new(catalogue()), 7, Settings::default());
    let (before, after): (Vec<_>, Vec<_>) = script.iter().cloned().partition(|(at, _)| *at < 200);
    play(&mut first, &before, 200);
    let snapshot = first.state().clone();

    let mut restored = World::load(Arc::new(catalogue()), snapshot).expect("fits");
    play(&mut restored, &after, 800);
    assert_eq!(restored.state(), whole.state());
}

#[test]
fn a_saved_world_must_fit_its_content() {
    let mut w = world();
    let mut state = w.state().clone();
    state
        .people
        .get_mut(&ALICE)
        .unwrap()
        .tardies
        .push(Id::new("no-such-mark"));
    assert_eq!(
        World::load(Arc::new(catalogue()), state).err(),
        Some(Mismatch("no mark no-such-mark".into()))
    );
    w.step();
    let mut fewer = catalogue();
    fewer.places.values_mut().for_each(|p| p.lairs.clear());
    assert!(World::load(Arc::new(fewer), w.state().clone()).is_err());
}

#[test]
fn what_a_saved_body_is_in_the_middle_of_must_fit_its_content_too() {
    let mut w = world();
    act(&mut w, ALICE, Intent::Dodge(Dir::NORTH)).unwrap();
    let dodging = w.state().clone();
    let mut no_dodge = catalogue();
    no_dodge.bodies.get_mut(&Id::new("person")).unwrap().dodge = None;
    assert!(World::load(Arc::new(no_dodge), dodging).is_err());

    let mut rite = w.state().clone();
    rite.people.get_mut(&ALICE).unwrap().body.action = Some(Action {
        doing: Doing::TurnUp {
            threshold: 0,
            length: 40,
        },
        started: 0,
    });
    let mut no_threshold = catalogue();
    no_threshold
        .places
        .values_mut()
        .for_each(|p| p.thresholds.clear());
    assert!(World::load(Arc::new(no_threshold), rite).is_err());

    // A creature whose kind can no longer hunt.
    w.step();
    assert_eq!(w.state().creatures.len(), 1);
    let mut tame = catalogue();
    tame.bodies.get_mut(&Id::new("beast")).unwrap().behaviour = None;
    assert!(World::load(Arc::new(tame), w.state().clone()).is_err());
}

#[test]
fn a_lair_added_since_the_world_was_saved_does_not_fit() {
    let w = world();
    let mut more = catalogue();
    more.places.values_mut().for_each(|p| {
        p.lairs.push(Lair {
            kind: Id::new("beast"),
            at: Vec2::new(-10_000, 10_000),
            side: Side::Living,
        })
    });
    assert!(World::load(Arc::new(more), w.state().clone()).is_err());
}
