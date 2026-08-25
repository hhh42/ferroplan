//! The three quality scorers, and the one rule they all obey: a board can
//! never acquire a number it did not measure.
//!
//! Ported from `benchmarks/standings.py` -- `makespan_quality` (:369), the
//! propositional length pass inlined at :823, and `bounds_quality` (:981).
//! Three different currencies, one shape: score every row that both sides can
//! be read for, keep W/T/L against the reference, and report the mean of the
//! capped ratios.
//!
//! The incident this module is built around is the 0.23 Phase 3 temporal
//! re-baseline. The runner only started recording `makespan` at 0.22, so a
//! cloud-era raw carries coverage and nothing else; a scorer that "helpfully"
//! filled the column would have published a makespan verdict for a sweep that
//! never measured one. The Python defends that by returning `None` and letting
//! the caller keep its coverage-only note. Here it is stronger than a
//! convention: `Wtl::build` is the ONLY constructor and it refuses an empty
//! ratio list, so a `Wtl` cannot exist without a measured row behind it. The
//! fallback is a `QualityNote::Fixed` string supplied by the manifest.
//!
//! Two more boundaries are load-bearing and are commented where they sit:
//!
//! * `MS_TIE`. The archive's plans stagger at 0.01 where ours epsilon-separate
//!   at 0.001, so a difference smaller than one epsilon slot at the coarsest
//!   granularity in play is bookkeeping, not a result. It bands W/T/L only --
//!   the ratio stays raw division, uncushioned, because the mean is a quality
//!   measure and must not be flattered.
//! * The archive and bounds joins go through `Instance::as_num`, so a
//!   multipart label (`"3_10_50_10"`) can never meet an integer archive key.
//!   That is the same join that put `ipc2026-numeric`'s 320 rows under 288
//!   keys when it was flattened; a "fix" that makes these labels join is a
//!   wrong number, not a repair. The variant-to-track table itself lives in
//!   `archive::arch_track`, where the archive that answers it lives; a row
//!   whose variant that table declines is simply not scored.

use crate::archive::{arch_key, Ipc5Archive};
use crate::bounds::BestKnownBounds;
use crate::raw::RawRow;
use crate::referee::Referee;

/// Makespan W/T/L tie band (`standings.py:366`).
///
/// One epsilon slot at the COARSEST granularity on either side of the
/// comparison: sgplan's archive plans stagger at 0.01, ours epsilon-separate at
/// 0.001. Inside the band the two plans finish at the same time and the
/// difference is epsilon bookkeeping, which must never book a win or a loss.
pub const MS_TIE: f64 = 0.011;

/// What a board is scored IN. Each track has exactly one honest currency, and
/// the prefix is how the table says which one it used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    /// IPC-5 propositional: plan LENGTH against the archive field's lengths.
    Length,
    /// IPC-5 time / metric-time: MAKESPAN against the archive field's.
    Makespan,
    /// The modern corpora: cost against the official best-known bounds.
    Bounds,
}

impl Currency {
    /// The cell's leading text, verbatim from the committed table
    /// (`benchmarks/ipc-standings.md`, the propositional/time/metric-time rows
    /// and the 2018/2023 satisficing rows). These strings are published, so
    /// they are checked against the artifact rather than re-invented.
    pub fn prefix(self) -> &'static str {
        match self {
            Currency::Length => "len vs best-of-field",
            Currency::Makespan => "makespan vs best-of-field",
            Currency::Bounds => "vs best-known bounds",
        }
    }
}

/// A scored board: wins, ties, losses, and the mean capped ratio behind them.
///
/// The fields are private and there is no other constructor on purpose -- see
/// the module header. `n` is the number of rows that were actually scored, not
/// the size of the board, and the table prints it so a reader can see how much
/// of a board the verdict rests on.
#[derive(Debug, Clone, PartialEq)]
pub struct Wtl {
    currency: Currency,
    w: usize,
    t: usize,
    l: usize,
    mean: f64,
    n: usize,
}

impl Wtl {
    /// The only way to make a `Wtl`, and it declines when nothing was scored.
    ///
    /// Python: `if not ratios: return None`. Everything downstream of that line
    /// -- the coverage-only note, the "raw predates the 0.22 makespan column"
    /// fallback -- exists because this returns nothing rather than a zero.
    pub(crate) fn build(
        currency: Currency,
        w: usize,
        t: usize,
        l: usize,
        ratios: &[f64],
    ) -> Option<Wtl> {
        if ratios.is_empty() {
            return None;
        }
        // Summed in ROW ITERATION ORDER, left to right, exactly as Python's
        // `sum(ratios)/len(ratios)`. Float addition is not associative: a
        // reordered or chunked sum can land the mean on the other side of a
        // 2-decimal boundary and move a published number.
        let mean = ratios.iter().sum::<f64>() / ratios.len() as f64;
        Some(Wtl {
            currency,
            w,
            t,
            l,
            mean,
            n: ratios.len(),
        })
    }

    pub fn currency(&self) -> Currency {
        self.currency
    }

    pub fn w(&self) -> usize {
        self.w
    }

    pub fn t(&self) -> usize {
        self.t
    }

    pub fn l(&self) -> usize {
        self.l
    }

    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// Rows scored -- the denominator of the mean, never the board's size.
    pub fn n(&self) -> usize {
        self.n
    }

    pub fn render(&self) -> String {
        self.to_string()
    }
}

impl std::fmt::Display for Wtl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Rust's `{:.2}` and Python's `{:.2f}` both round the EXACT binary
        // value, ties to even, so they agree digit for digit -- no `py_round`
        // detour, which would round twice and could disagree with both.
        write!(
            f,
            "{}: {}W/{}T/{}L, mean quality {:.2} ({} scored)",
            self.currency.prefix(),
            self.w,
            self.t,
            self.l,
            self.mean,
            self.n
        )
    }
}

/// What goes in a board's quality cell.
///
/// The two arms are not interchangeable: `Scored` is a measurement, `Fixed` is
/// prose the manifest supplies for a board that has no scorable currency (or
/// no raw that carries one). Keeping them apart in the type is what stops a
/// fallback string and a computed verdict being edited into each other.
#[derive(Debug, Clone, PartialEq)]
pub enum QualityNote {
    Fixed(String),
    Scored(Wtl),
}

impl QualityNote {
    /// Python's `q = scorer(...) or qnote`: the measured verdict when there is
    /// one, the manifest's fixed note otherwise.
    pub fn new(scored: Option<Wtl>, fallback: impl Into<String>) -> Self {
        match scored {
            Some(wtl) => QualityNote::Scored(wtl),
            None => QualityNote::Fixed(fallback.into()),
        }
    }

    pub fn render(&self) -> String {
        match self {
            QualityNote::Fixed(s) => s.clone(),
            QualityNote::Scored(wtl) => wtl.render(),
        }
    }
}

impl std::fmt::Display for QualityNote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualityNote::Fixed(s) => f.write_str(s),
            QualityNote::Scored(wtl) => wtl.fmt(f),
        }
    }
}

// ---------------------------------------------------------------------------
// The scorers. Each public entry point is a thin adapter over a private core
// that takes the reference lookup as a closure; the cores hold every semantic
// this file is responsible for and are unit-tested directly, so the scoring
// rules do not depend on how the archive and bounds modules spell their
// lookups.
//
// The three closures below are the ONLY places this file touches a sibling
// module: `archive::arch_key` + `best_length`/`best_makespan` for the IPC-5
// field, `BestKnownBounds::best` for the modern corpora. Best-of-field is the
// archive's own `min(field.values())`, and an absent key and an empty planner
// map are one answer there, exactly as `arch.get(key, {})` then
// `if not field: continue` is one answer in Python.
// ---------------------------------------------------------------------------

/// IPC-5 propositional: our plan length against the archive field's lengths.
///
/// Python guards the whole pass with `if label == "propositional" and arch`.
/// An empty archive needs no guard here: every lookup misses, no ratio is
/// collected, and `Wtl::build` declines -- the same answer by construction.
pub fn length_wtl(rows: &[RawRow], referee: &Referee, arch: &Ipc5Archive) -> Option<Wtl> {
    score_length(rows, referee, |variant, inst| {
        arch.best_length(&arch_key(variant, inst)?)
    })
}

/// IPC-5 time / metric-time: our makespan against the archive field's.
pub fn makespan_wtl(rows: &[RawRow], referee: &Referee, arch: &Ipc5Archive) -> Option<Wtl> {
    score_makespan(rows, referee, |variant, inst| {
        arch.best_makespan(&arch_key(variant, inst)?)
    })
}

/// The modern corpora: our cost against the official best-known bound.
///
/// `year_key` is track-scoped (`"2023-agl"`, `"2023-sat"`, `"2023-opt"`,
/// `"2018"`), never a bare year: those tracks carry DIFFERENT instance sets
/// under the same domain names, so a bare-year key would join a bound from one
/// track onto an instance from another.
pub fn bounds_wtl(
    rows: &[RawRow],
    referee: &Referee,
    bounds: &BestKnownBounds,
    year_key: &str,
    variant_suffix: &str,
) -> Option<Wtl> {
    score_bounds(rows, referee, variant_suffix, |dom, inst| {
        bounds.best(year_key, dom, inst)
    })
}

fn score_length<F>(rows: &[RawRow], referee: &Referee, best_len: F) -> Option<Wtl>
where
    F: Fn(&str, u64) -> Option<u64>,
{
    let (mut w, mut t, mut l) = (0usize, 0usize, 0usize);
    let mut ratios: Vec<f64> = Vec::new();
    for r in rows {
        // The same referee that decides coverage decides what may be scored:
        // a plan VAL rejected is not ours to compare against a field.
        if !referee.is_solved(r) {
            continue;
        }
        let Some(ours) = r.length else {
            continue;
        };
        // A multipart instance label can NEVER meet an integer archive key.
        // Python gets this from `dict.get` missing on a str key; here it is
        // `as_num` returning nothing. Both are the same refusal, and it is a
        // refusal on purpose -- see the module header.
        let Some(inst) = r.instance.as_num() else {
            continue;
        };
        // A variant the join table declines, a key the archive has never
        // heard of, a key no planner in the field solved: any of the three and
        // the row is not scored at all, rather than scored against nothing.
        let Some(best) = best_len(&r.variant, inst) else {
            continue;
        };
        // Capped at 1: being SHORTER than the whole field is a win in W/T/L,
        // not a quality above the field's best.
        //
        // A zero-length plan (goals already true in the initial state) would
        // divide by zero. Python raises ZeroDivisionError and takes the whole
        // regeneration with it, which is why no committed raw can hold one on
        // a scored board: the run that read it would have died. We yield 1.0
        // instead of aborting -- refusing to publish a table is not a better
        // answer than the cap, and it is the cap either way once it renders.
        ratios.push((best as f64 / ours as f64).min(1.0));
        // Three independent counters, exactly as Python's
        // `w += ours < best; t_ += ours == best; l += ours > best` -- and NO
        // tie band. Plan length is an integer count; equal means equal.
        if ours < best {
            w += 1;
        }
        if ours == best {
            t += 1;
        }
        if ours > best {
            l += 1;
        }
    }
    Wtl::build(Currency::Length, w, t, l, &ratios)
}

fn score_makespan<F>(rows: &[RawRow], referee: &Referee, best_ms: F) -> Option<Wtl>
where
    F: Fn(&str, u64) -> Option<f64>,
{
    let (mut w, mut t, mut l) = (0usize, 0usize, 0usize);
    let mut ratios: Vec<f64> = Vec::new();
    for r in rows {
        if !referee.is_solved(r) {
            continue;
        }
        // Python is `not ours or ours <= 0`: `not ours` already rejects a
        // missing column AND a 0.0, and the second test rejects a negative. A
        // row without the 0.22 makespan column is skipped here, which is how a
        // cloud-era raw ends up with no `Wtl` at all instead of a guess.
        let Some(ours) = r.makespan.filter(|m| *m > 0.0) else {
            continue;
        };
        let Some(inst) = r.instance.as_num() else {
            continue;
        };
        let Some(best) = best_ms(&r.variant, inst) else {
            continue;
        };
        // RAW division, uncushioned: the band below bands the verdict, never
        // the quality number.
        ratios.push((best / ours).min(1.0));
        if ours < best - MS_TIE {
            w += 1;
        } else if ours > best + MS_TIE {
            l += 1;
        } else {
            t += 1;
        }
    }
    Wtl::build(Currency::Makespan, w, t, l, &ratios)
}

fn score_bounds<F>(rows: &[RawRow], referee: &Referee, suffix: &str, bound_for: F) -> Option<Wtl>
where
    F: Fn(&str, u64) -> Option<f64>,
{
    let (mut w, mut t, mut l) = (0usize, 0usize, 0usize);
    let mut ratios: Vec<f64> = Vec::new();
    for r in rows {
        if !referee.is_solved(r) {
            continue;
        }
        // Python's `removesuffix` is a no-op when the suffix is absent, and
        // some variants in these corpora already carry the bare domain name.
        let dom = r.variant.strip_suffix(suffix).unwrap_or(&r.variant);
        let bound = r.instance.as_num().and_then(|inst| bound_for(dom, inst));
        // The board's cost currency: the `:metric` where the domain has one,
        // plan length where it does not. `metric: null` falls through to
        // length exactly as `is not None` does.
        let ours = r.metric.or_else(|| r.length.map(|n| n as f64));
        let (Some(bound), Some(ours)) = (bound, ours) else {
            continue;
        };
        // Python's `min(ref/ours, 1.0) if ours else 1.0`. The guard is not
        // decoration: a zero-cost plan is legal on a corpus whose metric is a
        // sum of action costs that can all be zero, and dividing by it would
        // end the run.
        ratios.push(if ours != 0.0 {
            (bound / ours).min(1.0)
        } else {
            1.0
        });
        // Three independent adds again -- a value that is comparable to the
        // bound in none of the three ways is counted in none of them, which is
        // what Python's `w += ours < ref` does and an if/else chain does not.
        if ours < bound {
            w += 1;
        }
        if ours == bound {
            t += 1;
        }
        if ours > bound {
            l += 1;
        }
    }
    Wtl::build(Currency::Bounds, w, t, l, &ratios)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(json: &str) -> RawRow {
        serde_json::from_str(json).expect("fixture row")
    }

    /// The archive stand-in the scorers see: `(variant, instance) ->
    /// best-of-field`, which is `archive::arch_key` plus `best_makespan`
    /// collapsed into one closure. Kept in the two-planner shape of the
    /// fixture in `test_standings.py::MakespanQuality` so the min is a min.
    fn field(
        entries: &'static [(&'static str, u64, &'static [f64])],
    ) -> impl Fn(&str, u64) -> Option<f64> {
        move |variant: &str, inst: u64| {
            entries
                .iter()
                .find(|e| e.0 == variant && e.1 == inst)
                .and_then(|e| e.2.iter().copied().reduce(f64::min))
        }
    }

    /// THE degradation rule, as a type: nothing scored, no verdict. Every
    /// "coverage-only" cell in the published table is downstream of this.
    #[test]
    fn an_empty_ratio_list_has_no_wtl() {
        assert!(Wtl::build(Currency::Makespan, 0, 0, 0, &[]).is_none());
        assert!(Wtl::build(Currency::Length, 0, 0, 0, &[]).is_none());
        assert!(Wtl::build(Currency::Bounds, 0, 0, 0, &[]).is_none());
        // One measured row is enough, and only a measured row is.
        assert!(Wtl::build(Currency::Makespan, 0, 0, 1, &[0.5]).is_some());
    }

    /// `test_standings.py::MakespanQuality::test_scores_only_rows_carrying_makespan`,
    /// ported verbatim: a raw without the 0.22 makespan column must not
    /// acquire a quality number.
    #[test]
    fn scores_only_rows_carrying_makespan() {
        let arch = field(&[("storage-time", 1, &[10.0, 12.0])]);
        let ref_ = Referee::default();

        let pre_022 = vec![row(
            r#"{"variant":"storage-time","instance":1,"solved":true,"val":true}"#,
        )];
        assert!(score_makespan(&pre_022, &ref_, &arch).is_none());

        let scored = vec![row(
            r#"{"variant":"storage-time","instance":1,"solved":true,"val":true,"makespan":20.0}"#,
        )];
        let q = score_makespan(&scored, &ref_, &arch)
            .expect("scored")
            .render();
        assert!(q.contains("0W/0T/1L"), "{q}");
        // best-of-field 10 / ours 20
        assert!(q.contains("0.50"), "{q}");
        assert_eq!(
            q,
            "makespan vs best-of-field: 0W/0T/1L, mean quality 0.50 (1 scored)"
        );
    }

    /// `test_standings.py::MakespanQuality::test_eps_bookkeeping_is_a_tie_not_a_loss`.
    /// Within one epsilon slot of the coarsest granularity in play, the two
    /// plans finish together; booking that as a loss is bookkeeping published
    /// as a result.
    #[test]
    fn eps_bookkeeping_is_a_tie_not_a_loss() {
        let arch = field(&[("storage-time", 1, &[10.0])]);
        let rows = vec![row(
            r#"{"variant":"storage-time","instance":1,"solved":true,"val":true,"makespan":10.01}"#,
        )];
        let wtl = score_makespan(&rows, &Referee::default(), &arch).expect("scored");
        assert!(wtl.render().contains("0W/1T/0L"), "{}", wtl.render());
        // ... while the RATIO stays raw division, uncushioned: 10/10.01 is
        // strictly below 1 even though the verdict is a tie. The band bands
        // the verdict, never the number -- it only READS as 1.00 because two
        // decimals cannot show the difference.
        assert!(wtl.mean() < 1.0, "{}", wtl.mean());
    }

    /// Just outside the band is a real verdict again, on both sides.
    #[test]
    fn outside_the_band_is_a_win_or_a_loss() {
        let arch = field(&[("storage-time", 1, &[10.0])]);
        let win = vec![row(
            r#"{"variant":"storage-time","instance":1,"solved":true,"val":true,"makespan":9.98}"#,
        )];
        let loss = vec![row(
            r#"{"variant":"storage-time","instance":1,"solved":true,"val":true,"makespan":10.02}"#,
        )];
        let r = Referee::default();
        assert!(score_makespan(&win, &r, &arch)
            .unwrap()
            .render()
            .contains("1W/0T/0L"));
        assert!(score_makespan(&loss, &r, &arch)
            .unwrap()
            .render()
            .contains("0W/0T/1L"));
    }

    /// The multipart-label join, held open deliberately: `"3_10_50_10"` is not
    /// instance 3, and no amount of archive data may let it become instance 3.
    #[test]
    fn a_multipart_instance_never_joins_an_integer_archive_key() {
        let arch = field(&[("storage-time", 3, &[10.0])]);
        let rows = vec![row(
            r#"{"variant":"storage-time","instance":"3_10_50_10","solved":true,"val":true,"makespan":20.0}"#,
        )];
        assert!(score_makespan(&rows, &Referee::default(), &arch).is_none());
    }

    /// No key, or a key no planner in the field solved: not a field to be
    /// beaten, and the row is not scored rather than scored against nothing.
    #[test]
    fn an_instance_with_no_field_is_not_scored() {
        let arch = field(&[("storage-time", 7, &[10.0])]);
        let rows = vec![row(
            r#"{"variant":"storage-time","instance":1,"solved":true,"val":true,"makespan":20.0}"#,
        )];
        assert!(score_makespan(&rows, &Referee::default(), &arch).is_none());
    }

    /// The length scorer has NO tie band -- the same 0.01 margin that is a tie
    /// in makespan is meaningless on an integer action count, and one action
    /// more is a loss.
    #[test]
    fn length_compares_exactly_with_no_tie_band() {
        let best = |variant: &str, inst: u64| {
            (variant == "storage-propositional" && inst == 1).then_some(10u64)
        };
        let r = Referee::default();
        let mk = |len: u64| {
            vec![row(&format!(
                r#"{{"variant":"storage-propositional","instance":1,"solved":true,"val":true,"length":{len}}}"#
            ))]
        };
        assert_eq!(
            score_length(&mk(11), &r, best).unwrap().render(),
            "len vs best-of-field: 0W/0T/1L, mean quality 0.91 (1 scored)"
        );
        assert!(score_length(&mk(10), &r, best)
            .unwrap()
            .render()
            .contains("0W/1T/0L"));
        assert!(score_length(&mk(9), &r, best)
            .unwrap()
            .render()
            .contains("1W/0T/0L"));
        // Capped: beating the field's best is a win, never a quality > 1.
        assert_eq!(score_length(&mk(9), &r, best).unwrap().mean(), 1.0);
    }

    /// A row with no `length` column carries no length currency, whatever the
    /// archive knows -- the same degradation as the missing makespan column.
    #[test]
    fn a_row_without_a_length_is_not_scored() {
        let best = |_: &str, _: u64| Some(10u64);
        let rows = vec![row(
            r#"{"variant":"storage-propositional","instance":1,"solved":true,"val":true}"#,
        )];
        assert!(score_length(&rows, &Referee::default(), best).is_none());
    }

    /// Quality answers to the same referee as coverage: a plan VAL rejected is
    /// not a plan to be compared against a field.
    #[test]
    fn a_val_rejected_row_is_not_scored() {
        let arch = field(&[("storage-time", 1, &[10.0])]);
        let rows = vec![row(
            r#"{"variant":"storage-time","instance":1,"solved":true,"val":false,"makespan":20.0}"#,
        )];
        assert!(score_makespan(&rows, &Referee::default(), &arch).is_none());
    }

    /// The `if ours` guard in `bounds_quality`: a zero-cost plan yields 1.0
    /// rather than a division by zero, and still counts as a win on W/T/L.
    #[test]
    fn a_zero_cost_plan_takes_the_ratio_guard() {
        let bounds = |dom: &str, inst: u64| (dom == "storage" && inst == 1).then_some(5.0);
        let rows = vec![row(
            r#"{"variant":"storage-agile","instance":1,"solved":true,"val":true,"length":0}"#,
        )];
        assert_eq!(
            score_bounds(&rows, &Referee::default(), "-agile", bounds)
                .unwrap()
                .render(),
            "vs best-known bounds: 1W/0T/0L, mean quality 1.00 (1 scored)"
        );
    }

    /// `metric` wins over `length` where the domain has one, and a null
    /// `metric` falls through to length rather than skipping the row.
    #[test]
    fn metric_leads_length_as_the_bounds_currency() {
        let bounds = |_: &str, _: u64| Some(10.0);
        let r = Referee::default();
        let with_metric = vec![row(
            r#"{"variant":"storage-agile","instance":1,"solved":true,"val":true,"metric":20.0,"length":5}"#,
        )];
        // 10/20 = 0.50 off the metric; off the length it would have been 1.00.
        assert!(score_bounds(&with_metric, &r, "-agile", bounds)
            .unwrap()
            .render()
            .contains("0.50"));
        let null_metric = vec![row(
            r#"{"variant":"storage-agile","instance":1,"solved":true,"val":true,"metric":null,"length":20}"#,
        )];
        assert!(score_bounds(&null_metric, &r, "-agile", bounds)
            .unwrap()
            .render()
            .contains("0.50"));
    }

    /// `removesuffix` is a no-op when the suffix is absent -- the domain name
    /// must survive a variant that never carried the track suffix.
    #[test]
    fn an_absent_suffix_leaves_the_domain_alone() {
        let bounds = |dom: &str, _: u64| (dom == "storage").then_some(10.0);
        let rows = vec![row(
            r#"{"variant":"storage","instance":1,"solved":true,"val":true,"length":10}"#,
        )];
        assert!(score_bounds(&rows, &Referee::default(), "-agile", bounds)
            .unwrap()
            .render()
            .contains("0W/1T/0L"));
    }

    /// The mean renders half-to-EVEN, like Python's `{:.2f}`. A naive
    /// `(x * 100).round() / 100` reads 0.13 here and publishes it.
    #[test]
    fn the_mean_renders_half_to_even() {
        let q = Wtl::build(Currency::Bounds, 0, 0, 1, &[0.125]).unwrap();
        assert_eq!(
            q.render(),
            "vs best-known bounds: 0W/0T/1L, mean quality 0.12 (1 scored)"
        );
        let q = Wtl::build(Currency::Bounds, 0, 0, 1, &[0.375]).unwrap();
        assert!(q.render().contains("0.38"));
    }

    /// The fallback path the degradation rule hands off to.
    #[test]
    fn a_note_falls_back_to_the_manifest_string() {
        let note = QualityNote::new(
            None,
            "coverage-only (raw predates the 0.22 makespan column)",
        );
        assert_eq!(
            note.render(),
            "coverage-only (raw predates the 0.22 makespan column)"
        );
        let scored = Wtl::build(Currency::Makespan, 1, 0, 0, &[1.0]).unwrap();
        let note = QualityNote::new(Some(scored), "coverage-only");
        assert!(note
            .render()
            .starts_with("makespan vs best-of-field: 1W/0T/0L"));
    }
}
