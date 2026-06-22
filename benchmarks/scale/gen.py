#!/usr/bin/env python3
"""Generate a scaling rpg-world problem to stress grounding + the reachability
axiom. Usage: gen.py N_LOCATIONS N_AGENTS [EXTRA_EDGES_PER_NODE] > problem.pddl"""
import sys
N = int(sys.argv[1]); M = int(sys.argv[2]); E = int(sys.argv[3]) if len(sys.argv)>3 else 2
locs = [f"L{i}" for i in range(N)]
ags  = [f"A{i}" for i in range(M)]
out = []
out.append(f"(define (problem scale-{N}x{M}) (:domain rpg-world)")
out.append("  (:objects " + " ".join(locs) + " - location " + " ".join(ags) + " - agent s1 - slot)")
out.append("  (:init")
# clustered stations + sites at L0 so a small goal is solvable
out.append("    (is-forest L0) (has-sawmill L0) (has-kiln L0)")
for a in ags: out.append(f"    (at {a} L0) (woodcutter {a}) (has-axe {a})")
# connected chain + E deterministic extra forward edges per node (static map)
links=[]
for i in range(N-1):
    links.append((i,i+1)); links.append((i+1,i))
    for k in range(1,E+1):
        j=(i*7+k*13)%N
        if j!=i: links.append((i,j))
for (i,j) in links:
    out.append(f"    (link L{i} L{j}) (= (dist L{i} L{j}) {1+((i+j)%5)})")
# initialize every resource fluent the domain references (read them from the domain)
import re,os
dom=open(os.path.join(os.path.dirname(__file__),"..","..","examples","rpg-world","domain.pddl")).read()
fi=dom.find("(:functions"); 
depth=0
for j in range(fi,len(dom)):
    if dom[j]=='(':depth+=1
    elif dom[j]==')':
        depth-=1
        if depth==0: fend=j; break
fns=set(re.findall(r'\(([a-z][\w-]*)\)', dom[fi:fend+1]))  # 0-ary fluents only
for f in sorted(fns):
    if f!="dist": out.append(f"    (= ({f}) 0)")
out.append("    (= (logs) 100))")  # seed logs so the goal is reachable
out.append("  (:goal (>= (planks) 2)))")  # 1 saw at L0; grounding dominates
print("\n".join(out))
