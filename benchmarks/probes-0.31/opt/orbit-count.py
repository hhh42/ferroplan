#!/usr/bin/env python3
"""C.s's receipt (0.31): how many of the banked-unsolved opt cells carry an
object-symmetry orbit TODAY, and how large the group is -- read off the
`orbits:` note the optimal path now writes (api.rs, 2026-11-01).

BOX TIME: runs `ff --mode optimal` on every cell in `unsolved-rows.txt`
with a 3 s wall (grounding + detection + a few seconds of A*; the plan is
not the point, the note is). ~1,100 cells x <= 3 s. Never beside a sweep.

    FERROPLAN_IPC_CORPUS=~/ferroplan/benchmarks/.ipc-corpus \
    python3 benchmarks/probes-0.31/opt/orbit-count.py [--ff target/release/ff] [--wall 3]

Writes `orbit-count.csv` beside the rows file and prints the tally.
"""
import argparse
import collections
import csv
import json
import os
import re
import sqlite3
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
CORPUS = os.environ.get("FERROPLAN_IPC_CORPUS") or os.path.join(ROOT, "benchmarks", ".ipc-corpus")


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
    for f in os.listdir(idir):
        if label_of(f) == label:
            shared = os.path.join(vdir, "domain.pddl")
            dom = shared if os.path.isfile(shared) else os.path.join(vdir, "domains", f"domain-{digit_groups(f)[0]}.pddl")
            return dom, os.path.join(idir, f)
    raise FileNotFoundError(f"{ipc}/{variant}/{label}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ff", default=os.path.join(ROOT, "target", "release", "ff"))
    ap.add_argument("--rows", default=os.path.join(HERE, "unsolved-rows.txt"))
    ap.add_argument("--db", default=os.path.expanduser("~/.crucible/db/crucible.db"))
    ap.add_argument("--wall", type=float, default=3.0)
    ap.add_argument("--limit", type=int, default=0, help="first N rows only (a smoke run)")
    args = ap.parse_args()
    db = sqlite3.connect(f"file:{args.db}?immutable=1", uri=True)
    ipc_of = dict(db.execute("SELECT name, ipc FROM variant"))
    rows = [l.strip() for l in open(args.rows) if l.strip() and not l.startswith("#")]
    if args.limit:
        rows = rows[: args.limit]
    out = os.path.join(HERE, "orbit-count.csv")
    tally = collections.Counter()
    by_variant = collections.defaultdict(collections.Counter)
    env = dict(os.environ, FF_TIME_LIMIT=str(args.wall))
    with open(out, "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(["variant", "label", "orbits", "members", "log10_group", "note"])
        for n, row in enumerate(rows, 1):
            variant, label = row.rsplit("/", 1)
            try:
                dom, prb = resolve(ipc_of[variant], variant, label)
            except (KeyError, FileNotFoundError) as e:
                w.writerow([variant, label, "", "", "", f"unresolved: {e}"])
                tally["unresolved"] += 1
                continue
            try:
                p = subprocess.run(
                    ["perl", "-e", "alarm shift; exec @ARGV", str(int(args.wall) + 20),
                     args.ff, "--mode", "optimal", "--json", "-o", dom, "-f", prb],
                    capture_output=True, text=True, env=env,
                )
                sol = json.loads(p.stdout) if p.stdout.strip().startswith("{") else {}
            except Exception as e:  # noqa: BLE001
                sol = {"notes": [f"run failed: {e}"]}
            note = next((x for x in sol.get("notes", []) if x.startswith("orbits:")), "")
            if note.startswith("orbits: none") or not note:
                kind = "none" if note else "no-note"
                w.writerow([variant, label, 0, "", "", note or "(no orbit note: ground/parse failed or rejected)"])
            else:
                m = re.search(r"(\d+) orbit\(s\), members \[([^\]]*)\], group ~10\^([\d.]+)", note)
                members = m.group(2) if m else ""
                lg = float(m.group(3)) if m else 0.0
                kind = "group>=1e3" if lg >= 3 else "group<1e3"
                w.writerow([variant, label, m.group(1) if m else "", members, lg, note])
            tally[kind] += 1
            by_variant[variant][kind] += 1
            if n % 50 == 0:
                print(f"  {n}/{len(rows)} {dict(tally)}", file=sys.stderr)
    print(f"cells {len(rows)}: {dict(tally)}")
    print("\nvariants with an orbit on some unsolved cell (cells with group>=1e3 / <1e3 / none):")
    for v, c in sorted(by_variant.items(), key=lambda kv: -(kv[1]["group>=1e3"] + kv[1]["group<1e3"])):
        if c["group>=1e3"] + c["group<1e3"]:
            print(f"  {c['group>=1e3']:3d} / {c['group<1e3']:3d} / {c['none'] + c['no-note']:3d}  {v}")
    print(f"\nwrote {out}")


if __name__ == "__main__":
    main()
