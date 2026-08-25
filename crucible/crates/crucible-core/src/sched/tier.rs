//! What history says an instance will do, and what the scheduler is allowed to
//! do about it.
//!
//! # A CORRECTION TO THE SPEC, and it is the whole point of this file
//!
//! `crucible-spec.md` §5.1 gives Tier A the policy "pack densely -- many
//! concurrent, timing not precious", on the stated rationale that "coverage is
//! the metric that matters; absolute timing matters only at the frontier".
//! That rationale is right and the conclusion drawn from it is exactly
//! backwards for THIS repo, because here the metric is COVERAGE AT A 60-SECOND
//! WALL. Timing is not a separate quantity that can be sacrificed to buy
//! throughput: an instance slowed by a neighbour crosses the wall it would
//! otherwise have beaten, and the board loses a solved row. Packing densely
//! does not merely risk contention, it IS contention -- the self-inflicted
//! kind, which `monitor::sample` cannot even see, because our own tree is
//! excluded from the competitor tally by design.
//!
//! The measured record on this box is unambiguous. Four parallel `cargo test
//! --all --release` jobs took `elevator-2011 i10` from 22 s to 122 s; the 0.21
//! hygiene comment in every sweep driver concludes "a board measured under
//! that is not a slow board, it is a WRONG board", and the v0.18 backfill lost
//! one to exactly this. Concurrency we choose is not different in kind from
//! concurrency somebody else's browser chose.
//!
//! So tiering survives here in two forms and no others:
//!
//! 1. **Ordering WITHIN a board.** Known-fast instances first. This banks the
//!    most coverage soonest, which matters because a board is interrupted by
//!    contention, by an operator, or by the box being wanted -- the shell
//!    driver's own "a driver that dies early banks something", moved down from
//!    board granularity to instance granularity where per-instance resume can
//!    actually use it. It also bounds the waste at a contention window's edges:
//!    a window that clips a Tier A instance costs a two-second re-run, while
//!    one that clips a Tier D instance costs a full budget.
//! 2. **The ETA estimator.** Callers want to know whether a board lands before
//!    breakfast. That is advice, and advice cannot corrupt a measurement.
//!
//! **Tiering must NEVER raise `jobs`.** `jobs` is declared by the board, is
//! stamped into every row, is part of the identity the resume gate compares
//! EXACTLY, and is capped at 2 on this box for measured reasons -- with the mco
//! boards pinned to 1 by the competition's wall-clock rule. A scheduler that
//! raised it for a stretch of Tier A work would produce rows that no longer
//! join to the board they belong to, and would do it invisibly. There is
//! therefore no concurrency knob in this module: [`order`] returns a
//! PERMUTATION and nothing else, and `jobs` reaches the runner from the
//! manifest, through `sched::budget`, never from here.

use super::resume::RowKey;
use std::collections::BTreeMap;
use std::time::Duration;

/// `[scheduler] tier_a_max_secs` in `crucible-spec.md` §12 (Configuration).
pub const TIER_A_MAX_SECS: f64 = 2.0;
/// `[scheduler] tier_c_min_secs`.
pub const TIER_C_MIN_SECS: f64 = 30.0;

/// The `tier` column: `A|B|C|D`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// Solved last time, well inside the wall.
    A,
    B,
    /// Solved, but close enough to the wall that a neighbour could cost it.
    C,
    /// Previously unsolved, or never run at all. Expect the whole budget.
    D,
}

impl Tier {
    pub fn label(self) -> &'static str {
        match self {
            Tier::A => "A",
            Tier::B => "B",
            Tier::C => "C",
            Tier::D => "D",
        }
    }
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    pub a_max_secs: f64,
    pub c_min_secs: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds {
            a_max_secs: TIER_A_MAX_SECS,
            c_min_secs: TIER_C_MIN_SECS,
        }
    }
}

/// What a previous tag measured for one instance under the same config.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prior {
    /// An UNSOLVED prior is Tier D whatever its runtime says. A row that spent
    /// 0.4 s and returned "no plan" tells you nothing about how long the next
    /// build will search before it finds one.
    pub solved: bool,
    /// Median seconds over the runs on record. `None` on a row from before the
    /// runner recorded a time.
    pub p50_secs: Option<f64>,
}

/// History, supplied by the caller.
///
/// A trait rather than a concrete query so this module stays testable without a
/// database -- and so the tiering rule can be exercised against a hand-built
/// map that states exactly the case under test.
pub trait History {
    fn prior(&self, key: &RowKey) -> Option<Prior>;
}

impl History for BTreeMap<RowKey, Prior> {
    fn prior(&self, key: &RowKey) -> Option<Prior> {
        self.get(key).copied()
    }
}

/// History that knows nothing: every instance is Tier D. What a new board, a
/// new corpus, or a first run on a new box actually looks like.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoHistory;

impl History for NoHistory {
    fn prior(&self, _key: &RowKey) -> Option<Prior> {
        None
    }
}

/// The tier rule. Unsolved and unknown are the same answer, because they lead
/// to the same expectation: this one will probably spend the whole wall.
pub fn classify(prior: Option<Prior>, t: &Thresholds) -> Tier {
    match prior {
        Some(Prior {
            solved: true,
            p50_secs: Some(p),
        }) => {
            if p < t.a_max_secs {
                Tier::A
            } else if p < t.c_min_secs {
                Tier::B
            } else {
                Tier::C
            }
        }
        // Solved but untimed: it finished inside the wall, and that is all we
        // know. B rather than D -- claiming a full budget for it would inflate
        // every ETA on a pre-0.20 board -- but never A, which is a claim.
        Some(Prior {
            solved: true,
            p50_secs: None,
        }) => Tier::B,
        _ => Tier::D,
    }
}

/// One instance, tiered, with what the estimator expects it to cost.
#[derive(Debug, Clone, PartialEq)]
pub struct Scheduled {
    pub key: RowKey,
    pub tier: Tier,
    /// Seconds. The prior median, or the full budget for Tier D.
    pub estimate_secs: f64,
}

/// Order a board's instances: fastest known work first, unknown work last.
///
/// A PERMUTATION of the input and nothing more -- no grouping, no batching, no
/// concurrency advice. Ties keep the caller's order, which is the corpus's own
/// -- `ipc67.py:instances` sorts the filenames by their digit groups as a tuple
/// of integers (`sorted(named, key=lambda f: tuple(int(g) for g in groups(f)))`),
/// NOT lexically, so instance 10 follows instance 9 -- and two runs of the same
/// board with the same history therefore schedule identically.
pub fn order(
    instances: &[RowKey],
    h: &dyn History,
    t: &Thresholds,
    budget_secs: f64,
) -> Vec<Scheduled> {
    let mut out: Vec<Scheduled> = instances
        .iter()
        .map(|key| {
            let prior = h.prior(key);
            let tier = classify(prior, t);
            let estimate_secs = match (tier, prior) {
                (Tier::D, _) => budget_secs,
                (
                    _,
                    Some(Prior {
                        p50_secs: Some(p), ..
                    }),
                ) => p,
                // Solved but untimed: assume it used the wall it was allowed
                // rather than the wall it might have wanted. Half the budget is
                // a guess, and a guess in an ETA is honest; a guess in a
                // measurement would not be.
                _ => budget_secs / 2.0,
            };
            Scheduled {
                key: key.clone(),
                tier,
                estimate_secs,
            }
        })
        .collect();
    // `sort_by` is stable, so equal elements keep the corpus order.
    out.sort_by(|a, b| {
        a.tier.cmp(&b.tier).then_with(|| {
            a.estimate_secs
                .partial_cmp(&b.estimate_secs)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });
    out
}

/// How long the board should take, in wall seconds.
///
/// Advice for an operator and for the quiet-hours steering in
/// `crucible-spec.md` §5.3 -- never an input to anything measured. `jobs` is
/// the board's declared concurrency, read here and NEVER written.
///
/// The sum accumulates in schedule order rather than in any convenient one:
/// float addition is not associative, and an ETA that changes when the list is
/// reordered is an ETA nobody can reproduce.
pub fn eta(plan: &[Scheduled], jobs: u32) -> Duration {
    let mut total = 0.0f64;
    for s in plan {
        total += s.estimate_secs;
    }
    let wall = total / f64::from(jobs.max(1));
    Duration::from_secs_f64(wall.max(0.0))
}

/// How many of each tier, for the operator line before a board starts.
pub fn census(plan: &[Scheduled]) -> BTreeMap<Tier, usize> {
    let mut m = BTreeMap::new();
    for s in plan {
        *m.entry(s.tier).or_insert(0) += 1;
    }
    m
}

#[cfg(test)]
mod tests {
    use super::super::resume::InstanceKey;
    use super::*;

    fn key(n: u64) -> RowKey {
        RowKey {
            ipc: Some("ipc2023".to_string()),
            variant: "numeric-2023".to_string(),
            instance: InstanceKey::Num(n),
        }
    }

    fn solved(p50: f64) -> Prior {
        Prior {
            solved: true,
            p50_secs: Some(p50),
        }
    }

    /// The boundaries, stated as the spec states them: A is `< 2`, B is
    /// `[2, 30)`, C is `>= 30`.
    #[test]
    fn the_tier_boundaries_are_the_spec_configuration() {
        let t = Thresholds::default();
        assert_eq!(classify(Some(solved(1.999)), &t), Tier::A);
        assert_eq!(classify(Some(solved(2.0)), &t), Tier::B);
        assert_eq!(classify(Some(solved(29.999)), &t), Tier::B);
        assert_eq!(classify(Some(solved(30.0)), &t), Tier::C);
    }

    /// Unsolved and never-run are the same answer, because they lead to the
    /// same expectation. A row that spent 0.4 s and returned "no plan" says
    /// nothing about how long the NEXT build will search before it finds one.
    #[test]
    fn unsolved_and_unknown_are_both_tier_d() {
        let t = Thresholds::default();
        assert_eq!(classify(None, &t), Tier::D);
        assert_eq!(
            classify(
                Some(Prior {
                    solved: false,
                    p50_secs: Some(0.4)
                }),
                &t
            ),
            Tier::D,
            "a fast REJECT is not a fast solve"
        );
    }

    /// A pre-0.20 row solved without recording a time. Calling it D would
    /// inflate every ETA on an old board; calling it A would be a claim.
    #[test]
    fn solved_but_untimed_is_b() {
        assert_eq!(
            classify(
                Some(Prior {
                    solved: true,
                    p50_secs: None
                }),
                &Thresholds::default()
            ),
            Tier::B
        );
    }

    /// THE RULE THIS MODULE EXISTS TO ENFORCE: ordering is a permutation. No
    /// instance is added, dropped or duplicated, and nothing here can touch
    /// `jobs` -- there is no concurrency knob to touch.
    #[test]
    fn ordering_is_a_permutation_and_nothing_more() {
        let keys: Vec<RowKey> = (0..6).map(key).collect();
        let mut h = BTreeMap::new();
        h.insert(key(0), solved(45.0));
        h.insert(key(1), solved(0.5));
        h.insert(key(3), solved(12.0));
        h.insert(
            key(4),
            Prior {
                solved: false,
                p50_secs: Some(60.0),
            },
        );
        let plan = order(&keys, &h, &Thresholds::default(), 60.0);
        assert_eq!(plan.len(), keys.len());
        let mut got: Vec<RowKey> = plan.iter().map(|s| s.key.clone()).collect();
        got.sort();
        let mut want = keys.clone();
        want.sort();
        assert_eq!(got, want, "every instance, exactly once");
    }

    /// Known-fast first, unknown last -- and ties keep the corpus order, so two
    /// runs of the same board with the same history schedule identically.
    #[test]
    fn fast_work_runs_first_and_ties_keep_corpus_order() {
        let keys: Vec<RowKey> = (0..5).map(key).collect();
        let mut h = BTreeMap::new();
        h.insert(key(0), solved(45.0)); // C
        h.insert(key(1), solved(0.5)); // A
        h.insert(key(2), solved(0.5)); // A, same estimate as 1
        h.insert(key(3), solved(12.0)); // B
                                        // key(4): unknown -> D
        let plan = order(&keys, &h, &Thresholds::default(), 60.0);
        let tiers: Vec<Tier> = plan.iter().map(|s| s.tier).collect();
        assert_eq!(tiers, vec![Tier::A, Tier::A, Tier::B, Tier::C, Tier::D]);
        assert_eq!(
            (plan[0].key.clone(), plan[1].key.clone()),
            (key(1), key(2)),
            "equal estimates keep the order the corpus listing gave them"
        );
    }

    /// Tier D expects the whole wall, which is the only honest estimate for an
    /// instance nothing has ever solved.
    #[test]
    fn tier_d_is_estimated_at_the_full_budget() {
        let plan = order(&[key(0)], &NoHistory, &Thresholds::default(), 300.0);
        assert_eq!(plan[0].tier, Tier::D);
        assert_eq!(plan[0].estimate_secs, 300.0);
    }

    /// The ETA divides by the board's DECLARED jobs. It reads the number; it
    /// has no way to change it.
    #[test]
    fn the_eta_divides_by_declared_jobs_and_never_sets_them() {
        let keys: Vec<RowKey> = (0..4).map(key).collect();
        let plan = order(&keys, &NoHistory, &Thresholds::default(), 60.0);
        assert_eq!(eta(&plan, 1), Duration::from_secs(240));
        assert_eq!(eta(&plan, 2), Duration::from_secs(120));
        // The mco wall-clock rule: threads-heavy boards run one at a time, and
        // their ETA is the honest, long one.
        assert_eq!(eta(&plan, 0), Duration::from_secs(240), "jobs 0 is jobs 1");
    }

    /// An ETA that changes when the list is reordered is an ETA nobody can
    /// reproduce, so the sum accumulates in schedule order.
    #[test]
    fn the_eta_is_summed_in_schedule_order() {
        let keys: Vec<RowKey> = (0..3).map(key).collect();
        let mut h = BTreeMap::new();
        h.insert(key(0), solved(0.1));
        h.insert(key(1), solved(0.2));
        h.insert(key(2), solved(0.3));
        let plan = order(&keys, &h, &Thresholds::default(), 60.0);
        let mut expect = 0.0f64;
        for s in &plan {
            expect += s.estimate_secs;
        }
        assert_eq!(eta(&plan, 1), Duration::from_secs_f64(expect));
    }

    /// The operator line before a board starts: how much of this is frontier
    /// work.
    #[test]
    fn the_census_counts_each_tier() {
        let keys: Vec<RowKey> = (0..3).map(key).collect();
        let mut h = BTreeMap::new();
        h.insert(key(0), solved(0.5));
        h.insert(key(1), solved(0.5));
        let plan = order(&keys, &h, &Thresholds::default(), 60.0);
        let c = census(&plan);
        assert_eq!(c.get(&Tier::A), Some(&2));
        assert_eq!(c.get(&Tier::D), Some(&1));
        assert_eq!(c.get(&Tier::B), None);
    }
}
