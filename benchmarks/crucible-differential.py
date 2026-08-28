#!/usr/bin/env python3
"""Prove crucible's pure layer agrees with the Python oracle, row by row.

Written in the house differential pattern (benchmarks/opt-differential.py):
resume-free, per-row verdict, named failures, non-zero exit on any MISMATCH.

Every transform crucible ports is a pure function of bytes already on disk, so
the whole historical corpus on this box -- every board raw under benchmarks/
and benchmarks/air*/ -- is available as an oracle without invoking the planner
once. That is the gate the port has to clear before crucible measures anything
that gets published (docs/roadmap-0.26.md).

There is no `inconclusive` verdict. The pure layer has no excuse for a
non-answer, and adding one would reproduce exactly the forgiveness that lets
divergences through.

    python3 benchmarks/crucible-differential.py [--transform classify,coverage]
                                                [--only SUBSTRING] [--out FILE]
"""
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "benchmarks"))
import standings as S  # noqa: E402

BIN = os.path.join(ROOT, "crucible", "target", "release", "crucible-replay")
VAL_MAP = os.path.join(ROOT, "benchmarks", "val-unavailable.json")


def arg(name, default=None):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


TRANSFORMS = arg("--transform", "classify,coverage,select").split(",")
ONLY = arg("--only")
OUT = arg("--out", os.path.join(ROOT, "benchmarks", "crucible-differential.jsonl"))


def boards():
    """(path, budget) for every board raw whose board name is in the registry.

    The registry supplies the budget; a raw whose rows carry their own stamps
    classifies off those either way, which is the point of the stamp."""
    seen = []
    for sub in ["", "air", "air21", "air22", "air23", "air24", "air25",
                "air25-entries", "air-0.18.0", "air-0.19.0", "air-0.21.0"]:
        d = os.path.join(ROOT, "benchmarks", sub)
        if not os.path.isdir(d):
            continue
        for f in sorted(os.listdir(d)):
            if not f.endswith(".jsonl"):
                continue
            reg = S.SWEEPS.get(f)
            if reg is None:
                # Staged boards use the board id; the registry uses the raw
                # name, and they differ for exactly one board.
                reg = S.SWEEPS.get({"ipc67-results.jsonl": "ipc67-default.jsonl"}
                                   .get(f, f))
            if reg is None:
                continue
            p = os.path.join(d, f)
            if ONLY and ONLY not in p:
                continue
            seen.append((p, reg[2]))
    return seen


def py_classify(path, budget):
    out = []
    with open(path) as fh:
        for line in fh:
            if not line.strip():
                continue
            try:
                r = json.loads(line)
            except ValueError:
                continue          # truncated tail from a killed pass
            out.append((f"{r.get('ipc') or ''}\t{r.get('variant')}\t"
                        f"{r.get('instance')}", S.classify(r, budget)))
    return out


def rs(cmd, path, budget):
    r = subprocess.run([BIN, cmd, "--raw", path, "--budget", str(budget),
                        "--val-map", VAL_MAP],
                       capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(f"crucible-replay {cmd} failed: {r.stderr.strip()}")
    return r.stdout.splitlines()


def main():
    if not os.path.exists(BIN):
        sys.exit(f"{BIN} not built -- "
                 "cargo build --release -p crucible-publish (in crucible/)")
    ledger, agree, mismatch, rows_checked = [], 0, [], 0

    for path, budget in boards():
        rel = os.path.relpath(path, ROOT)
        if "classify" in TRANSFORMS:
            want = py_classify(path, budget)
            got = [tuple(l.rsplit("\t", 1)) for l in rs("classify", path, budget)]
            rows_checked += len(want)
            if len(want) != len(got):
                mismatch.append((rel, "classify", f"row count {len(want)} vs {len(got)}"))
                ledger.append({"artifact": rel, "transform": "classify",
                               "verdict": "MISMATCH",
                               "detail": f"row count {len(want)} vs {len(got)}"})
            else:
                bad = [(k, a, b) for (k, a), (_, b) in zip(want, got) if a != b]
                if bad:
                    for k, a, b in bad[:5]:
                        mismatch.append((rel, "classify",
                                         f"{k}: python={a} rust={b}"))
                    ledger.append({"artifact": rel, "transform": "classify",
                                   "verdict": "MISMATCH", "n": len(bad)})
                else:
                    agree += 1
                    ledger.append({"artifact": rel, "transform": "classify",
                                   "verdict": "agree", "rows": len(want)})

        if "coverage" in TRANSFORMS:
            rows = S.load_jsonl(path)
            s, n, fails = S.coverage_line(rows, budget)
            want = f"{s}\t{n}\t{fails}"
            got = rs("coverage", path, budget)[0]
            if want != got:
                mismatch.append((rel, "coverage", f"\n      python: {want}\n      rust:   {got}"))
                ledger.append({"artifact": rel, "transform": "coverage",
                               "verdict": "MISMATCH", "py": want, "rs": got})
            else:
                agree += 1
                ledger.append({"artifact": rel, "transform": "coverage",
                               "verdict": "agree"})

    # THE CORPUS SELECTOR. Two of ipc67.py's TRACK_PATTERNS use negative
    # lookbehind, which Rust's regex crate cannot compile by design, so the
    # manifest expresses them as include/exclude pairs. Equivalence is not
    # something to reason about -- it is something to run. Selecting one
    # variant too many or too few silently changes a board's denominator.
    if "select" in TRANSFORMS:
        cru = os.path.join(ROOT, "crucible", "target", "debug", "crucible")
        if not os.path.exists(cru):
            cru = os.path.join(ROOT, "crucible", "target", "release", "crucible")
        if os.path.exists(cru):
            import importlib.util
            spec = importlib.util.spec_from_file_location(
                "ipc67", os.path.join(ROOT, "benchmarks", "ipc67.py"))
            I = importlib.util.module_from_spec(spec)
            argv, sys.argv = sys.argv, ["ipc67", "--list"]
            try:
                spec.loader.exec_module(I)
            except SystemExit:
                pass
            finally:
                sys.argv = argv
            for track in sorted(I.TRACK_PATTERNS):
                r = subprocess.run([cru, "--repo", ROOT, "list", "--track", track],
                                   capture_output=True, text=True)
                got = sorted(" ".join(l.split()[:2]) for l in r.stdout.splitlines() if l.strip())
                p = subprocess.run([sys.executable, os.path.join(ROOT, "benchmarks", "ipc67.py"),
                                    "--track", track, "--list"], capture_output=True, text=True)
                want = sorted(" ".join(l.split()[:2]) for l in p.stdout.splitlines() if l.strip())
                if got == want:
                    agree += 1
                    ledger.append({"artifact": track, "transform": "select",
                                   "verdict": "agree", "variants": len(want)})
                else:
                    only_rs = set(got) - set(want)
                    only_py = set(want) - set(got)
                    mismatch.append((track, "select",
                                     f"rust-only={sorted(only_rs)[:3]} python-only={sorted(only_py)[:3]}"))
                    ledger.append({"artifact": track, "transform": "select",
                                   "verdict": "MISMATCH"})
        else:
            print("SKIP select: crucible binary not built", file=sys.stderr)

    with open(OUT, "w") as f:
        for row in ledger:
            f.write(json.dumps(row) + "\n")

    print(f"{agree} agree, {len(mismatch)} MISMATCH "
          f"({rows_checked} rows classified, {len(boards())} boards)")
    print(f"ledger: {os.path.relpath(OUT, ROOT)}")
    if mismatch:
        print("\nfirst mismatches:")
        for rel, t, d in mismatch[:20]:
            print(f"  {rel} [{t}] {d}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
