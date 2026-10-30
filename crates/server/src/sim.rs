//! The simulation thread: the one owner of the world. It steps at a fixed
//! rate, applies inputs as they arrive from the network, sends each
//! connected person what they observe, and hands accepted inputs, each
//! step's completion and snapshots to the journal's writer, off its own
//! path. If the writer fails, the world stops: play that isn't saved isn't
//! offered.

use crate::observe::{Index, Observer};
use crate::store::{ContentId, Store, StoreError};
use std::collections::BTreeMap;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tme_sim::world::{Input, Intent, PersonId, Presence, Settings, State};
use tme_sim::{Catalogue, STEPS_PER_SECOND, World};
use tme_wire::{ServerMsg, Turned};
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{Receiver as Inbox, Sender as Outbox};

/// A connection, numbered by the network side.
pub type Conn = u64;

/// What the network asks of the simulation.
pub enum ToSim {
    /// A player whose key and account the network has checked.
    Join {
        conn: Conn,
        person: PersonId,
        name: String,
        out: Outbox<ServerMsg>,
    },
    Act {
        conn: Conn,
        seq: u32,
        intent: Intent,
    },
    /// The connection closed.
    Leave { conn: Conn },
}

/// How many messages the world will hold for a client still sending the
/// last ones: a few seconds of steps. One that falls further behind is let
/// go, as every observation builds on the one before.
pub const OUTBOX: usize = 64;

/// How many messages from the network can wait for the world.
pub const INBOX: usize = 4096;

/// What the journal's writer is handed.
enum ToWriter {
    /// Inputs accepted since the last, and the steps completed now.
    Entries {
        entries: Vec<(u64, Input)>,
        stepped: u64,
    },
    Snapshot(Box<State>),
}

/// How many batches the writer can fall behind before the world waits for
/// it: about three seconds of steps.
const WRITER_BACKLOG: usize = 64;

/// The writer has stopped: it failed, and says why when it's joined.
struct WriterGone;

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
    out: Outbox<ServerMsg>,
}

struct Sim {
    world: World,
    connected: BTreeMap<Conn, Connected>,
    /// The world as observers saw it at the last step.
    index: Index,
    /// Accepted inputs not yet handed to the writer.
    entries: Vec<(u64, Input)>,
    writer: SyncSender<ToWriter>,
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
                        let _ = out.try_send(ServerMsg::Welcome {
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
                        let _ = out.try_send(ServerMsg::Turned(Turned::AlreadyHere));
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
                    self.send(conn, ServerMsg::Refused { seq, why });
                }
            }
            ToSim::Leave { conn } => self.leave(conn),
        }
    }

    /// Hand `msg` to `conn`'s connection, or let them go if they've fallen
    /// too far behind to take it.
    fn send(&mut self, conn: Conn, msg: ServerMsg) {
        let Some(c) = self.connected.get(&conn) else {
            return;
        };
        if let Err(TrySendError::Full(_)) = c.out.try_send(msg) {
            self.leave(conn);
        }
    }

    /// `conn` has gone, or been let go: their person leaves. Dropping their
    /// outbox closes the connection, if it's still open.
    fn leave(&mut self, conn: Conn) {
        if let Some(c) = self.connected.remove(&conn) {
            let _ = self.apply(Input::Leave {
                person: c.observer.person,
            });
        }
    }

    fn step(&mut self) -> Result<(), WriterGone> {
        self.world.step();
        let events = self.world.take_events();
        self.index = Index::new(&self.world, &self.index);
        let mut seen = Vec::new();
        for (conn, c) in &mut self.connected {
            if let Some(o) = c.observer.observe(&self.world, &self.index, &events) {
                seen.push((*conn, o));
            }
        }
        for (conn, o) in seen {
            self.send(conn, ServerMsg::Observe(Box::new(o)));
        }
        self.flush(false)
    }

    /// Hand the writer the inputs accepted since the last flush and the
    /// steps completed, and a snapshot when one is due (or `snapshot` asks
    /// for one). It waits while the writer is far behind.
    fn flush(&mut self, snapshot: bool) -> Result<(), WriterGone> {
        let entries = ToWriter::Entries {
            entries: std::mem::take(&mut self.entries),
            stepped: self.world.step_count(),
        };
        self.writer.send(entries).map_err(|_| WriterGone)?;
        if snapshot || self.world.step_count().is_multiple_of(SNAPSHOT_EVERY) {
            let state = Box::new(self.world.state().clone());
            self.writer
                .send(ToWriter::Snapshot(state))
                .map_err(|_| WriterGone)?;
        }
        Ok(())
    }
}

/// A world restored and saved again, ready to run.
pub struct Restored {
    pub world: World,
    /// The content it runs, as the store numbers it.
    pub content: ContentId,
}

/// Restore the world from its database and make it ready to run on
/// `catalogue` with `settings`. What was saved is replayed on the content
/// it was played on: the latest snapshot, the journal since, and every step
/// completed after. Anyone who was here when the server stopped leaves.
/// Then the world moves onto `catalogue` and `settings`, which it must fit,
/// and that's saved as a snapshot along with the leaving. Nothing is
/// written unless all of it works. A world never saved starts from `seed`.
pub fn restore(
    store: &mut Store,
    catalogue: Arc<Catalogue>,
    seed: u64,
    settings: Settings,
) -> Result<Restored, String> {
    let saved = store.load().map_err(|e| e.to_string())?;
    let mut world = match saved.snapshot {
        Some((state, played_on)) => {
            World::load(Arc::new(played_on), state).map_err(|e| format!("the saved world: {e}"))?
        }
        None if saved.since.is_empty() => {
            World::new(Arc::clone(&catalogue), seed, settings.clone())
        }
        None => return Err("the world's database has a journal and no snapshot".into()),
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
    while world.step_count() < saved.stepped {
        world.step();
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
    let mut state = world.state().clone();
    state.settings = settings;
    let world = World::load(catalogue, state).map_err(|e| e.to_string())?;
    let content = store
        .begin(&left, world.state(), world.catalogue())
        .map_err(|e| e.to_string())?;
    Ok(Restored { world, content })
}

/// Run the world on its own thread, and the journal's writer on another.
/// The world's thread ends when the inbox closes, after a last snapshot,
/// or as soon as the writer fails, with its error.
pub fn spawn(
    restored: Restored,
    store: Store,
    inbox: Inbox<ToSim>,
    times: Arc<Mutex<StepTimes>>,
    tick: Duration,
) -> JoinHandle<Result<(), String>> {
    let (writer_tx, writer_rx) = mpsc::sync_channel(WRITER_BACKLOG);
    let content = restored.content;
    let writer = thread::Builder::new()
        .name("journal".into())
        .spawn(move || write(store, content, writer_rx))
        .expect("a thread starts");
    thread::Builder::new()
        .name("world".into())
        .spawn(move || {
            let mut sim = Sim {
                world: restored.world,
                connected: BTreeMap::new(),
                index: Index::default(),
                entries: Vec::new(),
                writer: writer_tx,
            };
            let ran = run(&mut sim, inbox, &times, tick).and_then(|()| {
                // Everyone leaves; the last snapshot holds the world as it
                // stopped.
                let conns: Vec<Conn> = sim.connected.keys().copied().collect();
                for conn in conns {
                    sim.leave(conn);
                }
                sim.flush(true)
            });
            drop(sim);
            let wrote = match writer.join() {
                Ok(wrote) => wrote.map_err(|e| format!("saving the world: {e}")),
                Err(_) => Err("saving the world: the journal's writer panicked".to_owned()),
            };
            match (ran, wrote) {
                (_, Err(e)) => Err(e),
                (Err(WriterGone), Ok(())) => Err("saving the world: the writer stopped".into()),
                (Ok(()), Ok(())) => Ok(()),
            }
        })
        .expect("a thread starts")
}

fn run(
    sim: &mut Sim,
    mut inbox: Inbox<ToSim>,
    times: &Mutex<StepTimes>,
    tick: Duration,
) -> Result<(), WriterGone> {
    // The network hands inputs over on an async channel, bounded so it
    // waits when the world's behind; this runtime is only the waiting.
    let waiting = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("a runtime to wait in");
    let mut next = Instant::now() + tick;
    loop {
        // Inputs are applied the moment they arrive, until the step is due.
        loop {
            let due = tokio::time::Instant::from_std(next);
            match waiting.block_on(async { tokio::time::timeout_at(due, inbox.recv()).await }) {
                Ok(Some(msg)) => sim.handle(msg),
                Ok(None) => return Ok(()),
                Err(_) => break,
            }
        }
        let started = Instant::now();
        sim.step()?;
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

fn write(
    mut store: Store,
    content: ContentId,
    inbox: Receiver<ToWriter>,
) -> Result<(), StoreError> {
    while let Ok(first) = inbox.recv() {
        // Everything waiting goes in one go: entries in one transaction.
        let mut entries = Vec::new();
        let mut stepped = 0;
        for msg in std::iter::once(first).chain(inbox.try_iter()) {
            match msg {
                ToWriter::Entries {
                    entries: e,
                    stepped: s,
                } => {
                    entries.extend(e);
                    stepped = s;
                }
                ToWriter::Snapshot(state) => {
                    store.journal(&std::mem::take(&mut entries), stepped)?;
                    store.snapshot(&state, content)?;
                }
            }
        }
        store.journal(&entries, stepped)?;
    }
    Ok(())
}
