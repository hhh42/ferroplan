# Architecture

ferroplan is **data-oriented**: states are bitsets of fact ids plus dense fluent
vectors; operators are stored column-wise (CSR) so the hot loops stream
contiguous memory and parallelize over immutable shared task data.

Pipeline:

1. **Parse** (`parser`, `lexer`) — PDDL domain + problem to an AST.
2. **Ground** (`ground`) — parallel per-action binding enumeration, DNF of
   preconditions, ADL expansion (`forall`/`exists`/`when`), negative-precondition
   compilation, relaxed-reachability pruning, CSR packing.
3. **Search** (`search`, `heuristic`) — weighted best-first (`1·g + 5·h`) with a
   delete-relaxation relaxed-plan heuristic; deferred (lazy) heuristic
   evaluation; parallel batch evaluation with order-preserving determinism.
4. **Modes** — classic FF; SGPlan-style `partition`+`resolve`; PDDL3
   `pddl3` (Keyder–Geffner soft-goal compilation + anytime branch-and-bound);
   the decision-epoch `temporal` search (snap-action compilation, pending-end
   agenda, symmetry-reduced since 0.13); a budget-aware sequential
   `portfolio`; and the game-embedding [`Session`](./session.md) (ground
   once, think forever — with a fixpoint grounding path whose transient
   memory is up to ~117× smaller on sparse-reachable worlds).

Performance notes: an in-tree FxHash hasher, a compact relevant-only visited
key, and size-gated parallelism (serial for small frontiers, capped threads)
keep both small and large problems fast.
