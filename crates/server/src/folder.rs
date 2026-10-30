//! A world's folder: `world.toml` (its settings and invite code),
//! `world.key` (the secret that proves it's this world; keep it),
//! `world.db` (accounts, journal and snapshots) and `world.lock`, which the
//! server running the world holds. The first two are credentials, readable
//! by the host alone. `world.toml` is read when the world starts: its
//! settings and content apply from then, and its seed only to a world
//! that's never run.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tme_sim::world::Settings;
use tme_wire::{Identity, Invite, create_private};

/// `world.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The world's name, as players see it.
    pub name: String,
    /// The seed the world was rolled from.
    pub seed: u64,
    /// Where the server listens: `0.0.0.0:7420`.
    pub bind: String,
    /// Where players reach it, as invites say: `host:port`.
    pub address: String,
    /// The code an invite carries; someone new needs it to join.
    pub invite: String,
    /// The content folder the world runs.
    pub content: PathBuf,
    #[serde(default)]
    pub harm_between_players: bool,
}

impl Config {
    pub fn settings(&self) -> Settings {
        Settings {
            harm_between_players: self.harm_between_players,
        }
    }
}

/// An opened world folder.
pub struct Folder {
    pub path: PathBuf,
    pub config: Config,
    pub identity: Identity,
}

#[derive(Debug)]
pub struct FolderError(pub String);

impl std::fmt::Display for FolderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FolderError {}

/// What a new world is made with.
pub struct NewWorld<'a> {
    pub name: &'a str,
    pub content: &'a Path,
    pub bind: &'a str,
    pub address: &'a str,
}

impl Folder {
    /// Make a new world in `path`, which must not hold one already.
    pub fn create(path: &Path, new: NewWorld) -> Result<Folder, FolderError> {
        if path.join("world.toml").exists() {
            return Err(FolderError(format!(
                "{} already holds a world",
                path.display()
            )));
        }
        let content = fs::canonicalize(new.content).map_err(|e| {
            FolderError(format!("the content folder {}: {e}", new.content.display()))
        })?;
        tme_content::load(&content)
            .map_err(|p| FolderError(format!("the content doesn't compile:\n{p}")))?;
        // The folder holds the world's credentials: a new one is its host's
        // alone, and each credential's file is, wherever the folder is.
        let mut folder = fs::DirBuilder::new();
        folder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            folder.mode(0o700);
        }
        folder
            .create(path)
            .map_err(|e| FolderError(format!("can't make {}: {e}", path.display())))?;
        let config = Config {
            name: new.name.to_owned(),
            // TOML's integers are signed, so 63 bits.
            seed: u64::from_le_bytes(random()[..8].try_into().expect("8 bytes")) >> 1,
            bind: new.bind.to_owned(),
            address: new.address.to_owned(),
            invite: invite_code(),
            content,
            harm_between_players: false,
        };
        let key = path.join("world.key");
        let identity = Identity::create(&key)
            .map_err(|e| FolderError(format!("can't write {}: {e}", key.display())))?;
        let text = toml::to_string(&config).expect("the config encodes");
        create_private(&path.join("world.toml"))
            .and_then(|mut f| f.write_all(text.as_bytes()))
            .map_err(|e| FolderError(format!("can't write world.toml: {e}")))?;
        Ok(Folder {
            path: path.to_owned(),
            config,
            identity,
        })
    }

    pub fn open(path: &Path) -> Result<Folder, FolderError> {
        let toml_path = path.join("world.toml");
        let text = fs::read_to_string(&toml_path).map_err(|e| {
            FolderError(format!(
                "{}: {e} (make a world with `tme-server init`)",
                toml_path.display()
            ))
        })?;
        // Not the error's own text: it quotes the line, which can be the
        // invite code.
        let config: Config = toml::from_str(&text).map_err(|e| {
            let line = e
                .span()
                .map(|s| format!(" on line {}", text[..s.start].lines().count().max(1)));
            FolderError(format!(
                "{}{}: {}",
                toml_path.display(),
                line.unwrap_or_default(),
                e.message()
            ))
        })?;
        let identity = Identity::load(&path.join("world.key"))
            .map_err(|e| FolderError(format!("world.key: {e}")))?;
        Ok(Folder {
            path: path.to_owned(),
            config,
            identity,
        })
    }

    pub fn database(&self) -> PathBuf {
        self.path.join("world.db")
    }

    /// Hold the world for one server: the lock on `world.lock` lasts as
    /// long as what's returned, and ends with the process however it ends.
    /// One server at a time restores and runs a world, however its folder
    /// is reached.
    pub fn lock(&self) -> Result<fs::File, FolderError> {
        let path = self.path.join("world.lock");
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(|e| FolderError(format!("{}: {e}", path.display())))?;
        match file.try_lock() {
            Ok(()) => Ok(file),
            Err(fs::TryLockError::WouldBlock) => Err(FolderError(format!(
                "another server is running the world in {}",
                self.path.display()
            ))),
            Err(fs::TryLockError::Error(e)) => Err(FolderError(format!("{}: {e}", path.display()))),
        }
    }

    /// The link that brings someone new into this world.
    pub fn invite(&self) -> Invite {
        Invite {
            address: self.config.address.clone(),
            world: self.identity.public(),
            code: Some(self.config.invite.clone()),
        }
    }
}

fn random() -> [u8; 32] {
    let mut out = [0u8; 32];
    getrandom::fill(&mut out).expect("the operating system has randomness");
    out
}

/// Twelve letters and digits, about 62 bits.
fn invite_code() -> String {
    const ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789";
    random()[..12]
        .iter()
        .map(|b| ALPHABET[usize::from(*b) % ALPHABET.len()] as char)
        .collect()
}
