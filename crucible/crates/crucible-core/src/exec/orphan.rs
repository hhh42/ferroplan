//! Killing what a dead supervisor left behind, without killing a stranger.
//!
//! A `kill -9` of the harness -- or a panic, or a lid closed on a wedged run --
//! leaves planners behind. The dangerous ones are not the planners still
//! running: they are the ones the throttle had `SIGSTOP`ped when the supervisor
//! died. A stopped orphan sits on its resident pages -- a ballooned planner's
//! worth of them -- for as long as the box stays up, and nothing will ever
//! wake it. That is the case §14 names first: "Parent dies with children
//! `SIGSTOP`'d -> orphans stopped forever."
//!
//! So the reaper KILLS. It does not ask. The spec's escalation for a live child
//! (`SIGTERM` -> 5s -> `SIGKILL`, §6.3) is the wrong instrument against a
//! stopped one: a stopped process does not run, so it never runs a `SIGTERM`
//! handler, and a planner that installs one to flush a partial plan simply
//! stays stopped while the grace period elapses. Measured on this box: a
//! stopped child that ignores `SIGTERM` survives a polite reaper indefinitely,
//! and dies twenty milliseconds after a group `SIGKILL`.
//!
//! The `SIGCONT` that precedes the kill is deliberate and it is NOT what makes
//! the kill land -- worth writing down, because it invites exactly the wrong
//! edit in both directions. Darwin wakes a stopped task to deliver a `SIGKILL`,
//! and so does Linux, so `SIGKILL` alone would do here. It is sent anyway
//! because nothing outside those two kernels promises as much, because it costs
//! one syscall, and because it makes this path identical to `exec::run`'s
//! cancellation path, which wakes the child before signalling it for the reason
//! above. Deleting it saves nothing; relying on it to make politeness work
//! against a planner with a handler is the mistake it is here to prevent.
//!
//! The `SIGCONT` is also unconditional: it does NOT consult the recorded
//! `stopped` flag. The window this whole module exists for is the one between
//! sending a `SIGSTOP` and committing the row that says we sent it, so a record
//! reading `stopped: false` is exactly what a crash mid-suspension looks like.
//! A flag that is only sometimes right is not allowed to decide anything.
//!
//! **A CORRECTION TO THE SPEC, and the reason this is more than one function.**
//! §6.3 and §14 both say to reap "from recorded pids". PIDs RECYCLE -- on macOS
//! they wrap below 100000, so a workstation that has been up for a week has
//! handed our old numbers out many times over. `killpg` on a recycled group
//! does not kill our orphan; it kills whatever now holds that number, which on
//! a personal machine is as likely to be the user's editor or their shell's
//! job as anything else. A reaper that trusts a stored pid is a reaper that
//! destroys unrelated work on startup, and it would do it precisely when the
//! operator is already recovering from a crash.
//!
//! The defence is to verify before signalling. `Platform::proc_identity` reads
//! the executable path and the kernel's own start time for a live pid; the pair
//! is stable for the life of a process and cannot survive a recycle. Unless the
//! live identity matches the record exactly, and the process is still in the
//! group we recorded, nothing is signalled at all -- the row is closed with a
//! verdict saying why. Refusing to kill is always safe here: the worst case is
//! one orphan left for the operator, against a worst case on the other side of
//! deleting somebody's afternoon.
//!
//! Two caveats worth stating in the file rather than in a commit message:
//!
//! * **Across a reboot the identity check is not enough.** `proc_start_tvsec`
//!   is an epoch second on Darwin but ticks-since-boot on Linux, and pids start
//!   again from the bottom either way, so records written before a reboot can
//!   in principle match a stranger. The caller must not hand `reap` rows from a
//!   previous boot; the live-children table is scoped to a boot, not to a
//!   database.
//! * **[`GroupGuard`] and [`install_panic_reaper`] are belt and braces.** They
//!   close the ordinary exits -- an early return, a `?`, a panic in the
//!   scheduler -- so that the common case never reaches this module at all. The
//!   real mechanism is the persisted record plus a [`reap`] at startup, because
//!   a `SIGKILL` of the supervisor runs neither a destructor nor a panic hook,
//!   and that is the failure this is for.

use crate::platform::{Pid, Platform, ProcIdentity};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// One row of the live-children table: enough to decide, on a later startup,
/// whether the pid we wrote down is still the process we wrote it for.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveChild {
    pub pid: Pid,
    /// The group to signal. The spawner's `setpgid(0, 0)` makes this the pid,
    /// and a failure there fails the spawn, so `pgid == pid` is an invariant of
    /// every record we write -- and it is what makes the group verifiable at
    /// all, since a group whose leader we have identified cannot have been
    /// recycled out from under us.
    pub pgid: Pid,
    /// The run this child was measuring, so a reap can close the row honestly
    /// instead of leaving a run that never ended.
    pub run_id: Option<i64>,
    /// Half of the identity. Compared, never interpreted.
    pub binary_path: String,
    /// The other half: the kernel's start time for the pid. Units are whatever
    /// [`ProcIdentity`] carries on this platform and are only ever compared
    /// with a value read the same way on the same boot.
    pub proc_start_tvsec: i64,
    /// Wall-clock epoch seconds, for the operator's message. Deliberately NOT
    /// part of the identity check: the wall clock moves, and a reaper that
    /// refused to act because the machine had resynced its time would leave the
    /// orphans it exists to kill.
    pub spawned_at: f64,
    /// What we believed at the last commit. Reported, never trusted -- see the
    /// module header on why the `SIGCONT` is unconditional.
    pub stopped: bool,
}

impl LiveChild {
    /// The record to persist for a child that has just been spawned.
    ///
    /// `pgid` is the pid by construction rather than by lookup: the spawn hook
    /// has already made the child its own group leader, so asking the kernel
    /// again would only add a syscall that can race the child's exit.
    pub fn record(pid: Pid, run_id: Option<i64>, id: &ProcIdentity, spawned_at: f64) -> Self {
        Self {
            pid,
            pgid: pid,
            run_id,
            binary_path: id.path.clone(),
            proc_start_tvsec: id.start_tvsec,
            spawned_at,
            stopped: false,
        }
    }
}

/// What a single recorded child turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reaped {
    /// Verified as ours and killed: `SIGCONT` then `SIGKILL`, to the group.
    Killed { pid: Pid, pgid: Pid },
    /// The pid no longer names a process. Nothing was signalled, and nothing
    /// needed to be.
    Vanished { pid: Pid },
    /// The pid names a process that is not ours. NOTHING WAS SIGNALLED. The
    /// strings are diagnostic, not machine-readable -- they exist so the
    /// operator can see which stranger was spared.
    Recycled {
        pid: Pid,
        expected: String,
        found: String,
    },
}

impl Reaped {
    pub fn pid(&self) -> Pid {
        match self {
            Reaped::Killed { pid, .. }
            | Reaped::Vanished { pid }
            | Reaped::Recycled { pid, .. } => *pid,
        }
    }

    /// Whether this verdict sent any signal at all. Two of the three do not.
    pub fn signalled(&self) -> bool {
        matches!(self, Reaped::Killed { .. })
    }
}

/// Render an identity for the operator.
///
/// The start time is in the string on purpose. The commonest recycle on a
/// workstation is the same binary launched again -- one `ff` finishing and the
/// next starting under the old pid -- and a `Recycled` verdict that printed two
/// identical paths would read as a bug in the reaper rather than as a stranger
/// being spared.
fn ident(path: &str, start: i64, pgid: Pid) -> String {
    format!("{path}@{start} pg={pgid}")
}

/// Our own process group, which must never be a target.
fn own_pgid() -> Pid {
    // SAFETY: getpgrp takes no arguments and cannot fail.
    unsafe { libc::getpgrp() }
}

/// The group a live pid is in right now, or None if it has gone.
fn live_pgid(pid: Pid) -> Option<Pid> {
    // SAFETY: a read-only query about a pid we are about to decide on.
    let pg = unsafe { libc::getpgid(pid) };
    if pg < 0 {
        None
    } else {
        Some(pg)
    }
}

/// Groups we refuse to signal under any circumstances.
///
/// `killpg(0, ...)` means "my own group" -- under `cargo test`, or under a
/// shell, that is the whole foreground job, so a stray zero in a restored
/// database would take out the harness and everything sharing its terminal.
/// Group 1 is `launchd`/`init`. Neither can ever be a planner we spawned.
fn signallable(pgid: Pid) -> bool {
    pgid > 1 && pgid != own_pgid()
}

/// Wake the group, then kill it.
///
/// `SIGKILL`, never `SIGTERM`: see the module header on why politeness cannot
/// reach a stopped planner. Nothing in the group has a result anyone will read,
/// so there is nothing to flush and no reason to wait. The `SIGCONT` is the
/// belt-and-braces half, kept in step with `exec::run`'s cancellation path.
fn cont_then_kill(pgid: Pid) {
    // Silent rather than asserted: this also runs from `Drop`, and a panic
    // there during an unwind aborts the process -- which would turn a defence
    // against stranded children into a way to strand them.
    if !signallable(pgid) {
        return;
    }
    // SAFETY: two signals to a process group this process created -- verified
    // by identity on the `reap` path, still unwaited-for on the guard path.
    unsafe {
        libc::killpg(pgid, libc::SIGCONT);
        libc::killpg(pgid, libc::SIGKILL);
    }
}

/// Kill every recorded child that is still, verifiably, the child we recorded.
///
/// Returns one verdict per input, in the same order, so a caller can zip the
/// results back onto the rows it read and close each one.
///
/// This does not wait for the kills to land. The orphans belong to `launchd`
/// now, not to us, so there is nobody to `waitpid` on -- and a startup path
/// that blocked on a planner wedged in an uninterruptible disk wait would hang
/// the recovery it is part of.
pub fn reap<P: Platform>(children: &[LiveChild], plat: &P) -> Vec<Reaped> {
    children.iter().map(|c| reap_one(c, plat)).collect()
}

fn reap_one<P: Platform>(c: &LiveChild, plat: &P) -> Reaped {
    let expected = ident(&c.binary_path, c.proc_start_tvsec, c.pgid);

    // Gone is the ordinary case: the sweep died, the planners noticed their
    // parent go and exited, or the operator cleaned up by hand.
    let Some(live) = plat.proc_identity(c.pid) else {
        return Reaped::Vanished { pid: c.pid };
    };

    // THE CHECK THAT STOPS US KILLING A STRANGER. Path and start time both, and
    // both exact: a path match alone is satisfied by the very next `ff` to
    // start, which is the likeliest recycle there is.
    if live.path != c.binary_path || live.start_tvsec != c.proc_start_tvsec {
        return Reaped::Recycled {
            pid: c.pid,
            expected,
            found: ident(
                &live.path,
                live.start_tvsec,
                live_pgid(c.pid).unwrap_or_default(),
            ),
        };
    }

    // The identity is the LEADER's. Confirming that the leader is still in the
    // group we are about to signal is what extends the verification from one
    // pid to the whole group; without it a stale `pgid` column would aim a
    // group kill at whatever inherited that number.
    let Some(pg) = live_pgid(c.pid) else {
        // It exited between the identity read and this one. Nothing to do, and
        // nothing was signalled.
        return Reaped::Vanished { pid: c.pid };
    };
    if pg != c.pgid {
        return Reaped::Recycled {
            pid: c.pid,
            expected,
            found: ident(&live.path, live.start_tvsec, pg),
        };
    }
    if !signallable(pg) {
        // Only reachable from a record we did not write. Refusing is the same
        // decision as for a recycled pid, and for the same reason.
        return Reaped::Recycled {
            pid: c.pid,
            expected,
            found: format!(
                "{} -- group {pg} is the supervisor's own or the init group",
                ident(&live.path, live.start_tvsec, pg)
            ),
        };
    }

    cont_then_kill(pg);
    Reaped::Killed {
        pid: c.pid,
        pgid: pg,
    }
}

/// Process groups an armed [`GroupGuard`] is holding, for the panic hook.
///
/// Every critical section below is a push, a remove, or a take on a
/// `Vec<i32>`. Nothing in one can panic, which is what makes it safe for the
/// panic hook to take this lock: it can never be the lock the panicking thread
/// was holding, and a `Mutex` is not reentrant.
static GROUPS: Mutex<Vec<Pid>> = Mutex::new(Vec::new());
static HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);

fn with_groups<R>(f: impl FnOnce(&mut Vec<Pid>) -> R) -> R {
    // A poisoned registry still holds the pids of live children, and dropping
    // them because some other thread panicked is exactly the outcome this
    // module exists to prevent.
    let mut g = GROUPS.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut g)
}

/// Kills a process group when it goes out of scope, unless disarmed.
///
/// This is for the ordinary exits -- an early return, a `?` on a database
/// write, an unwinding panic. It carries no identity check and needs none: it
/// is sound only while the child has not yet been waited for, and an unwaited
/// child holds its pid even as a zombie, so the number cannot have been reused.
/// **Disarm it before you `waitpid`** (or drop it first). An armed guard that
/// outlives the wait is the recycled-pid hazard [`reap`] exists to defend
/// against, with none of the defences.
#[derive(Debug)]
pub struct GroupGuard {
    pgid: Pid,
    armed: bool,
}

impl GroupGuard {
    /// Arm a guard over a group. Registers it with the panic reaper, so a
    /// panic on another thread reaps this group too.
    pub fn new(pgid: Pid) -> Self {
        with_groups(|g| g.push(pgid));
        Self { pgid, armed: true }
    }

    pub fn pgid(&self) -> Pid {
        self.pgid
    }

    pub fn armed(&self) -> bool {
        self.armed
    }

    /// The child exited on its own terms. Do not kill anything.
    pub fn disarm(&mut self) {
        self.armed = false;
        self.deregister();
    }

    fn deregister(&self) {
        with_groups(|g| {
            if let Some(i) = g.iter().position(|p| *p == self.pgid) {
                // One entry, not every entry: two guards over the same group
                // would otherwise deregister each other.
                g.remove(i);
            }
        });
    }
}

impl Drop for GroupGuard {
    fn drop(&mut self) {
        self.deregister();
        if self.armed {
            cont_then_kill(self.pgid);
        }
    }
}

/// Chain a panic hook that reaps every armed [`GroupGuard`].
///
/// Unwinding drops the guards on the panicking thread's own stack, so this is
/// for the two cases where that is not enough: a panic under
/// `panic = "abort"`, where no destructor runs at all, and a panic on one
/// thread taking down a process whose other threads are supervising children of
/// their own. A scheduler bug must cost the sweep, not leave a suspended
/// planner holding eight gigabytes until the next reboot.
///
/// Idempotent: returns true if this call installed the hook, false if it was
/// already installed. The previous hook is kept and called afterwards, so the
/// panic message and backtrace are unchanged.
pub fn install_panic_reaper() -> bool {
    if HOOK_INSTALLED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return false;
    }
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        reap_registered();
        prev(info);
    }));
    true
}

/// Kill every group an armed guard is holding. Also usable directly from a
/// signal-driven shutdown path.
pub fn reap_registered() {
    let groups = with_groups(std::mem::take);
    for pgid in groups {
        cont_then_kill(pgid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{KeepAwake, MemCap, Topology};

    /// A platform whose only real answer is `proc_identity`, so the decision
    /// table can be driven without spawning anything. The paths that DO signal
    /// are covered by `tests/orphan_reaping.rs` against real processes; the
    /// paths here are the ones where the correct behaviour is to signal
    /// nothing, which is exactly what a unit test can prove.
    struct Stub(Option<ProcIdentity>);

    impl Platform for Stub {
        fn topology(&self) -> Topology {
            Topology {
                p_cores: 1,
                e_cores: 0,
                logical: 1,
                mem_bytes: 0,
            }
        }
        fn probe_mem_cap(&self, _: u64) -> MemCap {
            MemCap::Off
        }
        fn rss_bytes(&self, _: Pid) -> Option<u64> {
            None
        }
        fn cpu_ms(&self, _: Pid) -> Option<u64> {
            None
        }
        fn demote(&self, _: Pid) -> std::io::Result<()> {
            Ok(())
        }
        fn promote(&self, _: Pid) -> std::io::Result<()> {
            Ok(())
        }
        unsafe fn set_self_qos_background(&self) -> i32 {
            0
        }
        fn swap_used_mb(&self) -> Option<f64> {
            None
        }
        fn cpu_speed_limit(&self) -> Option<u32> {
            None
        }
        fn proc_identity(&self, _: Pid) -> Option<ProcIdentity> {
            self.0.clone()
        }
        fn descendants(&self, root: Pid) -> Vec<Pid> {
            vec![root]
        }
        fn keep_awake(&self) -> Option<Box<dyn KeepAwake>> {
            None
        }
    }

    fn row() -> LiveChild {
        LiveChild {
            // A pid far above any the kernel will hand out, so that even a
            // regression that ignored the Stub and signalled for real would hit
            // ESRCH rather than a neighbour.
            pid: 0x7fff_0000,
            pgid: 0x7fff_0000,
            run_id: Some(3),
            binary_path: "/opt/ff/bin/ff".into(),
            proc_start_tvsec: 1_700_000_000,
            spawned_at: 1_700_000_000.5,
            stopped: true,
        }
    }

    /// The ordinary case, and the one that must never panic: the sweep died,
    /// the planner went with it, and startup finds a pid that is simply gone.
    #[test]
    fn a_pid_that_no_longer_exists_is_vanished() {
        let out = reap(&[row()], &Stub(None));
        assert_eq!(out, vec![Reaped::Vanished { pid: row().pid }]);
        assert!(!out[0].signalled());
    }

    /// The pid was handed to something else. This is the verdict that keeps
    /// crucible from killing the user's editor on startup.
    #[test]
    fn a_pid_now_held_by_another_binary_is_recycled_and_unsignalled() {
        let live = ProcIdentity {
            path: "/Applications/Some.app/Contents/MacOS/Some".into(),
            start_tvsec: 1_700_000_900,
        };
        let out = reap(&[row()], &Stub(Some(live)));
        assert!(matches!(out[0], Reaped::Recycled { .. }), "{:?}", out[0]);
        assert!(!out[0].signalled(), "a recycled pid must be left alone");
    }

    /// The likeliest recycle of all: the SAME binary started again under the
    /// old number. A check on the path alone waves this through and kills a
    /// planner belonging to someone else's run.
    #[test]
    fn the_same_binary_restarted_under_the_old_pid_is_still_recycled() {
        let live = ProcIdentity {
            path: "/opt/ff/bin/ff".into(),
            start_tvsec: 1_700_000_001,
        };
        let out = reap(&[row()], &Stub(Some(live)));
        match &out[0] {
            Reaped::Recycled {
                expected, found, ..
            } => assert_ne!(
                expected, found,
                "the start time has to be visible in the message, or this \
                 verdict reads as a reaper bug"
            ),
            other => panic!("expected Recycled, got {other:?}"),
        }
    }

    /// One verdict per row, in order: the caller closes database rows by
    /// zipping this back onto what it read.
    #[test]
    fn every_row_gets_exactly_one_verdict_in_order() {
        let mut a = row();
        a.pid = 0x7fff_0001;
        let mut b = row();
        b.pid = 0x7fff_0002;
        let out = reap(&[a, b], &Stub(None));
        assert_eq!(
            out.iter().map(|r| r.pid()).collect::<Vec<_>>(),
            vec![0x7fff_0001, 0x7fff_0002]
        );
    }

    /// `killpg(0)` is "my own group": under `cargo test` that is the whole
    /// foreground job. Nothing may ever reach it.
    #[test]
    fn the_groups_we_must_never_signal_are_refused() {
        assert!(!signallable(0), "0 means our own group to killpg");
        assert!(!signallable(1), "1 is launchd/init");
        assert!(!signallable(-1), "-1 means every process we may signal");
        assert!(!signallable(own_pgid()));
        assert!(signallable(0x7fff_0000));
    }

    /// A disarmed guard leaves nothing behind for the panic hook to find,
    /// which is what makes "disarmed on clean exit" mean disarmed everywhere.
    #[test]
    fn disarming_a_guard_deregisters_it() {
        // Unsignallable by construction: if the arming logic regressed, this
        // test would hit ESRCH, not a neighbour's process group.
        let pgid = 0x7ffe_0000;
        let mut g = GroupGuard::new(pgid);
        assert!(with_groups(|v| v.contains(&pgid)));
        g.disarm();
        assert!(!g.armed());
        assert!(!with_groups(|v| v.contains(&pgid)));
        drop(g);
        assert!(!with_groups(|v| v.contains(&pgid)));
    }

    /// Installing twice would drop the first hook's chain -- and with it
    /// whatever the host installed to print the panic.
    #[test]
    fn the_panic_reaper_installs_exactly_once() {
        let first = install_panic_reaper();
        let second = install_panic_reaper();
        assert!(!second, "the second install must be a no-op");
        if first {
            // Only meaningful when this test won the race to install it; the
            // point is the second call, either way.
            assert!(HOOK_INSTALLED.load(Ordering::SeqCst));
        }
    }
}
