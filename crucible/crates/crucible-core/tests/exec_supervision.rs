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

/// THE CPU CLOCK, the R2 instrument. Two children, one that spins and one
/// that sleeps, and the ratio of CPU to effective wall must say which is
/// which. The R1 runner could not have passed this: it polled `pidrusage`
/// every 250 ms and read Mach units as nanoseconds, so a spinning child
/// reported ~2% of its wall as CPU and a 200 ms child reported nothing.
#[test]
fn cpu_time_comes_from_wait4_and_tracks_the_effective_clock() {
    // 2 s of spinning: the supervisor reaps on its tick, so the effective
    // clock can read up to a tick long, and the fixture has to be long enough
    // that the tick cannot decide the verdict.
    let out = Case::new()
        .set("FAKEFF_BURN_MS", "2000")
        .run(no_ctl())
        .unwrap();
    assert_eq!(out.cpu_instrument, exec::CPU_INSTRUMENT);
    // A factor of two still pins the units (the bug read 41x low); the
    // floor is loose because a packed sweep may be sharing the box.
    assert!(
        (900..=2300).contains(&out.cpu_ms),
        "2 s spin recorded {} ms of CPU",
        out.cpu_ms
    );
    // The units are what this pins (the cpu_ms bound above); the ratio's
    // floor is loose because the test box may be running a sweep.
    let rho = out.cpu_ms as f64 / out.effective.as_millis().max(1) as f64;
    assert!(
        (0.60..=1.05).contains(&rho),
        "spinning child: rho {rho:.3} (cpu {} ms over {} ms effective)",
        out.cpu_ms,
        out.effective.as_millis()
    );

    let out = Case::new()
        .set("FAKEFF_SLEEP_MS", "600")
        .run(no_ctl())
        .unwrap();
    let rho = out.cpu_ms as f64 / out.effective.as_millis().max(1) as f64;
    assert!(
        rho < 0.20,
        "sleeping child: rho {rho:.3} (cpu {} ms over {} ms effective)",
        out.cpu_ms,
        out.effective.as_millis()
    );
}

/// A short run is exactly the case the poll missed. 120 ms of spinning must
/// still come back as ~120 ms of CPU, not as the last tick's stale reading.
#[test]
fn a_short_spinning_child_is_not_undercounted() {
    let out = Case::new()
        .set("FAKEFF_BURN_MS", "120")
        .run(no_ctl())
        .unwrap();
    assert!(
        out.cpu_ms >= 90,
        "120 ms spin recorded only {} ms of CPU",
        out.cpu_ms
    );
}

/// THE ORPHAN. Stopping crucible with SIGTERM used to leave its planner
/// running under pid 1 (the 0.26 sweep did exactly that). Now an interrupt
/// cancels the child the way an operator Cancel does -- SIGTERM, then the
/// grace period, then SIGKILL, all to the group -- and the run comes back
/// marked cancelled with the child reaped.
///
/// The flag is process-global, so this test runs its body in a child
/// process of the test binary -- setting it in-process would cancel every
/// sibling test's planner too.
#[test]
fn an_interrupt_cancels_and_reaps_the_child() {
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args(["--ignored", "--exact", "interrupt_body", "--test-threads=1"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "inner test failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
#[ignore = "the body of an_interrupt_cancels_and_reaps_the_child; runs in its own process"]
fn interrupt_body() {
    exec::set_interrupted(false);
    let (_tx, rx) = mpsc::channel();
    let h = std::thread::spawn(move || {
        Case::new()
            .set("FAKEFF_SLEEP_MS", "20000")
            .timeout_ms(60_000)
            .run(rx)
    });
    std::thread::sleep(Duration::from_millis(600));
    let t0 = Instant::now();
    exec::set_interrupted(true);
    let out = h.join().unwrap().unwrap();
    exec::set_interrupted(false);
    assert_eq!(out.killed, Some(exec::Killed::Cancelled));
    assert!(
        t0.elapsed() < Duration::from_secs(8),
        "cancelled within the grace period, not at the 20 s sleep: {:?}",
        t0.elapsed()
    );
    // Reaped: signalling the pid must fail with ESRCH, not reach a zombie or
    // an orphan.
    // SAFETY: kill(2) with signal 0 only checks for existence.
    let rc = unsafe { libc::kill(out.pid, 0) };
    assert_ne!(rc, 0, "the child is still there");
}
