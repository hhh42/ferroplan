//! Where the sweep's narration goes.
//!
//! Headless, `say!` is `println!`. Under the dashboard the alternate screen
//! owns stdout, so the same lines land in a ring the UI thread renders as the
//! log -- and are flushed to stdout when the screen is given back, so a
//! transcript still ends with the same text a headless run prints.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static QUIET: AtomicBool = AtomicBool::new(false);
static RING: Mutex<VecDeque<(String, String)>> = Mutex::new(VecDeque::new());
const KEEP: usize = 2000;

/// Route `say!` into the ring instead of stdout.
pub fn quiet(on: bool) {
    QUIET.store(on, Ordering::Relaxed);
}

pub fn is_quiet() -> bool {
    QUIET.load(Ordering::Relaxed)
}

fn stamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Local time without a dependency: the offset the platform reports.
    let local = now as i64 + crate::out::tz_offset_secs();
    let s = local.rem_euclid(86_400);
    format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}

/// The local UTC offset, in seconds, via `localtime_r`.
pub fn tz_offset_secs() -> i64 {
    let t: libc::time_t = 0;
    // SAFETY: a zeroed tm is a valid out-buffer for localtime_r.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: localtime_r with valid pointers (it consults TZ itself).
    unsafe {
        libc::localtime_r(&t, &mut tm);
    }
    tm.tm_gmtoff
}

pub fn say(line: String) {
    if is_quiet() {
        let mut r = RING.lock().unwrap();
        r.push_back((stamp(), line));
        while r.len() > KEEP {
            r.pop_front();
        }
    } else {
        println!("{line}");
    }
}

/// The last `n` lines, oldest first.
pub fn recent(n: usize) -> Vec<(String, String)> {
    let r = RING.lock().unwrap();
    r.iter().rev().take(n).rev().cloned().collect()
}

/// Print everything the ring holds and empty it -- when the dashboard exits.
pub fn flush_to_stdout() {
    let mut r = RING.lock().unwrap();
    for (at, line) in r.drain(..) {
        println!("{at}  {line}");
    }
}

#[macro_export]
macro_rules! say {
    () => {
        $crate::out::say(String::new())
    };
    ($($arg:tt)*) => {
        $crate::out::say(format!($($arg)*))
    };
}
