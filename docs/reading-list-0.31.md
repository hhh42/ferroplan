# Reading list — 0.31 (fetched 2026-10-23)

The papers behind `docs/roadmap-0.31.md`, grouped by the lane they inform.
PDFs are in `docs/papers/` (open-access copies: AAAI/ICAPS/JAIR OJS, arXiv,
author and institutional pages, the IPC 2023 abstracts site). Two journal
versions are paywalled and not vendored; their conference versions are,
and the DOI is given. Read the starred items first — they are the ones a
lane's mechanism is written from.

## Lane A — the ladder learns its slices

- ★ `helmert-etal-2011-stone-soup.pdf` — Helmert, Röger, Karpas (with
  Seipp). *Fast Downward Stone Soup: A Baseline for Building Planner
  Portfolios.* IPC 2011 planner abstract. The greedy coverage-per-second
  slice allocation; three pages. The whole of Lane A's allocator.
- `roeger-etal-2014-stone-soup.pdf` — Röger, Pommerening, Seipp. *Fast
  Downward Stone Soup 2014.* IPC 2014 abstract — the same recipe three
  years on, with the lessons.
- ★ `seipp-etal-2015-cedalion.pdf` — Seipp, Sievers, Helmert, Hutter.
  *Automatic Configuration of Sequential Planning Portfolios.* AAAI 2015.
  Cedalion: append the (config, slice) pair that maximizes improvement per
  second; the leave-one-out discipline Lane A's held-out receipt copies.
- `ipc2023-fdss.pdf` — FDSS 2023 abstract (Stone Soup, current form).

## Lane B — BFWS done the winners' way

- ★ `lipovetzky-geffner-2017-bfws.pdf` — Lipovetzky, Geffner. *Best-First
  Width Search: Exploration and Exploitation in Classical Planning.* AAAI
  2017. BFWS(f5): novelty relative to ⟨#g, #r⟩, open list ⟨w, #g, h⟩. The
  configuration Lane B builds.
- ★ `frances-etal-2017-bfws-simulators.pdf` — Francès, Ramírez,
  Lipovetzky, Geffner. *Purely Declarative Action Descriptions are
  Overrated: Classical Planning with Simulators.* IJCAI 2017. k-BFWS
  (polynomial pruned passes) and the dual BFWS schedule.
- `katz-etal-2017-adapting-novelty.pdf` — Katz, Lipovetzky, Moshkovich,
  Tuisov. *Adapting Novelty to Classical Planning.* ICAPS 2017. Novelty
  inside an FD-style heuristic search (the shape our ladder has); the
  "quantified novelty" variants.
- `singh-etal-2021-approximate-novelty.pdf` — Singh, Lipovetzky, Ramírez,
  Segovia-Aguas. *Approximate Novelty Search.* (arXiv 2105.07691 / ICAPS
  2021.) Bloom-filter novelty tables: how w=2 stays affordable.
- ★ `ipc2023-scorpion-maidu.pdf` — Corrêa, Francès, Pommerening, Seipp
  et al. *Scorpion Maidu* IPC 2023 abstract (satisficing + agile winner):
  width search with forgetting inside Scorpion.
- `ipc2023-levitron.pdf` — Levitron abstract (Maidu + PowerLifted
  portfolio; the other 2023 winner).
- `ipc2023-apx-novelty-fd.pdf` — Approximate Novelty FD, IPC 2023.
- `ipc2023-dalai.pdf` — DALAI (disjunctive action landmarks), IPC 2023.
- `richter-westphal-2010-lama.pdf` — Richter, Westphal. *The LAMA Planner.*
  JAIR 2010 (arXiv copy). Reference for the rung we already have, and
  for its anytime loop (Lane E).

## Lane C — the optimal half

- ★ `seipp-helmert-2018-cegar-jair.pdf` — Seipp, Helmert.
  *Counterexample-Guided Cartesian Abstraction Refinement for Classical
  Planning.* JAIR 62, 2018. Cartesian abstractions, and the first
  statement of saturated cost partitioning.
- ★ `seipp-etal-2020-scp-jair.pdf` — Seipp, Keller, Helmert. *Saturated
  Cost Partitioning for Optimal Classical Planning.* JAIR 67, 2020. SCP
  in full: orders, diversification, what to sum. C.1's core.
- `seipp-helmert-2019-subset-scp.pdf` — *Subset-Saturated Cost
  Partitioning.* ICAPS 2019. The tighter variant; read after the JAIR.
- ★ `haslum-etal-2007-ipdb.pdf` — Haslum, Botea, Helmert, Bonet, Koenig.
  *Domain-Independent Construction of Pattern Database Heuristics for
  Cost-Optimal Planning.* AAAI 2007. iPDB: hill-climbing pattern
  selection. The first pattern selector C.1 builds.
- `rovner-etal-2019-cegar-patterns.pdf` — Rovner, Sievers, Helmert.
  *CEGAR for Pattern Selection.* ICAPS 2019. The faster selector, once
  iPDB is in.
- `ipc2023-scorpion.pdf` — Scorpion 2023 abstract (2nd, optimal).
- `ipc2023-ragnarok.pdf` — Ragnarok abstract (1st, optimal): the
  portfolio of SCP abstractions + symbolic search that the lane's two
  halves are drawn from.
- `ipc2023-complementary-pdb.pdf`, `ipc2023-cegarplusplus.pdf` — the
  other two abstraction entries, for the pattern-selection variants.
- ★ `speck-etal-2025-symbolic-jair.pdf` — Speck et al., JAIR 2025 (the
  current symbolic-search reference from the SymK group; read for C.2
  instead of the paywalled AIJ 2017).
- `ipc2023-symk.pdf`, `ipc2023-symbd.pdf` — SymK and SymBD abstracts:
  symbolic bidirectional search as entered in 2023.
- NOT VENDORED (paywalled): Torralba, Alcázar, Kissmann, Edelkamp.
  *Efficient symbolic search for cost-optimal planning.* AIJ 242, 2017.
  doi:10.1016/j.artint.2016.10.001 — the image-computation and
  invariant-pruning study; its domain table is C.2's band source.
- ★ `kuroiwa-etal-2021-nlmcut-icaps.pdf` — Kuroiwa, Shleyfman,
  Piacentini, Castro, Beck. *LM-cut and Operator Counting Heuristics for
  Optimal Numeric Planning with Simple Conditions.* ICAPS 2021. C.3.
- `kuroiwa-etal-2022-nlmcut-jair.pdf` — *The LM-Cut Heuristic Family for
  Optimal Numeric Planning with Simple Conditions.* JAIR 2022 — the long
  version.

## Lane D — red-black relaxation

- ★ `katz-etal-2013-who-said-relax-all.pdf` — Katz, Hoffmann, Domshlak.
  *Who Said We Need to Relax All Variables?* ICAPS 2013. Red-black
  planning defined; the tractable fragments.
- ★ `katz-etal-2013-red-black-relaxed-plan.pdf` — Katz, Hoffmann,
  Domshlak. *Red-Black Relaxed Plan Heuristics.* AAAI 2013. The heuristic
  itself; the invertibility-based painting Lane D starts from.
- `katz-hoffmann-2014-red-black-reloaded-socs.pdf` — *Red-Black Relaxed
  Plan Heuristics Reloaded.* SoCS 2013. The practical fixes.
- `icaps14-hsdip-proceedings.pdf` — ICAPS 2014 HSDIP workshop
  proceedings; contains Katz & Hoffmann, *Pushing the Limits of Partial
  Delete Relaxation: Red-Black DAG Heuristics* (the Mercury heuristic;
  Mercury was IPC 2014 seq-sat runner-up).
- NOT VENDORED (paywalled): Domshlak, Hoffmann, Katz. *Red–black
  planning: A new systematic approach to partial delete relaxation.* AIJ
  221, 2015. doi:10.1016/j.artint.2014.12.008 — the journal synthesis of
  the three above.

## Lane E — preference points

- ★ `keyder-geffner-2009-soft-goals.pdf` — Keyder, Geffner. *Soft Goals
  Can Be Compiled Away.* JAIR 36, 2009 (arXiv copy). The compile.
- ★ `richter-etal-2010-joy-of-forgetting.pdf` — Richter, Thayer, Ruml.
  *The Joy of Forgetting: Faster Anytime Search via Restarting.* ICAPS
  2010. RWA*.

## Probes — constraints, lifted search

- `baier-etal-2009-tep-aij.pdf` — Baier, Bacchus, McIlraith. *A
  heuristic search approach to planning with temporally extended
  preferences.* AIJ 173, 2009 (author copy). Automata compile +
  progress-aware heuristics; the constraints probe's mechanism.
- `baier-etal-2006-hplanp-ipc5.pdf` — HPlan-P's IPC-5 abstract.
- `correa-etal-2020-powerlifted.pdf` — Corrêa, Pommerening, Helmert,
  Francès. *Lifted Successor Generation using Query Optimization
  Techniques.* ICAPS 2020. PowerLifted.
- `ipc2023-powerlifted.pdf` — its 2023 entry.

## How to add to this list

Fetch with `curl -sL -A 'Mozilla/5.0' -o docs/papers/<author-year-slug>.pdf
<url>` and check `head -c 5` reads `%PDF` before keeping it; prefer the
OJS `download` URL, arXiv, or the author's page over aggregators. Add a
line here under the lane it informs, with the full citation and the one
sentence that says why.
