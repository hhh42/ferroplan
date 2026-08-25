//! External validation, and the tristate that three separate incidents came
//! out of.
//!
//! `val` is **not** a boolean. It is:
//!
//! * `Some(true)`  -- VAL read the plan and accepted it.
//! * `Some(false)` -- VAL read the plan and REJECTED it. A first-class alarm:
//!   either an engine soundness bug or a harness gap, and never to be lumped
//!   in with search losses.
//! * `None`        -- VAL could not render a verdict AT ALL.
//!
//! That third case is the one that keeps going wrong, because it looks like a
//! rejection from the outside. VAL emits its refusals BEFORE it reads any plan:
//! a domain it cannot parse, a typechecker complaint, a crash, a timeout. The
//! 0.20 runner tested for one refusal signature out of six, so
//! `data-network-2018` and `factory-robot-2026` arrived as `val: false`,
//! `standings.py` drops those from coverage, and the published table read
//! 46/240 and 113/320 where the boards beside it said 53 and 121 -- fifteen
//! instances light, on a released record, for a cycle.
//!
//! So the decision table below is separated from the subprocess entirely. It
//! takes a captured `(rc, stdout, stderr)` and returns a verdict, which is why
//! every branch can be tested against frozen real output instead of against a
//! validator that must be built, must be the right build, and crashes
//! differently on different machines.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// The wall VAL itself is held to. Running out of time is NOT a rejected plan --
/// the same shape as the 0.20 finding that graceful wall-exits were booked as
/// engine rejects.
pub const VAL_TIMEOUT: Duration = Duration::from_secs(120);

/// Half of `ff`'s decision-epoch EPS (0.001).
///
/// VAL groups happenings whose gap does not strictly EXCEED the tolerance, so
/// validating at exactly EPS treats our epsilon-separated pairs as simultaneous
/// and manufactures boundary mutex violations. At EPS/2 every epsilon gap
/// clears while true coincidences still group.
pub const TEMPORAL_TOLERANCE: &str = "0.0005";

/// Every way VAL says "I cannot read this", none of which is a verdict on a
/// plan. Kept in step with `benchmarks/val-availability.py`, which probes the
/// corpus for the domains that hit them.
pub const UNAVAILABLE_SIGNATURES: &[&str] = &[
    "Parser failed",
    "Problem in domain definition!",
    "Problem in problem definition!",
    "Syntax error",
    // VAL's typechecker refuses the files BEFORE reading any plan.
    // markettrader's instances init undeclared fluents -- a commented-out
    // metric's leftovers -- and hit the problem-side one; 0.21 booked that
    // board's only VAL-RED through exactly this gap.
    "Type problem in domain description!",
    "Type problem in problem specification!",
];

/// Why validation was unavailable. New here: in the Python the `None` is
/// anonymous, and every one of these incidents began with "we could not tell
/// why VAL said nothing."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unavailable {
    /// VAL could not ingest the domain or problem.
    Ingest,
    /// VAL died before rendering a verdict.
    Crash,
    Timeout,
    NoValidator,
}

impl Unavailable {
    pub fn label(self) -> &'static str {
        match self {
            Unavailable::Ingest => "ingest",
            Unavailable::Crash => "crash",
            Unavailable::Timeout => "timeout",
            Unavailable::NoValidator => "no-validator",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Valid,
    /// VAL read it and said no.
    Rejected,
    Unavailable(Unavailable),
}

impl Verdict {
    /// The row's `val` field: `true` / `false` / `null`.
    pub fn as_json(self) -> Option<bool> {
        match self {
            Verdict::Valid => Some(true),
            Verdict::Rejected => Some(false),
            Verdict::Unavailable(_) => None,
        }
    }

    pub fn reason(self) -> Option<&'static str> {
        match self {
            Verdict::Unavailable(u) => Some(u.label()),
            _ => None,
        }
    }
}

/// THE DECISION TABLE. Pure, so every branch is testable against frozen real
/// output from a real VAL on a real domain.
pub fn judge(rc: Option<i32>, signal: Option<i32>, stdout: &str, stderr: &str) -> Verdict {
    let blob = format!("{stdout}{stderr}");

    // A validator that DIED before rendering a verdict has not rejected the
    // plan. VAL SIGBUSes deterministically on several storage-time-constraints
    // plans -- exit -10/-11 with zero output -- and 0.23 booked those as
    // rejections. The guard is "and no output": a crash that still printed a
    // verdict IS a verdict.
    let died = signal.is_some() || !matches!(rc, Some(0) | Some(1));
    if died && blob.trim().is_empty() {
        return Verdict::Unavailable(Unavailable::Crash);
    }

    // Could not INGEST the files, independent of the plan. Checked on stdout
    // AND stderr, because VAL is not consistent about which it uses.
    if UNAVAILABLE_SIGNATURES.iter().any(|s| blob.contains(s)) {
        return Verdict::Unavailable(Unavailable::Ingest);
    }

    if rc == Some(0) && stdout.contains("Plan valid") {
        Verdict::Valid
    } else {
        Verdict::Rejected
    }
}

/// One step of a plan, as `ff --json` reports it.
#[derive(Debug, Clone)]
pub struct Step {
    pub action: String,
    pub args: Vec<String>,
    pub time: Option<f64>,
    pub duration: Option<f64>,
}

/// Render a plan in the format VAL parses.
///
/// Temporal steps are `time: (action args) [duration]`, which is what
/// `TimedPlan::to_ipc` emits; a classical step inside a temporal plan has no
/// duration and drops the brackets. Actions and arguments are lowercased,
/// because PDDL is case-insensitive and VAL is not.
pub fn render_plan(steps: &[Step], temporal: bool) -> String {
    let mut out = String::new();
    for s in steps {
        let mut act = String::from("(");
        act.push_str(&s.action.to_lowercase());
        for a in &s.args {
            act.push(' ');
            act.push_str(&a.to_lowercase());
        }
        act.push(')');
        if temporal {
            out.push_str(&format!("{}: {act}", s.time.unwrap_or(0.0)));
            if let Some(d) = s.duration {
                out.push_str(&format!(" [{d}]"));
            }
        } else {
            out.push_str(&act);
        }
        out.push('\n');
    }
    out
}

/// Where the validator is, following the same order the Python uses.
pub fn find(repo: &Path, configured: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = configured {
        if p.is_file() {
            return Some(p.to_path_buf());
        }
    }
    if let Some(p) = std::env::var_os("FERROPLAN_VAL").map(PathBuf::from) {
        if p.is_file() {
            return Some(p);
        }
    }
    let local = repo.join("benchmarks/.val/VAL/build/bin/Validate");
    if local.is_file() {
        return Some(local);
    }
    None
}

/// Validate one plan. `plan_path` is where the rendered plan is written -- a
/// deterministic path rather than a temp file, so a VAL-RED row's plan is still
/// on disk afterwards and the rejection can be reproduced by hand.
pub fn validate(
    val: Option<&Path>,
    domain: &Path,
    problem: &Path,
    steps: &[Step],
    temporal: bool,
    plan_path: &Path,
) -> Verdict {
    let Some(val) = val else {
        return Verdict::Unavailable(Unavailable::NoValidator);
    };
    if let Some(dir) = plan_path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if std::fs::write(plan_path, render_plan(steps, temporal)).is_err() {
        return Verdict::Unavailable(Unavailable::NoValidator);
    }

    let mut cmd = std::process::Command::new(val);
    if temporal {
        cmd.arg("-t").arg(TEMPORAL_TOLERANCE);
    }
    cmd.arg(domain).arg(problem).arg(plan_path);

    match cmd.output() {
        Ok(o) => {
            #[cfg(unix)]
            let signal = {
                use std::os::unix::process::ExitStatusExt;
                o.status.signal()
            };
            #[cfg(not(unix))]
            let signal = None;
            judge(
                o.status.code(),
                signal,
                &String::from_utf8_lossy(&o.stdout),
                &String::from_utf8_lossy(&o.stderr),
            )
        }
        Err(_) => Verdict::Unavailable(Unavailable::NoValidator),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE FIFTEEN INSTANCES LIGHT. All six refusal signatures mean UNAVAILABLE,
    /// on stdout or on stderr, and never `Rejected`.
    #[test]
    fn every_ingest_refusal_is_unavailable_not_a_rejection() {
        for sig in UNAVAILABLE_SIGNATURES {
            assert_eq!(
                judge(Some(1), None, sig, ""),
                Verdict::Unavailable(Unavailable::Ingest),
                "on stdout: {sig}"
            );
            assert_eq!(
                judge(Some(1), None, "", sig),
                Verdict::Unavailable(Unavailable::Ingest),
                "on stderr: {sig}"
            );
        }
    }

    /// The signature 0.21 was missing, which cost that board its only VAL-RED.
    #[test]
    fn the_typechecker_refusal_is_covered() {
        assert!(UNAVAILABLE_SIGNATURES.contains(&"Type problem in problem specification!"));
    }

    /// A validator that DIED has not rejected the plan. 0.23 found VAL
    /// SIGBUSing deterministically on storage-time-constraints, exit -10/-11
    /// with zero output.
    #[test]
    fn a_crash_with_no_output_is_unavailable() {
        assert_eq!(
            judge(None, Some(11), "", ""),
            Verdict::Unavailable(Unavailable::Crash)
        );
        assert_eq!(
            judge(Some(2), None, "  \n ", ""),
            Verdict::Unavailable(Unavailable::Crash)
        );
    }

    /// ...but a crash that still printed a verdict IS a verdict. The Python's
    /// guard is `and not blob.strip()`, and dropping that half would turn real
    /// rejections into silence.
    #[test]
    fn a_crash_that_still_spoke_is_a_verdict() {
        assert_eq!(judge(None, Some(11), "Plan invalid", ""), Verdict::Rejected);
    }

    #[test]
    fn the_ordinary_verdicts() {
        assert_eq!(judge(Some(0), None, "Plan valid", ""), Verdict::Valid);
        assert_eq!(judge(Some(1), None, "Plan invalid", ""), Verdict::Rejected);
        // rc 0 is in the allowed set, but without the words it is not a pass.
        assert_eq!(judge(Some(0), None, "", ""), Verdict::Rejected);
    }

    #[test]
    fn the_tristate_maps_onto_the_row() {
        assert_eq!(Verdict::Valid.as_json(), Some(true));
        assert_eq!(Verdict::Rejected.as_json(), Some(false));
        assert_eq!(
            Verdict::Unavailable(Unavailable::Ingest).as_json(),
            None,
            "unavailable is null in the row, never false"
        );
        assert_eq!(
            Verdict::Unavailable(Unavailable::Timeout).reason(),
            Some("timeout")
        );
    }

    /// A classical plan is bare parenthesised actions, lowercased.
    #[test]
    fn a_classical_plan_renders_without_times() {
        let s = vec![Step {
            action: "MOVE".into(),
            args: vec!["ROOMA".into(), "RoomB".into()],
            time: None,
            duration: None,
        }];
        assert_eq!(render_plan(&s, false), "(move rooma roomb)\n");
    }

    /// A temporal step carries its time and duration; a classical step INSIDE a
    /// temporal plan has no duration and drops the brackets.
    #[test]
    fn a_temporal_plan_renders_times_and_optional_brackets() {
        let s = vec![
            Step {
                action: "a".into(),
                args: vec![],
                time: Some(0.0),
                duration: Some(5.0),
            },
            Step {
                action: "b".into(),
                args: vec!["x".into()],
                time: Some(5.001),
                duration: None,
            },
        ];
        assert_eq!(render_plan(&s, true), "0: (a) [5]\n5.001: (b x)\n");
    }

    /// EPS/2, not EPS. At exactly EPS, VAL groups our epsilon-separated
    /// happenings as simultaneous and invents boundary mutex violations.
    #[test]
    fn the_temporal_tolerance_is_half_the_decision_epoch() {
        assert_eq!(TEMPORAL_TOLERANCE, "0.0005");
        assert!(TEMPORAL_TOLERANCE.parse::<f64>().unwrap() < 0.001);
    }

    /// No validator at all is UNAVAILABLE -- a board with no VAL renders
    /// "VAL not available", never a board of rejected plans.
    #[test]
    fn a_missing_validator_is_unavailable() {
        let v = validate(
            None,
            Path::new("d.pddl"),
            Path::new("p.pddl"),
            &[],
            false,
            Path::new("/tmp/x.plan"),
        );
        assert_eq!(v, Verdict::Unavailable(Unavailable::NoValidator));
        assert_eq!(v.as_json(), None);
    }
}
