#!/usr/bin/env python3
"""Big-catalog stress fixture: the village's grounding profile, priced
before the village is built (0.17 Phase 1; docs/landscape-2026.md gap #3).

ABSTRACT RULES ONLY, exactly the game's contract: ONE gather rule and ONE
make rule; the catalog (which items exist, what each recipe consumes) is
pure INIT DATA via static predicates. The generated recipe graph is a
layered binary DAG: layer-0 items are raw (gatherable); every higher item
is made from two items one layer down; the goal is the single top item.

The engine question is scaling shape: `make` is syntactically N^3 ground
instances (?out ?a ?b over N items) — a grounder that resolves the static
needs1/needs2 predicates grounds ~N ops (one per recipe); one that
doesn't, dies cubically. Solve difficulty itself is trivial-by-design
(the DAG plan is the derivation); this fixture measures GROUNDING, the
dimension the village actually threatens.

  python3 benchmarks/bench/gen_catalog.py N [--consume] > problem-dir/
    writes catalog-N/{domain.pddl,problem.pddl}

--consume makes inputs consumable (delete (have ?a)/(have ?b)) and the
goal still one top item: each item is then needed multiply and the plan
re-gathers — a mild search dimension on top of the same grounding shape.
"""

import os
import sys


def gen(n, consume, outdir):
    layers = []
    items = [f"i{k}" for k in range(n)]
    # layer sizes halve; layer 0 gets the remainder (raw items)
    sizes = []
    rest = n
    while rest > 1:
        top = max(1, rest // 3)
        sizes.append(top)
        rest -= top
    sizes.append(rest)
    sizes.reverse()  # sizes[0] = raw layer (largest)
    idx = 0
    for s in sizes:
        layers.append(items[idx:idx + s])
        idx += s
    raw = layers[0]
    recipes = []  # (out, in1, in2)
    for li in range(1, len(layers)):
        below = layers[li - 1]
        for j, out in enumerate(layers[li]):
            recipes.append((out, below[j % len(below)],
                            below[(j + 1) % len(below)]))
    top_item = layers[-1][0]

    eff = ["(have ?out)"]
    if consume:
        eff += ["(not (have ?a))", "(not (have ?b))"]
    domain = f"""(define (domain catalog)
  (:requirements :strips :typing)
  (:types item)
  (:predicates (have ?i - item) (raw ?i - item)
               (needs1 ?out - item ?in - item)
               (needs2 ?out - item ?in - item))
  (:action gather
    :parameters (?i - item)
    :precondition (raw ?i)
    :effect (have ?i))
  (:action make
    :parameters (?out - item ?a - item ?b - item)
    :precondition (and (needs1 ?out ?a) (needs2 ?out ?b)
                       (have ?a) (have ?b))
    :effect (and {' '.join(eff)})))
"""
    init = [f"(raw {i})" for i in raw]
    init += [f"(needs1 {o} {a})" for o, a, _ in recipes]
    init += [f"(needs2 {o} {b})" for o, _, b in recipes]
    problem = f"""(define (problem catalog-{n}) (:domain catalog)
  (:objects {' '.join(items)} - item)
  (:init {' '.join(init)})
  (:goal (have {top_item})))
"""
    os.makedirs(outdir, exist_ok=True)
    open(os.path.join(outdir, "domain.pddl"), "w").write(domain)
    open(os.path.join(outdir, "problem.pddl"), "w").write(problem)
    print(f"catalog-{n}: {len(layers)} layers, {len(raw)} raw, "
          f"{len(recipes)} recipes -> {outdir}")


if __name__ == "__main__":
    n = int(sys.argv[1])
    consume = "--consume" in sys.argv
    out = sys.argv[sys.argv.index("--out") + 1] if "--out" in sys.argv \
        else f"catalog-{n}{'-consume' if consume else ''}"
    gen(n, consume, out)
