//! What each person observes: the world indexed once a step, every body's
//! view made once and marked if it changed since the last step, and each
//! observer sent only what changed within their view, or came into it.

use std::collections::{BTreeSet, HashMap};
use tme_sim::catalogue::Place;
use tme_sim::geom::{METRE, Mm};
use tme_sim::world::{Event, PersonId};
use tme_sim::{BodyId, Id, Vec2, World};
use tme_wire::{BodyView, Observation, You};

/// How far a person sees, in every direction.
pub const VIEW: Mm = 30 * METRE;

/// The side of a bucket.
const BUCKET: Mm = 8 * METRE;

/// Where bodies are in one place and level, by bucket.
type Buckets = HashMap<(i32, i32), Vec<(BodyId, Vec2)>>;

/// The world as observers see it this step: every body's view, whether it
/// changed since the last step, and where it is in coarse buckets.
#[derive(Default)]
pub struct Index {
    places: HashMap<(Id<Place>, i8), Buckets>,
    views: HashMap<BodyId, (BodyView, bool)>,
}

impl Index {
    /// The world as it is now, marked against `last`, the index of the step
    /// before.
    pub fn new(world: &World, last: &Index) -> Index {
        let state = world.state();
        let bodies = state
            .people
            .iter()
            .filter(|(_, p)| p.in_world())
            .map(|(id, _)| BodyId::Person(*id))
            .chain(state.creatures.keys().map(|id| BodyId::Creature(*id)));
        let mut index = Index::default();
        for id in bodies {
            let Some(view) = BodyView::of(world, id) else {
                continue;
            };
            let bucket = (view.pos.x.div_euclid(BUCKET), view.pos.y.div_euclid(BUCKET));
            index
                .places
                .entry((view.place.clone(), view.level))
                .or_default()
                .entry(bucket)
                .or_default()
                .push((id, view.pos));
            let changed = last.views.get(&id).is_none_or(|(was, _)| *was != view);
            index.views.insert(id, (view, changed));
        }
        index
    }

    /// Every body within `radius` of `at`, on both sides.
    pub fn near(&self, place: &Id<Place>, level: i8, at: Vec2, radius: Mm) -> Vec<BodyId> {
        let Some(buckets) = self.places.get(&(place.clone(), level)) else {
            return Vec::new();
        };
        let r2 = i64::from(radius) * i64::from(radius);
        let (lo, hi) = (
            at - Vec2::new(radius, radius),
            at + Vec2::new(radius, radius),
        );
        let mut out = Vec::new();
        for bx in lo.x.div_euclid(BUCKET)..=hi.x.div_euclid(BUCKET) {
            for by in lo.y.div_euclid(BUCKET)..=hi.y.div_euclid(BUCKET) {
                if let Some(bodies) = buckets.get(&(bx, by)) {
                    out.extend(
                        bodies
                            .iter()
                            .filter(|(_, p)| p.distance_sq(at) <= r2)
                            .map(|(id, _)| *id),
                    );
                }
            }
        }
        out.sort();
        out
    }
}

/// One connected person, and who was in their view last step.
pub struct Observer {
    pub person: PersonId,
    /// The last intent applied, by its number.
    pub ack: u32,
    /// Everyone in view at the last step: each was sent as they were then,
    /// so only a change, or coming into view, needs sending now.
    in_view: BTreeSet<BodyId>,
}

impl Observer {
    pub fn new(person: PersonId) -> Observer {
        Observer {
            person,
            ack: 0,
            in_view: BTreeSet::new(),
        }
    }

    /// What this person observes now, given this step's index and events.
    /// It must be asked every step, as the index is marked against the
    /// step before.
    pub fn observe(
        &mut self,
        world: &World,
        index: &Index,
        events: &[Event],
    ) -> Option<Observation> {
        let Some(you) = You::of(world, self.person) else {
            // Not in the world this step: everything is new when they are.
            self.in_view.clear();
            return None;
        };
        let me = BodyId::Person(self.person);
        let mut seen = Vec::new();
        let mut now = BTreeSet::new();
        for id in index.near(&you.body.place, you.body.level, you.body.pos, VIEW) {
            if id == me {
                continue;
            }
            let Some((view, changed)) = index.views.get(&id) else {
                continue;
            };
            if *changed || !self.in_view.contains(&id) {
                seen.push(view.clone());
            }
            now.insert(id);
        }
        let gone = self.in_view.difference(&now).copied().collect();
        self.in_view = now;
        let in_view = |id: &BodyId| *id == me || self.in_view.contains(id);
        let events = events
            .iter()
            .filter(|e| subjects(e).iter().any(in_view))
            .cloned()
            .collect();
        Some(Observation {
            step: world.step_count(),
            ack: self.ack,
            you,
            seen,
            gone,
            events,
        })
    }
}

/// The bodies an event is about.
fn subjects(e: &Event) -> Vec<BodyId> {
    match e {
        Event::Joined { person } | Event::Left { person } => vec![BodyId::Person(*person)],
        Event::WentToLately { person } | Event::TurnedUp { person, .. } => {
            vec![BodyId::Person(*person)]
        }
        Event::Began { who, .. } => vec![*who],
        Event::Struck { by, target, .. } => vec![*by, *target],
        Event::Fell { creature } | Event::Arose { creature } => vec![BodyId::Creature(*creature)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tme_sim::world::{Input, Intent, Settings};
    use tme_sim::{Dir, PersonId};

    const ALICE: PersonId = PersonId(1);
    const BOB: PersonId = PersonId(2);

    /// Lattermead with Alice and Bob standing where everyone arrives.
    fn two_people() -> World {
        let content = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let catalogue = Arc::new(tme_content::load(&content).expect("content").catalogue);
        let mut world = World::new(catalogue, 1, Settings::default());
        for (person, name) in [(ALICE, "Alice"), (BOB, "Bob")] {
            world
                .apply(&Input::Join {
                    person,
                    name: name.into(),
                })
                .expect("joins");
        }
        world
    }

    fn walk(world: &mut World, person: PersonId, x: i32) {
        world
            .apply(&Input::Act {
                person,
                intent: Intent::Move(Dir::new(x, 0)),
            })
            .expect("walks");
    }

    /// Alice, and the index of the last step she observed.
    struct Watching {
        alice: Observer,
        index: Index,
    }

    /// A step, and what Alice observes after it.
    fn tick(world: &mut World, w: &mut Watching) -> Observation {
        world.step();
        let events = world.take_events();
        w.index = Index::new(world, &w.index);
        w.alice
            .observe(world, &w.index, &events)
            .expect("Alice is here")
    }

    fn sees_bob(o: &Observation) -> bool {
        o.seen.iter().any(|v| v.id == BodyId::Person(BOB))
    }

    #[test]
    fn a_body_is_sent_when_it_comes_into_view_or_changes_and_not_otherwise() {
        let mut world = two_people();
        let mut alice = Watching {
            alice: Observer::new(ALICE),
            index: Index::default(),
        };
        assert!(sees_bob(&tick(&mut world, &mut alice)), "Bob, first seen");
        assert!(!sees_bob(&tick(&mut world, &mut alice)), "Bob, unchanged");
        walk(&mut world, BOB, i32::from(Dir::UNIT));
        assert!(sees_bob(&tick(&mut world, &mut alice)), "Bob, walking");
        assert!(sees_bob(&tick(&mut world, &mut alice)), "Bob, further on");
        // East until he's out of view, then back until he's in it again.
        let gone = (0..400).find(|_| {
            tick(&mut world, &mut alice)
                .gone
                .contains(&BodyId::Person(BOB))
        });
        assert!(gone.is_some(), "Bob walks out of view");
        walk(&mut world, BOB, -i32::from(Dir::UNIT));
        let back = (0..400).find(|_| sees_bob(&tick(&mut world, &mut alice)));
        assert!(back.is_some(), "Bob, back in view");
        walk(&mut world, BOB, 0);
        assert!(sees_bob(&tick(&mut world, &mut alice)), "Bob, stopping");
        assert!(!sees_bob(&tick(&mut world, &mut alice)), "Bob, stood still");
    }

    #[test]
    fn buckets_find_bodies_across_their_edges_and_no_further() {
        let mut buckets = Buckets::new();
        let mut put = |id: u32, at: Vec2| {
            buckets
                .entry((at.x.div_euclid(BUCKET), at.y.div_euclid(BUCKET)))
                .or_default()
                .push((BodyId::Creature(tme_sim::CreatureId(id)), at));
        };
        put(1, Vec2::new(-100, -100));
        put(2, Vec2::new(29 * METRE, 0));
        put(3, Vec2::new(31 * METRE, 0));
        let b = Index {
            places: HashMap::from([((Id::new("p"), 0), buckets)]),
            views: HashMap::new(),
        };
        let near = b.near(&Id::new("p"), 0, Vec2::new(100, 100), VIEW);
        assert_eq!(
            near,
            vec![
                BodyId::Creature(tme_sim::CreatureId(1)),
                BodyId::Creature(tme_sim::CreatureId(2))
            ]
        );
        assert!(b.near(&Id::new("p"), 1, Vec2::ZERO, VIEW).is_empty());
    }
}
