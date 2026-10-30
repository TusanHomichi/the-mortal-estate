//! The world: everyone in it, both sides, stepped at a fixed rate. Inputs
//! are applied between steps, in the order they arrived; given the same
//! inputs on the same steps, a world always comes out the same.

use crate::body::{self, Action, Address, Body, Doing};
use crate::catalogue::{Catalogue, Id, Mark, Side};
use crate::creature::{self, Creature, LairState};
use crate::geom::{Dir, Vec2};
use crate::nav::Nav;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

/// Who a person is in this world: their account, made the first time they
/// join.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PersonId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CreatureId(pub u32);

/// Any body in the world. People sort before creatures, so the order bodies
/// act in each step is fixed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BodyId {
    Person(PersonId),
    Creature(CreatureId),
}

/// A life in the world. In this slice a person is one life; lineages come
/// with succession.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    pub body: Body,
    /// Whether their player is here. Someone who leaves stays where they
    /// were, on whichever side, until they come back.
    pub present: bool,
    /// The marks their returns have left, oldest first.
    pub tardies: Vec<Id<Mark>>,
    pub deaths: u32,
}

/// The world's settings, which its host chooses.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Whether people can hurt each other. Off unless the host turns it on.
    pub harm_between_players: bool,
}

/// Everything a world is, as its snapshots save it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    /// How many steps the world has taken. Time in the world is this count,
    /// never the clock on the wall.
    pub step: u64,
    pub rng: Rng,
    pub settings: Settings,
    pub people: BTreeMap<PersonId, Person>,
    pub creatures: BTreeMap<CreatureId, Creature>,
    pub lairs: Vec<LairState>,
    pub next_creature: u32,
}

/// What a player asks their person to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    /// Steer: held until changed, walked whenever nothing is under way.
    Move(Dir),
    /// Swing toward a direction (or straight ahead, given none).
    Attack(Dir),
    /// Dash a direction (or straight ahead, given none).
    Dodge(Dir),
    /// Use what's here: in Lately, a threshold.
    Use,
}

/// Something that happens to the world from outside it, in the order it
/// happened. The journal keeps these.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input {
    /// A player arrives; the first time, as someone new.
    Join {
        person: PersonId,
        name: String,
    },
    Leave {
        person: PersonId,
    },
    Act {
        person: PersonId,
        intent: Intent,
    },
}

/// Why an input changed nothing. Busy input is refused, never queued.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Refused {
    /// They aren't here (never joined, or left).
    NotHere,
    /// They're already here.
    AlreadyHere,
    /// They're in the middle of something.
    Committed,
    /// There's nothing here to use.
    NothingToUse,
    /// Too many tardies: this life can't turn up again.
    TooMarked,
    /// Their body has no dodge.
    CantDodge,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Refused::NotHere => "not here",
            Refused::AlreadyHere => "already here",
            Refused::Committed => "in the middle of something",
            Refused::NothingToUse => "nothing here to use",
            Refused::TooMarked => "too marked to turn up again",
            Refused::CantDodge => "can't dodge",
        })
    }
}

/// What happened, for whoever's watching: observers, logs and tests.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    Joined {
        person: PersonId,
    },
    Left {
        person: PersonId,
    },
    Began {
        who: BodyId,
        doing: Doing,
    },
    Struck {
        by: BodyId,
        target: BodyId,
        damage: i32,
    },
    /// A creature fell.
    Fell {
        creature: CreatureId,
    },
    /// A creature came out of its lair.
    Arose {
        creature: CreatureId,
    },
    /// A person died and crossed over: they've gone to Lately.
    WentToLately {
        person: PersonId,
    },
    /// A person came back from Lately, with a new tardy.
    TurnedUp {
        person: PersonId,
        tardy: Option<Id<Mark>>,
    },
}

/// A saved world that doesn't fit the catalogue it's loaded with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mismatch(pub String);

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the saved world doesn't fit its content: {}", self.0)
    }
}

impl std::error::Error for Mismatch {}

/// A running world.
pub struct World {
    catalogue: Arc<Catalogue>,
    state: State,
    events: Vec<Event>,
    nav: Nav,
}

impl World {
    /// A new world, rolled from `seed`.
    pub fn new(catalogue: Arc<Catalogue>, seed: u64, settings: Settings) -> World {
        let lairs = creature::lairs(&catalogue);
        World {
            state: State {
                step: 0,
                rng: Rng::new(seed),
                settings,
                people: BTreeMap::new(),
                creatures: BTreeMap::new(),
                lairs,
                next_creature: 0,
            },
            catalogue,
            events: Vec::new(),
            nav: Nav::default(),
        }
    }

    /// A saved world, checked against the catalogue it will run on.
    pub fn load(catalogue: Arc<Catalogue>, state: State) -> Result<World, Mismatch> {
        let missing = |what: String| Err(Mismatch(what));
        let bodies = state
            .people
            .values()
            .map(|p| &p.body)
            .chain(state.creatures.values().map(|c| &c.body));
        for b in bodies {
            if !catalogue.bodies.contains_key(&b.kind) {
                return missing(format!("no body kind {}", b.kind));
            }
            if !catalogue.places.contains_key(&b.at.place) {
                return missing(format!("no place {}", b.at.place));
            }
        }
        for p in state.people.values() {
            if let Some(t) = p.tardies.iter().find(|t| !catalogue.marks.contains_key(t)) {
                return missing(format!("no mark {t}"));
            }
        }
        if state.lairs != creature::relaid(&catalogue, &state.lairs) {
            return missing("its lairs have changed".to_owned());
        }
        Ok(World {
            catalogue,
            state,
            events: Vec::new(),
            nav: Nav::default(),
        })
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn catalogue(&self) -> &Arc<Catalogue> {
        &self.catalogue
    }

    pub fn step_count(&self) -> u64 {
        self.state.step
    }

    pub fn person(&self, id: PersonId) -> Option<&Person> {
        self.state.people.get(&id)
    }

    pub fn body(&self, id: BodyId) -> Option<&Body> {
        match id {
            BodyId::Person(p) => self.state.people.get(&p).map(|p| &p.body),
            BodyId::Creature(c) => self.state.creatures.get(&c).map(|c| &c.body),
        }
    }

    /// The events since the last time they were taken.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Apply one input now, before the next step.
    pub fn apply(&mut self, input: &Input) -> Result<(), Refused> {
        match input {
            Input::Join { person, name } => self.join(*person, name),
            Input::Leave { person } => {
                let p = self.present_mut(*person)?;
                p.present = false;
                p.body.action = None;
                p.body.held = Dir::NONE;
                self.events.push(Event::Left { person: *person });
                Ok(())
            }
            Input::Act { person, intent } => self.act(*person, *intent),
        }
    }

    fn join(&mut self, id: PersonId, name: &str) -> Result<(), Refused> {
        if let Some(p) = self.state.people.get_mut(&id) {
            if p.present {
                return Err(Refused::AlreadyHere);
            }
            p.present = true;
        } else {
            let rules = &self.catalogue.rules;
            let place = self.catalogue.place(&rules.arrival);
            let body = Body {
                kind: rules.person.clone(),
                at: Address {
                    place: rules.arrival.clone(),
                    level: 0,
                    pos: place.arrival,
                },
                side: Side::Living,
                facing: Dir::NORTH,
                health: body::max_health(&self.catalogue, &rules.person, &[]),
                held: Dir::NONE,
                action: None,
            };
            let person = Person {
                name: name.to_owned(),
                body,
                present: true,
                tardies: Vec::new(),
                deaths: 0,
            };
            self.state.people.insert(id, person);
        }
        self.events.push(Event::Joined { person: id });
        Ok(())
    }

    fn present_mut(&mut self, id: PersonId) -> Result<&mut Person, Refused> {
        match self.state.people.get_mut(&id) {
            Some(p) if p.present => Ok(p),
            _ => Err(Refused::NotHere),
        }
    }

    fn act(&mut self, id: PersonId, intent: Intent) -> Result<(), Refused> {
        let now = self.state.step;
        let catalogue = Arc::clone(&self.catalogue);
        let person = self.present_mut(id)?;
        let kind = catalogue.body(&person.body.kind);
        let doing = match intent {
            Intent::Move(dir) => {
                person.body.held = dir;
                return Ok(());
            }
            Intent::Attack(aim) => {
                if person.body.action.is_some() {
                    return Err(Refused::Committed);
                }
                if let Some(aim) = aim.full() {
                    person.body.facing = aim;
                }
                Doing::Attack
            }
            Intent::Dodge(dir) => {
                if kind.dodge.is_none() {
                    return Err(Refused::CantDodge);
                }
                // A wind-up is the one thing a dodge can break out of.
                let winding_up = person.body.phase(kind, now) == Some(body::Phase::WindUp);
                if person.body.action.is_some() && !winding_up {
                    return Err(Refused::Committed);
                }
                let dir = dir.full().unwrap_or(person.body.facing);
                person.body.facing = dir;
                Doing::Dodge { dir }
            }
            Intent::Use => {
                if person.body.action.is_some() {
                    return Err(Refused::Committed);
                }
                if person.body.side != Side::Lately {
                    return Err(Refused::NothingToUse);
                }
                let place = catalogue.place(&person.body.at.place);
                let pos = person.body.at.pos;
                let threshold = place
                    .thresholds
                    .iter()
                    .position(|t| {
                        pos.distance_sq(t.at) <= i64::from(t.radius) * i64::from(t.radius)
                    })
                    .ok_or(Refused::NothingToUse)?;
                let rule = &catalogue.rules.turning_up;
                let carried = person.tardies.len();
                if carried >= rule.max_tardies {
                    return Err(Refused::TooMarked);
                }
                let length = rule.base + rule.per_tardy * carried as u32;
                Doing::TurnUp { threshold, length }
            }
        };
        person.body.action = Some(Action {
            doing,
            started: now,
        });
        self.events.push(Event::Began {
            who: BodyId::Person(id),
            doing,
        });
        Ok(())
    }

    /// Move the world on one step.
    pub fn step(&mut self) {
        self.state.step += 1;
        let now = self.state.step;
        creature::arise(&self.catalogue, &mut self.state, &mut self.events);
        creature::think(
            &self.catalogue,
            &mut self.state,
            &mut self.nav,
            &mut self.events,
        );
        let ids: Vec<BodyId> = self
            .state
            .people
            .iter()
            .filter(|(_, p)| p.present)
            .map(|(id, _)| BodyId::Person(*id))
            .chain(self.state.creatures.keys().map(|id| BodyId::Creature(*id)))
            .collect();
        for id in ids {
            self.advance(id, now);
        }
        self.resolve_deaths(now);
    }

    /// One body's step: its action carried on, or a walk.
    fn advance(&mut self, id: BodyId, now: u64) {
        let catalogue = Arc::clone(&self.catalogue);
        let (body, tardies) = match id {
            BodyId::Person(p) => {
                let p = self.state.people.get_mut(&p).expect("a listed person");
                (&mut p.body, Some(p.tardies.clone()))
            }
            BodyId::Creature(c) => (
                &mut self
                    .state
                    .creatures
                    .get_mut(&c)
                    .expect("a listed creature")
                    .body,
                None,
            ),
        };
        let kind = catalogue.body(&body.kind);
        let place = catalogue.place(&body.at.place);
        let Some(action) = body.action.clone() else {
            let speed = match &tardies {
                Some(t) => body::speed(&catalogue, &body.kind, t),
                None => kind.speed,
            };
            if let Some(face) = body.held.full() {
                body.facing = face;
                let travel = body.held.travel(speed);
                body.at.pos = body::walk(place, body.side, body.at.pos, kind.radius, travel);
            }
            return;
        };
        let into = now - action.started;
        match action.doing {
            Doing::Attack => {
                if into == u64::from(kind.attack.wind_up) {
                    self.land(id);
                }
                if into >= u64::from(kind.attack.length()) {
                    self.body_mut(id).action = None;
                }
            }
            Doing::Dodge { dir } => {
                let dodge = kind
                    .dodge
                    .as_ref()
                    .expect("a dodge was only begun with one");
                let duration = u64::from(dodge.duration.max(1));
                if into <= duration {
                    // Spread evenly, the remainder on the last step.
                    let share =
                        |n: u64| (i64::from(dodge.distance) * n as i64 / duration as i64) as i32;
                    let travel = dir.travel(share(into) - share(into - 1));
                    body.at.pos = body::walk(place, body.side, body.at.pos, kind.radius, travel);
                }
                if into >= duration + u64::from(dodge.recover) {
                    body.action = None;
                }
            }
            Doing::TurnUp { threshold, length } => {
                if into >= u64::from(length) {
                    let BodyId::Person(person) = id else {
                        unreachable!("only people turn up")
                    };
                    self.turn_up(person, threshold);
                }
            }
        }
    }

    fn body_mut(&mut self, id: BodyId) -> &mut Body {
        match id {
            BodyId::Person(p) => &mut self.state.people.get_mut(&p).expect("a listed person").body,
            BodyId::Creature(c) => {
                &mut self
                    .state
                    .creatures
                    .get_mut(&c)
                    .expect("a listed creature")
                    .body
            }
        }
    }

    /// A swing lands: everyone hostile in its arc on this side is struck.
    fn land(&mut self, by: BodyId) {
        let attacker = self.body(by).expect("the attacker is here").clone();
        let kind = self.catalogue.body(&attacker.kind).clone();
        let harm_people = self.state.settings.harm_between_players;
        let targets: Vec<BodyId> = self
            .state
            .people
            .iter()
            .filter(|(_, p)| p.present)
            .map(|(id, p)| (BodyId::Person(*id), &p.body))
            .chain(
                self.state
                    .creatures
                    .iter()
                    .map(|(id, c)| (BodyId::Creature(*id), &c.body)),
            )
            .filter(|(id, b)| {
                let hostile = match (by, *id) {
                    (BodyId::Creature(_), BodyId::Person(_))
                    | (BodyId::Person(_), BodyId::Creature(_)) => true,
                    (BodyId::Person(a), BodyId::Person(b)) => harm_people && a != b,
                    (BodyId::Creature(_), BodyId::Creature(_)) => false,
                };
                hostile
                    && b.side == attacker.side
                    && b.at.place == attacker.at.place
                    && b.at.level == attacker.at.level
                    && b.health > 0
                    && in_arc(&attacker, &kind, b, self.catalogue.body(&b.kind).radius)
            })
            .map(|(id, _)| id)
            .collect();
        for target in targets {
            let damage = kind.attack.damage;
            self.body_mut(target).health -= damage;
            self.events.push(Event::Struck { by, target, damage });
        }
    }

    fn resolve_deaths(&mut self, now: u64) {
        let catalogue = Arc::clone(&self.catalogue);
        for (id, p) in self.state.people.iter_mut() {
            if !p.present || p.body.health > 0 {
                continue;
            }
            // Dying crosses over, where they stand: the same land, as it was.
            let place = catalogue.place(&p.body.at.place);
            let radius = catalogue.body(&p.body.kind).radius;
            p.body.side = Side::Lately;
            p.body.at.pos = body::settle(place, Side::Lately, p.body.at.pos, radius);
            p.body.health = body::max_health(&catalogue, &p.body.kind, &p.tardies);
            p.body.action = None;
            p.body.held = Dir::NONE;
            p.deaths += 1;
            self.events.push(Event::WentToLately { person: *id });
        }
        creature::fall(&catalogue, &mut self.state, now, &mut self.events);
    }

    fn turn_up(&mut self, id: PersonId, threshold: usize) {
        let catalogue = Arc::clone(&self.catalogue);
        let state = &mut self.state;
        let person = state.people.get_mut(&id).expect("a listed person");
        let place = catalogue.place(&person.body.at.place);
        let exit = place.thresholds[threshold].exit;
        let unworn: Vec<&Id<Mark>> = catalogue
            .marks
            .keys()
            .filter(|m| !person.tardies.contains(m))
            .collect();
        let tardy = (!unworn.is_empty())
            .then(|| unworn[state.rng.below(unworn.len() as u32) as usize].clone());
        if let Some(t) = &tardy {
            person.tardies.push(t.clone());
        }
        let radius = catalogue.body(&person.body.kind).radius;
        person.body.side = Side::Living;
        person.body.at.pos = body::settle(place, Side::Living, exit, radius);
        person.body.health = body::max_health(&catalogue, &person.body.kind, &person.tardies);
        person.body.action = None;
        self.events.push(Event::TurnedUp { person: id, tardy });
    }
}

/// Whether `target`, of `radius`, is inside the attacker's swing.
fn in_arc(
    attacker: &Body,
    kind: &crate::catalogue::BodyKind,
    target: &Body,
    radius: crate::geom::Mm,
) -> bool {
    let to = target.at.pos - attacker.at.pos;
    let reach = i64::from(kind.radius + kind.attack.reach + radius);
    if to.length_sq() > reach * reach {
        return false;
    }
    if to == Vec2::ZERO {
        return true;
    }
    let facing = attacker.facing.as_vec();
    facing.dot(to) * 1024 >= i64::from(kind.attack.arc_cos) * facing.length() * to.length()
}
