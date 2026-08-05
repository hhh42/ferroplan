#!/usr/bin/env python3
"""Price a shipped feature by running the SAME instances with its hatch on.

The re-baseline rule forbids comparing an Air board to a cloud board, so a
cycle's phase claims cannot be settled by "coverage went up since 0.19". What
CAN settle them is an Air-vs-Air differential: every 0.20 phase shipped a
discriminator hatch, so run the named witnesses twice on one box — once as
shipped, once with the hatch set — and report what the feature actually bought.

    python3 benchmarks/hatch-differential.py --spec NAME [--timeout 60] [--jobs 2]

Both arms run under identical conditions (same box, same budget, same job
count, interleaved) so the only difference is the env var. Instances are
addressed by (ipc, variant, instance) exactly as the boards record them.
"""
import json
import os
import re
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FF = os.path.join(ROOT, "target", "release", "ff")
CORPUS = os.path.join(ROOT, "benchmarks", ".ipc-corpus")


def arg(name, default=None):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


TIMEOUT = int(arg("--timeout", "60"))
# SERIAL by default, unlike the boards. A differential selects the HARDEST
# instances (the ones a feature is being credited with), so running them 2-up
# pairs two memory-hungry searches with each other — contention the board never
# had, where each hard instance sat among its ordinary neighbours. Measured:
# elevator-2011 i10 proves in ~22s solo on a quiet box (deterministic, 213,531
# expansions every run, 1.1x wall spread) and 27s on the board at --jobs 2, but
# blows past 60s when paired with another hard optimal instance. That artifact
# reported LM-cut as worth +0. What a differential must hold constant is the
# A-vs-B conditions, NOT the board's job count; serial gives the cleanest A/B.
JOBS = int(arg("--jobs", "1"))
REPEAT = int(arg("--repeat", "1"))

# Each spec: the hatch, the mode, and how to find its witnesses.
SPECS = {
    # Phase 2. Does h^max alone close what LM-cut closed? Instances are the
    # 13 rows whose PROVEN note names LM-cut as prover.
    "lmcut": {
        "hatch": "FF_NO_LMCUT",
        "mode": "optimal",
        "from_boards": ["ipc-opt-2008-11", "ipc2014-opt"],
        "prover": "LM-cut",
        "question": "does h^max alone still prove these?",
    },
    # Phase 2, the other rung: is the h^max SPRINT carrying instances LM-cut
    # would also get, or is it load-bearing?
    "hmax-sprint": {
        "hatch": "FF_NO_HMAX_SPRINT",
        "mode": "optimal",
        "from_boards": ["ipc-opt-2008-11", "ipc2014-opt"],
        "prover": "h^max",
        "limit": 30,
        "question": "can LM-cut alone prove what the sprint proved?",
    },
    # Phase 3. visit-all-2014 is the named witness: 20/20 as shipped.
    "novlight": {
        "hatch": "FF_NO_NOVLIGHT",
        "mode": None,
        "domain": ("ipc-2014", "visit-all-sequential-satisficing"),
        "question": "how much of visit-all is the light rung?",
    },
    # Phase 1. tpp-numeric is where the early-exit witness lived.
    "refill": {
        "hatch": "FF_NO_REFILL",
        "mode": None,
        "domain": ("ipc-2023n", "tpp-numeric-satisficing"),
        "question": "does the refill loop buy coverage, or only spend the wall?",
    },
}


def instances_from_boards(boards, prover, limit=None):
    out = []
    for b in boards:
        p = os.path.join(ROOT, "benchmarks", f"{b}.jsonl")
        if not os.path.exists(p):
            continue
        for line in open(p):
            r = json.loads(line)
            if not r["solved"]:
                continue
            n = r.get("notes") or []
            txt = " ".join(map(str, n)) if isinstance(n, list) else str(n)
            m = re.search(r"certified by A\* \+ admissible ([\w^-]+)", txt)
            if m and m.group(1) == prover:
                cost = re.search(r"cost (\d+)", txt)
                out.append({"ipc": r["ipc"], "variant": r["variant"],
                            "instance": r["instance"],
                            "cost": int(cost.group(1)) if cost else None})
    out.sort(key=lambda r: (r["ipc"], r["variant"], r["instance"]))
    return out[:limit] if limit else out


def instances_from_domain(ipc, variant):
    vdir = os.path.join(CORPUS, ipc, "domains", variant)
    idir = os.path.join(vdir, "instances")
    names = [f for f in os.listdir(idir) if re.search(r"\d+", f)]
    names.sort(key=lambda n: int(re.search(r"\d+", n).group()))
    return [{"ipc": ipc, "variant": variant,
             "instance": int(re.search(r"\d+", f).group()), "cost": None}
            for f in names]


def paths(rec):
    vdir = os.path.join(CORPUS, rec["ipc"], "domains", rec["variant"])
    shared = os.path.join(vdir, "domain.pddl")
    dom = shared if os.path.isfile(shared) else os.path.join(
        vdir, "domains", f"domain-{rec['instance']}.pddl")
    idir = os.path.join(vdir, "instances")
    for f in os.listdir(idir):
        m = re.search(r"\d+", f)
        if m and int(m.group()) == rec["instance"]:
            return dom, os.path.join(idir, f)
    return dom, None


def run_one(rec, mode, env_extra):
    dom, prob = paths(rec)
    if not prob or not os.path.isfile(dom):
        return {**rec, "solved": False, "note": "missing files", "time": None}
    cmd = [FF, "-o", dom, "-f", prob, "--json", "--threads", "1"]
    if mode:
        cmd += ["--mode", mode]
    env = dict(os.environ, FF_TIME_LIMIT=str(TIMEOUT), **env_extra)
    t = time.perf_counter()
    try:
        # EXACTLY the board's kill, not a grace period: ipc67.py uses
        # timeout=TIMEOUT, and FF_TIME_LIMIT is soft (checked between rungs),
        # so a +10s grace silently gave these instances 70s where the board
        # gave 60 — and near-wall proofs are precisely what is in question.
        r = subprocess.run(cmd, capture_output=True, text=True,
                           timeout=TIMEOUT, env=env)
        el = time.perf_counter() - t
        s = json.loads(r.stdout) if r.stdout.strip() else {}
        notes = s.get("notes") or []
        txt = " ".join(map(str, notes)) if isinstance(notes, list) else str(notes)
        cost = None
        m = re.search(r"cost (\d+)", txt)
        if m:
            cost = int(m.group(1))
        pm = re.search(r"admissible ([\w^-]+)", txt)
        return {**rec, "solved": bool(s.get("solved")), "time": round(el, 1),
                "got_cost": cost, "prover": pm.group(1) if pm else None}
    except subprocess.TimeoutExpired:
        return {**rec, "solved": False, "time": TIMEOUT, "got_cost": None,
                "prover": None}
    except Exception as e:
        return {**rec, "solved": False, "time": None, "note": str(e)[:60]}


def arm(recs, mode, env_extra, label):
    print(f"  running {label} ({len(recs)} instances, jobs={JOBS})...", flush=True)
    with ThreadPoolExecutor(max_workers=JOBS) as ex:
        return list(ex.map(lambda r: run_one(r, mode, env_extra), recs))


def main():
    name = arg("--spec")
    if name not in SPECS:
        sys.exit(f"--spec must be one of: {', '.join(SPECS)}")
    spec = SPECS[name]
    if "from_boards" in spec:
        recs = instances_from_boards(spec["from_boards"], spec["prover"],
                                     spec.get("limit"))
    else:
        recs = instances_from_domain(*spec["domain"])
    if not recs:
        sys.exit("no witness instances found")

    print(f"=== {name}: {spec['question']}")
    print(f"    hatch={spec['hatch']}  budget={TIMEOUT}s  witnesses={len(recs)}")
    # Repeat the shipped arm: an instance that proves in 55s of a 60s wall is
    # measuring load as much as search, so report how OFTEN it proves.
    runs = [arm(recs, spec["mode"], {}, f"as shipped (rep {i + 1}/{REPEAT})")
            for i in range(REPEAT)]
    hits = [sum(1 for r in rs if r["solved"]) for rs in runs]
    base = [max(rs, key=lambda r: r["solved"]) for rs in zip(*runs)]
    solved_n = [sum(1 for rs in run_set if rs["solved"])
                for run_set in zip(*runs)]
    var = arm(recs, spec["mode"], {spec["hatch"]: "1"}, f"{spec['hatch']}=1")
    if REPEAT > 1:
        print(f"\n  shipped arm across {REPEAT} repeats: {hits} solved "
              f"(union {sum(1 for r in base if r['solved'])})")

    print(f"\n  {'instance':<52} {'shipped':>16} {'hatched':>16}")
    lost = kept = gained = 0
    for b, v in zip(base, var):
        tag = f"{b['ipc']}/{b['variant']}"[:44] + f" i{b['instance']}"
        bs = (f"{b.get('got_cost')} ({b.get('prover') or '-'}) {b['time']}s"
              if b["solved"] else f"unsolved {b['time']}s")
        vs = (f"{v.get('got_cost')} ({v.get('prover') or '-'}) {v['time']}s"
              if v["solved"] else f"unsolved {v['time']}s")
        flag = ""
        if REPEAT > 1:
            k = solved_n[base.index(b)] if b in base else 0
            bs += f" [{k}/{REPEAT}]"
        if b["solved"] and not v["solved"]:
            lost += 1
            flag = "  <== hatch LOSES it"
        elif b["solved"] and v["solved"]:
            kept += 1
            if b.get("got_cost") != v.get("got_cost"):
                flag = "  <== COST MISMATCH"
        elif not b["solved"] and v["solved"]:
            gained += 1
            flag = "  <== hatch WINS it"
        print(f"  {tag:<52} {bs:>16} {vs:>16}{flag}")

    print(f"\n  shipped solved {sum(r['solved'] for r in base)}/{len(base)}  "
          f"hatched solved {sum(r['solved'] for r in var)}/{len(var)}")
    print(f"  the feature is worth: +{lost} instances "
          f"(kept-by-both {kept}, hatch-only {gained})")
    out = os.path.join(ROOT, "benchmarks", f"differential-{name}.json")
    json.dump({"spec": name, "hatch": spec["hatch"], "timeout": TIMEOUT,
               "jobs": JOBS, "shipped": base, "hatched": var,
               "feature_worth": lost, "hatch_only": gained},
              open(out, "w"), indent=1)
    print(f"  wrote {out}")


if __name__ == "__main__":
    main()
