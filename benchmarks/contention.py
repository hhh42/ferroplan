#!/usr/bin/env python3
"""Record what ELSE the machine was doing while a board was measured.

This box is not a benchmark rig. It is somebody's laptop: a browser, a Docker
VM running CI, Time Machine, a dev thread compiling in another worktree. A
board measured against that competition is not a slow board, it is a WRONG
board — and the failure is asymmetric, because contention only ever depresses
coverage. It manufactures REGRESSIONS and hides GAINS, which is the expensive
direction to be wrong in when the output is a release record.

The first version of this guard sampled total CPU idle and nothing else. That
was not enough, three times over:
  - it could not name the competitor, so every diagnosis needed a manual `ps`;
  - it never saw a 28 GB `rm` (disk I/O does not move idle% much);
  - it had no memory-pressure or thermal signal at all, and this chassis is
    fanless.

So: sample broadly, attribute the load to processes by name, and write the
whole thing next to the board as JSON. A board should carry its own conditions
the way a plan carries its own certificate.

    python3 benchmarks/contention.py --out FILE [--interval 20] [--self ff]

Runs until killed; writes/updates the JSON summary continuously so an
interrupted board still leaves a readable record.
"""
import json
import os
import re
import signal
import statistics
import subprocess
import sys
import time

TOP_N = 4          # how many competing processes to name
SELF_HINTS = ("target/release/ff", "ipc67.py", "cut21-sweeps", "caffeinate",
              "contention.py", "top", "ps")


def arg(name, default=None):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


def cpu_idle():
    """Percent idle. `top -l 2` because the first sample is since-boot."""
    try:
        out = subprocess.run(["top", "-l", "2", "-n", "0", "-s", "1"],
                             capture_output=True, text=True, timeout=20).stdout
    except (OSError, subprocess.SubprocessError):
        return None
    line = [l for l in out.splitlines() if l.startswith("CPU usage")]
    if not line:
        return None
    m = re.search(r"([\d.]+)%\s+idle", line[-1])
    return float(m.group(1)) if m else None


def loadavg():
    try:
        return os.getloadavg()[0]
    except OSError:
        return None


def competitors():
    """Top non-sweep processes by CPU, as {name: pcpu}. Names the competition
    so a degraded board says WHO, not just THAT."""
    try:
        out = subprocess.run(["ps", "-Ao", "pcpu,comm", "-r"],
                             capture_output=True, text=True, timeout=20).stdout
    except (OSError, subprocess.SubprocessError):
        return {}
    found = {}
    for line in out.splitlines()[1:14]:
        parts = line.strip().split(None, 1)
        if len(parts) != 2:
            continue
        try:
            pc = float(parts[0])
        except ValueError:
            continue
        cmd = parts[1]
        if pc < 5.0 or any(h in cmd for h in SELF_HINTS):
            continue
        # Collapse "…/Brave Browser Helper (Renderer).app/…/Brave Browser
        # Helper (Renderer)" to something readable, and SUM per name so three
        # renderer processes read as one 300% competitor rather than three.
        name = os.path.basename(cmd)
        name = re.sub(r"\.(app|xpc)$", "", name)[:44]
        found[name] = found.get(name, 0.0) + pc
        if len(found) >= TOP_N * 3:
            break
    return dict(sorted(found.items(), key=lambda kv: -kv[1])[:TOP_N])


def mem_pressure():
    """Swap used, MiB. A swapping box slows search while looking CPU-idle."""
    try:
        out = subprocess.run(["sysctl", "-n", "vm.swapusage"],
                             capture_output=True, text=True, timeout=10).stdout
        m = re.search(r"used\s*=\s*([\d.]+)M", out)
        return float(m.group(1)) if m else None
    except (OSError, subprocess.SubprocessError):
        return None


def thermal():
    """Non-zero means the kernel reported a thermal/perf warning — on a
    fanless chassis a long sweep is exactly when that shows up."""
    try:
        out = subprocess.run(["pmset", "-g", "therm"], capture_output=True,
                             text=True, timeout=10).stdout
        m = re.search(r"CPU_Speed_Limit\s*=\s*(\d+)", out)
        return int(m.group(1)) if m else None
    except (OSError, subprocess.SubprocessError):
        return None


def summarize(idles, loads, comp_totals, swaps, therms, started,
              timeline=None, interval=None):
    def pct(v, p):
        if not v:
            return None
        s = sorted(v)
        return round(s[min(len(s) - 1, int(len(s) * p))], 1)
    med = pct(idles, 0.5)
    comp_total = (round(sum(comp_totals.values()) / max(len(idles), 1), 1)
                  if idles else None)
    return {
        "started": started,
        "ended": time.strftime("%Y-%m-%d %H:%M:%S"),
        "samples": len(idles),
        "idle_pct": {"median": med, "p25": pct(idles, 0.25),
                     "min": round(min(idles), 1) if idles else None},
        "loadavg": {"median": round(statistics.median(loads), 2) if loads else None,
                    "max": round(max(loads), 2) if loads else None},
        # Mean CPU% each competitor held ACROSS the board, so a process that
        # spiked once does not read the same as one that ran the whole time.
        "competitors_mean_pcpu": {k: round(v / max(len(idles), 1), 1)
                                  for k, v in sorted(comp_totals.items(),
                                                     key=lambda kv: -kv[1])[:TOP_N]},
        "competitors_total_pcpu": comp_total,
        "swap_mb": {"max": round(max(swaps), 1) if swaps else None},
        "cpu_speed_limit": {"min": min(therms) if therms else None},
        # 0.24: verdict moved OFF raw idle_pct onto named-competitor load.
        # idle_pct is whole-machine and includes the board's OWN threads —
        # a --threads 4/8 mco board burns 40-80% of this 10-core box by
        # design, so a fixed idle floor (was >=65%) can never pass those
        # boards even in an empty room (measured: mco-t8 read 38-40% idle
        # with a combined 4-5% of real competing load). competitors_total
        # already excludes the sweep's own process (SELF_HINTS), so it is
        # the actual "who else is on this box" signal the module's own
        # docstring says was the point. 25% clears every historical clean
        # run on record (15-24% typical with Brave/Spotlight/etc. open) and
        # still catches real contention (Brave+WindowServer 36%+, a pegged
        # renderer at 100%, Spotlight mid-index at 40%).
        "verdict": ("clean" if comp_total is not None and comp_total < 25.0
                    else "DEGRADED" if med is not None else "unknown"),
        # The per-sample timeline (PER-INSTANCE-RETRY.md step 1): every
        # sample as [epoch_ts, idle_pct, competitors_total_pcpu], so a
        # later pass can intersect an instance's wall-clock window
        # against the actual contention windows instead of throwing the
        # whole board away over one bad stretch. ~360 samples on a
        # 2-hour board at the default 20 s interval — trivial size. The
        # rollup above stays the whole-board verdict; the timeline is
        # the fine print a resume reads.
        "interval": interval,
        "timeline": timeline or [],
    }


def main():
    out = arg("--out")
    if not out:
        sys.exit("--out FILE is required")
    interval = float(arg("--interval", "20"))
    started = time.strftime("%Y-%m-%d %H:%M:%S")
    idles, loads, swaps, therms = [], [], [], []
    comp_totals = {}
    timeline = []
    stop = {"now": False}

    def bye(*_):
        stop["now"] = True
    signal.signal(signal.SIGTERM, bye)
    signal.signal(signal.SIGINT, bye)

    while not stop["now"]:
        i = cpu_idle()
        if i is not None:
            idles.append(i)
        l = loadavg()
        if l is not None:
            loads.append(l)
        s = mem_pressure()
        if s is not None:
            swaps.append(s)
        t = thermal()
        if t is not None:
            therms.append(t)
        comp_now = competitors()
        for k, v in comp_now.items():
            comp_totals[k] = comp_totals.get(k, 0.0) + v
        timeline.append([
            round(time.time(), 1),
            round(i, 1) if i is not None else None,
            round(sum(comp_now.values()), 1),
        ])
        # Rewrite every sample so an interrupted board still has its record.
        if idles:
            with open(out, "w") as f:
                json.dump(summarize(idles, loads, comp_totals, swaps, therms,
                                    started, timeline, interval), f, indent=1)
                f.write("\n")
        for _ in range(int(max(interval, 1))):
            if stop["now"]:
                break
            time.sleep(1)


if __name__ == "__main__":
    main()
