// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/prop/assignment.rs); `enqueue_assignment`
// takes its three parts explicitly (it is the hot leaf every propagation
// path funnels through), the rest take `&mut Context`.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Partial assignment and backtracking.
use crate::{
    context::Context,
    lit::{Lit, LitIdx, Var},
};

use super::{graph::ImplGraph, Reason};

/// Current partial assignment.
#[derive(Default)]
pub struct Assignment {
    assignment: Vec<Option<bool>>,
    last_value: Vec<bool>,
}

/// This compares two `Option<bool>` values as bytes. Workaround for bad code generation.
pub fn fast_option_eq(a: Option<bool>, b: Option<bool>) -> bool {
    unsafe {
        std::mem::transmute::<Option<bool>, u8>(a) == std::mem::transmute::<Option<bool>, u8>(b)
    }
}

impl Assignment {
    /// Update structures for a new variable count.
    pub fn set_var_count(&mut self, count: usize) {
        self.assignment.resize(count, None);
        self.last_value.resize(count, false);
    }

    /// Current partial assignment as slice.
    pub fn assignment(&self) -> &[Option<bool>] {
        &self.assignment
    }

    /// Value assigned to a variable.
    pub fn var_value(&self, var: Var) -> Option<bool> {
        self.assignment[var.index()]
    }

    /// Value last assigned to a variable.
    ///
    /// If the variable is currently assigned this returns the previously assigned value. If the
    /// variable was never assigned this returns false.
    pub fn last_var_value(&self, var: Var) -> bool {
        self.last_value[var.index()]
    }

    /// Value assigned to a literal.
    pub fn lit_value(&self, lit: Lit) -> Option<bool> {
        self.assignment[lit.index()].map(|b| b ^ lit.is_negative())
    }

    pub fn lit_is_true(&self, lit: Lit) -> bool {
        fast_option_eq(self.assignment[lit.index()], Some(lit.is_positive()))
    }

    pub fn lit_is_false(&self, lit: Lit) -> bool {
        fast_option_eq(self.assignment[lit.index()], Some(lit.is_negative()))
    }

    pub fn lit_is_unk(&self, lit: Lit) -> bool {
        fast_option_eq(self.assignment[lit.index()], None)
    }

    pub fn assign_lit(&mut self, lit: Lit) {
        self.assignment[lit.index()] = lit.is_positive().into()
    }

    pub fn unassign_var(&mut self, var: Var) {
        self.assignment[var.index()] = None;
    }

    /// Set the assigned/unassigned value of a variable.
    pub fn set_var(&mut self, var: Var, assignment: Option<bool>) {
        self.assignment[var.index()] = assignment;
    }
}

/// Decision and propagation history.
#[derive(Default)]
pub struct Trail {
    /// Stack of all propagated and all enqueued assignments
    trail: Vec<Lit>,
    /// Next assignment in trail to propagate
    queue_head_pos: usize,
    /// Decision levels as trail indices.
    decisions: Vec<LitIdx>,
    /// Number of unit clauses removed from the trail.
    units_removed: usize,
}

impl Trail {
    /// Return the next assigned literal to propagate.
    pub fn queue_head(&self) -> Option<Lit> {
        self.trail.get(self.queue_head_pos).cloned()
    }

    ///  Return the next assigned literal to propagate and remove it from the queue.
    pub fn pop_queue(&mut self) -> Option<Lit> {
        let head = self.queue_head();
        if head.is_some() {
            self.queue_head_pos += 1;
        }
        head
    }

    /// Assigned literals in assignment order.
    pub fn trail(&self) -> &[Lit] {
        &self.trail
    }

    /// Clear the trail.
    ///
    /// This simply removes all entries without performing any backtracking. Can only be called with
    /// no active decisions.
    pub fn clear(&mut self) {
        assert!(self.decisions.is_empty());
        self.units_removed += self.trail.len();
        self.trail.clear();
        self.queue_head_pos = 0;
    }

    /// Start a new decision level.
    ///
    /// Does not enqueue the decision itself.
    pub fn new_decision_level(&mut self) {
        self.decisions.push(self.trail.len() as LitIdx)
    }

    /// Current decision level.
    pub fn current_level(&self) -> usize {
        self.decisions.len()
    }
}

/// Enqueues the assignment of true to a literal.
///
/// This updates the assignment and trail, but does not perform any propagation. The literal has to
/// be unassigned when calling this.
pub fn enqueue_assignment(
    assignment: &mut Assignment,
    impl_graph: &mut ImplGraph,
    trail: &mut Trail,
    lit: Lit,
    reason: Reason,
) {
    debug_assert!(assignment.lit_value(lit).is_none());

    assignment.assign_lit(lit);

    trail.trail.push(lit);

    let node = &mut impl_graph.nodes[lit.index()];
    node.reason = reason;
    node.level = trail.decisions.len() as LitIdx;
    node.depth = trail.trail.len() as LitIdx;
}

/// Undo all assignments in decision levels deeper than the given level.
pub fn backtrack(ctx: &mut Context, level: usize) {
    let Context {
        assignment,
        trail,
        vsids,
        ..
    } = ctx;

    // level can actually be greater than the current level. This happens when restart is called
    // after a conflict backtracked into the assumptions, but before any assumption was reenqueued.
    if level >= trail.decisions.len() {
        return;
    }

    let new_trail_len = trail.decisions[level] as usize;

    trail.queue_head_pos = new_trail_len;
    trail.decisions.truncate(level);

    let trail_end = &trail.trail[new_trail_len..];
    for &lit in trail_end {
        vsids.make_available(lit.var());
        let var_assignment = &mut assignment.assignment[lit.index()];
        assignment.last_value[lit.index()] = *var_assignment == Some(true);
        *var_assignment = None;
    }
    trail.trail.truncate(new_trail_len);
}

/// Undo all decisions and assumptions.
pub fn full_restart(ctx: &mut Context) {
    ctx.assumptions.full_restart();
    backtrack(ctx, 0);
}

/// Undo all decisions.
pub fn restart(ctx: &mut Context) {
    let level = ctx.assumptions.assumption_levels();
    backtrack(ctx, level);
}
