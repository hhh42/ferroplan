//! Release history -- the banked snapshots the "vs previous" column of
//! `STANDINGS.md` is computed from, and the two ordering laws that decide
//! *which* snapshot a release is measured against.
//!
//! Ported from `benchmarks/standings.py` (`_current_version` :564, the
//! previous-release selection in `write_summary` :606-613, `_delta` :746) and
//! from `scripts/standings-snapshot.py` (the file format and its
//! replace-append-sort).
//!
//! Both laws were conventions in the Python -- one comment each, obeyed by one
//! call site. Here they are the shape of the types, because each is a rule this
//! record learned the hard way, and a convention only holds until the second
//! call site:
//!
//! * **Same box.** Coverage at a fixed time budget is a property of the
//!   HARDWARE as much as of the engine. The cloud-to-Air move would have
//!   rendered a silicon upgrade as engine progress on every board at once, so a
//!   snapshot is only ever compared against a predecessor sharing its
//!   `measured_on`.
//! * **By version, not by date.** A backfilled old tag is measured LATE:
//!   `benchmarks/standings-history.json` still carries 0.19.0 measured
//!   2026-08-02, the day AFTER 0.20.0. Picking "the most recent measurement"
//!   therefore compares a release to its GRANDPARENT and skips one entirely.
//!   On the 2018 seq-sat board at the 0.21 cut that is the difference between
//!   the published `+7.1 pts (vs 0.20.0)` and `+2.9 pts (vs 0.19.0)`.
//!
//! [`History::comparable_predecessor`] is the only way to obtain a predecessor:
//! [`ComparablePredecessor`] has a private field and no public constructor, and
//! no function here accepts two bare [`Snapshot`]s. [`MeasuredAt`] is
//! deliberately given no `Ord`, so "pick the most recent measurement" cannot be
//! written down at all -- the one place a date is ordered is the file-layout
//! sort inside [`History::upsert`], which decides where a line sits in a text
//! file and nothing else.
//!
//! The JSON writer is hand-rolled for one reason: this file is read, modified
//! and rewritten in place by a release step, and a rewrite whose diff is not
//! empty when nothing changed is a rewrite nobody reads. `serde_json` matches
//! neither of Python's two defaults here (`indent=1`; `ensure_ascii=True`), and
//! its map type sorts keys, which would reorder every snapshot object.

use std::path::Path;

/// The `_comment` a fresh history is born with, verbatim from
/// `scripts/standings-snapshot.py` -- so a history created here and one created
/// by the Python are the same bytes.
const DEFAULT_COMMENT: &str =
    "Per-release standings snapshots. `measured_on` is the BOX: coverage at a \
fixed time budget depends on hardware, so STANDINGS.md only ever compares \
snapshots sharing a box. `measured_at` is when the boards were SWEPT, \
which differs from `released` for backfilled runs of old versions on new \
hardware. Written by scripts/standings-snapshot.py.";

/// No predecessor at all: the first snapshot ever banked on this box.
pub const BASELINE_CELL: &str = "\u{2014} *baseline*";
/// A predecessor exists but has nothing to say about this board.
pub const NEW_CELL: &str = "\u{2014} *new*";

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// The file exists and is not the document this module knows how to
    /// rewrite. Refusing is deliberate: see [`History::try_load`].
    #[error("{path}: {msg}")]
    Malformed { path: String, msg: String },
}

// ---------------------------------------------------------------------------
// VersionKey
// ---------------------------------------------------------------------------

/// A version, ordered the way Python orders `tuple(int(x) for x in v.split("."))`.
///
/// `Vec<u64>` compares lexicographically and a prefix sorts below its
/// extension, exactly as a Python tuple does -- `0.21 < 0.21.0` in both. That
/// equivalence is why this is a `Vec` and not a `(u64, u64, u64)`: a struct of
/// three fields would have to invent a value for the missing component and
/// would put `0.21` and `0.21.0` in the wrong order relative to each other.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionKey(Vec<u64>);

impl VersionKey {
    /// Parse a dotted version. Anything unparseable is `[0]`, matching the
    /// Python's `except ValueError: return (0,)` -- a fallback that sorts below
    /// every real release, so an unreadable version can never be selected as
    /// somebody's predecessor and can never suppress a real one.
    ///
    /// Python's `int()` is looser than this in three corners that no version
    /// string in this project has ever used: it accepts a sign, digit-grouping
    /// underscores (`int("1_0") == 10`) and non-ASCII decimal digits. Each of
    /// those lands here as `[0]` instead, which is the same shape of answer --
    /// "this is not a version I can order" -- rather than a different one.
    pub fn parse(s: &str) -> Self {
        let mut parts = Vec::new();
        for comp in s.split('.') {
            // `int()` tolerates surrounding whitespace, so " 0 . 21 " parses in
            // Python; keep that, since it is the only looseness that could
            // plausibly reach us from a hand-edited file.
            match comp.trim().parse::<u64>() {
                Ok(n) => parts.push(n),
                Err(_) => return Self(vec![0]),
            }
        }
        Self(parts)
    }

    /// The components, for callers that render or diff them.
    pub fn parts(&self) -> &[u64] {
        &self.0
    }
}

impl std::fmt::Display for VersionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, n) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(".")?;
            }
            write!(f, "{n}")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// BoxId / MeasuredAt
// ---------------------------------------------------------------------------

/// The machine a sweep ran on (`measured_on`). LAW 1 lives on this type: two
/// snapshots are comparable only when these are equal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoxId(String);

impl BoxId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BoxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The day the BOARDS were swept -- which is not the release date, and for a
/// backfill is not even in the same order as the releases.
///
/// **This type has no `Ord`, on purpose.** Every incident behind this module is
/// somebody reaching for the most recently measured snapshot; with no ordering
/// on the date, that sentence has no Rust spelling. `as_str` exists so a
/// renderer can print the date -- not so a caller can rebuild the comparison it
/// is missing. If you find yourself sorting these, you are about to compare a
/// release to its grandparent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MeasuredAt(String);

impl MeasuredAt {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MeasuredAt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Snapshot
// ---------------------------------------------------------------------------

/// A snapshot's boards: `label -> (solved, total)`, in the order the file lists
/// them. Named because it is passed around, not because the tuple is obscure.
pub type Tracks = Vec<(String, (usize, usize))>;

/// One banked release: what was measured, where, when, and on which boards.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub version: String,
    pub released: String,
    pub measured_on: BoxId,
    pub measured_at: MeasuredAt,
    pub note: String,
    /// `label -> (solved, total)` in the order the file lists them.
    ///
    /// A `Vec` rather than a map because Python dicts iterate in INSERTION
    /// order and this one is written straight back out: a `HashMap` would
    /// scramble the file on every rewrite, and a `BTreeMap` would silently
    /// alphabetise a list whose order is the sweep registry's.
    pub tracks: Tracks,
}

impl Snapshot {
    /// This snapshot's board, by label. Linear because the list is ~25 long and
    /// its order is load-bearing; the Python is a dict lookup on the same data.
    pub fn track(&self, label: &str) -> Option<(usize, usize)> {
        self.tracks
            .iter()
            .find(|(k, _)| k == label)
            .map(|(_, v)| *v)
    }

    pub fn version_key(&self) -> VersionKey {
        VersionKey::parse(&self.version)
    }
}

// ---------------------------------------------------------------------------
// The predecessor, and the delta cell
// ---------------------------------------------------------------------------

/// A snapshot that has EARNED the right to be compared against: same box,
/// strictly lower version, and the highest such version in the history.
///
/// The field is private and there is no public constructor, so the only way to
/// hold one of these is [`History::comparable_predecessor`]. That is the whole
/// enforcement mechanism: a caller cannot assemble a comparison out of two
/// `Snapshot`s it picked itself.
pub struct ComparablePredecessor<'h> {
    snap: &'h Snapshot,
}

impl<'h> ComparablePredecessor<'h> {
    /// The version this delta is quoted against -- the string as banked, not a
    /// re-rendered `VersionKey`, so `0.21` never prints as `0.21.0`.
    pub fn version(&self) -> &str {
        &self.snap.version
    }

    /// The `vs previous` cell for one board. Port of `_delta` :746-758.
    pub fn delta(&self, label: &str, solved: usize, total: usize) -> String {
        let Some((was_s, was_n)) = self.snap.track(label) else {
            // The predecessor never measured this board: a new entry, not
            // movement. Python reaches this by `not p` on a missing dict.
            return NEW_CELL.to_string();
        };
        if was_n == 0 {
            // `not p.get("total")`: a zero denominator is not a share.
            return NEW_CELL.to_string();
        }
        if total == 0 {
            // Unreachable from the renderer -- `write_summary` drops boards
            // with no rows before it builds the column, and Python would raise
            // ZeroDivisionError here. An empty board has no share, so there is
            // no delta to state; of the two dashes, "*new*" is the one that
            // already means "no comparable share exists".
            return NEW_CELL.to_string();
        }
        // SHARES, not raw counts: a corpus can grow between releases -- the
        // denominator is not a constant across this history, and differencing
        // counts across a corpus change books instances ADDED as instances
        // solved.
        //
        // Multiply BEFORE dividing, as Python does. `100.0 * s / n` and
        // `100.0 * (s / n)` differ in the last bit, and the 0.05 test below is
        // decided at exactly that scale.
        let was = 100.0 * was_s as f64 / was_n as f64;
        let now = 100.0 * solved as f64 / total as f64;
        let d = now - was;
        if d.abs() < 0.05 {
            // Below the printed resolution: say "no movement" rather than
            // print a signed 0.0.
            return format!("= (vs {})", self.snap.version);
        }
        // U+2212 MINUS SIGN, not ASCII hyphen: this renders in a markdown table
        // beside a "+", and the two must be the same visual weight.
        let sign = if d > 0.0 { '+' } else { '\u{2212}' };
        // Rust's `{:.1}` and Python's `{:.1f}` both round half-to-even on the
        // decimal representation of the same f64, so this needs no detour
        // through `fmt::py_round` -- and there is no arithmetic rounding here
        // to route through it either. (Python's `round()` is the half-to-even
        // hazard; `format` is not.)
        format!("{sign}{:.1} pts (vs {})", d.abs(), self.snap.version)
    }
}

/// The `vs previous` cell including the no-predecessor case, so a renderer
/// cannot forget the baseline string. `_delta`'s first branch.
pub fn delta_cell(
    prev: Option<&ComparablePredecessor<'_>>,
    label: &str,
    solved: usize,
    total: usize,
) -> String {
    match prev {
        None => BASELINE_CELL.to_string(),
        Some(p) => p.delta(label, solved, total),
    }
}

// ---------------------------------------------------------------------------
// Trend
// ---------------------------------------------------------------------------

/// One board's history on one box, in VERSION order.
///
/// Version order, not measurement order, for LAW 2's reason: plotted by date,
/// the 0.19.0 backfill draws a spike between 0.20.0 and 0.21.0 that no release
/// ever had.
#[derive(Debug, Clone, PartialEq)]
pub struct Trend {
    pub label: String,
    pub points: Vec<(VersionKey, (usize, usize))>,
}

// ---------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------

/// One top-level key of the document. The snapshots array keeps its POSITION
/// among the others: Python rewrites `doc["snapshots"]` in place, which leaves
/// the key where the file had it, and `_comment` stays first.
#[derive(Debug, Clone, PartialEq)]
enum TopSlot {
    Snapshots,
    Other(String, Json),
}

/// `benchmarks/standings-history.json`, parsed, plus everything needed to write
/// it back out unchanged.
#[derive(Debug, Clone, PartialEq)]
pub struct History {
    top: Vec<TopSlot>,
    snapshots: Vec<Snapshot>,
}

impl Default for History {
    /// A fresh document, byte-for-byte what `standings-snapshot.py` writes when
    /// the history file does not exist yet.
    fn default() -> Self {
        Self {
            top: vec![
                TopSlot::Other(
                    "_comment".to_string(),
                    Json::Str(DEFAULT_COMMENT.to_string()),
                ),
                TopSlot::Snapshots,
            ],
            snapshots: Vec::new(),
        }
    }
}

impl History {
    /// Load, degrading a missing OR unreadable file to an empty history.
    ///
    /// Python's `_history()` does the `os.path.exists` half of this and lets a
    /// malformed file raise. This signature cannot report the difference, and
    /// an empty history renders `— *baseline*` on every board -- a claim, not a
    /// blank. Release tooling should call [`History::try_load`] and refuse to
    /// publish on `Err`; this exists for the read-only paths.
    pub fn load(path: &Path) -> Self {
        Self::try_load(path).unwrap_or_default()
    }

    /// Load, distinguishing "no history yet" from "this file is not that".
    ///
    /// A missing file is `Ok(empty)` -- the Python's `if not os.path.exists`.
    /// Anything else that cannot be represented losslessly is an error rather
    /// than a best-effort read, because every caller of this type rewrites the
    /// file afterwards: a field quietly dropped on load is a field deleted from
    /// the record on the next release.
    pub fn try_load(path: &Path) -> Result<Self, HistoryError> {
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => {
                return Err(HistoryError::Io {
                    path: path.display().to_string(),
                    source: e,
                })
            }
        };
        Self::from_json(&src).map_err(|msg| HistoryError::Malformed {
            path: path.display().to_string(),
            msg,
        })
    }

    /// Parse a history document from JSON text.
    pub fn from_json(src: &str) -> Result<Self, String> {
        let doc = parse_json(src)?;
        let Json::Obj(entries) = doc else {
            return Err("top level is not a JSON object".to_string());
        };
        let mut top = Vec::new();
        let mut snapshots = Vec::new();
        let mut seen_snapshots = false;
        for (k, v) in entries {
            if k == "snapshots" {
                let Json::Arr(items) = v else {
                    return Err("\"snapshots\" is not an array".to_string());
                };
                for (i, item) in items.into_iter().enumerate() {
                    snapshots
                        .push(snapshot_from_json(item).map_err(|e| format!("snapshot {i}: {e}"))?);
                }
                seen_snapshots = true;
                top.push(TopSlot::Snapshots);
            } else {
                top.push(TopSlot::Other(k, v));
            }
        }
        if !seen_snapshots {
            // Python's `doc.get("snapshots", [])` tolerates the absence; the
            // writer then creates the key, at the end.
            top.push(TopSlot::Snapshots);
        }
        Ok(Self { top, snapshots })
    }

    /// The snapshots in file order (which [`upsert`](Self::upsert) keeps sorted
    /// by `(measured_at, version)` -- a layout, not a semantics).
    pub fn snapshots(&self) -> &[Snapshot] {
        &self.snapshots
    }

    /// The release this one should be measured against, or `None`.
    ///
    /// Both laws in one loop: same `measured_on`, STRICTLY lower version, and
    /// the maximum of those. Port of `write_summary` :611-613.
    pub fn comparable_predecessor(
        &self,
        box_: &BoxId,
        cur: &VersionKey,
    ) -> Option<ComparablePredecessor<'_>> {
        let mut best: Option<(&Snapshot, VersionKey)> = None;
        for s in &self.snapshots {
            // LAW 1. Different silicon, different board -- not a comparison.
            if &s.measured_on != box_ {
                continue;
            }
            let k = s.version_key();
            // Strictly lower: a snapshot of the version being cut is the SAME
            // measurement, and comparing it to itself is what `_current_version`
            // exists to prevent -- every row would read "= (vs X)": true, and
            // useless.
            if k >= *cur {
                continue;
            }
            // A manual fold with a strict `>`, because Python's `max()` returns
            // the FIRST maximum and Rust's `max_by_key` returns the last. Two
            // snapshots can tie on key without tying on text ("0.20.0" and
            // "0.20.00" both parse to [0,20,0]), and the quoted version string
            // is published.
            let better = match &best {
                None => true,
                Some((_, bk)) => k > *bk,
            };
            if better {
                best = Some((s, k));
            }
        }
        best.map(|(snap, _)| ComparablePredecessor { snap })
    }

    /// One board's points on one box, oldest release first.
    pub fn trend(&self, label: &str, box_: &BoxId) -> Trend {
        let mut points: Vec<(VersionKey, (usize, usize))> = self
            .snapshots
            .iter()
            .filter(|s| &s.measured_on == box_)
            .filter_map(|s| s.track(label).map(|v| (s.version_key(), v)))
            .collect();
        // Stable, so two snapshots that tie on version key keep file order --
        // the same tie-break `comparable_predecessor` applies.
        points.sort_by(|a, b| a.0.cmp(&b.0));
        Trend {
            label: label.to_string(),
            points,
        }
    }

    /// Bank a snapshot: replace any snapshot with the same `(version, box)`,
    /// append, then sort by `(measured_at, version)`.
    ///
    /// Port of `standings-snapshot.py`'s three lines. The identity is the PAIR:
    /// the same tag measured on two boxes is two records, and collapsing them
    /// would overwrite the record of one machine with another's.
    pub fn upsert(&mut self, s: Snapshot) {
        self.snapshots
            .retain(|x| !(x.version == s.version && x.measured_on == s.measured_on));
        self.snapshots.push(s);
        // The ONE place a measurement date is ordered, and it decides only
        // where a line sits in a text file. Nothing reads this order back as
        // meaning -- `comparable_predecessor` re-derives its own.
        //
        // Python sorts strings by codepoint and Rust by byte; UTF-8 is
        // order-preserving, so the two agree, and these are ASCII dates and
        // versions anyway. Both sorts are stable.
        self.snapshots.sort_by(|a, b| {
            (a.measured_at.0.as_str(), a.version.as_str())
                .cmp(&(b.measured_at.0.as_str(), b.version.as_str()))
        });
    }

    /// Serialise exactly as `json.dump(doc, f, indent=1)` followed by `"\n"`.
    ///
    /// One space per level, `": "` between key and value, `ensure_ascii` (every
    /// non-ASCII character escaped, astral planes as surrogate pairs). The
    /// point is a clean diff: this file is rewritten on every release, and a
    /// reformatting rewrite hides the one line that changed.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        write_value(&mut out, &self.document(), 0);
        out.push('\n');
        out
    }

    fn document(&self) -> Json {
        let snaps = Json::Arr(self.snapshots.iter().map(snapshot_to_json).collect());
        let mut entries = Vec::new();
        for slot in &self.top {
            match slot {
                TopSlot::Snapshots => entries.push(("snapshots".to_string(), snaps.clone())),
                TopSlot::Other(k, v) => entries.push((k.clone(), v.clone())),
            }
        }
        Json::Obj(entries)
    }
}

/// Workspace version, so the delta column never compares a release to ITSELF.
///
/// Port of `_current_version` :564. The snapshot for the release being cut is
/// banked from the same boards the table is generated from, so without this
/// every row reads `= (vs X)` -- technically true and completely useless.
///
/// Deliberately the same crude scan as the Python: the first line whose
/// leading-whitespace-stripped form starts with `version`, split once on `=`.
/// It is reading the ROOT manifest, where that is `[workspace.package]`'s
/// version. Where Python raises (a `version` line with no `=`, a non-UTF-8
/// file) this returns `None`, which the caller already handles: a missing
/// current version yields no predecessor and every board renders as a baseline.
pub fn current_version(root: &Path) -> Option<String> {
    let src = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
    for line in src.lines() {
        if line.trim_start().starts_with("version") {
            let (_, rest) = line.split_once('=')?;
            // `.strip('"')` strips every leading and trailing quote, not one.
            return Some(rest.trim().trim_matches('"').to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Snapshot <-> JSON
// ---------------------------------------------------------------------------

fn snapshot_to_json(s: &Snapshot) -> Json {
    // The writer's key order, which is the order the committed file has.
    Json::Obj(vec![
        ("version".to_string(), Json::Str(s.version.clone())),
        ("released".to_string(), Json::Str(s.released.clone())),
        (
            "measured_on".to_string(),
            Json::Str(s.measured_on.0.clone()),
        ),
        (
            "measured_at".to_string(),
            Json::Str(s.measured_at.0.clone()),
        ),
        ("note".to_string(), Json::Str(s.note.clone())),
        (
            "tracks".to_string(),
            Json::Obj(
                s.tracks
                    .iter()
                    .map(|(label, (solved, total))| {
                        (
                            label.clone(),
                            Json::Obj(vec![
                                ("solved".to_string(), Json::Num(solved.to_string())),
                                ("total".to_string(), Json::Num(total.to_string())),
                            ]),
                        )
                    })
                    .collect(),
            ),
        ),
    ])
}

fn snapshot_from_json(v: Json) -> Result<Snapshot, String> {
    let Json::Obj(entries) = v else {
        return Err("not an object".to_string());
    };
    let mut version = None;
    let mut released = None;
    let mut measured_on = None;
    let mut measured_at = None;
    let mut note = None;
    let mut tracks = None;
    for (k, val) in entries {
        match k.as_str() {
            "version" => version = Some(text_of(&val).ok_or("\"version\" is not a string")?),
            "released" => released = Some(text_of(&val).unwrap_or_default()),
            "measured_on" => {
                measured_on = Some(text_of(&val).ok_or("\"measured_on\" is not a string")?)
            }
            "measured_at" => {
                measured_at = Some(text_of(&val).ok_or("\"measured_at\" is not a string")?)
            }
            "note" => note = Some(text_of(&val).unwrap_or_default()),
            "tracks" => tracks = Some(tracks_from_json(&val)?),
            // Refuse rather than lose: this document is rewritten in place, so
            // a key we do not model would be deleted from the record by the
            // next release rather than merely ignored.
            other => return Err(format!("unknown key {other:?}")),
        }
    }
    Ok(Snapshot {
        // Required: a snapshot with no version cannot be ordered against
        // anything, and the Python's own sort raises on its absence.
        version: version.ok_or("no \"version\"")?,
        released: released.unwrap_or_default(),
        // Required, and the reason is LAW 1: an unattributed measurement has no
        // hardware to be comparable on, and defaulting it to some empty box id
        // would let it match a caller that also passed one.
        measured_on: BoxId(measured_on.ok_or("no \"measured_on\"")?),
        measured_at: MeasuredAt(measured_at.ok_or("no \"measured_at\"")?),
        note: note.unwrap_or_default(),
        // Python is `prev.get("tracks", {})`: a snapshot with no boards is a
        // snapshot that measured nothing, and every board reads "*new*".
        tracks: tracks.unwrap_or_default(),
    })
}

fn tracks_from_json(v: &Json) -> Result<Tracks, String> {
    let Json::Obj(entries) = v else {
        return Err("\"tracks\" is not an object".to_string());
    };
    let mut out = Vec::with_capacity(entries.len());
    for (label, tv) in entries {
        let Json::Obj(fields) = tv else {
            return Err(format!("track {label:?} is not an object"));
        };
        let mut solved: Option<usize> = None;
        let mut total: Option<usize> = None;
        for (k, val) in fields {
            match k.as_str() {
                "solved" => solved = count_of(val),
                // `not p.get("total")` in `_delta` treats a null and a 0 alike:
                // no denominator, so no share, so the board reads "*new*".
                "total" => total = count_of(val),
                other => return Err(format!("track {label:?}: unknown key {other:?}")),
            }
        }
        let total = total.unwrap_or(0);
        // `solved` is only ever READ when there is a denominator; Python would
        // raise KeyError on its absence there and never look otherwise.
        let solved = match solved {
            Some(n) => n,
            None if total == 0 => 0,
            None => return Err(format!("track {label:?}: no \"solved\"")),
        };
        out.push((label.clone(), (solved, total)));
    }
    Ok(out)
}

/// A JSON value read as text. Python would `str()` a number here, and the
/// stored literal is exactly that for the integers and plain decimals a
/// hand-edit could leave behind.
fn text_of(v: &Json) -> Option<String> {
    match v {
        Json::Str(s) => Some(s.clone()),
        Json::Num(n) => Some(n.clone()),
        // `null` is Python's absent-ish: the fields that tolerate it default to
        // the empty string, the ones that do not reject it.
        Json::Null => None,
        _ => None,
    }
}

/// A non-negative integer count. Counts are cardinalities of row lists; a
/// negative or fractional one is a corrupt record, not a small number.
fn count_of(v: &Json) -> Option<usize> {
    match v {
        Json::Num(n) => n.parse::<usize>().ok(),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// A JSON value that round-trips
// ---------------------------------------------------------------------------

/// Just enough JSON to re-emit this document unchanged.
///
/// Numbers keep their SOURCE TEXT. Python re-emits an int as `repr(int)`, which
/// is the input spelling for every integer, and this file has never held a
/// float; keeping the literal is exact where re-parsing to `f64` and
/// re-printing would not be.
#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<Json>),
    /// Insertion-ordered, like a Python dict.
    Obj(Vec<(String, Json)>),
}

// ---- writer ----

fn write_value(out: &mut String, v: &Json, level: usize) {
    match v {
        Json::Null => out.push_str("null"),
        Json::Bool(true) => out.push_str("true"),
        Json::Bool(false) => out.push_str("false"),
        Json::Num(n) => out.push_str(n),
        Json::Str(s) => escape_into(out, s),
        Json::Arr(items) => {
            // Python emits `[]` with no inner newline for an empty container.
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                indent(out, level + 1);
                write_value(out, item, level + 1);
            }
            out.push('\n');
            indent(out, level);
            out.push(']');
        }
        Json::Obj(entries) => {
            if entries.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            for (i, (k, val)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                indent(out, level + 1);
                escape_into(out, k);
                // The key separator under `indent=` is ": ", not ":".
                out.push_str(": ");
                write_value(out, val, level + 1);
            }
            out.push('\n');
            indent(out, level);
            out.push('}');
        }
    }
}

/// ONE space per level -- `indent=1`, not 4 and not a tab.
fn indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push(' ');
    }
}

/// `ensure_ascii=True`: every non-ASCII character as `\uXXXX`, lowercase hex,
/// astral characters as a surrogate PAIR. Python's default; `serde_json` emits
/// raw UTF-8 instead, which would rewrite every line carrying an em dash.
fn escape_into(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Everything else PRINTABLE passes through, and the range stops
            // at `~`: Python escapes DEL as `\\u007f` even though it is ascii,
            // and does NOT escape `/` even though escaping it would be valid
            // JSON. Both spellings are diff noise if you get them backwards.
            c if (' '..='~').contains(&c) => out.push(c),
            c => {
                let cp = c as u32;
                if cp < 0x10000 {
                    out.push_str(&format!("\\u{cp:04x}"));
                } else {
                    let v = cp - 0x1_0000;
                    out.push_str(&format!(
                        "\\u{:04x}\\u{:04x}",
                        0xd800 + (v >> 10),
                        0xdc00 + (v & 0x3ff)
                    ));
                }
            }
        }
    }
    out.push('"');
}

// ---- parser ----

/// Recursion cap. A hand-rolled descent parser on adversarial input is the one
/// way this pure module could abort a process, and "no panics on bad input
/// data" includes a stack overflow.
const MAX_DEPTH: usize = 64;

fn parse_json(src: &str) -> Result<Json, String> {
    let mut p = Parser {
        s: src,
        b: src.as_bytes(),
        i: 0,
    };
    p.ws();
    let v = p.value(0)?;
    p.ws();
    if p.i != p.b.len() {
        return Err(format!("trailing data at byte {}", p.i));
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a str,
    b: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn eat(&mut self, lit: &str) -> bool {
        if self.s[self.i..].starts_with(lit) {
            self.i += lit.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth > MAX_DEPTH {
            return Err(format!(
                "nesting deeper than {MAX_DEPTH} at byte {}",
                self.i
            ));
        }
        match self.peek() {
            None => Err("unexpected end of input".to_string()),
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') if self.eat("true") => Ok(Json::Bool(true)),
            Some(b'f') if self.eat("false") => Ok(Json::Bool(false)),
            Some(b'n') if self.eat("null") => Ok(Json::Null),
            Some(_) => self.number(),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, String> {
        self.i += 1; // '{'
        let mut entries: Vec<(String, Json)> = Vec::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Json::Obj(entries));
        }
        loop {
            self.ws();
            if self.peek() != Some(b'"') {
                return Err(format!("expected a key at byte {}", self.i));
            }
            let k = self.string()?;
            self.ws();
            if self.peek() != Some(b':') {
                return Err(format!("expected ':' at byte {}", self.i));
            }
            self.i += 1;
            self.ws();
            let v = self.value(depth + 1)?;
            // A repeated key: Python keeps the LAST value at the FIRST
            // position, which is what dict assignment does.
            match entries.iter_mut().find(|(ek, _)| *ek == k) {
                Some(slot) => slot.1 = v,
                None => entries.push((k, v)),
            }
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Json::Obj(entries));
                }
                _ => return Err(format!("expected ',' or '}}' at byte {}", self.i)),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json, String> {
        self.i += 1; // '['
        let mut items = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            self.ws();
            items.push(self.value(depth + 1)?);
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(Json::Arr(items));
                }
                _ => return Err(format!("expected ',' or ']' at byte {}", self.i)),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.i += 1; // '"'
        let mut out = String::new();
        loop {
            let Some(c) = self.peek() else {
                return Err("unterminated string".to_string());
            };
            match c {
                b'"' => {
                    self.i += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.i += 1;
                    let Some(e) = self.peek() else {
                        return Err("unterminated escape".to_string());
                    };
                    self.i += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let cp = if (0xd800..0xdc00).contains(&hi) {
                                // A surrogate pair, the way `ensure_ascii`
                                // writes anything above the BMP.
                                if !self.eat("\\u") {
                                    return Err(format!("lone high surrogate at byte {}", self.i));
                                }
                                let lo = self.hex4()?;
                                if !(0xdc00..0xe000).contains(&lo) {
                                    return Err(format!("bad surrogate pair at byte {}", self.i));
                                }
                                0x1_0000 + ((hi - 0xd800) << 10) + (lo - 0xdc00)
                            } else {
                                hi
                            };
                            // Python's decoder tolerates a lone surrogate and
                            // yields a string Rust cannot hold. Refusing beats
                            // inventing a replacement character in a file we
                            // are about to rewrite.
                            let Some(ch) = char::from_u32(cp) else {
                                return Err(format!("\\u{cp:04x} is not a character"));
                            };
                            out.push(ch);
                        }
                        other => {
                            return Err(format!(
                                "bad escape \\{} at byte {}",
                                other as char, self.i
                            ))
                        }
                    }
                }
                // Strict mode, like Python's default: a raw control character
                // inside a string is a corrupt file, not a literal.
                0x00..=0x1f => return Err(format!("control character at byte {}", self.i)),
                _ => {
                    // Multi-byte UTF-8 passes through whole; the source is a
                    // &str, so the boundary is already known good.
                    let ch = self.s[self.i..]
                        .chars()
                        .next()
                        .ok_or("unterminated string")?;
                    self.i += ch.len_utf8();
                    out.push(ch);
                }
            }
        }
    }

    /// Exactly four HEX DIGITS, validated a byte at a time.
    ///
    /// Not a `&str` slice: `self.i + 4` can land INSIDE a multi-byte character
    /// -- `"\u00` followed by a euro sign puts it one byte into the euro's
    /// three -- and slicing a `&str` off a char boundary panics. CPython's
    /// scanner answers that input with `Invalid \uXXXX escape`, and an aborted
    /// process is not an error message; this module's whole contract is that a
    /// corrupt file is refused, not that it takes the release step down with
    /// it.
    ///
    /// Reading the digits directly also refuses the leading `+` that
    /// `u32::from_str_radix` accepts (`\u+abc`), which that same scanner
    /// rejects.
    fn hex4(&mut self) -> Result<u32, String> {
        if self.i + 4 > self.b.len() {
            return Err("truncated \\u escape".to_string());
        }
        let mut v: u32 = 0;
        for k in 0..4 {
            let c = self.b[self.i + k];
            let d = match c {
                b'0'..=b'9' => u32::from(c - b'0'),
                b'a'..=b'f' => u32::from(c - b'a') + 10,
                b'A'..=b'F' => u32::from(c - b'A') + 10,
                _ => return Err(format!("bad \\u escape at byte {}", self.i)),
            };
            v = v * 16 + d;
        }
        self.i += 4;
        Ok(v)
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        let digits = |p: &mut Self| {
            let s = p.i;
            while matches!(p.peek(), Some(b'0'..=b'9')) {
                p.i += 1;
            }
            p.i > s
        };
        if !digits(self) {
            return Err(format!("expected a value at byte {start}"));
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            if !digits(self) {
                return Err(format!("truncated number at byte {}", self.i));
            }
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.i += 1;
            }
            if !digits(self) {
                return Err(format!("truncated exponent at byte {}", self.i));
            }
        }
        Ok(Json::Num(self.s[start..self.i].to_string()))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The repo root, reached the way `tests/common/mod.rs` reaches it.
    const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../..");

    fn real_history() -> History {
        let p = format!("{REPO}/benchmarks/standings-history.json");
        History::try_load(Path::new(&p)).unwrap_or_else(|e| panic!("{e}"))
    }

    fn snap(version: &str, box_: &str, at: &str, tracks: &[(&str, usize, usize)]) -> Snapshot {
        Snapshot {
            version: version.to_string(),
            released: at.to_string(),
            measured_on: BoxId::new(box_),
            measured_at: MeasuredAt::new(at),
            note: String::new(),
            tracks: tracks
                .iter()
                .map(|(l, s, n)| (l.to_string(), (*s, *n)))
                .collect(),
        }
    }

    /// LAW 2, against the committed file. 0.19.0 was BACKFILLED on 2026-08-02,
    /// the day after 0.20.0 was measured, so it is the most recent measurement
    /// below 0.21.0 -- and the wrong answer. Picking by date here would have
    /// published `+2.9 pts (vs 0.19.0)` on the 2018 board where the record says
    /// `+7.1 pts (vs 0.20.0)`.
    #[test]
    fn predecessor_is_the_previous_version_not_the_latest_measurement() {
        let h = real_history();
        let air = BoxId::new("m5-air");
        let prev = h
            .comparable_predecessor(&air, &VersionKey::parse("0.21.0"))
            .expect("0.20.0 is in the committed history");
        assert_eq!(prev.version(), "0.20.0");
        assert_eq!(prev.delta("2018 seq-sat", 70, 240), "+7.1 pts (vs 0.20.0)");

        // The date-ordered answer, spelled out so the test names what it is
        // defending against rather than only what it wants.
        let by_date = h
            .snapshots()
            .iter()
            .rfind(|s| s.measured_on == air && s.version_key() < VersionKey::parse("0.21.0"))
            .unwrap();
        assert_eq!(by_date.version, "0.19.0");
    }

    /// LAW 1. A predecessor on another box is not a predecessor at all, however
    /// close its version -- the cloud-to-Air jump would otherwise have rendered
    /// as engine progress on every board at once.
    #[test]
    fn a_different_box_is_never_comparable() {
        let mut h = History::default();
        h.upsert(snap(
            "0.20.0",
            "cloud-4c",
            "2026-07-01",
            &[("seq-sat", 400, 580)],
        ));
        h.upsert(snap(
            "0.21.0",
            "m5-air",
            "2026-08-04",
            &[("seq-sat", 486, 580)],
        ));
        assert!(h
            .comparable_predecessor(&BoxId::new("m5-air"), &VersionKey::parse("0.22.0"))
            .is_some());
        let cell = delta_cell(
            h.comparable_predecessor(&BoxId::new("m5-air"), &VersionKey::parse("0.21.0"))
                .as_ref(),
            "seq-sat",
            486,
            580,
        );
        assert_eq!(
            cell, BASELINE_CELL,
            "the cloud snapshot must not be reachable"
        );
    }

    /// Python's `max()` returns the FIRST maximum; `max_by_key` returns the
    /// last. Two version strings can tie on key without tying on text, and the
    /// text is what gets published in "(vs X)".
    #[test]
    fn a_tie_on_version_key_keeps_the_first_snapshot() {
        let mut h = History::default();
        h.upsert(snap("0.20.0", "m5-air", "2026-01-01", &[("seq-sat", 1, 2)]));
        h.upsert(snap(
            "0.20.00",
            "m5-air",
            "2026-01-02",
            &[("seq-sat", 2, 2)],
        ));
        assert_eq!(
            VersionKey::parse("0.20.0"),
            VersionKey::parse("0.20.00"),
            "int(\"00\") == 0, so these tie"
        );
        let prev = h
            .comparable_predecessor(&BoxId::new("m5-air"), &VersionKey::parse("0.21.0"))
            .unwrap();
        assert_eq!(prev.version(), "0.20.0");
    }

    /// The self-comparison guard: a snapshot of the version being cut is the
    /// same measurement, and `_current_version` exists so the column does not
    /// read "= (vs X)" on every row.
    #[test]
    fn the_current_version_is_not_its_own_predecessor() {
        let h = real_history();
        let air = BoxId::new("m5-air");
        let prev = h
            .comparable_predecessor(&air, &VersionKey::parse("0.24.0"))
            .unwrap();
        assert_eq!(prev.version(), "0.23.0");
    }

    /// `_delta`'s four branches, including the U+2212 minus (not an ASCII
    /// hyphen) that the published tables carry.
    #[test]
    fn delta_renders_every_branch() {
        let mut h = History::default();
        h.upsert(snap(
            "0.23.0",
            "m5-air",
            "2026-08-16",
            &[("2018 seq-sat", 80, 240), ("empty board", 0, 0)],
        ));
        let p = h
            .comparable_predecessor(&BoxId::new("m5-air"), &VersionKey::parse("0.24.0"))
            .unwrap();
        assert_eq!(p.delta("2018 seq-sat", 82, 240), "+0.8 pts (vs 0.23.0)");
        assert_eq!(
            p.delta("2018 seq-sat", 79, 240),
            "\u{2212}0.4 pts (vs 0.23.0)"
        );
        assert!(p.delta("2018 seq-sat", 79, 240).contains('\u{2212}'));
        assert!(!p.delta("2018 seq-sat", 79, 240).contains('-'));
        assert_eq!(p.delta("2018 seq-sat", 80, 240), "= (vs 0.23.0)");
        assert_eq!(p.delta("a board it never ran", 1, 2), NEW_CELL);
        assert_eq!(p.delta("empty board", 1, 2), NEW_CELL);
        assert_eq!(delta_cell(None, "2018 seq-sat", 82, 240), BASELINE_CELL);
    }

    /// Shares, not counts. A board's denominator is not fixed across releases
    /// (the registry gains instances, and 0.25 grew the table by nine boards),
    /// and differencing raw counts would book instances ADDED as instances
    /// solved.
    #[test]
    fn delta_compares_shares_because_a_corpus_can_grow() {
        let mut h = History::default();
        h.upsert(snap("0.24.0", "m5-air", "2026-08-20", &[("board", 30, 60)]));
        let p = h
            .comparable_predecessor(&BoxId::new("m5-air"), &VersionKey::parse("0.25.0"))
            .unwrap();
        // 130/260 is the same HALF, on four times the corpus.
        assert_eq!(p.delta("board", 130, 260), "= (vs 0.24.0)");
    }

    /// The `= ` band is exactly the printed resolution: 0.05 points, tested on
    /// both sides so a re-derivation cannot quietly widen it.
    #[test]
    fn the_no_movement_band_is_half_of_the_printed_digit() {
        let mut h = History::default();
        h.upsert(snap("0.24.0", "m5-air", "2026-08-20", &[("b", 0, 100_000)]));
        let p = h
            .comparable_predecessor(&BoxId::new("m5-air"), &VersionKey::parse("0.25.0"))
            .unwrap();
        // 49/100000 -> 0.049 pts, inside the band; 51 -> 0.051, outside.
        assert_eq!(p.delta("b", 49, 100_000), "= (vs 0.24.0)");
        assert_eq!(p.delta("b", 51, 100_000), "+0.1 pts (vs 0.24.0)");
    }

    /// The committed file, read and written back, must be the same bytes: this
    /// document is rewritten on every release and a reformatting diff hides the
    /// one line that changed.
    #[test]
    fn the_committed_history_round_trips_byte_for_byte() {
        let p = format!("{REPO}/benchmarks/standings-history.json");
        let src = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"));
        let h = History::try_load(Path::new(&p)).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(h.to_json(), src);
    }

    /// `ensure_ascii=True`, which `serde_json` does not do: an em dash is
    /// `—` and an astral character is a surrogate PAIR. The notes field
    /// carries prose, and prose in this project carries em dashes.
    #[test]
    fn non_ascii_is_escaped_the_way_python_escapes_it() {
        let mut h = History::default();
        let mut s = snap("0.26.0", "m5-air", "2026-09-01", &[("b", 1, 2)]);
        // DEL is ascii and Python escapes it anyway; "/" is not escaped.
        s.note = "an \u{2014} em dash, a \u{1f600} face, a \"quote\", \u{7f}, a/b".to_string();
        h.upsert(s);
        let out = h.to_json();
        assert!(
            out.contains(
                r#""note": "an \u2014 em dash, a \ud83d\ude00 face, a \"quote\", \u007f, a/b""#
            ),
            "{out}"
        );
        assert!(out.is_ascii());
        // And it comes back the way it went in.
        let back = History::from_json(&out).unwrap();
        assert_eq!(back.snapshots()[0].note, h.snapshots()[0].note);
        assert_eq!(back.to_json(), out);
    }

    /// One space per indent level, `": "` between key and value, empty
    /// containers inline -- `json.dump(..., indent=1)`, not `indent=4` and not
    /// a compact dump.
    #[test]
    fn a_fresh_history_is_the_document_python_would_create() {
        let h = History::default();
        let out = h.to_json();
        assert!(out.starts_with("{\n \"_comment\": \""), "{out}");
        assert!(out.ends_with("\"snapshots\": []\n}\n"), "{out}");
    }

    /// Replace-append-sort: the identity is the PAIR (version, box), so the
    /// same tag on two boxes is two records and re-banking one leaves the other
    /// alone.
    #[test]
    fn upsert_replaces_only_the_same_version_on_the_same_box() {
        let mut h = History::default();
        h.upsert(snap("0.24.0", "m5-air", "2026-08-20", &[("b", 1, 10)]));
        h.upsert(snap("0.24.0", "cloud-4c", "2026-08-21", &[("b", 2, 10)]));
        h.upsert(snap("0.24.0", "m5-air", "2026-08-22", &[("b", 3, 10)]));
        assert_eq!(h.snapshots().len(), 2);
        let air = h
            .snapshots()
            .iter()
            .find(|s| s.measured_on == BoxId::new("m5-air"))
            .unwrap();
        assert_eq!(air.track("b"), Some((3, 10)));
        // Sorted by (measured_at, version): the cloud row was measured first.
        assert_eq!(h.snapshots()[0].measured_on, BoxId::new("cloud-4c"));
    }

    /// The file-layout sort ties on date, and then on version string. Both
    /// sorts are stable in both languages, so equal pairs keep insertion order.
    #[test]
    fn upsert_sorts_by_date_then_version() {
        let mut h = History::default();
        h.upsert(snap("0.9.0", "m5-air", "2026-01-01", &[]));
        h.upsert(snap("0.10.0", "m5-air", "2026-01-01", &[]));
        // String order, not version order -- this is a file layout, and the
        // Python sorts the same strings the same way.
        let got: Vec<&str> = h.snapshots().iter().map(|s| s.version.as_str()).collect();
        assert_eq!(got, ["0.10.0", "0.9.0"]);
    }

    /// A trend is plotted in VERSION order. By date, the 0.19.0 backfill draws
    /// a spike between 0.20.0 and 0.21.0 that no release ever had.
    #[test]
    fn trend_is_ordered_by_version_not_by_measurement() {
        let t = real_history().trend("2018 seq-sat", &BoxId::new("m5-air"));
        let versions: Vec<String> = t.points.iter().map(|(k, _)| k.to_string()).collect();
        // The releases every cut since has kept, in version order -- and the
        // cuts after 0.24.0 follow them in the same order. Not pinned to a
        // tail: this list grows by one every cycle, and a pinned tail is one
        // cut stale the moment the next cut promotes.
        assert!(
            versions.starts_with(&[
                "0.19.0".to_string(),
                "0.20.0".to_string(),
                "0.21.0".to_string(),
                "0.22.0".to_string(),
                "0.23.0".to_string(),
                "0.24.0".to_string(),
            ]),
            "{versions:?}"
        );
        assert!(
            versions
                .windows(2)
                .all(|w| w[0] < w[1] || w[0].len() < w[1].len()),
            "not in version order: {versions:?}"
        );
        assert_eq!(t.points[0].1, (63, 240));
        assert_eq!(t.label, "2018 seq-sat");
    }

    /// A board only some releases measured yields only those points, and a box
    /// nobody swept on yields none.
    #[test]
    fn trend_skips_releases_that_never_ran_the_board() {
        let h = real_history();
        let t = h.trend("2026 numeric-opt", &BoxId::new("m5-air"));
        // First banked at 0.21.0, so exactly two fewer points than the board
        // every release measured -- however many releases there are by now.
        let all = h.trend("2018 seq-sat", &BoxId::new("m5-air")).points.len();
        assert_eq!(t.points.len(), all - 2, "first banked at 0.21.0");
        assert_eq!(t.points[0].0.to_string(), "0.21.0");
        assert!(h
            .trend("seq-sat", &BoxId::new("nobodys-box"))
            .points
            .is_empty());
    }

    /// Python tuple order, reproduced: a prefix sorts below its extension, and
    /// components are compared as NUMBERS (so 10 > 9).
    #[test]
    fn version_keys_order_like_python_tuples() {
        assert!(VersionKey::parse("0.21") < VersionKey::parse("0.21.0"));
        assert!(VersionKey::parse("0.9.0") < VersionKey::parse("0.10.0"));
        assert!(VersionKey::parse("0.20.1") < VersionKey::parse("0.21.0"));
    }

    /// The `except ValueError: return (0,)` fallback -- which sorts below every
    /// real release, so an unreadable version never wins a selection.
    #[test]
    fn an_unparseable_version_falls_back_to_zero() {
        for s in ["", "0.21.0-rc1", "None", "v0.21.0", "0..1", "nightly"] {
            assert_eq!(VersionKey::parse(s), VersionKey(vec![0]), "{s}");
        }
        assert!(VersionKey::parse("None") < VersionKey::parse("0.19.0"));
        // And as `cur`, it selects nothing: no real release is below (0,).
        assert!(real_history()
            .comparable_predecessor(&BoxId::new("m5-air"), &VersionKey::parse("None"))
            .is_none());
    }

    /// `_current_version` against the real workspace manifest: the first
    /// `version` line of the ROOT Cargo.toml, quotes stripped.
    #[test]
    fn current_version_reads_the_workspace_manifest() {
        let v = current_version(Path::new(REPO)).expect("the root manifest has a version");
        assert_eq!(VersionKey::parse(&v).parts().len(), 3, "{v}");
        // Missing manifest degrades to None, like the Python's `except OSError`.
        assert!(current_version(Path::new("/nonexistent-box")).is_none());
    }

    /// A missing file is an empty history, not an error -- the Python's
    /// `if not os.path.exists`. A malformed one is an error, because the next
    /// step rewrites the file.
    #[test]
    fn a_missing_history_is_empty_and_a_broken_one_is_loud() {
        let missing = Path::new("/nonexistent-box/standings-history.json");
        assert!(History::load(missing).snapshots().is_empty());
        assert!(History::try_load(missing).is_ok());
        assert!(History::from_json("{\"snapshots\": [{}]}").is_err());
        // Refuse rather than lose: an unmodelled key would be deleted from the
        // record by the next rewrite.
        let doc = r#"{"snapshots": [{"version": "0.1.0", "measured_on": "b",
                       "measured_at": "2026-01-01", "conditions": "clean"}]}"#;
        assert!(History::from_json(doc).is_err());
    }

    /// Track objects: a null or zero total is "no denominator" and reads as a
    /// new board, exactly as `not p.get("total")` does.
    #[test]
    fn a_track_with_no_denominator_is_new_not_zero_percent() {
        let doc = r#"{"snapshots": [{"version": "0.1.0", "measured_on": "b",
                       "measured_at": "2026-01-01",
                       "tracks": {"a": {"solved": 0, "total": null},
                                  "b": {"total": 0}}}]}"#;
        let h = History::from_json(doc).unwrap();
        let p = h
            .comparable_predecessor(&BoxId::new("b"), &VersionKey::parse("0.2.0"))
            .unwrap();
        assert_eq!(p.delta("a", 5, 10), NEW_CELL);
        assert_eq!(p.delta("b", 5, 10), NEW_CELL);
    }

    /// Insertion order survives a load: the tracks list is the sweep registry's
    /// order, and a map type would alphabetise it on the way back out.
    #[test]
    fn track_order_is_the_files_order() {
        let h = real_history();
        let first = &h.snapshots()[0];
        let labels: Vec<&str> = first.tracks.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(labels[0], "seq-sat");
        assert_eq!(labels[1], "tempo-sat");
        assert!(labels.windows(2).any(|w| w[0] > w[1]), "not alphabetical");
    }

    /// A `\u` escape whose four bytes straddle a multi-byte character must be
    /// REFUSED, not panic. Slicing `&str[i..i + 4]` there aborts the process,
    /// and this parser runs inside a release step on a file anyone may have
    /// hand-edited; CPython answers the same bytes with `Invalid \uXXXX
    /// escape`. The `+` case is the other half: `from_str_radix` accepts a
    /// sign, and CPython's scanner does not.
    #[test]
    fn a_bad_unicode_escape_is_an_error_not_a_panic() {
        for tail in ["\u{20ac}\"}", "\u{2014}ab\"}", "+ab\"}", " 12\"}", "0x1\"}"] {
            let doc = format!("{{\"snapshots\": [], \"x\": \"\\u00{tail}");
            assert!(
                History::from_json(&doc).is_err(),
                "{doc:?} must be refused, not accepted"
            );
        }
        // A well-formed escape still decodes, in either case of hex, and a
        // surrogate PAIR still recombines into one astral character.
        let h =
            History::from_json("{\"snapshots\": [], \"x\": \"\\u2014\\uD83D\\ude00\"}").unwrap();
        assert_eq!(
            h.to_json(),
            "{\n \"snapshots\": [],\n \"x\": \"\\u2014\\ud83d\\ude00\"\n}\n"
        );
    }
}
