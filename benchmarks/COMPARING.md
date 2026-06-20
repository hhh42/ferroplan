# Reproducing the cross-planner comparison

`ferroplan` is benchmarked against two C reference planners. **Neither is bundled
in this repository** — Metric-FF is GPL and SGPlan is distributed under a
non-commercial research licence, both incompatible with ferroplan's MIT/Apache-2.0
licensing. The comparison harness ([`compare.py`](compare.py)) shells out to
whatever oracle binaries you point it at, and skips any that are absent — so a
clean checkout still runs (ferroplan-only), and the committed
[`results.md`](results.md) records the numbers from a local run that *did* have the
oracles.

## Get the oracles

**Metric-FF** (Joerg Hoffmann) — <https://fai.cs.uni-saarland.de/hoffmann/metric-ff.html>

```sh
# unpack the source, then build a native binary. On modern clang the K&R C89
# source needs a few flags:
make CC=clang CFLAGS="-O3 -std=gnu89 -w \
  -Wno-implicit-function-declaration -Wno-implicit-int -Wno-return-type"
# (if it stops on a `conflicting types for 'opserr'/'fcterr'` error, the cause is
#  an `errno` parameter name colliding with <errno.h>; rename it to e.g. `e_no`.)
```

**SGPlan6** (Chih-Wei Hsu & Benjamin Wah) — <http://wah.cse.illinois.edu/sgplan/>
ships as a Linux/x86 binary; run it under Docker (`--platform linux/386`) or qemu.

## Run

```sh
# point the harness at the oracle binaries (either is optional)
export FF_METRICFF=/path/to/metric-ff        # native arm64 or x86_64 (auto-detected)
export FF_SGPLAN6=/path/to/sgplan6           # used via Docker linux/386

python3 benchmarks/compare.py \
    --corpus /path/to/ipc-corpus \
    --cat strips,numeric,adl,pref \
    --timeout 20
```

Output is a per-problem table plus a summary: relative speed (geomean vs
Metric-FF) and an IPC-5 metric scoreboard (vs SGPlan6, lower = better). Use
`--no-docker` / `--no-rosetta` to skip an oracle, and `--corpus` to point at a
larger problem set than the small vendored subset under [`ipc/`](ipc).

> Absolute times are machine- and load-dependent; only same-run *ratios* are
> meaningful. Metric-FF run under Rosetta carries ~10 ms/run emulation overhead —
> use a native build for a fair speed comparison.
