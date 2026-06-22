#!/usr/bin/env python3
"""Bust out a broad suite of rpg-world problems across flavors x scales, to map
where the engine solves vs struggles. Writes examples/rpg-world/suite/*.pddl."""
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
# classify 1-arity predicates: location is-/has- => station/site (provision);
# agent-arg => tool/role (provision). Multi-arg + outcome predicates are skipped.
STATIONS, KIT = [], []
for name, args in re.findall(r'\(([a-z][\w-]*)((?:\s+\?[\w-]+\s*-\s*[a-z][\w-]*)*)\s*\)', section("predicates")):
    if args.count('?') != 1:
        continue
    t = re.search(r'-\s*([a-z][\w-]*)', args)
    t = t.group(1) if t else ""
    if t == "location" and (name.startswith("is-") or name.startswith("has-")):
        STATIONS.append(name)
    elif t == "agent":
        KIT.append(name)

def fluents(seed):
    return [f"    (= ({f}) {seed.get(f, 0)})" for f in FLUENTS]

def write(name, lines, goal, blurb):
    out = [f";; {blurb}", f"(define (problem {name}) (:domain rpg-world)"] + lines + \
          ["  )", f"  (:goal (and {goal})))"]
    open(os.path.join(ROOT, "examples/rpg-world/suite", name + ".pddl"), "w").write("\n".join(out) + "\n")

def hub(name, extra_obj, extra_init, seed, goal, blurb):
    objs = "a0 - agent  hub - location" + ("  " + extra_obj if extra_obj else "")
    lines = [f"  (:objects {objs})", "  (:init",
             "    (at a0 hub)",
             "    " + " ".join(f"({s} hub)" for s in STATIONS),
             "    " + " ".join(f"({k} a0)" for k in KIT)]
    lines += ["    " + l for l in extra_init]
    lines += fluents(seed)
    write(name, lines, goal, blurb)

def maze(name, n, extra_init, seed, goal, blurb, extra_obj=""):
    nodes = [f"n{i}" for i in range(n)]
    links = []
    for i in range(n - 1):
        links += [(i, i + 1), (i + 1, i)]
    for i in range(0, n, 3):                      # a few shortcut edges -> maze
        j = (i + 4) % n
        if j != i: links += [(i, j), (j, i)]
    lines = [f"  (:objects {' '.join(nodes)} - location  a0 - agent" + ("  " + extra_obj if extra_obj else "") + ")",
             "  (:init", "    (at a0 n0)",
             "    " + " ".join(f"({k} a0)" for k in KIT)]
    lines += ["    " + " ".join(f"(link n{i} n{j}) (= (dist n{i} n{j}) {1+((i+j)%3)})" for (i, j) in links[k:k+6])
              for k in range(0, len(links), 6)]
    lines += ["    " + l for l in extra_init]
    lines += fluents(seed)
    write(name, lines, goal, blurb)

# ---------------- the suite ----------------
# TRADE / marketplace
hub("trade-stall",  "", [], {"planks":8},  "(>= (coin) 4)",  "marketplace: sell planks for coin (small)")
hub("trade-market", "", [], {"planks":24}, "(>= (coin) 12)", "marketplace: a busy market (medium)")
hub("trade-bazaar", "", [], {"planks":60}, "(>= (coin) 30)", "marketplace: a grand bazaar (long sell chain)")
# TREES / woodline
hub("grove",    "", [], {}, "(>= (planks) 4)",  "forestry: chop + saw (small)")
hub("woodlot",  "", [], {}, "(>= (planks) 12)", "forestry: a working woodlot (medium)")
hub("forestry", "", [], {}, "(>= (planks) 30)", "forestry: a deep lumber order (long accumulate)")
# MINING
hub("prospect",  "", [], {}, "(>= (ore) 4)",  "mining: surface ore (small)")
hub("mineshaft", "", [], {}, "(>= (ore) 14)", "mining: a working shaft (medium)")
hub("deepmine",  "", [], {}, "(>= (ore) 30)", "mining: a deep ore order (long accumulate)")
# SMELTING chain
hub("foundry",       "", [], {"logs":12}, "(>= (ingots) 3)", "smelting: mine + charcoal + smelt (small chain)")
hub("great-foundry", "", [], {"logs":24}, "(>= (ingots) 8)", "smelting: a foundry order (deep chain)")
# MAGIC / mana
hub("apothecary",    "", [], {"herbs":4, "water":4},   "(>= (potions) 3)",  "magic: meditate<->brew cycle (small)")
hub("grand-alchemy", "", [], {"herbs":12, "water":12}, "(>= (potions) 10)", "magic: a deep potion order (mana cycle x many)")
# JOB-SHOP / conjunctive orders
hub("order-2", "", [], {"ore":4, "charcoal":4, "stone":4},
    "(>= (ingots) 2) (>= (blocks) 2)", "job-shop: a 2-part order")
hub("order-3", "", [], {"ore":4, "charcoal":4, "stone":4, "logs":4},
    "(>= (ingots) 2) (>= (blocks) 2) (>= (planks) 4)", "job-shop: a 3-part order")
hub("order-5", "", [], {"ore":4, "charcoal":4, "stone":4, "logs":6, "fiber":6, "food":4, "water":4},
    "(>= (ingots) 2) (>= (blocks) 2) (>= (planks) 4) (>= (cloth) 1) (>= (meals) 2)",
    "job-shop: a 5-part order (the conjunctive-goal stress)")
# FARMING
hub("smallholding", "", [], {"seeds":1, "water":2}, "(>= (grain) 2)", "farming: one till->plant->irrigate->harvest cycle")
hub("farmstead",    "", [], {"seeds":1, "water":6}, "(>= (grain) 6)", "farming: several crop cycles (re-tilling)")
# COMBAT
hub("skirmish", "", ["(threat hub)"], {"ration":2}, "(cleared hub)", "combat: an armed agent clears a threat")
# TRANSPORT
hub("freight", "", [], {"planks":8, "carry-cap":12}, "(>= (cargo) 4)", "transport: load planks into cargo at a depot")
# CONSTRUCTION
hub("raise-wall",   "", [], {"blocks":4}, "(built-wall)", "construction: raise a wall (pre-stocked)")
hub("raise-square", "s0 s1 - slot", [], {"planks":8, "bricks":4, "blocks":4}, "(built-square)",
    "construction: 2 houses + the square (processed pre-stocked)")
hub("found-village", "s0 s1 - slot", [], {"logs":40, "stone":40, "clay":40},
    "(built-square) (well-dug)", "construction: a whole village FROM RAW (the monolithic limit)")
# MAZE / travel
maze("maze-6",  6,  [], {}, "(at a0 n5)",  "travel: cross a small maze (reachability + travel)")
maze("maze-15", 15, [], {}, "(at a0 n14)", "travel: cross a medium maze")
maze("maze-40", 40, [], {}, "(at a0 n39)", "travel: cross a large maze (reachability-closure scale)")
# COMBAT patrol across a map
maze("patrol", 5, ["(threat n2)", "(threat n4)"], {"ration":4},
     "(cleared n2) (cleared n4)", "combat: patrol a map, clearing threats at distant nodes")

print("wrote", len(os.listdir(os.path.join(ROOT, "examples/rpg-world/suite"))), "problems")
