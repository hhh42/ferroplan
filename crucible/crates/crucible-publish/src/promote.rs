//! The promotion gate: the one step that moves a cycle's staged boards into
//! `benchmarks/`, and the two-headline record the cut is written from.
//!
//! Ported from `benchmarks/promote-air25.sh`, which is 97 lines of shell that
//! four separate incidents are visible in. Each one is a gate here:
//!
//! * **A half-promoted set mixes cycles under one name.** The shell's `.done`
//!   sweep exists because a board copied out of a stage the driver had not
//!   finished writing keeps its old name and its new rows, and nothing
//!   downstream can tell. So: every marker is checked before anything is
//!   copied, and every missing one is named in a single refusal -- a gate that
//!   stops at the first hole makes you run it N times, once per hole, and the
//!   shell already knew that.
//!
//! * **The stamp gate reads ONE LINE.** `json.loads(next(f)).get("budget")` --
//!   line one of the raw and nothing else. That is a real hole, and it is
//!   precisely aimed at the situation the stamp mechanism was invented for: a
//!   stitched or resumed raw can legitimately span a tier change (0.23's
//!   temporal move, then 0.25's `ipc5-time` / `ipc5-metric-time` move), and a
//!   raw whose first row says 60 and whose hundredth says 30 sails straight
//!   through. Every row is read here, and a raw carrying two stamps is refused
//!   by name -- see [`Gate::NonUniformStamps`].
//!
//! * **Flip-then-prove.** The shell TEXT-EDITS `standings.py`'s source to flip
//!   two budgets from 30 to 60, and only then runs the stamp gate "to prove the
//!   flip against the raws' own stamps". A proof that runs after the edit it is
//!   proving passes just as happily when the edit landed on the wrong board:
//!   flip `ipc5-prop` instead of `ipc5-time` and the gate reports that
//!   everything matches, because the two `str.replace` calls that did not match
//!   anything are silent. The direction is inverted here. The evidence is read
//!   first; the uniform stamp `B` is compared against the manifest's
//!   `budget_secs`; a difference is a **tier move discovered from evidence**,
//!   which requires an explicit acceptance ([`TierMovePolicy`]) before a
//!   [`Change`] to the registry is even described. Nothing is edited to make a
//!   check pass.
//!
//! * **Version drift across a merged board.** `PER-INSTANCE-RETRY.md`'s first
//!   care point: "a stitched board must never mix rows from two different `ff`
//!   builds". It is enforced nowhere -- not in the shell, not in
//!   `standings.py`, not in the resume path's own gate at the point a board is
//!   published. It is enforced here, at the last moment a board is still a
//!   candidate rather than a published number.
//!
//! Two staging directories are deliberate, not a leftover: the standing 22 keep
//! their like-for-like identity in one stage while the entries land in another,
//! because the cut record carries TWO headlines. [`TwoHeadlines`] is the type
//! that makes the second one impossible to forget -- see its own docs.
//!
//! # Reads first, writes last
//!
//! [`plan`] performs every read and every gate and returns a [`Promotion`] that
//! already holds the BYTES of every file it will write. [`Promotion::apply`]
//! then only writes. A gate that fires halfway through a copy loop leaves a
//! set half-promoted, which is the same failure the `.done` markers defend
//! against one level up, and it would be absurd to defend it there and
//! reintroduce it here.
//!
//! Each individual write goes to a temp sibling and is `rename`d into place, so
//! a destination is either the old file or the new one and never a torn
//! prefix. That matters more than it looks: a truncated `.jsonl` is not a
//! read error, it is a board that is quietly SHORT SOME ROWS -- `parse_rows`
//! skips a truncated tail line by design, so the next standings run would
//! publish a smaller denominator without a word.

use std::path::{Path, PathBuf};

use crate::fmt;
use crate::history::{ComparablePredecessor, NEW_CELL};
use crate::manifest::Manifest;
use crate::parse_rows;

/// Where a promoted board lands, relative to the repo root.
///
/// The asymmetry is the manifest's and is reproduced rather than corrected: a
/// `[[board]]`'s `raw` and `md` are bare filenames relative to `benchmarks/`
/// (`standings.py`'s `B`), while a `[[set]]`'s `stage` is root-relative and
/// carries its own `benchmarks/` prefix.
pub const BOARDS_DIR: &str = "benchmarks";

/// The shell's refusal when a `.done` marker is missing, verbatim.
pub const PARTIAL_SWEEP_REFUSAL: &str = "refusing to promote a partial sweep";

/// The shell's refusal when a raw's budget stamp does not match the registry,
/// verbatim -- including the line break, which is two `print()` calls there.
///
/// "SWEEPS" is `standings.py`'s registry dict, which `benchmarks/manifest.toml`
/// has since replaced; the wording is kept as it was published so that a person
/// who has seen this refusal before recognises it, and because a refusal that
/// changes its words between implementations is one more thing to have to
/// prove is the same refusal.
pub const STAMP_REFUSAL: &str = "refusing to promote: fix the SWEEPS budget or re-sweep the\n\
                                 mismatched board at the registry budget.";

/// The generic refusal, for a hole the shell had no sentence for.
pub const REFUSAL: &str = "refusing to promote";

/// The suffix a pending write carries until its `rename`.
const TMP_SUFFIX: &str = ".crucible-tmp";

// ---------------------------------------------------------------------------
// Gates
// ---------------------------------------------------------------------------

/// One reason this promotion is refused.
///
/// Every variant names the board, because the operator's next action is
/// board-shaped: re-sweep this one, accept this tier move, fix this stage.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Gate {
    /// The completeness gate. Zero-byte markers: EXISTENCE only, content never
    /// read, exactly as the shell's `[ -f ... ]`.
    ///
    /// The shell hard-coded the parenthesised word as `cut` or `entry`, one per
    /// array; here it is the `[[set]]`'s own name, so a cycle with a third
    /// staging set says which one without anybody editing a string.
    #[error("NOT DONE ({set}): {board}")]
    NotDone { set: String, board: String },

    #[error("no [[set]] named {name:?} in the manifest")]
    UnknownSet { name: String },

    #[error("set {set:?} names board {board:?}, which the manifest does not define")]
    UnknownBoard { set: String, board: String },

    /// The same id staged twice. Two stages holding one board is two cycles
    /// under one name -- the thing the `.done` sweep exists to prevent, arriving
    /// through the front door instead.
    #[error("{board}: staged by both set {first:?} and set {second:?}")]
    DuplicateBoard {
        board: String,
        first: String,
        second: String,
    },

    #[error("{board}: missing {path}")]
    Missing { board: String, path: String },

    /// The presence gate's second half. Python's `next(f)` on an empty raw
    /// raises a bare `StopIteration` traceback with no board name in it, which
    /// is how an empty raw used to be diagnosed.
    #[error("{board}: {path} has no rows")]
    EmptyRaw { board: String, path: String },

    #[error("{board}: {msg}")]
    Unreadable { board: String, msg: String },

    /// No row carries a `budget` stamp at all. A pre-0.23 raw: the tier
    /// mechanism cannot see it, so there is no evidence to promote against.
    #[error(
        "{board}: no row carries a budget stamp (a pre-0.23 raw); \
             re-sweep it, or promote it from a cycle that stamps"
    )]
    Unstamped { board: String },

    /// A stamp that is not a positive, finite number of seconds. `referee`'s
    /// `budget_for` treats a stamp of `0` as ABSENT (Python's falsy `or`), so a
    /// zero would be scored at the registry budget and never seen again.
    #[error("{board}: budget stamp {stamp} is not a positive number of seconds")]
    BadStamp { board: String, stamp: String },

    /// THE hole the shell's one-line read leaves open.
    #[error(
        "{board} (set {set}): rows carry {count} different budget stamps ({stamps}) -- \
             a raw stitched across a tier change; the shell read only line one"
    )]
    NonUniformStamps {
        set: String,
        board: String,
        count: usize,
        stamps: String,
    },

    /// `PER-INSTANCE-RETRY.md`'s care point, enforced.
    #[error(
        "{board} (set {set}): rows carry {count} different `ver` values ({versions}) -- \
             a board stitched across two ff builds"
    )]
    VersionDrift {
        set: String,
        board: String,
        count: usize,
        versions: String,
    },

    /// The inverted registry mutation. The evidence says one budget, the
    /// manifest says another; that is a tier move, and a tier move is an
    /// operator decision, not a side effect of promoting.
    #[error(
        "{board}: raw stamped {stamp}s, registry says {registry}s -- a tier move \
             discovered from the evidence; re-run with --accept-tier-moves (or \
             --accept-tier-move {board}) to move the registry to the raws"
    )]
    TierMoveNotAccepted {
        board: String,
        stamp: String,
        registry: String,
    },
}

impl Gate {
    /// Is this one of the failures the shell prefixed `STAMP MISMATCH`?
    fn is_stamp(&self) -> bool {
        matches!(
            self,
            Gate::Unstamped { .. }
                | Gate::BadStamp { .. }
                | Gate::NonUniformStamps { .. }
                | Gate::TierMoveNotAccepted { .. }
        )
    }

    /// The line as it appears in the report, with the shell's indentation.
    fn report_line(&self) -> String {
        match self {
            // The shell prints this one flush left, before its refusal.
            Gate::NotDone { .. } => self.to_string(),
            g if g.is_stamp() => format!("  STAMP MISMATCH {g}"),
            g => format!("  {g}"),
        }
    }
}

/// Every reason at once. There is no way to get one failure out of this crate:
/// the report is the error type, so a caller that prints `{e}` prints all of
/// them and the refusal.
#[derive(Debug, Clone, PartialEq)]
pub struct GateReport {
    failures: Vec<Gate>,
}

impl GateReport {
    fn new(failures: Vec<Gate>) -> Self {
        Self { failures }
    }

    pub fn failures(&self) -> &[Gate] {
        &self.failures
    }

    /// The sentence this report ends with, chosen the way the shell chose:
    /// a partial sweep has its own refusal, a stamp problem has its own, and
    /// anything else falls back to the bare one.
    pub fn refusal(&self) -> &'static str {
        if self
            .failures
            .iter()
            .any(|g| matches!(g, Gate::NotDone { .. }))
        {
            PARTIAL_SWEEP_REFUSAL
        } else if self.failures.iter().any(Gate::is_stamp) {
            STAMP_REFUSAL
        } else {
            REFUSAL
        }
    }
}

impl std::fmt::Display for GateReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for g in &self.failures {
            writeln!(f, "{}", g.report_line())?;
        }
        f.write_str(self.refusal())
    }
}

impl std::error::Error for GateReport {}

/// Writing failed. Reading never gets here: a read problem is a [`Gate`], so it
/// is reported with all the others instead of aborting the run.
#[derive(Debug, thiserror::Error)]
pub enum PromoteError {
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

// ---------------------------------------------------------------------------
// Tier moves
// ---------------------------------------------------------------------------

/// A budget change the EVIDENCE asked for: the raws are uniformly stamped `to`
/// while the manifest still scores the board at `from`.
#[derive(Debug, Clone, PartialEq)]
pub struct TierMove {
    pub board: String,
    pub from: f64,
    pub to: f64,
}

/// Which tier moves this run is allowed to make.
///
/// Default is [`TierMovePolicy::Refuse`], and that is the whole inversion: a
/// tier move never happens because a promotion happened. Someone has to have
/// looked at the raws and said so.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TierMovePolicy {
    #[default]
    Refuse,
    /// `--accept-tier-moves`.
    AcceptAll,
    /// `--accept-tier-move <board>`, repeatable. The per-board form is the one
    /// to reach for on a cycle where only some boards moved: it cannot silently
    /// bless a SECOND move that nobody noticed, which is the failure mode
    /// flip-then-prove had.
    Accept(Vec<String>),
}

impl TierMovePolicy {
    pub fn accepts(&self, board: &str) -> bool {
        match self {
            TierMovePolicy::Refuse => false,
            TierMovePolicy::AcceptAll => true,
            TierMovePolicy::Accept(ids) => ids.iter().any(|b| b == board),
        }
    }
}

/// A registry edit this promotion has decided on but deliberately does NOT
/// perform.
///
/// The manifest is TOML with load-bearing comments -- `timeout_secs = 60  #
/// DIFFERS from budget_secs: a tier move in flight` is the specification of the
/// state this change ends. Rewriting the file through a serializer drops every
/// one of those comments, and a format-preserving editor is a dependency this
/// crate does not carry and would not be allowed to add for one line of output.
///
/// The deeper reason is the one this module exists for. An automatic edit is
/// flip-then-prove wearing a different hat: the check and the mutation must not
/// be in the same hand. So the change is DESCRIBED, the operator applies it,
/// and the next run's stamp gate -- which now has nothing to prove for anybody
/// -- either agrees or refuses.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    pub board: String,
    pub field: &'static str,
    pub from: f64,
    pub to: f64,
    /// The board's `timeout_secs` was the in-flight half of this tier move and
    /// is now equal to `budget_secs`, so it says nothing and should go.
    pub drop_timeout_secs: bool,
}

impl std::fmt::Display for Change {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "benchmarks/manifest.toml: [[board]] id = \"{}\" -- {} {} {} {}",
            self.board,
            self.field,
            self.from,
            fmt::glyph::ARROW,
            self.to
        )?;
        if self.drop_timeout_secs {
            write!(
                f,
                " (and drop the now-redundant timeout_secs = {})",
                self.to
            )?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The plan
// ---------------------------------------------------------------------------

/// One file, already read, waiting to be written somewhere else.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    board: String,
    src: PathBuf,
    dst: PathBuf,
    bytes: Vec<u8>,
}

impl Placement {
    pub fn board(&self) -> &str {
        &self.board
    }
    pub fn src(&self) -> &Path {
        &self.src
    }
    pub fn dst(&self) -> &Path {
        &self.dst
    }
    /// The exact bytes read from `src`. A promotion is a COPY: no re-encode, no
    /// normalisation, no trailing-newline opinion.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// A gated, fully-read promotion. Holding one of these means every gate has
/// already passed and the bytes are in hand.
#[derive(Debug, Clone, PartialEq)]
pub struct Promotion {
    placements: Vec<Placement>,
    changes: Vec<Change>,
    tier_moves: Vec<TierMove>,
}

impl Promotion {
    pub fn placements(&self) -> &[Placement] {
        &self.placements
    }

    /// Registry edits for the caller to apply. See [`Change`] on why this crate
    /// does not apply them itself.
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    /// The tier moves the evidence asked for and the policy accepted.
    pub fn tier_moves(&self) -> &[TierMove] {
        &self.tier_moves
    }

    /// The shell's per-board promotion lines, one per board rather than one per
    /// file: `  promoted ipc67-results       -> ipc67-results.md / ipc67-default.jsonl`.
    pub fn promoted_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < self.placements.len() {
            let board = &self.placements[i].board;
            // The span is counted from the placements, NOT from the names that
            // survive rendering: a `dst` whose file name did not render would
            // otherwise shorten the step and print the same board twice.
            let span = self.placements[i..]
                .iter()
                .take_while(|p| &p.board == board)
                .count()
                .max(1);
            let files: Vec<String> = self.placements[i..i + span]
                .iter()
                .map(|p| match p.dst.file_name() {
                    Some(n) => n.to_string_lossy().into_owned(),
                    None => p.dst.display().to_string(),
                })
                .collect();
            // ASCII `->`, because the shell's `printf '  promoted %-22s -> %s.md
            // / %s.jsonl\n'` is ASCII. `fmt::glyph::ARROW` is U+2192 and exists
            // for the generated README's "Full standings \u{2192}" line; reaching
            // for it here would silently reword an operator-facing line that has
            // a byte-exact predecessor.
            out.push(format!("  promoted {:<22} -> {}", board, files.join(" / ")));
            i += span;
        }
        out
    }

    /// Write every placement. Reads are already done, so a failure here cannot
    /// be a gate deciding late -- only the filesystem refusing.
    pub fn apply(&self) -> Result<(), PromoteError> {
        for p in &self.placements {
            if let Some(parent) = p.dst.parent() {
                std::fs::create_dir_all(parent).map_err(|source| PromoteError::Io {
                    path: parent.display().to_string(),
                    source,
                })?;
            }
            write_atomic(&p.dst, &p.bytes).map_err(|source| PromoteError::Io {
                path: p.dst.display().to_string(),
                source,
            })?;
        }
        Ok(())
    }
}

/// Read every staged board named by `sets`, run every gate, and return the
/// promotion -- or every reason it is refused.
///
/// `root` is the repo root: stages hang off it by their manifest-declared
/// path, destinations off `root/`[`BOARDS_DIR`].
pub fn plan(
    root: &Path,
    manifest: &Manifest,
    sets: &[&str],
    policy: &TierMovePolicy,
) -> Result<Promotion, GateReport> {
    // ---- phase 1: resolve, and the completeness gate ----
    //
    // Two phases rather than one, for the reason the shell had: a board with no
    // `.done` may be MID-WRITE, so every later gate on it would be reporting on
    // a file the driver has not finished. Running phase 2 anyway would bury the
    // one actionable line under a screenful of derived noise.
    let mut failures: Vec<Gate> = Vec::new();
    let mut staged: Vec<(String, String, PathBuf)> = Vec::new(); // set, board id, stage dir
    for name in sets {
        let Some(set) = manifest.set(name) else {
            failures.push(Gate::UnknownSet {
                name: (*name).to_string(),
            });
            continue;
        };
        let stage = root.join(&set.stage);
        for id in &set.boards {
            if let Some((prev_set, _, _)) = staged.iter().find(|(_, b, _)| b == id) {
                failures.push(Gate::DuplicateBoard {
                    board: id.clone(),
                    first: prev_set.clone(),
                    second: set.name.clone(),
                });
                continue;
            }
            if manifest.board(id).is_none() {
                failures.push(Gate::UnknownBoard {
                    set: set.name.clone(),
                    board: id.clone(),
                });
                continue;
            }
            // Existence only. The markers are zero bytes and their content is
            // never read -- a driver writes one by touching it, and reading it
            // would invent a second contract nobody maintains.
            if !stage.join(format!("{id}.done")).is_file() {
                failures.push(Gate::NotDone {
                    set: set.name.clone(),
                    board: id.clone(),
                });
                continue;
            }
            staged.push((set.name.clone(), id.clone(), stage.clone()));
        }
    }
    if !failures.is_empty() {
        return Err(GateReport::new(failures));
    }

    // ---- phase 2: presence, stamps, versions ----
    let mut placements: Vec<Placement> = Vec::new();
    let mut changes: Vec<Change> = Vec::new();
    let mut tier_moves: Vec<TierMove> = Vec::new();
    for (set, id, stage) in &staged {
        let spec = manifest.board(id).expect("phase 1 resolved every board");
        let md_src = stage.join(format!("{id}.md"));
        let raw_src = stage.join(format!("{id}.jsonl"));

        let md_bytes = match read_bytes(&md_src) {
            Ok(b) => Some(b),
            Err(g) => {
                failures.push(gate_for(id, &md_src, g));
                None
            }
        };
        let raw_bytes = match read_bytes(&raw_src) {
            Ok(b) => Some(b),
            Err(g) => {
                failures.push(gate_for(id, &raw_src, g));
                None
            }
        };
        let (Some(md_bytes), Some(raw_bytes)) = (md_bytes, raw_bytes) else {
            continue;
        };

        // The presence gate's non-empty half, before anything tries to read a
        // first line out of it.
        if raw_bytes.is_empty() {
            failures.push(Gate::EmptyRaw {
                board: id.clone(),
                path: raw_src.display().to_string(),
            });
            continue;
        }
        let shown = raw_src.display().to_string();
        let Ok(text) = std::str::from_utf8(&raw_bytes) else {
            failures.push(Gate::Unreadable {
                board: id.clone(),
                msg: format!("{shown}: not UTF-8"),
            });
            continue;
        };
        let rows = match parse_rows(text, &shown) {
            Ok(r) => r,
            Err(msg) => {
                failures.push(Gate::Unreadable {
                    board: id.clone(),
                    msg,
                });
                continue;
            }
        };
        if rows.is_empty() {
            failures.push(Gate::EmptyRaw {
                board: id.clone(),
                path: shown,
            });
            continue;
        }

        // EVERY row, not line one. `distinct` keeps first-seen order so the
        // refusal lists the stamps in the order the raw does, which is the
        // order they were swept in and the order a stitched board's segments
        // appear in.
        let stamps = distinct(rows.iter().map(|r| r.budget));
        let versions = distinct(rows.iter().map(|r| r.ver.clone()));

        if versions.len() > 1 {
            failures.push(Gate::VersionDrift {
                set: set.clone(),
                board: id.clone(),
                count: versions.len(),
                versions: versions
                    .iter()
                    .map(|v| match v {
                        Some(s) => format!("{s:?}"),
                        None => "unstamped".to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            });
        }

        let stamp = if stamps.len() > 1 {
            failures.push(Gate::NonUniformStamps {
                set: set.clone(),
                board: id.clone(),
                count: stamps.len(),
                stamps: stamps
                    .iter()
                    .map(|b| match b {
                        Some(v) => format!("{v}s"),
                        None => "unstamped".to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            });
            None
        } else {
            match stamps[0] {
                None => {
                    failures.push(Gate::Unstamped { board: id.clone() });
                    None
                }
                // Not merely `!= 0.0`: `referee::budget_for` reads a stamp of 0
                // as ABSENT, so a zero here would silently be scored at the
                // registry budget and this gate would have waved it through.
                Some(b) if !(b.is_finite() && b > 0.0) => {
                    failures.push(Gate::BadStamp {
                        board: id.clone(),
                        stamp: format!("{b}"),
                    });
                    None
                }
                Some(b) => Some(b),
            }
        };

        // THE INVERTED MUTATION. The evidence is already in hand; the manifest
        // is compared against IT, not the other way round.
        if let Some(b) = stamp {
            if b != spec.budget_secs {
                if policy.accepts(id) {
                    tier_moves.push(TierMove {
                        board: id.clone(),
                        from: spec.budget_secs,
                        to: b,
                    });
                    changes.push(Change {
                        board: id.clone(),
                        field: "budget_secs",
                        from: spec.budget_secs,
                        to: b,
                        drop_timeout_secs: spec.timeout_secs.map(|t| t as f64) == Some(b),
                    });
                } else {
                    failures.push(Gate::TierMoveNotAccepted {
                        board: id.clone(),
                        stamp: format!("{b}"),
                        registry: format!("{}", spec.budget_secs),
                    });
                }
            }
        }

        // The naming exception is DATA: the destination names come from the
        // manifest's (id, raw, md) triple, so `ipc67-results.jsonl` landing as
        // `ipc67-default.jsonl` is a row in a table rather than a branch in a
        // shell function.
        let boards = root.join(BOARDS_DIR);
        placements.push(Placement {
            board: id.clone(),
            src: md_src,
            dst: boards.join(&spec.md),
            bytes: md_bytes,
        });
        placements.push(Placement {
            board: id.clone(),
            src: raw_src,
            dst: boards.join(&spec.raw),
            bytes: raw_bytes,
        });
    }

    if !failures.is_empty() {
        return Err(GateReport::new(failures));
    }
    Ok(Promotion {
        placements,
        changes,
        tier_moves,
    })
}

/// Write via a temp sibling and a `rename`, so `dst` is either the old file or
/// the new one and never a torn prefix.
///
/// Shared with [`crate::snapshot`] because both release steps have the same
/// thing to lose. For a `.jsonl` a torn write is not even a read error: a
/// truncated tail line is SKIPPED by `parse_rows` on purpose, so the next
/// standings run would publish a board short some rows without a word. For
/// `standings-history.json` a torn write loses the record every "vs previous"
/// column in the project is computed from.
pub(crate) fn write_atomic(dst: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let mut tmp = dst.to_path_buf().into_os_string();
    tmp.push(TMP_SUFFIX);
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes)?;
    if let Err(e) = std::fs::rename(&tmp, dst) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

/// Missing, or unreadable-but-present. The caller turns this into a [`Gate`]
/// so both cases carry the board name.
enum ReadFail {
    Missing,
    Io(std::io::Error),
}

fn read_bytes(p: &Path) -> Result<Vec<u8>, ReadFail> {
    match std::fs::read(p) {
        Ok(b) => Ok(b),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(ReadFail::Missing),
        Err(e) => Err(ReadFail::Io(e)),
    }
}

fn gate_for(board: &str, path: &Path, f: ReadFail) -> Gate {
    match f {
        ReadFail::Missing => Gate::Missing {
            board: board.to_string(),
            path: path.display().to_string(),
        },
        ReadFail::Io(e) => Gate::Unreadable {
            board: board.to_string(),
            msg: format!("{}: {e}", path.display()),
        },
    }
}

/// Distinct values in FIRST-SEEN order.
///
/// Linear on purpose: a raw carries one or two distinct stamps, and the order
/// is the raw's own. A `HashSet` would be faster and would scramble the order
/// the refusal reports them in, which is the order that tells you which segment
/// of a stitched board came from where.
///
/// `Option<f64>` compares with `==`, so a NaN stamp never equals itself and
/// lands here as many distinct values -- a refusal, which is the safe
/// direction: JSON cannot spell NaN, so one could only arrive through a
/// corruption this gate should not be waving through.
fn distinct<T: PartialEq>(items: impl Iterator<Item = T>) -> Vec<T> {
    let mut out: Vec<T> = Vec::new();
    for it in items {
        if !out.contains(&it) {
            out.push(it);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The two headlines
// ---------------------------------------------------------------------------

/// One live board's coverage, as the standings renderer computed it.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveBoard {
    pub label: String,
    pub solved: usize,
    pub total: usize,
}

/// A coverage total over some partition of the live boards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Headline {
    boards: usize,
    solved: usize,
    total: usize,
}

impl Headline {
    pub fn boards(&self) -> usize {
        self.boards
    }
    pub fn solved(&self) -> usize {
        self.solved
    }
    pub fn total(&self) -> usize {
        self.total
    }

    /// The share. Integer sums, so the float-associativity hazard that governs
    /// the rest of this crate does not reach here: `solved` and `total` are
    /// exact whatever order the boards were added in, and only the final
    /// division is floating point.
    pub fn pct(&self) -> f64 {
        fmt::pct(self.solved, self.total)
    }

    /// `write_summary`'s headline sentence, for one partition.
    pub fn sentence(&self, box_: &str) -> String {
        let plural = if self.boards == 1 { "board" } else { "boards" };
        format!(
            "**{}% coverage across {} IPC {plural}** ({}/{}), measured on `{box_}`.",
            fmt::fmt_f(self.pct(), 0),
            self.boards,
            fmt::thousands(self.solved as u64),
            fmt::thousands(self.total as u64),
        )
    }
}

/// The cut record's two headline numbers, in one value that cannot render one
/// without the other.
///
/// `docs/roadmap-0.25.md` Phase 6 states the law: *"The denominator grows and
/// the total percentage may DROP on entry day -- the record says so first,
/// loudly, so a bigger honest table never reads as a regression."*
///
/// A convention would have been enough exactly once. The like-for-like number
/// is the instrument every prior cut used and the full table is the honest one,
/// and on entry day they disagree in the direction that looks like a
/// regression: ten new boards enter at whatever coverage they enter at, and the
/// total falls even though not one board got worse. Publishing the full table
/// alone reads as a regression that did not happen; publishing the
/// like-for-like alone hides ten boards. So [`Display`](std::fmt::Display)
/// prints BOTH, always, and leads with the drop when there is one -- and the
/// only way to obtain this type is [`TwoHeadlines::compute`], which derives the
/// two flags rather than accepting them.
///
/// The partition itself is not a set membership question with a second source
/// of truth: `like_for_like` is the boards the PREDECESSOR measured, which is
/// the definition of like-for-like, and which is exactly what a
/// [`ComparablePredecessor`] is able to answer.
#[derive(Debug, Clone, PartialEq)]
pub struct TwoHeadlines {
    like_for_like: Headline,
    full_table: Headline,
    entries: Vec<String>,
    denominator_grew: bool,
    pct_dropped_on_entry_day: bool,
    box_: String,
    against: Option<String>,
}

impl TwoHeadlines {
    /// Partition `live` against the release it is being measured against.
    ///
    /// Membership is asked through [`ComparablePredecessor::delta`] rather than
    /// by reaching into the predecessor's track list, and that is deliberate:
    /// the delta column is the one blessed comparison in this crate, and
    /// "the predecessor has a comparable share for this board" is precisely the
    /// question it answers. A board it reports as [`NEW_CELL`] has either never
    /// been measured or was measured with no denominator, and neither can sit
    /// inside a like-for-like total.
    ///
    /// With no comparable predecessor the like-for-like partition is EMPTY, not
    /// the full table: the first cut on a box has no prior instrument, and
    /// pretending it does would quote a movement against nothing.
    pub fn compute(
        box_: &str,
        prev: Option<&ComparablePredecessor<'_>>,
        live: &[LiveBoard],
    ) -> Self {
        let mut like_for_like = Headline::default();
        let mut full_table = Headline::default();
        let mut entries = Vec::new();
        for b in live {
            full_table.boards += 1;
            full_table.solved += b.solved;
            full_table.total += b.total;
            let known = prev
                .map(|p| p.delta(&b.label, b.solved, b.total) != NEW_CELL)
                .unwrap_or(false);
            if known {
                like_for_like.boards += 1;
                like_for_like.solved += b.solved;
                like_for_like.total += b.total;
            } else {
                entries.push(b.label.clone());
            }
        }
        Self {
            denominator_grew: full_table.total > like_for_like.total,
            // The TRUE share, with a strict `<`, and not the printed integer.
            // The two failures are not symmetric: an unnecessary sentence costs
            // a line of prose, while a missing one costs a published number
            // that reads as a regression nobody caused. The sentence quotes one
            // decimal so a hair-thin drop is visibly hair-thin.
            pct_dropped_on_entry_day: full_table.pct() < like_for_like.pct(),
            like_for_like,
            full_table,
            entries,
            box_: box_.to_string(),
            against: prev.map(|p| p.version().to_string()),
        }
    }

    pub fn like_for_like(&self) -> Headline {
        self.like_for_like
    }
    pub fn full_table(&self) -> Headline {
        self.full_table
    }
    /// The labels the predecessor never measured, in the order `live` gave.
    pub fn entries(&self) -> &[String] {
        &self.entries
    }
    pub fn denominator_grew(&self) -> bool {
        self.denominator_grew
    }
    pub fn pct_dropped_on_entry_day(&self) -> bool {
        self.pct_dropped_on_entry_day
    }
    /// The release the like-for-like partition is drawn from, if any.
    pub fn against(&self) -> Option<&str> {
        self.against.as_deref()
    }
}

impl std::fmt::Display for TwoHeadlines {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The drop goes FIRST. Everything below it is the evidence for it, and
        // a reader who stops after one line has still been told the thing that
        // would otherwise be read as a regression.
        if self.pct_dropped_on_entry_day {
            writeln!(
                f,
                "THE TOTAL PERCENTAGE DROPPED ON ENTRY DAY: {}% {} {}%. The denominator grew \
                 by {} instances across {} new {}; a bigger honest table is not a regression.",
                fmt::fmt_f(self.like_for_like.pct(), 1),
                fmt::glyph::ARROW,
                fmt::fmt_f(self.full_table.pct(), 1),
                fmt::thousands(
                    self.full_table
                        .total
                        .saturating_sub(self.like_for_like.total) as u64
                ),
                self.entries.len(),
                if self.entries.len() == 1 {
                    "board"
                } else {
                    "boards"
                },
            )?;
            writeln!(f)?;
        }
        match &self.against {
            Some(v) => writeln!(
                f,
                "like-for-like (the boards {v} also measured): {}",
                self.like_for_like.sentence(&self.box_)
            )?,
            None => writeln!(
                f,
                "like-for-like: no comparable predecessor on `{}` \u{2014} this is the baseline.",
                self.box_
            )?,
        }
        write!(
            f,
            "full table (every live board): {}",
            self.full_table.sentence(&self.box_)
        )?;
        if !self.entries.is_empty() {
            write!(
                f,
                "\nentries ({}): {}",
                self.entries.len(),
                self.entries
                    .iter()
                    .map(|e| format!("`{e}`"))
                    .collect::<Vec<_>>()
                    .join(&format!(" {} ", fmt::glyph::MIDDOT))
            )?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{BoxId, History, MeasuredAt, Snapshot, VersionKey};
    use std::sync::atomic::{AtomicUsize, Ordering};

    const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../..");

    fn tmproot(tag: &str) -> PathBuf {
        static N: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "crucible-promote-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// A manifest with the boards a test names. `(id, raw, md, budget_secs,
    /// timeout_secs)`.
    fn manifest(
        boards: &[(&str, &str, &str, f64, Option<u64>)],
        sets: &[(&str, &str, &[&str])],
    ) -> Manifest {
        let mut t = String::from(
            "schema = 1\n\
             [corpus]\nroot = \".c\"\ndomain_shared = \"domain.pddl\"\n\
             domain_per_instance = \"domains/domain-{first}.pddl\"\n\
             [defaults]\ntimeout_secs = 60\njobs = 2\nthreads = 1\nmode = \"auto\"\nmem_gb = 6.0\n",
        );
        for (id, raw, md, budget, timeout) in boards {
            t.push_str(&format!(
                "[[board]]\nid = \"{id}\"\nraw = \"{raw}\"\nmd = \"{md}\"\n\
                 label = \"{id} label\"\ncompetition = \"x\"\nbudget_secs = {budget}\n\
                 track = \"t\"\n"
            ));
            if let Some(ts) = timeout {
                t.push_str(&format!("timeout_secs = {ts}\n"));
            }
        }
        for (name, stage, ids) in sets {
            t.push_str(&format!(
                "[[set]]\nname = \"{name}\"\nstage = \"{stage}\"\nboards = ["
            ));
            for id in *ids {
                t.push_str(&format!("\"{id}\","));
            }
            t.push_str("]\n");
        }
        Manifest::parse(&t, "test-manifest.toml").unwrap_or_else(|e| panic!("{e}"))
    }

    /// One raw row, with whatever `budget`/`ver` the test needs.
    fn row(inst: u64, budget: Option<&str>, ver: Option<&str>) -> String {
        let mut s = format!("{{\"variant\": \"v\", \"instance\": {inst}, \"solved\": true");
        if let Some(b) = budget {
            s.push_str(&format!(", \"budget\": {b}"));
        }
        if let Some(v) = ver {
            s.push_str(&format!(", \"ver\": \"{v}\""));
        }
        s.push('}');
        s
    }

    /// Stage a board: `.done`, `.md`, `.jsonl`.
    fn stage(root: &Path, dir: &str, id: &str, rows: &[String], done: bool) {
        let d = root.join(dir);
        std::fs::create_dir_all(&d).unwrap();
        if done {
            std::fs::write(d.join(format!("{id}.done")), b"").unwrap();
        }
        std::fs::write(d.join(format!("{id}.md")), format!("# {id}\n")).unwrap();
        std::fs::write(
            d.join(format!("{id}.jsonl")),
            rows.iter().map(|r| format!("{r}\n")).collect::<String>(),
        )
        .unwrap();
    }

    /// The shell prints EVERY missing board before it exits, and so does this:
    /// a gate that stopped at the first hole would be run once per hole.
    #[test]
    fn every_missing_marker_is_named_in_one_refusal() {
        let root = tmproot("partial");
        let m = manifest(
            &[
                ("a", "a.jsonl", "a.md", 60.0, None),
                ("b", "b.jsonl", "b.md", 60.0, None),
                ("c", "c.jsonl", "c.md", 60.0, None),
            ],
            &[("s", "benchmarks/stage", &["a", "b", "c"])],
        );
        stage(
            &root,
            "benchmarks/stage",
            "a",
            &[row(1, Some("60"), None)],
            true,
        );
        stage(
            &root,
            "benchmarks/stage",
            "b",
            &[row(1, Some("60"), None)],
            false,
        );
        stage(
            &root,
            "benchmarks/stage",
            "c",
            &[row(1, Some("60"), None)],
            false,
        );
        let err = plan(&root, &m, &["s"], &TierMovePolicy::Refuse).unwrap_err();
        assert_eq!(err.failures().len(), 2, "{err}");
        assert_eq!(err.refusal(), PARTIAL_SWEEP_REFUSAL);
        let text = err.to_string();
        assert!(text.contains("NOT DONE (s): b"), "{text}");
        assert!(text.contains("NOT DONE (s): c"), "{text}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A board still being written must not have its half-written raw
    /// diagnosed: the completeness gate runs alone, then everything else.
    #[test]
    fn a_partial_sweep_reports_only_the_partiality() {
        let root = tmproot("phases");
        let m = manifest(
            &[
                ("a", "a.jsonl", "a.md", 60.0, None),
                ("b", "b.jsonl", "b.md", 60.0, None),
            ],
            &[("s", "benchmarks/stage", &["a", "b"])],
        );
        // `a` is done but its raw is stitched across a tier change -- a phase-2
        // failure that must NOT be reported while `b` is still in flight.
        stage(
            &root,
            "benchmarks/stage",
            "a",
            &[row(1, Some("60"), None), row(2, Some("30"), None)],
            true,
        );
        stage(
            &root,
            "benchmarks/stage",
            "b",
            &[row(1, Some("60"), None)],
            false,
        );
        let err = plan(&root, &m, &["s"], &TierMovePolicy::Refuse).unwrap_err();
        assert_eq!(err.failures().len(), 1, "{err}");
        assert!(matches!(err.failures()[0], Gate::NotDone { .. }));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// THE hole in the shell. `json.loads(next(f))` reads line ONE; a raw
    /// stitched across a tier change is uniform there and wrong everywhere
    /// after, which is exactly the situation the stamp mechanism exists for.
    #[test]
    fn the_stamp_gate_reads_every_row_not_the_first() {
        let root = tmproot("stitched");
        let m = manifest(
            &[("a", "a.jsonl", "a.md", 60.0, None)],
            &[("s", "benchmarks/stage", &["a"])],
        );
        stage(
            &root,
            "benchmarks/stage",
            "a",
            &[
                row(1, Some("60"), None),
                row(2, Some("60"), None),
                row(3, Some("30"), None),
            ],
            true,
        );
        let err = plan(&root, &m, &["s"], &TierMovePolicy::Refuse).unwrap_err();
        let text = err.to_string();
        assert!(
            matches!(err.failures()[0], Gate::NonUniformStamps { count: 2, .. }),
            "{text}"
        );
        // First-seen order, so the segments read the way the raw is laid out.
        assert!(text.contains("(60s, 30s)"), "{text}");
        assert!(text.contains("set s"), "{text}");
        assert_eq!(err.refusal(), STAMP_REFUSAL);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// PER-INSTANCE-RETRY.md's first care point, enforced nowhere until here:
    /// a stitched board must never mix rows from two `ff` builds.
    #[test]
    fn version_drift_across_a_stitched_board_refuses() {
        let root = tmproot("verdrift");
        let m = manifest(
            &[("a", "a.jsonl", "a.md", 60.0, None)],
            &[("s", "benchmarks/stage", &["a"])],
        );
        stage(
            &root,
            "benchmarks/stage",
            "a",
            &[
                row(1, Some("60"), Some("ff 0.25.0")),
                row(2, Some("60"), Some("ff 0.25.1")),
            ],
            true,
        );
        let err = plan(&root, &m, &["s"], &TierMovePolicy::Refuse).unwrap_err();
        assert!(
            matches!(err.failures()[0], Gate::VersionDrift { count: 2, .. }),
            "{err}"
        );
        // Uniformly UNSTAMPED is uniform: the pre-0.25 boards carry no `ver` at
        // all and must promote, or the gate would refuse the whole standing 22.
        let root2 = tmproot("verlegacy");
        stage(
            &root2,
            "benchmarks/stage",
            "a",
            &[row(1, Some("60"), None), row(2, Some("60"), None)],
            true,
        );
        assert!(plan(&root2, &m, &["s"], &TierMovePolicy::Refuse).is_ok());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&root2);
    }

    /// A stamp of `0` is the one value `referee::budget_for` reads as ABSENT
    /// (Python's falsy `or`), so it would have been scored at the registry
    /// budget and never seen again.
    #[test]
    fn a_zero_stamp_is_refused_rather_than_silently_falling_back() {
        let root = tmproot("zero");
        let m = manifest(
            &[("a", "a.jsonl", "a.md", 60.0, None)],
            &[("s", "benchmarks/stage", &["a"])],
        );
        stage(
            &root,
            "benchmarks/stage",
            "a",
            &[row(1, Some("0"), None)],
            true,
        );
        let err = plan(&root, &m, &["s"], &TierMovePolicy::Refuse).unwrap_err();
        assert!(matches!(err.failures()[0], Gate::BadStamp { .. }), "{err}");

        // And a raw with no stamp anywhere is a pre-0.23 board: the tier
        // mechanism cannot see it, so there is nothing to promote against.
        let root2 = tmproot("nostamp");
        stage(&root2, "benchmarks/stage", "a", &[row(1, None, None)], true);
        let err2 = plan(&root2, &m, &["s"], &TierMovePolicy::Refuse).unwrap_err();
        assert!(
            matches!(err2.failures()[0], Gate::Unstamped { .. }),
            "{err2}"
        );
        assert_eq!(err2.refusal(), STAMP_REFUSAL);
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&root2);
    }

    /// The inversion. The 0.25 shape: the manifest scores `ipc5-time` at 30 with
    /// `timeout_secs = 60` in flight, and the raws come back uniformly stamped
    /// 60. That is a tier move DISCOVERED, and it does not happen by itself.
    #[test]
    fn a_tier_move_is_discovered_from_evidence_and_must_be_accepted() {
        let root = tmproot("tier");
        let m = manifest(
            &[(
                "ipc5-time",
                "ipc5-time.jsonl",
                "ipc5-time.md",
                30.0,
                Some(60),
            )],
            &[("s", "benchmarks/stage", &["ipc5-time"])],
        );
        stage(
            &root,
            "benchmarks/stage",
            "ipc5-time",
            &[row(1, Some("60"), None), row(2, Some("60"), None)],
            true,
        );
        let err = plan(&root, &m, &["s"], &TierMovePolicy::Refuse).unwrap_err();
        assert!(
            matches!(err.failures()[0], Gate::TierMoveNotAccepted { .. }),
            "{err}"
        );
        // The shell's refusal, word for word.
        assert_eq!(err.refusal(), STAMP_REFUSAL);

        // Accepting it DESCRIBES the registry edit; it does not perform one.
        let p = plan(&root, &m, &["s"], &TierMovePolicy::AcceptAll).unwrap();
        assert_eq!(
            p.tier_moves(),
            &[TierMove {
                board: "ipc5-time".into(),
                from: 30.0,
                to: 60.0
            }]
        );
        let c = &p.changes()[0];
        assert_eq!(
            (c.field, c.from, c.to, c.drop_timeout_secs),
            ("budget_secs", 30.0, 60.0, true)
        );
        assert!(c.to_string().contains("budget_secs 30"), "{c}");
        assert!(c.to_string().contains("timeout_secs = 60"), "{c}");

        // The per-board flag names ONE board and blesses only that one: the
        // failure flip-then-prove had was a second move nobody looked at.
        let other = TierMovePolicy::Accept(vec!["ipc5-prop".into()]);
        assert!(plan(&root, &m, &["s"], &other).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The evidence agreeing with the registry is the ordinary case, and it
    /// must produce NO change at all -- flip-then-prove's edit ran every time.
    #[test]
    fn a_matching_stamp_changes_nothing() {
        let root = tmproot("match");
        let m = manifest(
            &[("a", "a.jsonl", "a.md", 60.0, None)],
            &[("s", "benchmarks/stage", &["a"])],
        );
        stage(
            &root,
            "benchmarks/stage",
            "a",
            &[row(1, Some("60"), None)],
            true,
        );
        let p = plan(&root, &m, &["s"], &TierMovePolicy::AcceptAll).unwrap();
        assert!(p.changes().is_empty());
        assert!(p.tier_moves().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The one naming exception is a row in the manifest, not a branch: staged
    /// as `ipc67-results.jsonl`, promoted to `benchmarks/ipc67-default.jsonl`
    /// while its `.md` keeps the board's own name.
    #[test]
    fn the_naming_exception_comes_from_the_manifest_triple() {
        let root = tmproot("naming");
        let m = manifest(
            &[(
                "ipc67-results",
                "ipc67-default.jsonl",
                "ipc67-results.md",
                60.0,
                None,
            )],
            &[("s", "benchmarks/air25", &["ipc67-results"])],
        );
        stage(
            &root,
            "benchmarks/air25",
            "ipc67-results",
            &[row(1, Some("60"), None)],
            true,
        );
        let p = plan(&root, &m, &["s"], &TierMovePolicy::Refuse).unwrap();
        p.apply().unwrap();
        assert!(root.join("benchmarks/ipc67-default.jsonl").is_file());
        assert!(root.join("benchmarks/ipc67-results.md").is_file());
        assert!(!root.join("benchmarks/ipc67-results.jsonl").exists());
        assert_eq!(
            p.promoted_lines(),
            vec![format!(
                "  promoted {:<22} -> ipc67-results.md / ipc67-default.jsonl",
                "ipc67-results"
            )]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The reads-then-writes property, proved rather than asserted: the staged
    /// files are DELETED between planning and applying, and the promotion still
    /// writes the right bytes. Nothing is read after a gate has passed.
    #[test]
    fn every_read_happens_before_any_write() {
        let root = tmproot("order");
        let m = manifest(
            &[("a", "a.jsonl", "a.md", 60.0, None)],
            &[("s", "benchmarks/stage", &["a"])],
        );
        stage(
            &root,
            "benchmarks/stage",
            "a",
            &[row(7, Some("60"), None)],
            true,
        );
        let p = plan(&root, &m, &["s"], &TierMovePolicy::Refuse).unwrap();
        std::fs::remove_dir_all(root.join("benchmarks/stage")).unwrap();
        p.apply().unwrap();
        let raw = std::fs::read_to_string(root.join("benchmarks/a.jsonl")).unwrap();
        assert!(raw.contains("\"instance\": 7"), "{raw}");
        assert_eq!(
            std::fs::read_to_string(root.join("benchmarks/a.md")).unwrap(),
            "# a\n"
        );
        // No temp sibling survives a successful apply.
        assert!(!root
            .join(format!("benchmarks/a.jsonl{TMP_SUFFIX}"))
            .exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Presence: both files, and a raw with rows in it. Python's `next(f)` on
    /// an empty raw is a bare `StopIteration` with no board name in it.
    #[test]
    fn presence_names_the_board_an_empty_raw_belongs_to() {
        let root = tmproot("presence");
        let m = manifest(
            &[
                ("a", "a.jsonl", "a.md", 60.0, None),
                ("b", "b.jsonl", "b.md", 60.0, None),
            ],
            &[("s", "benchmarks/stage", &["a", "b"])],
        );
        stage(&root, "benchmarks/stage", "a", &[], true);
        stage(
            &root,
            "benchmarks/stage",
            "b",
            &[row(1, Some("60"), None)],
            true,
        );
        std::fs::remove_file(root.join("benchmarks/stage/b.md")).unwrap();
        let err = plan(&root, &m, &["s"], &TierMovePolicy::Refuse).unwrap_err();
        let text = err.to_string();
        assert!(
            text.contains("a: ") && text.contains("has no rows"),
            "{text}"
        );
        assert!(text.contains("b: missing"), "{text}");
        assert_eq!(err.refusal(), REFUSAL);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// One board staged twice is two cycles under one name, which is the whole
    /// reason the `.done` sweep exists -- arriving through the manifest instead.
    #[test]
    fn a_board_staged_by_two_sets_is_refused() {
        let root = tmproot("dupe");
        let m = manifest(
            &[("a", "a.jsonl", "a.md", 60.0, None)],
            &[
                ("cut", "benchmarks/air25", &["a"]),
                ("entries", "benchmarks/air25-entries", &["a"]),
            ],
        );
        stage(
            &root,
            "benchmarks/air25",
            "a",
            &[row(1, Some("60"), None)],
            true,
        );
        stage(
            &root,
            "benchmarks/air25-entries",
            "a",
            &[row(1, Some("60"), None)],
            true,
        );
        let err = plan(&root, &m, &["cut", "entries"], &TierMovePolicy::Refuse).unwrap_err();
        assert!(
            matches!(err.failures()[0], Gate::DuplicateBoard { .. }),
            "{err}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    // ---- the two headlines ----

    fn history_with(prev_version: &str, tracks: &[(&str, usize, usize)]) -> History {
        let mut h = History::default();
        h.upsert(Snapshot {
            version: prev_version.to_string(),
            released: "2026-08-20".to_string(),
            measured_on: BoxId::new("m5-air"),
            measured_at: MeasuredAt::new("2026-08-20"),
            note: String::new(),
            tracks: tracks
                .iter()
                .map(|(l, s, n)| ((*l).to_string(), (*s, *n)))
                .collect(),
        });
        h
    }

    /// The law, as a type: BOTH partitions render, always, and the drop leads.
    /// The shape is the real one -- 0.24.0 measured two boards, ten entries
    /// arrive weaker, and the total falls without a single board getting worse.
    #[test]
    fn both_headlines_always_render_and_a_drop_leads() {
        let h = history_with("0.24.0", &[("old-a", 80, 100), ("old-b", 60, 100)]);
        let air = BoxId::new("m5-air");
        let prev = h
            .comparable_predecessor(&air, &VersionKey::parse("0.25.0"))
            .unwrap();
        let live = vec![
            LiveBoard {
                label: "old-a".into(),
                solved: 82,
                total: 100,
            },
            LiveBoard {
                label: "old-b".into(),
                solved: 61,
                total: 100,
            },
            LiveBoard {
                label: "entry-1".into(),
                solved: 10,
                total: 100,
            },
            LiveBoard {
                label: "entry-2".into(),
                solved: 20,
                total: 100,
            },
        ];
        let t = TwoHeadlines::compute("m5-air", Some(&prev), &live);
        assert_eq!(
            (t.like_for_like().solved(), t.like_for_like().total()),
            (143, 200)
        );
        assert_eq!(
            (t.full_table().solved(), t.full_table().total()),
            (173, 400)
        );
        assert_eq!(t.entries(), ["entry-1", "entry-2"]);
        assert!(t.denominator_grew());
        assert!(t.pct_dropped_on_entry_day());

        let s = t.to_string();
        // The drop is the FIRST thing said.
        assert!(
            s.starts_with("THE TOTAL PERCENTAGE DROPPED ON ENTRY DAY"),
            "{s}"
        );
        assert!(s.contains("71.5% \u{2192} 43.2%"), "{s}");
        // Both partitions, named, in every rendering.
        assert!(
            s.contains("like-for-like (the boards 0.24.0 also measured)"),
            "{s}"
        );
        assert!(s.contains("full table (every live board)"), "{s}");
        assert!(
            s.contains("**72% coverage across 2 IPC boards** (143/200)"),
            "{s}"
        );
        assert!(
            s.contains("**43% coverage across 4 IPC boards** (173/400)"),
            "{s}"
        );
        assert!(s.contains("`entry-1` \u{b7} `entry-2`"), "{s}");
    }

    /// No drop, no lead sentence -- but still both headlines. A cut where the
    /// entries come in strong is not licence to print one number.
    #[test]
    fn both_headlines_render_when_nothing_dropped() {
        let h = history_with("0.24.0", &[("old-a", 50, 100)]);
        let air = BoxId::new("m5-air");
        let prev = h
            .comparable_predecessor(&air, &VersionKey::parse("0.25.0"))
            .unwrap();
        let live = vec![
            LiveBoard {
                label: "old-a".into(),
                solved: 55,
                total: 100,
            },
            LiveBoard {
                label: "entry-1".into(),
                solved: 90,
                total: 100,
            },
        ];
        let t = TwoHeadlines::compute("m5-air", Some(&prev), &live);
        assert!(!t.pct_dropped_on_entry_day());
        assert!(t.denominator_grew());
        let s = t.to_string();
        assert!(!s.contains("DROPPED"), "{s}");
        assert!(s.contains("like-for-like"), "{s}");
        assert!(s.contains("full table"), "{s}");
    }

    /// Membership is the PREDECESSOR's question. A board it never measured is
    /// an entry; so is one it measured with no denominator, because a zero
    /// denominator is not a share and cannot sit inside a like-for-like total.
    #[test]
    fn a_board_the_predecessor_cannot_share_is_an_entry() {
        let h = history_with("0.24.0", &[("kept", 5, 10), ("hollow", 0, 0)]);
        let air = BoxId::new("m5-air");
        let prev = h
            .comparable_predecessor(&air, &VersionKey::parse("0.25.0"))
            .unwrap();
        let live = vec![
            LiveBoard {
                label: "kept".into(),
                solved: 6,
                total: 10,
            },
            LiveBoard {
                label: "hollow".into(),
                solved: 1,
                total: 10,
            },
            LiveBoard {
                label: "brand-new".into(),
                solved: 2,
                total: 10,
            },
        ];
        let t = TwoHeadlines::compute("m5-air", Some(&prev), &live);
        assert_eq!(t.entries(), ["hollow", "brand-new"]);
        assert_eq!(t.like_for_like().boards(), 1);
    }

    /// The first cut on a box has no prior instrument. The like-for-like
    /// partition is EMPTY rather than a copy of the full table, and the
    /// rendering says so instead of quoting a movement against nothing.
    #[test]
    fn no_predecessor_leaves_the_like_for_like_partition_empty() {
        let live = vec![LiveBoard {
            label: "a".into(),
            solved: 1,
            total: 2,
        }];
        let t = TwoHeadlines::compute("m5-air", None, &live);
        assert_eq!(t.like_for_like().boards(), 0);
        assert_eq!(t.entries(), ["a"]);
        assert!(!t.pct_dropped_on_entry_day());
        assert!(t.against().is_none());
        let s = t.to_string();
        assert!(s.contains("no comparable predecessor"), "{s}");
        assert!(s.contains("full table"), "{s}");
    }

    /// The two refusals, byte for byte as the shell printed them. An operator
    /// who has seen one of these before should recognise it, and a refusal that
    /// quietly reworded itself in the port is one more thing to have to prove
    /// is the same refusal.
    #[test]
    fn the_shells_refusals_are_preserved_verbatim() {
        assert_eq!(PARTIAL_SWEEP_REFUSAL, "refusing to promote a partial sweep");
        // Built line by line rather than with a `\`-continuation, so this
        // cannot pass by sharing the const's own escaping.
        assert_eq!(
            STAMP_REFUSAL.lines().collect::<Vec<_>>(),
            vec![
                "refusing to promote: fix the SWEEPS budget or re-sweep the",
                "mismatched board at the registry budget.",
            ]
        );
    }

    /// The real manifest's two staging directories, still two: the standing 22
    /// keep their like-for-like identity in one stage and the entries land in
    /// another, which is what makes the second headline computable at all.
    #[test]
    fn the_committed_manifest_still_stages_the_two_partitions_apart() {
        let m = Manifest::load(Path::new(&format!("{REPO}/benchmarks/manifest.toml")))
            .unwrap_or_else(|e| panic!("{e}"));
        let cut = m.set("cut25").expect("cut25");
        let entries = m.set("entries25").expect("entries25");
        assert_ne!(cut.stage, entries.stage);
        // Every board a set names is a board the manifest defines: the
        // `ipc5-complex-pref` incident was a board registered in SWEEPS and
        // swept by a script while appearing in no driver array at all.
        for s in &m.sets {
            for b in &s.boards {
                assert!(
                    m.board(b).is_some(),
                    "set {} names unknown board {b}",
                    s.name
                );
            }
        }
    }
}
