//! A world's SQLite file: its accounts, its journal of accepted inputs and
//! its snapshots. Restoring is the latest snapshot plus the journal since,
//! which works because the simulation is deterministic.

use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use tme_sim::world::{Input, PersonId, State};
use tme_wire::PublicKey;

/// The layout of what's saved: snapshots and journal entries are the
/// simulation's own types, encoded. A world saved in another layout won't
/// load (the schema resets freely until worlds have players).
const FORMAT: i64 = 1;

/// How many snapshots to keep; the journal is kept whole.
const KEEP_SNAPSHOTS: i64 = 3;

pub struct Store {
    db: Connection,
}

/// What a world was, as saved: its latest snapshot, if any, and every
/// journal entry since, in order.
pub struct Saved {
    pub snapshot: Option<State>,
    pub since: Vec<(u64, Input)>,
}

#[derive(Debug)]
pub enum StoreError {
    Db(rusqlite::Error),
    /// Saved by a server whose layout differs.
    Format {
        found: i64,
    },
    /// An entry that doesn't decode.
    Corrupt(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Db(e) => write!(f, "the world's database: {e}"),
            StoreError::Format { found } => write!(
                f,
                "the world was saved in layout {found} and this server reads layout {FORMAT}; start a new world, or run the server it was saved with"
            ),
            StoreError::Corrupt(what) => write!(f, "the world's database is damaged: {what}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> StoreError {
        StoreError::Db(e)
    }
}

impl Store {
    /// Open the world's database, making it if it's new.
    pub fn open(path: &Path) -> Result<Store, StoreError> {
        let db = Connection::open(path)?;
        db.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS accounts (
                 key BLOB PRIMARY KEY,
                 person INTEGER NOT NULL UNIQUE
             );
             CREATE TABLE IF NOT EXISTS journal (
                 seq INTEGER PRIMARY KEY AUTOINCREMENT,
                 step INTEGER NOT NULL,
                 input BLOB NOT NULL
             );
             CREATE TABLE IF NOT EXISTS snapshots (
                 after_seq INTEGER PRIMARY KEY,
                 step INTEGER NOT NULL,
                 state BLOB NOT NULL
             );",
        )?;
        let found: Option<i64> = db
            .query_row("SELECT value FROM meta WHERE key = 'format'", [], |r| {
                r.get(0)
            })
            .optional()?;
        match found {
            None => {
                db.execute(
                    "INSERT INTO meta (key, value) VALUES ('format', ?1)",
                    [FORMAT],
                )?;
            }
            Some(FORMAT) => {}
            Some(found) => return Err(StoreError::Format { found }),
        }
        Ok(Store { db })
    }

    /// The person `key` is in this world, if they've been here before.
    pub fn account(&self, key: &PublicKey) -> Result<Option<PersonId>, StoreError> {
        Ok(self
            .db
            .query_row(
                "SELECT person FROM accounts WHERE key = ?1",
                [&key.0[..]],
                |r| r.get(0),
            )
            .optional()?
            .map(PersonId))
    }

    /// A new account for `key`, as the next person in the world.
    pub fn create_account(&self, key: &PublicKey) -> Result<PersonId, StoreError> {
        let next: u32 = self.db.query_row(
            "SELECT COALESCE(MAX(person), 0) + 1 FROM accounts",
            [],
            |r| r.get(0),
        )?;
        self.db.execute(
            "INSERT INTO accounts (key, person) VALUES (?1, ?2)",
            params![&key.0[..], next],
        )?;
        Ok(PersonId(next))
    }

    /// Append accepted inputs, each with the step it was applied before, in
    /// one transaction.
    pub fn journal(&mut self, entries: &[(u64, Input)]) -> Result<(), StoreError> {
        let tx = self.db.transaction()?;
        {
            let mut insert =
                tx.prepare_cached("INSERT INTO journal (step, input) VALUES (?1, ?2)")?;
            for (step, input) in entries {
                let bytes = postcard::to_stdvec(input).expect("inputs encode");
                insert.execute(params![*step as i64, bytes])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Save a snapshot taken after every entry journalled so far.
    pub fn snapshot(&mut self, state: &State) -> Result<(), StoreError> {
        let bytes = postcard::to_stdvec(state).expect("a world's state encodes");
        let tx = self.db.transaction()?;
        let after: i64 = tx.query_row("SELECT COALESCE(MAX(seq), 0) FROM journal", [], |r| {
            r.get(0)
        })?;
        tx.execute(
            "INSERT OR REPLACE INTO snapshots (after_seq, step, state) VALUES (?1, ?2, ?3)",
            params![after, state.step as i64, bytes],
        )?;
        tx.execute(
            "DELETE FROM snapshots WHERE after_seq NOT IN
                 (SELECT after_seq FROM snapshots ORDER BY after_seq DESC LIMIT ?1)",
            [KEEP_SNAPSHOTS],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// The world as last saved.
    pub fn load(&self) -> Result<Saved, StoreError> {
        let snap: Option<(i64, Vec<u8>)> = self
            .db
            .query_row(
                "SELECT after_seq, state FROM snapshots ORDER BY after_seq DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (after, snapshot) = match snap {
            Some((after, bytes)) => {
                let state = postcard::from_bytes(&bytes)
                    .map_err(|e| StoreError::Corrupt(format!("a snapshot: {e}")))?;
                (after, Some(state))
            }
            None => (0, None),
        };
        let mut query = self
            .db
            .prepare("SELECT seq, step, input FROM journal WHERE seq > ?1 ORDER BY seq")?;
        let rows = query.query_map([after], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Vec<u8>>(2)?,
            ))
        })?;
        let mut since = Vec::new();
        for row in rows {
            let (seq, step, bytes) = row?;
            let input = postcard::from_bytes(&bytes)
                .map_err(|e| StoreError::Corrupt(format!("journal entry {seq}: {e}")))?;
            since.push((step as u64, input));
        }
        Ok(Saved { snapshot, since })
    }
}
