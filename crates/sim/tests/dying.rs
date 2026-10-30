//! The beast, dying into Lately, and turning up again.

mod common;

use common::*;
use tme_sim::world::{Event, Intent};
use tme_sim::{BodyId, Dir, Refused, Side, Vec2};

/// Alice walks to within the beast's sense and waits there.
fn alice_near_the_beast() -> tme_sim::World {
    let mut w = world();
    place(&mut w, ALICE, LAIR - Vec2::new(4000, 0));
    w
}

#[test]
fn the_beast_shows_its_wind_up_before_it_lands() {
    let mut w = alice_near_the_beast();
    let began = until(&mut w, 400, |e| {
        matches!(
            e,
            Event::Began {
                who: BodyId::Creature(_),
                ..
            }
        )
    })
    .expect("the beast swings");
    let landed = until(&mut w, 100, |e| matches!(e, Event::Struck { .. })).expect("and lands");
    assert_eq!(landed - began, 16, "a wind-up you can see coming");
}

#[test]
fn dodging_away_during_the_wind_up_avoids_the_blow() {
    let mut w = alice_near_the_beast();
    until(&mut w, 400, |e| {
        matches!(
            e,
            Event::Began {
                who: BodyId::Creature(_),
                ..
            }
        )
    })
    .unwrap();
    act(&mut w, ALICE, Intent::Dodge(west())).unwrap();
    let events = run(&mut w, 20);
    assert!(!events.iter().any(|e| matches!(e, Event::Struck { .. })));
}

#[test]
fn the_beast_can_kill_you_and_you_cross_into_lately() {
    let mut w = alice_near_the_beast();
    let fell_at = pos(&w, ALICE);
    until(
        &mut w,
        2000,
        |e| matches!(e, Event::WentToLately { person } if *person == ALICE),
    )
    .expect("three blows of 12 kill a person of 30");
    let alice = w.person(ALICE).unwrap();
    assert_eq!(alice.body.side, Side::Lately);
    assert_eq!(alice.body.health, 30);
    assert_eq!(alice.deaths, 1);
    // The same land: she crosses where she fell, give or take the steps the
    // fight moved her.
    assert!(alice.body.at.pos.distance_sq(fell_at) < 2000 * 2000);
    // The beast lives on the living side: it has nobody left to hunt.
    let events = run(&mut w, 400);
    assert!(!events.iter().any(|e| matches!(e, Event::Struck { .. })));
}

#[test]
fn a_beast_falls_to_enough_blows_and_its_lair_lets_out_another() {
    let mut w = alice_near_the_beast();
    let mut fell = None;
    for _ in 0..2000 {
        let beast = w.state().creatures.values().next().map(|c| c.body.at.pos);
        if let Some(b) = beast {
            let aim = Dir::toward(pos(&w, ALICE), b);
            let _ = act(&mut w, ALICE, Intent::Attack(aim));
        }
        w.step();
        if let Some(Event::Fell { .. }) = w
            .take_events()
            .into_iter()
            .find(|e| matches!(e, Event::Fell { .. }))
        {
            fell = Some(w.step_count());
            break;
        }
    }
    let fell = fell.expect("two blows of 10 fell a beast of 20");
    let arose = until(&mut w, 400, |e| matches!(e, Event::Arose { .. })).expect("another comes");
    assert_eq!(arose - fell, 200);
}

/// Alice, dead and standing on the threshold.
fn alice_dead_at_the_threshold() -> tme_sim::World {
    let mut w = alice_near_the_beast();
    until(&mut w, 2000, |e| matches!(e, Event::WentToLately { .. })).unwrap();
    place(&mut w, ALICE, THRESHOLD);
    w
}

#[test]
fn the_threshold_brings_you_back_with_a_tardy() {
    let mut w = alice_dead_at_the_threshold();
    act(&mut w, ALICE, Intent::Use).unwrap();
    // The rite holds you.
    assert_eq!(
        act(&mut w, ALICE, Intent::Dodge(Dir::NORTH)),
        Err(Refused::Committed)
    );
    let started = w.step_count();
    let up = until(&mut w, 100, |e| matches!(e, Event::TurnedUp { .. })).expect("turns up");
    assert_eq!(up - started, 40);
    let alice = w.person(ALICE).unwrap();
    assert_eq!(alice.body.side, Side::Living);
    assert_eq!(alice.body.at.pos, EXIT);
    assert_eq!(alice.tardies.len(), 1);
}

#[test]
fn the_living_have_no_threshold_to_use() {
    let mut w = world();
    place(&mut w, ALICE, THRESHOLD);
    assert_eq!(act(&mut w, ALICE, Intent::Use), Err(Refused::NothingToUse));
    // Nor the dead away from one.
    let mut w = alice_near_the_beast();
    until(&mut w, 2000, |e| matches!(e, Event::WentToLately { .. })).unwrap();
    assert_eq!(act(&mut w, ALICE, Intent::Use), Err(Refused::NothingToUse));
}

#[test]
fn each_tardy_makes_the_next_return_longer_until_there_is_none() {
    let mut w = alice_dead_at_the_threshold();
    let mut lengths = Vec::new();
    for _ in 0..2 {
        act(&mut w, ALICE, Intent::Use).unwrap();
        let started = w.step_count();
        lengths
            .push(until(&mut w, 200, |e| matches!(e, Event::TurnedUp { .. })).unwrap() - started);
        // Back to the beast, and dead again.
        place(&mut w, ALICE, LAIR - Vec2::new(4000, 0));
        until(&mut w, 2000, |e| matches!(e, Event::WentToLately { .. })).unwrap();
        place(&mut w, ALICE, THRESHOLD);
    }
    assert_eq!(lengths, vec![40, 60]);
    let alice = w.person(ALICE).unwrap();
    assert_eq!(alice.tardies.len(), 2);
    assert_ne!(
        alice.tardies[0], alice.tardies[1],
        "a tardy isn't worn twice"
    );
    // Two is as many as a life can carry back: this one stays in Lately.
    assert_eq!(alice.body.side, Side::Lately);
    assert_eq!(act(&mut w, ALICE, Intent::Use), Err(Refused::TooMarked));
}

#[test]
fn a_tardy_changes_the_life_it_marks() {
    use tme_sim::body;
    let cat = catalogue();
    let person = tme_sim::Id::new("person");
    assert_eq!(
        body::max_health(&cat, &person, &[tme_sim::Id::new("pale")]),
        25
    );
    assert_eq!(
        body::speed(&cat, &person, &[tme_sim::Id::new("light-foot")]),
        220
    );
    assert!(body::sees_lately(&cat, &[tme_sim::Id::new("second-sight")]));
    assert!(!body::sees_lately(&cat, &[tme_sim::Id::new("pale")]));
}

#[test]
fn a_beast_with_a_wall_between_it_and_you_finds_a_way_round() {
    let mut cat = catalogue();
    let field = cat.places.values_mut().next().unwrap();
    // Its lair just east of the living wall, which runs north-south.
    field.lairs[0].at = Vec2::new(-3500, 0);
    let mut w = tme_sim::World::new(
        std::sync::Arc::new(cat),
        3,
        tme_sim::world::Settings::default(),
    );
    join(&mut w, ALICE);
    // Round the wall's north end to stand just west of it, face to face
    // with the beast through it.
    place(&mut w, ALICE, Vec2::new(0, 4500));
    place(&mut w, ALICE, Vec2::new(-8000, 4500));
    place(&mut w, ALICE, Vec2::new(-7000, 0));
    let struck = until(
        &mut w,
        2000,
        |e| matches!(e, Event::Struck { target, .. } if *target == BodyId::Person(ALICE)),
    );
    assert!(struck.is_some(), "the beast gets round the wall");
}
