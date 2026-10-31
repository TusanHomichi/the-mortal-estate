//! How bodies look to an observer: what a client needs to draw them, and no
//! more.

use serde::{Deserialize, Serialize};
use tme_sim::body::{self, Action, Phase};
use tme_sim::catalogue::{BodyKind, Mark, Place};
use tme_sim::{BodyId, Dir, Id, Side, Vec2, World};

/// A body as anyone in view sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyView {
    pub id: BodyId,
    pub kind: Id<BodyKind>,
    /// A person's name; creatures have none.
    pub name: Option<String>,
    pub place: Id<Place>,
    pub level: i8,
    pub pos: Vec2,
    pub side: Side,
    pub facing: Dir,
    pub health: i32,
    pub max_health: i32,
    /// Where it is in what it's doing, so a wind-up shows.
    pub phase: Option<Phase>,
    /// The step that phase's action began on.
    pub since: u64,
    /// Whether it's walking.
    pub moving: bool,
}

impl BodyView {
    /// `id` as it stands in `world` now.
    pub fn of(world: &World, id: BodyId) -> Option<BodyView> {
        let b = world.body(id)?;
        let catalogue = world.catalogue();
        let kind = catalogue.body(&b.kind);
        let (name, max_health) = match id {
            BodyId::Person(p) => {
                let person = world.person(p)?;
                (
                    Some(person.name.clone()),
                    body::max_health(catalogue, &b.kind, &person.tardies),
                )
            }
            BodyId::Creature(_) => (None, kind.health),
        };
        Some(BodyView {
            id,
            kind: b.kind.clone(),
            name,
            place: b.at.place.clone(),
            level: b.at.level,
            pos: b.at.pos,
            side: b.side,
            facing: b.facing,
            health: b.health,
            max_health,
            phase: b.phase(kind, world.step_count()),
            since: b.action.as_ref().map_or(0, |a| a.started),
            moving: b.action.is_none() && !b.held.is_none(),
        })
    }
}

/// A person's own state, in full: enough to predict their own walking.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct You {
    pub body: BodyView,
    pub tardies: Vec<Id<Mark>>,
    pub deaths: u32,
    /// What they're committed to, if anything.
    pub action: Option<Action>,
    /// The way the world holds them steering. It can let go of a steer on
    /// its own (dying does), so a client can't take its last one as held.
    pub held: Dir,
    /// Whether the dead show clearly to them.
    pub sees_lately: bool,
}

impl You {
    pub fn of(world: &World, id: tme_sim::PersonId) -> Option<You> {
        let person = world.person(id)?;
        Some(You {
            body: BodyView::of(world, BodyId::Person(id))?,
            tardies: person.tardies.clone(),
            deaths: person.deaths,
            action: person.body.action.clone(),
            held: person.body.held,
            sees_lately: body::sees_lately(world.catalogue(), &person.tardies),
        })
    }
}
