#!/usr/bin/env python3
"""Read an `--env` subset sweep against the banked rows of a baseline engine
(0.31: Lane B's ladder claim, Lane A′'s portfolio).

The crucible lands `--env K=V` rows on a board row of their own (same name,
the env in its identity), so the question "what did the knob change" is a
join on (board, variant, label) between the env board's best row and the
baseline engine's banked row on the plain board. Best-of-N on both sides:
a cell is solved if any done row solved it, timed by its fastest solve.

    python3 benchmarks/subset-read.py --engine 6facbec9163c --baseline b02061eded16 \
        --env FF_LAMA_NOV --board ipc2014-sat --board ipc2014-agile ...

Opens the database read-only (`?immutable=1`); run it beside a sweep only
briefly -- the crucible's throttle counts it as foreign load.
"""
import argparse
import collections
import os
import sqlite3
import statistics


def open_db(path):
    return sqlite3.connect(f"file:{path}?immutable=1", uri=True)


def cells(db, engine, boards, env_like, banked_only):
    q = f"""
    SELECT b.name, v.name, i.label, r.solved, r.time_secs, r.length, r.banked, r.peak_rss
      FROM run r
      JOIN board b ON b.id = r.board_id
      JOIN instance i ON i.id = r.instance_id
      JOIN variant v ON v.id = i.variant_id
      JOIN engine e ON e.id = r.engine_id
     WHERE e.blake3 LIKE ? AND b.env LIKE ? AND r.state = 'done'
       AND b.name IN ({",".join("?" * len(boards))})
    """
    out = {}
    for board, var, label, solved, t, length, banked, rss in db.execute(q, (engine + "%", env_like, *boards)):
        if banked_only and not banked:
            continue
        key = (board, var, label)
        d = out.setdefault(key, {"solved": False, "t": None, "len": None, "rows": 0, "rss": 0})
        d["rows"] += 1
        d["rss"] = max(d["rss"], rss or 0)
        if solved:
            t = 60.0 if t is None else t
            if not d["solved"] or t < d["t"]:
                d["solved"], d["t"], d["len"] = True, t, length
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", default=os.path.expanduser("~/.crucible/db/crucible.db"))
    ap.add_argument("--engine", required=True, help="blake3 prefix of the candidate engine (the subset's rows)")
    ap.add_argument("--baseline", required=True, help="blake3 prefix of the banked baseline on the plain boards")
    ap.add_argument("--env", required=True, help="a key the subset's board env carries, e.g. FF_LAMA_NOV")
    ap.add_argument("--board", action="append", required=True)
    ap.add_argument("--all-rows", action="store_true", help="count the baseline's unbanked rows too")
    args = ap.parse_args()
    db = open_db(args.db)
    cand = cells(db, args.engine, args.board, f"%{args.env}%", banked_only=False)
    base = cells(db, args.baseline, args.board, "{}", banked_only=not args.all_rows)

    print(f"subset {args.env} on {args.engine}: {len(cand)} cells reached; baseline {args.baseline} banked: {len(base)} cells")
    both = sorted(k for k in cand if k in base)
    print(f"cells on both sides: {len(both)}\n")

    tot_gain = tot_loss = 0
    print(f"{'board':<18} {'cells':>5} {'base':>5} {'cand':>5} {'gain':>5} {'loss':>5} {'net':>5}")
    per = collections.defaultdict(list)
    for k in both:
        per[k[0]].append(k)
    for board in args.board:
        ks = per.get(board, [])
        if not ks:
            continue
        b = sum(base[k]["solved"] for k in ks)
        c = sum(cand[k]["solved"] for k in ks)
        gain = sum(cand[k]["solved"] and not base[k]["solved"] for k in ks)
        loss = sum(base[k]["solved"] and not cand[k]["solved"] for k in ks)
        tot_gain += gain
        tot_loss += loss
        flag = "  <- loss > gain" if loss > gain else ""
        print(f"{board:<18} {len(ks):>5} {b:>5} {c:>5} {gain:>+5} {loss:>-5} {c - b:>+5}{flag}")
    print(f"{'total':<18} {len(both):>5} {sum(base[k]['solved'] for k in both):>5} {sum(cand[k]['solved'] for k in both):>5} {tot_gain:>+5} {tot_loss:>-5} {tot_gain - tot_loss:>+5}")

    solved_both = [k for k in both if cand[k]["solved"] and base[k]["solved"]]
    if solved_both:
        mc = statistics.median(cand[k]["t"] for k in solved_both)
        mb = statistics.median(base[k]["t"] for k in solved_both)
        lc = [cand[k]["len"] for k in solved_both if cand[k]["len"] and base[k]["len"]]
        lb = [base[k]["len"] for k in solved_both if cand[k]["len"] and base[k]["len"]]
        print(f"\nboth solved {len(solved_both)}: median time {mc:.2f} s v {mb:.2f} s; "
              f"plan length sum {sum(lc)} v {sum(lb)} on {len(lc)} cells with lengths "
              f"(shorter {sum(a < b for a, b in zip(lc, lb))}, longer {sum(a > b for a, b in zip(lc, lb))})")

    lost = [k for k in both if base[k]["solved"] and not cand[k]["solved"]]
    gained = [k for k in both if cand[k]["solved"] and not base[k]["solved"]]
    print(f"\nLOST ({len(lost)}), the baseline's time and the candidate's rows / peak RSS:")
    for k in lost:
        print(f"  {k[0]:<16} {k[1]}/{k[2]:<6} base {base[k]['t']:6.2f} s   cand rows {cand[k]['rows']} rss {cand[k]['rss'] / 2**30:.2f} GiB")
    print(f"\nGAINED ({len(gained)}), by variant:")
    for v, n in collections.Counter(k[1] for k in gained).most_common():
        print(f"  {n:3d} {v}")
    missing = [k for k in base if k not in cand]
    if missing:
        print(f"\nbaseline cells the subset has not reached: {len(missing)}")


if __name__ == "__main__":
    main()
