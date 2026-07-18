//! Ground once, replan many — the embedding API for callers that re-solve the same
//! world every tick (a game's villagers, a simulation loop, an agent runtime).
//!
//! [`crate::solve`] re-parses and re-grounds from scratch on every call; for small
//! problems grounding dominates the wall clock, so per-tick replanning pays a large
//! fixed tax for identical work. A [`Session`] parses, compiles `:derived` axioms,
//! and grounds ONCE, then holds the *current world state*: mutate it with
//! [`Session::set_fact`] / [`Session::set_fluent`] as the world evolves and call
//! [`Session::replan`] to solve from wherever the world now stands — paying only
//! the search.
//!
//! ```no_run
//! use ferroplan::{Options, Session};
//! # let (domain_src, problem_src) = (String::new(), String::new());
//! let mut s = Session::new(&domain_src, &problem_src, &Options::default())?;
//! let first = s.replan();                       // plan from the problem's :init
//! s.set_fact("(at v1 field)", true)?;           // the world moved
//! s.set_fluent("(grain)", 3.0)?;
//! let next = s.replan();                        // replan from the current state
//! # Ok::<(), String>(())
//! ```
//!
//! **Scope (v1).** Classical / numeric / ADL domains — the paths whose search reads
//! a grounded task's initial state directly. Temporal domains are rejected (the
//! decision-epoch pipeline re-compiles snap actions per solve; use [`crate::solve`]),
//! as are PDDL3 preference problems (the metric optimizer compiles the problem per
//! solve). The goal is fixed at construction.
//!
//! **Why static facts are rejected.** Grounding enumerates operator parameters
//! restricted by *static* predicates read from `:init` — a static fact flipped
//! after grounding could require operators that were never enumerated. Rather than
//! hand back silently-wrong plans, [`Session::set_fact`] only accepts facts some
//! operator can add or delete (the world's *dynamic* facts) and errors on the rest.
//! Fluent values are all runtime-read, so any grounded fluent may be set.

use crate::api::{stats, steps_of, Mode, Options, Plan, Search, Solution};
use crate::ground::ground_task;
use crate::hash::FxHashMap;
use crate::packed::PackedTask;
use crate::search;

/// A grounded, replannable world. See the module docs.
pub struct Session {
    task: PackedTask,
    threads: usize,
    weight_g: f64,
    weight_h: f64,
    max_evaluated: Option<usize>,
    ehc_first: bool,
    /// Display name (uppercase, e.g. `(AT V1 FIELD)`) -> fact id.
    fact_ids: FxHashMap<String, u32>,
    /// Per fact id: does any operator add or delete it? (Static facts are baked
    /// into the grounding and must not change — see the module docs.)
    dynamic: Vec<bool>,
    /// Display name (uppercase, e.g. `(GRAIN)`) -> fluent id.
    fluent_ids: FxHashMap<String, u32>,
}

impl Session {
    /// Parse, compile `:derived` axioms, and ground once. Errors on parse/grounding
    /// failure, and on temporal or PDDL3-preference inputs (unsupported — see the
    /// module docs).
    pub fn new(domain_src: &str, problem_src: &str, opts: &Options) -> Result<Session, String> {
        let domain = crate::parser::parse_domain(domain_src).map_err(|e| format!("domain: {e}"))?;
        let problem =
            crate::parser::parse_problem(problem_src).map_err(|e| format!("problem: {e}"))?;
        let (domain, problem) = crate::derived::compile(&domain, &problem)?;
        if !domain.constraints.is_empty() || !problem.constraints.is_empty() {
            return Err("Session does not support PDDL3 trajectory constraints yet \
                 (ferroplan::solve enforces the hard untimed ones); use \
                 ferroplan::solve per instance"
                .into());
        }
        if crate::temporal::is_temporal(&domain) {
            return Err(
                "Session does not support temporal (durative-action) domains yet; \
                 use ferroplan::solve per instance"
                    .into(),
            );
        }
        if crate::pddl3::has_preferences(&problem) {
            return Err("Session does not support PDDL3 preference problems yet; \
                 use ferroplan::solve per instance"
                .into());
        }
        let threads = if opts.threads == 0 {
            crate::par::num_threads()
        } else {
            opts.threads
        };
        // Force a task even when the base goal is trivially true/false — a session's
        // world moves, so the base-init verdict says nothing about later replans.
        let task = ground_task(&domain, &problem, threads)
            .ok_or_else(|| "grounding failed (empty type)".to_string())?;

        let fact_ids = task
            .fact_names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i as u32))
            .collect();
        let mut dynamic = vec![false; task.n_facts];
        for oi in 0..task.n_ops {
            for &f in task.add.slice(oi).iter().chain(task.del.slice(oi)) {
                dynamic[f as usize] = true;
            }
            for ce in task.cond_effs(oi) {
                for &f in ce.add.iter().chain(ce.del.iter()) {
                    dynamic[f as usize] = true;
                }
            }
        }
        let fluent_ids = task
            .fluent_names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i as u32))
            .collect();

        Ok(Session {
            task,
            threads,
            weight_g: opts.weight_g,
            weight_h: opts.weight_h,
            max_evaluated: opts.max_evaluated,
            ehc_first: opts.search != Search::BestFirst,
            fact_ids,
            dynamic,
            fluent_ids,
        })
    }

    /// Set a world fact true/false in the current state, e.g.
    /// `set_fact("(at v1 field)", true)`. Case-insensitive. Errors if the fact was
    /// never grounded, or is static (grounding-baked — see the module docs).
    pub fn set_fact(&mut self, name: &str, value: bool) -> Result<(), String> {
        let key = name.to_ascii_uppercase();
        let &id = self
            .fact_ids
            .get(&key)
            .ok_or_else(|| format!("unknown fact `{key}` (not in the grounded task)"))?;
        if !self.dynamic[id as usize] {
            return Err(format!(
                "fact `{key}` is static — grounding baked it in; changing it could \
                 require operators that were never enumerated"
            ));
        }
        let (w, b) = (id as usize / 64, id as usize % 64);
        if value {
            self.task.init_bits[w] |= 1 << b;
        } else {
            self.task.init_bits[w] &= !(1 << b);
        }
        Ok(())
    }

    /// Set a fluent's current value, e.g. `set_fluent("(grain)", 3.0)`.
    /// Case-insensitive. Errors if the fluent was never grounded.
    pub fn set_fluent(&mut self, name: &str, value: f64) -> Result<(), String> {
        let key = name.to_ascii_uppercase();
        let &id = self
            .fluent_ids
            .get(&key)
            .ok_or_else(|| format!("unknown fluent `{key}` (not in the grounded task)"))?;
        self.task.fv0[id as usize] = value;
        self.task.fdef0[id as usize] = true;
        Ok(())
    }

    /// Read a fact in the current state (`None` if it was never grounded).
    pub fn fact(&self, name: &str) -> Option<bool> {
        let &id = self.fact_ids.get(&name.to_ascii_uppercase())?;
        Some((self.task.init_bits[id as usize / 64] >> (id as usize % 64)) & 1 == 1)
    }

    /// Read a fluent's current value (`None` if never grounded or undefined).
    pub fn fluent(&self, name: &str) -> Option<f64> {
        let &id = self.fluent_ids.get(&name.to_ascii_uppercase())?;
        self.task.fdef0[id as usize].then(|| self.task.fv0[id as usize])
    }

    /// Solve from the CURRENT world state toward the session's goal, paying only
    /// the search (no re-parse, no re-ground). Same structured [`Solution`] as
    /// [`crate::solve`]; `solved: false` when the goal is unreachable from here.
    pub fn replan(&self) -> Solution {
        if self.task.goal_met(&self.task.initial()) {
            return Solution {
                solved: true,
                mode: Mode::Ff,
                plan: Some(Plan {
                    steps: Vec::new(),
                    length: 0,
                    metric: None,
                    makespan: None,
                }),
                statistics: stats(&self.task, 0, self.threads),
                notes: vec!["goal already satisfied; the empty plan solves it".into()],
            };
        }
        let cfg = crate::search::SearchCfg::from_weights(
            self.weight_g,
            self.weight_h,
            self.max_evaluated,
        );
        let o = search::plan(&self.task, self.threads, cfg, self.ehc_first);
        let mut notes = Vec::new();
        if o.ehc_fell_back && o.ops.is_some() {
            notes.push("EHC found no improving state; used weighted best-first".into());
        }
        match o.ops {
            Some(ops) => {
                let steps = steps_of(&self.task, &ops, None);
                Solution {
                    solved: true,
                    mode: Mode::Ff,
                    plan: Some(Plan {
                        length: steps.len(),
                        steps,
                        metric: None,
                        makespan: None,
                    }),
                    statistics: stats(&self.task, o.evaluated, self.threads),
                    notes,
                }
            }
            None => Solution {
                solved: false,
                mode: Mode::Ff,
                plan: None,
                statistics: stats(&self.task, o.evaluated, self.threads),
                notes,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOM: &str = "
    (define (domain farm) (:requirements :strips :typing :numeric-fluents)
      (:types agent place)
      (:predicates (at ?a - agent ?p - place) (road ?x ?y - place) (fertile ?p - place))
      (:functions (grain))
      (:action walk :parameters (?a - agent ?from ?to - place)
        :precondition (and (at ?a ?from) (road ?from ?to))
        :effect (and (not (at ?a ?from)) (at ?a ?to)))
      (:action harvest :parameters (?a - agent ?p - place)
        :precondition (and (at ?a ?p) (fertile ?p))
        :effect (increase (grain) 1)))";
    const PRB: &str = "
    (define (problem p) (:domain farm)
      (:objects v1 - agent hut field - place)
      (:init (at v1 hut) (road hut field) (road field hut) (fertile field) (= (grain) 0))
      (:goal (>= (grain) 2)))";

    #[test]
    fn replan_solves_and_tracks_world_state() {
        let mut s = Session::new(DOM, PRB, &Options::default()).expect("session");
        let first = s.replan();
        assert!(first.solved, "base problem solves");
        // walk + harvest x2 = 3 steps
        assert_eq!(first.plan.as_ref().unwrap().steps.len(), 3);

        // The world moved: the villager is already at the field with 1 grain.
        s.set_fact("(at v1 hut)", false).unwrap();
        s.set_fact("(at v1 field)", true).unwrap();
        s.set_fluent("(grain)", 1.0).unwrap();
        assert_eq!(s.fact("(at v1 field)"), Some(true));
        assert_eq!(s.fluent("(grain)"), Some(1.0));

        let next = s.replan();
        assert!(next.solved);
        // one harvest remains
        assert_eq!(next.plan.as_ref().unwrap().steps.len(), 1);
        assert_eq!(next.plan.unwrap().steps[0].action, "HARVEST");
    }

    #[test]
    fn goal_already_met_returns_empty_plan() {
        let mut s = Session::new(DOM, PRB, &Options::default()).expect("session");
        s.set_fluent("(grain)", 5.0).unwrap();
        let sol = s.replan();
        assert!(sol.solved);
        assert_eq!(sol.plan.unwrap().steps.len(), 0);
    }

    #[test]
    fn static_and_unknown_facts_are_rejected() {
        let mut s = Session::new(DOM, PRB, &Options::default()).expect("session");
        // `road` is static: no operator adds or deletes it.
        assert!(s.set_fact("(road hut field)", false).is_err());
        assert!(s.set_fact("(at v1 nowhere)", true).is_err());
        assert!(s.set_fluent("(gold)", 1.0).is_err());
    }

    #[test]
    fn temporal_domains_are_rejected() {
        let dom = "
        (define (domain t) (:requirements :durative-actions)
          (:predicates (done))
          (:durative-action work :parameters ()
            :duration (= ?duration 1) :condition () :effect (at end (done))))";
        let prb = "(define (problem p) (:domain t) (:init) (:goal (done)))";
        assert!(Session::new(dom, prb, &Options::default()).is_err());
    }
}
