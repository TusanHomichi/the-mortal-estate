//! A world's server: the only authority over one persistent world. It
//! holds the world in memory, steps it on its own thread, journals what it
//! accepts and snapshots what it is, and talks to clients over WebSockets.
//! Anyone can host one: one binary, one folder.

pub mod folder;
pub mod observe;
pub mod sim;
pub mod store;

mod net;

use folder::Folder;
use sim::{StepTimes, ToSim};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use store::Store;
use tme_wire::Invite;

/// A running server.
pub struct Server {
    /// Where it's listening.
    pub local: SocketAddr,
    invite: Invite,
    sim: mpsc::Sender<ToSim>,
    world: Option<JoinHandle<Result<(), String>>>,
    net: Option<(tokio::sync::oneshot::Sender<()>, JoinHandle<()>)>,
    times: Arc<Mutex<StepTimes>>,
    /// Closed when the world has stopped, asked to or not.
    stopped: tokio::sync::watch::Receiver<()>,
}

#[derive(Debug)]
pub struct StartError(pub String);

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for StartError {}

/// How a server runs, beyond what its world folder says.
#[derive(Clone, Debug)]
pub struct Options {
    /// How long a step lasts on the wall clock. Tests shorten it; the world
    /// itself only counts steps.
    pub step: std::time::Duration,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            step: std::time::Duration::from_millis(u64::from(tme_sim::STEP_MS)),
        }
    }
}

impl Server {
    /// Open the world in `path`, restore it and start serving it.
    pub fn start(path: &Path) -> Result<Server, StartError> {
        Server::start_with(path, Options::default())
    }

    pub fn start_with(path: &Path, options: Options) -> Result<Server, StartError> {
        let fail = |what: &str, e: &dyn std::fmt::Display| StartError(format!("{what}: {e}"));
        let folder = Folder::open(path).map_err(|e| fail("the world folder", &e))?;
        // Nothing reads or writes the world's database before this.
        let lock = folder.lock().map_err(|e| fail("the world folder", &e))?;
        let content = tme_content::load(&folder.config.content)
            .map_err(|e| fail("the world's content", &e))?;
        let mut store =
            Store::open(&folder.database()).map_err(|e| fail("the world's database", &e))?;
        let config = folder.config.clone();
        let restored = sim::restore(
            &mut store,
            Arc::new(content.catalogue),
            config.seed,
            config.settings(),
        )
        .map_err(|e| fail("restoring the world", &e))?;
        let accounts =
            Store::open(&folder.database()).map_err(|e| fail("the world's database", &e))?;

        let listener = std::net::TcpListener::bind(&config.bind)
            .map_err(|e| fail(&format!("listening on {}", config.bind), &e))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| fail("the listener", &e))?;
        let local = listener
            .local_addr()
            .map_err(|e| fail("the listening address", &e))?;

        let (sim_tx, sim_rx) = mpsc::channel();
        let times = Arc::new(Mutex::new(StepTimes::default()));
        let (stopping, stopped) = tokio::sync::watch::channel(());
        let world = sim::spawn(restored, store, sim_rx, Arc::clone(&times), options.step);
        // The world's thread holds the lock until it has saved everything,
        // and says it's stopped by ending.
        let world = std::thread::Builder::new()
            .name("world-watch".into())
            .spawn(move || {
                let ended = world.join();
                drop(stopping);
                drop(lock);
                ended.unwrap_or_else(|_| Err("the world's thread panicked".into()))
            })
            .map_err(|e| fail("the world's thread", &e))?;
        let shared = Arc::new(net::Shared::new(
            config.name.clone(),
            folder.identity.clone(),
            config.invite.clone(),
            accounts,
            sim_tx.clone(),
        ));
        // The network runs on a thread of its own with its own runtime, so a
        // server starts and stops the same from async code or plain code.
        // It stops when asked to, or when the world has.
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
        let mut world_stopped = stopped.clone();
        let net = std::thread::Builder::new()
            .name("net".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .expect("a network runtime");
                runtime.block_on(async move {
                    let listener = tokio::net::TcpListener::from_std(listener).expect("a listener");
                    tokio::select! {
                        () = net::serve(listener, shared) => {}
                        _ = stop_rx => {}
                        _ = world_stopped.changed() => {}
                    }
                });
            })
            .map_err(|e| fail("the network thread", &e))?;
        Ok(Server {
            local,
            invite: folder.invite(),
            sim: sim_tx,
            world: Some(world),
            net: Some((stop_tx, net)),
            times,
            stopped,
        })
    }

    /// Wait until the world stops without being asked to: it couldn't save
    /// what was played. [`Server::stop`] then says why.
    pub async fn stopped(&self) {
        let mut stopped = self.stopped.clone();
        while stopped.changed().await.is_ok() {}
    }

    /// The world's invite link.
    pub fn invite(&self) -> &Invite {
        &self.invite
    }

    /// How long the recent steps took to compute, sorted.
    pub fn step_times(&self) -> Vec<std::time::Duration> {
        self.times.lock().expect("not poisoned").sorted()
    }

    /// Stop: everyone leaves, the world is snapshotted and the journal
    /// written out.
    pub fn stop(mut self) -> Result<(), StartError> {
        self.shut()
    }

    fn shut(&mut self) -> Result<(), StartError> {
        // Close every connection first, then stop the world.
        if let Some((stop, net)) = self.net.take() {
            let _ = stop.send(());
            let _ = net.join();
        }
        let _ = self.sim.send(ToSim::Shutdown);
        match self.world.take() {
            Some(world) => world
                .join()
                .map_err(|_| StartError("the world's thread panicked".into()))?
                .map_err(StartError),
            None => Ok(()),
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.shut();
    }
}
