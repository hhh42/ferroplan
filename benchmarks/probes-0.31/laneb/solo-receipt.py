#!/usr/bin/env python3
"""Lane B's owed receipts (0.31, the ladder read of 2026-11-01): the cells
the new defaults LOST against the banked 0.30.0 rows, each run solo at the
board wall under four arms, so the record says WHICH knob spends the
memory or the wall:

    default           -- the 0.31 ladder (novelty-first LAMA + refreshing driver)
    lamanov-off       -- FF_LAMA_NOV=0
    driver-plain      -- FF_NOV_REFRESH=0 FF_NOV_FORGET=0
    all-off           -- the 0.30 ladder by the hatches

Prints solved / seconds / peak RSS (`/usr/bin/time -l`) / the rung that
solved it. BOX TIME: cells x 4 x <= 60 s, serial. Never beside a sweep.

    FERROPLAN_IPC_CORPUS=~/ferroplan/benchmarks/.ipc-corpus \
    python3 benchmarks/probes-0.31/laneb/solo-receipt.py [cells...]
"""
import argparse
import json
import os
import re
import sqlite3
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
CORPUS = os.environ.get("FERROPLAN_IPC_CORPUS") or os.path.join(ROOT, "benchmarks", ".ipc-corpus")

DEFAULT_CELLS = [
    "woodworking-sequential-satisficing/15",
    "woodworking-sequential-satisficing/16",
    "child-snack-sequential-satisficing/7",
    "child-snack-sequential-agile/7",
    "thoughtful-sequential-satisficing/9",
    "thoughtful-sequential-agile/9",
    "termes-sequential-satisficing/5",
    "sokoban-sequential-satisficing/20",
    "pipesworld-propositional-strips/28",
    "pipesworld-propositional-strips/45",
]
ARMS = [
    ("default", {}),
    ("lamanov-off", {"FF_LAMA_NOV": "0"}),
    ("driver-plain", {"FF_NOV_REFRESH": "0", "FF_NOV_FORGET": "0"}),
    ("all-off", {"FF_LAMA_NOV": "0", "FF_NOV_REFRESH": "0", "FF_NOV_FORGET": "0"}),
]


def digit_groups(name):
    return re.findall(r"\d+", name)


def label_of(file):
    g = digit_groups(file)
    if not g:
        return None
    return str(int(g[0])) if len(g) == 1 else "_".join(g)


def resolve(ipc, variant, label):
    vdir = os.path.join(CORPUS, ipc, "domains", variant)
    idir = os.path.join(vdir, "instances")
    for f in sorted(os.listdir(idir)):
        if label_of(f) == label:
            shared = os.path.join(vdir, "domain.pddl")
            dom = shared if os.path.isfile(shared) else os.path.join(vdir, "domains", f"domain-{digit_groups(f)[0]}.pddl")
            return dom, os.path.join(idir, f)
    raise FileNotFoundError(f"{ipc}/{variant}/{label}")


def run(ff, dom, prb, wall, env_extra):
    env = dict(os.environ, FF_TIME_LIMIT=str(wall))
    for k in ("FF_LAMA_NOV", "FF_NOV_REFRESH", "FF_NOV_FORGET"):
        env.pop(k, None)
    env.update(env_extra)
    t0 = time.time()
    p = subprocess.run(
        ["perl", "-e", "alarm shift; exec @ARGV", str(int(wall) + 30),
         "/usr/bin/time", "-l", ff, "--json", "--threads", "1", "-o", dom, "-f", prb],
        capture_output=True, text=True, env=env,
    )
    secs = time.time() - t0
    m = re.search(r"(\d+)\s+maximum resident set size", p.stderr)
    rss = int(m.group(1)) / 2**30 if m else float("nan")
    try:
        sol = json.loads(p.stdout)
    except ValueError:
        return False, secs, rss, "(no json)", None
    rung = next((n for n in sol.get("notes", []) if n.startswith("rung: solved by")), "")
    plan = sol.get("plan") or {}
    return bool(sol.get("solved")), secs, rss, rung.replace("rung: solved by ", ""), plan.get("length")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("cells", nargs="*", default=DEFAULT_CELLS)
    ap.add_argument("--ff", default=os.path.join(ROOT, "target", "release", "ff"))
    ap.add_argument("--db", default=os.path.expanduser("~/.crucible/db/crucible.db"))
    ap.add_argument("--wall", type=float, default=60.0)
    args = ap.parse_args()
    db = sqlite3.connect(f"file:{args.db}?immutable=1", uri=True)
    ipc_of = dict(db.execute("SELECT name, ipc FROM variant"))
    print(f"ff {args.ff} ({time.ctime(os.path.getmtime(args.ff))}), wall {args.wall:.0f} s, one thread")
    print(f"{'cell':40} {'arm':13} {'solved':>6} {'secs':>6} {'GiB':>5} {'len':>4}  rung")
    for cell in args.cells:
        variant, label = cell.rsplit("/", 1)
        dom, prb = resolve(ipc_of[variant], variant, label)
        for arm, env in ARMS:
            solved, secs, rss, rung, length = run(args.ff, dom, prb, args.wall, env)
            print(f"{cell:40} {arm:13} {str(solved):>6} {secs:6.1f} {rss:5.2f} {str(length or '-'):>4}  {rung}", flush=True)
        print()


if __name__ == "__main__":
    main()
