//! The MCV join-order golden battery (0.22 Phase 7 lever 1) and the
//! threshold-routed fixpoint composition (lever 2).
//!
//! Lever 1's contract is BYTE-IDENTITY BY CONSTRUCTION: above
//! `FF_MCV_THRESHOLD` the binding enumeration recurses in a greedy
//! bound-connected most-constrained-variable order, but survivors are
//! SORTED BACK to declaration row-major before emission, so the RawOp
//! stream and the fact-intern order never move. The recorded sokoban-t
//! regression class (fixpoint grounding shifted fact-id first-reference
//! order and cost real coverage, 0.12) is structurally impossible here —
//! and this battery is the fixture that says so, not a discipline note:
//! gripper (the unary-restriction shape), tpp-metric i1 (numeric,
//! fluent-fold interactions), sokoban-mini (the three-location push join
//! the sitting re-attributed sokoban-t's grounding wall to), each
//! fingerprinted with MCV forced onto EVERY action via
//! `FF_MCV_THRESHOLD=1` and with the `FF_NO_MCV_JOIN` hatch pulled.
//!
//! Lever 2's composition fixture runs in CHILD processes (the
//! tests/ground_wall.rs convention): the wall clock and FF_* knobs are
//! process-global, and the route narration lands on stderr.

use std::process::Command;

use ferroplan::ground::{self, Outcome};
use ferroplan::packed::PackedTask;
use ferroplan::parser::{parse_domain, parse_problem};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/../../benchmarks/bench/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    ))
    .expect("fixture exists")
}

fn ipc_fixture(rel: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/../../benchmarks/ipc/{}",
        env!("CARGO_MANIFEST_DIR"),
        rel
    ))
    .expect("ipc fixture exists")
}

/// Full-task fingerprint: every field that could betray a moved op, fact,
/// fluent, or tie-break. Debug formatting is stable within one binary,
/// which is all a same-process A/B needs.
fn fingerprint(t: &PackedTask) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(
        s,
        "n_facts={} words={} n_ops={}",
        t.n_facts, t.words, t.n_ops
    );
    let _ = writeln!(s, "fact_names={:?}", t.fact_names);
    let _ = writeln!(s, "fluent_names={:?}", t.fluent_names);
    let _ = writeln!(s, "static_fluents={:?}", t.static_fluents);
    let _ = writeln!(s, "init_bits={:?}", t.init_bits);
    let _ = writeln!(
        s,
        "fv0={:?}",
        t.fv0.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
    let _ = writeln!(s, "fdef0={:?}", t.fdef0);
    let _ = writeln!(s, "goal_pos={:?} goal_num={:?}", t.goal_pos, t.goal_num);
    let _ = writeln!(s, "rel_fluents={:?}", t.rel_fluents);
    for oi in 0..t.n_ops {
        let _ = writeln!(
            s,
            "op {oi} `{}` pre={:?} add={:?} del={:?} pre_num={:?} num_eff={:?} cond={:?} mon={}",
            t.op_display[oi],
            t.pre_pos.slice(oi),
            t.add.slice(oi),
            t.del.slice(oi),
            t.pre_num.slice(oi),
            t.num_eff.slice(oi),
            t.cond.slice(oi),
            t.monitored[oi],
        );
    }
    let _ = writeln!(s, "shared_cond={:?}", t.shared_cond);
    s
}

fn ground_fp(dom: &str, prb: &str) -> String {
    let d = parse_domain(dom).expect("domain parses");
    let p = parse_problem(prb).expect("problem parses");
    match ground::ground(&d, &p, 1) {
        Outcome::Task(t) => fingerprint(&t),
        other => panic!(
            "battery member must ground to a task, got {}",
            match other {
                Outcome::GoalTrue => "GoalTrue",
                Outcome::GoalFalse(_) => "GoalFalse",
                _ => "non-task",
            }
        ),
    }
}

fn solve_sig(dom: &str, prb: &str) -> (Vec<String>, usize) {
    let opts = ferroplan::Options {
        threads: 1,
        ..Default::default()
    };
    let sol = ferroplan::solve(dom, prb, &opts).expect("solve runs");
    assert!(sol.solved, "battery member must solve: {:?}", sol.notes);
    (
        sol.plan
            .unwrap()
            .steps
            .iter()
            .map(|s| format!("{} {}", s.action, s.args.join(" ")))
            .collect(),
        sol.statistics.evaluated_states,
    )
}

/// THE BATTERY: forced-MCV and hatched groundings must be byte-identical
/// to the plain path on every member, and the sokoban-mini solve must not
/// move by a single evaluation.
#[test]
fn mcv_join_order_is_byte_identical_across_the_battery() {
    let clear = || {
        std::env::remove_var("FF_MCV_THRESHOLD");
        std::env::remove_var("FF_NO_MCV_JOIN");
    };
    clear();

    let members: [(&str, String, String); 3] = [
        (
            "gripper-p01",
            ipc_fixture("strips/gripper/domain.pddl"),
            ipc_fixture("strips/gripper/p01.pddl"),
        ),
        (
            "tpp-metric-i1",
            fixture("tpp-metric-domain.pddl"),
            fixture("tpp-metric-i1.pddl"),
        ),
        (
            "sokoban-mini",
            fixture("sokoban-mini-domain.pddl"),
            fixture("sokoban-mini.pddl"),
        ),
    ];

    for (name, dom, prb) in &members {
        let plain = ground_fp(dom, prb);

        // MCV forced onto every action (typed product > 1 everywhere).
        std::env::set_var("FF_MCV_THRESHOLD", "1");
        let forced = ground_fp(dom, prb);

        // the candidate-list lever pulled alone (0.24 Phase 6): forced
        // MCV with FF_NO_JOIN_INDEX must STILL be byte-identical — the
        // index may only ever skip values the level check rejects
        std::env::set_var("FF_NO_JOIN_INDEX", "1");
        let no_index = ground_fp(dom, prb);
        std::env::remove_var("FF_NO_JOIN_INDEX");

        // the hatch pulls the lever back out, threshold notwithstanding
        std::env::set_var("FF_NO_MCV_JOIN", "1");
        let hatched = ground_fp(dom, prb);
        clear();

        assert!(
            plain == forced,
            "{name}: forced MCV must reproduce the plain grounding byte-for-byte"
        );
        assert!(
            plain == no_index,
            "{name}: the candidate-list lever must change nothing but the walk"
        );
        assert!(
            plain == hatched,
            "{name}: FF_NO_MCV_JOIN must be the plain path"
        );
    }

    // Solve-level pin on the sokoban shape: same plan, same eval count.
    let (dom, prb) = (&members[2].1, &members[2].2);
    let plain = solve_sig(dom, prb);
    std::env::set_var("FF_MCV_THRESHOLD", "1");
    let forced = solve_sig(dom, prb);
    clear();
    assert_eq!(
        plain, forced,
        "sokoban-mini: search must not move under MCV"
    );
}

// ---- lever 2: the threshold-routed fixpoint, composed with MCV ----------

/// A dynamic-gated join in miniature (the organic-synthesis shape): REAP's
/// gating literal TOK is produced only by SEED, so static pruning has
/// nothing to hold and the plain path enumerates the full 40*40*3 = 4,800
/// product. Routed through the fixpoint, round 1 reaps nothing, round 2
/// joins against the single reached TOK atom — with MCV active inside the
/// enumeration (product 4,800 > the forced 100 threshold). Since 0.23
/// Phase 6 TOK, a FULL-COVER literal, is excluded from the ordering
/// signal (it binds only at the last level under any order), so the walk
/// runs in declaration order here; the genuinely-permuted sort-back is
/// exercised by the golden battery above and the pushstorm pin below.
const ROUTE_DOM: &str = "(define (domain fixroute) (:requirements :typing)
  (:types item key - object)
  (:predicates (tok ?a ?b - item ?c - key) (done ?a - item) (seeded))
  (:action seed :parameters ()
    :precondition (and) :effect (and (seeded) (tok i1 i2 k1)))
  (:action reap :parameters (?a ?b - item ?c - key)
    :precondition (and (seeded) (tok ?a ?b ?c))
    :effect (done ?a)))";

fn route_prb() -> String {
    let objs: Vec<String> = (1..=40).map(|i| format!("i{i}")).collect();
    format!(
        "(define (problem fr) (:domain fixroute)
           (:objects {} - item k1 k2 k3 - key)
           (:init ) (:goal (done i1)))",
        objs.join(" ")
    )
}

fn run_route_child(scenario: &str) -> (String, String) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args(["--exact", "fixpoint_route_composes_with_mcv", "--nocapture"])
        .env("MCV_ROUTE_CHILD", scenario);
    // Deterministic env regardless of sibling tests in the parent process.
    for k in [
        "FF_MCV_THRESHOLD",
        "FF_NO_MCV_JOIN",
        "FF_FIXPOINT_THRESHOLD",
        "FF_NO_FIXPOINT_GROUND",
        "FF_TIME_LIMIT",
    ] {
        cmd.env_remove(k);
    }
    match scenario {
        // route: product 4,800 > threshold 1,000 => fixpoint; MCV inside.
        "routed" => {
            cmd.env("FF_FIXPOINT_THRESHOLD", "1000")
                .env("FF_MCV_THRESHOLD", "100")
                .env("FF_WALL_DEBUG", "1");
        }
        // hatch: same thresholds, FF_NO_FIXPOINT_GROUND pins the plain path.
        "hatched" => {
            cmd.env("FF_FIXPOINT_THRESHOLD", "1000")
                .env("FF_MCV_THRESHOLD", "100")
                .env("FF_NO_FIXPOINT_GROUND", "1")
                .env("FF_WALL_DEBUG", "1");
        }
        // default thresholds: 64,000 sits far below 1e8 => never routed
        // (the in-tree analog of the agricola near-threshold control).
        "default" => {
            cmd.env("FF_WALL_DEBUG", "1");
        }
        other => panic!("unknown scenario {other}"),
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "child {scenario} failed");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn fixpoint_route_composes_with_mcv() {
    if std::env::var("MCV_ROUTE_CHILD").is_ok() {
        let opts = ferroplan::Options {
            threads: 1,
            ..Default::default()
        };
        let sol = ferroplan::solve(ROUTE_DOM, &route_prb(), &opts).unwrap();
        println!("CHILD-SOLVED:{}", sol.solved);
        let steps: Vec<String> = sol
            .plan
            .map(|p| {
                p.steps
                    .iter()
                    .map(|s| format!("{} {}", s.action, s.args.join(" ")))
                    .collect()
            })
            .unwrap_or_default();
        println!("CHILD-PLAN:{}", steps.join(" | "));
        return;
    }

    // Routed: solves, names the route on stderr, and the plan is the
    // forced two-step SEED -> REAP.
    let (stdout, stderr) = run_route_child("routed");
    assert!(stdout.contains("CHILD-SOLVED:true"), "{stdout}\n{stderr}");
    assert!(
        stdout.contains("SEED") && stdout.contains("REAP I1 I2 K1"),
        "routed plan must be seed->reap:\n{stdout}"
    );
    assert!(
        stderr.contains("fixpoint route"),
        "the route must narrate under FF_WALL_DEBUG:\n{stderr}"
    );

    // Hatched: same solve, no route.
    let (stdout, stderr) = run_route_child("hatched");
    assert!(stdout.contains("CHILD-SOLVED:true"), "{stdout}\n{stderr}");
    assert!(
        stdout.contains("SEED") && stdout.contains("REAP I1 I2 K1"),
        "hatched plan must be seed->reap:\n{stdout}"
    );
    assert!(
        !stderr.contains("fixpoint route"),
        "FF_NO_FIXPOINT_GROUND must pin the plain path:\n{stderr}"
    );

    // Default thresholds: the 64,000-product task stays plain-path — the
    // near-threshold negative control in miniature.
    let (stdout, stderr) = run_route_child("default");
    assert!(stdout.contains("CHILD-SOLVED:true"), "{stdout}\n{stderr}");
    assert!(
        !stderr.contains("fixpoint route"),
        "default thresholds must not route a 4.8e3 product:\n{stderr}"
    );
}

// ---- the stratum-2 ordering signal (0.23 Phase 6) ------------------------

/// Sokoban-t's snap shape, distilled: a 6-parameter durative PUSH whose
/// `over all` MOVE-DIR statics are the only real join structure. The snap
/// compile conjoins them into BOTH endpoints, and the stratified pass gates
/// PUSH-END on the producer-known RUNNING-PUSH token — a literal that names
/// EVERY parameter. Full-cover literals carry zero ordering information
/// (they bind at the last level under any permutation), but they used to
/// flood `mcv_order`'s connectivity classes: every parameter reads
/// "connected" from the first pick, the tie-break degrades to domain size,
/// and the MOVE-DIR statics bind five levels deep — the |d|·P·S·L² prefix
/// walk that ground sokoban-t 2008 i21 for >10 minutes against a 30 s wall
/// while `FF_NO_STRAT_GROUND=1` ground the same instance in 1.8 s.
///
/// The MOVE-DIR line covers only FIVE locations, so the survivor set (and
/// with it every post-enumeration phase) stays trivial: the defeated prefix
/// walk is the ONLY cost in the fixture, exactly the attribution the
/// receipts carry.
fn pushstorm(agents: usize, crates_: usize, locs: usize) -> (String, String) {
    let mut objs = String::new();
    for i in 0..agents {
        objs.push_str(&format!(" p{i}"));
    }
    objs.push_str(" - agent");
    for i in 0..crates_ {
        objs.push_str(&format!(" s{i}"));
    }
    objs.push_str(" - crate");
    for i in 0..locs {
        objs.push_str(&format!(" l{i}"));
    }
    objs.push_str(" - loc east west - dir");
    let mut init = String::from(" (go)");
    for i in 0..4.min(locs - 1) {
        init.push_str(&format!(
            " (move-dir l{i} l{} east) (move-dir l{} l{i} west)",
            i + 1,
            i + 1
        ));
    }
    (
        "(define (domain pushstorm)
          (:requirements :typing :durative-actions)
          (:types agent crate loc dir)
          (:predicates (move-dir ?a ?b - loc ?d - dir) (go) (gw))
          (:durative-action push
            :parameters (?p - agent ?s - crate ?ppos ?from ?to - loc ?d - dir)
            :duration (= ?duration 1)
            :condition (and (over all (move-dir ?ppos ?from ?d))
                            (over all (move-dir ?from ?to ?d)))
            :effect (at end (gw)))
          (:durative-action easy
            :parameters ()
            :duration (= ?duration 1)
            :condition (at start (go))
            :effect (at end (gw))))"
            .into(),
        format!(
            "(define (problem ps) (:domain pushstorm) (:objects{objs}) \
             (:init{init}) (:goal (gw)))"
        ),
    )
}

/// THE PIN: under an armed wall the stratified temporal grounding must
/// SOLVE the pushstorm — the MCV order keyed off the MOVE-DIR statics
/// collapses the walk regardless of the full-cover RUNNING token. RED
/// before the `mcv_order` fix: the defeated order cannot finish inside the
/// wall, and the (0.23 Phase 6) temporal grounding wall turns the grind
/// into an honest solved:false. One child process (wall clock is global).
#[test]
fn stratified_gating_does_not_defeat_the_mcv_order() {
    if std::env::var("MCV_STRAT_CHILD").is_ok() {
        let (dom, prb) = if cfg!(debug_assertions) {
            pushstorm(12, 12, 120)
        } else {
            pushstorm(20, 20, 250)
        };
        let opts = ferroplan::Options {
            threads: 1,
            ..Default::default()
        };
        let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
        println!("CHILD-SOLVED:{}", sol.solved);
        return;
    }
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args([
        "--exact",
        "stratified_gating_does_not_defeat_the_mcv_order",
        "--nocapture",
    ])
    .env("MCV_STRAT_CHILD", "1")
    .env(
        "FF_TIME_LIMIT",
        if cfg!(debug_assertions) { "20" } else { "5" },
    );
    for k in ["FF_MCV_THRESHOLD", "FF_NO_MCV_JOIN", "FF_NO_STRAT_GROUND"] {
        cmd.env_remove(k);
    }
    let t0 = std::time::Instant::now();
    let out = cmd.output().unwrap();
    let secs = t0.elapsed().as_secs_f64();
    assert!(out.status.success(), "child failed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("CHILD-SOLVED:true"),
        "the statics' MCV signal must survive the stratum-2 gating token \
         (defeated order = honest wall exit = solved:false):\n{stdout}"
    );
    if !cfg!(debug_assertions) {
        assert!(
            secs < 5.0,
            "the ordered walk must finish well inside the wall: {secs:.1} s"
        );
    }
}
