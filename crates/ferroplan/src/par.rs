//! Minimal data-parallel primitives over `std::thread::scope` (no external deps).
//! The thread count comes from `FFDP_THREADS` (if set) else available cores;
//! `threads <= 1` falls back to a plain sequential map, so behaviour is
//! deterministic regardless of parallelism.
//!
//! Two cost-control knobs keep parallelism a net win across problem sizes:
//!  - `MIN_PAR`: rounds with fewer than this many items run SERIALLY, so a tiny
//!    frontier (the common case on small problems, and the tail of large ones)
//!    never pays thread-spawn cost. Output is identical either way.
//!  - `MAX_DEFAULT_THREADS`: the auto thread count is capped, because measured
//!    scaling plateaus by ~4 cores (Amdahl: serial successor-gen/dedup/heap);
//!    spawning one thread per core per round past that is pure overhead. An
//!    explicit `FFDP_THREADS` is honoured uncapped.

/// Below this item count a round runs serially (spawning isn't worth it).
pub const MIN_PAR: usize = 32;
/// Cap on the auto-selected worker count (scaling plateaus ~4; small margin).
const MAX_DEFAULT_THREADS: usize = 6;

/// Resolve the worker count: `FFDP_THREADS` env override (uncapped), else
/// `min(cores, MAX_DEFAULT_THREADS)`.
pub fn num_threads() -> usize {
    if let Ok(s) = std::env::var("FFDP_THREADS") {
        if let Ok(n) = s.parse::<usize>() {
            if n >= 1 {
                return n;
            }
        }
    }
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(MAX_DEFAULT_THREADS)
}

/// Map `f` over `items` across `threads` scoped workers, preserving input order.
/// Each worker owns a contiguous chunk; results are concatenated in order, so
/// the output is identical to a sequential map (only the cost is parallel).
pub fn par_map<T, R, F>(items: &[T], threads: usize, f: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync,
{
    let n = items.len();
    if threads <= 1 || n < MIN_PAR {
        return items.iter().map(&f).collect();
    }
    let t = threads.min(n);
    let chunk = n.div_ceil(t);
    let chunks: Vec<&[T]> = items.chunks(chunk).collect();
    let f = &f;
    let mut parts: Vec<Vec<R>> = Vec::with_capacity(chunks.len());
    std::thread::scope(|scope| {
        let handles: Vec<_> = chunks
            .iter()
            .map(|c| scope.spawn(move || c.iter().map(f).collect::<Vec<R>>()))
            .collect();
        for h in handles {
            parts.push(h.join().expect("worker thread panicked"));
        }
    });
    parts.into_iter().flatten().collect()
}

/// Like `par_map`, but each worker first builds a private `state` via `init`
/// (e.g. reusable scratch buffers) and threads it through `f`, so per-item
/// allocation is amortised across a chunk. Output order matches input.
pub fn par_map_with<T, R, S, I, F>(items: &[T], threads: usize, init: I, f: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    I: Fn() -> S + Sync,
    F: Fn(&mut S, &T) -> R + Sync,
{
    let n = items.len();
    if threads <= 1 || n < MIN_PAR {
        let mut s = init();
        return items.iter().map(|x| f(&mut s, x)).collect();
    }
    let t = threads.min(n);
    let chunk = n.div_ceil(t);
    let chunks: Vec<&[T]> = items.chunks(chunk).collect();
    let init = &init;
    let f = &f;
    let mut parts: Vec<Vec<R>> = Vec::with_capacity(chunks.len());
    std::thread::scope(|scope| {
        let handles: Vec<_> = chunks
            .iter()
            .map(|c| {
                scope.spawn(move || {
                    let mut s = init();
                    c.iter().map(|x| f(&mut s, x)).collect::<Vec<R>>()
                })
            })
            .collect();
        for h in handles {
            parts.push(h.join().expect("worker thread panicked"));
        }
    });
    parts.into_iter().flatten().collect()
}
