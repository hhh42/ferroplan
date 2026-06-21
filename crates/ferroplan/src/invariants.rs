//! Mutex-group synthesis: Helmert-style monotonicity invariants (multi-predicate).
//!
//! Produces **mutex groups** (sets of ground facts of which at most one is ever
//! true) WITHOUT moving the planner off its propositional bitset state. The groups
//! are exactly what a SAS+ representation would give as multi-valued variables and
//! are the load-bearing dependency for SGPlan/ESPC subgoal partitioning.
//!
//! A candidate invariant has `nparams` universally-quantified parameters and one
//! "counted" variable. Each **member** is a predicate whose argument positions are
//! roles: `Some(j)` binds parameter `j`, `None` is the counted variable (≤1 per
//! member). The group, for each binding of the parameters, is the union over
//! members of the matching ground atoms — at most one ever true.
//!
//! Verification (per action, lifted):
//!   - an add of a member is **balanced** only by a delete of a member with the
//!     same parameter binding *that is also a positive precondition* of the action
//!     (so the removed unit was provably the true one — this is what makes it sound);
//!   - two adds with the same binding ⇒ the candidate is false ("too heavy");
//!   - an unbalanced add is **refined**: branch, extending the candidate with each
//!     deleted-and-required fact that could supply the missing balance, then
//!     re-verify to a fixpoint.
//!
//! Finally every instantiated group must be at-most-one true in the initial state.
//! Anything uncertain is rejected, so emitted groups are always sound.

use crate::packed::PackedTask;
use crate::types::{Domain, Effect, Formula, Term};
use crate::{bitset, hash::FxHashMap};

type Atom = (String, Vec<Term>); // lifted: (lowercased predicate, term args)
type GAtom = (String, Vec<String>); // ground: (lowercased predicate, object args)

const MAX_MEMBERS: usize = 5;
const REFINE_BUDGET: usize = 600;

/// Synthesize sound mutex groups (each a set of ground fact ids, ≥2 members, at
/// most one true at a time) for a grounded task.
pub fn synthesize(domain: &Domain, task: &PackedTask) -> Vec<Vec<u32>> {
    // fact id -> (predicate, args); None for compiled `(NOT …)` facts.
    let keys: Vec<Option<GAtom>> = task.fact_names.iter().map(|s| parse_fact(s)).collect();
    let mut by_pred: FxHashMap<String, Vec<u32>> = FxHashMap::default();
    for (id, k) in keys.iter().enumerate() {
        if let Some((p, _)) = k {
            by_pred.entry(p.clone()).or_default().push(id as u32);
        }
    }

    // Pre-compute each action's precondition / add / delete atoms.
    let effs: Vec<ActionEffects> = domain.actions.iter().map(ActionEffects::of).collect();

    let mut groups: Vec<Vec<u32>> = Vec::new();
    for (name, args) in &domain.predicates {
        let arity = args.len();
        if arity == 0 {
            continue; // nullary predicates only enter as refined members
        }
        let pred = name.to_ascii_lowercase();
        if by_pred.get(&pred).map(|v| v.len()).unwrap_or(0) < 2 {
            continue;
        }
        for counted in 0..arity {
            let seed = seed_candidate(&pred, arity, counted);
            for cand in refine(seed, &effs) {
                if let Some(mut gs) = instantiate(&cand, &keys, &by_pred, task) {
                    groups.append(&mut gs);
                }
            }
        }
    }

    for g in &mut groups {
        g.sort_unstable();
    }
    groups.sort();
    groups.dedup();
    groups
}

/// Parse a rendered fact name `"(pred a0 a1)"` into `(pred, [args])`.
/// Returns None only for `"(NOT …)"` compiled facts. Nullary facts are kept.
fn parse_fact(s: &str) -> Option<GAtom> {
    let inner = s.strip_prefix('(')?.strip_suffix(')')?.trim();
    let mut toks = inner.split_whitespace();
    let pred = toks.next()?;
    if pred.eq_ignore_ascii_case("not") {
        return None;
    }
    let args = toks.map(|t| t.to_string()).collect();
    Some((pred.to_ascii_lowercase(), args))
}

#[derive(Clone)]
struct Member {
    pred: String,
    roles: Vec<Option<usize>>, // Some(param) | None (counted)
}

#[derive(Clone)]
struct Candidate {
    nparams: usize,
    members: Vec<Member>,
}

impl Candidate {
    fn has_pred(&self, p: &str) -> bool {
        self.members.iter().any(|m| m.pred == p)
    }
    /// Parameter binding if `(pred,args)` matches a member, else None.
    fn binding(&self, pred: &str, args: &[Term]) -> Option<Vec<Term>> {
        for m in &self.members {
            if m.pred == pred && m.roles.len() == args.len() {
                let mut bind: Vec<Option<Term>> = vec![None; self.nparams];
                for (i, r) in m.roles.iter().enumerate() {
                    if let Some(j) = r {
                        bind[*j] = Some(args[i].clone());
                    }
                }
                let mut out = Vec::with_capacity(self.nparams);
                for b in bind {
                    out.push(b?);
                }
                return Some(out);
            }
        }
        None
    }
}

fn seed_candidate(pred: &str, arity: usize, counted: usize) -> Candidate {
    let mut roles = Vec::with_capacity(arity);
    let mut next = 0;
    for i in 0..arity {
        if i == counted {
            roles.push(None);
        } else {
            roles.push(Some(next));
            next += 1;
        }
    }
    Candidate {
        nparams: arity - 1,
        members: vec![Member {
            pred: pred.to_string(),
            roles,
        }],
    }
}

struct ActionEffects {
    pre: Vec<Atom>,
    adds: Vec<Atom>,
    dels: Vec<Atom>,
    guarded: Vec<String>, // predicates touched under when/forall (unverifiable)
}

impl ActionEffects {
    fn of(act: &crate::types::Action) -> Self {
        let mut pre = Vec::new();
        collect_pre(&act.precond, &mut pre);
        let mut ae = ActionEffects {
            pre,
            adds: Vec::new(),
            dels: Vec::new(),
            guarded: Vec::new(),
        };
        collect_eff(&act.effect, &mut ae, false);
        ae
    }
}

fn collect_pre(f: &Formula, out: &mut Vec<Atom>) {
    match f {
        Formula::And(fs) => fs.iter().for_each(|x| collect_pre(x, out)),
        Formula::Atom(p, a) => out.push((p.to_ascii_lowercase(), a.clone())),
        _ => {} // negative / disjunctive precond atoms are not "guaranteed true"
    }
}

fn collect_eff(e: &Effect, out: &mut ActionEffects, guarded: bool) {
    match e {
        Effect::And(es) => es.iter().for_each(|x| collect_eff(x, out, guarded)),
        Effect::Add(p, a) => {
            let p = p.to_ascii_lowercase();
            if guarded {
                out.guarded.push(p.clone());
            }
            out.adds.push((p, a.clone()));
        }
        Effect::Del(p, a) => {
            let p = p.to_ascii_lowercase();
            if guarded {
                out.guarded.push(p.clone());
            }
            out.dels.push((p, a.clone()));
        }
        Effect::When(_, inner) => collect_eff(inner, out, true),
        Effect::Forall(_, inner) => collect_eff(inner, out, true),
        _ => {}
    }
}

fn term_eq(a: &Term, b: &Term) -> bool {
    match (a, b) {
        (Term::Var(x), Term::Var(y)) => x.eq_ignore_ascii_case(y),
        (Term::Const(x), Term::Const(y)) => x.eq_ignore_ascii_case(y),
        _ => false,
    }
}

fn terms_eq(a: &[Term], b: &[Term]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| term_eq(x, y))
}

fn atom_required(pre: &[Atom], pred: &str, args: &[Term]) -> bool {
    pre.iter().any(|(p, a)| p == pred && terms_eq(a, args))
}

enum Verdict {
    Balanced,
    Drop,
    Refine(Vec<Member>),
}

fn first_problem(cand: &Candidate, effs: &[ActionEffects]) -> Verdict {
    for ae in effs {
        if cand.members.iter().any(|m| ae.guarded.contains(&m.pred)) {
            return Verdict::Drop;
        }
        let adds: Vec<Vec<Term>> = ae
            .adds
            .iter()
            .filter_map(|(p, a)| cand.binding(p, a))
            .collect();
        // too heavy: two adds into the same group instance
        for i in 0..adds.len() {
            for j in (i + 1)..adds.len() {
                if terms_eq(&adds[i], &adds[j]) {
                    return Verdict::Drop;
                }
            }
        }
        // deletes that the action also *requires* (so removal is effective)
        let dels_req: Vec<Vec<Term>> = ae
            .dels
            .iter()
            .filter(|(p, a)| atom_required(&ae.pre, p, a))
            .filter_map(|(p, a)| cand.binding(p, a))
            .collect();
        for ab in &adds {
            if !dels_req.iter().any(|db| terms_eq(ab, db)) {
                let props = propose(cand, ae, ab);
                return if props.is_empty() {
                    Verdict::Drop
                } else {
                    Verdict::Refine(props)
                };
            }
        }
    }
    Verdict::Balanced
}

/// New members that would balance an unbalanced add `ab`: a deleted-and-required
/// fact of a not-yet-member predicate, whose args bind the parameters of `ab`.
fn propose(cand: &Candidate, ae: &ActionEffects, ab: &[Term]) -> Vec<Member> {
    let mut out = Vec::new();
    for (p, a) in &ae.dels {
        if cand.has_pred(p) || !atom_required(&ae.pre, p, a) {
            continue;
        }
        if let Some(m) = build_member(p, a, ab, cand.nparams) {
            out.push(m);
        }
    }
    out
}

fn build_member(pred: &str, args: &[Term], binding: &[Term], nparams: usize) -> Option<Member> {
    let mut roles = vec![None; args.len()];
    let mut covered = vec![false; nparams];
    let mut counted = 0;
    for (i, t) in args.iter().enumerate() {
        match (0..nparams).find(|&j| term_eq(t, &binding[j])) {
            Some(j) => {
                roles[i] = Some(j);
                covered[j] = true;
            }
            None => counted += 1,
        }
    }
    if counted > 1 || !covered.iter().all(|&c| c) {
        return None; // need every param bound and at most one counted position
    }
    Some(Member {
        pred: pred.to_string(),
        roles,
    })
}

/// Branch-and-verify a seed to all balanced extensions.
fn refine(seed: Candidate, effs: &[ActionEffects]) -> Vec<Candidate> {
    let mut out = Vec::new();
    let mut stack = vec![seed];
    let mut budget = REFINE_BUDGET;
    while let Some(cand) = stack.pop() {
        if budget == 0 {
            break;
        }
        budget -= 1;
        match first_problem(&cand, effs) {
            Verdict::Balanced => out.push(cand),
            Verdict::Drop => {}
            Verdict::Refine(props) => {
                if cand.members.len() < MAX_MEMBERS {
                    for m in props {
                        if cand.has_pred(&m.pred) {
                            continue;
                        }
                        let mut c2 = cand.clone();
                        c2.members.push(m);
                        stack.push(c2);
                    }
                }
            }
        }
    }
    out
}

/// Instantiate the per-parameter-binding ground groups; None if a group is not
/// at-most-one true in the initial state.
fn instantiate(
    cand: &Candidate,
    keys: &[Option<GAtom>],
    by_pred: &FxHashMap<String, Vec<u32>>,
    task: &PackedTask,
) -> Option<Vec<Vec<u32>>> {
    let mut by_binding: FxHashMap<Vec<String>, Vec<u32>> = FxHashMap::default();
    for m in &cand.members {
        let Some(ids) = by_pred.get(&m.pred) else {
            continue;
        };
        for &id in ids {
            let (_, args) = keys[id as usize].as_ref()?;
            if args.len() != m.roles.len() {
                continue;
            }
            let mut tuple = vec![String::new(); cand.nparams];
            for (i, r) in m.roles.iter().enumerate() {
                if let Some(j) = r {
                    tuple[*j] = args[i].to_ascii_lowercase();
                }
            }
            by_binding.entry(tuple).or_default().push(id);
        }
    }
    let mut out = Vec::new();
    for (_, g) in by_binding {
        let true_in_init = g
            .iter()
            .filter(|&&id| bitset::test(&task.init_bits, id as usize))
            .count();
        if true_in_init > 1 {
            return None;
        }
        if g.len() >= 2 {
            out.push(g);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground::{ground, Outcome};
    use crate::parser::{parse_domain, parse_problem};

    fn groups_of(dom: &str, prob: &str) -> (PackedTask, Vec<Vec<u32>>) {
        let d = parse_domain(dom).expect("domain");
        let p = parse_problem(prob).expect("problem");
        let task = match ground(&d, &p, 1) {
            Outcome::Task(t) => t,
            _ => panic!("expected a task"),
        };
        let g = synthesize(&d, &task);
        (task, g)
    }

    fn names(task: &PackedTask, g: &[u32]) -> Vec<String> {
        let mut v: Vec<String> = g
            .iter()
            .map(|&i| task.fact_names[i as usize].to_lowercase())
            .collect();
        v.sort();
        v
    }

    fn has_group_with(task: &PackedTask, groups: &[Vec<u32>], preds: &[&str]) -> bool {
        groups.iter().any(|g| {
            let ns = names(task, g);
            preds
                .iter()
                .all(|pred| ns.iter().any(|n| n.starts_with(&format!("({pred} "))))
        })
    }

    const LOGISTICS: &str = "(define (domain log)
      (:requirements :strips :typing)
      (:types vehicle package location)
      (:predicates (at ?x - object ?l - location) (in ?p - package ?v - vehicle)
                   (road ?a ?b - location))
      (:action drive :parameters (?v - vehicle ?from ?to - location)
        :precondition (and (at ?v ?from) (road ?from ?to))
        :effect (and (not (at ?v ?from)) (at ?v ?to)))
      (:action load :parameters (?p - package ?v - vehicle ?l - location)
        :precondition (and (at ?p ?l) (at ?v ?l))
        :effect (and (not (at ?p ?l)) (in ?p ?v)))
      (:action unload :parameters (?p - package ?v - vehicle ?l - location)
        :precondition (and (in ?p ?v) (at ?v ?l))
        :effect (and (not (in ?p ?v)) (at ?p ?l))))";
    const LOG_PROB: &str = "(define (problem p) (:domain log)
      (:objects v1 - vehicle pk1 - package a b c - location)
      (:init (at v1 a) (at pk1 b) (road a b) (road b c) (road a c))
      (:goal (at pk1 c)))";

    #[test]
    fn package_location_spans_at_and_in() {
        let (task, groups) = groups_of(LOGISTICS, LOG_PROB);
        // a package's location is the multi-predicate variable {at(pk,·), in(pk,·)}
        assert!(
            has_group_with(&task, &groups, &["at", "in"]),
            "expected a package-location group spanning at + in; got {:?}",
            groups.iter().map(|g| names(&task, g)).collect::<Vec<_>>()
        );
    }

    const BLOCKS: &str = "(define (domain blocks)
      (:requirements :strips :typing)
      (:types block)
      (:predicates (on ?x ?y - block) (ontable ?x - block) (clear ?x - block)
                   (handempty) (holding ?x - block))
      (:action pickup :parameters (?x - block)
        :precondition (and (clear ?x) (ontable ?x) (handempty))
        :effect (and (not (ontable ?x)) (not (clear ?x)) (not (handempty)) (holding ?x)))
      (:action putdown :parameters (?x - block)
        :precondition (holding ?x)
        :effect (and (not (holding ?x)) (clear ?x) (handempty) (ontable ?x)))
      (:action stack :parameters (?x ?y - block)
        :precondition (and (holding ?x) (clear ?y))
        :effect (and (not (holding ?x)) (not (clear ?y)) (clear ?x) (handempty) (on ?x ?y)))
      (:action unstack :parameters (?x ?y - block)
        :precondition (and (on ?x ?y) (clear ?x) (handempty))
        :effect (and (holding ?x) (clear ?y) (not (clear ?x)) (not (on ?x ?y)) (not (handempty)))))";
    const BLOCKS_PROB: &str = "(define (problem p) (:domain blocks)
      (:objects a b c - block)
      (:init (ontable a) (on b a) (on c b) (clear c) (handempty))
      (:goal (on a b)))";

    #[test]
    fn blocks_support_and_hand_are_groups() {
        let (task, groups) = groups_of(BLOCKS, BLOCKS_PROB);
        let pretty = || groups.iter().map(|g| names(&task, g)).collect::<Vec<_>>();
        // a block's support variable: {on(x,·), ontable(x), holding(x)}
        assert!(
            has_group_with(&task, &groups, &["on", "ontable", "holding"]),
            "expected block-support group; got {:?}",
            pretty()
        );
        // the hand variable: {handempty, holding(·)}
        assert!(
            groups.iter().any(|g| {
                let ns = names(&task, g);
                ns.iter().any(|n| n.starts_with("(handempty"))
                    && ns.iter().any(|n| n.starts_with("(holding "))
            }),
            "expected hand group {{handempty, holding}}; got {:?}",
            pretty()
        );
    }

    const GRIPPER: &str = "(define (domain gripper)
      (:requirements :strips :typing)
      (:types room ball)
      (:predicates (at-robby ?r - room) (ball-at ?b - ball ?r - room) (carry ?b - ball))
      (:action move :parameters (?from ?to - room)
        :precondition (at-robby ?from)
        :effect (and (not (at-robby ?from)) (at-robby ?to)))
      (:action pick :parameters (?b - ball ?r - room)
        :precondition (and (ball-at ?b ?r) (at-robby ?r))
        :effect (and (not (ball-at ?b ?r)) (carry ?b)))
      (:action drop :parameters (?b - ball ?r - room)
        :precondition (and (carry ?b) (at-robby ?r))
        :effect (and (not (carry ?b)) (ball-at ?b ?r))))";
    const GRIP_PROB: &str = "(define (problem p) (:domain gripper)
      (:objects ra rb - room b1 - ball)
      (:init (at-robby ra) (ball-at b1 ra))
      (:goal (at-robby rb)))";

    #[test]
    fn robby_room_and_ball_location() {
        let (task, groups) = groups_of(GRIPPER, GRIP_PROB);
        let pretty = || groups.iter().map(|g| names(&task, g)).collect::<Vec<_>>();
        assert!(
            has_group_with(&task, &groups, &["at-robby"]),
            "expected at-robby group; got {:?}",
            pretty()
        );
        // a ball's location spans {ball-at(b,·), carry(b)}
        assert!(
            has_group_with(&task, &groups, &["ball-at", "carry"]),
            "expected ball-location group spanning ball-at + carry; got {:?}",
            pretty()
        );
    }
}
