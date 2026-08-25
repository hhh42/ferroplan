//! The `<board>.conditions.json` document: what ELSE the machine was doing
//! while the board was measured.
//!
//! A board measured against a browser, a Spotlight reindex or a `cargo build`
//! in another worktree is not a slow board, it is a WRONG board -- and the
//! failure is asymmetric, because contention only ever DEPRESSES coverage. It
//! manufactures regressions and hides gains, which is the expensive direction
//! to be wrong in when the output is a release record. So every board carries
//! its own conditions the way a plan carries its own certificate, and this is
//! the file that makes that claim auditable after the fact.
//!
//! Ported from `benchmarks/contention.py:summarize`. Five things in here look
//! like they could be tidied and cannot be:
//!
//! * **The percentile is a truncating index.** `s[min(len(s)-1, int(len(s)*p))]`
//!   on the sorted list. It is neither nearest-rank nor interpolated, and it is
//!   not `statistics.quantiles`. Every `p25` ever published came out of that
//!   expression; "fixing" it re-verdicts boards that were judged years ago and
//!   silently rewrites the record they were judged against.
//!
//! * **`loadavg.median` is a DIFFERENT median.** That one really is
//!   `statistics.median`, which averages the two middle values on an even
//!   count. Two medians, two rules, in one document -- reproduced, not
//!   reconciled.
//!
//! * **The verdict is named-competitor load, not idle.** This moved at 0.24 and
//!   the reason must not be lost: `idle_pct` is whole-machine and includes the
//!   board's OWN threads, so a `--threads 8` mco board burns 40-80% of this
//!   ten-core box BY DESIGN and could never clear a fixed idle floor even in an
//!   empty room (measured: mco-t8 read 38-40% idle against 4-5% of real
//!   competing load). The line itself is [`SAMPLE_CLEAN_PCPU`], imported and
//!   never re-typed -- both Python files carry a comment saying it is kept in
//!   one place so the whole-run rule and the per-sample resume rule cannot
//!   drift apart, and a second copy of `25.0` here would be exactly that drift.
//!   The comparison is against the ROUNDED total, because that is the variable
//!   the Python tests: a raw 24.96 rounds to 25.0 and is NOT clean.
//!
//! * **`competitors_mean_pcpu` divides by the SAMPLE COUNT.** It is each
//!   competitor's accumulated pcpu over the whole board divided by the number
//!   of samples, so a process that spiked once does not read like one that ran
//!   the whole time. Publishing the accumulated total instead would make every
//!   long board look catastrophically contended.
//!
//! * **`0` and `0.0` are different bytes.** The timeline's third column is
//!   `round(sum(comp_now.values()), 1)`, and `sum()` of an EMPTY dict is the
//!   int `0`, not the float `0.0`. Real files are full of both -- 244 of 364
//!   samples in `timeline-mco-t2.json` are the bare int -- so the distinction
//!   is "were there any competitors", not "is the value zero".
//!
//! And one thing about reading: 72 of the 76 committed conditions files predate
//! the timeline and stop at `verdict`. An ABSENT key and a `null` key are
//! different documents, which is why [`Slot`] exists rather than
//! `Option<Option<T>>`.
//!
//! The provenance trio (`idle_source`, `aggregate`, `self_exclusion`) is new
//! here and is appended after `verdict` so nothing that reads the older shape
//! is disturbed. This repo annotates its instrument changes obsessively; a
//! rollup that does not say which instrument produced it, how it aggregated,
//! and whose processes it left out would be the odd one out.

use crate::monitor::SAMPLE_CLEAN_PCPU;
use crucible_publish::fmt::py_round;
use crucible_publish::pyjson;
use serde_json::{Number, Value};

/// How many competing processes get named. `contention.py:TOP_N`.
pub const TOP_N: usize = 4;

/// The verdicts, spelled exactly as the Python spells them -- `DEGRADED` is
/// upper-case because it is meant to be impossible to skim past in a log.
pub const CLEAN: &str = "clean";
pub const DEGRADED: &str = "DEGRADED";
pub const UNKNOWN: &str = "unknown";

/// A key that a pre-0.24 conditions file simply does not have.
///
/// `Absent` and `Null` are different bytes and a round-trip must not confuse
/// them. Three keys are affected -- `competitors_total_pcpu`, `interval`,
/// `timeline` -- and all three are missing entirely from the 72 older files
/// rather than present and null.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Slot<T> {
    #[default]
    Absent,
    Null,
    Value(T),
}

impl<T> Slot<T> {
    pub fn as_option(&self) -> Option<&T> {
        match self {
            Slot::Value(v) => Some(v),
            _ => None,
        }
    }

    /// True when the key is written at all -- `null` counts, missing does not.
    pub fn is_present(&self) -> bool {
        !matches!(self, Slot::Absent)
    }
}

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Slot<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // `#[serde(default)]` on the field is what produces `Absent`; reaching
        // this impl at all means the key was physically there.
        Ok(match Option::<T>::deserialize(d)? {
            Some(v) => Slot::Value(v),
            None => Slot::Null,
        })
    }
}

/// One sample of the timeline: `[epoch_ts, idle_pct_or_null, competitors_total]`.
///
/// All three columns are modelled nullable because the Python's own resume gate
/// tests `t[0] is not None` and `t[2] is None` before trusting a row -- the
/// reader contemplates nulls even where the writer never emits them, and a port
/// that made them non-nullable would reject a file the Python reads.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct TimelineEntry(pub Option<Number>, pub Option<Number>, pub Option<Number>);

impl TimelineEntry {
    pub fn new(
        at: Option<Number>,
        idle_pct: Option<Number>,
        competitors_total: Option<Number>,
    ) -> Self {
        TimelineEntry(at, idle_pct, competitors_total)
    }

    /// Epoch seconds, `round(time.time(), 1)`.
    pub fn at(&self) -> Option<f64> {
        self.0.as_ref().and_then(Number::as_f64)
    }

    pub fn idle_pct(&self) -> Option<f64> {
        self.1.as_ref().and_then(Number::as_f64)
    }

    pub fn competitors_total(&self) -> Option<f64> {
        self.2.as_ref().and_then(Number::as_f64)
    }
}

/// `idle_pct`.
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
pub struct IdlePct {
    pub median: Option<Number>,
    pub p25: Option<Number>,
    pub min: Option<Number>,
}

/// `loadavg`. Its median is `statistics.median`, not the truncating percentile.
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
pub struct LoadAvg {
    pub median: Option<Number>,
    pub max: Option<Number>,
}

/// `swap_mb`. A swapping box slows search while looking CPU-idle, which is why
/// it is recorded at all.
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
pub struct SwapMb {
    pub max: Option<Number>,
}

/// `cpu_speed_limit`. Non-zero means the kernel reported a thermal warning --
/// on a fanless chassis a long sweep is exactly when that shows up.
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
pub struct CpuSpeedLimit {
    pub min: Option<Number>,
}

/// The named competitors, in the order the document lists them.
///
/// Order is descending by accumulated pcpu and it is OBSERVABLE -- it is the
/// answer to "who was the worst offender". A `HashMap` or a `BTreeMap` would
/// re-alphabetise it, so entries stay a `Vec` and deserialization walks the
/// document's own order (serde_json's `MapAccess` yields entries as they appear
/// in the text).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Competitors(pub Vec<(String, Option<Number>)>);

impl<'de> serde::Deserialize<'de> for Competitors {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = Competitors;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a map of competitor name to mean pcpu")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut m: M,
            ) -> Result<Competitors, M::Error> {
                let mut out = Vec::new();
                while let Some((k, v)) = m.next_entry::<String, Option<Number>>()? {
                    out.push((k, v));
                }
                Ok(Competitors(out))
            }
        }
        d.deserialize_map(V)
    }
}

/// Which instrument produced the rollup, how it aggregated, and whose processes
/// it left out.
///
/// Appended after `verdict`, never interleaved with the older keys: a consumer
/// that reads the pre-0.24 shape sees exactly what it saw before.
#[derive(Debug, Clone, PartialEq)]
pub struct Provenance {
    pub idle_source: String,
    pub aggregate: String,
    /// The substrings that mark a process as OURS. Recording them matters
    /// because the 25% line was calibrated against a specific exclusion set; a
    /// board judged with a different one was judged by a different instrument.
    pub self_exclusion: Vec<String>,
}

/// What `idle_pct` is measured with. `top -l 2` because the FIRST sample of
/// `top` is since-boot and would report the box as idle no matter what.
pub const IDLE_SOURCE_TOP: &str = "top -l 2 -n 0 -s 1 (CPU usage line, % idle)";

/// How `median`/`p25` are taken. Written out because the obvious reading of
/// "p25" is a percentile function, and this is not one.
pub const AGGREGATE_TRUNCATING_INDEX: &str =
    "sorted samples, index min(n-1, int(n*p)) -- truncating, not nearest-rank, not interpolated";

impl Provenance {
    /// The current instrument's answers, with the caller's exclusion list.
    pub fn of(self_exclusion: Vec<String>) -> Self {
        Provenance {
            idle_source: IDLE_SOURCE_TOP.to_string(),
            aggregate: AGGREGATE_TRUNCATING_INDEX.to_string(),
            self_exclusion,
        }
    }
}

/// The `<board>.conditions.json` document, in document order.
///
/// Field order here IS the file's key order: Python dicts iterate in insertion
/// order and `json.dump` writes them that way, so the struct is the schema.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Conditions {
    pub started: String,
    pub ended: String,
    pub samples: u64,
    pub idle_pct: IdlePct,
    pub loadavg: LoadAvg,
    /// Required, not defaulted: every conditions file this project has ever
    /// written carries the key, and quietly inventing an empty one would let a
    /// document round-trip into a DIFFERENT document.
    pub competitors_mean_pcpu: Competitors,
    #[serde(default)]
    pub competitors_total_pcpu: Slot<Number>,
    pub swap_mb: SwapMb,
    pub cpu_speed_limit: CpuSpeedLimit,
    pub verdict: String,
    #[serde(default)]
    pub idle_source: Slot<String>,
    #[serde(default)]
    pub aggregate: Slot<String>,
    #[serde(default)]
    pub self_exclusion: Slot<Vec<String>>,
    #[serde(default)]
    pub interval: Slot<Number>,
    #[serde(default)]
    pub timeline: Slot<Vec<TimelineEntry>>,
}

/// A conditions file that will not parse.
#[derive(Debug, thiserror::Error)]
pub enum ConditionsError {
    #[error("{path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
}

impl Conditions {
    /// Read a conditions document, naming the file on a bad one.
    ///
    /// A MISSING file is not this function's problem -- the callers that tolerate
    /// one (`load_resume` returns `{}`) never get here.
    pub fn parse(text: &str, path: &str) -> Result<Conditions, ConditionsError> {
        serde_json::from_str(text).map_err(|source| ConditionsError::Parse {
            path: path.to_string(),
            source,
        })
    }

    /// The file's bytes: `json.dump(..., indent=1)` plus the trailing newline
    /// the Python writes separately.
    pub fn to_json(&self) -> String {
        let mut fields: Vec<(&str, String)> = vec![
            ("started", jstr(&self.started)),
            ("ended", jstr(&self.ended)),
            ("samples", Number::from(self.samples).to_string()),
            (
                "idle_pct",
                obj(
                    2,
                    &[
                        ("median", jnum(&self.idle_pct.median)),
                        ("p25", jnum(&self.idle_pct.p25)),
                        ("min", jnum(&self.idle_pct.min)),
                    ],
                ),
            ),
            (
                "loadavg",
                obj(
                    2,
                    &[
                        ("median", jnum(&self.loadavg.median)),
                        ("max", jnum(&self.loadavg.max)),
                    ],
                ),
            ),
            ("competitors_mean_pcpu", {
                let entries: Vec<(&str, String)> = self
                    .competitors_mean_pcpu
                    .0
                    .iter()
                    .map(|(k, v)| (k.as_str(), jnum(v)))
                    .collect();
                obj(2, &entries)
            }),
        ];
        push_slot(
            &mut fields,
            "competitors_total_pcpu",
            &self.competitors_total_pcpu,
            |n| n.to_string(),
        );
        fields.push(("swap_mb", obj(2, &[("max", jnum(&self.swap_mb.max))])));
        fields.push((
            "cpu_speed_limit",
            obj(2, &[("min", jnum(&self.cpu_speed_limit.min))]),
        ));
        fields.push(("verdict", jstr(&self.verdict)));
        push_slot(&mut fields, "idle_source", &self.idle_source, |s| jstr(s));
        push_slot(&mut fields, "aggregate", &self.aggregate, |s| jstr(s));
        push_slot(&mut fields, "self_exclusion", &self.self_exclusion, |v| {
            let arr = Value::Array(v.iter().map(|s| Value::String(s.clone())).collect());
            nested(&arr, 2)
        });
        push_slot(&mut fields, "interval", &self.interval, |n| n.to_string());
        push_slot(&mut fields, "timeline", &self.timeline, |t| {
            let arr = Value::Array(
                t.iter()
                    .map(|e| {
                        Value::Array(vec![
                            opt_num_value(&e.0),
                            opt_num_value(&e.1),
                            opt_num_value(&e.2),
                        ])
                    })
                    .collect(),
            );
            nested(&arr, 2)
        });
        let mut out = obj(1, &fields);
        out.push('\n');
        out
    }
}

/// Everything `contention.py:main` accumulates over a board, in the order it
/// accumulates it.
///
/// `competitor_totals` is a `Vec`, not a map, and that is load-bearing twice
/// over: the whole-board total is `sum(comp_totals.values())` in INSERTION
/// order (float addition is not associative), and Python's `sorted` is stable,
/// so ties in the top-4 ranking fall back to first-seen order too.
#[derive(Debug, Clone, Default)]
pub struct Rollup {
    pub started: String,
    pub idles: Vec<f64>,
    pub loads: Vec<f64>,
    pub swaps: Vec<f64>,
    pub therms: Vec<i64>,
    pub competitor_totals: Vec<(String, f64)>,
    pub timeline: Vec<TimelineEntry>,
    /// The sampler's `--interval`, seconds. `None` writes `null`, which is what
    /// `summarize`'s default argument does.
    pub interval: Option<f64>,
}

/// One pass of the sampling loop.
pub struct Reading<'a> {
    /// `time.time()`, unrounded -- the rounding is this module's job.
    pub at: f64,
    pub idle_pct: Option<f64>,
    pub loadavg: Option<f64>,
    pub swap_mb: Option<f64>,
    pub cpu_speed_limit: Option<i64>,
    /// This sample's competitors, ALREADY in the order the sampler ranked them
    /// (descending pcpu, top 4). The order decides first-seen order in
    /// `competitor_totals`, which decides tie-breaks and float summation order
    /// in the rollup.
    pub competitors: &'a [(String, f64)],
}

impl Rollup {
    /// Fold one sample in, exactly as the Python loop does.
    ///
    /// A `None` reading is DROPPED rather than recorded as zero -- `samples` is
    /// `len(idles)`, so a failed `top` shortens the record instead of diluting
    /// it with a fake 0% idle.
    pub fn observe(&mut self, r: &Reading<'_>) {
        if let Some(i) = r.idle_pct {
            self.idles.push(i);
        }
        if let Some(l) = r.loadavg {
            self.loads.push(l);
        }
        if let Some(s) = r.swap_mb {
            self.swaps.push(s);
        }
        if let Some(t) = r.cpu_speed_limit {
            self.therms.push(t);
        }
        for (name, pcpu) in r.competitors {
            match self.competitor_totals.iter_mut().find(|(n, _)| n == name) {
                Some((_, v)) => *v += pcpu,
                None => self.competitor_totals.push((name.clone(), *pcpu)),
            }
        }
        // `sum()` of an EMPTY sequence is the int `0`; of anything else a float.
        // Both spellings are all over the committed timelines, and the split is
        // "were there competitors", not "is the value zero".
        let total = if r.competitors.is_empty() {
            Some(Number::from(0))
        } else {
            let mut s = 0.0;
            for (_, pcpu) in r.competitors {
                s += pcpu;
            }
            num(py_round(s, 1))
        };
        self.timeline.push(TimelineEntry(
            num(py_round(r.at, 1)),
            r.idle_pct.and_then(|i| num(py_round(i, 1))),
            total,
        ));
    }
}

/// `contention.py:summarize`.
///
/// `ended` is a parameter rather than a clock read so this stays a pure
/// function of its inputs -- the whole file is rewritten on every sample, and a
/// writer that consulted the clock could not be tested against a fixture.
pub fn summarize(r: &Rollup, ended: &str, provenance: Option<&Provenance>) -> Conditions {
    let n = r.idles.len();
    // `max(len(idles), 1)`: the divisor never reaches zero, so a board that
    // never got a single idle reading still writes a document instead of
    // raising.
    let denom = n.max(1) as f64;

    let median = percentile(&r.idles, 0.5);
    let p25 = percentile(&r.idles, 0.25);
    let min_idle = fold_min(&r.idles).map(|v| py_round(v, 1));

    // Float addition is not associative: this walks `competitor_totals` in
    // insertion order because the Python walks the dict in insertion order.
    let mut comp_sum = 0.0;
    for (_, v) in &r.competitor_totals {
        comp_sum += v;
    }
    let comp_total = if r.idles.is_empty() {
        None
    } else {
        Some(py_round(comp_sum / denom, 1))
    };

    // `sorted(..., key=lambda kv: -kv[1])` is a STABLE descending sort, so
    // equal totals keep first-seen order. Rust's `sort_by` is stable too.
    let mut ranked = r.competitor_totals.clone();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked.truncate(TOP_N);
    let competitors = Competitors(
        ranked
            .into_iter()
            .map(|(k, v)| (k, num(py_round(v / denom, 1))))
            .collect(),
    );

    // The verdict reads the ROUNDED total, because that is the variable the
    // Python compares: a raw 24.96 becomes 25.0 and is not clean.
    let verdict = if comp_total.is_some_and(|c| c < SAMPLE_CLEAN_PCPU) {
        CLEAN
    } else if median.is_some() {
        DEGRADED
    } else {
        UNKNOWN
    };

    Conditions {
        started: r.started.clone(),
        ended: ended.to_string(),
        samples: n as u64,
        idle_pct: IdlePct {
            median: median.and_then(num),
            p25: p25.and_then(num),
            min: min_idle.and_then(num),
        },
        loadavg: LoadAvg {
            median: statistics_median(&r.loads)
                .map(|v| py_round(v, 2))
                .and_then(num),
            max: fold_max(&r.loads).map(|v| py_round(v, 2)).and_then(num),
        },
        competitors_mean_pcpu: competitors,
        competitors_total_pcpu: match comp_total.and_then(num) {
            Some(v) => Slot::Value(v),
            None => Slot::Null,
        },
        swap_mb: SwapMb {
            max: fold_max(&r.swaps).map(|v| py_round(v, 1)).and_then(num),
        },
        cpu_speed_limit: CpuSpeedLimit {
            min: r.therms.iter().copied().min().map(Number::from),
        },
        verdict: verdict.to_string(),
        idle_source: slot_str(provenance.map(|p| p.idle_source.clone())),
        aggregate: slot_str(provenance.map(|p| p.aggregate.clone())),
        self_exclusion: match provenance {
            Some(p) => Slot::Value(p.self_exclusion.clone()),
            None => Slot::Absent,
        },
        interval: match r.interval.and_then(num) {
            Some(v) => Slot::Value(v),
            None => Slot::Null,
        },
        timeline: Slot::Value(r.timeline.clone()),
    }
}

/// What the rollup claims, recomputed from the timeline alone.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TimelineRollup {
    pub samples: usize,
    pub median: Option<f64>,
    pub p25: Option<f64>,
    pub min: Option<f64>,
    /// `None` when any sample's competitor column is `null` -- the recompute
    /// fails CLOSED rather than treating a missing reading as zero load, which
    /// is the same call `load_resume` makes about a null third column.
    pub competitors_total_pcpu: Option<f64>,
}

/// Recompute the rollup from the fine print.
///
/// The rollup at the top of a conditions file is supposed to be a pure function
/// of the timeline underneath it, and it holds exactly on every committed file
/// that carries one. That is worth being able to check at any time: a rollup
/// that no longer follows from its own samples means the two were written by
/// different code paths, and the whole-board verdict stops being evidence.
pub fn rollup_from_timeline(timeline: &[TimelineEntry]) -> TimelineRollup {
    let idles: Vec<f64> = timeline.iter().filter_map(|e| e.idle_pct()).collect();
    let mut total = Some(0.0f64);
    for e in timeline {
        match (total.as_mut(), e.competitors_total()) {
            (Some(acc), Some(v)) => *acc += v,
            (Some(_), None) => total = None,
            (None, _) => {}
        }
    }
    let denom = idles.len().max(1) as f64;
    TimelineRollup {
        samples: idles.len(),
        median: percentile(&idles, 0.5),
        p25: percentile(&idles, 0.25),
        min: fold_min(&idles).map(|v| py_round(v, 1)),
        competitors_total_pcpu: if idles.is_empty() {
            None
        } else {
            total.map(|t| py_round(t / denom, 1))
        },
    }
}

/// `contention.py`'s local `pct(v, p)`.
///
/// `int(len(s) * p)` TRUNCATES. At four samples `p25` is index 1 and the median
/// is index 2 -- the upper of the two middle values, not their average. Every
/// published `p25` came out of this expression.
pub fn percentile(v: &[f64], p: f64) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    // `as usize` truncates toward zero and saturates, which is `int()` on a
    // non-negative float. `n * 0.5` and `n * 0.25` are exact in binary for any
    // count this sampler can reach, so no tie is manufactured here.
    let idx = ((s.len() as f64) * p) as usize;
    Some(py_round(s[idx.min(s.len() - 1)], 1))
}

/// `statistics.median` -- the OTHER median in this document. Averages the two
/// middle values on an even count, where [`percentile`] takes the upper one.
pub fn statistics_median(v: &[f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    let n = s.len();
    Some(if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    })
}

/// Python's `min()`: the FIRST minimum, kept with a strict `<`.
fn fold_min(v: &[f64]) -> Option<f64> {
    let mut it = v.iter().copied();
    let mut best = it.next()?;
    for x in it {
        if x < best {
            best = x;
        }
    }
    Some(best)
}

/// Python's `max()`: the FIRST maximum. `max_by` would return the last.
fn fold_max(v: &[f64]) -> Option<f64> {
    let mut it = v.iter().copied();
    let mut best = it.next()?;
    for x in it {
        if x > best {
            best = x;
        }
    }
    Some(best)
}

/// A finite reading as a JSON number.
///
/// A non-finite one becomes `null`. Python would write a bare `Infinity`, which
/// no other JSON reader accepts; recording "no reading" is the honest
/// degradation and is unreachable anyway -- idle comes from a `[\d.]+` match and
/// the load average from `getloadavg`.
fn num(x: f64) -> Option<Number> {
    Number::from_f64(x)
}

fn slot_str(s: Option<String>) -> Slot<String> {
    match s {
        Some(v) => Slot::Value(v),
        None => Slot::Absent,
    }
}

fn push_slot<T>(
    fields: &mut Vec<(&'static str, String)>,
    key: &'static str,
    slot: &Slot<T>,
    render: impl Fn(&T) -> String,
) {
    match slot {
        Slot::Absent => {}
        Slot::Null => fields.push((key, "null".to_string())),
        Slot::Value(v) => fields.push((key, render(v))),
    }
}

fn opt_num_value(n: &Option<Number>) -> Value {
    match n {
        Some(n) => Value::Number(n.clone()),
        None => Value::Null,
    }
}

fn jstr(s: &str) -> String {
    let mut out = String::new();
    pyjson::write_str(s, &mut out);
    out
}

fn jnum(n: &Option<Number>) -> String {
    match n {
        Some(n) => n.to_string(),
        None => "null".to_string(),
    }
}

/// One object in `indent=1` style, its members indented `depth` spaces.
///
/// Written out rather than handed to `serde_json` because key ORDER is the
/// schema here and `serde_json::Map` is a `BTreeMap` in this workspace -- it
/// would alphabetise the document into `competitors_mean_pcpu, ..., verdict`
/// and diff against all 76 committed files.
fn obj(depth: usize, fields: &[(&str, String)]) -> String {
    if fields.is_empty() {
        // Python's indent mode keeps empty containers on one line.
        return "{}".to_string();
    }
    let pad = " ".repeat(depth);
    let mut s = String::from("{\n");
    for (i, (k, v)) in fields.iter().enumerate() {
        if i > 0 {
            s.push_str(",\n");
        }
        s.push_str(&pad);
        pyjson::write_str(k, &mut s);
        s.push_str(": ");
        s.push_str(v);
    }
    s.push('\n');
    s.push_str(&" ".repeat(depth - 1));
    s.push('}');
    s
}

/// A value whose key order does not matter, rendered by `pyjson::write_indent1`
/// and pushed down to `depth`.
///
/// `write_indent1` renders a value as if it were the whole document; nesting it
/// one level deeper adds exactly one space after every newline. That is a safe
/// textual shift because `pyjson::write_str` escapes newlines inside strings,
/// so no literal `\n` in the output belongs to a string.
fn nested(v: &Value, depth: usize) -> String {
    let mut s = String::new();
    pyjson::write_indent1(v, &mut s);
    if depth > 1 {
        s.replace('\n', &format!("\n{}", " ".repeat(depth - 1)))
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MCO_T2: &str = include_str!("../../../../tests/fixtures/conditions/timeline-mco-t2.json");
    const COMPLEX_PREF: &str =
        include_str!("../../../../tests/fixtures/conditions/timeline-complex-pref.json");
    const NUMERIC_OPT: &str =
        include_str!("../../../../tests/fixtures/conditions/timeline-numeric-opt.json");
    const OPT_FULL: &str =
        include_str!("../../../../tests/fixtures/conditions/timeline-opt-full.json");
    const ROLLUP_ONLY: &str =
        include_str!("../../../../tests/fixtures/conditions/rollup-only.json");
    const DEGRADED_T4: &str =
        include_str!("../../../../tests/fixtures/conditions/degraded-old-idle-rule-mco-t4.json");
    const DEGRADED_T8: &str =
        include_str!("../../../../tests/fixtures/conditions/degraded-old-idle-rule-mco-t8.json");

    const WITH_TIMELINE: &[(&str, &str)] = &[
        ("timeline-mco-t2.json", MCO_T2),
        ("timeline-complex-pref.json", COMPLEX_PREF),
        ("timeline-numeric-opt.json", NUMERIC_OPT),
        ("timeline-opt-full.json", OPT_FULL),
    ];

    const ALL_FIXTURES: &[(&str, &str)] = &[
        ("timeline-mco-t2.json", MCO_T2),
        ("timeline-complex-pref.json", COMPLEX_PREF),
        ("timeline-numeric-opt.json", NUMERIC_OPT),
        ("timeline-opt-full.json", OPT_FULL),
        ("rollup-only.json", ROLLUP_ONLY),
        ("degraded-old-idle-rule-mco-t4.json", DEGRADED_T4),
        ("degraded-old-idle-rule-mco-t8.json", DEGRADED_T8),
    ];

    /// THE invariant this file exists to keep: the rollup at the top of a
    /// conditions document must follow from the timeline underneath it. If it
    /// ever stops, the two were produced by different code and the whole-board
    /// verdict is no longer evidence about the samples it claims to summarise.
    #[test]
    fn the_rollup_is_a_pure_function_of_the_timeline() {
        for (name, text) in WITH_TIMELINE {
            let c = Conditions::parse(text, name).unwrap();
            let tl = c.timeline.as_option().unwrap();
            let re = rollup_from_timeline(tl);
            assert_eq!(re.samples as u64, c.samples, "{name}: sample count");
            assert_eq!(num(re.median.unwrap()), c.idle_pct.median, "{name}: median");
            assert_eq!(num(re.p25.unwrap()), c.idle_pct.p25, "{name}: p25");
            assert_eq!(num(re.min.unwrap()), c.idle_pct.min, "{name}: min");
            assert_eq!(
                num(re.competitors_total_pcpu.unwrap()),
                c.competitors_total_pcpu.as_option().cloned(),
                "{name}: competitors_total_pcpu"
            );
        }
    }

    /// Every committed conditions file, read and written back byte for byte.
    /// This is what proves the emitter reproduces `json.dump(..., indent=1)` --
    /// key order, one-space indent, `", "`/`": "`, the trailing newline, and the
    /// int-vs-float spelling of every number token.
    #[test]
    fn every_committed_conditions_file_round_trips_byte_for_byte() {
        for (name, text) in ALL_FIXTURES {
            let c = Conditions::parse(text, name).unwrap();
            assert_eq!(&c.to_json(), text, "{name} did not round-trip");
        }
    }

    /// The old files stop at `verdict`: those three keys are ABSENT, not null,
    /// and writing `"timeline": null` where the Python wrote nothing at all
    /// would diff against 72 of the 76 real files.
    #[test]
    fn a_pre_timeline_file_keeps_its_missing_keys_missing() {
        let c = Conditions::parse(ROLLUP_ONLY, "rollup-only.json").unwrap();
        assert_eq!(c.competitors_total_pcpu, Slot::Absent);
        assert_eq!(c.interval, Slot::Absent);
        assert_eq!(c.timeline, Slot::Absent);
        assert!(!c.to_json().contains("timeline"));
    }

    /// The truncating index, at the lengths where every competing definition of
    /// a percentile disagrees with this one. Values are the sorted list itself
    /// so the assertions read as "which element", not "which number".
    #[test]
    fn the_percentile_is_a_truncating_index() {
        // len 1: int(1*0.5) = 0, int(1*0.25) = 0.
        assert_eq!(percentile(&[7.0], 0.5), Some(7.0));
        assert_eq!(percentile(&[7.0], 0.25), Some(7.0));
        // len 2: int(2*0.5) = 1 -- the UPPER value, not the average. A true
        // median would answer 15.0 here.
        assert_eq!(percentile(&[10.0, 20.0], 0.5), Some(20.0));
        assert_eq!(percentile(&[10.0, 20.0], 0.25), Some(10.0));
        // len 3: int(3*0.5) = 1, int(3*0.75... ) -> p25 index 0.
        assert_eq!(percentile(&[10.0, 20.0, 30.0], 0.5), Some(20.0));
        assert_eq!(percentile(&[10.0, 20.0, 30.0], 0.25), Some(10.0));
        // len 4: median index 2, p25 index 1. Nearest-rank would say index 1
        // and 0; linear interpolation would invent 25.0 and 17.5.
        assert_eq!(percentile(&[10.0, 20.0, 30.0, 40.0], 0.5), Some(30.0));
        assert_eq!(percentile(&[10.0, 20.0, 30.0, 40.0], 0.25), Some(20.0));
        // len 100: indices 50 and 25, both exact.
        let hundred: Vec<f64> = (0..100).map(|i| i as f64).collect();
        assert_eq!(percentile(&hundred, 0.5), Some(50.0));
        assert_eq!(percentile(&hundred, 0.25), Some(25.0));
        // Unsorted input must sort first, and the result is rounded to 1dp.
        assert_eq!(percentile(&[30.05, 10.0, 20.0], 0.5), Some(20.0));
        assert_eq!(percentile(&[], 0.5), None);
    }

    /// `loadavg.median` is `statistics.median`, which averages the two middle
    /// values -- deliberately NOT the same rule as `idle_pct.median` three lines
    /// above it in the same document.
    #[test]
    fn loadavg_uses_the_other_median() {
        assert_eq!(statistics_median(&[10.0, 20.0]), Some(15.0));
        assert_eq!(percentile(&[10.0, 20.0], 0.5), Some(20.0));
        assert_eq!(statistics_median(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(statistics_median(&[]), None);
    }

    fn sample(at: f64, idle: f64, comps: &[(&str, f64)]) -> Rollup {
        let mut r = Rollup::default();
        let c: Vec<(String, f64)> = comps.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        r.observe(&Reading {
            at,
            idle_pct: Some(idle),
            loadavg: Some(1.0),
            swap_mb: Some(1.0),
            cpu_speed_limit: None,
            competitors: &c,
        });
        r
    }

    /// The mean is per SAMPLE, not the accumulated total. A process that pegged
    /// one core for one sample out of a hundred reads as 1.0, not 100.0 -- and
    /// the whole-board verdict is built on that number.
    #[test]
    fn a_one_sample_spike_does_not_read_like_sustained_load() {
        let mut r = Rollup::default();
        let spike: Vec<(String, f64)> = vec![("hog".to_string(), 100.0)];
        for i in 0..100 {
            r.observe(&Reading {
                at: 1000.0 + i as f64,
                idle_pct: Some(80.0),
                loadavg: Some(1.0),
                swap_mb: None,
                cpu_speed_limit: None,
                competitors: if i == 0 { &spike } else { &[] },
            });
        }
        let c = summarize(&r, "2026-08-25 00:00:00", None);
        assert_eq!(
            c.competitors_mean_pcpu.0[0].1,
            num(1.0),
            "100 pcpu over 100 samples is a mean of 1.0"
        );
        assert_eq!(c.verdict, CLEAN);
    }

    /// The 0.24 rule, both sides of it. The line lives in
    /// `monitor::SAMPLE_CLEAN_PCPU` and is never re-typed here, because the
    /// whole-run verdict and the per-sample resume gate drifting apart is the
    /// exact failure both Python files warn about.
    #[test]
    fn the_verdict_is_competitor_load_and_the_line_is_exclusive() {
        // A thread-heavy board in an empty room: 38% idle by design, clean.
        let r = sample(1000.0, 38.0, &[("logd", 4.5)]);
        assert_eq!(summarize(&r, "x", None).verdict, CLEAN);

        // The rounded total is what is compared: 24.96 rounds to 25.0, which is
        // not below the line.
        let r = sample(1000.0, 90.0, &[("hog", 24.96)]);
        let c = summarize(&r, "x", None);
        assert_eq!(c.competitors_total_pcpu, Slot::Value(num(25.0).unwrap()));
        assert_eq!(c.verdict, DEGRADED);

        let r = sample(1000.0, 90.0, &[("hog", 24.9)]);
        assert_eq!(summarize(&r, "x", None).verdict, CLEAN);

        // No samples at all: no median, so not even DEGRADED.
        let empty = Rollup::default();
        assert_eq!(summarize(&empty, "x", None).verdict, UNKNOWN);
        assert_eq!(
            summarize(&empty, "x", None).competitors_total_pcpu,
            Slot::Null
        );
    }

    /// `sum()` over an empty dict is the int `0`. 244 of the 364 samples in
    /// `timeline-mco-t2.json` are spelled that way, and `0.0` in their place is
    /// a byte diff on two thirds of every timeline this project has written.
    #[test]
    fn an_empty_competitor_set_writes_the_int_zero() {
        let mut r = Rollup::default();
        r.observe(&Reading {
            at: 1787445537.0,
            idle_pct: Some(74.8),
            loadavg: None,
            swap_mb: None,
            cpu_speed_limit: None,
            competitors: &[],
        });
        // A competitor that sums to zero is still a FLOAT sum.
        r.observe(&Reading {
            at: 1787445558.4,
            idle_pct: Some(87.9),
            loadavg: None,
            swap_mb: None,
            cpu_speed_limit: None,
            competitors: &[("ghost".to_string(), 0.0)],
        });
        assert_eq!(r.timeline[0].2.as_ref().unwrap().to_string(), "0");
        assert_eq!(r.timeline[1].2.as_ref().unwrap().to_string(), "0.0");
        // And the epoch stamps keep the shape the committed files use.
        assert_eq!(
            r.timeline[0].0.as_ref().unwrap().to_string(),
            "1787445537.0"
        );
        assert_eq!(
            r.timeline[1].0.as_ref().unwrap().to_string(),
            "1787445558.4"
        );
    }

    /// Ties in the top-4 ranking fall back to FIRST-SEEN order, because Python's
    /// `sorted` is stable over a dict in insertion order. Alphabetising them
    /// would rename the worst offender on any board with a tie.
    #[test]
    fn equal_competitors_keep_first_seen_order() {
        let mut r = Rollup::default();
        let comps: Vec<(String, f64)> = vec![
            ("zulu".to_string(), 9.0),
            ("alpha".to_string(), 9.0),
            ("mike".to_string(), 20.0),
        ];
        r.observe(&Reading {
            at: 1.0,
            idle_pct: Some(50.0),
            loadavg: None,
            swap_mb: None,
            cpu_speed_limit: None,
            competitors: &comps,
        });
        let c = summarize(&r, "x", None);
        let names: Vec<&str> = c
            .competitors_mean_pcpu
            .0
            .iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(names, vec!["mike", "zulu", "alpha"]);
    }

    /// Only the top four are named, however many were seen -- but the TOTAL
    /// counts all of them, which is why a board can show 3.4 of named load and
    /// a 9.1 total (`timeline-mco-t2.json` does exactly that).
    #[test]
    fn the_total_counts_competitors_the_document_never_names() {
        let mut r = Rollup::default();
        let comps: Vec<(String, f64)> = (0..8).map(|i| (format!("p{i}"), 10.0)).collect();
        r.observe(&Reading {
            at: 1.0,
            idle_pct: Some(50.0),
            loadavg: None,
            swap_mb: None,
            cpu_speed_limit: None,
            competitors: &comps,
        });
        let c = summarize(&r, "x", None);
        assert_eq!(c.competitors_mean_pcpu.0.len(), TOP_N);
        assert_eq!(c.competitors_total_pcpu, Slot::Value(num(80.0).unwrap()));
    }

    /// The provenance trio is appended AFTER `verdict` and before the timeline
    /// keys, and a document written without it is byte-identical to what the
    /// Python writes. Both shapes have to keep working: the differential runs
    /// against the un-annotated form.
    #[test]
    fn provenance_is_appended_after_the_verdict() {
        let mut r = sample(1000.0, 80.0, &[("logd", 6.0)]);
        r.started = "2026-08-25 09:00:00".to_string();
        r.interval = Some(20.0);

        let plain = summarize(&r, "2026-08-25 09:30:00", None).to_json();
        assert!(!plain.contains("idle_source"));
        assert_eq!(
            plain
                .find("\"verdict\"")
                .map(|i| plain[i..].find("\"interval\"").is_some()),
            Some(true)
        );

        let prov = Provenance::of(vec![
            "target/release/ff".to_string(),
            "Validate".to_string(),
        ]);
        let annotated = summarize(&r, "2026-08-25 09:30:00", Some(&prov)).to_json();
        let v = annotated.find("\"verdict\"").unwrap();
        let s = annotated.find("\"idle_source\"").unwrap();
        let a = annotated.find("\"aggregate\"").unwrap();
        let x = annotated.find("\"self_exclusion\"").unwrap();
        let i = annotated.find("\"interval\"").unwrap();
        assert!(v < s && s < a && a < x && x < i, "order: {annotated}");
        // Everything before `verdict` is untouched by the annotation.
        assert_eq!(plain[..v], annotated[..v]);
        // And the annotated document is still readable and still round-trips.
        let back = Conditions::parse(&annotated, "annotated.json").unwrap();
        assert_eq!(back.to_json(), annotated);
        assert_eq!(back.self_exclusion.as_option().unwrap().len(), 2);
    }

    /// A board that never produced a competitor list still writes `{}`, and
    /// Python's indent mode keeps an empty container on one line.
    #[test]
    fn an_empty_competitor_map_stays_on_one_line() {
        let mut r = Rollup::default();
        r.observe(&Reading {
            at: 1.0,
            idle_pct: Some(99.0),
            loadavg: Some(0.5),
            swap_mb: None,
            cpu_speed_limit: None,
            competitors: &[],
        });
        let out = summarize(&r, "x", None).to_json();
        assert!(out.contains("\"competitors_mean_pcpu\": {},\n"), "{out}");
        assert!(out.ends_with("}\n"));
    }

    /// The whole pipeline against CPython. These bytes were produced by running
    /// `benchmarks/contention.py:summarize` on the five readings below and
    /// dumping the result with `json.dump(..., indent=1)`, so this is the one
    /// test that checks the ARITHMETIC and the FORMATTING together rather than
    /// re-emitting bytes that were already correct.
    ///
    /// Chosen to make every hazard fire at once: an idle reading that goes
    /// missing (so `samples` is 4 while the timeline is 5 long), a sample with
    /// no competitors at all (the int `0`), two half-to-even roundings that a
    /// naive `f64::round` gets wrong (74.85 to 74.8, 80.55 to 80.5), a
    /// competitor first seen on the fourth sample, a thermal reading of exactly
    /// zero (an int, and one that must not be mistaken for "no reading"), and
    /// epoch stamps long enough to expose any exponent formatting.
    #[test]
    fn a_whole_document_matches_cpython() {
        /// at, idle_pct, loadavg, swap_mb, cpu_speed_limit, competitors.
        type R = (
            f64,
            Option<f64>,
            Option<f64>,
            Option<f64>,
            Option<i64>,
            &'static [(&'static str, f64)],
        );
        const READINGS: &[R] = &[
            (
                1787445537.0,
                Some(74.85),
                Some(2.005),
                Some(100.25),
                Some(0),
                &[("Brave Browser", 30.0), ("logd", 5.5)],
            ),
            (1787445558.4, Some(87.9), Some(1.0), Some(200.0), None, &[]),
            (
                1787445579.9,
                Some(80.55),
                Some(3.5),
                Some(150.0),
                Some(50),
                &[("logd", 5.5), ("Brave Browser", 5.5)],
            ),
            (
                1787445601.3,
                None,
                Some(4.25),
                None,
                None,
                &[("kernel_task", 12.34)],
            ),
            (1787445622.7, Some(44.45), Some(2.5), Some(50.0), None, &[]),
        ];
        const EXPECTED: &str = concat!(
            "{\n",
            " \"started\": \"2026-08-25 09:00:00\",\n",
            " \"ended\": \"2026-08-25 09:30:00\",\n",
            " \"samples\": 4,\n",
            " \"idle_pct\": {\n  \"median\": 80.5,\n  \"p25\": 74.8,\n  \"min\": 44.5\n },\n",
            " \"loadavg\": {\n  \"median\": 2.5,\n  \"max\": 4.25\n },\n",
            " \"competitors_mean_pcpu\": {\n",
            "  \"Brave Browser\": 8.9,\n  \"kernel_task\": 3.1,\n  \"logd\": 2.8\n },\n",
            " \"competitors_total_pcpu\": 14.7,\n",
            " \"swap_mb\": {\n  \"max\": 200.0\n },\n",
            " \"cpu_speed_limit\": {\n  \"min\": 0\n },\n",
            " \"verdict\": \"clean\",\n",
            " \"interval\": 20.0,\n",
            " \"timeline\": [\n",
            "  [\n   1787445537.0,\n   74.8,\n   35.5\n  ],\n",
            "  [\n   1787445558.4,\n   87.9,\n   0\n  ],\n",
            "  [\n   1787445579.9,\n   80.5,\n   11.0\n  ],\n",
            "  [\n   1787445601.3,\n   null,\n   12.3\n  ],\n",
            "  [\n   1787445622.7,\n   44.5,\n   0\n  ]\n",
            " ]\n}\n",
        );

        let mut r = Rollup {
            started: "2026-08-25 09:00:00".to_string(),
            interval: Some(20.0),
            ..Default::default()
        };
        for (at, idle, load, swap, therm, comps) in READINGS {
            let c: Vec<(String, f64)> = comps.iter().map(|(k, v)| (k.to_string(), *v)).collect();
            r.observe(&Reading {
                at: *at,
                idle_pct: *idle,
                loadavg: *load,
                swap_mb: *swap,
                cpu_speed_limit: *therm,
                competitors: &c,
            });
        }
        let doc = summarize(&r, "2026-08-25 09:30:00", None).to_json();
        assert_eq!(doc, EXPECTED);
        // And it reads back as itself, so the parser agrees with CPython too.
        assert_eq!(Conditions::parse(&doc, "d.json").unwrap().to_json(), doc);
    }

    /// A conditions file cut off mid-write is DATA, not a bug: it names the file
    /// and returns an error rather than panicking.
    #[test]
    fn a_truncated_file_is_an_error_not_a_panic() {
        let e = Conditions::parse("{\"started\": \"x\"", "half.json").unwrap_err();
        assert!(e.to_string().starts_with("half.json:"), "{e}");
    }
}
