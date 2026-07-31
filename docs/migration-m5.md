# Migration: cloud container → M5 MacBook Air (local)

Written at the 0.20 cut (2026-07-31), by direct request. The cloud
sessions' container restarts killed long sweeps six-plus times in the
0.20 cycle alone (the resume-aware drivers and self-check-ins exist
purely to survive them), the corpus keeps growing (ipc-2026n added
~300 instances), and the Air's P-cores are far faster single-threaded
than the shared cloud vCPUs — which is the whole game for
coverage-at-budget sweeps (`ff --threads 1` per instance). The 0.20
cut finishes on the cloud box; everything after happens on the Air.

## First session on the Air, in order

1. **Toolchain**: rustup (stable), Python 3.11+, node (for the wasm
   smoke), maturin, cmake+a C++ compiler (for VAL). Clone the repo;
   `main` is the source of truth — the working branch history is
   in it (house rule: finish in main).
2. **Corpus + VAL**: `bash benchmarks/get-ipc.sh` (multi-GB; the
   .ipc-corpus dir is gitignored) and `bash benchmarks/get-val.sh`
   (builds VAL under benchmarks/.val — CMake project, builds fine on
   macOS/ARM).
3. **Green check**: `cargo test --all --release` — all suites must
   pass before anything else. Known macOS notes below.
4. **THE RE-BASELINE** (mandatory before any A/B claim): every
   scoreboard number in the repo is relative to the old 4-core cloud
   box — budget-edge flappers were documented at 29 s and 59 s
   against 60 s lines. Faster silicon inflates every board, so
   nothing on the Air may be compared against a cloud-box number.
   Re-sweep EVERY canonical board with the same driver pattern as
   `benchmarks/cut20-sweeps.sh` (all twelve boards), promote, and
   regenerate `benchmarks/standings.py`. Record the re-baseline as
   its own phase in the next roadmap ("the new box"), with the old
   boards preserved in git history as the cloud-era record. From
   then on, A/B is Air-vs-Air only.
5. **Thermals, honestly**: the Air is fanless and a 10-hour
   all-core sweep is its worst case. Run sweeps at `--jobs 2` (not
   3) so clocks stay stable and coverage-at-timeout stays
   comparable across a sweep; a plugged-in, lid-open (or
   `caffeinate -s`) machine. If sweep noise shows up between
   morning and evening boards, that is throttle — note it in the
   record rather than reading it as engine movement.

## macOS porting notes (all small, none blocking)

- **RLIMIT clamp** (`search.rs rlimit_budget`): reads
  `/proc/self/limits`, Linux-only. On macOS it degrades safely to
  no-clamp (usize::MAX) — with 16–32 GB unified memory and the
  Phase-4 node model this is fine for single/dual-job sweeps.
- **Runner mem-cap** (`ipc67.py` `RLIMIT_AS` via
  `resource.setrlimit`): macOS does not reliably enforce
  `RLIMIT_AS`. The per-job memory cap therefore may not fire; if
  mem-cap rows matter on a board, replace it with an rusage
  watchdog (poll `ru_maxrss`, kill over budget) — a small runner
  patch, named here so it isn't a surprise when the mem-cap column
  reads zero.
- **Wheels**: a local `maturin build` produces macOS-ARM wheels,
  not the manylinux x86 wheel that ships. Publish manylinux via CI
  (GitHub Actions + maturin) or zig-cross; the release checklist's
  wheel gate becomes "the CI wheel builds", not a local artifact.
- **`/usr/bin/time`, `nproc`, GNU sed-isms**: the benchmark scripts
  already avoid `/usr/bin/time` (absent on the cloud box too); any
  new script should keep using Python's `resource`/`time` instead.
- **wasm + screens**: `crates/ferroplan-wasm/build.sh` and the
  Playwright smoke work on macOS; Playwright will fetch its own
  chromium (the cloud box pinned a preinstalled one via
  `PLAYWRIGHT_BROWSERS_PATH` — unset that locally).

## What does NOT change

- Every working agreement in CLAUDE.md: finish in main, cycle
  discipline in docs/roadmap-0.N.md, fixtures first, measured win
  or recorded negative, casualties named and solo-checked, full
  pre-flight before any cut (RELEASING.md).
- The corpus/runner/standings machinery — it is all path-relative
  and platform-clean apart from the notes above.
- The resume-aware sweep-driver pattern — still useful (a laptop
  sleeps, a session ends); `.done` markers stay.

## State at handoff

- 0.19.0 published. 0.20 phases 1–5 complete and in main; the 0.20
  cut sweeps run on the cloud box and the cut record, pre-flight,
  and publish-ready main land there (see docs/roadmap-0.20.md,
  Phase 6). If migration happens before the cut completes, the cut
  can equally be re-run on the Air — but then its boards are
  Air-baselined and MUST NOT be A/B'd against the 0.19 cloud
  boards; the honest cut record would say exactly that.
- Deferred ledger (carry into 0.21 scoping): the temporal
  emission-layer repair (map-analyzer third decode, Phase 5
  record), numeric-reachability wall (sailing class), per-node
  fv/fdef sharing, lifted grounding watch, h-surgery bet, the
  optimal-mode entry for the 2026 -opt domain pairs, and the
  macOS mem-cap watchdog above.
