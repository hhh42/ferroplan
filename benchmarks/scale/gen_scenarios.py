#!/usr/bin/env python3
"""Emit rpg-world 'problem space' scenarios that each stress a different mechanic.
Run with no args; writes examples/rpg-world/scenarios/*.pddl."""
import os, re
ROOT = os.path.join(os.path.dirname(__file__), "..", "..")
dom = open(os.path.join(ROOT, "examples/rpg-world/domain.pddl")).read()
fi = dom.find("(:functions"); d=0
for j in range(fi, len(dom)):
    if dom[j]=='(': d+=1
    elif dom[j]==')':
        d-=1
        if d==0: fe=j; break
FLUENTS = [f for f in sorted(set(re.findall(r'\(([a-z][\w-]*)\)', dom[fi:fe+1]))) if f!="dist"]

def emit(name, objects, init_lines, seed, goal, blurb):
    o = [f";; {blurb}", f"(define (problem {name}) (:domain rpg-world)",
         f"  (:objects {objects})", "  (:init"]
    o += [f"    {l}" for l in init_lines]
    o += [f"    (= ({f}) {seed.get(f,0)})" for f in FLUENTS]
    o += ["  )", f"  (:goal (and {goal})))"]
    p = os.path.join(ROOT, "examples/rpg-world/scenarios", name + ".pddl")
    open(p, "w").write("\n".join(o) + "\n")
    print("wrote", os.path.relpath(p, ROOT))

# 1) BOOTSTRAP: build your own sawmill (from pre-cut components) then use it.
emit("bootstrap-a-workshop",
     "b1 - agent  site - location",
     ["(at b1 site) (is-buildsite site) (builder b1)"],
     {"timber-frame":1, "cut-fittings":2, "blocks":2, "logs":6},
     "(>= (planks) 2)",
     "BOOTSTRAP: no sawmill exists; build one (timber-frame+cut-fittings+blocks) then saw planks. Stresses self-built workstations + the build-then-use dependency.")

# 2) LOGISTICS: gather from POIs several hops away (travel + reachability axiom).
emit("logistics-run",
     "c1 - agent  camp forest junction quarry - location",
     ["(at c1 camp) (has-axe c1) (has-pick c1)",
      "(is-forest forest) (is-quarry quarry)",
      "(link camp forest) (link forest camp) (= (dist camp forest) 2) (= (dist forest camp) 2)",
      "(link camp junction) (link junction camp) (= (dist camp junction) 2) (= (dist junction camp) 2)",
      "(link junction quarry) (link quarry junction) (= (dist junction quarry) 2) (= (dist quarry junction) 2)"],
     {},
     "(>= (logs) 3) (>= (stone) 2)",
     "LOGISTICS: the quarry is 2 hops from camp (via a junction); chop at the forest, mine at the quarry, no processing. Stresses durative travel + the derived reachability axiom + multi-site accumulation.")

# 3) MANA CYCLE: a mage must refill mana to brew several potions.
emit("mana-cycle",
     "m1 - agent  tower - location",
     ["(at m1 tower) (has-altar tower) (mage m1)"],
     {"herbs":3, "water":3, "mana":0},
     "(>= (potions) 3)",
     "MANA CYCLE: brewing costs mana, meditation restores it. Three potions need meditate->brew interleaving. Stresses the renewable mana resource (consume/regenerate) loop.")

# 4) GUILD ORDER: one order with three distinct deliverables (a conjunctive goal).
emit("guild-order",
     "smith1 mason1 cook1 - agent  hub - location",
     ["(at smith1 hub) (at mason1 hub) (at cook1 hub)",
      "(has-forge hub) (has-mason hub) (has-hearth hub)",
      "(smith smith1) (mason-skill mason1) (cook cook1)"],
     {"ore":4, "charcoal":4, "stone":4, "food":4, "water":4},
     "(>= (ingots) 2) (>= (blocks) 2) (>= (meals) 2)",
     "GUILD ORDER: one order, three distinct deliverables (ingots + blocks + meals). Stresses a MULTI-PART (conjunctive) goal — the temporal search's known weak spot — which solves here because each part is short. NOTE: roles are yield BONUSES not gates, and the model has no agent-exclusion, so the planner may fill the whole order with one craftsman; forcing true division of labour needs a per-agent 'busy' token.")
