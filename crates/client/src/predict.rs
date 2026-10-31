//! The player's own body, walked here as the world will walk it, so moving
//! answers the keys at once instead of a round trip later. The world's word
//! still wins: each observation is taken as where the body was a round trip
//! ago, and the steps walked since are walked again from there.

use crate::net::Net;
use crate::seen::{Content, Seen, ground};
use bevy::prelude::*;
use std::collections::VecDeque;
use tme_sim::geom::METRE;
use tme_sim::{Dir, Intent, STEP_MS, Vec2, body};

pub fn plugin(app: &mut App) {
    app.init_resource::<Me>()
        .add_systems(Update, (reconcile, walk, show).chain().in_set(Predicted));
}

/// Where the player's body is drawn is known for this frame.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Predicted;

/// The player's body as this client predicts it.
#[derive(Resource, Default)]
pub struct Me {
    /// Whether an observation has said where we are yet.
    pub known: bool,
    /// Where the body is, in the world's millimetres.
    pub pos: Vec2,
    /// Where it's drawn: `pos`, with any correction eased in.
    pub shown: Vec3,
    /// Which way it's drawn facing.
    pub facing: Dir,
    /// The way the player is steering, as far as the world knows.
    held: Dir,
    /// The last steer sent, by its number.
    moved: u32,
    /// What each step walked here did, newest last: replayed from the
    /// world's word when it comes.
    walked: VecDeque<Dir>,
    /// Time not yet walked, in seconds.
    clock: f64,
    /// A correction still being eased out.
    offset: Vec3,
    /// The intent whose answer is being timed, and when it went.
    timing: Option<(u32, f64)>,
    /// A round trip, in seconds: how stale an observation is when it comes.
    rtt: f64,
    /// An attack or dodge sent and not yet acknowledged: nothing is walked
    /// until the world says what came of it.
    pending: Option<u32>,
    /// The observation last reconciled with.
    step: u64,
}

/// How far a correction can be before it's jumped rather than eased.
const SNAP: i64 = 2 * METRE as i64;
/// The most steps kept for replaying.
const KEPT: usize = 40;

impl Me {
    /// Steer: sent only when it isn't the way the world holds.
    pub fn steer(&mut self, dir: Dir, net: &mut Net, now: f64) {
        if dir == self.held {
            return;
        }
        self.held = dir;
        let seq = net.act(Intent::Move(dir));
        self.moved = seq;
        self.time(seq, now);
    }

    /// Start something the body is then committed to, if the world agrees:
    /// a swing, a dash, a rite.
    pub fn commit(&mut self, intent: Intent, net: &mut Net, now: f64) {
        let seq = net.act(intent);
        self.pending = Some(seq);
        self.time(seq, now);
    }

    /// Whether the body walked on the latest step.
    pub fn walking(&self) -> bool {
        self.walked.back().is_some_and(|d| !d.is_none())
    }

    fn time(&mut self, seq: u32, now: f64) {
        if self.timing.is_none() {
            self.timing = Some((seq, now));
        }
    }

    /// Whether the body is in the middle of something, as far as anyone
    /// here knows.
    fn committed(&self, seen: &Seen) -> bool {
        self.pending.is_some() || seen.you.as_ref().is_some_and(|y| y.action.is_some())
    }
}

/// Take each new observation as where the body was a round trip ago, and
/// walk the steps since again from there.
fn reconcile(mut me: ResMut<Me>, seen: Res<Seen>, content: Res<Content>, time: Res<Time>) {
    let Some(you) = &seen.you else {
        return;
    };
    if me.known && seen.step == me.step {
        return;
    }
    me.step = seen.step;
    let now = time.elapsed_secs_f64();
    if let Some((seq, sent)) = me.timing
        && seen.ack >= seq
    {
        let sample = now - sent;
        me.rtt = if me.rtt == 0.0 {
            sample
        } else {
            me.rtt * 0.8 + sample * 0.2
        };
        me.timing = None;
    }
    if me.pending.is_some_and(|seq| seen.ack >= seq) {
        me.pending = None;
    }
    // Once the world has every steer sent, the way it holds is the way the
    // body goes. If it let go of one (dying does), the keys' way differs and
    // goes again.
    if seen.ack >= me.moved {
        me.held = you.held;
    }
    let body = &you.body;
    if you.action.is_some() || !me.known {
        // Committed: the world moves the body (a dash, a rite), not the
        // keys. Follow it.
        me.walked.clear();
        settle(&mut me, body.pos);
        me.facing = body.facing;
        me.known = true;
        return;
    }
    let catalogue = &content.0.catalogue;
    let place = catalogue.place(&body.place);
    let speed = body::speed(catalogue, &body.kind, &you.tardies);
    let radius = catalogue.body(&body.kind).radius;
    let behind = ((me.rtt * 1000.0) / f64::from(STEP_MS)).round() as usize;
    let mut pos = body.pos;
    for dir in me
        .walked
        .iter()
        .skip(me.walked.len().saturating_sub(behind))
    {
        pos = body::walk(place, body.side, pos, radius, dir.travel(speed));
    }
    settle(&mut me, pos);
}

/// Move to `pos`: eased if it's near, jumped if it isn't.
fn settle(me: &mut Me, pos: Vec2) {
    let was = ground(me.pos) + me.offset;
    me.offset = if me.known && pos.distance_sq(me.pos) <= SNAP * SNAP {
        was - ground(pos)
    } else {
        Vec3::ZERO
    };
    me.pos = pos;
}

/// Walk the steps that have come due, the way the world walks them.
fn walk(mut me: ResMut<Me>, seen: Res<Seen>, content: Res<Content>, time: Res<Time>) {
    let Some(you) = &seen.you else {
        return;
    };
    me.clock += time.delta_secs_f64();
    let step = f64::from(STEP_MS) / 1000.0;
    let catalogue = &content.0.catalogue;
    let body = &you.body;
    let place = catalogue.place(&body.place);
    let speed = body::speed(catalogue, &body.kind, &you.tardies);
    let radius = catalogue.body(&body.kind).radius;
    while me.clock >= step {
        me.clock -= step;
        let dir = if me.committed(&seen) {
            Dir::NONE
        } else {
            me.held
        };
        if let Some(face) = dir.full() {
            me.pos = body::walk(place, body.side, me.pos, radius, dir.travel(speed));
            me.facing = face;
        }
        me.walked.push_back(dir);
        if me.walked.len() > KEPT {
            me.walked.pop_front();
        }
    }
}

/// Ease any correction out, a tenth of a second or so.
fn show(mut me: ResMut<Me>, time: Res<Time>) {
    let keep = (-time.delta_secs() * 12.0).exp();
    me.offset *= keep;
    me.shown = ground(me.pos) + me.offset;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use std::path::Path;
    use std::sync::Arc;
    use tme_sim::world::{Input, PersonId, Settings};
    use tme_wire::You;
    use tokio::sync::mpsc::UnboundedReceiver;

    const ME: PersonId = PersonId(1);

    /// A client that has joined Lattermead, and what it sends.
    struct Client {
        app: World,
        net: Net,
        sent: UnboundedReceiver<Intent>,
        /// The player as the world first shows them.
        you: You,
    }

    impl Client {
        fn new() -> Client {
            let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
            let content = tme_content::load(&dir).expect("content/ compiles");
            let catalogue = Arc::new(content.catalogue.clone());
            let mut world = tme_sim::World::new(catalogue, 1, Settings::default());
            world
                .apply(&Input::Join {
                    person: ME,
                    name: "Me".into(),
                })
                .unwrap();
            let you = You::of(&world, ME).unwrap();
            let mut app = World::new();
            app.insert_resource(Content(Arc::new(content)));
            app.init_resource::<Seen>();
            app.init_resource::<Me>();
            app.insert_resource(Time::<()>::default());
            let (net, sent) = Net::offline();
            Client {
                app,
                net,
                sent,
                you,
            }
        }

        fn steer(&mut self, dir: Dir) {
            self.app.resource_mut::<Me>().steer(dir, &mut self.net, 0.0);
        }

        /// The world says it has applied intents up to `ack` and holds the
        /// player steering `held`.
        fn observe(&mut self, ack: u32, held: Dir) {
            let mut seen = self.app.resource_mut::<Seen>();
            seen.step += 1;
            seen.ack = ack;
            seen.you = Some(You {
                held,
                ..self.you.clone()
            });
            self.app.run_system_once(reconcile).unwrap();
        }

        fn sent(&mut self) -> Vec<Intent> {
            std::iter::from_fn(|| self.sent.try_recv().ok()).collect()
        }
    }

    #[test]
    fn a_steer_the_world_lets_go_of_is_sent_again() {
        let mut client = Client::new();
        client.steer(Dir::NORTH);
        client.observe(1, Dir::NORTH);
        client.steer(Dir::NORTH);
        assert_eq!(client.sent(), [Intent::Move(Dir::NORTH)]);
        // The player dies with the key still down: the world lets go.
        client.observe(1, Dir::NONE);
        assert_eq!(client.app.resource::<Me>().held, Dir::NONE, "walks on");
        client.steer(Dir::NORTH);
        assert_eq!(client.sent(), [Intent::Move(Dir::NORTH)]);
        // Let go of in turn, nothing more needs saying.
        client.observe(2, Dir::NORTH);
        client.steer(Dir::NONE);
        client.observe(3, Dir::NONE);
        client.steer(Dir::NONE);
        assert_eq!(client.sent(), [Intent::Move(Dir::NONE)]);
    }

    #[test]
    fn a_steer_on_its_way_is_not_taken_back() {
        let east = Dir::new(Dir::UNIT.into(), 0);
        let mut client = Client::new();
        client.steer(Dir::NORTH);
        client.steer(east);
        // The world has only the first.
        client.observe(1, Dir::NORTH);
        client.steer(east);
        client.observe(2, east);
        client.steer(east);
        assert_eq!(
            client.sent(),
            [Intent::Move(Dir::NORTH), Intent::Move(east)]
        );
    }
}
