//! Banking a release: the step that writes one line of the record every
//! "vs previous" column in this project is later computed from.
//!
//! Ported from `scripts/standings-snapshot.py`. That script is 103 lines and
//! three of its fields are load-bearing in ways that are invisible in the
//! output, which is why it has a docstring longer than most of its functions:
//!
//! * **`measured_on` is the BOX, and it is not decoration.** Coverage at a
//!   fixed time budget is a property of the HARDWARE as much as of the engine,
//!   so a snapshot is only ever compared against a predecessor from the same
//!   box. That law lives in [`crate::history`], on the types; this module's job
//!   is to make sure the box is on the record in the first place.
//!
//! * **`measured_at` is not `released`.** An old version re-swept on new
//!   hardware to backfill the trend is honestly a TODAY measurement of a July
//!   release, and must never be presented as a July number. The committed
//!   history still carries 0.19.0 measured 2026-08-02, the day after 0.20.0 --
//!   which is exactly the shape that made "pick the most recent measurement"
//!   compare a release to its grandparent.
//!
//! * **`--from-dir` is a different source of truth.** It banks a sweep that was
//!   never promoted: a backfill of an old tag, read out of a staging directory,
//!   which must land in the history without disturbing a single live board. In
//!   the Python that distinction is a variable called `src` and an `if src:`
//!   inside the loop, so both readers are in scope on every iteration and the
//!   promoted path is one missing `continue` away. Here it is [`Source`], and
//!   the backfill arm has no `benchmarks/` path to reach for at all.
//!
//! # The naming exception, as data
//!
//! `standings-snapshot.py` special-cases the `ipc67-results` / `ipc67-default`
//! split TWICE -- once as `"ipc67-results" if fname == "ipc67-default.jsonl"`
//! in the backfill arm, and once as a two-entry `MD_FOR` dict in the promoted
//! arm. Both are the same fact: exactly one board in the system has an id that
//! is not its raw's stem. Here that fact is the manifest's `(id, raw, md)`
//! triple and nothing else, which this module's
//! `the_naming_exception_is_a_manifest_triple_not_a_special_case` test asserts
//! over the real, committed manifest.
//!
//! # Bytes
//!
//! The history file is rewritten in place on every release, so a rewrite whose
//! diff is not empty when nothing changed is a rewrite nobody reads.
//! [`crate::history::History::to_json`] already reproduces `json.dump(...,
//! indent=1)` plus the trailing newline plus `ensure_ascii`; this module owns
//! only the replace-then-append-then-sort that decides what goes into it, and
//! the reads-before-writes split that keeps a failed bank from leaving a
//! half-written record.

use std::path::{Path, PathBuf};

use crate::history::{BoxId, History, HistoryError, MeasuredAt, Snapshot, Tracks};
use crate::manifest::Manifest;
use crate::parse_rows;
use crate::promote::{write_atomic, BOARDS_DIR};
use crate::referee::Referee;

/// The box a snapshot is attributed to when nothing says otherwise.
pub const DEFAULT_BOX: &str = "m5-air";

/// The environment variable `--box` falls back to.
///
/// Named here rather than read here: this crate is pure, so the caller reads
/// the environment and passes the value in. That is not ceremony -- it is what
/// lets a test prove the three-way precedence (`--box`, then the variable, then
/// [`DEFAULT_BOX`]) without touching the process it runs in.
pub const BOX_ENV: &str = "FERROPLAN_BOX";

/// `standings.py`'s `HISTORY`, relative to the repo root.
pub const HISTORY_FILE: &str = "standings-history.json";

/// Banking failed. Every message a person will actually see is the Python's,
/// character for character, including its em dashes: an operator who has hit
/// one of these before should recognise it, and a message that changed in the
/// port is one more thing to have to prove is the same message.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("--version is required (e.g. --version 0.20.0)")]
    MissingVersion,

    #[error(
        "--measured-at is required (YYYY-MM-DD) \u{2014} the date the BOARDS were swept, \
         which is not necessarily the release date"
    )]
    MissingMeasuredAt,

    /// Python's `arg()` is `sys.argv[sys.argv.index(name) + 1]`, so a flag in
    /// final position raises a bare `IndexError` traceback.
    #[error("{flag} is the last argument and has no value")]
    DanglingFlag { flag: String },

    #[error("no promoted boards found \u{2014} promote before snapshotting")]
    NoBoards,

    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// A board raw that could not be read as rows. Python's `json.loads` raises
    /// here with no file in the message; this names the file and the line.
    #[error("{0}")]
    Raw(String),

    #[error(transparent)]
    History(#[from] HistoryError),
}

// ---------------------------------------------------------------------------
// Arguments
// ---------------------------------------------------------------------------

/// Where a snapshot's numbers come from.
///
/// Two variants, no shared state, and the backfill carries its own directory --
/// so there is no expression anywhere in this module that reads a promoted
/// board while a `--from-dir` is in play. The Python's `if src:` inside the
/// loop had both readers in scope on every iteration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// The promoted boards in `benchmarks/`. Both the raw AND its `.md` must
    /// exist: `ipc67.py` writes the `.md` at sweep END, so a raw without one is
    /// a sweep still in flight and must not be banked as a finished board.
    Promoted,
    /// `--from-dir DIR`: a sweep that was never promoted. A BACKFILL of an old
    /// tag, banked without disturbing the live boards.
    Backfill { dir: PathBuf },
}

/// A parsed `standings-snapshot` invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub version: String,
    /// The day the BOARDS were swept. Required, and separate from `released`
    /// for the backfill reason in the module header.
    pub measured_at: String,
    pub box_: String,
    pub released: String,
    pub note: String,
    pub source: Source,
}

/// Parse the command line, with `env_box` supplying [`BOX_ENV`].
///
/// `argv` excludes the program name. That makes no difference to the port:
/// Python's `arg()` searches `sys.argv` by VALUE, so `argv[0]` only participates
/// if the script is itself invoked under a name that looks like a flag.
///
/// The lookup is faithfully positional -- first occurrence of the literal
/// token, then whatever follows it. So `--note --version 0.26.0` really does
/// make `--note` take `--version` as its value, and it is left that way because
/// a "smarter" parser would silently accept a command line the Python rejects
/// (or vice versa), and the release step is not the place to discover that the
/// two implementations disagree about what was asked for.
pub fn parse_args(argv: &[String], env_box: Option<&str>) -> Result<Args, SnapshotError> {
    let get = |flag: &str| -> Result<Option<String>, SnapshotError> {
        match argv.iter().position(|a| a == flag) {
            None => Ok(None),
            Some(i) => {
                argv.get(i + 1)
                    .cloned()
                    .map(Some)
                    .ok_or_else(|| SnapshotError::DanglingFlag {
                        flag: flag.to_string(),
                    })
            }
        }
    };
    let version = get("--version")?.ok_or(SnapshotError::MissingVersion)?;
    // Python tests `if not version`, so an EMPTY --version is missing too.
    if version.is_empty() {
        return Err(SnapshotError::MissingVersion);
    }
    let measured_at = get("--measured-at")?.ok_or(SnapshotError::MissingMeasuredAt)?;
    if measured_at.is_empty() {
        return Err(SnapshotError::MissingMeasuredAt);
    }
    let box_ = get("--box")?
        .or_else(|| env_box.map(str::to_string))
        .unwrap_or_else(|| DEFAULT_BOX.to_string());
    let released = get("--released")?.unwrap_or_else(|| measured_at.clone());
    let note = get("--note")?.unwrap_or_default();
    // Python is `src = arg("--from-dir")` and then a bare `if src:` inside the
    // loop -- a FALSY test, so `--from-dir ""` is not a backfill at all and the
    // promoted arm runs. Reproduced rather than tightened, because the two
    // implementations disagreeing about what was asked for is exactly what the
    // positional lookup above refuses to risk. It also happens to be the only
    // sane reading: `Some("")` here would build a backfill rooted at the EMPTY
    // path, and `PathBuf::from("").join("x.jsonl")` is `x.jsonl` -- relative to
    // whatever directory the process was started in, silently banking whatever
    // happens to be there under the release's name.
    let source = match get("--from-dir")?.filter(|d| !d.is_empty()) {
        Some(d) => Source::Backfill {
            dir: PathBuf::from(d),
        },
        None => Source::Promoted,
    };
    Ok(Args {
        version,
        measured_at,
        box_,
        released,
        note,
        source,
    })
}

// ---------------------------------------------------------------------------
// Reading the boards
// ---------------------------------------------------------------------------

/// Every board that has numbers, in the manifest's FILE order.
///
/// That order is the registry's: `crucible/tools/gen-manifest.py` wrote the
/// `[[board]]` tables in `SWEEPS`'s insertion order, and a Python dict iterates
/// in insertion order, so the `tracks` object this produces lists its boards in
/// the same sequence the Python's does. It is not an alphabetical order and
/// must not be turned into one -- it is written straight back out to a file
/// that is diffed release over release.
///
/// A board with rows but a total of zero is skipped, exactly as Python's
/// `if n:` skips it: an empty board measured nothing, and banking `0/0` would
/// put a track in the record that the delta column then has to special-case
/// back out again.
pub fn tracks(
    root: &Path,
    manifest: &Manifest,
    referee: &Referee,
    source: &Source,
) -> Result<Tracks, SnapshotError> {
    let mut out: Tracks = Vec::new();
    for board in &manifest.boards {
        let raw = match source {
            // The promoted pair, both halves named by the manifest triple.
            Source::Promoted => {
                let raw = root.join(BOARDS_DIR).join(&board.raw);
                let md = root.join(BOARDS_DIR).join(&board.md);
                if !(raw.exists() && md.exists()) {
                    continue;
                }
                raw
            }
            // A staging directory names its files by BOARD ID, which is where
            // the Python's `"ipc67-results" if fname == "ipc67-default.jsonl"`
            // came from. It is the id, so it is read as the id.
            Source::Backfill { dir } => {
                let p = dir.join(format!("{}.jsonl", board.id));
                if !p.exists() {
                    continue;
                }
                p
            }
        };
        let shown = raw.display().to_string();
        let text = std::fs::read_to_string(&raw).map_err(|source| SnapshotError::Io {
            path: shown.clone(),
            source,
        })?;
        let rows = parse_rows(&text, &shown).map_err(SnapshotError::Raw)?;
        let cov = referee.coverage(&rows, board.budget_secs);
        if cov.total != 0 {
            out.push((board.label.clone(), (cov.solved, cov.total)));
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Banking
// ---------------------------------------------------------------------------

/// A snapshot, the history it belongs in, and the exact bytes of the file --
/// all computed, nothing written yet.
///
/// The same split [`crate::promote`] uses, for the same reason: every read and
/// every decision happens before the first byte is written, so a failure cannot
/// leave the record half-updated.
#[derive(Debug, Clone)]
pub struct Banked {
    pub path: PathBuf,
    pub history: History,
    pub snapshot: Snapshot,
    /// `json.dump(doc, f, indent=1)` plus the trailing newline, `ensure_ascii`.
    pub json: String,
    /// The two lines the Python prints, in order.
    pub report: Vec<String>,
}

impl Banked {
    /// Write the history. Nothing is read here.
    pub fn write(&self) -> Result<(), SnapshotError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| SnapshotError::Io {
                path: parent.display().to_string(),
                source,
            })?;
        }
        write_atomic(&self.path, self.json.as_bytes()).map_err(|source| SnapshotError::Io {
            path: self.path.display().to_string(),
            source,
        })
    }
}

/// Read the boards and the existing history, and compute the new file.
///
/// The upsert is `standings-snapshot.py`'s three lines, and it lives on
/// [`History`]: replace any snapshot with the same `(version, box)` PAIR,
/// append, then sort by `(measured_at, version)`. The identity is the pair
/// because the same tag measured on two boxes is two records, and collapsing
/// them would overwrite one machine's record with another's.
pub fn bank(
    root: &Path,
    manifest: &Manifest,
    referee: &Referee,
    args: &Args,
) -> Result<Banked, SnapshotError> {
    let tracks = tracks(root, manifest, referee, &args.source)?;
    if tracks.is_empty() {
        return Err(SnapshotError::NoBoards);
    }
    let path = root.join(BOARDS_DIR).join(HISTORY_FILE);
    let mut history = History::try_load(&path)?;

    let snapshot = Snapshot {
        version: args.version.clone(),
        released: args.released.clone(),
        measured_on: BoxId::new(args.box_.clone()),
        measured_at: MeasuredAt::new(args.measured_at.clone()),
        note: args.note.clone(),
        tracks,
    };
    history.upsert(snapshot.clone());

    // Plain integers, no thousands separators: this is the operator's receipt,
    // not a published table, and the Python prints them bare.
    let tot_s: usize = snapshot.tracks.iter().map(|(_, (s, _))| *s).sum();
    let tot_n: usize = snapshot.tracks.iter().map(|(_, (_, n))| *n).sum();
    let report = vec![
        format!(
            "snapshotted {} on {} ({}): {} tracks, {tot_s}/{tot_n}",
            snapshot.version,
            snapshot.measured_on,
            snapshot.measured_at,
            snapshot.tracks.len(),
        ),
        format!(
            "wrote {} ({} snapshots)",
            path.display(),
            history.snapshots().len()
        ),
    ];
    let json = history.to_json();
    Ok(Banked {
        path,
        history,
        snapshot,
        json,
        report,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{NAMING_EXCEPTION_ID, NAMING_EXCEPTION_RAW};
    use crate::referee::ValUnavailable;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../..");

    fn real_manifest() -> Manifest {
        Manifest::load(Path::new(&format!("{REPO}/benchmarks/manifest.toml")))
            .unwrap_or_else(|e| panic!("{e}"))
    }

    fn tmproot(tag: &str) -> PathBuf {
        static N: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "crucible-snapshot-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    /// A manifest holding the one naming exception plus an ordinary board, so a
    /// test can drive both spellings through the same code path.
    fn two_board_manifest() -> Manifest {
        Manifest::parse(
            "schema = 1\n\
             [corpus]\nroot = \".c\"\ndomain_shared = \"domain.pddl\"\n\
             domain_per_instance = \"domains/domain-{first}.pddl\"\n\
             [defaults]\ntimeout_secs = 60\njobs = 2\nthreads = 1\nmode = \"auto\"\nmem_gb = 6.0\n\
             [[board]]\nid = \"ipc67-results\"\nraw = \"ipc67-default.jsonl\"\n\
             md = \"ipc67-results.md\"\nlabel = \"seq-sat\"\ncompetition = \"ipc67\"\n\
             budget_secs = 60\ntrack = \"seq-sat\"\n\
             [[board]]\nid = \"plain\"\nraw = \"plain.jsonl\"\nmd = \"plain.md\"\n\
             label = \"plain label\"\ncompetition = \"x\"\nbudget_secs = 60\ntrack = \"t\"\n",
            "test-manifest.toml",
        )
        .unwrap_or_else(|e| panic!("{e}"))
    }

    fn rows(n: usize, solved: usize) -> String {
        (0..n)
            .map(|i| {
                format!(
                    "{{\"variant\": \"v\", \"instance\": {i}, \"solved\": {}, \"budget\": 60}}\n",
                    if i < solved { "true" } else { "false" }
                )
            })
            .collect()
    }

    fn referee() -> Referee {
        Referee::new(ValUnavailable::default())
    }

    /// THE assertion this port is asked for: the `ipc67-results` /
    /// `ipc67-default` split is not a special case here, it is the manifest's
    /// `(id, raw, md)` triple. Both of the Python's hard-coded spellings fall
    /// out of it -- `MD_FOR`'s two entries are `md == "{id}.md"`, and the
    /// backfill arm's `"ipc67-results" if fname == "ipc67-default.jsonl"` is
    /// just the id -- and there is exactly ONE board in the system for which
    /// the raw is not `"{id}.jsonl"`.
    #[test]
    fn the_naming_exception_is_a_manifest_triple_not_a_special_case() {
        let m = real_manifest();
        let mut exceptions = Vec::new();
        for b in &m.boards {
            // `MD_FOR` had two entries and only one of them was an exception;
            // the other, `ipc67-temporal.jsonl -> ipc67-temporal.md`, is this
            // rule. Every board obeys it.
            assert_eq!(b.md, format!("{}.md", b.id), "board {}", b.id);
            if b.raw != format!("{}.jsonl", b.id) {
                exceptions.push((b.id.as_str(), b.raw.as_str()));
            }
        }
        assert_eq!(
            exceptions,
            vec![(NAMING_EXCEPTION_ID, NAMING_EXCEPTION_RAW)],
            "exactly one board's id is not its raw's stem"
        );
    }

    /// The same fact, exercised rather than inspected: the exception board is
    /// found under its RAW name when promoted and under its ID when backfilled,
    /// and both land on the same label with the same numbers.
    #[test]
    fn the_exception_board_is_found_under_both_of_its_names() {
        let root = tmproot("exception");
        let m = two_board_manifest();
        let b = root.join("benchmarks");
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(b.join("ipc67-default.jsonl"), rows(10, 7)).unwrap();
        std::fs::write(b.join("ipc67-results.md"), "# board\n").unwrap();
        let promoted = tracks(&root, &m, &referee(), &Source::Promoted).unwrap();
        assert_eq!(promoted, vec![("seq-sat".to_string(), (7, 10))]);

        // The backfill stage names it by id.
        let stage = root.join("stage");
        std::fs::create_dir_all(&stage).unwrap();
        std::fs::write(stage.join("ipc67-results.jsonl"), rows(10, 7)).unwrap();
        let back = tracks(
            &root,
            &m,
            &referee(),
            &Source::Backfill { dir: stage.clone() },
        )
        .unwrap();
        assert_eq!(back, promoted);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A backfill must not disturb the live boards, and here it cannot even
    /// SEE them: the promoted raw says 9 solved and the staged one says 2, and
    /// the banked number is the staged one.
    #[test]
    fn a_backfill_never_reads_the_promoted_boards() {
        let root = tmproot("backfill");
        let m = two_board_manifest();
        let b = root.join("benchmarks");
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(b.join("plain.jsonl"), rows(10, 9)).unwrap();
        std::fs::write(b.join("plain.md"), "# plain\n").unwrap();
        let stage = root.join("old-tag");
        std::fs::create_dir_all(&stage).unwrap();
        std::fs::write(stage.join("plain.jsonl"), rows(10, 2)).unwrap();

        let a = parse_args(
            &args(&[
                "--version",
                "0.19.0",
                "--measured-at",
                "2026-08-02",
                "--from-dir",
                stage.to_str().unwrap(),
            ]),
            None,
        )
        .unwrap();
        let banked = bank(&root, &m, &referee(), &a).unwrap();
        assert_eq!(banked.snapshot.track("plain label"), Some((2, 10)));
        // The backfilled tag is banked with TODAY's measurement date and the
        // old release's version: a July release measured in August.
        assert_eq!(banked.snapshot.measured_at.as_str(), "2026-08-02");
        assert_eq!(banked.snapshot.released, "2026-08-02");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// `ipc67.py` writes the `.md` at sweep END, so a raw without one is a
    /// sweep still in flight. Banking it would put a partial board in the
    /// record under a finished release's name.
    #[test]
    fn a_promoted_raw_without_its_md_is_a_sweep_in_flight() {
        let root = tmproot("inflight");
        let m = two_board_manifest();
        let b = root.join("benchmarks");
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(b.join("plain.jsonl"), rows(10, 5)).unwrap();
        assert!(tracks(&root, &m, &referee(), &Source::Promoted)
            .unwrap()
            .is_empty());
        std::fs::write(b.join("plain.md"), "# plain\n").unwrap();
        assert_eq!(
            tracks(&root, &m, &referee(), &Source::Promoted).unwrap(),
            vec![("plain label".to_string(), (5, 10))]
        );
        // And with nothing to bank, the refusal is the Python's own sentence.
        let root2 = tmproot("empty");
        let a = parse_args(
            &args(&["--version", "0.26.0", "--measured-at", "2026-09-01"]),
            None,
        )
        .unwrap();
        let e = bank(&root2, &m, &referee(), &a).unwrap_err();
        assert_eq!(
            e.to_string(),
            "no promoted boards found \u{2014} promote before snapshotting"
        );
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&root2);
    }

    /// `--version` and `--measured-at` are required, and their refusals are the
    /// Python's words -- em dash included.
    #[test]
    fn version_and_measured_at_are_required() {
        let e = parse_args(&args(&["--measured-at", "2026-09-01"]), None).unwrap_err();
        assert_eq!(
            e.to_string(),
            "--version is required (e.g. --version 0.20.0)"
        );
        let e = parse_args(&args(&["--version", "0.26.0"]), None).unwrap_err();
        assert_eq!(
            e.to_string(),
            "--measured-at is required (YYYY-MM-DD) \u{2014} the date the BOARDS were swept, \
             which is not necessarily the release date"
        );
        // A flag in final position: Python's `arg()` raises IndexError here.
        assert!(matches!(
            parse_args(&args(&["--version"]), None),
            Err(SnapshotError::DanglingFlag { .. })
        ));
    }

    /// The defaults, all three of them: `released` falls back to `measured_at`
    /// (they differ only for a backfill), the note to empty, and the box down
    /// the chain `--box` -> `$FERROPLAN_BOX` -> `m5-air`.
    #[test]
    fn the_defaults_are_the_pythons() {
        let base = args(&["--version", "0.26.0", "--measured-at", "2026-09-01"]);
        let a = parse_args(&base, None).unwrap();
        assert_eq!(a.released, "2026-09-01");
        assert_eq!(a.note, "");
        assert_eq!(a.box_, DEFAULT_BOX);
        assert_eq!(a.source, Source::Promoted);

        // Python's `if src:` is FALSY, so an empty `--from-dir` -- the shape a
        // shell hands over when its stage variable is unset -- is not a
        // backfill. A `Some("")` backfill would read `{id}.jsonl` relative to
        // the process's working directory.
        let mut empty_dir = base.clone();
        empty_dir.extend(args(&["--from-dir", ""]));
        assert_eq!(
            parse_args(&empty_dir, None).unwrap().source,
            Source::Promoted
        );

        assert_eq!(
            parse_args(&base, Some("cloud-c7")).unwrap().box_,
            "cloud-c7"
        );

        let mut with_box = base.clone();
        with_box.extend(args(&[
            "--box",
            "m5-air",
            "--released",
            "2026-07-01",
            "--note",
            "n",
        ]));
        let a = parse_args(&with_box, Some("cloud-c7")).unwrap();
        assert_eq!(a.box_, "m5-air", "--box wins over the environment");
        assert_eq!(a.released, "2026-07-01");
        assert_eq!(a.note, "n");
    }

    /// The committed history, read and written back through this module's
    /// writer, must be the same BYTES. This file is rewritten on every release;
    /// a reformatting rewrite hides the one line that actually changed, and
    /// neither of `serde_json`'s defaults (`indent`, `ensure_ascii`) matches
    /// Python's.
    #[test]
    fn the_committed_history_round_trips_byte_for_byte() {
        let p = format!("{REPO}/benchmarks/{HISTORY_FILE}");
        let src = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"));
        let h = History::try_load(Path::new(&p)).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(h.to_json(), src);
    }

    /// Re-banking a snapshot that is already in the record must not move a
    /// byte: replace-then-append-then-sort has to be idempotent, or every
    /// re-run of the release step produces a diff that says nothing.
    #[test]
    fn re_banking_an_existing_snapshot_changes_no_bytes() {
        let p = format!("{REPO}/benchmarks/{HISTORY_FILE}");
        let src = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"));
        let mut h = History::try_load(Path::new(&p)).unwrap_or_else(|e| panic!("{e}"));
        let again = h
            .snapshots()
            .last()
            .expect("the history is not empty")
            .clone();
        h.upsert(again);
        assert_eq!(h.to_json(), src);
    }

    /// The whole step, end to end, on a scratch root: every read happens in
    /// `bank`, so the sources can be deleted before `write` and the file still
    /// lands -- and it lands atomically, leaving no temp sibling behind.
    #[test]
    fn bank_reads_everything_before_write_touches_anything() {
        let root = tmproot("endtoend");
        let m = two_board_manifest();
        let b = root.join("benchmarks");
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(b.join("plain.jsonl"), rows(4, 3)).unwrap();
        std::fs::write(b.join("plain.md"), "# plain\n").unwrap();
        let a = parse_args(
            &args(&[
                "--version",
                "0.26.0",
                "--measured-at",
                "2026-09-01",
                "--note",
                "an \u{2014} em dash",
            ]),
            None,
        )
        .unwrap();
        let banked = bank(&root, &m, &referee(), &a).unwrap();
        assert_eq!(
            banked.report,
            vec![
                "snapshotted 0.26.0 on m5-air (2026-09-01): 1 tracks, 3/4".to_string(),
                format!("wrote {} (1 snapshots)", banked.path.display()),
            ]
        );
        std::fs::remove_file(b.join("plain.jsonl")).unwrap();
        banked.write().unwrap();
        let written = std::fs::read_to_string(&banked.path).unwrap();
        assert_eq!(written, banked.json);
        // `ensure_ascii`, which serde_json does not do.
        assert!(written.contains("an \\u2014 em dash"), "{written}");
        assert!(written.ends_with("}\n"), "trailing newline");
        assert!(!b.join(format!("{HISTORY_FILE}.crucible-tmp")).exists());

        // And a second bank of the same (version, box) REPLACES rather than
        // appends: the pair is the identity.
        std::fs::write(b.join("plain.jsonl"), rows(4, 4)).unwrap();
        let again = bank(&root, &m, &referee(), &a).unwrap();
        assert_eq!(again.history.snapshots().len(), 1);
        assert_eq!(again.snapshot.track("plain label"), Some((4, 4)));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The board order in a banked `tracks` object is the manifest's FILE
    /// order, which is `SWEEPS`'s insertion order -- not alphabetical. It is
    /// written straight back out to a file that is diffed release over release,
    /// so a map that sorted its keys would rewrite every line.
    #[test]
    fn track_order_is_the_registrys_not_alphabetical() {
        let root = tmproot("order");
        let m = two_board_manifest();
        let b = root.join("benchmarks");
        std::fs::create_dir_all(&b).unwrap();
        for (raw, md) in [
            ("ipc67-default.jsonl", "ipc67-results.md"),
            ("plain.jsonl", "plain.md"),
        ] {
            std::fs::write(b.join(raw), rows(2, 1)).unwrap();
            std::fs::write(b.join(md), "#\n").unwrap();
        }
        let t = tracks(&root, &m, &referee(), &Source::Promoted).unwrap();
        // "seq-sat" sorts AFTER "plain label"; the manifest lists it first.
        assert_eq!(
            t.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>(),
            vec!["seq-sat", "plain label"]
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
