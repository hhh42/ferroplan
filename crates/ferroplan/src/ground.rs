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
    GoalFalse,
    GoalUndefinedFluent,
    EmptyType {
        kind: &'static str,
        pred: String,
        ty: String,
    },
}

// ----- DNF over ground formulas (string atoms) -----------------------------

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

/// AND-combine two DNF lists (cartesian product of conjuncts).
fn and_merge(acc: &[Conjunct], cd: &[Conjunct]) -> Vec<Conjunct> {
    let mut next = Vec::with_capacity(acc.len() * cd.len());
    for a in acc {
        for c in cd {
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
fn for_each_binding(
    params: &[(Sym, Sym)],
    domains: &[Vec<Sym>],
    static_lits: &[(Sym, Vec<Term>)],
    init_atom_set: &HashSet<(Sym, Vec<Sym>)>,
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
    fn rec(
        k: usize,
        params: &[(Sym, Sym)],
        domains: &[Vec<Sym>],
        lits_at: &[Vec<&(Sym, Vec<Term>)>],
        init: &HashSet<(Sym, Vec<Sym>)>,
        binding: &mut HashMap<Sym, Sym>,
        f: &mut impl FnMut(&HashMap<Sym, Sym>),
    ) {
        if k == params.len() {
            f(binding);
            return;
        }
        let var = &params[k].0;
        for o in &domains[k] {
            binding.insert(var.clone(), o.clone());
            let ok = lits_at[k]
                .iter()
                .all(|lit| init.contains(&(lit.0.clone(), subst_args(&lit.1, binding))));
            if ok {
                rec(k + 1, params, domains, lits_at, init, binding, f);
            }
        }
        binding.remove(var);
    }
    rec(
        0,
        params,
        domains,
        &lits_at,
        init_atom_set,
        &mut binding,
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
    for_each_binding(&action.params, &domains, &join_lits, join_atoms, |b| {
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
    });
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
pub fn ground(domain: &Domain, problem: &Problem, threads: usize) -> Outcome {
    ground_v(domain, problem, threads, false, false)
}

/// Like [`ground`], with stratified Phase B (see the block in `ground_v`):
/// actions gated on producer-known predicates ground join-restricted to the
/// atoms stratum 1 produced. Same post-reachability op set and order; fact-id
/// first-reference order may differ from [`ground`], so the classical path
/// stays on the plain entry. The temporal snap path uses this.
pub fn ground_stratified(domain: &Domain, problem: &Problem, threads: usize) -> Outcome {
    ground_v(domain, problem, threads, false, true)
}

/// Always return the grounded Task (skips goal TRUE/FALSE/undefined verdicts);
/// None only on a fatal empty-type error.
pub fn ground_task(domain: &Domain, problem: &Problem, threads: usize) -> Option<PackedTask> {
    match ground_v(domain, problem, threads, true, false) {
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

fn ground_v(
    domain: &Domain,
    problem: &Problem,
    threads: usize,
    validate: bool,
    stratified: bool,
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
    let raws: Vec<RawOp> = raw_chunks.into_iter().flatten().collect();
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
    let goal_dnf = to_dnf(
        &problem.goal,
        &HashMap::new(),
        false,
        &objects_of_type,
        dnf_static.then_some(&goal_stx),
    );
    if goal_dnf.is_empty() {
        return Outcome::GoalFalse;
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
                return Outcome::GoalUndefinedFluent;
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
                return Outcome::GoalFalse;
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

    let mut op_display = Vec::with_capacity(n_reach_actions);
    let mut pre_pos = CsrBuilder::new();
    let mut add = CsrBuilder::new();
    let mut del = CsrBuilder::new();
    let mut pre_num = CsrBuilder::new();
    let mut num_eff = CsrBuilder::new();
    let mut cond_b = CsrBuilder::new();
    // add-by-fact buckets + relevant-fluent set (for the heuristic hot path)
    let mut add_buckets: Vec<Vec<u32>> = vec![Vec::new(); n_facts_packed];
    let mut neff_buckets: Vec<Vec<u32>> = vec![Vec::new(); fv.len()];
    let mut relevant_fluent = vec![false; fv.len()];
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
    let mut monitored_v: Vec<bool> = Vec::with_capacity(n_reach_actions);
    let remap_ce = |ce: &CondEff| CondEff {
        cond_pos: ce.cond_pos.iter().map(|&f| remap(f)).collect(),
        cond_neg: ce.cond_neg.iter().map(|&f| remap(f)).collect(),
        cond_num: ce.cond_num.clone(),
        add: ce.add.iter().map(|&f| remap(f)).collect(),
        del: ce.del.iter().map(|&f| remap(f)).collect(),
        num: ce.num.clone(),
    };
    for (oi, op) in reach_ops.iter().enumerate() {
        op_display.push(op.display.clone());
        pre_pos.push_row(op.pre_pos.iter().map(|&f| remap(f)));
        add.push_row(op.add.iter().map(|&f| remap(f)));
        del.push_row(op.del.iter().map(|&f| remap(f)));
        pre_num.push_row(op.pre_num.iter().cloned());
        num_eff.push_row(op.num_eff.iter().cloned());
        cond_b.push_row(op.cond.iter().map(remap_ce));
        monitored_v.push(op.monitored);
        for &f in &op.add {
            add_buckets[remap(f) as usize].push(oi as u32);
        }
        // fluent -> ops with a numeric effect on it (distinct targets, op-id order)
        let mut seen_t: Vec<u32> = Vec::new();
        for ne in &op.num_eff {
            if !seen_t.contains(&ne.target) {
                seen_t.push(ne.target);
                neff_buckets[ne.target as usize].push(oi as u32);
            }
        }
        // conditional adds also have this op as an achiever — including the
        // shared monitor block's, in the 0.7 suffix order (own conds first)
        for ce in &op.cond {
            for &f in &ce.add {
                add_buckets[remap(f) as usize].push(oi as u32);
            }
            for np in &ce.cond_num {
                mark(np, &mut relevant_fluent);
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
        for np in &op.pre_num {
            mark(np, &mut relevant_fluent);
        }
    }
    // numeric comparisons inside shared monitor conditions read fluents too
    if any_monitored_reachable {
        for ce in &shared_cond {
            for np in &ce.cond_num {
                mark(np, &mut relevant_fluent);
            }
        }
    }
    for np in &goal_num {
        mark(np, &mut relevant_fluent);
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
                if relevant_fluent[ne.target as usize] {
                    let mut v = Vec::new();
                    ne.value.collect_fluents(&mut v);
                    for f in v {
                        let f = f as usize;
                        if f < relevant_fluent.len() && !relevant_fluent[f] {
                            relevant_fluent[f] = true;
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
    let mut add_by_fact = CsrBuilder::new();
    for bucket in add_buckets {
        add_by_fact.push_row(bucket);
    }
    let mut neff_by_fluent = CsrBuilder::new();
    for bucket in neff_buckets {
        neff_by_fluent.push_row(bucket);
    }

    // fluent id -> display string (for metric / cost-fluent lookup in sgp)
    let mut fluent_names = vec![String::new(); intern.fluent_id.len()];
    for ((name, args), id) in &intern.fluent_id {
        fluent_names[*id as usize] = if args.is_empty() {
            format!("({})", name)
        } else {
            format!("({} {})", name, args.join(" "))
        };
    }

    let rel_fluents: Vec<u32> = (0..relevant_fluent.len())
        .filter(|&i| relevant_fluent[i])
        .map(|i| i as u32)
        .collect();

    Outcome::Task(PackedTask {
        n_facts: n_facts_packed,
        words,
        n_ops: n_reach_actions,
        op_display,
        pre_pos: pre_pos.finish(),
        add: add.finish(),
        del: del.finish(),
        pre_num: pre_num.finish(),
        num_eff: num_eff.finish(),
        cond: cond_b.finish(),
        shared_cond,
        monitored: monitored_v,
        add_by_fact: add_by_fact.finish(),
        neff_by_fluent: neff_by_fluent.finish(),
        relevant_fluent,
        rel_fluents,
        init_bits,
        fv0: fv,
        fdef0: fdef,
        goal_pos,
        goal_num,
        fact_names: fact_names_packed,
        fluent_names,
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
