//! The encrypted session every connection runs in: a Noise XK handshake,
//! then every frame sealed both ways. The client knows the world's key
//! from its invite before it says anything, so only that world can answer;
//! the client's own key, and the invite code it brings, travel encrypted
//! and only to that world. After the handshake each frame is sealed in
//! order, so nothing between can read, change, replay, reorder or add one.
//!
//! The first two frames carry the protocol's version in the clear, so a
//! world and a client that speak different versions can say so; the
//! version is bound into the handshake, so it can't be changed on the way.

use crate::keys::{Identity, PublicKey};
use serde::{Deserialize, Serialize};
use snow::{Builder, HandshakeState, StatelessTransportState};
use std::fmt;
use std::sync::Arc;

/// Noise's name for the handshake and the ciphers.
const NOISE: &str = "Noise_XK_25519_ChaChaPoly_BLAKE2s";

/// The longest Noise message, and the authentication tag each carries.
const MESSAGE: usize = 65535;
const TAG: usize = 16;

/// The client's first frame.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: u16,
    pub noise: Vec<u8>,
}

/// The world's first frame: the handshake's answer, or why there isn't
/// one. The last two are in the clear and prove nothing; they only say
/// why a client won't get in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Greeting {
    Noise(Vec<u8>),
    /// The version the world speaks instead.
    Protocol {
        world: u16,
    },
    /// The hello was for another world's key.
    NotThisWorld,
}

/// The handshake failed, or a sealed frame didn't open: whatever sent it
/// isn't the other end of this session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Broken;

impl fmt::Display for Broken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the session's encryption failed")
    }
}

impl std::error::Error for Broken {}

impl From<snow::Error> for Broken {
    fn from(_: snow::Error) -> Broken {
        Broken
    }
}

/// What both ends mix into the handshake: the version they speak.
fn prologue(protocol: u16) -> Vec<u8> {
    [b"tme".as_slice(), &protocol.to_le_bytes()].concat()
}

/// A client part way through joining.
pub struct Dialing {
    noise: HandshakeState,
}

/// Start a session with the world whose key is `world`, as `me`: the
/// first frame's handshake.
pub fn dial(me: &Identity, world: &PublicKey, protocol: u16) -> (Dialing, Hello) {
    let (prologue, secret) = (prologue(protocol), me.secret());
    let mut noise = Builder::new(NOISE.parse().expect("a valid Noise name"))
        .prologue(&prologue)
        .and_then(|b| b.local_private_key(&secret))
        .and_then(|b| b.remote_public_key(&world.0))
        .and_then(|b| b.build_initiator())
        .expect("a complete handshake");
    let mut out = vec![0; MESSAGE];
    let n = noise.write_message(&[], &mut out).expect("a first message");
    out.truncate(n);
    (
        Dialing { noise },
        Hello {
            protocol,
            noise: out,
        },
    )
}

impl Dialing {
    /// Take the world's answer, which carries `greeting` from it, and send
    /// it `payload` in the third frame, which finishes the handshake.
    pub fn finish(mut self, answer: &[u8], payload: &[u8]) -> Result<Finished, Broken> {
        let mut greeting = vec![0; MESSAGE];
        let n = self.noise.read_message(answer, &mut greeting)?;
        greeting.truncate(n);
        let mut third = vec![0; MESSAGE];
        let n = self.noise.write_message(payload, &mut third)?;
        third.truncate(n);
        Ok(Finished {
            greeting,
            third,
            session: Session::new(self.noise.into_stateless_transport_mode()?),
        })
    }
}

/// A client's side of a finished handshake.
pub struct Finished {
    /// What the world said in its answer.
    pub greeting: Vec<u8>,
    /// The frame to send it.
    pub third: Vec<u8>,
    pub session: Session,
}

/// A world part way through letting a client in.
pub struct Answering {
    noise: HandshakeState,
}

/// Answer a client's hello as `world`, saying `greeting`: the second frame.
pub fn answer(
    world: &Identity,
    hello: &Hello,
    greeting: &[u8],
) -> Result<(Answering, Vec<u8>), Broken> {
    let (prologue, secret) = (prologue(hello.protocol), world.secret());
    let mut noise = Builder::new(NOISE.parse().expect("a valid Noise name"))
        .prologue(&prologue)
        .and_then(|b| b.local_private_key(&secret))
        .and_then(|b| b.build_responder())
        .expect("a complete handshake");
    let mut ignored = vec![0; MESSAGE];
    noise.read_message(&hello.noise, &mut ignored)?;
    let mut out = vec![0; MESSAGE];
    let n = noise.write_message(greeting, &mut out)?;
    out.truncate(n);
    Ok((Answering { noise }, out))
}

impl Answering {
    /// The client's third frame: who it is, what it said, and the session.
    pub fn finish(mut self, third: &[u8]) -> Result<(PublicKey, Vec<u8>, Session), Broken> {
        let mut payload = vec![0; MESSAGE];
        let n = self.noise.read_message(third, &mut payload)?;
        payload.truncate(n);
        let client = self
            .noise
            .get_remote_static()
            .and_then(|k| k.try_into().ok())
            .map(PublicKey)
            .ok_or(Broken)?;
        Ok((
            client,
            payload,
            Session::new(self.noise.into_stateless_transport_mode()?),
        ))
    }
}

/// A finished handshake's two directions, which can go their own ways.
pub struct Session {
    pub seal: Seal,
    pub open: Open,
}

impl Session {
    fn new(state: StatelessTransportState) -> Session {
        let state = Arc::new(state);
        Session {
            seal: Seal {
                state: Arc::clone(&state),
                next: 0,
            },
            open: Open { state, next: 0 },
        }
    }
}

/// What each chunk of a frame starts with: whether another follows it.
const MORE: u8 = 1;
const LAST: u8 = 0;

/// How much of a message one Noise message carries, after its flag.
const CHUNK: usize = MESSAGE - TAG - 1;

/// Sealing what this end sends, in order.
pub struct Seal {
    state: Arc<StatelessTransportState>,
    next: u64,
}

impl Seal {
    /// A message as a sealed frame: in as many Noise messages as it takes,
    /// each saying whether another follows, so a frame cut short, split or
    /// run into the next doesn't open.
    pub fn seal(&mut self, message: &[u8]) -> Vec<u8> {
        let count = message.len().div_ceil(CHUNK).max(1);
        let mut frame = Vec::with_capacity(message.len() + count * (TAG + 1));
        let (mut plain, mut sealed) = (Vec::with_capacity(CHUNK + 1), vec![0; MESSAGE]);
        for i in 0..count {
            let chunk =
                &message[(i * CHUNK).min(message.len())..((i + 1) * CHUNK).min(message.len())];
            plain.clear();
            plain.push(if i + 1 < count { MORE } else { LAST });
            plain.extend_from_slice(chunk);
            let n = self
                .state
                .write_message(self.next, &plain, &mut sealed)
                .expect("a chunk fits a message");
            self.next += 1;
            frame.extend_from_slice(&sealed[..n]);
        }
        frame
    }
}

/// Opening what the other end sent, in the order it sent it.
pub struct Open {
    state: Arc<StatelessTransportState>,
    next: u64,
}

impl Open {
    /// A sealed frame's message. A frame that doesn't open, isn't the next
    /// one sent, or isn't all of one, breaks the session.
    pub fn open(&mut self, frame: &[u8]) -> Result<Vec<u8>, Broken> {
        let mut message = Vec::with_capacity(frame.len());
        let mut plain = vec![0; MESSAGE];
        let mut chunks = frame.chunks(MESSAGE).peekable();
        if chunks.peek().is_none() {
            return Err(Broken);
        }
        while let Some(sealed) = chunks.next() {
            let n = self.state.read_message(self.next, sealed, &mut plain)?;
            self.next += 1;
            let expected = if chunks.peek().is_some() { MORE } else { LAST };
            match plain[..n].split_first() {
                Some((flag, chunk)) if *flag == expected => message.extend_from_slice(chunk),
                _ => return Err(Broken),
            }
        }
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A client and a world through the handshake: their sessions, and what
    /// each heard from the other on the way.
    fn handshake(client: &Identity, world: &Identity) -> (Session, Session, Vec<u8>, Vec<u8>) {
        let (dialing, hello) = dial(client, &world.public(), 7);
        let (answering, second) = answer(world, &hello, b"Lattermead").unwrap();
        let finished = dialing.finish(&second, b"invite abc").unwrap();
        let (who, said, theirs) = answering.finish(&finished.third).unwrap();
        assert_eq!(who, client.public(), "the world learns who the client is");
        (finished.session, theirs, finished.greeting, said)
    }

    #[test]
    fn a_session_carries_messages_both_ways() {
        let (client, world) = (Identity::generate(), Identity::generate());
        let (mut c, mut w, greeting, said) = handshake(&client, &world);
        assert_eq!(greeting, b"Lattermead");
        assert_eq!(said, b"invite abc");
        for message in [b"".to_vec(), b"hello".to_vec(), vec![7; 200_000]] {
            assert_eq!(w.open.open(&c.seal.seal(&message)).unwrap(), message);
            assert_eq!(c.open.open(&w.seal.seal(&message)).unwrap(), message);
        }
    }

    #[test]
    fn only_the_invited_world_can_answer() {
        let (client, world, impostor) = (
            Identity::generate(),
            Identity::generate(),
            Identity::generate(),
        );
        let (dialing, hello) = dial(&client, &world.public(), 7);
        // An impostor can't even read the hello; a world that answers
        // without the right key isn't believed.
        assert!(answer(&impostor, &hello, b"").is_err());
        let (_, hello_to_impostor) = dial(&client, &impostor.public(), 7);
        let (_, second) = answer(&impostor, &hello_to_impostor, b"").unwrap();
        assert!(dialing.finish(&second, b"invite abc").is_err());
    }

    #[test]
    fn a_version_changed_on_the_way_breaks_the_handshake() {
        let (client, world) = (Identity::generate(), Identity::generate());
        let (_, mut hello) = dial(&client, &world.public(), 7);
        hello.protocol = 6;
        assert!(answer(&world, &hello, b"").is_err());
    }

    #[test]
    fn frames_changed_replayed_reordered_or_added_do_not_open() {
        let (client, world) = (Identity::generate(), Identity::generate());
        let (mut c, mut w, _, _) = handshake(&client, &world);
        let mut changed = c.seal.seal(b"first");
        changed[0] ^= 1;
        assert_eq!(w.open.open(&changed), Err(Broken), "changed");
        let (mut c, mut w, _, _) = handshake(&client, &world);
        let (_, second) = (c.seal.seal(b"first"), c.seal.seal(b"second"));
        assert_eq!(w.open.open(&second), Err(Broken), "out of order");
        let (mut c, mut w, _, _) = handshake(&client, &world);
        let first = c.seal.seal(b"first");
        assert_eq!(w.open.open(&first).unwrap(), b"first");
        assert_eq!(w.open.open(&first), Err(Broken), "replayed");
        let (mut other, _, _, _) = handshake(&client, &world);
        let elsewhere = other.seal.seal(b"second");
        assert_eq!(w.open.open(&elsewhere), Err(Broken), "from another session");
    }

    #[test]
    fn a_long_message_cut_short_split_or_run_together_does_not_open() {
        let (client, world) = (Identity::generate(), Identity::generate());
        let long = vec![3; 100_000];
        let (mut c, mut w, _, _) = handshake(&client, &world);
        let frame = c.seal.seal(&long);
        assert_eq!(w.open.open(&frame[..MESSAGE]), Err(Broken), "cut short");
        let (mut c, mut w, _, _) = handshake(&client, &world);
        let (a, b) = (c.seal.seal(b"one"), c.seal.seal(b"two"));
        assert_eq!(w.open.open(&[a, b].concat()), Err(Broken), "run together");
    }
}
