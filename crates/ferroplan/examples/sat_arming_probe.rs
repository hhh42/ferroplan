//! The Wing II arming audit (0.25 Phase 3 step 1): for a temporal
//! domain/problem pair, print the required-concurrency detector's
//! verdict AND the per-predicate breakdown behind it — every over-all
//! predicate, who produces it and how (during-envelope vs classical /
//! at-end / at-start-non-envelope adders), whether it pre-exists in
//! init or arrives by TIL. The audit question this answers: on the
//! field-receipted families (storage-t, parc-printer-t, floor-tile-t,
//! TMS) that never reach the SAT face, WHICH gate refuses them — the
//! detector (this probe), or ladder exhaustion eating the wall (run
//! with FF_WALL_DEBUG for that half).
//!
//! Usage: sat_arming_probe <domain.pddl> <problem.pddl>

use ferroplan::types::{Effect, Formula, TimeSpec};
use std::collections::HashSet;

fn collect_pos_atoms(f: &Formula, out: &mut HashSet<String>) {
    match f {
        Formula::Atom(p, _) => {
            out.insert(p.to_ascii_uppercase());
        }
        Formula::And(v) => v.iter().for_each(|x| collect_pos_atoms(x, out)),
        Formula::Forall(_, inner) => collect_pos_atoms(inner, out),
        _ => {}
    }
}

fn adds(e: &Effect, pred: &str) -> bool {
    match e {
        Effect::Add(p, _) => p.eq_ignore_ascii_case(pred),
        Effect::And(v) => v.iter().any(|x| adds(x, pred)),
        Effect::When(_, i) | Effect::Forall(_, i) => adds(i, pred),
        _ => false,
    }
}

fn dels(e: &Effect, pred: &str) -> bool {
    match e {
        Effect::Del(p, _) => p.eq_ignore_ascii_case(pred),
        Effect::And(v) => v.iter().any(|x| dels(x, pred)),
        Effect::When(_, i) | Effect::Forall(_, i) => dels(i, pred),
        _ => false,
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dom = std::fs::read_to_string(&a[1]).unwrap();
    let prob = std::fs::read_to_string(&a[2]).unwrap();
    let d = ferroplan::parser::parse_domain(&dom).unwrap();
    let p = ferroplan::parser::parse_problem(&prob).unwrap();

    println!(
        "requires_concurrency: {}",
        ferroplan::sat::requires_concurrency(&d, &p)
    );

    let mut overall: HashSet<String> = HashSet::new();
    for da in &d.durative_actions {
        for (ts, f) in &da.conditions {
            if *ts == TimeSpec::All {
                collect_pos_atoms(f, &mut overall);
            }
        }
    }
    let init: HashSet<String> = p
        .init_atoms
        .iter()
        .map(|(q, _)| q.to_ascii_uppercase())
        .collect();
    let til: HashSet<String> = p
        .til
        .iter()
        .filter(|t| t.add)
        .map(|t| t.pred.to_ascii_uppercase())
        .collect();

    let mut names: Vec<&String> = overall.iter().collect();
    names.sort();
    println!("over-all predicates: {}", names.len());
    for pred in names {
        let mut env: Vec<&str> = Vec::new();
        let mut other: Vec<String> = Vec::new();
        for act in &d.actions {
            if adds(&act.effect, pred) {
                other.push(format!("{} (classical)", act.name));
            }
        }
        for da in &d.durative_actions {
            let at = |want: TimeSpec, f: &dyn Fn(&Effect, &str) -> bool| {
                da.effects.iter().any(|(ts, e)| *ts == want && f(e, pred))
            };
            let adds_start = at(TimeSpec::Start, &adds);
            let adds_end = at(TimeSpec::End, &adds);
            let dels_end = at(TimeSpec::End, &dels);
            if adds_end {
                other.push(format!("{} (at-end add)", da.name));
            }
            if adds_start {
                let own_overall = da.conditions.iter().any(|(ts, f)| {
                    *ts == TimeSpec::All && {
                        let mut s = HashSet::new();
                        collect_pos_atoms(f, &mut s);
                        s.contains(pred)
                    }
                });
                if dels_end && !own_overall {
                    env.push(&da.name);
                } else {
                    other.push(format!(
                        "{} (at-start add{})",
                        da.name,
                        if dels_end {
                            ", own over-all"
                        } else {
                            ", no end-del"
                        }
                    ));
                }
            }
        }
        let flags = format!(
            "{}{}",
            if init.contains(pred) { " INIT" } else { "" },
            if til.contains(pred) { " TIL" } else { "" },
        );
        let verdict = if !flags.is_empty() {
            "disqualified (pre-exists / exogenous)"
        } else if !env.is_empty() && other.is_empty() {
            "ENVELOPE-ONLY -> detector fires"
        } else if env.is_empty() {
            "no envelope producer"
        } else {
            "mixed producers -> detector quiet"
        };
        println!("  {pred}{flags}: {verdict}");
        for e in env {
            println!("    envelope: {e}");
        }
        for o in other {
            println!("    other:    {o}");
        }
    }
}
