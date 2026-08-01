//! Data-parallel weighted best-first search.
//!
//! Each round pops a *batch* of the lowest-f nodes, expands all their applicable
//! successors, dedups against the visited set, then evaluates the FF heuristic
//! for the whole batch of successors IN PARALLEL (the dominant cost). Because
//! `par_map` preserves order and all control flow is on one thread, the plan
//! found is identical regardless of the worker count — only the wall-clock of
//! heuristic evaluation changes.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

use crate::hash::{FxHashMap, FxHashSet};
use crate::heuristic::{relaxed_costed, relaxed_helpful, relaxed_to, Scratch};
use crate::packed::{PackedTask, State, StateKey};
use crate::par;
use crate::types::NumPre;

/// Frontier batch size — FIXED (independent of thread count) so the search
/// expansion order, and thus the plan AND the evaluated-state count, are
/// identical for any worker count; threads only split each batch's h-eval.
const BATCH: usize = 256;
/// Default safety cap on evaluated states (deterministic).
pub const DEFAULT_MAX_EVAL: usize = 5_000_000;
/// Fixed-point scale for fractional heuristic weights (keeps the priority key an
/// integer, so the heap order — and thus the plan — stays deterministic).
const WEIGHT_SCALE: f64 = 256.0;

/// Deterministic retained-memory target for one `search_from` pass (0.8
/// Phase 3, docs/roadmap-0.8.md): the append-only `nodes` store grows one
/// full state per inserted successor (and until 0.20 Phase 4 the visited
/// set cloned each state's bitset again) — on monitor-widened tasks a
/// single unbounded pass OOMs a 15 GB box while `max_eval` (which counts
/// only POPPED nodes) never fires. The insertion cap derives from this byte target over a MODEL of
/// per-insertion cost (never RSS, never wall clock — the count is serial and
/// the model uses only static task dimensions, so the cap is identical on
/// any machine at any thread count). A capped pass returns its anytime
/// incumbent (or `Unsolvable{capped:true}`), which every caller's
/// ladder/budget machinery already treats as inconclusive — `proven` stays
/// honest. The default is far above every green fixture's retained size
/// (largest measured: ~5.4 GB total on storage qualpref p08);
/// `FF_SEARCH_NODE_CAP` overrides the node count directly (`0` disables).
///
/// On 32-bit targets (wasm32) `8 << 30` does not fit a usize — shl silently
/// DROPS the high bits, so the "8 GiB" target wrapped to a ZERO cap and
/// every default-cap search (all of temporal, the classical best-first
/// fallback) died at its first insertion while EHC-solvable instances
/// sneaked through — the 0.18 screens smoke test caught it. 2 GiB is the
/// honest 32-bit ceiling (wasm memory tops out at 4 GiB).
pub(crate) const NODE_CAP_TARGET_BYTES: usize = if usize::BITS < 64 { 2 << 30 } else { 8 << 30 };

/// The per-insertion byte model behind [`NODE_CAP_TARGET_BYTES`]: one stored
/// `State` (bits + fluent vecs) in `nodes` plus the hash→index dedup entry
/// (0.20 Phase 4 — the visited set no longer clones the bitset).
pub(crate) fn node_cap_for(task: &PackedTask) -> usize {
    node_cap_for_bytes(task, NODE_CAP_TARGET_BYTES.min(rlimit_budget()))
}

/// The address-space budget the process ACTUALLY has (0.19 Phase 4): the
/// fixed 8 GiB retained-bytes target silently exceeded the benchmark
/// runner's per-job `RLIMIT_AS` (phys/jobs) on small-state numeric tasks —
/// tiny states evaluate fast, the model allowed ~40M insertions, and the
/// OOM kill fired before the internal cap did (105 mem-cap rows on the
/// fresh 2023-numeric board, ALL search-state-owned per the
/// RSS-at-forced-cap attribution: 24 MB and 44–276 ops at a 10k cap).
/// When an RLIMIT_AS is set, retained state may spend at most 60% of it
/// (the rest covers the task tables, open list, and transient churn the
/// per-node model does not count). No limit ⇒ `usize::MAX` (dev boxes
/// keep today's exact caps — classical baselines are byte-identical).
/// Read via `/proc/self/limits` (no libc dependency — the crate stays
/// serde+thiserror only); non-Linux platforms simply keep the fixed
/// target.
fn rlimit_budget() -> usize {
    let Ok(limits) = std::fs::read_to_string("/proc/self/limits") else {
        return usize::MAX;
    };
    for line in limits.lines() {
        if line.starts_with("Max address space") {
            // "Max address space  <soft>  <hard>  bytes"
            let mut it = line.split_whitespace().skip(3);
            if let Some(soft) = it.next() {
                if let Ok(bytes) = soft.parse::<u128>() {
                    return ((bytes * 6 / 10) as u64).try_into().unwrap_or(usize::MAX);
                }
                // "unlimited" parses as Err — no clamp
            }
        }
    }
    usize::MAX
}

/// [`node_cap_for`] against an explicit byte target (the budgeted-think
/// surface); `FF_SEARCH_NODE_CAP` still overrides the count directly.
pub(crate) fn node_cap_for_bytes(task: &PackedTask, bytes: usize) -> usize {
    if let Ok(v) = std::env::var("FF_SEARCH_NODE_CAP") {
        if let Ok(n) = v.trim().parse::<usize>() {
            return if n == 0 { usize::MAX } else { n };
        }
    }
    // One stored `State` in the arena + the hash->index dedup entry (0.20
    // Phase 4 dropped the visited set's second bitset copy — the old model
    // charged `2 * words * 8`). The +128 covers Node bookkeeping, the map
    // entry, and its singleton index bucket.
    let per_node = task.words * 8 + task.fv0.len() * 8 + task.fdef0.len() + 128;
    bytes / per_node.max(1)
}

/// Tunable weighted-best-first parameters (exposed via the library `Options`).
/// `w_g`/`w_h` are pre-scaled integers (`weight * WEIGHT_SCALE`), so the default
/// `1·g + 5·h` ordering is preserved exactly while fractional weights still work.
///
/// Key units, for calibrating new terms: one h-unit = 1280 (5·256), one g-step
/// = 256, one unsatisfied preference = `weight·100` (`SatGuidance::penalty`),
/// one metric-cost unit = `w_c·256`. `w_c` (default 0.0 = term absent, key
/// bit-identical to the historical one) adds the successor's accumulated
/// metric cost `fv[cost_fluent]` to the ordering — used by the preference-
/// metric B&B loops so a folded numeric metric (rovers' traverse costs) and
/// forgo-vs-satisfy trade-offs order the open list instead of only pruning at
/// the bound.
#[derive(Clone, Copy, Debug)]
pub struct SearchCfg {
    pub w_g: i64,
    pub w_h: i64,
    pub max_eval: usize,
    pub w_c: f64,
    /// Evaluate h as the COST-augmented relaxed plan ([`relaxed_costed`]:
    /// selected-op cost + length) toward the given cost fluent, instead of
    /// plain length. Used by the `:action-costs` improvement sweep so the
    /// guidance prefers cheap achievers. `None` (default) keeps the historical
    /// length-h bit-for-bit.
    pub h_cost: Option<usize>,
    /// Anytime in-sweep tightening for the bounded metric searches: on an
    /// accepting state, record it as the incumbent and TIGHTEN the bound in
    /// place instead of returning — the sweep keeps draining (no restart, no
    /// prefix re-tread) until the eval cap or open-list exhaustion, then
    /// returns the best incumbent. Off (`false`) reproduces the historical
    /// first-improvement behavior bit-for-bit; only the preference B&B loops
    /// set it (`FF_PREF_GREEDY=1` restores first-improvement there). All
    /// tightening happens in the serial acceptance section, so determinism
    /// and thread-count independence are preserved.
    pub anytime: bool,
    /// Plan-LENGTH bound: successors at depth >= this are not inserted (a
    /// goal reached at depth g is a plan of length g, so nothing at or past
    /// the bound can beat the incumbent that set it). `usize::MAX` (the
    /// default everywhere) is exactly the historical behavior — the check
    /// never fires. Used by the iterated-weight length sweep
    /// (`costs::improve_length`); pruned states are NOT visited-marked, so a
    /// shorter route to them through another parent stays reachable.
    pub g_bound: usize,
    /// Length-anytime WITHIN one search (0.10 Phase 3): on the first goal,
    /// record the incumbent, tighten a live g-bound to its length, and keep
    /// draining the SAME open list (no restart, no prefix re-tread) for a
    /// strictly shorter plan until the drain ceiling — the eval count DOUBLES
    /// at most (ceiling = evals-at-first-incumbent × 2, still under
    /// `max_eval`). OPT-IN via `FF_LEN_ANYTIME=1`, measured and default-OFF:
    /// at the 60 s scoreboard budget the drain's doubled wall cost 9
    /// instances of coverage across floor-tile/visit-all/sokoban (sokoban
    /// −7) against 4 shorter sokoban plans (−234 steps) and ZERO length
    /// gains on floor-tile/visit-all — the same verdict class as 0.9's
    /// improve_length restarts. Mutually exclusive with the metric
    /// `anytime`.
    pub len_anytime: bool,
    /// Landmark-count ordering term (0.11 Phase 3, pre-scaled like `w_g`):
    /// adds `w_lm × unaccepted-landmark-count` to the best-first key.
    /// 0 (default) = term absent, key bit-identical. The bounded classical
    /// experiment: `FF_CLM=<weight>` sets it on the ladder's best-first
    /// fallback only.
    pub w_lm: i64,
    /// Resource-trip ordering term (0.14 ext Phase 11, pre-scaled like
    /// `w_g`): adds `w_res × ⌈unmet linked goals / pool capacity⌉` to the
    /// best-first key — the ⌈demand/capacity⌉ semantic-landmark rung the
    /// delete relaxation can't see (counter levels accumulate under
    /// relaxation). 0 (default) = absent, key bit-identical. Opt-in:
    /// `FF_RESLM=<weight>` on the ladder's best-first fallback; the
    /// [`crate::resource::TripBound`] data rides the `RESLM` cell set by
    /// `resolve::solve` (which holds the mutex groups).
    pub w_res: i64,
    /// Per-search retained-memory target in BYTES (0.11 Phase 4, the
    /// budgeted-think surface): overrides the default 8 GiB target behind
    /// the internal `node_cap_for` per-node byte model.
    /// `None` = the default. Deterministic (static task dims only) — a
    /// think budget must bound memory without introducing wall-clock
    /// nondeterminism. `FF_SEARCH_NODE_CAP` still wins when set.
    pub node_bytes_target: Option<usize>,
}

impl Default for SearchCfg {
    fn default() -> Self {
        SearchCfg::from_weights(1.0, 5.0, None)
    }
}

impl SearchCfg {
    /// Build from human-facing f64 weights. `weight_g = 1.0, weight_h = 5.0`
    /// reproduces the historical `1·g + 5·h` ordering bit-for-bit.
    ///
    /// Inputs are sanitized so a malformed weight can never collapse or overflow
    /// the integer heap key: a non-finite or negative weight falls back to that
    /// term's default, weights are clamped to a sane maximum, and if both round
    /// to zero the defaults are restored (an all-zero key would degrade to
    /// insertion order).
    pub fn from_weights(weight_g: f64, weight_h: f64, max_eval: Option<usize>) -> Self {
        let san = |w: f64, default: f64| {
            if w.is_finite() && w >= 0.0 {
                w.min(1e9)
            } else {
                default
            }
        };
        let mut w_g = (san(weight_g, 1.0) * WEIGHT_SCALE).round() as i64;
        let mut w_h = (san(weight_h, 5.0) * WEIGHT_SCALE).round() as i64;
        if w_g == 0 && w_h == 0 {
            w_g = WEIGHT_SCALE as i64;
            w_h = (5.0 * WEIGHT_SCALE) as i64;
        }
        SearchCfg {
            w_g,
            w_h,
            max_eval: max_eval.unwrap_or(DEFAULT_MAX_EVAL),
            w_c: 0.0,
            h_cost: None,
            anytime: false,
            g_bound: usize::MAX,
            len_anytime: false,
            w_lm: 0,
            w_res: 0,
            node_bytes_target: None,
        }
    }

    /// Switch h to the cost-augmented relaxed plan toward `cost_fluent`
    /// (see the `h_cost` field docs).
    pub fn with_cost_h(mut self, cost_fluent: usize) -> Self {
        self.h_cost = Some(cost_fluent);
        self
    }

    /// Add a metric-cost ordering weight (see the struct docs). Non-finite or
    /// negative weights sanitize to 0.0 (term absent), like `from_weights`.
    pub fn with_cost_weight(mut self, w_c: f64) -> Self {
        self.w_c = if w_c.is_finite() && w_c > 0.0 {
            w_c.min(1e9)
        } else {
            0.0
        };
        self
    }
}

pub enum PlanResult {
    Plan {
        ops: Vec<usize>,
        advance: Vec<i32>,
        evaluated: usize,
        max_g: usize,
    },
    Unsolvable {
        evaluated: usize,
        capped: bool, // true if the MAX_EVAL safety cap was hit (not proven unsolvable)
    },
}

struct Node {
    state: State,
    father: usize,
    op: usize,
    g: usize,
    /// Landmarks accepted along the path (0.11 Phase 3, `w_lm` only;
    /// empty — no allocation — when the term is off).
    lm_acc: Vec<u64>,
}

/// (Duplicated tiny helpers — the lama.rs shape; three call sites across
/// search/lama/temporal keep their own copies deliberately: each search owns
/// its node layout and hot loop.)
fn clm_accept_into(accepted: &mut [u64], lms: &[u32], state: &State) {
    for (i, &f) in lms.iter().enumerate() {
        if accepted[i >> 6] & (1 << (i & 63)) == 0 && crate::bitset::test(&state.bits, f as usize) {
            accepted[i >> 6] |= 1 << (i & 63);
        }
    }
}

fn clm_unaccepted(accepted: &[u64], n: usize) -> i64 {
    n as i64 - accepted.iter().map(|w| w.count_ones() as i64).sum::<i64>()
}

/// A preference's phi in grounded DNF: it holds in a state iff ANY disjunct's
/// positive facts all hold and its numeric comparisons all pass (negative
/// literals arrive as compiled complementary facts, so positives suffice).
/// Built from the `P3COLLECT-i` ops' preconditions — one disjunct per op. An
/// EMPTY disjunct list means phi is unachievable (never holds).
pub struct PrefPhi {
    pub disjuncts: Vec<(Vec<u32>, Vec<NumPre>)>,
}

impl PrefPhi {
    #[inline]
    pub fn holds(&self, s: &State) -> bool {
        self.disjuncts.iter().any(|(pos, num)| {
            pos.iter()
                .all(|&f| crate::bitset::test(&s.bits, f as usize))
                && num
                    .iter()
                    .all(|np| crate::types::eval_numpre(np, &s.fv, &s.fdef).unwrap_or(false))
        })
    }
}

/// The EXACT preference-closure cost of a state: the summed weight of the
/// preference instances whose phi does not hold — precisely what the phase
/// tail ([`crate::pddl3::PhaseTail`]) will pay in forgo actions if the plan
/// stops here (`P3END` only toggles phase facts, changing no phi). Passing
/// this to [`search_from`] switches the goal test to metric-bounded
/// acceptance: a popped state is accepted iff the real goal holds AND
/// `cost-so-far + closure < bound` — the search then explores REAL states
/// only and the compiled bookkeeping goals never enter the search at all.
pub struct ClosureCost {
    /// `(forgo weight, phi)` per preference instance with positive weight.
    pub prefs: Vec<(f64, PrefPhi)>,
}

impl ClosureCost {
    pub fn cost(&self, s: &State) -> f64 {
        self.prefs
            .iter()
            .filter(|(_, phi)| !phi.holds(s))
            .map(|(w, _)| w)
            .sum()
    }
}

/// Metric search guidance: bias the open list toward states that satisfy more
/// preferences. Each entry is `(phi, heap penalty while that preference is
/// unsatisfied)` — phi in full DNF ([`PrefPhi`]), so `imply`/`exists`
/// preferences guide correctly instead of being invisible. Evaluated on the
/// concrete successor state, so it sees real satisfaction (unlike the
/// delete-relaxed heuristic, which the free Keyder-Geffner forgo action
/// blinds). Only the metric B&B passes this; it changes node *ordering* only,
/// never which nodes are legal.
/// Data for the `w_res` trip-bound term (0.14 ext Phase 11). Set once per
/// process by `resolve::solve` (the caller holding the mutex groups) when
/// `FF_RESLM` is on; read by `search_from` only when `cfg.w_res > 0`. An
/// experiment hatch, same lifecycle as the env vars that gate it.
pub(crate) static RESLM: std::sync::OnceLock<Option<crate::resource::TripBound>> =
    std::sync::OnceLock::new();

/// Wall-budget awareness (0.18 Phase 4): `FF_TIME_LIMIT=<secs>` is the
/// process's REAL wall budget. Armed on first touch — [`arm_wall_limit`]
/// is called at `api::solve` entry so the clock starts before grounding,
/// not at the first search. `None` = no limit set (all-rungs behavior).
/// On wasm the Clock is frozen at zero, so a limit never expires — the
/// gate degrades to always-affordable, which is correct for thinks.
static WALL: std::sync::OnceLock<Option<(crate::clock::Clock, f64)>> = std::sync::OnceLock::new();

/// Start the wall-budget clock (idempotent). Called at solve entry.
pub fn arm_wall_limit() {
    WALL.get_or_init(|| {
        std::env::var("FF_TIME_LIMIT")
            .ok()
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|s| s.is_finite() && *s > 0.0)
            .map(|s| (crate::clock::Clock::now(), s))
    });
}

/// Fraction of the wall budget remaining, `None` if no limit is set.
fn wall_remaining_frac() -> Option<f64> {
    WALL.get_or_init(|| None).as_ref().map(|(start, total)| {
        let used = start.elapsed_ms() as f64 / 1000.0;
        ((total - used) / total).max(0.0)
    })
}

pub struct SatGuidance {
    pub prefs: Vec<(PrefPhi, i64)>,
    /// Renewable resources whose live occupancy is penalized on the concrete
    /// state (what delete-relaxation hides). Empty unless a counter resource was
    /// detected; see [`crate::resource`].
    pub res: Vec<crate::resource::ResourceVar>,
    /// Per-`occupancy²` weight for the resource term. 0 disables it (the heap key
    /// is then bit-identical to the forgone-only behavior).
    pub res_weight: i64,
    /// Only occupancy ABOVE this threshold is penalized (penalize `(occ-thresh)²`).
    /// 0 penalizes all occupancy; a value near capacity penalizes only the
    /// dead-end zone (all stacks committed) without suppressing normal pipelining.
    pub res_thresh: i64,
    /// ESPC make-deadline guidance: `(trigger fact, deliverable fact, value)`
    /// triples. A **locked loss** — `trigger` present, `deliverable` absent — means
    /// a once-only conditional achievement (e.g. openstacks' `make-product`, which
    /// can fire only while `(not (made p))`) already fired WITHOUT delivering, so
    /// `value` of metric is permanently forfeit. The delete-relaxed RPG is blind to
    /// this (it can re-add the deliverable); reading it on the CONCRETE state steers
    /// the search to satisfy the enabling condition (start the order) BEFORE the
    /// trigger fires. Empty / weight 0 ⇒ inert (heap key bit-identical to today).
    pub deadline: Vec<(u32, u32, i64)>,
    /// λ multiplier for the deadline term (0 disables it; keeps the key integer).
    pub deadline_weight: i64,
}

impl SatGuidance {
    /// Total penalty of preferences not (yet) satisfied in `s`.
    fn forgone(&self, s: &State) -> i64 {
        let mut pen = 0;
        for (phi, p) in &self.prefs {
            if !phi.holds(s) {
                pen += *p;
            }
        }
        pen
    }

    /// Combined heap-ordering penalty: forgone preferences + a CONVEX renewable-
    /// resource occupancy term (`w · occupancy²`, summed over detected resources).
    /// Both are read off the concrete state, so the search sees the resource the
    /// delete-relaxed heuristic cannot. Ordering only — never legality — so
    /// completeness is preserved. The convex shape discourages high simultaneous
    /// occupancy (the peak), steering toward "release before acquiring more".
    fn penalty(&self, s: &State) -> i64 {
        let mut pen = self.forgone(s);
        if self.res_weight != 0 {
            for r in &self.res {
                let over = (r.occupancy(&s.bits) as i64 - self.res_thresh).max(0);
                pen += self.res_weight * over * over;
            }
        }
        if self.deadline_weight != 0 {
            for &(trigger, deliverable, val) in &self.deadline {
                if crate::bitset::test(&s.bits, trigger as usize)
                    && !crate::bitset::test(&s.bits, deliverable as usize)
                {
                    pen += self.deadline_weight * val;
                }
            }
        }
        pen
    }
}

/// Solve toward an ARBITRARY (sub)goal from an arbitrary start state over a
/// shared grounded task — the reusable subplanner entry point for SGPlan-style
/// partition-and-resolve. `search` is the whole-task convenience wrapper.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn search_from(
    task: &PackedTask,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    cost_fluent: Option<usize>,
    cost_bound: f64,
    threads: usize,
    cfg: SearchCfg,
    forbidden: &[bool],
    sat: Option<&SatGuidance>,
    closure: Option<&ClosureCost>,
) -> PlanResult {
    let batch = BATCH;
    let node_cap = match cfg.node_bytes_target {
        Some(b) => node_cap_for_bytes(task, b),
        None => node_cap_for(task),
    };
    // Phase-time attribution, printed only under FF_RES_DEBUG at the cap
    // return (measurement only — never affects behavior).
    let dbg = std::env::var("FF_RES_DEBUG").is_ok();
    let t_all = crate::clock::Clock::now();
    let (mut t_h, mut t_exp, mut t_ins) = (0u128, 0u128, 0u128);

    let init = start.clone();
    // early dead-end check: if the initial state is a relaxed dead end, unsolvable
    if relaxed_to(
        task,
        &mut Scratch::new(task),
        &init.bits,
        &init.fv,
        &init.fdef,
        goal_pos,
        goal_num,
    )
    .is_none()
    {
        return PlanResult::Unsolvable {
            evaluated: 1,
            capped: false,
        };
    }

    // Classical landmark-count term (0.11 Phase 3, `w_lm` only): landmarks
    // for THIS (start, goal) pair, accepted-bitsets per node.
    let clms: Vec<u32> = if cfg.w_lm > 0 {
        crate::landmarks::landmarks_for(task, start, goal_pos)
    } else {
        Vec::new()
    };
    let reslm: Option<&crate::resource::TripBound> = if cfg.w_res > 0 {
        RESLM.get().and_then(|o| o.as_ref())
    } else {
        None
    };
    let clm_words = clms.len().div_ceil(64);
    let mut root_clm = vec![0u64; clm_words];
    if !clms.is_empty() {
        clm_accept_into(&mut root_clm, &clms, &init);
    }
    let mut nodes: Vec<Node> = vec![Node {
        state: init.clone(),
        father: usize::MAX,
        op: usize::MAX,
        g: 0,
        lm_acc: root_clm,
    }];
    // Deferred evaluation: a node's priority is set from its PARENT's h at
    // insertion; its own h is computed only when it is popped. Many inserted
    // nodes are never popped, so far fewer heuristic evaluations are done.
    // The visited key excludes irrelevant fluents (termination); under
    // branch-and-bound it also appends the cost fluent so equal-fact/different-cost
    // states stay distinct (see PackedTask::state_key_with_cost).
    let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
    heap.push(Reverse((0, 0))); // init popped first
                                // Retained-state compression (0.20 Phase 4): visited is hash -> node
                                // indices; equality is checked EXACTLY against the arena state, so
                                // nothing stores a second copy of the bitset (the old StateKey set
                                // held bits + vals per entry — words*8 bytes of pure duplication on
                                // every inserted node). Dedup verdicts and expansion order are
                                // byte-identical: the hash only routes to candidates, the exact
                                // `state_key_eq` decides.
    let mut visited: FxHashMap<u64, Vec<u32>> = FxHashMap::default();
    visited.insert(task.state_key_hash(&init, cost_fluent), vec![0]);

    let mut evaluated = 0usize;
    let mut best = i32::MAX;
    let mut advance: Vec<i32> = Vec::new();
    let mut max_g = 0usize;
    // Anytime in-sweep tightening (`cfg.anytime`, metric B&B loops only): the
    // bound tightens in place on every acceptance and the sweep keeps going —
    // `best_acc` holds the incumbent's node index (nodes is append-only, so
    // reconstruction stays valid at return time).
    let mut cost_bound = cost_bound;
    let mut best_acc: Option<usize> = None;
    // Length-anytime (cfg.len_anytime): incumbent + live length bound + drain
    // ceiling (see the SearchCfg field docs).
    let mut len_bound: usize = usize::MAX;
    let mut len_acc: Option<usize> = None;
    let mut eval_ceiling: usize = cfg.max_eval;

    while !heap.is_empty() {
        // pop a batch of lowest-priority nodes
        let mut popped: Vec<usize> = Vec::with_capacity(batch);
        for _ in 0..batch {
            match heap.pop() {
                Some(Reverse((_, ni))) => popped.push(ni),
                None => break,
            }
        }

        // goal check (cheap, before any heuristic work). With `closure` set,
        // acceptance is METRIC-BOUNDED: the real goal must hold AND the exact
        // preference-closure completion must beat the incumbent bound — the
        // phase tail appended by the caller pays exactly `closure.cost`, so
        // this test accepts precisely the states that improve the metric.
        for &ni in &popped {
            max_g = max_g.max(nodes[ni].g);
            if !task.goal_met_with(&nodes[ni].state, goal_pos, goal_num) {
                continue;
            }
            if cfg.len_anytime {
                let g = nodes[ni].g;
                if g < len_bound {
                    len_bound = g;
                    len_acc = Some(ni);
                    // The drain may spend as much again as the first incumbent
                    // took; a later, shorter incumbent does not extend it.
                    if eval_ceiling == cfg.max_eval {
                        eval_ceiling = evaluated
                            .saturating_mul(2)
                            .max(evaluated + 10_000)
                            .min(cfg.max_eval);
                    }
                }
                continue;
            }
            if cfg.anytime {
                // In-sweep tightening: acceptance = effective plan cost
                // (cost-so-far + exact closure, or plain cost-so-far without a
                // closure) strictly beats the CURRENT bound. Accepting states
                // stay in the batch — a zero-cost extension can still satisfy
                // more preferences and improve again.
                let s = &nodes[ni].state;
                let g = cost_fluent
                    .map(|cf| if s.fdef[cf] { s.fv[cf] } else { 0.0 })
                    .unwrap_or(0.0);
                let eff = g + closure.map_or(0.0, |cl| cl.cost(s));
                if eff < cost_bound {
                    best_acc = Some(ni);
                    cost_bound = eff;
                    if eff <= 0.0 {
                        // Nothing can beat zero: the incumbent is optimal.
                        return PlanResult::Plan {
                            ops: reconstruct(&nodes, ni),
                            advance,
                            evaluated,
                            max_g,
                        };
                    }
                }
                continue;
            }
            if closure.map_or(true, |cl| {
                let s = &nodes[ni].state;
                let g = cost_fluent
                    .map(|cf| if s.fdef[cf] { s.fv[cf] } else { 0.0 })
                    .unwrap_or(0.0);
                g + cl.cost(s) < cost_bound
            }) {
                return PlanResult::Plan {
                    ops: reconstruct(&nodes, ni),
                    advance,
                    evaluated,
                    max_g,
                };
            }
        }

        // Length-anytime: a popped node at depth g spawns goals at >= g+1, so
        // g + 1 >= len_bound can never improve — drop before paying its h.
        if cfg.len_anytime && len_bound < usize::MAX {
            popped.retain(|&ni| nodes[ni].g + 1 < len_bound);
            if popped.is_empty() {
                continue;
            }
        }
        // Anytime: drop popped nodes the tightened bound has made dead — cost
        // is monotone and the closure is non-negative, so cost-so-far >= bound
        // can never reach an accepting state. Saves their h evaluations.
        if cfg.anytime {
            if let Some(cf) = cost_fluent {
                let bound_now = cost_bound;
                popped.retain(|&ni| {
                    let s = &nodes[ni].state;
                    !(s.fdef[cf] && s.fv[cf] >= bound_now)
                });
                if popped.is_empty() {
                    continue;
                }
            }
        }

        // PARALLEL: evaluate h for the popped batch (the only evaluations),
        // each worker reusing one Scratch across its chunk.
        let t_phase = crate::clock::Clock::now();
        let hs: Vec<Option<i32>> = par::par_map_with(
            &popped,
            threads,
            || Scratch::new(task),
            |sc, &ni| {
                let s = &nodes[ni].state;
                match cfg.h_cost {
                    Some(hcf) => {
                        relaxed_costed(task, sc, &s.bits, &s.fv, &s.fdef, goal_pos, goal_num, hcf)
                    }
                    None => relaxed_to(task, sc, &s.bits, &s.fv, &s.fdef, goal_pos, goal_num),
                }
            },
        );
        t_h += t_phase.elapsed_us();
        evaluated += popped.len();
        // The node cap (0.8 Phase 3) trips at the same batch boundary as the
        // eval cap: `nodes.len()` counts INSERTED successors — the quantity
        // that actually holds the memory — and is maintained serially, so the
        // check is thread-count independent. Overshoot is bounded by one
        // batch's insertions (the check precedes this batch's expansion).
        if evaluated > cfg.max_eval || evaluated > eval_ceiling || nodes.len() > node_cap {
            // Anytime: a capped sweep still hands back its incumbent — the
            // caller tightens to its cost and (with budget) sweeps again.
            if dbg {
                use std::sync::atomic::Ordering::Relaxed;
                eprintln!(
                    "[h] reset {}ms, build {}ms, extract {}ms (cumulative worker-thread time)",
                    crate::heuristic::T_RESET.load(Relaxed) / 1000,
                    crate::heuristic::T_BUILD.load(Relaxed) / 1000,
                    crate::heuristic::T_EXTRACT.load(Relaxed) / 1000,
                );
                eprintln!(
                    "[search] capped at {evaluated} evals: h {}ms, expand {}ms, insert {}ms, total {}ms",
                    t_h / 1000,
                    t_exp / 1000,
                    t_ins / 1000,
                    t_all.elapsed_ms()
                );
            }
            if let Some(ni) = best_acc.or(len_acc) {
                return PlanResult::Plan {
                    ops: reconstruct(&nodes, ni),
                    advance,
                    evaluated,
                    max_g,
                };
            }
            return PlanResult::Unsolvable {
                evaluated,
                capped: true,
            };
        }
        for h in hs.iter().flatten() {
            if *h < best {
                best = *h;
                advance.push(*h);
            }
        }

        // PARALLEL: expand non-dead-end popped nodes; successors carry the
        // parent's h as their (deferred) priority key.
        let live: Vec<(usize, i32)> = popped
            .iter()
            .zip(hs.iter())
            .filter_map(|(&ni, h)| h.map(|h| (ni, h)))
            .collect();
        let t_phase = crate::clock::Clock::now();
        let cand_chunks: Vec<Vec<(usize, usize, State, u64, i32)>> =
            par::par_map(&live, threads, |&(ni, ph)| {
                let st = &nodes[ni].state;
                let mut v = Vec::new();
                for oi in 0..task.n_ops {
                    if forbidden.get(oi).copied().unwrap_or(false) {
                        continue;
                    }
                    if task.op_applicable(oi, st) {
                        let ns = task.apply(oi, st);
                        if let Some(cf) = cost_fluent {
                            if ns.fdef[cf] && ns.fv[cf] >= cost_bound {
                                continue; // cost already >= bound: cannot beat incumbent
                            }
                        }
                        let k = task.state_key_hash(&ns, cost_fluent);
                        v.push((ni, oi, ns, k, ph));
                    }
                }
                v
            });

        t_exp += t_phase.elapsed_us();

        // SERIAL: dedup + insert (deterministic order, independent of threads).
        let t_phase = crate::clock::Clock::now();
        for chunk in cand_chunks {
            for (pi, oi, s, k, ph) in chunk {
                let g = nodes[pi].g + 1;
                if g >= cfg.g_bound || g >= len_bound {
                    continue; // cannot beat the length incumbent (see SearchCfg)
                }
                let bucket = visited.entry(k).or_default();
                if bucket
                    .iter()
                    .any(|&idx| task.state_key_eq(&nodes[idx as usize].state, &s, cost_fluent))
                {
                    continue;
                }
                bucket.push(nodes.len() as u32);
                {
                    // metric guidance: forgone-preference + renewable-resource
                    // occupancy penalty on the concrete successor (steers toward
                    // genuinely satisfying states that stay within resource pools).
                    let sat_pen = sat.map(|sg| sg.penalty(&s)).unwrap_or(0);
                    // metric-cost ordering (w_c, default 0.0 = exact zero term):
                    // single deterministic rounding, 1/256 cost resolution.
                    let cost_term = if cfg.w_c != 0.0 {
                        let c = cost_fluent
                            .map(|cf| if s.fdef[cf] { s.fv[cf] } else { 0.0 })
                            .unwrap_or(0.0);
                        (cfg.w_c * c * WEIGHT_SCALE).round() as i64
                    } else {
                        0
                    };
                    let res_term = reslm.map_or(0, |tb| cfg.w_res * tb.trips(&s.bits));
                    let lm_term = if clms.is_empty() {
                        0
                    } else {
                        let mut acc = nodes[pi].lm_acc.clone();
                        clm_accept_into(&mut acc, &clms, &s);
                        let un = clm_unaccepted(&acc, clms.len());
                        let idx = nodes.len();
                        nodes.push(Node {
                            state: s,
                            father: pi,
                            op: oi,
                            g,
                            lm_acc: acc,
                        });
                        heap.push(Reverse((
                            cfg.w_g * g as i64
                                + cfg.w_h * ph as i64
                                + cfg.w_lm * un
                                + res_term
                                + sat_pen
                                + cost_term,
                            idx,
                        )));
                        continue;
                    };
                    let idx = nodes.len();
                    nodes.push(Node {
                        state: s,
                        father: pi,
                        op: oi,
                        g,
                        lm_acc: Vec::new(),
                    });
                    heap.push(Reverse((
                        cfg.w_g * g as i64
                            + cfg.w_h * ph as i64
                            + lm_term
                            + res_term
                            + sat_pen
                            + cost_term,
                        idx,
                    )));
                }
            }
        }
        t_ins += t_phase.elapsed_us();
    }

    // Open list exhausted. Anytime: the incumbent is optimal under the original
    // bound (the caller's confirming re-sweep proves it via None + un-capped).
    if let Some(ni) = best_acc.or(len_acc) {
        return PlanResult::Plan {
            ops: reconstruct(&nodes, ni),
            advance,
            evaluated,
            max_g,
        };
    }
    PlanResult::Unsolvable {
        evaluated,
        capped: false,
    }
}

fn reconstruct(nodes: &[Node], mut ni: usize) -> Vec<usize> {
    let mut ops = Vec::new();
    while nodes[ni].father != usize::MAX {
        ops.push(nodes[ni].op);
        ni = nodes[ni].father;
    }
    ops.reverse();
    ops
}

/// Whole-task search (start = initial state, goal = task goal) with tunable
/// weighted-best-first parameters.
pub fn search(task: &PackedTask, threads: usize, cfg: SearchCfg) -> PlanResult {
    search_from(
        task,
        &task.initial(),
        &task.goal_pos,
        &task.goal_num,
        None,
        f64::INFINITY,
        threads,
        cfg,
        &[],
        None,
        None,
    )
}

/// Outcome of [`plan`]: the op sequence (if solved), states evaluated, and
/// whether EHC gave up and best-first took over.
pub struct PlanOutcome {
    pub ops: Option<Vec<usize>>,
    pub evaluated: usize,
    pub ehc_fell_back: bool,
}

/// Plan the whole task. With `ehc_first`, run enforced hill-climbing (fast on
/// most problems) and fall back to weighted best-first if it gets stuck;
/// otherwise run best-first directly. EHC plans are valid but not length-optimal
/// — this matches the FF/Metric-FF default and is the main speed lever.
pub fn plan(task: &PackedTask, threads: usize, cfg: SearchCfg, ehc_first: bool) -> PlanOutcome {
    plan_avoiding(task, threads, cfg, ehc_first, &[])
}

/// Like [`plan`], but never uses any op `oi` where `forbidden[oi]` is true. Used
/// by the metric optimizer's force-collect tightening (forbid forgo actions to
/// force their preferences to actually be satisfied).
pub fn plan_avoiding(
    task: &PackedTask,
    threads: usize,
    cfg: SearchCfg,
    ehc_first: bool,
    forbidden: &[bool],
) -> PlanOutcome {
    // Budget-aware ladder (0.18 Phase 4, the novelty referee's next idea):
    // `FF_TIME_LIMIT=<secs>` tells the ladder its REAL wall budget (the
    // runner passes its per-instance timeout). A bounded rung is a bet
    // that costs wall time before the complete fallback gets its shot —
    // the 0.17 referee measured that tax at −51 budget-edge instances for
    // the novelty rung. With the wall budget known, a bounded rung is
    // entered only while MORE THAN 40% of it remains: early failures
    // still buy the rungs' wins, late ones stop starving the fallback.
    // No limit set → all-rungs behavior, byte-identical to before.
    // (`map_or`, not `is_none_or`: the latter is stable only since 1.82,
    // above the crate's 1.74 MSRV.)
    let rungs_affordable = wall_remaining_frac().map_or(true, |f| f > 0.4);
    // The probe eyes (FF_ORBIT_DEBUG's pattern): narrate the gate's
    // verdict on stderr, never affect the search.
    if std::env::var("FF_WALL_DEBUG").is_ok() {
        eprintln!(
            "wall: remaining {:?}, bounded rungs {}",
            wall_remaining_frac(),
            if rungs_affordable {
                "affordable"
            } else {
                "SKIPPED"
            }
        );
    }
    // Probe hatch (A/B eyes for the novelty rung): skip straight to it.
    if std::env::var("FF_NOVELTY_ONLY").is_ok() {
        if let Some((ops, evaluated)) =
            crate::novelty::search(task, threads, cfg.max_eval, forbidden)
        {
            return PlanOutcome {
                ops: Some(ops),
                evaluated,
                ehc_fell_back: true,
            };
        }
    }
    // Probe hatch for the LIGHT novelty rung (0.20 Phase 3).
    if std::env::var("FF_NOVLIGHT_ONLY").is_ok() {
        if let Some((ops, evaluated)) = crate::novelty::search_light(task, cfg.max_eval, forbidden)
        {
            return PlanOutcome {
                ops: Some(ops),
                evaluated,
                ehc_fell_back: true,
            };
        }
    }
    if ehc_first {
        if let Some((ops, evaluated)) = ehc(task, forbidden, cfg.max_eval) {
            return PlanOutcome {
                ops: Some(ops),
                evaluated,
                ehc_fell_back: false,
            };
        }
        // Novelty-LIGHT rung (0.20 Phase 3): IW(1) + goal count, ZERO h
        // evaluations — the width-y shape EHC just died on (its BFS
        // lookahead pays hFF per state) is exactly where structural
        // novelty alone finishes in milliseconds (the visit-all witness:
        // 35 s of the h-guided rung's wall was ALL heuristic calls). A
        // pop here costs successor generation and a bitset OR, so the
        // rung's tax ahead of LAMA is small by construction. Same
        // budget-gate family as the h-guided rung below: default-on
        // under a declared affordable budget, `FF_NOVLIGHT=1` forces,
        // `FF_NO_NOVLIGHT=1` opts out.
        let novlight_on = std::env::var("FF_NOVLIGHT").is_ok()
            || (wall_remaining_frac().is_some() && std::env::var("FF_NO_NOVLIGHT").is_err());
        if novlight_on && rungs_affordable {
            // 300k pops: the width-y wins need plan-length pops (the
            // visit-all-2014 receipts: 899/3135/3248 — two orders below
            // this), while a hopeless domain's tax stays ~1 s (the
            // sokoban probe priced 2M pops at 7 s of wall).
            const NOVLIGHT_CAP: usize = 300_000;
            if let Some((ops, evaluated)) =
                crate::novelty::search_light(task, NOVLIGHT_CAP.min(cfg.max_eval), forbidden)
            {
                return PlanOutcome {
                    ops: Some(ops),
                    evaluated,
                    ehc_fell_back: true,
                };
            }
        }
        // LAMA rung (0.9 Phase 3): EHC gave up, i.e. the relaxed plan has
        // plateaued — exactly where landmark counting + preferred-operator
        // boosting keep a gradient. Bounded, so the complete weighted
        // fallback below still gets its shot; never entered under an
        // explicit --search bfs. `FF_NO_LAMA=1` restores the two-rung ladder.
        if std::env::var("FF_NO_LAMA").is_err() && rungs_affordable {
            const LAMA_CAP: usize = 400_000;
            if let Some((ops, evaluated)) =
                crate::lama::search(task, threads, LAMA_CAP.min(cfg.max_eval), forbidden)
            {
                return PlanOutcome {
                    ops: Some(ops),
                    evaluated,
                    ehc_fell_back: true,
                };
            }
        }
        // Novelty rung (0.17 Phase 3): width-1 novelty-first exploration
        // for where the relaxed gradient is flat or wrong. The 0.17
        // referee flipped it off-by-default (+7/−51: the rung's wall-time
        // tax ahead of the complete fallback), but the 0.18 budget gate
        // REVERSED that arithmetic — with FF_TIME_LIMIT declared the
        // gated rung measured +4/−0 (termes, organic-synthesis-split,
        // quantum-layout). So (0.19 Phase 5): the rung is DEFAULT-ON
        // exactly when a wall budget is declared and affordable —
        // `FF_NO_NOVELTY=1` opts out, `FF_NOVELTY=1` still forces it
        // without a budget, and with no FF_TIME_LIMIT set the ladder is
        // byte-identical to 0.17's.
        let novelty_on = std::env::var("FF_NOVELTY").is_ok()
            || (wall_remaining_frac().is_some() && std::env::var("FF_NO_NOVELTY").is_err());
        if novelty_on && rungs_affordable {
            const NOVELTY_CAP: usize = 400_000;
            if let Some((ops, evaluated)) =
                crate::novelty::search(task, threads, NOVELTY_CAP.min(cfg.max_eval), forbidden)
            {
                return PlanOutcome {
                    ops: Some(ops),
                    evaluated,
                    ehc_fell_back: true,
                };
            }
        }
    }
    // Length-anytime on the PLAIN length path only (no metric machinery in
    // play). Opt-in — see the SearchCfg field docs for the measured verdict.
    let mut cfg = cfg;
    if cfg.h_cost.is_none() && !cfg.anytime && std::env::var("FF_LEN_ANYTIME").is_ok() {
        cfg.len_anytime = true;
    }
    // Classical landmark-count ordering (0.11 Phase 3), opt-in experiment:
    // FF_CLM=<weight> adds w_lm × unaccepted-landmarks to the best-first
    // fallback's key (EHC and the LAMA rung are untouched).
    if cfg.h_cost.is_none() && !cfg.anytime {
        if let Ok(v) = std::env::var("FF_CLM") {
            let w = v.trim().parse::<f64>().unwrap_or(3.0);
            if w.is_finite() && w > 0.0 {
                cfg.w_lm = (w.min(1e9) * WEIGHT_SCALE).round() as i64;
            }
        }
        // Resource-trip term (0.14 ext Phase 11), same scoping.
        if let Ok(v) = std::env::var("FF_RESLM") {
            let w = v.trim().parse::<f64>().unwrap_or(3.0);
            if w.is_finite() && w > 0.0 {
                cfg.w_res = (w.min(1e9) * WEIGHT_SCALE).round() as i64;
            }
        }
    }
    // The refill loop (0.20 Phase 1: spend the whole wall). A CAPPED
    // fallback — eval cap or the node cap's memory model, not genuine
    // exhaustion — used to return unsolved with wall budget still on the
    // table (the tpp-numeric witness: rungs 27 s, fallback capped at
    // 7.8 s, 25 s of a 60 s wall handed back unspent). With a wall
    // budget declared and >10% of it remaining, re-enter GREEDIER
    // instead: w_h ×4 (deeper for the same node count — the memory
    // bound is untouched) and max_eval ×4, at most REFILL_MAX_ROUNDS
    // re-entries (escalation saturates; re-running a saturated config
    // is deterministic waste). Genuine exhaustion (`capped: false`)
    // returns immediately — completeness is weight-independent, so a
    // re-run cannot help. An EXPLICIT eval cap (api max_evaluated ≠
    // default) is a budgeted-think contract and disarms the loop.
    // No budget declared → single round, byte-identical to 0.19.
    // `FF_NO_REFILL=1` is the discriminator hatch.
    const REFILL_MAX_ROUNDS: usize = 6;
    let refill_armed = cfg.max_eval == DEFAULT_MAX_EVAL && std::env::var("FF_NO_REFILL").is_err();
    let mut round_cfg = cfg;
    let mut round = 0usize;
    let mut total_evaluated = 0usize;
    loop {
        match search_from(
            task,
            &task.initial(),
            &task.goal_pos,
            &task.goal_num,
            None,
            f64::INFINITY,
            threads,
            round_cfg,
            forbidden,
            None,
            None,
        ) {
            PlanResult::Plan { ops, evaluated, .. } => {
                return PlanOutcome {
                    ops: Some(ops),
                    evaluated: total_evaluated + evaluated,
                    ehc_fell_back: ehc_first,
                };
            }
            PlanResult::Unsolvable { evaluated, capped } => {
                total_evaluated += evaluated;
                let wall_ok = wall_remaining_frac().is_some_and(|f| f > 0.10);
                if !(capped && refill_armed && wall_ok && round < REFILL_MAX_ROUNDS) {
                    return PlanOutcome {
                        ops: None,
                        evaluated: total_evaluated,
                        ehc_fell_back: ehc_first,
                    };
                }
                round += 1;
                round_cfg.w_h = round_cfg
                    .w_h
                    .saturating_mul(4)
                    .min((1e9 * WEIGHT_SCALE) as i64);
                round_cfg.max_eval = round_cfg.max_eval.saturating_mul(4);
                if std::env::var("FF_WALL_DEBUG").is_ok() {
                    eprintln!(
                        "wall: refill round {} (w_h {}, max_eval {})",
                        round + 1,
                        round_cfg.w_h as f64 / WEIGHT_SCALE,
                        round_cfg.max_eval
                    );
                }
            }
        }
    }
}

/// Enforced hill-climbing toward the task goal. From the current state, run a
/// breadth-first lookahead restricted to HELPFUL actions until a strictly
/// lower-h state is found, then jump to it and repeat. Returns the plan + states
/// evaluated, or None if it gets stuck / hits a dead end (caller falls back to
/// best-first, which is complete). Single-threaded and deterministic.
fn ehc(task: &PackedTask, forbidden: &[bool], max_eval: usize) -> Option<(Vec<usize>, usize)> {
    let init = task.initial();
    let mut sc = Scratch::new(task);
    let (mut cur_h, _) = relaxed_helpful(
        task,
        &mut sc,
        &init.bits,
        &init.fv,
        &init.fdef,
        &task.goal_pos,
        &task.goal_num,
    )?;
    let mut evaluated = 1usize;
    if task.goal_met(&init) {
        return Some((Vec::new(), evaluated));
    }
    // Total work budget: if EHC hasn't solved it within this many evaluations it
    // is likely stuck, so bail and leave the time budget to the complete
    // best-first fallback (which often solves these much faster from scratch).
    // Scaled by op count: EHC's cumulative evals grow ~quadratically in problem
    // size, so a fixed 30k cap made large-but-easy instances (e.g. gripper) bail
    // into the unpruned best-first and explode (2.16M evals). Scaling with n_ops
    // lets EHC's near-greedy arm finish those, while the 30k floor keeps small/
    // medium domains bit-identical and the finite cap still hands genuine
    // plateaus off to the complete fallback.
    // The caller's eval budget bounds EHC too (0.11 Phase 4: a think budget
    // bounds EVERYTHING — a 1-eval think must not solve via EHC's internal
    // op-scaled cap).
    let total_cap = (200 * task.n_ops).max(30_000).min(max_eval);
    let mut current = init;
    let mut plan: Vec<usize> = Vec::new();
    loop {
        match bfs_improve(task, &mut sc, &current, cur_h, &mut evaluated, forbidden) {
            Some((ops, next, next_h)) => {
                plan.extend(ops);
                current = next;
                cur_h = next_h;
                if task.goal_met(&current) {
                    return Some((plan, evaluated));
                }
                if evaluated > total_cap {
                    return None; // taking too long — hand off to best-first
                }
            }
            None => return None, // stuck — let best-first take over
        }
    }
}

/// Breadth-first search from `start`, expanding each node with ITS helpful
/// actions, until a state with `h < h_start` is found. Returns (path, state, h).
fn bfs_improve(
    task: &PackedTask,
    sc: &mut Scratch,
    start: &State,
    h_start: i32,
    evaluated: &mut usize,
    forbidden: &[bool],
) -> Option<(Vec<usize>, State, i32)> {
    // Fail FAST: if a helpful-restricted lookahead can't improve h within this
    // many expansions it is almost certainly on a plateau EHC won't escape, so
    // bail and let the complete best-first fallback use the time budget. Kept
    // small because per-evaluation cost is high on big numeric tasks — a large
    // cap made EHC burn ~20s before falling back.
    const BFS_CAP: usize = 5_000;
    struct N {
        state: State,
        father: usize,
        op: usize,
    }
    let (_, root_helpful) = relaxed_helpful(
        task,
        sc,
        &start.bits,
        &start.fv,
        &start.fdef,
        &task.goal_pos,
        &task.goal_num,
    )?;
    let mut nodes = vec![N {
        state: start.clone(),
        father: usize::MAX,
        op: usize::MAX,
    }];
    let mut visited: FxHashSet<StateKey> = FxHashSet::default();
    visited.insert(task.state_key(start));
    let mut queue: VecDeque<(usize, Vec<u32>)> = VecDeque::new();
    queue.push_back((0, root_helpful));
    let mut expanded = 0usize;

    while let Some((ni, helpful)) = queue.pop_front() {
        for &oi in &helpful {
            let oi = oi as usize;
            if forbidden.get(oi).copied().unwrap_or(false) {
                continue;
            }
            if !task.op_applicable(oi, &nodes[ni].state) {
                continue;
            }
            let ns = task.apply(oi, &nodes[ni].state);
            if !visited.insert(task.state_key(&ns)) {
                continue;
            }
            *evaluated += 1;
            let (h_ns, helpful_ns) = match relaxed_helpful(
                task,
                sc,
                &ns.bits,
                &ns.fv,
                &ns.fdef,
                &task.goal_pos,
                &task.goal_num,
            ) {
                Some(x) => x,
                None => continue, // dead-end successor
            };
            let idx = nodes.len();
            nodes.push(N {
                state: ns.clone(),
                father: ni,
                op: oi,
            });
            if h_ns < h_start {
                let mut ops = Vec::new();
                let mut c = idx;
                while nodes[c].father != usize::MAX {
                    ops.push(nodes[c].op);
                    c = nodes[c].father;
                }
                ops.reverse();
                return Some((ops, ns, h_ns));
            }
            expanded += 1;
            if expanded > BFS_CAP {
                return None;
            }
            queue.push_back((idx, helpful_ns));
        }
    }
    None
}

/// Subplanner API: return the op sequence achieving `(goal_pos, goal_num)` from
/// `start`, or None if unsolvable. This is what `sgp` calls per partition.
pub fn solve_subgoal(
    task: &PackedTask,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    threads: usize,
    cfg: SearchCfg,
) -> Option<Vec<usize>> {
    solve_subgoal_avoiding(task, start, goal_pos, goal_num, &[], threads, cfg)
}

/// `solve_subgoal` but never using any op `oi` where `forbidden[oi]` is true —
/// the resolver's sibling-protection lever (forbid ops that delete an already-
/// achieved sibling's facts). An empty mask forbids nothing.
#[allow(clippy::too_many_arguments)]
pub fn solve_subgoal_avoiding(
    task: &PackedTask,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    forbidden: &[bool],
    threads: usize,
    cfg: SearchCfg,
) -> Option<Vec<usize>> {
    match search_from(
        task,
        start,
        goal_pos,
        goal_num,
        None,
        f64::INFINITY,
        threads,
        cfg,
        forbidden,
        None,
        None,
    ) {
        PlanResult::Plan { ops, .. } => Some(ops),
        PlanResult::Unsolvable { .. } => None,
    }
}

/// Subplanner with a monotone COST upper bound on `cost_fluent`: returns a plan
/// reaching the goal whose final cost is < `bound`, or None if none exists under
/// the bound. The anytime branch-and-bound metric optimizer (sgp) calls this
/// with a tightening bound.
#[allow(clippy::too_many_arguments)]
pub fn solve_subgoal_bounded(
    task: &PackedTask,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    cost_fluent: usize,
    bound: f64,
    threads: usize,
    cfg: SearchCfg,
    sat: Option<&SatGuidance>,
) -> (Option<Vec<usize>>, usize, bool) {
    match search_from(
        task,
        start,
        goal_pos,
        goal_num,
        Some(cost_fluent),
        bound,
        threads,
        cfg,
        &[],
        sat,
        None,
    ) {
        PlanResult::Plan { ops, evaluated, .. } => (Some(ops), evaluated, false),
        PlanResult::Unsolvable { evaluated, capped } => (None, evaluated, capped),
    }
}

/// The exact-closure metric subplanner (`crate::pddl3::metric_optimize`'s
/// default path): search REAL states only — `forbidden` masks every synthetic
/// `P3END`/collect/forgo op — accepting a state iff the real goal holds AND
/// `cost-so-far + closure(state) < bound` (the caller appends the phase tail,
/// which pays exactly `closure`). Returns the plan (sans tail) plus the number
/// of states evaluated, so the caller can run a DETERMINISTIC eval-count
/// budget across tightening iterations, and the capped flag (bound proven
/// unbeatable iff un-capped exhaustion).
#[allow(clippy::too_many_arguments)]
pub fn solve_closure_bounded(
    task: &PackedTask,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    cost_fluent: usize,
    bound: f64,
    closure: &ClosureCost,
    forbidden: &[bool],
    threads: usize,
    cfg: SearchCfg,
    sat: Option<&SatGuidance>,
) -> (Option<Vec<usize>>, usize, bool) {
    match search_from(
        task,
        &task.initial(),
        goal_pos,
        goal_num,
        Some(cost_fluent),
        bound,
        threads,
        cfg,
        forbidden,
        sat,
        Some(closure),
    ) {
        PlanResult::Plan { ops, evaluated, .. } => (Some(ops), evaluated, false),
        PlanResult::Unsolvable { evaluated, capped } => (None, evaluated, capped),
    }
}

/// [`solve_subgoal_avoiding`] + [`SatGuidance`]: the partitioned-ESPC per-stage
/// subplanner (`crate::espc`). Combines a forbidden-op mask (sibling protection)
/// with the λ-weighted penalty guidance — `search_from` supports both, but no
/// other wrapper exposes the combination. No cost bound: on the openstacks shape
/// the metric only accrues in the post-composition collect/forgo tail, so a
/// per-stage bound could never prune; bounds stay global (composed-plan cost vs
/// the incumbent).
#[allow(clippy::too_many_arguments)]
pub fn solve_subgoal_guided(
    task: &PackedTask,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    forbidden: &[bool],
    threads: usize,
    cfg: SearchCfg,
    sat: Option<&SatGuidance>,
) -> (Option<Vec<usize>>, usize) {
    match search_from(
        task,
        start,
        goal_pos,
        goal_num,
        None,
        f64::INFINITY,
        threads,
        cfg,
        forbidden,
        sat,
        None,
    ) {
        PlanResult::Plan { ops, evaluated, .. } => (Some(ops), evaluated),
        PlanResult::Unsolvable { evaluated, .. } => (None, evaluated),
    }
}
