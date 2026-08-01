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
--score-against PRIOR.jsonl computes the IPC formula against a PRIOR RUN's
per-instance costs (self-relative quality — regression tracking, not an
official IPC score; label it as such).
"""
import json, os, re, resource, shutil, subprocess, sys, tempfile, threading, time
from concurrent.futures import ThreadPoolExecutor

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FF = os.path.join(ROOT, "target", "release", "ff")


def arg(name, default):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


TIMEOUT = int(arg("--timeout", "60"))
TRACK = arg("--track", "seq-sat")
# ff-side thread count per instance (0.16: the IPC-7 multi-core track runs
# one instance with ALL cores — pair --threads N with --jobs 1 there).
THREADS = arg("--threads", "1")
LIST_ONLY = "--list" in sys.argv  # enumerate variants and exit (audit eyes)
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


def _rlimit_as_enforceable():
    """Can we actually lower RLIMIT_AS on this kernel?

    macOS reports RLIMIT_AS as INFINITY but rejects EVERY setrlimit on it
    with EINVAL (surfacing as ValueError). That is far worse than the cap
    silently not firing: raised inside `preexec_fn`, subprocess re-raises it
    as SubprocessError, and this runner's spawn-retry then books EVERY
    instance as `spawn-fail` after a 5 s breather — a full 12-board sweep
    burns hours to produce nothing but garbage rows. So probe once, honestly:
    lower the soft limit and put it back. Side-effect-free on both kernels.
    """
    if MEMGB <= 0:
        return False
    try:
        soft, hard = resource.getrlimit(resource.RLIMIT_AS)
        resource.setrlimit(resource.RLIMIT_AS, (int(MEMGB * (1 << 30)), hard))
        resource.setrlimit(resource.RLIMIT_AS, (soft, hard))
        return True
    except (ValueError, OSError):
        return False


RLIMIT_AS_OK = _rlimit_as_enforceable()
# Where RLIMIT_AS is unavailable, an RSS watchdog enforces the same budget by
# polling and killing (the rusage watchdog named in docs/migration-m5.md).
# RSS is what actually drives a box into swap, so on the watchdog path the
# mem-cap column measures resident bytes, not address space — a different
# instrument for the same column, and recorded as such wherever it is used.
MEMWATCH = bool(MEMGB > 0 and not RLIMIT_AS_OK)
# Self-relative quality: per-instance reference costs from a prior run's raw
# JSONL (see the scoring note above).
SCORE_AGAINST = arg("--score-against", None)


def load_reference(path):
    ref = {}
    with open(path) as f:
        for line in f:
            r = json.loads(line)
            cost = r.get("metric") if r.get("metric") is not None else r.get("length")
            if r.get("solved") and cost:
                ref[(r.get("variant") or "", r["instance"])] = cost
    return ref
OUT = arg("--out", os.path.join(ROOT, "benchmarks", "ipc67-results.md"))
RAW = arg("--raw", None)  # per-instance JSONL (default: OUT with .jsonl)
if RAW is None:
    RAW = os.path.splitext(OUT)[0] + ".jsonl"

TRACK_PATTERNS = {
    "seq-sat": r"sequential-satisficing",
    "net-benefit": r"net-benefit",
    "seq-opt": r"sequential-optimal",
    "tempo-sat": r"temporal-satisficing",
    # IPC-5 (2006) deterministic tracks — different naming scheme (0.16
    # standings audit). Preferences tracks stay on the curated vendored
    # boards (benchmarks/ipc5-*.md); these are the never-swept ones.
    "prop-2006": r"propositional(-strips)?$",
    "time-2006": r"(?<!metric)-time(-strips)?$",
    "metric-time-2006": r"metric-time(-strips)?$",
    "constraints-2006": r"constraints",
    # IPC-7 (2011) sequential multi-core — the data-parallel track.
    "seq-mco": r"sequential-multi-core",
    # 0.17 modern-corpus tracks (directory-scoped via TRACK_IPCS).
    "seq-sat-2014": r"sequential-satisficing",
    "seq-agile-2014": r"sequential-agile",
    "seq-mco-2014": r"sequential-multi-core",
    "seq-opt-2014": r"sequential-optimal",
    "tempo-sat-2014": r"temporal-satisficing",
    "sat-2018": r"sequential-satisficing",
    "agile-2023": r"-agile$",
    "numeric-2023": r"numeric-satisficing",
    # 0.20: the IPC-2026 numeric dataset (vendored by get-ipc.sh from the
    # competition's public repo after the track ran at ICAPS Dublin).
    "numeric-2026": r"numeric-2026",
}

# Which competition directories each track lives in.
TRACK_IPCS = {
    "prop-2006": ("ipc-2006",),
    "time-2006": ("ipc-2006",),
    "metric-time-2006": ("ipc-2006",),
    "constraints-2006": ("ipc-2006",),
    "seq-mco": ("ipc-2011",),
    # The modern corpora (0.17 frontier cycle; fetched/normalized by
    # get-ipc.sh). 2014 reuses the potassco variant naming, so the standing
    # seq-sat/tempo-sat patterns apply, scoped to its directory; 2018/2023
    # use the normalized names get-ipc.sh writes.
    "seq-sat-2014": ("ipc-2014",),
    "seq-agile-2014": ("ipc-2014",),
    "seq-mco-2014": ("ipc-2014",),
    # 0.19: the optimal-track entries (pair with `--mode optimal`; the
    # engine certifies or stays silent, so coverage = proof rate).
    "seq-opt-2014": ("ipc-2014",),
    "tempo-sat-2014": ("ipc-2014",),
    "sat-2018": ("ipc-2018",),
    "agile-2023": ("ipc-2023",),
    "numeric-2023": ("ipc-2023n",),
    "numeric-2026": ("ipc-2026n",),
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
    for ipc in TRACK_IPCS.get(TRACK, ("ipc-2008", "ipc-2011")):
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
    # A name with no digits cannot be addressed by instance number at all, so
    # it is skipped LOUDLY rather than crashing the sweep (a bad normalization
    # upstream once took out a whole board mid-run) — and never silently, or a
    # missing instance reads as a smaller corpus instead of a corpus bug.
    named, skipped = [], []
    for f in os.listdir(idir):
        m = re.search(r"\d+", f)
        (named if m else skipped).append(f)
    if skipped:
        print(f"WARN {vdir}: skipping un-numbered instance file(s): "
              f"{', '.join(sorted(skipped))}", file=sys.stderr)
    names = sorted(named, key=lambda n: int(re.search(r"\d+", n).group()))
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
        if "Parser failed" in (r.stdout or "") + (r.stderr or ""):
            # VAL could not PARSE the domain/problem (independent of the
            # plan — the 0.20 attribution: drone-numeric fails on any
            # problem with two objects of the location type, before a
            # plan is ever judged). Validation is UNAVAILABLE, which is
            # not the same verdict as a rejected plan: None, not False.
            return None
        return r.returncode == 0 and "Plan valid" in r.stdout
    except Exception:
        return False
    finally:
        os.unlink(path)


def _rss_bytes(pid):
    """Resident bytes for `pid`, or 0 if it has already gone."""
    try:
        out = subprocess.run(["ps", "-o", "rss=", "-p", str(pid)],
                             capture_output=True, text=True, timeout=5).stdout.strip()
        return int(out) * 1024 if out else 0   # ps reports KiB
    except (ValueError, OSError, subprocess.SubprocessError):
        return 0


def _run_capped(cmd, timeout, env, preexec):
    """subprocess.run, plus an RSS watchdog when RLIMIT_AS is unavailable.

    Returns (CompletedProcess, mem_exceeded). The watchdog kills the child the
    moment it crosses MEMGB so a runaway grounding cannot drag its sibling job
    (and the box) into swap for the rest of a multi-hour sweep.
    """
    if not MEMWATCH:
        return subprocess.run(cmd, capture_output=True, text=True,
                              timeout=timeout, preexec_fn=preexec, env=env), False
    cap = int(MEMGB * (1 << 30))
    hit = threading.Event()
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                            text=True, preexec_fn=preexec, env=env)

    def watch():
        # 0.25s: unlike RLIMIT_AS, a poll can only catch a balloon it sees, and
        # grounding transients are the fast ones (the elevator-11 spike above).
        # Four `ps` calls/sec/job is nothing against a multi-hour sweep; the
        # residual risk is whatever a job can allocate inside one interval, so
        # leave real headroom between (jobs x --mem-gb) and physical RAM.
        while proc.poll() is None:
            if _rss_bytes(proc.pid) > cap:
                hit.set()
                try:
                    proc.kill()
                except OSError:
                    pass
                return
            time.sleep(0.25)

    threading.Thread(target=watch, daemon=True).start()
    try:
        out, err = proc.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.communicate()
        raise
    return (subprocess.CompletedProcess(cmd, proc.returncode, out, err),
            hit.is_set())


def run_instance(val, n, d, p):
    """One ff invocation → per-instance record dict."""
    cmd = [FF, "-o", d, "-f", p, "--json", "--threads", THREADS]
    if MODE:
        cmd += ["--mode", MODE]
    # Budget-aware ladder (0.18): tell the engine its real wall budget so
    # bounded rungs stop starving the complete fallback near the edge.
    env = dict(os.environ, FF_TIME_LIMIT=str(TIMEOUT))

    # Only install a preexec_fn where the cap actually takes: an unusable
    # setrlimit raises INSIDE the fork hook, and every row becomes spawn-fail.
    # (preexec_fn is also documented-unsafe with threads, and this runner is a
    # thread pool — so skipping it on the watchdog path is a bonus, not a loss.)
    _limit = None
    if RLIMIT_AS_OK:
        def _limit():
            cap = int(MEMGB * (1 << 30))
            resource.setrlimit(resource.RLIMIT_AS, (cap, cap))
    rec = {"instance": n, "solved": False, "time": None, "metric": None,
           "length": None, "val": None, "notes": None}
    t = time.perf_counter()
    try:
        # One retry after a breather on spawn failure: a memory-bloated
        # PREDECESSOR can make this instance's fork() fail (the 0.16
        # seq-mco t4 sweep lost floor-tile i7-i12 to exactly this — a
        # consecutive spawn-fail cluster while the system recovered, logged
        # as engine rejects). The retry runs on a recovered system; if it
        # fails again the row is honestly marked spawn-fail, not engine.
        for attempt in (0, 1):
            try:
                r, mem_hit = _run_capped(cmd, TIMEOUT, env, _limit)
                break
            except (OSError, subprocess.SubprocessError) as e:
                if isinstance(e, subprocess.TimeoutExpired) or attempt == 1:
                    raise
                time.sleep(5)
        el = time.perf_counter() - t
        # Honest clocks (0.20 Phase 1): elapsed wall is recorded for
        # UNSOLVED rows too. Before this, a graceful engine exit at the
        # FF_TIME_LIMIT wall left time=None and the standings classed it
        # as an engine-reject — maintenance-2014's "8 rejects" were
        # ordinary timeouts wearing that costume.
        rec["time"] = round(el, 2)
        # Two instruments, one verdict: RLIMIT_AS makes the child fail its own
        # allocation; the RSS watchdog SIGKILLs it (returncode -9, no stderr).
        # Both are mem-cap, and the watchdog's verdict must be read before the
        # generic nonzero-exit branch below or it books as engine-exit--9.
        if mem_hit or (r.returncode != 0 and "allocation" in (r.stderr or "")):
            rec["notes"] = "mem-cap"
        s = json.loads(r.stdout) if r.stdout.strip() else {}
        plan = s.get("plan") or {}
        if not s and r.returncode != 0 and rec["notes"] is None:
            # No JSON came back and the exit was nonzero: a real engine
            # error/reject (parse failure, panic), distinct from a clean
            # "searched and found nothing" JSON verdict.
            rec["notes"] = f"engine-exit-{r.returncode}"
        if s.get("solved"):
            rec.update(solved=True,
                       metric=plan.get("metric"), length=plan.get("length"),
                       notes=s.get("notes"))
            if val:
                rec["val"] = val_check(val, d, p, plan.get("steps", []),
                                       temporal=plan.get("makespan") is not None)
        elif s.get("notes") and rec["notes"] is None:
            # Unsolved WITH a mechanism (e.g. "unsolvable at grounding:
            # ..." from the 0.19 named verdicts) — keep it for the
            # standings' reject-vs-search attribution.
            rec["notes"] = s.get("notes")
    except subprocess.TimeoutExpired:
        rec["time"] = TIMEOUT
    except (OSError, subprocess.SubprocessError):
        rec["notes"] = "spawn-fail"
    except Exception:
        pass
    return rec


def main():
    corpus = corpus_dir()
    if LIST_ONLY:
        for ipc, vname, vdir in variants(corpus):
            print(f"{ipc}  {vname}  ({len(instances(vdir))} instances)")
        return
    val = find_val()
    reference = load_reference(SCORE_AGAINST) if SCORE_AGAINST else None
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
            qual = None
            if reference is not None:
                qual = 0.0
                for r in recs:
                    cost = r["metric"] if r["metric"] is not None else r["length"]
                    rc = reference.get((vname, r["instance"]))
                    if r["solved"] and cost and rc:
                        qual += min(1.0, rc / cost)
                    elif r["solved"] and rc is None:
                        qual += 1.0  # solved something the reference never did
            summary.append((ipc, vname, solved, len(insts), cost_sum, t_sum, vtag, qual))
            qtag = f", quality {qual:.2f}" if qual is not None else ""
            print(f"{ipc}/{vname}: {solved}/{len(insts)} solved, "
                  f"cost {cost_sum:.0f}, {t_sum:.1f}s solve-time, val {vtag}{qtag}", flush=True)
    raw.close()

    out = [f"# IPC-2008/2011 {TRACK} full-corpus results\n",
           f"timeout {TIMEOUT}s/instance, jobs {JOBS}, mode {MODE or 'auto'}."
           + (" Plans externally validated with VAL."
              if val else " VAL not available.") + "\n",
           "| variant | coverage | summed cost | solve time | val |"
           + (" quality |" if reference is not None else ""),
           "|---|---|---|---|---|" + ("---|" if reference is not None else "")]
    for ipc, v, s, n, c, t, vt, q in summary:
        row = f"| {ipc}/{v} | {s}/{n} | {c:.0f} | {t:.1f}s | {vt} |"
        if reference is not None:
            row += f" {q:.2f} |"
        out.append(row)
    total = sum(s for _, _, s, *_ in summary)
    n = sum(n for _, _, _, n, *_ in summary)
    out.append(f"\ntotal coverage: **{total}/{n}**")
    if reference is not None:
        qt = sum(q for *_, q in summary if q is not None)
        out.append(f"\nself-relative quality vs `{SCORE_AGAINST}`: **{qt:.2f}** "
                   "(IPC formula against our own prior best — regression "
                   "tracking, NOT an official IPC score)")
    open(OUT, "w").write("\n".join(out) + "\n")
    print(f"\nwrote {OUT} (raw: {RAW}) — total coverage {total}/{n}")


if __name__ == "__main__":
    main()
