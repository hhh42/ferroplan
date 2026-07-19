#!/usr/bin/env python3
"""Full-corpus IPC-2008/IPC-2011 runs over a potassco/pddl-instances checkout.

Unlike run.py (which walks the small vendored regression subset), this runs
whole competition variants — coverage, plan cost, wall-clock, and (with VAL)
external validation — and writes a per-domain summary to
benchmarks/ipc67-results.md (or --out).

Corpus: benchmarks/.ipc-corpus (from benchmarks/get-ipc.sh) or
$FERROPLAN_IPC_CORPUS. VAL: $FERROPLAN_VAL or `Validate` on PATH
(benchmarks/get-val.sh).

Usage:
  python3 benchmarks/ipc67.py [--timeout N] [--track seq-sat|net-benefit|tempo-sat]
                              [--only <variant-substring>] [--max-instances N]
                              [--jobs N] [--mode M] [--out FILE] [--raw FILE]

--jobs N runs N instances concurrently (each ff invocation stays --threads 1);
per-instance wall-clock gets noisier under load, coverage-at-timeout does not
as long as jobs < cores. --mode passes `ff --mode M` (e.g. portfolio) so two
runs with different --out/--raw can be diffed per instance via the raw JSONL.

Scoring note: IPC quality score (ref-cost / your-cost, capped at 1) needs
per-instance reference costs, which this corpus does not carry; we report raw
summed cost instead. Do not present summed cost as an IPC quality score.
"""
import json, os, re, resource, shutil, subprocess, sys, tempfile, time
from concurrent.futures import ThreadPoolExecutor

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FF = os.path.join(ROOT, "target", "release", "ff")


def arg(name, default):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


TIMEOUT = int(arg("--timeout", "60"))
TRACK = arg("--track", "seq-sat")
ONLY = arg("--only", None)
MAXI = int(arg("--max-instances", "0"))  # 0 = all
JOBS = int(arg("--jobs", "1"))
MODE = arg("--mode", None)  # ff --mode passthrough (None = ff's default, auto)
# Per-job address-space cap in GiB (RLIMIT_AS on each ff): a memory spike
# kills ITS job with an allocation failure instead of inviting the OOM
# killer to execute sibling jobs (elevator-11 grounding transients did
# exactly that). Default: physical RAM / jobs, floored at 2 GiB; 0 = off.
_phys_gb = os.sysconf("SC_PAGE_SIZE") * os.sysconf("SC_PHYS_PAGES") / (1 << 30)
MEMGB = float(arg("--mem-gb", str(max(2, int(_phys_gb / max(JOBS, 1))))))
OUT = arg("--out", os.path.join(ROOT, "benchmarks", "ipc67-results.md"))
RAW = arg("--raw", None)  # per-instance JSONL (default: OUT with .jsonl)
if RAW is None:
    RAW = os.path.splitext(OUT)[0] + ".jsonl"

TRACK_PATTERNS = {
    "seq-sat": r"sequential-satisficing",
    "net-benefit": r"net-benefit",
    "seq-opt": r"sequential-optimal",
    "tempo-sat": r"temporal-satisficing",
}


def corpus_dir():
    c = os.environ.get("FERROPLAN_IPC_CORPUS") or os.path.join(
        ROOT, "benchmarks", ".ipc-corpus")
    if not os.path.isdir(c):
        sys.exit(f"corpus not found at {c}; run benchmarks/get-ipc.sh "
                 "or set FERROPLAN_IPC_CORPUS")
    return c


def find_val():
    p = os.environ.get("FERROPLAN_VAL")
    if p and os.path.isfile(p):
        return p
    # the conventional get-val.sh build location
    local = os.path.join(ROOT, "benchmarks", ".val", "VAL", "build", "bin", "Validate")
    if os.path.isfile(local):
        return local
    return shutil.which("Validate")


def variants(corpus):
    pat = re.compile(TRACK_PATTERNS[TRACK])
    out = []
    for ipc in ("ipc-2008", "ipc-2011"):
        droot = os.path.join(corpus, ipc, "domains")
        if not os.path.isdir(droot):
            continue
        for v in sorted(os.listdir(droot)):
            if not pat.search(v):
                continue
            if ONLY and ONLY not in v:
                continue
            out.append((ipc, v, os.path.join(droot, v)))
    return out


def instances(vdir):
    idir = os.path.join(vdir, "instances")
    shared = os.path.join(vdir, "domain.pddl")
    out = []
    names = sorted(os.listdir(idir),
                   key=lambda n: int(re.search(r"\d+", n).group()))
    for f in names:
        n = int(re.search(r"\d+", f).group())
        d = shared if os.path.isfile(shared) else os.path.join(
            vdir, "domains", f"domain-{n}.pddl")
        if os.path.isfile(d):
            out.append((n, d, os.path.join(idir, f)))
    if MAXI:
        out = out[:MAXI]
    return out


def val_check(val, domain, problem, steps, temporal=False):
    """VAL a plan. Temporal steps render as `time: (action) [duration]` —
    the format `TimedPlan::to_ipc` emits and VAL parses natively; classical
    steps inside a temporal plan (duration null) drop the brackets. The
    tolerance is HALF ff's decision-epoch EPS (0.001): VAL groups happenings
    whose gap does not strictly exceed the tolerance, so validating at
    exactly EPS treats our e-separated pairs as simultaneous (boundary
    mutex false-positives); at EPS/2 every e gap clears cleanly while true
    coincidences still group."""
    with tempfile.NamedTemporaryFile("w", suffix=".plan", delete=False) as f:
        for s in steps:
            act = "(" + " ".join([s["action"]] + s.get("args", [])).lower() + ")"
            if temporal:
                line = f"{s['time']}: {act}"
                if s.get("duration") is not None:
                    line += f" [{s['duration']}]"
                f.write(line + "\n")
            else:
                f.write(act + "\n")
        path = f.name
    cmd = [val, "-t", "0.0005", domain, problem, path] if temporal else [
        val, domain, problem, path]
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
        return r.returncode == 0 and "Plan valid" in r.stdout
    except Exception:
        return False
    finally:
        os.unlink(path)


def run_instance(val, n, d, p):
    """One ff invocation → per-instance record dict."""
    cmd = [FF, "-o", d, "-f", p, "--json", "--threads", "1"]
    if MODE:
        cmd += ["--mode", MODE]

    def _limit():
        if MEMGB > 0:
            cap = int(MEMGB * (1 << 30))
            resource.setrlimit(resource.RLIMIT_AS, (cap, cap))
    rec = {"instance": n, "solved": False, "time": None, "metric": None,
           "length": None, "val": None, "notes": None}
    t = time.perf_counter()
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=TIMEOUT,
                           preexec_fn=_limit)
        el = time.perf_counter() - t
        if r.returncode != 0 and "allocation" in (r.stderr or ""):
            rec["notes"] = "mem-cap"
        s = json.loads(r.stdout) if r.stdout.strip() else {}
        plan = s.get("plan") or {}
        if s.get("solved"):
            rec.update(solved=True, time=round(el, 2),
                       metric=plan.get("metric"), length=plan.get("length"),
                       notes=s.get("notes"))
            if val:
                rec["val"] = val_check(val, d, p, plan.get("steps", []),
                                       temporal=plan.get("makespan") is not None)
    except subprocess.TimeoutExpired:
        rec["time"] = TIMEOUT
    except Exception:
        pass
    return rec


def main():
    corpus = corpus_dir()
    val = find_val()
    print(f"corpus: {corpus}\nVAL: {val or 'not found (external validation skipped)'}\n"
          f"timeout {TIMEOUT}s, jobs {JOBS}, mode {MODE or 'auto'}", flush=True)
    subprocess.run(["cargo", "build", "--release", "-q", "-p", "ferroplan-cli"],
                   cwd=ROOT, check=True)
    summary = []
    raw = open(RAW, "w")
    with ThreadPoolExecutor(max_workers=JOBS) as pool:
        for ipc, vname, vdir in variants(corpus):
            insts = instances(vdir)
            recs = list(pool.map(lambda a: run_instance(val, *a), insts))
            solved = sum(r["solved"] for r in recs)
            valok = sum(r["val"] is True for r in recs)
            valfail = sum(r["val"] is False for r in recs)
            cost_sum = sum((r["metric"] if r["metric"] is not None
                            else r["length"] or 0) for r in recs if r["solved"])
            t_sum = sum(r["time"] or 0 for r in recs if r["solved"])
            for r in recs:
                if r["val"] is False:
                    print(f"    VAL FAIL: {vname}/instance-{r['instance']}", flush=True)
                raw.write(json.dumps({"ipc": ipc, "variant": vname, **r}) + "\n")
            raw.flush()
            vtag = f"{valok}/{valok + valfail}" if val else "-"
            summary.append((ipc, vname, solved, len(insts), cost_sum, t_sum, vtag))
            print(f"{ipc}/{vname}: {solved}/{len(insts)} solved, "
                  f"cost {cost_sum:.0f}, {t_sum:.1f}s solve-time, val {vtag}", flush=True)
    raw.close()

    out = [f"# IPC-2008/2011 {TRACK} full-corpus results\n",
           f"timeout {TIMEOUT}s/instance, jobs {JOBS}, mode {MODE or 'auto'}."
           + (" Plans externally validated with VAL."
              if val else " VAL not available.") + "\n",
           "| variant | coverage | summed cost | solve time | val |",
           "|---|---|---|---|---|"]
    for ipc, v, s, n, c, t, vt in summary:
        out.append(f"| {ipc}/{v} | {s}/{n} | {c:.0f} | {t:.1f}s | {vt} |")
    total = sum(s for _, _, s, *_ in summary)
    n = sum(n for _, _, _, n, *_ in summary)
    out.append(f"\ntotal coverage: **{total}/{n}**")
    open(OUT, "w").write("\n".join(out) + "\n")
    print(f"\nwrote {OUT} (raw: {RAW}) — total coverage {total}/{n}")


if __name__ == "__main__":
    main()
