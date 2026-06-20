//! PDDL2.1 temporal planning — durative actions (EPIC-Temporal).
//!
//! T2 (this module's [`compile`]): each `:durative-action` is split into two
//! instantaneous CLASSICAL actions so the existing grounder/heuristic can be
//! reused. `A-START` takes the action's `at start` conditions as its
//! precondition and applies its `at start` effects plus a `(RUNNING-A ?params)`
//! token; `A-END` requires the `at end` conditions and that token, applies the
//! `at end` effects, and deletes the token.
//!
//! The `over all` invariant and the duration are not expressible in classical
//! STRIPS, so they are kept in a side table ([`SnapInfo`]) that the decision-epoch
//! temporal search (T3) consumes: it only lets `A-END` fire `duration` after the
//! matching `A-START`, and checks the invariant holds across the interval.

use crate::types::{Action, Domain, Effect, Expr, Formula, Problem, Sym, Term, TimeSpec};

/// Temporal metadata for one durative action, paired with its snap-actions.
#[derive(Clone, Debug)]
pub struct SnapInfo {
    /// Name of the generated start snap-action (e.g. `MOVE-START`).
    pub start_action: Sym,
    /// Name of the generated end snap-action (e.g. `MOVE-END`).
    pub end_action: Sym,
    /// `RUNNING-…` token predicate that pairs a start with its end.
    pub running_pred: Sym,
    /// Duration expression (over the action's parameters / fluents).
    pub duration: Expr,
    /// `over all` invariant that must hold across the action's execution.
    pub invariant: Formula,
    /// The action's typed parameters (for grounding the duration/invariant).
    pub params: Vec<(Sym, Sym)>,
}

/// The result of compiling durative actions to classical snap-actions.
pub struct TemporalCompiled {
    /// Domain with `durative_actions` replaced by classical start/end actions.
    pub domain: Domain,
    pub problem: Problem,
    /// One entry per original durative action.
    pub snaps: Vec<SnapInfo>,
}

/// Does this domain use durative actions (i.e. is it a temporal problem)?
pub fn is_temporal(domain: &Domain) -> bool {
    !domain.durative_actions.is_empty()
}

fn and_formulas(parts: Vec<Formula>) -> Formula {
    match parts.len() {
        0 => Formula::True,
        1 => parts.into_iter().next().unwrap(),
        _ => Formula::And(parts),
    }
}

fn and_effects(mut parts: Vec<Effect>) -> Effect {
    if parts.len() == 1 {
        parts.pop().unwrap()
    } else {
        Effect::And(parts)
    }
}

fn pick_conditions(da: &crate::types::DurativeAction, when: TimeSpec) -> Formula {
    and_formulas(
        da.conditions
            .iter()
            .filter(|(t, _)| *t == when)
            .map(|(_, f)| f.clone())
            .collect(),
    )
}

fn pick_effects(da: &crate::types::DurativeAction, when: TimeSpec) -> Vec<Effect> {
    da.effects
        .iter()
        .filter(|(t, _)| *t == when)
        .map(|(_, e)| e.clone())
        .collect()
}

/// Compile a temporal domain (durative actions) into a classical domain of
/// snap-actions plus the [`SnapInfo`] side table.
pub fn compile(domain: &Domain, problem: &Problem) -> TemporalCompiled {
    let mut d = domain.clone();
    let mut snaps = Vec::new();

    for da in &domain.durative_actions {
        let running = format!("RUNNING-{}", da.name);
        let start_name = format!("{}-START", da.name);
        let end_name = format!("{}-END", da.name);
        let run_args: Vec<Term> = da
            .params
            .iter()
            .map(|(p, _)| Term::Var(p.clone()))
            .collect();
        let run_types: Vec<Sym> = da.params.iter().map(|(_, t)| t.clone()).collect();

        d.predicates.push((running.clone(), run_types));

        // start snap: at-start conditions -> effect = at-start effects + token
        let mut start_eff = pick_effects(da, TimeSpec::Start);
        start_eff.push(Effect::Add(running.clone(), run_args.clone()));
        d.actions.push(Action {
            name: start_name.clone(),
            params: da.params.clone(),
            precond: pick_conditions(da, TimeSpec::Start),
            effect: and_effects(start_eff),
        });

        // end snap: at-end conditions + token -> at-end effects, drop token
        let end_pre = and_formulas(vec![
            pick_conditions(da, TimeSpec::End),
            Formula::Atom(running.clone(), run_args.clone()),
        ]);
        let mut end_eff = pick_effects(da, TimeSpec::End);
        end_eff.push(Effect::Del(running.clone(), run_args.clone()));
        d.actions.push(Action {
            name: end_name.clone(),
            params: da.params.clone(),
            precond: end_pre,
            effect: and_effects(end_eff),
        });

        snaps.push(SnapInfo {
            start_action: start_name,
            end_action: end_name,
            running_pred: running,
            duration: da.duration.clone(),
            invariant: pick_conditions(da, TimeSpec::All),
            params: da.params.clone(),
        });
    }

    d.durative_actions.clear(); // now expressed as classical snap-actions
    TemporalCompiled {
        domain: d,
        problem: problem.clone(),
        snaps,
    }
}
