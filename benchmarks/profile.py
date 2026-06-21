#!/usr/bin/env python3
"""Turnkey hotspot profiler (macOS): capture a sampling profile of one ferroplan
run and print the top self-time functions as TEXT — no browser needed.

    python3 benchmarks/profile.py <domain.pddl> <problem.pddl> [-- <extra ff args>]

It builds the profiling binary (release + debug symbols), runs it under `samply
record --save-only`, then symbolicates the hottest sample addresses with `atos`.
For the interactive flamegraph instead, use `samply record` directly (see
PROFILING.md). Requires `samply` (cargo install samply) and macOS `atos`.

Pick a workload that runs for more than ~1s, else there are too few samples.
"""
import collections
import gzip
import json
import os
import subprocess
import sys

MACHO_PIE_BASE = 0x100000000  # macOS default __TEXT preferred load address


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(2)
    dom, prob = sys.argv[1], sys.argv[2]
    extra = sys.argv[4:] if len(sys.argv) > 3 and sys.argv[3] == "--" else sys.argv[3:]

    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    binary = os.path.join(root, "target/profiling/ff")
    print("building profiling binary…", file=sys.stderr)
    subprocess.run(["cargo", "build", "--profile", "profiling", "-p", "ferroplan-cli"],
                   cwd=root, check=True)

    prof = "/tmp/ferroplan-profile.json.gz"
    cmd = ["samply", "record", "--save-only", "-o", prof, "--",
           binary, "-o", dom, "-f", prob, "--threads", "1", *extra]
    print("recording:", " ".join(cmd), file=sys.stderr)
    subprocess.run(cmd, check=False)

    d = json.load(gzip.open(prof))
    th = max(d["threads"], key=lambda t: len(t["samples"]["stack"]))
    addr = th["frameTable"]["address"]
    st_frame = th["stackTable"]["frame"]
    cnt = collections.Counter()
    for s in th["samples"]["stack"]:
        if s is None:
            continue
        a = addr[st_frame[s]]
        if a and a > 0:
            cnt[a] += 1
    total = sum(cnt.values())
    if not total:
        print("no samples captured (run too short?)", file=sys.stderr)
        sys.exit(1)

    print(f"\n{total} self-time samples — top hotspots:\n")
    for a, c in cnt.most_common(20):
        absaddr = hex(MACHO_PIE_BASE + a)
        sym = subprocess.run(
            ["atos", "-o", binary, "-l", hex(MACHO_PIE_BASE), absaddr],
            capture_output=True, text=True).stdout.strip()
        # trim the long mangled/hash suffixes for readability
        sym = sym.split(" (in ")[0]
        print(f"{100 * c / total:5.1f}%  {sym[:88]}")


if __name__ == "__main__":
    main()
