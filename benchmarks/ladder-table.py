#!/usr/bin/env python3
"""The ladder's time table, learned from the per-rung matrix (0.31 Lane A).

Fast Downward Stone Soup's allocator (Helmert, Röger & Karpas 2011,
`docs/papers/helmert-etal-2011-stone-soup.pdf`): from a (cell x rung)
matrix of solo solve times, hill-climb over portfolios by adding
`granularity` seconds to one rung at a time, each step the rung whose
extra seconds solve the most new cells, until the wall is spent; then
reduce each rung's seconds to the least that keeps the score.

The matrix rows are the crucible's: boards whose `env` carries
`FF_RUNG_ONLY=<rung>` (one subset per rung, `benchmarks/matrix-sample.py`
writes the commands), on one engine. A rung's time on a cell is its row's
`time_secs` less the `ground: N.NNNs` note (the ladder grounds once);
an unsolved row is infinity. Rows the referee did not bank are not in the
matrix at all (the `run.state` filter), so a hot or demoted timing never
shapes a slice.

THE NUMBER IS HELD OUT. For every variant V the table is learned on every
other variant and scored on V (leave-one-domain-out); the held-out score,
summed, is the receipt beside the in-sample one -- and the table written
is the one learned on everything. A rung the matrix never ran keeps its
engine default (the table omits it).

    python3 benchmarks/ladder-table.py --engine <blake3 prefix> \\
        --out benchmarks/probes-0.31/matrix/table.txt [--wall 60] [--granularity 1]
"""

import argparse
import math
import os
import re
import sqlite3
import sys
from collections import defaultdict

LADDER_ORDER = ["ehc", "novlight", "lama", "driver", "bfs"]
# The fallback takes whatever the table leaves; its seconds are learned
# (so the allocator can prefer it) but not written.
IMPLICIT = {"bfs"}
GROUND_RE = re.compile(r'"?ground: ([0-9.]+)s"?')


def open_db(path):
    return sqlite3.connect(f"file:{path}?immutable=1", uri=True)


def load_matrix(db, engine, boards):
    """t[cell][rung] = solo search seconds (float) or inf; cells keyed
    `variant/label`; `domain[cell]` = variant (the held-out unit)."""
    q = """
    SELECT b.name, b.env, v.name, i.label, r.solved, r.time_secs, r.notes_json, r.timing_quality
      FROM run r
      JOIN board b ON b.id = r.board_id
      JOIN instance i ON i.id = r.instance_id
      JOIN variant v ON v.id = i.variant_id
      JOIN engine e ON e.id = r.engine_id
     WHERE e.blake3 LIKE ? AND b.env LIKE '%FF_RUNG_ONLY%' AND r.state = 'done'
    """
    t = defaultdict(dict)
    domain = {}
    counts = defaultdict(lambda: [0, 0, 0])  # rung -> [rows, solved, dirty]
    for board, env, variant, label, solved, secs, notes, quality in db.execute(q, (engine + "%",)):
        if boards and board not in boards:
            continue
        m = re.search(r'"FF_RUNG_ONLY"\s*:\s*"([a-z-]+)"', env or "")
        if not m:
            continue
        rung = m.group(1)
        cell = f"{variant}/{label}"
        domain[cell] = variant
        c = counts[rung]
        c[0] += 1
        if quality == "dirty":
            c[2] += 1
        if solved:
            c[1] += 1
            ground = 0.0
            gm = GROUND_RE.search(notes or "")
            if gm:
                ground = float(gm.group(1))
            val = max(0.0, (secs or 0.0) - ground)
            # Several attempts: the fastest banked one is the rung's time.
            t[cell][rung] = min(val, t[cell].get(rung, math.inf))
        else:
            t[cell].setdefault(rung, math.inf)
    return t, domain, counts


def coverage(t, cells, alloc):
    return sum(1 for c in cells if any(t[c].get(r, math.inf) <= s for r, s in alloc.items() if s > 0))


def stone_soup(t, cells, rungs, wall, gran):
    """Helmert et al. 2011, Figure 1: hill-climb then reduce."""
    alloc = {r: 0.0 for r in rungs}
    steps = int(wall // gran)
    for _ in range(steps):
        best, best_r = -1, None
        for r in rungs:  # ties: the earliest rung in the fixed order
            trial = dict(alloc)
            trial[r] += gran
            sc = coverage(t, cells, trial)
            if sc > best:
                best, best_r = sc, r
        alloc[best_r] += gran
    score = coverage(t, cells, alloc)
    # Reduce: the least whole-granularity seconds that keep the score.
    for r in rungs:
        while alloc[r] >= gran:
            trial = dict(alloc)
            trial[r] -= gran
            if coverage(t, cells, trial) < score:
                break
            alloc = trial
    return alloc, score


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--db", default=os.path.expanduser("~/.crucible/db/crucible.db"))
    ap.add_argument("--engine", required=True, help="blake3 (prefix) of the engine the matrix ran on")
    ap.add_argument("--board", action="append", default=[])
    ap.add_argument("--wall", type=float, default=60.0)
    ap.add_argument("--granularity", type=float, default=1.0)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    db = open_db(args.db)
    t, domain, counts = load_matrix(db, args.engine, args.board)
    if not t:
        sys.exit("no matrix rows: are the matrix31-<rung> subsets banked on this engine?")
    rungs = [r for r in LADDER_ORDER if any(r in t[c] for c in t)]
    cells = sorted(t)
    print(f"matrix: {len(cells)} cells, rungs {rungs}")
    for r in rungs:
        rows, solved, dirty = counts[r]
        print(f"  {r:<9} rows {rows:5d}  solved {solved:5d}  dirty-timing {dirty}")
    complete = [c for c in cells if all(r in t[c] for r in rungs)]
    print(f"  {len(complete)} cells have every rung ({len(cells) - len(complete)} partial; a missing rung reads as unsolved)")

    # The defaults as a portfolio on the same matrix, for the comparison:
    # fractions of the REMAINING wall, taken in order (0.25, 0.10, 0.25,
    # 0.50 of what is left; the fallback gets the rest).
    rem = args.wall
    default = {}
    for r, frac in (("ehc", 0.25), ("novlight", 0.10), ("lama", 0.25), ("driver", 0.50)):
        if r in rungs:
            default[r] = frac * rem
            rem -= default[r]
    if "bfs" in rungs:
        default["bfs"] = rem
    d_score = coverage(t, cells, default)
    best_single = max((coverage(t, cells, {r: args.wall}), r) for r in rungs)
    oracle = sum(1 for c in cells if any(v < math.inf for v in t[c].values()))
    print(f"\ncoverage on the matrix: defaults {d_score}, best single rung {best_single[1]} {best_single[0]}, "
          f"oracle (any rung inside the wall) {oracle}")

    alloc, score = stone_soup(t, cells, rungs, args.wall, args.granularity)
    print(f"\nin-sample table (coverage {score} of {len(cells)}):")
    for r in rungs:
        print(f"  {r:<9} {alloc[r]:5.1f} s")

    # Leave-one-domain-out: the receipt.
    held_table, held_default = 0, 0
    per_domain = []
    variants = sorted(set(domain.values()))
    for v in variants:
        train = [c for c in cells if domain[c] != v]
        test = [c for c in cells if domain[c] == v]
        a, _ = stone_soup(t, train, rungs, args.wall, args.granularity)
        ht = coverage(t, test, a)
        hd = coverage(t, test, default)
        held_table += ht
        held_default += hd
        per_domain.append((v, len(test), hd, ht))
    print(f"\nheld-out (leave-one-domain-out over {len(variants)} variants): defaults {held_default}, table {held_table}, "
          f"delta {held_table - held_default:+d}")
    for v, n, hd, ht in per_domain:
        if ht != hd:
            print(f"  {v:<40} {n:3d} cells  defaults {hd:3d}  table {ht:3d}  {ht - hd:+d}")

    os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)
    with open(args.out, "w") as f:
        f.write(f"# ladder table learned by benchmarks/ladder-table.py on engine {args.engine}\n")
        f.write(f"# matrix {len(cells)} cells; in-sample {score}/{len(cells)} (defaults {d_score}); "
                f"held-out {held_table} vs defaults {held_default} ({held_table - held_default:+d})\n")
        f.write(f"wall = {args.wall:g}\n")
        for r in rungs:
            if r in IMPLICIT:
                f.write(f"# {r} = {alloc[r]:g}  (the fallback takes what is left; not a slice)\n")
            else:
                f.write(f"{r} = {alloc[r]:g}\n")
    print(f"\nwrote {args.out}")


if __name__ == "__main__":
    main()
