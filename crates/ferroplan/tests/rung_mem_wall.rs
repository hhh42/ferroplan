//! A BOUNDED RUNG IS BOUNDED WORK (0.30 Lane 5, `MemWall::arm_rung`).
//!
//! The classical ladder's LAMA and novelty rungs are bets with the complete
//! best-first beneath them, and until 0.30 none of them read the measured
//! memory wall (`src/mem.rs`): on city-car i6 the LAMA rung reached the
//! runner's 6 GB cap at 20 s (1.98 GB at 5 s, 5.9 GB at 20 s) while the
//! best-first beneath it would have run the whole wall in under a gigabyte,
//! and the row was a SIGKILL with forty seconds unspent
//! (`benchmarks/probes-0.30/lane5/measure2.tsv`).
//!
//! The fixture is the visit-all grid LAMA cannot finish at fixture scale
//! (every cell a landmark, h_FF's plateau), run in a CHILD process so the
//! kernel's peak-resident reading is the child's own: the budget is derived
//! from the child's resident set after grounding, so the line sits a fixed
//! distance above whatever the test harness itself costs. Walled, LAMA ends
//! on the memory wall with the peak under the declared budget; hatched
//! (`FF_NO_MEM_WALL=1`), the same search outgrows it -- the RED shape.

use std::process::Command;

/// How far above the post-grounding baseline the TRIP line sits.
const HEADROOM_BYTES: u64 = 32 << 20;

fn grid(n: usize) -> (String, String) {
    let dom = "(define (domain visitgrid)
      (:requirements :strips)
      (:predicates (at ?x) (conn ?x ?y) (visited ?x))
      (:action move
        :parameters (?from ?to)
        :precondition (and (at ?from) (conn ?from ?to))
        :effect (and (not (at ?from)) (at ?to) (visited ?to))))"
        .to_string();
    let mut objs = String::new();
    let mut conn = String::new();
    let mut goal = String::new();
    for i in 0..n {
        for j in 0..n {
            objs.push_str(&format!(" c{i}-{j}"));
            goal.push_str(&format!(" (visited c{i}-{j})"));
            if j + 1 < n {
                conn.push_str(&format!(
                    " (conn c{i}-{j} c{i}-{}) (conn c{i}-{} c{i}-{j})",
                    j + 1,
                    j + 1
                ));
            }
            if i + 1 < n {
                conn.push_str(&format!(
                    " (conn c{i}-{j} c{}-{j}) (conn c{}-{j} c{i}-{j})",
                    i + 1,
                    i + 1
                ));
            }
        }
    }
    let prb = format!(
        "(define (problem vg) (:domain visitgrid) (:objects{objs}) \
         (:init (at c0-0) (visited c0-0){conn}) (:goal (and{goal})))"
    );
    (dom, prb)
}

fn run_child(hatched: bool) -> (String, String) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args([
        "--exact",
        "a_bounded_rung_stops_at_the_memory_wall",
        "--nocapture",
    ])
    .env("RUNG_MEM_WALL_CHILD", "1")
    .env("FF_WALL_DEBUG", "1");
    if hatched {
        cmd.env("FF_NO_MEM_WALL", "1");
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "child failed (hatched={hatched})");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn field<'a>(stdout: &'a str, key: &str) -> &'a str {
    stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix(key))
        .unwrap_or_else(|| panic!("child printed no {key}:\n{stdout}"))
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn a_bounded_rung_stops_at_the_memory_wall() {
    if std::env::var("RUNG_MEM_WALL_CHILD").is_ok() {
        let (n, max_eval) = if cfg!(debug_assertions) {
            (12, 15_000)
        } else {
            (20, 60_000)
        };
        let (dom, prb) = grid(n);
        let d = ferroplan::parser::parse_domain(&dom).unwrap();
        let p = ferroplan::parser::parse_problem(&prb).unwrap();
        let task = ferroplan::ground::ground_task(&d, &p, 1).unwrap();
        let base = ferroplan::mem::resident_bytes().expect("readable here");
        // The trip line is 75 % of the declared budget (`mem::TRIP_FRAC`):
        // declare the budget that puts the line HEADROOM above the baseline.
        let budget = ((base + HEADROOM_BYTES) as f64 / 0.75) as u64;
        std::env::set_var(
            "FF_MEM_BUDGET_GB",
            format!("{:.6}", budget as f64 / (1u64 << 30) as f64),
        );
        let t0 = std::time::Instant::now();
        let r = ferroplan::lama::search(&task, 1, max_eval, &[], None);
        println!("CHILD-SECS:{:.3}", t0.elapsed().as_secs_f64());
        println!("CHILD-SOLVED:{}", r.is_some());
        println!("CHILD-EVALS:{}", r.map_or(0, |(_, e)| e));
        println!("CHILD-BUDGET:{budget}");
        println!(
            "CHILD-PEAK:{}",
            ferroplan::mem::peak_resident_bytes().expect("readable here")
        );
        return;
    }

    let (stdout, stderr) = run_child(false);
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "false",
        "{stdout}\n{stderr}"
    );
    assert!(
        stderr.contains("wall: LAMA MEMORY checkpoint"),
        "the rung must have been ended by the memory wall:\n{stderr}"
    );
    let budget: u64 = field(&stdout, "CHILD-BUDGET:").parse().unwrap();
    let peak: u64 = field(&stdout, "CHILD-PEAK:").parse().unwrap();
    assert!(
        peak < budget,
        "peak resident {peak} must stay under the declared {budget}"
    );

    let (stdout, stderr) = run_child(true);
    assert!(
        !stderr.contains("MEMORY checkpoint"),
        "the hatch must disarm the rung's wall:\n{stderr}"
    );
    let budget: u64 = field(&stdout, "CHILD-BUDGET:").parse().unwrap();
    let peak: u64 = field(&stdout, "CHILD-PEAK:").parse().unwrap();
    assert!(
        peak > budget,
        "unwalled, the rung was expected to outgrow its budget (the RED shape): \
         peak {peak} against {budget} -- raise the grid or lower the headroom"
    );
}
