//! A client's connection to a world, for anything that plays: the game's
//! own client, the bot harness, an AI client, tests. It joins through the
//! handshake, checking the world is the one the invite named, then sends
//! intents and receives what the world sends.

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::fmt;
use tme_sim::world::{Intent, PersonId, Settings};
use tme_wire::{
    ClientMsg, Identity, Invite, Nonce, PROTOCOL, ServerMsg, Turned, decode, encode, join_proof,
    verify, world_proof,
};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug)]
pub enum LinkError {
    /// The world couldn't be reached, or the connection dropped.
    Connection(String),
    /// The world answering isn't the one the invite named.
    WrongWorld,
    /// The world turned us away.
    Turned(Turned),
    /// The world said something that isn't in the contract.
    Garbled,
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinkError::Connection(e) => write!(f, "can't reach the world: {e}"),
            LinkError::WrongWorld => {
                f.write_str("the world answering isn't the one the invite names")
            }
            LinkError::Turned(t) => write!(f, "the world turned us away: {t:?}"),
            LinkError::Garbled => f.write_str("the world sent something we can't read"),
        }
    }
}

impl std::error::Error for LinkError {}

/// What the world said on letting us in.
#[derive(Clone, Debug)]
pub struct Joined {
    pub world: String,
    pub you: PersonId,
    pub step: u64,
    pub settings: Settings,
}

/// Join the world `invite` names as `me`, called `name` if we're new there.
pub async fn join(
    invite: &Invite,
    me: &Identity,
    name: &str,
) -> Result<(Joined, Sender, Receiver), LinkError> {
    let url = format!("ws://{}/", invite.address);
    let (mut ws, _) = tokio_tungstenite::connect_async(url)
        .await
        .map_err(|e| LinkError::Connection(e.to_string()))?;
    if let Ok(stream) = match ws.get_ref() {
        MaybeTlsStream::Plain(s) => Ok(s),
        _ => Err(()),
    } {
        let _ = stream.set_nodelay(true);
    }
    let nonce = Nonce::random();
    send(
        &mut ws,
        &ClientMsg::Hello {
            protocol: PROTOCOL,
            key: me.public(),
            nonce,
        },
    )
    .await?;
    let ServerMsg::Challenge {
        world,
        name: world_name,
        challenge,
        proof,
    } = recv(&mut ws).await?
    else {
        return Err(LinkError::Garbled);
    };
    if world != invite.world || !verify(&world, &world_proof(&nonce, &me.public()), &proof) {
        return Err(LinkError::WrongWorld);
    }
    send(
        &mut ws,
        &ClientMsg::Join {
            name: name.to_owned(),
            invite: invite.code.clone(),
            signature: me.sign(&join_proof(&challenge, &world)),
        },
    )
    .await?;
    let ServerMsg::Welcome {
        you,
        step,
        settings,
    } = recv(&mut ws).await?
    else {
        return Err(LinkError::Garbled);
    };
    let (sink, stream) = ws.split();
    Ok((
        Joined {
            world: world_name,
            you,
            step,
            settings,
        },
        Sender { sink, seq: 0 },
        Receiver { stream },
    ))
}

async fn send(ws: &mut Ws, msg: &ClientMsg) -> Result<(), LinkError> {
    ws.send(Message::binary(encode(msg)))
        .await
        .map_err(|e| LinkError::Connection(e.to_string()))
}

async fn recv(ws: &mut Ws) -> Result<ServerMsg, LinkError> {
    loop {
        match ws.next().await {
            Some(Ok(Message::Binary(bytes))) => {
                return match decode(&bytes).map_err(|_| LinkError::Garbled)? {
                    ServerMsg::Turned(t) => Err(LinkError::Turned(t)),
                    msg => Ok(msg),
                };
            }
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
            Some(Ok(_)) | None => return Err(LinkError::Connection("closed".into())),
            Some(Err(e)) => return Err(LinkError::Connection(e.to_string())),
        }
    }
}

/// Intents out.
pub struct Sender {
    sink: SplitSink<Ws, Message>,
    seq: u32,
}

impl Sender {
    /// Send an intent; its number, which observations acknowledge.
    pub async fn act(&mut self, intent: Intent) -> Result<u32, LinkError> {
        self.seq += 1;
        let msg = ClientMsg::Act {
            seq: self.seq,
            intent,
        };
        self.sink
            .send(Message::binary(encode(&msg)))
            .await
            .map_err(|e| LinkError::Connection(e.to_string()))?;
        Ok(self.seq)
    }

    /// Leave the world.
    pub async fn close(mut self) {
        let _ = self.sink.close().await;
    }
}

/// What the world sends.
pub struct Receiver {
    stream: SplitStream<Ws>,
}

impl Receiver {
    /// The next message; `None` once the world has closed the connection.
    pub async fn next(&mut self) -> Option<Result<ServerMsg, LinkError>> {
        loop {
            return match self.stream.next().await? {
                Ok(Message::Binary(bytes)) => Some(decode(&bytes).map_err(|_| LinkError::Garbled)),
                Ok(Message::Close(_)) => None,
                Ok(_) => continue,
                Err(e) => Some(Err(LinkError::Connection(e.to_string()))),
            };
        }
    }
}
