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
use crate::hash::FxHashMap;
use crate::packed::{CondEff, CsrBuilder, PackedTask, State};
use crate::par;
use crate::types::*;

pub enum Outcome {
    Task(PackedTask),
    GoalTrue,
    GoalFalse,
    GoalUndefinedFluent,
    EmptyType { kind: &'static str, pred: String, ty: String },
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
    Conjunct { pos: vec![], neg: vec![], num: vec![] }
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
fn quant_expand(
    vars: &[(Sym, Sym)],
    inner: &Formula,
    b: &HashMap<Sym, Sym>,
    neg: bool,
    objs: &HashMap<Sym, Vec<Sym>>,
    use_and: bool,
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
            acc = and_merge(&acc, &to_dnf(inner, cb, neg, objs));
        }
        acc
    } else {
        combos.iter().flat_map(|cb| to_dnf(inner, cb, neg, objs)).collect()
    }
}

fn to_dnf(f: &Formula, b: &HashMap<Sym, Sym>, negated: bool, objs: &HashMap<Sym, Vec<Sym>>) -> Vec<Conjunct> {
    match (f, negated) {
        (Formula::True, false) | (Formula::False, true) => vec![empty_conj()],
        (Formula::False, false) | (Formula::True, true) => vec![],
        (Formula::Atom(p, a), false) => vec![Conjunct {
            pos: vec![(p.clone(), subst_args(a, b))],
            neg: vec![],
            num: vec![],
        }],
        (Formula::Atom(p, a), true) => vec![Conjunct {
            pos: vec![],
            neg: vec![(p.clone(), subst_args(a, b))],
            num: vec![],
        }],
        // `(not (= e1 e2))` has no single comparator: it is the DISJUNCTION
        // e1 < e2 OR e1 > e2, i.e. two DNF conjuncts.
        (Formula::Comp(CompOp::Eq, l, r), true) => {
            let le = subst_expr(l, b);
            let re = subst_expr(r, b);
            vec![
                Conjunct { pos: vec![], neg: vec![], num: vec![(CompOp::Lt, le.clone(), re.clone())] },
                Conjunct { pos: vec![], neg: vec![], num: vec![(CompOp::Gt, le, re)] },
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
        (Formula::Forall(vars, inner), neg) => quant_expand(vars, inner, b, neg, objs, !neg),
        (Formula::Exists(vars, inner), neg) => quant_expand(vars, inner, b, neg, objs, neg),
        // A preference is a SOFT goal — a classical planner ignores it (True).
        (Formula::Pref(_, _), false) => vec![empty_conj()],
        (Formula::Pref(_, _), true) => vec![],
        (Formula::Not(inner), neg) => to_dnf(inner, b, !neg, objs),
        (Formula::And(fs), false) | (Formula::Or(fs), true) => {
            let mut acc = vec![empty_conj()];
            for child in fs {
                acc = and_merge(&acc, &to_dnf(child, b, negated, objs));
            }
            acc
        }
        (Formula::Or(fs), false) | (Formula::And(fs), true) => {
            let mut acc = Vec::new();
            for child in fs {
                acc.extend(to_dnf(child, b, negated, objs));
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
        Effect::And(v) => v.iter().for_each(|x| ground_effect(x, b, objs, ctx, out)),
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
                ground_effect(inner, cb, objs, ctx, out);
            }
        }
        Effect::When(cond, inner) => {
            // each DNF disjunct of the condition is one conditional context
            for disj in to_dnf(cond, b, false, objs) {
                let merged = merge_conj(ctx, &disj);
                ground_effect(inner, b, objs, &merged, out);
            }
        }
    }
}

fn static_top_atoms(f: &Formula, add_preds: &HashSet<Sym>) -> Vec<(Sym, Vec<Term>)> {
    let mut out = Vec::new();
    fn rec(f: &Formula, add_preds: &HashSet<Sym>, out: &mut Vec<(Sym, Vec<Term>)>) {
        match f {
            Formula::And(v) => v.iter().for_each(|x| rec(x, add_preds, out)),
            Formula::Atom(p, a) => {
                if !add_preds.contains(p) {
                    out.push((p.clone(), a.clone()));
                }
            }
            _ => {}
        }
    }
    rec(f, add_preds, &mut out);
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
}

fn for_each_binding(
    params: &[(Sym, Sym)],
    objects_of_type: &HashMap<Sym, Vec<Sym>>,
    mut f: impl FnMut(&HashMap<Sym, Sym>),
) {
    let mut domains: Vec<&Vec<Sym>> = Vec::with_capacity(params.len());
    for (_, ty) in params {
        match objects_of_type.get(ty) {
            Some(v) if !v.is_empty() => domains.push(v),
            _ => return,
        }
    }
    let mut idx = vec![0usize; params.len()];
    let mut binding: HashMap<Sym, Sym> = HashMap::new();
    loop {
        for (k, (v, _)) in params.iter().enumerate() {
            binding.insert(v.clone(), domains[k][idx[k]].clone());
        }
        f(&binding);
        let mut k = params.len();
        loop {
            if k == 0 {
                return;
            }
            k -= 1;
            idx[k] += 1;
            if idx[k] < domains[k].len() {
                break;
            }
            idx[k] = 0;
        }
    }
}

/// Phase B (parallelisable): all ground RawOps for a single action.
fn ground_action(
    action: &Action,
    objects_of_type: &HashMap<Sym, Vec<Sym>>,
    init_atom_set: &HashSet<(Sym, Vec<Sym>)>,
    add_predicates: &HashSet<Sym>,
) -> Vec<RawOp> {
    let static_lits = static_top_atoms(&action.precond, add_predicates);
    let param_vars: Vec<Sym> = action.params.iter().map(|(v, _)| v.clone()).collect();
    let mut out = Vec::new();
    for_each_binding(&action.params, objects_of_type, |b| {
        for (p, a) in &static_lits {
            let ga = subst_args(a, b);
            if !init_atom_set.contains(&(p.clone(), ga)) {
                return;
            }
        }
        let dnf = to_dnf(&action.precond, b, false, objects_of_type);
        let multi = dnf.len() > 1;
        let mut eff = REff {
            add: vec![],
            del: vec![],
            num: vec![],
            cond: vec![],
        };
        ground_effect(&action.effect, b, objects_of_type, &empty_conj(), &mut eff);
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
            Expr::Add(x, y) => NExpr::Add(Box::new(self.resolve_expr(x, reads)), Box::new(self.resolve_expr(y, reads))),
            Expr::Sub(x, y) => NExpr::Sub(Box::new(self.resolve_expr(x, reads)), Box::new(self.resolve_expr(y, reads))),
            Expr::Mul(x, y) => NExpr::Mul(Box::new(self.resolve_expr(x, reads)), Box::new(self.resolve_expr(y, reads))),
            Expr::Div(x, y) => NExpr::Div(Box::new(self.resolve_expr(x, reads)), Box::new(self.resolve_expr(y, reads))),
            Expr::Neg(x) => NExpr::Neg(Box::new(self.resolve_expr(x, reads))),
        }
    }
}

/// A mid-form operator: fact ids interned, numeric resolved, neg still string.
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
    cond_atoms: Vec<(Vec<(Sym, Vec<Sym>)>, Vec<(Sym, Vec<Sym>)>)>,
}

/// Grounding entry. `ground` does PDDL goal simplification (TRUE/FALSE early
/// exits); `ground_task` forces a Task even for trivial/unreachable goals — for
/// validators that must execute a plan regardless of goal triviality.
pub fn ground(domain: &Domain, problem: &Problem, threads: usize) -> Outcome {
    ground_v(domain, problem, threads, false)
}

/// Always return the grounded Task (skips goal TRUE/FALSE/undefined verdicts);
/// None only on a fatal empty-type error.
pub fn ground_task(domain: &Domain, problem: &Problem, threads: usize) -> Option<PackedTask> {
    match ground_v(domain, problem, threads, true) {
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
        let mut cur = a.clone();
        loop {
            if &cur == b {
                return true;
            }
            match tp.get(&cur) {
                Some(p) => cur = p.clone(),
                None => return false,
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

fn ground_v(domain: &Domain, problem: &Problem, threads: usize, validate: bool) -> Outcome {
    // ---- type system ----
    let objects_of_type = objects_by_type(domain, problem);

    // ---- empty-type check (predicates then functions) ----
    let empty = |ty: &Sym| objects_of_type.get(ty).map(|v| v.is_empty()).unwrap_or(true);
    for (pname, argtypes) in &domain.predicates {
        for ty in argtypes {
            if empty(ty) {
                return Outcome::EmptyType {
                    kind: "predicate",
                    pred: pname.clone(),
                    ty: ty.clone(),
                };
            }
        }
    }
    for (fname, argtypes) in &domain.functions {
        for ty in argtypes {
            if empty(ty) {
                return Outcome::EmptyType {
                    kind: "function",
                    pred: fname.clone(),
                    ty: ty.clone(),
                };
            }
        }
    }

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
    let init_atom_set: HashSet<(Sym, Vec<Sym>)> = problem.init_atoms.iter().cloned().collect();

    // ---- Phase B: parallel per-action grounding ----
    let action_idx: Vec<usize> = (0..domain.actions.len()).collect();
    let raw_chunks: Vec<Vec<RawOp>> = par::par_map(&action_idx, threads, |&ai| {
        ground_action(&domain.actions[ai], &objects_of_type, &init_atom_set, &add_predicates)
    });
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
            num_eff.push(NumEff { op: *op, target, value });
        }
        // intern conditional effects. Condition reads are NOT added to `reads`:
        // an undefined fluent in a condition means it simply won't fire, it does
        // not make the operator illegal.
        let mut cond = Vec::new();
        let mut cond_atoms = Vec::new();
        for rc in &r.eff.cond {
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
                cnum.push(NumEff { op: *op, target, value });
            }
            cond.push(CondEff { cond_pos, cond_neg, cond_num, add: cadd, del: cdel, num: cnum });
            cond_atoms.push((rc.add.clone(), rc.del.clone()));
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
        });
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
    let goal_dnf = to_dnf(&problem.goal, &HashMap::new(), false, &objects_of_type);
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
        });
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
            });
        }
        goal_conj_owned = Conjunct { pos: vec![gatom], neg: vec![], num: vec![] };
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
            }
        }
        if !changed {
            break;
        }
    }
    let reach_ops: Vec<&FinalOp> = fops.iter().enumerate().filter(|(i, _)| live[*i]).map(|(_, o)| o).collect();

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
    let deletable: HashSet<u32> = reach_ops
        .iter()
        .flat_map(|o| o.del.iter().copied().chain(o.cond.iter().flat_map(|c| c.del.iter().copied())))
        .collect();
    let modified_fluents: HashSet<u32> = reach_ops
        .iter()
        .flat_map(|o| {
            o.num_eff
                .iter()
                .map(|ne| ne.target)
                .chain(o.cond.iter().flat_map(|c| c.num.iter().map(|ne| ne.target)))
        })
        .collect();
    let inertia_pos = |f: u32| init_true.get(f as usize).copied().unwrap_or(false) && !deletable.contains(&f);
    let mut np_reads = Vec::new();
    let inertia_num = |np: &NumPre, scratch: &mut Vec<u32>| {
        scratch.clear();
        np.lhs.collect_fluents(scratch);
        np.rhs.collect_fluents(scratch);
        eval_numpre(np, &fv, &fdef).unwrap_or(false) && scratch.iter().all(|fl| !modified_fluents.contains(fl))
    };
    let remaining_pos: Vec<u32> = goal_pos.iter().copied().filter(|&f| !inertia_pos(f)).collect();
    let remaining_num = goal_num.iter().filter(|np| !inertia_num(np, &mut np_reads)).count();
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

    // ---- pack into CSR ----
    let words = bitset::words_for(n_facts2);
    let mut init_bits = vec![0u64; words];
    for (i, &b) in init_true.iter().enumerate() {
        if b {
            bitset::set(&mut init_bits, i);
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
    let mut add_buckets: Vec<Vec<u32>> = vec![Vec::new(); n_facts2];
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
    for (oi, op) in reach_ops.iter().enumerate() {
        op_display.push(op.display.clone());
        pre_pos.push_row(op.pre_pos.iter().copied());
        add.push_row(op.add.iter().copied());
        del.push_row(op.del.iter().copied());
        pre_num.push_row(op.pre_num.iter().cloned());
        num_eff.push_row(op.num_eff.iter().cloned());
        cond_b.push_row(op.cond.iter().cloned());
        for &f in &op.add {
            add_buckets[f as usize].push(oi as u32);
        }
        // fluent -> ops with a numeric effect on it (distinct targets, op-id order)
        let mut seen_t: Vec<u32> = Vec::new();
        for ne in &op.num_eff {
            if !seen_t.contains(&ne.target) {
                seen_t.push(ne.target);
                neff_buckets[ne.target as usize].push(oi as u32);
            }
        }
        // conditional adds also have this op as an achiever
        for ce in &op.cond {
            for &f in &ce.add {
                add_buckets[f as usize].push(oi as u32);
            }
            for np in &ce.cond_num {
                mark(np, &mut relevant_fluent);
            }
        }
        for np in &op.pre_num {
            mark(np, &mut relevant_fluent);
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
            let neffs = op.num_eff.iter().chain(op.cond.iter().flat_map(|c| c.num.iter()));
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
        n_facts: n_facts2,
        words,
        n_ops: n_reach_actions,
        op_display,
        pre_pos: pre_pos.finish(),
        add: add.finish(),
        del: del.finish(),
        pre_num: pre_num.finish(),
        num_eff: num_eff.finish(),
        cond: cond_b.finish(),
        add_by_fact: add_by_fact.finish(),
        neff_by_fluent: neff_by_fluent.finish(),
        relevant_fluent,
        rel_fluents,
        init_bits,
        fv0: fv,
        fdef0: fdef,
        goal_pos,
        goal_num,
        fact_names: intern.fact_names,
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
