//! Keys: a client's is who its player is, in every world; a world's proves
//! it's the world its invite named. Both are ed25519.

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use serde::de::{self, SeqAccess, Visitor};
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// A public key: a player's identity, or a world's.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PublicKey(pub [u8; 32]);

impl PublicKey {
    pub fn to_hex(&self) -> String {
        hex(&self.0)
    }

    pub fn from_hex(s: &str) -> Option<PublicKey> {
        unhex(s)?.try_into().ok().map(PublicKey)
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey({})", self.to_hex())
    }
}

/// Random bytes a side asks the other to sign, so a signature can't be
/// replayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nonce(pub [u8; 32]);

impl Nonce {
    pub fn random() -> Nonce {
        Nonce(random_bytes())
    }
}

/// An ed25519 signature.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Signature(pub [u8; 64]);

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signature({})", hex(&self.0))
    }
}

// serde stops at 32-element arrays; a signature is 64 bytes.
impl Serialize for Signature {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut t = s.serialize_tuple(64)?;
        for b in &self.0 {
            t.serialize_element(b)?;
        }
        t.end()
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Signature, D::Error> {
        struct Bytes;
        impl<'de> Visitor<'de> for Bytes {
            type Value = Signature;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("64 bytes")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Signature, A::Error> {
                let mut out = [0u8; 64];
                for (i, b) in out.iter_mut().enumerate() {
                    *b = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(i, &self))?;
                }
                Ok(Signature(out))
            }
        }
        d.deserialize_tuple(64, Bytes)
    }
}

/// A secret key and the public key it signs as.
#[derive(Clone)]
pub struct Identity {
    key: SigningKey,
}

impl Identity {
    /// A new identity, from the operating system's randomness.
    pub fn generate() -> Identity {
        Identity::from_secret(random_bytes())
    }

    pub fn from_secret(secret: [u8; 32]) -> Identity {
        Identity {
            key: SigningKey::from_bytes(&secret),
        }
    }

    /// The secret, for saving: whoever has it is this identity.
    pub fn secret(&self) -> [u8; 32] {
        self.key.to_bytes()
    }

    pub fn public(&self) -> PublicKey {
        PublicKey(self.key.verifying_key().to_bytes())
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        Signature(self.key.sign(message).to_bytes())
    }

    /// A new identity, saved at `path`, which must not exist yet: readable
    /// by its owner only, where the system has owners.
    pub fn create(path: &Path) -> io::Result<Identity> {
        let identity = Identity::generate();
        create_private(path)?.write_all(&identity.secret())?;
        Ok(identity)
    }

    /// The identity saved at `path`.
    pub fn load(path: &Path) -> io::Result<Identity> {
        let secret: [u8; 32] = fs::read(path)?
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "not a 32-byte secret key"))?;
        Ok(Identity::from_secret(secret))
    }
}

impl fmt::Debug for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Identity({:?})", self.public())
    }
}

/// Whether `key` signed `message` with `signature`.
pub fn verify(key: &PublicKey, message: &[u8], signature: &Signature) -> bool {
    let Ok(key) = VerifyingKey::from_bytes(&key.0) else {
        return false;
    };
    key.verify(message, &ed25519_dalek::Signature::from_bytes(&signature.0))
        .is_ok()
}

/// What a world signs to prove it's the world a client meant to reach: the
/// client's nonce and key.
pub fn world_proof(nonce: &Nonce, client: &PublicKey) -> Vec<u8> {
    [b"tme world proof v1".as_slice(), &nonce.0, &client.0].concat()
}

/// What a client signs to join a world: the world's challenge and key.
pub fn join_proof(challenge: &Nonce, world: &PublicKey) -> Vec<u8> {
    [b"tme join v1".as_slice(), &challenge.0, &world.0].concat()
}

/// A new file at `path`, which must not exist yet, that only its owner can
/// read or write, where the system has owners: from the moment it exists,
/// whatever the umask.
pub fn create_private(path: &Path) -> io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

fn random_bytes() -> [u8; 32] {
    let mut out = [0u8; 32];
    getrandom::fill(&mut out).expect("the operating system has randomness");
    out
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.is_ascii() {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signature_verifies_only_for_its_key_and_message() {
        let (a, b) = (Identity::generate(), Identity::generate());
        let sig = a.sign(b"hello");
        assert!(verify(&a.public(), b"hello", &sig));
        assert!(!verify(&a.public(), b"hullo", &sig));
        assert!(!verify(&b.public(), b"hello", &sig));
    }

    #[test]
    fn an_identity_survives_saving() {
        let a = Identity::generate();
        assert_eq!(Identity::from_secret(a.secret()).public(), a.public());
        assert_eq!(PublicKey::from_hex(&a.public().to_hex()), Some(a.public()));
        assert_eq!(PublicKey::from_hex("zz"), None);
    }

    #[test]
    fn a_key_file_is_its_owners_alone_and_never_overwritten() {
        let path = std::env::temp_dir().join(format!("tme-key-{}", std::process::id()));
        let _ = fs::remove_file(&path);
        let made = Identity::create(&path).unwrap();
        assert_eq!(Identity::load(&path).unwrap().public(), made.public());
        assert!(Identity::create(&path).is_err(), "an existing key is kept");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o077, 0, "nobody but its owner can read it");
        }
        fs::write(&path, b"short").unwrap();
        assert!(Identity::load(&path).is_err());
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_signature_round_trips_through_the_wire() {
        let sig = Identity::generate().sign(b"x");
        let bytes = postcard::to_stdvec(&sig).unwrap();
        assert_eq!(bytes.len(), 64);
        assert_eq!(postcard::from_bytes::<Signature>(&bytes).unwrap(), sig);
    }
}
