#!/usr/bin/env python3
"""Generate matched crafting domains+problems for the encoding A/B/C benchmark.

Three encodings of the SAME crafting content, so the only variable is encoding style:

  specific    one hardcoded :action per recipe              (action fan-out)
  data-table  an ARITY-FAMILY of generic actions craft1/    (constants + data table
              craft2/craft3..., one per input-arity, each      "find the types"; thin
              gated by a static (recipeN ...) table over       operators, but grounding
              :constants, with per-recipe (inqK ?rec)/         scales R^(arity+1))
              (outq ?rec) quantity functions
  forall      one `craft ?rec` action quantifying over all  (fat ADL operator; handles
              resources via (need ?rec ?res)/(make ?rec ?res)  any arity/quantity in ONE
              quantity functions                                action, at a search cost)

Each emits in two modes:
  inst        instantaneous :action  -> classical FF path -> reports evaluated_states
  temporal    :durative-action       -> temporal mode     -> reports makespan

Content models (shared semantics across all encodings):
  chain     r0 -> r1 -> ... -> rK            (single-input unit recipes; knob K)
  converge  balanced binary assembly tree D  (two-input unit recipes; the ">=2
            contributions converge" heuristic-flat case, see ../../examples/BORDERS.md)
  techtree  a realistic RPG crafting tech-tree: multi-input recipes with quantities
            (e.g. house = 2 frame + 1 cutstone + 1 window) and shared intermediates.
            This is the case that needs the data-table arity-family and exposes its
            grounding cost vs forall-numeric's one-action generality.

A recipe is normalized to (name, inputs, out_res, out_qty) where inputs is a list of
(resource, qty). Plan length = total recipe firings, identical across encodings.

Usage:
  gen.py domain  --encoding E --mode M --content C [--recipes K | --depth D]
  gen.py problem --encoding E --mode M --content C [--recipes K | --depth D] --qty N
  gen.py emit-corpora --out DIR [--contents ...]

All templates here are verified to solve against this `ff` (see proto/ and README.md).
"""
import argparse
import os
import sys
from collections import defaultdict, deque

# --------------------------------------------------------------------------- #
# Content models: return (resources, recipes, raws, goal).
# recipe = (name, [(in_res, qty), ...], out_res, out_qty)
# --------------------------------------------------------------------------- #

def chain_content(K):
    """r0 -> r1 -> ... -> rK. Recipe c_i consumes 1 r_i, produces 1 r_{i+1}."""
    resources = [f"r{i}" for i in range(K + 1)]
    recipes = [(f"c{i}", [(f"r{i}", 1)], f"r{i + 1}", 1) for i in range(K)]
    return resources, recipes, ["r0"], f"r{K}"


def converge_content(D):
    """Balanced binary assembly tree, depth D. Internal node i needs both children
    (1 each), produces 1 of itself. Root r0 is the goal; leaves are raws."""
    n = 2 ** (D + 1) - 1
    resources = [f"r{i}" for i in range(n)]
    recipes = []
    for i in range(n):
        c1, c2 = 2 * i + 1, 2 * i + 2
        if c2 < n:
            recipes.append((f"c{i}", [(f"r{c1}", 1), (f"r{c2}", 1)], f"r{i}", 1))
    raws = [f"r{i}" for i in range(n) if 2 * i + 1 >= n]
    return resources, recipes, raws, "r0"


# A realistic RPG tech-tree: multi-input recipes with quantities and shared
# intermediates (plank feeds frame/tool/cart/sword; ingot feeds steel/tool; ...).
# brick and cloth are deliberate distractor recipes off the goal path. Single
# output per recipe so all three encodings express identical content. The goal
# `settlement` pulls most of the tree.
TECHTREE = [
    # name      inputs (res, qty)                              out          qty
    ("saw",     [("log", 1)],                                  "plank",      2),
    ("char",    [("log", 2)],                                  "charcoal",   1),
    ("smelt",   [("ore", 1), ("charcoal", 1)],                 "ingot",      1),
    ("block",   [("stone", 2)],                                "cutstone",   1),
    ("glasswk", [("sand", 2), ("coal", 1)],                    "glass",      1),
    ("mill",    [("grain", 1)],                                "flour",      1),
    ("brick",   [("clay", 2)],                                 "brick",      1),   # distractor
    ("weave",   [("fiber", 2)],                                "cloth",      1),   # distractor
    ("steelwk", [("ingot", 1), ("coal", 2)],                   "steel",      1),
    ("frame",   [("plank", 2)],                                "frame",      1),
    ("bake",    [("flour", 1), ("water", 1)],                  "bread",      1),
    ("toolwk",  [("ingot", 1), ("plank", 1)],                  "tool",       1),
    ("window",  [("glass", 2), ("frame", 1)],                  "window",     1),
    ("house",   [("frame", 2), ("cutstone", 1), ("window", 1)], "house",     1),
    ("cart",    [("plank", 2), ("tool", 1), ("steel", 1)],     "cart",       1),
    ("sword",   [("steel", 2), ("plank", 1)],                  "sword",      1),
    ("settle",  [("house", 1), ("cart", 1), ("bread", 2)],     "settlement", 1),
]


def techtree_content(_size=0):
    produced = {out for _, _, out, _ in TECHTREE}
    inputs = {r for _, ins, _, _ in TECHTREE for r, _ in ins}
    resources = sorted(produced | inputs)
    raws = sorted(inputs - produced)              # leaves with no producer
    return resources, list(TECHTREE), raws, "settlement"


def content_of(args):
    if args.content == "chain":
        return chain_content(args.recipes)
    if args.content == "converge":
        return converge_content(args.depth)
    return techtree_content()


def arities(recipes):
    return sorted({len(ins) for _, ins, _, _ in recipes})


def max_arity(recipes):
    return max((len(ins) for _, ins, _, _ in recipes), default=1)


def compute_demand(resources, recipes, goal, N):
    """Propagate demand down the recipe DAG (one producer per output) to get the
    EXACT raw amount needed to make N of `goal`, plus total firings (= plan length).
    Tight provisioning keeps the numeric search small (over-provisioning blows it up)."""
    producer = {out: (name, ins, oq) for name, ins, out, oq in recipes}
    succ, indeg = defaultdict(set), {r: 0 for r in resources}      # edge in -> out
    for _, ins, out, _ in recipes:
        for r, _q in ins:
            if out not in succ[r]:
                succ[r].add(out); indeg[out] += 1
    q = deque(r for r in resources if indeg[r] == 0)
    order = []
    while q:
        r = q.popleft(); order.append(r)
        for o in succ[r]:
            indeg[o] -= 1
            if indeg[o] == 0:
                q.append(o)
    demand = {r: 0 for r in resources}
    demand[goal] = N
    firings = {}
    for r in reversed(order):                                      # outputs before inputs
        if r in producer and demand[r] > 0:
            name, ins, oq = producer[r]
            f = -(-demand[r] // oq)                                # ceil(demand / out_qty)
            firings[name] = f
            for i, qy in ins:
                demand[i] += f * qy
    return demand, firings


def raw_init(resources, recipes, raws, goal, N):
    """Map every resource to its initial stock: exact demand for raws, 0 otherwise.
    Distractor leaves (demand 0) get a small live floor so their recipes are reachable
    (a realistic red-herring) without enlarging the search."""
    demand, _ = compute_demand(resources, recipes, goal, N)
    return {r: (demand[r] if demand[r] > 0 else 4) if r in raws else 0 for r in resources}

# --------------------------------------------------------------------------- #
# Domain emitters
# --------------------------------------------------------------------------- #

def _req(mode, inst, temporal):
    return inst if mode == "inst" else temporal


def dom_specific(resources, recipes, mode):
    L = ["(define (domain craft-specific)"]
    L.append("  (:requirements " + _req(mode, ":strips :numeric-fluents",
                                        ":strips :durative-actions :numeric-fluents") + ")")
    L.append("  (:functions " + " ".join(f"(stock_{r})" for r in resources) + ")")
    for name, ins, out, oq in recipes:
        if mode == "inst":
            L.append(f"  (:action {name} :parameters ()")
            L.append("    :precondition (and " + " ".join(f"(>= (stock_{r}) {q})" for r, q in ins) + ")")
            eff = " ".join(f"(decrease (stock_{r}) {q})" for r, q in ins) + f" (increase (stock_{out}) {oq})"
            L.append(f"    :effect (and {eff}))")
        else:
            L.append(f"  (:durative-action {name} :parameters ()")
            L.append("    :duration (= ?duration 2)")
            L.append("    :condition (and " + " ".join(f"(at start (>= (stock_{r}) {q}))" for r, q in ins) + ")")
            eff = " ".join(f"(at start (decrease (stock_{r}) {q}))" for r, q in ins) + f" (at end (increase (stock_{out}) {oq}))"
            L.append(f"    :effect (and {eff}))")
    L.append(")")
    return "\n".join(L) + "\n"


def dom_datatable(resources, recipes, mode):
    ars = arities(recipes)
    ma = max_arity(recipes)
    L = ["(define (domain craft-data-table)"]
    L.append("  (:requirements " + _req(mode, ":strips :typing :numeric-fluents",
                                        ":strips :typing :durative-actions :numeric-fluents") + ")")
    L.append("  (:types recipe resource - object)")
    L.append("  (:constants " + " ".join(n for n, _, _, _ in recipes) + " - recipe   "
             + " ".join(resources) + " - resource)")
    # one (recipeN ...) predicate per arity present
    preds = []
    for a in ars:
        types = " ".join(f"?in{k} - resource" for k in range(a))
        preds.append(f"(recipe{a} ?rec - recipe {types} ?out - resource)")
    L.append("  (:predicates " + " ".join(preds) + ")")
    # stock + per-recipe input-slot quantities (inq0..inq{ma-1}) + output quantity
    funcs = ["(stock ?res - resource)"] + [f"(inq{k} ?rec - recipe)" for k in range(ma)] + ["(outq ?rec - recipe)"]
    L.append("  (:functions " + " ".join(funcs) + ")")
    for a in ars:
        in_params = " ".join(f"?in{k}" for k in range(a))
        in_types = " ".join(f"?in{k} - resource" for k in range(a))
        if mode == "inst":
            L.append(f"  (:action craft{a} :parameters (?rec - recipe {in_types} ?out - resource)")
            L.append(f"    :precondition (and (recipe{a} ?rec {in_params} ?out) "
                     + " ".join(f"(>= (stock ?in{k}) (inq{k} ?rec))" for k in range(a)) + ")")
            L.append("    :effect (and " + " ".join(f"(decrease (stock ?in{k}) (inq{k} ?rec))" for k in range(a))
                     + " (increase (stock ?out) (outq ?rec))))")
        else:
            L.append(f"  (:durative-action craft{a} :parameters (?rec - recipe {in_types} ?out - resource)")
            L.append("    :duration (= ?duration 2)")
            L.append(f"    :condition (and (at start (recipe{a} ?rec {in_params} ?out)) "
                     + " ".join(f"(at start (>= (stock ?in{k}) (inq{k} ?rec)))" for k in range(a)) + ")")
            L.append("    :effect (and " + " ".join(f"(at start (decrease (stock ?in{k}) (inq{k} ?rec)))" for k in range(a))
                     + " (at end (increase (stock ?out) (outq ?rec)))))")
    L.append(")")
    return "\n".join(L) + "\n"


def dom_forall(resources, recipes, mode):
    L = ["(define (domain craft-forall)"]
    L.append("  (:requirements " + _req(
        mode, ":typing :numeric-fluents :universal-preconditions :conditional-effects",
        ":typing :durative-actions :numeric-fluents :universal-preconditions :conditional-effects") + ")")
    L.append("  (:types recipe resource - object)")
    L.append("  (:constants " + " ".join(n for n, _, _, _ in recipes) + " - recipe   "
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

def _stock_init(resources, ri, fluent):
    return " ".join(f"(= ({fluent(r)}) {ri[r]})" for r in resources)


def prob_specific(resources, recipes, raws, goal, N):
    stocks = _stock_init(resources, raw_init(resources, recipes, raws, goal, N), lambda r: f"stock_{r}")
    return (f"(define (problem craft) (:domain craft-specific)\n"
            f"  (:init {stocks})\n"
            f"  (:goal (>= (stock_{goal}) {N})))\n")


def prob_datatable(resources, recipes, raws, goal, N):
    ma = max_arity(recipes)
    facts = []
    for name, ins, out, oq in recipes:
        a = len(ins)
        in_res = " ".join(r for r, _ in ins)
        facts.append(f"(recipe{a} {name} {in_res} {out})")
        for k in range(ma):
            qty = ins[k][1] if k < a else 0
            facts.append(f"(= (inq{k} {name}) {qty})")
        facts.append(f"(= (outq {name}) {oq})")
    stocks = _stock_init(resources, raw_init(resources, recipes, raws, goal, N), lambda r: f"stock {r}")
    return (f"(define (problem craft) (:domain craft-data-table)\n"
            f"  (:init " + " ".join(facts) + "\n"
            f"         " + stocks + ")\n"
            f"  (:goal (>= (stock {goal}) {N})))\n")


def prob_forall(resources, recipes, raws, goal, N):
    need, make = {}, {}
    for name, ins, out, oq in recipes:
        for r, q in ins:
            need[(name, r)] = need.get((name, r), 0) + q
        make[(name, out)] = make.get((name, out), 0) + oq
    lines = ["  (:init",
             "    " + _stock_init(resources, raw_init(resources, recipes, raws, goal, N), lambda r: f"stock {r}")]
    for name, _, _, _ in recipes:
        lines.append("    " + " ".join(f"(= (need {name} {r}) {need.get((name, r), 0)})" for r in resources))
        lines.append("    " + " ".join(f"(= (make {name} {r}) {make.get((name, r), 0)})" for r in resources))
    lines.append("  )")
    return (f"(define (problem craft) (:domain craft-forall)\n"
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


# Default experiment grid (sizes/qtys chosen from a calibration sweep, see RESULTS.md).
# chain/converge sweep size; techtree is a fixed tree swept only by goal quantity N.
GRID = {
    "chain":    {"sizes": [8, 16, 24, 32], "qtys": [1, 2, 4]},
    "converge": {"sizes": [2, 3, 4],       "qtys": [1, 2, 4]},
    "techtree": {"sizes": [0],             "qtys": [1]},   # N>=2 hits the monolithic-search border
}


def _tag(content, size):
    return {"chain": f"k{size:02d}", "converge": f"d{size}", "techtree": "tree"}[content]


def cmd_emit_corpora(args):
    written = 0
    for content in args.contents:
        for enc in DOMAIN_EMIT:
            for mode in ["inst", "temporal"]:
                for size in GRID[content]["sizes"]:
                    ns = argparse.Namespace(content=content, recipes=size, depth=size)
                    resources, recipes, raws, goal = content_of(ns)
                    cdir = os.path.join(args.out, content, f"{enc}-{mode}", _tag(content, size))
                    os.makedirs(cdir, exist_ok=True)
                    with open(os.path.join(cdir, "domain.pddl"), "w") as f:
                        f.write(DOMAIN_EMIT[enc](resources, recipes, mode))
                    for N in GRID[content]["qtys"]:
                        with open(os.path.join(cdir, f"p_n{N:02d}.pddl"), "w") as f:
                            f.write(PROBLEM_EMIT[enc](resources, recipes, raws, goal, N))
                        written += 1
    print(f"wrote {written} problems under {args.out}", file=sys.stderr)


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    contents = ["chain", "converge", "techtree"]

    def add_common(p):
        p.add_argument("--encoding", choices=list(DOMAIN_EMIT), required=True)
        p.add_argument("--mode", choices=["inst", "temporal"], required=True)
        p.add_argument("--content", choices=contents, default="chain")
        p.add_argument("--recipes", type=int, default=8, help="chain length K")
        p.add_argument("--depth", type=int, default=3, help="converge tree depth D")

    d = sub.add_parser("domain"); add_common(d); d.set_defaults(func=cmd_domain)
    q = sub.add_parser("problem"); add_common(q)
    q.add_argument("--qty", type=int, default=2, help="goal quantity N")
    q.set_defaults(func=cmd_problem)

    e = sub.add_parser("emit-corpora")
    e.add_argument("--out", default=os.path.join(os.path.dirname(__file__), "corpora"))
    e.add_argument("--contents", nargs="+", choices=contents, default=contents)
    e.set_defaults(func=cmd_emit_corpora)

    args = ap.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
