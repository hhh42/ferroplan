//! TRPG-lite (0.23 Phase 4, probe 2, docs/roadmap-0.23.md): under
//! `FF_TRPG=1` the temporal solve path arms time-stamped-relaxation tables
//! on the task — END fires anchor at start + duration, TIL adds carry
//! their wall-time floor, and an END's relaxed payout is GATED on its
//! over-all-invariant windows (envelope width or TIL close). The heuristic
//! keys on the TABLE's presence, never the env (the `pair_end` rule), and
//! only the pruned pass's door (`relaxed_helpful`) picks the timed build —
//! the complete passes stay time-blind, so completeness never rests on the
//! gate. The gate mechanics pins live with the plumbing (temporal.rs unit
//! tests: envelope overrun refused, static TIL close refused, both RED
//! halves pinned time-blind); this file holds the do-no-harm surface.

use ferroplan::{solve, Options};

/// Env-mutating tests serialize here — the parallel runner must never
/// interleave a flagged solve with an unflagged one.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn bench(name: &str) -> String {
    let base = format!("{}/../../benchmarks/bench", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(format!("{base}/{name}")).unwrap()
}

/// Classical byte-identity: the classical grounding never builds the TRPG
/// tables, so FF_TRPG set vs unset must produce the identical plan and
/// eval count on a seq-sat bench fixture. Provable from `trpg: None`;
/// pinned anyway (the roadmap's standing rule for opt-in hatches).
#[test]
fn classical_flag_is_dormant_byte_identical() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dom = bench("visitgrid-domain.pddl");
    let prb = bench("visitgrid-7x7.pddl");
    let run = || {
        let s = solve(&dom, &prb, &Options::default()).expect("solve");
        assert!(s.solved, "visitgrid-7x7 must solve");
        let plan: Vec<String> = s
            .plan
            .as_ref()
            .unwrap()
            .steps
            .iter()
            .map(|st| format!("{} {}", st.action, st.args.join(" ")))
            .collect();
        (plan, s.statistics.evaluated_states)
    };
    let off = run();
    std::env::set_var("FF_TRPG", "1");
    let on = run();
    std::env::remove_var("FF_TRPG");
    assert_eq!(off.0, on.0, "classical plan must not move under the flag");
    assert_eq!(
        off.1, on.1,
        "classical eval count must not move under the flag"
    );
}

/// The kiln-pack pin (the 0.15 receipt: 29→539 evals across N=2..12 —
/// near-linear window packing, and the exact shape the envelope gate now
/// reasons about). Both ends of the ladder must keep solving inside a
/// 1,100-eval budget with the flag ON and OFF: if the timed build ever
/// bends the family away from its near-linear eval curve, this fails
/// before any board does. Budget via FF_TEVAL_BUDGET — the deterministic
/// measuring stick, never wall clock.
#[test]
fn kiln_pack_near_linearity_holds_flag_on() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dom = bench("kiln-pack-domain.pddl");
    let sizes = ["kiln-pack-2.pddl", "kiln-pack-12.pddl"];
    std::env::set_var("FF_NO_ESCALATE", "1");
    std::env::set_var("FF_TEVAL_BUDGET", "1100");
    for flag in [false, true] {
        if flag {
            std::env::set_var("FF_TRPG", "1");
        }
        for prb in sizes {
            let s = solve(&dom, &bench(prb), &Options::default()).expect("solve");
            assert!(
                s.solved,
                "{prb} must solve inside 1,100 evals (FF_TRPG={})",
                flag as u8
            );
        }
        std::env::remove_var("FF_TRPG");
    }
    std::env::remove_var("FF_TEVAL_BUDGET");
    std::env::remove_var("FF_NO_ESCALATE");
}
