//! The one seam between the scheduler and the operating system.
//!
//! macOS has **no thread affinity API**: you cannot pin a process to the
//! performance cores. The only lever is the Darwin scheduling band, so
//! "politeness" here means moving children into the background band (which
//! confines them to the E-cores and throttles their I/O) rather than reducing
//! how many of them run. Linux would use cgroups and `sched_setaffinity`
//! instead, which is why this is a trait and not a module of free functions.
//!
//! A CORRECTION TO THE SPEC, worth stating because it is easy to get wrong for
//! a whole cycle: `crucible-spec.md` §6 says POLITE should "re-set children to
//! `QOS_CLASS_BACKGROUND`". You cannot. `pthread_set_qos_class_self_np` is
//! self-only, and by the time you want to demote, the child has already
//! `exec`'d. Demoting an already-running process is
//! `setpriority(PRIO_DARWIN_PROCESS, pid, PRIO_DARWIN_BG)`. The QoS class is
//! still the right lever at SPAWN time, from inside `pre_exec`, and both are
//! exposed here.

use std::io;

pub type Pid = i32;

/// What the box is made of. Detected at startup, never hard-coded -- the same
/// binary has to be honest on a different machine, and a database carried to
/// one must be flagged rather than silently compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Topology {
    /// Performance cores (`hw.perflevel0.logicalcpu`).
    pub p_cores: u32,
    /// Efficiency cores (`hw.perflevel1.logicalcpu`). The POLITE budget.
    pub e_cores: u32,
    pub logical: u32,
    pub mem_bytes: u64,
}

/// Enough to tell a live child from a recycled pid.
///
/// Reaping by recorded pid alone is dangerous: pids recycle, and `killpg` on a
/// recycled process group kills a stranger's work. The pair
/// (executable path, process start time) is stable and cheap to verify.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcIdentity {
    pub path: String,
    pub start_tvsec: i64,
}

/// Which instrument is enforcing the per-run memory budget.
///
/// Two instruments, one column -- and the board must record which one measured
/// it, because they measure different quantities: `RLIMIT_AS` caps ADDRESS
/// SPACE, the watchdog caps RESIDENT bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemCap {
    Off,
    /// The kernel enforces it; set inside `pre_exec` and nowhere else.
    Rlimit(u64),
    /// We poll and kill. The macOS path, always.
    RssWatchdog(u64),
}

impl MemCap {
    pub fn bytes(self) -> Option<u64> {
        match self {
            MemCap::Off => None,
            MemCap::Rlimit(b) | MemCap::RssWatchdog(b) => Some(b),
        }
    }

    /// The label stored on every run, so a mem-cap row says which instrument
    /// judged it.
    pub fn instrument(self) -> &'static str {
        match self {
            MemCap::Off => "off",
            MemCap::Rlimit(_) => "rlimit-as",
            MemCap::RssWatchdog(_) => "rss-watchdog",
        }
    }
}

/// Something that keeps the machine awake for as long as it is held.
///
/// The shell drivers ran the whole sweep under `caffeinate` -- it is in
/// `contention.py`'s SELF_HINTS list. A three-day sweep that sleeps at hour
/// four is not a sweep.
pub trait KeepAwake: Send {}

pub trait Platform: Send + Sync + 'static {
    fn topology(&self) -> Topology;

    /// Probe ONCE whether the kernel will let us lower `RLIMIT_AS`.
    ///
    /// This must be side-effect-free and must never be assumed. macOS reports
    /// `RLIMIT_AS` as INFINITY and then rejects every `setrlimit` on it with
    /// EINVAL. Raised inside `pre_exec` that becomes a spawn failure, the
    /// runner's spawn-retry books EVERY instance as `spawn-fail` after a five
    /// second breather, and a full twelve-board sweep burns hours to produce
    /// nothing but garbage rows. That happened.
    fn probe_mem_cap(&self, cap_bytes: u64) -> MemCap;

    /// Resident bytes for a live pid, or None if it has already gone.
    ///
    /// Must not fork. `ipc67.py` shells out to `ps -o rss=` four times a second
    /// per job, which is why `contention.py`'s own self-filter has to list
    /// `"ps"` -- the watchdog was appearing in its own competitor table.
    fn rss_bytes(&self, pid: Pid) -> Option<u64>;

    /// Accumulated user+system CPU time. The IPC-comparable clock: a run
    /// demoted to the E-cores burns wall time it did not spend computing.
    fn cpu_ms(&self, pid: Pid) -> Option<u64>;

    /// Move a RUNNING process into the background scheduling band.
    fn demote(&self, pid: Pid) -> io::Result<()>;
    fn promote(&self, pid: Pid) -> io::Result<()>;

    /// Called INSIDE `pre_exec`, so it must be async-signal-safe: no
    /// allocation, no locks, no arbitrary library calls.
    ///
    /// # Safety
    /// Runs in the forked child between `fork` and `exec`.
    unsafe fn set_self_qos_background(&self) -> i32;

    fn swap_used_mb(&self) -> Option<f64>;

    /// Non-zero means the kernel reported a thermal or performance warning. On
    /// a fanless chassis a long sweep is exactly when that shows up.
    fn cpu_speed_limit(&self) -> Option<u32>;

    /// The kernel's own memory-pressure verdict: 1 normal, 2 warn, 4
    /// critical on Darwin (`kern.memorystatus_vm_pressure_level`). `None`
    /// where the platform has no such reading. This is a LEVEL, and it is
    /// what the throttle suspends on -- swap in use is a stock that never
    /// comes back down once idle pages have been paged out, and a throttle
    /// keyed on it sat SUSPENDED forever the first evening R2 ran.
    fn memory_pressure_level(&self) -> Option<u32> {
        None
    }

    /// Seconds since the operator last touched the keyboard or mouse
    /// (`HIDIdleTime` on Darwin). `None` where unknown. The width policy
    /// reads it: a box in use by day gets the P-cores; an idle one, or the
    /// night, gets everything.
    fn user_idle_secs(&self) -> Option<f64> {
        None
    }

    fn proc_identity(&self, pid: Pid) -> Option<ProcIdentity>;

    /// Every descendant of `root`, for excluding our own tree from the
    /// competitor tally by PID rather than by process name.
    fn descendants(&self, root: Pid) -> Vec<Pid>;

    fn keep_awake(&self) -> Option<Box<dyn KeepAwake>>;
}

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::MacOs as Host;

#[cfg(not(target_os = "macos"))]
mod generic;
#[cfg(not(target_os = "macos"))]
pub use generic::Generic as Host;

pub fn host() -> Host {
    Host
}
