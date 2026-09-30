# Lane 2 probe — is 0.28.0 slower on the same search? (2026-09-30)

Phase 0.3 left coins-2026 i18 as Lane 2's first fixture with one question
(docs/roadmap-0.29.md): 0.28.0 fails it 0/3 solo at 60 s where v0.27.1 solves
it 3/3 at 54 s, and no lane hatch restores it -- is 0.28.0 slower on the same
search, or searching differently?

Protocol: solo, quiet box, one thread, `FF_TIME_LIMIT=20 FF_MEM_BUDGET_GB=6
FF_WALL_DEBUG=1 FF_RES_DEBUG=1`, stderr timestamped (`coins18-20s-*.timeline`);
an 8 s `sample` of the best-first phase on the two cut binaries
(`coins18-sample-top-*.txt`, top self-time frames); then the fixed binary solo
at 60 s, three reps.

| binary | round-1 evals in 20 s | h ms | us/eval | refill round 2 fires |
|---|---|---|---|---|
| OLD  v0.27.1, the crucible's build (Sep 12)        | 1,716,551 | 13,247 | 7.72  | yes |
| REB  v0.27.1 rebuilt 2026-09-30, rustc 1.98.1      | 1,720,491 | 13,236 | 7.69  | yes |
| NEW  0.28.0 [89cfdc5f06ed]                          | 1,204,680 | 13,300 | 11.04 | no  |
| FIX  0.28.0 + `#[inline(always)] fn widen`          | 1,556,672 | 13,267 | 8.52  | yes |
| FIX + `FF_NO_NEED_DIRS=1`                           | 1,563,209 | 13,334 | 8.53  | yes |

Same search: all four run the identical trajectory (novelty-light 326,257
nodes, novelty-driver 431,768 nodes) -- the heuristic is value-identical, each
relaxed-graph build just costs 43% more. Not the toolchain (REB reproduces
OLD; stable was last updated Sep 4, before either cut binary was built). Not
a hatch. Not the per-evaluation deadline reads (`mach_absolute_time` samples
equal in both profiles). The profiles name it: `heuristic::widen` is the top
self-time frame in NEW and absent from OLD; `nm` shows the symbol only in the
0.28.0 binary. Lane N's `needs: Option<&NeedDirs>` parameter tipped LLVM
(thin LTO, one codegen unit) into keeping `widen` an out-of-line call, and the
numeric RPG calls it once per applied op per layer.

Fixed binary, coins i18 solo at 60 s, three reps: solved 3/3 at 54.76 /
55.02 / 55.00 s, 5,749,216 evaluations each (v0.27.1: 3/3 at 54 s). A
wall-edge cell: it banks only on a quiet box.

Left alone, on the record: the residual 8.5 vs 7.7 us (+10%); and both
profiles spend ~2% in `__findenv_locked` from `relaxed_to_inner`, which reads
up to six `FF_NUMPRE_*` variables per evaluation -- the unit tests set those
in-process, so a cache needs the tests reworked first.
