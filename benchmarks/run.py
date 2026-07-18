#!/usr/bin/env python3
"""Run ferroplan over the vendored IPC corpus and emit a results table.

Builds the release `ff`, runs it (`--json`) on every problem under
benchmarks/ipc/, and writes a Markdown table to benchmarks/results.md (consumed
by the docs site). Reports status, plan length, metric/cost, wall-clock, and —
when the VAL validator is available — external validation of every plan.

Corpus layout: benchmarks/ipc/<category>/<domain>/pNN.pddl with either a single
domain.pddl or a per-instance pNN-domain.pddl (IPC-2008 style).

VAL: point $FERROPLAN_VAL at the `Validate` binary (or have it on PATH); build
it with benchmarks/get-val.sh. Without VAL the val column shows "-" — plans are
then only checked by ferroplan's own semantics, and scoreboard claims should
say so.

Usage:  python3 benchmarks/run.py [--timeout N] [--only cat[/dom]]
"""
import json, os, re, shutil, subprocess, sys, tempfile, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CORPUS = os.path.join(ROOT, "benchmarks", "ipc")
FF = os.path.join(ROOT, "target", "release", "ff")
TIMEOUT = int(sys.argv[sys.argv.index("--timeout") + 1]) if "--timeout" in sys.argv else 30
ONLY = sys.argv[sys.argv.index("--only") + 1] if "--only" in sys.argv else None


def find_val():
    p = os.environ.get("FERROPLAN_VAL")
    if p and os.path.isfile(p):
        return p
    return shutil.which("Validate")


def problems():
    out = []
    for cat in sorted(os.listdir(CORPUS)):
        cdir = os.path.join(CORPUS, cat)
        if not os.path.isdir(cdir):
            continue
        for dom in sorted(os.listdir(cdir)):
            ddir = os.path.join(cdir, dom)
            if not os.path.isdir(ddir):
                continue
            if ONLY and not f"{cat}/{dom}".startswith(ONLY):
                continue
            shared = os.path.join(ddir, "domain.pddl")
            for f in sorted(x for x in os.listdir(ddir) if re.fullmatch(r"p\d+\.pddl", x)):
                pn = f[:-5]
                per = os.path.join(ddir, f"{pn}-domain.pddl")
                d = per if os.path.isfile(per) else shared
                if os.path.isfile(d):
                    out.append((cat, dom, pn, d, os.path.join(ddir, f)))
    return out


def val_check(val, domain, problem, steps):
    """Validate a sequential plan with VAL; returns 'ok', 'FAIL', or 'err'."""
    with tempfile.NamedTemporaryFile("w", suffix=".plan", delete=False) as f:
        for s in steps:
            f.write("(" + " ".join([s["action"]] + s.get("args", [])).lower() + ")\n")
        path = f.name
    try:
        r = subprocess.run([val, domain, problem, path],
                           capture_output=True, text=True, timeout=60)
        ok = r.returncode == 0 and "Plan valid" in r.stdout
        return "ok" if ok else "FAIL"
    except Exception:
        return "err"
    finally:
        os.unlink(path)


def main():
    print("building release ff ...", flush=True)
    subprocess.run(["cargo", "build", "--release", "-q", "-p", "ferroplan-cli"],
                   cwd=ROOT, check=True)
    val = find_val()
    print(f"VAL: {val or 'not found (external validation skipped)'}", flush=True)
    rows = []
    for cat, dom, pn, d, p in problems():
        t = time.perf_counter()
        v = "-"
        try:
            r = subprocess.run([FF, "-o", d, "-f", p, "--json", "--threads", "1"],
                               capture_output=True, text=True, timeout=TIMEOUT)
            ms = (time.perf_counter() - t) * 1000
            s = json.loads(r.stdout) if r.stdout.strip() else {}
            status = "solved" if s.get("solved") else "unsolved"
            plan = s.get("plan") or {}
            ln, metric = plan.get("length", "-"), plan.get("metric")
            if val and status == "solved" and plan.get("makespan") is None:
                v = val_check(val, d, p, plan.get("steps", []))
        except subprocess.TimeoutExpired:
            ms, status, ln, metric = TIMEOUT * 1000, "timeout", "-", None
        except Exception:
            ms, status, ln, metric = 0, "error", "-", None
        rows.append((f"{cat}/{dom}/{pn}", status, ln, metric, ms, v))
        print(f"  {cat}/{dom}/{pn:5} {status:8} len={ln} metric={metric} val={v} {ms:.0f}ms",
              flush=True)

    out = ["# ferroplan benchmark results\n",
           f"{len(rows)} problems, timeout {TIMEOUT}s."
           + (" Plans externally validated with VAL." if val else
              " VAL not available: plans checked by ferroplan's own semantics only.")
           + "\n",
           "| problem | status | len | metric | val | time |",
           "|---|---|---|---|---|---|"]
    for name, st, ln, m, ms, v in rows:
        out.append(f"| {name} | {st} | {ln} | {m if m is not None else ''} | {v} | {ms:.0f}ms |")
    if not ONLY:
        open(os.path.join(ROOT, "benchmarks", "results.md"), "w").write("\n".join(out) + "\n")
        print(f"\nwrote benchmarks/results.md", end="")
    solved = sum(1 for r in rows if r[1] == "solved")
    bad = sum(1 for r in rows if r[5] == "FAIL")
    print(f"\n{solved}/{len(rows)} solved" + (f", {bad} VAL FAILURES" if bad else ""))
    if bad:
        sys.exit(1)


if __name__ == "__main__":
    main()
