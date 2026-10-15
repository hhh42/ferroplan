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
//! The fixture is k agents on a ring of m cells whose goal reverses their
//! cyclic order -- impossible, invisible to the relaxation, C(m, k) states
//! wide, so LAMA fills memory for as long as it is allowed to (the lazy
//! frontier made the visit-all grid of the first version too cheap to
//! trip). It runs in a CHILD process so the kernel's peak-resident reading
//! is the child's own: the budget is derived from the child's resident set
//! after grounding, so the line sits a fixed distance above whatever the
//! test harness itself costs. Walled, LAMA ends on the memory wall with
//! the peak under the declared budget; hatched (`FF_NO_MEM_WALL=1`), the
//! same search outgrows it -- the RED shape.

use std::process::Command;

/// How far above the post-grounding baseline the TRIP line sits.
const HEADROOM_BYTES: u64 = 16 << 20;

/// `k` agents on a ring of `m` cells, goal: their cyclic order reversed.
fn ring(m: usize, k: usize) -> (String, String) {
    let dom = "(define (domain agentring)
      (:requirements :strips)
      (:predicates (at ?a ?c) (free ?c) (adj ?c1 ?c2))
      (:action move
        :parameters (?a ?from ?to)
        :precondition (and (at ?a ?from) (adj ?from ?to) (free ?to))
        :effect (and (not (at ?a ?from)) (at ?a ?to) (free ?from) (not (free ?to)))))"
        .to_string();
    let mut objs = String::new();
    let mut init = String::new();
    let mut goal = String::new();
    for c in 0..m {
        objs.push_str(&format!(" c{c}"));
        let n = (c + 1) % m;
        init.push_str(&format!(" (adj c{c} c{n}) (adj c{n} c{c})"));
        if c >= k {
            init.push_str(&format!(" (free c{c})"));
        }
    }
    for a in 0..k {
        objs.push_str(&format!(" a{a}"));
        init.push_str(&format!(" (at a{a} c{a})"));
        goal.push_str(&format!(" (at a{a} c{})", k - 1 - a));
    }
    let prb = format!(
        "(define (problem ring) (:domain agentring) (:objects{objs}) \
         (:init{init}) (:goal (and{goal})))"
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
    .env("FF_WALL_DEBUG", "1")
    // The MODELLED node cap (`search::node_cap_for`, ~200 B a held node)
    // now trips before the measured wall on a lazy frontier (~50 B an
    // open entry); lift it, so the measured wall is what ends the rung.
    .env("FF_SEARCH_NODE_CAP", "0");
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
        let (m, k, max_eval) = if cfg!(debug_assertions) {
            (24, 6, 60_000)
        } else {
            (40, 8, 150_000)
        };
        let (dom, prb) = ring(m, k);
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
        "the rung must have been ended by the memory wall:\n{stdout}\n{stderr}"
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
         peak {peak} against {budget} -- raise the cap or lower the headroom"
    );
}
