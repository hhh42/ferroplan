//! One row of a per-board `.jsonl`, exactly as `benchmarks/ipc67.py` writes it.
//!
//! Three fields are polymorphic in ways that are load-bearing, and each has
//! cost this project a wrong published number when flattened:
//!
//! * `instance` is a JSON **int** for a single-digit-group filename and an
//!   underscore-joined **string** (`"3_10_50_10"`) for a multipart one.
//!   Collapsing to the first group put `ipc2026-numeric`'s 320 rows under 288
//!   keys and silently broke the per-instance diff and the `--score-against`
//!   join (`ipc67.py:268-275`).
//! * `notes` is `null`, a **string** (runner-stamped classes: `mem-cap`,
//!   `spawn-fail`, `engine-exit-N`) or a **list of strings** (the engine's own
//!   `Solution.notes`). The classifier's mechanism tests read one text.
//! * `val` is a **tristate**: `true` valid, `false` REJECTED, `null`
//!   UNAVAILABLE. `null` is not a verdict, and reading it as one is the 0.20,
//!   0.21 and 0.23 incidents (`ipc67.py:291-363`).

use std::borrow::Cow;

/// An instance label. The int/string split is part of the contract, not an
/// encoding accident -- `as_num` is the only bridge to the archive and bounds
/// keys, so a multipart label cannot silently join to `p07`.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum Instance {
    Num(u64),
    Parts(String),
}

impl Instance {
    pub fn as_num(&self) -> Option<u64> {
        match self {
            Instance::Num(n) => Some(*n),
            Instance::Parts(_) => None,
        }
    }
}

impl std::fmt::Display for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instance::Num(n) => write!(f, "{n}"),
            Instance::Parts(s) => write!(f, "{s}"),
        }
    }
}

/// Engine notes arrive as a list; runner-stamped classes as a bare string.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum Notes {
    One(String),
    Many(Vec<serde_json::Value>),
}

impl Notes {
    /// The single text the mechanism tests read.
    ///
    /// Python: `notes if isinstance(notes, str) else " ".join(str(x) for x in notes)`,
    /// reached only after `notes or ""` -- so an EMPTY list and `null` are
    /// indistinguishable, both yielding `""`. A one-element list joins to that
    /// element exactly, which is why `["mem-cap"]` still matches the `mem-cap`
    /// test while `["a", "b"]` matches nothing.
    pub fn text(&self) -> Cow<'_, str> {
        match self {
            Notes::One(s) => Cow::Borrowed(s),
            Notes::Many(v) => {
                let parts: Vec<String> = v
                    .iter()
                    .map(|x| match x {
                        // Python's str() on a str is the bare string; on
                        // anything else it is the repr. Only strings occur in
                        // practice, but a non-string must not gain quotes it
                        // would not have in Python.
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .collect();
                Cow::Owned(parts.join(" "))
            }
        }
    }
}

/// A per-instance result row.
///
/// `solved` carries no `#[serde(default)]` on purpose: Python reads it as
/// `r["solved"]`, a bare index that raises `KeyError`. Rust fails the parse
/// instead and names the file and line -- a strictly better failure with the
/// same outcome, and never a silent `false`.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RawRow {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ipc: Option<String>,
    pub variant: String,
    pub instance: Instance,
    pub solved: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<serde_json::Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub val: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<Notes>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ver: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jobs: Option<u32>,
    /// A JSON **string** in every real row (`"2"`), because `ipc67.py` passes
    /// the CLI arg through unconverted and the resume gate compares
    /// `str(threads)`. Kept as a Value so the round-trip cannot change its type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threads: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_ts: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_ts: Option<f64>,
    /// Last, and absent on unsolved rows: it enters the Python dict via
    /// `rec.update(...)` after `end_ts` was already inserted, so it appends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub makespan: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub resumed_clean: bool,
    /// The runner stamps new columns every cycle (`budget` at 0.23, `makespan`
    /// at 0.22, `resumed_clean` at 0.25). Publication must never fail to read a
    /// board because the runner learned a new one.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,

    /// Which optional keys were physically present in the source line.
    ///
    /// `Option<T>` alone cannot carry this, and the difference is load-bearing:
    /// a SOLVED non-temporal row writes `"makespan": null`, while an UNSOLVED
    /// row omits the key entirely. Both parse to `makespan: None`, so without a
    /// presence flag a re-serialized board differs from every committed raw.
    #[serde(skip)]
    pub present: Present,
}

/// Which optional keys a row physically carried.
///
/// The runner grew its schema in three steps and the raws on this box show all
/// seven resulting key sequences: `makespan` arrived at 0.22, `budget` at 0.23,
/// and the resume stamps (`ver`/`mode`/`jobs`/`threads`/`start_ts`/`end_ts`) at
/// 0.25 as one block. Reproducing a board means reproducing which of them the
/// row had.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Present {
    pub ipc: bool,
    pub budget: bool,
    /// The 0.25 resume-stamp block: all six keys, or none of them.
    pub stamps: bool,
    /// Written only on the solved branch, where the value may still be null.
    pub makespan: bool,
    pub resumed_clean: bool,
}

impl Present {
    /// Derive presence from the parsed object.
    pub fn of(o: &serde_json::Map<String, serde_json::Value>) -> Self {
        Present {
            ipc: o.contains_key("ipc"),
            budget: o.contains_key("budget"),
            stamps: o.contains_key("ver"),
            makespan: o.contains_key("makespan"),
            resumed_clean: o.contains_key("resumed_clean"),
        }
    }

    /// What `ipc67.py` would have written for a freshly measured row: every
    /// current column, with `makespan` on the solved branch only.
    pub fn current(solved: bool) -> Self {
        Present {
            ipc: true,
            budget: true,
            stamps: true,
            makespan: solved,
            resumed_clean: false,
        }
    }
}

impl RawRow {
    /// Elapsed seconds, or `None` on a pre-0.20 row that never recorded it.
    pub fn time_secs(&self) -> Option<f64> {
        self.time.as_ref().and_then(|n| n.as_f64())
    }

    /// The mechanism text, with `null` and `[]` both collapsing to `""`.
    pub fn note_text(&self) -> Cow<'_, str> {
        match &self.notes {
            Some(n) => n.text(),
            None => Cow::Borrowed(""),
        }
    }

    /// `"{ipc}/{variant}"`, the key of the VAL-unavailable map.
    ///
    /// Python builds this with an f-string over `r.get('ipc')`, so a row with
    /// no `ipc` yields the literal `"None/..."`. No key in
    /// `benchmarks/val-unavailable.json` begins with `None/` and none can
    /// (`Referee::new` asserts it), so returning `None` here is behaviourally
    /// identical and avoids porting a typo as if it were a rule.
    pub fn domain_key(&self) -> Option<String> {
        self.ipc.as_ref().map(|i| format!("{i}/{}", self.variant))
    }
}

/// Serialize one row exactly as `ipc67.py` writes it.
///
/// The key ORDER is the Python dict's insertion order and is not negotiable:
/// `run_instance` builds the record with a fixed literal, `rec.update(...)`
/// appends `makespan` on the solved branch only (which is why it lands LAST,
/// after `end_ts`), the resume stitch appends `resumed_clean` after that, and
/// `main` prepends `ipc`/`variant` via `{"ipc": ipc, "variant": v, **r}`.
///
/// `time` is the one genuinely polymorphic number: the hard-timeout path
/// assigns the integer budget (`rec["time"] = TIMEOUT`) while every other path
/// writes `round(el, 2)`. Keeping it a `serde_json::Number` preserves the
/// token. `budget` is an integer in all 26,022 stamped rows on this box, so it
/// is written as one whenever it is whole.
pub fn write_row(r: &RawRow, out: &mut String) {
    use crate::pyjson::{write_str, write_value};

    let mut first = true;
    let mut key = |k: &str, out: &mut String| {
        if !first {
            out.push_str(", ");
        }
        first = false;
        write_str(k, out);
        out.push_str(": ");
    };

    out.push('{');
    if r.present.ipc {
        key("ipc", out);
        match &r.ipc {
            Some(s) => write_str(s, out),
            None => out.push_str("null"),
        }
    }
    key("variant", out);
    write_str(&r.variant, out);
    key("instance", out);
    match &r.instance {
        Instance::Num(n) => out.push_str(&n.to_string()),
        Instance::Parts(s) => write_str(s, out),
    }
    key("solved", out);
    out.push_str(if r.solved { "true" } else { "false" });

    // These five are ALWAYS written, null when unset -- they are literals in
    // the record `run_instance` builds, so no row has ever omitted one.
    key("time", out);
    match &r.time {
        Some(n) => out.push_str(&n.to_string()),
        None => out.push_str("null"),
    }
    key("metric", out);
    write_num(r.metric, out);
    key("length", out);
    match r.length {
        Some(n) => out.push_str(&n.to_string()),
        None => out.push_str("null"),
    }
    key("val", out);
    match r.val {
        Some(true) => out.push_str("true"),
        Some(false) => out.push_str("false"),
        None => out.push_str("null"),
    }
    key("notes", out);
    match &r.notes {
        Some(Notes::One(s)) => write_str(s, out),
        Some(Notes::Many(v)) => write_value(&serde_json::Value::Array(v.clone()), out),
        None => out.push_str("null"),
    }

    if r.present.budget {
        key("budget", out);
        match r.budget {
            Some(b) if b.fract() == 0.0 => out.push_str(&format!("{}", b as i64)),
            Some(b) => out.push_str(&fmt_f64(b)),
            None => out.push_str("null"),
        }
    }
    if r.present.stamps {
        key("ver", out);
        match &r.ver {
            Some(s) => write_str(s, out),
            None => out.push_str("null"),
        }
        key("mode", out);
        match &r.mode {
            Some(s) => write_str(s, out),
            None => out.push_str("null"),
        }
        key("jobs", out);
        match r.jobs {
            Some(n) => out.push_str(&n.to_string()),
            None => out.push_str("null"),
        }
        key("threads", out);
        match &r.threads {
            Some(v) => write_value(v, out),
            None => out.push_str("null"),
        }
        key("start_ts", out);
        write_num(r.start_ts, out);
        key("end_ts", out);
        write_num(r.end_ts, out);
    }
    if r.present.makespan {
        key("makespan", out);
        write_num(r.makespan, out);
    }
    if r.present.resumed_clean {
        key("resumed_clean", out);
        out.push_str(if r.resumed_clean { "true" } else { "false" });
    }
    // Columns this crate does not model, kept: the runner stamps a new one
    // most cycles, and the resume gate's `engine` hash is one of them. A
    // writer that dropped them would silently un-stamp every row it rewrote,
    // and the next pass would re-measure the lot. Last, in key order -- after
    // every key `run_instance` writes, so no committed row changes.
    for (k, v) in &r.extra {
        key(k, out);
        write_value(v, out);
    }
    out.push('}');
}

fn write_num(v: Option<f64>, out: &mut String) {
    match v {
        Some(x) => out.push_str(&fmt_f64(x)),
        None => out.push_str("null"),
    }
}

/// Python's `repr(float)` and Rust's shortest-round-trip formatter agree on
/// every value in this corpus, but Rust prints a bare `36` for `36.0_f64` in
/// some paths, so force the decimal point the way Python always does.
fn fmt_f64(x: f64) -> String {
    let s = format!("{x}");
    if s.contains(['.', 'e', 'E']) || s.contains("inf") || s.contains("NaN") {
        s
    } else {
        format!("{s}.0")
    }
}
