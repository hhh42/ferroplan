#!/usr/bin/env python3
"""Self-check ferroplan's temporal plans with ferroplan's OWN built-in validator.

For each durative (domain, problem) it solves with `--mode temporal`, then feeds
the emitted plan back to `ff --validate` (which replays it under ferroplan's own
sequential/epsilon semantics — NOT VAL's strict PDDL2.1). A plan that the planner
reports as "found legal" but that the validator rejects is a real discrepancy
worth a look (e.g. an epsilon-separation print artifact at a tight
produce-at-end / consume-at-start boundary).

This is both a demonstration of the new validator and a lightweight regression
guard for the temporal pipeline.

Usage: python3 benchmarks/encoding-ab/selfcheck.py
Env:   FF=/abs/ff   ROOT=/abs/repo-root
"""
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
FERRO = os.path.dirname(os.path.dirname(HERE))
ROOT = os.environ.get("ROOT", os.path.dirname(FERRO))
FF = os.environ.get("FF", os.path.join(FERRO, "target", "release", "ff"))

# Real durative content shipped with ferroplan + a couple generated cases.
CASES = [
    ("rpg/1worker",      "ferroplan/examples/rpg/domain.pddl",     "ferroplan/examples/rpg/build-1worker.pddl"),
    ("rpg/3workers",     "ferroplan/examples/rpg/domain.pddl",     "ferroplan/examples/rpg/build-3workers.pddl"),
    ("village/onesite",  "ferroplan/examples/village/domain.pddl", "ferroplan/examples/village/onesite.pddl"),
    ("rpg-world/grove",  "ferroplan/examples/rpg-world/domain.pddl", "ferroplan/examples/rpg-world/suite/grove.pddl"),
    ("rpg-world/woodlot","ferroplan/examples/rpg-world/domain.pddl", "ferroplan/examples/rpg-world/suite/woodlot.pddl"),
]


def solve_temporal(dom, prob):
    r = subprocess.run([FF, "-o", dom, "-f", prob, "--mode", "temporal"],
                       capture_output=True, text=True, timeout=120)
    lines = [ln for ln in r.stdout.splitlines() if ln[:1].isdigit() and ln.rstrip().endswith("]")]
    return lines


def validate(dom, prob, plan_lines):
    with tempfile.NamedTemporaryFile("w", suffix=".plan", delete=False) as f:
        f.write("\n".join(plan_lines) + "\n")
        planf = f.name
    try:
        r = subprocess.run([FF, "-o", dom, "-f", prob, "--validate", planf],
                           capture_output=True, text=True, timeout=120)
        return r.stdout.strip()
    finally:
        os.unlink(planf)


def main():
    print(f"{'case':22} {'plan':>5}  validator")
    print("-" * 60)
    n_ok = n_bad = 0
    for label, dom, prob in CASES:
        dpath, ppath = os.path.join(ROOT, dom), os.path.join(ROOT, prob)
        if not (os.path.exists(dpath) and os.path.exists(ppath)):
            print(f"{label:22} {'--':>5}  SKIP (missing)"); continue
        plan = solve_temporal(dpath, ppath)
        if not plan:
            print(f"{label:22} {'--':>5}  (no plan)"); continue
        verdict = validate(dpath, ppath, plan)
        ok = verdict.startswith("Plan valid")
        n_ok += ok; n_bad += (not ok)
        print(f"{label:22} {len(plan):>5}  {verdict}")
    print("-" * 60)
    print(f"{n_ok} valid, {n_bad} flagged by the built-in validator")
    if n_bad:
        print("\nNote: a flagged plan is one the planner emitted but that does not replay\n"
              "under ferroplan's own semantics — typically an epsilon-separation print\n"
              "artifact at a same-timestamp produce/consume boundary. See README.md.")


if __name__ == "__main__":
    main()
