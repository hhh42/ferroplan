#!/usr/bin/env python3
"""Transcribe the five drifting registries into benchmarks/manifest.toml.

Today the same board is described in five places that can disagree:

  benchmarks/standings.py   SWEEPS          raw.jsonl -> (label, competition, budget)
                            AIR_REBASELINED which box produced it
                            PROOF_TRACKS    coverage IS proof rate
                            MD_FOR          the .md naming exceptions
  benchmarks/ipc67.py       TRACK_PATTERNS  regex over corpus variant dirs
                            TRACK_IPCS      which ipc-YYYY dirs to scan
  benchmarks/cut25-sweeps.sh    BOARDS=()   the standing 22
  benchmarks/entries25-sweeps.sh BOARDS=()  the 0.25 entries
  benchmarks/post-entries25.sh              the board the entries sweep missed

This reads all of them and emits ONE file. It is a transcription, not a new
source of truth: run with --check after any registry edit and it will refuse
to agree if the manifest has drifted from the Python.

    python3 crucible/tools/gen-manifest.py [--check]

Two TRACK_PATTERNS entries use negative lookbehind, which Rust's `regex`
crate cannot compile by design (it costs the linear-time guarantee). They are
emitted as include/exclude pairs, and the equivalence is PROVEN against every
variant directory on disk before the file is written -- see check_lookbehind.
"""
import importlib.util
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "benchmarks", "manifest.toml")
CHECK = "--check" in sys.argv


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

# The .md naming exception, inverted: raw filename -> board id.
BOARD_ID = {"ipc67-default.jsonl": "ipc67-results"}

# The two lookbehinds, hand-converted and machine-verified below.
LOOKBEHIND = {
    "time-2006":      ("-time(-strips)?$", "metric-time(-strips)?$"),
    "opt-2026-full":  ("-numeric-2026",    "-sat-numeric-2026"),
}


def corpus_variants():
    """Every variant directory on disk, as {ipc: [names]}. Empty if the
    corpus is absent -- the equivalence proof then reports itself skipped
    rather than silently passing."""
    root = os.environ.get("FERROPLAN_IPC_CORPUS") or os.path.join(
        ROOT, "benchmarks", ".ipc-corpus")
    out = {}
    if not os.path.isdir(root):
        return out
    for ipc in sorted(os.listdir(root)):
        d = os.path.join(root, ipc, "domains")
        if os.path.isdir(d):
            out[ipc] = sorted(os.listdir(d))
    return out


def check_lookbehind(variants):
    """Prove include/exclude selects EXACTLY what the lookbehind selects."""
    if not variants:
        return "SKIPPED (corpus absent)"
    for track, (inc, exc) in LOOKBEHIND.items():
        py = re.compile(I.TRACK_PATTERNS[track])
        ri, re_ = re.compile(inc), re.compile(exc)
        for ipc in I.TRACK_IPCS.get(track, ("ipc-2008", "ipc-2011")):
            for v in variants.get(ipc, []):
                a = bool(py.search(v))
                b = bool(ri.search(v)) and not re_.search(v)
                if a != b:
                    sys.exit(f"LOOKBEHIND MISMATCH {track} {ipc}/{v}: "
                             f"python={a} include/exclude={b}")
    n = sum(len(v) for v in variants.values())
    return f"verified over {n} variant dirs"


def boards_from(path):
    """Parse a sweep driver's BOARDS=() array: name track timeout [args...]."""
    m = re.search(r"BOARDS=\((.*?)\n\)", open(os.path.join(ROOT, path)).read(), re.S)
    out = []
    for line in m.group(1).splitlines():
        line = line.split("#")[0].strip().strip('"')
        if line:
            p = line.split()
            out.append((p[0], p[1], int(p[2]), p[3:]))
    return out


def parse_args(extra):
    """--mode optimal / --threads N -> structured fields.

    The mco wall-clock rule: a board carrying --threads runs ONE instance at
    a time whatever $JOBS says (cut25-sweeps.sh:83). The drivers infer that
    at runtime; the manifest ASSERTS it, so a board can never be scheduled
    against the rule by accident."""
    mode, threads, rest = None, 1, []
    it = iter(extra)
    for a in it:
        if a == "--mode":
            mode = next(it)
        elif a == "--threads":
            threads = int(next(it))
        else:
            rest.append(a)
    return mode, threads, (1 if threads > 1 else 2), rest


# (set name, stage, boards, required engine version)
REFEREE_SETS = [
    # F1 fallback enrichment: the old-binary leg (v0.25.0, the engine the
    # cut shipped) and the armed candidate, on the two boards the band claims.
    ("f1-before", "benchmarks/air26-f1-before", ["ipc5-prop", "ipc2018-sat"], "0.25"),
    ("f1-armed", "benchmarks/air26-f1", ["ipc5-prop", "ipc2018-sat"], "0.26"),
]


def q(s):
    return '"' + str(s).replace("\\", "\\\\").replace('"', '\\"') + '"'


def main():
    variants = corpus_variants()
    proof = check_lookbehind(variants)

    # board id -> (track, timeout, extra args, which set it belongs to)
    spec_of, sets = {}, []
    for set_name, path, stage in (
            ("cut25", "benchmarks/cut25-sweeps.sh", "benchmarks/air25"),
            ("entries25", "benchmarks/entries25-sweeps.sh", "benchmarks/air25-entries")):
        ids = []
        for bid, track, tmo, extra in boards_from(path):
            spec_of[bid] = (track, tmo, extra)
            ids.append(bid)
        sets.append((set_name, stage, ids, "0.25"))

    # The board the entries sweep MISSED: registered in SWEEPS, swept by
    # post-entries25.sh, in no BOARDS array. The manifest is where that stops
    # being possible.
    orphans = []
    for raw, (label, comp, budget) in S.SWEEPS.items():
        bid = BOARD_ID.get(raw, raw[:-len(".jsonl")])
        if bid not in spec_of:
            spec_of[bid] = ("complex-pref-2006" if "complex-pref" in bid else "?",
                            budget, [])
            orphans.append(bid)
    if orphans:
        sets.append(("post-entries25", "benchmarks/air25-entries", orphans, "0.25"))

    # Referee sets for the 0.26 builds (docs/field-gaps-execution-0.26.md):
    # the same boards measured twice on the same box, once by the previous
    # release's engine and once by the candidate, each into its own stage.
    # Declared here rather than in a shell driver, because from 0.26 on
    # crucible IS the driver and there is no BOARDS array to transcribe.
    for name, stage, ids, ver in REFEREE_SETS:
        for bid in ids:
            assert bid in spec_of, f"referee set {name}: unknown board {bid}"
        sets.append((name, stage, ids, ver))

    L = ["# benchmarks/manifest.toml -- the sweep instrument, versioned with the planner.",
         "#",
         "# GENERATED by crucible/tools/gen-manifest.py from the registries it",
         "# consolidates. Re-verify with `--check` after editing either side.",
         "#",
         f"# Lookbehind equivalence: {proof}.",
         "",
         "schema = 1",
         "",
         "[corpus]",
         'root = ".ipc-corpus"          # relative to benchmarks/; $FERROPLAN_IPC_CORPUS wins',
         'domain_shared = "domain.pddl"',
         'domain_per_instance = "domains/domain-{first}.pddl"',
         "",
         "[defaults]",
         "timeout_secs = 60",
         "jobs = 2                      # docs/migration-m5.md: 2 not 3 -- fanless box,",
         "                              # clocks must stay stable across a multi-hour sweep",
         "threads = 1",
         'mode = "auto"',
         "mem_gb = 6.0                  # not the phys/jobs default of 8: 2x6 against 16 GiB",
         "                              # leaves headroom for the 0.25 s RSS poll",
         "",
         "# ---------------------------------------------------------------------------",
         "# TRACKS -- the corpus selector. NOT an enumeration: the corpus is gitignored,",
         "# so a list of 6,584 instances would drift from disk with nothing to notice.",
         "# From ipc67.py TRACK_PATTERNS + TRACK_IPCS.",
         "# ---------------------------------------------------------------------------",
         ""]

    for track in sorted(I.TRACK_PATTERNS):
        ipcs = I.TRACK_IPCS.get(track, ("ipc-2008", "ipc-2011"))
        L.append(f"[track.{track}]")
        L.append("ipcs = [" + ", ".join(q(i) for i in ipcs) + "]")
        if track in LOOKBEHIND:
            inc, exc = LOOKBEHIND[track]
            L.append(f"include = {q(inc)}")
            L.append(f"exclude = {q(exc)}   # was Python's "
                     f"{I.TRACK_PATTERNS[track].split('(?<!')[1].split(')')[0]!r} lookbehind")
        else:
            L.append(f"include = {q(I.TRACK_PATTERNS[track])}")
        L.append("")

    L += ["# ---------------------------------------------------------------------------",
          "# BOARDS -- the unit of work AND the unit of row identity. The resume gate",
          "# compares (budget, mode, jobs, threads) EXACTLY, so this table is the",
          "# tuple every row is stamped with.",
          "# From standings.py SWEEPS/PROOF_TRACKS/AIR_REBASELINED + the BOARDS arrays.",
          "# ---------------------------------------------------------------------------",
          ""]

    for raw, (label, comp, budget) in S.SWEEPS.items():
        bid = BOARD_ID.get(raw, raw[:-len(".jsonl")])
        track, tmo, extra = spec_of[bid]
        mode, threads, jobs, rest = parse_args(extra)
        L.append("[[board]]")
        L.append(f"id = {q(bid)}")
        L.append(f"raw = {q(raw)}")
        if bid != raw[:-len(".jsonl")]:
            L.append(f"md = {q(bid + '.md')}   # THE one naming exception, as data not code")
        else:
            L.append(f"md = {q(bid + '.md')}")
        L.append(f"label = {q(label)}")
        L.append(f"competition = {q(comp)}")
        L.append(f"budget_secs = {budget}")
        L.append(f"track = {q(track)}")
        if tmo != budget:
            L.append(f"timeout_secs = {tmo}   # DIFFERS from budget_secs: a tier move in flight")
        if mode:
            L.append(f"mode = {q(mode)}")
        if threads != 1:
            L.append(f"threads = {threads}")
            L.append("jobs = 1                     # the mco wall-clock rule, asserted")
        if rest:
            L.append("extra_args = [" + ", ".join(q(a) for a in rest) + "]")
        if label in S.PROOF_TRACKS:
            L.append("proof_track = true           # coverage IS proof rate")
        L.append("rebaselined_on = [" +
                 ('"m5-air"' if label in S.AIR_REBASELINED else "") + "]")
        L.append("")

    L += ["# ---------------------------------------------------------------------------",
          "# SETS -- what a driver invocation sweeps. Two staging dirs by design: the",
          "# standing 22 keep their like-for-like identity, the entries stage apart,",
          "# and the cut record carries TWO headlines.",
          "# ---------------------------------------------------------------------------",
          ""]
    for name, stage, ids, ver in sets:
        L.append("[[set]]")
        L.append(f"name = {q(name)}")
        L.append(f"stage = {q(stage)}")
        L.append(f"requires_version = {q(ver)}")
        if name == "post-entries25":
            L.append("# Registered in SWEEPS and swept by post-entries25.sh, but absent from")
            L.append("# every BOARDS array -- the entries driver was not edited mid-run.")
        L.append("boards = [")
        for i in ids:
            L.append(f"  {q(i)},")
        L.append("]")
        L.append("")

    body = "\n".join(L)
    if CHECK:
        have = open(OUT).read() if os.path.exists(OUT) else None
        if have != body:
            print("MANIFEST DRIFT: benchmarks/manifest.toml differs from the registries",
                  file=sys.stderr)
            return 1
        print(f"manifest matches the registries ({proof})")
        return 0
    with open(OUT, "w") as f:
        f.write(body)
    print(f"wrote {os.path.relpath(OUT, ROOT)}: "
          f"{len(I.TRACK_PATTERNS)} tracks, {len(S.SWEEPS)} boards, {len(sets)} sets")
    print(f"lookbehind equivalence: {proof}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
