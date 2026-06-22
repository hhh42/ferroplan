#!/usr/bin/env python3
"""Generate a realistic village-build scenario for rpg-world.
Usage: gen_village.py M_AGENTS HOUSES PRESTOCK SPREAD EXTRAS
  M_AGENTS  number of craftsmen
  HOUSES    house slots to build (-> build-square needs ALL of them)
  PRESTOCK  1 = seed raw materials; 0 = gather everything from scratch
  SPREAD    1 = resources at separate POIs (travel needed); 0 = clustered at hub
  EXTRAS    comma list of extra goals: well,walls,feast,coin
"""
import sys
M=int(sys.argv[1]); H=int(sys.argv[2]); PRE=int(sys.argv[3]); SPREAD=int(sys.argv[4])
EX=set((sys.argv[5] if len(sys.argv)>5 else "").split(","))-{""}
ags=[f"c{i}" for i in range(M)]
slots=[f"s{i}" for i in range(H)]
o=[]
o.append(f"(define (problem village-M{M}-H{H}-{'pre' if PRE else 'raw'}-{'spread' if SPREAD else 'hub'}) (:domain rpg-world)")
o.append("  (:objects "+" ".join(ags)+" - agent  hub forest quarry mine field claypit water - location  "+" ".join(slots)+" - slot)")
o.append("  (:init")
# hub hosts all stations + buildsite + market
o.append("    (has-sawmill hub) (has-kiln hub) (has-forge hub) (has-anvil hub) (has-mason hub)")
o.append("    (has-loom hub) (has-hearth hub) (has-altar hub) (is-buildsite hub) (is-market hub)")
if SPREAD:
    o.append("    (is-forest forest) (is-quarry quarry) (is-mine mine) (is-field field) (is-claypit claypit) (is-water water)")
    for x in ["forest","quarry","mine","field","claypit","water"]:
        o.append(f"    (link hub {x}) (link {x} hub) (= (dist hub {x}) 2) (= (dist {x} hub) 2)")
else:
    # everything at the hub (no travel)
    o.append("    (is-forest hub) (is-quarry hub) (is-mine hub) (is-field hub) (is-claypit hub) (is-water hub)")
# craftsmen with rotating roles + the full toolkit
roles=["woodcutter","miner","mason-skill","smith","cook","mage"]
for i,a in enumerate(ags):
    o.append(f"    (at {a} hub) (has-axe {a}) (has-pick {a}) (has-shovel {a}) (has-hammer {a}) ({roles[i%len(roles)]} {a})")
# numeric fluents: all 0, then prestock if asked
import re,os
dom=open(os.path.join(os.path.dirname(__file__),"..","..","examples","rpg-world","domain.pddl")).read()
fi=dom.find("(:functions");d=0
for j in range(fi,len(dom)):
    if dom[j]=='(':d+=1
    elif dom[j]==')':
        d-=1
        if d==0:fe=j;break
fns=sorted(set(re.findall(r'\(([a-z][\w-]*)\)',dom[fi:fe+1])))
seed={"logs":40,"planks":0,"stone":40,"blocks":0,"clay":40,"bricks":0,"ore":20,"charcoal":20,"ingots":20,"food":20,"water":20,"fiber":10} if PRE else {}
for f in fns:
    if f=="dist":continue
    o.append(f"    (= ({f}) {seed.get(f,0)})")
o.append("  )")  # close :init
# goal
g=["(built-square)"] if H>0 else []
if "well" in EX: g.append("(well-dug)")
if "walls" in EX: g.append("(built-wall)")
if "feast" in EX: g.append("(feast-held)")
if "coin" in EX: g.append("(>= (coin) 3)")
if not g: g=["(>= (planks) 4)"]
o.append("  (:goal (and "+" ".join(g)+")))")
print("\n".join(o))
