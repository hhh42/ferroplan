//! What happens AROUND a planner process, tested against a planner that does
//! exactly what it is told.
//!
//! Each test here defends a specific way this has gone wrong before, or a
//! specific way a naive port would go wrong:
//!
//!   * a supervisor that waits without draining the pipes deadlocks the moment
//!     the child exceeds the 64 KiB pipe buffer, which `ff --json` does
//!     routinely on a long plan -- and Python's communicate() hid it;
//!   * a suspended run that keeps counting wall time times out while stopped,
//!     which would turn every contention window into fabricated coverage loss;
//!   * a memory cap that measures address space on Darwin measures nothing at
//!     all, because the kernel refuses every setrlimit on RLIMIT_AS;
//!   * a kill aimed at a bare pid leaves the process group behind.

use crucible_core::exec::{self, env, Ctl, ExecError, RunRequest};
use crucible_core::platform::{self, MemCap, Platform};
use std::collections::BTreeMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// `fakeff` is a bin target of this same crate, so cargo builds it for these
/// tests and hands us its path -- no guessing, and no way for the test to run
/// against a stale binary.
fn fakeff() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_fakeff"))
}

struct Case {
    envs: Vec<(std::ffi::OsString, std::ffi::OsString)>,
    timeout: Duration,
    mem_cap: MemCap,
}

impl Case {
    fn new() -> Self {
        Self {
            envs: env::build(60, 0.0, &BTreeMap::new()),
            timeout: Duration::from_secs(30),
            mem_cap: MemCap::Off,
        }
    }
    fn set(mut self, k: &str, v: &str) -> Self {
        self.envs.push((k.into(), v.into()));
        self
    }
    fn timeout_ms(mut self, ms: u64) -> Self {
        self.timeout = Duration::from_millis(ms);
        self
    }
    fn run(self, ctl: mpsc::Receiver<Ctl>) -> Result<exec::RunOutcome, ExecError> {
        let plat = platform::host();
        let bin = fakeff();
        let args: Vec<String> = vec!["-o".into(), "d.pddl".into(), "-f".into(), "p.pddl".into()];
        exec::run(
            &RunRequest {
                program: &bin,
                args: &args,
                envs: &self.envs,
                timeout: self.timeout,
                mem_cap: self.mem_cap,
                on_spawn: None,
            },
            &plat,
            &ctl,
        )
    }
}

fn no_ctl() -> mpsc::Receiver<Ctl> {
    let (_tx, rx) = mpsc::channel();
    rx
}

#[test]
fn a_plain_run_captures_stdout_and_exits_clean() {
    let out = Case::new().run(no_ctl()).expect("runs");
    assert_eq!(out.exit_code, Some(0));
    assert!(out.killed.is_none());
    assert!(out.stdout.contains("\"solved\":true"));
    assert!(out.end_ts >= out.start_ts);
}

/// THE DEADLOCK. A 400-step plan blows past the 64 KiB pipe buffer; a
/// supervisor that polls `try_wait` without reading stdout blocks the child in
/// `write` and then waits for it forever. This test hangs rather than fails if
/// the drain is removed, which is exactly the failure it is guarding.
#[test]
fn a_child_that_fills_the_pipe_buffer_does_not_deadlock() {
    let big = 4 * 1024 * 1024;
    let out = Case::new()
        .set("FAKEFF_STDOUT_BYTES", &big.to_string())
        .timeout_ms(20_000)
        .run(no_ctl())
        .expect("runs");
    assert!(
        out.killed.is_none(),
        "it should exit on its own, not be killed"
    );
    assert!(
        out.stdout.len() >= big,
        "captured {} bytes of {big}",
        out.stdout.len()
    );
}

#[test]
fn a_run_that_overruns_its_wall_is_killed_at_the_deadline() {
    let started = Instant::now();
    let out = Case::new()
        .set("FAKEFF_SLEEP_MS", "60000")
        .timeout_ms(1_000)
        .run(no_ctl())
        .expect("runs");
    assert_eq!(out.killed, Some(exec::Killed::Deadline));
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "the deadline must be enforced, not waited out"
    );
    // Immediate SIGKILL to the group, no SIGTERM grace: adding one would move
    // `time` on every timeout row in the corpus.
    assert_eq!(out.term_signal, Some(9));
}

/// A SUSPENDED RUN MUST NOT TIME OUT. This is the property the whole project
/// exists for: contention may cost a timing number, never hours of work.
#[test]
fn time_spent_suspended_is_not_charged_against_the_wall() {
    let (tx, rx) = mpsc::channel();
    let h = std::thread::spawn(move || {
        Case::new()
            .set("FAKEFF_SLEEP_MS", "3000")
            .timeout_ms(4_000)
            .run(rx)
    });
    // Stop it for longer than its remaining budget. On wall time it would
    // certainly time out; on effective time it must survive and finish.
    std::thread::sleep(Duration::from_millis(500));
    tx.send(Ctl::Stop).unwrap();
    std::thread::sleep(Duration::from_millis(3000));
    tx.send(Ctl::Cont).unwrap();

    let out = h.join().unwrap().expect("runs");
    assert!(
        out.killed.is_none(),
        "a stopped run must not time out; suspended={:?} wall={:?} effective={:?}",
        out.suspended,
        out.wall,
        out.effective
    );
    assert!(out.suspended >= Duration::from_millis(2500));
    assert!(
        out.wall > out.effective,
        "wall {:?} must exceed effective {:?}",
        out.wall,
        out.effective
    );
}

/// The RSS watchdog is the ONLY working instrument on Darwin, because the
/// kernel reports RLIMIT_AS as INFINITY and then rejects every setrlimit on it.
#[test]
fn the_probe_picks_a_cap_instrument_that_actually_works() {
    let plat = platform::host();
    let cap = plat.probe_mem_cap(1 << 30);
    assert_ne!(cap, MemCap::Off, "a non-zero cap must pick an instrument");
    #[cfg(target_os = "macos")]
    assert!(
        matches!(cap, MemCap::RssWatchdog(_)),
        "Darwin cannot enforce RLIMIT_AS; assuming it can turned a whole \
         12-board sweep into spawn-fail rows"
    );
    assert_eq!(plat.probe_mem_cap(0), MemCap::Off);
}

/// The probe must leave the parent's own limits exactly as it found them -- it
/// runs once, in a process that then supervises a multi-hour sweep.
#[test]
fn the_probe_is_side_effect_free() {
    let before = current_rlimit_as();
    let _ = platform::host().probe_mem_cap(1 << 20);
    assert_eq!(before, current_rlimit_as());
}

fn current_rlimit_as() -> (u64, u64) {
    let mut r = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: a correctly-sized rlimit for a read-only getrlimit.
    unsafe { libc::getrlimit(libc::RLIMIT_AS, &mut r) };
    (r.rlim_cur, r.rlim_max)
}

#[test]
fn a_ballooning_run_is_killed_and_marked_mem_cap() {
    let plat = platform::host();
    let cap = plat.probe_mem_cap(64 << 20); // 64 MiB
    let out = Case {
        mem_cap: cap,
        ..Case::new()
    }
    .set("FAKEFF_RSS_MB", "400")
    .set("FAKEFF_SLEEP_MS", "20000")
    .timeout_ms(20_000)
    .run(no_ctl())
    .expect("runs");
    assert!(
        out.mem_hit || out.exit_code.is_some_and(|c| c != 0) || out.term_signal.is_some(),
        "a run 6x over its cap must not simply succeed: {out:?}"
    );
    assert_ne!(
        out.killed,
        Some(exec::Killed::Deadline),
        "it died on memory, not time"
    );
}

/// A missing binary is fatal to the SWEEP, never a row. Booking it as a result
/// would produce 6,584 `spawn-fail` rows and call it a measurement.
#[test]
fn a_missing_planner_is_a_sweep_error_not_a_result() {
    let plat = platform::host();
    let missing = std::path::PathBuf::from("/nonexistent/ff");
    let err = exec::run(
        &RunRequest {
            program: &missing,
            args: &[],
            envs: &[],
            timeout: Duration::from_secs(1),
            mem_cap: MemCap::Off,
            on_spawn: None,
        },
        &plat,
        &no_ctl(),
    )
    .unwrap_err();
    assert!(matches!(err, ExecError::NotRunnable { .. }), "got {err:?}");
}

/// Cancellation asks politely, then insists. A child that ignores SIGTERM must
/// still die -- and it must not be left stopped.
#[test]
fn cancellation_escalates_to_a_group_kill() {
    let (tx, rx) = mpsc::channel();
    let h = std::thread::spawn(move || {
        Case::new()
            .set("FAKEFF_SLEEP_MS", "60000")
            .set("FAKEFF_IGNORE_TERM", "1")
            .timeout_ms(60_000)
            .run(rx)
    });
    std::thread::sleep(Duration::from_millis(600));
    tx.send(Ctl::Cancel).unwrap();
    let started = Instant::now();
    let out = h.join().unwrap().expect("runs");
    assert_eq!(out.killed, Some(exec::Killed::Cancelled));
    assert_eq!(
        out.term_signal,
        Some(9),
        "SIGTERM was ignored, so SIGKILL followed"
    );
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "escalation must not wait out the child's own sleep"
    );
}

/// Politeness keeps the work: a demoted run is slowed, never discarded.
#[test]
fn demotion_does_not_disturb_a_running_child() {
    let (tx, rx) = mpsc::channel();
    let h = std::thread::spawn(move || {
        Case::new()
            .set("FAKEFF_SLEEP_MS", "1500")
            .timeout_ms(30_000)
            .run(rx)
    });
    std::thread::sleep(Duration::from_millis(300));
    tx.send(Ctl::Demote).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    tx.send(Ctl::Promote).unwrap();
    let out = h.join().unwrap().expect("runs");
    assert!(out.killed.is_none());
    assert_eq!(out.exit_code, Some(0));
}
