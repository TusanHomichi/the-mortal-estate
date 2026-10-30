//! Creatures: where they come from, how they hunt, and how they fall. They
//! commit to their swings like anyone, and wind up where you can see it.

use crate::body::{Action, Address, Body, Doing};
use crate::catalogue::{Catalogue, Id, Place};
use crate::geom::{Dir, Vec2};
use crate::nav::Nav;
use crate::world::{BodyId, CreatureId, Event, PersonId, State};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Creature {
    pub body: Body,
    /// Which lair it came from: an index into the world's lairs.
    pub lair: usize,
    /// Who it's hunting.
    pub target: Option<PersonId>,
    /// Where it's wandering to, with nobody to hunt.
    pub roam_to: Option<Vec2>,
    /// The step it last stood idle from.
    pub idle_since: u64,
    /// The way it's following round whatever's between it and where it's
    /// going, and where that was when the way was found.
    pub path: Vec<Vec2>,
    pub path_to: Option<Vec2>,
}

/// A lair in the world: whether its creature is out, and when the next one
/// comes if not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LairState {
    pub place: Id<Place>,
    /// Which of the place's lairs.
    pub index: usize,
    pub creature: Option<CreatureId>,
    /// The step from which a new creature may come out.
    pub ready_at: u64,
}

/// Every lair in the catalogue, empty and ready.
pub fn lairs(catalogue: &Catalogue) -> Vec<LairState> {
    catalogue
        .places
        .iter()
        .flat_map(|(id, place)| {
            (0..place.lairs.len()).map(|index| LairState {
                place: id.clone(),
                index,
                creature: None,
                ready_at: 0,
            })
        })
        .collect()
}

/// `saved` as it would be laid out from `catalogue`: the same if the
/// catalogue's lairs are the ones it was saved with.
pub fn relaid(catalogue: &Catalogue, saved: &[LairState]) -> Vec<LairState> {
    lairs(catalogue)
        .into_iter()
        .zip(saved)
        .map(|(fresh, old)| LairState {
            creature: old.creature,
            ready_at: old.ready_at,
            ..fresh
        })
        .chain(
            saved
                .iter()
                .skip(lairs(catalogue).len())
                .cloned()
                .map(|mut extra| {
                    // A lair the catalogue no longer has can't match.
                    extra.index = usize::MAX;
                    extra
                }),
        )
        .collect()
}

/// Empty lairs that are ready let out a new creature.
pub fn arise(catalogue: &Catalogue, state: &mut State, events: &mut Vec<Event>) {
    let now = state.step;
    for (index, lair) in state.lairs.iter_mut().enumerate() {
        if lair.creature.is_some() || now < lair.ready_at {
            continue;
        }
        let spec = &catalogue.place(&lair.place).lairs[lair.index];
        let kind = catalogue.body(&spec.kind);
        let id = CreatureId(state.next_creature);
        state.next_creature += 1;
        let body = Body {
            kind: spec.kind.clone(),
            at: Address {
                place: lair.place.clone(),
                level: 0,
                pos: spec.at,
            },
            side: spec.side,
            facing: Dir::NORTH,
            health: kind.health,
            held: Dir::NONE,
            action: None,
        };
        state.creatures.insert(
            id,
            Creature {
                body,
                lair: index,
                target: None,
                roam_to: None,
                idle_since: now,
                path: Vec::new(),
                path_to: None,
            },
        );
        lair.creature = Some(id);
        events.push(Event::Arose { creature: id });
    }
}

/// How long a creature stands about before wandering on, in steps.
const IDLE_STEPS: u64 = 40;

/// How far what a creature is after can move before it finds a new way.
const REPATH: i32 = 2000;

/// Every creature decides what to do this step.
pub fn think(catalogue: &Catalogue, state: &mut State, nav: &mut Nav, events: &mut Vec<Event>) {
    let now = state.step;
    let ids: Vec<CreatureId> = state.creatures.keys().copied().collect();
    for id in ids {
        let creature = &state.creatures[&id];
        if creature.body.action.is_some() {
            continue;
        }
        let kind = catalogue.body(&creature.body.kind);
        let behaviour = kind
            .behaviour
            .as_ref()
            .expect("a creature's kind has a behaviour");
        let home =
            catalogue.place(&creature.body.at.place).lairs[state.lairs[creature.lair].index].at;
        let pos = creature.body.at.pos;
        let leashed = pos.distance_sq(home) > sq(behaviour.leash);
        let hunts = |p: &crate::world::Person| {
            p.present
                && p.body.side == creature.body.side
                && p.body.at.place == creature.body.at.place
                && p.body.at.level == creature.body.at.level
        };
        // Keep after its quarry while it's huntable and home isn't too far;
        // otherwise look for the nearest within sense.
        let target = creature
            .target
            .filter(|t| !leashed && state.people.get(t).is_some_and(hunts))
            .or_else(|| {
                if leashed {
                    return None;
                }
                state
                    .people
                    .iter()
                    .filter(|(_, p)| {
                        hunts(p) && p.body.at.pos.distance_sq(pos) <= sq(behaviour.sense)
                    })
                    .min_by_key(|(id, p)| (p.body.at.pos.distance_sq(pos), **id))
                    .map(|(id, _)| *id)
            });
        let mut roam_to = creature.roam_to;
        let mut idle_since = creature.idle_since;
        let goal = match target {
            Some(t) => Some(state.people[&t].body.at.pos),
            None if leashed => Some(home),
            None => None,
        };
        let (held, swing_at) = match target {
            Some(t) => {
                let quarry = &state.people[&t].body;
                let reach =
                    kind.radius + catalogue.body(&quarry.kind).radius + kind.attack.reach * 3 / 4;
                roam_to = None;
                if quarry.at.pos.distance_sq(pos) <= sq(reach) {
                    (Dir::NONE, Some(Dir::toward(pos, quarry.at.pos)))
                } else {
                    (Dir::toward(pos, quarry.at.pos), None)
                }
            }
            None if leashed => {
                roam_to = Some(home);
                (Dir::toward(pos, home), None)
            }
            None => {
                let arrived = roam_to.is_none_or(|to| to.distance_sq(pos) <= sq(300));
                if arrived && roam_to.is_some() {
                    roam_to = None;
                    idle_since = now;
                }
                if roam_to.is_none() && now >= idle_since + IDLE_STEPS {
                    let r = behaviour.roam;
                    roam_to = Some(home + Vec2::new(state.rng.spread(r), state.rng.spread(r)));
                }
                match roam_to {
                    // Wandering is an amble: half a push.
                    Some(to) => {
                        let d = Dir::toward(pos, to).as_vec();
                        (Dir::new(d.x / 2, d.y / 2), None)
                    }
                    None => (Dir::NONE, None),
                }
            }
        };
        // Where something stands in the way, it follows the shortest way
        // round rather than pressing against it.
        let goal = goal.or(roam_to);
        let mut path = creature.path.clone();
        let mut path_to = creature.path_to;
        let mut held = held;
        match goal {
            Some(g) if !held.is_none() => {
                let grid = nav.grid(
                    catalogue,
                    &creature.body.at.place,
                    creature.body.side,
                    kind.radius,
                );
                if grid.clear(pos, g) {
                    path.clear();
                    path_to = None;
                } else {
                    if path.is_empty() || path_to.is_none_or(|p| p.distance_sq(g) > sq(REPATH)) {
                        path = grid.path(pos, g).unwrap_or_default();
                        path_to = Some(g);
                    }
                    while path.len() > 1
                        && (pos.distance_sq(path[0]) < sq(400) || grid.clear(pos, path[1]))
                    {
                        path.remove(0);
                    }
                    match path.first() {
                        Some(next) => {
                            let push = held.as_vec().length() as i32;
                            let d = Dir::toward(pos, *next).as_vec().with_length(push);
                            held = Dir::new(d.x, d.y);
                        }
                        // No way there: a wanderer picks somewhere else; a
                        // hunter waits.
                        None => {
                            held = Dir::NONE;
                            if target.is_none() {
                                roam_to = None;
                                idle_since = now;
                            }
                        }
                    }
                }
            }
            _ => {
                path.clear();
                path_to = None;
            }
        }
        let creature = state.creatures.get_mut(&id).expect("listed");
        creature.target = target;
        creature.roam_to = roam_to;
        creature.idle_since = idle_since;
        creature.path = path;
        creature.path_to = path_to;
        creature.body.held = held;
        if let Some(aim) = swing_at {
            if let Some(face) = aim.full() {
                creature.body.facing = face;
            }
            creature.body.action = Some(Action {
                doing: Doing::Attack,
                started: now,
            });
            events.push(Event::Began {
                who: BodyId::Creature(id),
                doing: Doing::Attack,
            });
        }
    }
}

/// Creatures with no health left fall; their lairs start counting down.
pub fn fall(catalogue: &Catalogue, state: &mut State, now: u64, events: &mut Vec<Event>) {
    let fallen: Vec<CreatureId> = state
        .creatures
        .iter()
        .filter(|(_, c)| c.body.health <= 0)
        .map(|(id, _)| *id)
        .collect();
    for id in fallen {
        let creature = state.creatures.remove(&id).expect("listed");
        let kind = catalogue.body(&creature.body.kind);
        let respawn = kind.behaviour.as_ref().map_or(0, |b| b.respawn);
        let lair = &mut state.lairs[creature.lair];
        lair.creature = None;
        lair.ready_at = now + u64::from(respawn);
        events.push(Event::Fell { creature: id });
    }
}

fn sq(mm: i32) -> i64 {
    i64::from(mm) * i64::from(mm)
}
