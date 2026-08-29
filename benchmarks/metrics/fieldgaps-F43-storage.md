# F4.3 — storage-tc i8–i10: why the at-end fold stalls (0.26)

Executed 2026-08-29 ~07:40, the smallest probe of
`docs/field-gaps-execution-0.26.md` §F4.3, solo on a quiet box. Binary:
`target/release/ff` = the 0.26.0 candidate (F1 on; the constraints compile
and the temporal path are untouched since 0.25.0). Receipts:
`benchmarks/metrics/probes-0.26/F43-storage/` (json + stderr under
`FF_WALL_DEBUG=1 FF_RES_DEBUG=1`, 60 s wall, `FF_MEM_BUDGET_GB=6`). The
crate-ablation twin lives beside the receipts as
`i8-minus-crate7.twin.pddl` — never in the corpus directory.

## 1. i7 vs i8 (the one-crate cliff)

| run | outcome | temporal pass | nodes | time |
|---|---|---|---|---|
| i7 (7 crates) | **solved**, makespan 24.0 | prune=true, 82 words, 5,729 ops | (solved before the first 25k-node tick) | ~8 s |
| i8 (8 crates) | unsolved, "temporal ladder stopped at the wall" | prune=true, 120 words, 7,985 ops | **6,078 nodes in 42.7 s** — then the pass's wall slice expired | pass 2 (prune=false) opened with no wall left: 1 node at 25 ms |
| i8, `FF_NO_TRAJ_END=1` | unsolved, "temporal ladder exhausted its budgets with 58 s of wall left" | no temporal pass reached | — | ~2 s |

Two readings that the dossier's three hypotheses did not include, and one
they did:

- **The `budget left 18446744073709542244` in the cap line is not an
  underflow.** The pass ran UNBOUNDED on nodes (`usize::MAX`, decremented
  with `saturating_sub`); it printed `MAX − 9,372` because 9,372
  prototypes were charged. What ended the pass is the wall slice
  (`deadline_expired_reserving`), and the second pass then inherited a
  dead wall. Recorded so nobody chases a phantom.
- **i8 is a per-node-COST wall, not a decision-epoch cliff.** 6,078 nodes
  in 42.7 s is ~7 ms per node — the search never had the chance to plateau;
  it could barely move. Hypothesis (a) ("a plain decision-epoch cliff at 9
  crates × 5 depots", by count) does not fit a search that expanded six
  thousand states; what scales one crate → one cliff is the cost of each
  temporal evaluation over 7,985 ops and 120-word states with the at-end
  latch folded in.
- **`FF_NO_TRAJ_END=1` does exactly what the dossier predicted:** the
  re-opened goal-side DNF product chokes the ladder in 2 s ("exhausted its
  budgets with 58 s of wall left"). The END construction is not the stall;
  it is the thing that makes the instance attemptable at all.

## 2. The crate-ablation twin (8 → 7 crates on i8's own layout)

The first twin was invalid — the generator deleted the whole `:objects`
block (one paren group with no nested parens, and it contains the word
`crate7`), and a task with no objects "solves" in zero steps — recorded so
its 0.0-makespan row is never read as a result. The corrected twin (i8 with
`crate7` removed from `:objects`, its 8 atoms dropped, the `forall`
constraint untouched — `i8-minus-crate7.twin.pddl` beside the receipts):

| run | outcome | temporal pass | nodes |
|---|---|---|---|
| i8 − crate7 (7 crates on i8's 4 depots / 4 hoists / 2 containers) | **unsolved**, "temporal ladder stopped at the wall" | prune=true, 100 words, 6,929 ops | **953 nodes in 20.2 s (~21 ms each)**, then the slice expired |

**The layout, not the count.** Seven crates on i7's layout solve in 8 s;
seven crates on i8's layout expand 953 nodes in 20 s and die — each node
costs three times an i8 node. The cliff is the per-evaluation cost of this
depot/hoist arrangement under the at-end latch, and hypothesis (a) — a
smooth decision-epoch scaling wall by crate count, to be routed to the §1d
storage-time decode — is refuted by its own twin. What is left is local:
(b) the TRAJ-END lowering's interaction with the temporal search's per-node
work, or (c) the latch's charge flattening across depots — both per-node-
cost shapes, which is what the numbers show; this probe cannot separate
them without instrumenting the temporal evaluation itself.

## 3. Verdict

**Not routed away; not built either.** F4.3's smallest probe rules OUT the
easy answer (a smooth crate-count wall that belongs to the storage-time
decode) and rules IN a local per-node-cost mechanism on i8's layout under
the at-end fold — 7–21 ms per temporal node against i7's sub-second
budget. The build the spec sketched (heuristic- or lowering-side, hatch
`FF_NO_ACCFOLD`) needs one more measurement before it has a target: the
per-evaluation time split of the temporal fold on i8 versus the twin versus
i7 (the `[h]`-style phase attribution the classical search prints, which
the temporal pass does not yet). That instrument is the next step and is a
build of its own (small); until it names (b) or (c), F4.3 stays OPEN with
its +3 unpriced, and the storage-tc lead over SGPlan (15/30 vs 9/30) stands
as it is.
