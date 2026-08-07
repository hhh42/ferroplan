//! The numeric-admissible bound's RED/GREEN pair (0.22 Phase 4), on the
//! gap-60 pump chain (benchmarks/bench/numopt-p04.pddl): blind — the
//! 0.21 behavior, restored by `FF_OPT_NO_NUMH=1` — the proof pays the
//! junk toggles' 2^8 product (~14k expansions of a 60-step chain);
//! armed, num_h = 60 − charge is exact, every toggle successor prices at
//! f = 61 > 60, and the frontier collapses to the chain (~61 expansions,
//! two orders). Same certified cost BOTH ways — the arm buys allocation,
//! never a different optimum.
//!
//! `FF_*` are process-global env knobs, so each scenario runs in a CHILD
//! process (the tests/refill.rs convention). The battery also pins the
//! composition semantics of the discriminator hatches on the numeric
//! side: `FF_NO_LMCUT` / `FF_NO_HMAX_SPRINT` keep their pure-rung
//! meanings with the arm riding along, and `FF_OPT_NO_NUMH` strips
//! "+numRPG" from every prover name.

use std::process::Command;

fn run_child(scenario: &str) -> String {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args(["--exact", "num_arm_collapses_the_pump_chain", "--nocapture"])
        .env("NUMOPT_ARM_CHILD", scenario);
    match scenario {
        "armed" => {}
        "blind" => {
            cmd.env("FF_OPT_NO_NUMH", "1");
        }
        "armed-no-lmcut" => {
            cmd.env("FF_NO_LMCUT", "1");
        }
        "armed-no-sprint" => {
            cmd.env("FF_NO_HMAX_SPRINT", "1");
        }
        other => panic!("unknown scenario {other}"),
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "child {scenario} failed");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn note_expansions(stdout: &str) -> usize {
    let head = stdout
        .split(" expansions)")
        .next()
        .unwrap_or_else(|| panic!("no expansions in {stdout}"));
    let digits: String = head
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits
        .chars()
        .rev()
        .collect::<String>()
        .parse()
        .unwrap_or_else(|_| panic!("no expansion count in {stdout}"))
}

#[test]
fn num_arm_collapses_the_pump_chain() {
    if let Ok(scenario) = std::env::var("NUMOPT_ARM_CHILD") {
        let dom = include_str!("../../../benchmarks/bench/numopt-pump-domain.pddl");
        let prb = include_str!("../../../benchmarks/bench/numopt-p04.pddl");
        let opts = ferroplan::Options {
            mode: ferroplan::Mode::Optimal,
            threads: 1,
            ..Default::default()
        };
        let sol = ferroplan::solve(dom, prb, &opts).unwrap();
        println!("CHILD-{scenario}-SOLVED:{}", sol.solved);
        println!(
            "CHILD-LENGTH:{}",
            sol.plan.as_ref().map(|p| p.length).unwrap_or(0)
        );
        println!("CHILD-NOTE:{}", sol.notes.join(" | "));
        return;
    }

    // GREEN — the armed default: cost 60, collapsed frontier, the note
    // naming both admissible components.
    let armed = run_child("armed");
    assert!(armed.contains("CHILD-armed-SOLVED:true"), "{armed}");
    assert!(armed.contains("CHILD-LENGTH:60"), "{armed}");
    assert!(
        armed.contains("h^max+numRPG"),
        "the prover must name +numRPG: {armed}"
    );
    let armed_exp = note_expansions(&armed);

    // RED via hatch — FF_OPT_NO_NUMH restores the blind 0.21 shape: same
    // certificate, two orders more expansions (the onlycraft drowning,
    // in miniature).
    let blind = run_child("blind");
    assert!(blind.contains("CHILD-blind-SOLVED:true"), "{blind}");
    assert!(blind.contains("CHILD-LENGTH:60"), "{blind}");
    assert!(
        !blind.contains("+numRPG"),
        "the hatch must strip the numeric component: {blind}"
    );
    let blind_exp = note_expansions(&blind);
    assert!(
        blind_exp >= 100 * armed_exp,
        "blind {blind_exp} vs armed {armed_exp}: the collapse must be two orders"
    );

    // Pure-rung hatches keep their meanings WITH the arm riding along.
    let no_lmcut = run_child("armed-no-lmcut");
    assert!(no_lmcut.contains("h^max+numRPG"), "{no_lmcut}");
    assert!(no_lmcut.contains("CHILD-LENGTH:60"), "{no_lmcut}");
    let no_sprint = run_child("armed-no-sprint");
    assert!(no_sprint.contains("LM-cut+numRPG"), "{no_sprint}");
    assert!(no_sprint.contains("CHILD-LENGTH:60"), "{no_sprint}");
}
