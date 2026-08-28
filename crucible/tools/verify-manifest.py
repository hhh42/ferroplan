#!/usr/bin/env python3
"""Prove benchmarks/manifest.toml still says exactly what the Python says.

The manifest consolidates five registries. Consolidation is only a win if the
copy cannot drift from the original -- otherwise it is a sixth place to be
wrong. This is the gate: it parses the TOML and asserts, field by field,
that it reproduces SWEEPS, PROOF_TRACKS, AIR_REBASELINED, MD_FOR,
TRACK_PATTERNS and TRACK_IPCS, and that every track selector picks exactly
the variants the Python regex picks over the corpus on disk.

    python3 crucible/tools/verify-manifest.py

Exits non-zero and names every disagreement.
"""
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import importlib.util  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, os.path.join(ROOT, path))
    mod = importlib.util.module_from_spec(spec)
    argv, sys.argv = sys.argv, [name, "--list"]
    try:
        spec.loader.exec_module(mod)
    except SystemExit:
        pass
    finally:
        sys.argv = argv
    return mod


S = load("standings", "benchmarks/standings.py")
I = load("ipc67", "benchmarks/ipc67.py")
M = tomllib.load(open(os.path.join(ROOT, "benchmarks", "manifest.toml"), "rb"))

bad = []


def eq(what, a, b):
    if a != b:
        bad.append(f"{what}: manifest={a!r} python={b!r}")


# ---- boards reproduce SWEEPS / PROOF_TRACKS / AIR_REBASELINED / MD_FOR -----
by_raw = {b["raw"]: b for b in M["board"]}
eq("board count", len(M["board"]), len(S.SWEEPS))
eq("raw filenames", sorted(by_raw), sorted(S.SWEEPS))

for raw, (label, comp, budget) in S.SWEEPS.items():
    b = by_raw.get(raw)
    if b is None:
        bad.append(f"{raw}: missing from manifest")
        continue
    eq(f"{raw} label", b["label"], label)
    eq(f"{raw} competition", b["competition"], comp)
    eq(f"{raw} budget_secs", b["budget_secs"], budget)
    eq(f"{raw} proof_track", b.get("proof_track", False), label in S.PROOF_TRACKS)
    eq(f"{raw} rebaselined", bool(b.get("rebaselined_on")), label in S.AIR_REBASELINED)

# The .md naming exceptions, as standings.py's main() applies them.
MD_FOR = {"ipc67-default.jsonl": "ipc67-results.md",
          "ipc67-temporal.jsonl": "ipc67-temporal.md"}
for raw, b in by_raw.items():
    eq(f"{raw} md", b["md"], MD_FOR.get(raw, raw.replace(".jsonl", ".md")))

# Exactly ONE board may have id != raw stem, and it must be ipc67-results.
odd = [b["id"] for b in M["board"] if b["id"] != b["raw"][:-len(".jsonl")]]
eq("boards whose id != raw stem", odd, ["ipc67-results"])

# ---- tracks reproduce TRACK_PATTERNS / TRACK_IPCS --------------------------
eq("track count", len(M["track"]), len(I.TRACK_PATTERNS))
eq("track names", sorted(M["track"]), sorted(I.TRACK_PATTERNS))
for name, t in M["track"].items():
    want = I.TRACK_IPCS.get(name, ("ipc-2008", "ipc-2011"))
    eq(f"track {name} ipcs", tuple(t["ipcs"]), tuple(want))

# ---- the selectors pick EXACTLY the same variants, over the real corpus ----
corpus = os.environ.get("FERROPLAN_IPC_CORPUS") or os.path.join(
    ROOT, "benchmarks", ".ipc-corpus")
checked = 0
if os.path.isdir(corpus):
    dirs = {ipc: sorted(os.listdir(os.path.join(corpus, ipc, "domains")))
            for ipc in sorted(os.listdir(corpus))
            if os.path.isdir(os.path.join(corpus, ipc, "domains"))}
    for name, t in M["track"].items():
        py = re.compile(I.TRACK_PATTERNS[name])
        inc = re.compile(t["include"])
        exc = re.compile(t["exclude"]) if "exclude" in t else None
        for ipc in t["ipcs"]:
            for v in dirs.get(ipc, []):
                checked += 1
                a = bool(py.search(v))
                b = bool(inc.search(v)) and not (exc and exc.search(v))
                if a != b:
                    bad.append(f"selector {name} {ipc}/{v}: python={a} manifest={b}")
else:
    print("WARNING: corpus absent -- selector equivalence NOT checked",
          file=sys.stderr)

if bad:
    print(f"{len(bad)} disagreement(s):", file=sys.stderr)
    for b in bad[:40]:
        print("  " + b, file=sys.stderr)
    sys.exit(1)

print(f"manifest agrees with the registries: {len(M['board'])} boards, "
      f"{len(M['track'])} tracks, {checked} selector decisions checked")
