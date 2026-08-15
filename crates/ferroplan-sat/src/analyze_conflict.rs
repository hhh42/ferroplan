// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/analyze_conflict.rs); the clause-hash
// plumbing went with the proof seam, `vec_mut_scan` became an explicit
// write-index scan, and the helpers take their parts explicitly.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Learns a new clause by analyzing a conflict.
use std::mem::{swap, take};

use crate::{
    clause::{ClauseAlloc, ClauseRef},
    context::Context,
    decision::vsids::Vsids,
    lit::{Lit, Var},
    prop::{Conflict, ImplGraph, Reason},
};

/// Temporaries for conflict analysis
#[derive(Default)]
pub struct AnalyzeConflict {
    /// This is the learned clause after analysis finishes.
    clause: Vec<Lit>,
    /// Number of literals in the current clause at the current level.
    current_level_count: usize,
    /// Variables in the current clause.
    var_flags: Vec<bool>,
    /// Entries to clean in `var_flags`.
    to_clean: Vec<Var>,
    /// Clauses to bump.
    involved: Vec<ClauseRef>,
    /// Stack for recursive minimization.
    stack: Vec<Lit>,
}

impl AnalyzeConflict {
    /// Update structures for a new variable count.
    pub fn set_var_count(&mut self, count: usize) {
        self.var_flags.resize(count, false);
    }

    /// The learned clause.
    pub fn clause(&self) -> &[Lit] {
        &self.clause
    }

    /// Long clauses involved in the conflict.
    pub fn involved(&self) -> &[ClauseRef] {
        &self.involved
    }
}

/// Learns a new clause by analyzing a conflict.
///
/// Returns the lowest decision level that makes the learned clause asserting.
pub fn analyze_conflict(ctx: &mut Context, conflict: Conflict) -> usize {
    {
        let analyze = &mut ctx.analyze_conflict;
        analyze.clause.clear();
        analyze.involved.clear();
        analyze.current_level_count = 0;
    }

    if ctx.trail.current_level() == 0 {
        // Conflict with no decisions, generate empty clause
        return 0;
    }

    let Context {
        analyze_conflict: analyze,
        clause_alloc: alloc,
        impl_graph,
        trail,
        vsids,
        ..
    } = ctx;

    let current_level = trail.current_level();

    // We start with all the literals of the conflicted clause
    for &lit in conflict.lits(alloc) {
        add_literal(analyze, vsids, impl_graph, current_level, lit);
    }

    if let Conflict::Long(cref) = conflict {
        analyze.involved.push(cref);
    }

    // To get rid of all but one literal of the current level, we resolve the clause with the reason
    // for those literals. The correct order for this is reverse chronological.

    for &lit in trail.trail().iter().rev() {
        let lit_present = &mut analyze.var_flags[lit.index()];
        // Is the lit present in the current clause?
        if *lit_present {
            *lit_present = false;
            analyze.current_level_count -= 1;
            if analyze.current_level_count == 0 {
                // lit is the last literal of the current level present in the current clause,
                // therefore the resulting clause will assert !lit so we put in position 0
                analyze.clause.push(!lit);
                let end = analyze.clause.len() - 1;
                analyze.clause.swap(0, end);

                break;
            } else {
                // We removed the literal and now add its reason.
                let reason = *impl_graph.reason(lit.var());

                for &lit in reason.lits(alloc) {
                    add_literal(analyze, vsids, impl_graph, current_level, lit);
                }

                if let Reason::Long(cref) = reason {
                    analyze.involved.push(cref);
                }
            }
        }
    }

    // This needs var_flags set and keeps some var_flags set.
    minimize_clause(analyze, alloc, impl_graph);

    for var in analyze.to_clean.drain(..) {
        analyze.var_flags[var.index()] = false;
    }

    // We find the highest level literal besides the asserted literal and move it into position 1.
    // This is important to ensure the watchlist constraints are not violated on backtracking.
    let mut backtrack_to = 0;

    if analyze.clause.len() > 1 {
        let (prefix, rest) = analyze.clause.split_at_mut(2);
        let lit_1 = &mut prefix[1];
        backtrack_to = impl_graph.level(lit_1.var());
        for lit in rest.iter_mut() {
            let lit_level = impl_graph.level(lit.var());
            if lit_level > backtrack_to {
                backtrack_to = lit_level;
                swap(lit_1, lit);
            }
        }
    }

    vsids.decay();

    backtrack_to
}

/// Add a literal to the current clause.
fn add_literal(
    analyze: &mut AnalyzeConflict,
    vsids: &mut Vsids,
    impl_graph: &ImplGraph,
    current_level: usize,
    lit: Lit,
) {
    let lit_level = impl_graph.level(lit.var());
    // No need to add literals that are set by unit clauses or already present
    if lit_level > 0 && !analyze.var_flags[lit.index()] {
        vsids.bump(lit.var());

        analyze.var_flags[lit.index()] = true;
        if lit_level == current_level {
            analyze.current_level_count += 1;
        } else {
            analyze.clause.push(lit);
            analyze.to_clean.push(lit.var());
        }
    }
}

/// A Bloom filter of levels.
#[derive(Default)]
struct LevelAbstraction {
    bits: u64,
}

impl LevelAbstraction {
    /// Add a level to the Bloom filter.
    pub fn add(&mut self, level: usize) {
        self.bits |= 1 << (level % 64)
    }

    /// Test whether a level could be in the Bloom filter.
    pub fn test(&self, level: usize) -> bool {
        self.bits & (1 << (level % 64)) != 0
    }
}

/// Performs recursive clause minimization.
///
/// **Note:** Requires AnalyzeConflict's var_flags to be set for exactly the variables of the
/// unminimized clause. This also sets some more var_flags, but lists them in to_clean.
///
/// This routine tries to remove some redundant literals of the learned clause. The idea is to
/// detect literals of the learned clause that are already implied by other literals of the clause.
///
/// This is done by performing a DFS in the implication graph (following edges in reverse) for each
/// literal (apart from the asserting one). The search doesn't expand literals already known to be
/// implied by literals of the clause. When a decision literal that is not in the clause is found,
/// it means that the literal is not redundant.
///
/// There are two optimizations used here: The first one is to stop the search as soon as a literal
/// of a decision level not present in the clause is found. If the DFS would be continued it would
/// at some point reach the decision of that level. That decision belongs to a level not in the
/// clause and thus itself can't be in the clause. Checking whether the decision level is among the
/// clause's decision levels is done approximately using a Bloom filter.
///
/// The other optimization is to avoid duplicating work during the DFS searches. When one literal is
/// found to be redundant that means the whole search stayed within the implied literals. We
/// remember this and will not expand any of these literals for the following DFS searches.
///
/// In this implementation the var_flags array here has two purposes. At the beginning it is set for
/// all the literals of the clause. It is also used to mark the literals visited during the DFS.
/// This allows us to combine the already-visited-check with the literal-present-in-clause check. It
/// also allows for a neat implementation of the second optimization. When the search finds the
/// literal to be non-redundant, we clear var_flags for the literals we visited, resetting it to the
/// state at the beginning of the DFS. When the literal was redundant we keep it as is. This means
/// the following DFS will not expand these literals.
fn minimize_clause(analyze: &mut AnalyzeConflict, alloc: &ClauseAlloc, impl_graph: &ImplGraph) {
    let mut involved_levels = LevelAbstraction::default();

    for &lit in analyze.clause.iter() {
        involved_levels.add(impl_graph.level(lit.var()));
    }

    // Scan the clause in place; we always keep the first (asserting) literal.
    let mut clause = take(&mut analyze.clause);
    let mut write = 1;

    for read in 1..clause.len() {
        let lit = clause[read];
        if !lit_is_redundant(analyze, alloc, impl_graph, &involved_levels, lit) {
            clause[write] = lit;
            write += 1;
        }
    }

    clause.truncate(write);
    analyze.clause = clause;
}

/// The DFS of [`minimize_clause`] for a single literal of the clause.
fn lit_is_redundant(
    analyze: &mut AnalyzeConflict,
    alloc: &ClauseAlloc,
    impl_graph: &ImplGraph,
    involved_levels: &LevelAbstraction,
    lit: Lit,
) -> bool {
    if impl_graph.reason(lit.var()).is_unit() {
        return false;
    }

    // Start the DFS
    analyze.stack.clear();
    analyze.stack.push(!lit);

    // Used to remember which var_flags are set during this DFS
    let top = analyze.to_clean.len();

    while let Some(lit) = analyze.stack.pop() {
        let reason = impl_graph.reason(lit.var());

        for &reason_lit in reason.lits(alloc) {
            let reason_level = impl_graph.level(reason_lit.var());

            if !analyze.var_flags[reason_lit.index()] && reason_level > 0 {
                // We haven't established reason_lit to be redundant, haven't visited it yet and
                // it's not implied by unit clauses.

                if impl_graph.reason(reason_lit.var()).is_unit()
                    || !involved_levels.test(reason_level)
                {
                    // reason_lit is a decision not in the clause or in a decision level known
                    // not to be in the clause. Abort the search.

                    // Reset the var_flags set during _this_ DFS.
                    for lit in analyze.to_clean.drain(top..) {
                        analyze.var_flags[lit.index()] = false;
                    }
                    return false;
                } else {
                    analyze.var_flags[reason_lit.index()] = true;
                    analyze.to_clean.push(reason_lit.var());
                    analyze.stack.push(!reason_lit);
                }
            }
        }
    }

    true
}
