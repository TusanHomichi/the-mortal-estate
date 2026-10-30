//! A world's SQLite file: its accounts, its journal of accepted inputs,
//! how many steps it has completed, and its snapshots, each with the
//! content it was taken on. Restoring is the latest snapshot plus the
//! journal since, played on that snapshot's content up to the last step
//! completed, which works because the simulation is deterministic.

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use std::path::Path;
use std::time::Duration;
use tme_sim::Catalogue;
use tme_sim::world::{Input, PersonId, State};
use tme_wire::PublicKey;

/// The layout of what's saved: snapshots, content and journal entries are
/// the simulation's own types, encoded. A world saved in another layout
/// won't load (the schema resets freely until worlds have players).
const FORMAT: i64 = 2;

/// How many snapshots to keep; the journal is kept whole.
const KEEP_SNAPSHOTS: i64 = 3;

/// How long a write waits on someone else holding the database before it
/// fails: nothing else should write it while a server runs it.
const BUSY: Duration = Duration::from_secs(1);

/// The content a snapshot was taken on, as the store numbers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentId(i64);

pub struct Store {
    db: Connection,
}

/// What a world was, as saved: its latest snapshot and the content it was
/// taken on, if there's one yet, every journal entry since, in order, and
/// how many steps the world had completed.
#[derive(Debug, PartialEq)]
pub struct Saved {
    pub snapshot: Option<(State, Catalogue)>,
    pub since: Vec<(u64, Input)>,
    pub stepped: u64,
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
        db.busy_timeout(BUSY)?;
        db.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
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
             CREATE TABLE IF NOT EXISTS contents (
                 id INTEGER PRIMARY KEY,
                 catalogue BLOB NOT NULL UNIQUE
             );
             CREATE TABLE IF NOT EXISTS snapshots (
                 after_seq INTEGER PRIMARY KEY,
                 step INTEGER NOT NULL,
                 state BLOB NOT NULL,
                 content INTEGER NOT NULL REFERENCES contents (id)
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
                    "INSERT INTO meta (key, value) VALUES ('format', ?1), ('stepped', 0)",
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

    /// Append accepted inputs, each with the step it was applied before,
    /// and record that the world has completed `stepped` steps, in one
    /// transaction.
    pub fn journal(&mut self, entries: &[(u64, Input)], stepped: u64) -> Result<(), StoreError> {
        let tx = self.db.transaction()?;
        append(&tx, entries, stepped)?;
        tx.commit()?;
        Ok(())
    }

    /// Save a snapshot, taken on `content` after every entry journalled so
    /// far.
    pub fn snapshot(&mut self, state: &State, content: ContentId) -> Result<(), StoreError> {
        let tx = self.db.transaction()?;
        snapshot(&tx, state, content)?;
        tx.commit()?;
        Ok(())
    }

    /// Start a run: journal `entries`, then snapshot `state` on
    /// `catalogue`, which becomes the content later snapshots are taken on,
    /// in one transaction.
    pub fn begin(
        &mut self,
        entries: &[(u64, Input)],
        state: &State,
        catalogue: &Catalogue,
    ) -> Result<ContentId, StoreError> {
        let bytes = postcard::to_stdvec(catalogue).expect("a catalogue encodes");
        let tx = self.db.transaction()?;
        append(&tx, entries, state.step)?;
        tx.execute(
            "INSERT INTO contents (catalogue) VALUES (?1) ON CONFLICT DO NOTHING",
            [&bytes],
        )?;
        let content = ContentId(tx.query_row(
            "SELECT id FROM contents WHERE catalogue = ?1",
            [&bytes],
            |r| r.get(0),
        )?);
        snapshot(&tx, state, content)?;
        tx.commit()?;
        Ok(content)
    }

    /// The world as last saved.
    pub fn load(&self) -> Result<Saved, StoreError> {
        let snap: Option<(i64, Vec<u8>, Vec<u8>)> = self
            .db
            .query_row(
                "SELECT after_seq, state, catalogue FROM snapshots
                     JOIN contents ON contents.id = snapshots.content
                     ORDER BY after_seq DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let (after, snapshot) = match snap {
            Some((after, state, catalogue)) => {
                let state = postcard::from_bytes(&state)
                    .map_err(|e| StoreError::Corrupt(format!("a snapshot: {e}")))?;
                let catalogue = postcard::from_bytes(&catalogue)
                    .map_err(|e| StoreError::Corrupt(format!("a snapshot's content: {e}")))?;
                (after, Some((state, catalogue)))
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
        let stepped: i64 =
            self.db
                .query_row("SELECT value FROM meta WHERE key = 'stepped'", [], |r| {
                    r.get(0)
                })?;
        Ok(Saved {
            snapshot,
            since,
            stepped: stepped as u64,
        })
    }
}

fn append(tx: &Transaction, entries: &[(u64, Input)], stepped: u64) -> Result<(), StoreError> {
    let mut insert = tx.prepare_cached("INSERT INTO journal (step, input) VALUES (?1, ?2)")?;
    for (step, input) in entries {
        let bytes = postcard::to_stdvec(input).expect("inputs encode");
        insert.execute(params![*step as i64, bytes])?;
    }
    tx.execute(
        "UPDATE meta SET value = MAX(value, ?1) WHERE key = 'stepped'",
        [stepped as i64],
    )?;
    Ok(())
}

fn snapshot(tx: &Transaction, state: &State, content: ContentId) -> Result<(), StoreError> {
    let bytes = postcard::to_stdvec(state).expect("a world's state encodes");
    let after: i64 = tx.query_row("SELECT COALESCE(MAX(seq), 0) FROM journal", [], |r| {
        r.get(0)
    })?;
    tx.execute(
        "INSERT OR REPLACE INTO snapshots (after_seq, step, state, content)
             VALUES (?1, ?2, ?3, ?4)",
        params![after, state.step as i64, bytes, content.0],
    )?;
    tx.execute(
        "UPDATE meta SET value = MAX(value, ?1) WHERE key = 'stepped'",
        [state.step as i64],
    )?;
    tx.execute(
        "DELETE FROM snapshots WHERE after_seq NOT IN
             (SELECT after_seq FROM snapshots ORDER BY after_seq DESC LIMIT ?1)",
        [KEEP_SNAPSHOTS],
    )?;
    tx.execute(
        "DELETE FROM contents WHERE id NOT IN (SELECT content FROM snapshots)",
        [],
    )?;
    Ok(())
}
