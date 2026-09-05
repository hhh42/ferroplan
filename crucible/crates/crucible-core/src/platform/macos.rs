//! The Darwin implementation. Everything here is `#[cfg(target_os = "macos")]`.

use super::{KeepAwake, MemCap, Pid, Platform, ProcIdentity, Topology};
use std::io;
use std::sync::OnceLock;

/// `setpriority(PRIO_DARWIN_PROCESS, pid, PRIO_DARWIN_BG)` -- the only way to
/// move an already-running process into the background band. Not in the `libc`
/// crate's constant set on every version, so they are named here.
const PRIO_DARWIN_PROCESS: libc::c_int = 4;
const PRIO_DARWIN_BG: libc::c_int = 0x1000;

/// Darwin QoS classes, from `sys/qos.h`.
const QOS_CLASS_BACKGROUND: libc::c_uint = 0x09;

extern "C" {
    fn pthread_set_qos_class_self_np(qos: libc::c_uint, relpri: libc::c_int) -> libc::c_int;
}

#[derive(Default)]
pub struct MacOs;

/// Mach absolute-time units to nanoseconds, via the timebase the kernel
/// publishes. Queried once; (1, 1) if the call fails, which is the Intel
/// value and therefore the only safe default.
pub fn mach_ticks_to_ns(ticks: u64) -> u64 {
    // libc marks its binding deprecated in favour of the `mach2` crate; one
    // two-field struct is not worth a dependency, so the binding is declared
    // here. <mach/mach_time.h>: kern_return_t mach_timebase_info(mach_timebase_info_t).
    #[repr(C)]
    struct MachTimebaseInfo {
        numer: u32,
        denom: u32,
    }
    extern "C" {
        fn mach_timebase_info(info: *mut MachTimebaseInfo) -> libc::c_int;
    }
    static TIMEBASE: OnceLock<(u64, u64)> = OnceLock::new();
    let (numer, denom) = *TIMEBASE.get_or_init(|| {
        let mut tb = MachTimebaseInfo { numer: 0, denom: 0 };
        // SAFETY: a plain out-pointer call into libSystem.
        let rc = unsafe { mach_timebase_info(&mut tb) };
        if rc != 0 || tb.denom == 0 {
            (1, 1)
        } else {
            (tb.numer as u64, tb.denom as u64)
        }
    });
    ticks.saturating_mul(numer) / denom
}

fn sysctl_u32(name: &str) -> Option<u32> {
    let c = std::ffi::CString::new(name).ok()?;
    let mut out: u32 = 0;
    let mut len = std::mem::size_of::<u32>();
    // SAFETY: `out` and `len` are correctly sized for a u32 sysctl.
    let rc = unsafe {
        libc::sysctlbyname(
            c.as_ptr(),
            &mut out as *mut u32 as *mut libc::c_void,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    (rc == 0).then_some(out)
}

fn sysctl_u64(name: &str) -> Option<u64> {
    let c = std::ffi::CString::new(name).ok()?;
    let mut out: u64 = 0;
    let mut len = std::mem::size_of::<u64>();
    // SAFETY: as above, for a u64 sysctl.
    let rc = unsafe {
        libc::sysctlbyname(
            c.as_ptr(),
            &mut out as *mut u64 as *mut libc::c_void,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    (rc == 0).then_some(out)
}

struct Caffeinate(std::process::Child);
impl KeepAwake for Caffeinate {}
impl Drop for Caffeinate {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Platform for MacOs {
    fn topology(&self) -> Topology {
        static T: OnceLock<Topology> = OnceLock::new();
        *T.get_or_init(|| {
            let logical = sysctl_u32("hw.logicalcpu").unwrap_or(1);
            // An older or non-asymmetric Mac reports no perflevels at all; then
            // every core is a performance core and there is no polite band.
            let p = sysctl_u32("hw.perflevel0.logicalcpu").unwrap_or(logical);
            let e = sysctl_u32("hw.perflevel1.logicalcpu").unwrap_or(0);
            Topology {
                p_cores: p,
                e_cores: e,
                logical,
                mem_bytes: sysctl_u64("hw.memsize").unwrap_or(0),
            }
        })
    }

    fn probe_mem_cap(&self, cap_bytes: u64) -> MemCap {
        static PROBE: OnceLock<bool> = OnceLock::new();
        if cap_bytes == 0 {
            return MemCap::Off;
        }
        let usable = *PROBE.get_or_init(|| {
            let mut cur = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            // SAFETY: getrlimit/setrlimit with a correctly-sized rlimit.
            unsafe {
                if libc::getrlimit(libc::RLIMIT_AS, &mut cur) != 0 {
                    return false;
                }
                let attempt = libc::rlimit {
                    rlim_cur: cap_bytes as libc::rlim_t,
                    rlim_max: cur.rlim_max,
                };
                if libc::setrlimit(libc::RLIMIT_AS, &attempt) != 0 {
                    return false;
                }
                // Put it back. The probe must be side-effect-free -- it runs in
                // the parent, which then goes on to supervise a whole sweep.
                libc::setrlimit(libc::RLIMIT_AS, &cur) == 0
            }
        });
        if usable {
            MemCap::Rlimit(cap_bytes)
        } else {
            MemCap::RssWatchdog(cap_bytes)
        }
    }

    fn rss_bytes(&self, pid: Pid) -> Option<u64> {
        use libproc::libproc::pid_rusage::{pidrusage, RUsageInfoV2};
        pidrusage::<RUsageInfoV2>(pid)
            .ok()
            .map(|r| r.ri_resident_size)
    }

    fn cpu_ms(&self, pid: Pid) -> Option<u64> {
        use libproc::libproc::pid_rusage::{pidrusage, RUsageInfoV2};
        // ri_user_time and ri_system_time are in MACH ABSOLUTE TIME UNITS,
        // not nanoseconds. On Apple Silicon one unit is 125/3 ns, so reading
        // them as nanoseconds -- which this function did through the whole
        // 0.26 cycle -- came out 41.67x low on every row in the database.
        // The receipt is the run table itself: mean cpu/wall 0.023 where the
        // corrected figure is 0.96. `mach_timebase_info` is the conversion
        // the kernel publishes, and it is 1/1 on Intel, so this is right on
        // both.
        let r = pidrusage::<RUsageInfoV2>(pid).ok()?;
        Some(mach_ticks_to_ns(r.ri_user_time.saturating_add(r.ri_system_time)) / 1_000_000)
    }

    fn memory_pressure_level(&self) -> Option<u32> {
        let name = b"kern.memorystatus_vm_pressure_level\0";
        let mut val: libc::c_int = 0;
        let mut len = std::mem::size_of::<libc::c_int>();
        // SAFETY: a plain sysctlbyname read into a correctly sized int.
        let rc = unsafe {
            libc::sysctlbyname(
                name.as_ptr() as *const libc::c_char,
                &mut val as *mut libc::c_int as *mut libc::c_void,
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        if rc == 0 && val >= 0 {
            Some(val as u32)
        } else {
            None
        }
    }

    fn demote(&self, pid: Pid) -> io::Result<()> {
        // SAFETY: a plain setpriority on a pid we own.
        let rc =
            unsafe { libc::setpriority(PRIO_DARWIN_PROCESS, pid as libc::id_t, PRIO_DARWIN_BG) };
        if rc == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn promote(&self, pid: Pid) -> io::Result<()> {
        // SAFETY: as above. 0 clears the background band.
        let rc = unsafe { libc::setpriority(PRIO_DARWIN_PROCESS, pid as libc::id_t, 0) };
        if rc == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    unsafe fn set_self_qos_background(&self) -> i32 {
        pthread_set_qos_class_self_np(QOS_CLASS_BACKGROUND, 0)
    }

    fn swap_used_mb(&self) -> Option<f64> {
        // A swapping box slows search while looking perfectly CPU-idle, which
        // is why the contention record carries this at all.
        let out = std::process::Command::new("sysctl")
            .args(["-n", "vm.swapusage"])
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&out.stdout);
        let used = s.split("used =").nth(1)?.trim_start();
        let num: String = used
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        num.parse().ok()
    }

    fn cpu_speed_limit(&self) -> Option<u32> {
        let out = std::process::Command::new("pmset")
            .args(["-g", "therm"])
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&out.stdout);
        let line = s.lines().find(|l| l.contains("CPU_Speed_Limit"))?;
        line.split('=').nth(1)?.trim().parse().ok()
    }

    fn user_idle_secs(&self) -> Option<f64> {
        // `ioreg -c IOHIDSystem` carries "HIDIdleTime" = <nanoseconds>.
        let out = std::process::Command::new("ioreg")
            .args(["-c", "IOHIDSystem", "-d", "4", "-r"])
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&out.stdout);
        let line = s.lines().find(|l| l.contains("HIDIdleTime"))?;
        let ns: f64 = line.split('=').nth(1)?.trim().parse().ok()?;
        Some(ns / 1e9)
    }

    fn proc_identity(&self, pid: Pid) -> Option<ProcIdentity> {
        use libproc::libproc::{bsd_info::BSDInfo, proc_pid};
        let path = proc_pid::pidpath(pid).ok()?;
        let info = proc_pid::pidinfo::<BSDInfo>(pid, 0).ok()?;
        Some(ProcIdentity {
            path,
            start_tvsec: info.pbi_start_tvsec as i64,
        })
    }

    fn descendants(&self, root: Pid) -> Vec<Pid> {
        // Excluding our own tree by PID rather than by process NAME. The
        // Python matches substrings, which never excluded `Validate` -- so
        // VAL's bursts of a full core were counted as foreign competition on
        // every temporal board.
        use libproc::libproc::{bsd_info::BSDInfo, proc_pid};
        use libproc::processes;
        let Ok(all) = processes::pids_by_type(processes::ProcFilter::All) else {
            return vec![root];
        };
        let mut parent = std::collections::HashMap::new();
        for p in &all {
            let pid = *p as Pid;
            if let Ok(info) = proc_pid::pidinfo::<BSDInfo>(pid, 0) {
                parent.insert(pid, info.pbi_ppid as Pid);
            }
        }
        let mut out = vec![root];
        // Bounded walk: a cycle in the ppid map (which a racing exit can
        // briefly produce) must not hang the monitor.
        for _ in 0..8 {
            let before = out.len();
            for (pid, ppid) in &parent {
                if out.contains(ppid) && !out.contains(pid) {
                    out.push(*pid);
                }
            }
            if out.len() == before {
                break;
            }
        }
        out
    }

    fn keep_awake(&self) -> Option<Box<dyn KeepAwake>> {
        // `-w <pid>` makes it die with us, so a crashed supervisor cannot leave
        // the machine pinned awake forever.
        std::process::Command::new("caffeinate")
            .args(["-i", "-w", &std::process::id().to_string()])
            .spawn()
            .ok()
            .map(|c| Box::new(Caffeinate(c)) as Box<dyn KeepAwake>)
    }
}
