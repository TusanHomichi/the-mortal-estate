//! Free movement, and committed actions.

mod common;

use common::*;
use std::sync::Arc;
use tme_sim::geom::{Rect, Shape};
use tme_sim::world::{Event, Input, Intent, Presence, Settings};
use tme_sim::{BodyId, Dir, Id, Refused, Side, Vec2, World};

#[test]
fn a_held_direction_walks_at_the_bodys_speed() {
    let mut w = world();
    act(&mut w, ALICE, Intent::Move(Dir::NORTH)).unwrap();
    run(&mut w, 10);
    assert_eq!(pos(&w, ALICE), Vec2::new(0, 10 * 200));
    // Half a push, half the pace.
    act(&mut w, ALICE, Intent::Move(Dir::new(0, -512))).unwrap();
    run(&mut w, 10);
    assert_eq!(pos(&w, ALICE), Vec2::new(0, 10 * 200 - 10 * 100));
}

#[test]
fn walls_stop_you_and_each_side_has_its_own() {
    let mut w = world();
    // The living land has a wall whose east face is 5m west.
    act(&mut w, ALICE, Intent::Move(west())).unwrap();
    run(&mut w, 60);
    assert_eq!(pos(&w, ALICE).x, -5000 + 400);
    // East is open in the living land: Lately's wall isn't here.
    act(&mut w, ALICE, Intent::Move(east())).unwrap();
    run(&mut w, 60);
    assert!(pos(&w, ALICE).x > 5000);
}

#[test]
fn a_dodge_cannot_carry_you_through_a_wall() {
    let mut w = world();
    place(&mut w, ALICE, Vec2::new(-3500, 0));
    act(&mut w, ALICE, Intent::Dodge(west())).unwrap();
    run(&mut w, 20);
    assert_eq!(pos(&w, ALICE).x, -5000 + 400);
}

#[test]
fn an_attack_commits_you_until_it_recovers() {
    let mut w = world();
    act(&mut w, ALICE, Intent::Attack(east())).unwrap();
    // Busy input is refused, not queued.
    assert_eq!(
        act(&mut w, ALICE, Intent::Attack(east())),
        Err(Refused::Committed)
    );
    assert_eq!(act(&mut w, ALICE, Intent::Use), Err(Refused::Committed));
    // Steering is held but not walked while the swing is under way.
    act(&mut w, ALICE, Intent::Move(Dir::NORTH)).unwrap();
    run(&mut w, 11);
    assert_eq!(pos(&w, ALICE), Vec2::ZERO);
    assert_eq!(
        act(&mut w, ALICE, Intent::Attack(east())),
        Err(Refused::Committed)
    );
    // Recovered: the held direction walks again, and a new swing is allowed.
    run(&mut w, 2);
    assert!(pos(&w, ALICE).y > 0);
    act(&mut w, ALICE, Intent::Attack(east())).unwrap();
}

#[test]
fn a_dodge_is_the_one_way_out_of_a_wind_up() {
    let mut w = world();
    act(&mut w, ALICE, Intent::Attack(east())).unwrap();
    run(&mut w, 2);
    act(&mut w, ALICE, Intent::Dodge(Dir::NORTH)).unwrap();
    let events = run(&mut w, 6);
    assert_eq!(pos(&w, ALICE), Vec2::new(0, 3000));
    // The swing never landed: nothing was struck, and the dodge commits too.
    assert!(!events.iter().any(|e| matches!(e, Event::Struck { .. })));
    assert_eq!(
        act(&mut w, ALICE, Intent::Dodge(Dir::NORTH)),
        Err(Refused::Committed)
    );
    // But not out of a recovery.
    run(&mut w, 10);
    act(&mut w, ALICE, Intent::Attack(east())).unwrap();
    run(&mut w, 7);
    assert_eq!(
        act(&mut w, ALICE, Intent::Dodge(Dir::NORTH)),
        Err(Refused::Committed)
    );
}

#[test]
fn leaving_keeps_you_where_you_were() {
    let mut w = world();
    act(&mut w, ALICE, Intent::Move(Dir::NORTH)).unwrap();
    run(&mut w, 5);
    let at = pos(&w, ALICE);
    w.apply(&tme_sim::Input::Leave { person: ALICE }).unwrap();
    assert_eq!(
        act(&mut w, ALICE, Intent::Move(Dir::NORTH)),
        Err(Refused::NotHere)
    );
    run(&mut w, 20);
    assert_eq!(pos(&w, ALICE), at);
    join(&mut w, ALICE);
    assert_eq!(pos(&w, ALICE), at);
    assert_eq!(w.person(ALICE).unwrap().body.side, Side::Living);
    assert_eq!(
        w.apply(&tme_sim::Input::Join {
            person: ALICE,
            name: "again".into()
        }),
        Err(Refused::AlreadyHere)
    );
}

#[test]
fn people_cannot_hurt_each_other_unless_the_host_allows_it() {
    for harm in [false, true] {
        let mut w = tme_sim::World::new(
            std::sync::Arc::new(catalogue()),
            1,
            tme_sim::world::Settings {
                harm_between_players: harm,
            },
        );
        join(&mut w, ALICE);
        join(&mut w, BOB);
        // Bob a step to Alice's east, inside her reach.
        act(&mut w, BOB, Intent::Move(east())).unwrap();
        run(&mut w, 4);
        act(&mut w, BOB, Intent::Move(Dir::NONE)).unwrap();
        act(&mut w, ALICE, Intent::Attack(east())).unwrap();
        let events = run(&mut w, 12);
        let struck = events
            .iter()
            .any(|e| matches!(e, Event::Struck { target, .. } if *target == BodyId::Person(BOB)));
        assert_eq!(struck, harm, "harm between players: {harm}");
    }
}

#[test]
fn leaving_is_no_way_out_of_a_swing() {
    let mut w = harmful(catalogue());
    join(&mut w, BOB);
    place(&mut w, BOB, Vec2::new(1200, 0));
    let began = w.step_count();
    act(&mut w, ALICE, Intent::Attack(east())).unwrap();
    run(&mut w, 2);
    w.apply(&Input::Leave { person: ALICE }).unwrap();
    // Gone as far as her player goes; her body sees the swing through.
    assert_eq!(
        act(&mut w, ALICE, Intent::Attack(east())),
        Err(Refused::NotHere)
    );
    let events = run(&mut w, 4);
    assert!(events.iter().any(|e| matches!(e, Event::Struck { .. })));
    assert!(!events.iter().any(|e| matches!(e, Event::Left { .. })));
    // Back on the step it landed: still recovering, no second swing.
    join(&mut w, ALICE);
    assert_eq!(
        act(&mut w, ALICE, Intent::Attack(east())),
        Err(Refused::Committed)
    );
    // Leaving again, she goes once the recovery is over, and not before.
    w.apply(&Input::Leave { person: ALICE }).unwrap();
    let left = until(
        &mut w,
        20,
        |e| matches!(e, Event::Left { person } if *person == ALICE),
    );
    assert_eq!(left, Some(began + 12), "a swing takes 12 steps");
    assert_eq!(w.person(ALICE).unwrap().presence, Presence::Away);
    join(&mut w, ALICE);
    act(&mut w, ALICE, Intent::Attack(east())).unwrap();
}

#[test]
fn leaving_mid_dodge_finishes_the_dash() {
    let mut w = world();
    act(&mut w, ALICE, Intent::Dodge(Dir::NORTH)).unwrap();
    w.apply(&Input::Leave { person: ALICE }).unwrap();
    run(&mut w, 20);
    assert_eq!(pos(&w, ALICE), Vec2::new(0, 3000));
    assert_eq!(w.person(ALICE).unwrap().presence, Presence::Away);
}

/// A world on `catalogue` where people can hurt each other, Alice in it.
fn harmful(catalogue: tme_sim::Catalogue) -> World {
    let mut w = World::new(
        Arc::new(catalogue),
        1,
        Settings {
            harm_between_players: true,
        },
    );
    join(&mut w, ALICE);
    w
}

#[test]
fn a_swing_does_not_go_through_a_wall_but_does_round_its_end() {
    let mut c = catalogue();
    // A thin wall in the living land, from 1m north to 5m north.
    c.places
        .get_mut(&Id::new("field"))
        .unwrap()
        .living
        .push(Shape::Box(Rect {
            min: Vec2::new(0, 1000),
            max: Vec2::new(200, 5000),
        }));
    let mut w = harmful(c);
    join(&mut w, BOB);
    // Either side of it, well within reach of each other.
    place(&mut w, ALICE, Vec2::new(-500, 0));
    place(&mut w, ALICE, Vec2::new(-500, 3000));
    place(&mut w, BOB, Vec2::new(700, 0));
    place(&mut w, BOB, Vec2::new(700, 3000));
    let struck = |events: &[Event]| {
        events
            .iter()
            .any(|e| matches!(e, Event::Struck { target, .. } if *target == BodyId::Person(BOB)))
    };
    act(&mut w, ALICE, Intent::Attack(east())).unwrap();
    assert!(!struck(&run(&mut w, 12)), "the wall is in the way");
    // Past its north end, the same distance apart, nothing between.
    place(&mut w, ALICE, Vec2::new(-500, 6000));
    place(&mut w, BOB, Vec2::new(700, 6000));
    act(&mut w, ALICE, Intent::Attack(east())).unwrap();
    assert!(struck(&run(&mut w, 12)));
}
