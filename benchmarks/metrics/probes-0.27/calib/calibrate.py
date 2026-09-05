#!/usr/bin/env python3
"""crucible R2 Phase 0 calibrations (docs/roadmap-0.27.md 0.b / 0.c).

Runs the 0.26 cut binary on instances chosen from the 0.26 sweep's database
(calib-manifest.json, written by the selection query) and measures each child
with os.wait4 -- the same rusage the R2 runner reads -- so rho = cpu / wall is
exactly the referee's input.

  A  rho_min      60 clean-window instances, solo, three wall buckets
  B  packing      40 PACK-class instances, solo x3 then 4-wide x3
  C  timeouts     20 prior timeouts, solo x1 then 2-wide x1

Every run records loadavg at start so a contended sitting is visible in its
own receipts rather than argued about afterwards.
"""
import json, os, re, subprocess, sys, time, statistics, glob, concurrent.futures as cf

ROOT = "/Users/harold/ferroplan"
FF = f"{ROOT}/target/release/ff"
CORPUS = f"{ROOT}/benchmarks/.ipc-corpus"
HERE = os.path.dirname(os.path.abspath(__file__))
MAN = json.load(open(sys.argv[1] if len(sys.argv) > 1 else f"{HERE}/calib-manifest.json"))
OUT = f"{HERE}/results.jsonl"
MEM_GB = "6"

def groups(name):
    return re.findall(r"\d+", name)

def resolve(r):
    vdir = f"{CORPUS}/{r['ipc']}/domains/{r['variant']}"
    label = str(r["label"])
    cand = f"{vdir}/instances/instance-{label}.pddl"
    if not os.path.exists(cand):
        want = groups(label)
        for f in glob.glob(f"{vdir}/instances/*.pddl"):
            if groups(os.path.basename(f)) == want:
                cand = f
                break
    dom = f"{vdir}/domain.pddl"
    if not os.path.exists(dom):
        dom = f"{vdir}/domains/domain-{groups(label)[0]}.pddl"
    assert os.path.exists(cand) and os.path.exists(dom), (r, cand, dom)
    return dom, cand

def env_for(budget):
    keep = {k: os.environ[k] for k in ("PATH", "HOME", "TMPDIR", "USER", "SHELL", "LANG", "TERM", "TZ") if k in os.environ}
    keep["FF_TIME_LIMIT"] = str(int(budget))
    keep["FF_MEM_BUDGET_GB"] = MEM_GB
    return keep

# Per-child rusage: wait4 on the exact pid, exactly what the R2 runner does.
def run_one_rusage(r, tag, budget=None):
    budget = budget or r["budget_secs"]
    dom, prob = resolve(r)
    argv = [FF, "-o", dom, "-f", prob, "--json", "--threads", "1"]
    if r["mode"] and r["mode"] != "auto":
        argv += ["--mode", r["mode"]]
    load = os.getloadavg()[0]
    t0 = time.monotonic()
    with open(os.devnull, "w") as devnull:
        p = subprocess.Popen(argv, env=env_for(budget), stdin=subprocess.DEVNULL,
                             stdout=subprocess.PIPE, stderr=devnull, start_new_session=True)
    # Drain stdout in a thread so a long plan cannot deadlock the pipe.
    import threading
    buf = []
    th = threading.Thread(target=lambda: buf.append(p.stdout.read()), daemon=True)
    th.start()
    deadline = t0 + budget + 2
    while True:
        pid, status, ru = os.wait4(p.pid, os.WNOHANG)
        if pid == p.pid:
            break
        if time.monotonic() > deadline:
            try:
                os.killpg(p.pid, 9)
            except ProcessLookupError:
                pass
            pid, status, ru = os.wait4(p.pid, 0)
            break
        time.sleep(0.05)
    wall = time.monotonic() - t0
    th.join(timeout=5)
    p.stdout.close()
    cpu = ru.ru_utime + ru.ru_stime
    out = (buf[0] if buf else b"").decode(errors="replace").strip()
    solved = False
    try:
        # ff --json pretty-prints one document; parse the whole stream.
        solved = bool(json.loads(out).get("solved"))
    except Exception:
        try:
            solved = bool(json.loads(out.splitlines()[-1]).get("solved"))
        except Exception:
            pass
    return {"tag": tag, "board": r["board"], "variant": r["variant"], "label": str(r["label"]),
            "budget": budget, "wall": round(wall, 3), "cpu": round(cpu, 3),
            "rho": round(cpu / wall, 3) if wall > 0 else None, "solved": solved,
            "load": round(load, 2), "prior_wall_ms": r["wall_ms"], "prior_solved": r["solved"],
            "at": time.time()}

def record(res):
    with open(OUT, "a") as f:
        f.write(json.dumps(res) + "\n")
    print(f"  {res['tag']:<10} {res['board']:<20} {res['variant']}/{res['label']:<8} "
          f"wall {res['wall']:7.2f} cpu {res['cpu']:7.2f} rho {res['rho']} solved {res['solved']} load {res['load']}", flush=True)

def part_a():
    print("== A: rho on 60 clean-window instances, solo", flush=True)
    for bucket, rows in MAN["rho"].items():
        for r in rows:
            record(run_one_rusage(r, f"A:{bucket}"))

def part_b():
    rows = MAN["pack"]
    print("== B: packing calibration -- solo x3", flush=True)
    for rep in range(3):
        for r in rows:
            record(run_one_rusage(r, f"B:solo{rep}"))
    print("== B: packing calibration -- 4-wide x3", flush=True)
    for rep in range(3):
        with cf.ThreadPoolExecutor(max_workers=4) as ex:
            for res in ex.map(lambda r: run_one_rusage(r, f"B:pack4-{rep}"), rows):
                record(res)

def part_d():
    rows = MAN["pack"]
    print("== D: packing calibration -- 2-wide x2 (the width the Python sweeps ran at)", flush=True)
    for rep in range(2):
        with cf.ThreadPoolExecutor(max_workers=2) as ex:
            for res in ex.map(lambda r: run_one_rusage(r, f"B:pack2-{rep}"), rows):
                record(res)

def part_c():
    rows = MAN["timeouts"]
    print("== C: prior timeouts -- solo", flush=True)
    for r in rows:
        record(run_one_rusage(r, "C:solo"))
    print("== C: prior timeouts -- 2-wide", flush=True)
    with cf.ThreadPoolExecutor(max_workers=2) as ex:
        for res in ex.map(lambda r: run_one_rusage(r, "C:pack2"), rows):
            record(res)

if __name__ == "__main__":
    parts = sys.argv[2] if len(sys.argv) > 2 else "ABC"
    print(f"calibration start {time.strftime('%F %T')} ff={subprocess.run([FF,'--version'],capture_output=True,text=True).stdout.strip()} parts={parts}", flush=True)
    if "A" in parts: part_a()
    if "B" in parts: part_b()
    if "C" in parts: part_c()
    if "D" in parts: part_d()
    print(f"CALIBRATION DONE {time.strftime('%F %T')}", flush=True)
