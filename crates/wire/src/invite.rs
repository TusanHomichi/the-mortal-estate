//! Invite links: `tme://HOST:PORT/WORLD-KEY?invite=CODE`. The address says
//! where the world is, the key proves it's the world meant, and the code
//! lets someone new in. A returning player's key is enough without one.

use crate::keys::PublicKey;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invite {
    /// `host:port`.
    pub address: String,
    pub world: PublicKey,
    pub code: Option<String>,
}

const SCHEME: &str = "tme://";

impl fmt::Display for Invite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{SCHEME}{}/{}", self.address, self.world.to_hex())?;
        if let Some(code) = &self.code {
            write!(f, "?invite={code}")?;
        }
        Ok(())
    }
}

/// Why a link isn't an invite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotAnInvite(pub &'static str);

impl fmt::Display for NotAnInvite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not an invite link: {}", self.0)
    }
}

impl std::error::Error for NotAnInvite {}

impl std::str::FromStr for Invite {
    type Err = NotAnInvite;

    fn from_str(link: &str) -> Result<Invite, NotAnInvite> {
        let rest = link
            .trim()
            .strip_prefix(SCHEME)
            .ok_or(NotAnInvite("it doesn't start tme://"))?;
        let (address, rest) = rest
            .split_once('/')
            .ok_or(NotAnInvite("there's no world key after the address"))?;
        if address.is_empty() || !address.contains(':') {
            return Err(NotAnInvite("the address isn't host:port"));
        }
        let (key, query) = match rest.split_once('?') {
            Some((k, q)) => (k, Some(q)),
            None => (rest, None),
        };
        let world =
            PublicKey::from_hex(key).ok_or(NotAnInvite("the world key isn't 64 hex digits"))?;
        let code = match query {
            None => None,
            Some(q) => Some(
                q.strip_prefix("invite=")
                    .filter(|c| !c.is_empty() && c.chars().all(|c| c.is_ascii_alphanumeric()))
                    .ok_or(NotAnInvite("the invite code isn't letters and digits"))?
                    .to_owned(),
            ),
        };
        Ok(Invite {
            address: address.to_owned(),
            world,
            code,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::Identity;

    #[test]
    fn an_invite_link_reads_back_as_written() {
        let invite = Invite {
            address: "lattermead.example:7420".into(),
            world: Identity::generate().public(),
            code: Some("k3ttle".into()),
        };
        let link = invite.to_string();
        assert!(link.starts_with("tme://lattermead.example:7420/"));
        assert_eq!(link.parse::<Invite>(), Ok(invite.clone()));
        let without = Invite {
            code: None,
            ..invite
        };
        assert_eq!(without.to_string().parse::<Invite>(), Ok(without));
    }

    #[test]
    fn a_broken_link_says_what_is_wrong() {
        assert!("https://x:1/ab".parse::<Invite>().is_err());
        assert_eq!(
            "tme://x:1/abcd".parse::<Invite>(),
            Err(NotAnInvite("the world key isn't 64 hex digits"))
        );
        let key = Identity::generate().public().to_hex();
        assert!(
            format!("tme://x:1/{key}?invite=a b")
                .parse::<Invite>()
                .is_err()
        );
    }
}
