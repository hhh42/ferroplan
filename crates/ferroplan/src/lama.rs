//! LAMA-style satisficing rung (0.9 roadmap Phase 3): greedy best-first over
//! TWO signals — the FF relaxed-plan heuristic and a path-dependent
//! **landmark count** ([`crate::landmarks`]) — with **preferred-operator**
//! boosting via a dual open list (successors reached by a parent's helpful
//! action sit in a second, favored heap; LAMA's core recipe).
//!
//! Why a separate rung: EHC + plain weighted best-first (the FF lineage) die
//! exactly where the relaxed plan plateaus — long goal-interaction chains
//! (parking, floortile, barman, tidybot). Landmarks not yet achieved on the
//! path keep a progress gradient across those plateaus, and helpful-action
//! boosting keeps the branching factor near the relaxed plan's. This rung
//! runs BOUNDED, after EHC gives up and before the complete weighted
//! fallback, so it can only add coverage — `FF_NO_LAMA=1` removes it, and
//! explicit `--search bfs` never enters it.
//!
//! Determinism: fixed batch sizes popped from each heap, order-preserving
//! parallel h evaluation, serial insertion — the plan is identical at any
//! thread count (same contract as `search_from`).

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::hash::FxHashMap;
use crate::heuristic::{relaxed_helpful, Scratch};
use crate::packed::{PackedTask, State};
use crate::par;

/// Popped per round from the preferred heap (boosted) and the normal heap.
const PREF_BATCH: usize = 192;
const NORM_BATCH: usize = 64;
/// FF-h weight vs landmark-count weight in the (greedy) priority key.
const W_FF: i64 = 2;
const W_LM: i64 = 4;

/// One expansion candidate: (parent idx, op, successor state, visited key,
/// parent's FF h, reached via a helpful op).
type Cand = (usize, usize, State, u64, i32, bool);

struct Node {
    state: State,
    father: usize,
    op: usize,
    /// Landmarks accepted on the path to this node (bitset over the
    /// landmark LIST index, not fact ids).
    accepted: Vec<u64>,
}

fn accept_into(accepted: &mut [u64], lms: &[u32], state: &State) {
    for (i, &f) in lms.iter().enumerate() {
        if accepted[i >> 6] & (1 << (i & 63)) == 0 && crate::bitset::test(&state.bits, f as usize) {
            accepted[i >> 6] |= 1 << (i & 63);
        }
    }
}

fn unaccepted(accepted: &[u64], n: usize) -> i64 {
    n as i64 - accepted.iter().map(|w| w.count_ones() as i64).sum::<i64>()
}

/// Bounded landmark/preferred greedy search toward the task goal. Returns the
/// plan ops and states evaluated, or None (dead end, cap, or node cap).
///
/// `slice` (0.22 Phase 5A a1): the rung's armed wall deadline — (start
/// clock, seconds) — checked at the batch boundary, where the wall is
/// actually spent (each batch's h evaluations dominate). The ladder
/// passes `FF_LAMA_WALL_FRAC` (default 0.25) of the REMAINING wall;
/// the portfolio and the partition cascade pass `None` (their budget
/// discipline is their own). `None` ⇒ unchecked ⇒ byte-identical.
pub fn search(
    task: &PackedTask,
    threads: usize,
    max_eval: usize,
    forbidden: &[bool],
    slice: Option<(crate::clock::Clock, f64)>,
) -> Option<(Vec<usize>, usize)> {
    let init = task.initial();
    // Length-anytime on the whole-task rung only (subgoal probes return on
    // first goal — a cascade merge wants speed, not polish). Opt-in; see
    // SearchCfg::len_anytime for the measured default-off verdict.
    let len_anytime = std::env::var("FF_LEN_ANYTIME").is_ok();
    search_subgoal(
        task,
        &init,
        &task.goal_pos,
        &task.goal_num,
        threads,
        max_eval,
        forbidden,
        len_anytime,
        slice,
    )
}

/// [`search`] generalized over a start state and subgoal — the form the
/// partition cascade (`resolve::solve`) needs: landmarks are recomputed for
/// exactly this (start, subgoal) pair, so the count stays a sound
/// remaining-necessary-work signal for the piece being solved.
#[allow(clippy::too_many_arguments)]
pub fn search_subgoal(
    task: &PackedTask,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[crate::types::NumPre],
    threads: usize,
    max_eval: usize,
    forbidden: &[bool],
    len_anytime: bool,
    slice: Option<(crate::clock::Clock, f64)>,
) -> Option<(Vec<usize>, usize)> {
    let lms = crate::landmarks::landmarks_for(task, start, goal_pos);
    let lm_words = lms.len().div_ceil(64);
    let node_cap = crate::search::node_cap_for(task);
    // The MEASURED memory wall for a bounded rung (0.30 Lane 5): the
    // modelled cap above under-charges this rung's nodes (`accepted` per
    // generated successor), and city-car i6 reached the runner's 6 GB at
    // 20 s here. One kernel read per batch; a trip hands down like any
    // exhausted budget.
    let mem_wall = crate::mem::MemWall::arm_rung();

    let init = start.clone();
    let mut accepted0 = vec![0u64; lm_words];
    accept_into(&mut accepted0, &lms, &init);
    let mut nodes = vec![Node {
        state: init.clone(),
        father: usize::MAX,
        op: usize::MAX,
        accepted: accepted0,
    }];
    if task.goal_met_with(&init, goal_pos, goal_num) {
        return Some((Vec::new(), 0));
    }

    // THE LAZY FRONTIER (0.30 Lane 5, after the novelty driver's): `nodes`
    // holds the POPPED states only. An open entry is (key, father, op) on
    // a heap -- on both heaps when the op was helpful, so the normal
    // queue's completeness is untouched -- and its state is regenerated
    // from the father's when it is popped. `keys` maps a state-key hash to
    // the closed index of that state, or `OPEN` while it is only on a
    // heap: a hash seen at generation is a duplicate (the eager shape
    // compared stored states exactly, 0.20 Phase 4), and a hash already
    // closed at pop time is the other heap's copy of the same entry (the
    // eager `expanded` flag). On city-car i6 this rung held the process's
    // peak -- 3.2 GB at 10 s, 3.4 GB when its slice ended at 13 s -- in
    // generated successors it never evaluated.
    const OPEN: u32 = u32::MAX;
    let mut pref_heap: BinaryHeap<Reverse<(i64, u32, u32)>> = BinaryHeap::new();
    let mut norm_heap: BinaryHeap<Reverse<(i64, u32, u32)>> = BinaryHeap::new();
    // The root is already closed at 0: `op == u32::MAX` marks it.
    norm_heap.push(Reverse((0, 0, u32::MAX)));
    let mut keys: FxHashMap<u64, u32> = FxHashMap::default();
    keys.insert(task.state_key_hash(&init, None), 0);
    let mut evaluated = 0usize;

    /// Take up to `n` entries off `heap` into `popped`, regenerating each
    /// open entry's state and closing it; an entry whose state the other
    /// heap already closed is skipped, as the eager `expanded` flag skipped
    /// it. Counts attempts, as the eager loop did.
    fn pop_lazy(
        heap: &mut BinaryHeap<Reverse<(i64, u32, u32)>>,
        n: usize,
        task: &PackedTask,
        lms: &[u32],
        nodes: &mut Vec<Node>,
        keys: &mut FxHashMap<u64, u32>,
        popped: &mut Vec<usize>,
    ) {
        for _ in 0..n {
            let Some(Reverse((_, father, op))) = heap.pop() else {
                break;
            };
            if op == u32::MAX {
                popped.push(father as usize);
                continue;
            }
            let f = father as usize;
            let ns = task.apply(op as usize, &nodes[f].state);
            let k = task.state_key_hash(&ns, None);
            match keys.get_mut(&k) {
                Some(slot) if *slot == OPEN => {
                    let mut accepted = nodes[f].accepted.clone();
                    accept_into(&mut accepted, lms, &ns);
                    *slot = nodes.len() as u32;
                    nodes.push(Node {
                        state: ns,
                        father: f,
                        op: op as usize,
                        accepted,
                    });
                    popped.push(nodes.len() - 1);
                }
                _ => continue,
            }
        }
    }
    // Length-anytime incumbent (see search.rs SearchCfg::len_anytime): keep
    // draining the same dual open lists for a strictly shorter plan until the
    // drain ceiling (evals-at-first-incumbent × 2).
    let mut best_plan: Option<Vec<usize>> = None;
    let mut best_len = usize::MAX;
    let mut eval_ceiling = max_eval;
    // The slice is PROGRESS-CONDITIONAL (0.22 Phase 5A a1): the EHC
    // precedent pairs its slice with a progress exit ("no improving
    // state"), and a flat deadline here trades named receipts against
    // each other — hiking-2014 i6's board solve is 43 s of STEADY
    // landmark progress inside LAMA (no fraction of a 60 s wall covers
    // it), while tetris i4's board loss is 400k evals of NO progress
    // eating the clock. The ladder's hard wall checkpoint still
    // backstops the extensions.
    //
    // The ARRIVAL test (0.24 Phase 6, OPT-IN `FF_LAMA_EXT_ARRIVAL=1`;
    // the default stays 0.22's recency rule — the honest negative,
    // receipts below): the 0.22 shape extends whenever SOME improvement
    // landed within the last half-tranche — and a running minimum in a
    // fresh search improves almost every batch, so a descending-but-
    // never-arriving h reads as perpetual progress ("progress 0.00 s
    // ago", nurikabe i12: the slice ate the wall to remaining 0.0 and
    // the novelty slot was SKIPPED). The 0.24 sitting measured both
    // named shapes solo and the whole drop-TREND family died on the
    // receipts: nurikabe drips STEADILY (window key-drops 128 → 34 →
    // 26 → … , six recency extensions to wall zero, never arriving)
    // while spider p01 COLLAPSES (268 → 74 → 58 → 46) and then
    // CONVERTS — any trend rule that keeps the canary keeps the
    // wall-eater more. The one local discriminator is ARRIVAL — extend
    // only while the demonstrated pace can still reach key zero inside
    // the remaining wall (the heap's own W_FF·h + W_LM·lm currency,
    // EXT_OPTIMISM slack) — and under it nurikabe hands down at ~23 s
    // with the ladder's wall intact (the 0.23 casualty class re-opened)
    // and hiking i6 still solves in-slice. But spider p01's conversion
    // NEEDS the wall-eating shape: it arrives on a cliff after ~5
    // windows of drip, demanding optimism ≥ 5.4 at its window four,
    // while nurikabe re-extends at ≥ 5.67 one window earlier — the
    // constants CROSS, no factor separates them to conversion, and the
    // roadmap's canary ("the fix must discriminate, not amputate")
    // outranks the fix. So: DEFAULT recency (byte-identical to 0.22,
    // spider keeps its solve), arrival behind the flag with fixtures
    // on both sides, and the swap's true price (a freed ladder on the
    // nurikabe class vs the spider class's conversions) is a
    // board-scale question only a sweep A/B can answer.
    let mut deadline = slice.map(|(_, s)| s);
    let tranche = slice.map_or(0.0, |(_, s)| s);
    let (mut best_ph, mut best_hlm) = (i64::MAX, i64::MAX);
    let mut last_improve_s = 0.0f64;
    let mut improved = false;
    let arrival = std::env::var("FF_LAMA_EXT_ARRIVAL").is_ok();
    // Window baselines form at each signal's FIRST finite value (not at
    // slice start — i64::MAX is not a drop), then advance per extension.
    let (mut win_ph, mut win_lm) = (i64::MAX, i64::MAX);
    // The key level over whichever signals are finite (the snapshot and
    // the running best share finiteness by construction — the snapshot
    // fills the moment a signal turns finite).
    let key_of = |ph: i64, lm: i64| -> Option<i64> {
        match (ph != i64::MAX, lm != i64::MAX) {
            (true, true) => Some(W_FF * ph + W_LM * lm),
            (true, false) => Some(W_FF * ph),
            (false, true) => Some(W_LM * lm),
            (false, false) => None,
        }
    };

    loop {
        // Deterministic mixed batch: boosted share from the preferred heap,
        // the rest from the normal one.
        let mut popped: Vec<usize> = Vec::with_capacity(PREF_BATCH + NORM_BATCH);
        pop_lazy(
            &mut pref_heap,
            PREF_BATCH,
            task,
            &lms,
            &mut nodes,
            &mut keys,
            &mut popped,
        );
        pop_lazy(
            &mut norm_heap,
            NORM_BATCH,
            task,
            &lms,
            &mut nodes,
            &mut keys,
            &mut popped,
        );
        if popped.is_empty() {
            // both open lists exhausted (with an incumbent: it is final)
            return best_plan.map(|p| (p, evaluated));
        }

        for &ni in &popped {
            if task.goal_met_with(&nodes[ni].state, goal_pos, goal_num) {
                if !len_anytime {
                    return Some((reconstruct(&nodes, ni), evaluated));
                }
                let plan = reconstruct(&nodes, ni);
                if plan.len() < best_len {
                    best_len = plan.len();
                    best_plan = Some(plan);
                    if eval_ceiling == max_eval {
                        eval_ceiling = evaluated
                            .saturating_mul(2)
                            .max(evaluated + 10_000)
                            .min(max_eval);
                    }
                }
            }
        }

        // PARALLEL: FF h + helpful set per popped node (the only evaluations).
        let hs: Vec<Option<(i32, Vec<u32>)>> = par::par_map_with(
            &popped,
            threads,
            || Scratch::new(task),
            |sc, &ni| {
                let s = &nodes[ni].state;
                relaxed_helpful(task, sc, &s.bits, &s.fv, &s.fdef, goal_pos, goal_num)
            },
        );
        evaluated += popped.len();
        // The node cap counts what is HELD: the closed states and the open
        // entries on the normal heap (every open entry is there), as the
        // eager shape's `nodes.len()` did.
        if evaluated > max_eval
            || evaluated > eval_ceiling
            || nodes.len() + norm_heap.len() > node_cap
        {
            // budget spent: the incumbent (if any), else hand off to the fallback
            if std::env::var("FF_RES_DEBUG").is_ok() {
                eprintln!(
                    "[lama] capped: {evaluated} evals (max {max_eval}), {} nodes ({} closed)",
                    nodes.len() + norm_heap.len(),
                    nodes.len()
                );
            }
            return best_plan.map(|p| (p, evaluated));
        }
        if mem_wall.hit() {
            if std::env::var("FF_WALL_DEBUG").is_ok() {
                eprintln!(
                    "wall: LAMA MEMORY checkpoint at {evaluated} evals, {} nodes ({} closed), handing down the ladder",
                    nodes.len() + norm_heap.len(),
                    nodes.len()
                );
            }
            return best_plan.map(|p| (p, evaluated));
        }
        // The wall slice (0.22 Phase 5A a1) + the hard checkpoint (Phase 2
        // lever 1), both at the batch boundary where the h evaluations
        // spend the wall. A trip hands down like any exhausted budget —
        // the incumbent (if any) is still a plan, never discarded.
        if let Some((t0, _)) = &slice {
            let now = t0.elapsed_secs();
            if improved {
                last_improve_s = now;
                improved = false;
            }
            // Baselines form at the first finite reading of each signal
            // (see the tracker docs above) so window one measures a real
            // drop, not a descent from i64::MAX.
            if win_ph == i64::MAX {
                win_ph = best_ph;
            }
            if win_lm == i64::MAX {
                win_lm = best_hlm;
            }
            if let Some(d) = deadline.as_mut() {
                if now > *d {
                    let key_now = key_of(best_ph, best_hlm);
                    let key_drop = match (key_of(win_ph, win_lm), key_now) {
                        (Some(prev), Some(cur)) => Some(prev - cur),
                        _ => None,
                    };
                    // Half-tranches the process wall can still afford —
                    // the arrival horizon the demonstrated pace must fit.
                    let affordable = (crate::search::wall_remaining_secs().unwrap_or(f64::INFINITY)
                        / (0.5 * tranche).max(1e-9))
                    .min(1e6) as i64;
                    let extend = if arrival {
                        extend_window(key_now, key_drop, affordable)
                    } else {
                        // The 0.22 recency rule — the standing default.
                        now - last_improve_s < 0.5 * tranche
                    };
                    let rule = if arrival { "arrival" } else { "recency" };
                    if extend {
                        // Still earning: another half-tranche instead of
                        // handing down, and the window advances.
                        *d = now + 0.5 * tranche;
                        (win_ph, win_lm) = (best_ph, best_hlm);
                        if std::env::var("FF_WALL_DEBUG").is_ok() {
                            eprintln!(
                                "wall: LAMA slice extended to {:.2}s ({rule}; key {key_now:?} drop {key_drop:?} affordable {affordable})",
                                *d
                            );
                        }
                    } else {
                        if std::env::var("FF_WALL_DEBUG").is_ok() {
                            eprintln!(
                                "wall: LAMA slice exhausted ({rule} refused: key {key_now:?} \
                                 drop {key_drop:?} affordable {affordable}; {evaluated} evals in \
                                 {now:.2}s), handing down the ladder"
                            );
                        }
                        return best_plan.map(|p| (p, evaluated));
                    }
                }
            }
        }
        if crate::search::wall_hard_expired() {
            return best_plan.map(|p| (p, evaluated));
        }

        // PARALLEL: expand live nodes; preferred = successor via a helpful op.
        let chunks: Vec<Vec<Cand>> = {
            let live: Vec<(usize, i32, &Vec<u32>)> = popped
                .iter()
                .zip(hs.iter())
                .filter_map(|(&ni, h)| h.as_ref().map(|(h, help)| (ni, *h, help)))
                .collect();
            par::par_map(&live, threads, |&(ni, ph, helpful)| {
                let st = &nodes[ni].state;
                let mut v = Vec::new();
                let mut cands = Vec::new();
                task.applicable_ops(st, &mut cands);
                for &oi in &cands {
                    let oi = oi as usize;
                    if forbidden.get(oi).copied().unwrap_or(false) {
                        continue;
                    }
                    {
                        let ns = task.apply(oi, st);
                        let k = task.state_key_hash(&ns, None);
                        let pref = helpful.contains(&(oi as u32));
                        v.push((ni, oi, ns, k, ph, pref));
                    }
                }
                v
            })
        };

        // SERIAL: dedup + insert (deterministic).
        for chunk in chunks {
            for (pi, oi, s, k, ph, pref) in chunk {
                if keys.contains_key(&k) {
                    continue;
                }
                keys.insert(k, OPEN);
                {
                    // Transient: the successor's accepted set prices its
                    // key; the vector is rebuilt from the father's if the
                    // entry is popped.
                    let mut accepted = nodes[pi].accepted.clone();
                    accept_into(&mut accepted, &lms, &s);
                    // Landmark count is EXACT for the successor (cheap bit
                    // math); the FF term is deferred from the parent.
                    let h_lm = unaccepted(&accepted, lms.len());
                    // The slice's convergence signal (see the tracker
                    // docs at the loop head).
                    if h_lm < best_hlm {
                        best_hlm = h_lm;
                        improved = true;
                    }
                    if (ph as i64) < best_ph {
                        best_ph = ph as i64;
                        improved = true;
                    }
                    let key = W_FF * ph as i64 + W_LM * h_lm;
                    norm_heap.push(Reverse((key, pi as u32, oi as u32)));
                    if pref {
                        pref_heap.push(Reverse((key, pi as u32, oi as u32)));
                    }
                }
            }
        }
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

/// The arrival test's optimism factor: descent accelerates near the
/// goal, so the demonstrated pace under-forecasts. Priced on the two
/// measured window-two shapes — spider p01 (key 584, drop 58,
/// affordable 4) demands ≥ 2.52 to keep that extension; nurikabe i12
/// (key 496, drop 20, affordable 4) tolerates anything < 6.2 before
/// the wall-eater extends again. 4 separates the early windows with
/// margin both ways — but the LATER windows cross (spider's fourth
/// needs ≥ 5.4 while nurikabe's third re-extends at ≥ 5.67), which is
/// exactly why the arrival rule is opt-in rather than the default:
/// no constant carries spider all the way to its conversion without
/// re-feeding the wall-eater.
const EXT_OPTIMISM: i64 = 4;

/// The 5A extension verdict (0.24 Phase 6, the ARRIVAL test): extend
/// only while the pace this window demonstrated can still reach key
/// zero inside the wall that remains — `key_now ≤ EXT_OPTIMISM ·
/// key_drop · affordable_windows`. A stalled or regressing window
/// (`key_drop ≤ 0`) never extends, whatever the wall; a drip against a
/// key an order of magnitude above its pace hands down with the wall
/// still worth spending (nurikabe i12's measured shape); a collapsing
/// drop over a commensurate key keeps earning (spider p01's measured
/// shape — the do-not-give-back canary). `key_now = None` (no finite
/// signal after a full window) refuses.
fn extend_window(key_now: Option<i64>, key_drop: Option<i64>, affordable_windows: i64) -> bool {
    match (key_now, key_drop) {
        (Some(k), Some(d)) if d > 0 => {
            k <= d
                .saturating_mul(affordable_windows.max(0))
                .saturating_mul(EXT_OPTIMISM)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::extend_window;

    /// The RED shape, pinned at the MEASURED numbers (nurikabe i12's
    /// window two: key 496, drop 20, affordable 4): the drip cannot
    /// arrive and hands down with the wall still worth spending; the
    /// recency rule extended it six times to remaining 0.0 and the
    /// novelty slot was SKIPPED (the 0.23 docket receipt, reproduced
    /// solo during this build).
    #[test]
    fn drip_that_cannot_arrive_hands_down() {
        assert!(
            !extend_window(Some(496), Some(20), 4),
            "nurikabe's window-two shape is a wall-eater and must refuse"
        );
        assert!(
            !extend_window(Some(100), Some(0), 50),
            "stall never extends"
        );
        assert!(
            !extend_window(Some(100), Some(-8), 50),
            "regression never extends"
        );
        assert!(
            !extend_window(None, None, 50),
            "no finite signal, no verdict"
        );
        assert!(
            !extend_window(Some(100), Some(30), 0),
            "no affordable window left: the extension would not even fit"
        );
    }

    /// The GREEN side, pinned at the MEASURED numbers (spider p01's
    /// window two: key 584, drop 58, affordable 4): a collapsing drop
    /// over a commensurate key still fits its arrival and keeps the
    /// extension the conversion rides on.
    #[test]
    fn pace_that_fits_keeps_extending() {
        assert!(
            extend_window(Some(584), Some(58), 4),
            "spider's window-two shape must keep its extension"
        );
        assert!(
            extend_window(Some(100), Some(20), 3),
            "a drip that fits the horizon is convergence, not churn"
        );
        assert!(
            extend_window(Some(0), Some(1), 1),
            "key zero is arrival by definition"
        );
    }
}
