//! What the client knows of the world: everything it has observed, kept
//! for drawing, and the news worth telling the player.

use crate::net::{FromWorld, Net};
use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tme_sim::world::Refused;
use tme_sim::{BodyId, Event, STEPS_PER_SECOND};
use tme_wire::{BodyView, ServerMsg, You};

pub fn plugin(app: &mut App) {
    app.init_resource::<Seen>().add_systems(PreUpdate, receive);
}

/// What the world runs on, compiled from the content folder.
#[derive(Resource)]
pub struct Content(pub Arc<tme_content::Content>);

/// Everything observed so far.
#[derive(Resource, Default)]
pub struct Seen {
    /// The player's own state, as of the latest observation.
    pub you: Option<You>,
    /// The latest observation's step, and when it arrived (seconds of the
    /// app's clock).
    pub step: u64,
    pub arrived: f64,
    /// The last intent the world has applied.
    pub ack: u32,
    /// Everyone else in view.
    pub bodies: HashMap<BodyId, Tracked>,
    /// Why the world let go of us, once it has.
    pub closed: Option<String>,
    /// Lines for the player, newest last, with when they were told.
    pub news: VecDeque<(f64, String)>,
    /// What happened in view since last frame.
    pub events: Vec<Event>,
}

/// A body in view, and where it's been seen lately.
pub struct Tracked {
    pub view: BodyView,
    /// (step, place) pairs, oldest first.
    pub trail: VecDeque<(u64, tme_sim::Vec2)>,
}

/// How many observations of a body to keep for drawing it between them.
const TRAIL: usize = 8;
/// How many news lines to keep.
const NEWS: usize = 6;

impl Seen {
    /// The world's step about now, between whole steps, by the clock since
    /// the latest observation.
    pub fn now(&self, elapsed: f64) -> f64 {
        self.step as f64 + (elapsed - self.arrived) * f64::from(STEPS_PER_SECOND)
    }

    fn tell(&mut self, at: f64, line: String) {
        self.news.push_back((at, line));
        while self.news.len() > NEWS {
            self.news.pop_front();
        }
    }

    /// What to call a body in a sentence.
    fn name(&self, id: BodyId, content: &tme_content::Content) -> String {
        if self.you.as_ref().is_some_and(|y| y.body.id == id) {
            return "you".into();
        }
        match self.bodies.get(&id) {
            Some(t) => match &t.view.name {
                Some(n) => n.clone(),
                None => format!(
                    "the {}",
                    content.catalogue.body(&t.view.kind).name.to_lowercase()
                ),
            },
            None => "something".into(),
        }
    }

    /// The news an event makes, if any.
    fn news_of(&self, e: &Event, content: &tme_content::Content) -> Option<String> {
        let me = self.you.as_ref()?.body.id;
        let person = |p| self.name(BodyId::Person(p), content);
        Some(match e {
            Event::Struck { by, target, damage } if *target == me || *by == me => {
                let line = format!(
                    "{} struck {} ({damage})",
                    self.name(*by, content),
                    self.name(*target, content)
                );
                capitalised(&line)
            }
            Event::WentToLately { person: p } if BodyId::Person(*p) == me => {
                "You died where you stood, and went to Lately.".into()
            }
            Event::WentToLately { person: p } => format!("{} went to Lately.", person(*p)),
            Event::TurnedUp { person: p, tardy } => {
                let who = capitalised(&person(*p));
                match tardy {
                    Some(t) => format!(
                        "{who} turned up, marked {}.",
                        content.catalogue.marks[t].name
                    ),
                    None => format!("{who} turned up."),
                }
            }
            Event::Fell { creature } => format!(
                "{} fell.",
                capitalised(&self.name(BodyId::Creature(*creature), content))
            ),
            Event::Joined { person: p } if BodyId::Person(*p) != me => {
                format!("{} arrived.", person(*p))
            }
            Event::Left { person: p } if BodyId::Person(*p) != me => {
                format!("{} left.", person(*p))
            }
            _ => return None,
        })
    }
}

fn capitalised(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

impl Tracked {
    /// Where it stood at step `t`, between the observations either side.
    pub fn at(&self, t: f64) -> Vec3 {
        let mut before = self.trail.front().copied();
        for &(step, pos) in &self.trail {
            if step as f64 <= t {
                before = Some((step, pos));
            } else {
                let Some((s0, p0)) = before else {
                    return ground(pos);
                };
                let f = ((t - s0 as f64) / (step - s0).max(1) as f64).clamp(0.0, 1.0) as f32;
                return ground(p0).lerp(ground(pos), f);
            }
        }
        before.map_or(Vec3::ZERO, |(_, p)| ground(p))
    }
}

/// Where a model the content names is, among the assets: content names
/// them from `assets/models/`.
pub fn model(name: &str) -> String {
    format!("models/{name}")
}

/// A point on the ground in the world's millimetres, as a point in the
/// scene: x east, y up, north into the screen.
pub fn ground(p: tme_sim::Vec2) -> Vec3 {
    Vec3::new(p.x as f32 / 1000.0, 0.0, -(p.y as f32) / 1000.0)
}

/// The turn that faces a model (made facing +Z) along `dir`.
pub fn facing(dir: tme_sim::Dir) -> Quat {
    Quat::from_rotation_y((dir.x() as f32).atan2(-(dir.y() as f32)))
}

fn receive(net: Res<Net>, mut seen: ResMut<Seen>, time: Res<Time>, content: Res<Content>) {
    let now = time.elapsed_secs_f64();
    seen.events.clear();
    for msg in net.drain() {
        let msg = match msg {
            FromWorld::Said(said) => Ok(said),
            FromWorld::Closed(why) => Err(why),
        };
        match msg {
            Ok(ServerMsg::Observe(o)) => {
                seen.step = o.step;
                seen.arrived = now;
                seen.ack = o.ack;
                seen.you = Some(o.you);
                for view in o.seen {
                    let tracked = seen.bodies.entry(view.id).or_insert_with(|| Tracked {
                        view: view.clone(),
                        trail: VecDeque::new(),
                    });
                    tracked.trail.push_back((o.step, view.pos));
                    if tracked.trail.len() > TRAIL {
                        tracked.trail.pop_front();
                    }
                    tracked.view = view;
                }
                for event in &o.events {
                    if let Some(line) = seen.news_of(event, &content.0) {
                        seen.tell(now, line);
                    }
                }
                for id in o.gone {
                    seen.bodies.remove(&id);
                }
                seen.events.extend(o.events);
            }
            Ok(ServerMsg::Refused { why, .. }) => {
                let line = match why {
                    Refused::NothingToUse => "There's nothing here to use.".to_owned(),
                    Refused::TooMarked => {
                        "This life carries too many tardies to turn up again.".to_owned()
                    }
                    // Pressing during a swing is ordinary; no need to say.
                    Refused::Committed => continue,
                    other => format!("Can't: {other}."),
                };
                seen.tell(now, line);
            }
            Ok(ServerMsg::Turned(t)) => {
                seen.closed = Some(format!("The world turned us away: {t:?}"));
            }
            Ok(_) => {}
            Err(why) => seen.closed = Some(why),
        }
    }
}
