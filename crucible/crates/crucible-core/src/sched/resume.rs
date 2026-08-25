//! Which prior measurements may be reused, and -- far more important -- which
//! may not.
//!
//! `benchmarks/entries25-sweeps.sh` burned most of 2026-08-21 re-running whole
//! boards from scratch: three passes, roughly nine board-hours, nearly all of
//! it re-measuring rows that were already clean the first time. A board is
//! 100-400 instances and runs forty minutes to two and a half hours; the
//! contention that kills it is typically a five-to-fifteen minute window -- a
//! background `cargo build`, a Spotlight reindex -- somewhere in the middle.
//! Everything measured before and after that window was fine, and the
//! whole-board retry threw it away anyway. `PER-INSTANCE-RETRY.md` is the
//! design record for making the ATOM an instance instead of a board, and this
//! module is its gate: for each instance, may the prior row stand?
//!
//! The gate exists to be refused. Every branch below fails CLOSED, because the
//! failure modes are wildly asymmetric: a needless re-run costs sixty seconds,
//! while a wrongly reused row silently publishes a number that was never
//! measured under the conditions it claims. `ipc67.py:540` puts it in one line
//! -- `load_resume`'s own docstring, "a silently stitched row measured under
//! different settings is worse than a discarded board" -- and that sentence is
//! the whole specification for this file.
//!
//! # What each refusal defends
//!
//! * **The engine, and a DELIBERATE STRENGTHENING of the Python.**
//!   `ipc67.py:572` compares the `ff --version` STRING. That is too weak to do
//!   the job the design record asks of it: every dev build of a cycle reports
//!   `ff 0.25.0`, so a board stitched across a morning's rebuild passes the
//!   check while mixing two different engines row by row -- exactly the
//!   "version drift across a merged board" risk the document names and then
//!   leaves open ("and probably also the git SHA if the binary carries one").
//!   This port closes it: the gate compares the BLAKE3 of the binary, supplied
//!   by the caller and stamped on the row under [`ENGINE_KEY`]. `ver` is still
//!   written to every row -- the artifacts, the archive and the standings all
//!   read it -- it simply no longer decides anything on its own. A row that
//!   carries no engine stamp (every raw written before this port) is refused
//!   rather than fallen back to the version string: it costs a re-run, never a
//!   wrong number. The same is true of a stitched row whose engine stamp did
//!   not survive being rewritten -- it re-runs.
//!
//! * **The run parameters.** budget, mode, jobs and threads must match
//!   EXACTLY. These four plus the engine are what a row's identity is made of;
//!   the manifest's `[[board]]` table exists to hold precisely this tuple. Two
//!   Python subtleties are reproduced rather than tidied, because tidying
//!   either one changes which rows are reused: an absent or empty `mode` reads
//!   as `"auto"` (`(r.get("mode") or "auto")`), and `threads` is compared as a
//!   STRING (`str(r.get("threads")) != str(THREADS)`), which makes the JSON
//!   number `2` and the JSON string `"2"` the same value. Real raws carry the
//!   string, because the runner passes the CLI argument through unconverted.
//!
//! * **`resumed_clean` -- a prior judgment IS the record.** A row stitched by
//!   an earlier pass is reused on its stamp alone. This is not laziness: by the
//!   time a third pass runs, the conditions file that justified the second
//!   pass's judgment has been overwritten by the current pass's watcher. The
//!   judgment is the only surviving evidence, and re-deriving it is impossible,
//!   not merely expensive. It short-circuits the CONTENTION half only; the
//!   engine and parameter gates still run first, so a stitched row from another
//!   build is still refused.
//!
//! * **The straddle rule, which costs nothing.** A row reaches the raw only
//!   once its run finished somehow -- `end_ts` is stamped on every exit path.
//!   A pass killed mid-instance therefore leaves a row with no `end_ts` BY
//!   CONSTRUCTION, and `PER-INSTANCE-RETRY.md`'s "instances that straddle the
//!   resume boundary ... treat as needs re-run, not clean by omission" falls
//!   out of the missing-stamp refusal for free.
//!
//! * **No timeline at all.** The watcher only began writing a per-sample
//!   timeline at 0.25 (step 1 of the design record). SEVENTY-TWO of the 76
//!   conditions files on this box are rollups only. A rollup cannot answer "was
//!   instance N running during the bad window?", so it answers nothing: no
//!   timeline means no contention-based reuse, never "clean by omission".
//!
//! * **PER-SAMPLE, NEVER THE RUN MEDIAN.** The design record's own care point:
//!   "an instance's window is clean only if every sample overlapping it was
//!   under threshold, not just the run's overall median. Getting this wrong
//!   silently reintroduces the exact failure mode the watcher exists to
//!   prevent." Every fixture in `tests/fixtures/conditions/timeline-*.json`
//!   carries the overall verdict `clean` AND a stretch of samples far over the
//!   line -- `timeline-numeric-opt.json` has 53 of them. A median-based gate
//!   would reuse contention-suppressed rows out of a file the watcher called
//!   clean.
//!
//! * **Unknown is not clean.** A sample whose `competitors_total` is null told
//!   us nothing about that moment, and a window is only clean if every moment
//!   in it was observed to be.
//!
//! The threshold itself is [`SAMPLE_CLEAN_PCPU`], the one the throttle state
//! machine reads, so the "is this box busy" line cannot drift between the
//! watcher and the gate. For the same reason [`Conditions::from_document`]
//! bridges to `artifact::conditions`, which models the whole file: two readers
//! of one document is the shape of half the incidents in this project, and a
//! test holds them against each other on every real conditions file on the box.

use crate::monitor::SAMPLE_CLEAN_PCPU;
use crucible_publish::raw::{Instance, RawRow};
use std::collections::BTreeMap;
use std::path::Path;

/// The row column carrying the BLAKE3 of the binary that measured it.
///
/// A new column, so it arrives in [`RawRow::extra`]. The writer must keep it
/// there across a rewrite; if it is lost, the affected rows re-run.
pub const ENGINE_KEY: &str = "engine";

/// The watcher's default sampling interval, and the padding on each side of an
/// instance's window.
///
/// Python: `float(cond.get("interval") or 20)` -- note `or`, so a recorded
/// interval of zero falls back to this too.
pub const DEFAULT_INTERVAL_SECS: f64 = 20.0;

/// An instance label as a map key.
///
/// The int/string split is part of the row contract (`raw.rs`: first-group-only
/// labels once collapsed `ipc2026-numeric`'s 320 rows onto 288 keys), so the
/// key preserves it instead of flattening both to a string where the integer
/// `3` and the label `"3"` would join.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum InstanceKey {
    Num(u64),
    Parts(String),
}

impl From<&Instance> for InstanceKey {
    fn from(i: &Instance) -> Self {
        match i {
            Instance::Num(n) => InstanceKey::Num(*n),
            Instance::Parts(s) => InstanceKey::Parts(s.clone()),
        }
    }
}

impl std::fmt::Display for InstanceKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstanceKey::Num(n) => write!(f, "{n}"),
            InstanceKey::Parts(s) => write!(f, "{s}"),
        }
    }
}

/// `(ipc, variant, instance)` -- the Python's resume key, typed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RowKey {
    pub ipc: Option<String>,
    pub variant: String,
    pub instance: InstanceKey,
}

impl RowKey {
    pub fn of(r: &RawRow) -> Self {
        RowKey {
            ipc: r.ipc.clone(),
            variant: r.variant.clone(),
            instance: (&r.instance).into(),
        }
    }
}

impl std::fmt::Display for RowKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.ipc {
            Some(i) => write!(f, "{i}/{}/{}", self.variant, self.instance),
            None => write!(f, "{}/{}", self.variant, self.instance),
        }
    }
}

/// One row of the watcher's timeline: `[epoch_ts, idle_pct, competitors_total]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineSample {
    pub at: f64,
    /// Whole-machine idle, kept for the record. The gate does NOT read it --
    /// see `monitor::sample` on why the verdict moved off idle at 0.24.
    pub idle_pct: Option<f64>,
    /// `None` means the sample failed to observe the box. Not clean.
    pub competitors_total: Option<f64>,
}

/// A prior pass's `.conditions.json`, reduced to what the gate reads.
#[derive(Debug, Clone)]
pub struct Conditions {
    pub interval: f64,
    /// In FILE order, exactly as the watcher appended it. The span check reads
    /// the first and last entries, not the minimum and maximum, because that is
    /// what `ipc67.py` does -- and the watcher only ever appends.
    pub timeline: Vec<TimelineSample>,
}

impl Default for Conditions {
    fn default() -> Self {
        Conditions {
            interval: DEFAULT_INTERVAL_SECS,
            timeline: Vec::new(),
        }
    }
}

impl Conditions {
    /// Read a conditions file. `None` means "this file cannot justify reusing
    /// anything" -- missing, unreadable, not JSON, or carrying a timeline entry
    /// this gate refuses to guess about.
    ///
    /// The Python degrades a missing file to an empty resume map (nothing is
    /// reused, including rows that carry `resumed_clean`), and that is the
    /// behaviour a `None` here produces in [`Resume::load`].
    pub fn load(path: &Path) -> Option<Self> {
        Self::parse(&std::fs::read_to_string(path).ok()?)
    }

    /// Parse from text already in hand.
    ///
    /// Entries are filtered the way `ipc67.py:559-560` filters them: an entry
    /// that is not an array, is shorter than three elements, or has a null
    /// timestamp is DROPPED. An entry whose timestamp or load is present but
    /// not a number is different -- Python would raise a `TypeError` out of
    /// `load_resume` and take the whole pass down with it. Rather than port a
    /// crash or, worse, silently drop a sample that might have been the dirty
    /// one, the whole file is refused: fail closed, and the pass re-measures.
    pub fn parse(text: &str) -> Option<Self> {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        let obj = v.as_object()?;
        // Python: `float(cond.get("interval") or 20)`. Zero is falsy there, so
        // a zero interval means the default rather than no padding at all.
        let interval = match obj.get("interval").and_then(|i| i.as_f64()) {
            Some(i) if i != 0.0 => i,
            _ => DEFAULT_INTERVAL_SECS,
        };
        let mut timeline = Vec::new();
        if let Some(rows) = obj.get("timeline") {
            // A `timeline` key of the wrong shape is not a timeline. Python's
            // `(cond.get("timeline") or [])` would iterate a string character
            // by character and drop every one; refusing is the same outcome
            // reached honestly.
            let rows = match rows {
                serde_json::Value::Array(a) => a,
                serde_json::Value::Null => return Some(Conditions { interval, timeline }),
                _ => return None,
            };
            for row in rows {
                let Some(cols) = row.as_array() else {
                    continue; // not a list: dropped, as in Python
                };
                if cols.len() < 3 || cols[0].is_null() {
                    continue; // short, or no timestamp: dropped, as in Python
                }
                let at = cols[0].as_f64()?; // non-numeric: refuse the file
                let idle_pct = match &cols[1] {
                    serde_json::Value::Null => None,
                    other => Some(other.as_f64()?),
                };
                let competitors_total = match &cols[2] {
                    serde_json::Value::Null => None, // observed nothing: not clean
                    other => Some(other.as_f64()?),
                };
                timeline.push(TimelineSample {
                    at,
                    idle_pct,
                    competitors_total,
                });
            }
        }
        Some(Conditions { interval, timeline })
    }

    /// The same reduction from an already-parsed conditions DOCUMENT.
    ///
    /// `artifact::conditions::Conditions` is the byte-identical model of the
    /// whole file -- the thing that has to round-trip. This type is the gate's
    /// view of it: three columns, one filter, no verdict. Both readers exist on
    /// purpose, and this bridge is what stops that being a drift risk. A caller
    /// holding the document must come through here rather than re-reading the
    /// bytes, and `the_two_conditions_readers_agree` proves the two agree on
    /// every real file on this box.
    ///
    /// The document reader is the STRICTER of the two -- it refuses a file
    /// whose keys are not all present, where this one would shrug and read the
    /// timeline. That direction is safe: a refusal re-measures.
    pub fn from_document(doc: &crate::artifact::conditions::Conditions) -> Self {
        let interval = match doc.interval.as_option().and_then(|n| n.as_f64()) {
            Some(i) if i != 0.0 => i,
            _ => DEFAULT_INTERVAL_SECS,
        };
        let timeline = doc
            .timeline
            .as_option()
            .map(|rows| {
                rows.iter()
                    // A null timestamp is dropped, exactly as in Python's own
                    // filter -- and note that a null LOAD is not, because that
                    // one is a refusal rather than an omission.
                    .filter_map(|e| {
                        e.at().map(|at| TimelineSample {
                            at,
                            idle_pct: e.idle_pct(),
                            competitors_total: e.competitors_total(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Conditions { interval, timeline }
    }

    /// A rollup with no per-sample timeline -- 72 of the 76 files on this box.
    pub fn has_timeline(&self) -> bool {
        !self.timeline.is_empty()
    }
}

/// What this pass is measuring under. Every field is compared exactly.
#[derive(Debug, Clone)]
pub struct RunParams {
    /// BLAKE3 of the planner binary. The real engine gate; see the header.
    pub engine: String,
    /// `ff --version`, still stamped on every row. Compared only as a
    /// corroboration when both sides carry one -- it can refuse, never admit.
    pub ver: Option<String>,
    pub budget_secs: f64,
    /// `ff --mode` passthrough. `None` and `Some("")` both read as `"auto"`.
    pub mode: Option<String>,
    pub jobs: u32,
    /// The CLI argument verbatim, because the comparison is `str(threads)`.
    pub threads: String,
}

impl RunParams {
    /// Python's `(MODE or "auto")`: an empty string is falsy there too.
    pub fn mode_str(&self) -> &str {
        match self.mode.as_deref() {
            Some("") | None => "auto",
            Some(m) => m,
        }
    }
}

/// Why a prior row may not stand. Every variant is a re-run.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Reject {
    #[error("line is not JSON (a truncated tail from a killed pass)")]
    Unparseable,
    #[error("line is JSON but not a result row: {0}")]
    Malformed(String),
    #[error(
        "no `{ENGINE_KEY}` stamp: written before the engine hash existed, or \
         the stamp was lost in a rewrite"
    )]
    EngineUnstamped,
    #[error("measured by a different binary (row {row}, want {want})")]
    EngineMismatch { row: String, want: String },
    #[error("engine matches but `ver` disagrees (row {row}, want {want})")]
    VerMismatch { row: String, want: String },
    #[error("budget {row:?}s, want {want}s")]
    BudgetMismatch { row: Option<f64>, want: f64 },
    #[error("mode `{row}`, want `{want}`")]
    ModeMismatch { row: String, want: String },
    #[error("threads `{row}`, want `{want}`")]
    ThreadsMismatch { row: String, want: String },
    #[error("jobs {row:?}, want {want}")]
    JobsMismatch { row: Option<u32>, want: u32 },
    #[error("no start_ts")]
    NoStartTs,
    #[error("no end_ts: the pass was killed while this instance was running")]
    NoEndTs,
    #[error("the prior conditions file is a rollup with no per-sample timeline")]
    NoTimeline,
    #[error("window [{start}, {end}] is not inside the sampled span [{first}, {last}]")]
    WindowOutsideSpan {
        start: f64,
        end: f64,
        first: f64,
        last: f64,
    },
    #[error("no sample overlaps window [{start}, {end}]")]
    NoOverlappingSamples { start: f64, end: f64 },
    #[error("sample at {at} measured {total} competing (>= {SAMPLE_CLEAN_PCPU})")]
    Contended { at: f64, total: f64 },
    #[error("sample at {at} observed no competitor load: unknown is not clean")]
    UnknownLoad { at: f64 },
}

impl Reject {
    /// A stable slug for counting refusals in the operator log.
    pub fn kind(&self) -> &'static str {
        match self {
            Reject::Unparseable => "unparseable",
            Reject::Malformed(_) => "malformed",
            Reject::EngineUnstamped => "engine-unstamped",
            Reject::EngineMismatch { .. } => "engine-mismatch",
            Reject::VerMismatch { .. } => "ver-mismatch",
            Reject::BudgetMismatch { .. } => "budget-mismatch",
            Reject::ModeMismatch { .. } => "mode-mismatch",
            Reject::ThreadsMismatch { .. } => "threads-mismatch",
            Reject::JobsMismatch { .. } => "jobs-mismatch",
            Reject::NoStartTs => "no-start-ts",
            Reject::NoEndTs => "no-end-ts",
            Reject::NoTimeline => "no-timeline",
            Reject::WindowOutsideSpan { .. } => "window-outside-span",
            Reject::NoOverlappingSamples { .. } => "no-overlapping-samples",
            Reject::Contended { .. } => "contended",
            Reject::UnknownLoad { .. } => "unknown-load",
        }
    }
}

/// Python's `str()` of whatever `threads` held.
///
/// `str(2)` and `str("2")` are both `"2"`, which is why a row carrying the JSON
/// number and a caller carrying the string agree. `str(None)` is `"None"`, and
/// nothing a caller can pass equals that, so an absent `threads` refuses.
fn py_str_threads(v: Option<&serde_json::Value>) -> String {
    match v {
        None | Some(serde_json::Value::Null) => "None".to_string(),
        Some(serde_json::Value::String(s)) => s.clone(),
        // `arbitrary_precision` keeps the original token, so `2` prints as `2`
        // and `2.0` as `2.0` -- the same two strings Python's `str()` gives.
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => {
            if *b {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        Some(other) => other.to_string(),
    }
}

/// May this prior row stand? On `Ok`, the row is returned STITCHED -- stamped
/// `resumed_clean` so the next pass can honour its judgment, and so the board's
/// `.md` can say it was stitched.
pub fn judge(row: &RawRow, cond: &Conditions, want: &RunParams) -> Result<RawRow, Reject> {
    // The engine first, and on the hash rather than the version string. Order
    // matters for the message a rejected row reports, and "measured by a
    // different binary" is the one an operator most needs to see.
    let stamped = row
        .extra
        .get(ENGINE_KEY)
        .and_then(|v| v.as_str())
        .ok_or(Reject::EngineUnstamped)?;
    if stamped != want.engine {
        return Err(Reject::EngineMismatch {
            row: stamped.to_string(),
            want: want.engine.clone(),
        });
    }
    // Corroboration only. The hash already proves the binary, so this can never
    // be the reason a row is ADMITTED -- but a row whose two stamps contradict
    // each other has been through something, and fail-closed is the house rule.
    if let (Some(rv), Some(wv)) = (row.ver.as_deref(), want.ver.as_deref()) {
        if rv != wv {
            return Err(Reject::VerMismatch {
                row: rv.to_string(),
                want: wv.to_string(),
            });
        }
    }
    // Budgets on this box are whole seconds (60, 300, 30) and exact in binary
    // floating point, so equality is the right comparison and not a tolerance.
    // A row with NO budget stamp predates 0.23 and refuses here.
    if row.budget != Some(want.budget_secs) {
        return Err(Reject::BudgetMismatch {
            row: row.budget,
            want: want.budget_secs,
        });
    }
    let row_mode = match row.mode.as_deref() {
        Some("") | None => "auto",
        Some(m) => m,
    };
    if row_mode != want.mode_str() {
        return Err(Reject::ModeMismatch {
            row: row_mode.to_string(),
            want: want.mode_str().to_string(),
        });
    }
    // Threads before jobs: Python evaluates `str(threads) != str(THREADS) or
    // jobs != JOBS` left to right, so this is the reason it would report.
    let row_threads = py_str_threads(row.threads.as_ref());
    if row_threads != want.threads {
        return Err(Reject::ThreadsMismatch {
            row: row_threads,
            want: want.threads.clone(),
        });
    }
    if row.jobs != Some(want.jobs) {
        return Err(Reject::JobsMismatch {
            row: row.jobs,
            want: want.jobs,
        });
    }

    // A prior stitch's judgment IS the record: the conditions file that
    // justified it was overwritten passes ago, so this cannot be re-derived.
    if row.resumed_clean {
        return Ok(row.clone());
    }

    let start = row.start_ts.ok_or(Reject::NoStartTs)?;
    // No end_ts means the runner was killed while this instance was running --
    // the straddle rule, falling out of the stamp rather than needing a check.
    let end = row.end_ts.ok_or(Reject::NoEndTs)?;
    if !cond.has_timeline() {
        return Err(Reject::NoTimeline);
    }

    let first = cond.timeline[0].at;
    let last = cond.timeline[cond.timeline.len() - 1].at;
    if start < first - cond.interval || end > last + cond.interval {
        return Err(Reject::WindowOutsideSpan {
            start,
            end,
            first,
            last,
        });
    }

    // One interval of padding each side: a sample taken just before the window
    // opened describes the box the instance was about to run on.
    let lo = start - cond.interval;
    let hi = end + cond.interval;
    let mut seen = false;
    for s in &cond.timeline {
        if s.at < lo || s.at > hi {
            continue;
        }
        seen = true;
        match s.competitors_total {
            None => return Err(Reject::UnknownLoad { at: s.at }),
            Some(t) if t >= SAMPLE_CLEAN_PCPU => {
                return Err(Reject::Contended { at: s.at, total: t })
            }
            Some(_) => {}
        }
    }
    if !seen {
        // Inside the span but between samples: the watcher missed a stretch.
        // Nothing observed the window, so nothing can vouch for it.
        return Err(Reject::NoOverlappingSamples { start, end });
    }

    let mut out = row.clone();
    out.resumed_clean = true;
    // ...and say so on disk. Without the presence flag the re-serialized row
    // would omit the key and the NEXT pass would re-derive a judgment whose
    // evidence is gone.
    out.present.resumed_clean = true;
    Ok(out)
}

/// A row that will be re-measured, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Rejected {
    /// 1-based line in the prior raw, so an operator can go and look at it.
    pub line: usize,
    pub key: Option<RowKey>,
    pub why: Reject,
}

/// Why no prior row was even considered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disabled {
    /// No `--resume-raw`/`--resume-conditions` pair: a first pass.
    NotRequested,
    NoPriorRaw,
    /// Missing, unreadable, or carrying a timeline entry the gate refuses to
    /// guess about. Nothing is reused, not even rows already judged clean.
    NoPriorConditions,
}

/// The prior pass's rows that pass the gate, keyed by instance.
#[derive(Debug, Default)]
pub struct Resume {
    reusable: BTreeMap<RowKey, RawRow>,
    rejected: Vec<Rejected>,
    disabled: Option<Disabled>,
}

impl Resume {
    /// A first pass: no prior artifacts, and nothing to explain.
    pub fn none() -> Self {
        Resume {
            disabled: Some(Disabled::NotRequested),
            ..Default::default()
        }
    }

    /// Read the prior raw and conditions and judge every row.
    ///
    /// Either file missing degrades to "reuse nothing", never to an error: the
    /// pass simply measures everything, which is what it would have done before
    /// this module existed.
    pub fn load(raw: &Path, conditions: &Path, want: &RunParams) -> Self {
        let Some(cond) = Conditions::load(conditions) else {
            return Resume {
                disabled: Some(Disabled::NoPriorConditions),
                ..Default::default()
            };
        };
        let Ok(text) = std::fs::read_to_string(raw) else {
            return Resume {
                disabled: Some(Disabled::NoPriorRaw),
                ..Default::default()
            };
        };
        Self::judge_lines(&text, &cond, want)
    }

    /// The gate over a prior raw's text. Split out so the branch-by-branch
    /// tests never touch the filesystem.
    pub fn judge_lines(text: &str, cond: &Conditions, want: &RunParams) -> Self {
        let mut out = Resume::default();
        for (i, line) in text.lines().enumerate() {
            let line_no = i + 1;
            // Python catches `ValueError` on EVERY line, not only the last:
            // a killed pass leaves a truncated tail, and a blank line in the
            // middle is no more readable than a half-written one.
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                out.rejected.push(Rejected {
                    line: line_no,
                    key: None,
                    why: Reject::Unparseable,
                });
                continue;
            };
            let present = value
                .as_object()
                .map(crucible_publish::Present::of)
                .unwrap_or_default();
            let mut row: RawRow = match serde_json::from_value(value) {
                Ok(r) => r,
                Err(e) => {
                    // JSON, but missing a column a result row always has.
                    // Python would sail past this and die later on `r["solved"]`.
                    out.rejected.push(Rejected {
                        line: line_no,
                        key: None,
                        why: Reject::Malformed(e.to_string()),
                    });
                    continue;
                }
            };
            row.present = present;
            let key = RowKey::of(&row);
            match judge(&row, cond, want) {
                // Last duplicate wins, as in Python's `out[k] = r`.
                Ok(stitched) => {
                    out.reusable.insert(key, stitched);
                }
                Err(why) => out.rejected.push(Rejected {
                    line: line_no,
                    key: Some(key),
                    why,
                }),
            }
        }
        out
    }

    pub fn get(&self, key: &RowKey) -> Option<&RawRow> {
        self.reusable.get(key)
    }

    pub fn len(&self) -> usize {
        self.reusable.len()
    }

    pub fn is_empty(&self) -> bool {
        self.reusable.is_empty()
    }

    pub fn rejected(&self) -> &[Rejected] {
        &self.rejected
    }

    pub fn disabled(&self) -> Option<Disabled> {
        self.disabled
    }

    /// Refusals by reason. The operator line a stitched board should print --
    /// "reused 318, re-running 82 (61 contended, 21 no-end-ts)" -- so a gate
    /// that is refusing everything for one silly reason is visible immediately.
    pub fn reject_counts(&self) -> BTreeMap<&'static str, usize> {
        let mut m = BTreeMap::new();
        for r in &self.rejected {
            *m.entry(r.why.kind()).or_insert(0) += 1;
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURES: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/conditions"
    );

    const ENGINE: &str = "b3:9f1c0d5a";

    fn want() -> RunParams {
        RunParams {
            engine: ENGINE.to_string(),
            ver: Some("ff 0.25.0".to_string()),
            budget_secs: 60.0,
            mode: None,
            jobs: 2,
            threads: "1".to_string(),
        }
    }

    fn fixture(name: &str) -> Conditions {
        Conditions::load(&Path::new(FIXTURES).join(name))
            .unwrap_or_else(|| panic!("{name} should parse"))
    }

    /// A row with every stamp the gate wants, plus whatever the test overrides.
    fn row_json(extra: &str) -> String {
        format!(
            r#"{{"ipc":"ipc2023","variant":"numeric-2023","instance":7,"solved":true,
                 "time":3.5,"metric":null,"length":12,"val":true,"notes":null,
                 "budget":60,"ver":"ff 0.25.0","mode":"auto","jobs":2,"threads":"1",
                 "start_ts":1787455172.4,"end_ts":1787455256.1,"engine":"{ENGINE}"{extra}}}"#
        )
        .replace('\n', "")
    }

    fn parse(json: &str) -> RawRow {
        let v: serde_json::Value = serde_json::from_str(json).expect("test row is JSON");
        let present = crucible_publish::Present::of(v.as_object().unwrap());
        let mut r: RawRow = serde_json::from_value(v).expect("test row is a RawRow");
        r.present = present;
        r
    }

    fn judged(json: &str, cond: &Conditions) -> Result<RawRow, Reject> {
        judge(&parse(json), cond, &want())
    }

    /// The window the whole battery reuses: five samples of
    /// `timeline-numeric-opt.json`, every one of them clean.
    #[test]
    fn a_clean_window_inside_the_span_is_reused_and_stamped() {
        let out = judged(&row_json(""), &fixture("timeline-numeric-opt.json"))
            .expect("a clean window is reusable");
        assert!(out.resumed_clean, "the stitch must be recorded on the row");
        assert!(
            out.present.resumed_clean,
            "and must survive being written back out, or the NEXT pass \
             re-derives a judgment whose evidence is gone"
        );
    }

    /// THE CARE POINT of PER-INSTANCE-RETRY.md, on real data: this fixture's
    /// whole-run verdict is `clean` and 53 of its samples are over the line. A
    /// run-median gate would reuse a row measured inside one of them.
    #[test]
    fn per_sample_not_run_median() {
        let cond = fixture("timeline-numeric-opt.json");
        // The file the watcher itself called clean.
        assert!(cond
            .timeline
            .iter()
            .any(|s| s.competitors_total.unwrap() >= SAMPLE_CLEAN_PCPU));
        let dirty = row_json(r#","x":0"#)
            .replace("1787455172.4", "1787453904.4")
            .replace("1787455256.1", "1787453906.4");
        let err = judged(&dirty, &cond).expect_err("a window over a dirty sample must re-run");
        assert!(
            matches!(err, Reject::Contended { total, .. } if total >= SAMPLE_CLEAN_PCPU),
            "{err:?}"
        );
    }

    /// THE STRENGTHENING. Both builds report `ff 0.25.0`, which is the whole
    /// problem: the Python's version-string gate admits this row and stitches
    /// two engines into one board.
    #[test]
    fn a_different_build_of_the_same_version_is_refused() {
        let other = row_json("").replace(ENGINE, "b3:00000000");
        let err = judged(&other, &fixture("timeline-numeric-opt.json")).unwrap_err();
        assert!(matches!(err, Reject::EngineMismatch { .. }), "{err:?}");
    }

    /// Every raw written before this port carries no hash. Falling back to the
    /// version string would reopen exactly what the hash closed, so it re-runs.
    #[test]
    fn a_row_with_no_engine_stamp_is_refused_not_fallen_back_on() {
        let bare = row_json("").replace(&format!(r#","engine":"{ENGINE}""#), "");
        assert!(bare.contains(r#""ver":"ff 0.25.0""#), "ver is still there");
        let err = judged(&bare, &fixture("timeline-numeric-opt.json")).unwrap_err();
        assert_eq!(err, Reject::EngineUnstamped);
    }

    /// The hash decides, but two stamps that contradict each other mean
    /// something happened to this row, and the house rule is fail closed.
    #[test]
    fn contradicting_version_stamps_are_refused() {
        let odd = row_json("").replace("ff 0.25.0", "ff 0.24.3");
        let err = judged(&odd, &fixture("timeline-numeric-opt.json")).unwrap_err();
        assert!(matches!(err, Reject::VerMismatch { .. }), "{err:?}");
    }

    /// A pre-0.23 row has no budget stamp; a 30 s row on a 60 s board has the
    /// wrong one. The tier move makes both real.
    #[test]
    fn budget_must_match_and_an_unstamped_row_refuses() {
        let cond = fixture("timeline-numeric-opt.json");
        let thirty = row_json("").replace(r#""budget":60"#, r#""budget":30"#);
        assert!(matches!(
            judged(&thirty, &cond).unwrap_err(),
            Reject::BudgetMismatch { row: Some(_), .. }
        ));
        let unstamped = row_json("").replace(r#""budget":60,"#, "");
        assert_eq!(
            judged(&unstamped, &cond).unwrap_err(),
            Reject::BudgetMismatch {
                row: None,
                want: 60.0
            }
        );
    }

    /// `(r.get("mode") or "auto")`: absent, null and empty all read as `auto`,
    /// so a row from a driver that did not pass `--mode` matches one that
    /// passed nothing.
    #[test]
    fn an_unset_mode_reads_as_auto_on_both_sides() {
        let cond = fixture("timeline-numeric-opt.json");
        for spelling in [r#""mode":null"#, r#""mode":"""#, r#""mode":"auto""#] {
            let r = row_json("").replace(r#""mode":"auto""#, spelling);
            assert!(judged(&r, &cond).is_ok(), "{spelling} should read as auto");
        }
        let optimal = row_json("").replace(r#""mode":"auto""#, r#""mode":"optimal""#);
        assert!(matches!(
            judged(&optimal, &cond).unwrap_err(),
            Reject::ModeMismatch { .. }
        ));
    }

    /// `str(threads)`: the JSON number 2 and the JSON string "2" are the same
    /// value to the Python gate, and real raws carry the string only because
    /// the runner passes the CLI argument through unconverted.
    #[test]
    fn threads_is_compared_as_a_string() {
        let cond = fixture("timeline-numeric-opt.json");
        let mut w = want();
        w.threads = "4".to_string();
        for spelling in [r#""threads":4"#, r#""threads":"4""#] {
            let r = parse(&row_json("").replace(r#""threads":"1""#, spelling));
            assert!(
                judge(&r, &cond, &w).is_ok(),
                "{spelling} should equal \"4\""
            );
        }
        let two = parse(&row_json("").replace(r#""threads":"1""#, r#""threads":"2""#));
        assert!(matches!(
            judge(&two, &cond, &w).unwrap_err(),
            Reject::ThreadsMismatch { .. }
        ));
        // `str(None)` is "None", which no caller can ask for.
        let absent = parse(&row_json("").replace(r#""threads":"1","#, ""));
        assert!(matches!(
            judge(&absent, &cond, &w).unwrap_err(),
            Reject::ThreadsMismatch { row, .. } if row == "None"
        ));
    }

    /// jobs is part of row identity -- a row measured two-at-a-time is not the
    /// same measurement as one measured alone, which is the mco wall-clock rule
    /// seen from the other end.
    #[test]
    fn jobs_must_match_exactly() {
        let cond = fixture("timeline-numeric-opt.json");
        let one = row_json("").replace(r#""jobs":2"#, r#""jobs":1"#);
        assert!(matches!(
            judged(&one, &cond).unwrap_err(),
            Reject::JobsMismatch {
                row: Some(1),
                want: 2
            }
        ));
        let absent = row_json("").replace(r#""jobs":2,"#, "");
        assert!(matches!(
            judged(&absent, &cond).unwrap_err(),
            Reject::JobsMismatch { row: None, .. }
        ));
    }

    /// A prior stitch's judgment IS the record: the conditions file that
    /// justified it is long overwritten. Proven against the rollup fixture,
    /// which cannot justify anything by itself.
    #[test]
    fn a_prior_judgment_stands_without_a_timeline() {
        let cond = fixture("rollup-only.json");
        assert!(!cond.has_timeline(), "72 of 76 real files look like this");
        let plain = row_json("");
        assert_eq!(judged(&plain, &cond).unwrap_err(), Reject::NoTimeline);
        let stitched = row_json(r#","resumed_clean":true"#);
        assert!(judged(&stitched, &cond).is_ok());
    }

    /// ...but only the contention half. A stitched row from another binary is
    /// still a stitched row from another binary.
    #[test]
    fn a_prior_judgment_does_not_excuse_a_parameter_mismatch() {
        let cond = fixture("rollup-only.json");
        let stitched = row_json(r#","resumed_clean":true"#).replace(ENGINE, "b3:deadbeef");
        assert!(matches!(
            judged(&stitched, &cond).unwrap_err(),
            Reject::EngineMismatch { .. }
        ));
    }

    /// THE STRADDLE RULE. A pass killed mid-instance never wrote that row's
    /// `end_ts`, so "needs re-run, not clean by omission" costs no extra check.
    #[test]
    fn a_row_from_a_pass_killed_mid_instance_has_no_end_ts() {
        let cond = fixture("timeline-numeric-opt.json");
        let killed = row_json("").replace(r#""end_ts":1787455256.1"#, r#""end_ts":null"#);
        assert_eq!(judged(&killed, &cond).unwrap_err(), Reject::NoEndTs);
        let no_start = row_json("").replace(r#""start_ts":1787455172.4"#, r#""start_ts":null"#);
        assert_eq!(judged(&no_start, &cond).unwrap_err(), Reject::NoStartTs);
    }

    /// The two DEGRADED records on this box are both rollups, and a rollup
    /// cannot answer "was instance N running during the bad window?".
    #[test]
    fn a_rollup_only_conditions_file_reuses_nothing_on_contention() {
        for name in [
            "rollup-only.json",
            "degraded-old-idle-rule-mco-t4.json",
            "degraded-old-idle-rule-mco-t8.json",
        ] {
            let cond = fixture(name);
            assert_eq!(cond.interval, DEFAULT_INTERVAL_SECS, "{name}");
            assert_eq!(
                judged(&row_json(""), &cond).unwrap_err(),
                Reject::NoTimeline
            );
        }
    }

    /// A window outside the sampled span was measured while nothing was
    /// watching -- before the watcher started, or after it was killed.
    #[test]
    fn a_window_outside_the_sampled_span_is_refused() {
        let cond = fixture("timeline-numeric-opt.json");
        let early = row_json("")
            .replace("1787455172.4", "1787453000.0")
            .replace("1787455256.1", "1787453100.0");
        assert!(matches!(
            judged(&early, &cond).unwrap_err(),
            Reject::WindowOutsideSpan { .. }
        ));
        let late = row_json("")
            .replace("1787455172.4", "1787461050.0")
            .replace("1787455256.1", "1787461100.0");
        assert!(matches!(
            judged(&late, &cond).unwrap_err(),
            Reject::WindowOutsideSpan { .. }
        ));
    }

    /// Inside the span but between samples: the watcher missed a stretch, so
    /// nothing can vouch for the window.
    #[test]
    fn a_window_with_no_overlapping_sample_is_refused() {
        // Two samples an hour apart, both clean; the instance ran in the gap.
        let cond = Conditions::parse(
            r#"{"interval": 20, "timeline": [[1000.0, 90.0, 1.0], [4600.0, 90.0, 1.0]]}"#,
        )
        .unwrap();
        let r = row_json("")
            .replace("1787455172.4", "2000.0")
            .replace("1787455256.1", "2100.0");
        assert!(matches!(
            judged(&r, &cond).unwrap_err(),
            Reject::NoOverlappingSamples { .. }
        ));
    }

    /// A sample that failed to observe the box told us nothing about that
    /// moment, and a window is clean only if every moment in it was observed.
    #[test]
    fn a_sample_with_unknown_load_is_not_clean() {
        let cond = Conditions::parse(
            r#"{"interval": 20, "timeline": [[1000.0, 90.0, 1.0], [1020.0, null, null],
                                             [1040.0, 90.0, 1.0]]}"#,
        )
        .unwrap();
        let r = row_json("")
            .replace("1787455172.4", "1010.0")
            .replace("1787455256.1", "1030.0");
        assert!(matches!(
            judged(&r, &cond).unwrap_err(),
            Reject::UnknownLoad { .. }
        ));
    }

    /// The clean line is the throttle's line, and it is EXCLUSIVE: 24.9 passes,
    /// 25.0 does not. One shared constant, so the two cannot drift.
    #[test]
    fn the_clean_line_is_exclusive_and_shared_with_the_throttle() {
        let r = row_json("")
            .replace("1787455172.4", "1010.0")
            .replace("1787455256.1", "1030.0");
        let just_under = Conditions::parse(
            r#"{"interval": 20, "timeline": [[1000.0, 90.0, 24.9], [1040.0, 90.0, 24.9]]}"#,
        )
        .unwrap();
        assert!(judged(&r, &just_under).is_ok());
        let on_the_line = Conditions::parse(
            r#"{"interval": 20, "timeline": [[1000.0, 90.0, 24.9], [1040.0, 90.0, 25.0]]}"#,
        )
        .unwrap();
        assert!(matches!(
            judged(&r, &on_the_line).unwrap_err(),
            Reject::Contended { total, .. } if total == 25.0
        ));
    }

    /// The padding is ONE interval each side and INCLUSIVE at the boundary: a
    /// sample taken just before the window opened describes the box the
    /// instance was about to run on.
    #[test]
    fn the_window_is_padded_one_interval_each_side_inclusively() {
        let r = row_json("")
            .replace("1787455172.4", "1100.0")
            .replace("1787455256.1", "1100.0");
        // Dirty sample exactly one interval before the window: still counts.
        let edge = Conditions::parse(
            r#"{"interval": 20, "timeline": [[1080.0, 40.0, 90.0], [1120.0, 90.0, 1.0]]}"#,
        )
        .unwrap();
        assert!(matches!(
            judged(&r, &edge).unwrap_err(),
            Reject::Contended { at, .. } if at == 1080.0
        ));
        // One tick further out and it is somebody else's problem.
        let clear = Conditions::parse(
            r#"{"interval": 20, "timeline": [[1079.9, 40.0, 90.0], [1120.0, 90.0, 1.0]]}"#,
        )
        .unwrap();
        assert!(judged(&r, &clear).is_ok());
    }

    /// A recorded interval of zero is falsy in Python, so it means the default
    /// rather than no padding at all.
    #[test]
    fn a_zero_interval_falls_back_to_the_default() {
        let c = Conditions::parse(r#"{"interval": 0, "timeline": []}"#).unwrap();
        assert_eq!(c.interval, DEFAULT_INTERVAL_SECS);
        let c = Conditions::parse(r#"{"timeline": []}"#).unwrap();
        assert_eq!(c.interval, DEFAULT_INTERVAL_SECS);
    }

    /// Python drops malformed timeline entries; it CRASHES on a non-numeric
    /// one. This port refuses the file instead -- dropping the sample could
    /// drop the dirty one, and that is the wrong direction to be wrong in.
    #[test]
    fn a_timeline_entry_the_gate_cannot_read_refuses_the_whole_file() {
        let dropped = Conditions::parse(
            r#"{"interval": 20, "timeline": [[null, 9, 1], [1, 2], "x", [1000.0, 90.0, 1.0]]}"#,
        )
        .expect("Python drops these three and keeps the fourth");
        assert_eq!(dropped.timeline.len(), 1);
        assert!(Conditions::parse(r#"{"timeline": [["not-a-ts", 90.0, 1.0]]}"#).is_none());
        assert!(Conditions::parse(r#"{"timeline": [[1000.0, 90.0, "busy"]]}"#).is_none());
        assert!(Conditions::parse("not json at all").is_none());
    }

    /// Every real conditions file on this box parses, including the four that
    /// carry a timeline and the three that do not.
    #[test]
    fn every_committed_conditions_fixture_parses() {
        let dir = std::fs::read_dir(FIXTURES).expect("fixtures are committed");
        let mut n = 0;
        for e in dir.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let c = Conditions::load(&p).unwrap_or_else(|| panic!("{} should parse", p.display()));
            if p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("timeline-")
            {
                assert!(c.has_timeline(), "{}", p.display());
            }
            n += 1;
        }
        assert_eq!(n, 7, "the fixture set is seven files");
    }

    /// TWO READERS OF ONE FILE is the shape of half the incidents in this
    /// project, so the gate's view and the document model are held against each
    /// other on every real conditions file on this box. If they ever disagree
    /// about an interval or a sample, this fails rather than one of them
    /// quietly reusing a row the other would have refused.
    #[test]
    fn the_two_conditions_readers_agree() {
        for e in std::fs::read_dir(FIXTURES)
            .expect("fixtures are committed")
            .flatten()
        {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let text = std::fs::read_to_string(&p).unwrap();
            let doc =
                crate::artifact::conditions::Conditions::parse(&text, &p.display().to_string())
                    .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            let mine = Conditions::parse(&text).unwrap();
            let bridged = Conditions::from_document(&doc);
            assert_eq!(mine.interval, bridged.interval, "{}", p.display());
            assert_eq!(mine.timeline, bridged.timeline, "{}", p.display());
        }
    }

    /// A truncated tail line from a killed pass is skipped, not fatal -- and it
    /// is COUNTED, so a raw that is mostly rubble is visible rather than
    /// quietly producing a tiny resume map.
    #[test]
    fn a_truncated_tail_line_is_skipped_and_counted() {
        let text = format!("{}\n{{\"ipc\": \"ipc2023\", \"var", row_json(""));
        let r = Resume::judge_lines(&text, &fixture("timeline-numeric-opt.json"), &want());
        assert_eq!(r.len(), 1);
        assert_eq!(r.rejected().len(), 1);
        assert_eq!(r.rejected()[0].why, Reject::Unparseable);
        assert_eq!(r.rejected()[0].line, 2);
    }

    /// JSON, but not a result row. Python sails past this and dies later on
    /// `r["solved"]`; here it re-runs the instance and says why.
    #[test]
    fn a_line_that_is_json_but_not_a_row_is_refused() {
        let r = Resume::judge_lines(
            r#"{"variant": "numeric-2023", "instance": 7}"#,
            &fixture("timeline-numeric-opt.json"),
            &want(),
        );
        assert!(r.is_empty());
        assert!(matches!(r.rejected()[0].why, Reject::Malformed(_)));
    }

    /// `out[k] = r`: the LAST row for an instance wins, which is what a raw
    /// containing a re-measurement of the same instance means.
    #[test]
    fn the_last_row_for_an_instance_wins() {
        let first = row_json(r#","x":1"#);
        let second = row_json("").replace(r#""time":3.5"#, r#""time":9.25"#);
        let r = Resume::judge_lines(
            &format!("{first}\n{second}"),
            &fixture("timeline-numeric-opt.json"),
            &want(),
        );
        assert_eq!(r.len(), 1);
        let key = RowKey::of(&parse(&second));
        assert_eq!(r.get(&key).unwrap().time_secs(), Some(9.25));
    }

    /// The int/string split in `instance` is part of the row contract: the
    /// integer 3 and the label "3" are different instances and must not join.
    #[test]
    fn an_integer_label_and_a_string_label_are_different_keys() {
        let a = parse(&row_json("").replace(r#""instance":7"#, r#""instance":3"#));
        let b = parse(&row_json("").replace(r#""instance":7"#, r#""instance":"3""#));
        assert_ne!(RowKey::of(&a), RowKey::of(&b));
        let m = parse(&row_json("").replace(r#""instance":7"#, r#""instance":"3_10_50_10""#));
        assert_eq!(RowKey::of(&m).instance.to_string(), "3_10_50_10");
    }

    /// A missing prior artifact is a first pass, not an error -- and the reason
    /// is recorded so an operator who EXPECTED a resume can see why they did
    /// not get one.
    #[test]
    fn a_missing_prior_artifact_degrades_to_measuring_everything() {
        let dir = Path::new(FIXTURES);
        let r = Resume::load(
            &dir.join("does-not-exist.jsonl"),
            &dir.join("timeline-numeric-opt.json"),
            &want(),
        );
        assert!(r.is_empty());
        assert_eq!(r.disabled(), Some(Disabled::NoPriorRaw));

        let r = Resume::load(
            &dir.join("does-not-exist.jsonl"),
            &dir.join("does-not-exist.json"),
            &want(),
        );
        assert_eq!(
            r.disabled(),
            Some(Disabled::NoPriorConditions),
            "the conditions file is checked first: without it nothing may be \
             reused, not even a row already judged clean"
        );
    }

    /// The operator line: refusals grouped by reason, so a gate refusing
    /// everything for one silly reason is visible at a glance.
    #[test]
    fn refusals_are_counted_by_reason() {
        let cond = fixture("timeline-numeric-opt.json");
        let text = [
            row_json(""),
            row_json("").replace(ENGINE, "b3:other"),
            row_json("")
                .replace(r#""instance":7"#, r#""instance":8"#)
                .replace(ENGINE, "b3:other"),
            row_json("")
                .replace(r#""instance":7"#, r#""instance":9"#)
                .replace(r#""end_ts":1787455256.1"#, r#""end_ts":null"#),
        ]
        .join("\n");
        let r = Resume::judge_lines(&text, &cond, &want());
        assert_eq!(r.len(), 1);
        let counts = r.reject_counts();
        assert_eq!(counts.get("engine-mismatch"), Some(&2));
        assert_eq!(counts.get("no-end-ts"), Some(&1));
    }
}
