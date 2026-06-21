#!/usr/bin/env python3
"""ferroplan performance harness — measure and TRACK whether we're improving.

Two subcommands:

  perf.py run [--corpus DIR] [--out FILE] [--threads N] [--timeout S] [--label L]
      Solve every (domain, problem) pair under the corpus and write a metrics
      JSON. The tracked metrics are DETERMINISTIC (machine-independent) so they
      compare cleanly across runs and machines:
        - coverage      (solved count)            — did we solve it
        - evaluated     (states expanded)         — search efficiency
        - length/metric (plan length / PDDL3 cost) — solution quality
      Wall-time (ms) is also recorded but is machine/load-dependent: use it for
      local profiling, NOT for the improvement verdict.

  perf.py compare BASELINE CURRENT
      Diff two metrics files and report improvements/regressions. The verdict is
      based only on the deterministic metrics; exits non-zero on a regression
      (coverage drop, more states evaluated, or worse plans) so it can gate CI.

The `ff` binary is taken from $FF (default target/release/ff).
"""
import argparse
import json
import math
import os
import subprocess
import sys
import time

FF = os.environ.get("FF", "target/release/ff")


def git_sha():
    try:
        return subprocess.run(["git", "rev-parse", "--short", "HEAD"],
                              capture_output=True, text=True).stdout.strip() or "?"
    except Exception:
        return "?"


def find_cases(corpus):
    """Yield (id, domain_path, problem_path) for every problem under corpus."""
    for root, _dirs, files in os.walk(corpus):
        if "domain.pddl" not in files:
            continue
        dom = os.path.join(root, "domain.pddl")
        probs = sorted(f for f in files
                       if f.endswith(".pddl") and f != "domain.pddl")
        rel = os.path.relpath(root, corpus)
        for p in probs:
            yield (f"{rel}/{p}", dom, os.path.join(root, p))


def run_corpus(corpus, threads, timeout):
    cases = sorted(find_cases(corpus))
    out = {}
    for cid, dom, prob in cases:
        t = time.perf_counter()
        try:
            r = subprocess.run([FF, "-o", dom, "-f", prob, "--json",
                                "--threads", str(threads)],
                               capture_output=True, text=True, timeout=timeout)
            ms = (time.perf_counter() - t) * 1000
            s = json.loads(r.stdout) if r.stdout.strip().startswith("{") else None
        except subprocess.TimeoutExpired:
            out[cid] = {"solved": False, "evaluated": None, "length": None,
                        "metric": None, "ms": timeout * 1000, "note": "timeout"}
            print(f"  {cid:46} TIMEOUT", file=sys.stderr)
            continue
        if not s:
            out[cid] = {"solved": False, "evaluated": None, "length": None,
                        "metric": None, "ms": ms, "note": "error"}
            continue
        plan = s.get("plan") or {}
        out[cid] = {
            "solved": bool(s.get("solved")),
            "evaluated": (s.get("statistics") or {}).get("evaluated_states"),
            "length": plan.get("length"),
            "metric": plan.get("metric"),
            "ms": round(ms, 1),
        }
        print(f"  {cid:46} {'ok' if out[cid]['solved'] else '--':3} "
              f"eval={out[cid]['evaluated']} len={out[cid]['length']} "
              f"{out[cid]['ms']:.0f}ms", file=sys.stderr)
    return out


def geomean(xs):
    xs = [x for x in xs if x and x > 0]
    return round(math.exp(sum(math.log(x) for x in xs) / len(xs)), 1) if xs else 0.0


def aggregate(probs):
    solved = [p for p in probs.values() if p["solved"]]
    return {
        "problems": len(probs),
        "solved": len(solved),
        "total_evaluated": sum(p["evaluated"] or 0 for p in solved),
        "geomean_evaluated": geomean([p["evaluated"] for p in solved]),
        "geomean_ms": geomean([p["ms"] for p in solved]),
    }


def cmd_run(args):
    probs = run_corpus(args.corpus, args.threads, args.timeout)
    doc = {
        "label": args.label,
        "git": git_sha(),
        "threads": args.threads,
        "timeout_s": args.timeout,
        "aggregate": aggregate(probs),
        "problems": probs,
    }
    text = json.dumps(doc, indent=2, sort_keys=True)
    if args.out:
        with open(args.out, "w") as f:
            f.write(text + "\n")
        print(f"wrote {args.out}", file=sys.stderr)
    else:
        print(text)
    a = doc["aggregate"]
    print(f"\n{a['solved']}/{a['problems']} solved | total_evaluated={a['total_evaluated']} "
          f"| geomean_eval={a['geomean_evaluated']} | geomean_ms={a['geomean_ms']}",
          file=sys.stderr)


def cmd_compare(args):
    base = json.load(open(args.baseline))
    cur = json.load(open(args.current))
    bp, cp = base["problems"], cur["problems"]
    keys = sorted(set(bp) | set(cp))

    regressions, improvements = [], []
    print(f"{'problem':46} {'eval Δ':>14} {'len Δ':>10}  verdict")
    print("-" * 86)
    for k in keys:
        b, c = bp.get(k), cp.get(k)
        if b is None:
            print(f"{k:46} {'(new)':>14}"); continue
        if c is None:
            print(f"{k:46} {'(removed)':>14}"); continue
        # coverage
        if b["solved"] and not c["solved"]:
            regressions.append(k)
            print(f"{k:46} {'':>14} {'':>10}  *** COVERAGE LOST"); continue
        if c["solved"] and not b["solved"]:
            improvements.append(k)
            print(f"{k:46} {'':>14} {'':>10}  +++ NOW SOLVED"); continue
        if not (b["solved"] and c["solved"]):
            continue
        de = (c["evaluated"] or 0) - (b["evaluated"] or 0)
        dl = (c["length"] or 0) - (b["length"] or 0)
        epct = (100.0 * de / b["evaluated"]) if b.get("evaluated") else 0.0
        verdict = ""
        if de > 0 and abs(epct) >= 1.0:
            verdict = "regression (more states)"; regressions.append(k)
        elif de < 0 and abs(epct) >= 1.0:
            verdict = "improvement (fewer states)"; improvements.append(k)
        if dl > 0:
            verdict = (verdict + " worse-plan").strip();
            if k not in regressions: regressions.append(k)
        elif dl < 0:
            verdict = (verdict + " better-plan").strip()
            if k not in improvements: improvements.append(k)
        if de or dl:
            print(f"{k:46} {de:>+10} {epct:>+5.1f}% {dl:>+10}  {verdict}")

    ba, ca = base["aggregate"], cur["aggregate"]
    print("\n=== aggregate (deterministic) ===")
    print(f"  coverage:        {ba['solved']} -> {ca['solved']}")
    print(f"  total_evaluated: {ba['total_evaluated']} -> {ca['total_evaluated']} "
          f"({pct(ba['total_evaluated'], ca['total_evaluated'])})")
    print(f"  geomean_eval:    {ba['geomean_evaluated']} -> {ca['geomean_evaluated']} "
          f"({pct(ba['geomean_evaluated'], ca['geomean_evaluated'])})")
    print(f"  geomean_ms:      {ba['geomean_ms']} -> {ca['geomean_ms']} "
          f"({pct(ba['geomean_ms'], ca['geomean_ms'])})  [machine-dependent, informational]")
    print(f"\n  improvements: {len(improvements)}   regressions: {len(regressions)}")
    if regressions:
        print("  REGRESSED:", ", ".join(regressions[:10]))
        sys.exit(1)
    print("  no deterministic regressions ✓")


def pct(a, b):
    if not a:
        return "n/a"
    d = 100.0 * (b - a) / a
    return f"{d:+.1f}%"


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("run", help="run the corpus, emit a metrics JSON")
    r.add_argument("--corpus", default="benchmarks/ipc")
    r.add_argument("--out")
    r.add_argument("--threads", type=int, default=1)
    r.add_argument("--timeout", type=float, default=30.0)
    r.add_argument("--label", default="")
    r.set_defaults(func=cmd_run)
    c = sub.add_parser("compare", help="diff two metrics JSONs (exit 1 on regression)")
    c.add_argument("baseline")
    c.add_argument("current")
    c.set_defaults(func=cmd_compare)
    args = ap.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
