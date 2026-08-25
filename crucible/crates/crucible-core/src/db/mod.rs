//! The database. This repo's first, and the four places its spec was wrong.
//!
//! `crucible-spec.md` §8 sketches a schema. The sketch is right about the
//! driver, the journal mode and the single writer, and wrong in four specific
//! ways -- each of which would have cost a number rather than an afternoon.
//! All four are fixed here, and the fixes are why this module exists at all
//! rather than being a transcription:
//!
//! **1. It keys the engine on `tag`.** The primary trigger for a sweep is an
//! UNTAGGED working-tree candidate, so `tag` is NULL in the normal case; and
//! `ver` is not unique either, because every dev build of a cycle reports the
//! same `ff 0.25.0`. Two builds reporting one version and producing different
//! coverage is precisely the comparison this harness exists to make, and a
//! version-keyed table cannot express it. The engine is keyed on the binary's
//! BLAKE3, supplied by the caller -- this crate hashes nothing itself, and does
//! not take a `blake3` dependency to do it.
//!
//! **2. There is no `board` concept.** The spec models a run as
//! `(tag, problem, config)`, which cannot express what the resume gate actually
//! compares. `ipc67.py:load_resume` reuses a prior row only when `ver`,
//! `budget`, `mode`, `jobs` and `threads` all match EXACTLY, so the unit of
//! work and of row identity is `(name, budget, mode, jobs, threads, env, args)`
//! and `board` carries a UNIQUE over exactly that tuple. A 30 s -> 60 s tier
//! move therefore creates a NEW board row, which is correct: it is a different
//! measurement, and `standings.py` denominates the timeout class in the row's
//! own budget. Merging the two would silently re-class every 30 s wall-exit as
//! an early-exit -- a lie in the one column the refill loop is refereed by.
//!
//! **3. `domain` and `problem` are the wrong nouns.** A "domain" in this repo
//! is a corpus VARIANT DIRECTORY (`elevator-sequential-satisficing`); several
//! variants share one PDDL domain file and some carry a per-instance one. And a
//! problem's key is an instance LABEL that is an integer for a
//! single-digit-group filename and an underscore-joined string (`"3_10_50_10"`)
//! otherwise -- collapsing multipart labels to their first group once put
//! `ipc2026-numeric`'s 320 rows under 288 keys. The label is stored as TEXT
//! with a `label_is_int` flag and a zero-padded `sort_key`, so `ORDER BY`
//! reproduces the Python's numeric-tuple sort exactly.
//!
//! **4. `validated INTEGER` flattens a tristate and drops the reason.** `val`
//! is NULL for UNAVAILABLE, 0 for REJECTED and 1 for valid, with a `val_reason`
//! beside it. **Every query touching `val` must use `IS NULL`, never `= 0`.**
//! That single confusion is three separate published-number incidents -- the
//! 0.20 table read 46/240 and 113/320 where the boards beside it said 53 and
//! 121, fifteen instances light, because domains VAL could not INGEST were
//! counted as domains VAL had REJECTED. The warning is repeated on the column
//! itself, where somebody writing a new query will actually be looking.
//!
//! # One thing the schema deliberately does not do
//!
//! It does not store a verdict. The spec's `state` column carries
//! `solved|unsolved|timeout|error|invalid`, which is a taxonomy -- and this
//! codebase already has exactly one, in `crucible_publish::Referee`, ported
//! line by line from `standings.py` with a test per incident. A second
//! taxonomy in SQL would drift from it, and the drift would be invisible until
//! a table disagreed with a board. `run.state` is the QUEUE state and nothing
//! more; classification happens where the tests are.
//!
//! # Shape
//!
//! * [`lock`] -- one crucible per database directory, enforced by `flock`.
//! * [`schema`] -- the DDL and the `user_version` ladder.
//! * [`writer`] -- the single thread that owns the one read-write connection.
//! * [`read`] -- `query_only` connections, and the canonical export order.
//! * [`rebuild`] -- the repo is the source of truth; this puts it in and takes
//!   it back out.

pub mod lock;
pub mod model;
pub mod read;
pub mod rebuild;
pub mod schema;
pub mod writer;

pub use lock::{DirLock, LockError};
pub use model::{
    BoardFacts, BoardKey, BoardPassRec, Cleanliness, EngineFacts, EngineKey, EventRec, InstanceKey,
    LiveChild, Measured, PassVerdict, RunRecord, RunState, SampleRec, ThrottleWindowRec,
    TimingQuality, ValReason, VariantKey,
};
pub use read::Reader;
pub use rebuild::{export, export_to, rebuild_from_artifacts, RebuiltBoard};
pub use writer::{Writer, WriterHandle};

use std::path::{Path, PathBuf};

/// The database file's name inside the directory the lock guards.
pub const DB_FILE_NAME: &str = "crucible.db";

/// Anything that can go wrong between a caller and the disk.
///
/// `WriterGone` is separate from a SQL error on purpose: it means the writer
/// thread died, which is not a failed statement and is never retryable.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("sqlite: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error(transparent)]
    Migrate(#[from] schema::MigrateError),
    #[error(transparent)]
    Lock(#[from] LockError),
    #[error("the database writer thread is gone")]
    WriterGone,
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// A board raw exists and does not parse. Distinct from a MISSING raw,
    /// which is a known state of the world and degrades to nothing.
    #[error("{0}")]
    Parse(String),
    /// A board raw carries two rows for one instance, so it cannot be cached
    /// without losing one.
    ///
    /// `run` is UNIQUE on (board, instance, engine, attempt) and the import
    /// upserts, so the second row would overwrite the first and the board would
    /// come back out SHORTER than it went in. Two committed raws are like this
    /// -- `benchmarks/air/ipc2026-numeric.jsonl` and its 0.19 sibling, written
    /// before `ipc67.py` learned to keep every digit group of a multipart
    /// filename, which is the collapse that put 320 rows under 288 keys.
    /// Refusing is the only honest answer available: a silently 288-row board
    /// is a wrong denominator, and nothing downstream would say so.
    #[error(
        "{path}: {variant} instance {instance} appears {count} times in one board. \
         Raws written before the 0.20 multipart-label fix collapse every digit \
         group but the first, so the file carries fewer distinct instances than \
         rows -- caching it would drop the duplicates silently. Re-sweep the \
         board, or read this raw directly; it is not cacheable as it stands"
    )]
    DuplicateInstance {
        path: String,
        variant: String,
        instance: String,
        count: usize,
    },
    #[error("encoding a row for storage: {0}")]
    Encode(String),
}

/// An open database: the lock, the writer thread, and a factory for readers.
///
/// Dropping it flushes the batch, stops the writer and releases the lock -- in
/// that order, because a Stop that raced the batch would throw away the last
/// two hundred milliseconds of telemetry.
pub struct Db {
    path: PathBuf,
    writer: Writer,
    // Declared last so it is dropped last: the lock must outlive the writer, or
    // a second crucible could start against a database still being written to.
    _lock: DirLock,
}

impl Db {
    /// Open (or create) the database in `dir`, taking the directory lock.
    ///
    /// Fails with [`LockError::Busy`] if another crucible already holds it,
    /// which is an operational answer rather than a fault: the other one is
    /// still running.
    pub fn open(dir: &Path) -> Result<Db, DbError> {
        let lock = DirLock::acquire(dir)?;
        let path = dir.join(DB_FILE_NAME);
        let conn = rusqlite::Connection::open(&path)?;
        configure(&conn)?;
        schema::migrate(&conn)?;
        Ok(Db {
            path,
            writer: Writer::start(conn),
            _lock: lock,
        })
    }

    /// The one writer. Cheap to clone out of.
    pub fn writer(&self) -> &WriterHandle {
        self.writer.handle()
    }

    /// A fresh read-only connection. One per reader, never shared: SQLite
    /// connections are not `Sync`, and sharing one behind a mutex would
    /// serialise the readers WAL mode exists to keep concurrent.
    pub fn reader(&self) -> Result<Reader, DbError> {
        Reader::open(&self.path)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// The pragmas, and why each one.
fn configure(conn: &rusqlite::Connection) -> Result<(), DbError> {
    // WAL: readers never block the writer, which is what lets the TUI redraw
    // four times a second against a database a sweep is writing to.
    //
    // An in-memory database answers "memory" here and cannot do WAL at all;
    // that is not a fault, so the answer is read and discarded rather than
    // asserted. Tests open memory databases.
    let _mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
    conn.execute_batch(
        // synchronous=NORMAL in WAL does not fsync on every commit. It still
        // survives a crash of THIS process -- the data is in the OS -- which is
        // the failure this harness actually suffers; only a power cut can lose
        // the tail. And for that case the repo's committed `.jsonl` files are
        // the durable record and this database is a cache, which is the whole
        // premise of `db::rebuild`.
        "PRAGMA synchronous=NORMAL;\
         PRAGMA foreign_keys=ON;\
         PRAGMA busy_timeout=5000;",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn scratch(tag: &str) -> Scratch {
        let d = std::env::temp_dir().join(format!(
            "crucible-db-{}-{}-{tag}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&d).unwrap();
        Scratch(d)
    }

    /// The lock is held for the life of the `Db`, not just of `open`. A second
    /// crucible starting against a live database is two schedulers each
    /// believing they own the queue.
    #[test]
    fn a_second_db_on_the_same_directory_is_refused() {
        let dir = scratch("busy");
        let _first = Db::open(&dir.0).expect("first open");
        match Db::open(&dir.0) {
            Err(DbError::Lock(LockError::Busy { .. })) => {}
            Err(other) => panic!("expected a busy lock, got {other:?}"),
            // `Db` is deliberately not `Debug` -- it owns a live connection and
            // a thread -- so the success case is named rather than formatted.
            Ok(_) => panic!("a second crucible opened a database another holds"),
        }
    }

    /// ...and released when it drops, or a clean restart would be refused with
    /// no symptom except a sweep that never starts.
    #[test]
    fn closing_releases_the_lock() {
        let dir = scratch("reopen");
        drop(Db::open(&dir.0).expect("first open"));
        let _second = Db::open(&dir.0).expect("reopen after close");
    }

    /// Foreign keys must be ON. They are OFF by default in SQLite, and a
    /// dangling `run.instance_id` is a row that exports as nothing at all.
    #[test]
    fn foreign_keys_are_enforced() {
        let dir = scratch("fk");
        let db = Db::open(&dir.0).expect("open");
        let r = db.reader().expect("reader");
        let on: i64 = r
            .conn()
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(on, 1);
    }

    /// A reader must not be able to write, whatever it asks for. `query_only`
    /// is the guarantee; without it a stray statement in a rendering path
    /// could take the write lock out from under a sweep.
    #[test]
    fn a_reader_cannot_write() {
        let dir = scratch("ro");
        let db = Db::open(&dir.0).expect("open");
        let r = db.reader().expect("reader");
        assert!(r
            .conn()
            .execute("INSERT INTO variant(ipc,name) VALUES('x','y')", [])
            .is_err());
    }
}
