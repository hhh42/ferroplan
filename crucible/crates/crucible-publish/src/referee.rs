//! Who decides what a row means. Ported from `benchmarks/standings.py`
//! (`solved` :236, `classify` :243, `coverage_line` :403).
//!
//! These sixty lines are the load-bearing part of the whole port. Every
//! boundary in them was moved at least once, for a reason recorded in the
//! Python's own comments, and `crucible/tests/` carries one test per reason.

use crate::class::{Class, Coverage};
use crate::raw::RawRow;
use std::collections::{BTreeMap, BTreeSet};

/// The timeout line: 90% of the row's budget, not 95%.
///
/// The refill loop refuses a new round below 10% of an armed wall, so a
/// graceful exit anywhere in [90%, 100%] is a spent wall by construction. At 95
/// the ten rows in [90%, 95%) booked as `early-exit` were a DEFINITIONAL gap
/// between the two lines, not give-ups -- the 0.21 decode's boundary-sliver
/// finding.
pub const TIMEOUT_FRAC: f64 = 0.90;

/// Domains VAL cannot INGEST at all, from `benchmarks/val-unavailable.json`.
///
/// VAL emits its refusals BEFORE judging any plan, so `val: false` on such a
/// domain is validation UNAVAILABLE, not a rejected plan. The 0.20 runner knew
/// only the "Parser failed" signature, which is why `data-network-2018` and
/// `factory-robot-2026` arrived as `false` and were dropped from coverage
/// outright: the table read 46/240 and 113/320 where the boards beside it said
/// 53 and 121. Fifteen instances light.
#[derive(Debug, Clone, Default)]
pub struct ValUnavailable {
    keys: BTreeSet<String>,
}

impl ValUnavailable {
    /// Build from the `unavailable` object's keys. A missing file is an empty
    /// map, matching Python -- the standings still render, just without the
    /// exemption.
    pub fn new<I: IntoIterator<Item = String>>(keys: I) -> Self {
        let keys: BTreeSet<String> = keys.into_iter().collect();
        debug_assert!(
            !keys.iter().any(|k| k.starts_with("None/")),
            "a val-unavailable key beginning \"None/\" would only ever match a \
             row with no `ipc`, which is a Python f-string artifact rather than \
             a rule; see RawRow::domain_key"
        );
        Self { keys }
    }

    pub fn contains(&self, r: &RawRow) -> bool {
        r.domain_key().is_some_and(|k| self.keys.contains(&k))
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

/// Classifies rows. The VAL map is a field rather than a module global, so a
/// test can construct a referee that knows nothing and prove the difference.
#[derive(Debug, Clone, Default)]
pub struct Referee {
    pub val_unavailable: ValUnavailable,
}

impl Referee {
    pub fn new(val_unavailable: ValUnavailable) -> Self {
        Self { val_unavailable }
    }

    /// Did this row produce a plan that counts?
    ///
    /// A `false` from a domain VAL cannot read is a harness gap, not a verdict.
    pub fn is_solved(&self, r: &RawRow) -> bool {
        if !r.solved {
            return false;
        }
        r.val != Some(false) || self.val_unavailable.contains(r)
    }

    /// The budget this row is denominated in.
    ///
    /// Python is `r.get("budget") or budget` -- and `or` is FALSY, so a stamp
    /// of `0` falls back exactly as `null` does. The row's own stamp beating the
    /// registry is the tier-move mechanism: one board name spans raws from two
    /// tiers, and a timeout is denominated in the budget the row actually ran
    /// under, not the tier the registry currently declares.
    pub fn budget_for(&self, r: &RawRow, registry: f64) -> f64 {
        match r.budget {
            Some(b) if b != 0.0 => b,
            _ => registry,
        }
    }

    pub fn classify(&self, r: &RawRow, registry_budget: f64) -> Class {
        let budget = self.budget_for(r, registry_budget);

        // Before the VAL-RED test, deliberately: a `val: false` row on a
        // VAL-unavailable domain is SOLVED and must never read VAL-RED.
        if self.is_solved(r) {
            return Class::Solved;
        }
        if r.solved && r.val == Some(false) {
            return Class::ValRed;
        }

        let ntext = r.note_text();

        // SEAM -- the pinned bug. `ipc67.py:493` also emits
        // "mem-cap (self-inflicted: node byte target raised)", which this
        // exact-equality test does not match, so those rows fall through to
        // `early-exit`. Seven of them are in the published table right now.
        // Held here on purpose: a port that changes a number cannot prove it is
        // a port. The fix, its regenerated goldens and the recorded movement
        // are a separate change -- docs/roadmap-0.26.md, Phase 0.
        if ntext == "mem-cap" {
            return Class::MemCap;
        }
        if ntext == "spawn-fail" {
            return Class::SpawnFail;
        }

        // Pre-0.20 rows only: elapsed was not recorded for unsolved rows, so a
        // graceful engine exit AT the armed wall is indistinguishable from a
        // true reject here. The 0.20 audit showed maintenance-2014's "8
        // rejects" were wall-exit timeouts. This class empties as boards
        // re-sweep.
        let Some(t) = r.time_secs() else {
            return Class::EngineRejectOrError;
        };

        if t >= budget * TIMEOUT_FRAC {
            return Class::Timeout;
        }

        // A named mechanism, and only AFTER the timeout test: a row carrying
        // "unsolvable at grounding" that ALSO spent its wall is a timeout.
        if ntext.starts_with("engine-exit")
            || ntext.contains("unsolvable")
            || ntext.contains("reject")
        {
            return Class::EngineRejectOrError;
        }

        Class::EarlyExit
    }

    pub fn coverage(&self, rows: &[RawRow], registry_budget: f64) -> Coverage {
        let mut classes: BTreeMap<Class, usize> = BTreeMap::new();
        let mut solved = 0usize;
        let mut unattested = 0usize;
        for r in rows {
            *classes
                .entry(self.classify(r, registry_budget))
                .or_insert(0) += 1;
            if self.is_solved(r) {
                solved += 1;
                if r.val.is_none() {
                    unattested += 1;
                }
            }
        }
        Coverage {
            solved,
            total: rows.len(),
            classes,
            unattested,
        }
    }
}
