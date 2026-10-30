//! The network side: WebSocket connections, the encrypted session's
//! handshake, and frames to and from the simulation thread. Nothing here
//! decides anything about the world.
//!
//! Everything a client can make the server hold is bounded: how many
//! connections can be joining at once, how long joining may take, from
//! being accepted to the last byte, account lookup included, how big a frame can be, how fast intents
//! may come, and how far a client can fall behind reading, in the system's
//! buffers and the world's outbox, before it's let go.

use crate::sim::{Conn, OUTBOX, ToSim};
use crate::store::Store;
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tme_sim::PersonId;
use tme_wire::session::{self, Open, Seal, Session};
use tme_wire::{
    ClientMsg, Greeting, Hello, Identity, Join, PROTOCOL, ServerMsg, Turned, decode, encode,
};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};
use tokio::time::{Instant, timeout, timeout_at};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

/// What every connection shares.
pub struct Shared {
    pub name: String,
    pub identity: Identity,
    pub invite: String,
    pub accounts: Mutex<Store>,
    pub sim: mpsc::Sender<ToSim>,
    /// How long a step lasts on the wall clock, and how long a connection
    /// has to join.
    step: Duration,
    handshake: Duration,
    /// Room for connections still joining.
    joining: Arc<Semaphore>,
    next_conn: AtomicU64,
}

impl Shared {
    pub fn new(
        name: String,
        identity: Identity,
        invite: String,
        accounts: Store,
        sim: mpsc::Sender<ToSim>,
        options: &crate::Options,
    ) -> Shared {
        Shared {
            name,
            identity,
            invite,
            accounts: Mutex::new(accounts),
            sim,
            step: options.step,
            handshake: options.handshake,
            joining: Arc::new(Semaphore::new(JOINING)),
            next_conn: AtomicU64::new(1),
        }
    }
}

/// How many connections can be joining at once; more wait in the system's
/// queue to be accepted until one of those is through or out of time.
const JOINING: usize = 64;

/// The most a client may send in one frame: one sealed Noise message.
const LARGEST_FRAME: usize = 64 << 10;

/// How long a frame to a client may take to go before it's let go.
const SENDING: Duration = Duration::from_secs(10);

/// How much the system holds for a client that isn't reading, beyond its
/// outbox: plenty for a game's traffic, and a few seconds of it.
const SEND_BUFFER: usize = 128 << 10;

/// How many intents a client may send in a burst, and how many a step
/// after that: well past anyone's hands.
const BURST: f64 = 60.0;
const PER_STEP: f64 = 3.0;

/// The longest name a person can have, in characters.
const NAME_CHARS: usize = 24;

pub async fn serve(listener: TcpListener, shared: Arc<Shared>) {
    loop {
        let Ok(joining) = Arc::clone(&shared.joining).acquire_owned().await else {
            return;
        };
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        // A connection's time to join starts as it's accepted.
        let deadline = Instant::now() + shared.handshake;
        let _ = stream.set_nodelay(true);
        let _ = socket2::SockRef::from(&stream).set_send_buffer_size(SEND_BUFFER);
        let shared = Arc::clone(&shared);
        tokio::spawn(async move {
            let admitted = timeout_at(deadline, admit(stream, &shared, joining)).await;
            // A timeout takes what's ready over its deadline, so the
            // deadline is checked again.
            let Ok(Some((ws, session, person, name))) = admitted else {
                return;
            };
            if Instant::now() >= deadline {
                return;
            }
            play(ws, session, person, name, &shared).await;
        });
    }
}

type Ws = tokio_tungstenite::WebSocketStream<TcpStream>;

/// The handshake, and who it lets in: `None` for anyone it doesn't, once
/// they've been told why where they can be. It holds its joining slot
/// until it's done.
async fn admit(
    stream: TcpStream,
    shared: &Arc<Shared>,
    joining: OwnedSemaphorePermit,
) -> Option<(Ws, Session, PersonId, String)> {
    let config = WebSocketConfig::default()
        .max_message_size(Some(LARGEST_FRAME))
        .max_frame_size(Some(LARGEST_FRAME));
    let mut ws = tokio_tungstenite::accept_async_with_config(stream, Some(config))
        .await
        .ok()?;
    let hello: Hello = decode(&next_frame(&mut ws).await?).ok()?;
    if hello.protocol != PROTOCOL {
        let turned = Greeting::Protocol { world: PROTOCOL };
        let _ = ws.send(Message::binary(encode(&turned))).await;
        return None;
    }
    let Ok((answering, answer)) = session::answer(&shared.identity, &hello, &encode(&shared.name))
    else {
        let _ = ws
            .send(Message::binary(encode(&Greeting::NotThisWorld)))
            .await;
        return None;
    };
    ws.send(Message::binary(encode(&Greeting::Noise(answer))))
        .await
        .ok()?;
    let (key, join, mut session) = answering.finish(&next_frame(&mut ws).await?).ok()?;
    // Accounts are in the database, which can be slow or busy: they're
    // looked up off the network's threads, and the lookup keeps the slot
    // until it ends, even if the connection runs out of time first.
    let accounts = Arc::clone(shared);
    let (let_in, _joining) = tokio::task::spawn_blocking(move || {
        let let_in = match decode::<Join>(&join) {
            Ok(join) => {
                account(&accounts, &key, &join).map(|person| (person, clean_name(&join.name)))
            }
            Err(_) => Err(Turned::OutOfTurn),
        };
        (let_in, joining)
    })
    .await
    .ok()?;
    match let_in {
        Ok((person, name)) => Some((ws, session, person, name)),
        Err(turned) => {
            let frame = session.seal.seal(&encode(&ServerMsg::Turned(turned)));
            let _ = ws.send(Message::binary(frame)).await;
            None
        }
    }
}

/// The person `key` is here, or becomes if they bring the invite.
fn account(shared: &Shared, key: &tme_wire::PublicKey, join: &Join) -> Result<PersonId, Turned> {
    let accounts = shared.accounts.lock().expect("not poisoned");
    match accounts.account(key).map_err(|_| Turned::OutOfTurn)? {
        Some(person) => Ok(person),
        None if join.invite.as_deref() == Some(shared.invite.as_str()) => {
            accounts.create_account(key).map_err(|_| Turned::OutOfTurn)
        }
        None => Err(Turned::NeedInvite),
    }
}

/// A joined connection: intents to the world, what the world sends back.
async fn play(mut ws: Ws, session: Session, person: PersonId, name: String, shared: &Shared) {
    let Session { mut seal, mut open } = session;
    let conn: Conn = shared.next_conn.fetch_add(1, Ordering::Relaxed);
    let (out, mut outbox) = mpsc::channel(OUTBOX);
    let join = ToSim::Join {
        conn,
        person,
        name,
        out,
    };
    if shared.sim.send(join).await.is_err() {
        return;
    }
    let mut pace = Pace::new(shared.step);
    loop {
        tokio::select! {
            incoming = ws.next() => {
                let Some(Ok(frame)) = incoming else { break };
                let Message::Binary(bytes) = frame else {
                    if frame.is_close() { break } else { continue }
                };
                let Some(ClientMsg::Act { seq, intent }) = opened(&mut open, &bytes) else {
                    break;
                };
                if !pace.allow() {
                    let _ = send(&mut ws, &mut seal, &ServerMsg::Turned(Turned::TooFast)).await;
                    break;
                }
                if shared.sim.send(ToSim::Act { conn, seq, intent }).await.is_err() {
                    break;
                }
            }
            outgoing = outbox.recv() => {
                let Some(msg) = outgoing else { break };
                let last = matches!(msg, ServerMsg::Turned(_));
                if !send(&mut ws, &mut seal, &msg).await || last {
                    break;
                }
            }
        }
    }
    let _ = shared.sim.send(ToSim::Leave { conn }).await;
    let _ = timeout(SENDING, ws.close(None)).await;
}

/// A sealed frame's message, if it opens and reads as one.
fn opened(open: &mut Open, frame: &[u8]) -> Option<ClientMsg> {
    decode(&open.open(frame).ok()?).ok()
}

/// Send `msg`, sealed; whether it went in time.
async fn send(ws: &mut Ws, seal: &mut Seal, msg: &ServerMsg) -> bool {
    let frame = Message::binary(seal.seal(&encode(msg)));
    matches!(timeout(SENDING, ws.send(frame)).await, Ok(Ok(())))
}

/// The next frame's bytes, skipping pings; `None` once there are none.
async fn next_frame(ws: &mut Ws) -> Option<Vec<u8>> {
    loop {
        match ws.next().await? {
            Ok(Message::Binary(bytes)) => return Some(bytes.to_vec()),
            Ok(Message::Ping(_) | Message::Pong(_)) => continue,
            _ => return None,
        }
    }
}

/// How fast a client's intents come: a bucket of [`BURST`] that refills
/// [`PER_STEP`] a step.
struct Pace {
    tokens: f64,
    at: Instant,
    per_second: f64,
}

impl Pace {
    fn new(step: Duration) -> Pace {
        Pace {
            tokens: BURST,
            at: Instant::now(),
            per_second: PER_STEP / step.as_secs_f64(),
        }
    }

    fn allow(&mut self) -> bool {
        let now = Instant::now();
        let refill = (now - self.at).as_secs_f64() * self.per_second;
        self.tokens = (self.tokens + refill).min(BURST);
        self.at = now;
        if self.tokens < 1.0 {
            return false;
        }
        self.tokens -= 1.0;
        true
    }
}

/// A name as the world keeps it: printable, trimmed and not too long.
fn clean_name(name: &str) -> String {
    let name: String = name
        .chars()
        .filter(|c| !c.is_control())
        .take(NAME_CHARS)
        .collect();
    match name.trim() {
        "" => "Stranger".to_owned(),
        n => n.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn names_are_kept_printable_and_short() {
        assert_eq!(super::clean_name("  Wren \n"), "Wren");
        assert_eq!(super::clean_name("\u{7}"), "Stranger");
        assert_eq!(super::clean_name(&"a".repeat(100)).len(), 24);
    }
}
