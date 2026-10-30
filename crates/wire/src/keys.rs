//! Keys: a client's is who its player is, in every world; a world's proves
//! it's the world its invite named. Both are X25519, the keys the
//! encrypted session is made with: a session can only be had by whoever
//! holds the secret.

use snow::params::DHChoice;
use snow::resolvers::{CryptoResolver, DefaultResolver};
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// A public key: a player's identity, or a world's.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
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

/// A secret key and its public key.
#[derive(Clone)]
pub struct Identity {
    secret: [u8; 32],
    public: PublicKey,
}

impl Identity {
    /// A new identity, from the operating system's randomness.
    pub fn generate() -> Identity {
        Identity::from_secret(random_bytes())
    }

    pub fn from_secret(secret: [u8; 32]) -> Identity {
        let mut dh = DefaultResolver
            .resolve_dh(&DHChoice::Curve25519)
            .expect("X25519 is built in");
        dh.set(&secret);
        let public = PublicKey(dh.pubkey().try_into().expect("a 32-byte public key"));
        Identity { secret, public }
    }

    /// The secret, for saving and for making sessions: whoever has it is
    /// this identity.
    pub fn secret(&self) -> [u8; 32] {
        self.secret
    }

    pub fn public(&self) -> PublicKey {
        self.public
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
    fn an_identity_survives_saving() {
        let a = Identity::generate();
        assert_eq!(Identity::from_secret(a.secret()).public(), a.public());
        assert_ne!(Identity::generate().public(), a.public());
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
}
