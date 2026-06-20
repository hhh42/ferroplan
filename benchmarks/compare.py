#!/usr/bin/env python3
"""Cross-planner comparison: ferroplan vs the C reference planners.

Runs ferroplan, Metric-FF (Rosetta), and SGPlan6 (Docker) over a corpus and
reports, per problem, solved/plan-length/PDDL3-metric/time — plus a summary:
relative speed (geomean vs Metric-FF) and an IPC-5 metric scoreboard
(ferroplan vs SGPlan6: wins / ties / losses).

Oracle paths are configurable (env or flags) and gracefully skipped if absent:
  FF_METRICFF   path to the x86_64 Metric-FF `ff` binary (run via `arch -x86_64`)
  FF_SGPLAN6    path to the sgplan6 binary (run in a linux/386 Docker container)

Usage:
  python3 benchmarks/compare.py [--corpus DIR] [--cat a,b,c] [--timeout N]
                                [--no-docker] [--no-rosetta]
"""
import json, math, os, re, shutil, subprocess, sys, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def arg(flag, default=None):
    return sys.argv[sys.argv.index(flag) + 1] if flag in sys.argv else default


CORPUS = arg("--corpus", os.path.join(ROOT, "benchmarks", "ipc"))
CATS = (arg("--cat") or "numeric,strips,adl,pref").split(",")
TIMEOUT = int(arg("--timeout", "30"))
NO_DOCKER = "--no-docker" in sys.argv
NO_ROSETTA = "--no-rosetta" in sys.argv
FF = os.path.join(ROOT, "target", "release", "ff")
METRICFF = os.environ.get("FF_METRICFF", os.path.join(ROOT, "..", "ff"))
SGPLAN6 = os.environ.get("FF_SGPLAN6", os.path.join(ROOT, "..", "planner", "bin", "sgplan6"))

PLAN = re.compile(r"^\s*(?:step\s+)?\d+:\s")
IPC = re.compile(r"^\s*\d+\.\d+:\s")


def classify(out):
    metric = None
    m = re.search(r"MetricValue\s+([0-9.]+)", out) or re.search(r"metric value\s+([0-9.]+)", out)
    if m:
        metric = float(m.group(1))
    if "found legal plan as follows" in out:
        return "solved", sum(1 for l in out.splitlines() if PLAN.match(l)), metric
    if "Solution found" in out or any(IPC.match(l) for l in out.splitlines()):
        n = sum(1 for l in out.splitlines() if IPC.match(l))
        mm = re.search(r"NrActions (\d+)", out)
        return "solved", (int(mm.group(1)) if mm else n), metric
    if "simplified to TRUE" in out:
        return "trivial", 0, metric
    if "unsolvable" in out or "search space empty" in out or "simplified to FALSE" in out:
        return "unsolv", 0, metric
    return "error", 0, metric


def run(cmd, t=TIMEOUT):
    start = time.perf_counter()
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=t, cwd=ROOT)
        return r.stdout + r.stderr, (time.perf_counter() - start) * 1000
    except subprocess.TimeoutExpired:
        return "__TIMEOUT__", t * 1000.0


def problems():
    out = []
    for cat in CATS:
        cdir = os.path.join(CORPUS, cat)
        if not os.path.isdir(cdir):
            continue
        for dom in sorted(os.listdir(cdir)):
            d = os.path.join(cdir, dom, "domain.pddl")
            if not os.path.isfile(d):
                continue
            for f in sorted(x for x in os.listdir(os.path.join(cdir, dom)) if re.match(r"p\d+\.pddl", x)):
                out.append((cat, dom, f[:-5], d, os.path.join(cdir, dom, f)))
    return out


def main():
    subprocess.run(["cargo", "build", "--release", "-q"], cwd=ROOT, check=True)
    probs = problems()
    have_mff = (not NO_ROSETTA) and os.path.exists(METRICFF)
    have_sgp = (not NO_DOCKER) and shutil.which("docker") and os.path.exists(SGPLAN6)
    print(f"corpus {len(probs)} problems | ferroplan{' +metric-ff' if have_mff else ''}"
          f"{' +sgplan6' if have_sgp else ''} | timeout {TIMEOUT}s\n", flush=True)

    speed_ratios, wins, ties, losses = [], 0, 0, 0
    print(f"{'problem':28} {'ferroplan':>22} {'metric-ff':>14} {'sgplan6':>22}")
    for cat, dom, pn, d, p in probs:
        out, fms = run([FF, "-o", d, "-f", p, "--json", "--threads", "0"])
        try:
            s = json.loads(out) if out != "__TIMEOUT__" else {}
        except Exception:
            s = {}
        fsolved = s.get("solved", False)
        fplan = s.get("plan") or {}
        flen, fmetric = fplan.get("length", "-"), fplan.get("metric")
        fcell = (f"{'t/o' if out=='__TIMEOUT__' else ('ok' if fsolved else 'no')} "
                 f"l={flen} {fms:.0f}ms") + (f" m={fmetric:g}" if fmetric is not None else "")

        mcell = ""
        if have_mff:
            mo, mms = run(["arch", "-x86_64", METRICFF, "-o", d, "-f", p])
            mst, mlen, _ = classify(mo) if mo != "__TIMEOUT__" else ("t/o", 0, None)
            mcell = f"{mst[:3]} {mms:.0f}ms"
            if fsolved and mst == "solved" and fms > 0 and mms > 0:
                speed_ratios.append(mms / fms)

        scell = ""
        if have_sgp:
            so, _ = run(["docker", "run", "--rm", "--platform", "linux/386", "-v", ROOT + "/..:/w",
                         "debian:bullseye-slim", "sh", "-c",
                         f"cd /tmp && timeout {TIMEOUT} {SGPLAN6.replace(ROOT+'/..','/w')} "
                         f"-o {d.replace(ROOT+'/..','/w')} -f {p.replace(ROOT+'/..','/w')} 2>&1"])
            sst, slen, smetric = classify(so) if so != "__TIMEOUT__" else ("t/o", 0, None)
            scell = f"{sst[:3]} l={slen}" + (f" m={smetric:g}" if smetric is not None else "")
            # IPC-5 metric scoreboard (lower is better)
            if cat == "pref" and fmetric is not None and smetric is not None:
                if fmetric < smetric - 1e-6:
                    wins += 1
                elif abs(fmetric - smetric) <= 1e-6:
                    ties += 1
                else:
                    losses += 1
        print(f"{cat}/{dom}/{pn:5} {fcell:>22} {mcell:>14} {scell:>22}", flush=True)

    print("\n=== summary ===")
    if speed_ratios:
        g = math.exp(sum(math.log(r) for r in speed_ratios) / len(speed_ratios))
        print(f"speed: ferroplan is {g:.2f}x metric-ff (geomean over {len(speed_ratios)} both-solved)")
    if wins + ties + losses:
        print(f"IPC-5 metric vs sgplan6 (lower=better): {wins} wins, {ties} ties, {losses} losses")


if __name__ == "__main__":
    main()
