//! Build the child's environment from scratch.
//!
//! `ipc67.py` builds it as `dict(os.environ, FF_TIME_LIMIT=...)`. There are
//! **132 `FF_*` hatches** in the engine. An operator with any one of them
//! exported in their shell silently changes every board in the sweep, and
//! nothing in any row records that it happened -- so a board could be measured
//! under a configuration nobody can name afterwards, which is the one thing a
//! benchmark record cannot survive.
//!
//! So: inherit only what a process genuinely needs to run, drop every ambient
//! `FF_*`, inject the budgets, then apply the board's declared `env` last. The
//! board's `env` is stored alongside its results, so a row can no longer have
//! been measured under a hatch that is not on the record.

use std::collections::BTreeMap;
use std::ffi::OsString;

/// Variables a child needs regardless of what it is. Everything else goes.
const INHERIT: &[&str] = &[
    "PATH",
    "HOME",
    "TMPDIR",
    "USER",
    "SHELL",
    "LANG",
    "TERM",
    "TZ",
    // The corpus and validator locations are how the harness is pointed at
    // vendored data, and both are honoured for continuity with the Python.
    "FERROPLAN_IPC_CORPUS",
    "FERROPLAN_VAL",
];

/// The engine's wall budget. Not a CLI flag -- `ff` has none. Telling the
/// engine its real budget is what lets the bounded rungs stop starving the
/// complete fallback near the edge.
pub const TIME_LIMIT: &str = "FF_TIME_LIMIT";

/// The engine's retained-state budget. It exists because Darwin cannot enforce
/// `RLIMIT_AS`, so the cap has to trip INTERNALLY (the engine sheds state and
/// spends its wall) rather than externally (the watchdog kills the job with
/// wall unspent).
pub const MEM_BUDGET_GB: &str = "FF_MEM_BUDGET_GB";

/// Assemble the child environment. `board_env` is applied last and wins.
pub fn build(
    timeout_secs: u64,
    mem_gb: f64,
    board_env: &BTreeMap<String, String>,
) -> Vec<(OsString, OsString)> {
    let mut out: Vec<(OsString, OsString)> = Vec::new();
    for k in INHERIT {
        if let Some(v) = std::env::var_os(k) {
            out.push((OsString::from(k), v));
        }
    }
    out.push((TIME_LIMIT.into(), timeout_secs.to_string().into()));
    if mem_gb > 0.0 {
        out.push((MEM_BUDGET_GB.into(), mem_gb.to_string().into()));
    }
    for (k, v) in board_env {
        out.retain(|(ek, _)| ek != std::ffi::OsStr::new(k));
        out.push((k.clone().into(), v.clone().into()));
    }
    out
}

/// Reject a board whose declared `env` would contradict its own budget stamp.
///
/// A row carries `budget`, and `standings.py` denominates the timeout class in
/// it. If the board also exported a different `FF_TIME_LIMIT`, the engine would
/// be running to one wall while the row claims another -- exactly the class of
/// lie the stamp exists to prevent.
pub fn validate(
    timeout_secs: u64,
    mem_gb: f64,
    board_env: &BTreeMap<String, String>,
) -> Result<(), String> {
    if let Some(v) = board_env.get(TIME_LIMIT) {
        if v != &timeout_secs.to_string() {
            return Err(format!(
                "board env sets {TIME_LIMIT}={v} but the board's timeout is \
                 {timeout_secs}s; the row's budget stamp would be a lie"
            ));
        }
    }
    if let Some(v) = board_env.get(MEM_BUDGET_GB) {
        if v.parse::<f64>().ok() != Some(mem_gb) {
            return Err(format!(
                "board env sets {MEM_BUDGET_GB}={v} but the board's mem_gb is \
                 {mem_gb}; two budgets, one column"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Defends the incident this module exists for: an ambient hatch must not
    /// reach the child.
    #[test]
    fn ambient_ff_hatches_are_scrubbed() {
        std::env::set_var("FF_NO_LAMA", "1");
        let e = build(60, 6.0, &BTreeMap::new());
        assert!(
            !e.iter().any(|(k, _)| k == "FF_NO_LAMA"),
            "an FF_* export in the operator's shell must never reach a board"
        );
        std::env::remove_var("FF_NO_LAMA");
    }

    #[test]
    fn the_budgets_are_always_injected() {
        let e = build(300, 6.0, &BTreeMap::new());
        assert!(e.iter().any(|(k, v)| k == TIME_LIMIT && v == "300"));
        assert!(e.iter().any(|(k, _)| k == MEM_BUDGET_GB));
    }

    /// mem_gb == 0 means the cap is off, and the engine must not be told a
    /// budget it is not being held to.
    #[test]
    fn a_disabled_mem_cap_injects_nothing() {
        let e = build(60, 0.0, &BTreeMap::new());
        assert!(!e.iter().any(|(k, _)| k == MEM_BUDGET_GB));
    }

    #[test]
    fn a_board_env_declaring_a_different_wall_is_refused() {
        let mut env = BTreeMap::new();
        env.insert(TIME_LIMIT.to_string(), "30".to_string());
        assert!(validate(60, 6.0, &env).is_err());
        env.insert(TIME_LIMIT.to_string(), "60".to_string());
        assert!(validate(60, 6.0, &env).is_ok());
    }

    /// A board's own declared hatch DOES reach the child -- that is how a
    /// config variant is expressed at all, and it is recorded on the board.
    #[test]
    fn a_declared_board_hatch_is_applied_last() {
        let mut env = BTreeMap::new();
        env.insert("FF_NO_SAT_RATEBAIL".into(), "1".into());
        let e = build(60, 6.0, &env);
        assert!(e.iter().any(|(k, v)| k == "FF_NO_SAT_RATEBAIL" && v == "1"));
    }
}
