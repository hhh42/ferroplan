#!/usr/bin/env python3
"""Drive the encoding A/B/C experiment end to end and write RESULTS.md.

For every (content, encoding, mode) it solves the matched corpus and records the
deterministic metrics, then assembles a 3-way comparison:

  instantaneous : evaluated_states (node expansions)  <- the headline "who searches
                  better"; plus plan length + coverage + wall-clock.
  temporal      : makespan + plan length + coverage + wall-clock (the classical FF
                  evaluated_states is 0 on the temporal path, so it is omitted there).

Instantaneous corpora are solved via the existing benchmarks/perf.py (so its
`compare` works on the emitted JSON); temporal corpora are solved here with
`--mode temporal` (perf.py does not pass a mode). Both write the same JSON schema.

Usage:
  python3 benchmarks/encoding-ab/run_experiment.py [--build] [--contents chain converge]
                  [--max-evaluated N] [--timeout S] [--threads T]
Env: FF=/abs/path/to/ff  (default: ferroplan/target/release/ff)
"""
import argparse
import json
import math
import os
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
BENCH = os.path.dirname(HERE)                       # benchmarks/
FERRO = os.path.dirname(BENCH)                      # ferroplan/
sys.path.insert(0, BENCH)
import perf  # find_cases, geomean  (reused corpus walker + geomean)

FF = os.environ.get("FF", os.path.join(FERRO, "target", "release", "ff"))
GEN = os.path.join(HERE, "gen.py")
CORPORA = os.path.join(HERE, "corpora")
METRICS = os.path.join(HERE, "metrics")
ENCODINGS = ["specific", "data-table", "forall"]
MODES = ["inst", "temporal"]


def sh(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def git_sha():
    r = sh(["git", "-C", FERRO, "rev-parse", "--short", "HEAD"])
    return r.stdout.strip() or "?"


def solve(domain, problem, mode, max_eval, timeout, threads):
    """Solve one instance, return a perf.py-schema record."""
    cmd = [FF, "-o", domain, "-f", problem, "--json", "--threads", str(threads)]
    if mode == "temporal":
        cmd += ["--mode", "temporal"]
    if max_eval:
        cmd += ["--max-evaluated", str(max_eval)]
    t = time.perf_counter()
    try:
        r = sh(cmd, timeout=timeout)
        ms = (time.perf_counter() - t) * 1000
        d = json.loads(r.stdout) if r.stdout.strip().startswith("{") else None
    except subprocess.TimeoutExpired:
        return {"solved": False, "evaluated": None, "length": None, "metric": None,
                "makespan": None, "ms": timeout * 1000, "note": "timeout"}
    if not d:
        return {"solved": False, "evaluated": None, "length": None, "metric": None,
                "makespan": None, "ms": ms, "note": "error"}
    pl = d.get("plan") or {}
    st = d.get("statistics") or {}
    return {
        "solved": bool(d.get("solved")),
        "evaluated": st.get("evaluated_states"),
        "grounded_actions": st.get("grounded_actions"),
        "grounded_facts": st.get("grounded_facts"),
        "length": pl.get("length"),
        "metric": pl.get("metric"),
        "makespan": pl.get("makespan"),
        "ms": round(ms, 1),
    }


def run_corpus(corpus, mode, label, args):
    probs = {}
    for cid, dom, prob in sorted(perf.find_cases(corpus)):
        rec = solve(dom, prob, mode, args.max_evaluated, args.timeout, args.threads)
        probs[cid] = rec
        flag = "ok" if rec["solved"] else "--"
        extra = f"mk={rec['makespan']}" if mode == "temporal" else f"ev={rec['evaluated']}"
        print(f"  {label:28} {cid:18} {flag:3} {extra} len={rec['length']} {rec['ms']:.0f}ms",
              file=sys.stderr)
    solved = [p for p in probs.values() if p["solved"]]
    agg = {
        "problems": len(probs),
        "solved": len(solved),
        "total_evaluated": sum(p["evaluated"] or 0 for p in solved),
        "geomean_evaluated": perf.geomean([p["evaluated"] for p in solved]),
        "geomean_makespan": perf.geomean([p["makespan"] for p in solved]),
        "geomean_ms": perf.geomean([p["ms"] for p in solved]),
    }
    return {"label": label, "git": git_sha(), "threads": args.threads,
            "timeout_s": args.timeout, "max_evaluated": args.max_evaluated,
            "aggregate": agg, "problems": probs}


# ----------------------------------------------------------------------------- #
# RESULTS.md assembly
# ----------------------------------------------------------------------------- #

def fmt(v):
    return "-" if v is None else (f"{v:g}" if isinstance(v, float) else str(v))


def per_size_table(docs, mode):
    """docs: {enc: metricdoc}. Rows = problem ids (k/d tag + qty), cols = encodings.
    Cell = evaluated_states (inst) or makespan (temporal); '×' if unsolved."""
    cids = sorted({c for d in docs.values() for c in d["problems"]})
    metric = "evaluated" if mode == "inst" else "makespan"
    head = f"| problem | " + " | ".join(ENCODINGS) + " | winner |"
    sep = "|" + "---|" * (len(ENCODINGS) + 2)
    rows = [head, sep]
    for cid in cids:
        cells, vals = [], {}
        for enc in ENCODINGS:
            p = docs[enc]["problems"].get(cid, {})
            if p.get("solved"):
                v = p.get(metric)
                vals[enc] = v
                cells.append(fmt(v))
            else:
                cells.append("×")
        if not vals:
            winner = "-"
        else:
            best = min(vals.values())
            winners = [e for e in ENCODINGS if vals.get(e) == best]
            winner = "all tie" if len(winners) == len(ENCODINGS) else "+".join(winners)
        rows.append(f"| {cid} | " + " | ".join(cells) + f" | {winner} |")
    return "\n".join(rows)


def assemble_results(all_docs, args):
    sample = next(iter(all_docs.values()))  # stored run settings (accurate for report-only)
    L = []
    L.append("# Encoding A/B/C results — specific vs data-table vs forall-numeric\n")
    L.append(f"_ff @ `{sample.get('git', git_sha())}` · threads={sample.get('threads')} · "
             f"max_evaluated={sample.get('max_evaluated')} · timeout={sample.get('timeout_s')}s · "
             f"lower is better (fewer node expansions / shorter makespan)._\n")
    L.append("**specific** = one `:action` per recipe · **data-table** = one `craft "
             "?rec ?in ?out` + static `(recipe …)` table over `:constants` · "
             "**forall** = one `craft ?rec` quantifying over all resources via "
             "`(need/make ?rec ?res)`.\n")

    for content in args.contents:
        for mode in MODES:
            docs = {enc: all_docs[(content, enc, mode)] for enc in ENCODINGS}
            metric = "evaluated_states (node expansions)" if mode == "inst" else "makespan"
            L.append(f"\n## {content} — {mode}  ·  metric = {metric}\n")
            # coverage line
            cov = " · ".join(f"{enc}: {docs[enc]['aggregate']['solved']}/"
                             f"{docs[enc]['aggregate']['problems']}" for enc in ENCODINGS)
            L.append(f"coverage — {cov}\n")
            L.append(per_size_table(docs, mode))
            # aggregate
            if mode == "inst":
                agg = " · ".join(
                    f"**{enc}** total_eval={docs[enc]['aggregate']['total_evaluated']}, "
                    f"geomean_eval={docs[enc]['aggregate']['geomean_evaluated']}, "
                    f"geomean_ms={docs[enc]['aggregate']['geomean_ms']}"
                    for enc in ENCODINGS)
            else:
                agg = " · ".join(
                    f"**{enc}** geomean_makespan={docs[enc]['aggregate']['geomean_makespan']}, "
                    f"geomean_ms={docs[enc]['aggregate']['geomean_ms']}"
                    for enc in ENCODINGS)
            L.append(f"\naggregate — {agg}\n")
    L.append("\n## Pairwise (instantaneous, via perf.py compare)\n")
    L.append("```\n" + assemble_pairwise(args) + "```\n")
    return "\n".join(L)


def assemble_pairwise(args):
    """Run perf.py compare for data-table-vs-specific and forall-vs-specific on each
    content's inst metrics, capturing the per-problem eval deltas. (Treat 'specific'
    as the baseline; perf.py prints each generic encoding's eval delta against it.)"""
    out = []
    for content in args.contents:
        spec = os.path.join(METRICS, f"{content}-specific-inst.json")
        for enc in ["data-table", "forall"]:
            cur = os.path.join(METRICS, f"{content}-{enc}-inst.json")
            out.append(f"$ perf.py compare {content}-specific-inst {content}-{enc}-inst")
            r = sh([sys.executable, os.path.join(BENCH, "perf.py"), "compare", spec, cur])
            out.append((r.stdout or r.stderr).rstrip() + "\n")
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--build", action="store_true", help="cargo build --release first")
    ap.add_argument("--contents", nargs="+", choices=["chain", "converge", "techtree"],
                    default=["chain", "converge"])
    ap.add_argument("--max-evaluated", type=int, default=2_000_000)
    ap.add_argument("--timeout", type=float, default=60.0)
    ap.add_argument("--threads", type=int, default=1)
    ap.add_argument("--report-only", action="store_true",
                    help="skip solving; reassemble RESULTS.md from existing metrics/*.json")
    args = ap.parse_args()

    if args.report_only:
        all_docs = {}
        for content in args.contents:
            for enc in ENCODINGS:
                for mode in MODES:
                    path = os.path.join(METRICS, f"{content}-{enc}-{mode}.json")
                    with open(path) as f:
                        all_docs[(content, enc, mode)] = json.load(f)
        with open(os.path.join(HERE, "RESULTS.md"), "w") as f:
            f.write(assemble_results(all_docs, args) + "\n")
        print(f"wrote {os.path.join(HERE, 'RESULTS.md')} (report-only)", file=sys.stderr)
        return

    if args.build:
        print("building ff ...", file=sys.stderr)
        b = sh(["cargo", "build", "--release", "-p", "ferroplan-cli"], cwd=FERRO)
        if b.returncode:
            print(b.stderr[-2000:], file=sys.stderr); sys.exit(1)
    if not os.path.exists(FF):
        print(f"ff not found at {FF} (use --build or set $FF)", file=sys.stderr); sys.exit(1)

    print("generating corpora ...", file=sys.stderr)
    sh([sys.executable, GEN, "emit-corpora", "--out", CORPORA, "--contents", *args.contents])
    os.makedirs(METRICS, exist_ok=True)

    all_docs = {}
    for content in args.contents:
        for enc in ENCODINGS:
            for mode in MODES:
                corpus = os.path.join(CORPORA, content, f"{enc}-{mode}")
                label = f"{content}-{enc}-{mode}"
                print(f"== {label} ==", file=sys.stderr)
                doc = run_corpus(corpus, mode, label, args)
                with open(os.path.join(METRICS, f"{label}.json"), "w") as f:
                    json.dump(doc, f, indent=2, sort_keys=True)
                all_docs[(content, enc, mode)] = doc

    results = assemble_results(all_docs, args)
    with open(os.path.join(HERE, "RESULTS.md"), "w") as f:
        f.write(results + "\n")
    print(f"\nwrote {os.path.join(HERE, 'RESULTS.md')}", file=sys.stderr)


if __name__ == "__main__":
    main()
