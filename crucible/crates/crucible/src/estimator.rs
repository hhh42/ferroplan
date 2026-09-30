//! What a board comparison would say if N were controlled (0.29 Phase 0.2).
//!
//! A published board row is the attempt the crucible BANKED, and the crucible
//! re-runs an instance when the referee owes it. That is right for a false
//! timeout and wrong for a comparison: two engines with different attempt
//! counts are not scored on the same instrument, and banking the max for both
//! turns run-to-run variance into coverage. 0.27's +115 decomposed into
//! first-attempt −18 and a rescue difference of +133; 0.28's +344 into
//! first-attempt +531 and rescues −185. Neither banked number was the engine.
//!
//! This is the port of `benchmarks/attempts-estimator.py`, which lived beside
//! the harness for two cuts and was run by hand at each. `crucible compare`
//! prints it now, over every attempt of every cell both engines measured:
//!
//! * `banked`     what the boards publish (the banked attempt)
//! * `any`        solved on ANY attempt (max over N, N uncontrolled)
//! * `first`      solved on the FIRST attempt (N = 1 for both)
//! * `equal-N`    max over the first min(N_A, N_B) attempts, per cell
//! * `per-run`    solved runs / runs per cell -- the expected coverage of ONE run
//! * `clean-only` per-run over runs the harness marked `clean`, where both have one
//!
//! No estimator here is the "true" one. The point is the SPREAD: when the
//! delta changes sign across them, the instrument's variance exceeds the
//! effect and no number has been measured yet. The cut rule (docs/roadmap-0.29.md)
//! publishes `first` as the comparative claim, `equal-N` beside it, and
//! `banked` only as the boards' standing number.

use crucible_core::db::CellAttempt;
use std::collections::BTreeMap;

/// One attempt, reduced to what the estimators read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attempt {
    pub solved: bool,
    pub banked: bool,
    pub clean: bool,
}

/// A cell's attempts under one engine, oldest first. Never empty: a cell
/// with no attempt is not measured and is not a cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell(pub Vec<Attempt>);

impl Cell {
    fn banked_solved(&self) -> bool {
        self.0.iter().find(|a| a.banked).is_some_and(|a| a.solved)
    }
    fn any(&self) -> bool {
        self.0.iter().any(|a| a.solved)
    }
    fn first(&self) -> bool {
        self.0[0].solved
    }
    fn any_of_first(&self, k: usize) -> bool {
        self.0.iter().take(k).any(|a| a.solved)
    }
    fn per_run(&self) -> f64 {
        self.0.iter().filter(|a| a.solved).count() as f64 / self.0.len() as f64
    }
    fn clean_per_run(&self) -> Option<f64> {
        let c: Vec<&Attempt> = self.0.iter().filter(|a| a.clean).collect();
        (!c.is_empty()).then(|| c.iter().filter(|a| a.solved).count() as f64 / c.len() as f64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Estimator {
    Banked,
    Any,
    First,
    EqualN,
    PerRun,
    CleanOnly,
}

impl Estimator {
    pub const ALL: [Estimator; 6] = [
        Estimator::Banked,
        Estimator::Any,
        Estimator::First,
        Estimator::EqualN,
        Estimator::PerRun,
        Estimator::CleanOnly,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Estimator::Banked => "banked",
            Estimator::Any => "any",
            Estimator::First => "first",
            Estimator::EqualN => "equal-N",
            Estimator::PerRun => "per-run",
            Estimator::CleanOnly => "clean-only",
        }
    }
}

/// The two engines' scores over paired cells under one estimator. The pairs
/// are (A's cell, B's cell) for the same instance; the caller pairs them.
pub fn estimate(pairs: &[(&Cell, &Cell)], mode: Estimator) -> (f64, f64) {
    let mut out = (0.0, 0.0);
    for (a, b) in pairs {
        match mode {
            Estimator::Banked => {
                out.0 += a.banked_solved() as u8 as f64;
                out.1 += b.banked_solved() as u8 as f64;
            }
            Estimator::Any => {
                out.0 += a.any() as u8 as f64;
                out.1 += b.any() as u8 as f64;
            }
            Estimator::First => {
                out.0 += a.first() as u8 as f64;
                out.1 += b.first() as u8 as f64;
            }
            Estimator::EqualN => {
                let k = a.0.len().min(b.0.len());
                out.0 += a.any_of_first(k) as u8 as f64;
                out.1 += b.any_of_first(k) as u8 as f64;
            }
            Estimator::PerRun => {
                out.0 += a.per_run();
                out.1 += b.per_run();
            }
            Estimator::CleanOnly => {
                if let (Some(x), Some(y)) = (a.clean_per_run(), b.clean_per_run()) {
                    out.0 += x;
                    out.1 += y;
                }
            }
        }
    }
    out
}

/// xorshift64*: a small, seeded, dependency-free generator. The bootstrap
/// wants reproducibility from one run to the next, not cryptography; the
/// Python used `random.Random(20260919)`, so the two ports agree on the
/// point estimates exactly and on the intervals to the width of the noise.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.max(1))
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Resample cells with replacement; the 2.5th and 97.5th percentiles of
/// B − A. `n` resamples; the Python's index arithmetic, verbatim.
pub fn bootstrap(pairs: &[(&Cell, &Cell)], mode: Estimator, n: usize, seed: u64) -> (f64, f64) {
    let mut rng = Rng::new(seed);
    let mut deltas = Vec::with_capacity(n);
    let mut sample: Vec<(&Cell, &Cell)> = Vec::with_capacity(pairs.len());
    for _ in 0..n {
        sample.clear();
        for _ in 0..pairs.len() {
            sample.push(pairs[rng.below(pairs.len())]);
        }
        let (a, b) = estimate(&sample, mode);
        deltas.push(b - a);
    }
    deltas.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let lo = deltas[(0.025 * n as f64) as usize];
    let hi = deltas[((0.975 * n as f64) as usize).saturating_sub(1).min(n - 1)];
    (lo, hi)
}

/// The paired cells of one board comparison, keyed by instance, from each
/// side's attempt list. A cell counts only when BOTH engines measured it;
/// `keep` restricts to the (variant, label) keys a selection admits.
pub fn pair_cells(
    a: &[CellAttempt],
    b: &[CellAttempt],
    keep: Option<&std::collections::HashSet<(String, String)>>,
) -> Vec<(Cell, Cell)> {
    fn group(rows: &[CellAttempt]) -> BTreeMap<i64, (String, String, Cell)> {
        let mut m: BTreeMap<i64, (String, String, Cell)> = BTreeMap::new();
        for r in rows {
            m.entry(r.instance_id)
                .or_insert_with(|| (r.variant.clone(), r.label.clone(), Cell(Vec::new())))
                .2
                 .0
                .push(Attempt {
                    solved: r.solved,
                    banked: r.banked,
                    clean: r.timing == "clean",
                });
        }
        m
    }
    let (ga, gb) = (group(a), group(b));
    ga.into_iter()
        .filter_map(|(id, (v, l, ca))| {
            let (_, _, cb) = gb.get(&id)?;
            if keep.is_some_and(|k| !k.contains(&(v, l))) {
                return None;
            }
            Some((ca, cb.clone()))
        })
        .collect()
}

/// Per side: first-attempt solves, first-attempt failures, failures re-run,
/// rescued (a first-attempt failure solved on a later attempt).
pub type Trigger = (usize, usize, usize, usize);

/// One estimator's line: A's score, B's score, the bootstrap interval on B − A.
pub struct EstRow {
    pub mode: Estimator,
    pub a: f64,
    pub b: f64,
    pub ci: Option<(f64, f64)>,
}

/// What `crucible compare` prints below the board table.
pub struct Report {
    pub cells: usize,
    pub runs: (usize, usize),
    pub max_n: (usize, usize),
    pub clean_pct: (f64, f64),
    pub trigger: (Trigger, Trigger),
    pub rows: Vec<EstRow>,
}

impl Report {
    pub fn build(pairs: &[(Cell, Cell)], bootstrap_n: usize) -> Report {
        let refs: Vec<(&Cell, &Cell)> = pairs.iter().map(|(a, b)| (a, b)).collect();
        let side = |pick: fn(&(Cell, Cell)) -> &Cell| {
            let cells: Vec<&Cell> = pairs.iter().map(pick).collect();
            let runs: usize = cells.iter().map(|c| c.0.len()).sum();
            let max_n = cells.iter().map(|c| c.0.len()).max().unwrap_or(0);
            let clean = cells
                .iter()
                .flat_map(|c| c.0.iter())
                .filter(|a| a.clean)
                .count();
            let clean_pct = 100.0 * clean as f64 / runs.max(1) as f64;
            let s1 = cells.iter().filter(|c| c.first()).count();
            let f1: Vec<&&Cell> = cells.iter().filter(|c| !c.first()).collect();
            let rerun_f = f1.iter().filter(|c| c.0.len() > 1).count();
            let rescued = f1.iter().filter(|c| c.any()).count();
            (runs, max_n, clean_pct, (s1, f1.len(), rerun_f, rescued))
        };
        let a = side(|p| &p.0);
        let b = side(|p| &p.1);
        let rows = Estimator::ALL
            .iter()
            .map(|&m| {
                let (x, y) = estimate(&refs, m);
                let ci = (bootstrap_n > 0 && !refs.is_empty())
                    .then(|| bootstrap(&refs, m, bootstrap_n, 20_260_919));
                EstRow {
                    mode: m,
                    a: x,
                    b: y,
                    ci,
                }
            })
            .collect();
        Report {
            cells: pairs.len(),
            runs: (a.0, b.0),
            max_n: (a.1, b.1),
            clean_pct: (a.2, b.2),
            trigger: (a.3, b.3),
            rows,
        }
    }

    pub fn render(&self) -> String {
        let mut s = String::new();
        let n = self.cells.max(1) as f64;
        s.push_str(&format!(
            "{} cells measured by BOTH engines, every attempt counted\n",
            self.cells
        ));
        for (name, runs, max_n, clean) in [
            ("A", self.runs.0, self.max_n.0, self.clean_pct.0),
            ("B", self.runs.1, self.max_n.1, self.clean_pct.1),
        ] {
            s.push_str(&format!(
                "  {name}: {runs} runs, mean N={:.2}, max N={max_n}, {clean:.0}% clean\n",
                runs as f64 / n
            ));
        }
        s.push_str("\n  what triggers a re-run (a re-run is fired by FAILURE, so extra attempts can only add solves):\n");
        s.push_str(&format!(
            "  {:<8}{:<30}{:<30}{}\n",
            "engine", "attempt 1 SOLVED", "attempt 1 FAILED", "rescued"
        ));
        for (name, (s1, f1, rf, resc)) in [("A", self.trigger.0), ("B", self.trigger.1)] {
            s.push_str(&format!(
                "  {name:<8}{:<30}{:<30}{resc}\n",
                format!("{s1} cells"),
                format!(
                    "{f1} cells, re-run {:.1}%",
                    100.0 * rf as f64 / f1.max(1) as f64
                )
            ));
        }
        let ((fa, _, _, ra), (fb, _, _, rb)) = self.trigger;
        let (df, dr) = (fb as i64 - fa as i64, rb as i64 - ra as i64);
        s.push_str(&format!(
            "  decomposition of `any`: first-attempt {df:+}  +  rescues {dr:+}  =  {:+}\n",
            df + dr
        ));
        s.push_str(&format!(
            "\n  {:<12}{:>9}{:>9}{:>9}    {}\n",
            "estimator", "A", "B", "B - A", "95% CI (cell bootstrap)"
        ));
        for r in &self.rows {
            let ci =
                r.ci.map(|(lo, hi)| format!("[{lo:+.0}, {hi:+.0}]"))
                    .unwrap_or_default();
            s.push_str(&format!(
                "  {:<12}{:>9.1}{:>9.1}{:>+9.1}    {ci}\n",
                r.mode.label(),
                r.a,
                r.b,
                r.b - r.a
            ));
        }
        let delta = |m: Estimator| self.rows.iter().find(|r| r.mode == m).map(|r| r.b - r.a);
        let (first, banked, equal) = (
            delta(Estimator::First),
            delta(Estimator::Banked),
            delta(Estimator::EqualN),
        );
        if let (Some(f), Some(bk), Some(eq)) = (first, banked, equal) {
            let same_sign = (f >= 0.0) == (bk >= 0.0) && (f >= 0.0) == (eq >= 0.0);
            s.push_str(if same_sign {
                "\n  the estimators agree in sign: `first` is the comparative claim, `banked` the boards' standing number\n"
            } else {
                "\n  THE ESTIMATORS DISAGREE IN SIGN: the instrument's variance exceeds the effect; no delta is claimed\n"
            });
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(runs: &[(bool, bool, bool)]) -> Cell {
        Cell(
            runs.iter()
                .map(|&(solved, banked, clean)| Attempt {
                    solved,
                    banked,
                    clean,
                })
                .collect(),
        )
    }

    /// The whole reason the module exists, on one cell: a failure re-run
    /// into a solve counts under `banked` and `any`, not under `first`, and
    /// half under `per-run`; against a one-shot solve on the other side,
    /// `equal-N` reads the first attempt only.
    #[test]
    fn a_rescue_counts_differently_under_each_estimator() {
        let rescued = cell(&[(false, false, true), (true, true, true)]);
        let one_shot = cell(&[(true, true, true)]);
        let pairs = [(&rescued, &one_shot)];
        assert_eq!(estimate(&pairs, Estimator::Banked), (1.0, 1.0));
        assert_eq!(estimate(&pairs, Estimator::Any), (1.0, 1.0));
        assert_eq!(estimate(&pairs, Estimator::First), (0.0, 1.0));
        assert_eq!(estimate(&pairs, Estimator::EqualN), (0.0, 1.0));
        assert_eq!(estimate(&pairs, Estimator::PerRun), (0.5, 1.0));
        assert_eq!(estimate(&pairs, Estimator::CleanOnly), (0.5, 1.0));
    }

    /// `clean-only` scores a cell only where BOTH sides have a clean run.
    #[test]
    fn clean_only_needs_a_clean_run_on_both_sides() {
        let a = cell(&[(true, true, false)]);
        let b = cell(&[(true, true, true)]);
        assert_eq!(estimate(&[(&a, &b)], Estimator::CleanOnly), (0.0, 0.0));
    }

    /// The decomposition identity the report prints: `any` = first-attempt
    /// solves + rescues, on each side.
    #[test]
    fn any_is_first_plus_rescues() {
        let cells_a = [
            cell(&[(true, true, true)]),
            cell(&[(false, false, true), (true, true, true)]),
            cell(&[(false, false, true), (false, true, true)]),
        ];
        let cells_b = [
            cell(&[
                (false, false, true),
                (false, false, true),
                (true, true, true),
            ]),
            cell(&[(true, true, true)]),
            cell(&[(false, true, true)]),
        ];
        let pairs: Vec<(Cell, Cell)> = cells_a
            .iter()
            .cloned()
            .zip(cells_b.iter().cloned())
            .collect();
        let r = Report::build(&pairs, 0);
        let ((s1a, _, _, ra), (s1b, _, _, rb)) = r.trigger;
        let refs: Vec<(&Cell, &Cell)> = pairs.iter().map(|(a, b)| (a, b)).collect();
        let (anya, anyb) = estimate(&refs, Estimator::Any);
        assert_eq!((s1a + ra) as f64, anya);
        assert_eq!((s1b + rb) as f64, anyb);
        assert!(r
            .render()
            .contains("decomposition of `any`: first-attempt +0  +  rescues +0  =  +0"));
    }

    /// The bootstrap is seeded: the same pairs give the same interval twice,
    /// and the interval brackets the point estimate.
    #[test]
    fn the_bootstrap_is_reproducible_and_brackets_the_estimate() {
        let mut pairs = Vec::new();
        for i in 0..200 {
            pairs.push((
                cell(&[(i % 3 == 0, true, true)]),
                cell(&[(i % 2 == 0, true, true)]),
            ));
        }
        let refs: Vec<(&Cell, &Cell)> = pairs.iter().map(|(a, b)| (a, b)).collect();
        let (a, b) = estimate(&refs, Estimator::First);
        let ci1 = bootstrap(&refs, Estimator::First, 500, 7);
        let ci2 = bootstrap(&refs, Estimator::First, 500, 7);
        assert_eq!(ci1, ci2);
        assert!(ci1.0 <= b - a && b - a <= ci1.1, "{ci1:?} around {}", b - a);
    }

    /// Pairing keys on the instance, keeps only cells both engines measured,
    /// and honours a selection.
    #[test]
    fn pairing_keeps_only_cells_both_engines_measured() {
        let row = |id: i64, label: &str, attempt: u32, solved: bool| CellAttempt {
            instance_id: id,
            variant: "v".into(),
            label: label.into(),
            attempt,
            solved,
            banked: true,
            timing: "clean".into(),
        };
        let a = vec![
            row(1, "1", 1, true),
            row(2, "2", 1, false),
            row(2, "2", 2, true),
        ];
        let b = vec![row(2, "2", 1, true), row(3, "3", 1, true)];
        let pairs = pair_cells(&a, &b, None);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0 .0.len(), 2, "A's two attempts on instance 2");
        let keep: std::collections::HashSet<(String, String)> =
            [("v".to_string(), "9".to_string())].into_iter().collect();
        assert!(pair_cells(&a, &b, Some(&keep)).is_empty());
    }
}
