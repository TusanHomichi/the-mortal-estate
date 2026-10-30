//! A client's connection to a world, for anything that plays: the game's
//! own client, the bot harness, an AI client, tests. It joins through the
//! encrypted session's handshake, which only the world the invite named can
//! answer, then sends intents and receives what the world sends, every
//! frame sealed.

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use std::fmt;
use tme_sim::world::{Intent, PersonId, Settings};
use tme_wire::session::{self, Open, Seal};
use tme_wire::{
    ClientMsg, Greeting, Identity, Invite, Join, PROTOCOL, ServerMsg, Turned, decode, encode,
};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// The most a world may send in one frame.
const LARGEST_FRAME: usize = 16 << 20;

#[derive(Debug)]
pub enum LinkError {
    /// The world couldn't be reached, or the connection dropped.
    Connection(String),
    /// The world answering isn't the one the invite named, or something
    /// between us and it tampered with what was sent.
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
    let config = WebSocketConfig::default()
        .max_message_size(Some(LARGEST_FRAME))
        .max_frame_size(Some(LARGEST_FRAME));
    let (mut ws, _) = tokio_tungstenite::connect_async_with_config(url, Some(config), true)
        .await
        .map_err(|e| LinkError::Connection(e.to_string()))?;
    let (dialing, hello) = session::dial(me, &invite.world, PROTOCOL);
    send(&mut ws, encode(&hello)).await?;
    let answer = match decode(&recv(&mut ws).await?).map_err(|_| LinkError::Garbled)? {
        Greeting::Noise(answer) => answer,
        Greeting::Protocol { world } => return Err(LinkError::Turned(Turned::Protocol { world })),
        Greeting::NotThisWorld => return Err(LinkError::WrongWorld),
    };
    let join = Join {
        name: name.to_owned(),
        invite: invite.code.clone(),
    };
    let finished = dialing
        .finish(&answer, &encode(&join))
        .map_err(|_| LinkError::WrongWorld)?;
    let world: String = decode(&finished.greeting).map_err(|_| LinkError::Garbled)?;
    send(&mut ws, finished.third).await?;
    let session::Session { seal, mut open } = finished.session;
    let welcome = open
        .open(&recv(&mut ws).await?)
        .map_err(|_| LinkError::WrongWorld)?;
    let (you, step, settings) = match decode(&welcome).map_err(|_| LinkError::Garbled)? {
        ServerMsg::Welcome {
            you,
            step,
            settings,
        } => (you, step, settings),
        ServerMsg::Turned(t) => return Err(LinkError::Turned(t)),
        _ => return Err(LinkError::Garbled),
    };
    let (sink, stream) = ws.split();
    Ok((
        Joined {
            world,
            you,
            step,
            settings,
        },
        Sender { sink, seal, seq: 0 },
        Receiver { stream, open },
    ))
}

async fn send(ws: &mut Ws, frame: Vec<u8>) -> Result<(), LinkError> {
    ws.send(Message::binary(frame))
        .await
        .map_err(|e| LinkError::Connection(e.to_string()))
}

/// The next frame's bytes.
async fn recv(ws: &mut Ws) -> Result<Vec<u8>, LinkError> {
    loop {
        match ws.next().await {
            Some(Ok(Message::Binary(bytes))) => return Ok(bytes.to_vec()),
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
            Some(Ok(_)) | None => return Err(LinkError::Connection("closed".into())),
            Some(Err(e)) => return Err(LinkError::Connection(e.to_string())),
        }
    }
}

/// Intents out.
pub struct Sender {
    sink: SplitSink<Ws, Message>,
    seal: Seal,
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
            .send(Message::binary(self.seal.seal(&encode(&msg))))
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
    open: Open,
}

impl Receiver {
    /// The next message; `None` once the world has closed the connection.
    pub async fn next(&mut self) -> Option<Result<ServerMsg, LinkError>> {
        loop {
            return match self.stream.next().await? {
                Ok(Message::Binary(bytes)) => Some(match self.open.open(&bytes) {
                    Ok(msg) => decode(&msg).map_err(|_| LinkError::Garbled),
                    Err(_) => Err(LinkError::WrongWorld),
                }),
                Ok(Message::Close(_)) => None,
                Ok(_) => continue,
                Err(e) => Some(Err(LinkError::Connection(e.to_string()))),
            };
        }
    }
}
