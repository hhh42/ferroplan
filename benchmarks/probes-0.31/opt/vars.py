#!/usr/bin/env python3
"""Lane C.0 (a): the variable cover over the opt boards' cells.

For every cell of the optimal boards (as the crucible database lists them
for the baseline engine), run the engine with `FF_VARS_DEBUG=1` under a
short wall and a one-evaluation cap, and record the `vars:` line it prints
after grounding: mutex groups, the greedy disjoint cover, facts covered,
singletons, variables, synthesis seconds. The receipt Lane C opens on:
how many cells get a real multi-valued view, how many variables, and what
the synthesis costs against the 60 s wall.

    python3 benchmarks/probes-0.31/opt/vars.py --engine <baseline blake3 prefix> \
        --ff ~/ferroplan-0.31/target/release/ff --out benchmarks/probes-0.31/opt/vars.tsv [-j 4]

Never run beside a sweep (docs/roadmap-0.31.md's discipline).
"""

import argparse
import os
import re
import sqlite3
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

OPT_BOARDS = ["ipc-opt-2008-11", "ipc2014-opt", "ipc2018-opt", "ipc2023-opt", "ipc2026-opt", "ipc2026-opt-full", "ipc2023-numeric-opt"]
VARS_RE = re.compile(r"vars: (\d+) groups, (\d+) disjoint \((\d+) facts of (\d+) covered, largest (\d+)\), (\d+) singletons, (\d+) variables, ([0-9.]+)s")


def cells(db, engine, boards):
    q = """
    SELECT DISTINCT b.name, v.ipc, v.name, i.label
      FROM run r JOIN board b ON b.id = r.board_id
      JOIN instance i ON i.id = r.instance_id JOIN variant v ON v.id = i.variant_id
      JOIN engine e ON e.id = r.engine_id
     WHERE e.blake3 LIKE ? AND b.env = '{}'
    """
    return [row for row in db.execute(q, (engine + "%",)) if row[0] in boards]


def paths(corpus, ipc, variant, label):
    vdir = os.path.join(corpus, ipc, "domains", variant)
    prb = os.path.join(vdir, "instances", f"instance-{label}.pddl")
    dom = os.path.join(vdir, "domain.pddl")
    if not os.path.exists(dom):
        first = re.match(r"\d*", label).group(0)
        dom = os.path.join(vdir, "domains", f"domain-{first}.pddl")
    return dom, prb


def run(ff, dom, prb, wall):
    env = {k: v for k, v in os.environ.items() if not k.startswith("FF_")}
    env.update({"FF_VARS_DEBUG": "1", "FF_TIME_LIMIT": str(wall), "FF_MEM_BUDGET_GB": "6"})
    try:
        p = subprocess.run([ff, "-o", dom, "-f", prb, "--json", "--max-evaluated", "1", "--threads", "1"],
                           env=env, capture_output=True, text=True, timeout=wall + 30)
        err = p.stderr
    except subprocess.TimeoutExpired as e:
        err = (e.stderr or b"").decode() if isinstance(e.stderr, bytes) else (e.stderr or "")
    m = VARS_RE.search(err)
    return m.groups() if m else None


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--db", default=os.path.expanduser("~/.crucible/db/crucible.db"))
    ap.add_argument("--engine", required=True)
    ap.add_argument("--ff", required=True)
    ap.add_argument("--corpus", default=os.path.expanduser("~/ferroplan/benchmarks/.ipc-corpus"))
    ap.add_argument("--board", action="append", default=[])
    ap.add_argument("--wall", type=int, default=20)
    ap.add_argument("--out", required=True)
    ap.add_argument("-j", type=int, default=2)
    args = ap.parse_args()
    db = sqlite3.connect(f"file:{args.db}?immutable=1", uri=True)
    rows = cells(db, args.engine, args.board or OPT_BOARDS)
    if not rows:
        sys.exit("no opt cells for that engine")
    print(f"{len(rows)} cells", file=sys.stderr)

    def one(row):
        board, ipc, variant, label = row
        dom, prb = paths(args.corpus, ipc, variant, label)
        if not (os.path.exists(dom) and os.path.exists(prb)):
            return (board, variant, label, "missing")
        g = run(args.ff, dom, prb, args.wall)
        return (board, variant, label, "\t".join(g) if g else "no-vars-line")

    os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)
    with open(args.out, "w") as f, ThreadPoolExecutor(max_workers=args.j) as ex:
        f.write("board\tvariant\tlabel\tgroups\tdisjoint\tcovered\tfacts\tlargest\tsingletons\tvariables\tsecs\n")
        for n, res in enumerate(ex.map(one, rows), 1):
            f.write("\t".join(res) + "\n")
            f.flush()
            if n % 50 == 0:
                print(f"  {n}/{len(rows)}", file=sys.stderr)
    print(f"wrote {args.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
