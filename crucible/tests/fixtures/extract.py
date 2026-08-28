#!/usr/bin/env python3
"""Rescue the incident evidence out of .gitignore'd staging dirs.

Every fixture here is a REAL row from a REAL sweep that a comment in
benchmarks/ipc67.py or benchmarks/standings.py names as the cause of a wrong
published number. Most of them live in benchmarks/air*/, which .gitignore
excludes -- so a disk failure destroys the only physical evidence for two of
the incidents crucible's tests are supposed to defend.

Re-runnable and deterministic: same inputs, byte-identical outputs. Rows keep
their exact source bytes (no re-serialization), so a fixture can never drift
from what the runner actually wrote.

    python3 crucible/tests/fixtures/extract.py [--check]

--check re-extracts into memory and diffs against what is on disk, exiting
non-zero on any difference. That is the guard against someone hand-editing a
fixture to make a test pass.
"""
import json, os, sys, glob, hashlib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
CHECK = "--check" in sys.argv


def rows(path):
    """(raw_line, parsed) for every parseable line, source bytes preserved."""
    out = []
    with open(os.path.join(ROOT, path)) as f:
        for ln in f:
            if not ln.strip():
                continue
            try:
                out.append((ln.rstrip("\n"), json.loads(ln)))
            except ValueError:
                continue
    return out


def pick(path, pred):
    return [ln for ln, r in rows(path) if pred(r)]


def note_text(r):
    n = r.get("notes")
    return n if isinstance(n, str) else " ".join(str(x) for x in (n or []))


# --- the fixtures, each with the incident it defends -------------------------
FIXTURES = {}

# THE 15. VAL refuses to INGEST these domains before reading any plan, so the
# runner booked val=false and standings.py dropped them from coverage: the
# table read 46/240 and 113/320 where the boards beside it said 53 and 121.
# Both source files are .gitignore'd; this is the only copy that survives.
FIXTURES["incidents/val-unavailable-15.jsonl"] = (
    pick("benchmarks/air/ipc2018-sat.jsonl", lambda r: r.get("val") is False)
    + pick("benchmarks/air/ipc2026-numeric.jsonl", lambda r: r.get("val") is False))

# A REAL VAL-RED: the engine produced a plan and VAL rejected it, on a domain
# VAL can read perfectly well. The whole VAL-RED class rests on these rows.
# Taken from air-0.19.0/, which IS tracked -- kept here so the two live side by
# side and a test can prove they classify differently.
FIXTURES["incidents/val-red-map-analyzer.jsonl"] = pick(
    "benchmarks/air-0.19.0/ipc2014-tempo.jsonl", lambda r: r.get("val") is False)

# markettrader's instances init undeclared fluents, so VAL's TYPECHECKER
# refuses the problem -- "Type problem in problem specification!", the
# signature 0.21 was missing. It booked the board's only VAL-RED through
# exactly that gap.
FIXTURES["incidents/val-false-markettrader.jsonl"] = pick(
    "benchmarks/air21/ipc2023-numeric.jsonl", lambda r: r.get("val") is False)

# THE LIVE BUG. ipc67.py emits "mem-cap (self-inflicted: node byte target
# raised)"; standings.py matches "mem-cap" by EXACT equality, so these fall
# through to early-exit -- the one column the refill loop is refereed by.
# Seven of them are in the published table right now.
FIXTURES["incidents/memcap-self-inflicted.jsonl"] = sorted(set(
    ln for p in glob.glob(os.path.join(ROOT, "benchmarks", "*.jsonl"))
    for ln in pick(os.path.relpath(p, ROOT),
                   lambda r: note_text(r).startswith("mem-cap ("))))

# Two instruments, one verdict: RLIMIT_AS makes the child fail its own
# allocation, the RSS watchdog SIGKILLs it (rc -9, no stderr). The watchdog's
# verdict must be read BEFORE the generic nonzero-exit branch or it books as
# engine-exit--9 -- which is what these rows are.
FIXTURES["incidents/engine-exit-signal.jsonl"] = sorted(set(
    ln for p in glob.glob(os.path.join(ROOT, "benchmarks", "**", "*.jsonl"),
                          recursive=True) if ".ipc-corpus" not in p
    for ln in pick(os.path.relpath(p, ROOT),
                   lambda r: note_text(r).startswith("engine-exit--"))))

# First-group-only labels collapsed 20 distinct problems onto 3-5 labels:
# ipc2026-numeric held 320 rows under 288 keys, silently breaking the
# per-instance diff and the --score-against join. `instance` is an int for
# single-number filenames and an underscore-joined STRING otherwise, and the
# type is part of the contract.
FIXTURES["incidents/multipart-labels.jsonl"] = pick(
    "benchmarks/air-0.21.0/ipc2026-numeric.jsonl",
    lambda r: isinstance(r.get("instance"), str))

# Pre-0.23 rows carry no `budget` stamp, so classify() falls back to the
# registry. The tier-move mechanism depends on the row's own stamp winning
# where it exists -- these prove the fallback still works.
FIXTURES["incidents/budget-unstamped.jsonl"] = pick(
    "benchmarks/air-0.19.0/ipc2014-agile.jsonl",
    lambda r: r.get("budget") is None)[:40]


# --- whole-file fixtures: the contention records ----------------------------
# Only 4 of the 76 conditions files on this box carry a per-sample `timeline`
# (the watcher only started writing one at 0.25, per PER-INSTANCE-RETRY.md
# step 1). The resume gate's entire contention side is untestable without
# them, and every one is .gitignore'd.
COPIES = {
    "conditions/timeline-mco-t2.json":
        "benchmarks/air25-entries/ipc2014-mco-t2.conditions.json",
    "conditions/timeline-numeric-opt.json":
        "benchmarks/air25-entries/ipc2023-numeric-opt.conditions.json",
    "conditions/timeline-opt-full.json":
        "benchmarks/air25-entries/ipc2026-opt-full.conditions.json",
    "conditions/timeline-complex-pref.json":
        "benchmarks/air25-entries/ipc5-complex-pref.conditions.json",
    # No timeline at all -- the fail-closed case the gate must reject rather
    # than treat as clean by omission. 72 of 76 real files look like this.
    "conditions/rollup-only.json":
        "benchmarks/air24/ipc2014-sat.conditions.json",
    # The 0.24 verdict change, on the board that forced it: an mco --threads 4
    # board burns 40-80% of this 10-core box BY DESIGN, so the old idle-floor
    # rule read DEGRADED in an empty room. These two files are the only
    # DEGRADED records on the box, and both are mco.
    "conditions/degraded-old-idle-rule-mco-t4.json":
        "benchmarks/air23/ipc7-mco-t4.conditions.json",
    "conditions/degraded-old-idle-rule-mco-t8.json":
        "benchmarks/air23/ipc7-mco-t8.conditions.json",
}


def main():
    manifest, bad = [], []
    for rel, src in sorted(COPIES.items()):
        body = open(os.path.join(ROOT, src)).read()
        dest = os.path.join(HERE, rel)
        digest = hashlib.blake2b(body.encode(), digest_size=8).hexdigest()
        manifest.append((rel, body.count(chr(10)), digest))
        if CHECK:
            have = open(dest).read() if os.path.exists(dest) else None
            if have != body:
                bad.append(rel)
        else:
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            with open(dest, "w") as f:
                f.write(body)

    for rel, lines in sorted(FIXTURES.items()):
        body = "".join(l + "\n" for l in lines)
        dest = os.path.join(HERE, rel)
        digest = hashlib.blake2b(body.encode(), digest_size=8).hexdigest()
        manifest.append((rel, len(lines), digest))
        if CHECK:
            have = open(dest).read() if os.path.exists(dest) else None
            if have != body:
                bad.append(rel)
        else:
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            with open(dest, "w") as f:
                f.write(body)

    for rel, n, digest in manifest:
        print(f"{n:5d} lines {digest}  {rel}")
    if CHECK and bad:
        print("\nFIXTURE DRIFT: " + ", ".join(bad), file=sys.stderr)
        return 1
    if CHECK:
        print("\nall fixtures match")
    return 0


if __name__ == "__main__":
    sys.exit(main())
