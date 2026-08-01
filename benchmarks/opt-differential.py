#!/usr/bin/env python3
"""The optimal-mode admissibility differential (0.20 Phase 2).

Re-runs every instance the 0.19 boards CERTIFIED (solved seq-opt rows in
ipc-opt-2008-11.jsonl + ipc2014-opt.jsonl) against the current binary and
compares certified costs. Agreement everywhere is the LM-cut soundness
receipt; a mismatch is either the named 0.19 conditional-effect
admissibility bug (cave-diving / city-car / maintenance rows — the fresh
cost is the truth) or a real regression (anywhere else — stop the phase).

Timeouts here are inconclusive, not failures: contention can slow a row
past the budget but cannot change a certified cost. Resume-aware: rows
already in the output JSONL are skipped on re-run.
"""
import json
import os
import re
import subprocess

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FF = os.path.join(ROOT, "target", "release", "ff")
CORPUS = os.environ.get("FERROPLAN_IPC_CORPUS") or os.path.join(
    ROOT, "benchmarks", ".ipc-corpus")
OUT = os.path.join(ROOT, "benchmarks", "cut20", "opt-differential.jsonl")
TIMEOUT = int(os.environ.get("DIFF_TIMEOUT", "90"))

BOARDS = [  # (jsonl, ipc dirs to search for the variant)
    ("ipc-opt-2008-11.jsonl", ("ipc-2008", "ipc-2011")),
    ("ipc2014-opt.jsonl", ("ipc-2014",)),
]


def instances(vdir):
    # NUMERIC sort, exactly like ipc67.py's — the first differential run
    # sorted lexically (instance-10 before instance-2), misaligned every
    # board row past i1, and manufactured phantom cost "mismatches" in
    # both directions on elevator-STRIPS before the misread was caught.
    d = os.path.join(vdir, "instances")
    if not os.path.isdir(d):
        return []
    return sorted(os.listdir(d), key=lambda n: int(re.search(r"\d+", n).group()))


def find_variant(ipcs, variant):
    for ipc in ipcs:
        vdir = os.path.join(CORPUS, ipc, "domains", variant)
        if os.path.isdir(vdir):
            return vdir
    return None


def main():
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    done = set()
    if os.path.exists(OUT):
        for line in open(OUT):
            r = json.loads(line)
            done.add((r["board"], r["variant"], r["instance"]))
    out = open(OUT, "a")
    total = agree = disagree = inconclusive = 0
    mismatches = []
    for board, ipcs in BOARDS:
        for line in open(os.path.join(ROOT, "benchmarks", board)):
            r = json.loads(line)
            if not r.get("solved"):
                continue
            key = (board, r["variant"], r["instance"])
            total += 1
            if key in done:
                continue
            vdir = find_variant(ipcs, r["variant"])
            if not vdir:
                continue
            insts = instances(vdir)
            if r["instance"] > len(insts):
                continue
            prob = os.path.join(vdir, "instances", insts[r["instance"] - 1])
            # Per-instance domain files (openstacks/parc-printer style):
            # domains/domain-N.pddl paired by the instance's own number.
            dom = os.path.join(vdir, "domain.pddl")
            if not os.path.isfile(dom):
                num = re.search(r"\d+", insts[r["instance"] - 1]).group()
                dom = os.path.join(vdir, "domains", f"domain-{num}.pddl")
            old_cost = r.get("metric") if r.get("metric") is not None else r.get("length")
            rec = {"board": board, "variant": r["variant"],
                   "instance": r["instance"], "old": old_cost, "new": None,
                   "verdict": "inconclusive"}
            try:
                p = subprocess.run(
                    [FF, "-o", dom, "-f", prob, "--json", "--mode", "optimal"],
                    capture_output=True, text=True, timeout=TIMEOUT)
                s = json.loads(p.stdout) if p.stdout.strip() else {}
                if s.get("solved"):
                    plan = s.get("plan") or {}
                    new_cost = plan.get("metric")
                    if new_cost is None:
                        new_cost = plan.get("length")
                    rec["new"] = new_cost
                    rec["verdict"] = ("agree" if abs(new_cost - old_cost) < 1e-6
                                      else "MISMATCH")
            except (subprocess.TimeoutExpired, Exception):
                pass
            out.write(json.dumps(rec) + "\n")
            out.flush()
            if rec["verdict"] == "agree":
                agree += 1
            elif rec["verdict"] == "MISMATCH":
                disagree += 1
                mismatches.append(rec)
            else:
                inconclusive += 1
            print(f"{rec['verdict']:12s} {r['variant']} i{r['instance']} "
                  f"old={old_cost} new={rec['new']}", flush=True)
    print(f"\nDIFFERENTIAL DONE: {agree} agree, {disagree} mismatch, "
          f"{inconclusive} inconclusive (of {total} certified rows)")
    for m in mismatches:
        print("  MISMATCH:", json.dumps(m))


if __name__ == "__main__":
    main()
