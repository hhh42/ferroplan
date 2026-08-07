#!/usr/bin/env python3
"""The optimal-mode admissibility differential (0.20 Phase 2; board-budget
mode 0.22 Phase 3).

Re-runs every instance the boards CERTIFIED (solved seq-opt rows in
ipc-opt-2008-11.jsonl + ipc2014-opt.jsonl + ipc2026-opt.jsonl) against the
current binary and compares certified costs. Agreement everywhere is the
LM-cut soundness receipt; a mismatch is either the named 0.19
conditional-effect admissibility bug (cave-diving / city-car / maintenance
rows — the fresh cost is the truth) or a real regression (anywhere else —
stop the phase).

TWO budgets, two verdicts for a non-answer (0.22 Phase 3's instrument
repair): the default replay runs at 90 s with no armed wall and books
timeouts as "inconclusive, not failures" — contention can slow a row past
the budget but cannot change a certified cost. That forgiveness is how ten
dying certificates sailed through an all-306-recertify gate at 0.21: they
re-certified at 90 s unarmed and died at 60 s on the boards.
`--board-budget` closes the gap: BOARD conditions (60 s external kill,
`FF_TIME_LIMIT` armed, `FF_MEM_BUDGET_GB` set), and a certificate that
cannot re-certify inside them is a REGRESSION, named per instance.

Resume-aware: rows already in the output JSONL are skipped on re-run.

Flags (the ipc67.py arg convention):
  --board-budget      60 s board conditions; timeout/no-cert = REGRESSION
  --slice-loss        add the twelve 0.21-A/B-named slice-loss certs (old
                      costs from the v0.20.0 board raws / v0.19 backfill)
  --sample N          random-sample N certified rows (seed --seed, default 22)
  --only S            comma-separated variant-substring filters; a token
                      "substr:iN" also pins the instance number
  --jobs N            parallel replays (default 1)
  --out PATH          output JSONL override
"""
import json
import os
import random
import re
import subprocess
import sys
import threading
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FF = os.path.join(ROOT, "target", "release", "ff")
CORPUS = os.environ.get("FERROPLAN_IPC_CORPUS") or os.path.join(
    ROOT, "benchmarks", ".ipc-corpus")


def arg(flag, default=None):
    if flag in sys.argv:
        i = sys.argv.index(flag)
        if i + 1 < len(sys.argv) and not sys.argv[i + 1].startswith("--"):
            return sys.argv[i + 1]
        return True
    return default


BOARD_BUDGET = arg("--board-budget") is True
SLICE_LOSS = arg("--slice-loss") is True
SAMPLE = int(arg("--sample", "0"))
SEED = int(arg("--seed", "22"))
ONLY = arg("--only", None)
JOBS = int(arg("--jobs", "1"))

# Board conditions are 60 s (cut22-sweeps.sh); the forgiving replay stays 90.
TIMEOUT = int(os.environ.get("DIFF_TIMEOUT", "60" if BOARD_BUDGET else "90"))
MEMGB = os.environ.get("MEMGB", "6")
OUT = arg("--out") or os.path.join(
    ROOT, "benchmarks", "cut22" if BOARD_BUDGET else "cut20",
    "opt-differential-board.jsonl" if BOARD_BUDGET else "opt-differential.jsonl")

BOARDS = [  # (jsonl, ipc dirs to search for the variant)
    ("ipc-opt-2008-11.jsonl", ("ipc-2008", "ipc-2011")),
    ("ipc2014-opt.jsonl", ("ipc-2014",)),
    # 0.22 Phase 4: the numeric proof board's certificates join the
    # regression surface (21 rows; LENGTH currency — metric is null there).
    ("ipc2026-opt.jsonl", ("ipc-2026n",)),
]

# The twelve slice-loss certificates the 0.21 A/B named (docs/roadmap-0.22.md
# Phase 3): certified by v0.20.0 (board raws at the v0.20.0 tag; cave-diving
# i18 by the v0.19 backfill, benchmarks/air-0.19.0), lost to the
# unconditional 24 s sprint slice at 0.21. They are not solved rows in the
# current JSONLs, so they enter the replay set here, old costs frozen from
# those raws — the honest baseline the Phase 3 levers must move.
SLICE_LOSS_ROWS = [
    ("ipc-opt-2008-11.jsonl", "sokoban-sequential-optimal-strips", 15, 76.0),
    ("ipc-opt-2008-11.jsonl", "sokoban-sequential-optimal-strips", 22, 44.0),
    ("ipc-opt-2008-11.jsonl", "sokoban-sequential-optimal", 16, 76.0),
    ("ipc-opt-2008-11.jsonl", "sokoban-sequential-optimal", 19, 44.0),
    ("ipc-opt-2008-11.jsonl", "peg-solitaire-sequential-optimal-strips", 26, 9.0),
    ("ipc-opt-2008-11.jsonl", "peg-solitaire-sequential-optimal", 16, 9.0),
    ("ipc-opt-2008-11.jsonl", "tidybot-sequential-optimal", 5, 38),
    ("ipc-opt-2008-11.jsonl", "tidybot-sequential-optimal", 11, 29),
    ("ipc-opt-2008-11.jsonl", "parc-printer-sequential-optimal-strips", 14, 1020512.0),
    ("ipc-opt-2008-11.jsonl", "parc-printer-sequential-optimal", 12, 1020512.0),
    ("ipc-opt-2008-11.jsonl", "elevator-sequential-optimal", 12, 53.0),
    ("ipc2014-opt.jsonl", "cave-diving-sequential-optimal", 18, 131.0),
]


def instance_map(vdir):
    # Keyed by the ipc67.py instance LABEL (first digit group; every group
    # joined for multipart names) — NOT by sorted position. The positional
    # read misaligns any 0-based instance set: sailing-wind-opt's rows are
    # numbered from instance-0, and position (i-1) would shift the whole
    # board one problem over, manufacturing phantom mismatches exactly the
    # way the first differential's lexical sort did on elevator-STRIPS.
    d = os.path.join(vdir, "instances")
    if not os.path.isdir(d):
        return {}
    out = {}
    for f in os.listdir(d):
        gs = re.findall(r"\d+", f)
        if not gs:
            continue
        label = int(gs[0]) if len(gs) == 1 else "_".join(gs)
        out[label] = (f, gs[0])
    return out


def find_variant(ipcs, variant):
    for ipc in ipcs:
        vdir = os.path.join(CORPUS, ipc, "domains", variant)
        if os.path.isdir(vdir):
            return vdir
    return None


def only_match(variant, instance):
    if not ONLY:
        return True
    for tok in ONLY.split(","):
        tok = tok.strip()
        if ":" in tok:
            sub, inst = tok.rsplit(":", 1)
            if sub in variant and inst.lstrip("i") == str(instance):
                return True
        elif tok in variant:
            return True
    return False


def gather():
    """The replay set: (board, ipcs, variant, instance, old_cost) rows."""
    rows = []
    ipcs_of = dict(BOARDS)
    for board, ipcs in BOARDS:
        for line in open(os.path.join(ROOT, "benchmarks", board)):
            r = json.loads(line)
            if not r.get("solved"):
                continue
            if not only_match(r["variant"], r["instance"]):
                continue
            old = r.get("metric") if r.get("metric") is not None else r.get("length")
            rows.append((board, ipcs, r["variant"], r["instance"], old))
    if SAMPLE and SAMPLE < len(rows):
        rows = random.Random(SEED).sample(rows, SAMPLE)
    if SLICE_LOSS:
        for board, variant, inst, old in SLICE_LOSS_ROWS:
            if only_match(variant, inst):
                rows.append((board, ipcs_of[board], variant, inst, old))
    return rows


def replay(board, ipcs, variant, instance, old_cost):
    vdir = find_variant(ipcs, variant)
    if not vdir:
        return None
    imap = instance_map(vdir)
    if instance not in imap:
        return None
    fname, first_group = imap[instance]
    prob = os.path.join(vdir, "instances", fname)
    # Per-instance domain files (openstacks/parc-printer style):
    # domains/domain-N.pddl paired by the instance's own number.
    dom = os.path.join(vdir, "domain.pddl")
    if not os.path.isfile(dom):
        dom = os.path.join(vdir, "domains", f"domain-{first_group}.pddl")
    rec = {"board": board, "variant": variant, "instance": instance,
           "old": old_cost, "new": None, "time": None, "note": None,
           "verdict": "REGRESSION" if BOARD_BUDGET else "inconclusive"}
    env = dict(os.environ)
    if BOARD_BUDGET:
        # Board conditions (cut22-sweeps.sh / ipc67.py): the armed wall and
        # the memory budget — a certificate is only "kept" if it re-certifies
        # under the exact discipline the boards charge.
        env["FF_TIME_LIMIT"] = str(TIMEOUT)
        env["FF_MEM_BUDGET_GB"] = MEMGB
    try:
        t0 = time.perf_counter()
        p = subprocess.run(
            [FF, "-o", dom, "-f", prob, "--json", "--mode", "optimal"],
            capture_output=True, text=True, timeout=TIMEOUT, env=env)
        s = json.loads(p.stdout) if p.stdout.strip() else {}
        if s.get("solved"):
            plan = s.get("plan") or {}
            new_cost = plan.get("metric")
            if new_cost is None:
                new_cost = plan.get("length")
            rec["new"] = new_cost
            rec["time"] = round(time.perf_counter() - t0, 2)
            rec["note"] = (s.get("notes") or [None])[0]
            rec["verdict"] = ("agree" if abs(new_cost - old_cost) < 1e-6
                              else "MISMATCH")
    except (subprocess.TimeoutExpired, Exception):
        pass
    return rec


def main():
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    done = set()
    if os.path.exists(OUT):
        for line in open(OUT):
            r = json.loads(line)
            done.add((r["board"], r["variant"], r["instance"]))
    rows = [r for r in gather() if (r[0], r[2], r[3]) not in done]
    total = len(rows) + len(done)
    out = open(OUT, "a")
    lock = threading.Lock()
    tallies = {"agree": 0, "MISMATCH": 0, "inconclusive": 0, "REGRESSION": 0}
    bad = []

    def one(row):
        rec = replay(*row)
        if rec is None:
            return
        with lock:
            out.write(json.dumps(rec) + "\n")
            out.flush()
            tallies[rec["verdict"]] += 1
            if rec["verdict"] in ("MISMATCH", "REGRESSION"):
                bad.append(rec)
            print(f"{rec['verdict']:12s} {rec['variant']} i{rec['instance']} "
                  f"old={rec['old']} new={rec['new']}"
                  + (f" ({rec['time']}s)" if rec.get("time") else ""),
                  flush=True)

    if JOBS > 1:
        from concurrent.futures import ThreadPoolExecutor
        with ThreadPoolExecutor(max_workers=JOBS) as pool:
            list(pool.map(one, rows))
    else:
        for row in rows:
            one(row)
    print(f"\nDIFFERENTIAL DONE ({'board-budget ' + str(TIMEOUT) + 's' if BOARD_BUDGET else str(TIMEOUT) + 's forgiving'}): "
          f"{tallies['agree']} agree, {tallies['MISMATCH']} mismatch, "
          f"{tallies['REGRESSION']} REGRESSION, "
          f"{tallies['inconclusive']} inconclusive (of {total} certified rows)")
    for m in bad:
        print(f"  {m['verdict']}:", json.dumps(m))


if __name__ == "__main__":
    main()
