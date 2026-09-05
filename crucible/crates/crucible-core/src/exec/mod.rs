//! Spawn one planner, watch it, and account for its time honestly.
//!
//! Everything difficult about the runner is here, and every difficulty is
//! recorded in the Python this replaces:
//!
//! * **Draining the pipes.** `Child::try_wait()` in a poll loop WITHOUT reading
//!   stdout deadlocks the moment the child exceeds the 64 KiB pipe buffer,
//!   which `ff --json` does routinely on a long plan. Python's `communicate()`
//!   hides this; a naive port reintroduces it as a hang that only shows up on
//!   the boards with the biggest plans.
//! * **The effective clock.** A suspended run must not time out. Elapsed is
//!   wall minus accumulated suspension, and a machine sleep folds into the same
//!   accumulator -- which is why sleeping mid-run needs no separate machinery.
//! * **Kill semantics.** Deadline and memory-cap kills are an immediate
//!   `SIGKILL` to the process GROUP, matching `ipc67.py`'s bare `proc.kill()`.
//!   Adding the spec's five-second `SIGTERM` grace here would change the `time`
//!   recorded on every timeout row in the corpus. Escalation is reserved for
//!   operator cancellation, where no number is being recorded.
//! * **Groups, never bare pids.** A wedged planner with worker threads, or a
//!   VAL that forked, must not outlive its group.
//! * **Spawn retry.** A memory-bloated PREDECESSOR can make this instance's
//!   `fork()` fail. The 0.16 seq-mco sweep lost floor-tile i7-i12 to exactly
//!   that, logged as engine rejects. One retry after a breather; a second
//!   failure is recorded honestly as `spawn-fail`.

pub mod env;
pub mod orphan;

use crate::platform::{MemCap, Pid, Platform};
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};

/// Set by [`install_interrupt_handler`] on SIGINT/SIGTERM. The supervisor
/// reads it every tick and treats it as an operator cancel of the running
/// child -- SIGTERM, a grace period, SIGKILL, all to the process group --
/// so that stopping crucible never leaves a planner running under pid 1.
/// The 0.26 sweep was stopped with SIGTERM and did exactly that.
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

pub fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::Relaxed)
}

extern "C" fn on_interrupt(_sig: libc::c_int) {
    INTERRUPTED.store(true, Ordering::Relaxed);
}

/// Route SIGINT and SIGTERM to the flag. Idempotent; call once at startup.
pub fn install_interrupt_handler() {
    // SAFETY: installing a handler that does one relaxed atomic store, which
    // is async-signal-safe.
    unsafe {
        libc::signal(
            libc::SIGINT,
            on_interrupt as extern "C" fn(libc::c_int) as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGTERM,
            on_interrupt as extern "C" fn(libc::c_int) as libc::sighandler_t,
        );
    }
}

/// Tests only: pretend the operator pressed ^C, or clear it.
pub fn set_interrupted(on: bool) {
    INTERRUPTED.store(on, Ordering::Relaxed);
}
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

/// How often the supervisor wakes to check budgets and control messages.
const TICK: Duration = Duration::from_millis(250);

/// A monotonic gap this much larger than a tick means the machine slept.
/// Nothing else pauses a running supervisor for whole seconds.
const SLEEP_GAP: Duration = Duration::from_secs(5);

/// Grace before `SIGKILL` on an operator-initiated stop. Deliberately NOT used
/// for deadline or memory kills.
const TERM_GRACE: Duration = Duration::from_secs(5);

/// Wait after a resource-class spawn failure before the single retry.
const SPAWN_BREATHER: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Killed {
    Deadline,
    MemCap,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ctl {
    /// Foreign load is high: stop burning cores, keep the work.
    Stop,
    Cont,
    /// Move into the background scheduling band. The run keeps running.
    Demote,
    Promote,
    /// The operator asked. Polite, then firm.
    Cancel,
}

#[derive(Debug, Clone)]
pub struct RunOutcome {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub term_signal: Option<i32>,
    pub killed: Option<Killed>,
    /// Real elapsed time, suspension included.
    pub wall: Duration,
    /// Wall minus suspension: what the deadline is compared against.
    pub effective: Duration,
    pub suspended: Duration,
    /// User + system CPU time of the child, from `wait4(2)`'s `rusage` at
    /// reap: exact, no polling gap, and in a unit the kernel documents. The
    /// R1 runner polled `Platform::cpu_ms` every tick and kept the last
    /// reading -- short runs recorded whatever the first tick caught (often
    /// nothing), and on Apple Silicon the reading itself was in Mach units
    /// read as nanoseconds, 41.67x low. Every `cpu_ms` written before this
    /// field existed carries that error; `cpu_instrument` says which.
    pub cpu_ms: u64,
    /// `"wait4"`. Rows written by the R1 runner have no instrument stamp and
    /// are treated as CPU-unknown by the referee.
    pub cpu_instrument: &'static str,
    pub peak_rss: u64,
    pub mem_hit: bool,
    pub spawn_attempts: u32,
    /// Non-zero means the monotonic clock jumped: the machine slept, and every
    /// number here is suspect.
    pub clock_jump: Duration,
    pub start_ts: f64,
    pub end_ts: f64,
    /// The child that ran, and its group -- equal by construction, because the
    /// spawn hook makes every child its own group leader.
    pub pid: Pid,
    pub pgid: Pid,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    /// Both attempts failed with a resource-class error. Recorded as
    /// `spawn-fail`, which is environmental and NOT an engine verdict.
    #[error("spawn failed twice: {0}")]
    SpawnFail(std::io::Error),
    /// The binary is missing or not executable. Fatal to the SWEEP -- a missing
    /// `ff` must not quietly produce 6,584 `spawn-fail` rows.
    #[error("planner not runnable at {path}: {source}")]
    NotRunnable {
        path: String,
        source: std::io::Error,
    },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub struct RunRequest<'a> {
    pub program: &'a std::path::Path,
    pub args: &'a [String],
    pub envs: &'a [(std::ffi::OsString, std::ffi::OsString)],
    pub timeout: Duration,
    pub mem_cap: MemCap,
    /// Called once, with the pid and the spawn's epoch timestamp, the moment
    /// the child exists and BEFORE anything is waited on. This is the hook the
    /// `live_child` table hangs off: a record written here survives a
    /// `kill -9` of the supervisor, which is the only reason the table exists.
    pub on_spawn: Option<&'a dyn Fn(Pid, f64)>,
}

fn now_epoch() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| (d.as_secs_f64() * 100.0).round() / 100.0)
        .unwrap_or(0.0)
}

/// EAGAIN/ENOMEM/EMFILE/ENFILE mean the SYSTEM could not fork right now, which
/// is worth one retry. ENOENT means the binary is wrong, which is not.
fn is_resource_error(e: &std::io::Error) -> bool {
    matches!(
        e.raw_os_error(),
        Some(libc::EAGAIN) | Some(libc::ENOMEM) | Some(libc::EMFILE) | Some(libc::ENFILE)
    )
}

fn killpg(pgid: Pid, sig: i32) {
    if pgid > 0 {
        // SAFETY: signalling a process group we created.
        unsafe {
            libc::killpg(pgid, sig);
        }
    }
}

/// Run one planner to completion, a deadline, a memory cap, or a cancellation.
pub fn run<P: Platform>(
    req: &RunRequest<'_>,
    plat: &P,
    ctl: &Receiver<Ctl>,
) -> Result<RunOutcome, ExecError> {
    let mut attempts = 0u32;
    let mut child = loop {
        attempts += 1;
        match spawn(req, plat) {
            Ok(c) => break c,
            Err(e) if is_resource_error(&e) && attempts == 1 => {
                // The retry runs on a recovered system.
                std::thread::sleep(SPAWN_BREATHER);
            }
            Err(e) if is_resource_error(&e) => return Err(ExecError::SpawnFail(e)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(ExecError::NotRunnable {
                    path: req.program.display().to_string(),
                    source: e,
                })
            }
            Err(e) => return Err(ExecError::Io(e)),
        }
    };

    let pid = child.id() as Pid;
    // The child called setpgid(0, 0), so its group id IS its pid.
    let pgid = pid;
    let start = Instant::now();
    let start_ts = now_epoch();
    if let Some(hook) = req.on_spawn {
        hook(pid, start_ts);
    }

    // Reader threads rather than poll(2). Two extra threads per child is
    // nothing against a multi-hour board, and it makes the "never deadlock on
    // a full pipe" property structural instead of dependent on getting partial
    // reads and EOF handling right in the same loop that enforces deadlines.
    let mut out_pipe = child.stdout.take();
    let mut err_pipe = child.stderr.take();
    let out_h = std::thread::spawn(move || {
        let mut s = Vec::new();
        if let Some(p) = out_pipe.as_mut() {
            let _ = p.read_to_end(&mut s);
        }
        s
    });
    let err_h = std::thread::spawn(move || {
        let mut s = Vec::new();
        if let Some(p) = err_pipe.as_mut() {
            let _ = p.read_to_end(&mut s);
        }
        s
    });

    let mut suspended = Duration::ZERO;
    let mut suspended_since: Option<Instant> = None;
    let mut clock_jump = Duration::ZERO;
    let mut peak_rss = 0u64;
    let mut mem_hit = false;
    let mut killed: Option<Killed> = None;
    let mut cancel_sent: Option<Instant> = None;
    let mut last_tick = Instant::now();

    let (status, rusage) = loop {
        if let Some(r) = reap(pid)? {
            break r;
        }

        std::thread::sleep(TICK);
        let tick_now = Instant::now();

        // A monotonic gap far larger than the tick means the machine slept.
        // Treat it as suspension: the run did not get that time, so it must not
        // be charged for it -- and the result is marked dirty by the caller.
        let gap = tick_now.duration_since(last_tick);
        if gap > SLEEP_GAP {
            let skipped = gap - TICK;
            clock_jump += skipped;
            suspended += skipped;
        }
        last_tick = tick_now;

        // Sample only while running: a stopped process's RSS is stale.
        if suspended_since.is_none() {
            if let Some(r) = plat.rss_bytes(pid) {
                peak_rss = peak_rss.max(r);
            }
        }

        if let Some(cap) = req.mem_cap.bytes() {
            if matches!(req.mem_cap, MemCap::RssWatchdog(_)) && peak_rss > cap {
                mem_hit = true;
                killed = Some(Killed::MemCap);
                killpg(pgid, libc::SIGKILL);
            }
        }

        loop {
            match ctl.try_recv() {
                Ok(Ctl::Stop) if suspended_since.is_none() => {
                    killpg(pgid, libc::SIGSTOP);
                    suspended_since = Some(Instant::now());
                }
                Ok(Ctl::Cont) => {
                    if let Some(t) = suspended_since.take() {
                        killpg(pgid, libc::SIGCONT);
                        suspended += t.elapsed();
                    }
                }
                Ok(Ctl::Demote) => {
                    let _ = plat.demote(pid);
                }
                Ok(Ctl::Promote) => {
                    let _ = plat.promote(pid);
                }
                Ok(Ctl::Cancel) => {
                    // A stopped process never processes SIGTERM, so wake it
                    // first or the grace period is spent talking to a corpse.
                    if suspended_since.take().is_some() {
                        killpg(pgid, libc::SIGCONT);
                    }
                    killpg(pgid, libc::SIGTERM);
                    cancel_sent = Some(Instant::now());
                    killed = Some(Killed::Cancelled);
                }
                Ok(Ctl::Stop) => {}
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        // The operator's ^C or a SIGTERM to crucible: cancel the child the
        // same way a Ctl::Cancel would, once.
        if interrupted() && cancel_sent.is_none() {
            if suspended_since.take().is_some() {
                killpg(pgid, libc::SIGCONT);
            }
            killpg(pgid, libc::SIGTERM);
            cancel_sent = Some(Instant::now());
            killed = Some(Killed::Cancelled);
        }

        if let Some(t) = cancel_sent {
            if t.elapsed() >= TERM_GRACE {
                killpg(pgid, libc::SIGKILL);
            }
        }

        // THE EFFECTIVE CLOCK. A suspended run must not time out.
        let live = suspended + suspended_since.map(|t| t.elapsed()).unwrap_or_default();
        let effective = start.elapsed().saturating_sub(live);
        if killed.is_none() && effective >= req.timeout {
            killed = Some(Killed::Deadline);
            // Immediate, and to the group. No grace: a five-second SIGTERM
            // window would move `time` on every timeout row in the corpus.
            killpg(pgid, libc::SIGKILL);
        }
    };

    if let Some(t) = suspended_since.take() {
        suspended += t.elapsed();
    }
    let wall = start.elapsed();
    let effective = wall.saturating_sub(suspended);
    let cpu_ms = timeval_us(&rusage.ru_utime).saturating_add(timeval_us(&rusage.ru_stime)) / 1000;

    let stdout = String::from_utf8_lossy(&out_h.join().unwrap_or_default()).into_owned();
    let stderr = String::from_utf8_lossy(&err_h.join().unwrap_or_default()).into_owned();

    Ok(RunOutcome {
        stdout,
        stderr,
        exit_code: status.code(),
        term_signal: signal_of(&status),
        killed,
        wall,
        effective,
        suspended,
        cpu_ms,
        cpu_instrument: CPU_INSTRUMENT,
        peak_rss,
        mem_hit,
        spawn_attempts: attempts,
        clock_jump,
        start_ts,
        end_ts: now_epoch(),
        pid,
        pgid,
    })
}

/// What `RunOutcome::cpu_instrument` says when the CPU time came from
/// `wait4`. The referee trusts this stamp and no other.
pub const CPU_INSTRUMENT: &str = "wait4";

fn timeval_us(t: &libc::timeval) -> u64 {
    (t.tv_sec.max(0) as u64)
        .saturating_mul(1_000_000)
        .saturating_add(t.tv_usec.max(0) as u64)
}

/// Non-blocking reap of OUR child, with its resource usage. `None` while it
/// is still running (or stopped -- no `WUNTRACED`, so a `SIGSTOP`ped child
/// is simply not reported). After this returns `Some`, the pid is gone and
/// `std::process::Child` must not be waited on again.
fn reap(pid: Pid) -> std::io::Result<Option<(std::process::ExitStatus, libc::rusage)>> {
    use std::os::unix::process::ExitStatusExt;
    let mut status: libc::c_int = 0;
    // SAFETY: a zeroed rusage is a valid rusage; wait4 fills it.
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: waiting on a child this process spawned; both out-pointers are
    // valid for the duration of the call.
    let r = unsafe { libc::wait4(pid, &mut status, libc::WNOHANG, &mut ru) };
    if r == 0 {
        return Ok(None);
    }
    if r < 0 {
        let e = std::io::Error::last_os_error();
        if e.raw_os_error() == Some(libc::EINTR) {
            return Ok(None);
        }
        return Err(e);
    }
    Ok(Some((ExitStatusExt::from_raw(status), ru)))
}

#[cfg(unix)]
fn signal_of(s: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    s.signal()
}

fn spawn<P: Platform>(req: &RunRequest<'_>, plat: &P) -> std::io::Result<std::process::Child> {
    let mut cmd = Command::new(req.program);
    cmd.args(req.args)
        .env_clear()
        .envs(req.envs.iter().cloned())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Only install an RLIMIT_AS hook where the probe said the kernel accepts
    // one. An unusable setrlimit raises INSIDE the fork hook, and the
    // spawn-retry then books every instance as spawn-fail after a breather --
    // a twelve-board sweep producing nothing but garbage rows.
    let rlimit = match req.mem_cap {
        MemCap::Rlimit(b) => Some(b),
        _ => None,
    };
    // Politeness is applied AFTER spawn, via Platform::demote: a Darwin QoS
    // class can only be set on the calling thread, and by the time the
    // scheduler wants to demote, the child has long since exec'd. Nothing here
    // needs the platform, which is just as well -- a reference to it could not
    // cross into the forked child's closure.
    let _ = plat;

    // SAFETY: the hook runs between fork and exec and calls only
    // async-signal-safe functions.
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(move || {
            // Own process group, ALWAYS: the whole kill story depends on
            // killpg, never a bare pid.
            if libc::setpgid(0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if let Some(b) = rlimit {
                let lim = libc::rlimit {
                    rlim_cur: b as libc::rlim_t,
                    rlim_max: b as libc::rlim_t,
                };
                if libc::setrlimit(libc::RLIMIT_AS, &lim) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    cmd.spawn()
}
