//! Two runs of the same board, and what a report is allowed to say about the
//! difference.
//!
//! Seeded from `benchmarks/ipc67-diff.py` (85 lines) and `crucible-spec.md`
//! §10. This is the one part of the publication layer with no Python oracle
//! behind it, which makes it the one part where a rule can be *chosen* rather
//! than merely reproduced -- so every choice that departs from the seed is
//! named here, on the record, instead of sitting in the code looking like an
//! accident.
//!
//! **The seed counts VAL-REJECTED plans as coverage.** `ipc67-diff.py` reads
//! `r["solved"]`, the engine's own claim. The standings layer does not:
//! `Referee::is_solved` drops a row the external referee rejected, unless VAL
//! could not ingest the domain at all. The two disagree on real committed data,
//! and the disagreement is not academic. On `2014 tempo-sat`,
//! `map-analyzer-temporal-satisficing` 17, 18 and 20 produced plans at 0.19.0
//! that VAL **rejected**, and plans at 0.21.0 that VAL accepted. The seed books
//! those three as "solved by both" -- no movement at all -- where they are in
//! fact three plan-soundness fixes, and the only three the 0.19→0.21 temporal
//! board has. This module uses the referee, so it reports 5 gains where the
//! seed reports 2. That is a correctness improvement over the seed, deliberate,
//! and it is why `standings.py` and a diff of the same boards can no longer
//! print different coverage.
//!
//! **The seed joins on `(variant, instance)`.** Every other join in this crate
//! is `(ipc, variant, instance)`, and it has to be: `seq-sat`, `tempo-sat` and
//! `seq-opt` raws each span two competitions, `ipc67-results.jsonl` alone
//! carries 300 `ipc-2008` rows beside 280 `ipc-2011` rows, and the two
//! competitions' variant names are disjoint only by today's luck. Inheriting
//! the narrower key would mean the first collision published a merged number
//! nobody could see. [`InstanceKey`] carries the competition.
//!
//! That key also needs a TOTAL ORDER, and the seed's tuple does not have one:
//! `instance` is an int for `p07` and a string for `3_10_50_10`, so
//! `sorted(set(A) | set(B))` raises `TypeError: '<' not supported between
//! instances of 'int' and 'str'` -- `ipc67-diff.py` cannot run on
//! `ipc2026-numeric` at all, today, on committed data. Here the ordering is a
//! property of the type.
//!
//! **Losses are the loud case.** A problem solved on the previous tag and not
//! on this one is a REGRESSION. It is never a count: every one is named, and
//! carries the class it held on each side, so a report says "solved in 0.19.0,
//! now timeout" rather than the useless "lost 2".
//!
//! **Quality can only be asked of a coverage diff.** `crucible-spec.md` §10
//! says "never compare cost over differing solved-sets". A convention holds
//! until the second call site, so [`QualityDiff`] has no public constructor and
//! the only way to obtain one is [`CoverageDiff::quality`], which uses the
//! commonly-solved set it computed itself, over the two runs it is borrowing.
//! There is no spelling for "mean cost of A over A's solved set versus mean
//! cost of B over B's".
//!
//! **Percentiles are NEAREST-RANK**, `sorted[ceil(q*n) - 1]` 1-based, and the
//! rank is computed in INTEGERS (`(n*num + den - 1) / den`). There is no Python
//! original to match, so the definition is stated rather than inherited, and
//! `ceil(0.95 * n)` in floating point is not a thing to depend on for a number
//! that gets published.
//!
//! **Three exclusion counts, never two.** The spec asks for "how many runs were
//! excluded as dirty"; two counts would be a lie by omission, because a run
//! whose conditions were never recorded is not clean and is not dirty either.
//! That case is the common one, not a corner: of the 76 conditions files on
//! this box only four carry a per-sample timeline, and the committed
//! `benchmarks/air25-entries/*.conditions.json` carry none, so every row on
//! every one of those boards is genuinely `Unknown`. [`Percentiles`]
//! deliberately does not implement `Display` -- only [`TimingDiff`] does -- so a
//! percentile cannot reach a page without its exclusion counts beside it.
//!
//! **A wall-clock fallback confesses.** `crucible-spec.md:153` says
//! IPC-comparable numbers come from `cpu_ms` on clean runs. No committed raw
//! carries `cpu_ms` yet -- it is a crucible-era column -- so every timing here
//! falls back to the runner's wall `time`, and [`Clock`] makes the report say
//! so instead of letting a wall number wear a CPU number's authority.

use crate::class::Class;
use crate::fmt::{fmt_f, glyph};
use crate::history::Trend;
use crate::raw::{Instance, RawRow};
use crate::referee::Referee;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

// ---------------------------------------------------------------------------
// Cleanliness -- the same per-sample rule the resume gate uses
// ---------------------------------------------------------------------------

/// The clean line, in named-competitor CPU percent.
///
/// **This is the same line as `crucible_core::monitor::SAMPLE_CLEAN_PCPU`, the
/// one `crucible_core::sched::resume` gates a reused row on, and the two must
/// not drift.** `contention.py` and `ipc67.py` each carry a comment saying the
/// rule is kept in one place precisely so it cannot; the port has to say it
/// twice only because `crucible-core` depends on this crate and not the other
/// way round, so the resume gate over there cannot be the home of a constant
/// this pure layer needs. If they are ever reconciled, this is the lower crate
/// and therefore the right home: `crucible-core` can import this, this crate
/// can never import `crucible-core`.
///
/// A disagreement here is not a cosmetic one. A row this module counts as clean
/// and the gate would have re-run is a published percentile taken over a window
/// the runner itself considered unusable.
///
/// The verdict is competitor load, not idle. It moved off `idle_pct` at 0.24
/// because `idle_pct` is whole-machine and includes the board's OWN threads: a
/// `--threads 8` mco board burns most of this ten-core box by design and could
/// never clear a fixed idle floor even in an empty room.
pub const SAMPLE_CLEAN_PCPU: f64 = 25.0;

/// What the machine was doing while one row was measured.
///
/// Three states, not two. `Unknown` is not a hedge -- it is the answer for
/// every row on a board whose watcher predates the 0.25 timeline, and for every
/// row whose window falls outside the sampled span. It must never be folded
/// into `Clean`: "clean by omission" is exactly the claim the whole-board retry
/// existed to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cleanliness {
    Clean,
    Dirty,
    Unknown,
}

impl Cleanliness {
    pub fn label(self) -> &'static str {
        match self {
            Cleanliness::Clean => "clean",
            Cleanliness::Dirty => "dirty",
            Cleanliness::Unknown => "unstamped",
        }
    }

    /// The verdict on a PAIR of rows, one per side, for a number computed from
    /// both.
    ///
    /// `Dirty` wins over `Unknown` on purpose: when one side is measurably
    /// contended the pair is excluded for a reason we can name, and naming it is
    /// more useful than recording that the other side was never watched.
    pub fn worse(self, other: Cleanliness) -> Cleanliness {
        match (self, other) {
            (Cleanliness::Dirty, _) | (_, Cleanliness::Dirty) => Cleanliness::Dirty,
            (Cleanliness::Unknown, _) | (_, Cleanliness::Unknown) => Cleanliness::Unknown,
            _ => Cleanliness::Clean,
        }
    }
}

/// One sample of the contention timeline: `[epoch_ts, idle_pct, competitors]`.
///
/// `idle_pct` is parsed and dropped. It is column 1 of the file and the rule
/// does not read it -- see [`SAMPLE_CLEAN_PCPU`] for why the verdict left it at
/// 0.24. Keeping a field nobody reads would invite somebody to read it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TimelineSample {
    at: f64,
    /// `None` means the watcher recorded a sample it could not attribute, which
    /// the gate treats as dirty rather than as absent.
    competitors: Option<f64>,
}

/// A board's `*.conditions.json`, reduced to what the per-row rule reads.
///
/// A missing or malformed file degrades to an empty timeline, exactly as
/// `ipc67.py:load_resume` does (`except (OSError, ValueError): return {}`).
/// Every row then reads `Unknown`, which is the honest answer and the one that
/// fails closed.
#[derive(Debug, Clone)]
pub struct Conditions {
    interval: f64,
    timeline: Vec<TimelineSample>,
}

/// Hand-written rather than derived: a derived `Default` would give `interval`
/// the value 0, which is a different rule (no padding at all) wearing the name
/// of "nothing was recorded".
impl Default for Conditions {
    fn default() -> Self {
        Self::none()
    }
}

impl Conditions {
    /// No conditions were recorded. Every row is `Unknown`.
    pub fn none() -> Self {
        Conditions {
            interval: 20.0,
            timeline: Vec::new(),
        }
    }

    /// Parse a conditions document, degrading anything unreadable to
    /// [`Conditions::none`].
    pub fn from_json(src: &str) -> Self {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(src) else {
            return Self::none();
        };
        // `float(cond.get("interval") or 20)` -- and `or` is FALSY in Python, so
        // a recorded interval of 0 falls back to 20 just as a missing key does.
        // A zero interval would make the padding vanish and quietly narrow the
        // window every row is judged over.
        let interval = match v.get("interval").and_then(|x| x.as_f64()) {
            Some(i) if i != 0.0 => i,
            _ => 20.0,
        };
        let mut timeline = Vec::new();
        if let Some(items) = v.get("timeline").and_then(|x| x.as_array()) {
            for t in items {
                // `isinstance(t, list) and len(t) >= 3 and t[0] is not None`:
                // a sample the watcher could not timestamp cannot be placed
                // against a window, so it is dropped rather than guessed at.
                let Some(a) = t.as_array() else { continue };
                if a.len() < 3 || a[0].is_null() {
                    continue;
                }
                // PAST that filter, a column that is present but is not a
                // number is where three implementations could part company, and
                // only one of the three answers is safe. Python raises out of
                // `load_resume` and takes the whole pass down;
                // `crucible_core::sched::resume::Conditions::parse` REFUSES the
                // file and re-measures; dropping the sample -- which is what
                // this did -- is the one option that fails OPEN, because the
                // sample dropped may be the contended one and the window it
                // would have condemned then reads `Clean`. This gate has to
                // agree with the resume gate, so it refuses too: an unreadable
                // conditions file records nothing, every row reads `Unknown`,
                // and no percentile is published from a window nobody watched.
                let Some(at) = a[0].as_f64() else {
                    return Self::none();
                };
                let (Some(_idle), Some(competitors)) = (num_or_null(&a[1]), num_or_null(&a[2]))
                else {
                    return Self::none();
                };
                timeline.push(TimelineSample { at, competitors });
            }
        }
        Conditions { interval, timeline }
    }

    /// Load from disk. A missing OR unreadable file is [`Conditions::none`],
    /// the way a missing input file degrades everywhere else in this crate.
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(s) => Self::from_json(&s),
            Err(_) => Self::none(),
        }
    }

    /// Whether a per-sample timeline was recorded at all. A board without one
    /// can only ever answer `Unknown`, and a report should be able to say that
    /// once rather than repeat it per row.
    pub fn has_timeline(&self) -> bool {
        !self.timeline.is_empty()
    }

    pub fn interval(&self) -> f64 {
        self.interval
    }

    /// The verdict for one row's wall-clock window.
    ///
    /// Ported line for line from the resume gate in `ipc67.py:load_resume`,
    /// because the two must agree: a row this call declines to count is a row
    /// that gate would have re-run, and if they disagree then a number gets
    /// published from a window the runner itself considered unusable.
    ///
    /// The one difference is the shape of the answer, not the rule. The gate has
    /// two outcomes -- reuse, or re-run -- and folds "no timeline" in with
    /// "contended". A report needs the third state, so the two reasons are kept
    /// apart here.
    pub fn cleanliness(&self, r: &RawRow) -> Cleanliness {
        // A row already stitched by an earlier pass carries the judgment
        // itself: its own conditions file is long gone, and the stamp IS the
        // record. Checked first, as the gate checks it first.
        if r.resumed_clean {
            return Cleanliness::Clean;
        }
        let (Some(s), Some(e)) = (r.start_ts, r.end_ts) else {
            // A row with no window predates the 0.25 stamps, or was written by
            // a runner killed mid-instance. Nothing to intersect.
            return Cleanliness::Unknown;
        };
        let (Some(first), Some(last)) = (self.timeline.first(), self.timeline.last()) else {
            return Cleanliness::Unknown;
        };
        // FIRST and LAST, not min and max: `contention.py` appends in time
        // order and the gate indexes `tl[0]` / `tl[-1]`. Sorting here would be
        // a different rule wearing the same name.
        if s < first.at - self.interval || e > last.at + self.interval {
            // The window is not covered by the sampled span -- the watcher
            // started late or died early, and part of this run was unobserved.
            return Cleanliness::Unknown;
        }
        let mut saw_one = false;
        let mut dirty = false;
        for t in &self.timeline {
            if t.at < s - self.interval || t.at > e + self.interval {
                continue;
            }
            saw_one = true;
            // PER-SAMPLE, never the run median. A clean median over a dirty
            // window is precisely the lie the whole-board retry existed to
            // prevent, and an unattributed sample counts against the row.
            if t.competitors.counts_as_dirty(SAMPLE_CLEAN_PCPU) {
                dirty = true;
            }
        }
        if !saw_one {
            return Cleanliness::Unknown;
        }
        if dirty {
            Cleanliness::Dirty
        } else {
            Cleanliness::Clean
        }
    }
}

/// One timeline column, distinguishing "the watcher recorded nothing here"
/// from "this file is not a timeline".
///
/// `Some(None)` is a JSON `null`, which is a VALUE in this format -- the
/// watcher took a sample and could not attribute it, and
/// [`SampleLoad::counts_as_dirty`] books that against the row. `None` is
/// anything else non-numeric, which means the caller cannot read this file the
/// way the resume gate reads it and must refuse the whole document.
fn num_or_null(v: &serde_json::Value) -> Option<Option<f64>> {
    match v {
        serde_json::Value::Null => Some(None),
        other => other.as_f64().map(Some),
    }
}

/// `t[2] is None or t[2] >= SAMPLE_CLEAN_PCPU`, kept as one named test so the
/// `None` half cannot be dropped in a later tidy-up. A sample the watcher could
/// not attribute is not evidence of quiet.
trait SampleLoad {
    fn counts_as_dirty(self, threshold: f64) -> bool;
}

impl SampleLoad for Option<f64> {
    fn counts_as_dirty(self, threshold: f64) -> bool {
        match self {
            None => true,
            Some(v) => v >= threshold,
        }
    }
}

// ---------------------------------------------------------------------------
// The join key
// ---------------------------------------------------------------------------

/// What two runs are joined on: competition, variant, instance.
///
/// See the module header for why the competition is in here and why the seed's
/// two-part key must not be inherited.
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceKey {
    pub ipc: Option<String>,
    pub variant: String,
    pub instance: Instance,
}

impl InstanceKey {
    pub fn of(r: &RawRow) -> Self {
        InstanceKey {
            ipc: r.ipc.clone(),
            variant: r.variant.clone(),
            instance: r.instance.clone(),
        }
    }

    /// `variant/instance` -- the seed's per-instance label, kept because the
    /// acceptance evidence in `benchmarks/COMPARING.md` is written in it.
    pub fn label(&self) -> String {
        format!("{}/{}", self.variant, self.instance)
    }

    /// `ipc/variant/instance`, for the case where two competitions have brought
    /// the same variant name to the same board.
    pub fn qualified(&self) -> String {
        format!(
            "{}/{}/{}",
            self.ipc.as_deref().unwrap_or("-"),
            self.variant,
            self.instance
        )
    }
}

/// Instance ordering: integers first and numerically, multipart labels after
/// and by byte order.
///
/// Python's `sorted()` cannot express this at all -- comparing `7` to
/// `"3_10_50_10"` raises -- which is why the seed dies on the 2026 numeric
/// board. Any total order will do; what matters is that ONE exists, so a diff
/// of a mixed board has a stable, reproducible instance sequence.
fn instance_cmp(a: &Instance, b: &Instance) -> Ordering {
    match (a, b) {
        (Instance::Num(x), Instance::Num(y)) => x.cmp(y),
        // Byte order over UTF-8 is codepoint order, which is what Python's
        // `sorted()` on `str` gives -- so a board of only multipart labels
        // orders identically in both languages.
        (Instance::Parts(x), Instance::Parts(y)) => x.as_bytes().cmp(y.as_bytes()),
        (Instance::Num(_), Instance::Parts(_)) => Ordering::Less,
        (Instance::Parts(_), Instance::Num(_)) => Ordering::Greater,
    }
}

impl Ord for InstanceKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ipc
            .cmp(&other.ipc)
            .then_with(|| self.variant.cmp(&other.variant))
            .then_with(|| instance_cmp(&self.instance, &other.instance))
    }
}

impl PartialOrd for InstanceKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// `Instance` derives `PartialEq` and not `Eq` (see `raw.rs`), so the total
/// equality a map key needs is asserted here rather than derived. It is sound:
/// both arms hold a `u64` or a `String`, and no float ever enters this key.
impl Eq for InstanceKey {}

impl std::fmt::Display for InstanceKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label())
    }
}

// ---------------------------------------------------------------------------
// Cost
// ---------------------------------------------------------------------------

/// A row's plan cost: `metric` where the row has one, `length` otherwise.
///
/// Kept as an enum rather than collapsed to `f64` so the rendering keeps the
/// seed's shape -- a metric prints `1303.0` and a length prints `107`, because
/// one came back from Python as a float and the other as an int. Comparisons go
/// through [`Cost::value`], so an int length and a float metric still compare.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cost {
    Metric(f64),
    Length(u64),
}

impl Cost {
    /// `r.get("metric") if r.get("metric") is not None else r.get("length")`.
    ///
    /// `is not None`, never truthiness: a plan of metric 0.0 is a plan, and
    /// `standings.py`'s reference loader gets this wrong in the other direction
    /// on purpose (`if r.get("solved") and cost`), which is a different
    /// function with a different job.
    pub fn of(r: &RawRow) -> Option<Cost> {
        match r.metric {
            Some(m) => Some(Cost::Metric(m)),
            None => r.length.map(Cost::Length),
        }
    }

    pub fn value(self) -> f64 {
        match self {
            Cost::Metric(m) => m,
            Cost::Length(n) => n as f64,
        }
    }
}

impl std::fmt::Display for Cost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cost::Metric(m) => f.write_str(&py_repr_f(*m)),
            Cost::Length(n) => write!(f, "{n}"),
        }
    }
}

/// Python's `str(float)`. `raw.rs` has the same helper for the same reason and
/// keeps it private; this one exists so a report's `cost 1303.0` matches the
/// seed's, not so the two can drift.
///
/// Rust's `{}` is NOT `str(float)`, and the gap is not only the `.0` on a whole
/// number. CPython's `float_repr` switches to EXPONENTIAL notation when the
/// decimal point of the shortest round-trip digit string sits at position
/// `<= -4` or `> 16`; Rust's `Display` never does, at any magnitude. So
/// `str(1e16)` is `1e+16` where `format!("{}")` writes seventeen digits, and
/// `str(1e-05)` is `1e-05` where it writes `0.00001`. Today's corpus cannot
/// reach either boundary -- the largest `metric` on this box is `5048589.0` and
/// the smallest non-zero is `1.0` -- which is exactly why it is closed here
/// rather than left for the first board that does.
///
/// One residual difference, and it is stated rather than papered over: on the
/// FIXED side, a value whose shortest representation is exactly halfway between
/// two decimal strings of the same length is broken toward even by CPython's
/// `_Py_dg_dtoa` and away from zero by Rust's formatter (`2181495296738027.2`
/// against `...27.3`). Reproducing that needs a dtoa port, not a branch. It
/// takes 16 significant digits to reach -- measured over 204,207 doubles, every
/// disagreement had magnitude above 1.8e13 -- so no cost this project can
/// publish comes near it.
fn py_repr_f(x: f64) -> String {
    if x.is_nan() {
        // Rust spells it `NaN`, Python spells it `nan`. Unreachable from a real
        // plan, closed here rather than left as a landmine.
        return "nan".to_string();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    // `{:e}` is the same shortest-round-trip digit string, already normalised
    // to one digit before the point, so the exponent it prints IS `decpt - 1`.
    let sci = format!("{x:e}");
    // `{:e}` always emits exactly one `e`; the fallback falls through to the
    // fixed branch, which is the harmless answer if it ever does not.
    let (mantissa, exponent) = sci.split_once('e').unwrap_or(("", "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    // CPython's `float_repr` switches to exponential when `decpt <= -4 ||
    // decpt > 16`, where decpt is the position of the decimal point -- one more
    // than the normalised exponent. Named rather than folded into the
    // comparison, so the correspondence with CPython stays checkable.
    let decpt = exponent + 1;
    if decpt <= -4 || decpt > 16 {
        // Python pads the exponent to at least two digits and always signs it:
        // `1e+16`, `1e-05`, `1.2345678901234568e+18`.
        let sign = if exponent < 0 { '-' } else { '+' };
        return format!("{mantissa}e{sign}{:02}", exponent.unsigned_abs());
    }
    let s = format!("{x}");
    if s.contains(['.', 'e', 'E']) {
        s
    } else {
        format!("{s}.0")
    }
}

// ---------------------------------------------------------------------------
// RunRef
// ---------------------------------------------------------------------------

/// One side of a comparison: a named run's rows, keyed for the join, plus the
/// budget its timeouts are denominated in.
///
/// The budget lives here rather than on the diff because the two sides can
/// legitimately differ -- a board that moved tier between tags is exactly the
/// case `Referee::budget_for` exists for, and a row's own `budget` stamp still
/// beats this whenever it has one.
#[derive(Debug, Clone)]
pub struct RunRef {
    pub name: String,
    pub rows: BTreeMap<InstanceKey, RawRow>,
    pub budget: f64,
}

/// A run plus what building it COST, which is not always nothing.
///
/// `air-0.19.0/ipc2026-numeric.jsonl` is 320 rows under 288 keys: the runner of
/// that era took only the first digit group of a multipart filename, so 32 rows
/// land on a key another row already holds. Keying silently discards them, and
/// a diff that reports "288 rows" against a 320-row board is wrong twice over.
/// The collapsed keys come back with the run so a caller can say so.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub run: RunRef,
    /// Keys that more than one row claimed, in the order the collisions
    /// happened.
    pub collapsed: Vec<InstanceKey>,
}

impl RunRef {
    /// Key a board's rows.
    ///
    /// On a collision the LAST row wins, matching the seed's `out[k] = r`. That
    /// is not a considered rule so much as the one the existing evidence was
    /// produced under; the collision itself is the thing worth reporting.
    pub fn from_rows(name: impl Into<String>, budget: f64, rows: Vec<RawRow>) -> Loaded {
        let mut map = BTreeMap::new();
        let mut collapsed = Vec::new();
        for r in rows {
            let k = InstanceKey::of(&r);
            if map.insert(k.clone(), r).is_some() {
                collapsed.push(k);
            }
        }
        Loaded {
            run: RunRef {
                name: name.into(),
                rows: map,
                budget,
            },
            collapsed,
        }
    }

    /// Parse and key a board raw. `path` is only used to locate a bad line.
    pub fn from_jsonl(
        name: impl Into<String>,
        budget: f64,
        src: &str,
        path: &str,
    ) -> Result<Loaded, String> {
        Ok(Self::from_rows(name, budget, crate::parse_rows(src, path)?))
    }

    pub fn get(&self, k: &InstanceKey) -> Option<&RawRow> {
        self.rows.get(k)
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Rows the REFEREE calls solved -- not the rows that claim to be.
    pub fn solved(&self, referee: &Referee) -> usize {
        self.rows.values().filter(|r| referee.is_solved(r)).count()
    }

    /// The class this run gives a row, in this run's own budget.
    fn class_of(&self, referee: &Referee, k: &InstanceKey) -> Option<Class> {
        self.rows.get(k).map(|r| referee.classify(r, self.budget))
    }
}

// ---------------------------------------------------------------------------
// Coverage
// ---------------------------------------------------------------------------

/// A problem solved on A and not on B. The loud case.
///
/// `was` is `Class::Solved` by construction; it is carried anyway so the record
/// reads as a sentence -- "solved in 0.19.0, now timeout" -- and so the JSON
/// form is self-describing. `now` is `None` when B has no row for this key at
/// all, which is a different fact from any failure class and must not be
/// rendered as one.
#[derive(Debug, Clone, PartialEq)]
pub struct Lost {
    pub key: InstanceKey,
    pub was: Class,
    pub now: Option<Class>,
}

/// A problem solved on B and not on A.
#[derive(Debug, Clone, PartialEq)]
pub struct Gained {
    pub key: InstanceKey,
    pub was: Option<Class>,
    pub now: Class,
}

/// The coverage half of a diff, and the gate every other half is reached
/// through.
///
/// Borrows both runs and the referee for its whole life. That is the
/// enforcement: [`quality`](Self::quality) and [`timing`](Self::timing) compute
/// over the commonly-solved set THIS value found, in the runs THIS value was
/// built from. A caller cannot hand them a different pair.
#[derive(Debug)]
pub struct CoverageDiff<'r> {
    a: &'r RunRef,
    b: &'r RunRef,
    referee: &'r Referee,
    a_solved: usize,
    b_solved: usize,
    gained: Vec<Gained>,
    lost: Vec<Lost>,
    common: Vec<InstanceKey>,
    /// Variant names that occur under more than one competition across the
    /// union. Empty on every board this project has ever published; when it is
    /// not, every label for those variants is qualified so two competitions
    /// cannot share a table row.
    ambiguous: BTreeSet<String>,
}

impl<'r> CoverageDiff<'r> {
    /// Union the key sets and sort every row into gained, lost or common.
    pub fn new(referee: &'r Referee, a: &'r RunRef, b: &'r RunRef) -> Self {
        let mut keys: BTreeSet<&InstanceKey> = BTreeSet::new();
        keys.extend(a.rows.keys());
        keys.extend(b.rows.keys());

        let mut gained = Vec::new();
        let mut lost = Vec::new();
        let mut common = Vec::new();
        // `variant -> the competitions it appeared under`.
        let mut seen: BTreeMap<&str, BTreeSet<Option<&str>>> = BTreeMap::new();

        for k in keys {
            seen.entry(k.variant.as_str())
                .or_default()
                .insert(k.ipc.as_deref());
            let sa = a.rows.get(k).is_some_and(|r| referee.is_solved(r));
            let sb = b.rows.get(k).is_some_and(|r| referee.is_solved(r));
            match (sa, sb) {
                (true, false) => lost.push(Lost {
                    key: k.clone(),
                    // Present and solved on A, so the class is known.
                    was: a.class_of(referee, k).unwrap_or(Class::Solved),
                    now: b.class_of(referee, k),
                }),
                (false, true) => gained.push(Gained {
                    key: k.clone(),
                    was: a.class_of(referee, k),
                    now: b.class_of(referee, k).unwrap_or(Class::Solved),
                }),
                (true, true) => common.push(k.clone()),
                (false, false) => {}
            }
        }

        let ambiguous = seen
            .into_iter()
            .filter(|(_, ipcs)| ipcs.len() > 1)
            .map(|(v, _)| v.to_string())
            .collect();

        CoverageDiff {
            a,
            b,
            referee,
            a_solved: a.solved(referee),
            b_solved: b.solved(referee),
            gained,
            lost,
            common,
            ambiguous,
        }
    }

    pub fn a(&self) -> &RunRef {
        self.a
    }

    pub fn b(&self) -> &RunRef {
        self.b
    }

    pub fn a_solved(&self) -> usize {
        self.a_solved
    }

    pub fn b_solved(&self) -> usize {
        self.b_solved
    }

    pub fn gained(&self) -> &[Gained] {
        &self.gained
    }

    /// Every problem solved on A and not on B, named. There is no count-only
    /// accessor on purpose.
    pub fn lost(&self) -> &[Lost] {
        &self.lost
    }

    pub fn common(&self) -> &[InstanceKey] {
        &self.common
    }

    /// Movement in solved COUNT, which is only meaningful when the two boards
    /// have the same rows. `STANDINGS.md` quotes shares for the cross-release
    /// case precisely because a corpus can grow between tags -- see
    /// `history::ComparablePredecessor::delta`.
    pub fn delta(&self) -> i64 {
        self.b_solved as i64 - self.a_solved as i64
    }

    /// Any loss at all. Red, toasted and pinned, per `crucible-spec.md` §10.
    pub fn is_regression(&self) -> bool {
        !self.lost.is_empty()
    }

    /// The label a report prints for a key: the seed's `variant/instance`,
    /// qualified with the competition only where it must be.
    pub fn label(&self, k: &InstanceKey) -> String {
        if self.ambiguous.contains(&k.variant) {
            k.qualified()
        } else {
            k.label()
        }
    }

    /// The table's row label for a key, on the same rule.
    fn variant_label(&self, k: &InstanceKey) -> String {
        if self.ambiguous.contains(&k.variant) {
            format!("{}/{}", k.ipc.as_deref().unwrap_or("-"), k.variant)
        } else {
            k.variant.clone()
        }
    }

    /// Cost comparison over the commonly-solved set. The ONLY way to build a
    /// [`QualityDiff`].
    pub fn quality(&self) -> QualityDiff {
        let mut cheaper_a = 0usize;
        let mut cheaper_b = 0usize;
        let mut equal = 0usize;
        let mut total_a = 0.0f64;
        let mut total_b = 0.0f64;
        let mut scored = 0usize;
        // Keyed and therefore SORTED by label, which is the seed's `for v in
        // sorted(stats)` and not the order the variants were first seen. The
        // sums inside each row are float sums accumulated in `common` order,
        // which the map type does not touch -- reordering those can move the
        // last digit of a published cell, so the map orders the ROWS and
        // nothing else.
        let mut variants: BTreeMap<String, VariantRow> = BTreeMap::new();

        for k in &self.common {
            let (Some(ra), Some(rb)) = (self.a.get(k), self.b.get(k)) else {
                continue;
            };
            let row = variants
                .entry(self.variant_label(k))
                .or_insert_with(|| VariantRow {
                    label: self.variant_label(k),
                    n: 0,
                    cheaper_a: 0,
                    cheaper_b: 0,
                    time_a: 0.0,
                    time_b: 0.0,
                });
            row.n += 1;
            // `s["ta"] += ra["time"] or 0` -- a missing elapsed contributes
            // nothing rather than poisoning the column.
            row.time_a += ra.time_secs().unwrap_or(0.0);
            row.time_b += rb.time_secs().unwrap_or(0.0);

            let (Some(ca), Some(cb)) = (Cost::of(ra), Cost::of(rb)) else {
                continue;
            };
            scored += 1;
            // Accumulated in key order, which is the order `common` was built
            // in. Float addition is not associative.
            total_a += ca.value();
            total_b += cb.value();
            match ca.value().partial_cmp(&cb.value()) {
                Some(Ordering::Less) => {
                    cheaper_a += 1;
                    row.cheaper_a += 1;
                }
                Some(Ordering::Greater) => {
                    cheaper_b += 1;
                    row.cheaper_b += 1;
                }
                // Equal, or a NaN cost -- which the seed's `<` / `<` pair also
                // books as neither cheaper.
                _ => equal += 1,
            }
        }

        QualityDiff {
            a_name: self.a.name.clone(),
            b_name: self.b.name.clone(),
            common: self.common.len(),
            scored,
            cheaper_a,
            cheaper_b,
            equal,
            total_a,
            total_b,
            variants: variants.into_values().collect(),
        }
    }

    /// p50/p95 over the commonly-solved set, clean runs only. The ONLY way to
    /// build a [`TimingDiff`].
    pub fn timing(&self, cond_a: &Conditions, cond_b: &Conditions) -> TimingDiff {
        let mut clean = 0usize;
        let mut dirty = 0usize;
        let mut unstamped = 0usize;
        let mut cpu_rows = 0usize;
        let mut wall_rows = 0usize;
        let mut ms_a: Vec<f64> = Vec::new();
        let mut ms_b: Vec<f64> = Vec::new();

        for k in &self.common {
            let (Some(ra), Some(rb)) = (self.a.get(k), self.b.get(k)) else {
                continue;
            };
            // Judged as a PAIR. Two percentiles over two different subsets are
            // the same sin the quality rule forbids, one column to the right.
            match cond_a.cleanliness(ra).worse(cond_b.cleanliness(rb)) {
                Cleanliness::Dirty => {
                    dirty += 1;
                    continue;
                }
                Cleanliness::Unknown => {
                    unstamped += 1;
                    continue;
                }
                Cleanliness::Clean => {}
            }
            let (Some((a_ms, a_clock)), Some((b_ms, b_clock))) = (row_ms(ra), row_ms(rb)) else {
                // Clean but unmeasured. Not an exclusion the spec names, and
                // never reachable from a solved row (`ipc67.py` stamps elapsed
                // on every exit path since 0.20), so it is simply not counted
                // rather than booked under a heading it does not belong to.
                continue;
            };
            clean += 1;
            for c in [a_clock, b_clock] {
                match c {
                    RowClock::Cpu => cpu_rows += 1,
                    RowClock::Wall => wall_rows += 1,
                }
            }
            ms_a.push(a_ms);
            ms_b.push(b_ms);
        }

        ms_a.sort_by(f64::total_cmp);
        ms_b.sort_by(f64::total_cmp);
        let pct = if ms_a.is_empty() {
            None
        } else {
            Some((Percentiles::of(&ms_a), Percentiles::of(&ms_b)))
        };

        TimingDiff {
            a_name: self.a.name.clone(),
            b_name: self.b.name.clone(),
            clock: Clock::of(cpu_rows, wall_rows),
            pct,
            clean,
            dirty,
            unstamped,
        }
    }

    /// The referee both halves are refereed by, for a caller assembling its own
    /// view of the same diff.
    pub fn referee(&self) -> &Referee {
        self.referee
    }
}

// ---------------------------------------------------------------------------
// Quality
// ---------------------------------------------------------------------------

/// One row of the per-variant table, in the seed's columns.
#[derive(Debug, Clone, PartialEq)]
pub struct VariantRow {
    pub label: String,
    /// Instances solved by BOTH runs in this variant -- not the variant's size.
    pub n: usize,
    pub cheaper_a: usize,
    pub cheaper_b: usize,
    pub time_a: f64,
    pub time_b: f64,
}

/// Cost over the problems both runs solved, and nothing else.
///
/// No public constructor: see [`CoverageDiff::quality`]. Every count in here is
/// over one set, computed once, and there is no field a caller could fill from
/// a differently-shaped comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct QualityDiff {
    a_name: String,
    b_name: String,
    common: usize,
    scored: usize,
    cheaper_a: usize,
    cheaper_b: usize,
    equal: usize,
    total_a: f64,
    total_b: f64,
    variants: Vec<VariantRow>,
}

impl QualityDiff {
    /// Problems solved by both runs.
    pub fn common(&self) -> usize {
        self.common
    }

    /// Of those, the ones with a readable cost on BOTH sides. The denominator
    /// of every mean below, and never the board's size.
    pub fn scored(&self) -> usize {
        self.scored
    }

    pub fn cheaper_a(&self) -> usize {
        self.cheaper_a
    }

    pub fn cheaper_b(&self) -> usize {
        self.cheaper_b
    }

    pub fn equal(&self) -> usize {
        self.equal
    }

    pub fn total_a(&self) -> f64 {
        self.total_a
    }

    pub fn total_b(&self) -> f64 {
        self.total_b
    }

    /// `None` when nothing was scorable -- a zero would be a claim.
    pub fn mean_a(&self) -> Option<f64> {
        (self.scored > 0).then(|| self.total_a / self.scored as f64)
    }

    pub fn mean_b(&self) -> Option<f64> {
        (self.scored > 0).then(|| self.total_b / self.scored as f64)
    }

    pub fn variants(&self) -> &[VariantRow] {
        &self.variants
    }

    /// The per-variant table, byte-for-byte in `ipc67-diff.py`'s shape.
    ///
    /// This is the acceptance-evidence format the Phase 6 portfolio decision
    /// was recorded in, and it is pasted into cut records. The header line, the
    /// `|---|` rule, the column order and the `{:.0}s` time cells are all
    /// reproduced exactly; only the SET the numbers are computed over changes,
    /// because the referee decides it now.
    pub fn table(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "| variant | both | {} cheaper | {} cheaper | {} time | {} time |\n",
            self.a_name, self.b_name, self.a_name, self.b_name
        ));
        out.push_str("|---|---|---|---|---|---|\n");
        for v in &self.variants {
            // `{:.0}` and Python's `{:.0f}` are both exact-decimal, ties to
            // even, over the same f64 -- so they agree digit for digit.
            out.push_str(&format!(
                "| {} | {} | {} | {} | {:.0}s | {:.0}s |\n",
                v.label, v.n, v.cheaper_a, v.cheaper_b, v.time_a, v.time_b
            ));
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Timing
// ---------------------------------------------------------------------------

/// Which clock ONE row was measured on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowClock {
    Cpu,
    Wall,
}

/// A row's timing in MILLISECONDS, and the clock it came off.
///
/// `cpu_ms` is a crucible-era column: no committed raw carries one, so it
/// arrives through `RawRow::extra`, which is exactly what that flattened map
/// exists for. Everything else falls back to the runner's wall `time`, in
/// seconds, converted here so the two can never be summed in different units.
fn row_ms(r: &RawRow) -> Option<(f64, RowClock)> {
    if let Some(ms) = r.extra.get("cpu_ms").and_then(|v| v.as_f64()) {
        return Some((ms, RowClock::Cpu));
    }
    r.time_secs().map(|s| (s * 1000.0, RowClock::Wall))
}

/// Which clock a timing verdict was measured on, and how honest it is.
///
/// `crucible-spec.md:153`: "IPC-comparable numbers come from `cpu_ms` on clean
/// runs." A wall number is not that, and `Wall` is how the report says so.
/// `Mixed` counts both, because a percentile computed half from CPU time and
/// half from wall time is not a percentile of anything and the reader has to be
/// told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clock {
    Cpu,
    Wall,
    Mixed { cpu: usize, wall: usize },
}

impl Clock {
    fn of(cpu: usize, wall: usize) -> Clock {
        match (cpu, wall) {
            (0, _) => Clock::Wall,
            (_, 0) => Clock::Cpu,
            (cpu, wall) => Clock::Mixed { cpu, wall },
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Clock::Cpu => "cpu_ms",
            Clock::Wall => "wall (no cpu_ms on these rows)",
            Clock::Mixed { .. } => "MIXED cpu_ms and wall",
        }
    }
}

impl std::fmt::Display for Clock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Clock::Mixed { cpu, wall } => {
                write!(f, "MIXED: {cpu} rows cpu_ms, {wall} rows wall")
            }
            other => f.write_str(other.name()),
        }
    }
}

/// p50 and p95 in milliseconds.
///
/// **Deliberately not `Display`.** A percentile without its exclusion counts is
/// a number that looks like a measurement and is not one; `TimingDiff` is the
/// only thing here that renders, and it cannot render these without also
/// rendering how many runs were thrown away to get them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Percentiles {
    pub p50: f64,
    pub p95: f64,
}

impl Percentiles {
    /// NEAREST RANK over an already-sorted, non-empty slice.
    fn of(sorted: &[f64]) -> Percentiles {
        Percentiles {
            p50: nearest_rank(sorted, 1, 2),
            p95: nearest_rank(sorted, 95, 100),
        }
    }
}

/// `sorted[ceil(num/den * n) - 1]`, 1-based, with the rank in INTEGERS.
///
/// No Python original to match, so the definition is chosen and stated: nearest
/// rank, which always returns a value that was actually measured. No
/// interpolation, because an interpolated p95 is a number no run produced and
/// this record publishes measurements.
///
/// `(n * num).div_ceil(den)` is `ceil(n*num/den)` exactly. The float spelling
/// `(q * n as f64).ceil()` happens to agree for every `n` up to 200,000 today,
/// which is precisely the kind of agreement not to build a published number on:
/// `0.95` is not representable, so `0.95 * n` is a rounded product and its
/// ceiling is a coin toss at the boundaries.
fn nearest_rank(sorted: &[f64], num: u64, den: u64) -> f64 {
    debug_assert!(!sorted.is_empty(), "Percentiles::of guards the empty slice");
    let n = sorted.len() as u64;
    let rank = (n * num).div_ceil(den).clamp(1, n);
    sorted[(rank - 1) as usize]
}

/// p50/p95 over the commonly-solved set, and the three exclusion counts that
/// have to travel with them.
///
/// No public constructor: see [`CoverageDiff::timing`].
#[derive(Debug, Clone, PartialEq)]
pub struct TimingDiff {
    a_name: String,
    b_name: String,
    clock: Clock,
    pct: Option<(Percentiles, Percentiles)>,
    clean: usize,
    dirty: usize,
    unstamped: usize,
}

impl TimingDiff {
    pub fn clock(&self) -> Clock {
        self.clock
    }

    /// `None` when no commonly-solved pair survived the cleanliness gate. A
    /// zero would be a measurement; this is the absence of one.
    pub fn percentiles(&self) -> Option<(Percentiles, Percentiles)> {
        self.pct
    }

    /// Pairs both runs measured cleanly. The n behind the percentiles.
    pub fn clean(&self) -> usize {
        self.clean
    }

    /// Pairs excluded because at least one side ran against measured
    /// contention.
    pub fn dirty(&self) -> usize {
        self.dirty
    }

    /// Pairs excluded because at least one side was never watched -- no
    /// timeline, no window stamps, or a window outside the sampled span.
    /// Not clean. Not dirty. The count that must not be folded into either.
    pub fn unstamped(&self) -> usize {
        self.unstamped
    }
}

impl std::fmt::Display for TimingDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let excluded = format!(
            "{} clean, {} excluded dirty, {} excluded unstamped",
            self.clean, self.dirty, self.unstamped
        );
        match self.pct {
            None => write!(
                f,
                "p50/p95 [{}]: not computed -- no clean commonly-solved runs ({excluded})",
                self.clock
            ),
            Some((a, b)) => write!(
                f,
                "p50/p95 [{}]: {} {:.0}/{:.0} ms   {} {:.0}/{:.0} ms   ({excluded})",
                self.clock, self.a_name, a.p50, a.p95, self.b_name, b.p50, b.p95
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Historical trend
// ---------------------------------------------------------------------------

/// Render one board's coverage over releases.
///
/// [`Trend`] is `history.rs`'s, not this module's, and it is only ever
/// DERIVED by `History::trend(label, box)` -- which filters on the box before
/// it collects a single point. Nothing here takes a `History` or a `BoxId`, so
/// there is no widening this into a cross-box series: coverage at a fixed
/// budget is a property of the hardware as much as of the engine, and a
/// cloud-to-Air move rendered as engine progress is the incident that law
/// exists for.
///
/// Points come out in VERSION order, not measurement order -- a backfilled old
/// tag is measured late, and plotted by date it draws a spike no release ever
/// had.
pub fn render_trend(t: &Trend) -> String {
    if t.points.is_empty() {
        return format!("{}: no banked snapshots on this box", t.label);
    }
    let cells: Vec<String> = t
        .points
        .iter()
        .map(|(v, (s, n))| format!("{v} {s}/{n} {}%", fmt_f(crate::fmt::pct(*s, *n), 1)))
        .collect();
    format!(
        "{}: {}",
        t.label,
        cells.join(&format!(" {} ", glyph::MIDDOT))
    )
}

// ---------------------------------------------------------------------------
// The report
// ---------------------------------------------------------------------------

/// How a diff renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Human text, the seed's shape. The default.
    #[default]
    Text,
    /// Markdown, for pasting into a cut record.
    Markdown,
    /// The structured form, `indent=1` like every other JSON this project
    /// writes.
    Json,
}

/// A complete comparison: coverage, quality, timing, and optionally the
/// board's history.
///
/// The three halves are private and are built together, so a report cannot
/// carry a quality table computed over one solved-set beside a coverage table
/// computed over another.
#[derive(Debug)]
pub struct Diff<'r> {
    coverage: CoverageDiff<'r>,
    quality: QualityDiff,
    timing: TimingDiff,
    trend: Option<Trend>,
}

impl<'r> Diff<'r> {
    pub fn new(
        referee: &'r Referee,
        a: &'r RunRef,
        b: &'r RunRef,
        cond_a: &Conditions,
        cond_b: &Conditions,
    ) -> Self {
        let coverage = CoverageDiff::new(referee, a, b);
        let quality = coverage.quality();
        let timing = coverage.timing(cond_a, cond_b);
        Diff {
            coverage,
            quality,
            timing,
            trend: None,
        }
    }

    /// Attach the board's history. See [`render_trend`] for why this cannot be
    /// a cross-box series.
    pub fn with_trend(mut self, t: Trend) -> Self {
        self.trend = Some(t);
        self
    }

    pub fn coverage(&self) -> &CoverageDiff<'r> {
        &self.coverage
    }

    pub fn quality(&self) -> &QualityDiff {
        &self.quality
    }

    pub fn timing(&self) -> &TimingDiff {
        &self.timing
    }

    pub fn render(&self, mode: Mode) -> String {
        match mode {
            Mode::Text => self.text(),
            Mode::Markdown => self.markdown(),
            Mode::Json => self.to_json(),
        }
    }

    /// The cost/time detail the seed prints beside a flipped instance.
    fn detail(&self, r: Option<&RawRow>) -> String {
        let Some(r) = r else {
            return "(no row)".to_string();
        };
        let t = match &r.time {
            // The raw token, not a re-rounded float: `time` is an int on the
            // hard-timeout path and a 2dp float everywhere else, and the seed
            // prints whichever Python parsed.
            Some(n) => n.to_string(),
            None => "?".to_string(),
        };
        match Cost::of(r) {
            Some(c) => format!("({t}s, cost {c})"),
            None => format!("({t}s, cost -)"),
        }
    }

    fn text(&self) -> String {
        let c = &self.coverage;
        let (na, nb) = (c.a().name.clone(), c.b().name.clone());
        let mut o = String::new();

        // The seed's two header lines, unchanged in shape -- the numbers now
        // come from the referee.
        o.push_str(&format!(
            "{na}: {}/{} solved   {nb}: {}/{} solved\n",
            c.a_solved(),
            c.a().len(),
            c.b_solved(),
            c.b().len()
        ));
        o.push_str(&format!(
            "solved by both: {}   only {na}: {}   only {nb}: {}\n\n",
            c.common().len(),
            c.lost().len(),
            c.gained().len()
        ));

        if c.is_regression() {
            o.push_str(&format!(
                "REGRESSION: {} solved on {na} and not on {nb}\n",
                c.lost().len()
            ));
            for l in c.lost() {
                let now = match l.now {
                    Some(cl) => cl.to_string(),
                    None => "gone from the board".to_string(),
                };
                o.push_str(&format!(
                    "  lost: {}  {}  {} in {na}, now {now}\n",
                    c.label(&l.key),
                    self.detail(c.a().get(&l.key)),
                    l.was
                ));
            }
            o.push('\n');
        } else {
            o.push_str(&format!(
                "no regressions: nothing solved on {na} was lost\n\n"
            ));
        }

        if !c.gained().is_empty() {
            o.push_str(&format!("gained: {}\n", c.gained().len()));
            for g in c.gained() {
                let was = match g.was {
                    Some(cl) => cl.to_string(),
                    None => "not on the board".to_string(),
                };
                o.push_str(&format!(
                    "  gain: {}  {}  {was} in {na}, now {}\n",
                    c.label(&g.key),
                    self.detail(c.b().get(&g.key)),
                    g.now
                ));
            }
            o.push('\n');
        }

        let q = &self.quality;
        o.push_str(&format!(
            "quality over the {} problems solved by both ({} cost-comparable):\n",
            q.common(),
            q.scored()
        ));
        o.push_str(&format!(
            "  {na} cheaper: {}   {nb} cheaper: {}   equal: {}\n",
            q.cheaper_a(),
            q.cheaper_b(),
            q.equal()
        ));
        match (q.mean_a(), q.mean_b()) {
            (Some(ma), Some(mb)) => o.push_str(&format!(
                "  mean cost: {na} {}   {nb} {}\n",
                fmt_f(ma, 2),
                fmt_f(mb, 2)
            )),
            // No scorable row. The Python's `if not ratios: return None` rule,
            // one module over: an absent measurement is not a zero.
            _ => o.push_str("  mean cost: not measured (no row carries a cost on both sides)\n"),
        }
        o.push('\n');

        o.push_str(&format!("{}\n\n", self.timing));

        if let Some(t) = &self.trend {
            o.push_str(&format!("{}\n\n", render_trend(t)));
        }

        o.push_str(&q.table());
        o
    }

    fn markdown(&self) -> String {
        let c = &self.coverage;
        let (na, nb) = (c.a().name.clone(), c.b().name.clone());
        let mut o = String::new();
        o.push_str(&format!("## {na} {} {nb}\n\n", glyph::ARROW));

        let d = c.delta();
        // U+2212, not an ASCII hyphen: this sits beside a "+" in a markdown
        // table and the two have to be the same visual weight -- the same rule
        // `history::ComparablePredecessor::delta` follows.
        let signed = if d >= 0 {
            format!("+{d}")
        } else {
            format!("{}{}", glyph::MINUS, -d)
        };
        o.push_str(&format!(
            "**Coverage** {} {na} {}/{}, {nb} {}/{} (**{signed}**)\n\n",
            glyph::EM_DASH,
            c.a_solved(),
            c.a().len(),
            c.b_solved(),
            c.b().len()
        ));

        if c.is_regression() {
            o.push_str(&format!(
                "> **REGRESSION {} {} lost**\n>\n",
                glyph::EM_DASH,
                c.lost().len()
            ));
            for l in c.lost() {
                let now = match l.now {
                    Some(cl) => cl.to_string(),
                    None => "gone from the board".to_string(),
                };
                o.push_str(&format!(
                    "> - `{}` {} {} in {na}, now **{now}**\n",
                    c.label(&l.key),
                    glyph::EM_DASH,
                    l.was
                ));
            }
            o.push('\n');
        } else {
            o.push_str(&format!(
                "No regressions {} nothing solved on {na} was lost.\n\n",
                glyph::EM_DASH
            ));
        }

        if !c.gained().is_empty() {
            o.push_str(&format!("**Gained ({})**\n\n", c.gained().len()));
            for g in c.gained() {
                let was = match g.was {
                    Some(cl) => cl.to_string(),
                    None => "not on the board".to_string(),
                };
                o.push_str(&format!(
                    "- `{}` {} {was} in {na}, now **{}**\n",
                    c.label(&g.key),
                    glyph::EM_DASH,
                    g.now
                ));
            }
            o.push('\n');
        }

        let q = &self.quality;
        o.push_str(&format!(
            "**Quality** {} over the {} problems solved by both ({} cost-comparable): \
             {na} cheaper {}, {nb} cheaper {}, equal {}.",
            glyph::EM_DASH,
            q.common(),
            q.scored(),
            q.cheaper_a(),
            q.cheaper_b(),
            q.equal()
        ));
        match (q.mean_a(), q.mean_b()) {
            (Some(ma), Some(mb)) => o.push_str(&format!(
                " Mean cost {na} {}, {nb} {}.\n\n",
                fmt_f(ma, 2),
                fmt_f(mb, 2)
            )),
            _ => o.push_str(" Mean cost not measured.\n\n"),
        }

        o.push_str(&format!(
            "**Timing** {} {}\n\n",
            glyph::EM_DASH,
            self.timing
        ));

        if let Some(t) = &self.trend {
            o.push_str(&format!(
                "**Trend** {} {}\n\n",
                glyph::EM_DASH,
                render_trend(t)
            ));
        }

        o.push_str(&q.table());
        o
    }

    /// The structured form.
    ///
    /// Key order is `serde_json`'s (sorted), unlike a board raw, where the
    /// order is the Python dict's insertion order and is load-bearing. Nothing
    /// downstream of this reads it positionally and there is no oracle to match,
    /// so the sorted order is a feature: two runs of this produce the same bytes
    /// regardless of how the structs were filled.
    pub fn to_json(&self) -> String {
        use serde_json::{Map, Value};

        let c = &self.coverage;
        let flip = |key: &InstanceKey, was: Option<Class>, now: Option<Class>| -> Value {
            let mut m = Map::new();
            m.insert(
                "ipc".to_string(),
                match &key.ipc {
                    Some(s) => Value::String(s.clone()),
                    None => Value::Null,
                },
            );
            m.insert("variant".to_string(), Value::String(key.variant.clone()));
            m.insert("instance".to_string(), inst_json(&key.instance));
            m.insert(
                "was".to_string(),
                was.map_or(Value::Null, |x| Value::String(x.label().to_string())),
            );
            m.insert(
                "now".to_string(),
                now.map_or(Value::Null, |x| Value::String(x.label().to_string())),
            );
            Value::Object(m)
        };

        let mut cov = Map::new();
        cov.insert("a_solved".to_string(), c.a_solved().into());
        cov.insert("a_total".to_string(), c.a().len().into());
        cov.insert("b_solved".to_string(), c.b_solved().into());
        cov.insert("b_total".to_string(), c.b().len().into());
        cov.insert("delta".to_string(), c.delta().into());
        cov.insert("common".to_string(), c.common().len().into());
        cov.insert("regression".to_string(), c.is_regression().into());
        cov.insert(
            "lost".to_string(),
            Value::Array(
                c.lost()
                    .iter()
                    .map(|l| flip(&l.key, Some(l.was), l.now))
                    .collect(),
            ),
        );
        cov.insert(
            "gained".to_string(),
            Value::Array(
                c.gained()
                    .iter()
                    .map(|g| flip(&g.key, g.was, Some(g.now)))
                    .collect(),
            ),
        );

        let q = &self.quality;
        let mut qual = Map::new();
        qual.insert("common".to_string(), q.common().into());
        qual.insert("scored".to_string(), q.scored().into());
        qual.insert("cheaper_a".to_string(), q.cheaper_a().into());
        qual.insert("cheaper_b".to_string(), q.cheaper_b().into());
        qual.insert("equal".to_string(), q.equal().into());
        qual.insert("total_a".to_string(), num(q.total_a()));
        qual.insert("total_b".to_string(), num(q.total_b()));
        qual.insert("mean_a".to_string(), q.mean_a().map_or(Value::Null, num));
        qual.insert("mean_b".to_string(), q.mean_b().map_or(Value::Null, num));
        qual.insert(
            "variants".to_string(),
            Value::Array(
                q.variants()
                    .iter()
                    .map(|v| {
                        let mut m = Map::new();
                        m.insert("variant".to_string(), Value::String(v.label.clone()));
                        m.insert("both".to_string(), v.n.into());
                        m.insert("cheaper_a".to_string(), v.cheaper_a.into());
                        m.insert("cheaper_b".to_string(), v.cheaper_b.into());
                        m.insert("time_a".to_string(), num(v.time_a));
                        m.insert("time_b".to_string(), num(v.time_b));
                        Value::Object(m)
                    })
                    .collect(),
            ),
        );

        let t = &self.timing;
        let mut tim = Map::new();
        tim.insert("clock".to_string(), Value::String(t.clock().to_string()));
        tim.insert("clean".to_string(), t.clean().into());
        tim.insert("dirty_excluded".to_string(), t.dirty().into());
        tim.insert("unstamped_excluded".to_string(), t.unstamped().into());
        match t.percentiles() {
            Some((a, b)) => {
                tim.insert("a_p50_ms".to_string(), num(a.p50));
                tim.insert("a_p95_ms".to_string(), num(a.p95));
                tim.insert("b_p50_ms".to_string(), num(b.p50));
                tim.insert("b_p95_ms".to_string(), num(b.p95));
            }
            None => {
                for k in ["a_p50_ms", "a_p95_ms", "b_p50_ms", "b_p95_ms"] {
                    tim.insert(k.to_string(), Value::Null);
                }
            }
        }

        let mut top = Map::new();
        top.insert("a".to_string(), Value::String(c.a().name.clone()));
        top.insert("b".to_string(), Value::String(c.b().name.clone()));
        top.insert("coverage".to_string(), Value::Object(cov));
        top.insert("quality".to_string(), Value::Object(qual));
        top.insert("timing".to_string(), Value::Object(tim));
        if let Some(tr) = &self.trend {
            let mut m = Map::new();
            m.insert("label".to_string(), Value::String(tr.label.clone()));
            m.insert(
                "points".to_string(),
                Value::Array(
                    tr.points
                        .iter()
                        .map(|(v, (s, n))| {
                            let mut p = Map::new();
                            p.insert("version".to_string(), Value::String(v.to_string()));
                            p.insert("solved".to_string(), (*s).into());
                            p.insert("total".to_string(), (*n).into());
                            Value::Object(p)
                        })
                        .collect(),
                ),
            );
            top.insert("trend".to_string(), Value::Object(m));
        }

        let mut out = String::new();
        crate::pyjson::write_indent1(&Value::Object(top), &mut out);
        out.push('\n');
        out
    }
}

fn inst_json(i: &Instance) -> serde_json::Value {
    match i {
        Instance::Num(n) => (*n).into(),
        Instance::Parts(s) => serde_json::Value::String(s.clone()),
    }
}

/// A finite float, or `null`.
///
/// Python's `json.dumps` writes a bare `NaN`/`Infinity`, which is not JSON and
/// which nothing downstream could read anyway. Unreachable from a real board;
/// degrading to `null` keeps the document parseable if it ever is not.
fn num(x: f64) -> serde_json::Value {
    serde_json::Number::from_f64(x).map_or(serde_json::Value::Null, serde_json::Value::Number)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::referee::ValUnavailable;

    const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../..");
    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures");

    /// The real referee, from the committed VAL-availability probe.
    fn referee() -> Referee {
        let src = std::fs::read_to_string(format!("{REPO}/benchmarks/val-unavailable.json"))
            .expect("benchmarks/val-unavailable.json is committed");
        let v: serde_json::Value = serde_json::from_str(&src).expect("well-formed");
        let keys = v["unavailable"]
            .as_object()
            .expect("an object")
            .keys()
            .cloned();
        Referee::new(ValUnavailable::new(keys))
    }

    fn board(dir: &str, file: &str, budget: f64, name: &str) -> Loaded {
        let path = format!("{REPO}/benchmarks/{dir}/{file}");
        let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        RunRef::from_jsonl(name, budget, &src, &path).expect("a committed board parses")
    }

    /// One of the rescued incident fixtures, in its exact source bytes.
    fn fixture_rows(rel: &str) -> Vec<RawRow> {
        let path = format!("{FIXTURES}/{rel}");
        let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        crate::parse_rows(&src, &path).expect("a committed fixture parses")
    }

    fn pair(file: &str, budget: f64) -> (RunRef, RunRef) {
        (
            board("air-0.19.0", file, budget, "0.19.0").run,
            board("air-0.21.0", file, budget, "0.21.0").run,
        )
    }

    fn row(ipc: Option<&str>, variant: &str, inst: u64, solved: bool, time: f64) -> RawRow {
        let length = if solved {
            serde_json::json!(10)
        } else {
            serde_json::Value::Null
        };
        let v = serde_json::json!({
            "ipc": ipc,
            "variant": variant,
            "instance": inst,
            "solved": solved,
            "time": time,
            "metric": null,
            "length": length,
            "val": null,
            "notes": null,
        });
        serde_json::from_value(v).expect("a hand-built row parses")
    }

    fn key(variant: &str, inst: u64) -> InstanceKey {
        InstanceKey {
            ipc: None,
            variant: variant.to_string(),
            instance: Instance::Num(inst),
        }
    }

    // -----------------------------------------------------------------------
    // The join key
    // -----------------------------------------------------------------------

    /// Defends the key widening: the seed joins `(variant, instance)`, so two
    /// competitions sharing a variant name would silently become one problem.
    #[test]
    fn key_carries_the_competition() {
        let a = RunRef::from_rows(
            "A",
            60.0,
            vec![
                row(Some("ipc-2008"), "sokoban-sequential-optimal", 1, true, 1.0),
                row(
                    Some("ipc-2011"),
                    "sokoban-sequential-optimal",
                    1,
                    false,
                    60.0,
                ),
            ],
        )
        .run;
        let b = RunRef::from_rows(
            "B",
            60.0,
            vec![
                row(
                    Some("ipc-2008"),
                    "sokoban-sequential-optimal",
                    1,
                    false,
                    60.0,
                ),
                row(Some("ipc-2011"), "sokoban-sequential-optimal", 1, true, 1.0),
            ],
        )
        .run;
        assert_eq!(a.len(), 2, "two competitions, two rows, two keys");
        let r = referee();
        let d = CoverageDiff::new(&r, &a, &b);
        // Under the seed's narrower key these two rows are one problem and the
        // diff is empty. Under this key it is one loss and one gain.
        assert_eq!(d.lost().len(), 1);
        assert_eq!(d.gained().len(), 1);
        assert_eq!(d.lost()[0].key.ipc.as_deref(), Some("ipc-2008"));
        assert_eq!(d.gained()[0].key.ipc.as_deref(), Some("ipc-2011"));
        // And the labels are qualified, because the bare variant name is now
        // ambiguous and two table rows must not share it.
        assert_eq!(
            d.label(&d.lost()[0].key),
            "ipc-2008/sokoban-sequential-optimal/1"
        );
    }

    /// Defends the total order the seed's tuple lacks: `sorted()` on a board of
    /// mixed int and string instance labels raises `TypeError` in Python, which
    /// is why `ipc67-diff.py` cannot run on `ipc2026-numeric` at all.
    #[test]
    fn mixed_instance_labels_have_a_total_order() {
        let num = InstanceKey {
            ipc: None,
            variant: "v".into(),
            instance: Instance::Num(7),
        };
        let parts = InstanceKey {
            ipc: None,
            variant: "v".into(),
            instance: Instance::Parts("3_10_50_10".into()),
        };
        assert_eq!(num.cmp(&parts), Ordering::Less);
        assert_eq!(parts.cmp(&num), Ordering::Greater);
        assert_ne!(num, parts, "3 and \"3\" are different problems");
        // The real board the seed dies on.
        let b = board("air-0.21.0", "ipc2026-numeric.jsonl", 60.0, "0.21.0");
        assert_eq!(b.run.len(), 320);
        assert!(b.collapsed.is_empty());
    }

    /// Defends the collision report. `air-0.19.0/ipc2026-numeric.jsonl` really
    /// is 320 rows under 288 keys -- the first-group-only label bug -- and a
    /// diff that silently reported 288 rows would be wrong twice.
    #[test]
    fn flattened_labels_are_reported_not_swallowed() {
        let l = board("air-0.19.0", "ipc2026-numeric.jsonl", 60.0, "0.19.0");
        assert_eq!(l.run.len(), 288, "the 0.19-era raw keys to 288");
        assert_eq!(l.collapsed.len(), 32, "and loses 32 rows doing it");
    }

    // -----------------------------------------------------------------------
    // Coverage, against the two committed backfill sets
    // -----------------------------------------------------------------------

    /// The headline of the seed's own output on a real board, reproduced.
    #[test]
    fn coverage_matches_the_seed_where_the_referee_agrees() {
        let (a, b) = pair("ipc2018-sat.jsonl", 60.0);
        let r = referee();
        let d = CoverageDiff::new(&r, &a, &b);
        assert_eq!((d.a_solved(), a.len()), (63, 240));
        assert_eq!((d.b_solved(), b.len()), (69, 240));
        assert_eq!(d.common().len(), 61);
        assert_eq!(d.lost().len(), 2);
        assert_eq!(d.gained().len(), 8);
        assert_eq!(d.delta(), 6);
    }

    /// The named losses, with the classes that make the sentence say something.
    /// Two `data-network` instances solved at 0.19.0 spend the wall at 0.21.0 --
    /// and `data-network` is a VAL-unavailable domain, so the raw `val: false`
    /// on those rows must NOT be read as a rejection.
    #[test]
    fn losses_are_named_with_was_and_now() {
        let (a, b) = pair("ipc2018-sat.jsonl", 60.0);
        let r = referee();
        let d = CoverageDiff::new(&r, &a, &b);
        let named: Vec<String> = d.lost().iter().map(|l| l.key.label()).collect();
        assert_eq!(
            named,
            vec![
                "data-network-sequential-satisficing/6",
                "data-network-sequential-satisficing/17",
            ]
        );
        for l in d.lost() {
            assert_eq!(l.was, Class::Solved);
            assert_eq!(l.now, Some(Class::Timeout));
        }
        assert!(d.is_regression());
    }

    /// The optimal board's six losses, one per domain per competition -- and
    /// the two competitions' variants are distinct keys, so all six are named.
    #[test]
    fn optimal_board_names_all_six_losses() {
        let (a, b) = pair("ipc-opt-2008-11.jsonl", 60.0);
        let r = referee();
        let d = CoverageDiff::new(&r, &a, &b);
        let named: Vec<String> = d.lost().iter().map(|l| l.key.qualified()).collect();
        assert_eq!(
            named,
            vec![
                "ipc-2008/parc-printer-sequential-optimal-strips/14",
                "ipc-2008/peg-solitaire-sequential-optimal-strips/26",
                "ipc-2008/sokoban-sequential-optimal-strips/22",
                "ipc-2011/parc-printer-sequential-optimal/12",
                "ipc-2011/peg-solitaire-sequential-optimal/16",
                "ipc-2011/sokoban-sequential-optimal/19",
            ]
        );
        assert!(d.lost().iter().all(|l| l.now == Some(Class::Timeout)));
        assert_eq!(d.gained().len(), 47);
    }

    /// **The reconciliation.** `ipc67-diff.py` reads `r["solved"]` and so books
    /// `map-analyzer` 17, 18 and 20 as "solved by both" -- no movement. They
    /// were VAL-REJECTED at 0.19.0 and valid at 0.21.0: three plan-soundness
    /// fixes the seed cannot see. The referee reports 5 gains where the seed
    /// reports 2.
    #[test]
    fn val_rejected_plans_are_not_coverage() {
        let (a, b) = pair("ipc2014-tempo.jsonl", 60.0);
        let r = referee();
        let d = CoverageDiff::new(&r, &a, &b);

        // What the seed would say, computed the seed's way.
        let raw_a = a.rows.values().filter(|x| x.solved).count();
        let raw_b = b.rows.values().filter(|x| x.solved).count();
        assert_eq!((raw_a, raw_b), (68, 70));
        assert_eq!(
            (d.a_solved(), d.b_solved()),
            (65, 70),
            "three A rows carry plans VAL rejected"
        );
        assert_eq!(d.gained().len(), 5, "the seed sees only 2 of these");
        assert!(d.lost().is_empty());

        let red: Vec<&Gained> = d
            .gained()
            .iter()
            .filter(|g| g.was == Some(Class::ValRed))
            .collect();
        let names: Vec<String> = red.iter().map(|g| g.key.label()).collect();
        assert_eq!(
            names,
            vec![
                "map-analyzer-temporal-satisficing/17",
                "map-analyzer-temporal-satisficing/18",
                "map-analyzer-temporal-satisficing/20",
            ]
        );
        assert!(red.iter().all(|g| g.now == Class::Solved));
    }

    /// **Fifteen instances light, seen from the comparison side.** VAL refuses
    /// to INGEST `data-network-2018` and `factory-robot-2026`, so the `val:
    /// false` on these fifteen committed rows is validation UNAVAILABLE, not a
    /// rejected plan. A referee that does not know that never counts them as
    /// coverage -- and a diff built on it reports NO regression on a board where
    /// fifteen solved problems vanished, which is the quietest way this record
    /// can be wrong.
    #[test]
    fn an_empty_val_map_hides_a_regression() {
        let rows = fixture_rows("incidents/val-unavailable-15.jsonl");
        assert_eq!(rows.len(), 15);
        let before = RunRef::from_rows("before", 60.0, rows).run;
        let after = RunRef::from_rows("after", 60.0, Vec::new()).run;

        let real = referee();
        let seen = CoverageDiff::new(&real, &before, &after);
        assert_eq!(seen.lost().len(), 15);
        assert!(seen.is_regression());

        let blind = Referee::default();
        let missed = CoverageDiff::new(&blind, &before, &after);
        assert_eq!(missed.a_solved(), 0, "VAL's refusals read as rejections");
        assert!(
            !missed.is_regression(),
            "and fifteen lost problems become invisible"
        );
    }

    // -----------------------------------------------------------------------
    // Quality
    // -----------------------------------------------------------------------

    /// The per-variant table, byte-for-byte in the seed's shape. Generated by
    /// running `benchmarks/ipc67-diff.py` over the same two committed boards.
    #[test]
    fn variant_table_reproduces_the_seed() {
        let (a, b) = pair("ipc2023-agile.jsonl", 60.0);
        let r = referee();
        let q = CoverageDiff::new(&r, &a, &b).quality();
        assert_eq!(
            q.table(),
            "\
| variant | both | 0.19.0 cheaper | 0.21.0 cheaper | 0.19.0 time | 0.21.0 time |
|---|---|---|---|---|---|
| quantum-layout-agile | 17 | 8 | 0 | 92s | 45s |
| recharging-robots-agile | 4 | 0 | 0 | 70s | 67s |
| ricochet-robots-agile | 2 | 0 | 0 | 95s | 92s |
| rubiks-cube-agile | 5 | 0 | 0 | 7s | 12s |
| slitherlink-agile | 2 | 0 | 0 | 15s | 13s |
"
        );
    }

    /// Quality is computed over the commonly-solved set and nothing else.
    #[test]
    fn quality_is_over_the_common_set() {
        let (a, b) = pair("ipc2018-sat.jsonl", 60.0);
        let r = referee();
        let d = CoverageDiff::new(&r, &a, &b);
        let q = d.quality();
        assert_eq!(q.common(), 61);
        assert_eq!(q.scored(), 61);
        assert_eq!((q.cheaper_a(), q.cheaper_b(), q.equal()), (10, 1, 50));
        assert_eq!(fmt_f(q.mean_a().unwrap(), 2), "353.90");
        assert_eq!(fmt_f(q.mean_b().unwrap(), 2), "359.28");
        // The counts partition the scored set -- no row is counted twice and
        // none is dropped.
        assert_eq!(q.cheaper_a() + q.cheaper_b() + q.equal(), q.scored());
    }

    /// The structural rule: a `QualityDiff` cannot be built beside a coverage
    /// diff, only from one. This is a compile-time property, so the test states
    /// it and checks the one observable consequence -- the two agree on n.
    #[test]
    fn quality_cannot_outlive_its_coverage_set() {
        let (a, b) = pair("ipc2023-agile.jsonl", 60.0);
        let r = referee();
        let d = CoverageDiff::new(&r, &a, &b);
        let q = d.quality();
        assert_eq!(q.common(), d.common().len());
        let table_n: usize = q.variants().iter().map(|v| v.n).sum();
        assert_eq!(
            table_n,
            d.common().len(),
            "every commonly-solved problem lands in exactly one table row"
        );
    }

    /// A cost of 0.0 is a cost. `Cost::of` tests `is not None`, never
    /// truthiness -- the opposite of `standings.py`'s reference loader, which
    /// is a different function with a different job.
    #[test]
    fn zero_metric_is_a_cost() {
        let mut r = row(None, "v", 1, true, 1.0);
        r.metric = Some(0.0);
        assert_eq!(Cost::of(&r), Some(Cost::Metric(0.0)));
        // And a metric wins over a length, even at zero.
        assert_eq!(Cost::of(&r).map(|c| c.value()), Some(0.0));
    }

    /// The seed prints a metric as a float and a length as an int, because
    /// that is what Python's `str()` gave it. A pasted cut record has to keep
    /// reading the same way.
    #[test]
    fn cost_renders_like_python() {
        assert_eq!(Cost::Metric(1303.0).to_string(), "1303.0");
        assert_eq!(Cost::Length(107).to_string(), "107");
        assert_eq!(Cost::Metric(37.5).to_string(), "37.5");
    }

    /// `str(float)` is not `format!("{}")`: CPython switches to exponential
    /// notation at `decpt <= -4` and `decpt > 16`, and Rust's `Display` never
    /// does. Every value here was read back out of `python3 -c 'print(str(x))'`.
    /// Today's corpus stops at `5048589.0`, so this is the boundary being held
    /// before a board reaches it, not one being repaired after.
    #[test]
    fn a_cost_past_the_exponent_boundary_still_reads_like_python() {
        // Fixed on both sides of the high switch: decpt 16 stays fixed, 17 goes
        // exponential.
        assert_eq!(py_repr_f(1e15), "1000000000000000.0");
        assert_eq!(py_repr_f(9999999999999998.0), "9999999999999998.0");
        assert_eq!(py_repr_f(1e16), "1e+16");
        assert_eq!(py_repr_f(1.5e16), "1.5e+16");
        assert_eq!(py_repr_f(1e100), "1e+100");
        assert_eq!(py_repr_f(-1e16), "-1e+16");
        // And the low switch: decpt -3 stays fixed, -4 goes exponential. The
        // exponent is signed and padded to two digits.
        assert_eq!(py_repr_f(1e-4), "0.0001");
        assert_eq!(py_repr_f(1e-5), "1e-05");
        assert_eq!(py_repr_f(1.23e-7), "1.23e-07");
        // The values a real board actually carries are untouched.
        assert_eq!(py_repr_f(0.0), "0.0");
        assert_eq!(py_repr_f(1020512.0), "1020512.0");
        assert_eq!(py_repr_f(5048589.0), "5048589.0");
        assert_eq!(py_repr_f(0.1), "0.1");
        // The non-finite spellings Python uses, closed rather than left as a
        // landmine -- Rust says `NaN` and `inf`.
        assert_eq!(py_repr_f(f64::NAN), "nan");
        assert_eq!(py_repr_f(f64::INFINITY), "inf");
        assert_eq!(py_repr_f(f64::NEG_INFINITY), "-inf");
    }

    // -----------------------------------------------------------------------
    // Percentiles
    // -----------------------------------------------------------------------

    /// Nearest rank, `sorted[ceil(q*n) - 1]` 1-based, at the sizes named in the
    /// spec for this module. Every answer is a value that was measured.
    #[test]
    fn percentiles_are_nearest_rank() {
        let mk = |n: usize| -> Vec<f64> { (1..=n).map(|i| i as f64).collect() };

        let p = Percentiles::of(&mk(1));
        assert_eq!((p.p50, p.p95), (1.0, 1.0));

        let p = Percentiles::of(&mk(2));
        assert_eq!((p.p50, p.p95), (1.0, 2.0));

        let p = Percentiles::of(&mk(3));
        assert_eq!((p.p50, p.p95), (2.0, 3.0));

        let p = Percentiles::of(&mk(20));
        assert_eq!((p.p50, p.p95), (10.0, 19.0));

        let p = Percentiles::of(&mk(100));
        assert_eq!((p.p50, p.p95), (50.0, 95.0));
    }

    /// The rank is integer arithmetic, not `ceil(q * n)` in floating point.
    /// They agree for every n up to 200,000 today; the point is that the
    /// published number does not depend on that continuing to be true.
    #[test]
    fn rank_arithmetic_is_exact() {
        for n in 1u64..=2000 {
            let v: Vec<f64> = (1..=n).map(|i| i as f64).collect();
            let want50 = n.div_ceil(2);
            let want95 = (n * 95).div_ceil(100);
            assert_eq!(nearest_rank(&v, 1, 2), want50 as f64, "p50 at n={n}");
            assert_eq!(nearest_rank(&v, 95, 100), want95 as f64, "p95 at n={n}");
        }
    }

    // -----------------------------------------------------------------------
    // Cleanliness
    // -----------------------------------------------------------------------

    fn conditions(name: &str) -> Conditions {
        Conditions::load(Path::new(&format!("{FIXTURES}/conditions/{name}")))
    }

    /// A real 0.25 board against its real per-sample timeline: 280 rows split
    /// 175 clean / 105 dirty. Both classes are present, so neither the
    /// threshold nor the window intersection can be no-ops.
    #[test]
    fn cleanliness_splits_a_real_board() {
        let l = board("air25-entries", "ipc2014-mco-t2.jsonl", 60.0, "entries");
        let c = conditions("timeline-mco-t2.json");
        assert!(c.has_timeline());
        let mut clean = 0;
        let mut dirty = 0;
        let mut unknown = 0;
        for r in l.run.rows.values() {
            match c.cleanliness(r) {
                Cleanliness::Clean => clean += 1,
                Cleanliness::Dirty => dirty += 1,
                Cleanliness::Unknown => unknown += 1,
            }
        }
        assert_eq!((clean, dirty, unknown), (175, 105, 0));
    }

    /// **Unknown is not clean.** The committed `air25-entries` conditions files
    /// carry no timeline -- 72 of the 76 on this box do not -- so every row on
    /// that board is genuinely unstamped, and the gate fails closed rather than
    /// treating an unwatched run as a quiet one.
    #[test]
    fn no_timeline_is_unknown_not_clean() {
        let l = board("air25-entries", "ipc2018-opt.jsonl", 60.0, "entries");
        let c = Conditions::load(Path::new(&format!(
            "{REPO}/benchmarks/air25-entries/ipc2018-opt.conditions.json"
        )));
        assert!(
            !c.has_timeline(),
            "this committed file is a rollup with no per-sample record"
        );
        assert_eq!(l.run.len(), 240);
        assert!(l
            .run
            .rows
            .values()
            .all(|r| c.cleanliness(r) == Cleanliness::Unknown));
    }

    /// A missing conditions file degrades to "nothing was recorded", the way a
    /// missing input file degrades everywhere else here -- not to "clean".
    #[test]
    fn a_missing_conditions_file_is_unknown() {
        let c = Conditions::load(Path::new("/nonexistent/conditions.json"));
        assert!(!c.has_timeline());
        let mut r = row(None, "v", 1, true, 1.0);
        r.start_ts = Some(100.0);
        r.end_ts = Some(200.0);
        assert_eq!(c.cleanliness(&r), Cleanliness::Unknown);
        // And so does a file that is not JSON at all.
        assert!(!Conditions::from_json("not json").has_timeline());
    }

    /// The per-sample rule, not the run median: one contended sample anywhere
    /// in the padded window is enough. This is the whole reason the resume gate
    /// reads a timeline instead of the rollup's verdict.
    #[test]
    fn one_dirty_sample_condemns_the_window() {
        let src = r#"{"interval": 20,
          "timeline": [[1000.0, 90.0, 1.0], [1020.0, 90.0, 2.0],
                       [1040.0, 10.0, 40.0], [1060.0, 90.0, 3.0]]}"#;
        let c = Conditions::from_json(src);
        let mut r = row(None, "v", 1, true, 1.0);
        r.start_ts = Some(1005.0);
        r.end_ts = Some(1050.0);
        // Three of the four overlapping samples are quiet; the run median, and
        // the rollup verdict computed from it, would have passed this window.
        assert_eq!(c.cleanliness(&r), Cleanliness::Dirty);
        // A window that ends before the contention starts is clean, because the
        // padding is ONE interval either side and not the whole board -- which
        // is the entire reason a timeline beats a whole-board retry.
        r.start_ts = Some(1000.0);
        r.end_ts = Some(1005.0);
        assert_eq!(c.cleanliness(&r), Cleanliness::Clean);
    }

    /// A sample the watcher could not attribute counts against the row --
    /// `t[2] is None or t[2] >= SAMPLE_CLEAN_PCPU`, both halves.
    #[test]
    fn an_unattributed_sample_is_dirty() {
        let src = r#"{"interval": 20, "timeline": [[1000.0, 90.0, null]]}"#;
        let c = Conditions::from_json(src);
        let mut r = row(None, "v", 1, true, 1.0);
        r.start_ts = Some(1000.0);
        r.end_ts = Some(1001.0);
        assert_eq!(c.cleanliness(&r), Cleanliness::Dirty);
    }

    /// **An unreadable column refuses the whole file, it does not drop the
    /// sample.** Dropping is the one answer that fails OPEN: the sample thrown
    /// away may be the contended one, and the window it would have condemned
    /// then reads `Clean`. Python raises out of `load_resume` on this input and
    /// `crucible_core::sched::resume::Conditions::parse` refuses the file; this
    /// gate has to agree with that one, so it refuses too.
    #[test]
    fn an_unreadable_timeline_column_refuses_the_file() {
        let mut r = row(None, "v", 1, true, 1.0);
        r.start_ts = Some(1000.0);
        r.end_ts = Some(1005.0);

        // The dirty sample is the one with the unreadable timestamp. Drop it
        // and this window reads Clean off the two quiet neighbours.
        let bad_ts = r#"{"interval": 20, "timeline": [
            [1000.0, 90.0, 1.0], ["1002.0", 10.0, 99.0], [1004.0, 90.0, 1.0]]}"#;
        let c = Conditions::from_json(bad_ts);
        assert!(!c.has_timeline(), "the file is refused, not partly read");
        assert_eq!(c.cleanliness(&r), Cleanliness::Unknown);

        // Same for a load column that is present and not a number. (A `null`
        // load is a VALUE -- the watcher observed nothing -- and stays dirty;
        // see `an_unattributed_sample_is_dirty`.)
        let bad_load = r#"{"interval": 20, "timeline": [[1000.0, 90.0, "99.0"]]}"#;
        assert!(!Conditions::from_json(bad_load).has_timeline());
        let bad_idle = r#"{"interval": 20, "timeline": [[1000.0, "90.0", 1.0]]}"#;
        assert!(!Conditions::from_json(bad_idle).has_timeline());

        // A null TIMESTAMP is still the Python filter's own drop, not a
        // refusal: `t[0] is not None` is where `load_resume` discards it.
        let null_ts = r#"{"interval": 20, "timeline": [
            [null, 90.0, 99.0], [1000.0, 90.0, 1.0], [1004.0, 90.0, 1.0]]}"#;
        let c = Conditions::from_json(null_ts);
        assert!(c.has_timeline());
        assert_eq!(c.cleanliness(&r), Cleanliness::Clean);
    }

    /// A window that runs off the end of the sampled span is unobserved, not
    /// clean: the watcher started late or died early.
    #[test]
    fn a_window_outside_the_span_is_unknown() {
        let src = r#"{"interval": 20, "timeline": [[1000.0, 90.0, 1.0], [1020.0, 90.0, 1.0]]}"#;
        let c = Conditions::from_json(src);
        let mut r = row(None, "v", 1, true, 1.0);
        r.start_ts = Some(1000.0);
        r.end_ts = Some(5000.0);
        assert_eq!(c.cleanliness(&r), Cleanliness::Unknown);
        r.start_ts = Some(1.0);
        r.end_ts = Some(1010.0);
        assert_eq!(c.cleanliness(&r), Cleanliness::Unknown);
    }

    /// A row stitched clean by an earlier pass carries the judgment itself --
    /// its own conditions file is gone by now, and the stamp IS the record.
    #[test]
    fn resumed_clean_is_believed() {
        let c = Conditions::none();
        let mut r = row(None, "v", 1, true, 1.0);
        r.resumed_clean = true;
        assert_eq!(c.cleanliness(&r), Cleanliness::Clean);
    }

    /// The threshold is a strict `>=` at 25.0, in the same currency as
    /// `contention.py`'s whole-run verdict. If this ever disagrees with
    /// `crucible_core::monitor::SAMPLE_CLEAN_PCPU` the two rules have drifted.
    #[test]
    fn the_threshold_is_the_shared_line() {
        assert_eq!(SAMPLE_CLEAN_PCPU, 25.0);
        let just_under = r#"{"interval": 20, "timeline": [[1000.0, 90.0, 24.9]]}"#;
        let at_the_line = r#"{"interval": 20, "timeline": [[1000.0, 90.0, 25.0]]}"#;
        let mut r = row(None, "v", 1, true, 1.0);
        r.start_ts = Some(1000.0);
        r.end_ts = Some(1001.0);
        assert_eq!(
            Conditions::from_json(just_under).cleanliness(&r),
            Cleanliness::Clean
        );
        assert_eq!(
            Conditions::from_json(at_the_line).cleanliness(&r),
            Cleanliness::Dirty
        );
    }

    // -----------------------------------------------------------------------
    // Timing
    // -----------------------------------------------------------------------

    /// Three counts, never two, on real stamped rows: 157 commonly-solved,
    /// 99 clean, 58 excluded dirty.
    #[test]
    fn timing_shows_all_three_exclusion_counts() {
        let l = board("air25-entries", "ipc2014-mco-t2.jsonl", 60.0, "entries");
        let c = conditions("timeline-mco-t2.json");
        let r = referee();
        // The same board on both sides: the commonly-solved set is its solved
        // set, and the exclusion accounting is what is under test.
        let d = CoverageDiff::new(&r, &l.run, &l.run);
        assert_eq!(d.common().len(), 157);
        let t = d.timing(&c, &c);
        assert_eq!((t.clean(), t.dirty(), t.unstamped()), (99, 58, 0));
        let (pa, pb) = t.percentiles().expect("99 clean pairs");
        assert!((pa.p50 - 6360.0).abs() < 1e-6);
        assert!((pa.p95 - 52530.0).abs() < 1e-6);
        assert_eq!((pa.p50, pa.p95), (pb.p50, pb.p95));
        assert!(t
            .to_string()
            .contains("99 clean, 58 excluded dirty, 0 excluded unstamped"));
    }

    /// The wall-clock confession. No committed raw carries `cpu_ms`, so every
    /// timing on the backfill sets is wall time and has to say so rather than
    /// wear a CPU number's authority.
    #[test]
    fn a_wall_fallback_confesses() {
        let l = board("air25-entries", "ipc2014-mco-t2.jsonl", 60.0, "entries");
        let c = conditions("timeline-mco-t2.json");
        let r = referee();
        let t = CoverageDiff::new(&r, &l.run, &l.run).timing(&c, &c);
        assert_eq!(t.clock(), Clock::Wall);
        assert!(t.to_string().contains("no cpu_ms"));
    }

    /// `cpu_ms` wins where a row carries it, and a board carrying both has to
    /// say the percentile is mixed -- half a CPU distribution and half a wall
    /// one is not a distribution of anything.
    #[test]
    fn cpu_ms_wins_and_a_mixed_board_says_so() {
        let mut with_cpu = row(None, "v", 1, true, 9.0);
        with_cpu
            .extra
            .insert("cpu_ms".into(), serde_json::json!(1234));
        assert_eq!(row_ms(&with_cpu), Some((1234.0, RowClock::Cpu)));

        let plain = row(None, "v", 2, true, 2.0);
        assert_eq!(row_ms(&plain), Some((2000.0, RowClock::Wall)));

        let mut r0 = with_cpu.clone();
        r0.resumed_clean = true;
        let mut r1 = plain.clone();
        r1.resumed_clean = true;
        let run = RunRef::from_rows("X", 60.0, vec![r0, r1]).run;
        let r = referee();
        let t = CoverageDiff::new(&r, &run, &run).timing(&Conditions::none(), &Conditions::none());
        assert_eq!(t.clock(), Clock::Mixed { cpu: 2, wall: 2 });
        assert!(t.to_string().contains("MIXED"));
    }

    /// The backfill sets have no conditions files at all, so a diff of them
    /// reports zero clean runs and says why -- rather than quoting a percentile
    /// over 61 unwatched runs.
    #[test]
    fn unwatched_boards_yield_no_percentiles() {
        let (a, b) = pair("ipc2018-sat.jsonl", 60.0);
        let r = referee();
        let t = CoverageDiff::new(&r, &a, &b).timing(&Conditions::none(), &Conditions::none());
        assert_eq!((t.clean(), t.dirty(), t.unstamped()), (0, 0, 61));
        assert!(t.percentiles().is_none());
        assert!(t.to_string().contains("not computed"));
    }

    /// A dirty side and an unstamped side exclude the pair as DIRTY: the
    /// nameable reason wins.
    #[test]
    fn a_pair_takes_the_worse_verdict() {
        use Cleanliness::*;
        assert_eq!(Clean.worse(Clean), Clean);
        assert_eq!(Clean.worse(Dirty), Dirty);
        assert_eq!(Unknown.worse(Dirty), Dirty);
        assert_eq!(Dirty.worse(Unknown), Dirty);
        assert_eq!(Clean.worse(Unknown), Unknown);
    }

    // -----------------------------------------------------------------------
    // Trend
    // -----------------------------------------------------------------------

    /// A trend is one board on ONE box. `History::trend` filters before it
    /// collects, and nothing in this module takes a `History` or a `BoxId`, so
    /// there is no way to widen a rendered series across silicon.
    #[test]
    fn a_trend_is_single_box_by_construction() {
        use crate::history::{BoxId, History};
        let doc = r#"{"snapshots": [
          {"version": "0.19.0", "released": "2026-07-30", "measured_on": "air",
           "measured_at": "2026-08-02", "note": "",
           "tracks": {"2018 seq-sat": {"solved": 63, "total": 240}}},
          {"version": "0.20.0", "released": "2026-08-01", "measured_on": "cloud",
           "measured_at": "2026-08-01", "note": "",
           "tracks": {"2018 seq-sat": {"solved": 200, "total": 240}}},
          {"version": "0.21.0", "released": "2026-08-10", "measured_on": "air",
           "measured_at": "2026-08-10", "note": "",
           "tracks": {"2018 seq-sat": {"solved": 69, "total": 240}}}
        ]}"#;
        let h = History::from_json(doc).expect("a well-formed history");
        let t = h.trend("2018 seq-sat", &BoxId::new("air"));
        assert_eq!(t.points.len(), 2, "the cloud snapshot is a different board");
        let line = render_trend(&t);
        assert!(line.contains("63/240"));
        assert!(line.contains("69/240"));
        assert!(!line.contains("200/240"), "no cross-box point can appear");
    }

    #[test]
    fn an_empty_trend_says_so_rather_than_drawing_nothing() {
        use crate::history::{BoxId, History};
        let h = History::default();
        let t = h.trend("2018 seq-sat", &BoxId::new("air"));
        assert!(render_trend(&t).contains("no banked snapshots"));
    }

    // -----------------------------------------------------------------------
    // The report
    // -----------------------------------------------------------------------

    /// The default text report on a real pair: the seed's two header lines, the
    /// losses named individually and loudly, and the seed's table at the end.
    #[test]
    fn text_report_names_the_regression() {
        let (a, b) = pair("ipc2018-sat.jsonl", 60.0);
        let r = referee();
        let out =
            Diff::new(&r, &a, &b, &Conditions::none(), &Conditions::none()).render(Mode::default());
        assert!(out.starts_with("0.19.0: 63/240 solved   0.21.0: 69/240 solved\n"));
        assert!(out.contains("solved by both: 61   only 0.19.0: 2   only 0.21.0: 8\n"));
        assert!(out.contains("REGRESSION: 2 solved on 0.19.0 and not on 0.21.0"));
        assert!(out.contains(
            "  lost: data-network-sequential-satisficing/6  (53.6s, cost 1303.0)  \
             solved in 0.19.0, now timeout\n"
        ));
        assert!(out.contains("| variant | both | 0.19.0 cheaper |"));
        assert!(out.contains("| data-network-sequential-satisficing | 9 | 2 | 1 | 83s | 134s |"));
        // The timing line cannot appear without its three counts.
        assert!(out.contains("0 clean, 0 excluded dirty, 61 excluded unstamped"));
    }

    /// A clean board says so instead of leaving the reader to infer it from a
    /// missing section.
    #[test]
    fn a_board_with_no_losses_says_no_regressions() {
        let (a, b) = pair("ipc2014-tempo.jsonl", 60.0);
        let r = referee();
        let out =
            Diff::new(&r, &a, &b, &Conditions::none(), &Conditions::none()).render(Mode::Text);
        assert!(out.contains("no regressions"));
        assert!(out.contains("gained: 5"));
        assert!(out.contains("VAL-RED in 0.19.0, now solved"));
    }

    /// Markdown carries the same facts, with the minus sign that matches a "+"
    /// in weight -- the rule `history::ComparablePredecessor::delta` follows.
    #[test]
    fn markdown_uses_a_real_minus_sign() {
        let (a, b) = pair("ipc2014-opt.jsonl", 60.0);
        let r = referee();
        let d = Diff::new(&r, &a, &b, &Conditions::none(), &Conditions::none());
        assert_eq!(d.coverage().delta(), -7);
        let md = d.render(Mode::Markdown);
        assert!(md.contains("**\u{2212}7**"), "U+2212, not an ASCII hyphen");
        assert!(md.contains("REGRESSION"));
        assert!(md.contains("`city-car-sequential-optimal/8`"));
        assert!(md.contains("| variant | both |"));
    }

    /// The structured form carries every count the text does, including the
    /// third exclusion count and the named classes on each flip.
    #[test]
    fn json_form_carries_the_named_flips() {
        let (a, b) = pair("ipc2018-sat.jsonl", 60.0);
        let r = referee();
        let js = Diff::new(&r, &a, &b, &Conditions::none(), &Conditions::none()).to_json();
        let v: serde_json::Value = serde_json::from_str(&js).expect("valid JSON");
        assert_eq!(v["coverage"]["a_solved"], 63);
        assert_eq!(v["coverage"]["delta"], 6);
        assert_eq!(v["coverage"]["regression"], true);
        let lost = v["coverage"]["lost"].as_array().unwrap();
        assert_eq!(lost.len(), 2);
        assert_eq!(lost[0]["variant"], "data-network-sequential-satisficing");
        assert_eq!(lost[0]["instance"], 6);
        assert_eq!(lost[0]["was"], "solved");
        assert_eq!(lost[0]["now"], "timeout");
        assert_eq!(v["timing"]["unstamped_excluded"], 61);
        assert_eq!(v["timing"]["a_p50_ms"], serde_json::Value::Null);
        // indent=1, like every other JSON this project writes.
        assert!(js.starts_with("{\n \"a\": \"0.19.0\","));
    }

    /// A row absent from the other side is a different fact from any failure
    /// class, and must not be rendered as one.
    #[test]
    fn a_vanished_row_is_not_a_failure_class() {
        let a = RunRef::from_rows("A", 60.0, vec![row(None, "v", 1, true, 1.0)]).run;
        let b = RunRef::from_rows("B", 60.0, vec![]).run;
        let r = referee();
        let d = CoverageDiff::new(&r, &a, &b);
        assert_eq!(d.lost().len(), 1);
        assert_eq!(d.lost()[0].now, None);
        let out =
            Diff::new(&r, &a, &b, &Conditions::none(), &Conditions::none()).render(Mode::Text);
        assert!(out.contains("now gone from the board"));
    }

    /// A run's own budget decides its timeouts, so the two sides of a diff can
    /// sit in different tiers -- the 0.23 tier-move mechanism, seen from the
    /// comparison side.
    #[test]
    fn each_side_classifies_in_its_own_budget() {
        // 28 s of wall: a timeout at the 30 s tier, an early exit at 60 s.
        let a = RunRef::from_rows("30s", 30.0, vec![row(None, "v", 1, false, 28.0)]).run;
        let b = RunRef::from_rows("60s", 60.0, vec![row(None, "v", 1, false, 28.0)]).run;
        let r = referee();
        let d = CoverageDiff::new(&r, &a, &b);
        assert_eq!(a.class_of(&r, &key("v", 1)), Some(Class::Timeout));
        assert_eq!(b.class_of(&r, &key("v", 1)), Some(Class::EarlyExit));
        // Neither is solved, so neither shows as a flip -- the classes differ,
        // the coverage does not.
        assert!(d.lost().is_empty() && d.gained().is_empty());
    }

    /// The whole report against the real banked history: the trend renders in
    /// VERSION order (0.19.0 then 0.20.0, though 0.19.0 was MEASURED a day
    /// later -- the backfill that law 2 exists for), and the seed's table is
    /// still the last thing on the page, unchanged, so a cut record can be
    /// pasted from either implementation.
    #[test]
    fn a_full_report_carries_every_section() {
        use crate::history::{BoxId, History};
        let (a, b) = pair("ipc2023-agile.jsonl", 60.0);
        let r = referee();
        let h = History::load(Path::new(&format!(
            "{REPO}/benchmarks/standings-history.json"
        )));
        let t = h.trend("2023 classical", &BoxId::new("m5-air"));
        let out = Diff::new(&r, &a, &b, &Conditions::none(), &Conditions::none())
            .with_trend(t)
            .render(Mode::Text);
        assert!(out.contains("no regressions"));
        assert!(out.contains("gained: 3"));
        assert!(out.contains("mean cost: 0.19.0 35.30   0.21.0 39.67"));
        assert!(out.contains("30 excluded unstamped"));
        // Version order, not measurement order.
        let trend_line = out
            .lines()
            .find(|l| l.starts_with("2023 classical:"))
            .expect("the trend renders");
        let i19 = trend_line.find("0.19.0").expect("0.19.0 is banked");
        let i20 = trend_line.find("0.20.0").expect("0.20.0 is banked");
        assert!(i19 < i20, "0.19.0 was measured LATER and still sorts first");
        assert!(out
            .trim_end()
            .ends_with("| slitherlink-agile | 2 | 0 | 0 | 15s | 13s |"));
    }
}
