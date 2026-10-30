//! A world is a function of its seed and its inputs, so a snapshot and the
//! inputs since restore it exactly.

mod common;

use common::*;
use std::sync::Arc;
use tme_sim::world::{Input, Intent, Mismatch, Settings};
use tme_sim::{Dir, Id, World};

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
