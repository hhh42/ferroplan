//! Grounding into the data-oriented `PackedTask`.
//!
//! Phase B (the expensive cartesian binding enumeration + DNF + effect
//! instantiation) runs in parallel across actions via scoped threads, each
//! producing string-form `RawOp`s without touching shared state. Phase C
//! (interning, negative-precondition compilation, defined-fluent/illegal
//! pruning, relaxed reachability, goal simplification, CSR packing) is a fast
//! sequential merge.

use std::collections::{HashMap, HashSet};

use crate::bitset;
use crate::hash::{FxHashMap, FxHashSet};
use crate::packed::{CondEff, CsrBuilder, PackedTask, State};
use crate::par;
use crate::types::*;

#[allow(clippy::large_enum_variant)]
pub enum Outcome {
    Task(PackedTask),
    GoalTrue,
    /// The goal is provably unsatisfiable; the payload NAMES the mechanism
    /// (0.19 Phase 1: three whole 2018 domains returned `solved: false`
    /// with zero facts and no explanation — a silent verdict is a bug in
    /// itself, whatever else is wrong).
    GoalFalse(String),
    /// A numeric goal reads a fluent with no defined value; payload is the
    /// fluent's display.
    GoalUndefinedFluent(String),
    EmptyType {
        kind: &'static str,
        pred: String,
        ty: String,
    },
    /// Grounding hit a declared budget and stopped honestly — the armed
    /// `FF_TIME_LIMIT` mid-DNF-expansion or mid-binding-enumeration, or
    /// the byte budget on a DNF balloon (0.22 Phases 1+2). NOT a verdict
    /// on the task — callers report "budget exhausted", never
    /// "unsolvable", and NEVER receive a partial task. The receipts that
    /// demanded it: block-grouping i3's 21 disjunctive coordinate-Eq
    /// goals DNF-multiply to 4^21 conjuncts (76 s past a 60 s wall, every
    /// stack sample inside `and_merge`); 2048 spent 67–74 s of a 60 s
    /// budget in binding enumeration with no clock check at all. Raised
    /// only on plain solve entries; the validator/session/temporal
    /// entries never trip — a plan already found must still be
    /// groundable for verification after the wall.
    WallExhausted(String),
}

// ----- DNF over ground formulas (string atoms) -----------------------------

/// The goal-DNF wall check (0.22 Phase 1, Phase 2's clock discipline
/// extended to the one pre-search loop with no driver above it): armed
/// only around the GOAL `to_dnf` call — action grounding runs in
/// parallel elsewhere and has its own phase's checks — and probed every
/// [`DNF_WALL_STRIDE`] produced conjuncts inside `and_merge`, where the
/// cartesian product actually balloons. Thread-local because grounding
/// tests run concurrently in one process; zero cost while unarmed.
struct DnfWall {
    armed: bool,
    hit: bool,
    produced: usize,
    /// The declared retained-byte budget (`FF_MEM_BUDGET_GB` /
    /// RLIMIT_AS, the search's own model), snapshot at arm time.
    /// `usize::MAX` when nothing is declared.
    budget_bytes: usize,
}
thread_local! {
    static DNF_WALL: std::cell::RefCell<DnfWall> = const {
        std::cell::RefCell::new(DnfWall {
            armed: false,
            hit: false,
            produced: 0,
            budget_bytes: usize::MAX,
        })
    };
}
const DNF_WALL_STRIDE: usize = 1 << 16;

/// True once the armed wall has expired (checked at stride boundaries).
fn dnf_wall_hit() -> bool {
    DNF_WALL.with(|w| {
        let mut w = w.borrow_mut();
        if !w.armed {
            return false;
        }
        if !w.hit {
            w.produced += 1;
            if w.produced % DNF_WALL_STRIDE == 0
                && crate::search::wall_remaining_secs() == Some(0.0)
            {
                w.hit = true;
            }
        }
        w.hit
    })
}

/// The predictive arm of the same check: a product level whose ESTIMATED
/// build alone outruns the remaining wall is refused before it starts.
/// The stride check alone fires mid-level and leaves a partial product
/// whose DROP is as wide as its build — measured 20 s of tail past a 15 s
/// wall (and 2 min past a 60 s one) on block-grouping i3, because each
/// cloned conjunct carries every literal accumulated so far. The estimate
/// is therefore denominated in LITERALS (products x conjunct width), at a
/// measured ~60 ns per cloned literal on this class of box, doubled for
/// the eventual drop. Unarmed (no `FF_TIME_LIMIT`): never fires.
fn dnf_wall_doomed(est_products: usize, width: usize) -> bool {
    DNF_WALL.with(|w| {
        let mut w = w.borrow_mut();
        if !w.armed || w.hit {
            return w.armed && w.hit;
        }
        if let Some(remaining) = crate::search::wall_remaining_secs() {
            let est_secs = est_products as f64 * (width.max(1) as f64) * 120e-9;
            if remaining < est_secs {
                w.hit = true;
                return true;
            }
        }
        // The byte-model arm (same discipline, the other currency): a
        // declared FF_MEM_BUDGET_GB used to be enforced only in search,
        // so a 257-or-goal DNF (block-grouping i13/i20) ballooned until
        // the runner's RSS watchdog killed it from outside. ~100 B is a
        // floor per cloned literal (three vecs + heap Exprs); a product
        // level whose floor alone exceeds the retained share can never
        // pack into a task under it.
        let est_bytes = est_products
            .saturating_mul(width.max(1))
            .saturating_mul(100);
        if est_bytes > w.budget_bytes {
            w.hit = true;
            return true;
        }
        false
    })
}

struct Conjunct {
    pos: Vec<(Sym, Vec<Sym>)>,
    neg: Vec<(Sym, Vec<Sym>)>,
    num: Vec<(CompOp, Expr, Expr)>,
}

fn subst_term(t: &Term, b: &HashMap<Sym, Sym>) -> Sym {
    match t {
        Term::Const(c) => c.clone(),
        Term::Var(v) => b.get(v).cloned().unwrap_or_else(|| v.clone()),
    }
}
fn subst_args(args: &[Term], b: &HashMap<Sym, Sym>) -> Vec<Sym> {
    args.iter().map(|t| subst_term(t, b)).collect()
}
fn neg_comp(op: CompOp) -> CompOp {
    match op {
        CompOp::Lt => CompOp::Ge,
        CompOp::Le => CompOp::Gt,
        CompOp::Gt => CompOp::Le,
        CompOp::Ge => CompOp::Lt,
        CompOp::Eq => CompOp::Eq,
    }
}
fn subst_expr(e: &Expr, b: &HashMap<Sym, Sym>) -> Expr {
    match e {
        Expr::Num(n) => Expr::Num(*n),
        Expr::Fluent(f, a) => Expr::Fluent(
            f.clone(),
            a.iter().map(|t| Term::Const(subst_term(t, b))).collect(),
        ),
        Expr::Add(x, y) => Expr::Add(Box::new(subst_expr(x, b)), Box::new(subst_expr(y, b))),
        Expr::Sub(x, y) => Expr::Sub(Box::new(subst_expr(x, b)), Box::new(subst_expr(y, b))),
        Expr::Mul(x, y) => Expr::Mul(Box::new(subst_expr(x, b)), Box::new(subst_expr(y, b))),
        Expr::Div(x, y) => Expr::Div(Box::new(subst_expr(x, b)), Box::new(subst_expr(y, b))),
        Expr::Neg(x) => Expr::Neg(Box::new(subst_expr(x, b))),
    }
}

fn empty_conj() -> Conjunct {
    Conjunct {
        pos: vec![],
        neg: vec![],
        num: vec![],
    }
}

fn merge_conj(a: &Conjunct, b: &Conjunct) -> Conjunct {
    Conjunct {
        pos: a.pos.iter().chain(&b.pos).cloned().collect(),
        neg: a.neg.iter().chain(&b.neg).cloned().collect(),
        num: a.num.iter().chain(&b.num).cloned().collect(),
    }
}

/// AND-combine two DNF lists (cartesian product of conjuncts). Under an
/// armed goal-DNF wall the product truncates the moment the wall expires —
/// the caller reads the hit flag and reports budget, never a verdict.
fn and_merge(acc: &[Conjunct], cd: &[Conjunct]) -> Vec<Conjunct> {
    let est = acc.len().saturating_mul(cd.len());
    let width = acc
        .first()
        .map(|c| c.pos.len() + c.neg.len() + c.num.len())
        .unwrap_or(0)
        + cd.first()
            .map(|c| c.pos.len() + c.neg.len() + c.num.len())
            .unwrap_or(0);
    if est > DNF_WALL_STRIDE && dnf_wall_doomed(est, width) {
        return Vec::new();
    }
    let mut next = Vec::with_capacity(est.min(1 << 20));
    for a in acc {
        for c in cd {
            if dnf_wall_hit() {
                return next;
            }
            next.push(Conjunct {
                pos: a.pos.iter().chain(&c.pos).cloned().collect(),
                neg: a.neg.iter().chain(&c.neg).cloned().collect(),
                num: a.num.iter().chain(&c.num).cloned().collect(),
            });
        }
    }
    next
}

/// Expand a quantifier over typed objects: AND the per-binding DNFs (universal)
/// or OR them (existential). Empty domain -> True (AND) / False (OR), vacuously.
#[allow(clippy::too_many_arguments)]
fn quant_expand(
    vars: &[(Sym, Sym)],
    inner: &Formula,
    b: &HashMap<Sym, Sym>,
    neg: bool,
    objs: &HashMap<Sym, Vec<Sym>>,
    use_and: bool,
    st: Option<&DnfStatics>,
) -> Vec<Conjunct> {
    let mut combos: Vec<HashMap<Sym, Sym>> = vec![b.clone()];
    for (v, ty) in vars {
        let dom: &[Sym] = objs.get(ty).map(|x| x.as_slice()).unwrap_or(&[]);
        let mut next = Vec::new();
        for c in &combos {
            for o in dom {
                let mut e = c.clone();
                e.insert(v.clone(), o.clone());
                next.push(e);
            }
        }
        combos = next;
    }
    if use_and {
        let mut acc = vec![empty_conj()];
        for cb in &combos {
            acc = and_merge(&acc, &to_dnf(inner, cb, neg, objs, st));
        }
        acc
    } else {
        let mut acc = Vec::new();
        for cb in &combos {
            let d = to_dnf(inner, cb, neg, objs, st);
            // existential over a statically-true binding: the whole
            // disjunction is True (same absorption as the Or arm)
            if st.is_some() && d.iter().any(ctx_empty) {
                return vec![empty_conj()];
            }
            acc.extend(d);
        }
        acc
    }
}

/// Static-resolution context for DNF expansion: a fully-bound literal on a
/// STATIC predicate (never added by any action) resolves to True/False
/// against init DURING expansion, and a disjunction with a True disjunct
/// collapses to True. This kills the 2^k conjunct explosion of
/// `forall (imply (static ...) (dynamic ...))` preconditions —
/// openstacks-ADL's make-product/ship-order ground instance-5 to 45k
/// redundant ops and instance-7 to 15 GB RSS and death; collapsed, the DNF
/// is a single conjunct. `None` (the `FF_NO_DNF_STATIC=1` hatch) restores
/// the raw expansion.
struct DnfStatics<'a> {
    init: &'a HashSet<(Sym, Vec<Sym>)>,
    add_preds: &'a HashSet<Sym>,
    /// Predicates some action/monitor DELETES. Folding a literal AWAY (to
    /// True) needs full inertia — never added AND never deleted (a
    /// delete-only predicate like the END construction's TRAJ-PLANNING is
    /// init-true yet must keep gating after its delete fires). Dropping a
    /// conjunct (to False) only needs "can never become true" (never added).
    del_preds: &'a HashSet<Sym>,
}

fn to_dnf(
    f: &Formula,
    b: &HashMap<Sym, Sym>,
    negated: bool,
    objs: &HashMap<Sym, Vec<Sym>>,
    st: Option<&DnfStatics>,
) -> Vec<Conjunct> {
    match (f, negated) {
        (Formula::True, false) | (Formula::False, true) => vec![empty_conj()],
        (Formula::False, false) | (Formula::True, true) => vec![],
        (Formula::Atom(p, a), neg) => {
            // Static resolution: decidable now iff the predicate is static and
            // every arg is a constant or a bound variable (an unbound —
            // quantified-elsewhere — variable falls through to the literal).
            if let Some(stx) = st {
                let bound = |t: &Term| match t {
                    Term::Const(_) => true,
                    Term::Var(v) => b.contains_key(v),
                };
                if !stx.add_preds.contains(p) && a.iter().all(bound) {
                    let truth = stx.init.contains(&(p.clone(), subst_args(a, b)));
                    let deletable = stx.del_preds.contains(p);
                    match (truth, neg) {
                        // literal can never become true -> conjunct dies /
                        // negation holds forever (delete-independent)
                        (false, false) => return vec![],
                        (false, true) => return vec![empty_conj()],
                        // init-true folds only under full inertia
                        (true, false) if !deletable => return vec![empty_conj()],
                        (true, true) if !deletable => return vec![],
                        _ => {} // init-true but deletable: keep the literal
                    }
                }
            }
            if !neg {
                vec![Conjunct {
                    pos: vec![(p.clone(), subst_args(a, b))],
                    neg: vec![],
                    num: vec![],
                }]
            } else {
                vec![Conjunct {
                    pos: vec![],
                    neg: vec![(p.clone(), subst_args(a, b))],
                    num: vec![],
                }]
            }
        }
        // `(not (= e1 e2))` has no single comparator: it is the DISJUNCTION
        // e1 < e2 OR e1 > e2, i.e. two DNF conjuncts.
        (Formula::Comp(CompOp::Eq, l, r), true) => {
            let le = subst_expr(l, b);
            let re = subst_expr(r, b);
            vec![
                Conjunct {
                    pos: vec![],
                    neg: vec![],
                    num: vec![(CompOp::Lt, le.clone(), re.clone())],
                },
                Conjunct {
                    pos: vec![],
                    neg: vec![],
                    num: vec![(CompOp::Gt, le, re)],
                },
            ]
        }
        (Formula::Comp(op, l, r), neg) => {
            let op = if neg { neg_comp(*op) } else { *op };
            vec![Conjunct {
                pos: vec![],
                neg: vec![],
                num: vec![(op, subst_expr(l, b), subst_expr(r, b))],
            }]
        }
        // object equality: resolve both terms and decide statically
        (Formula::Eq(x, y), neg) => {
            let eq = subst_term(x, b) == subst_term(y, b);
            if eq != neg {
                vec![empty_conj()] // True
            } else {
                vec![] // False
            }
        }
        (Formula::Forall(vars, inner), neg) => quant_expand(vars, inner, b, neg, objs, !neg, st),
        (Formula::Exists(vars, inner), neg) => quant_expand(vars, inner, b, neg, objs, neg, st),
        // A preference is a SOFT goal — a classical planner ignores it (True).
        (Formula::Pref(_, _), false) => vec![empty_conj()],
        (Formula::Pref(_, _), true) => vec![],
        (Formula::Not(inner), neg) => to_dnf(inner, b, !neg, objs, st),
        (Formula::And(fs), false) | (Formula::Or(fs), true) => {
            let mut acc = vec![empty_conj()];
            for child in fs {
                acc = and_merge(&acc, &to_dnf(child, b, negated, objs, st));
            }
            acc
        }
        (Formula::Or(fs), false) | (Formula::And(fs), true) => {
            let mut acc = Vec::new();
            for child in fs {
                let d = to_dnf(child, b, negated, objs, st);
                // A True disjunct absorbs the whole disjunction — without this
                // the statically-true branches of `imply` multiply into 2^k
                // subsumed conjuncts under an enclosing forall/and.
                if st.is_some() && d.iter().any(ctx_empty) {
                    return vec![empty_conj()];
                }
                acc.extend(d);
            }
            acc
        }
    }
}

/// String-form ground effect.
/// A grounded conditional effect (string form): apply add/del/num iff the
/// condition holds in the source state.
#[derive(Clone)]
struct RCondEff {
    cond_pos: Vec<(Sym, Vec<Sym>)>,
    cond_neg: Vec<(Sym, Vec<Sym>)>,
    cond_num: Vec<(CompOp, Expr, Expr)>,
    add: Vec<(Sym, Vec<Sym>)>,
    del: Vec<(Sym, Vec<Sym>)>,
    num: Vec<(AssignOp, Sym, Vec<Sym>, Expr)>,
}

struct REff {
    add: Vec<(Sym, Vec<Sym>)>,
    del: Vec<(Sym, Vec<Sym>)>,
    num: Vec<(AssignOp, Sym, Vec<Sym>, Expr)>,
    cond: Vec<RCondEff>,
}

fn ctx_empty(c: &Conjunct) -> bool {
    c.pos.is_empty() && c.neg.is_empty() && c.num.is_empty()
}

/// Ground an effect tree, carrying the accumulated when-condition `ctx`.
/// `forall` expands over objects; `when` DNF-expands its condition (each disjunct
/// becomes a separate conditional effect); leaves under a non-empty condition
/// become conditional effects, else unconditional.
fn ground_effect(
    e: &Effect,
    b: &HashMap<Sym, Sym>,
    objs: &HashMap<Sym, Vec<Sym>>,
    ctx: &Conjunct,
    out: &mut REff,
    st: Option<&DnfStatics>,
) {
    let emit_add = |out: &mut REff, atom: (Sym, Vec<Sym>)| {
        if ctx_empty(ctx) {
            out.add.push(atom);
        } else {
            out.cond.push(RCondEff {
                cond_pos: ctx.pos.clone(),
                cond_neg: ctx.neg.clone(),
                cond_num: ctx.num.clone(),
                add: vec![atom],
                del: vec![],
                num: vec![],
            });
        }
    };
    match e {
        Effect::And(v) => v
            .iter()
            .for_each(|x| ground_effect(x, b, objs, ctx, out, st)),
        Effect::Add(p, a) => emit_add(out, (p.clone(), subst_args(a, b))),
        Effect::Del(p, a) => {
            let atom = (p.clone(), subst_args(a, b));
            if ctx_empty(ctx) {
                out.del.push(atom);
            } else {
                out.cond.push(RCondEff {
                    cond_pos: ctx.pos.clone(),
                    cond_neg: ctx.neg.clone(),
                    cond_num: ctx.num.clone(),
                    add: vec![],
                    del: vec![atom],
                    num: vec![],
                });
            }
        }
        Effect::Num(op, f, a, val) => {
            let ne = (*op, f.clone(), subst_args(a, b), subst_expr(val, b));
            if ctx_empty(ctx) {
                out.num.push(ne);
            } else {
                out.cond.push(RCondEff {
                    cond_pos: ctx.pos.clone(),
                    cond_neg: ctx.neg.clone(),
                    cond_num: ctx.num.clone(),
                    add: vec![],
                    del: vec![],
                    num: vec![ne],
                });
            }
        }
        Effect::Forall(vars, inner) => {
            // expand the universal effect over object combinations
            let mut combos: Vec<HashMap<Sym, Sym>> = vec![b.clone()];
            for (v, ty) in vars {
                let dom: &[Sym] = objs.get(ty).map(|x| x.as_slice()).unwrap_or(&[]);
                let mut next = Vec::new();
                for c in &combos {
                    for o in dom {
                        let mut e2 = c.clone();
                        e2.insert(v.clone(), o.clone());
                        next.push(e2);
                    }
                }
                combos = next;
            }
            for cb in &combos {
                ground_effect(inner, cb, objs, ctx, out, st);
            }
        }
        Effect::When(cond, inner) => {
            // each DNF disjunct of the condition is one conditional context
            for disj in to_dnf(cond, b, false, objs, st) {
                let merged = merge_conj(ctx, &disj);
                ground_effect(inner, b, objs, &merged, out, st);
            }
        }
    }
}

fn static_top_atoms(f: &Formula, add_preds: &HashSet<Sym>) -> Vec<(Sym, Vec<Term>)> {
    let mut out = Vec::new();
    fn rec(f: &Formula, add_preds: &HashSet<Sym>, out: &mut Vec<(Sym, Vec<Term>)>) {
        match f {
            Formula::And(v) => v.iter().for_each(|x| rec(x, add_preds, out)),
            Formula::Atom(p, a) if !add_preds.contains(p) => {
                out.push((p.clone(), a.clone()));
            }
            _ => {}
        }
    }
    rec(f, add_preds, &mut out);
    out
}

/// Top-level positive conjunct atoms whose predicate IS in `preds` — the
/// stratum-2 gating literals of the stratified grounding (see `ground_v`).
fn gating_top_atoms(f: &Formula, preds: &HashSet<Sym>) -> Vec<(Sym, Vec<Term>)> {
    let mut out = Vec::new();
    fn rec(f: &Formula, preds: &HashSet<Sym>, out: &mut Vec<(Sym, Vec<Term>)>) {
        match f {
            Formula::And(v) => v.iter().for_each(|x| rec(x, preds, out)),
            Formula::Atom(p, a) if preds.contains(p) => {
                out.push((p.clone(), a.clone()));
            }
            _ => {}
        }
    }
    rec(f, preds, &mut out);
    out
}

/// A grounded operator in string form (produced in parallel, interned later).
struct RawOp {
    display: String,
    pos: Vec<(Sym, Vec<Sym>)>,
    neg: Vec<(Sym, Vec<Sym>)>,
    num_pre: Vec<(CompOp, Expr, Expr)>,
    eff: REff,
    multi: bool,
    /// This op applies the domain's shared monitor block (0.8 Phase 2).
    monitored: bool,
}

/// The armed wall, shared across Phase B's parallel workers (0.22
/// Phase 2 lever 1): 2048 spends 67–74 s of a 60 s budget inside the
/// binding enumeration with no clock check at all (solo receipts,
/// docs/roadmap-0.22.md). Each worker counts binding nodes through a
/// [`WallTick`] and reads the clock every 8192; the first to see the
/// deadline flips `tripped` and every enumeration unwinds. The caller
/// then returns [`Outcome::WallExhausted`] — never a partial task.
struct GroundWall {
    clock: crate::clock::Clock,
    total: f64,
    tripped: std::sync::atomic::AtomicBool,
}

impl GroundWall {
    fn tripped(&self) -> bool {
        self.tripped.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// One worker's counting handle on the shared [`GroundWall`]. `None`
/// wall ⇒ every check is two instructions and the enumeration is
/// byte-identical to the unchecked shape.
struct WallTick<'a> {
    wall: Option<&'a GroundWall>,
    n: u32,
}

impl WallTick<'_> {
    const STRIDE: u32 = 8192;

    #[inline]
    fn tripped(&mut self) -> bool {
        use std::sync::atomic::Ordering::Relaxed;
        let Some(w) = self.wall else { return false };
        self.n = self.n.wrapping_add(1);
        if self.n & (Self::STRIDE - 1) != 0 {
            return false;
        }
        if w.tripped.load(Relaxed) {
            return true;
        }
        if w.clock.elapsed_secs() >= w.total {
            w.tripped.store(true, Relaxed);
            return true;
        }
        false
    }
}

/// Enumerate parameter bindings in row-major (natural declaration) order,
/// pruning a whole subtree as soon as a STATIC precondition literal has all
/// its variables bound and fails against init. This is the join-style
/// grounding that makes grid-coordinate domains tractable: tidybot11's
/// 9-parameter actions over `sum-x/sum-y/leftof` statics enumerate ~10^8
/// raw bindings under the plain cartesian product (91 s to ground p01) but
/// only thousands survive — checking each static at the FIRST level where
/// it is fully bound visits the survivors' prefixes only (p01: 0.2 s).
/// The visiting ORDER of surviving bindings is identical to the plain
/// product's (pruning only skips bindings the post-filter would reject), so
/// the emitted op sequence — and every downstream tie-break — is
/// byte-identical.
///
/// `tick`: the wall checkpoint handle — a tripped wall abandons the
/// remaining subtree (the partial output is discarded wholesale by the
/// caller, so the abort point never shapes a task).
fn for_each_binding(
    params: &[(Sym, Sym)],
    domains: &[Vec<Sym>],
    static_lits: &[(Sym, Vec<Term>)],
    init_atom_set: &HashSet<(Sym, Vec<Sym>)>,
    tick: &mut WallTick,
    mut f: impl FnMut(&HashMap<Sym, Sym>),
) {
    if domains.iter().any(|d| d.is_empty()) {
        return;
    }
    // For each static literal: the highest param index among its variables
    // (the level where it becomes fully bound). Literals over constants only
    // (or over no params — impossible for well-formed input, treated alike)
    // are checked once, up front.
    let param_pos = |v: &Sym| params.iter().position(|(pv, _)| pv == v);
    let mut lits_at: Vec<Vec<&(Sym, Vec<Term>)>> = vec![Vec::new(); params.len()];
    for lit in static_lits {
        let mut level: Option<usize> = None;
        let mut all_known = true;
        for t in &lit.1 {
            if let Term::Var(v) = t {
                match param_pos(v) {
                    Some(k) => level = Some(level.map_or(k, |l: usize| l.max(k))),
                    None => all_known = false, // quantified/unknown var: post-check only
                }
            }
        }
        match (level, all_known) {
            (Some(k), true) => lits_at[k].push(lit),
            // fully ground literal: decide the whole action here
            (None, true)
                if !init_atom_set
                    .contains(&(lit.0.clone(), subst_args(&lit.1, &HashMap::new()))) =>
            {
                return;
            }
            _ => {} // not decidable during enumeration; the caller's post-filter has it
        }
    }
    let mut binding: HashMap<Sym, Sym> = HashMap::new();
    // Returns false iff the wall tripped — the whole recursion unwinds.
    #[allow(clippy::too_many_arguments)]
    fn rec(
        k: usize,
        params: &[(Sym, Sym)],
        domains: &[Vec<Sym>],
        lits_at: &[Vec<&(Sym, Vec<Term>)>],
        init: &HashSet<(Sym, Vec<Sym>)>,
        binding: &mut HashMap<Sym, Sym>,
        tick: &mut WallTick,
        f: &mut impl FnMut(&HashMap<Sym, Sym>),
    ) -> bool {
        if tick.tripped() {
            return false;
        }
        if k == params.len() {
            f(binding);
            return true;
        }
        let var = &params[k].0;
        for o in &domains[k] {
            binding.insert(var.clone(), o.clone());
            let ok = lits_at[k]
                .iter()
                .all(|lit| init.contains(&(lit.0.clone(), subst_args(&lit.1, binding))));
            if ok && !rec(k + 1, params, domains, lits_at, init, binding, tick, f) {
                return false;
            }
        }
        binding.remove(var);
        true
    }
    rec(
        0,
        params,
        domains,
        &lits_at,
        init_atom_set,
        &mut binding,
        tick,
        &mut f,
    );
}

/// Phase B (parallelisable): all ground RawOps for a single action.
///
/// `extra_join_lits` / `join_atoms`: the stratified-grounding hook. For a
/// stratum-2 action, `extra_join_lits` are its gating literals (positive
/// preconds on producer-known predicates) and `join_atoms` is init PLUS the
/// atoms stratum 1 actually produced, so the gating literals prune binding
/// subtrees exactly like statics do. Static predicates are disjoint from
/// producer predicates (statics are never added), so checking BOTH literal
/// kinds against the union set is exact. Stratum-1 callers pass `&[]` and
/// the plain init set — byte-identical to the unstratified path.
#[allow(clippy::too_many_arguments)]
fn ground_action(
    action: &Action,
    objects_of_type: &HashMap<Sym, Vec<Sym>>,
    init_unary: &FxHashMap<Sym, FxHashSet<Sym>>,
    join_atoms: &HashSet<(Sym, Vec<Sym>)>,
    add_predicates: &HashSet<Sym>,
    del_predicates: &HashSet<Sym>,
    extra_join_lits: &[(Sym, Vec<Term>)],
    dnf_static: bool,
    skip_bindings: Option<&FxHashSet<Vec<Sym>>>,
    wall: Option<&GroundWall>,
) -> Vec<RawOp> {
    let static_lits = static_top_atoms(&action.precond, add_predicates);
    let param_vars: Vec<Sym> = action.params.iter().map(|(v, _)| v.clone()).collect();
    // Restrict each parameter's domain by its STATIC UNARY preconditions before
    // enumerating bindings: a precond `(P ?x)` with P static (never added by any
    // action) means ?x must be an object with `(P ?x)` in init. This avoids
    // enumerating the full cartesian product over an untyped `object` domain
    // (e.g. gripper: 154^3 instead of 150*2*2). The post-filter below still
    // checks every static literal, so the set of ground ops is identical.
    let mut domains: Vec<Vec<Sym>> = action
        .params
        .iter()
        .map(|(_, ty)| objects_of_type.get(ty).cloned().unwrap_or_default())
        .collect();
    for (p, pargs) in &static_lits {
        if pargs.len() == 1 {
            if let Term::Var(v) = &pargs[0] {
                if let Some(pos) = param_vars.iter().position(|pv| pv == v) {
                    match init_unary.get(p) {
                        Some(allowed) => domains[pos].retain(|o| allowed.contains(o)),
                        None => domains[pos].clear(),
                    }
                }
            }
        }
    }
    // Gating literals join the static list AFTER the unary-domain restriction
    // above (that map is init-derived; a no-init gating predicate must not
    // clear a domain) but BEFORE enumeration, so they prune subtrees too.
    let mut join_lits = static_lits;
    join_lits.extend(extra_join_lits.iter().cloned());
    let mut out = Vec::new();
    let mut tick = WallTick { wall, n: 0 };
    for_each_binding(
        &action.params,
        &domains,
        &join_lits,
        join_atoms,
        &mut tick,
        |b| {
            // Fixpoint rounds (0.12 Phase 3): a binding emitted in an earlier
            // round is final — skip it wholesale (its DNF conjuncts came
            // together), so each round pays only for NEW bindings' emission.
            if let Some(skip) = skip_bindings {
                let args: Vec<Sym> = param_vars.iter().map(|v| b[v].clone()).collect();
                if skip.contains(&args) {
                    return;
                }
            }
            // The enumeration already pruned on every static literal decidable
            // during binding; this post-filter keeps the remainder (literals
            // with quantified/unknown variables) AND stays the semantic oracle
            // for the pruning — the surviving set is identical by construction.
            for (p, a) in &join_lits {
                let ga = subst_args(a, b);
                if !join_atoms.contains(&(p.clone(), ga)) {
                    return;
                }
            }
            let stx = DnfStatics {
                init: join_atoms,
                add_preds: add_predicates,
                del_preds: del_predicates,
            };
            let st = dnf_static.then_some(&stx);
            let dnf = to_dnf(&action.precond, b, false, objects_of_type, st);
            let multi = dnf.len() > 1;
            let mut eff = REff {
                add: vec![],
                del: vec![],
                num: vec![],
                cond: vec![],
            };
            ground_effect(
                &action.effect,
                b,
                objects_of_type,
                &empty_conj(),
                &mut eff,
                st,
            );
            let args: Vec<Sym> = param_vars.iter().map(|v| b[v].clone()).collect();
            let display = if args.is_empty() {
                action.name.clone()
            } else {
                format!("{} {}", action.name, args.join(" "))
            };
            for conj in &dnf {
                out.push(RawOp {
                    display: display.clone(),
                    pos: conj.pos.clone(),
                    neg: conj.neg.clone(),
                    num_pre: conj.num.clone(),
                    eff: REff {
                        add: eff.add.clone(),
                        del: eff.del.clone(),
                        num: eff.num.clone(),
                        cond: eff.cond.clone(),
                    },
                    multi,
                    monitored: action.monitored,
                });
            }
        },
    );
    out
}

// ----- sequential interner for phase C -------------------------------------

struct Interner {
    fact_id: FxHashMap<(Sym, Vec<Sym>), u32>,
    fact_names: Vec<String>,
    fluent_id: FxHashMap<(Sym, Vec<Sym>), u32>,
}
impl Interner {
    fn fact(&mut self, key: &(Sym, Vec<Sym>)) -> u32 {
        if let Some(&id) = self.fact_id.get(key) {
            return id;
        }
        let id = self.fact_names.len() as u32;
        let disp = if key.1.is_empty() {
            format!("({})", key.0)
        } else {
            format!("({} {})", key.0, key.1.join(" "))
        };
        self.fact_names.push(disp);
        self.fact_id.insert(key.clone(), id);
        id
    }
    fn fluent(&mut self, name: &str, args: &[Sym]) -> u32 {
        let key = (name.to_string(), args.to_vec());
        if let Some(&id) = self.fluent_id.get(&key) {
            return id;
        }
        let id = self.fluent_id.len() as u32;
        self.fluent_id.insert(key, id);
        id
    }
    fn resolve_expr(&mut self, e: &Expr, reads: &mut Vec<u32>) -> NExpr {
        match e {
            Expr::Num(n) => NExpr::Num(*n),
            Expr::Fluent(f, a) => {
                let args: Vec<Sym> = a
                    .iter()
                    .map(|t| match t {
                        Term::Const(c) => c.clone(),
                        Term::Var(v) => v.clone(),
                    })
                    .collect();
                let id = self.fluent(f, &args);
                reads.push(id);
                NExpr::Fluent(id)
            }
            Expr::Add(x, y) => NExpr::Add(
                Box::new(self.resolve_expr(x, reads)),
                Box::new(self.resolve_expr(y, reads)),
            ),
            Expr::Sub(x, y) => NExpr::Sub(
                Box::new(self.resolve_expr(x, reads)),
                Box::new(self.resolve_expr(y, reads)),
            ),
            Expr::Mul(x, y) => NExpr::Mul(
                Box::new(self.resolve_expr(x, reads)),
                Box::new(self.resolve_expr(y, reads)),
            ),
            Expr::Div(x, y) => NExpr::Div(
                Box::new(self.resolve_expr(x, reads)),
                Box::new(self.resolve_expr(y, reads)),
            ),
            Expr::Neg(x) => NExpr::Neg(Box::new(self.resolve_expr(x, reads))),
        }
    }
}

/// A mid-form operator: fact ids interned, numeric resolved, neg still string.
#[allow(clippy::type_complexity)]
struct MidOp {
    display: String,
    pre_pos: Vec<u32>,
    neg: Vec<(Sym, Vec<Sym>)>,
    pre_num: Vec<NumPre>,
    add: Vec<u32>,
    del: Vec<u32>,
    add_atoms: Vec<(Sym, Vec<Sym>)>,
    del_atoms: Vec<(Sym, Vec<Sym>)>,
    num_eff: Vec<NumEff>,
    reads: Vec<u32>,
    /// interned conditional effects (negative conditions checked directly at
    /// apply time, so they need no complementary-fact compilation)
    cond: Vec<CondEff>,
    /// per-conditional-effect (add_atoms, del_atoms) — kept for complementary
    /// toggling of negated facts in the final-op pass
    cond_atoms: Vec<CondAtoms>,
    /// this op applies the shared monitor block (0.8 Phase 2)
    monitored: bool,
}

/// A conditional effect's `(add_atoms, del_atoms)` string form, kept for
/// complementary-fact toggling in the final-op pass.
type CondAtoms = (Vec<(Sym, Vec<Sym>)>, Vec<(Sym, Vec<Sym>)>);

/// Intern one string-form conditional effect (shared by the per-op Phase-C
/// loop and the shared monitor block). Returns the interned [`CondEff`] plus
/// its [`CondAtoms`]. Condition reads are NOT recorded as op reads: an
/// undefined fluent in a condition means it simply won't fire.
fn intern_cond(intern: &mut Interner, rc: &RCondEff) -> (CondEff, CondAtoms) {
    let cond_pos: Vec<u32> = rc.cond_pos.iter().map(|k| intern.fact(k)).collect();
    let cond_neg: Vec<u32> = rc.cond_neg.iter().map(|k| intern.fact(k)).collect();
    let mut cond_num = Vec::new();
    for (op, l, rr) in &rc.cond_num {
        let mut rd = Vec::new();
        let lhs = intern.resolve_expr(l, &mut rd);
        let rhs = intern.resolve_expr(rr, &mut rd);
        cond_num.push(NumPre { op: *op, lhs, rhs });
    }
    let cadd: Vec<u32> = rc.add.iter().map(|k| intern.fact(k)).collect();
    let cdel: Vec<u32> = rc.del.iter().map(|k| intern.fact(k)).collect();
    let mut cnum = Vec::new();
    for (op, fname, fargs, val) in &rc.num {
        let target = intern.fluent(fname, fargs);
        let mut rd = Vec::new();
        let value = intern.resolve_expr(val, &mut rd);
        cnum.push(NumEff {
            op: *op,
            target,
            value,
        });
    }
    (
        CondEff {
            cond_pos,
            cond_neg,
            cond_num,
            add: cadd,
            del: cdel,
            num: cnum,
        },
        (rc.add.clone(), rc.del.clone()),
    )
}

/// Grounding entry. `ground` does PDDL goal simplification (TRUE/FALSE early
/// exits); `ground_task` forces a Task even for trivial/unreachable goals — for
/// validators that must execute a plan regardless of goal triviality.
///
/// The solve entries (`ground`, `ground_stratified`) fold+compact the fluent
/// space (0.21 Phase 6); the session entry (`ground_fixpoint`) and the
/// validator entry (`ground_task`) keep FULL fluent tables — `set_fluent` on
/// an op-untouched fluent must stay live (the MCP world-edit contract,
/// pinned by session.rs tests).
pub fn ground(domain: &Domain, problem: &Problem, threads: usize) -> Outcome {
    ground_v(domain, problem, threads, false, false, false, true)
}

/// Like [`ground_stratified`] with reached-restricted FIXPOINT enumeration
/// (0.12 Phase 3): enumeration cost tracks the REACHABLE op set instead of
/// the typed product (elevator-11 p04: 5.7 GB → 48.8 MB transient). The
/// SURVIVING op set is identical to the other entries, but fact-id
/// first-reference order shifts (doomed candidates are never enumerated, so
/// they no longer intern atoms early) — search tie-breaks move with it,
/// which cost sokoban-t real coverage in the corpus A/B. The corpus solve
/// paths therefore stay on [`ground_stratified`]; the temporal SESSION (the
/// game track) uses this entry, where the memory win is the point and no
/// scoreboard baseline is disturbed. `FF_NO_FIXPOINT_GROUND=1` falls back.
pub fn ground_fixpoint(domain: &Domain, problem: &Problem, threads: usize) -> Outcome {
    ground_v(domain, problem, threads, false, true, true, false)
}

/// Like [`ground`], with stratified Phase B (see the block in `ground_v`):
/// actions gated on producer-known predicates ground join-restricted to the
/// atoms stratum 1 produced. Same post-reachability op set and order; fact-id
/// first-reference order may differ from [`ground`], so the classical path
/// stays on the plain entry. The temporal snap path uses this.
pub fn ground_stratified(domain: &Domain, problem: &Problem, threads: usize) -> Outcome {
    ground_v(domain, problem, threads, false, true, false, true)
}

/// Always return the grounded Task (skips goal TRUE/FALSE/undefined verdicts);
/// None only on a fatal empty-type error.
pub fn ground_task(domain: &Domain, problem: &Problem, threads: usize) -> Option<PackedTask> {
    match ground_v(domain, problem, threads, true, false, false, false) {
        Outcome::Task(t) => Some(t),
        _ => None,
    }
}

/// Build the `type -> objects` map honoring the type hierarchy (subtypes
/// included; `OBJECT` is every object). Shared by grounding and the PDDL3
/// compiler's forall-preference expansion.
pub fn objects_by_type(domain: &Domain, problem: &Problem) -> HashMap<Sym, Vec<Sym>> {
    let mut type_parent: HashMap<Sym, Sym> = domain.type_parent.iter().cloned().collect();
    let mut all_objects: Vec<(Sym, Sym)> = domain.constants.clone();
    all_objects.extend(problem.objects.iter().cloned());
    let ensure = |ty: &Sym, tp: &mut HashMap<Sym, Sym>| {
        if ty != "OBJECT" && !tp.contains_key(ty) {
            tp.insert(ty.clone(), "OBJECT".to_string());
        }
    };
    for (_, ty) in &all_objects {
        ensure(ty, &mut type_parent);
    }
    for (_, ty) in &domain.type_parent {
        ensure(ty, &mut type_parent);
    }
    let is_sub = |a: &Sym, b: &Sym, tp: &HashMap<Sym, Sym>| -> bool {
        if b == "OBJECT" {
            return true;
        }
        // Hop-bounded walk: the parser rejects cyclic (:types ...) input,
        // but Domain fields are public — a programmatically-built cycle
        // must degrade to "not a subtype", never a hang.
        let mut cur = a.clone();
        let mut hops = 0usize;
        loop {
            if &cur == b {
                return true;
            }
            match tp.get(&cur) {
                Some(p) => cur = p.clone(),
                None => return false,
            }
            hops += 1;
            if hops > tp.len() {
                return false;
            }
        }
    };
    let mut type_names: HashSet<Sym> = type_parent.keys().cloned().collect();
    type_names.insert("OBJECT".to_string());
    let mut objects_of_type: HashMap<Sym, Vec<Sym>> = HashMap::new();
    for tn in &type_names {
        let v: Vec<Sym> = all_objects
            .iter()
            .filter(|(_, oty)| is_sub(oty, tn, &type_parent))
            .map(|(o, _)| o.clone())
            .collect();
        objects_of_type.insert(tn.clone(), v);
    }
    objects_of_type
}

#[allow(clippy::too_many_arguments)]
fn ground_v(
    domain: &Domain,
    problem: &Problem,
    threads: usize,
    validate: bool,
    stratified: bool,
    fixpoint: bool,
    fold_fluents: bool,
) -> Outcome {
    // ---- type system ----
    let objects_of_type = objects_by_type(domain, problem);

    // Empty types are TOLERATED (standard PDDL): a predicate, function, or action
    // parameterized by a type with no objects simply grounds to zero instances.
    // This lets a problem use a SUBSET of a broad "universal" domain's types (e.g.
    // a smithing contract that declares no building `slot`s) without the whole task
    // failing — important for decomposing one big domain into many sub-tasks.
    // (The `EmptyType` outcome is retained for callers but no longer raised here.)

    let mut add_predicates: HashSet<Sym> = HashSet::new();
    fn collect_add(e: &Effect, out: &mut HashSet<Sym>) {
        match e {
            Effect::Add(p, _) => {
                out.insert(p.clone());
            }
            Effect::And(v) => v.iter().for_each(|x| collect_add(x, out)),
            // predicates added by conditional/universal effects are NOT static
            // inertia — must recurse so the static-precondition guard does not
            // wrongly prune actions whose precondition reads them.
            Effect::When(_, inner) => collect_add(inner, out),
            Effect::Forall(_, inner) => collect_add(inner, out),
            _ => {}
        }
    }
    for a in &domain.actions {
        collect_add(&a.effect, &mut add_predicates);
    }
    // the shared monitor block's conditional adds are not static inertia either
    for e in &domain.monitors {
        collect_add(e, &mut add_predicates);
    }
    // deleted predicates: the True-side folding guard in `DnfStatics`
    let mut del_predicates: HashSet<Sym> = HashSet::new();
    fn collect_del(e: &Effect, out: &mut HashSet<Sym>) {
        match e {
            Effect::Del(p, _) => {
                out.insert(p.clone());
            }
            Effect::And(v) => v.iter().for_each(|x| collect_del(x, out)),
            Effect::When(_, inner) => collect_del(inner, out),
            Effect::Forall(_, inner) => collect_del(inner, out),
            _ => {}
        }
    }
    for a in &domain.actions {
        collect_del(&a.effect, &mut del_predicates);
    }
    for e in &domain.monitors {
        collect_del(e, &mut del_predicates);
    }
    let init_atom_set: HashSet<(Sym, Vec<Sym>)> = problem.init_atoms.iter().cloned().collect();
    // predicate -> objects appearing in a unary init atom `(P o)`, for static
    // parameter-domain restriction in `ground_action`.
    let mut init_unary: FxHashMap<Sym, FxHashSet<Sym>> = FxHashMap::default();
    for (p, args) in &problem.init_atoms {
        if args.len() == 1 {
            init_unary
                .entry(p.clone())
                .or_default()
                .insert(args[0].clone());
        }
    }

    // DNF static resolution (see `DnfStatics`); one env read for all actions.
    let dnf_static = std::env::var("FF_NO_DNF_STATIC").is_err();

    // ---- Phase B: parallel per-action grounding (optionally stratified) ----
    //
    // Stratified grounding (opt-in via `ground_stratified`; the temporal snap
    // path uses it): an action whose precondition is gated on a PRODUCER-KNOWN
    // predicate — dynamic, zero init atoms, added only by stratum-1 actions —
    // grounds in a second pass, join-restricted to the atoms stratum 1
    // actually produced. The motivating case is the snap compilation's END
    // actions: BOARD-END's 5-ary RUNNING-BOARD token otherwise enumerates the
    // full typed space (elevator-08-t p22: 470k raw candidates, ~40 s and
    // ~8 GB transient, OOM under parallel jobs) when the only atoms that can
    // ever hold are the ones its START mints. The post-reachability op SET is
    // identical (reachability already killed producer-less candidates — this
    // just never enumerates them); raw-op ORDER is preserved by splicing each
    // action's ops back at its original index. Fact-id FIRST-REFERENCE order
    // can shift (a dropped candidate no longer mints its atoms early), which
    // is why the classical path keeps this off — its exact fixtures pin
    // today's ids. `FF_NO_STRAT_GROUND=1` disables for A/B measurement.
    let n_actions = domain.actions.len();
    // The grounding wall checkpoint (0.22 Phase 2 lever 1), armed ONLY
    // on the plain solve entry (`ground`): the validator entry must
    // still ground a found plan's task after the wall (a plan found is
    // a plan, never discarded), and the temporal/session entries keep
    // their own budget discipline (0.23's tier). Unarmed `FF_TIME_LIMIT`
    // or `FF_NO_RUNG_WALLCAP=1` ⇒ `None` ⇒ byte-identical enumeration.
    let gwall: Option<GroundWall> =
        (!validate && !stratified && !fixpoint && crate::search::rung_wallcap_on())
            .then(crate::search::wall_deadline)
            .flatten()
            .map(|(clock, total)| GroundWall {
                clock,
                total,
                tripped: std::sync::atomic::AtomicBool::new(false),
            });
    // Reached-restricted FIXPOINT grounding (0.12 Phase 3, temporal entry
    // only, `FF_NO_FIXPOINT_GROUND=1` falls back to the stratified pass):
    // every action joins its positive dynamic top-level literals against the
    // atoms REACHED so far (init + emitted ops' adds), rounds to fixpoint,
    // bindings deduped across rounds. Enumeration cost tracks the REACHABLE
    // op set instead of the typed product — elevator-11 p04 enumerated ~100×
    // its reachable set (158 s, 11.1 GB transient for a 16.7k-op task).
    // Subsumes the producer-known stratification (RUNNING-* literals are
    // dynamic literals like any other). Dense-reachable domains (the bazaar
    // fixture: 197k of 211k candidates real) pay only the round overhead.
    let fixpoint_raws: Option<Vec<RawOp>> = if fixpoint
        && std::env::var("FF_NO_FIXPOINT_GROUND").is_err()
    {
        let dyn_lits: Vec<Vec<(Sym, Vec<Term>)>> = domain
            .actions
            .iter()
            .map(|a| gating_top_atoms(&a.precond, &add_predicates))
            .collect();
        let mut reached = init_atom_set.clone();
        let mut emitted: Vec<FxHashSet<Vec<Sym>>> =
            (0..n_actions).map(|_| FxHashSet::default()).collect();
        let mut per_action: Vec<Vec<RawOp>> = (0..n_actions).map(|_| Vec::new()).collect();
        let action_idx: Vec<usize> = (0..n_actions).collect();
        loop {
            let chunks: Vec<Vec<RawOp>> = par::par_map(&action_idx, threads, |&ai| {
                ground_action(
                    &domain.actions[ai],
                    &objects_of_type,
                    &init_unary,
                    &reached,
                    &add_predicates,
                    &del_predicates,
                    &dyn_lits[ai],
                    dnf_static,
                    Some(&emitted[ai]),
                    gwall.as_ref(),
                )
            });
            let mut new_atom = false;
            for (ai, ops) in chunks.into_iter().enumerate() {
                for op in ops {
                    let args: Vec<Sym> = op
                        .display
                        .split_whitespace()
                        .skip(1)
                        .map(|s| s.to_string())
                        .collect();
                    emitted[ai].insert(args);
                    for (p, a) in op
                        .eff
                        .add
                        .iter()
                        .chain(op.eff.cond.iter().flat_map(|ce| ce.add.iter()))
                    {
                        if add_predicates.contains(p) && reached.insert((p.clone(), a.clone())) {
                            new_atom = true;
                        }
                    }
                    per_action[ai].push(op);
                }
            }
            if !new_atom {
                break;
            }
        }
        Some(per_action.into_iter().flatten().collect::<Vec<RawOp>>())
    } else {
        None
    };
    let raws: Vec<RawOp> = if let Some(r) = fixpoint_raws {
        r
    } else {
        let strat_on = stratified && std::env::var("FF_NO_STRAT_GROUND").is_err();
        let mut gating_of: Vec<Vec<(Sym, Vec<Term>)>> = vec![Vec::new(); n_actions];
        if strat_on {
            // Candidate gating predicates: dynamic, no init atoms, and not added
            // by the shared monitor block (its adds bypass the action lists).
            let mut monitor_adds: HashSet<Sym> = HashSet::new();
            for e in &domain.monitors {
                collect_add(e, &mut monitor_adds);
            }
            let init_preds: HashSet<&Sym> = problem.init_atoms.iter().map(|(p, _)| p).collect();
            let candidates: HashSet<Sym> = add_predicates
                .iter()
                .filter(|p| !init_preds.contains(p) && !monitor_adds.contains(*p))
                .cloned()
                .collect();
            if !candidates.is_empty() {
                let adds_of: Vec<HashSet<Sym>> = domain
                    .actions
                    .iter()
                    .map(|a| {
                        let mut s = HashSet::new();
                        collect_add(&a.effect, &mut s);
                        s
                    })
                    .collect();
                for (ai, a) in domain.actions.iter().enumerate() {
                    gating_of[ai] = gating_top_atoms(&a.precond, &candidates);
                }
                // Fixpoint demotion: a stratum-2 action's gating predicates must
                // be produced ONLY by stratum-1 actions (multi-level chains demote
                // to stratum 1 — correct, just unoptimized).
                loop {
                    let s2_added: HashSet<&Sym> = (0..n_actions)
                        .filter(|&i| !gating_of[i].is_empty())
                        .flat_map(|i| adds_of[i].iter())
                        .collect();
                    let mut changed = false;
                    for g in gating_of.iter_mut() {
                        if !g.is_empty() && g.iter().any(|(p, _)| s2_added.contains(p)) {
                            g.clear();
                            changed = true;
                        }
                    }
                    if !changed {
                        break;
                    }
                }
            }
        }

        let idx1: Vec<usize> = (0..n_actions)
            .filter(|&i| gating_of[i].is_empty())
            .collect();
        let chunks1: Vec<Vec<RawOp>> = par::par_map(&idx1, threads, |&ai| {
            ground_action(
                &domain.actions[ai],
                &objects_of_type,
                &init_unary,
                &init_atom_set,
                &add_predicates,
                &del_predicates,
                &[],
                dnf_static,
                None,
                gwall.as_ref(),
            )
        });
        let idx2: Vec<usize> = (0..n_actions)
            .filter(|&i| !gating_of[i].is_empty())
            .collect();
        let chunks2: Vec<Vec<RawOp>> = if idx2.is_empty() {
            Vec::new()
        } else {
            // The join universe: init plus every atom stratum 1 produced on a
            // gating predicate (unconditional AND conditional adds).
            let gating_preds: HashSet<&Sym> = idx2
                .iter()
                .flat_map(|&i| gating_of[i].iter().map(|(p, _)| p))
                .collect();
            let mut join_atoms = init_atom_set.clone();
            for ops in &chunks1 {
                for op in ops {
                    for (p, a) in op
                        .eff
                        .add
                        .iter()
                        .chain(op.eff.cond.iter().flat_map(|ce| ce.add.iter()))
                    {
                        if gating_preds.contains(p) {
                            join_atoms.insert((p.clone(), a.clone()));
                        }
                    }
                }
            }
            par::par_map(&idx2, threads, |&ai| {
                ground_action(
                    &domain.actions[ai],
                    &objects_of_type,
                    &init_unary,
                    &join_atoms,
                    &add_predicates,
                    &del_predicates,
                    &gating_of[ai],
                    dnf_static,
                    None,
                    gwall.as_ref(),
                )
            })
        };
        // Splice both strata back in original action order — the raw-op sequence
        // (and every downstream tie-break) is ordered exactly as unstratified.
        let mut raw_chunks: Vec<Vec<RawOp>> = (0..n_actions).map(|_| Vec::new()).collect();
        for (&ai, ops) in idx1.iter().zip(chunks1) {
            raw_chunks[ai] = ops;
        }
        for (&ai, ops) in idx2.iter().zip(chunks2) {
            raw_chunks[ai] = ops;
        }
        raw_chunks.into_iter().flatten().collect()
    };
    // A tripped wall discards the partial Phase B output WHOLESALE:
    // the abort point depends on scheduling, and a task shaped by it
    // would be nondeterministic — honest failure or a whole task,
    // nothing in between.
    if gwall.as_ref().is_some_and(|g| g.tripped()) {
        if std::env::var("FF_WALL_DEBUG").is_ok() {
            eprintln!("wall: grounding checkpoint expired mid-enumeration (no task, no verdict)");
        }
        return Outcome::WallExhausted(
            "wall budget exhausted during binding enumeration (FF_TIME_LIMIT): \
             no task grounded, no verdict"
                .into(),
        );
    }
    let n_easy = raws.iter().filter(|r| !r.multi).count();
    let n_hard = raws.iter().filter(|r| r.multi).count();

    // ---- Phase C: intern + resolve numeric ----
    let mut intern = Interner {
        fact_id: FxHashMap::default(),
        fact_names: Vec::new(),
        fluent_id: FxHashMap::default(),
    };
    let mut mids: Vec<MidOp> = Vec::with_capacity(raws.len());
    for r in &raws {
        let mut reads = Vec::new();
        let pre_pos: Vec<u32> = r.pos.iter().map(|k| intern.fact(k)).collect();
        let add: Vec<u32> = r.eff.add.iter().map(|k| intern.fact(k)).collect();
        let del: Vec<u32> = r.eff.del.iter().map(|k| intern.fact(k)).collect();
        let mut pre_num = Vec::new();
        for (op, l, rr) in &r.num_pre {
            let lhs = intern.resolve_expr(l, &mut reads);
            let rhs = intern.resolve_expr(rr, &mut reads);
            pre_num.push(NumPre { op: *op, lhs, rhs });
        }
        let mut num_eff = Vec::new();
        for (op, fname, fargs, val) in &r.eff.num {
            let target = intern.fluent(fname, fargs);
            if *op != AssignOp::Assign {
                reads.push(target);
            }
            let value = intern.resolve_expr(val, &mut reads);
            num_eff.push(NumEff {
                op: *op,
                target,
                value,
            });
        }
        // intern conditional effects. Condition reads are NOT added to `reads`:
        // an undefined fluent in a condition means it simply won't fire, it does
        // not make the operator illegal.
        let mut cond = Vec::new();
        let mut cond_atoms = Vec::new();
        for rc in &r.eff.cond {
            let (ce, atoms) = intern_cond(&mut intern, rc);
            cond.push(ce);
            cond_atoms.push(atoms);
        }
        mids.push(MidOp {
            display: r.display.clone(),
            pre_pos,
            neg: r.neg.clone(),
            pre_num,
            add,
            del,
            add_atoms: r.eff.add.clone(),
            del_atoms: r.eff.del.clone(),
            num_eff,
            reads,
            cond,
            cond_atoms,
            monitored: r.monitored,
        });
    }
    drop(raws);

    // ---- shared monitor block (0.8 Phase 2): ground + intern ONCE ----
    // `domain.monitors` holds the trajectory-monitor transitions, fully
    // ground and byte-identical for every binding of every monitored action
    // (constraints::compile pre-expands all quantifiers). Grounding it once
    // here — instead of per ground op — is what removes the monitor-count x
    // ground-action memory product. A When whose condition folded to a
    // constant TRUE emits as an unconditional leaf; normalize those into one
    // empty-condition CondEff (fires on every application — same semantics
    // as the 0.7 per-op unconditional add).
    let mut shared_reff = REff {
        add: vec![],
        del: vec![],
        num: vec![],
        cond: vec![],
    };
    for e in &domain.monitors {
        // No static resolution here (None): the constraint fixtures pin the
        // 0.7/0.8 shared-block shapes, and monitor conditions are TRAJ-fact
        // driven anyway — nothing meaningful to fold.
        ground_effect(
            e,
            &HashMap::new(),
            &objects_of_type,
            &empty_conj(),
            &mut shared_reff,
            None,
        );
    }
    debug_assert!(
        shared_reff.num.is_empty(),
        "the monitor block carries no numeric effects"
    );
    if !shared_reff.add.is_empty() || !shared_reff.del.is_empty() {
        let add = std::mem::take(&mut shared_reff.add);
        let del = std::mem::take(&mut shared_reff.del);
        shared_reff.cond.push(RCondEff {
            cond_pos: vec![],
            cond_neg: vec![],
            cond_num: vec![],
            add,
            del,
            num: vec![],
        });
    }
    let mut shared_cond: Vec<CondEff> = Vec::with_capacity(shared_reff.cond.len());
    let mut shared_cond_atoms = Vec::with_capacity(shared_reff.cond.len());
    for rc in &shared_reff.cond {
        let (ce, atoms) = intern_cond(&mut intern, rc);
        shared_cond.push(ce);
        shared_cond_atoms.push(atoms);
    }

    // ---- defined-fluents fixpoint + illegal-op pruning ----
    let n_fluents_pre = intern.fluent_id.len();
    let mut fv = vec![0.0f64; n_fluents_pre];
    let mut fdef = vec![false; n_fluents_pre];
    for ((name, args), val) in &problem.init_fluents {
        let id = intern.fluent(name, args) as usize;
        if id >= fv.len() {
            fv.resize(id + 1, 0.0);
            fdef.resize(id + 1, false);
        }
        fv[id] = *val;
        fdef[id] = true;
    }
    let nfl = intern.fluent_id.len();
    if fv.len() < nfl {
        fv.resize(nfl, 0.0);
        fdef.resize(nfl, false);
    }
    // PDDL 3.1 `:action-costs` convention (0.19 Phase 1): `(total-cost)`
    // starts at 0 IMPLICITLY — IPC-2018's agricola/flashfill/settlers omit
    // `(= (total-cost) 0)` from init, and the undefined fluent killed every
    // cost-bearing action in the fixpoint below (60 instances returned
    // "unsolvable" with zero facts). Fast Downward defaults it the same
    // way. Only the exact zero-arity TOTAL-COST fluent, only when init
    // leaves it undefined — every other undefined read stays a real error.
    if let Some(&id) = intern
        .fluent_id
        .get(&("TOTAL-COST".to_string(), Vec::new()))
    {
        if !fdef[id as usize] {
            fv[id as usize] = 0.0;
            fdef[id as usize] = true;
        }
    }
    loop {
        let mut changed = false;
        for m in &mids {
            if m.reads.iter().all(|&fl| fdef[fl as usize]) {
                for ne in &m.num_eff {
                    if ne.op == AssignOp::Assign && !fdef[ne.target as usize] {
                        fdef[ne.target as usize] = true;
                        changed = true;
                    }
                }
            }
            // conditional assigns also DEFINE their target at runtime (packed.rs
            // apply sets fdef[t]=true when a conditional assign fires). Propagate
            // definedness from them too, gated on the VALUE expression being
            // defined; the condition reads must NOT gate this (an undefined
            // condition only suppresses firing, it does not preclude the assign).
            for ce in &m.cond {
                for ne in &ce.num {
                    if ne.op == AssignOp::Assign && !fdef[ne.target as usize] {
                        let mut vreads = Vec::new();
                        ne.value.collect_fluents(&mut vreads);
                        if vreads.iter().all(|&fl| fdef[fl as usize]) {
                            fdef[ne.target as usize] = true;
                            changed = true;
                        }
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    mids.retain(|m| m.reads.iter().all(|&fl| fdef[fl as usize]));

    // ---- negative-precondition compilation to complementary facts ----
    let mut neg_atoms: HashSet<(Sym, Vec<Sym>)> = HashSet::new();
    for m in &mids {
        for a in &m.neg {
            neg_atoms.insert(a.clone());
        }
    }
    let goal_stx = DnfStatics {
        init: &init_atom_set,
        add_preds: &add_predicates,
        del_preds: &del_predicates,
    };
    // The goal expansion runs under the armed wall (0.22 Phase 1): a goal
    // whose disjunctive items DNF-multiply (block-grouping's pairwise
    // coordinate-Eq network: 4^21 conjuncts on i3) used to balloon here
    // unchecked, 76 s past a 60 s budget, upstream of every narrating
    // driver. Armed ONLY around this call — action grounding is parallel
    // and out of scope; unarmed, the probe is a thread-local flag read.
    DNF_WALL.with(|w| {
        *w.borrow_mut() = DnfWall {
            armed: true,
            hit: false,
            produced: 0,
            budget_bytes: crate::search::rlimit_budget(),
        }
    });
    let goal_dnf = to_dnf(
        &problem.goal,
        &HashMap::new(),
        false,
        &objects_of_type,
        dnf_static.then_some(&goal_stx),
    );
    let wall_hit = DNF_WALL.with(|w| {
        let hit = w.borrow().hit;
        *w.borrow_mut() = DnfWall {
            armed: false,
            hit: false,
            produced: 0,
            budget_bytes: usize::MAX,
        };
        hit
    });
    if wall_hit {
        return Outcome::WallExhausted(format!(
            "goal DNF expansion ({} goal items) exceeded the declared budget",
            match &problem.goal {
                Formula::And(fs) => fs.len(),
                _ => 1,
            }
        ));
    }
    if goal_dnf.is_empty() {
        return Outcome::GoalFalse(
            "the goal simplifies to FALSE against static init (no satisfiable disjunct)".into(),
        );
    }
    // collect negative atoms from EVERY disjunct (a disjunctive goal is compiled
    // below; each disjunct may carry its own negative literals)
    for conj in &goal_dnf {
        for a in &conj.neg {
            neg_atoms.insert(a.clone());
        }
    }
    let mut neg_fact: HashMap<(Sym, Vec<Sym>), u32> = HashMap::new();
    for a in &neg_atoms {
        let id = intern.fact_names.len() as u32;
        let disp = if a.1.is_empty() {
            format!("(NOT ({}))", a.0)
        } else {
            format!("(NOT ({} {}))", a.0, a.1.join(" "))
        };
        intern.fact_names.push(disp);
        neg_fact.insert(a.clone(), id);
    }

    // build final per-op fact lists with complementary toggles
    struct FinalOp {
        display: String,
        pre_pos: Vec<u32>,
        pre_num: Vec<NumPre>,
        add: Vec<u32>,
        del: Vec<u32>,
        num_eff: Vec<NumEff>,
        cond: Vec<CondEff>,
        monitored: bool,
    }
    let mut fops: Vec<FinalOp> = Vec::with_capacity(mids.len());
    for m in &mids {
        let mut pre_pos = m.pre_pos.clone();
        for a in &m.neg {
            pre_pos.push(neg_fact[a]);
        }
        let mut add = m.add.clone();
        let mut del = m.del.clone();
        // Complementary (NOT p) maintenance via blind toggles, matching
        // Metric-FF's negative-precondition compilation: every add of p deletes
        // (NOT p) and every del of p adds (NOT p), resolved per-fact add-wins.
        // (This faithfully reproduces FF's behaviour — including the inconsistent
        // p AND (NOT p) state an action that both adds and deletes p can yield —
        // verified against the C oracle.)
        for a in &m.add_atoms {
            if let Some(&c) = neg_fact.get(a) {
                del.push(c);
            }
        }
        for a in &m.del_atoms {
            if let Some(&c) = neg_fact.get(a) {
                add.push(c);
            }
        }
        // conditional effects: same complementary toggling on their add/del
        let mut cond = m.cond.clone();
        for (ce, (add_atoms, del_atoms)) in cond.iter_mut().zip(&m.cond_atoms) {
            for a in add_atoms {
                if let Some(&c) = neg_fact.get(a) {
                    ce.del.push(c);
                }
            }
            for a in del_atoms {
                if let Some(&c) = neg_fact.get(a) {
                    ce.add.push(c);
                }
            }
        }
        fops.push(FinalOp {
            display: m.display.clone(),
            pre_pos,
            pre_num: m.pre_num.clone(),
            add,
            del,
            num_eff: m.num_eff.clone(),
            cond,
            monitored: m.monitored,
        });
    }
    // shared monitor block: the same complementary toggling, applied ONCE
    for (ce, (add_atoms, del_atoms)) in shared_cond.iter_mut().zip(&shared_cond_atoms) {
        for a in add_atoms {
            if let Some(&c) = neg_fact.get(a) {
                ce.del.push(c);
            }
        }
        for a in del_atoms {
            if let Some(&c) = neg_fact.get(a) {
                ce.add.push(c);
            }
        }
    }

    // ---- disjunctive / existential goal compilation ----
    // A goal whose DNF has >1 disjunct (from `or`, `exists`, or negated numeric
    // equality) cannot be a single fact conjunction. Compile it Metric-FF style:
    // an artificial fact GOAL-REACHED with one synthetic operator per disjunct
    // (precondition = that disjunct, effect = add GOAL-REACHED); the real goal
    // becomes the single fact GOAL-REACHED.
    let goal_conj_owned: Conjunct;
    let goal_conj: &Conjunct = if goal_dnf.len() > 1 {
        let gatom = ("GOAL-REACHED".to_string(), Vec::new());
        let gid = intern.fact(&gatom);
        for conj in &goal_dnf {
            let mut pre_pos: Vec<u32> = conj.pos.iter().map(|k| intern.fact(k)).collect();
            for a in &conj.neg {
                pre_pos.push(neg_fact[a]);
            }
            let mut pre_num = Vec::new();
            for (op, l, r) in &conj.num {
                let mut rd = Vec::new();
                let lhs = intern.resolve_expr(l, &mut rd);
                let rhs = intern.resolve_expr(r, &mut rd);
                pre_num.push(NumPre { op: *op, lhs, rhs });
            }
            fops.push(FinalOp {
                display: "REACH-GOAL".to_string(),
                pre_pos,
                pre_num,
                add: vec![gid],
                del: vec![],
                num_eff: vec![],
                cond: vec![],
                monitored: false,
            });
        }
        goal_conj_owned = Conjunct {
            pos: vec![gatom],
            neg: vec![],
            num: vec![],
        };
        &goal_conj_owned
    } else {
        &goal_dnf[0]
    };

    // ---- initial state facts ----
    let mut init_ids: Vec<u32> = problem.init_atoms.iter().map(|k| intern.fact(k)).collect();
    init_ids.sort_unstable();
    init_ids.dedup();
    let n_facts = intern.fact_names.len();
    let mut init_true = vec![false; n_facts];
    for &id in &init_ids {
        init_true[id as usize] = true;
    }
    for (a, &c) in &neg_fact {
        if !init_atom_set.contains(a) {
            init_true[c as usize] = true;
        }
    }

    // ---- relaxed reachability (prune ops) ----
    let mut reached = init_true.clone();
    let mut live = vec![false; fops.len()];
    // the shared block's adds become reachable with the FIRST live monitored
    // op (every monitored op applies it — 0.7-equivalent fixpoint)
    let mut shared_marked = shared_cond.is_empty();
    loop {
        let mut changed = false;
        for (i, op) in fops.iter().enumerate() {
            if live[i] {
                continue;
            }
            if op.pre_pos.iter().all(|&f| reached[f as usize]) {
                live[i] = true;
                changed = true;
                for &f in &op.add {
                    reached[f as usize] = true;
                }
                // conditional adds are reachable too (over-approximate: assume the
                // condition can hold) so reachability never under-counts facts
                for ce in &op.cond {
                    for &f in &ce.add {
                        reached[f as usize] = true;
                    }
                }
                if op.monitored && !shared_marked {
                    shared_marked = true;
                    for ce in &shared_cond {
                        for &f in &ce.add {
                            reached[f as usize] = true;
                        }
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    let reach_ops: Vec<&FinalOp> = fops
        .iter()
        .enumerate()
        .filter(|(i, _)| live[*i])
        .map(|(_, o)| o)
        .collect();

    // ---- goal analysis ----
    let mut goal_num: Vec<NumPre> = Vec::new();
    for (op, l, r) in &goal_conj.num {
        let mut reads = Vec::new();
        let lhs = intern.resolve_expr(l, &mut reads);
        let rhs = intern.resolve_expr(r, &mut reads);
        let tf = intern.fluent_id.len();
        if fv.len() < tf {
            fv.resize(tf, 0.0);
            fdef.resize(tf, false);
        }
        for fl in &reads {
            if !fdef[*fl as usize] && !validate {
                let disp = intern
                    .fluent_id
                    .iter()
                    .find(|(_, &id)| id == *fl)
                    .map(|((name, args), _)| {
                        if args.is_empty() {
                            format!("({name})")
                        } else {
                            format!("({} {})", name, args.join(" "))
                        }
                    })
                    .unwrap_or_else(|| format!("fluent #{fl}"));
                return Outcome::GoalUndefinedFluent(disp);
            }
        }
        goal_num.push(NumPre { op: *op, lhs, rhs });
    }
    let mut goal_pos: Vec<u32> = goal_conj.pos.iter().map(|k| intern.fact(k)).collect();
    for a in &goal_conj.neg {
        goal_pos.push(neg_fact[a]);
    }
    let n_facts2 = intern.fact_names.len();
    if init_true.len() < n_facts2 {
        init_true.resize(n_facts2, false);
        reached.resize(n_facts2, false);
    }

    // inertia-based goal simplification
    let any_monitored_reachable = reach_ops.iter().any(|o| o.monitored);
    let mut deletable: HashSet<u32> = reach_ops
        .iter()
        .flat_map(|o| {
            o.del
                .iter()
                .copied()
                .chain(o.cond.iter().flat_map(|c| c.del.iter().copied()))
        })
        .collect();
    if any_monitored_reachable {
        deletable.extend(shared_cond.iter().flat_map(|c| c.del.iter().copied()));
    }
    let modified_fluents: HashSet<u32> = reach_ops
        .iter()
        .flat_map(|o| {
            o.num_eff
                .iter()
                .map(|ne| ne.target)
                .chain(o.cond.iter().flat_map(|c| c.num.iter().map(|ne| ne.target)))
        })
        .collect();
    let inertia_pos =
        |f: u32| init_true.get(f as usize).copied().unwrap_or(false) && !deletable.contains(&f);
    let mut np_reads = Vec::new();
    let inertia_num = |np: &NumPre, scratch: &mut Vec<u32>| {
        scratch.clear();
        np.lhs.collect_fluents(scratch);
        np.rhs.collect_fluents(scratch);
        eval_numpre(np, &fv, &fdef).unwrap_or(false)
            && scratch.iter().all(|fl| !modified_fluents.contains(fl))
    };
    let remaining_pos: Vec<u32> = goal_pos
        .iter()
        .copied()
        .filter(|&f| !inertia_pos(f))
        .collect();
    let remaining_num = goal_num
        .iter()
        .filter(|np| !inertia_num(np, &mut np_reads))
        .count();
    if remaining_pos.is_empty() && remaining_num == 0 && !validate {
        return Outcome::GoalTrue;
    }
    if !validate {
        for &f in &remaining_pos {
            if !reached[f as usize] {
                return Outcome::GoalFalse(format!(
                    "goal fact {} is unreachable: no surviving grounded action adds it",
                    intern.fact_names[f as usize]
                ));
            }
        }
    }

    // ---- fact-space compaction ----
    // Phase C interned atoms from EVERY raw candidate op; reachability then
    // pruned the ops but left their fact ids behind, so `words` — and with it
    // every State bitset and visited key — was sized by the RAW atom space.
    // On temporal snap-compiled tasks the gap is catastrophic: elevator-08-t
    // p22 mints 2.35M RUNNING-* atoms from typed enumeration of unreachable
    // END candidates while ~2.4k facts are live (287 KB per state, 8 GB RSS
    // in 40 s). Keep facts that are reached, referenced by a surviving op
    // (del / conditional references may legally point at unreached facts),
    // or in the goal; renumber MONOTONICALLY so every relative order is
    // preserved and search behavior is unchanged — only ids, dims, and bytes
    // shrink. `FF_NO_FACT_COMPACT=1` restores the raw packing.
    let compact_on = std::env::var("FF_NO_FACT_COMPACT").is_err();
    let mut fact_names_all = std::mem::take(&mut intern.fact_names);
    let (fact_map, n_facts_packed, fact_names_packed): (Vec<u32>, usize, Vec<String>) =
        if compact_on {
            let mut keep = reached.clone(); // ⊇ init_true (incl. NOT-compiled facts)
            let mark_ce = |ce: &CondEff, keep: &mut Vec<bool>| {
                for &f in ce
                    .cond_pos
                    .iter()
                    .chain(ce.cond_neg.iter())
                    .chain(ce.add.iter())
                    .chain(ce.del.iter())
                {
                    keep[f as usize] = true;
                }
            };
            for op in &reach_ops {
                for &f in op.pre_pos.iter().chain(op.add.iter()).chain(op.del.iter()) {
                    keep[f as usize] = true;
                }
                for ce in &op.cond {
                    mark_ce(ce, &mut keep);
                }
            }
            // Always walk the shared block (not only when a monitored op is
            // reachable): it lands in the PackedTask either way and must never
            // carry dangling ids.
            for ce in &shared_cond {
                mark_ce(ce, &mut keep);
            }
            for &f in &goal_pos {
                keep[f as usize] = true;
            }
            let mut map = vec![u32::MAX; n_facts2];
            let mut names = Vec::new();
            for (i, &k) in keep.iter().enumerate() {
                if k {
                    map[i] = names.len() as u32;
                    names.push(std::mem::take(&mut fact_names_all[i]));
                }
            }
            let n = names.len();
            (map, n, names)
        } else {
            ((0..n_facts2 as u32).collect(), n_facts2, fact_names_all)
        };
    let remap = |f: u32| fact_map[f as usize];
    let goal_pos: Vec<u32> = goal_pos.iter().map(|&f| remap(f)).collect();
    let shared_cond: Vec<CondEff> = shared_cond
        .iter()
        .map(|ce| CondEff {
            cond_pos: ce.cond_pos.iter().map(|&f| remap(f)).collect(),
            cond_neg: ce.cond_neg.iter().map(|&f| remap(f)).collect(),
            cond_num: ce.cond_num.clone(),
            add: ce.add.iter().map(|&f| remap(f)).collect(),
            del: ce.del.iter().map(|&f| remap(f)).collect(),
            num: ce.num.clone(),
        })
        .collect();

    // ---- pack into CSR ----
    let words = bitset::words_for(n_facts_packed);
    let mut init_bits = vec![0u64; words];
    for (i, &b) in init_true.iter().enumerate() {
        if b {
            bitset::set(&mut init_bits, remap(i as u32) as usize);
        }
    }
    let nfl_final = intern.fluent_id.len();
    if fv.len() < nfl_final {
        fv.resize(nfl_final, 0.0);
        fdef.resize(nfl_final, false);
    }

    let n_reach_actions = reach_ops.len();
    let n_reach_facts = reached.iter().filter(|&&x| x).count();
    let n_relevant_fluents = fdef.iter().filter(|&&x| x).count();

    // ---- static-fluent fold + fluent-space compaction (0.21 Phase 6) ----
    // Fluents never got the 0.20 fact compaction above: price/cost/duration
    // tables intern into `fv0` and clone into EVERY search node (tpp i12:
    // 20.6 KB/node, 99% of it fv+fdef, 62% static; data-network i12: 386 of
    // 387 fluents static). Two levers, hatched separately:
    //  - FOLD (`FF_NO_FLUENT_FOLD=1` off): a `Fluent` read of a
    //    DEFINED-STATIC fluent (defined in init, written by no numeric
    //    effect incl. conditional ones) substitutes the same f64
    //    bit-for-bit as `NExpr::Num`. Folding is fenced to IRRELEVANT
    //    statics: pre/goal/cond-read statics are exactly the relevant
    //    ones — they stay in `fv0` regardless (visited keys and orbit
    //    detection read `rel_fluents`, which must not move), and folding
    //    them would flip shape-sensitive dispatch (numeric_achiever's
    //    bare/linear split, `as_threshold`, `linearize`'s const
    //    detection) — the 0.20 Phase 4 byte-identity bar outranks fold
    //    coverage. Undefined statics stay unfolded (their reads must keep
    //    evaluating to None).
    //  - COMPACTION (`FF_NO_FLUENT_COMPACT=1` off): keep WRITTEN +
    //    still-referenced + relevant + metric-named fluents, renumbered
    //    MONOTONICALLY like the facts above; dropped DEFINED statics go
    //    to the task-side `static_fluents` name table for the
    //    name-resolved readers (temporal duration grounding/eval). The
    //    per-node byte models read `fv0.len()` and raise their caps for
    //    free.
    // Both levers run on the solve()/planner entries ONLY (`fold_fluents`)
    // — the session and validator entries keep FULL tables so `set_fluent`
    // on an op-untouched fluent stays live (pinned by session.rs tests).
    let fold_on = fold_fluents && std::env::var("FF_NO_FLUENT_FOLD").is_err();
    let fcompact_on = fold_fluents && std::env::var("FF_NO_FLUENT_COMPACT").is_err();

    // Relevance, in the RAW fluent space over the UNFOLDED sources — the
    // fold is fenced by it, so it is computed before any transform. Same
    // marks as always: numeric pre/cond/goal reads, then the transitive
    // closure over effect RHS reads (see the comment at the loop).
    let mut relevant_raw = vec![false; nfl_final];
    let mark = |np: &NumPre, rel: &mut [bool]| {
        let mut v = Vec::new();
        np.lhs.collect_fluents(&mut v);
        np.rhs.collect_fluents(&mut v);
        for f in v {
            if (f as usize) < rel.len() {
                rel[f as usize] = true;
            }
        }
    };
    for op in &reach_ops {
        for np in &op.pre_num {
            mark(np, &mut relevant_raw);
        }
        for ce in &op.cond {
            for np in &ce.cond_num {
                mark(np, &mut relevant_raw);
            }
        }
    }
    // numeric comparisons inside shared monitor conditions read fluents too
    if any_monitored_reachable {
        for ce in &shared_cond {
            for np in &ce.cond_num {
                mark(np, &mut relevant_raw);
            }
        }
    }
    for np in &goal_num {
        mark(np, &mut relevant_raw);
    }
    // Transitive closure: a fluent read by the RHS of a numeric effect that
    // WRITES a relevant fluent is itself relevant (it determines that target's
    // delta), so it must not be zeroed out of the visited-set key. Fixpoint over
    // a finite bool vec -> terminates. Pure write-only accumulators (walkedTime,
    // drivenTime, fuelUsed) never feed a relevant target and stay irrelevant,
    // preserving the search-termination optimization.
    loop {
        let mut changed = false;
        for op in &reach_ops {
            let neffs = op
                .num_eff
                .iter()
                .chain(op.cond.iter().flat_map(|c| c.num.iter()));
            for ne in neffs {
                if relevant_raw[ne.target as usize] {
                    let mut v = Vec::new();
                    ne.value.collect_fluents(&mut v);
                    for f in v {
                        let f = f as usize;
                        if f < relevant_raw.len() && !relevant_raw[f] {
                            relevant_raw[f] = true;
                            changed = true;
                        }
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }

    // WRITTEN = target of any surviving numeric effect (incl. conditional
    // and the shared monitor block, which lands in the task either way).
    let mut written = vec![false; nfl_final];
    {
        let mut w = |nes: &[NumEff]| {
            for ne in nes {
                written[ne.target as usize] = true;
            }
        };
        for op in &reach_ops {
            w(&op.num_eff);
            for ce in &op.cond {
                w(&ce.num);
            }
        }
        for ce in &shared_cond {
            w(&ce.num);
        }
    }
    let foldable = |f: u32| {
        let f = f as usize;
        fold_on && fdef[f] && !written[f] && !relevant_raw[f]
    };

    // The census — every fluent id the packed task will carry, built as
    // code from the same holders the pack loop folds (pre_num, effect
    // values, conditional numeric parts, goal_num, the shared block), so a
    // dropped id can never be referenced. The metric's fluents survive by
    // NAME (costs::metric_fluent resolves them through `fluent_id` after
    // packing, so an unwritten metric fluent must not vanish).
    fn keep_metric_fluents(
        e: &Expr,
        fluent_id: &FxHashMap<(Sym, Vec<Sym>), u32>,
        fkeep: &mut [bool],
    ) {
        match e {
            Expr::Num(_) => {}
            Expr::Fluent(name, terms) => {
                let args: Vec<Sym> = terms
                    .iter()
                    .map(|t| match t {
                        Term::Const(c) => c.clone(),
                        Term::Var(v) => v.clone(),
                    })
                    .collect();
                if let Some(&id) = fluent_id.get(&(name.clone(), args)) {
                    fkeep[id as usize] = true;
                }
            }
            Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => {
                keep_metric_fluents(a, fluent_id, fkeep);
                keep_metric_fluents(b, fluent_id, fkeep);
            }
            Expr::Neg(a) => keep_metric_fluents(a, fluent_id, fkeep),
        }
    }
    let (fluent_map, nfl_packed): (Vec<u32>, usize) = if fcompact_on {
        let mut fkeep = written.clone();
        for (i, &r) in relevant_raw.iter().enumerate() {
            if r {
                fkeep[i] = true;
            }
        }
        let cite = |e: &NExpr, fkeep: &mut Vec<bool>| {
            let mut v = Vec::new();
            e.collect_fluents(&mut v);
            for f in v {
                if !foldable(f) {
                    fkeep[f as usize] = true;
                }
            }
        };
        let cite_ce = |ce: &CondEff, fkeep: &mut Vec<bool>| {
            for np in &ce.cond_num {
                cite(&np.lhs, fkeep);
                cite(&np.rhs, fkeep);
            }
            for ne in &ce.num {
                cite(&ne.value, fkeep);
            }
        };
        for op in &reach_ops {
            for np in &op.pre_num {
                cite(&np.lhs, &mut fkeep);
                cite(&np.rhs, &mut fkeep);
            }
            for ne in &op.num_eff {
                cite(&ne.value, &mut fkeep);
            }
            for ce in &op.cond {
                cite_ce(ce, &mut fkeep);
            }
        }
        for ce in &shared_cond {
            cite_ce(ce, &mut fkeep);
        }
        for np in &goal_num {
            cite(&np.lhs, &mut fkeep);
            cite(&np.rhs, &mut fkeep);
        }
        if let Some((_, e)) = &problem.metric {
            keep_metric_fluents(e, &intern.fluent_id, &mut fkeep);
        }
        // The zero-arity TOTAL-COST convention fluent is read by NAME after
        // packing even when no surviving op writes it and no metric names
        // it (the pddl3 compiler injects it and api/planner resolve it via
        // fluent_id) — the same carve-out as its implicit-zero init above.
        if let Some(&id) = intern
            .fluent_id
            .get(&("TOTAL-COST".to_string(), Vec::new()))
        {
            fkeep[id as usize] = true;
        }
        let mut map = vec![u32::MAX; nfl_final];
        let mut n = 0u32;
        for (i, &k) in fkeep.iter().enumerate() {
            if k {
                map[i] = n;
                n += 1;
            }
        }
        (map, n as usize)
    } else {
        ((0..nfl_final as u32).collect(), nfl_final)
    };
    // Dropped ids map to u32::MAX — the poison namespace: a pre-compaction
    // id surviving into a packed holder trips this assert, or lands >=
    // nfl_packed and trips the census sweep before Task construction.
    let fremap = |f: u32| -> u32 {
        let nf = fluent_map[f as usize];
        debug_assert!(
            nf != u32::MAX,
            "pre-compaction fluent id {f} survived the census"
        );
        nf
    };
    fn fold_nexpr<FB: Fn(u32) -> bool, FR: Fn(u32) -> u32>(
        e: &NExpr,
        fv: &[f64],
        foldable: &FB,
        fremap: &FR,
    ) -> NExpr {
        let rec = |x: &NExpr| Box::new(fold_nexpr(x, fv, foldable, fremap));
        match e {
            NExpr::Num(n) => NExpr::Num(*n),
            NExpr::Fluent(f) => {
                if foldable(*f) {
                    NExpr::Num(fv[*f as usize])
                } else {
                    NExpr::Fluent(fremap(*f))
                }
            }
            NExpr::Add(a, b) => NExpr::Add(rec(a), rec(b)),
            NExpr::Sub(a, b) => NExpr::Sub(rec(a), rec(b)),
            NExpr::Mul(a, b) => NExpr::Mul(rec(a), rec(b)),
            NExpr::Div(a, b) => NExpr::Div(rec(a), rec(b)),
            NExpr::Neg(a) => NExpr::Neg(rec(a)),
        }
    }
    let fold_np = |np: &NumPre| NumPre {
        op: np.op,
        lhs: fold_nexpr(&np.lhs, &fv, &foldable, &fremap),
        rhs: fold_nexpr(&np.rhs, &fv, &foldable, &fremap),
    };
    let fold_ne = |ne: &NumEff| NumEff {
        op: ne.op,
        target: fremap(ne.target),
        value: fold_nexpr(&ne.value, &fv, &foldable, &fremap),
    };
    let shared_cond: Vec<CondEff> = shared_cond
        .iter()
        .map(|ce| CondEff {
            cond_pos: ce.cond_pos.clone(),
            cond_neg: ce.cond_neg.clone(),
            cond_num: ce.cond_num.iter().map(&fold_np).collect(),
            add: ce.add.clone(),
            del: ce.del.clone(),
            num: ce.num.iter().map(&fold_ne).collect(),
        })
        .collect();
    let goal_num: Vec<NumPre> = goal_num.iter().map(&fold_np).collect();

    let mut op_display = Vec::with_capacity(n_reach_actions);
    let mut pre_pos = CsrBuilder::new();
    let mut add = CsrBuilder::new();
    let mut del = CsrBuilder::new();
    let mut pre_num = CsrBuilder::new();
    let mut num_eff = CsrBuilder::new();
    let mut cond_b = CsrBuilder::new();
    // add-by-fact buckets (for the heuristic hot path); the fluent-indexed
    // buckets live in the PACKED fluent space
    let mut add_buckets: Vec<Vec<u32>> = vec![Vec::new(); n_facts_packed];
    let mut neff_buckets: Vec<Vec<u32>> = vec![Vec::new(); nfl_packed];
    let mut monitored_v: Vec<bool> = Vec::with_capacity(n_reach_actions);
    let remap_ce = |ce: &CondEff| CondEff {
        cond_pos: ce.cond_pos.iter().map(|&f| remap(f)).collect(),
        cond_neg: ce.cond_neg.iter().map(|&f| remap(f)).collect(),
        cond_num: ce.cond_num.iter().map(&fold_np).collect(),
        add: ce.add.iter().map(|&f| remap(f)).collect(),
        del: ce.del.iter().map(|&f| remap(f)).collect(),
        num: ce.num.iter().map(&fold_ne).collect(),
    };
    for (oi, op) in reach_ops.iter().enumerate() {
        op_display.push(op.display.clone());
        pre_pos.push_row(op.pre_pos.iter().map(|&f| remap(f)));
        add.push_row(op.add.iter().map(|&f| remap(f)));
        del.push_row(op.del.iter().map(|&f| remap(f)));
        pre_num.push_row(op.pre_num.iter().map(&fold_np));
        num_eff.push_row(op.num_eff.iter().map(&fold_ne));
        cond_b.push_row(op.cond.iter().map(remap_ce));
        monitored_v.push(op.monitored);
        for &f in &op.add {
            add_buckets[remap(f) as usize].push(oi as u32);
        }
        // fluent -> ops with a numeric effect on it (distinct targets, op-id order)
        let mut seen_t: Vec<u32> = Vec::new();
        for ne in &op.num_eff {
            let t = fremap(ne.target);
            if !seen_t.contains(&t) {
                seen_t.push(t);
                neff_buckets[t as usize].push(oi as u32);
            }
        }
        // conditional adds also have this op as an achiever — including the
        // shared monitor block's, in the 0.7 suffix order (own conds first)
        for ce in &op.cond {
            for &f in &ce.add {
                add_buckets[remap(f) as usize].push(oi as u32);
            }
        }
        if op.monitored {
            // shared_cond was remapped above — its ids are already packed ids
            for ce in &shared_cond {
                for &f in &ce.add {
                    add_buckets[f as usize].push(oi as u32);
                }
            }
        }
    }
    let mut add_by_fact = CsrBuilder::new();
    for bucket in add_buckets {
        add_by_fact.push_row(bucket);
    }
    let mut neff_by_fluent = CsrBuilder::new();
    for bucket in neff_buckets {
        neff_by_fluent.push_row(bucket);
    }

    // the relevant set, mapped into the packed space (monotone renumber
    // keeps `rel_fluents` sorted — visited-key order is unchanged)
    let mut relevant_fluent = vec![false; nfl_packed];
    let mut rel_fluents: Vec<u32> = Vec::new();
    for (i, &r) in relevant_raw.iter().enumerate() {
        if r {
            let nf = fremap(i as u32);
            relevant_fluent[nf as usize] = true;
            rel_fluents.push(nf);
        }
    }

    // fluent id -> display string (for metric / cost-fluent lookup in sgp),
    // RAW space first, then split into packed names + the dropped-static
    // side table (name-resolved duration/introspection readers).
    let mut fluent_names_raw = vec![String::new(); nfl_final];
    for ((name, args), id) in &intern.fluent_id {
        fluent_names_raw[*id as usize] = if args.is_empty() {
            format!("({})", name)
        } else {
            format!("({} {})", name, args.join(" "))
        };
    }
    let mut fv0 = Vec::with_capacity(nfl_packed);
    let mut fdef0 = Vec::with_capacity(nfl_packed);
    let mut fluent_names: Vec<String> = Vec::with_capacity(nfl_packed);
    let mut static_fluents: Vec<(String, f64)> = Vec::new();
    if fcompact_on {
        for i in 0..nfl_final {
            if fluent_map[i] != u32::MAX {
                fv0.push(fv[i]);
                fdef0.push(fdef[i]);
                fluent_names.push(std::mem::take(&mut fluent_names_raw[i]));
            } else if fdef[i] {
                static_fluents.push((std::mem::take(&mut fluent_names_raw[i]), fv[i]));
            }
            // dropped UNDEFINED fluents get no table entry: a name-resolved
            // read misses both sources, exactly like an undefined fluent
        }
    } else {
        // clone, not move: the fold closures above still borrow fv/fdef
        fv0 = fv.clone();
        fdef0 = fdef.clone();
        fluent_names = fluent_names_raw;
    }

    // Debug sweep (the poison-namespace referee): every fluent id in the
    // packed holders must live in [0, nfl_packed) — a missed remap keeps a
    // raw id, which on any compacting task lands at or past nfl_packed.
    #[cfg(debug_assertions)]
    {
        let ok = |e: &NExpr| {
            let mut v = Vec::new();
            e.collect_fluents(&mut v);
            v.into_iter().all(|f| (f as usize) < nfl_packed)
        };
        let ok_np = |np: &NumPre| ok(&np.lhs) && ok(&np.rhs);
        let ok_ne = |ne: &NumEff| (ne.target as usize) < nfl_packed && ok(&ne.value);
        let ok_ce = |ce: &CondEff| ce.cond_num.iter().all(ok_np) && ce.num.iter().all(ok_ne);
        assert!(
            pre_num.flat.iter().all(ok_np)
                && num_eff.flat.iter().all(ok_ne)
                && cond_b.flat.iter().all(ok_ce)
                && shared_cond.iter().all(ok_ce)
                && goal_num.iter().all(ok_np)
                && rel_fluents.iter().all(|&f| (f as usize) < nfl_packed),
            "a pre-compaction fluent id survived into the packed task"
        );
    }

    Outcome::Task(PackedTask {
        n_facts: n_facts_packed,
        words,
        n_ops: n_reach_actions,
        op_display: op_display.into(),
        pre_pos: pre_pos.finish(),
        add: add.finish(),
        del: del.finish(),
        pre_num: pre_num.finish(),
        num_eff: num_eff.finish(),
        cond: cond_b.finish(),
        shared_cond: shared_cond.into(),
        monitored: monitored_v.into(),
        add_by_fact: add_by_fact.finish(),
        neff_by_fluent: neff_by_fluent.finish(),
        relevant_fluent,
        rel_fluents,
        init_bits,
        fv0,
        fdef0,
        goal_pos,
        goal_num,
        // The temporal entries (stratified snap path, fixpoint session)
        // keep the numeric-precondition charge OFF — see the field docs.
        charge_pre_num: !stratified,
        // The end-gate pair table is a TEMPORAL think-time overlay
        // (temporal.rs `endgate_pairs`, 0.21 Phase 8) — never grounded in.
        pair_end: None,
        fact_names: fact_names_packed.into(),
        fluent_names: fluent_names.into(),
        static_fluents: static_fluents.into(),
        n_easy,
        n_hard,
        n_reach_facts,
        n_reach_actions,
        n_relevant_fluents,
    })
}

// re-export for the heuristic/search modules
pub use crate::packed::PackedTask as Task;
pub fn initial_state(t: &PackedTask) -> State {
    t.initial()
}
