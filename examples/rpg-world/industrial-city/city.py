#!/usr/bin/env python3
"""Industrial city showcase — run a whole functioning metal/stone/wood industry on
the BASE rpg-world domain by scheduling it as a pipeline of in-border contracts that
share one city stockpile (exactly what BORDERS.md prescribes; a monolithic plan
would hit every wall at once). This is a hand-authored demo scheduler for ONE city,
not the general decomposer.

Each contract: emit a problem (full world + current stockpile + a sub-goal), run
ferroplan, then REPLAY the returned plan through effects auto-extracted from the
domain to advance the stockpile. The engine does the planning; we carry state."""
import os, re, subprocess, sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
DOM = os.path.join(ROOT, "examples/rpg-world/domain.pddl")
FF = os.path.join(ROOT, "target/release/ff")
TMP = "/tmp/city"; os.makedirs(TMP, exist_ok=True)
dom = open(DOM).read()

# ---- parse domain: fluents, station/site/kit predicates, per-action effects ----
def sect(kw):
    i = dom.find("(:" + kw); d = 0
    for j in range(i, len(dom)):
        if dom[j] == '(': d += 1
        elif dom[j] == ')':
            d -= 1
            if d == 0: return dom[i:j + 1]
FLUENTS = [f for f in sorted(set(re.findall(r'\(([a-z][\w-]*)\)', sect("functions")))) if f != "dist"]
STATIONS, KIT = [], []
for name, args in re.findall(r'\(([a-z][\w-]*)((?:\s+\?[\w-]+\s*-\s*[a-z][\w-]*)*)\s*\)', sect("predicates")):
    if args.count('?') != 1: continue
    t = re.search(r'-\s*([a-z][\w-]*)', args); t = t.group(1) if t else ""
    if t == "location" and (name.startswith("is-") or name.startswith("has-")): STATIONS.append(name)
    elif t == "agent": KIT.append(name)

def action_blocks():
    out = []; i = 0
    while True:
        i = dom.find("(:durative-action", i)
        if i < 0: break
        d = 0; j = i
        while j < len(dom):
            if dom[j] == '(': d += 1
            elif dom[j] == ')':
                d -= 1
                if d == 0: break
            j += 1
        out.append(dom[i:j + 1]); i = j + 1
    return out
EFF = {}                                   # action -> (consumes{f:n}, produces{f:n})
for blk in action_blocks():
    name = re.match(r'\(:durative-action\s+([\w-]+)', blk).group(1)
    e = blk[blk.find(":effect"):]
    cons, prod = {}, {}
    for op, f, v in re.findall(r'\((increase|decrease)\s+\((\w[\w-]*)\)\s+([\d.]+)\)', e):
        (prod if op == "increase" else cons).setdefault(f, 0)
        (prod if op == "increase" else cons)[f] += float(v)
    EFF[name] = (cons, prod)

# ---- the city: sites (each its own location) + a hub with every station ----
SITES = {"is-forest": "forest", "is-mine": "mine", "is-coalmine": "coalmine",
         "is-quarry": "quarry", "is-claypit": "claypit", "is-coppermine": "coppermine",
         "is-tinmine": "tinmine", "is-goldmine": "goldmine"}
HUB_STATIONS = [s for s in STATIONS if s not in SITES]      # has-* + is-buildsite + is-market
LOCS = ["hub"] + list(SITES.values())
SLOTS = ["s0", "s1", "s2"]
# one full-kit worker stationed at each site + four at the hub
AGENTS = {f"w_{loc}": loc for loc in SITES.values()} | {f"h{i}": "hub" for i in range(1, 5)}

def problem(goal_line, stock, facts):
    L = [f"(define (problem city) (:domain rpg-world)",
         f"  (:objects {' '.join(LOCS)} - location  {' '.join(SLOTS)} - slot  {' '.join(AGENTS)} - agent)",
         "  (:init"]
    for a, loc in AGENTS.items():
        L.append(f"    (at {a} {loc}) " + " ".join(f"({k} {a})" for k in KIT))
    L.append("    " + " ".join(f"({st} hub)" for st in HUB_STATIONS))
    L.append("    " + " ".join(f"({p} {loc})" for p, loc in SITES.items()))
    L += sorted(f"    {f}" for f in facts)
    L += [f"    (= ({f}) {stock.get(f, 0)})" for f in FLUENTS]
    L += ["  )", f"  (:goal (and {goal_line})))"]
    return "\n".join(L) + "\n"

def run(goal_line, stock, facts, tag):
    p = f"{TMP}/{tag}.pddl"; open(p, "w").write(problem(goal_line, stock, facts))
    r = subprocess.run(["perl", "-e", "alarm 60; exec @ARGV", FF, "-o", DOM, "-f", p],
                       capture_output=True, text=True)
    ms = re.search(r"makespan:\s*([\d.]+)", r.stdout)
    acts = re.findall(r"^[\d.]+:\s*\(([a-z][\w-]*)([^)]*)\)", r.stdout, re.M)
    return (float(ms.group(1)) if ms else None), acts

def replay(acts, stock):
    for name, _args in acts:
        cons, prod = EFF.get(name, ({}, {}))
        for f, v in cons.items(): stock[f] = stock.get(f, 0) - v
        for f, v in prod.items(): stock[f] = stock.get(f, 0) + v

# ---- the production pipeline (each contract is within BORDERS.md limits) ----
# ("name", "stage", goal) ; goal is ("num",fluent,add) [produce `add` more] or ("fact","(pred args)")
PIPELINE = [
 ("forestry","RAW EXTRACTION",("num","logs",140)),
 ("iron-mine","RAW EXTRACTION",("num","ore",60)),
 ("coal-mine","RAW EXTRACTION",("num","coal",30)),
 ("quarry","RAW EXTRACTION",("num","stone",70)),
 ("clay-pit","RAW EXTRACTION",("num","clay",30)),
 ("copper-mine","RAW EXTRACTION",("num","copper-ore",16)),
 ("tin-mine","RAW EXTRACTION",("num","tin-ore",16)),
 ("charcoal-kiln","PRIMARY PROCESSING",("num","charcoal",40)),
 ("sawmill","PRIMARY PROCESSING",("num","planks",60)),
 ("smeltery","PRIMARY PROCESSING",("num","ingots",30)),
 ("masonry","PRIMARY PROCESSING",("num","blocks",40)),
 ("brickworks","PRIMARY PROCESSING",("num","bricks",20)),
 ("copper-smelt","PRIMARY PROCESSING",("num","copper-ingot",12)),
 ("tin-smelt","PRIMARY PROCESSING",("num","tin-ingot",12)),
 ("steelworks","MANUFACTURING",("num","steel",8)),
 ("bronze-foundry","MANUFACTURING",("num","bronze",8)),
 ("axe-forge","MANUFACTURING",("num","axe-stock",6)),
 ("pick-forge","MANUFACTURING",("num","pick-stock",6)),
 ("carpentry-frames","MANUFACTURING",("num","timber-frame",8)),
 ("fittings","MANUFACTURING",("num","cut-fittings",12)),
 ("house-s0","CONSTRUCTION",("fact","(built-house s0)")),
 ("house-s1","CONSTRUCTION",("fact","(built-house s1)")),
 ("house-s2","CONSTRUCTION",("fact","(built-house s2)")),
 ("town-wall","CONSTRUCTION",("fact","(built-wall)")),
 ("town-square","CONSTRUCTION",("fact","(built-square)")),
 ("town-well","CONSTRUCTION",("fact","(well-dug)")),
]

if len(sys.argv) > 1 and sys.argv[1] == "monolith":
    # the SAME city asked for in ONE plan from a cold start — the contrast case
    goal = " ".join(["(built-square)", "(built-wall)", "(well-dug)",
                     "(built-house s0)", "(built-house s1)", "(built-house s2)",
                     "(>= (steel) 4)", "(>= (bronze) 4)", "(>= (axe-stock) 4)"])
    print("MONOLITHIC whole-city goal, one plan, cold start...")
    ms, acts = run(goal, {}, set(), "monolith")
    print(f"  -> {'SOLVED ms=%.1f' % ms if ms else 'NO PLAN  (as predicted: the city must be decomposed)'}")
    sys.exit(0)

stock = {}; facts = set(); rows = []; total_ms = 0.0; total_acts = 0; stage = None
for name, st, goal in PIPELINE:
    if st != stage: stage = st; print(f"\n=== {stage} ===")
    if goal[0] == "num":
        _, f, add = goal; line = f"(>= ({f}) {stock.get(f,0)+add})"; want = f
    else:
        line = goal[1]; want = goal[1]
    ms, acts = run(line, stock, facts, name)
    if ms is None:
        print(f"  {name:18} FAILED (no plan) — goal {line}"); rows.append((name, st, "FAIL", 0)); continue
    before = dict(stock); replay(acts, stock)
    if goal[0] == "fact": facts.add(goal[1])
    total_ms += ms; total_acts += len(acts)
    delta = want if goal[0]=="fact" else f"+{int(stock.get(want,0)-before.get(want,0))} {want} (={int(stock.get(want,0))})"
    print(f"  {name:18} ms={ms:7.2f}  {len(acts):3d} ops   {delta}")
    rows.append((name, st, f"{ms:.1f}", len(acts)))

print("\n" + "="*60)
print("CITY STATUS")
print(f"  contracts run : {len([r for r in rows if r[2]!='FAIL'])}/{len(rows)} solved")
print(f"  total ops     : {total_acts}")
print(f"  scheduled makespan (sequential) : {total_ms:.1f}")
print(f"  structures    : {sorted(facts)}")
inv = {f:int(v) for f,v in sorted(stock.items()) if v>0.5}
print(f"  stockpile     : {inv}")
