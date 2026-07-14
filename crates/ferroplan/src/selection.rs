//! Exact preference-subset SELECTION for the PDDL3 metric path.
//!
//! The forensics that motivate this module (docs/forensics-tpp.md): on
//! zero-action-cost preference domains, plan quality is decided entirely by
//! WHICH jointly-satisfiable preference subset the end state lands in —
//! SGPlan5's tpp p05 score is the closed-form optimum of that selection, and
//! h-guided search structurally cannot coordinate it (goods5 held at L2
//! purely so goods6 can match it). So: solve the selection exactly as a tiny
//! combinatorial problem, then hand the chosen facts to the planner as a
//! concrete TARGET (see `pddl3::metric_optimize_closure`).
//!
//! Model, built entirely from what compile()/grounding already produce:
//! - A VARIABLE per invariant mutex group touched by a preference disjunct
//!   (domain = the member facts that appear in some disjunct, plus ⊥) and a
//!   boolean variable per ungrouped disjunct fact.
//! - A preference is SATISFIED iff some DNF disjunct (its `P3COLLECT` op's
//!   non-P3 precondition facts) has every fact chosen.
//! - Minimize violated weight, by DFS branch-and-bound: variables ordered by
//!   descending touched weight, values by descending immediately-satisfied
//!   weight, pruned on (violated-so-far ≥ best). A deterministic node cap
//!   keeps the worst case bounded; the best assignment found so far is kept
//!   (the storage p08 class has thousands of instances — the cap is
//!   load-bearing).
//!
//! The returned `bound` is ADMISSIBLE-OPTIMISTIC: per-fact relaxed
//! reachability (implied by grounding) ignores joint resource caps (tpp's
//! market supply, storage's crates-must-sit-somewhere), and ungrouped
//! complement facts are not mutually excluded — the true optimum can never
//! beat it, so `final metric == bound` PROVES optimality. Preferences with a
//! numeric-precondition disjunct or no groupable structure are counted
//! satisfied (keeps the bound admissible) but are never targeted.

use crate::hash::FxHashMap;
use crate::packed::PackedTask;

pub struct Selection {
    /// Chosen facts per selected preference index: `(pref_idx, disjunct facts)`.
    pub chosen: Vec<(usize, Vec<u32>)>,
    /// Admissible-optimistic violated-weight bound for the whole task.
    pub bound: f64,
}

/// One disjunct requirement on a variable: the variable must equal the fact
/// (`positive`) or must NOT equal it (a compiled `(NOT ...)` complement fact —
/// modeled as a ≠-constraint on its positive twin's variable, which is what
/// couples `not (stored g1 level3)` to the stored-level mutex group and lets
/// the solver derive coordinated choices like g5@L2-so-g6-can-match).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Atom {
    var: usize,
    fact: u32,
    positive: bool,
}

struct Pref {
    idx: usize,
    weight: f64,
    disjuncts: Vec<Vec<Atom>>,
}

const NODE_CAP: usize = 200_000;

/// Extract, model, and solve. `dnf` supplies each preference's disjunct fact
/// sets (the caller already extracts them for the guidance/seed machinery).
/// `banned` marks facts the repair loop has established unreachable: their
/// disjuncts are dropped, and a preference with no surviving disjunct counts
/// VIOLATED (the ban encodes ground truth, so the bound stays honest).
pub fn select(
    task: &PackedTask,
    groups: &[Vec<u32>],
    weights: &[f64],
    dnf: &FxHashMap<usize, Vec<Vec<u32>>>,
    banned: &crate::hash::FxHashSet<u32>,
) -> Option<Selection> {
    // fact → variable id: mutex-group index, or a fresh boolean var per
    // ungrouped fact (allocated below on first sight).
    let mut var_of: FxHashMap<u32, usize> = FxHashMap::default();
    for (gi, g) in groups.iter().enumerate() {
        for &f in g {
            var_of.insert(f, gi);
        }
    }
    let mut n_vars = groups.len();
    // `(NOT <p>)` complement fact → its positive twin, by grounded name.
    let mut twin: FxHashMap<u32, u32> = FxHashMap::default();
    {
        let mut by_name: FxHashMap<&str, u32> = FxHashMap::default();
        for (f, name) in task.fact_names.iter().enumerate() {
            by_name.insert(name.as_str(), f as u32);
        }
        for (f, name) in task.fact_names.iter().enumerate() {
            let up = name.to_ascii_uppercase();
            if let Some(inner) = up.strip_prefix("(NOT ").and_then(|s| s.strip_suffix(')')) {
                if let Some(&pf) = by_name.get(inner) {
                    twin.insert(f as u32, pf);
                } else if let Some(&pf) = by_name.get(inner.to_ascii_lowercase().as_str()) {
                    twin.insert(f as u32, pf);
                }
            }
        }
    }
    let fresh = |f: u32, var_of: &mut FxHashMap<u32, usize>, n_vars: &mut usize| -> usize {
        *var_of.entry(f).or_insert_with(|| {
            let v = *n_vars;
            *n_vars += 1;
            v
        })
    };

    // Model each preference; unmodelable ones count satisfied (admissible),
    // banned-out ones count violated (the ban is ground truth).
    let mut prefs: Vec<Pref> = Vec::new();
    let mut forced_violated = 0.0;
    for (i, &weight) in weights.iter().enumerate() {
        let Some(djs) = dnf.get(&i) else {
            continue;
        };
        let mut disjuncts: Vec<Vec<Atom>> = Vec::new();
        let mut trivially_true = false;
        let mut any_banned = false;
        for facts in djs {
            if facts.is_empty() {
                trivially_true = true;
                break;
            }
            if facts
                .iter()
                .any(|f| banned.contains(f) && !twin.contains_key(f))
            {
                any_banned = true;
                continue;
            }
            let mut req: Vec<Atom> = facts
                .iter()
                .map(|&f| match twin.get(&f) {
                    // A complement fact constrains its positive twin's
                    // variable to NOT take the twin's value.
                    Some(&pf) => Atom {
                        var: fresh(pf, &mut var_of, &mut n_vars),
                        fact: pf,
                        positive: false,
                    },
                    None => Atom {
                        var: fresh(f, &mut var_of, &mut n_vars),
                        fact: f,
                        positive: true,
                    },
                })
                .collect();
            req.sort_unstable();
            req.dedup();
            // Drop internally-inconsistent disjuncts: two different Eq values
            // on one variable, or Eq and Neq of the same fact. An Eq that
            // implies a same-variable Neq subsumes it.
            let ok = req.iter().all(|a| {
                req.iter().all(|b| {
                    a == b
                        || a.var != b.var
                        || (a.positive != b.positive && a.fact != b.fact)
                        || (!a.positive && !b.positive)
                })
            });
            if ok {
                let keep: Vec<bool> = req
                    .iter()
                    .map(|a| {
                        a.positive
                            || !req
                                .iter()
                                .any(|b| b.positive && b.var == a.var && b.fact != a.fact)
                    })
                    .collect();
                let mut it = keep.iter();
                req.retain(|_| *it.next().unwrap());
                disjuncts.push(req);
            }
        }
        if trivially_true {
            continue;
        }
        if disjuncts.is_empty() {
            if any_banned {
                forced_violated += weight; // every route runs through a banned fact
            }
            // else: unsatisfiable-by-structure — counted satisfied (optimistic).
            continue;
        }
        prefs.push(Pref {
            idx: i,
            weight,
            disjuncts,
        });
    }
    if prefs.len() < 2 {
        return None;
    }

    // Variable domains = values a positive atom demands, ⊥ implicit (Neq
    // atoms add no values — ⊥ or any other value satisfies them).
    let mut domain: Vec<Vec<u32>> = vec![Vec::new(); n_vars];
    for p in &prefs {
        for d in &p.disjuncts {
            for a in d {
                if a.positive && !domain[a.var].contains(&a.fact) {
                    domain[a.var].push(a.fact);
                }
            }
        }
    }
    for d in &mut domain {
        d.sort_unstable();
    }

    // Order variables by descending total weight of touching preferences.
    let mut touch_w: Vec<f64> = vec![0.0; n_vars];
    for p in &prefs {
        let mut seen: Vec<usize> = p
            .disjuncts
            .iter()
            .flat_map(|d| d.iter().map(|a| a.var))
            .collect();
        seen.sort_unstable();
        seen.dedup();
        for v in seen {
            touch_w[v] += p.weight;
        }
    }
    let mut order: Vec<usize> = (0..n_vars).filter(|&v| !domain[v].is_empty()).collect();
    order.sort_by(|&a, &b| {
        touch_w[b]
            .partial_cmp(&touch_w[a])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });

    // DFS branch-and-bound over assignments. `assign[v]`: None = undecided,
    // Some(0) = ⊥, Some(f+1) = fact f chosen.
    struct Dfs<'a> {
        prefs: &'a [Pref],
        domain: &'a [Vec<u32>],
        order: &'a [usize],
        assign: Vec<Option<u32>>,
        best_cost: f64,
        best_assign: Vec<Option<u32>>,
        nodes: usize,
    }
    impl Dfs<'_> {
        /// Violated weight already forced under the current partial
        /// assignment. An atom is CONTRADICTED when its variable is decided
        /// against it: Eq needs the value, Neq dies only on exactly that
        /// value (⊥ and undecided satisfy Neq).
        fn split(&self) -> (f64, f64) {
            let mut forced = 0.0;
            for p in self.prefs {
                let dead = p.disjuncts.iter().all(|d| {
                    d.iter().any(|a| match self.assign[a.var] {
                        Some(x) if a.positive => x != a.fact + 1,
                        Some(x) => x == a.fact + 1,
                        None => false,
                    })
                });
                if dead {
                    forced += p.weight;
                }
            }
            (forced, 0.0)
        }
        fn go(&mut self, depth: usize) {
            self.nodes += 1;
            if self.nodes > NODE_CAP {
                return;
            }
            let (forced, _) = self.split();
            if forced >= self.best_cost {
                return; // cannot improve
            }
            if depth == self.order.len() {
                self.best_cost = forced;
                self.best_assign = self.assign.clone();
                return;
            }
            let v = self.order[depth];
            // Try each value ordered by immediately-satisfied weight, then ⊥.
            let mut vals: Vec<(f64, u32)> = self.domain[v]
                .iter()
                .map(|&f| {
                    let w: f64 = self
                        .prefs
                        .iter()
                        .filter(|p| {
                            p.disjuncts
                                .iter()
                                .any(|d| d.iter().any(|a| a.positive && a.var == v && a.fact == f))
                        })
                        .map(|p| p.weight)
                        .sum();
                    (w, f)
                })
                .collect();
            vals.sort_by(|a, b| {
                b.0.partial_cmp(&a.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.1.cmp(&b.1))
            });
            for (_, f) in vals {
                self.assign[v] = Some(f + 1);
                self.go(depth + 1);
                if self.nodes > NODE_CAP {
                    return;
                }
            }
            self.assign[v] = Some(0); // ⊥
            self.go(depth + 1);
            self.assign[v] = None;
        }
    }
    let mut dfs = Dfs {
        prefs: &prefs,
        domain: &domain,
        order: &order,
        assign: vec![None; n_vars],
        best_cost: f64::INFINITY,
        best_assign: vec![None; n_vars],
        nodes: 0,
    };
    // Seed the incumbent with the all-⊥ assignment (everything positive
    // violated) so a capped search still returns something.
    dfs.best_cost = prefs.iter().map(|p| p.weight).sum::<f64>() + 1e-9;
    dfs.go(0);

    // Read out the satisfied preferences and their chosen disjuncts.
    let assign = dfs.best_assign;
    let mut chosen: Vec<(usize, Vec<u32>)> = Vec::new();
    let mut bound = forced_violated;
    for p in &prefs {
        let sat = p.disjuncts.iter().find(|d| {
            d.iter().all(|a| {
                if a.positive {
                    matches!(assign[a.var], Some(x) if x == a.fact + 1)
                } else {
                    !matches!(assign[a.var], Some(x) if x == a.fact + 1)
                }
            })
        });
        match sat {
            // Only positive atoms become TARGET facts — a Neq is enforced by
            // the mutex group once the group's chosen value is achieved (or
            // by simply not achieving the fact).
            Some(d) => chosen.push((
                p.idx,
                d.iter().filter(|a| a.positive).map(|a| a.fact).collect(),
            )),
            None => bound += p.weight,
        }
    }
    if chosen.is_empty() {
        return None;
    }
    let _ = task;
    Some(Selection { chosen, bound })
}
