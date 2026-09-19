#!/usr/bin/env python3
"""What a board row would say if N were controlled.

A published board row is the attempt the crucible BANKED, and the crucible
re-runs an instance when conditions look dirty (PER-INSTANCE-RETRY.md, the
SUSPECT rule). That is right for a false timeout and wrong for a comparison:
two engines with different attempt counts are not scored on the same
instrument, and banking the max for both turns run-to-run variance into
coverage.

This reads every attempt out of the crucible db and reports the same
comparison under estimators that differ only in how they treat N:

  banked      what the boards publish today (the banked attempt)
  any         solved on ANY attempt              (max over N, N uncontrolled)
  first       solved on the FIRST attempt        (N=1 for both, but inherits
                                                  whatever conditions that
                                                  first attempt ran under)
  equal-N     max over the first k attempts, k = min(N_A, N_B) PER CELL
                                                 (the like-for-like max)
  per-run     sum of (solved runs / runs) per cell — the expected coverage
              of ONE run, which is what a 60 s board claims to report
  clean-only  per-run, restricted to runs the harness marked timing_quality
              'clean', over cells where BOTH engines have such a run

No estimator here is the "true" one. The point is the SPREAD: when the
delta changes sign across them, the instrument's variance exceeds the
effect and no number has been measured yet.

Usage:
  python3 benchmarks/attempts-estimator.py [--db PATH] [--a BLAKE3] [--b BLAKE3]
                                           [--boards NAME,NAME] [--per-board]
                                           [--bootstrap N]
"""
import argparse
import random
import sqlite3
import sys
from collections import Counter, defaultdict

DEFAULT_DB = "~/.crucible/db/crucible.db"
# The two engines benchmarks/cut27-compare-0.26-vs-0.27.txt compared.
DEFAULT_A = "8fe896b4fd53"  # v0.26.0 (the backfill build)
DEFAULT_B = "86302e06d81b"  # ff 0.27.0
# Three distinct 0.26.0 builds sit in this db (3709aeefc039, 03a17198744b,
# 8fe896b4fd53). Keying cells by `ver` merges them and splices three
# engines' attempts into one sequence, which is why this tool keys on the
# build hash and never on the version string.


def connect(db):
    """A read connection that never writes to the crucible's directory.

    The db is in WAL mode, so `mode=ro` cannot open it (a read-only
    connection still wants the -shm), and a plain open would CREATE
    -wal/-shm next to a database another process may own. Snapshot to a
    temp file and read that: 25 MB, and the crucible never notices.
    """
    try:
        return sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    except sqlite3.OperationalError:
        pass
    import shutil
    import tempfile

    tmp = tempfile.NamedTemporaryFile(suffix=".db", delete=False)
    tmp.close()
    shutil.copyfile(db, tmp.name)
    return sqlite3.connect(tmp.name)


def load(db, blake3):
    con = connect(db)
    con.row_factory = sqlite3.Row
    eng = con.execute(
        "select id, ver, tag, blake3 from engine where blake3 like ?", (blake3 + "%",)
    ).fetchall()
    if len(eng) != 1:
        sys.exit(f"engine {blake3!r} matched {len(eng)} rows; be more specific")
    e = eng[0]
    rows = con.execute(
        """select bd.name as board, r.instance_id, r.attempt, r.solved,
                  r.timing_quality, r.banked
           from run r join board bd on bd.id = r.board_id
           where r.engine_id = ? and r.state = 'done'
           order by r.attempt, r.id""",
        (e["id"],),
    ).fetchall()
    cells = defaultdict(list)
    for r in rows:
        cells[(r["board"], r["instance_id"])].append(dict(r))
    return e, cells


def estimate(cells_a, cells_b, common, mode):
    """Return (score_a, score_b) over `common` cells under one estimator."""
    out = [0.0, 0.0]
    for key in common:
        a, b = cells_a[key], cells_b[key]
        if mode == "banked":
            for i, rows in enumerate((a, b)):
                bk = [x for x in rows if x["banked"] == 1]
                out[i] += 1 if (bk and bk[0]["solved"]) else 0
        elif mode == "any":
            for i, rows in enumerate((a, b)):
                out[i] += 1 if any(x["solved"] for x in rows) else 0
        elif mode == "first":
            for i, rows in enumerate((a, b)):
                out[i] += 1 if rows[0]["solved"] else 0
        elif mode == "equal-N":
            k = min(len(a), len(b))
            for i, rows in enumerate((a, b)):
                out[i] += 1 if any(x["solved"] for x in rows[:k]) else 0
        elif mode == "per-run":
            for i, rows in enumerate((a, b)):
                out[i] += sum(1 for x in rows if x["solved"]) / len(rows)
        elif mode == "clean-only":
            ca = [x for x in a if x["timing_quality"] == "clean"]
            cb = [x for x in b if x["timing_quality"] == "clean"]
            if not ca or not cb:
                continue
            out[0] += sum(1 for x in ca if x["solved"]) / len(ca)
            out[1] += sum(1 for x in cb if x["solved"]) / len(cb)
    return out


def bootstrap(cells_a, cells_b, common, mode, n, seed=20260919):
    """Resample cells with replacement; return (p2.5, p97.5) of the delta."""
    rng = random.Random(seed)
    keys = list(common)
    deltas = []
    for _ in range(n):
        sample = [keys[rng.randrange(len(keys))] for _ in range(len(keys))]
        a, b = estimate(cells_a, cells_b, sample, mode)
        deltas.append(b - a)
    deltas.sort()
    lo = deltas[int(0.025 * len(deltas))]
    hi = deltas[int(0.975 * len(deltas)) - 1]
    return lo, hi


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", default=DEFAULT_DB)
    ap.add_argument("--a", default=DEFAULT_A)
    ap.add_argument("--b", default=DEFAULT_B)
    ap.add_argument("--boards", default=None, help="comma-separated board names")
    ap.add_argument("--per-board", action="store_true")
    ap.add_argument("--bootstrap", type=int, default=2000)
    args = ap.parse_args()

    import os

    db = os.path.expanduser(args.db)
    ea, ca = load(db, args.a)
    eb, cb = load(db, args.b)
    if args.boards:
        keep = set(args.boards.split(","))
        ca = {k: v for k, v in ca.items() if k[0] in keep}
        cb = {k: v for k, v in cb.items() if k[0] in keep}
    common = sorted(set(ca) & set(cb))

    def label(e):
        return f"{e['ver']}{' ' + e['tag'] if e['tag'] else ''} [{e['blake3'][:12]}]"

    print(f"A  {label(ea)}")
    print(f"B  {label(eb)}")
    print(f"\n{len(common)} instances measured by BOTH (A alone {len(ca)}, B alone {len(cb)})\n")

    for name, cells in (("A", ca), ("B", cb)):
        ns = [len(cells[k]) for k in common]
        tq = Counter(x["timing_quality"] for k in common for x in cells[k])
        tot = sum(tq.values())
        clean = 100 * tq.get("clean", 0) / max(tot, 1)
        print(
            f"  {name}: {sum(ns)} runs over {len(common)} cells, "
            f"mean N={sum(ns)/max(len(ns),1):.2f}, max N={max(ns)}, {clean:.0f}% clean"
        )
        # Does 'dirty' actually cost solves? If not, first-attempt is not
        # unfair to whichever sweep ran dirtier.
        for q in ("clean", "dirty"):
            sel = [x for k in common for x in cells[k] if x["timing_quality"] == q]
            if sel:
                print(
                    f"       {q:5} runs {len(sel):6}  solve rate "
                    f"{100*sum(1 for x in sel if x['solved'])/len(sel):.1f}%"
                )

    # WHY the estimators disagree: a re-run is triggered by FAILURE and
    # never by success, so extra attempts can only add solves. If the two
    # engines were re-run at different rates, the banked delta is partly
    # that difference and not the engines.
    print("\n  what triggers a re-run:")
    print("  %-10s %-28s %-28s %s" % ("engine", "attempt 1 SOLVED", "attempt 1 FAILED", "rescued"))
    trig = {}
    for name, cells in (("A", ca), ("B", cb)):
        s1 = [cells[k] for k in common if cells[k][0]["solved"]]
        f1 = [cells[k] for k in common if not cells[k][0]["solved"]]
        rs = sum(1 for v in s1 if len(v) > 1)
        rf = sum(1 for v in f1 if len(v) > 1)
        resc = sum(1 for v in f1 if any(x["solved"] for x in v))
        trig[name] = (len(s1), resc)
        print(
            "  %-10s %5d cells, re-run %5.1f%%   %5d cells, re-run %5.1f%%   %d"
            % (name, len(s1), 100 * rs / max(len(s1), 1), len(f1), 100 * rf / max(len(f1), 1), resc)
        )
    (fa, ra), (fb, rb) = trig["A"], trig["B"]
    print(
        f"\n  decomposition of the banked delta: "
        f"first-attempt {fb - fa:+d}  +  rescues {rb - ra:+d}  =  {(fb - fa) + (rb - ra):+d}"
    )

    print("\n  estimator     A        B        B - A      95% CI (cell bootstrap)")
    for mode in ("banked", "any", "first", "equal-N", "per-run", "clean-only"):
        a, b = estimate(ca, cb, common, mode)
        ci = ""
        if args.bootstrap:
            lo, hi = bootstrap(ca, cb, common, mode, args.bootstrap)
            ci = f"[{lo:+.0f}, {hi:+.0f}]"
        print(f"  {mode:12} {a:8.1f} {b:8.1f} {b-a:+9.1f}    {ci}")

    if args.per_board:
        print("\n  per board (banked vs equal-N vs per-run):")
        boards = sorted({k[0] for k in common})
        print("  %-24s %8s %8s %8s" % ("board", "banked", "equal-N", "per-run"))
        for bd in boards:
            sub = [k for k in common if k[0] == bd]
            row = []
            for mode in ("banked", "equal-N", "per-run"):
                a, b = estimate(ca, cb, sub, mode)
                row.append(b - a)
            print("  %-24s %+8.0f %+8.0f %+8.1f" % (bd, *row))


if __name__ == "__main__":
    main()
