# Introduction

**ferroplan** is a fast, data-parallel [PDDL](https://en.wikipedia.org/wiki/Planning_Domain_Definition_Language)
planner in Rust — a from-scratch reimplementation of the FF planner family.

It combines:

- a **delete-relaxation FF heuristic** over a data-oriented task representation
  (bitset states, structure-of-arrays / CSR operator tables);
- **data parallelism** — parallel grounding and parallel batch heuristic
  evaluation, with bit-for-bit identical plans regardless of thread count;
- **ADL** (conditional effects, `forall`/`exists`, equality) and **numeric
  fluents**;
- **derived predicates / axioms** (`:derived`, static/stratified);
- **PDDL3 preferences** with anytime branch-and-bound metric optimization;
- **PDDL2.1 temporal** planning — durative actions with constant or
  parameter-dependent durations and required concurrency (see
  [Temporal planning](./temporal.md));
- an optional **SGPlan-style partition-and-resolve** mode.

It is offered as a Rust **library** (with a structured, JSON-serializable API)
and the **`ff`** command-line binary, a drop-in for Metric-FF.

## Acknowledgments

ferroplan owes an enormous debt to the planners it learns from. Above all
**SGPlan** (Chih-Wei Hsu and Benjamin W. Wah, University of Illinois), which has set
the standard in satisficing planning with preferences and temporal/resource
constraints for nearly two decades — coming even *close* to it on a slice of the
benchmarks is genuinely an honor, and a tribute to the depth and durability of that
team's research. And to Jörg Hoffmann's **FF / Metric-FF**, whose relaxed-plan
heuristic and enforced hill-climbing are this engine's backbone, and to **VAL**
(Derek Long & Maria Fox) for independent temporal-plan validation.
