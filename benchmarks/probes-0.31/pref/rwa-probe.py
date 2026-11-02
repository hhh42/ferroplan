#!/usr/bin/env python3
"""Lane E's pre-registered probe (0.31, docs/roadmap-0.31.md): restarting
weighted A* over the compiled preference task, `FF_PREF_RWA=1` against
unset, on the ten worst cells of the three IPC-5 preference boards --
the cells where SGPlan5 reaches metric 0 and the banked 0.30.0 plan does
not. Solo at the board wall (60 s, one thread, as the boards run).

A lane opens only if FIVE of the ten move toward SGPlan5's metric (0);
then the three boards through the crucible with `--env FF_PREF_RWA=1`.

BOX TIME: 20 runs x <= 60 s, serial. Never beside a sweep.

    FERROPLAN_IPC_CORPUS=~/ferroplan/benchmarks/.ipc-corpus \
    python3 benchmarks/probes-0.31/pref/rwa-probe.py [--ff target/release/ff] [--wall 60]
"""
import argparse
import json
import os
import re
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
CORPUS = os.environ.get("FERROPLAN_IPC_CORPUS") or os.path.join(ROOT, "benchmarks", ".ipc-corpus")

# (variant, label, banked 0.30.0 metric); SGPlan5's metric is 0 on every one
# (benchmarks/standings.py over benchmarks/IPC5-results.tgz, 2026-11-01).
CELLS = [
    ("trucks-preferences-simple", "6", 6.0),
    ("trucks-preferences-simple", "14", 6.0),
    ("trucks-preferences-simple", "15", 5.0),
    ("trucks-preferences-simple", "16", 11.0),
    ("trucks-preferences-simple", "17", 11.0),
    ("trucks-preferences-qualitative", "4", 2.0),
    ("trucks-preferences-qualitative", "5", 2.0),
    ("tpp-preferences-complex", "3", 12.0),
    ("openstacks-preferences-simple", "15", 6.0),
    ("openstacks-preferences-simple", "18", 31.0),
]


def digit_groups(name):
    return re.findall(r"\d+", name)


def label_of(file):
    g = digit_groups(file)
    if not g:
        return None
    return str(int(g[0])) if len(g) == 1 else "_".join(g)


def resolve(variant, label):
    vdir = os.path.join(CORPUS, "ipc-2006", "domains", variant)
    idir = os.path.join(vdir, "instances")
    for f in sorted(os.listdir(idir)):
        if label_of(f) == label:
            shared = os.path.join(vdir, "domain.pddl")
            dom = shared if os.path.isfile(shared) else os.path.join(vdir, "domains", f"domain-{digit_groups(f)[0]}.pddl")
            return dom, os.path.join(idir, f)
    raise FileNotFoundError(f"{variant}/{label}")


def run(ff, dom, prb, wall, env_extra):
    env = dict(os.environ, FF_TIME_LIMIT=str(wall))
    env.pop("FF_PREF_RWA", None)  # the plain arm is unset, whatever the shell says
    env.update(env_extra)
    t0 = time.time()
    p = subprocess.run(
        ["perl", "-e", "alarm shift; exec @ARGV", str(int(wall) + 30),
         ff, "--json", "--threads", "1", "-o", dom, "-f", prb],
        capture_output=True, text=True, env=env,
    )
    secs = time.time() - t0
    try:
        sol = json.loads(p.stdout)
    except ValueError:
        return {"solved": False, "metric": None, "length": None, "secs": secs, "note": (p.stderr or "")[-200:]}
    plan = sol.get("plan") or {}
    return {
        "solved": bool(sol.get("solved")),
        "metric": plan.get("metric"),
        "length": len(plan.get("steps") or plan.get("actions") or []) or plan.get("length"),
        "secs": secs,
        "note": "; ".join(n for n in sol.get("notes", []) if "restart" in n or "bnb" in n or "pref" in n)[:160],
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ff", default=os.path.join(ROOT, "target", "release", "ff"))
    ap.add_argument("--wall", type=float, default=60.0)
    ap.add_argument("--sched", default="1", help="FF_PREF_RWA value for the armed arm")
    args = ap.parse_args()
    print(f"ff {args.ff} ({time.ctime(os.path.getmtime(args.ff))}), wall {args.wall:.0f} s, FF_PREF_RWA={args.sched}")
    print(f"{'cell':38} {'banked':>7} {'plain':>7} {'rwa':>7}  {'plain s':>8} {'rwa s':>7}  verdict")
    moved = 0
    for variant, label, banked in CELLS:
        dom, prb = resolve(variant, label)
        a = run(args.ff, dom, prb, args.wall, {})
        b = run(args.ff, dom, prb, args.wall, {"FF_PREF_RWA": args.sched})
        ma = a["metric"] if a["solved"] else None
        mb = b["metric"] if b["solved"] else None
        if mb is not None and (ma is None or mb < ma):
            verdict = "toward"
            moved += 1
        elif mb is None and ma is not None:
            verdict = "LOST"
        elif mb is not None and ma is not None and mb > ma:
            verdict = "away"
        else:
            verdict = "level"
        fmt = lambda m: "-" if m is None else f"{m:g}"
        print(f"{variant + '/' + label:38} {banked:7g} {fmt(ma):>7} {fmt(mb):>7}  {a['secs']:8.1f} {b['secs']:7.1f}  {verdict}", flush=True)
        if a["note"] or b["note"]:
            print(f"    plain: {a['note']}\n    rwa:   {b['note']}", file=sys.stderr)
    print(f"\nmoved toward SGPlan5 on {moved} of {len(CELLS)} (lane opens at 5)")


if __name__ == "__main__":
    main()
