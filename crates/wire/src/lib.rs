//! The one wire contract: every message between a server and a client is a
//! type here, which both depend on, so there's one definition and nothing to
//! generate or copy. Messages travel as binary WebSocket frames, each one
//! postcard-encoded.
//!
//! Joining: the client says hello with its key and a nonce; the world
//! answers with its key, its name, a challenge and its signature over the
//! client's nonce; the client checks that against the key its invite named
//! and signs the challenge to join. After that the client sends intents and
//! the world sends what its person observes, step by step.

mod invite;
mod keys;
mod view;

pub use invite::{Invite, NotAnInvite};
pub use keys::{
    Identity, Nonce, PublicKey, Signature, create_private, join_proof, verify, world_proof,
};
pub use view::{BodyView, You};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tme_sim::world::{Event, Intent, PersonId, Refused, Settings};

/// This contract's version. A world and a client that differ don't talk.
pub const PROTOCOL: u16 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClientMsg {
    Hello {
        protocol: u16,
        key: PublicKey,
        nonce: Nonce,
    },
    Join {
        /// What a new person is called; a returning one keeps their name.
        name: String,
        /// The invite's code: needed the first time, not after.
        invite: Option<String>,
        /// Over [`join_proof`] of the world's challenge and key.
        signature: Signature,
    },
    /// An intent, numbered so observations can say which they've applied.
    Act { seq: u32, intent: Intent },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ServerMsg {
    Challenge {
        world: PublicKey,
        name: String,
        challenge: Nonce,
        /// Over [`world_proof`] of the client's nonce and key.
        proof: Signature,
    },
    Welcome {
        you: PersonId,
        step: u64,
        settings: Settings,
    },
    /// The world won't have you; the connection closes after this.
    Turned(Turned),
    Observe(Observation),
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
    /// The join wasn't signed by the key that said hello.
    BadSignature,
    /// Someone new needs an invite, and brought none or a wrong one.
    NeedInvite,
    /// That key is already in the world on another connection.
    AlreadyHere,
    /// Something was said out of turn.
    OutOfTurn,
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

/// A frame's bytes as a message; an error for anything malformed.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, postcard::Error> {
    postcard::from_bytes(bytes)
}
