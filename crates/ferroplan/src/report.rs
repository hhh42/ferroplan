//! Output rendering. Default = classic Metric-FF format (drop-in for the planner
//! crate + the differential validator); `-ipc` = SGPlan6's IPC temporal format.

use crate::packed::PackedTask;

use crate::resolve::Stats;

/// Classic-FF plan body: `step    0: NAME ARGS` framing + trailing 5-space line.
pub fn ff_plan(task: &PackedTask, ops: &[usize]) -> String {
    let mut s = String::from("\nff: found legal plan as follows\n\nstep ");
    for (i, &oi) in ops.iter().enumerate() {
        s.push_str(&format!("{:4}: {}", i, task.op_display[oi]));
        s.push_str("\n     ");
    }
    s
}

/// SGPlan6 IPC temporal format: `0.001: (NAME ARGS) [1]` + `; ` headers.
/// `metric` fills the `; MetricValue` line when present (PDDL3 optimization).
pub fn ipc_plan(task: &PackedTask, ops: &[usize], metric: Option<f64>) -> String {
    let mv = match metric {
        Some(v) => format!("; MetricValue {}\n", v),
        None => "; MetricValue\n".to_string(),
    };
    let mut s = format!(
        "\n; Time 0.00\n; ParsingTime 0.00\n; NrActions {}\n; MakeSpan\n{}\
         ; PlanningTechnique partition-and-resolve over modified-FF (ffdp) subplanner\n\n",
        ops.len(),
        mv
    );
    for (i, &oi) in ops.iter().enumerate() {
        // start times mirror sgplan6: 0.001, 1.002, 2.003, ... (carry-correct for
        // plans >= 1000 steps, unlike a naive "{i}.{i+1:03}").
        let t = i as f64 * 1.001 + 0.001;
        s.push_str(&format!("{:.3}: ({}) [1]\n", t, task.op_display[oi]));
    }
    s
}

pub fn timing(stats: &Stats, threads: usize) -> String {
    format!(
        "\ntime spent:    0.00 seconds partition-and-resolve ({} threads)\n\
         \x20              {} initial groups, {} final groups, {} merges{}\n\
         \x20              0.00 seconds total time\n\n",
        threads,
        stats.init_groups,
        stats.final_groups,
        stats.merges,
        if stats.fallback {
            " (monolithic fallback)"
        } else {
            ""
        },
    )
}

/// Footer for the PDDL3 metric-optimization path (classic-FF mode).
pub fn metric_footer(
    cost: f64,
    iterations: usize,
    n_prefs: usize,
    threads: usize,
    warn_other: bool,
) -> String {
    let warn = if warn_other {
        "\n               note: metric has terms beyond is-violated/total-cost; optimized the supported part"
    } else {
        ""
    };
    format!(
        "\ntime spent:    0.00 seconds PDDL3 metric optimization ({} threads)\n\
         \x20              metric value {}, {} preferences, {} branch-and-bound iterations{}\n\
         \x20              0.00 seconds total time\n\n",
        threads, cost, n_prefs, iterations, warn
    )
}

/// The post-parse search-config banner.
pub fn preamble(threads: usize) -> String {
    format!(
        "\n\nno metric specified. plan length assumed.\n\n\
         checking for cyclic := effects --- OK.\n\n\
         ff: search configuration is SGPlan partition-and-resolve over the ffdp subplanner [{} threads]\n",
        threads
    )
}
