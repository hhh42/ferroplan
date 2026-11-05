#!/usr/bin/env python3
"""Phase 0.6's receipt (0.31): the grounder's bytes per op on the grounding
class -- the ~50 rows whose wall the grounder alone spends (folding,
caldera, organic-synthesis, storage-complex). One `FF_GROUND_DEBUG=1` run a
cell at the board wall, one thread; prints the engine's `ground:` line (ops,
facts, conditional-effect rows, resident bytes, bytes/op) beside peak RSS,
or the reason there is none (the grounder did not finish inside the wall).
A lane opens only if the receipt names a fat field.

BOX TIME: 15 cells x <= 60 s, serial. Never beside a sweep.

    FERROPLAN_IPC_CORPUS=/Volumes/Data/Projects/ferroplan/benchmarks/.ipc-corpus \
    python3 benchmarks/probes-0.31/ground/ground-profile.py
"""
import argparse
import os
import re
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))

CORPUS = os.environ.get("FERROPLAN_IPC_CORPUS") or os.path.join(ROOT, "benchmarks", ".ipc-corpus")

CELLS = (
    [("ipc-2023", "folding-satisficing", str(i)) for i in range(1, 6)]
    + [("ipc-2018", "caldera-sequential-satisficing", str(i)) for i in range(3, 7)]
    + [("ipc-2018", "organic-synthesis-sequential-satisficing", str(i)) for i in range(5, 9)]
    + [("ipc-2006", "storage-preferences-complex", str(i)) for i in (19, 20)]
)


def resolve(ipc, variant, label):
    vdir = os.path.join(CORPUS, ipc, "domains", variant)
    idir = os.path.join(vdir, "instances")
    for f in sorted(os.listdir(idir)):
        g = re.findall(r"\d+", f)
        if g and str(int(g[0])) == label and len(g) == 1:
            shared = os.path.join(vdir, "domain.pddl")
            dom = shared if os.path.isfile(shared) else os.path.join(vdir, "domains", f"domain-{g[0]}.pddl")
            return dom, os.path.join(idir, f)
    raise FileNotFoundError(f"{ipc}/{variant}/{label}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ff", default=os.path.join(ROOT, "target", "release", "ff"))
    ap.add_argument("--wall", type=float, default=60.0)
    args = ap.parse_args()
    print(f"ff {args.ff} ({time.ctime(os.path.getmtime(args.ff))}), wall {args.wall:.0f} s, one thread")
    for ipc, variant, label in CELLS:
        dom, prb = resolve(ipc, variant, label)
        env = dict(os.environ, FF_TIME_LIMIT=str(args.wall), FF_GROUND_DEBUG="1")
        t0 = time.time()
        p = subprocess.run(
            ["perl", "-e", "alarm shift; exec @ARGV", str(int(args.wall) + 30),
             "/usr/bin/time", "-l", args.ff, "--json", "--threads", "1", "-o", dom, "-f", prb],
            capture_output=True, text=True, env=env,
        )
        secs = time.time() - t0
        m = re.search(r"(\d+)\s+maximum resident set size", p.stderr)
        rss = int(m.group(1)) / 2**30 if m else float("nan")
        g = next((ln.strip() for ln in p.stderr.splitlines() if ln.strip().startswith("ground:")), None)
        print(f"{variant}/{label:3} {secs:6.1f}s peak {rss:5.2f} GiB  {g or '(no ground line: the grounder did not finish)'}", flush=True)


if __name__ == "__main__":
    main()
