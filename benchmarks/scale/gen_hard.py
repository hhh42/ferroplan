#!/usr/bin/env python3
"""Adversarial 'where does it break' problems for rpg-world: deep dependency DAGs,
long corridors, tight allocation, big conjunctions, forced multi-path goals.
Writes examples/rpg-world/hard/*.pddl."""
import os, re
ROOT = os.path.join(os.path.dirname(__file__), "..", "..")
DOM = os.path.join(ROOT, "examples/rpg-world/domain.pddl")
dom = open(DOM).read()

def section(kw):
    i = dom.find("(:" + kw); d = 0
    for j in range(i, len(dom)):
        if dom[j] == '(': d += 1
        elif dom[j] == ')':
            d -= 1
            if d == 0: return dom[i:j + 1]

FLUENTS = [f for f in sorted(set(re.findall(r'\(([a-z][\w-]*)\)', section("functions")))) if f != "dist"]
STATIONS, KIT = [], []
for name, args in re.findall(r'\(([a-z][\w-]*)((?:\s+\?[\w-]+\s*-\s*[a-z][\w-]*)*)\s*\)', section("predicates")):
    if args.count('?') != 1:
        continue
    t = re.search(r'-\s*([a-z][\w-]*)', args); t = t.group(1) if t else ""
    if t == "location" and (name.startswith("is-") or name.startswith("has-")):
        STATIONS.append(name)
    elif t == "agent":
        KIT.append(name)

def flu(seed): return [f"    (= ({f}) {seed.get(f, 0)})" for f in FLUENTS]

def write(name, lines, goal, blurb):
    out = [f";; {blurb}", f"(define (problem {name}) (:domain rpg-world)"] + lines + ["  )", f"  (:goal (and {goal})))"]
    open(os.path.join(ROOT, "examples/rpg-world/hard", name + ".pddl"), "w").write("\n".join(out) + "\n")

def hub(name, seed, goal, blurb, drop=(), extra_obj="", extra_init=()):
    sts = [s for s in STATIONS if s not in drop]
    objs = "a0 - agent  hub - location" + ("  " + extra_obj if extra_obj else "")
    lines = [f"  (:objects {objs})", "  (:init", "    (at a0 hub)",
             "    " + " ".join(f"({s} hub)" for s in sts),
             "    " + " ".join(f"({k} a0)" for k in KIT)]
    lines += ["    " + l for l in extra_init] + flu(seed)
    write(name, lines, goal, blurb)

def corridor(name, n, goal, blurb):
    nodes = [f"n{i}" for i in range(n)]
    links = []
    for i in range(n - 1): links += [(i, i + 1), (i + 1, i)]   # pure chain, NO shortcuts
    lines = [f"  (:objects {' '.join(nodes)} - location  a0 - agent)", "  (:init", "    (at a0 n0)",
             "    " + " ".join(f"({k} a0)" for k in KIT)]
    lines += ["    " + " ".join(f"(link n{i} n{j}) (= (dist n{i} n{j}) 1)" for (i, j) in links[k:k+8])
              for k in range(0, len(links), 8)]
    lines += flu({})
    write(name, lines, goal, blurb)

# ---- deep dependency DAGs ----
hub("tech-steel", {}, "(>= (steel) 2)", "DEEP DAG: ore+coal->ingot->steel, x2")
hub("tech-bronze", {}, "(>= (bronze) 2)", "DEEP DAG: copper+tin->ingots->bronze, x2")
hub("tech-bronze-steel", {}, "(>= (bronze) 1) (>= (steel) 1)", "DEEP DAG x2: bronze AND steel chains")
hub("bread-line", {"seeds": 1, "water": 10}, "(>= (bread) 2)", "DEEP DAG: farm grain -> grind flour -> bake bread")
# ---- forced multi-path economy ----
hub("mint-fortune", {}, "(>= (coin) 15)", "MULTI-PATH economy at depth: coin via sell OR mint OR haul (all reachable), 15 deep")
# ---- tight allocation (no extra resource) ----
hub("log-scarcity", {"logs": 4}, "(>= (planks) 4) (>= (charcoal) 1)",
    "ALLOCATION: split exactly 4 logs between planks (saw) and charcoal (burn); no forest to chop more", drop=("is-forest",))
# ---- big conjunctions ----
hub("order-8", {"ore": 10, "charcoal": 10, "stone": 10, "logs": 16, "fiber": 10, "food": 10, "water": 10, "clay": 10, "herbs": 6, "mana": 4},
    "(>= (ingots) 2) (>= (blocks) 2) (>= (planks) 4) (>= (cloth) 1) (>= (meals) 2) (>= (bricks) 2) (>= (clothing) 1) (>= (potions) 1)",
    "BIG CONJUNCTION: an 8-part order")
hub("order-12", {"ore": 16, "charcoal": 16, "stone": 16, "logs": 24, "fiber": 16, "food": 16, "water": 16, "clay": 16, "herbs": 8, "mana": 8, "grain": 6, "carry-cap": 12},
    "(>= (ingots) 2) (>= (blocks) 2) (>= (planks) 4) (>= (cloth) 1) (>= (meals) 2) (>= (bricks) 2) (>= (clothing) 1) (>= (potions) 2) (>= (charcoal) 4) (>= (coin) 2) (>= (flour) 2) (>= (cargo) 1)",
    "BIG CONJUNCTION: a 12-part order")
# ---- deep travel (no shortcuts) ----
corridor("corridor-12", 12, "(at a0 n11)", "DEEP TRAVEL: a 12-node corridor, no shortcuts")
corridor("corridor-20", 20, "(at a0 n19)", "DEEP TRAVEL: a 20-node corridor, no shortcuts")
# ---- the village shape, small (travel + gather + process + build) ----
def gatherbuild():
    lines = ["  (:objects a0 - agent  camp forest quarry - location)", "  (:init",
             "    (at a0 camp) " + " ".join(f"({k} a0)" for k in KIT),
             "    (is-buildsite camp) (has-mason camp) (is-forest forest) (is-quarry quarry)",
             "    (link camp forest) (link forest camp) (= (dist camp forest) 2) (= (dist forest camp) 2)",
             "    (link camp quarry) (link quarry camp) (= (dist camp quarry) 2) (= (dist quarry camp) 2)"]
    lines += flu({})
    write("gather-build", lines, "(built-wall)",
          "VILLAGE SHAPE (small): travel to the quarry, mine stone, return, mason blocks, build a wall")
gatherbuild()
# ---- cyclic at scale ----
hub("farmstead-big", {"seeds": 1, "water": 12}, "(>= (grain) 10)", "CYCLIC x5: five crop cycles on one field")

print("wrote", len(os.listdir(os.path.join(ROOT, "examples/rpg-world/hard"))), "adversarial problems")
