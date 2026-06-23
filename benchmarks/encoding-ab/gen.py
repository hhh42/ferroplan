#!/usr/bin/env python3
"""Generate matched crafting domains+problems for the encoding A/B/C benchmark.

Three encodings of the SAME crafting content, so the only variable is encoding style:

  specific    one hardcoded :action per recipe              (action fan-out)
  data-table  one `craft` action gated by a static          (constants + data table
              (recipe ...) table; recipes/resources           "find the types")
              are :constants
  forall      one `craft ?rec` action quantifying over all  (fat ADL operator over
              resources via (need ?rec ?res)/(make ?rec ?res)  per-recipe qty functions)

Each emits in two modes:
  inst        instantaneous :action  -> classical FF path -> reports evaluated_states
  temporal    :durative-action       -> temporal mode     -> reports makespan

Content models (shared semantics across all encodings):
  chain     r0 -> r1 -> ... -> rK   (single-input recipes; knobs K depth, N goal qty)
  converge  balanced binary assembly tree of depth D (two-input recipes; the
            "(>=2 contributions converge)" case where the delete-relaxed heuristic
            goes flat -- see ../../examples/BORDERS.md)

Usage:
  gen.py domain  --encoding E --mode M --content C [--recipes K | --depth D]
  gen.py problem --encoding E --mode M --content C [--recipes K | --depth D] --qty N
  gen.py emit-corpora --out DIR    (writes the whole matched corpus tree)

All templates here are verified to solve against this `ff` (see proto/ and README.md).
"""
import argparse
import os
import sys

# --------------------------------------------------------------------------- #
# Content models: return (resources, recipes, raws, goal).
# A recipe is (name, [input_resource, ...], output_resource); every qty is 1.
# --------------------------------------------------------------------------- #

def chain_content(K):
    """r0 -> r1 -> ... -> rK. Recipe c_i consumes r_i, produces r_{i+1}."""
    resources = [f"r{i}" for i in range(K + 1)]
    recipes = [(f"c{i}", [f"r{i}"], f"r{i + 1}") for i in range(K)]
    raws = ["r0"]
    goal = f"r{K}"
    return resources, recipes, raws, goal


def converge_content(D):
    """Balanced binary assembly tree, depth D. Node i has children 2i+1, 2i+2;
    internal node i needs both children, produces itself. Root r0 is the goal,
    leaves are raws. Every recipe has exactly two inputs."""
    n = 2 ** (D + 1) - 1
    resources = [f"r{i}" for i in range(n)]
    recipes = []
    for i in range(n):
        c1, c2 = 2 * i + 1, 2 * i + 2
        if c2 < n:  # internal node -> a two-input recipe
            recipes.append((f"c{i}", [f"r{c1}", f"r{c2}"], f"r{i}"))
    raws = [f"r{i}" for i in range(n) if 2 * i + 1 >= n]  # leaves
    goal = "r0"
    return resources, recipes, raws, goal


def content_of(args):
    if args.content == "chain":
        return chain_content(args.recipes)
    return converge_content(args.depth)


def max_inputs(recipes):
    return max((len(ins) for _, ins, _ in recipes), default=1)

# --------------------------------------------------------------------------- #
# Domain emitters
# --------------------------------------------------------------------------- #

def dom_specific(resources, recipes, mode):
    L = ["(define (domain craft-specific)"]
    req = ":strips :numeric-fluents" if mode == "inst" else ":strips :durative-actions :numeric-fluents"
    L.append(f"  (:requirements {req})")
    L.append("  (:functions " + " ".join(f"(stock_{r})" for r in resources) + ")")
    for name, ins, out in recipes:
        if mode == "inst":
            L.append(f"  (:action {name} :parameters ()")
            L.append("    :precondition (and " + " ".join(f"(>= (stock_{i}) 1)" for i in ins) + ")")
            eff = " ".join(f"(decrease (stock_{i}) 1)" for i in ins) + f" (increase (stock_{out}) 1)"
            L.append(f"    :effect (and {eff}))")
        else:
            L.append(f"  (:durative-action {name} :parameters ()")
            L.append("    :duration (= ?duration 2)")
            L.append("    :condition (and " + " ".join(f"(at start (>= (stock_{i}) 1))" for i in ins) + ")")
            eff = " ".join(f"(at start (decrease (stock_{i}) 1))" for i in ins) + f" (at end (increase (stock_{out}) 1))"
            L.append(f"    :effect (and {eff}))")
    L.append(")")
    return "\n".join(L) + "\n"


def dom_datatable(resources, recipes, mode):
    arity = max_inputs(recipes)
    pred = "recipe" if arity == 1 else f"recipe{arity}"
    in_params = " ".join(f"?in{k}" for k in range(arity))
    in_types = " ".join(f"?in{k} - resource" for k in range(arity))
    L = ["(define (domain craft-data-table)"]
    req = ":strips :typing :numeric-fluents" if mode == "inst" else ":strips :typing :durative-actions :numeric-fluents"
    L.append(f"  (:requirements {req})")
    L.append("  (:types recipe resource - object)")
    L.append("  (:constants " + " ".join(n for n, _, _ in recipes) + " - recipe   "
             + " ".join(resources) + " - resource)")
    L.append(f"  (:predicates ({pred} ?rec - recipe {in_types} ?out - resource))")
    L.append("  (:functions (stock ?res - resource))")
    if mode == "inst":
        L.append(f"  (:action craft :parameters (?rec - recipe {in_types} ?out - resource)")
        L.append(f"    :precondition (and ({pred} ?rec {in_params} ?out) "
                 + " ".join(f"(>= (stock ?in{k}) 1)" for k in range(arity)) + ")")
        L.append("    :effect (and " + " ".join(f"(decrease (stock ?in{k}) 1)" for k in range(arity))
                 + " (increase (stock ?out) 1)))")
    else:
        L.append(f"  (:durative-action craft :parameters (?rec - recipe {in_types} ?out - resource)")
        L.append("    :duration (= ?duration 2)")
        L.append(f"    :condition (and (at start ({pred} ?rec {in_params} ?out)) "
                 + " ".join(f"(at start (>= (stock ?in{k}) 1))" for k in range(arity)) + ")")
        L.append("    :effect (and " + " ".join(f"(at start (decrease (stock ?in{k}) 1))" for k in range(arity))
                 + " (at end (increase (stock ?out) 1))))")
    L.append(")")
    return "\n".join(L) + "\n"


def dom_forall(resources, recipes, mode):
    L = ["(define (domain craft-forall)"]
    if mode == "inst":
        req = ":typing :numeric-fluents :universal-preconditions :conditional-effects"
    else:
        req = ":typing :durative-actions :numeric-fluents :universal-preconditions :conditional-effects"
    L.append(f"  (:requirements {req})")
    L.append("  (:types recipe resource - object)")
    L.append("  (:constants " + " ".join(n for n, _, _ in recipes) + " - recipe   "
             + " ".join(resources) + " - resource)")
    L.append("  (:functions (stock ?res - resource)")
    L.append("              (need ?rec - recipe ?res - resource)")
    L.append("              (make ?rec - recipe ?res - resource))")
    if mode == "inst":
        L.append("  (:action craft :parameters (?rec - recipe)")
        L.append("    :precondition (forall (?res - resource) (>= (stock ?res) (need ?rec ?res)))")
        L.append("    :effect (forall (?res - resource)")
        L.append("              (and (decrease (stock ?res) (need ?rec ?res))")
        L.append("                   (increase (stock ?res) (make ?rec ?res)))))")
    else:
        L.append("  (:durative-action craft :parameters (?rec - recipe)")
        L.append("    :duration (= ?duration 2)")
        L.append("    :condition (at start (forall (?res - resource) (>= (stock ?res) (need ?rec ?res))))")
        L.append("    :effect (and (at start (forall (?res - resource) (decrease (stock ?res) (need ?rec ?res))))")
        L.append("                 (at end (forall (?res - resource) (increase (stock ?res) (make ?rec ?res))))))")
    L.append(")")
    return "\n".join(L) + "\n"


DOMAIN_EMIT = {"specific": dom_specific, "data-table": dom_datatable, "forall": dom_forall}

# --------------------------------------------------------------------------- #
# Problem emitters (semantically identical init+goal across encodings)
# --------------------------------------------------------------------------- #

def prob_specific(resources, recipes, raws, goal, N):
    stocks = " ".join(f"(= (stock_{r}) {N if r in raws else 0})" for r in resources)
    return (f"(define (problem chain) (:domain craft-specific)\n"
            f"  (:init {stocks})\n"
            f"  (:goal (>= (stock_{goal}) {N})))\n")


def prob_datatable(resources, recipes, raws, goal, N):
    arity = max_inputs(recipes)
    pred = "recipe" if arity == 1 else f"recipe{arity}"
    table = " ".join(f"({pred} {name} {' '.join(ins)} {out})" for name, ins, out in recipes)
    stocks = " ".join(f"(= (stock {r}) {N if r in raws else 0})" for r in resources)
    return (f"(define (problem chain) (:domain craft-data-table)\n"
            f"  (:init {table}\n"
            f"         {stocks})\n"
            f"  (:goal (>= (stock {goal}) {N})))\n")


def prob_forall(resources, recipes, raws, goal, N):
    need = {}  # (rec,res)->qty
    make = {}
    for name, ins, out in recipes:
        for i in ins:
            need[(name, i)] = need.get((name, i), 0) + 1
        make[(name, out)] = make.get((name, out), 0) + 1
    lines = []
    lines.append("  (:init")
    lines.append("    " + " ".join(f"(= (stock {r}) {N if r in raws else 0})" for r in resources))
    for name, _, _ in recipes:
        lines.append("    " + " ".join(f"(= (need {name} {r}) {need.get((name, r), 0)})" for r in resources))
        lines.append("    " + " ".join(f"(= (make {name} {r}) {make.get((name, r), 0)})" for r in resources))
    lines.append(f"  )")
    return (f"(define (problem chain) (:domain craft-forall)\n"
            + "\n".join(lines) + "\n"
            f"  (:goal (>= (stock {goal}) {N})))\n")


PROBLEM_EMIT = {"specific": prob_specific, "data-table": prob_datatable, "forall": prob_forall}

# --------------------------------------------------------------------------- #
# CLI
# --------------------------------------------------------------------------- #

def cmd_domain(args):
    resources, recipes, _, _ = content_of(args)
    sys.stdout.write(DOMAIN_EMIT[args.encoding](resources, recipes, args.mode))


def cmd_problem(args):
    resources, recipes, raws, goal = content_of(args)
    sys.stdout.write(PROBLEM_EMIT[args.encoding](resources, recipes, raws, goal, args.qty))


# Default experiment grid, chosen from a calibration sweep (see RESULTS.md):
#  - chain stays solvable up to large K, so it pushes K high to expose the
#    data-table grounding cost (K*R^2 candidate groundings) while node-expansions
#    stay encoding-identical;
#  - converge is the divergence zone (forall-numeric penalty), kept at D<=4 so the
#    data-table R^3 grounding and the search both finish.
# K = chain length (#recipes); D = tree depth; N = goal quantity.
GRID = {
    "chain":    {"sizes": [8, 16, 24, 32], "qtys": [1, 2, 4]},
    "converge": {"sizes": [2, 3, 4],       "qtys": [1, 2, 4]},
}


def cmd_emit_corpora(args):
    encodings = ["specific", "data-table", "forall"]
    modes = ["inst", "temporal"]
    contents = args.contents
    written = 0
    for content in contents:
        sizes = GRID[content]["sizes"]
        qtys = GRID[content]["qtys"]
        for enc in encodings:
            for mode in modes:
                for size in sizes:
                    # build a namespace mimicking parsed args for content_of()
                    ns = argparse.Namespace(content=content, recipes=size, depth=size)
                    resources, recipes, raws, goal = content_of(ns)
                    tag = f"k{size:02d}" if content == "chain" else f"d{size}"
                    cdir = os.path.join(args.out, content, f"{enc}-{mode}", tag)
                    os.makedirs(cdir, exist_ok=True)
                    with open(os.path.join(cdir, "domain.pddl"), "w") as f:
                        f.write(DOMAIN_EMIT[enc](resources, recipes, mode))
                    for N in qtys:
                        with open(os.path.join(cdir, f"p_n{N:02d}.pddl"), "w") as f:
                            f.write(PROBLEM_EMIT[enc](resources, recipes, raws, goal, N))
                        written += 1
    print(f"wrote {written} problems under {args.out}", file=sys.stderr)


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    def add_common(p):
        p.add_argument("--encoding", choices=list(DOMAIN_EMIT), required=True)
        p.add_argument("--mode", choices=["inst", "temporal"], required=True)
        p.add_argument("--content", choices=["chain", "converge"], default="chain")
        p.add_argument("--recipes", type=int, default=8, help="chain length K")
        p.add_argument("--depth", type=int, default=3, help="converge tree depth D")

    d = sub.add_parser("domain"); add_common(d); d.set_defaults(func=cmd_domain)
    q = sub.add_parser("problem"); add_common(q)
    q.add_argument("--qty", type=int, default=2, help="goal quantity N")
    q.set_defaults(func=cmd_problem)

    e = sub.add_parser("emit-corpora")
    e.add_argument("--out", default=os.path.join(os.path.dirname(__file__), "corpora"))
    e.add_argument("--contents", nargs="+", choices=["chain", "converge"],
                   default=["chain", "converge"])
    e.set_defaults(func=cmd_emit_corpora)

    args = ap.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
