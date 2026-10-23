#!/usr/bin/env python3
"""The per-rung matrix's sample (0.31 Phase 0.5, docs/roadmap-0.31.md Lane A).

Picks a stratified sample of cells from the crucible database -- every
variant of the named boards, at most N instances a variant spread over the
instance index -- and writes the rows file `crucible sweep --rows` reads,
plus the sweep commands that build the matrix: one subset per rung,
`--env FF_RUNG_ONLY=<rung>`, named `matrix31-<rung>` so each lands on its
own board row and its own stage.

    python3 benchmarks/matrix-sample.py --engine <blake3 of the banked 0.30.0>
        --per-variant 12 --out benchmarks/probes-0.31/matrix/rows.txt

`--per-variant 6` is the 600-cell fallback. The sample is DETERMINISTIC
for a given database (sorted variants, evenly spaced indices), so a re-run
of the script picks the same rows and the matrix can be resumed.

The database is opened immutably; nothing here writes to it.
"""

import argparse
import os
import sqlite3
import sys

# The satisficing / agile / multi-core boards the classical ladder runs on:
# the table is learned for THESE track classes. Temporal, optimal and
# preference boards have their own ladders and are not in the matrix.
DEFAULT_BOARDS = [
    "ipc67-results",  # 2008 + 2011 seq-sat
    "ipc5-prop",
    "ipc2014-sat",
    "ipc2014-agile",
    "ipc2018-sat",
    "ipc2023-sat",
]

RUNGS = ["ehc", "novlight", "lama", "driver", "bfs"]


def open_db(path):
    uri = f"file:{path}?immutable=1"
    return sqlite3.connect(uri, uri=True)


def cells(db, engine, boards):
    """(board, variant, label, sort_key) for every run row of `engine` on
    the boards, de-duplicated by cell."""
    q = """
    SELECT b.name, v.name, i.label, i.sort_key
      FROM run r
      JOIN board b ON b.id = r.board_id
      JOIN instance i ON i.id = r.instance_id
      JOIN variant v ON v.id = i.variant_id
      JOIN engine e ON e.id = r.engine_id
     WHERE e.blake3 LIKE ? AND b.env = '{}'
    """
    rows = db.execute(q, (engine + "%",)).fetchall()
    seen = set()
    out = []
    for board, variant, label, sort_key in rows:
        if boards and board not in boards:
            continue
        key = (board, variant, label)
        if key in seen:
            continue
        seen.add(key)
        out.append((board, variant, label, sort_key))
    return out


def stratify(rows, per_variant):
    """At most `per_variant` labels a variant, evenly spaced over the sorted
    instance index: the small, the middle and the large of every domain."""
    by_variant = {}
    for board, variant, label, sort_key in rows:
        by_variant.setdefault((board, variant), []).append((sort_key, label))
    picked = []
    for (board, variant), labels in sorted(by_variant.items()):
        labels.sort()
        n = len(labels)
        if n <= per_variant:
            idx = range(n)
        else:
            idx = sorted({round(k * (n - 1) / (per_variant - 1)) for k in range(per_variant)})
        for k in idx:
            picked.append((board, variant, labels[k][1]))
    return picked


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--db", default=os.path.expanduser("~/.crucible/db/crucible.db"))
    ap.add_argument("--engine", required=True, help="blake3 (prefix) of the engine whose rows define the cells -- the banked baseline")
    ap.add_argument("--board", action="append", default=[], help="board id (repeatable); default: the classical sat/agile/mco boards")
    ap.add_argument("--per-variant", type=int, default=12)
    ap.add_argument("--out", required=True, help="rows file to write (variant/label per line)")
    ap.add_argument("--ff", default=os.path.expanduser("~/ferroplan-0.31/target/release/ff"), help="the engine the matrix measures")
    ap.add_argument("--repo", default=os.path.expanduser("~/ferroplan"))
    ap.add_argument("--set", default="cut30", help="the manifest set the boards belong to (a subset never writes the set's stage; cut31 is added at the cut)")
    args = ap.parse_args()

    boards = args.board or DEFAULT_BOARDS
    db = open_db(args.db)
    rows = cells(db, args.engine, boards)
    if not rows:
        sys.exit(f"no rows for engine {args.engine!r} on {boards}: is it the banked baseline's hash?")
    picked = stratify(rows, args.per_variant)
    os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)
    with open(args.out, "w") as f:
        f.write(f"# matrix sample: {len(picked)} cells, <= {args.per_variant} a variant, engine {args.engine}\n")
        for board, variant, label in picked:
            f.write(f"{variant}/{label}  # {board}\n")
    per_board = {}
    for board, _, _ in picked:
        per_board[board] = per_board.get(board, 0) + 1
    print(f"{len(picked)} cells from {len(rows)} ({len(set((b, v) for b, v, _, _ in rows))} variants):")
    for b, n in sorted(per_board.items()):
        print(f"  {b:<22} {n}")
    serial_h = len(picked) * len(RUNGS) * 60 / 3600
    print(f"worst case {serial_h:.0f} child-hours serial over {len(RUNGS)} rungs; ~{serial_h / 8:.0f} h at width 8")
    print("\nthe sweeps, one per rung (quiet hours; the referee runs every cell solo):")
    board_flags = " ".join(f"--board {b}" for b in sorted(per_board))
    for rung in RUNGS:
        print(
            f"  crucible --repo {args.repo} sweep --set {args.set} {board_flags} "
            f"--rows {args.out} --engine {args.ff} --env FF_RUNG_ONLY={rung} "
            f"--name matrix31-{rung} --headless"
        )
    print("\nthen: python3 benchmarks/ladder-table.py --engine <the matrix engine's blake3> --out benchmarks/probes-0.31/matrix/table.txt")


if __name__ == "__main__":
    main()
