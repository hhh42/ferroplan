//! Goal-respecting object-symmetry orbits (0.14 ext Phase 10) — the
//! research lever the 0.13 TMS diagnosis spec'd: temporal-machine-shop is
//! 0/20 because interchangeable pieces, distinguished ONLY by which
//! `(baked-structure p q)` goal pair they serve, make every
//! subset-assignment of "which identical piece is baking" a distinct
//! visited state.
//!
//! The reduction: detect orbits of interchangeable MEMBER UNITS (single
//! objects, or the goal-pair tuples of the TMS shape), then canonicalize
//! every visited key under member relabeling — states differing only by a
//! permutation of interchangeable members collapse to one representative.
//! Plans stay concrete; only the visited space shrinks.
//!
//! Grounded machinery: every fact/op/fluent display touching an orbit
//! object joins a FAMILY — displays sharing one (head, literal/slot
//! pattern), stored as a dense table over member coordinates. A member
//! permutation σ then acts on the whole grounded task by table lookup,
//! so facts COUPLING several members (TMS grounds `(assemble p q)` for
//! every cross-pair combination) permute right along with the per-member
//! ones instead of killing the orbit.
//!
//! Soundness by construction, conservative at every step:
//!
//! - Candidate members must have identical init profiles (statics and
//!   fluents included), appear in no action schema literally, and pass a
//!   per-family CLOSURE check: within each equality-pattern class of
//!   member coordinates, cells are uniformly present or uniformly absent
//!   — i.e. the grounded task really is closed under every member
//!   transposition. Any violation drops detection entirely.
//! - Goal facts must be per-member and shared by every member of their
//!   orbit (the goal SET is then σ-invariant); numeric goals over touched
//!   fluents, TILs, derived rules, PDDL3 constraints, and non-total-time
//!   metrics all bail — each could distinguish members invisibly.
//! - The canonical form σ(s) is a pure function of the state, chosen by
//!   sorting per-member signatures, so determinism and t1 ≡ t8 hold. Any
//!   σ is sound: canon(s1) = canon(s2) implies s2 = σ2⁻¹σ1(s1), a true
//!   automorphism image (ties may MISS merges, never mis-merge).
//! - Applied to VISITED KEYS only — the temporal key (state bits,
//!   relevant fluent values, and the pending-end agenda's op ids all
//!   permute together) since 0.14 ext, and the CLASSICAL keys since 0.22
//!   Phase 6 ([`OrbitMap::canonical_skey`]: optimal mode's
//!   visited/closed/best_g sites and the satisficing dedup in the
//!   parallel successor phase). Nodes stay CONCRETE everywhere, so plan
//!   extraction and VAL are untouched. Callers must pass a σ-invariant
//!   `forbidden` mask (the CLI passes none; Session/tresolve pass no
//!   orbit at all — recorded decision).
//! - A non-total-time metric bails UNLESS it is the classical
//!   single-fluent `minimize` shape AND every op family's constant costs
//!   are uniform within each equality class (the 0.22 Phase 6 L2 gate —
//!   the soundness trap is merging quality-distinct states, so any cost
//!   the gate cannot certify as a symmetric constant drops detection).
//!
//! `FF_NO_ORBIT=1` disables detection entirely;
//! `FF_NO_ORBIT_CLASSICAL=1` kills only the classical consumer
//! ([`detect_classical`]) while the temporal one keeps its orbit.
//!
//! **The goal-isomorphism arm (0.23 Phase 4 probe 1, `FF_ORBIT_ISO=1`,
//! default OFF):** detection extends from goal-σ-INVARIANCE to
//! goal-ISOMORPHISM — units form GOAL-BLIND (type + init profile only,
//! the goal never splits an orbit), and each goal atom touching an orbit
//! becomes a DESIGNATION (family + member coords) instead of an
//! invariance obligation. σ then acts jointly on objects and goal atoms:
//! canonicalization merges states across the full goal-blind group, and
//! the goal test relaxes to "some σ maps this state onto the goal"
//! ([`OrbitMap::iso_goal_witness`], an exact designation-matching search
//! that returns the WITNESS permutation). Consumers wired for the arm
//! (the temporal search, optimal A*) apply the witness to the emitted
//! plan through the op family tables ([`OrbitMap::iso_remap_op`]), so
//! the plan that reaches σ(goal) is emitted as its σ-image serving the
//! ORIGINAL goal — sound because equal init profiles force σ(init) =
//! init (see the round-trip fixtures). Pruning stays complete because a
//! merged state's every reachable goal image is still recognized by the
//! relaxed test; optimal mode keeps its certificate by weakening h to
//! the σ-invariant goal part (admissible for every σ-image). Consumers
//! NOT wired for the relaxed test never receive an iso map
//! ([`detect_classical`] stays strict by construction).
//!
//! Probe verdict, recorded at the probe (the 0.23 roadmap's
//! pre-registered reads): TMS-2011 i1 FAILED both halves — the
//! goal-blind orbits form exactly as designed ([10, 15, 25] pieces, 25
//! designations live) but the distinct-visited-class collapse reads
//! 1.27× at the 60k eval budget (11,114 → 8,780; ≥10× pre-registered)
//! and best_h sits at the 110 stock floor at 15k/30k/60k, BOTH arms —
//! the 0.14 pair orbits had already banked the group's cheap coset, and
//! the plateau is start-epoch choreography, exactly what the 0.22
//! agenda-doom receipts predicted. The TEMPORAL constituency records
//! DEAD; the arm is CLASSICAL-ONLY (opt-in, default-off), refereed on
//! the goal-paired classical shapes — child-snack stays INERT by
//! construction (uniform unary designations are σ-invariant already,
//! so its residue is out of this lever's reach and its optimal h is
//! untouched).

use crate::hash::{FxHashMap, FxHashSet};
use crate::packed::{PackedTask, State};
use crate::types::{AssignOp, Domain, Expr, Formula, NumPre, Problem, Sym, Term};
use std::collections::{BTreeMap, BTreeSet};

/// One orbit: `k` interchangeable member units. Per member, the SAME
/// template list (single-member facts / relevant-fluent slots / ops, in
/// family order) — the sort key that picks σ. Cross-member entries live
/// in the families, not here.
#[derive(Clone)]
pub struct Orbit {
    /// member -> fact ids, in template order.
    pub facts: Vec<Vec<u32>>,
    /// member -> relevant-fluent SLOT indexes (into `rel_fluents`), template order.
    pub fluent_slots: Vec<Vec<usize>>,
    /// member -> op ids, in template order (for agenda signatures).
    pub ops: Vec<Vec<usize>>,
}

/// All displays sharing one (head, pattern) — the pattern fixes literals
/// and (orbit, obj-within-member) slots; the table stores the concrete id
/// per member-coordinate tuple, row-major, `u32::MAX` = absent.
#[derive(Clone)]
struct Family {
    /// orbit index per slot position.
    axes: Vec<u16>,
    /// member count per slot position.
    dims: Vec<u32>,
    table: Vec<u32>,
}

impl Family {
    fn flat(&self, coords: &[u16]) -> usize {
        let mut ix = 0usize;
        for (d, &c) in coords.iter().enumerate() {
            ix = ix * self.dims[d] as usize + c as usize;
        }
        ix
    }
    /// Image id under per-orbit member permutations (`sigma[orbit][src] = dst`).
    fn map(&self, coords: &[u16], sigma: &[Vec<u16>]) -> u32 {
        let mut ix = 0usize;
        for (d, &c) in coords.iter().enumerate() {
            ix = ix * self.dims[d] as usize + sigma[self.axes[d] as usize][c as usize] as usize;
        }
        self.table[ix]
    }
    /// [`Family::map`] with a functional σ — avoids materializing per-orbit
    /// permutation vectors for the pairwise stabilizer tests.
    fn map_with(&self, coords: &[u16], sigma: impl Fn(u16, u16) -> u16) -> u32 {
        let mut ix = 0usize;
        for (d, &c) in coords.iter().enumerate() {
            ix = ix * self.dims[d] as usize + sigma(self.axes[d], c) as usize;
        }
        self.table[ix]
    }
    /// Closure under every member transposition: cells whose coordinates
    /// share an equality pattern (per orbit — different orbits never
    /// interact) must be uniformly present/absent. σ preserves equality
    /// and distinctness of same-orbit coordinates, so class uniformity is
    /// exactly "the table is closed under the whole product group".
    fn closed(&self) -> bool {
        let n = self.axes.len(); // ≤ 16, enforced at creation
        let mut coords = vec![0u16; n];
        let mut classes: FxHashMap<u128, bool> = FxHashMap::default();
        loop {
            let mut code: u128 = 0;
            for d in 0..n {
                let mut c = d as u128;
                for e in 0..d {
                    if self.axes[e] == self.axes[d] && coords[e] == coords[d] {
                        c = e as u128;
                        break;
                    }
                }
                code = (code << 8) | c;
            }
            let present = self.table[self.flat(&coords)] != u32::MAX;
            if *classes.entry(code).or_insert(present) != present {
                return false;
            }
            // odometer
            let mut d = n;
            loop {
                if d == 0 {
                    return true;
                }
                d -= 1;
                coords[d] += 1;
                if (coords[d] as u32) < self.dims[d] {
                    break;
                }
                coords[d] = 0;
            }
        }
    }
    /// The 0.22 Phase 6 L2 gate, per op family: σ maps a cell to any
    /// other cell of its equality class, so canonicalization merges
    /// states whose remaining plans swap those ops — sound for quality
    /// only if every present cell of a class carries the SAME certified
    /// constant cost. A cell whose cost the caller could not certify
    /// (`None`: state-dependent, conditional, or non-increase) fails the
    /// gate outright. Same equality-class walk as [`Self::closed`].
    fn cost_uniform(&self, cost: &[Option<f64>]) -> bool {
        let n = self.axes.len(); // ≤ 16, enforced at creation
        let mut coords = vec![0u16; n];
        let mut classes: FxHashMap<u128, f64> = FxHashMap::default();
        loop {
            let id = self.table[self.flat(&coords)];
            if id != u32::MAX {
                let Some(c) = cost[id as usize] else {
                    return false;
                };
                let mut code: u128 = 0;
                for d in 0..n {
                    let mut cc = d as u128;
                    for e in 0..d {
                        if self.axes[e] == self.axes[d] && coords[e] == coords[d] {
                            cc = e as u128;
                            break;
                        }
                    }
                    code = (code << 8) | cc;
                }
                if *classes.entry(code).or_insert(c) != c {
                    return false;
                }
            }
            // odometer
            let mut d = n;
            loop {
                if d == 0 {
                    return true;
                }
                d -= 1;
                coords[d] += 1;
                if (coords[d] as u32) < self.dims[d] {
                    break;
                }
                coords[d] = 0;
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
enum Pat {
    Lit(String),
    Slot(u16, u8),
}

/// Family index under construction for one id space (facts, ops, fluents).
struct FamSet {
    idx: FxHashMap<(String, Vec<Pat>), u32>,
    fams: Vec<Family>,
    /// (id, family, coords) for every touched display.
    touch: Vec<(u32, u32, Vec<u16>)>,
}

/// Total table cells across all families — a runaway grounded space (huge
/// same-type object counts × arity) bails detection rather than eating
/// memory. TMS instance 20 sits far below this.
const CELL_CAP: usize = 1 << 22;

impl FamSet {
    fn new() -> Self {
        FamSet {
            idx: FxHashMap::default(),
            fams: Vec::new(),
            touch: Vec::new(),
        }
    }
    /// Register one display. `record` gates the touch (rewrite) list: a
    /// STATIC fact still enters its family table — closure must verify the
    /// automorphism fixes statics — but its bit is init-constant and
    /// σ-invariant, so the per-node rewrite skips it. `Ok(true)` = touched,
    /// `Ok(false)` = no orbit object, `Err(())` = table budget blown.
    fn add(
        &mut self,
        disp: &str,
        id: u32,
        owner: &FxHashMap<String, (u16, u16, u8)>,
        k: &[u32],
        cells: &mut usize,
        record: bool,
    ) -> Result<bool, ()> {
        let (head, args) = parse(disp);
        let mut pats = Vec::with_capacity(args.len());
        let mut axes = Vec::new();
        let mut coords: Vec<u16> = Vec::new();
        for a in args {
            match owner.get(a.as_str()) {
                Some(&(o, m, oi)) => {
                    pats.push(Pat::Slot(o, oi));
                    axes.push(o);
                    coords.push(m);
                }
                None => pats.push(Pat::Lit(a)),
            }
        }
        if coords.is_empty() {
            return Ok(false);
        }
        use std::collections::hash_map::Entry;
        let fam = match self.idx.entry((head, pats)) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(v) => {
                if axes.len() > 16 {
                    return Err(()); // closure's class code packs 8 bits/position
                }
                let dims: Vec<u32> = axes.iter().map(|&o| k[o as usize]).collect();
                let mut size = 1usize;
                for &d in &dims {
                    size = size.saturating_mul(d as usize);
                }
                *cells += size;
                if *cells > CELL_CAP {
                    return Err(());
                }
                let f = self.fams.len() as u32;
                v.insert(f);
                self.fams.push(Family {
                    axes,
                    dims,
                    table: vec![u32::MAX; size],
                });
                f
            }
        };
        let f = &mut self.fams[fam as usize];
        let ix = f.flat(&coords);
        f.table[ix] = id;
        if record {
            self.touch.push((id, fam, coords));
        }
        Ok(true)
    }
}

/// `FF_ORBIT_DEBUG=1` narration of why detection bailed (probe eyes only —
/// the planner itself never prints).
fn odbg(msg: impl FnOnce() -> String) {
    if std::env::var("FF_ORBIT_DEBUG").is_ok() {
        eprintln!("orbit: {}", msg());
    }
}

/// `(head with NOT folded in, args)` from a grounded display string.
fn parse(disp: &str) -> (String, Vec<String>) {
    let inner = disp
        .trim()
        .trim_start_matches("(NOT ")
        .trim_start_matches('(')
        .trim_end_matches(')');
    let mut it = inner.split_whitespace();
    let head = format!(
        "{}{}",
        if disp.trim_start().starts_with("(NOT ") {
            "NOT "
        } else {
            ""
        },
        it.next().unwrap_or("")
    );
    (head, it.map(|s| s.to_string()).collect())
}

/// Per-op certified-constant cost on `cf`, `None` where no constant can
/// be certified — the same evidence bar as optimal mode's `op_costs`
/// (sum of `Increase` effects whose expression reads only never-written,
/// init-defined fluents; a conditional effect on `cf` is state-dependent
/// by construction). Kept local: the L2 gate needs VALUES to compare,
/// not the mode's reject wording, and only family ops are ever read.
fn const_op_costs(task: &PackedTask, cf: usize) -> Vec<Option<f64>> {
    let mut written = vec![false; task.fv0.len()];
    for oi in 0..task.n_ops {
        for ne in task.num_eff.slice(oi) {
            written[ne.target as usize] = true;
        }
        for ce in task.cond_effs(oi) {
            for ne in &ce.num {
                written[ne.target as usize] = true;
            }
        }
    }
    let mut reads = Vec::new();
    (0..task.n_ops)
        .map(|oi| {
            if task
                .cond_effs(oi)
                .any(|ce| ce.num.iter().any(|ne| ne.target as usize == cf))
            {
                return None;
            }
            let mut total = 0.0f64;
            for ne in task.num_eff.slice(oi) {
                if ne.target as usize != cf {
                    continue;
                }
                if ne.op != AssignOp::Increase {
                    return None;
                }
                reads.clear();
                ne.value.collect_fluents(&mut reads);
                if !reads
                    .iter()
                    .all(|&f| !written[f as usize] && task.fdef0[f as usize])
                {
                    return None;
                }
                total += ne.value.eval(&task.fv0, &task.fdef0)?;
            }
            Some(total)
        })
        .collect()
}

/// The goal-isomorphism designation tables (0.23 Phase 4 probe 1): which
/// goal atoms the goal-blind orbits must SERVE, as family cells. Present
/// only on maps detected through an iso entry with at least one
/// designated goal atom; its mere presence obligates the consumer to the
/// relaxed goal test + witness plan remap, which is why strict entries
/// never produce it.
#[derive(Clone)]
pub struct IsoGoal {
    /// One designated goal atom per entry: (fact-family index, member
    /// coords). Every designated (orbit, member) coordinate is distinct
    /// across entries (detection bails otherwise), so a witness σ is a
    /// consistent permutation by construction.
    desig: Vec<(u32, Vec<u16>)>,
    /// The σ-fixed goal part, tested concretely: facts no orbit touches,
    /// plus whole-diagonal goal facts (σ permutes the diagonal SET onto
    /// itself, so concrete all-true is σ-image truth verbatim).
    untouched: Vec<u32>,
    /// Snapshot of `task.goal_pos`: the relaxed test arms ONLY on this
    /// exact goal (a subgoal solve must never inherit it).
    goal: Vec<u32>,
}

#[derive(Clone)]
pub struct OrbitMap {
    pub orbits: Vec<Orbit>,
    /// `Some` = this map was detected under the goal-isomorphism arm and
    /// carries designations; consumers must goal-test through
    /// [`Self::iso_goal_witness`] and remap emitted plans.
    pub iso: Option<IsoGoal>,
    /// op id -> (orbit, member, template) for per-member agenda signatures.
    pub op_owner: FxHashMap<usize, (usize, usize, usize)>,
    /// Per orbit: some goal fact's family touches it (the whole-diagonal
    /// invariance is verified below, so permuting these members is sound
    /// for the WHOLE goal — but not for a subgoal SUBSET, which is what
    /// [`Self::goal_free_view`] freezes them for).
    pub goal_bound: Vec<bool>,
    /// Per orbit: σ pinned to identity (the L5 passdown view). Frozen
    /// orbits keep their families in place — cross-orbit tables still
    /// rewrite correctly — but never permute.
    frozen: Vec<bool>,
    fact_fams: Vec<Family>,
    fact_touch: Vec<(u32, u32, Vec<u16>)>,
    op_fams: Vec<Family>,
    op_touch: FxHashMap<usize, (u32, Vec<u16>)>,
    flu_fams: Vec<Family>,
    flu_touch: Vec<(u32, u32, Vec<u16>)>,
}

/// Detect orbits on the lifted problem, then materialize them against the
/// grounded task. `None` = no usable symmetry (or `FF_NO_ORBIT=1`).
/// This entry keeps the TEMPORAL consumer's 0.21 strictness — a
/// non-total-time metric bails; the classical L2 cost carve-out lives
/// only behind [`detect_classical`], so temporal behavior is
/// byte-identical this cycle by construction. Under `FF_ORBIT_ISO=1`
/// (0.23 Phase 4 probe 1) the goal-isomorphism arm arms — safe here
/// because the temporal search is wired for the relaxed goal test.
pub fn detect(domain: &Domain, problem: &Problem, task: &PackedTask) -> Option<OrbitMap> {
    detect_gated(domain, problem, task, false, iso_armed())
}

/// The `FF_ORBIT_ISO` opt-in (0.23 Phase 4 probe 1): default OFF, and
/// with it off every detection entry is byte-identical to 0.22.
fn iso_armed() -> bool {
    std::env::var("FF_ORBIT_ISO").is_ok()
}

/// The goal-isomorphism arm, UNCONDITIONALLY armed — the fixture/probe
/// entry, so tests exercise the arm without touching process-global env.
/// Production call sites go through [`detect`] / [`detect_classical_iso`],
/// which arm it only under `FF_ORBIT_ISO=1`.
pub fn detect_iso(domain: &Domain, problem: &Problem, task: &PackedTask) -> Option<OrbitMap> {
    detect_gated(domain, problem, task, true, true)
}

fn detect_gated(
    domain: &Domain,
    problem: &Problem,
    task: &PackedTask,
    allow_cost_metric: bool,
    iso: bool,
) -> Option<OrbitMap> {
    if std::env::var("FF_NO_ORBIT").is_ok() {
        return None;
    }
    // Anything that could distinguish members OUTSIDE the grounded
    // fact/op/fluent spaces bails wholesale: scheduled exogenous events,
    // axioms expanded at ground time, trajectory constraints, and metrics
    // beyond total-time (an asymmetric metric could make merged states
    // quality-distinct).
    if !problem.til.is_empty()
        || !domain.derived.is_empty()
        || !domain.constraints.is_empty()
        || !problem.constraints.is_empty()
    {
        odbg(|| "TILs / derived rules / constraints present".into());
        return None;
    }
    // A non-total-time metric could make merged states quality-distinct.
    // The 0.22 Phase 6 L2 carve-out: the classical single-fluent minimize
    // shape (IPC `:action-costs`) is admitted PROVISIONALLY — the fluent
    // is recorded here and the grounded cost-uniformity gate below must
    // then certify every op family's costs symmetric, or detection bails.
    let mut cost_cf: Option<usize> = None;
    if let Some((_, e)) = &problem.metric {
        fn only_total_time(e: &Expr) -> bool {
            match e {
                Expr::Num(_) => true,
                Expr::Fluent(f, args) => args.is_empty() && f.eq_ignore_ascii_case("total-time"),
                Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => {
                    only_total_time(a) && only_total_time(b)
                }
                Expr::Neg(a) => only_total_time(a),
            }
        }
        if !only_total_time(e) {
            match crate::costs::metric_fluent(problem)
                .filter(|_| allow_cost_metric)
                .and_then(|d| task.fluent_id(&d))
            {
                Some(cf) => cost_cf = Some(cf),
                None => {
                    odbg(|| "metric reads more than total-time".into());
                    return None;
                }
            }
        }
    }

    // ---- 1. lifted-level candidates ------------------------------------
    // Object type map (problem objects + domain constants).
    let mut ty: FxHashMap<&str, &str> = FxHashMap::default();
    for (o, t) in problem.objects.iter().chain(domain.constants.iter()) {
        ty.insert(o.as_str(), t.as_str());
    }
    // Objects an action schema names literally can never be relabeled.
    let mut named: BTreeSet<String> = BTreeSet::new();
    fn note(t: &Term, named: &mut BTreeSet<String>) {
        if let Term::Const(c) = t {
            named.insert(c.to_ascii_uppercase());
        }
    }
    fn walk_f(f: &Formula, named: &mut BTreeSet<String>) {
        match f {
            Formula::Atom(_, args) => args.iter().for_each(|t| note(t, named)),
            Formula::Eq(a, b) => {
                note(a, named);
                note(b, named);
            }
            Formula::And(fs) | Formula::Or(fs) => fs.iter().for_each(|g| walk_f(g, named)),
            Formula::Not(g) | Formula::Pref(_, g) => walk_f(g, named),
            Formula::Exists(_, g) | Formula::Forall(_, g) => walk_f(g, named),
            Formula::Comp(..) | Formula::True | Formula::False => {}
        }
    }
    fn walk_e(e: &crate::types::Effect, named: &mut BTreeSet<String>) {
        use crate::types::Effect as E;
        match e {
            E::Add(_, args) | E::Del(_, args) | E::Num(_, _, args, _) => {
                args.iter().for_each(|t| note(t, named))
            }
            E::And(es) => es.iter().for_each(|x| walk_e(x, named)),
            E::When(f, x) => {
                walk_f(f, named);
                walk_e(x, named);
            }
            E::Forall(_, x) => walk_e(x, named),
        }
    }
    for a in &domain.actions {
        walk_f(&a.precond, &mut named);
        walk_e(&a.effect, &mut named);
    }

    // Init profile per object, ONE pass over init: multiset of (pred,
    // position, other-args-with-self-abstracted) over init atoms and
    // fluents. Statics included — an automorphism must fix them too.
    let mut prof: FxHashMap<String, BTreeMap<String, usize>> = FxHashMap::default();
    for (pred, args) in &problem.init_atoms {
        for (i, a) in args.iter().enumerate() {
            let others: Vec<String> = args
                .iter()
                .enumerate()
                .map(|(j, x)| {
                    if j == i {
                        "<SELF>".into()
                    } else {
                        x.to_ascii_uppercase()
                    }
                })
                .collect();
            *prof
                .entry(a.to_ascii_uppercase())
                .or_default()
                .entry(format!("{pred} {}", others.join(" ")))
                .or_default() += 1;
        }
    }
    for ((f, args), v) in &problem.init_fluents {
        for (i, a) in args.iter().enumerate() {
            let others: Vec<String> = args
                .iter()
                .enumerate()
                .map(|(j, x)| {
                    if j == i {
                        "<SELF>".into()
                    } else {
                        x.to_ascii_uppercase()
                    }
                })
                .collect();
            *prof
                .entry(a.to_ascii_uppercase())
                .or_default()
                .entry(format!("={f} {} {v}", others.join(" ")))
                .or_default() += 1;
        }
    }
    let profile = |o: &str| -> String { format!("{:?}", prof.get(&o.to_ascii_uppercase())) };

    // Conjunctive goal atoms only — any other goal shape bails (a numeric
    // or ADL goal could distinguish members in ways this pass can't see).
    fn collect_goal(f: &Formula, out: &mut Option<Vec<(Sym, Vec<String>)>>) {
        match f {
            Formula::And(fs) => fs.iter().for_each(|g| collect_goal(g, out)),
            Formula::Atom(p, args) => {
                let mut a = Vec::new();
                for t in args {
                    match t {
                        Term::Const(c) => a.push(c.to_ascii_uppercase()),
                        Term::Var(_) => {
                            *out = None;
                            return;
                        }
                    }
                }
                if let Some(v) = out.as_mut() {
                    v.push((p.clone(), a));
                }
            }
            Formula::True => {}
            _ => *out = None,
        }
    }
    let mut collected = Some(Vec::new());
    collect_goal(&problem.goal, &mut collected);
    if collected.is_none() {
        odbg(|| "goal is not a conjunction of ground atoms".into());
    }
    let goal_atoms: Vec<(Sym, Vec<String>)> = collected?;
    let mut goal_count: FxHashMap<&str, usize> = FxHashMap::default();
    for (_, args) in &goal_atoms {
        for a in args {
            *goal_count.entry(a.as_str()).or_default() += 1;
        }
    }

    // Member units. Singletons: objects in NO goal atom. Pairs: the two
    // objects of a binary goal atom, each appearing in exactly that one
    // goal atom. Unit key groups interchangeable candidates.
    #[derive(Clone)]
    struct Unit {
        objs: Vec<String>, // uppercase
        key: String,
    }
    let mut units: Vec<Unit> = Vec::new();
    let mut in_unit: BTreeSet<String> = BTreeSet::new();
    // The goal-isomorphism arm forms units GOAL-BLIND: every typed,
    // non-action-named object is a 1-object unit keyed by type + init
    // profile alone — the goal never splits an orbit; which goal atom a
    // member serves becomes a DESIGNATION below instead of a grouping
    // constraint (TMS: all same-type pieces join ONE orbit whether their
    // goal partner is a type-2, a type-3, or nobody).
    if iso {
        for (o, t) in problem.objects.iter() {
            let up = o.to_ascii_uppercase();
            if named.contains(&up) || in_unit.contains(&up) {
                continue;
            }
            units.push(Unit {
                key: format!("ISO {t} {}", profile(o)),
                objs: vec![up.clone()],
            });
            in_unit.insert(up);
        }
    }
    if !iso {
        for (pred, args) in &goal_atoms {
            if args.len() == 2
                && args[0] != args[1]
                && args.iter().all(|a| {
                    goal_count.get(a.as_str()) == Some(&1)
                        && !named.contains(a)
                        && ty.contains_key(a.as_str())
                })
            {
                let sig = format!(
                    "PAIR {pred} {} {} {} {}",
                    ty[args[0].as_str()],
                    ty[args[1].as_str()],
                    profile(&args[0]),
                    profile(&args[1])
                );
                units.push(Unit {
                    objs: args.clone(),
                    key: sig,
                });
                in_unit.extend(args.iter().cloned());
            }
        }
        // Unary-goal SOLO units (0.22 Phase 6 L4): an object whose ONLY goal
        // appearance is one UNARY atom (cave-diving's four identical divers,
        // child-snack's children). The key carries the goal predicate, so
        // members group only with same-goal-shape peers; the grounded
        // goal-invariance check below still verifies every member's diagonal
        // goal fact is present before the orbit is trusted.
        for (pred, args) in &goal_atoms {
            if let [a] = args.as_slice() {
                if goal_count.get(a.as_str()) == Some(&1)
                    && !in_unit.contains(a)
                    && !named.contains(a)
                    && ty.contains_key(a.as_str())
                {
                    units.push(Unit {
                        key: format!("SOLO1 {pred} {} {}", ty[a.as_str()], profile(a)),
                        objs: vec![a.clone()],
                    });
                    in_unit.insert(a.clone());
                }
            }
        }
        for (o, t) in problem.objects.iter() {
            let up = o.to_ascii_uppercase();
            if goal_count.contains_key(up.as_str()) || in_unit.contains(&up) || named.contains(&up)
            {
                continue;
            }
            units.push(Unit {
                key: format!("SOLO {t} {}", profile(o)),
                objs: vec![up.clone()],
            });
            in_unit.insert(up);
        }
    }

    // Group units into candidate orbits (same key, ≥2 members). This runs
    // BEFORE any grounded-task scan so the common no-symmetry case exits
    // on lifted work alone (elevator grounds ~10^6 displays; parsing them
    // to learn "no candidates anyway" cost more than the answer).
    let group = |units: &[Unit]| -> Vec<Vec<usize>> {
        let mut groups: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (i, u) in units.iter().enumerate() {
            groups.entry(u.key.as_str()).or_default().push(i);
        }
        groups.into_values().filter(|v| v.len() >= 2).collect()
    };
    if group(&units).is_empty() {
        odbg(|| format!("no candidate groups ({} units, all size 1)", units.len()));
        return None;
    }

    // Objects that can matter to a visited key: named by some op, some
    // DYNAMIC fact (added/deleted by an op, conditional effects included),
    // or some relevant fluent. Anything else (sokoban's wall squares,
    // whose only grounded trace is a static IS-NONGOAL) can never vary a
    // member signature — drop its unit before it mints a do-nothing orbit.
    // Static facts also skip the per-node rewrite: with equal init
    // profiles, σ maps a constant-true bit to a constant-true bit.
    let mut dynamic = vec![false; task.n_facts];
    for oi in 0..task.n_ops {
        for &f in task.add.slice(oi) {
            dynamic[f as usize] = true;
        }
        for &f in task.del.slice(oi) {
            dynamic[f as usize] = true;
        }
        for ce in task.cond_effs(oi) {
            for &f in ce.add.iter().chain(ce.del.iter()) {
                dynamic[f as usize] = true;
            }
        }
    }
    // No op-display scan: an object whose signature could ever vary
    // appears in some dynamic fact (a durative op's RUNNING token names
    // every parameter) or relevant fluent; op-args-only objects are
    // signature-empty dead weight. Elevator grounds ~10^6 op displays —
    // skipping them keeps a fruitless detect under 100ms.
    let mut active: BTreeSet<String> = BTreeSet::new();
    for (id, disp) in task.fact_names.iter().enumerate() {
        if dynamic[id] {
            active.extend(parse(disp).1);
        }
    }
    for &fid in task.rel_fluents.iter() {
        active.extend(parse(&task.fluent_names[fid as usize]).1);
    }
    units.retain(|u| u.objs.iter().all(|o| active.contains(o)));

    let candidate_orbits: Vec<Vec<&Unit>> = group(&units)
        .into_iter()
        .map(|v| v.into_iter().map(|i| &units[i]).collect())
        .collect();
    if candidate_orbits.is_empty() || candidate_orbits.len() > u16::MAX as usize {
        odbg(|| "no candidate groups after the active-object filter".into());
        return None;
    }
    odbg(|| {
        let sizes: Vec<usize> = candidate_orbits.iter().map(|m| m.len()).collect();
        format!("candidate orbits {sizes:?}")
    });

    // ---- 2. materialize against the grounded task ----------------------
    // object -> (orbit, member, obj-within-member).
    let mut owner: FxHashMap<String, (u16, u16, u8)> = FxHashMap::default();
    let k: Vec<u32> = candidate_orbits.iter().map(|m| m.len() as u32).collect();
    for (oi, members) in candidate_orbits.iter().enumerate() {
        if members.len() > u16::MAX as usize {
            return None;
        }
        for (mi, u) in members.iter().enumerate() {
            for (xi, o) in u.objs.iter().enumerate() {
                owner.insert(o.clone(), (oi as u16, mi as u16, xi as u8));
            }
        }
    }

    let mut cells = 0usize;
    let mut facts = FamSet::new();
    for (id, disp) in task.fact_names.iter().enumerate() {
        facts
            .add(disp, id as u32, &owner, &k, &mut cells, dynamic[id])
            .ok()?;
    }
    let mut ops = FamSet::new();
    for (id, disp) in task.op_display.iter().enumerate() {
        ops.add(disp, id as u32, &owner, &k, &mut cells, true)
            .ok()?;
    }
    // Fluents: only the RELEVANT ones exist for the visited key; a
    // relevant fluent whose image is missing or irrelevant is a closure
    // hole and bails below.
    let mut flu = FamSet::new();
    for &fid in task.rel_fluents.iter() {
        flu.add(
            &task.fluent_names[fid as usize],
            fid,
            &owner,
            &k,
            &mut cells,
            true,
        )
        .ok()?;
    }
    let check = |fs: &FamSet, what: &str, name: &dyn Fn(u32) -> String| -> bool {
        for (fi, fam) in fs.fams.iter().enumerate() {
            if !fam.closed() {
                odbg(|| {
                    let rep = fs
                        .touch
                        .iter()
                        .find(|(_, f, _)| *f as usize == fi)
                        .map(|(id, _, _)| name(*id))
                        .unwrap_or_default();
                    format!("{what} family not closed, e.g. {rep}")
                });
                return false;
            }
        }
        true
    };
    if !check(&facts, "fact", &|id| task.fact_names[id as usize].clone())
        || !check(&ops, "op", &|id| task.op_display[id as usize].clone())
        || !check(&flu, "fluent", &|id| task.fluent_names[id as usize].clone())
    {
        return None;
    }
    // The metric/cost-uniformity gate (0.22 Phase 6 L2): with a minimized
    // cost fluent admitted above, every op family must carry equal
    // certified-constant costs within each equality class — merging
    // states whose plans differ in cost would certify wrong optima and
    // silently degrade B&B bounds. Ops OUTSIDE every family are fixed by
    // σ and never need checking.
    if let Some(cf) = cost_cf {
        let op_cost = const_op_costs(task, cf);
        if !ops.fams.iter().all(|fam| fam.cost_uniform(&op_cost)) {
            odbg(|| "op family cost not a uniform certified constant".into());
            return None;
        }
    }

    // Per-member signature templates: families whose axes all sit in ONE
    // orbit contribute their diagonal (all-coordinates-equal) cells, one
    // per member, aligned by family order. Closure makes the diagonal
    // uniformly present or absent.
    let n_orbits = candidate_orbits.len();
    let mut orb_facts: Vec<Vec<Vec<u32>>> = (0..n_orbits)
        .map(|o| vec![Vec::new(); k[o] as usize])
        .collect();
    let mut orb_slots: Vec<Vec<Vec<usize>>> = (0..n_orbits)
        .map(|o| vec![Vec::new(); k[o] as usize])
        .collect();
    let mut orb_ops: Vec<Vec<Vec<usize>>> = (0..n_orbits)
        .map(|o| vec![Vec::new(); k[o] as usize])
        .collect();
    let mut op_owner: FxHashMap<usize, (usize, usize, usize)> = FxHashMap::default();
    let diagonal = |fam: &Family, m: u16| -> u32 {
        let coords = vec![m; fam.axes.len()];
        fam.table[fam.flat(&coords)]
    };
    let single_orbit = |fam: &Family| -> Option<usize> {
        let o = *fam.axes.first()?;
        fam.axes.iter().all(|&a| a == o).then_some(o as usize)
    };
    for fam in &facts.fams {
        if let Some(o) = single_orbit(fam) {
            // Static diagonals stay out of the signature: their bits are
            // init-constant, so they can never distinguish members.
            if diagonal(fam, 0) != u32::MAX
                && (0..k[o]).all(|m| dynamic[diagonal(fam, m as u16) as usize])
            {
                for m in 0..k[o] {
                    orb_facts[o][m as usize].push(diagonal(fam, m as u16));
                }
            }
        }
    }
    let slot_of: FxHashMap<u32, usize> = task
        .rel_fluents
        .iter()
        .enumerate()
        .map(|(s, &f)| (f, s))
        .collect();
    for fam in &flu.fams {
        if let Some(o) = single_orbit(fam) {
            if diagonal(fam, 0) != u32::MAX {
                for m in 0..k[o] {
                    orb_slots[o][m as usize].push(slot_of[&diagonal(fam, m as u16)]);
                }
            }
        }
    }
    for fam in &ops.fams {
        if let Some(o) = single_orbit(fam) {
            if diagonal(fam, 0) != u32::MAX {
                for m in 0..k[o] {
                    let op = diagonal(fam, m as u16) as usize;
                    let tj = orb_ops[o][m as usize].len();
                    orb_ops[o][m as usize].push(op);
                    op_owner.insert(op, (o, m as usize, tj));
                }
            }
        }
    }
    let orbits: Vec<Orbit> = (0..n_orbits)
        .map(|o| Orbit {
            facts: std::mem::take(&mut orb_facts[o]),
            fluent_slots: std::mem::take(&mut orb_slots[o]),
            ops: std::mem::take(&mut orb_ops[o]),
        })
        .collect();
    // Nothing state-bearing to permute anywhere -> no reduction possible.
    if orbits
        .iter()
        .all(|o| o.facts[0].is_empty() && o.fluent_slots[0].is_empty() && o.ops[0].is_empty())
    {
        odbg(|| "no orbit has any state-bearing signature".into());
        return None;
    }

    // Goal invariance: every goal fact must be untouched, or a
    // single-orbit diagonal fact whose WHOLE diagonal is in the goal (the
    // goal set is then fixed by every σ). Numeric goals reading a touched
    // fluent bail. Orbits touched by goal facts are recorded as
    // GOAL-BOUND: sound for the whole goal, frozen by
    // [`OrbitMap::goal_free_view`] for subgoal-subset searches.
    //
    // The ISO arm replaces the invariance OBLIGATION with a designation
    // TABLE: each goal fact touching an orbit is recorded as (family,
    // coords) for the relaxed goal test, and every designated (orbit,
    // member) coordinate must be distinct across the whole goal — a
    // member designated twice would need one server to satisfy two atoms
    // at once, a consistency constraint this probe arm does not carry
    // (recorded narrowing: `(on a b) (on b c)` chains bail).
    let mut goal_bound = vec![false; n_orbits];
    let goal_set: std::collections::HashSet<u32> = task.goal_pos.iter().copied().collect();
    let fact_of: FxHashMap<u32, (u32, &Vec<u16>)> = facts
        .touch
        .iter()
        .map(|(id, fam, c)| (*id, (*fam, c)))
        .collect();
    let mut iso_desig: Vec<(u32, Vec<u16>)> = Vec::new();
    let mut iso_untouched: Vec<u32> = Vec::new();
    if iso {
        let mut designated: FxHashSet<(u16, u16)> = FxHashSet::default();
        for &g in task.goal_pos.iter() {
            let Some(&(famix, coords)) = fact_of.get(&g) else {
                iso_untouched.push(g);
                continue;
            };
            let fam = &facts.fams[famix as usize];
            // The σ-INVARIANT shape (whole diagonal in the goal — the
            // strict arm's own soundness case) joins the concrete-check
            // bucket instead of the designation table: every σ fixes the
            // SET, so "all diagonal bits true" is σ-image truth verbatim,
            // and keeping it out of `desig` keeps h intact and the whole
            // arm INERT on shapes where iso ≡ strict (child-snack's
            // uniform `served` goals — the classical read's control).
            if let (Some(o), true) = (single_orbit(fam), coords.iter().all(|&c| c == coords[0])) {
                if (0..k[o]).all(|m| goal_set.contains(&diagonal(fam, m as u16))) {
                    iso_untouched.push(g);
                    goal_bound[o] = true;
                    continue;
                }
            }
            // Distinctness across atoms; repeats WITHIN one atom are the
            // same coordinate and stay consistent by construction.
            let mut here: Vec<(u16, u16)> = fam
                .axes
                .iter()
                .copied()
                .zip(coords.iter().copied())
                .collect();
            here.sort_unstable();
            here.dedup();
            for oc in here {
                if !designated.insert(oc) {
                    odbg(|| {
                        format!(
                            "iso: member designated by two goal atoms, e.g. {}",
                            task.fact_names[g as usize]
                        )
                    });
                    return None;
                }
            }
            for &a in fam.axes.iter() {
                goal_bound[a as usize] = true;
            }
            iso_desig.push((famix, coords.clone()));
        }
    } else {
        for &g in task.goal_pos.iter() {
            if let Some(&(famix, coords)) = fact_of.get(&g) {
                let fam = &facts.fams[famix as usize];
                let (Some(o), true) = (single_orbit(fam), coords.iter().all(|&c| c == coords[0]))
                else {
                    odbg(|| format!("cross-member goal fact {}", task.fact_names[g as usize]));
                    return None;
                };
                for m in 0..k[o] {
                    if !goal_set.contains(&diagonal(fam, m as u16)) {
                        odbg(|| {
                            format!(
                                "goal fact {} not orbit-uniform",
                                task.fact_names[g as usize]
                            )
                        });
                        return None;
                    }
                }
                goal_bound[o] = true;
            }
        }
    }
    let mut goal_fluents: Vec<u32> = Vec::new();
    for np in task.goal_num.iter() {
        np.lhs.collect_fluents(&mut goal_fluents);
        np.rhs.collect_fluents(&mut goal_fluents);
    }
    let touched_fluents: std::collections::HashSet<u32> =
        flu.touch.iter().map(|(id, _, _)| *id).collect();
    if goal_fluents.iter().any(|f| touched_fluents.contains(f)) {
        odbg(|| "numeric goal reads a touched fluent".into());
        return None;
    }

    // With no designated atom the iso arm collapses to strict semantics
    // (the whole goal is σ-fixed) — no relaxed test, no remap obligation,
    // so the map carries no IsoGoal and consumers behave exactly as 0.22.
    let iso_goal = (iso && !iso_desig.is_empty()).then(|| IsoGoal {
        desig: iso_desig,
        untouched: iso_untouched,
        goal: task.goal_pos.to_vec(),
    });
    if iso {
        odbg(|| match &iso_goal {
            Some(ig) => format!(
                "iso: {} designations, {} sigma-fixed goal facts",
                ig.desig.len(),
                ig.untouched.len()
            ),
            None => "iso: no designations (arm inert, strict semantics)".into(),
        });
    }

    Some(OrbitMap {
        orbits,
        iso: iso_goal,
        op_owner,
        frozen: vec![false; goal_bound.len()],
        goal_bound,
        fact_fams: facts.fams,
        fact_touch: facts.touch,
        op_fams: ops.fams,
        op_touch: ops
            .touch
            .into_iter()
            .map(|(id, fam, c)| (id as usize, (fam, c)))
            .collect(),
        flu_fams: flu.fams,
        flu_touch: flu.touch,
    })
}

/// The classical consumer's detection entry (0.22 Phase 6):
/// [`detect`] behind the consumer-scoped hatch — `FF_NO_ORBIT_CLASSICAL=1`
/// kills ONLY the classical canonical keys/dedup while the temporal
/// consumer keeps its orbit; `FF_NO_ORBIT=1` still kills both. STRICT by
/// construction, `FF_ORBIT_ISO` notwithstanding: its maps flow to the
/// satisficing dedup, the partition passdown, and the B&B sweep — none
/// of which carry the relaxed goal test an iso map obligates.
pub fn detect_classical(domain: &Domain, problem: &Problem, task: &PackedTask) -> Option<OrbitMap> {
    if std::env::var("FF_NO_ORBIT_CLASSICAL").is_ok() {
        return None;
    }
    detect_gated(domain, problem, task, true, false)
}

/// [`detect_classical`] with the goal-isomorphism arm armed under
/// `FF_ORBIT_ISO=1` (0.23 Phase 4 probe 1) — the entry for the ONE
/// classical consumer wired for the relaxed goal test + witness remap:
/// optimal A* (`api::solve_optimal`). Flag off ⇒ byte-identical to
/// [`detect_classical`].
pub fn detect_classical_iso(
    domain: &Domain,
    problem: &Problem,
    task: &PackedTask,
) -> Option<OrbitMap> {
    if std::env::var("FF_NO_ORBIT_CLASSICAL").is_ok() {
        return None;
    }
    detect_gated(domain, problem, task, true, iso_armed())
}

impl OrbitMap {
    /// The canonical visited key under member relabeling: per orbit, sort
    /// members by their (fact-bits, fluent-values, pending-agenda)
    /// signature to pick σ, then rewrite the ENTIRE key — per-member and
    /// cross-member facts, relevant fluents, and pending-end agenda ops —
    /// through the family tables. Returns (canonical StateKey, canonical
    /// agenda). Sound for ANY σ; the signature sort just makes π-related
    /// states usually agree.
    pub fn canonical_key(
        &self,
        task: &PackedTask,
        state: &State,
        agenda: &[(i64, usize)],
    ) -> (crate::packed::StateKey, Vec<(i64, usize)>) {
        let (sigma, identity) = self.member_sigma(task, state, agenda);
        let mut ag: Vec<(i64, usize)> = agenda.to_vec();
        if identity {
            ag.sort_unstable();
            return (task.state_key(state), ag);
        }
        let canon = self.rewrite_state(state, &sigma);
        for e in ag.iter_mut() {
            if let Some((fam, coords)) = self.op_touch.get(&e.1) {
                e.1 = self.op_fams[*fam as usize].map(coords, &sigma) as usize;
            }
        }
        ag.sort_unstable();
        (task.state_key(&canon), ag)
    }

    /// The classical canonical visited key (0.22 Phase 6): the temporal
    /// pattern with an empty agenda, plus the branch-and-bound cost
    /// fluent appended AFTER canonicalization (the σ-invariance of the
    /// cost value is exactly what the L2 gate certified; the fluent is
    /// 0-ary in every admitted metric shape, so the rewrite never moves
    /// it). Matches [`PackedTask::state_key_with_cost`] content-for-content
    /// on the identity path.
    pub fn canonical_skey(
        &self,
        task: &PackedTask,
        state: &State,
        cost_fluent: Option<usize>,
    ) -> crate::packed::StateKey {
        let (sigma, identity) = self.member_sigma(task, state, &[]);
        if identity {
            return task.state_key_with_cost(state, cost_fluent);
        }
        let canon = self.rewrite_state(state, &sigma);
        task.state_key_with_cost(&canon, cost_fluent)
    }

    /// Streaming-hash companion of [`Self::canonical_skey`] (the
    /// [`PackedTask::state_key_hash`] contract): the identity σ — the
    /// overwhelmingly common case — pays no clone at all, so the
    /// canonicalization tax on asymmetric states is per-DUPLICATE, not
    /// per-node (docs/roadmap-0.22.md Phase 6 L3).
    pub fn canonical_skey_hash(
        &self,
        task: &PackedTask,
        state: &State,
        cost_fluent: Option<usize>,
    ) -> u64 {
        let (sigma, identity) = self.member_sigma(task, state, &[]);
        if identity {
            return task.state_key_hash(state, cost_fluent);
        }
        let canon = self.rewrite_state(state, &sigma);
        task.state_key_hash(&canon, cost_fluent)
    }

    /// Does this map carry goal-isomorphism designations? `true` obligates
    /// the consumer to [`Self::iso_goal_witness`] + the emission remap.
    pub fn iso_active(&self) -> bool {
        self.iso.is_some()
    }

    /// The σ-invariant goal part (goal facts no orbit touches) — the
    /// admissible heuristic target for optimal mode under the iso arm:
    /// it is a subset of EVERY σ-image of the goal, so h against it
    /// never overestimates the distance to the nearest accepted state.
    pub fn iso_untouched_goal(&self) -> Option<&[u32]> {
        self.iso.as_ref().map(|i| i.untouched.as_slice())
    }

    /// The relaxed goal test (0.23 Phase 4 probe 1): does SOME σ in the
    /// goal-blind group map `state` onto the goal? Arms only when
    /// `goal_pos` is exactly the detection-time goal (a subgoal solve
    /// must never inherit designations), the σ-fixed goal part holds
    /// concretely (untouched facts + numeric conjuncts), and an exact
    /// designation matching exists: per designated goal atom, a
    /// TRUE-in-`state` cell of the same family and equality pattern,
    /// all chosen cells member-disjoint. Returns the WITNESS σ (per
    /// orbit, member → destination), completed bijectively on
    /// undesignated members — σ(state) ⊇ goal by construction, and the
    /// emitted plan remapped through [`Self::iso_remap_op`] serves the
    /// ORIGINAL goal (the round-trip fixture is the referee).
    ///
    /// Deterministic: candidates enumerate in family-table order and the
    /// backtracking visits designations fewest-candidates-first with a
    /// stable tiebreak, so t1 ≡ t8 holds. The matcher is EXACT within a
    /// step budget; a blown budget reads as "not a goal" — recorded
    /// probe caveat, never observed on the fixture shapes.
    pub fn iso_goal_witness(
        &self,
        task: &PackedTask,
        state: &State,
        goal_pos: &[u32],
        goal_num: &[NumPre],
    ) -> Option<Vec<Vec<u16>>> {
        let iso = self.iso.as_ref()?;
        if goal_pos != iso.goal.as_slice() || !task.goal_met_with(state, &iso.untouched, goal_num) {
            return None;
        }
        // Candidate cells per designation, then the exact backtracking.
        let mut cands: Vec<Vec<Vec<u16>>> = Vec::with_capacity(iso.desig.len());
        for (famix, dcoords) in &iso.desig {
            let fam = &self.fact_fams[*famix as usize];
            let n = fam.axes.len();
            let pattern_ok = |coords: &[u16]| -> bool {
                for d in 0..n {
                    for e in 0..d {
                        if fam.axes[e] == fam.axes[d]
                            && (coords[e] == coords[d]) != (dcoords[e] == dcoords[d])
                        {
                            return false;
                        }
                    }
                }
                true
            };
            let mut cells: Vec<Vec<u16>> = Vec::new();
            let mut coords = vec![0u16; n];
            'odo: loop {
                let id = fam.table[fam.flat(&coords)];
                if id != u32::MAX
                    && crate::bitset::test(&state.bits, id as usize)
                    && pattern_ok(&coords)
                {
                    cells.push(coords.clone());
                }
                let mut d = n;
                loop {
                    if d == 0 {
                        break 'odo;
                    }
                    d -= 1;
                    coords[d] += 1;
                    if (coords[d] as u32) < fam.dims[d] {
                        break;
                    }
                    coords[d] = 0;
                }
            }
            if cells.is_empty() {
                return None; // some designation has no server at all
            }
            cands.push(cells);
        }
        let mut order: Vec<usize> = (0..cands.len()).collect();
        order.sort_by_key(|&i| (cands[i].len(), i));
        let mut used: Vec<Vec<bool>> = self
            .orbits
            .iter()
            .map(|o| vec![false; o.facts.len()])
            .collect();
        let mut chosen: Vec<usize> = vec![usize::MAX; cands.len()];
        let mut budget = 1usize << 20;
        if !self.iso_bt(iso, &cands, &order, 0, &mut used, &mut chosen, &mut budget) {
            if budget == 0 {
                odbg(|| "iso: witness matcher budget exhausted (state read as non-goal)".into());
            }
            return None;
        }
        // Build σ: chosen servers → designated coordinates, leftovers →
        // leftover destinations ascending (any completion is sound; a
        // FIXED one keeps the emitted plan a pure function of the state).
        let mut sigma: Vec<Vec<u16>> = self
            .orbits
            .iter()
            .map(|o| vec![u16::MAX; o.facts.len()])
            .collect();
        let mut dst_taken: Vec<Vec<bool>> = self
            .orbits
            .iter()
            .map(|o| vec![false; o.facts.len()])
            .collect();
        for (di, (famix, dcoords)) in iso.desig.iter().enumerate() {
            let fam = &self.fact_fams[*famix as usize];
            let cell = &cands[di][chosen[di]];
            for (d, (&m, &c)) in cell.iter().zip(dcoords.iter()).enumerate() {
                let o = fam.axes[d] as usize;
                sigma[o][m as usize] = c;
                dst_taken[o][c as usize] = true;
            }
        }
        for (o, s) in sigma.iter_mut().enumerate() {
            let mut free = (0..s.len() as u16).filter(|&c| !dst_taken[o][c as usize]);
            for dst in s.iter_mut() {
                if *dst == u16::MAX {
                    *dst = free.next().expect("iso witness completion is bijective");
                }
            }
        }
        Some(sigma)
    }

    /// Exact designation matching, depth-first over `order` with
    /// member-disjointness pruning. `true` fills `chosen` for every
    /// designation; `budget` bounds total candidate trials.
    #[allow(clippy::too_many_arguments)]
    fn iso_bt(
        &self,
        iso: &IsoGoal,
        cands: &[Vec<Vec<u16>>],
        order: &[usize],
        pos: usize,
        used: &mut [Vec<bool>],
        chosen: &mut [usize],
        budget: &mut usize,
    ) -> bool {
        if pos == order.len() {
            return true;
        }
        let di = order[pos];
        let fam = &self.fact_fams[iso.desig[di].0 as usize];
        'cand: for (ci, cell) in cands[di].iter().enumerate() {
            if *budget == 0 {
                return false;
            }
            *budget -= 1;
            for (d, &m) in cell.iter().enumerate() {
                if used[fam.axes[d] as usize][m as usize] {
                    continue 'cand;
                }
            }
            for (d, &m) in cell.iter().enumerate() {
                used[fam.axes[d] as usize][m as usize] = true;
            }
            chosen[di] = ci;
            if self.iso_bt(iso, cands, order, pos + 1, used, chosen, budget) {
                return true;
            }
            for (d, &m) in cell.iter().enumerate() {
                used[fam.axes[d] as usize][m as usize] = false;
            }
        }
        false
    }

    /// Apply the witness to one emitted-plan op through the op family
    /// tables: the σ-image op serving the ORIGINAL goal. Ops outside
    /// every family are σ-fixed and pass through.
    pub fn iso_remap_op(&self, sigma: &[Vec<u16>], op: usize) -> usize {
        match self.op_touch.get(&op) {
            Some((fam, coords)) => self.op_fams[*fam as usize].map(coords, sigma) as usize,
            None => op,
        }
    }

    /// The L5 passdown view (0.22 Phase 6): goal-BOUND orbits freeze to
    /// identity — a partition SUBGOAL is a strict subset of the goal, so
    /// permuting members the goal distinguishes is no longer invariant —
    /// while goal-free orbits (child-snack's sandwiches) keep merging.
    /// `None` when every orbit is goal-bound: nothing left to permute,
    /// so callers skip the tax entirely.
    pub fn goal_free_view(&self) -> Option<OrbitMap> {
        if self.goal_bound.iter().all(|&b| b) {
            return None;
        }
        let mut v = self.clone();
        v.frozen = self.goal_bound.clone();
        Some(v)
    }

    /// σ per orbit (member -> destination), plus the all-identity flag.
    /// Chosen by sorting per-member signatures — fact bits (template
    /// order), fluent (defined, value) pairs, this member's pending
    /// agenda entries (time, template), src index last as tiebreak — so
    /// the canonical form is a pure function of the state and t1 ≡ t8
    /// holds. Frozen orbits ([`Self::goal_free_view`]) pin identity.
    fn member_sigma(
        &self,
        task: &PackedTask,
        state: &State,
        agenda: &[(i64, usize)],
    ) -> (Vec<Vec<u16>>, bool) {
        let mut sigma: Vec<Vec<u16>> = Vec::with_capacity(self.orbits.len());
        let mut identity = true;
        for (oi, orbit) in self.orbits.iter().enumerate() {
            let k = orbit.facts.len();
            if self.frozen[oi] {
                sigma.push((0..k as u16).collect());
                continue;
            }
            #[allow(clippy::type_complexity)]
            let mut sig: Vec<(Vec<bool>, Vec<(bool, i64)>, Vec<(i64, usize)>, usize)> =
                Vec::with_capacity(k);
            for mi in 0..k {
                let fb: Vec<bool> = orbit.facts[mi]
                    .iter()
                    .map(|&f| crate::bitset::test(&state.bits, f as usize))
                    .collect();
                let fvals: Vec<(bool, i64)> = orbit.fluent_slots[mi]
                    .iter()
                    .map(|&slot| {
                        let fid = task.rel_fluents[slot] as usize;
                        (state.fdef[fid], (state.fv[fid] * 1000.0).round() as i64)
                    })
                    .collect();
                let mut pend: Vec<(i64, usize)> = agenda
                    .iter()
                    .filter_map(|&(t, op)| match self.op_owner.get(&op) {
                        Some(&(o, m, tj)) if o == oi && m == mi => Some((t, tj)),
                        _ => None,
                    })
                    .collect();
                pend.sort_unstable();
                sig.push((fb, fvals, pend, mi));
            }
            sig.sort();
            let mut dest = vec![0u16; k];
            for (j, (_, _, _, src)) in sig.iter().enumerate() {
                dest[*src] = j as u16;
                if j != *src {
                    identity = false;
                }
            }
            sigma.push(dest);
        }
        (sigma, identity)
    }

    /// σ applied to bits/fluents. σ is a bijection on each touched id
    /// space (closure-checked at detection), so writing every image
    /// exactly once from the PRISTINE source state is a complete,
    /// alias-free rewrite.
    fn rewrite_state(&self, state: &State, sigma: &[Vec<u16>]) -> State {
        let mut bits = state.bits.clone();
        let mut fv = state.fv.clone();
        let mut fdef = state.fdef.clone();
        for (f, fam, coords) in &self.fact_touch {
            let nf = self.fact_fams[*fam as usize].map(coords, sigma) as usize;
            if crate::bitset::test(&state.bits, *f as usize) {
                crate::bitset::set(&mut bits, nf);
            } else {
                crate::bitset::clear(&mut bits, nf);
            }
        }
        for (fid, fam, coords) in &self.flu_touch {
            let nf = self.flu_fams[*fam as usize].map(coords, sigma) as usize;
            fv[nf] = state.fv[*fid as usize];
            fdef[nf] = state.fdef[*fid as usize];
        }
        State { bits, fv, fdef }
    }

    /// Stabilizer classes for GENERATION-side symmetry skipping (0.15
    /// Phase 1): per orbit, group members whose pairwise TRANSPOSITION
    /// provably fixes the whole state — every touched fact bit, fluent
    /// value/definedness, and pending-agenda entry maps to an equal one
    /// (cross-member facts included; the per-member signature alone is NOT
    /// enough — `(STRUCTURE m1 x)` true with `(STRUCTURE m2 x)` false
    /// distinguishes m1 from m2 even when their own facts agree). Two ops
    /// that are the same template on same-class members produce
    /// π-equivalent successors, so the expansion generates only the first:
    /// the duplicate never exists instead of being deduped after the fact.
    /// Swap-fixes is transitive on a chain of pairwise checks
    /// ((a c) = (a b)(b c)(a b)), so greedy class assignment is exact.
    pub fn stabilizer_classes(&self, state: &State, agenda: &[(f64, usize)]) -> Vec<Vec<u16>> {
        let mut out = Vec::with_capacity(self.orbits.len());
        for (oi, orbit) in self.orbits.iter().enumerate() {
            let k = orbit.facts.len();
            let mut class: Vec<u16> = (0..k as u16).collect();
            for a in 0..k {
                if class[a] != a as u16 {
                    continue;
                }
                #[allow(clippy::needless_range_loop)]
                for b in (a + 1)..k {
                    if class[b] == b as u16
                        && self.swap_fixes(oi as u16, a as u16, b as u16, state, agenda)
                    {
                        class[b] = a as u16;
                    }
                }
            }
            out.push(class);
        }
        out
    }

    /// Class-canonical GENERATION key for `op` under `classes` (from
    /// [`Self::stabilizer_classes`]): two ops with equal keys are images of
    /// one another under a state-fixing σ — same family, per-coordinate
    /// stabilizer-class representatives, and the same equality pattern
    /// among same-orbit coordinates (so `(REL m1 m2)` never conflates with
    /// a hypothetical `(REL m3 m3)`). Any within-class permutation is a
    /// product of class transpositions, each of which fixes the state, so
    /// the composed σ fixes it too. `None` = op touches no orbit (always
    /// generate).
    pub fn gen_key(&self, op: usize, classes: &[Vec<u16>]) -> Option<(u32, Vec<u16>)> {
        let (fam_ix, coords) = self.op_touch.get(&op)?;
        let fam = &self.op_fams[*fam_ix as usize];
        let mut key = Vec::with_capacity(coords.len() * 2);
        for (d, &c) in coords.iter().enumerate() {
            key.push(classes[fam.axes[d] as usize][c as usize]);
            let mut pat = d as u16;
            #[allow(clippy::needless_range_loop)]
            for e in 0..d {
                if fam.axes[e] == fam.axes[d] && coords[e] == c {
                    pat = e as u16;
                    break;
                }
            }
            key.push(pat);
        }
        Some((*fam_ix, key))
    }

    /// Does the transposition (a b) within orbit `oi` fix `state` + `agenda`?
    fn swap_fixes(&self, oi: u16, a: u16, b: u16, state: &State, agenda: &[(f64, usize)]) -> bool {
        let sig = |orb: u16, c: u16| -> u16 {
            if orb == oi {
                if c == a {
                    b
                } else if c == b {
                    a
                } else {
                    c
                }
            } else {
                c
            }
        };
        let touches = |fam_axes: &[u16], coords: &[u16]| -> bool {
            fam_axes
                .iter()
                .zip(coords.iter())
                .any(|(&ax, &c)| ax == oi && (c == a || c == b))
        };
        for (f, fam, coords) in &self.fact_touch {
            let fam = &self.fact_fams[*fam as usize];
            if touches(&fam.axes, coords) {
                let nf = fam.map_with(coords, sig);
                if crate::bitset::test(&state.bits, *f as usize)
                    != crate::bitset::test(&state.bits, nf as usize)
                {
                    return false;
                }
            }
        }
        for (fid, fam, coords) in &self.flu_touch {
            let fam = &self.flu_fams[*fam as usize];
            if touches(&fam.axes, coords) {
                let nf = fam.map_with(coords, sig) as usize;
                let (i, j) = (*fid as usize, nf);
                if state.fdef[i] != state.fdef[j]
                    || (state.fdef[i] && (state.fv[i] - state.fv[j]).abs() > 1e-9)
                {
                    return false;
                }
            }
        }
        // Agenda: every touched pending entry's image must be pending at the
        // SAME time. σ is an involution, so one direction suffices.
        for &(t, op) in agenda {
            if let Some((fam, coords)) = self.op_touch.get(&op) {
                let fam = &self.op_fams[*fam as usize];
                if touches(&fam.axes, coords) {
                    let nop = fam.map_with(coords, sig) as usize;
                    if nop != op && !agenda.iter().any(|&(t2, o2)| o2 == nop && t2 == t) {
                        return false;
                    }
                }
            }
        }
        true
    }
}
