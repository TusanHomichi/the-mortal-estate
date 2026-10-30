//! The simulation thread: the one owner of the world. It steps at a fixed
//! rate, applies inputs as they arrive from the network, sends each
//! connected person what they observe, and hands accepted inputs and
//! snapshots to the journal's writer, off its own path.

use crate::observe::{Index, Observer};
use crate::store::{Store, StoreError};
use std::collections::BTreeMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tme_sim::world::{Input, Intent, PersonId, Presence, State};
use tme_sim::{STEPS_PER_SECOND, World};
use tme_wire::{ServerMsg, Turned};
use tokio::sync::mpsc::UnboundedSender;

/// A connection, numbered by the network side.
pub type Conn = u64;

/// What the network asks of the simulation.
pub enum ToSim {
    /// A player whose key and account the network has checked.
    Join {
        conn: Conn,
        person: PersonId,
        name: String,
        out: UnboundedSender<ServerMsg>,
    },
    Act {
        conn: Conn,
        seq: u32,
        intent: Intent,
    },
    /// The connection closed.
    Leave { conn: Conn },
    /// Everyone leaves, a last snapshot is saved, and the thread ends.
    Shutdown,
}

/// What the journal's writer is handed.
enum ToWriter {
    Entries(Vec<(u64, Input)>),
    Snapshot(Box<State>),
}

/// How often a snapshot is taken.
const SNAPSHOT_EVERY: u64 = 60 * STEPS_PER_SECOND as u64;

/// How long each recent step took to compute, for the bot harness.
#[derive(Default)]
pub struct StepTimes {
    recent: Vec<Duration>,
}

impl StepTimes {
    const KEEP: usize = 20 * 60;

    fn record(&mut self, d: Duration) {
        if self.recent.len() == Self::KEEP {
            self.recent.remove(0);
        }
        self.recent.push(d);
    }

    /// The recent steps' times, sorted.
    pub fn sorted(&self) -> Vec<Duration> {
        let mut v = self.recent.clone();
        v.sort();
        v
    }
}

struct Connected {
    observer: Observer,
    out: UnboundedSender<ServerMsg>,
}

struct Sim {
    world: World,
    connected: BTreeMap<Conn, Connected>,
    /// The world as observers saw it at the last step.
    index: Index,
    /// Accepted inputs not yet handed to the writer.
    entries: Vec<(u64, Input)>,
    writer: Sender<ToWriter>,
}

impl Sim {
    /// Apply an input and keep it for the journal if it changed anything.
    fn apply(&mut self, input: Input) -> Result<(), tme_sim::Refused> {
        let step = self.world.step_count();
        self.world.apply(&input)?;
        self.entries.push((step, input));
        Ok(())
    }

    fn handle(&mut self, msg: ToSim) {
        match msg {
            ToSim::Join {
                conn,
                person,
                name,
                out,
            } => {
                let here = self.connected.values().any(|c| c.observer.person == person);
                let joined = if here {
                    Err(tme_sim::Refused::AlreadyHere)
                } else {
                    self.apply(Input::Join { person, name })
                };
                match joined {
                    Ok(()) => {
                        let _ = out.send(ServerMsg::Welcome {
                            you: person,
                            step: self.world.step_count(),
                            settings: self.world.state().settings.clone(),
                        });
                        self.connected.insert(
                            conn,
                            Connected {
                                observer: Observer::new(person),
                                out,
                            },
                        );
                    }
                    Err(_) => {
                        let _ = out.send(ServerMsg::Turned(Turned::AlreadyHere));
                    }
                }
            }
            ToSim::Act { conn, seq, intent } => {
                let Some(person) = self.connected.get(&conn).map(|c| c.observer.person) else {
                    return;
                };
                let result = self.apply(Input::Act { person, intent });
                let c = self.connected.get_mut(&conn).expect("checked above");
                c.observer.ack = seq;
                if let Err(why) = result {
                    let _ = c.out.send(ServerMsg::Refused { seq, why });
                }
            }
            ToSim::Leave { conn } => {
                if let Some(c) = self.connected.remove(&conn) {
                    let _ = self.apply(Input::Leave {
                        person: c.observer.person,
                    });
                }
            }
            ToSim::Shutdown => unreachable!("handled by the loop"),
        }
    }

    fn step(&mut self) {
        self.world.step();
        let events = self.world.take_events();
        self.index = Index::new(&self.world, &self.index);
        for c in self.connected.values_mut() {
            if let Some(o) = c.observer.observe(&self.world, &self.index, &events) {
                let _ = c.out.send(ServerMsg::Observe(o));
            }
        }
        self.flush(false);
    }

    /// Hand journal entries to the writer, and a snapshot when one is due
    /// (or `snapshot` asks for one).
    fn flush(&mut self, snapshot: bool) {
        if !self.entries.is_empty() {
            let _ = self
                .writer
                .send(ToWriter::Entries(std::mem::take(&mut self.entries)));
        }
        if snapshot || self.world.step_count().is_multiple_of(SNAPSHOT_EVERY) {
            let _ = self
                .writer
                .send(ToWriter::Snapshot(Box::new(self.world.state().clone())));
        }
    }
}

/// Restore the world from its database: the latest snapshot and the
/// journal since, replayed. Anyone who was here when the server stopped
/// has left.
pub fn restore(
    store: &mut Store,
    fresh: impl FnOnce() -> World,
    load: impl FnOnce(State) -> Result<World, String>,
) -> Result<World, String> {
    let saved = store.load().map_err(|e| e.to_string())?;
    let mut world = match saved.snapshot {
        Some(state) => load(state)?,
        None => fresh(),
    };
    for (step, input) in saved.since {
        while world.step_count() < step {
            world.step();
        }
        // Inputs were journalled only once accepted, so they're accepted again.
        world.apply(&input).map_err(|r| {
            format!("replaying the journal at step {step}: {input:?} was refused ({r})")
        })?;
    }
    let present: Vec<PersonId> = world
        .state()
        .people
        .iter()
        .filter(|(_, p)| p.presence == Presence::Here)
        .map(|(id, _)| *id)
        .collect();
    let mut left = Vec::new();
    for person in present {
        let input = Input::Leave { person };
        world.apply(&input).expect("a present person can leave");
        left.push((world.step_count(), input));
    }
    if !left.is_empty() {
        store.journal(&left).map_err(|e| e.to_string())?;
    }
    world.take_events();
    Ok(world)
}

/// Run the world on its own thread, and the journal's writer on another.
pub fn spawn(
    world: World,
    store: Store,
    inbox: Receiver<ToSim>,
    times: Arc<Mutex<StepTimes>>,
    tick: Duration,
) -> JoinHandle<Result<(), StoreError>> {
    let (writer_tx, writer_rx) = mpsc::channel();
    let writer = thread::Builder::new()
        .name("journal".into())
        .spawn(move || write(store, writer_rx))
        .expect("a thread starts");
    thread::Builder::new()
        .name("world".into())
        .spawn(move || {
            let mut sim = Sim {
                world,
                connected: BTreeMap::new(),
                index: Index::default(),
                entries: Vec::new(),
                writer: writer_tx,
            };
            run(&mut sim, &inbox, &times, tick);
            // Everyone leaves; the last snapshot holds the world as it stopped.
            let conns: Vec<Conn> = sim.connected.keys().copied().collect();
            for conn in conns {
                sim.handle(ToSim::Leave { conn });
            }
            sim.flush(true);
            drop(sim);
            writer.join().expect("the writer doesn't panic")
        })
        .expect("a thread starts")
}

fn run(sim: &mut Sim, inbox: &Receiver<ToSim>, times: &Mutex<StepTimes>, tick: Duration) {
    let mut next = Instant::now() + tick;
    loop {
        // Inputs are applied the moment they arrive, until the step is due.
        loop {
            let now = Instant::now();
            if now >= next {
                break;
            }
            match inbox.recv_timeout(next - now) {
                Ok(ToSim::Shutdown) | Err(RecvTimeoutError::Disconnected) => return,
                Ok(msg) => sim.handle(msg),
                Err(RecvTimeoutError::Timeout) => break,
            }
        }
        let started = Instant::now();
        sim.step();
        times
            .lock()
            .expect("not poisoned")
            .record(started.elapsed());
        next += tick;
        // Far behind (a stalled machine): carry on from now rather than race.
        if Instant::now() > next + Duration::from_secs(1) {
            next = Instant::now() + tick;
        }
    }
}

fn write(mut store: Store, inbox: Receiver<ToWriter>) -> Result<(), StoreError> {
    while let Ok(first) = inbox.recv() {
        // Everything waiting goes in one go: entries in one transaction.
        let mut entries = Vec::new();
        for msg in std::iter::once(first).chain(inbox.try_iter()) {
            match msg {
                ToWriter::Entries(e) => entries.extend(e),
                ToWriter::Snapshot(state) => {
                    store.journal(&std::mem::take(&mut entries))?;
                    store.snapshot(&state)?;
                }
            }
        }
        if !entries.is_empty() {
            store.journal(&entries)?;
        }
    }
    Ok(())
}
