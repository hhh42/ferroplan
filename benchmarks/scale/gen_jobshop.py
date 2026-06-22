#!/usr/bin/env python3
"""Generate a scaling ladder of job-shop problems for examples/jobshop/domain.pddl.
N jobs x K stages x M machines, classic rotated routes. Writes examples/jobshop/*.pddl.
The point: job-shop has none of rpg-world's heuristic killers (jobs are independent
linear chains that never converge) — its only wall is GROUNDING size (~N*K*M operate
instances). This ladder brackets that wall so it is reproducible."""
import os
ROOT = os.path.join(os.path.dirname(__file__), "..", "..")
OUT = os.path.join(ROOT, "examples/jobshop")

def problem(name, N, K, M, blurb):
    jobs = [f"j{i}" for i in range(1, N + 1)]
    stages = [f"s{k}" for k in range(1, K + 1)]
    machines = [f"m{m}" for m in range(1, M + 1)]
    o = [f";; {name}: {N} jobs x {K} stages x {M} machines (~{N*K*M} operate groundings). {blurb}",
         f"(define (problem {name}) (:domain jobshop)",
         f"  (:objects {' '.join(jobs)} - job  {' '.join(stages)} - stage  {' '.join(machines)} - machine)",
         "  (:init"]
    for k in range(K - 1):
        o.append(f"    (next s{k+1} s{k+2})")
    for i in range(1, N + 1):
        o.append(f"    (at-stage j{i} s1) (final-stage j{i} s{K})")
        for k in range(1, K + 1):
            m = ((i + k) % M) + 1                      # rotated route
            o.append(f"    (route j{i} s{k} m{m}) (= (proctime j{i} s{k}) {2 + ((i + k) % 3)})")
    for m in machines:
        o.append(f"    (free {m})")
    o.append("  )")
    o.append("  (:goal (and " + " ".join(f"(job-complete {j})" for j in jobs) + ")))")
    open(os.path.join(OUT, name + ".pddl"), "w").write("\n".join(o) + "\n")

# clear stale problem files (keep the domain)
for fn in os.listdir(OUT):
    if fn.endswith(".pddl") and fn != "domain.pddl":
        os.remove(os.path.join(OUT, fn))

# the ladder: tiny -> grounding wall
problem("p1", 1, 2, 2, "smoke")
problem("p2", 2, 2, 2, "two jobs, crossed routes")
problem("p3", 3, 3, 3, "classic 3x3")
problem("p4", 4, 3, 3, "contention: more jobs than machines")
problem("p5", 5, 5, 5, "5x5")
problem("s10", 10, 10, 10, "1k groundings")
problem("s20", 20, 10, 10, "2k")
problem("s50", 50, 10, 10, "5k")
problem("s50w", 50, 20, 20, "20k")
problem("s100", 100, 20, 20, "40k — near the wall")
problem("s100g", 100, 30, 30, "90k — over the grounding wall")
print("wrote jobshop ladder:", sorted(f for f in os.listdir(OUT) if f != "domain.pddl"))
