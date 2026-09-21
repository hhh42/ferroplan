//! MEMORY IS A WALL TOO (0.28 Lane M).
//!
//! The engine has always had a memory MODEL -- bytes per node times nodes,
//! against 60 % of whatever budget the environment declares
//! (`search::rlimit_budget`). It has never had a memory MEASUREMENT, and the
//! model is the thing this project keeps recording as wrong for exactly the
//! classes that matter: `storage-preferences-complex` growing to 528 GB of
//! address space under a declared 6 GB, the `mem-cap` rows that die 3-10 s
//! into a 60 s wall with the runner's RSS watchdog, never the engine, doing
//! the killing.
//!
//! What made that urgent is the rest of 0.28: those lanes put a valid plan in
//! hand EARLY and then go on working -- a quality chase, a bigger task to
//! ground so the plan can be priced and improved. The first board read through
//! the crucible (6 GB a run, where the hand-rolled sit had allowed 8) lost
//! fifteen rows the sit had solved, every one `mem-cap`, every one with its
//! plan already found: the watchdog killed the process in the middle of the
//! OPTIONAL work and the plan went with it. Dying of memory with a plan in
//! hand is the same sin as dying of the wall with one, and it gets the same
//! cure: the budget is read at the checkpoints that already exist, a trip is
//! an honest capped return, and whatever was banked is what comes back.
//!
//! BOUNDED WORK ONLY ([`MemWall::arm`]). A first search has nothing to fall
//! back on, and a wall that stopped it at 4.5 GB of a 6 GB budget would take
//! the solve it was 1 GB from: over the record, one run in ten peaks above
//! 4.4 GB. So the wall arms only inside a `search::ScopedDeadline` -- the
//! quality chase, the grounding that prices a plan already found, a rung's
//! bet -- where a trip costs an improvement and never a row.
//!
//! RESIDENT bytes, because that is what the runner's watchdog measures
//! (Darwin refuses every `setrlimit(RLIMIT_AS)`, so resident is the only
//! budget it can enforce). Read from the kernel directly: `/proc/self/status`
//! on Linux, `task_info` on macOS -- declared by hand, because this crate
//! takes no `libc` dependency and `std` already links the symbols. Anywhere
//! else (wasm included) there is no reading and no trip, and the engine is
//! exactly what it was.

/// The share of the declared budget at which bounded work stops. The runner
/// kills AT the budget, reads are a checkpoint cadence apart, and a wide task
/// allocates a great deal between two of them: at 0.85 a snap-compiled
/// pipesworld grounding tripped at 5.1 GB and still peaked at 5.97 of 6.
const TRIP_FRAC: f64 = 0.75;

/// This process's CURRENT resident set, in bytes. `None` where the kernel
/// cannot be asked.
pub fn resident_bytes() -> Option<u64> {
    imp::resident_bytes()
}

/// This process's PEAK resident set, in bytes -- what a runner's watchdog
/// would have seen at its worst. For receipts and tests; the wall itself reads
/// [`resident_bytes`].
pub fn peak_resident_bytes() -> Option<u64> {
    imp::peak_resident_bytes()
}

/// The whole-process byte budget the environment declares
/// (`FF_MEM_BUDGET_GB`, fractional GiB -- what the runner's watchdog will
/// enforce). `None` = undeclared = no memory wall, as before 0.28.
pub fn declared_budget_bytes() -> Option<u64> {
    let gb: f64 = std::env::var("FF_MEM_BUDGET_GB")
        .ok()?
        .trim()
        .parse()
        .ok()?;
    (gb.is_finite() && gb > 0.0).then_some((gb * (1u64 << 30) as f64) as u64)
}

/// A cached budget for a hot loop: read the environment once, then each
/// check is one kernel read.
#[derive(Clone, Copy, Debug)]
pub struct MemWall {
    trip_at: Option<u64>,
}

impl MemWall {
    /// Armed iff a budget is declared AND the calling thread is inside
    /// BOUNDED work (`search::bounded_work`): a chase over a banked plan, the
    /// grounding that only prices a plan already found, a rung's bet. A first
    /// search -- one with nothing to fall back on -- is never cut by this
    /// wall: if it would have solved at 5.5 GB of 6, it still does.
    /// `FF_NO_MEM_WALL=1` is the 0.27 restore. Call on the thread that
    /// entered the solve (the marker is thread-local); the wall itself is
    /// `Copy` and travels to workers.
    pub fn arm() -> Self {
        let trip_at = if std::env::var("FF_NO_MEM_WALL").is_ok() || !crate::search::bounded_work() {
            None
        } else {
            declared_budget_bytes().map(|b| (b as f64 * TRIP_FRAC) as u64)
        };
        MemWall { trip_at }
    }

    /// Never trips: for entries that must run whatever they cost (the
    /// validator's grounding of a plan already found).
    pub fn unarmed() -> Self {
        MemWall { trip_at: None }
    }

    pub fn armed(&self) -> bool {
        self.trip_at.is_some()
    }

    /// Has the resident set crossed the trip line? Unarmed, or unreadable,
    /// is always `false`.
    #[inline]
    pub fn hit(&self) -> bool {
        match self.trip_at {
            Some(line) => resident_bytes().is_some_and(|r| r >= line),
            None => false,
        }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    pub fn resident_bytes() -> Option<u64> {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let line = status.lines().find(|l| l.starts_with("VmRSS:"))?;
        let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
        Some(kb * 1024)
    }

    pub fn peak_resident_bytes() -> Option<u64> {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let line = status.lines().find(|l| l.starts_with("VmHWM:"))?;
        let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
        Some(kb * 1024)
    }
}

#[cfg(target_os = "macos")]
mod imp {
    // <mach/task_info.h>, `mach_task_basic_info`: three 64-bit sizes, two
    // (seconds, microseconds) pairs, policy, suspend count -- 48 bytes, and
    // the same layout under the header's `#pragma pack(4)` as under repr(C).
    #[repr(C)]
    struct MachTaskBasicInfo {
        virtual_size: u64,
        resident_size: u64,
        resident_size_max: u64,
        user_time: [i32; 2],
        system_time: [i32; 2],
        policy: i32,
        suspend_count: i32,
    }
    const MACH_TASK_BASIC_INFO: u32 = 20;
    extern "C" {
        static mach_task_self_: u32;
        fn task_info(task: u32, flavor: u32, info: *mut i32, count: *mut u32) -> i32;
    }

    pub fn resident_bytes() -> Option<u64> {
        info().map(|i| i.resident_size)
    }

    pub fn peak_resident_bytes() -> Option<u64> {
        info().map(|i| i.resident_size_max)
    }

    fn info() -> Option<MachTaskBasicInfo> {
        let mut info = std::mem::MaybeUninit::<MachTaskBasicInfo>::zeroed();
        let mut count =
            (std::mem::size_of::<MachTaskBasicInfo>() / std::mem::size_of::<i32>()) as u32;
        // SAFETY: `task_info` writes at most `count` 32-bit words into `info`,
        // which is exactly that large and zero-initialised; the task port is
        // this process's own, read from the static libSystem exports.
        let kr = unsafe {
            task_info(
                mach_task_self_,
                MACH_TASK_BASIC_INFO,
                info.as_mut_ptr().cast(),
                &mut count,
            )
        };
        // SAFETY: zeroed is a valid bit pattern for this all-integer struct,
        // whether or not the call filled it.
        (kr == 0).then(|| unsafe { info.assume_init() })
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn resident_bytes() -> Option<u64> {
        None
    }

    pub fn peak_resident_bytes() -> Option<u64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn the_kernel_answers_and_the_answer_moves_with_the_heap() {
        let before = resident_bytes().expect("this platform can be asked");
        assert!(
            before > 1 << 20,
            "a test binary is more than a megabyte: {before}"
        );
        // Touch every page, or the allocation is address space and not memory.
        let block: Vec<u8> = (0..64usize << 20).map(|i| (i % 251) as u8).collect();
        let during = resident_bytes().unwrap();
        assert!(
            during >= before + (48 << 20),
            "64 MiB touched must show as resident: {before} -> {during}"
        );
        assert_eq!(block[12345], (12345 % 251) as u8);
    }

    #[test]
    fn an_undeclared_budget_is_no_wall() {
        // (`FF_MEM_BUDGET_GB` is unset under `cargo test`.)
        if std::env::var("FF_MEM_BUDGET_GB").is_err() {
            assert!(!MemWall::arm().hit());
        }
    }
}
