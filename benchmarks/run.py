#!/usr/bin/env python3
"""Run ferroplan over the vendored IPC corpus and emit a results table.

Builds the release `ff`, runs it (`--json`) on every problem under
benchmarks/ipc/, and writes a Markdown table to benchmarks/results.md (consumed
by the docs site). Reports status, plan length, PDDL3 metric, and wall-clock.

Usage:  python3 benchmarks/run.py [--timeout N]
"""
import json, os, re, subprocess, sys, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CORPUS = os.path.join(ROOT, "benchmarks", "ipc")
FF = os.path.join(ROOT, "target", "release", "ff")
TIMEOUT = int(sys.argv[sys.argv.index("--timeout") + 1]) if "--timeout" in sys.argv else 30


def problems():
    out = []
    for cat in sorted(os.listdir(CORPUS)):
        cdir = os.path.join(CORPUS, cat)
        if not os.path.isdir(cdir):
            continue
        for dom in sorted(os.listdir(cdir)):
            ddir = os.path.join(cdir, dom)
            d = os.path.join(ddir, "domain.pddl")
            if not os.path.isfile(d):
                continue
            for f in sorted(x for x in os.listdir(ddir) if re.match(r"p\d+\.pddl", x)):
                out.append((cat, dom, f[:-5], d, os.path.join(ddir, f)))
    return out


def main():
    print("building release ff ...", flush=True)
    subprocess.run(["cargo", "build", "--release", "-q"], cwd=ROOT, check=True)
    rows = []
    for cat, dom, pn, d, p in problems():
        t = time.perf_counter()
        try:
            r = subprocess.run([FF, "-o", d, "-f", p, "--json", "--threads", "1"],
                               capture_output=True, text=True, timeout=TIMEOUT)
            ms = (time.perf_counter() - t) * 1000
            s = json.loads(r.stdout) if r.stdout.strip() else {}
            status = "solved" if s.get("solved") else "unsolved"
            plan = s.get("plan") or {}
            ln, metric = plan.get("length", "-"), plan.get("metric")
        except subprocess.TimeoutExpired:
            ms, status, ln, metric = TIMEOUT * 1000, "timeout", "-", None
        except Exception as e:
            ms, status, ln, metric = 0, f"error", "-", None
        rows.append((f"{cat}/{dom}/{pn}", status, ln, metric, ms))
        print(f"  {cat}/{dom}/{pn:5} {status:8} len={ln} {ms:.0f}ms", flush=True)

    out = ["# ferroplan benchmark results\n",
           f"{len(rows)} problems, timeout {TIMEOUT}s.\n",
           "| problem | status | len | metric | time |",
           "|---|---|---|---|---|"]
    for name, st, ln, m, ms in rows:
        out.append(f"| {name} | {st} | {ln} | {m if m is not None else ''} | {ms:.0f}ms |")
    open(os.path.join(ROOT, "benchmarks", "results.md"), "w").write("\n".join(out) + "\n")
    print(f"\nwrote benchmarks/results.md ({sum(1 for r in rows if r[1]=='solved')}/{len(rows)} solved)")


if __name__ == "__main__":
    main()
