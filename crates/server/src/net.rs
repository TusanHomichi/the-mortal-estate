//! The network side: WebSocket connections, the joining handshake, and
//! frames to and from the simulation thread. Nothing here decides anything
//! about the world.

use crate::sim::{Conn, ToSim};
use crate::store::Store;
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tme_wire::{
    ClientMsg, Identity, Nonce, PROTOCOL, ServerMsg, Turned, decode, encode, join_proof, verify,
    world_proof,
};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

/// What every connection shares.
pub struct Shared {
    pub name: String,
    pub identity: Identity,
    pub invite: String,
    pub accounts: Mutex<Store>,
    pub sim: Mutex<Sender<ToSim>>,
    next_conn: AtomicU64,
}

impl Shared {
    pub fn new(
        name: String,
        identity: Identity,
        invite: String,
        accounts: Store,
        sim: Sender<ToSim>,
    ) -> Shared {
        Shared {
            name,
            identity,
            invite,
            accounts: Mutex::new(accounts),
            sim: Mutex::new(sim),
            next_conn: AtomicU64::new(1),
        }
    }

    fn to_sim(&self, msg: ToSim) {
        let _ = self.sim.lock().expect("not poisoned").send(msg);
    }
}

/// How long a client has to say hello and join.
const HANDSHAKE: Duration = Duration::from_secs(10);

/// The longest name a person can have, in characters.
const NAME_CHARS: usize = 24;

pub async fn serve(listener: TcpListener, shared: Arc<Shared>) {
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        let _ = stream.set_nodelay(true);
        tokio::spawn(connection(stream, Arc::clone(&shared)));
    }
}

async fn connection(stream: TcpStream, shared: Arc<Shared>) {
    let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await else {
        return;
    };
    let joined = tokio::time::timeout(HANDSHAKE, handshake(&mut ws, &shared)).await;
    let (person, name) = match joined {
        Ok(Ok(joined)) => joined,
        Ok(Err(turned)) => {
            let _ = ws
                .send(Message::binary(encode(&ServerMsg::Turned(turned))))
                .await;
            let _ = ws.close(None).await;
            return;
        }
        Err(_) => return,
    };
    let conn: Conn = shared.next_conn.fetch_add(1, Ordering::Relaxed);
    let (out, mut outbox) = tokio::sync::mpsc::unbounded_channel();
    shared.to_sim(ToSim::Join {
        conn,
        person,
        name,
        out,
    });
    loop {
        tokio::select! {
            incoming = ws.next() => {
                let Some(Ok(frame)) = incoming else { break };
                match frame {
                    Message::Binary(bytes) => match decode::<ClientMsg>(&bytes) {
                        Ok(ClientMsg::Act { seq, intent }) => shared.to_sim(ToSim::Act { conn, seq, intent }),
                        _ => break,
                    },
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            outgoing = outbox.recv() => {
                let Some(msg) = outgoing else { break };
                let last = matches!(msg, ServerMsg::Turned(_));
                if ws.send(Message::binary(encode(&msg))).await.is_err() || last {
                    break;
                }
            }
        }
    }
    shared.to_sim(ToSim::Leave { conn });
    let _ = ws.close(None).await;
}

type Ws = tokio_tungstenite::WebSocketStream<TcpStream>;

async fn next_msg(ws: &mut Ws) -> Result<ClientMsg, Turned> {
    loop {
        match ws.next().await {
            Some(Ok(Message::Binary(bytes))) => {
                return decode(&bytes).map_err(|_| Turned::OutOfTurn);
            }
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
            _ => return Err(Turned::OutOfTurn),
        }
    }
}

/// Hello, challenge, join: who this is, and the person they are here.
async fn handshake(ws: &mut Ws, shared: &Shared) -> Result<(tme_sim::PersonId, String), Turned> {
    let ClientMsg::Hello {
        protocol,
        key,
        nonce,
    } = next_msg(ws).await?
    else {
        return Err(Turned::OutOfTurn);
    };
    if protocol != PROTOCOL {
        return Err(Turned::Protocol { world: PROTOCOL });
    }
    let world = shared.identity.public();
    let challenge = Nonce::random();
    let hello = ServerMsg::Challenge {
        world,
        name: shared.name.clone(),
        challenge,
        proof: shared.identity.sign(&world_proof(&nonce, &key)),
    };
    ws.send(Message::binary(encode(&hello)))
        .await
        .map_err(|_| Turned::OutOfTurn)?;
    let ClientMsg::Join {
        name,
        invite,
        signature,
    } = next_msg(ws).await?
    else {
        return Err(Turned::OutOfTurn);
    };
    if !verify(&key, &join_proof(&challenge, &world), &signature) {
        return Err(Turned::BadSignature);
    }
    let accounts = shared.accounts.lock().expect("not poisoned");
    let known = accounts.account(&key).map_err(|_| Turned::OutOfTurn)?;
    let person = match known {
        Some(p) => p,
        None if invite.as_deref() == Some(shared.invite.as_str()) => accounts
            .create_account(&key)
            .map_err(|_| Turned::OutOfTurn)?,
        None => return Err(Turned::NeedInvite),
    };
    Ok((person, clean_name(&name)))
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
