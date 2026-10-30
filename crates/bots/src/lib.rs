//! The bot harness: a world hosted for the run, headless players joining it
//! over real sockets, and what that cost. Each bot wanders the place, swings
//! and dodges now and then, and once it's dead walks to a threshold and
//! turns up. The harness records how long each of the server's steps took
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
use tme_sim::catalogue::Catalogue;
use tme_sim::rng::Rng;
use tme_sim::{Dir, Event, Intent, Side, Vec2};
use tme_wire::{Identity, Invite, ServerMsg, You};

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
        },
    )
    .map_err(|e| e.to_string())?;
    let server =
        Server::start_with(&dir, Options { step: plan.step }).map_err(|e| e.to_string())?;
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
    let mut mind = Mind::new(n as u64, Instant::now());
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
        if let Some(intent) = mind.next(&observation.you, &catalogue, heard) {
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

/// What a bot means to do: walk somewhere at random, swing or dodge every
/// few seconds, and when dead, make for the nearest threshold and use it.
struct Mind {
    rng: Rng,
    /// Where it's walking, and when it gives up getting there.
    going: Option<(Vec2, Instant)>,
    /// When it next sets its course again, and next acts.
    steer_at: Instant,
    act_at: Instant,
}

impl Mind {
    fn new(seed: u64, now: Instant) -> Mind {
        let mut rng = Rng::new(seed);
        let act_at = now + Duration::from_millis(u64::from(1000 + rng.below(3000)));
        Mind {
            rng,
            going: None,
            steer_at: now,
            act_at,
        }
    }

    fn next(&mut self, you: &You, catalogue: &Catalogue, now: Instant) -> Option<Intent> {
        let pos = you.body.pos;
        let place = catalogue.place(&you.body.place);
        if you.body.side == Side::Lately {
            let t = place
                .thresholds
                .iter()
                .min_by_key(|t| t.at.distance_sq(pos))?;
            if pos.distance_sq(t.at) <= i64::from(t.radius).pow(2) {
                return self.act(now, Duration::from_secs(1), |_| Intent::Use);
            }
            return self.steer(now, Dir::toward(pos, t.at));
        }
        let wait = Duration::from_millis(u64::from(1000 + self.rng.below(3000)));
        if let Some(swing) = self.act(now, wait, |rng| {
            let u = i32::from(Dir::UNIT);
            let way = Dir::new(rng.spread(u), rng.spread(u));
            if rng.below(4) == 0 {
                Intent::Dodge(way)
            } else {
                Intent::Attack(way)
            }
        }) {
            return Some(swing);
        }
        let there = |(to, by): (Vec2, Instant)| pos.distance_sq(to) < 1_000_000 || now >= by;
        if self.going.is_none_or(there) {
            let b = place.bounds;
            let to = Vec2::new(
                (b.min.x + b.max.x) / 2 + self.rng.spread((b.max.x - b.min.x) / 2),
                (b.min.y + b.max.y) / 2 + self.rng.spread((b.max.y - b.min.y) / 2),
            );
            self.going = Some((to, now + Duration::from_secs(15)));
            self.steer_at = now;
        }
        let (to, _) = self.going?;
        self.steer(now, Dir::toward(pos, to))
    }

    /// Set course, at most once a second.
    fn steer(&mut self, now: Instant, way: Dir) -> Option<Intent> {
        if now < self.steer_at {
            return None;
        }
        self.steer_at = now + Duration::from_secs(1);
        Some(Intent::Move(way))
    }

    /// Do something, if it's time, and not again for `wait`.
    fn act(
        &mut self,
        now: Instant,
        wait: Duration,
        what: impl FnOnce(&mut Rng) -> Intent,
    ) -> Option<Intent> {
        if now < self.act_at {
            return None;
        }
        self.act_at = now + wait;
        Some(what(&mut self.rng))
    }
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
