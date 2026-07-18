#!/usr/bin/env python3
"""Full-corpus IPC-2008/IPC-2011 runs over a potassco/pddl-instances checkout.

Unlike run.py (which walks the small vendored regression subset), this runs
whole competition variants — coverage, plan cost, wall-clock, and (with VAL)
external validation — and writes a per-domain summary to
benchmarks/ipc67-results.md.

Corpus: benchmarks/.ipc-corpus (from benchmarks/get-ipc.sh) or
$FERROPLAN_IPC_CORPUS. VAL: $FERROPLAN_VAL or `Validate` on PATH
(benchmarks/get-val.sh).

Usage:
  python3 benchmarks/ipc67.py [--timeout N] [--track seq-sat|net-benefit]
                              [--only <variant-substring>] [--max-instances N]

Scoring note: IPC quality score (ref-cost / your-cost, capped at 1) needs
per-instance reference costs, which this corpus does not carry; we report raw
summed cost instead. Do not present summed cost as an IPC quality score.
"""
import json, os, re, shutil, subprocess, sys, tempfile, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FF = os.path.join(ROOT, "target", "release", "ff")


def arg(name, default):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


TIMEOUT = int(arg("--timeout", "60"))
TRACK = arg("--track", "seq-sat")
ONLY = arg("--only", None)
MAXI = int(arg("--max-instances", "0"))  # 0 = all

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


def val_check(val, domain, problem, steps):
    with tempfile.NamedTemporaryFile("w", suffix=".plan", delete=False) as f:
        for s in steps:
            f.write("(" + " ".join([s["action"]] + s.get("args", [])).lower() + ")\n")
        path = f.name
    try:
        r = subprocess.run([val, domain, problem, path],
                           capture_output=True, text=True, timeout=120)
        return r.returncode == 0 and "Plan valid" in r.stdout
    except Exception:
        return False
    finally:
        os.unlink(path)


def main():
    corpus = corpus_dir()
    val = find_val()
    print(f"corpus: {corpus}\nVAL: {val or 'not found (external validation skipped)'}",
          flush=True)
    subprocess.run(["cargo", "build", "--release", "-q", "-p", "ferroplan-cli"],
                   cwd=ROOT, check=True)
    summary = []
    for ipc, vname, vdir in variants(corpus):
        insts = instances(vdir)
        solved, valok, valfail, cost_sum, t_sum = 0, 0, 0, 0.0, 0.0
        for n, d, p in insts:
            t = time.perf_counter()
            try:
                r = subprocess.run([FF, "-o", d, "-f", p, "--json", "--threads", "1"],
                                   capture_output=True, text=True, timeout=TIMEOUT)
                el = time.perf_counter() - t
                s = json.loads(r.stdout) if r.stdout.strip() else {}
                plan = s.get("plan") or {}
                if s.get("solved"):
                    solved += 1
                    t_sum += el
                    m = plan.get("metric")
                    cost_sum += m if m is not None else plan.get("length", 0)
                    if val and plan.get("makespan") is None:
                        if val_check(val, d, p, plan.get("steps", [])):
                            valok += 1
                        else:
                            valfail += 1
                            print(f"    VAL FAIL: {vname}/instance-{n}", flush=True)
            except subprocess.TimeoutExpired:
                pass
            except Exception:
                pass
        vtag = f"{valok}/{valok + valfail}" if val else "-"
        summary.append((ipc, vname, solved, len(insts), cost_sum, t_sum, vtag))
        print(f"{ipc}/{vname}: {solved}/{len(insts)} solved, "
              f"cost {cost_sum:.0f}, {t_sum:.1f}s solve-time, val {vtag}", flush=True)

    out = [f"# IPC-2008/2011 {TRACK} full-corpus results\n",
           f"timeout {TIMEOUT}s/instance."
           + (" Plans externally validated with VAL."
              if val else " VAL not available.") + "\n",
           "| variant | coverage | summed cost | solve time | val |",
           "|---|---|---|---|---|"]
    for ipc, v, s, n, c, t, vt in summary:
        out.append(f"| {ipc}/{v} | {s}/{n} | {c:.0f} | {t:.1f}s | {vt} |")
    dest = os.path.join(ROOT, "benchmarks", "ipc67-results.md")
    open(dest, "w").write("\n".join(out) + "\n")
    total = sum(s for _, _, s, *_ in summary)
    n = sum(n for _, _, _, n, *_ in summary)
    print(f"\nwrote {dest} — total coverage {total}/{n}")


if __name__ == "__main__":
    main()
