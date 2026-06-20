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
- **PDDL3 preferences** with anytime branch-and-bound metric optimization;
- an optional **SGPlan-style partition-and-resolve** mode.

It is offered as a Rust **library** (with a structured, JSON-serializable API)
and the **`ff`** command-line binary, a drop-in for Metric-FF.
