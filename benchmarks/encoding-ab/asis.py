#!/usr/bin/env python3
"""Part A — the *confounded* "as-is" comparison of the encodings that already exist
in the repo, before the controlled matched experiment (run_experiment.py).

CAVEAT (printed in the output too): these domains are NOT a fair head-to-head.
  - generic side   = the original `Prohibited` domain (instantaneous inventory/
                     consumption; one `consume` action over (consumable ?t ?verb)
                     constants) -> classical FF path, reports evaluated_states.
  - specific side  = the durative crafting domains rpg / village / rpg-world
                     -> temporal mode, reports makespan (evaluated_states = 0).
Different CONTENT and different planner MODE, so this measures content+mode, not
encoding style. It is a raw data point only; the verdict is in RESULTS.md.

Usage: python3 benchmarks/encoding-ab/asis.py
Env:   FF=/abs/path/to/ff   ROOT=/abs/path/to/repo-root (default: three levels up)
"""
import json
import os
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
FERRO = os.path.dirname(os.path.dirname(HERE))          # ferroplan/
ROOT = os.environ.get("ROOT", os.path.dirname(FERRO))   # repo root (/Volumes/M4Data/ff)
FF = os.environ.get("FF", os.path.join(FERRO, "target", "release", "ff"))

# (label, style, domain, problem). domain/problem are repo-relative.
CASES = [
    # generic / data-driven, instantaneous (classical FF path)
    ("Prohibited/consume", "generic-inst",
     "planner/src/resources/simple.pddl",
     "ferroplan/benchmarks/encoding-ab/asis/prohibited-consume.pddl"),
    ("Prohibited/simple(shipped)", "generic-inst",
     "planner/src/resources/simple.pddl", "planner/bin/simple_problem.pddl"),
    # action-specific, durative (temporal path)
    ("rpg/1worker", "specific-durative",
     "ferroplan/examples/rpg/domain.pddl", "ferroplan/examples/rpg/build-1worker.pddl"),
    ("rpg/3workers", "specific-durative",
     "ferroplan/examples/rpg/domain.pddl", "ferroplan/examples/rpg/build-3workers.pddl"),
    ("village/onesite", "specific-durative",
     "ferroplan/examples/village/domain.pddl", "ferroplan/examples/village/onesite.pddl"),
    ("village/graph", "specific-durative",
     "ferroplan/examples/village/domain.pddl", "ferroplan/examples/village/graph.pddl"),
    ("rpg-world/woodlot", "specific-durative",
     "ferroplan/examples/rpg-world/domain.pddl", "ferroplan/examples/rpg-world/suite/woodlot.pddl"),
    ("rpg-world/grove", "specific-durative",
     "ferroplan/examples/rpg-world/domain.pddl", "ferroplan/examples/rpg-world/suite/grove.pddl"),
    ("rpg-world/woodline", "specific-durative",
     "ferroplan/examples/rpg-world/domain.pddl", "ferroplan/examples/rpg-world/contracts/woodline.pddl"),
]


def solve(domain, problem, timeout=60):
    cmd = [FF, "-o", domain, "-f", problem, "--json", "--threads", "1"]
    t = time.perf_counter()
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        ms = (time.perf_counter() - t) * 1000
        d = json.loads(r.stdout) if r.stdout.strip().startswith("{") else None
    except subprocess.TimeoutExpired:
        return {"solved": False, "note": "timeout", "ms": timeout * 1000}
    if not d:
        return {"solved": False, "note": "error", "ms": ms}
    pl = d.get("plan") or {}
    st = d.get("statistics") or {}
    return {"solved": bool(d.get("solved")), "evaluated": st.get("evaluated_states"),
            "length": pl.get("length"), "makespan": pl.get("makespan"),
            "ga": st.get("grounded_actions"), "ms": round(ms, 1)}


def main():
    print("# Part A — as-is (CONFOUNDED) comparison\n")
    print("WARNING: different content AND different mode (generic=instantaneous "
          "consumption, specific=durative crafting). Raw data point only — not the verdict.\n")
    hdr = f"{'case':24} {'style':18} {'solved':7} {'eval':>9} {'len':>5} {'makespan':>9} {'g.acts':>7} {'ms':>8}"
    print(hdr)
    print("-" * len(hdr))
    for label, style, dom, prob in CASES:
        dpath, ppath = os.path.join(ROOT, dom), os.path.join(ROOT, prob)
        if not (os.path.exists(dpath) and os.path.exists(ppath)):
            print(f"{label:24} {style:18} {'SKIP (missing file)'}")
            continue
        r = solve(dpath, ppath)
        ev = "-" if r.get("evaluated") in (None, 0) else r["evaluated"]
        mk = "-" if r.get("makespan") is None else f"{r['makespan']:g}"
        print(f"{label:24} {style:18} {str(r['solved']):7} {str(ev):>9} "
              f"{str(r.get('length','-')):>5} {mk:>9} {str(r.get('ga','-')):>7} {r.get('ms',0):>8.0f}")
    print("\nReading: the generic (instantaneous) row reports `eval` (node expansions); "
          "the specific (durative) rows report `makespan` (eval is 0 on the temporal path). "
          "Because content and mode both differ, these numbers are NOT directly comparable — "
          "see RESULTS.md for the controlled matched experiment.")


if __name__ == "__main__":
    main()
