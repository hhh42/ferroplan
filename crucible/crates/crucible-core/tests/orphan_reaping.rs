//! Reaping proved against real processes, because the interesting failures are
//! all in the kernel's half of the story.
//!
//! Each test here stands for a way this goes wrong:
//!
//!   * a reaper that asks politely never kills a SIGSTOPped orphan -- a stopped
//!     process does not run, so it never runs a SIGTERM handler, and the
//!     spec's five-second grace elapses against a process that will still be
//!     sitting on its resident pages after the reboot;
//!   * a reaper that trusts a recorded pid kills whatever now holds that
//!     number. Pids recycle below 100000 on macOS; on a personal workstation
//!     the stranger it kills is the operator's own editor, and it does it on
//!     the startup right after a crash;
//!   * a pid that has simply gone must be an ordinary verdict, not an error and
//!     certainly not a panic in the recovery path;
//!   * a guard that kills on drop is worth nothing if it also kills after being
//!     disarmed, because then nobody will leave it armed.
//!
//! "Gone" here always means WAITED FOR. A zombie still answers `kill(pid, 0)`,
//! so a test that polled for signal delivery would pass against a reaper that
//! killed nothing at all.

use crucible_core::exec::orphan::{self, GroupGuard, LiveChild, Reaped};
use crucible_core::platform::{self, Pid, Platform, ProcIdentity};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// `fakeff` is a bin target of this same crate, so cargo builds it for these
/// tests and hands us its path.
fn fakeff() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_fakeff"))
}

/// Spawn a fakeff that will still be there when we come back, in its OWN
/// process group -- the same `setpgid(0, 0)` the runner's spawn hook does,
/// because the whole kill story is a group story.
fn spawn_sleeper() -> Child {
    spawn_with(&[])
}

fn spawn_with(extra: &[(&str, &str)]) -> Child {
    let mut cmd = Command::new(fakeff());
    cmd.env("FAKEFF_SLEEP_MS", "600000")
        .envs(extra.iter().copied())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: the hook runs between fork and exec and calls only setpgid.
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(|| {
            if libc::setpgid(0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    cmd.spawn().expect("fakeff spawns")
}

/// Wait until the child has actually `exec`'d.
///
/// Between fork and exec the pid still carries the TEST BINARY's path, so a
/// LiveChild recorded too early records the wrong identity -- and then the
/// reaper correctly refuses to kill it and the test fails for a reason that has
/// nothing to do with reaping.
fn exec_identity(pid: Pid, plat: &impl Platform) -> ProcIdentity {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Some(id) = plat.proc_identity(pid) {
            if id.path.ends_with("fakeff") {
                return id;
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("fakeff pid {pid} never exec'd");
}

fn record(pid: Pid, id: &ProcIdentity, stopped: bool) -> LiveChild {
    LiveChild {
        stopped,
        ..LiveChild::record(pid, Some(41), id, 1_700_000_000.0)
    }
}

/// True once the child has been waited for.
///
/// "Gone" cannot mean anything weaker. A zombie answers `kill(pid, 0)` exactly
/// as a live process does, so a poll built on signal delivery would pass
/// against a reaper that killed nothing at all. `try_wait` also keeps the
/// status once it has been collected, so calling this after it has already
/// returned true stays true instead of racing a second reap.
fn reaped(child: &mut Child) -> bool {
    matches!(child.try_wait(), Ok(Some(_)))
}

fn died_within(child: &mut Child, limit: Duration) -> bool {
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        if reaped(child) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    reaped(child)
}

/// Stop the group and wait until the kernel says it really is stopped. Without
/// this confirmation a test could race ahead and prove the easy thing -- that a
/// RUNNING orphan dies -- while claiming to have proved the hard one.
fn stop_group(pid: Pid) {
    // SAFETY: a group we created in this test.
    assert_eq!(unsafe { libc::killpg(pid, libc::SIGSTOP) }, 0);
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let mut st: libc::c_int = 0;
        // SAFETY: a non-blocking wait for a stop notification from our child.
        // WUNTRACED reports the stop and leaves any later exit status alone, so
        // the `try_wait` above still sees the death when it comes.
        let r = unsafe { libc::waitpid(pid, &mut st, libc::WNOHANG | libc::WUNTRACED) };
        if r == pid {
            assert!(libc::WIFSTOPPED(st), "the child exited instead of stopping");
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("pid {pid} never stopped");
}

/// Leave nothing running behind a test, whatever it asserted.
fn cleanup(child: &mut Child) {
    let pid = child.id() as Pid;
    // SAFETY: our own child's group.
    unsafe {
        libc::killpg(pid, libc::SIGCONT);
        libc::killpg(pid, libc::SIGKILL);
    }
    let _ = died_within(child, Duration::from_secs(5));
}

/// THE TEST. A crashed supervisor leaves children that are not merely running
/// but STOPPED, and the orphan has to end up DEAD -- not signalled, not asked,
/// dead and waited for. Everything else in this file is about who may be
/// killed; this is the one about whether killing works at all.
#[test]
fn a_stopped_orphan_is_actually_killed() {
    let plat = platform::host();
    let mut child = spawn_sleeper();
    let pid = child.id() as Pid;
    let id = exec_identity(pid, &plat);
    stop_group(pid);

    let out = orphan::reap(&[record(pid, &id, true)], &plat);
    assert!(
        matches!(out[0], Reaped::Killed { pid: p, pgid } if p == pid && pgid == pid),
        "a verified orphan must be killed, got {:?}",
        out[0]
    );
    assert!(
        died_within(&mut child, Duration::from_secs(10)),
        "the stopped orphan is still alive: it was asked to die but never woken"
    );
}

/// THE TEST THAT SAYS WHY IT IS A KILL. A stopped planner that ignores SIGTERM
/// is the shape politeness cannot reach at all: measured on this box, it
/// survives a SIGTERM-only reaper indefinitely and dies twenty milliseconds
/// after a group SIGKILL. Escalation belongs on the live-child path in
/// `exec::run`, where a number is being recorded; here there is nothing to
/// flush and nobody to read it.
#[test]
fn a_stopped_orphan_that_ignores_politeness_is_still_killed() {
    let plat = platform::host();
    let mut child = spawn_with(&[("FAKEFF_IGNORE_TERM", "1")]);
    let pid = child.id() as Pid;
    let id = exec_identity(pid, &plat);
    stop_group(pid);

    let out = orphan::reap(&[record(pid, &id, true)], &plat);
    assert!(matches!(out[0], Reaped::Killed { .. }), "{:?}", out[0]);
    assert!(
        died_within(&mut child, Duration::from_secs(10)),
        "a reaper that only asks leaves this one stopped until the reboot"
    );
}

/// The record says the child was running; the child is in fact stopped. That is
/// what a crash between sending SIGSTOP and committing the row looks like, and
/// it is the commonest shape of the incident. The SIGCONT must not be
/// conditional on a flag that is only sometimes right.
#[test]
fn a_record_that_claims_the_orphan_was_running_still_kills_a_stopped_one() {
    let plat = platform::host();
    let mut child = spawn_sleeper();
    let pid = child.id() as Pid;
    let id = exec_identity(pid, &plat);
    stop_group(pid);

    let out = orphan::reap(&[record(pid, &id, false)], &plat);
    assert!(matches!(out[0], Reaped::Killed { .. }), "{:?}", out[0]);
    assert!(
        died_within(&mut child, Duration::from_secs(10)),
        "the stopped flag is a report, never a decision"
    );
}

/// THE TEST THAT STOPS CRUCIBLE KILLING A STRANGER. The pid is live and it is
/// somebody else's. Nothing may be signalled -- not the pid, and above all not
/// its group, which on a workstation is a whole terminal job.
#[test]
fn a_pid_that_belongs_to_a_stranger_is_reported_and_left_alone() {
    let plat = platform::host();
    let mut child = spawn_sleeper();
    let pid = child.id() as Pid;
    let id = exec_identity(pid, &plat);

    let mut row = record(pid, &id, true);
    row.binary_path = "/opt/ff/bin/ff-from-a-previous-boot".into();

    let out = orphan::reap(&[row], &plat);
    assert!(matches!(out[0], Reaped::Recycled { .. }), "{:?}", out[0]);
    assert!(!out[0].signalled());

    // Give a mistaken kill time to land before believing the process survived.
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        !reaped(&mut child),
        "the reaper killed a process it could not identify"
    );
    cleanup(&mut child);
}

/// The same defence against the other half of the identity: a start time that
/// does not match means the pid was reused, even when the path is ours. This is
/// the likeliest recycle there is -- one `ff` ending, the next beginning.
#[test]
fn a_matching_path_with_the_wrong_start_time_is_still_refused() {
    let plat = platform::host();
    let mut child = spawn_sleeper();
    let pid = child.id() as Pid;
    let id = exec_identity(pid, &plat);

    let mut row = record(pid, &id, true);
    row.proc_start_tvsec = id.start_tvsec - 3600;

    let out = orphan::reap(&[row], &plat);
    assert!(matches!(out[0], Reaped::Recycled { .. }), "{:?}", out[0]);
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        !reaped(&mut child),
        "a start-time mismatch must spare the process"
    );
    cleanup(&mut child);
}

/// A pid that has gone is an ordinary verdict on a recovery path that must not
/// panic: this runs while the operator is already cleaning up after a crash.
#[test]
fn a_pid_that_no_longer_exists_is_vanished() {
    let plat = platform::host();
    let mut child = spawn_sleeper();
    let pid = child.id() as Pid;
    let id = exec_identity(pid, &plat);
    let row = record(pid, &id, true);
    cleanup(&mut child);

    let out = orphan::reap(&[row], &plat);
    assert!(
        matches!(out[0], Reaped::Vanished { pid: p } if p == pid),
        "{:?}",
        out[0]
    );
    assert!(!out[0].signalled());
}

/// The belt-and-braces path: an early return anywhere in the supervisor must
/// not leave a planner behind.
#[test]
fn a_group_guard_kills_the_group_when_it_is_dropped() {
    let plat = platform::host();
    let mut child = spawn_sleeper();
    let pid = child.id() as Pid;
    exec_identity(pid, &plat);
    {
        let _g = GroupGuard::new(pid);
    }
    assert!(
        died_within(&mut child, Duration::from_secs(10)),
        "the guard did not kill its group on drop"
    );
}

/// And the other half: a guard that killed after being disarmed would kill
/// every child that exited cleanly, so nobody could afford to arm one.
#[test]
fn a_disarmed_group_guard_leaves_the_group_alone() {
    let plat = platform::host();
    let mut child = spawn_sleeper();
    let pid = child.id() as Pid;
    exec_identity(pid, &plat);
    {
        let mut g = GroupGuard::new(pid);
        g.disarm();
        assert!(!g.armed());
    }
    std::thread::sleep(Duration::from_millis(300));
    assert!(!reaped(&mut child), "a disarmed guard must signal nothing");
    cleanup(&mut child);
}

/// A stopped child is exactly the case the guard shares with the reaper: it
/// must be woken, or the drop leaves it stopped forever.
#[test]
fn a_group_guard_kills_a_group_it_finds_stopped() {
    let plat = platform::host();
    let mut child = spawn_sleeper();
    let pid = child.id() as Pid;
    exec_identity(pid, &plat);
    stop_group(pid);
    {
        let _g = GroupGuard::new(pid);
    }
    assert!(
        died_within(&mut child, Duration::from_secs(10)),
        "the guard left a stopped group behind"
    );
}
