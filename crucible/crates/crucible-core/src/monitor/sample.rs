//! What ELSE the machine was doing while a board was measured.
//!
//! This box is not a benchmark rig; it is somebody's laptop. A board measured
//! against a browser, a Spotlight reindex or a `cargo build` in another
//! worktree is not a slow board, it is a WRONG board -- and the failure is
//! asymmetric, because contention only ever DEPRESSES coverage. It manufactures
//! regressions and hides gains, which is the expensive direction to be wrong in
//! when the output is a release record.
//!
//! THE VERDICT IS NAMED-COMPETITOR LOAD, NOT IDLE. This changed at 0.24 and the
//! reason must not be lost: `idle_pct` is whole-machine and includes the
//! board's OWN threads, so a `--threads 8` mco board burns 40-80% of this
//! ten-core box BY DESIGN and could never clear a fixed idle floor even in an
//! empty room (measured: mco-t8 read 38-40% idle against 4-5% of real competing
//! load). `competitors_total` excludes our own tree, so it is the actual "who
//! else is on this box" signal.
//!
//! The 25% line was calibrated against a very specific filter, so the filter is
//! reproduced exactly: `ps -Ao pcpu,comm -r`, the first 13 rows only, anything
//! below 5.0 pcpu dropped, names collapsed to a basename with a trailing
//! `.app`/`.xpc` stripped and truncated to 44 characters, summed per name, stop
//! after 12 distinct names, keep the top 4. A port that sums every process
//! instead produces systematically higher totals and starts failing boards that
//! used to pass.

use std::collections::BTreeMap;

/// The clean line, in the same currency as the whole-run verdict.
///
/// `contention.py` and `ipc67.py` each define this and each carries a comment
/// saying it is kept in one place so the two rules cannot drift apart silently.
/// Here it genuinely is one place: the throttle state machine and the resume
/// gate read this same constant.
pub const SAMPLE_CLEAN_PCPU: f64 = 25.0;

const TOP_N: usize = 4;
const PS_ROWS: usize = 13;
const MIN_PCPU: f64 = 5.0;
const NAME_MAX: usize = 44;

#[derive(Debug, Clone, Default)]
pub struct Sample {
    /// Epoch seconds, rounded to 1dp -- the timeline's first column.
    pub at: f64,
    pub idle_pct: Option<f64>,
    pub competitors: BTreeMap<String, f64>,
    pub competitors_total: f64,
    pub loadavg1: Option<f64>,
    pub swap_mb: Option<f64>,
    pub cpu_speed_limit: Option<u32>,
}

impl Sample {
    pub fn is_clean(&self) -> bool {
        self.competitors_total < SAMPLE_CLEAN_PCPU
    }
}

/// Parse `ps -Ao pcpu,comm -r` output into the competitor tally.
///
/// Separated from the syscall so the filter -- the part the 25% line was
/// calibrated against -- is testable against captured text.
pub fn attribute(ps_output: &str, exclude: &dyn Fn(&str) -> bool) -> BTreeMap<String, f64> {
    let mut found: Vec<(String, f64)> = Vec::new();
    for line in ps_output.lines().skip(1).take(PS_ROWS) {
        let line = line.trim();
        let Some((pc, cmd)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let Ok(pcpu) = pc.trim().parse::<f64>() else {
            continue;
        };
        let cmd = cmd.trim();
        if pcpu < MIN_PCPU || exclude(cmd) {
            continue;
        }
        // Collapse ".../Brave Browser Helper (Renderer).app/.../Brave Browser
        // Helper (Renderer)" to something readable, and SUM per name so three
        // renderers read as one 300% competitor rather than three.
        let base = cmd.rsplit('/').next().unwrap_or(cmd);
        let base = base
            .strip_suffix(".app")
            .or_else(|| base.strip_suffix(".xpc"))
            .unwrap_or(base);
        let name: String = base.chars().take(NAME_MAX).collect();
        match found.iter_mut().find(|(n, _)| *n == name) {
            Some((_, v)) => *v += pcpu,
            None => found.push((name, pcpu)),
        }
        if found.len() >= TOP_N * 3 {
            break;
        }
    }
    // Descending by cpu, ties broken by FIRST-SEEN order -- Python's sort is
    // stable over a dict in insertion order, and `found` preserves that.
    found.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    found.truncate(TOP_N);
    found.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: fn(&str) -> bool = |_| false;

    /// Defends the collapse-and-sum rule: three renderer processes are ONE
    /// competitor, not three, or the total under-reports a browser badly.
    #[test]
    fn same_name_processes_are_summed() {
        let ps = "%CPU COMM\n\
                  60.0 /Applications/Brave.app/Brave Browser Helper (Renderer)\n\
                  30.0 /Applications/Brave.app/Brave Browser Helper (Renderer)\n\
                  10.0 /usr/sbin/mDNSResponder\n";
        let c = attribute(ps, &NONE);
        assert_eq!(c.get("Brave Browser Helper (Renderer)"), Some(&90.0));
        assert_eq!(c.get("mDNSResponder"), Some(&10.0));
    }

    /// Anything under 5% is noise and was excluded when the 25% line was
    /// calibrated; including it would drift every board's total upward.
    #[test]
    fn sub_five_percent_processes_are_dropped() {
        let ps = "%CPU COMM\n4.9 /usr/bin/quiet\n5.0 /usr/bin/loud\n";
        let c = attribute(ps, &NONE);
        assert!(!c.contains_key("quiet"));
        assert_eq!(c.get("loud"), Some(&5.0));
    }

    /// Only the first 13 rows are read. `ps -r` sorts by CPU descending, so
    /// this bounds the work; a port that reads every row inflates the total.
    #[test]
    fn only_the_first_thirteen_rows_are_read() {
        let mut ps = String::from("%CPU COMM\n");
        for i in 0..30 {
            ps.push_str(&format!("10.0 /usr/bin/p{i}\n"));
        }
        let c = attribute(&ps, &NONE);
        assert!(c.values().sum::<f64>() <= 130.0);
        assert!(!c.contains_key("p20"));
    }

    #[test]
    fn app_and_xpc_suffixes_are_stripped_and_names_truncated() {
        let ps = format!("%CPU COMM\n40.0 /x/{}.app\n", "n".repeat(60));
        let c = attribute(&ps, &NONE);
        let k = c.keys().next().unwrap();
        assert_eq!(k.len(), NAME_MAX);
    }

    /// Our own tree must never count as competition. The Python excluded by
    /// substring and so never excluded `Validate`, which meant VAL's bursts of
    /// a full core were counted as foreign load on every temporal board.
    #[test]
    fn our_own_processes_are_excluded() {
        let ps = "%CPU COMM\n90.0 /repo/target/release/ff\n40.0 /usr/bin/real\n";
        let c = attribute(ps, &|cmd| cmd.contains("target/release/ff"));
        assert!(!c.keys().any(|k| k == "ff"));
        assert_eq!(c.get("real"), Some(&40.0));
    }

    /// The clean line itself: 24.9 passes, 25.0 does not.
    #[test]
    fn the_clean_line_is_exclusive() {
        let clean = Sample {
            competitors_total: 24.9,
            ..Default::default()
        };
        assert!(clean.is_clean());
        let dirty = Sample {
            competitors_total: 25.0,
            ..Default::default()
        };
        assert!(!dirty.is_clean());
    }

    /// The mco case, which is the whole reason the verdict moved off idle: a
    /// board burning most of the box by design is CLEAN when nothing else is
    /// running.
    #[test]
    fn a_thread_heavy_board_in_an_empty_room_is_clean() {
        let s = Sample {
            idle_pct: Some(38.0),
            competitors_total: 4.5,
            ..Default::default()
        };
        assert!(
            s.is_clean(),
            "mco-t8 reads 38-40% idle by design; an idle floor could never \
             pass it even in an empty room"
        );
    }
}
