#!/usr/bin/env python3
"""Read the per-rung matrix (0.31 Phase 0.5) straight from the database.

`ladder-table.py` learns slices; this reads what the rows say about the
rungs themselves -- the numbers Lane A's 2026-10-31 record is built on:

  * each rung's coverage with the whole wall, and the ORACLE (any rung);
  * the banked ladder of the baseline engine on the same cells (the real
    reference -- ladder-table.py's "defaults" is a restart-semantics
    simulation and reads ~150 cells low);
  * each rung's UNIQUE solves: cells no other rung reaches. A candidate
    rung (Lane B's lamanov / front / driverx) earns its place here, not
    on coverage;
  * the best k-subsets run side by side (the mco boards' lever).

    python3 benchmarks/matrix-read.py --engine 6facbec9163c --baseline b02061eded16
    python3 benchmarks/matrix-read.py --engine 6facbec9163c --baseline b02061eded16 \
        --candidate lamanov --candidate front --candidate driverx

Rungs are discovered from the board env (`FF_RUNG_ONLY`). `--candidate`
names rungs scored against the OTHERS (the standing five by default): a
candidate's unique solves are cells none of the standing rungs solved.
The database is opened immutably.
"""

import argparse
import collections
import itertools
import os
import sqlite3

STANDING = ["ehc", "novlight", "lama", "driver", "bfs"]
BOARDS = [
    "ipc67-results",
    "ipc5-prop",
    "ipc2014-sat",
    "ipc2014-agile",
    "ipc2018-sat",
    "ipc2023-sat",
]


def open_db(path):
    return sqlite3.connect(f"file:{path}?immutable=1", uri=True)


def matrix(db, engine):
    """(variant, label) -> rung -> best solve time (None = unsolved)."""
    q = """
    SELECT v.name, i.label, b.env, r.solved, r.time_secs
      FROM run r
      JOIN board b ON b.id = r.board_id
      JOIN instance i ON i.id = r.instance_id
      JOIN variant v ON v.id = i.variant_id
      JOIN engine e ON e.id = r.engine_id
     WHERE e.blake3 LIKE ? AND b.env LIKE '%FF_RUNG_ONLY%' AND r.state = 'done'
    """
    cell = collections.defaultdict(dict)
    for v, label, env, solved, t in db.execute(q, (engine + "%",)):
        rung = env.split('"')[3]
        d = cell[(v, label)]
        if solved:
            t = 60.0 if t is None else t
            prev = d.get(rung)
            d[rung] = t if prev is None else min(prev, t)
        else:
            d.setdefault(rung, None)
    return cell


def banked_ladder(db, engine, boards):
    q = f"""
    SELECT v.name, i.label, MAX(r.solved)
      FROM run r
      JOIN board b ON b.id = r.board_id
      JOIN instance i ON i.id = r.instance_id
      JOIN variant v ON v.id = i.variant_id
      JOIN engine e ON e.id = r.engine_id
     WHERE e.blake3 LIKE ? AND b.env = '{{}}' AND r.banked = 1
       AND b.name IN ({",".join("?" * len(boards))})
     GROUP BY 1, 2
    """
    return {(v, l): s for v, l, s in db.execute(q, (engine + "%", *boards))}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--db", default=os.path.expanduser("~/.crucible/db/crucible.db"))
    ap.add_argument("--engine", required=True, help="blake3 prefix of the matrix engine")
    ap.add_argument("--baseline", help="blake3 prefix of the banked baseline (the real ladder on the same cells)")
    ap.add_argument("--candidate", action="append", default=[], help="rung(s) judged on unique solves against the standing rungs")
    ap.add_argument("--standing", default=",".join(STANDING), help="comma-separated standing rungs")
    ap.add_argument("--wall", type=float, default=60.0)
    args = ap.parse_args()

    standing = args.standing.split(",")
    db = open_db(args.db)
    cell = matrix(db, args.engine)
    present = sorted({r for d in cell.values() for r in d})
    print(f"rungs in the database for {args.engine}: {present}")
    rungs = [r for r in standing + args.candidate if r in present]
    missing = [r for r in standing + args.candidate if r not in present]
    if missing:
        print(f"  not yet measured: {missing}")
    standing = [r for r in standing if r in present]
    candidates = [r for r in args.candidate if r in present]
    # The standing rungs are read on every cell that carries all of them; a
    # candidate mid-sweep is read on the cells it has reached so far, each
    # against the standing rungs on THOSE cells -- a partial candidate must
    # not shrink the standing read.
    cells = [c for c in cell if all(r in cell[c] for r in standing)]
    print(f"cells carrying every standing rung {standing}: {len(cells)}")

    def solved(c, r):
        t = cell[c].get(r)
        return t is not None and t <= args.wall

    print("\ncoverage, one rung with the whole wall:")
    for r in standing:
        print(f"  {r:<10} {sum(solved(c, r) for c in cells):5d}")
    oracle = sum(any(solved(c, r) for r in standing) for c in cells)
    print(f"oracle over the standing rungs: {oracle}")

    ladder = banked_ladder(db, args.baseline, BOARDS) if args.baseline else None
    if ladder is not None:
        have = [c for c in cells if c in ladder]
        n = sum(ladder[c] for c in have)
        print(f"\nbanked ladder {args.baseline} on these cells: {n} of {len(have)} ({len(cells) - len(have)} not banked)")
        print(f"  cells a standing rung solves that the ladder did not: {sum(any(solved(c, r) for r in standing) and not ladder[c] for c in have)}")
        print(f"  cells the ladder solves that no standing rung did:    {sum(ladder[c] and not any(solved(c, r) for r in standing) for c in have)}")

    print("\nunique solves among the standing rungs (cells no other standing rung reaches):")
    for r in standing:
        print(f"  {r:<10} {sum(solved(c, r) and not any(solved(c, o) for o in standing if o != r) for c in cells):5d}")

    print("\nside by side, each with the whole wall (the mco boards' lever):")
    for k in (2, 3, 4):
        if k > len(standing):
            break
        best = max(
            itertools.combinations(standing, k),
            key=lambda s: sum(any(solved(c, r) for r in s) for c in cells),
        )
        print(f"  best {k}-set {best}: {sum(any(solved(c, r) for r in best) for c in cells)}")

    fastest = collections.defaultdict(set)
    for c in cells:
        ts = {r: cell[c][r] for r in standing if cell[c][r] is not None}
        if ts:
            fastest[c[0]].add(min(ts, key=ts.get))
    print(f"\nvariants whose fastest rung varies across instances: {sum(len(s) > 1 for s in fastest.values())} of {len(fastest)}")

    for r in candidates:
        reached = [c for c in cells if r in cell[c]]
        cov = sum(solved(c, r) for c in reached)
        base = sum(any(solved(c, o) for o in standing) for c in reached)
        unique = sum(solved(c, r) and not any(solved(c, o) for o in standing) for c in reached)
        print(f"\ncandidate {r}: {len(reached)} of {len(cells)} cells reached so far")
        print(f"  coverage {cov}; standing oracle on the same cells {base}; with {r} {base + unique}")
        print(f"  UNIQUE solves (cells no standing rung reaches): {unique}   <- Lane B's judge")
        if ladder is not None:
            print(f"  cells {r} solves that the banked ladder did not: {sum(solved(c, r) and not ladder.get(c, 0) for c in reached)}")
        lost = sum(any(solved(c, o) for o in standing) and not solved(c, r) for c in reached)
        print(f"  cells some standing rung solves that {r} does not: {lost}")


if __name__ == "__main__":
    main()
