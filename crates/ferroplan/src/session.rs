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
//! **Scope.** Classical / numeric / ADL domains, and — since 0.12 — TEMPORAL
//! (durative-action) domains: the snap compilation grounds once and every
//! think runs the bounded decision-epoch ladder from the current AT-REST
//! state (no running intervals between thinks; `set_fact` fences the
//! compiler's `RUNNING-*` tokens, and a game models an in-flight action by
//! mirroring its end effects when it completes). Timed initial literals are
//! rejected — a TIL pins the absolute clock and session thinks are
//! clock-relative. PDDL3 preference problems stay rejected (the metric
//! optimizer compiles the problem per solve). The goal is retargetable
//! (0.13 Phase 1): [`Session::set_goal`] swaps in any ground conjunction
//! over the already-interned fact space without regrounding — one world,
//! changing desires. And a session is forkable (0.13 Phase 2):
//! [`Session::fork`] clones a mind over the SAME grounded world — the
//! grounded payload shares behind `Arc`, so a bazaar of N NPCs costs one
//! grounding plus N small state views, each free to diverge.
//!
//! **Why static facts are rejected.** Grounding enumerates operator parameters
//! restricted by *static* predicates read from `:init` — a static fact flipped
//! after grounding could require operators that were never enumerated. Rather than
//! hand back silently-wrong plans, [`Session::set_fact`] only accepts facts some
//! operator can add or delete (the world's *dynamic* facts) and errors on the rest.
//! Fluent values are all runtime-read, so any grounded fluent may be set.

use crate::api::{stats, steps_of, timed_steps, Mode, Options, Plan, Search, Solution};
use crate::ground::ground_task;
use crate::hash::FxHashMap;
use crate::packed::PackedTask;
use crate::search;
use crate::types::{Expr, Formula, NExpr, NumPre, Term};
use std::sync::Arc;

/// The predicate head of a fact display (`(AT V1 HUT)` -> `AT`), looking
/// through a complementary-mirror wrapper (`(NOT (RUNNING-BUILD W1))` ->
/// `RUNNING-BUILD`) so the `RUNNING-*` fence cannot be dodged via a mirror.
fn atom_head(display: &str) -> &str {
    let mut s = display.trim();
    loop {
        s = s.trim_start_matches('(').trim_start();
        let head = s
            .split(|c: char| c.is_whitespace() || c == '(' || c == ')')
            .next()
            .unwrap_or("");
        if head.eq_ignore_ascii_case("NOT") && !head.is_empty() {
            s = &s[head.len()..];
            continue;
        }
        return head;
    }
}

/// A grounded, replannable world. See the module docs.
pub struct Session {
    task: PackedTask,
    threads: usize,
    weight_g: f64,
    weight_h: f64,
    max_evaluated: Option<usize>,
    ehc_first: bool,
    /// Display name (uppercase, e.g. `(AT V1 FIELD)`) -> fact id. Shared by
    /// every fork (0.13 Phase 2) — immutable after construction, like every
    /// `Arc`'d field below: N minds, one copy.
    fact_ids: Arc<FxHashMap<String, u32>>,
    /// Per fact id: does any operator add or delete it? (Static facts are baked
    /// into the grounding and must not change — see the module docs.)
    dynamic: Arc<[bool]>,
    /// Display name (uppercase, e.g. `(GRAIN)`) -> fluent id.
    fluent_ids: Arc<FxHashMap<String, u32>>,
    /// Temporal session state (0.12 Phase 1): the snap compilation, kept so
    /// each think can REBUILD `build_kind`'s duration table against the
    /// CURRENT fluent values (a `set_fluent` on a fluent no op modifies must
    /// flow into parameter-dependent durations, not stay frozen at
    /// construction). `None` = classical session.
    temporal: Option<Arc<crate::temporal::TemporalCompiled>>,
    /// The demand tier, read ONCE at construction so a session's behavior is
    /// stable even if the process environment changes between thinks.
    tier: crate::features::DemandMode,
    /// Compiler-minted `RUNNING-*` token predicates (temporal only): a
    /// session's world is AT REST between thinks — no running intervals — so
    /// `set_fact` fences these exactly as it fences statics.
    running_preds: Vec<String>,
    /// Op display (`WALK V1 HUT FIELD`) -> op id, for suffix replay
    /// ([`Session::plan_still_valid`]).
    op_ids: Arc<FxHashMap<String, usize>>,
    /// Complementary-mirror pairing (0.13 Phase 1), BOTH directions: when
    /// grounding created a `(NOT (p ...))` mirror fact (negative
    /// preconditions/goals), [`Session::set_fact`] on either side keeps the
    /// other in sync — a stale mirror would silently corrupt every
    /// applicability and goal test that reads it.
    mirror: Arc<FxHashMap<u32, u32>>,
}

impl Session {
    /// Parse, compile `:derived` axioms, and ground once (temporal domains
    /// snap-compile first). Errors on parse/grounding failure, and on
    /// constraint, preference, or timed-initial-literal inputs (unsupported —
    /// see the module docs).
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
        // Temporal domains (0.12 Phase 1): snap-compile + stratified-ground
        // ONCE; every think then solves from the CURRENT at-rest state. TILs
        // are rejected — they pin the ABSOLUTE clock, and session thinks are
        // clock-relative (recorded follow-up if the game needs scheduled
        // exogenous events).
        let temporal_c = if crate::temporal::is_temporal(&domain) {
            if !problem.til.is_empty() {
                return Err("Session does not support timed initial literals \
                     (a TIL pins the absolute clock; session thinks are \
                     clock-relative); use ferroplan::solve per instance"
                    .into());
            }
            Some(crate::temporal::compile(&domain, &problem))
        } else {
            None
        };
        // Force a task even when the base goal is trivially true/false — a session's
        // world moves, so the base-init verdict says nothing about later replans.
        let task = match &temporal_c {
            Some(c) => match crate::ground::ground_fixpoint(&c.domain, &c.problem, threads) {
                crate::ground::Outcome::Task(t) => t,
                _ => return Err("grounding failed (empty type)".to_string()),
            },
            None => ground_task(&domain, &problem, threads)
                .ok_or_else(|| "grounding failed (empty type)".to_string())?,
        };

        let fact_ids: FxHashMap<String, u32> = task
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

        let running_preds = temporal_c
            .as_ref()
            .map(|c| c.snaps.iter().map(|s| s.running_pred.clone()).collect())
            .unwrap_or_default();
        let op_ids = task
            .op_display
            .iter()
            .enumerate()
            .map(|(i, d)| (d.clone(), i))
            .collect();
        // Mirror pairing: a fact displayed `(NOT (P ...))` complements the
        // fact displayed `(P ...)` (when the latter was interned at all).
        let mut mirror = FxHashMap::default();
        for (i, name) in task.fact_names.iter().enumerate() {
            if let Some(inner) = name.strip_prefix("(NOT ").and_then(|r| r.strip_suffix(')')) {
                if let Some(&base) = fact_ids.get(inner) {
                    mirror.insert(i as u32, base);
                    mirror.insert(base, i as u32);
                }
            }
        }
        Ok(Session {
            task,
            threads,
            weight_g: opts.weight_g,
            weight_h: opts.weight_h,
            max_evaluated: opts.max_evaluated,
            ehc_first: opts.search != Search::BestFirst,
            fact_ids: Arc::new(fact_ids),
            dynamic: dynamic.into(),
            fluent_ids: Arc::new(fluent_ids),
            temporal: temporal_c.map(Arc::new),
            tier: crate::features::demand_mode(),
            running_preds,
            op_ids: Arc::new(op_ids),
            mirror: Arc::new(mirror),
        })
    }

    /// Fork this mind (0.13 Phase 2): an independent `Session` over the SAME
    /// grounded world. N minds cost ONE grounding — the fork shares the
    /// grounded payload (operator columns, names, achiever indexes, the
    /// temporal compilation) behind `Arc` and copies only the small per-mind
    /// state: current facts and fluents, goal, fluent relevance.
    ///
    /// The fork starts from this session's CURRENT state and goal (not the
    /// problem's `:init`), then diverges freely: its [`Session::set_fact`] /
    /// [`Session::set_goal`] / [`Session::replan`] never touch a sibling —
    /// no shared tie-breaks, no cross-mind interference. Each fork keeps the
    /// parent's options (threads, weights, budget) and demand tier.
    pub fn fork(&self) -> Session {
        Session {
            task: self.task.clone(),
            threads: self.threads,
            weight_g: self.weight_g,
            weight_h: self.weight_h,
            max_evaluated: self.max_evaluated,
            ehc_first: self.ehc_first,
            fact_ids: Arc::clone(&self.fact_ids),
            dynamic: Arc::clone(&self.dynamic),
            fluent_ids: Arc::clone(&self.fluent_ids),
            temporal: self.temporal.clone(),
            tier: self.tier,
            running_preds: self.running_preds.clone(),
            op_ids: Arc::clone(&self.op_ids),
            mirror: Arc::clone(&self.mirror),
        }
    }

    /// The temporal think (0.12 Phase 1): rebuild the duration table against
    /// the CURRENT fluent values (so `set_fluent` on an op-unmodified fluent
    /// flows into parameter-dependent durations), then run the bounded
    /// decision-epoch ladder from the current at-rest state. The eval budget
    /// spans the WHOLE pass ladder; the memory target plumbs to the
    /// deterministic temporal node cap. No escalation beyond the ladder, no
    /// decomposer — a think is a bounded call, not a campaign.
    fn replan_temporal(
        &self,
        c: &crate::temporal::TemporalCompiled,
        budget_evals: Option<usize>,
        memory_mb: Option<usize>,
    ) -> Solution {
        let start = self.task.initial();
        if self.task.goal_met(&start) {
            return Solution {
                solved: true,
                mode: Mode::Temporal,
                plan: Some(Plan {
                    steps: Vec::new(),
                    length: 0,
                    metric: None,
                    makespan: Some(0.0),
                }),
                statistics: stats(&self.task, 0, self.threads),
                notes: vec!["goal already satisfied; the empty plan solves it".into()],
            };
        }
        let (kind, dur_exprs) = crate::temporal::build_kind(&self.task, c);
        let total = budget_evals.or(self.max_evaluated).unwrap_or(usize::MAX);
        let mut remaining = total;
        let node_bytes = memory_mb
            .map(|mb| mb.saturating_mul(1 << 20))
            .unwrap_or(crate::search::NODE_CAP_TARGET_BYTES);
        let tp = crate::temporal::solve_from(
            &self.task,
            &kind,
            &dur_exprs,
            &start,
            &self.task.goal_pos,
            &self.task.goal_num,
            &[],
            &[], // TILs rejected at construction
            self.threads,
            self.tier,
            &mut remaining,
            node_bytes,
        );
        let evaluated = total.saturating_sub(remaining);
        match tp {
            Some(tp) => {
                let steps = timed_steps(&tp);
                Solution {
                    solved: true,
                    mode: Mode::Temporal,
                    plan: Some(Plan {
                        length: steps.len(),
                        steps,
                        metric: None,
                        makespan: Some(tp.makespan),
                    }),
                    statistics: stats(&self.task, evaluated, self.threads),
                    notes: Vec::new(),
                }
            }
            None => Solution {
                solved: false,
                mode: Mode::Temporal,
                plan: None,
                statistics: stats(&self.task, evaluated, self.threads),
                notes: Vec::new(),
            },
        }
    }

    /// Follow before you rethink (0.12 Phase 2): does `plan`'s remaining
    /// suffix (steps `from_step..`) still execute from the CURRENT world
    /// state and end in the goal? A `true` costs a replay — no search, no
    /// think budget — so an agent whose world drifted IRRELEVANTLY keeps
    /// following its plan for free; only a broken suffix warrants a real
    /// rethink. Exact, not heuristic: classical suffixes replay op-by-op
    /// (applicability + effects), temporal suffixes replay their timed
    /// happenings in epoch order (the internal validator's machinery), and
    /// both end with the goal test.
    pub fn plan_still_valid(&self, plan: &Plan, from_step: usize) -> bool {
        let steps = match plan.steps.get(from_step..) {
            Some(s) => s,
            None => return false,
        };
        let display = |s: &crate::api::Step| {
            if s.args.is_empty() {
                s.action.clone()
            } else {
                format!("{} {}", s.action, s.args.join(" "))
            }
        };
        if self.temporal.is_some() {
            let tp = crate::temporal::TimedPlan {
                steps: steps
                    .iter()
                    .map(|s| crate::temporal::TimedStep {
                        time: s.time.unwrap_or(0.0),
                        action: display(s),
                        duration: s.duration,
                    })
                    .collect(),
                makespan: 0.0,
            };
            return match crate::temporal::treplay(&self.task, &self.task.initial(), &tp) {
                Some(end) => self.task.goal_met(&end),
                None => false,
            };
        }
        let mut state = self.task.initial();
        for s in steps {
            let oi = match self.op_ids.get(&display(s)) {
                Some(&oi) => oi,
                None => return false,
            };
            if !self.task.op_applicable(oi, &state) {
                return false;
            }
            state = self.task.apply(oi, &state);
        }
        self.task.goal_met(&state)
    }

    /// Set a world fact true/false in the current state, e.g.
    /// `set_fact("(at v1 field)", true)`. Case-insensitive. Errors if the fact was
    /// never grounded, or is static (grounding-baked — see the module docs).
    /// When grounding created the complementary `(NOT (p ...))` mirror fact
    /// (negative preconditions/goals), the mirror is kept in sync
    /// automatically — set either side, both move.
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
        // Temporal at-rest fence (0.12 Phase 1): RUNNING-* tokens mark
        // in-flight intervals — a session's world is AT REST between thinks,
        // so the game mirrors a completed action's END effects instead of
        // faking a running one. `atom_head` looks through `(NOT ...)` so the
        // fence also catches a running token's complementary mirror.
        if !self.running_preds.is_empty() {
            let head = atom_head(&key);
            if self.running_preds.iter().any(|p| p == head) {
                return Err(format!(
                    "fact `{key}` is a compiler-internal running-interval token; \
                     a session's world is at rest between thinks — mirror the \
                     action's end effects instead"
                ));
            }
        }
        self.write_fact_bit(id, value);
        if let Some(&m) = self.mirror.get(&id) {
            self.write_fact_bit(m, !value);
        }
        Ok(())
    }

    fn write_fact_bit(&mut self, id: u32, value: bool) {
        let (w, b) = (id as usize / 64, id as usize % 64);
        if value {
            self.task.init_bits[w] |= 1 << b;
        } else {
            self.task.init_bits[w] &= !(1 << b);
        }
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

    /// Retarget the session (0.13 Phase 1): replace the goal with a GROUND
    /// conjunction over the already-interned fact space — no regrounding,
    /// no re-parse of the world. One world, changing desires: an NPC that
    /// wanted iron and now wants bread swaps its goal and keeps thinking.
    ///
    /// Accepted grammar (the same `(:goal ...)` body syntax): nested `(and
    /// ...)` of ground atoms `(pred obj ...)`, negated atoms `(not (pred obj
    /// ...))` where grounding created the complementary `(NOT ...)` mirror
    /// fact, and numeric comparisons `(>= (fluent ...) expr)`. An empty
    /// `(and)` is the always-met goal.
    ///
    /// Errors — before touching the current goal — on: atoms/fluents the
    /// grounded world never contained (statics and unreachable atoms are
    /// compiled away — a session cannot want what its world cannot express),
    /// negations without a grounded mirror, compiler-reserved `RUNNING-*`
    /// tokens (temporal worlds are at rest between thinks), non-ground terms,
    /// and ADL connectives (`or`/`exists`/`forall`/object `=`) — those
    /// compile at grounding time, so re-create the `Session` for an ADL goal.
    ///
    /// A numeric goal may read a fluent the original goal never did: the
    /// visited-key relevance closure is re-run with the new fluents added.
    /// Relevance only ever GROWS within a session — state keys get finer,
    /// never coarser — so replay soundness and t1 ≡ t8 determinism hold
    /// across retargets.
    pub fn set_goal(&mut self, goal: &str) -> Result<(), String> {
        let f = crate::parser::parse_goal(goal)?;
        let mut pos: Vec<u32> = Vec::new();
        let mut num: Vec<NumPre> = Vec::new();
        self.goal_conj(&f, &mut pos, &mut num)?;
        pos.sort_unstable();
        pos.dedup();

        // Fluents newly read by this goal join the visited-key relevance
        // closure (see `PackedTask::state_key`): a fluent outside the key
        // cannot distinguish states, which is only sound while no
        // precondition or GOAL reads it. Mirror of ground.rs's closure —
        // any fluent read by an effect that writes a relevant fluent is
        // itself relevant.
        let mut rel = std::mem::take(&mut self.task.relevant_fluent);
        let mut scratch = Vec::new();
        let mut grew = false;
        for np in &num {
            np.lhs.collect_fluents(&mut scratch);
            np.rhs.collect_fluents(&mut scratch);
        }
        for &fl in &scratch {
            if !rel[fl as usize] {
                rel[fl as usize] = true;
                grew = true;
            }
        }
        if grew {
            loop {
                let mut changed = false;
                for oi in 0..self.task.n_ops {
                    let neffs = self
                        .task
                        .num_eff
                        .slice(oi)
                        .iter()
                        .chain(self.task.cond_effs(oi).flat_map(|c| c.num.iter()));
                    for ne in neffs {
                        if !rel[ne.target as usize] {
                            continue;
                        }
                        scratch.clear();
                        ne.value.collect_fluents(&mut scratch);
                        for &fl in &scratch {
                            if !rel[fl as usize] {
                                rel[fl as usize] = true;
                                changed = true;
                            }
                        }
                    }
                }
                if !changed {
                    break;
                }
            }
            self.task.rel_fluents = (0..rel.len() as u32).filter(|&i| rel[i as usize]).collect();
        }
        self.task.relevant_fluent = rel;

        self.task.goal_pos = pos;
        self.task.goal_num = num;
        Ok(())
    }

    /// Flatten a goal formula into the packed conjunction, validating every
    /// literal against the interned fact space. See [`Session::set_goal`].
    fn goal_conj(
        &self,
        f: &Formula,
        pos: &mut Vec<u32>,
        num: &mut Vec<NumPre>,
    ) -> Result<(), String> {
        match f {
            Formula::And(fs) => {
                for g in fs {
                    self.goal_conj(g, pos, num)?;
                }
                Ok(())
            }
            Formula::True => Ok(()),
            Formula::Atom(p, args) => {
                pos.push(self.goal_atom(p, args, false)?);
                Ok(())
            }
            Formula::Not(inner) => match &**inner {
                Formula::Atom(p, args) => {
                    pos.push(self.goal_atom(p, args, true)?);
                    Ok(())
                }
                _ => Err("set_goal supports `(not ...)` only directly around a \
                     ground atom"
                    .into()),
            },
            Formula::Comp(op, l, r) => {
                num.push(NumPre {
                    op: *op,
                    lhs: self.goal_nexpr(l)?,
                    rhs: self.goal_nexpr(r)?,
                });
                Ok(())
            }
            Formula::Or(_) | Formula::Exists(..) | Formula::Forall(..) | Formula::Eq(..) => Err(
                "set_goal supports ground conjunctions (atoms, negated atoms \
                 with grounded mirrors, numeric comparisons); ADL goal \
                 connectives compile at grounding time — re-create the Session \
                 with this goal instead"
                    .into(),
            ),
            Formula::Pref(..) => {
                Err("set_goal does not support PDDL3 preferences (soft goals)".into())
            }
            Formula::False => Err("goal `false` is unsatisfiable by construction".into()),
        }
    }

    /// Resolve one ground goal literal to its fact id (the mirror fact for a
    /// negated literal).
    fn goal_atom(&self, p: &str, args: &[Term], negated: bool) -> Result<u32, String> {
        let mut disp = String::from("(");
        disp.push_str(&p.to_ascii_uppercase());
        for a in args {
            match a {
                Term::Const(c) => {
                    disp.push(' ');
                    disp.push_str(&c.to_ascii_uppercase());
                }
                Term::Var(v) => {
                    return Err(format!(
                        "goal must be ground — variable `{v}` in goal atom `({p} ...)`"
                    ))
                }
            }
        }
        disp.push(')');
        if self
            .running_preds
            .iter()
            .any(|rp| rp == &p.to_ascii_uppercase())
        {
            return Err(format!(
                "goal atom `{disp}` is a compiler-internal running-interval \
                 token; a session's world is at rest between thinks"
            ));
        }
        if negated {
            let mirror_disp = format!("(NOT {disp})");
            return self.fact_ids.get(&mirror_disp).copied().ok_or_else(|| {
                format!(
                    "negative goal literal `(not {disp})` has no grounded mirror \
                     fact — grounding creates `(NOT ...)` mirrors only for atoms \
                     that occur negatively in the domain or the original goal; \
                     re-create the Session with this goal instead"
                )
            });
        }
        self.fact_ids.get(&disp).copied().ok_or_else(|| {
            format!(
                "goal atom `{disp}` is not in the grounded fact space (statics \
                 and unreachable atoms are compiled away; a session cannot want \
                 what its world cannot express)"
            )
        })
    }

    /// Ground a goal numeric expression over the interned fluent space.
    fn goal_nexpr(&self, e: &Expr) -> Result<NExpr, String> {
        Ok(match e {
            Expr::Num(n) => NExpr::Num(*n),
            Expr::Fluent(name, args) => {
                let mut disp = String::from("(");
                disp.push_str(&name.to_ascii_uppercase());
                for a in args {
                    match a {
                        Term::Const(c) => {
                            disp.push(' ');
                            disp.push_str(&c.to_ascii_uppercase());
                        }
                        Term::Var(v) => {
                            return Err(format!(
                                "goal must be ground — variable `{v}` in fluent `({name} ...)`"
                            ))
                        }
                    }
                }
                disp.push(')');
                let &id = self
                    .fluent_ids
                    .get(&disp)
                    .ok_or_else(|| format!("unknown fluent `{disp}` (not in the grounded task)"))?;
                NExpr::Fluent(id)
            }
            Expr::Add(a, b) => {
                NExpr::Add(Box::new(self.goal_nexpr(a)?), Box::new(self.goal_nexpr(b)?))
            }
            Expr::Sub(a, b) => {
                NExpr::Sub(Box::new(self.goal_nexpr(a)?), Box::new(self.goal_nexpr(b)?))
            }
            Expr::Mul(a, b) => {
                NExpr::Mul(Box::new(self.goal_nexpr(a)?), Box::new(self.goal_nexpr(b)?))
            }
            Expr::Div(a, b) => {
                NExpr::Div(Box::new(self.goal_nexpr(a)?), Box::new(self.goal_nexpr(b)?))
            }
            Expr::Neg(a) => NExpr::Neg(Box::new(self.goal_nexpr(a)?)),
        })
    }

    /// Estimated retained bytes of the SHARED grounded payload — operator
    /// columns, names, achiever indexes, the monitor block. This exists once
    /// per world however many forks live in it ([`Session::fork`]); flat
    /// array/string bytes only (nested conditional-effect allocations are
    /// not walked), so treat it as a floor, not an audit.
    pub fn world_bytes(&self) -> usize {
        use std::mem::size_of_val;
        let t = &self.task;
        let strings = |v: &[String]| {
            v.iter()
                .map(|s| s.len() + size_of::<String>())
                .sum::<usize>()
        };
        strings(&t.op_display)
            + strings(&t.fact_names)
            + strings(&t.fluent_names)
            + size_of_val(&t.pre_pos.flat[..])
            + size_of_val(&t.add.flat[..])
            + size_of_val(&t.del.flat[..])
            + size_of_val(&t.pre_num.flat[..])
            + size_of_val(&t.num_eff.flat[..])
            + size_of_val(&t.cond.flat[..])
            + size_of_val(&t.shared_cond[..])
            + size_of_val(&t.monitored[..])
            + size_of_val(&t.add_by_fact.flat[..])
            + size_of_val(&t.neff_by_fluent.flat[..])
    }

    /// Estimated retained bytes of this mind's PRIVATE state — current
    /// facts and fluents, goal, fluent relevance. This is what one more
    /// [`Session::fork`] costs; same flat-bytes caveat as
    /// [`Session::world_bytes`].
    pub fn mind_bytes(&self) -> usize {
        use std::mem::size_of_val;
        let t = &self.task;
        size_of_val(&t.init_bits[..])
            + size_of_val(&t.fv0[..])
            + size_of_val(&t.fdef0[..])
            + size_of_val(&t.goal_pos[..])
            + size_of_val(&t.goal_num[..])
            + size_of_val(&t.relevant_fluent[..])
            + size_of_val(&t.rel_fluents[..])
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

    /// [`Self::replan`] under an explicit THINK BUDGET (0.11 Phase 4, the
    /// game-embedding surface): `max_evaluated` bounds the states evaluated
    /// (the deterministic unit — never wall clock) and `memory_mb` bounds the
    /// search's retained memory via the deterministic per-node byte model
    /// (see `search::node_cap_for_bytes`). A budget-exhausted think returns
    /// `solved: false` honestly; identical inputs give identical results at
    /// any thread count.
    pub fn replan_budgeted(&self, max_evaluated: usize, memory_mb: Option<usize>) -> Solution {
        self.replan_inner(Some(max_evaluated), memory_mb)
    }

    /// Solve from the CURRENT world state toward the session's goal, paying only
    /// the search (no re-parse, no re-ground). Same structured [`Solution`] as
    /// [`crate::solve`]; `solved: false` when the goal is unreachable from here.
    pub fn replan(&self) -> Solution {
        self.replan_inner(None, None)
    }

    fn replan_inner(&self, budget_evals: Option<usize>, memory_mb: Option<usize>) -> Solution {
        if let Some(c) = &self.temporal {
            return self.replan_temporal(c, budget_evals, memory_mb);
        }
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
        let mut cfg = crate::search::SearchCfg::from_weights(
            self.weight_g,
            self.weight_h,
            budget_evals.or(self.max_evaluated),
        );
        cfg.node_bytes_target = memory_mb.map(|mb| mb.saturating_mul(1 << 20));
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
    fn budgeted_think_is_bounded_and_deterministic() {
        // A tiny budget returns an honest unsolved verdict without blowing
        // the cap; identical budgets give identical solutions at any thread
        // count (the eval budget is the deterministic unit, never wall
        // clock); an adequate budget solves.
        let s = Session::new(DOM, PRB, &Options::default()).expect("session");
        let tiny = s.replan_budgeted(1, Some(1));
        assert!(!tiny.solved, "1-eval think cannot solve the farm");
        let t1 = {
            let o = Options {
                threads: 1,
                ..Options::default()
            };
            let s = Session::new(DOM, PRB, &o).expect("session");
            s.replan_budgeted(10_000, Some(64))
        };
        let t8 = {
            let o = Options {
                threads: 8,
                ..Options::default()
            };
            let s = Session::new(DOM, PRB, &o).expect("session");
            s.replan_budgeted(10_000, Some(64))
        };
        assert!(t1.solved && t8.solved);
        let steps = |sol: &Solution| {
            sol.plan
                .as_ref()
                .unwrap()
                .steps
                .iter()
                .map(|st| (st.action.clone(), st.args.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            steps(&t1),
            steps(&t8),
            "budgeted think differs across threads"
        );
    }

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

    const TDOM: &str = "
    (define (domain workshop) (:requirements :strips :typing :durative-actions)
      (:types worker)
      (:predicates (idle ?w - worker) (built ?w - worker))
      (:durative-action build
        :parameters (?w - worker)
        :duration (= ?duration 5)
        :condition (at start (idle ?w))
        :effect (and (at start (not (idle ?w))) (at end (built ?w)))))";
    const TPRB: &str = "
    (define (problem shift) (:domain workshop)
      (:objects w1 w2 - worker)
      (:init (idle w1) (idle w2))
      (:goal (and (built w1) (built w2))))";

    #[test]
    fn follow_before_you_rethink_scripted_drift() {
        // The Phase 2 contract, scripted: irrelevant drift costs ZERO search
        // (the suffix replay says keep following); breaking drift is detected
        // exactly; think count over the script is exactly 2.
        let mut s = Session::new(DOM, PRB, &Options::default()).expect("session");
        let mut thinks = 0;

        thinks += 1;
        let think = s.replan();
        assert!(think.solved);
        let plan = think.plan.unwrap();
        assert_eq!(plan.length, 3, "walk + harvest x2");

        // Tick 1: the agent FOLLOWS step 0 (walk); the game mirrors it.
        s.set_fact("(at v1 hut)", false).unwrap();
        s.set_fact("(at v1 field)", true).unwrap();
        assert!(
            s.plan_still_valid(&plan, 1),
            "suffix after the walk must still execute"
        );

        // Tick 2: IRRELEVANT drift — a bird delivers grain. The suffix still
        // reaches the goal (grain 1 + 2 harvests >= 2): keep following, free.
        s.set_fluent("(grain)", 1.0).unwrap();
        assert!(
            s.plan_still_valid(&plan, 1),
            "helpful drift must not force a rethink"
        );

        // Tick 3: BREAKING drift — the villager is blown back home; the
        // remaining harvests are inapplicable. Exactly now we think again.
        s.set_fact("(at v1 field)", false).unwrap();
        s.set_fact("(at v1 hut)", true).unwrap();
        assert!(
            !s.plan_still_valid(&plan, 1),
            "breaking drift must be caught"
        );
        thinks += 1;
        let rethink = s.replan();
        assert!(rethink.solved);
        assert!(s.plan_still_valid(&rethink.plan.unwrap(), 0));

        assert_eq!(thinks, 2, "the whole script cost exactly two thinks");
    }

    #[test]
    fn temporal_suffix_replay_detects_breaks() {
        let mut s = Session::new(TDOM, TPRB, &Options::default()).expect("session");
        let think = s.replan_budgeted(50_000, Some(128));
        let plan = think.plan.unwrap();
        assert!(s.plan_still_valid(&plan, 0), "fresh plan replays");

        // w1's build completed out-of-band: the FULL plan (which re-starts
        // w1's build) breaks on (idle w1); the w2-only suffix still runs but
        // no longer reaches the goal alone... it DOES — (built w1) is now
        // true in the world. Both verdicts must be exact.
        s.set_fact("(built w1)", true).unwrap();
        s.set_fact("(idle w1)", false).unwrap();
        assert!(
            !s.plan_still_valid(&plan, 0),
            "re-starting w1's build must break on (idle w1)"
        );
        let w2_only = plan
            .steps
            .iter()
            .position(|st| st.args == vec!["W2".to_string()])
            .unwrap();
        assert!(
            s.plan_still_valid(&plan, w2_only.max(1)),
            "the w2-only suffix still reaches the goal"
        );
    }

    #[test]
    fn temporal_sessions_think_concurrently_and_replan() {
        // Two workers build in PARALLEL (genuine concurrency: both intervals
        // overlap); the world drifts (w1's build completes out-of-band) and
        // the rethink plans only w2's remaining work.
        let s = Session::new(TDOM, TPRB, &Options::default()).expect("temporal session");
        let think = s.replan_budgeted(50_000, Some(128));
        assert!(think.solved, "workshop must solve");
        let plan = think.plan.as_ref().unwrap();
        assert_eq!(plan.length, 2, "one build per worker");
        assert!(plan.makespan.unwrap() < 5.1, "concurrent, not sequential");
        let (a, b) = (&plan.steps[0], &plan.steps[1]);
        assert!(a.time.is_some() && a.duration.is_some(), "timed steps");
        // interval overlap: each starts before the other ends
        let (ta, da) = (a.time.unwrap(), a.duration.unwrap());
        let (tb, db) = (b.time.unwrap(), b.duration.unwrap());
        assert!(ta < tb + db && tb < ta + da, "intervals must overlap");

        let mut s = s;
        s.set_fact("(built w1)", true).unwrap();
        s.set_fact("(idle w1)", false).unwrap();
        let rethink = s.replan_budgeted(50_000, Some(128));
        assert!(rethink.solved);
        assert_eq!(
            rethink.plan.as_ref().unwrap().length,
            1,
            "only w2's build remains"
        );
    }

    #[test]
    fn temporal_think_budget_is_bounded_and_deterministic() {
        let s = Session::new(TDOM, TPRB, &Options::default()).expect("session");
        let tiny = s.replan_budgeted(1, Some(1));
        assert!(!tiny.solved, "a 1-eval temporal think cannot solve");
        let run = |threads: usize| {
            let o = Options {
                threads,
                ..Options::default()
            };
            let s = Session::new(TDOM, TPRB, &o).expect("session");
            s.replan_budgeted(50_000, Some(128))
        };
        let (t1, t8) = (run(1), run(8));
        assert!(t1.solved && t8.solved);
        let key = |sol: &Solution| {
            sol.plan
                .as_ref()
                .unwrap()
                .steps
                .iter()
                .map(|st| {
                    (
                        st.action.clone(),
                        st.args.clone(),
                        (st.time.unwrap() * 1000.0).round() as i64,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(key(&t1), key(&t8), "temporal think differs across threads");
    }

    #[test]
    fn set_goal_retargets_without_regrounding() {
        // One world, changing desires: the same session serves a numeric
        // goal, then a positional one, then an empty one — no regrounding,
        // and plan_still_valid always answers against the CURRENT goal.
        let mut s = Session::new(DOM, PRB, &Options::default()).expect("session");
        let grain_plan = s.replan();
        assert!(grain_plan.solved);
        let grain_plan = grain_plan.plan.unwrap();
        assert_eq!(grain_plan.length, 3, "walk + harvest x2");

        // Retarget: forget grain, just be at the field.
        s.set_goal("(at v1 field)").unwrap();
        let at_field = s.replan();
        assert!(at_field.solved);
        assert_eq!(at_field.plan.as_ref().unwrap().length, 1, "one walk");
        // The OLD plan still reaches the new goal (its walk passes through) —
        // replay must answer against the CURRENT goal, not the birth goal.
        assert!(s.plan_still_valid(&grain_plan, 0));
        // A goal the old plan does NOT reach: back at the hut.
        s.set_goal("(and (at v1 hut) (>= (grain) 1))").unwrap();
        assert!(
            !s.plan_still_valid(&grain_plan, 0),
            "the old plan ends at the field with grain 2 but never returns home"
        );
        let home = s.replan();
        assert!(home.solved);
        // walk + harvest + walk back (order may vary): 3 steps min? walk,
        // harvest, walk = 3.
        assert_eq!(home.plan.as_ref().unwrap().length, 3);

        // Empty conjunction: the always-met goal.
        s.set_goal("(and)").unwrap();
        let idle = s.replan();
        assert!(idle.solved);
        assert_eq!(idle.plan.unwrap().length, 0);
    }

    #[test]
    fn set_goal_rejects_unknown_and_adl() {
        let mut s = Session::new(DOM, PRB, &Options::default()).expect("session");
        let before = s.replan();
        // Unknown atom: never in the grounded world.
        let err = s.set_goal("(at v1 nowhere)").unwrap_err();
        assert!(err.contains("not in the grounded fact space"), "{err}");
        // Unknown fluent.
        let err = s.set_goal("(>= (gold) 1)").unwrap_err();
        assert!(err.contains("unknown fluent"), "{err}");
        // ADL connective.
        let err = s.set_goal("(or (at v1 hut) (at v1 field))").unwrap_err();
        assert!(err.contains("ADL"), "{err}");
        // Non-ground.
        let err = s.set_goal("(at ?a field)").unwrap_err();
        assert!(err.contains("ground"), "{err}");
        // Negation without a grounded mirror (farm has no negative pre/goals).
        let err = s.set_goal("(not (at v1 hut))").unwrap_err();
        assert!(err.contains("no grounded mirror"), "{err}");
        // Every rejection left the ORIGINAL goal untouched.
        let after = s.replan();
        assert_eq!(
            before.plan.unwrap().length,
            after.plan.unwrap().length,
            "a failed set_goal must not corrupt the current goal"
        );
    }

    const NEG_DOM: &str = "
    (define (domain lamp) (:requirements :strips :negative-preconditions)
      (:predicates (on) (broken))
      (:action switch-on :precondition (and (not (on)) (not (broken))) :effect (on))
      (:action switch-off :precondition (on) :effect (not (on))))";
    const NEG_PRB: &str = "
    (define (problem p) (:domain lamp)
      (:init) (:goal (on)))";

    #[test]
    fn set_goal_negative_literals_via_mirrors_and_set_fact_sync() {
        // `(not (on))` occurs negatively in a precondition, so grounding
        // created the mirror fact — a negated goal literal is expressible,
        // and set_fact keeps base and mirror in sync (a stale mirror would
        // corrupt applicability, not just goals).
        let mut s = Session::new(NEG_DOM, NEG_PRB, &Options::default()).expect("session");
        let on = s.replan();
        assert!(on.solved);
        assert_eq!(on.plan.unwrap().length, 1, "switch-on");

        // The lamp got switched on out-of-band; retarget to (not (on)).
        s.set_fact("(on)", true).unwrap();
        s.set_goal("(not (on))").unwrap();
        let off = s.replan();
        assert!(off.solved);
        assert_eq!(off.plan.unwrap().length, 1, "switch-off");

        // Mirror sync end-to-end: turn it back off out-of-band; switch-on
        // (which REQUIRES the mirror `(NOT (ON))` true) must be applicable
        // again — it would not be if set_fact left the mirror stale.
        s.set_fact("(on)", false).unwrap();
        s.set_goal("(on)").unwrap();
        let relit = s.replan();
        assert!(
            relit.solved,
            "stale mirror would make switch-on inapplicable"
        );
        assert_eq!(relit.plan.unwrap().length, 1);
    }

    #[test]
    fn set_goal_numeric_relevance_grows_the_state_key() {
        // A domain with a write-only accumulator: irrelevant at construction
        // (it is in no precondition/goal), so it is OUT of the visited key.
        // Retargeting the goal onto it must pull it INTO the key — otherwise
        // states differing only in the accumulator dedup and search is wrong.
        let dom = "
        (define (domain walkers) (:requirements :strips :typing :numeric-fluents)
          (:types agent place)
          (:predicates (at ?a - agent ?p - place) (road ?x ?y - place))
          (:functions (steps))
          (:action walk :parameters (?a - agent ?from ?to - place)
            :precondition (and (at ?a ?from) (road ?from ?to))
            :effect (and (not (at ?a ?from)) (at ?a ?to) (increase (steps) 1))))";
        let prb = "
        (define (problem p) (:domain walkers)
          (:objects v1 - agent a b - place)
          (:init (at v1 a) (road a b) (road b a) (= (steps) 0))
          (:goal (at v1 b)))";
        let mut s = Session::new(dom, prb, &Options::default()).expect("session");
        assert!(s.replan().solved);
        // Retarget onto the accumulator: pace until three steps are walked.
        s.set_goal("(>= (steps) 3)").unwrap();
        let paced = s.replan();
        assert!(
            paced.solved,
            "goal over a formerly-irrelevant fluent must solve (key must grow)"
        );
        assert_eq!(paced.plan.unwrap().length, 3, "a-b, b-a, a-b");
    }

    #[test]
    fn set_goal_temporal_retarget() {
        // Temporal sessions retarget too: build only w2 instead of both.
        let mut s = Session::new(TDOM, TPRB, &Options::default()).expect("session");
        let both = s.replan_budgeted(50_000, Some(128));
        assert!(both.solved);
        assert_eq!(both.plan.as_ref().unwrap().length, 2);

        s.set_goal("(built w2)").unwrap();
        let solo = s.replan_budgeted(50_000, Some(128));
        assert!(solo.solved);
        let plan = solo.plan.as_ref().unwrap();
        assert_eq!(plan.length, 1, "only w2's build serves the new goal");
        assert_eq!(plan.steps[0].args, vec!["W2".to_string()]);
        // The two-build plan still meets (built w2) — replay agrees.
        assert!(s.plan_still_valid(both.plan.as_ref().unwrap(), 0));
        // RUNNING tokens are fenced in goals exactly as in set_fact.
        let err = s.set_goal("(RUNNING-BUILD w1)").unwrap_err();
        assert!(err.contains("running-interval"), "{err}");
    }

    #[test]
    fn set_goal_retarget_is_deterministic_across_threads() {
        let run = |threads: usize| {
            let o = Options {
                threads,
                ..Options::default()
            };
            let mut s = Session::new(DOM, PRB, &o).expect("session");
            s.set_goal("(and (at v1 hut) (>= (grain) 2))").unwrap();
            s.replan_budgeted(10_000, Some(64))
        };
        let (t1, t8) = (run(1), run(8));
        assert!(t1.solved && t8.solved);
        let steps = |sol: &Solution| {
            sol.plan
                .as_ref()
                .unwrap()
                .steps
                .iter()
                .map(|st| (st.action.clone(), st.args.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            steps(&t1),
            steps(&t8),
            "retargeted think differs across threads"
        );
    }

    #[test]
    fn temporal_sessions_fence_tils_and_running_tokens() {
        let til_prb = "(define (problem p) (:domain workshop)
          (:objects w1 - worker)
          (:init (idle w1) (at 3 (idle w1)))
          (:goal (built w1)))";
        let err = match Session::new(TDOM, til_prb, &Options::default()) {
            Err(e) => e,
            Ok(_) => panic!("TIL problem must be rejected"),
        };
        assert!(
            err.contains("timed initial"),
            "TILs must be rejected: {err}"
        );

        let mut s = Session::new(TDOM, TPRB, &Options::default()).expect("session");
        let err = s.set_fact("(RUNNING-BUILD w1)", true).unwrap_err();
        assert!(
            err.contains("running-interval"),
            "RUNNING-* must be fenced: {err}"
        );
    }

    #[test]
    fn fork_shares_the_world_and_inherits_current_state() {
        let mut s = Session::new(DOM, PRB, &Options::default()).expect("session");
        // Move the world BEFORE forking: the fork starts from here, not :init.
        s.set_fact("(at v1 hut)", false).unwrap();
        s.set_fact("(at v1 field)", true).unwrap();
        s.set_fluent("(grain)", 1.0).unwrap();
        let f = s.fork();
        assert_eq!(f.fact("(at v1 field)"), Some(true));
        assert_eq!(f.fluent("(grain)"), Some(1.0));
        // The grounded payload is SHARED, not copied — the whole point.
        assert!(Arc::ptr_eq(&s.task.fact_names, &f.task.fact_names));
        assert!(Arc::ptr_eq(&s.task.op_display, &f.task.op_display));
        assert!(Arc::ptr_eq(&s.task.add.flat, &f.task.add.flat));
        assert!(Arc::ptr_eq(&s.fact_ids, &f.fact_ids));
        let sol = f.replan();
        assert!(sol.solved);
        assert_eq!(sol.plan.unwrap().length, 1, "one harvest from here");
    }

    #[test]
    fn forks_diverge_without_touching_siblings() {
        let s = Session::new(DOM, PRB, &Options::default()).expect("session");
        let mut hungry = s.fork();
        let mut idle = s.fork();
        // One mind's world-writes and retarget...
        hungry.set_fluent("(grain)", 1.0).unwrap();
        hungry.set_goal("(>= (grain) 5)").unwrap();
        // ...are invisible to its sibling and its parent.
        assert_eq!(idle.fluent("(grain)"), Some(0.0));
        assert_eq!(s.fluent("(grain)"), Some(0.0));
        let h = hungry.replan();
        assert!(h.solved);
        assert_eq!(h.plan.unwrap().length, 5, "walk + 4 more harvests");
        let i = idle.replan();
        assert!(i.solved);
        assert_eq!(
            i.plan.unwrap().length,
            3,
            "sibling still walks + harvests 2"
        );
        assert_eq!(s.replan().plan.unwrap().length, 3, "parent unmoved");
        // A fork's relevance growth (set_goal onto a new fluent elsewhere)
        // stays its own: idle's key/goal are untouched by hungry's retarget.
        idle.set_fact("(at v1 hut)", false).unwrap();
        idle.set_fact("(at v1 field)", true).unwrap();
        assert_eq!(hungry.fact("(at v1 field)"), Some(false));
    }

    #[test]
    fn fork_temporal_population_thinks_independently() {
        let s = Session::new(TDOM, TPRB, &Options::default()).expect("session");
        let mut a = s.fork();
        let mut b = s.fork();
        a.set_goal("(built w1)").unwrap();
        b.set_goal("(built w2)").unwrap();
        let pa = a.replan_budgeted(50_000, Some(128));
        let pb = b.replan_budgeted(50_000, Some(128));
        assert!(pa.solved && pb.solved);
        assert_eq!(
            pa.plan.as_ref().unwrap().steps[0].args,
            vec!["W1".to_string()]
        );
        assert_eq!(
            pb.plan.as_ref().unwrap().steps[0].args,
            vec!["W2".to_string()]
        );
        // Parent still wants both and still gets both.
        assert_eq!(s.replan_budgeted(50_000, Some(128)).plan.unwrap().length, 2);
    }

    #[test]
    fn fork_keeps_thread_determinism() {
        let run = |threads: usize| {
            let o = Options {
                threads,
                ..Options::default()
            };
            let s = Session::new(DOM, PRB, &o).expect("session");
            let mut f = s.fork();
            f.set_fluent("(grain)", 1.0).unwrap();
            f.set_goal("(>= (grain) 4)").unwrap();
            f.replan_budgeted(10_000, Some(64))
        };
        let (t1, t8) = (run(1), run(8));
        assert!(t1.solved && t8.solved);
        let steps = |sol: &Solution| {
            sol.plan
                .as_ref()
                .unwrap()
                .steps
                .iter()
                .map(|st| (st.action.clone(), st.args.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            steps(&t1),
            steps(&t8),
            "forked think differs across threads"
        );
    }
}
