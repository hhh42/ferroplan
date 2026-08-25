//! The failure-class taxonomy, ported from `benchmarks/standings.py:243-293`.
//!
//! The ORDER these render in is a property of the type, not of the renderer.
//! Python's `coverage_line` does `sorted(cls.items())`, which sorts by the
//! class *label string* -- and that puts `VAL-RED` first, because `'V'` (0x56)
//! sorts below `'e'` (0x65). Delegating `Ord` to `label()` and counting into a
//! `BTreeMap` reproduces it structurally, so nobody can reorder the rendering
//! by reaching for a different map type.

/// What happened to one row. Every unsolved row lands in exactly one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Class {
    Solved,
    /// The engine produced a plan and VAL rejected it, on a domain VAL can
    /// read. "A first-class signal -- either an engine soundness bug or a
    /// harness/VAL configuration gap -- never to be lumped into search losses."
    ValRed,
    /// Killed on the memory budget, by whichever of the two instruments was
    /// available: `RLIMIT_AS` makes the child fail its own allocation, the RSS
    /// watchdog SIGKILLs it.
    MemCap,
    /// Runner-side `fork()` failure under memory pressure. Environmental, not
    /// an engine verdict -- pre-0.16 sweeps logged these as engine rejects and
    /// the record names the floor-tile t4/t8 cluster.
    SpawnFail,
    /// A named mechanism (parse/feature reject, grounding verdict, nonzero exit
    /// without a JSON verdict), or a pre-0.20 row that recorded no elapsed.
    EngineRejectOrError,
    /// Spent the wall. The line is 90% of the row's budget, not 95%.
    Timeout,
    /// Finished with wall left, no plan, no named mechanism: the search gave up.
    /// The class the 0.20 refill loop exists to shrink.
    EarlyExit,
}

impl Class {
    pub fn label(self) -> &'static str {
        match self {
            Class::Solved => "solved",
            Class::ValRed => "VAL-RED",
            Class::MemCap => "mem-cap",
            Class::SpawnFail => "spawn-fail",
            Class::EngineRejectOrError => "engine-reject/error",
            Class::Timeout => "timeout",
            Class::EarlyExit => "early-exit",
        }
    }
}

impl std::fmt::Display for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl Ord for Class {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Byte order over the label, matching Python's codepoint sort. UTF-8 is
        // order-preserving on codepoints and every label is ASCII, so the two
        // agree by construction rather than by luck.
        self.label().cmp(other.label())
    }
}

impl PartialOrd for Class {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// A board's coverage and the failure-class histogram beside it.
#[derive(Debug, Clone, PartialEq)]
pub struct Coverage {
    pub solved: usize,
    pub total: usize,
    /// Every row including `Solved`; the rendering skips it.
    pub classes: std::collections::BTreeMap<Class, usize>,
    /// Solved rows judged by no external referee (`val` is null). Every one
    /// still passed the engine's own oracles, but the table must say which
    /// referee a row had -- 71 quiet rows at the 0.24 audit is how this
    /// line got here.
    pub unattested: usize,
}

impl Coverage {
    /// The failure-class cell, exactly as `coverage_line` renders it.
    pub fn failure_classes(&self) -> String {
        let mut s = self
            .classes
            .iter()
            .filter(|(k, v)| **k != Class::Solved && **v > 0)
            .map(|(k, v)| format!("{v} {k}"))
            .collect::<Vec<_>>()
            .join(", ");
        if self.unattested > 0 {
            let note = format!(
                "{} solved VAL-unavailable (engine-oracle only; \
                 see benchmarks/val-availability.py)",
                self.unattested
            );
            s = if s.is_empty() {
                note
            } else {
                format!("{s}, {note}")
            };
        }
        if s.is_empty() {
            "none".to_string()
        } else {
            s
        }
    }
}
