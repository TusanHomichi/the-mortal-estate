//! The bot harness: a world hosted for the run, headless players joining it
//! over real sockets, and what that cost. Each bot wanders the place, swings
//! and dodges now and then, and once it's dead walks to a threshold and
//! turns up. People can hurt each other in the harness's world, so where
//! bots crowd they die. The harness records how long each of the server's steps took
//! to compute and how long each intent took to be answered, so how many
//! players a world holds is measured, not hoped.

use std::collections::VecDeque;
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tme_link::{LinkError, join};
use tme_server::folder::{Folder, NewWorld};
use tme_server::{Options, Server};
use tme_sim::catalogue::{Catalogue, Threshold};
use tme_sim::geom::{METRE, Mm};
use tme_sim::nav::{Grid, Nav};
use tme_sim::rng::Rng;
use tme_sim::world::Settings;
use tme_sim::{Dir, Event, Intent, STEPS_PER_SECOND, Side, Vec2};
use tme_wire::{Identity, Invite, ServerMsg, You};

/// The harness's world: people can hurt each other, so a crowd of bots
/// fights and its dead walk back from Lately. A world where people die and
/// turn up is the heavier one to host.
pub const SETTINGS: Settings = Settings {
    harm_between_players: true,
};

/// A run: how many bots, for how long, and how long the world's step lasts.
#[derive(Clone, Debug)]
pub struct Plan {
    pub bots: usize,
    pub length: Duration,
    pub step: Duration,
}

/// What a run measured.
#[derive(Clone, Debug)]
pub struct Report {
    pub plan: Plan,
    /// The bots that got in and played to the end.
    pub played: usize,
    /// Why the others didn't, one line each.
    pub failed: Vec<String>,
    /// How long each step took to compute, sorted: the last minute's.
    pub steps: Vec<Duration>,
    /// How long each intent took to be answered, sorted.
    pub latency: Vec<Duration>,
    /// Intents sent and never answered.
    pub unanswered: usize,
    /// Intents the world refused.
    pub refused: usize,
    pub deaths: usize,
    pub turned_up: usize,
}

/// How long a bot waits to hear from the world before giving up on it.
const SILENCE: Duration = Duration::from_secs(5);
/// How long a bot keeps listening for answers after the run.
const DRAIN: Duration = Duration::from_secs(2);

/// Host a world running `content` for the run, set the bots loose in it,
/// and report.
pub async fn measure(content: &Path, plan: Plan) -> Result<Report, String> {
    static RUNS: AtomicUsize = AtomicUsize::new(0);
    let catalogue = tme_content::load(content)
        .map_err(|p| format!("the content doesn't compile:\n{p}"))?
        .catalogue;
    let catalogue = Arc::new(catalogue);
    let dir = std::env::temp_dir().join(format!(
        "tme-bots-{}-{}",
        std::process::id(),
        RUNS.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    Folder::create(
        &dir,
        NewWorld {
            name: "Bots",
            content,
            bind: "127.0.0.1:0",
            address: "unused:0",
            settings: SETTINGS,
        },
    )
    .map_err(|e| e.to_string())?;
    let server = Server::start_with(
        &dir,
        Options {
            step: plan.step,
            ..Options::default()
        },
    )
    .map_err(|e| e.to_string())?;
    let invite = Invite {
        address: server.local.to_string(),
        ..server.invite().clone()
    };
    let until = Instant::now() + plan.length;
    let mut bots = tokio::task::JoinSet::new();
    for n in 0..plan.bots {
        bots.spawn(play(n, invite.clone(), Arc::clone(&catalogue), until));
    }
    let mut report = Report {
        plan,
        played: 0,
        failed: Vec::new(),
        steps: Vec::new(),
        latency: Vec::new(),
        unanswered: 0,
        refused: 0,
        deaths: 0,
        turned_up: 0,
    };
    while let Some(done) = bots.join_next().await {
        match done.expect("a bot doesn't panic") {
            Ok(t) => {
                report.played += 1;
                report.latency.extend(t.latency);
                report.unanswered += t.unanswered;
                report.refused += t.refused;
                report.deaths += t.deaths;
                report.turned_up += t.turned_up;
            }
            Err(e) => report.failed.push(e.to_string()),
        }
    }
    report.steps = server.step_times();
    report.latency.sort();
    server.stop().map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(report)
}

/// What one bot saw of the run.
#[derive(Default)]
struct Tally {
    latency: Vec<Duration>,
    unanswered: usize,
    refused: usize,
    deaths: usize,
    turned_up: usize,
}

async fn play(
    n: usize,
    invite: Invite,
    catalogue: Arc<Catalogue>,
    until: Instant,
) -> Result<Tally, LinkError> {
    let (joined, mut tx, mut rx) =
        join(&invite, &Identity::generate(), &format!("Bot {n}")).await?;
    let mut mind = Mind::new(n as u64, joined.step);
    let mut tally = Tally::default();
    let mut waiting: VecDeque<(u32, Instant)> = VecDeque::new();
    loop {
        let now = Instant::now();
        if now >= until && waiting.is_empty() || now >= until + DRAIN {
            break;
        }
        let msg = match tokio::time::timeout(SILENCE, rx.next()).await {
            Ok(Some(msg)) => msg?,
            Ok(None) => return Err(LinkError::Connection("the world hung up".into())),
            Err(_) => return Err(LinkError::Connection("the world went quiet".into())),
        };
        let heard = Instant::now();
        let observation = match msg {
            ServerMsg::Observe(o) => o,
            ServerMsg::Refused { seq, .. } => {
                tally.refused += 1;
                answer(&mut waiting, seq, heard, &mut tally);
                continue;
            }
            _ => continue,
        };
        answer(&mut waiting, observation.ack, heard, &mut tally);
        for e in &observation.events {
            match e {
                Event::WentToLately { person } if *person == joined.you => tally.deaths += 1,
                Event::TurnedUp { person, .. } if *person == joined.you => tally.turned_up += 1,
                _ => {}
            }
        }
        if heard >= until || observation.you.action.is_some() {
            continue;
        }
        if let Some(intent) = mind.next(&observation.you, &catalogue, observation.step) {
            let seq = tx.act(intent).await?;
            waiting.push_back((seq, Instant::now()));
        }
    }
    tally.unanswered = waiting.len();
    tx.close().await;
    Ok(tally)
}

/// Every intent up to `seq` has been answered.
fn answer(waiting: &mut VecDeque<(u32, Instant)>, seq: u32, at: Instant, tally: &mut Tally) {
    while let Some(&(sent, when)) = waiting.front() {
        if sent > seq {
            break;
        }
        tally.latency.push(at - when);
        waiting.pop_front();
    }
}

/// A second, in the world's steps.
const SECOND: u64 = STEPS_PER_SECOND as u64;

/// What a bot means to do: wander the place along ways it can walk, swing
/// or dodge every few seconds, and when dead, walk the way to the nearest
/// threshold it can reach and use it. It keeps time in the world's steps.
struct Mind {
    rng: Rng,
    nav: Nav,
    /// The points still to walk through, and the step it gives up by.
    route: Vec<Vec2>,
    give_up: u64,
    /// The side it planned its route on: dying, or turning up, plans anew.
    side: Side,
    /// Which way it's steering, and when it may next change that.
    held: Dir,
    steer_at: u64,
    /// When it next swings, dodges or uses a threshold.
    act_at: u64,
    /// Where it was when it last checked it's getting anywhere, and when.
    progress: (Vec2, u64),
}

/// How near a point on a route counts as reached.
const REACHED: i64 = 400;
/// How far it must have got in [`STUCK`] steps to be getting anywhere.
const PROGRESS: i64 = 500;
const STUCK: u64 = 2 * SECOND;

impl Mind {
    fn new(seed: u64, step: u64) -> Mind {
        let mut rng = Rng::new(seed);
        let act_at = step + SECOND + u64::from(rng.below(3 * SECOND as u32));
        Mind {
            rng,
            nav: Nav::default(),
            route: Vec::new(),
            give_up: 0,
            side: Side::Living,
            held: Dir::NONE,
            steer_at: step,
            act_at,
            progress: (Vec2::ZERO, step),
        }
    }

    fn next(&mut self, you: &You, catalogue: &Catalogue, step: u64) -> Option<Intent> {
        let pos = you.body.pos;
        let side = you.body.side;
        let place = catalogue.place(&you.body.place);
        let radius = catalogue.body(&you.body.kind).radius;
        let grid = self.nav.grid(catalogue, &you.body.place, side, radius);
        let stuck = step >= self.progress.1 + STUCK
            && pos.distance_sq(self.progress.0) < PROGRESS * PROGRESS
            && !self.route.is_empty();
        if step >= self.progress.1 + STUCK {
            self.progress = (pos, step);
        }
        if side != self.side || stuck || step >= self.give_up {
            self.side = side;
            self.route.clear();
        }
        if side == Side::Lately {
            let within = |t: &&Threshold| pos.distance_sq(t.at) <= i64::from(t.radius).pow(2);
            if place.thresholds.iter().any(|t| within(&t)) {
                return act(&mut self.act_at, step, SECOND, || Intent::Use);
            }
            if self.route.is_empty() {
                // The nearest threshold by the way there, not as the crow
                // flies; stuck on the way, a step aside first.
                let way = if stuck {
                    None
                } else {
                    place
                        .thresholds
                        .iter()
                        .filter_map(|t| grid.path(pos, t.at))
                        .min_by_key(|way| length(pos, way))
                };
                self.route = match way {
                    Some(way) => way,
                    None => wander(&mut self.rng, grid, pos, 5 * METRE)?,
                };
                self.give_up = step + 60 * SECOND;
            }
        } else {
            let swing = act(&mut self.act_at, step, SECOND, || ());
            if swing.is_some() {
                self.act_at += u64::from(self.rng.below(3 * SECOND as u32));
                let u = i32::from(Dir::UNIT);
                let way = Dir::new(self.rng.spread(u), self.rng.spread(u));
                return Some(if self.rng.below(4) == 0 {
                    Intent::Dodge(way)
                } else {
                    Intent::Attack(way)
                });
            }
            if self.route.is_empty() {
                let b = place.bounds;
                let far = (b.max.x - b.min.x).max(b.max.y - b.min.y);
                self.route = wander(&mut self.rng, grid, pos, far)?;
                self.give_up = step + 20 * SECOND;
            }
        }
        // Walk to the furthest of the next few points in a clear line.
        while self
            .route
            .first()
            .is_some_and(|p| pos.distance_sq(*p) < REACHED * REACHED)
        {
            self.route.remove(0);
        }
        if let Some(ahead) = (1..self.route.len().min(8))
            .rev()
            .find(|i| grid.clear(pos, self.route[*i]))
        {
            self.route.drain(..ahead);
        }
        let way = self
            .route
            .first()
            .map_or(Dir::NONE, |p| Dir::toward(pos, *p));
        if way == self.held || step < self.steer_at {
            return None;
        }
        self.held = way;
        self.steer_at = step + 2;
        Some(Intent::Move(way))
    }
}

/// `what`, if it's time, and not again for `wait` steps.
fn act<T>(at: &mut u64, step: u64, wait: u64, what: impl FnOnce() -> T) -> Option<T> {
    if step < *at {
        return None;
    }
    *at = step + wait;
    Some(what())
}

/// The way to somewhere random within `within` of `pos` that can be
/// reached; `None` if nowhere tried can be.
fn wander(rng: &mut Rng, grid: &Grid, pos: Vec2, within: Mm) -> Option<Vec<Vec2>> {
    (0..8).find_map(|_| {
        let to = pos + Vec2::new(rng.spread(within), rng.spread(within));
        grid.path(pos, to)
    })
}

/// How far it is from `from` along `way`.
fn length(from: Vec2, way: &[Vec2]) -> i64 {
    std::iter::once(from)
        .chain(way.iter().copied())
        .zip(way.iter().copied())
        .map(|(a, b)| (b - a).length())
        .sum()
}

/// The value `share` of the way through `sorted`.
fn at(sorted: &[Duration], share: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let i = ((sorted.len() - 1) as f64 * share).round() as usize;
    sorted[i]
}

fn ms(d: Duration) -> String {
    format!("{:.2} ms", d.as_secs_f64() * 1000.0)
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = &self.plan;
        writeln!(
            f,
            "{} bots for {} s, a step every {}",
            p.bots,
            p.length.as_secs(),
            ms(p.step)
        )?;
        let (s, l) = (&self.steps, &self.latency);
        writeln!(
            f,
            "computing a step ({} steps): median {}, 99th percentile {}, most {}",
            s.len(),
            ms(at(s, 0.5)),
            ms(at(s, 0.99)),
            ms(at(s, 1.0)),
        )?;
        writeln!(
            f,
            "an intent answered ({} intents): median {}, 99th percentile {}, most {}",
            l.len(),
            ms(at(l, 0.5)),
            ms(at(l, 0.99)),
            ms(at(l, 1.0)),
        )?;
        writeln!(
            f,
            "played {}, refused {}, unanswered {}, died {}, turned up {}",
            self.played, self.refused, self.unanswered, self.deaths, self.turned_up
        )?;
        for why in &self.failed {
            writeln!(f, "a bot failed: {why}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tme_sim::world::Input;
    use tme_sim::{PersonId, World};

    const WREN: PersonId = PersonId(1);

    /// Wren, dead at `at` in Lately, guided by a bot's mind: the step she
    /// turned up on, if she did within a minute and a half.
    fn turns_up_from(catalogue: &Arc<Catalogue>, at: Vec2) -> Result<u64, Vec2> {
        let mut world = World::new(Arc::clone(catalogue), 7, Settings::default());
        let join = Input::Join {
            person: WREN,
            name: "Wren".into(),
        };
        world.apply(&join).expect("joins");
        let mut state = world.state().clone();
        let wren = state.people.get_mut(&WREN).expect("here");
        wren.body.side = Side::Lately;
        wren.body.at.pos = at;
        let mut world = World::load(Arc::clone(catalogue), state).expect("fits");
        let mut mind = Mind::new(1, world.step_count());
        for _ in 0..90 * SECOND {
            let you = You::of(&world, WREN).expect("here");
            if you.action.is_none()
                && let Some(intent) = mind.next(&you, catalogue, world.step_count())
            {
                let _ = world.apply(&Input::Act {
                    person: WREN,
                    intent,
                });
            }
            world.step();
            if world
                .take_events()
                .iter()
                .any(|e| matches!(e, Event::TurnedUp { person, .. } if *person == WREN))
            {
                return Ok(world.step_count());
            }
        }
        Err(You::of(&world, WREN).expect("here").body.pos)
    }

    /// How many of a crowd of bots, minds driving a world in the harness's
    /// settings, die and turn up in `seconds`.
    fn crowd(catalogue: &Arc<Catalogue>, bots: u32, seconds: u64) -> (usize, usize) {
        let mut world = World::new(Arc::clone(catalogue), 7, SETTINGS);
        let mut minds = Vec::new();
        for n in 1..=bots {
            let person = PersonId(n);
            let join = Input::Join {
                person,
                name: format!("Bot {n}"),
            };
            world.apply(&join).expect("joins");
            minds.push((person, Mind::new(u64::from(n), world.step_count())));
        }
        let (mut died, mut turned_up) = (0, 0);
        for _ in 0..seconds * SECOND {
            for (person, mind) in &mut minds {
                let you = You::of(&world, *person).expect("here");
                if you.action.is_none()
                    && let Some(intent) = mind.next(&you, catalogue, world.step_count())
                {
                    let person = *person;
                    let _ = world.apply(&Input::Act { person, intent });
                }
            }
            world.step();
            for e in world.take_events() {
                match e {
                    Event::WentToLately { .. } => died += 1,
                    Event::TurnedUp { .. } => turned_up += 1,
                    _ => {}
                }
            }
        }
        (died, turned_up)
    }

    #[test]
    fn a_crowd_of_bots_dies_and_turns_up() {
        let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let catalogue = Arc::new(tme_content::load(&content).expect("content").catalogue);
        let (died, turned_up) = crowd(&catalogue, 40, 60);
        assert!(
            died > 0 && turned_up > 0,
            "died {died}, turned up {turned_up}"
        );
    }

    #[test]
    fn a_dead_bot_walks_round_walls_to_a_threshold_and_turns_up() {
        let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let catalogue = Arc::new(tme_content::load(&content).expect("content").catalogue);
        let m = |x: i32, y: i32| Vec2::new(x * METRE, y * METRE);
        // The middle of the old hall and its far corner, whose only door
        // faces away from the threshold; behind the stage; the map's corners.
        for at in [m(17, 9), m(21, 13), m(-25, -14), m(30, -22), m(-30, 22)] {
            if let Err(stopped) = turns_up_from(&catalogue, at) {
                panic!("dead at {at:?}, never turned up: stopped at {stopped:?}");
            }
        }
    }
}
