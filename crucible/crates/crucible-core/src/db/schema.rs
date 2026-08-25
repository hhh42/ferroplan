//! The DDL, and the `user_version` ladder that applies it.
//!
//! Every column comment below is load-bearing; read them before changing a
//! type. The four places this schema deliberately departs from
//! `crucible-spec.md` §8 are argued in the module header of `db/mod.rs` -- this
//! file is where those arguments become constraints.
//!
//! # Migrations
//!
//! Keyed on `PRAGMA user_version`, applied in order, each inside its own
//! transaction (SQLite makes `PRAGMA user_version` transactional, so a
//! half-applied migration cannot leave the version claiming it finished).
//!
//! A database stamped with a version this binary does not know is REFUSED, not
//! opened. An old binary that reads a new schema will not crash -- it will
//! quietly ignore the columns it does not know about, write rows missing them,
//! and the damage shows up weeks later as a board that cannot be attributed.

use rusqlite::Connection;

/// The schema version this binary writes.
pub const USER_VERSION: i32 = 1;

/// Refusing to open, with the numbers a human needs to know which binary to run.
#[derive(Debug, thiserror::Error)]
pub enum MigrateError {
    #[error(
        "database schema version {found} is newer than this binary understands \
         ({known}); run a newer crucible rather than letting an old one write to it"
    )]
    FromTheFuture { found: i32, known: i32 },
    #[error("migration to v{version} failed: {source}")]
    Sql {
        version: i32,
        #[source]
        source: rusqlite::Error,
    },
    #[error("reading schema version: {0}")]
    Version(#[source] rusqlite::Error),
}

/// v1 -- the whole schema. There is no v0 to migrate from; the first release
/// of a database gets to be a single statement.
const V1: &str = r#"
BEGIN;

-- ---------------------------------------------------------------------------
-- engine: WHICH BINARY. Keyed on BLAKE3, not on a tag and not on a version
-- string. The primary trigger for a sweep is an untagged working-tree build,
-- so `tag` is NULL in the normal case; and `ver` is not unique -- every dev
-- build of a cycle reports the same "ff 0.25.0". Two builds that report the
-- same version and produce different coverage is precisely the comparison this
-- harness exists to make, and a version-keyed table cannot express it.
--
-- blake3 is supplied by the caller (crucible-core does not hash anything
-- itself) and is NULL only for an engine reconstructed from artifacts, where
-- the binary is long gone. The partial index below keeps exactly one such row
-- per version string, so a rebuild cannot fan a board out across
-- indistinguishable phantom engines.
-- ---------------------------------------------------------------------------
CREATE TABLE engine (
  id           INTEGER PRIMARY KEY,
  blake3       TEXT UNIQUE,                 -- the identity; NULL => rebuilt
  ver          TEXT,                        -- "ff 0.25.0"; NOT an identity
  tag          TEXT,                        -- NULL is the NORMAL case
  commit_sha   TEXT,
  binary_path  TEXT,
  built_at     INTEGER,
  build_status TEXT CHECK (build_status IS NULL OR build_status IN ('ok','failed')),
  build_log    TEXT,
  source       TEXT NOT NULL DEFAULT 'measured'
               CHECK (source IN ('measured','rebuilt'))
);
CREATE UNIQUE INDEX engine_rebuilt_ver ON engine(ifnull(ver,'')) WHERE blake3 IS NULL;

-- ---------------------------------------------------------------------------
-- board: the unit of work AND of row identity.
--
-- The spec has no board concept at all; it models a run as (tag, problem,
-- config), which cannot express the thing the resume gate actually compares.
-- `ipc67.py:load_resume` reuses a prior row ONLY when ver, budget, mode, jobs
-- and threads all match EXACTLY, so those are what a measurement is made of.
--
-- The UNIQUE below is over exactly that tuple, which means a 30s -> 60s tier
-- move creates a NEW board row rather than updating one. That is correct and
-- is the point: a row measured under a 60 s wall is a different measurement
-- from one measured under 30 s, and `standings.py` classifies the timeout line
-- as a fraction of the row's own budget. Merging them silently re-classes every
-- 30 s wall-exit as an early-exit.
--
-- budget_secs is the ARMED wall -- the row's own `budget` stamp, i.e. ipc67's
-- TIMEOUT -- and NOT the manifest's scored `budget_secs`, which may legitimately
-- differ from it for one cycle while a tier move is in flight.
--
-- threads is TEXT because the gate compares `str(threads)`: the runner passes
-- the CLI argument through unconverted, so the raws carry a JSON string "2".
-- threads_json holds the exact JSON token so an export cannot change its type.
-- ---------------------------------------------------------------------------
CREATE TABLE board (
  id           INTEGER PRIMARY KEY,
  name         TEXT NOT NULL,               -- the manifest board id
  budget_secs  REAL NOT NULL,               -- the ARMED wall, per the row's stamp
  mode         TEXT NOT NULL,               -- normalised: "auto", never NULL
  jobs         INTEGER NOT NULL,
  threads      TEXT NOT NULL,               -- str(threads), the gate's currency
  env          TEXT NOT NULL,               -- canonical JSON object, keys sorted
  args         TEXT NOT NULL,               -- canonical JSON array, order kept
  threads_json TEXT NOT NULL,               -- the exact token the raw carries
  label        TEXT,                        -- manifest label, for reporting only
  competition  TEXT,
  proof_track  INTEGER NOT NULL DEFAULT 0,
  UNIQUE (name, budget_secs, mode, jobs, threads, env, args)
);

-- ---------------------------------------------------------------------------
-- variant: a corpus VARIANT DIRECTORY (elevator-sequential-satisficing), which
-- is what this repo means by the word the spec spells "domain". A variant is
-- not a PDDL domain: several variants share one domain file, and one variant
-- can carry a per-instance domain.
--
-- SQLite's UNIQUE does not constrain NULLs, so a row that carried no `ipc` is
-- not protected by the index below. The single writer thread is what closes
-- that gap: resolution is a SELECT ... WHERE ipc IS ? followed by an INSERT on
-- one thread, so there is no interleaving to lose to.
-- ---------------------------------------------------------------------------
CREATE TABLE variant (
  id   INTEGER PRIMARY KEY,
  ipc  TEXT,                                -- ipc-2014, ipc-2026n; NULL possible
  name TEXT NOT NULL,
  UNIQUE (ipc, name)
);

-- ---------------------------------------------------------------------------
-- instance: keyed on the LABEL, which is an integer for a single-digit-group
-- filename and an underscore-joined string ("3_10_50_10") otherwise. Collapsing
-- a multipart label to its first group put ipc2026-numeric's 320 rows under 288
-- keys and silently broke both the per-instance diff and the --score-against
-- join; label_is_int is what lets an export put the int back as an int.
--
-- sort_key exists so ORDER BY reproduces `ipc67.py:instances`, which sorts by
-- the TUPLE of ints of every digit group. It is each group written as a
-- three-digit length prefix followed by its significant digits, joined by '.'
-- -- an encoding in which byte order equals numeric-tuple order, including the
-- prefix rule that (3,10) sorts before (3,10,50,10).
-- ---------------------------------------------------------------------------
CREATE TABLE instance (
  id             INTEGER PRIMARY KEY,
  variant_id     INTEGER NOT NULL REFERENCES variant(id),
  label          TEXT NOT NULL,             -- "7" or "3_10_50_10"
  label_is_int   INTEGER NOT NULL,          -- 1 => the raw writes a JSON int
  sort_key       TEXT NOT NULL,             -- byte order == numeric-tuple order
  pddl_path      TEXT,
  timing_matters INTEGER NOT NULL DEFAULT 0,
  UNIQUE (variant_id, label)
);
CREATE INDEX instance_sort_idx ON instance(variant_id, sort_key);

-- ---------------------------------------------------------------------------
-- run: one measured instance. THIS TABLE IS THE RECEIPT.
--
-- Every field an exported `.jsonl` line contains is read from here, including
-- the ones that also appear on `board` (mode, jobs, threads, budget). That
-- duplication is deliberate: reading a stamp back through a join makes the
-- exported bytes depend on the join being right, and this project's failure
-- mode is a wrong published number, not a wide table.
--
-- What is NOT here is a verdict. There is exactly one classifier in this
-- codebase -- crucible_publish::Referee -- and a `state` column carrying
-- solved/unsolved/timeout/error/invalid, as the spec sketches it, would be a
-- second one that drifts. `state` is the QUEUE state and nothing more.
-- ---------------------------------------------------------------------------
CREATE TABLE run (
  id            INTEGER PRIMARY KEY,
  board_id      INTEGER NOT NULL REFERENCES board(id),
  instance_id   INTEGER NOT NULL REFERENCES instance(id),
  engine_id     INTEGER NOT NULL REFERENCES engine(id),
  attempt       INTEGER NOT NULL DEFAULT 1,

  state         TEXT NOT NULL
                CHECK (state IN ('pending','running','suspended','done','abandoned')),

  -- Contention may cost a timing. It may never cost a result. 'unknown' is the
  -- default because the spec's DEFAULT 'clean' manufactures exactly the claim
  -- the contention watcher exists to refuse.
  timing_quality TEXT NOT NULL DEFAULT 'unknown'
                CHECK (timing_quality IN ('clean','dirty','unknown')),

  -- ------------------------------------------------------------------------
  -- The raw row, field for field.
  -- ------------------------------------------------------------------------
  solved        INTEGER NOT NULL,

  -- `time` is genuinely polymorphic: the hard-timeout path assigns the INTEGER
  -- budget, every other path writes round(el, 2). time_json is the exact JSON
  -- token and is what an export reads; time_secs is the same value as a number
  -- and is what a query reads. Two columns, one measurement, and the receipt
  -- is the one that cannot be rounded by a storage layer.
  time_secs     REAL,
  time_json     TEXT,

  -- *** NO DECLARED TYPE, AND THAT IS NOT AN OMISSION. ***
  --
  -- A REAL-affinity column runs every stored double through SQLite's integer
  -- optimisation (datatype3.html 3.1: "small floating point values with no
  -- fractional component ... are written to disk as integers"), and the round
  -- trip through an integer LOSES THE SIGN OF NEGATIVE ZERO. `metric: -0.0`
  -- comes back `0.0`.
  --
  -- That is not a curiosity here. Four rows of the committed
  -- `benchmarks/ipc67-netben.jsonl` carry `"metric": -0.0` (sixteen across the
  -- archives), because the net-benefit boards normalise a MAXIMISED metric
  -- into a minimised one and the sign is part of the measurement. With REAL
  -- affinity this board exports bytes the committed raw does not have, which
  -- is the one failure the round-trip test exists to catch.
  --
  -- The same trap applies to every float below that an export reads back, so
  -- all of them are declared the same way. `time_secs` keeps its REAL because
  -- it is the QUERY column -- the receipt is `time_json`, which is text.
  metric,
  length        INTEGER,

  -- TRISTATE. NULL = validation UNAVAILABLE, 0 = VAL REJECTED the plan,
  -- 1 = valid.
  --
  -- *** EVERY QUERY THAT TOUCHES THIS COLUMN MUST USE `val IS NULL`, NEVER
  -- `val = 0`. *** NULL is not a verdict. Reading it as one is the 0.20, 0.21
  -- and 0.23 incidents -- three separate published numbers, fifteen instances
  -- light on the 0.20 table alone, because domains VAL could not INGEST were
  -- counted as domains VAL had REJECTED. A `validated INTEGER` column of the
  -- kind the spec sketches cannot even express the difference.
  val           INTEGER CHECK (val IS NULL OR val IN (0,1)),
  -- Why validation was unavailable, when we know. NULL means "no reason
  -- recorded", which is itself honest -- it never means "valid".
  val_reason    TEXT CHECK (val_reason IS NULL OR
                            val_reason IN ('ingest','crash','timeout','no-validator')),

  -- null, a string (runner-stamped class) or a list of strings (the engine's
  -- own Solution.notes), stored as the exact JSON so neither shape is flattened.
  notes_json    TEXT,

  -- Untyped for the reason argued at `metric` above: these four are read back
  -- into an exported row, and REAL affinity would rewrite a negative zero.
  budget_secs,
  ver           TEXT,
  mode          TEXT,
  jobs          INTEGER,
  threads_json  TEXT,
  start_ts,
  end_ts,
  makespan,
  resumed_clean INTEGER NOT NULL DEFAULT 0,

  -- WHICH optional keys the source line physically carried. A solved
  -- non-temporal row writes "makespan": null; an unsolved row omits the key
  -- entirely. Both parse to None, so without these flags a re-serialised board
  -- differs from every committed raw.
  present_ipc           INTEGER NOT NULL DEFAULT 0,
  present_budget        INTEGER NOT NULL DEFAULT 0,
  present_stamps        INTEGER NOT NULL DEFAULT 0,
  present_makespan      INTEGER NOT NULL DEFAULT 0,
  present_resumed_clean INTEGER NOT NULL DEFAULT 0,

  -- Keys the runner learned after this binary was built. `write_row` does not
  -- emit them (the Python's key order is a fixed literal), so this is forensics
  -- rather than export -- but a database that is lossier than the file it was
  -- built from has no business being called a cache of it.
  extra_json    TEXT,

  -- ------------------------------------------------------------------------
  -- What the supervisor saw. Absent on a rebuilt run, which is why every one
  -- of these is nullable.
  -- ------------------------------------------------------------------------
  started_at    REAL,
  finished_at   REAL,
  wall_ms       INTEGER,
  cpu_ms        INTEGER,
  suspended_ms  INTEGER,
  peak_rss      INTEGER,
  mem_instrument TEXT,
  exit_code     INTEGER,
  term_signal   INTEGER,
  pid           INTEGER,
  pgid          INTEGER,

  UNIQUE (board_id, instance_id, engine_id, attempt)
);
CREATE INDEX run_state_idx  ON run(state);
CREATE INDEX run_board_idx  ON run(board_id, engine_id);
CREATE INDEX run_window_idx ON run(start_ts, end_ts);

-- ---------------------------------------------------------------------------
-- live_child: what we spawned, and enough to PROVE it is still what we spawned.
--
-- Startup orphan reaping without this table is a `killpg` on a number read off
-- disk. Pids recycle; process groups recycle with them; and killpg on a
-- recycled pgid kills a stranger's work with no error and no trace. The pair
-- (binary_path, proc_start_tvsec) is stable for the life of a process and cheap
-- to re-read, so a reaper can verify identity before it signals anything.
--
-- pid is the PRIMARY KEY precisely BECAUSE pids recycle: a fresh spawn onto a
-- recycled pid must REPLACE the stale row, not sit beside it.
-- ---------------------------------------------------------------------------
CREATE TABLE live_child (
  pid              INTEGER PRIMARY KEY,
  pgid             INTEGER NOT NULL,
  run_id           INTEGER REFERENCES run(id),
  binary_path      TEXT NOT NULL,
  proc_start_tvsec INTEGER NOT NULL,
  spawned_at       REAL NOT NULL,
  stopped          INTEGER NOT NULL DEFAULT 0
);

-- ---------------------------------------------------------------------------
-- board_pass: the .done marker, promoted from a zero-byte file to a record
-- with provenance. A marker on disk says a pass finished; it does not say what
-- the box was doing while it did, how many rows were actually measured, or how
-- many were stitched in from a prior pass -- all three of which change what the
-- board is allowed to claim.
--
-- verdict is contention.py's vocabulary verbatim, DEGRADED's shouting included,
-- plus 'feature-absent' for a board that could not be measured because the
-- engine under test lacks the feature it exercises. That last one is NOT a
-- degraded measurement and must never be counted as one.
-- ---------------------------------------------------------------------------
CREATE TABLE board_pass (
  id               INTEGER PRIMARY KEY,
  board_id         INTEGER NOT NULL REFERENCES board(id),
  engine_id        INTEGER NOT NULL REFERENCES engine(id),
  started_at       TEXT,                    -- as the conditions file spells it
  ended_at         TEXT,
  verdict          TEXT NOT NULL
                   CHECK (verdict IN ('clean','DEGRADED','unknown','feature-absent')),
  ran              INTEGER NOT NULL DEFAULT 0,
  reused           INTEGER NOT NULL DEFAULT 0,
  done_marker      TEXT,                    -- the .done path, when one exists
  raw_path         TEXT,
  conditions_path  TEXT,
  sample_interval  REAL,
  -- Identity for a rebuilt pass: the raw it was read from. NOT NULL with an
  -- empty default rather than nullable, because a table-level UNIQUE cannot
  -- carry an expression and SQLite's UNIQUE does not constrain NULLs anyway --
  -- a nullable column here would let a live pass be recorded twice. '' means
  -- "a live pass", which came from no file.
  source_path      TEXT NOT NULL DEFAULT '',
  UNIQUE (board_id, engine_id, source_path)
);

-- ---------------------------------------------------------------------------
-- sample / sample_process: the contention timeline, kept FOREVER.
--
-- This is the table that retires the conditions file. `ipc67.py:load_resume`
-- can only reuse a row whose window lies inside the span of ONE prior pass's
-- timeline, because a JSON blob beside a board raw is all it has; everything
-- outside that span fails closed and re-runs. A box-wide timeline in SQL turns
-- the gate into a range query with no span limitation at all.
--
-- competitors_total is NULLABLE on purpose: the historical timelines carry
-- nulls where the sampler could not attribute, and `load_resume` treats a null
-- as dirty. A NOT NULL column would have to invent a number there.
--
-- pass_id is set only on samples IMPORTED from a board's conditions file. The
-- live watcher's samples are box-wide and belong to no board.
-- ---------------------------------------------------------------------------
CREATE TABLE sample (
  id                INTEGER PRIMARY KEY,
  at                REAL NOT NULL,          -- epoch seconds, 1dp
  idle_pct          REAL,
  competitors_total REAL,                   -- NULL means "could not attribute"
  loadavg1          REAL,
  swap_mb           REAL,
  cpu_speed_limit   INTEGER,
  pass_id           INTEGER REFERENCES board_pass(id) ON DELETE CASCADE
);
CREATE INDEX sample_at_idx ON sample(at);

CREATE TABLE sample_process (
  sample_id INTEGER NOT NULL REFERENCES sample(id) ON DELETE CASCADE,
  name      TEXT NOT NULL,
  pcpu      REAL NOT NULL,
  PRIMARY KEY (sample_id, name)
) WITHOUT ROWID;

-- ---------------------------------------------------------------------------
-- throttle_window: what makes a run dirty. Kept as an interval rather than a
-- flag on the run, because a window that overlaps four concurrent runs has to
-- be able to mark all four without being written four times.
-- ---------------------------------------------------------------------------
CREATE TABLE throttle_window (
  id         INTEGER PRIMARY KEY,
  level      TEXT NOT NULL CHECK (level IN ('full','polite','suspended')),
  started_at REAL NOT NULL,
  ended_at   REAL,
  reason     TEXT
);
CREATE INDEX throttle_span_idx ON throttle_window(started_at, ended_at);

-- ---------------------------------------------------------------------------
-- event: the rolling log, and the audit trail behind every claim above.
-- ---------------------------------------------------------------------------
CREATE TABLE event (
  id       INTEGER PRIMARY KEY,
  at       REAL NOT NULL,
  level    TEXT NOT NULL CHECK (level IN ('info','warn','error')),
  kind     TEXT NOT NULL,
  run_id   INTEGER REFERENCES run(id),
  board_id INTEGER REFERENCES board(id),
  message  TEXT NOT NULL
);
CREATE INDEX event_at_idx ON event(at);

PRAGMA user_version = 1;
COMMIT;
"#;

/// Bring `conn` up to [`USER_VERSION`], or refuse to touch it.
pub fn migrate(conn: &Connection) -> Result<(), MigrateError> {
    let found: i32 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(MigrateError::Version)?;
    if found > USER_VERSION {
        return Err(MigrateError::FromTheFuture {
            found,
            known: USER_VERSION,
        });
    }
    if found < 1 {
        conn.execute_batch(V1)
            .map_err(|source| MigrateError::Sql { version: 1, source })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        migrate(&c).unwrap();
        c
    }

    /// A second migrate must be a no-op. The ladder is what a restart runs
    /// every single time, so an idempotency bug here is a crash on the second
    /// start, not the first -- the hardest kind to notice in a test run.
    #[test]
    fn migrate_is_idempotent() {
        let c = fresh();
        migrate(&c).unwrap();
        let v: i32 = c
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, USER_VERSION);
    }

    /// An old binary must refuse a database a newer one wrote. It would not
    /// crash on it -- it would write rows missing the columns it cannot see,
    /// and that surfaces weeks later as an unattributable board.
    #[test]
    fn a_newer_database_is_refused() {
        let c = fresh();
        c.pragma_update(None, "user_version", USER_VERSION + 7)
            .unwrap();
        match migrate(&c) {
            Err(MigrateError::FromTheFuture { found, known }) => {
                assert_eq!((found, known), (USER_VERSION + 7, USER_VERSION));
            }
            other => panic!("expected FromTheFuture, got {other:?}"),
        }
    }

    /// The tier-move rule, as a constraint rather than a convention: the same
    /// board name at two armed budgets is two rows, because it is two
    /// measurements.
    #[test]
    fn a_tier_move_is_a_new_board() {
        let c = fresh();
        let ins = |budget: f64| {
            c.execute(
                "INSERT INTO board(name,budget_secs,mode,jobs,threads,env,args,threads_json)
                 VALUES('ipc5-time',?1,'auto',2,'1','{}','[]','\"1\"')",
                rusqlite::params![budget],
            )
        };
        ins(30.0).unwrap();
        ins(60.0).unwrap();
        ins(60.0).unwrap_err(); // the same measurement twice is still one board
        let n: i64 = c
            .query_row("SELECT COUNT(*) FROM board", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2);
    }

    /// The column comment, enforced. `val` must be able to hold three states;
    /// anything outside them is a bug being written to disk.
    #[test]
    fn val_is_a_tristate_and_nothing_else() {
        let c = fresh();
        c.execute_batch(
            "INSERT INTO engine(blake3,ver) VALUES('abc','ff 0.25.0');
             INSERT INTO board(name,budget_secs,mode,jobs,threads,env,args,threads_json)
               VALUES('b',60,'auto',2,'1','{}','[]','\"1\"');
             INSERT INTO variant(ipc,name) VALUES('ipc-2014','v');
             INSERT INTO instance(variant_id,label,label_is_int,sort_key)
               VALUES(1,'1',1,'0011');",
        )
        .unwrap();
        let put = |val: i64| {
            c.execute(
                "INSERT INTO run(board_id,instance_id,engine_id,attempt,state,solved,val)
                 VALUES(1,1,1,?1,'done',0,?2)",
                rusqlite::params![val, val],
            )
        };
        put(0).unwrap();
        put(1).unwrap();
        assert!(put(2).is_err(), "val accepted a fourth state");
    }

    /// A negative zero must survive storage, because on the net-benefit boards
    /// the sign IS the measurement.
    ///
    /// `metric: -0.0` appears in four rows of the committed
    /// `benchmarks/ipc67-netben.jsonl`. A REAL-affinity column stores a
    /// fraction-free double as an integer and hands back `0.0`, so the board
    /// exports bytes the raw does not have -- and it does it silently, on the
    /// one column a net-benefit board is scored by. This test is the reason
    /// those columns carry no declared type.
    #[test]
    fn a_negative_zero_metric_keeps_its_sign() {
        let c = fresh();
        c.execute_batch(
            "INSERT INTO engine(blake3,ver) VALUES('abc','ff 0.25.0');
             INSERT INTO board(name,budget_secs,mode,jobs,threads,env,args,threads_json)
               VALUES('b',60,'auto',2,'1','{}','[]','\"1\"');
             INSERT INTO variant(ipc,name) VALUES('ipc-2008','v');
             INSERT INTO instance(variant_id,label,label_is_int,sort_key)
               VALUES(1,'24',1,'00224');",
        )
        .unwrap();
        c.execute(
            "INSERT INTO run(board_id,instance_id,engine_id,state,solved,metric,makespan)
             VALUES(1,1,1,'done',1,?1,?1)",
            rusqlite::params![-0.0f64],
        )
        .unwrap();
        let (m, ms): (f64, f64) = c
            .query_row("SELECT metric, makespan FROM run", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert!(
            m.is_sign_negative(),
            "metric lost the sign of -0.0: the column has REAL affinity again"
        );
        assert!(ms.is_sign_negative(), "makespan lost the sign of -0.0");
    }
}
