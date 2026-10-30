//! The one wire contract: every message between a server and a client is a
//! type here, which both depend on, so there's one definition and nothing to
//! generate or copy. Messages travel as binary WebSocket frames, each one
//! postcard-encoded.
//!
//! Joining: the client says [`Hello`], starting the [session]'s handshake
//! with the world its invite named; the world answers with a [`Greeting`]
//! that carries its name; the client finishes the handshake carrying
//! [`Join`]. From then on every frame is sealed in the session: the world
//! says [`ServerMsg::Welcome`] (or why it turns the client away), the client
//! sends intents, and the world sends what its person observes, step by
//! step.

mod invite;
mod keys;
pub mod session;
mod view;

pub use invite::{Invite, NotAnInvite};
pub use keys::{Identity, PublicKey, create_private, create_private_dir};
pub use session::{Greeting, Hello};
pub use view::{BodyView, You};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tme_sim::world::{Event, Intent, PersonId, Refused, Settings};

/// This contract's version. A world and a client that differ don't talk.
pub const PROTOCOL: u16 = 2;

/// What the client says as it finishes the handshake.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Join {
    /// What a new person is called; a returning one keeps their name.
    pub name: String,
    /// The invite's code: needed the first time, not after.
    pub invite: Option<String>,
}

/// What the client sends once it's in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClientMsg {
    /// An intent, numbered so observations can say which they've applied.
    Act { seq: u32, intent: Intent },
}

/// What the world sends once the handshake's done.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ServerMsg {
    Welcome {
        you: PersonId,
        step: u64,
        settings: Settings,
    },
    /// The world won't have you; the connection closes after this.
    Turned(Turned),
    Observe(Box<Observation>),
    /// An intent changed nothing.
    Refused {
        seq: u32,
        why: Refused,
    },
}

/// Why a world turned someone away.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Turned {
    /// The client speaks another version of this contract.
    Protocol { world: u16 },
    /// Someone new needs an invite, and brought none or a wrong one.
    NeedInvite,
    /// That key is already in the world on another connection.
    AlreadyHere,
    /// Something was said out of turn.
    OutOfTurn,
    /// Intents came faster than anyone plays.
    TooFast,
}

/// What a person observes after a step: their own state in full, and what
/// changed within their view since the last observation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub step: u64,
    /// The last intent applied, by its number.
    pub ack: u32,
    pub you: You,
    /// Bodies that came into view or changed.
    pub seen: Vec<BodyView>,
    /// Bodies that left view, fell or left the world.
    pub gone: Vec<tme_sim::BodyId>,
    /// What happened in view.
    pub events: Vec<Event>,
}

/// A message as bytes for a frame.
pub fn encode<T: Serialize>(msg: &T) -> Vec<u8> {
    postcard::to_stdvec(msg).expect("every message encodes")
}

/// A frame's bytes as a message: all of them, or an error.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, postcard::Error> {
    match postcard::take_from_bytes(bytes)? {
        (msg, []) => Ok(msg),
        _ => Err(postcard::Error::DeserializeBadEncoding),
    }
}
