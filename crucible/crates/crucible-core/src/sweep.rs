//! Measuring ONE instance: spawn the planner, read what came back, and turn it
//! into a row that says what actually happened.
//!
//! A direct port of `benchmarks/ipc67.py`'s `run_instance`, and the place where
//! most of that file's recorded incidents live. The ordering of the checks
//! below is not stylistic -- each one is there because putting it later
//! mislabelled a class of row, and every mislabelled row is a wrong number in a
//! published table.
//!
//! The single most important rule: **never classify on the exit code.** `ff`
//! exits 1 for "goal simplifies to TRUE, the empty plan solves it"
//! (`planner.rs:144`), so a trivially-solved problem looks like a failure from
//! the outside. The `--json` `solved` field is the verdict.

use crate::exec::{self, env as exec_env, Ctl, ExecError, RunRequest};
use crate::platform::{MemCap, Platform};
use crate::validate::{self, Verdict};
use crucible_publish::raw::{Instance as Label, Notes, Present, RawRow};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::Duration;

/// Everything about a board that a row is stamped with. This tuple IS the
/// row's identity -- the resume gate compares every field of it exactly -- so
/// it travels together and is never assembled ad hoc.
#[derive(Debug, Clone)]
pub struct BoardCfg {
    pub timeout_secs: u64,
    /// `None` renders as `"auto"` in the row, matching the Python.
    pub mode: Option<String>,
    pub jobs: u32,
    /// A STRING in the row, because `ipc67.py` passes the CLI argument through
    /// unconverted and the resume gate compares `str(threads)`.
    pub threads: u32,
    pub mem_gb: f64,
    pub env: BTreeMap<String, String>,
    pub extra_args: Vec<String>,
}

pub struct Engine {
    pub path: PathBuf,
    /// Exactly `ff --version`. Written into every row; NOT the identity the
    /// resume gate uses, because two builds of a cycle share it.
    pub ver: String,
    /// BLAKE3 of the binary -- the identity the resume gate DOES use. Stamped
    /// into every measured row under [`crate::sched::resume::ENGINE_KEY`], so
    /// a row is self-identifying in the database and in the exported raw
    /// alike. Empty means "unknown", and an unstamped row is refused by the
    /// gate rather than trusted.
    pub blake3: String,
}

/// What `ff --json` prints.
#[derive(serde::Deserialize)]
struct Solution {
    #[serde(default)]
    solved: bool,
    #[serde(default)]
    plan: Option<Plan>,
    #[serde(default)]
    notes: Vec<String>,
}

#[derive(serde::Deserialize)]
struct Plan {
    #[serde(default)]
    steps: Vec<PlanStep>,
    #[serde(default)]
    length: Option<u64>,
    #[serde(default)]
    metric: Option<f64>,
    #[serde(default)]
    makespan: Option<f64>,
}

#[derive(serde::Deserialize)]
struct PlanStep {
    action: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    time: Option<f64>,
    #[serde(default)]
    duration: Option<f64>,
}

/// The measurement of one instance, plus what crucible knows that the artifact
/// row does not.
pub struct Measured {
    pub row: RawRow,
    pub val_reason: Option<&'static str>,
    pub cpu_ms: u64,
    /// `Some("wait4")` when a child ran; `None` on the spawn-fail path.
    pub cpu_instrument: Option<&'static str>,
    /// The operator cancelled the run (a Ctl::Cancel, or SIGINT/SIGTERM to
    /// crucible). The row's numbers describe an interrupted run and must not
    /// be exported as a measurement.
    pub cancelled: bool,
    pub peak_rss: u64,
    pub suspended: Duration,
    /// The machine slept mid-run. Every number here is suspect.
    pub clock_jump: Duration,
    pub mem_instrument: &'static str,
    /// Real elapsed time, suspension included. Zero on the spawn-fail path.
    pub wall: Duration,
    pub exit_code: Option<i32>,
    pub term_signal: Option<i32>,
    /// The child that ran. `None` only when nothing was spawned.
    pub pid: Option<crate::platform::Pid>,
    pub pgid: Option<crate::platform::Pid>,
}

/// Build the argv `ipc67.py` builds.
pub fn argv(cfg: &BoardCfg, domain: &Path, problem: &Path) -> Vec<String> {
    let mut a = vec![
        "-o".into(),
        domain.to_string_lossy().into_owned(),
        "-f".into(),
        problem.to_string_lossy().into_owned(),
        "--json".into(),
        "--threads".into(),
        cfg.threads.to_string(),
    ];
    if let Some(m) = &cfg.mode {
        a.push("--mode".into());
        a.push(m.clone());
    }
    a.extend(cfg.extra_args.iter().cloned());
    a
}

#[allow(clippy::too_many_arguments)]
pub fn measure<P: Platform>(
    engine: &Engine,
    cfg: &BoardCfg,
    ipc: &str,
    variant: &str,
    inst: &crate::corpus::Instance,
    val: Option<&Path>,
    plan_dir: &Path,
    plat: &P,
    ctl: &Receiver<Ctl>,
    on_spawn: Option<&dyn Fn(crate::platform::Pid, f64)>,
) -> Measured {
    let mem_cap = plat.probe_mem_cap((cfg.mem_gb * (1u64 << 30) as f64) as u64);
    let envs = exec_env::build(cfg.timeout_secs, cfg.mem_gb, &cfg.env);
    let args = argv(cfg, &inst.domain, &inst.problem);

    let mut extra = serde_json::Map::new();
    if !engine.blake3.is_empty() {
        extra.insert(
            crate::sched::resume::ENGINE_KEY.to_string(),
            serde_json::Value::String(engine.blake3.clone()),
        );
    }
    let mut row = RawRow {
        ipc: Some(ipc.to_string()),
        variant: variant.to_string(),
        instance: if inst.label_is_int {
            Label::Num(inst.label.parse().unwrap_or(0))
        } else {
            Label::Parts(inst.label.clone())
        },
        solved: false,
        time: None,
        metric: None,
        length: None,
        val: None,
        notes: None,
        budget: Some(cfg.timeout_secs as f64),
        ver: Some(engine.ver.clone()),
        mode: Some(cfg.mode.clone().unwrap_or_else(|| "auto".into())),
        jobs: Some(cfg.jobs),
        threads: Some(serde_json::Value::String(cfg.threads.to_string())),
        start_ts: None,
        end_ts: None,
        makespan: None,
        resumed_clean: false,
        extra,
        present: Present::current(false),
    };

    let out = match exec::run(
        &RunRequest {
            program: &engine.path,
            args: &args,
            envs: &envs,
            timeout: Duration::from_secs(cfg.timeout_secs),
            mem_cap,
            on_spawn,
        },
        plat,
        ctl,
    ) {
        Ok(o) => o,
        Err(ExecError::SpawnFail(_)) => {
            // The SYSTEM could not fork, twice, five seconds apart. That is
            // environmental and NOT an engine verdict -- the 0.16 seq-mco sweep
            // lost floor-tile i7-i12 to exactly this, logged as engine rejects.
            row.notes = Some(Notes::One("spawn-fail".into()));
            return done(row, None, None, mem_cap);
        }
        Err(e) => {
            // A missing or unrunnable binary is fatal to the SWEEP, never a
            // row: booking it would produce 6,584 spawn-fail rows and call
            // them a measurement.
            panic!("planner not runnable: {e}");
        }
    };

    row.start_ts = Some(out.start_ts);

    // A hard-deadline kill records the BUDGET as the time, as an integer, which
    // is why `time` is polymorphic in every raw on this box.
    row.time = Some(if out.killed == Some(exec::Killed::Deadline) {
        serde_json::Number::from(cfg.timeout_secs)
    } else {
        serde_json::Number::from_f64((out.effective.as_secs_f64() * 100.0).round() / 100.0)
            .unwrap_or_else(|| serde_json::Number::from(0))
    });

    // MEM-CAP IS READ FIRST. Two instruments, one verdict: RLIMIT_AS makes the
    // child fail its own allocation, the RSS watchdog SIGKILLs it (rc -9, no
    // stderr). Reading the generic nonzero-exit branch first books the
    // watchdog's kill as `engine-exit--9`.
    if out.mem_hit || (out.exit_code.is_some_and(|c| c != 0) && out.stderr.contains("allocation")) {
        // A mem-cap row whose stderr carries the engine's node-raise narration
        // died on a byte target the refill re-entry raised past the declared
        // model -- self-inflicted, and the label says so.
        row.notes = Some(Notes::One(
            if out.stderr.contains("node byte target raised") {
                "mem-cap (self-inflicted: node byte target raised)".into()
            } else {
                "mem-cap".to_string()
            },
        ));
    }

    let sol: Option<Solution> = if out.stdout.trim().is_empty() {
        None
    } else {
        serde_json::from_str(&out.stdout).ok()
    };

    if sol.is_none() && out.exit_code.is_some_and(|c| c != 0) && row.notes.is_none() {
        // No JSON came back and the exit was nonzero: a real engine
        // error or reject, distinct from a clean "searched and found nothing"
        // JSON verdict.
        row.notes = Some(Notes::One(format!(
            "engine-exit-{}",
            out.term_signal.map(|s| -s).or(out.exit_code).unwrap_or(0)
        )));
    }

    let mut verdict = None;
    if let Some(s) = sol {
        if s.solved {
            let plan = s.plan.unwrap_or(Plan {
                steps: vec![],
                length: None,
                metric: None,
                makespan: None,
            });
            row.solved = true;
            row.metric = plan.metric;
            row.length = plan.length;
            row.makespan = plan.makespan;
            // `makespan` is written on the solved branch ONLY -- present here
            // even when null, absent entirely on an unsolved row.
            row.present.makespan = true;
            row.notes = if s.notes.is_empty() {
                None
            } else {
                Some(Notes::Many(
                    s.notes
                        .iter()
                        .map(|n| serde_json::Value::String(n.clone()))
                        .collect(),
                ))
            };

            let temporal = plan.makespan.is_some();
            let steps: Vec<validate::Step> = plan
                .steps
                .iter()
                .map(|p| validate::Step {
                    action: p.action.clone(),
                    args: p.args.clone(),
                    time: p.time,
                    duration: p.duration,
                })
                .collect();
            let plan_path = plan_dir.join(variant).join(format!("{}.plan", inst.label));
            let v = validate::validate(
                val,
                &inst.domain,
                &inst.problem,
                &steps,
                temporal,
                &plan_path,
            );
            row.val = v.as_json();
            verdict = Some(v);
            // A VALID plan's text is not evidence of anything; a REJECTED one
            // is, and stays on disk so the rejection is reproducible by hand.
            if v == Verdict::Valid {
                let _ = std::fs::remove_file(&plan_path);
            }
        } else if !s.notes.is_empty() && row.notes.is_none() {
            // Unsolved WITH a named mechanism -- "unsolvable at grounding: ..."
            // -- which the standings need for the reject-vs-search attribution.
            row.notes = Some(Notes::Many(
                s.notes
                    .iter()
                    .map(|n| serde_json::Value::String(n.clone()))
                    .collect(),
            ));
        }
    }

    done(row, verdict, Some(&out), mem_cap)
}

fn done(
    mut row: RawRow,
    verdict: Option<Verdict>,
    out: Option<&exec::RunOutcome>,
    mem_cap: MemCap,
) -> Measured {
    // Stamped on every exit path, because a row only reaches the artifact once
    // its run finished SOMEHOW. A row that was never written -- the runner
    // killed mid-instance -- has no end_ts by construction, and the resume
    // gate's straddle rule falls out of that for free.
    row.end_ts = Some(
        (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
            * 100.0)
            .round()
            / 100.0,
    );
    Measured {
        row,
        val_reason: verdict.and_then(|v| v.reason()),
        cpu_ms: out.map_or(0, |o| o.cpu_ms),
        cpu_instrument: out.map(|o| o.cpu_instrument),
        cancelled: out.is_some_and(|o| o.killed == Some(exec::Killed::Cancelled)),
        peak_rss: out.map_or(0, |o| o.peak_rss),
        suspended: out.map_or(Duration::ZERO, |o| o.suspended),
        clock_jump: out.map_or(Duration::ZERO, |o| o.clock_jump),
        mem_instrument: mem_cap.instrument(),
        wall: out.map_or(Duration::ZERO, |o| o.wall),
        exit_code: out.and_then(|o| o.exit_code),
        term_signal: out.and_then(|o| o.term_signal),
        pid: out.map(|o| o.pid),
        pgid: out.map(|o| o.pgid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> BoardCfg {
        BoardCfg {
            timeout_secs: 60,
            mode: None,
            jobs: 2,
            threads: 1,
            mem_gb: 6.0,
            env: BTreeMap::new(),
            extra_args: vec![],
        }
    }

    /// The argv is the Python's, in the Python's order.
    #[test]
    fn the_argv_matches_the_runner_it_replaces() {
        let a = argv(&cfg(), Path::new("d.pddl"), Path::new("p.pddl"));
        assert_eq!(
            a,
            ["-o", "d.pddl", "-f", "p.pddl", "--json", "--threads", "1"]
        );
        let mut c = cfg();
        c.mode = Some("optimal".into());
        c.threads = 4;
        let a = argv(&c, Path::new("d"), Path::new("p"));
        assert!(a.windows(2).any(|w| w == ["--mode", "optimal"]));
        assert!(a.windows(2).any(|w| w == ["--threads", "4"]));
    }

    /// `ff` has NO time-limit flag; the budget is an environment variable, and
    /// the engine needs it so its bounded rungs stop starving the complete
    /// fallback near the edge.
    #[test]
    fn the_budget_reaches_the_child_as_an_environment_variable() {
        let e = exec_env::build(300, 6.0, &BTreeMap::new());
        assert!(e.iter().any(|(k, v)| k == "FF_TIME_LIMIT" && v == "300"));
        assert!(!argv(&cfg(), Path::new("d"), Path::new("p"))
            .iter()
            .any(|a| a.contains("time-limit")));
    }
}
