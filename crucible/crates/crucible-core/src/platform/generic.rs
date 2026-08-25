//! The non-Darwin implementation.
//!
//! This exists so `trait Platform` is a real seam rather than a decorative one.
//! `preflight.sh` runs `cargo check --target x86_64-unknown-linux-gnu`; if a
//! macOS-only call escapes the trait, that check fails. It is a working Linux
//! path, not a stub -- `/proc` supplies everything except the thermal reading,
//! and `sched_setscheduler(SCHED_IDLE)` is the politeness lever there.

use super::{KeepAwake, MemCap, Pid, Platform, ProcIdentity, Topology};
use std::io;

#[derive(Default)]
pub struct Generic;

fn proc_stat_field(pid: Pid, idx: usize) -> Option<u64> {
    let s = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The comm field can contain spaces and parentheses, so fields are counted
    // from after the closing paren, not from the start of the line.
    let rest = &s[s.rfind(')')? + 2..];
    rest.split_whitespace().nth(idx)?.parse().ok()
}

impl Platform for Generic {
    fn topology(&self) -> Topology {
        let logical = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1);
        let mem_bytes = std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("MemTotal:"))?
                    .split_whitespace()
                    .nth(1)?
                    .parse::<u64>()
                    .ok()
            })
            .map(|kb| kb * 1024)
            .unwrap_or(0);
        // No asymmetry to detect here; every core is a performance core.
        Topology {
            p_cores: logical,
            e_cores: 0,
            logical,
            mem_bytes,
        }
    }

    fn probe_mem_cap(&self, cap_bytes: u64) -> MemCap {
        if cap_bytes == 0 {
            MemCap::Off
        } else {
            // Linux honours RLIMIT_AS, which is the whole reason the probe is a
            // probe and not a constant.
            MemCap::Rlimit(cap_bytes)
        }
    }

    fn rss_bytes(&self, pid: Pid) -> Option<u64> {
        let s = std::fs::read_to_string(format!("/proc/{pid}/statm")).ok()?;
        let pages: u64 = s.split_whitespace().nth(1)?.parse().ok()?;
        Some(pages * 4096)
    }

    fn cpu_ms(&self, pid: Pid) -> Option<u64> {
        // utime and stime are fields 11 and 12 after the comm field, in clock
        // ticks (100/s on every mainstream configuration).
        let utime = proc_stat_field(pid, 11)?;
        let stime = proc_stat_field(pid, 12)?;
        Some((utime + stime) * 10)
    }

    fn demote(&self, pid: Pid) -> io::Result<()> {
        // SAFETY: setpriority on a pid we own.
        let rc = unsafe { libc::setpriority(libc::PRIO_PROCESS, pid as libc::id_t, 19) };
        if rc == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn promote(&self, pid: Pid) -> io::Result<()> {
        // SAFETY: as above.
        let rc = unsafe { libc::setpriority(libc::PRIO_PROCESS, pid as libc::id_t, 0) };
        if rc == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    unsafe fn set_self_qos_background(&self) -> i32 {
        // No QoS classes here; nice(19) is the closest spawn-time equivalent
        // and is async-signal-safe.
        libc::nice(19)
    }

    fn swap_used_mb(&self) -> Option<f64> {
        let s = std::fs::read_to_string("/proc/meminfo").ok()?;
        let get = |k: &str| -> Option<f64> {
            s.lines()
                .find(|l| l.starts_with(k))?
                .split_whitespace()
                .nth(1)?
                .parse::<f64>()
                .ok()
        };
        Some((get("SwapTotal:")? - get("SwapFree:")?) / 1024.0)
    }

    fn cpu_speed_limit(&self) -> Option<u32> {
        // No portable equivalent of `pmset -g therm`. None is honest; a
        // fabricated 100 would read as "never throttled".
        None
    }

    fn proc_identity(&self, pid: Pid) -> Option<ProcIdentity> {
        let path = std::fs::read_link(format!("/proc/{pid}/exe"))
            .ok()?
            .to_string_lossy()
            .into_owned();
        // Field 20 after comm is starttime, in ticks since boot. Not an epoch
        // second, but it serves the same purpose: it distinguishes a live child
        // from a recycled pid.
        let start = proc_stat_field(pid, 19)? as i64;
        Some(ProcIdentity {
            path,
            start_tvsec: start,
        })
    }

    fn descendants(&self, root: Pid) -> Vec<Pid> {
        let mut parent = std::collections::HashMap::new();
        if let Ok(rd) = std::fs::read_dir("/proc") {
            for e in rd.flatten() {
                if let Ok(pid) = e.file_name().to_string_lossy().parse::<Pid>() {
                    if let Some(ppid) = proc_stat_field(pid, 1) {
                        parent.insert(pid, ppid as Pid);
                    }
                }
            }
        }
        let mut out = vec![root];
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
        None
    }
}
