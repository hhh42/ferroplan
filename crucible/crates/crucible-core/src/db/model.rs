//! The typed things that cross the database boundary, and the one non-obvious
//! encoding among them.
//!
//! Nothing here talks to SQLite. It exists so the writer's command channel
//! carries values with names instead of a positional bag of `rusqlite::Value`,
//! and so the two places that have to agree about what an instance LABEL is --
//! the importer and the exporter -- agree by sharing a function rather than by
//! both being careful.
//!
//! # The sort key
//!
//! `ipc67.py:instances` sorts instance filenames by the TUPLE of integers of
//! their digit groups, and that order is the order every board raw is written
//! in. An exported board has to reproduce it from a database that does not know
//! the filename any more, and `ORDER BY label` will not do it: as text, "10"
//! sorts before "9", and "3_10_50_10" sorts before "3_9".
//!
//! [`sort_key`] encodes each group as its significant digits preceded by a
//! three-digit LENGTH, joined with '.'. Byte order over that encoding is
//! numeric-tuple order:
//!
//! * same digit count compares digit by digit, which is numeric order;
//! * different digit counts compare on the length prefix first, so 99 < 100;
//! * a shorter tuple is a strict prefix of a longer one that starts the same
//!   way, and a prefix sorts first -- which is Python's tuple rule, so (3, 10)
//!   still lands before (3, 10, 50, 10);
//! * '.' is 0x2E, below every digit, so a separator can never outrank a digit
//!   and let (1, 2) drift past (12,).
//!
//! Leading zeros are stripped before encoding because Python compares `int(g)`:
//! a multipart label keeps the filename's original digit strings ("07_1"), so
//! "07" and "7" are the same group and must produce the same key.

use crucible_publish::raw::{Instance, RawRow};

/// Width of the length prefix. Three digits covers a 999-digit group; the
/// corpus's longest is four digits. A group longer than that is a corpus bug,
/// and the encoding says so by refusing to be silently wrong -- see
/// [`sort_key`].
const LEN_WIDTH: usize = 3;

/// Byte-orderable form of an instance label. See the module header.
///
/// A group with more than 999 digits cannot be encoded in a three-digit prefix.
/// Rather than wrap and produce a subtly wrong ORDER BY, such a group is capped
/// at 999 and prefixed with '~' (0x7E, above every digit), which sorts it last
/// and keeps it visible. No such instance exists; this is here so that if one
/// ever does, it shows up at the end of a board instead of in the middle of it.
pub fn sort_key(label: &str) -> String {
    let mut out = String::new();
    for (i, group) in label.split('_').enumerate() {
        if i > 0 {
            out.push('.');
        }
        let digits = group.trim_start_matches('0');
        let digits = if digits.is_empty() { "0" } else { digits };
        if digits.len() > 999 {
            out.push('~');
            out.push_str(digits);
        } else {
            out.push_str(&format!("{:0width$}", digits.len(), width = LEN_WIDTH));
            out.push_str(digits);
        }
    }
    out
}

/// How an instance is identified in the database.
///
/// `label_is_int` is not cosmetic: it is what puts a JSON int back as an int on
/// export. A single-digit-group filename yields `int(gs[0])` and a multipart one
/// yields `"_".join(gs)`, and a board where the two are confused joins wrongly
/// against the archive and the bounds tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceKey {
    pub label: String,
    pub label_is_int: bool,
}

impl InstanceKey {
    pub fn of(i: &Instance) -> Self {
        match i {
            Instance::Num(n) => InstanceKey {
                label: n.to_string(),
                label_is_int: true,
            },
            Instance::Parts(s) => InstanceKey {
                label: s.clone(),
                label_is_int: false,
            },
        }
    }

    pub fn sort_key(&self) -> String {
        sort_key(&self.label)
    }

    /// Back to the raw's polymorphic form. A label stored as an int that no
    /// longer parses as one is a corrupted row, and degrading it to a string
    /// silently changes the row's identity -- so it stays a string and the
    /// caller sees a label it can recognise as wrong.
    pub fn to_instance(&self) -> Instance {
        if self.label_is_int {
            if let Ok(n) = self.label.parse::<u64>() {
                return Instance::Num(n);
            }
        }
        Instance::Parts(self.label.clone())
    }
}

/// A corpus VARIANT DIRECTORY. Not a PDDL domain: several variants share one
/// domain file, and one variant can carry a per-instance domain.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariantKey {
    pub ipc: Option<String>,
    pub name: String,
}

impl VariantKey {
    pub fn of(r: &RawRow) -> Self {
        VariantKey {
            ipc: r.ipc.clone(),
            name: r.variant.clone(),
        }
    }
}

/// Which binary. Keyed on BLAKE3 -- supplied by the caller, because
/// `crucible-core` hashes nothing itself -- and NEVER on a tag or a version
/// string; see the `engine` table's comment.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct EngineKey {
    /// `None` only for an engine reconstructed from artifacts.
    pub blake3: Option<String>,
    /// `ff --version` output. Not an identity: every dev build of a cycle
    /// reports the same string.
    pub ver: Option<String>,
}

/// Everything about an engine that is not its identity.
#[derive(Debug, Clone, Default)]
pub struct EngineFacts {
    /// NULL is the NORMAL case: the primary trigger is an untagged working tree.
    pub tag: Option<String>,
    pub commit_sha: Option<String>,
    pub binary_path: Option<String>,
    pub built_at: Option<i64>,
    pub build_status: Option<String>,
    pub build_log: Option<String>,
    pub rebuilt: bool,
}

/// The seven things a measurement is made of. The UNIQUE on `board` is over
/// exactly this tuple; see the table's comment for why a tier move must land in
/// a new row.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardKey {
    pub name: String,
    /// The ARMED wall -- the row's own `budget` stamp -- not the manifest's
    /// scored budget, which may differ for a cycle while a tier move is in
    /// flight.
    pub budget_secs: f64,
    pub mode: String,
    pub jobs: u32,
    /// `str(threads)`: the currency the resume gate compares in.
    pub threads: String,
    /// Canonical JSON object with sorted keys, so one environment cannot
    /// produce two board identities.
    pub env: String,
    /// Canonical JSON array; argument ORDER is part of the identity.
    pub args: String,
}

impl BoardKey {
    /// A hashable stand-in for the cache. `f64` is neither `Eq` nor `Hash`, and
    /// rounding it to make it so would merge a 30 s board with a 30.0000001 s
    /// one -- so the bits are the key.
    pub(crate) fn cache_key(&self) -> (String, u64, String, u32, String, String, String) {
        (
            self.name.clone(),
            self.budget_secs.to_bits(),
            self.mode.clone(),
            self.jobs,
            self.threads.clone(),
            self.env.clone(),
            self.args.clone(),
        )
    }
}

/// Reporting-only columns on `board`. Nothing joins on these.
#[derive(Debug, Clone, Default)]
pub struct BoardFacts {
    pub label: Option<String>,
    pub competition: Option<String>,
    pub proof_track: bool,
    /// The exact JSON token the raws carry for `threads` -- a string `"2"` in
    /// every real row, because `ipc67.py` passes the CLI argument through
    /// unconverted.
    pub threads_json: String,
}

/// The QUEUE state of a run. Deliberately NOT a taxonomy of results: this
/// codebase has exactly one classifier and it lives in `crucible_publish`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Pending,
    Running,
    Suspended,
    /// A row was recorded, whatever it says.
    Done,
    /// The runner died mid-instance and no row was ever written. `ipc67.py`
    /// gets this for free -- a row with no `end_ts` was never written at all --
    /// and the database has to say it out loud.
    Abandoned,
}

impl RunState {
    pub fn as_str(self) -> &'static str {
        match self {
            RunState::Pending => "pending",
            RunState::Running => "running",
            RunState::Suspended => "suspended",
            RunState::Done => "done",
            RunState::Abandoned => "abandoned",
        }
    }
}

/// Whether this run's TIMING can be trusted. Its RESULT is trusted regardless:
/// contention may cost a number, never a coverage point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingQuality {
    Clean,
    Dirty,
    /// Nobody was watching, or nobody can say. A rebuilt run is always this.
    Unknown,
}

impl TimingQuality {
    pub fn as_str(self) -> &'static str {
        match self {
            TimingQuality::Clean => "clean",
            TimingQuality::Dirty => "dirty",
            TimingQuality::Unknown => "unknown",
        }
    }
}

/// Why validation was UNAVAILABLE. Never why a plan was rejected -- a rejection
/// is `val = 0` and needs no excuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValReason {
    /// VAL cannot parse the domain at all. It emits this refusal BEFORE judging
    /// any plan, which is why it is not a verdict.
    Ingest,
    Crash,
    Timeout,
    /// No VAL binary was found, so nothing was submitted to it.
    NoValidator,
}

impl ValReason {
    pub fn as_str(self) -> &'static str {
        match self {
            ValReason::Ingest => "ingest",
            ValReason::Crash => "crash",
            ValReason::Timeout => "timeout",
            ValReason::NoValidator => "no-validator",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "ingest" => ValReason::Ingest,
            "crash" => ValReason::Crash,
            "timeout" => ValReason::Timeout,
            "no-validator" => ValReason::NoValidator,
            _ => return None,
        })
    }
}

/// What the supervisor measured. All of it absent on a rebuilt run.
#[derive(Debug, Clone, Default)]
pub struct Measured {
    pub started_at: Option<f64>,
    pub finished_at: Option<f64>,
    pub wall_ms: Option<u64>,
    pub cpu_ms: Option<u64>,
    pub suspended_ms: Option<u64>,
    pub peak_rss: Option<u64>,
    /// Which instrument enforced the memory budget, because the two measure
    /// different quantities: `RLIMIT_AS` caps address space, the watchdog caps
    /// resident bytes.
    pub mem_instrument: Option<String>,
    pub exit_code: Option<i32>,
    pub term_signal: Option<i32>,
    pub pid: Option<i32>,
    pub pgid: Option<i32>,
}

/// One instance, ready to be written. The `RawRow` inside is the receipt and is
/// stored field for field; everything around it is context.
#[derive(Debug, Clone)]
pub struct RunRecord {
    pub board: BoardKey,
    pub board_facts: BoardFacts,
    pub engine: EngineKey,
    pub engine_facts: EngineFacts,
    pub attempt: i64,
    pub state: RunState,
    pub timing: TimingQuality,
    pub val_reason: Option<ValReason>,
    pub row: RawRow,
    pub measured: Measured,
}

/// One tick of the contention watcher, box-wide.
#[derive(Debug, Clone, Default)]
pub struct SampleRec {
    pub at: f64,
    pub idle_pct: Option<f64>,
    /// NULL where the sampler could not attribute. `load_resume` reads that as
    /// DIRTY, and so must every query here.
    pub competitors_total: Option<f64>,
    pub loadavg1: Option<f64>,
    pub swap_mb: Option<f64>,
    pub cpu_speed_limit: Option<u32>,
    /// Set only on a sample IMPORTED from a board's conditions file. The live
    /// watcher's samples belong to no board.
    pub pass_id: Option<i64>,
    pub processes: Vec<(String, f64)>,
}

impl SampleRec {
    /// Build a row from what the watcher just measured.
    ///
    /// One place knows this mapping, so a field added to [`crate::monitor::Sample`]
    /// has exactly one place to be forgotten rather than one per call site --
    /// and the field most easily forgotten is `competitors_total`, which IS the
    /// verdict. It moved off `idle_pct` at 0.24 because idle is whole-machine
    /// and includes the board's own threads: a `--threads 8` mco board burns
    /// 40-80% of this box by design and could never clear a fixed idle floor
    /// even in an empty room.
    pub fn of(s: &crate::monitor::Sample) -> SampleRec {
        SampleRec {
            at: s.at,
            idle_pct: s.idle_pct,
            competitors_total: Some(s.competitors_total),
            loadavg1: s.loadavg1,
            swap_mb: s.swap_mb,
            cpu_speed_limit: s.cpu_speed_limit,
            pass_id: None,
            processes: s.competitors.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        }
    }
}

/// A line of the rolling log.
#[derive(Debug, Clone)]
pub struct EventRec {
    pub at: f64,
    pub level: &'static str,
    pub kind: &'static str,
    pub run_id: Option<i64>,
    pub board_id: Option<i64>,
    pub message: String,
}

/// A stretch of time during which the harness was not running flat out.
#[derive(Debug, Clone)]
pub struct ThrottleWindowRec {
    pub level: &'static str,
    pub started_at: f64,
    pub ended_at: Option<f64>,
    pub reason: Option<String>,
}

/// A spawned planner and the evidence that it is still the one we spawned.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveChild {
    pub pid: i32,
    pub pgid: i32,
    pub run_id: Option<i64>,
    pub binary_path: String,
    /// From `ProcIdentity`. Together with `binary_path` this is what a reaper
    /// checks BEFORE it signals anything, because killpg on a recycled pgid
    /// kills a stranger.
    pub proc_start_tvsec: i64,
    pub spawned_at: f64,
    pub stopped: bool,
}

impl LiveChild {
    /// The identity a reaper compares against a live `proc_identity(pid)`.
    /// A mismatch means the pid was recycled and this row is a ghost.
    pub fn identity(&self) -> crate::platform::ProcIdentity {
        crate::platform::ProcIdentity {
            path: self.binary_path.clone(),
            start_tvsec: self.proc_start_tvsec,
        }
    }
}

/// What contention.py concluded about a whole board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassVerdict {
    Clean,
    /// Spelled in capitals in every conditions file on disk, and kept that way
    /// so a stored verdict can be grepped against the artifact it came from.
    Degraded,
    Unknown,
    /// The board could not be measured because the engine under test does not
    /// have the feature it exercises. NOT a degraded measurement, and never to
    /// be counted as one.
    FeatureAbsent,
}

impl PassVerdict {
    pub fn as_str(self) -> &'static str {
        match self {
            PassVerdict::Clean => "clean",
            PassVerdict::Degraded => "DEGRADED",
            PassVerdict::Unknown => "unknown",
            PassVerdict::FeatureAbsent => "feature-absent",
        }
    }

    /// Parse a conditions file's `verdict`. Anything unrecognised -- including
    /// a missing key -- is `Unknown`, never `Clean`.
    pub fn parse(s: &str) -> Self {
        match s {
            "clean" => PassVerdict::Clean,
            "DEGRADED" => PassVerdict::Degraded,
            "feature-absent" => PassVerdict::FeatureAbsent,
            _ => PassVerdict::Unknown,
        }
    }
}

/// The `.done` marker, with the provenance a zero-byte file cannot carry.
#[derive(Debug, Clone)]
pub struct BoardPassRec {
    pub board: BoardKey,
    pub board_facts: BoardFacts,
    pub engine: EngineKey,
    pub engine_facts: EngineFacts,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub verdict: PassVerdict,
    pub ran: i64,
    pub reused: i64,
    pub done_marker: Option<String>,
    pub raw_path: Option<String>,
    pub conditions_path: Option<String>,
    pub sample_interval: Option<f64>,
    /// Identity for a rebuilt pass: re-importing the same file updates the row
    /// instead of adding one.
    pub source_path: Option<String>,
}

/// What a window of the contention timeline says about a run that ran inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cleanliness {
    /// Every overlapping sample was under the clean line.
    Clean,
    /// At least one overlapping sample was over it, or could not be attributed.
    Dirty,
    /// No timeline covers this window. `load_resume` fails CLOSED here and
    /// re-runs the instance; so must anything reading this.
    Uncovered,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The one property the encoding exists for: byte order over sort keys is
    /// the order `ipc67.py:instances` writes rows in. Sorting these as plain
    /// text puts "10" before "9" and breaks every exported board.
    #[test]
    fn sort_key_reproduces_the_numeric_tuple_order() {
        let mut labels = vec!["10", "9", "1", "100", "2", "20"];
        labels.sort_by_key(|l| sort_key(l));
        assert_eq!(labels, ["1", "2", "9", "10", "20", "100"]);
    }

    /// Python compares TUPLES, so a shorter tuple that is a prefix of a longer
    /// one sorts first -- (3, 10) before (3, 10, 50, 10). A fixed-width padded
    /// key gets this right by accident; it is asserted here so a future
    /// encoding cannot lose it.
    #[test]
    fn a_shorter_tuple_sorts_before_the_one_it_prefixes() {
        let mut labels = vec!["3_10_50_10", "3_10", "3_9", "3_10_50"];
        labels.sort_by_key(|l| sort_key(l));
        assert_eq!(labels, ["3_9", "3_10", "3_10_50", "3_10_50_10"]);
    }

    /// The separator must never outrank a digit, or (1, 2) drifts past (12,).
    #[test]
    fn a_group_boundary_outranks_nothing() {
        let mut labels = vec!["12", "1_2", "1"];
        labels.sort_by_key(|l| sort_key(l));
        assert_eq!(labels, ["1", "1_2", "12"]);
    }

    /// A multipart label keeps the filename's original digit strings, so "07"
    /// and "7" are the same group to Python's `int(g)` and must key the same.
    #[test]
    fn leading_zeros_are_not_a_different_instance() {
        assert_eq!(sort_key("07_1"), sort_key("7_1"));
        assert_eq!(sort_key("000"), sort_key("0"));
    }

    /// `label_is_int` is what puts a JSON int back as an int. Losing it turns
    /// instance 7 into "7" and silently breaks the archive join.
    #[test]
    fn an_int_label_round_trips_as_an_int() {
        let k = InstanceKey::of(&Instance::Num(7));
        assert!(k.label_is_int);
        assert_eq!(k.to_instance(), Instance::Num(7));

        let k = InstanceKey::of(&Instance::Parts("3_10_50_10".into()));
        assert!(!k.label_is_int);
        assert_eq!(k.to_instance(), Instance::Parts("3_10_50_10".into()));
    }

    /// An unrecognised verdict must degrade to `unknown`, never to `clean`.
    /// Defaulting the other way is how a board claims a cleanliness nobody
    /// measured.
    #[test]
    fn an_unknown_verdict_is_not_clean() {
        assert_eq!(PassVerdict::parse("clean"), PassVerdict::Clean);
        assert_eq!(PassVerdict::parse("DEGRADED"), PassVerdict::Degraded);
        assert_eq!(PassVerdict::parse("Clean"), PassVerdict::Unknown);
        assert_eq!(PassVerdict::parse(""), PassVerdict::Unknown);
    }
}
