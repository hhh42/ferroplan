#!/usr/bin/env python3
"""Diff two ipc67.py raw JSONL runs per instance (e.g. default vs portfolio).

Reports coverage flips (solved in one run only), cost changes on
commonly-solved instances, and a per-variant win/tie/loss table — the
evidence format the Phase 6 portfolio acceptance asks for.

Usage:
  python3 benchmarks/ipc67-diff.py A.jsonl B.jsonl [--label-a X] [--label-b Y]
"""
import json, sys


def arg(name, default):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


def load(path):
    out = {}
    for line in open(path):
        r = json.loads(line)
        out[(r["variant"], r["instance"])] = r
    return out


def cost(r):
    m = r.get("metric")
    return m if m is not None else r.get("length")


def main():
    a_path, b_path = sys.argv[1], sys.argv[2]
    la = arg("--label-a", "A")
    lb = arg("--label-b", "B")
    A, B = load(a_path), load(b_path)
    keys = sorted(set(A) | set(B))
    only_a, only_b, both = [], [], []
    for k in keys:
        ra, rb = A.get(k), B.get(k)
        sa = bool(ra and ra["solved"])
        sb = bool(rb and rb["solved"])
        if sa and not sb:
            only_a.append(k)
        elif sb and not sa:
            only_b.append(k)
        elif sa and sb:
            both.append(k)

    print(f"{la}: {sum(1 for r in A.values() if r['solved'])}/{len(A)} solved   "
          f"{lb}: {sum(1 for r in B.values() if r['solved'])}/{len(B)} solved")
    print(f"solved by both: {len(both)}   only {la}: {len(only_a)}   "
          f"only {lb}: {len(only_b)}\n")
    for tag, lst, run in ((la, only_a, A), (lb, only_b, B)):
        for v, n in lst:
            r = run[(v, n)]
            print(f"  only {tag}: {v}/{n}  ({r['time']}s, cost {cost(r)})")
    if only_a or only_b:
        print()

    # cost + time deltas on commonly-solved instances, per variant
    stats = {}
    for v, n in both:
        ra, rb = A[(v, n)], B[(v, n)]
        s = stats.setdefault(v, {"n": 0, "cheaper_a": 0, "cheaper_b": 0,
                                 "ta": 0.0, "tb": 0.0})
        s["n"] += 1
        s["ta"] += ra["time"] or 0
        s["tb"] += rb["time"] or 0
        ca, cb = cost(ra), cost(rb)
        if ca is not None and cb is not None:
            if ca < cb:
                s["cheaper_a"] += 1
            elif cb < ca:
                s["cheaper_b"] += 1
    print(f"| variant | both | {la} cheaper | {lb} cheaper | "
          f"{la} time | {lb} time |")
    print("|---|---|---|---|---|---|")
    for v in sorted(stats):
        s = stats[v]
        print(f"| {v} | {s['n']} | {s['cheaper_a']} | {s['cheaper_b']} | "
              f"{s['ta']:.0f}s | {s['tb']:.0f}s |")


if __name__ == "__main__":
    main()
