# Incident fixtures

Every file here is **real data from a real sweep**, rescued from a directory
`.gitignore` excludes. Each one is the physical evidence for an incident that a
comment in `benchmarks/ipc67.py`, `benchmarks/contention.py` or
`benchmarks/standings.py` names as the cause of a wrong published number.

Regenerate or verify with:

```sh
python3 crucible/tests/fixtures/extract.py          # rewrite
python3 crucible/tests/fixtures/extract.py --check  # diff, non-zero on drift
```

Rows keep their exact source bytes — no re-serialization — so a fixture can
never drift from what the runner actually wrote, and nobody can hand-edit one to
make a test pass without `--check` noticing.

## Why these files exist at all

`.gitignore` excludes `benchmarks/air*/` and every `benchmarks/ipc*.jsonl`
except the three optimal boards' raws, which it un-ignores with a comment
arguing they are "evidence rather than logs". The same argument applies to the
rows below, and until this rescue **a disk failure would have destroyed the only
copy** of the evidence for two of the incidents crucible is supposed to defend.

## What each one defends

| File | Rows | The incident |
|---|---|---|
| `incidents/val-unavailable-15.jsonl` | 15 | **The 15 instances light.** VAL refuses to *ingest* `data-network-2018` and `factory-robot-2026` before reading any plan — "Problem in domain definition!", not the "Parser failed" the 0.20 runner tested for. The rows booked `val=false`, `standings.py` drops those from coverage, and the table read 46/240 and 113/320 where the boards beside it said 53 and 121. `val` is a **tristate**: `null` is *unavailable*, and that is not the verdict *rejected*. Both source files are gitignored; this is the only surviving copy. |
| `incidents/val-red-map-analyzer.jsonl` | 3 | A **real** VAL-RED: the engine produced a plan and VAL rejected it on a domain VAL reads fine. The whole VAL-RED class — "a first-class signal, never to be lumped into search losses" — rests on these three rows. Kept beside the file above so a test can prove the two classify differently. |
| `incidents/val-false-markettrader.jsonl` | 1 | VAL's *typechecker* refuses the problem — "Type problem in problem specification!" — a signature 0.21 was missing, which booked the board's only VAL-RED through exactly that gap. |
| `incidents/memcap-self-inflicted.jsonl` | 7 | **A live bug.** `ipc67.py:493` emits `"mem-cap (self-inflicted: node byte target raised)"`; `standings.py:262` matches `"mem-cap"` by *exact equality*. These seven rows fall through to `early-exit` — the one column the refill loop is refereed by — and all seven are in the published `benchmarks/ipc-standings.md` right now (6 on `2023 numeric`, 1 on `2014 seq-mco t4`). See `docs/roadmap-0.26.md`. |
| `incidents/engine-exit-signal.jsonl` | 11 | Two instruments, one verdict: `RLIMIT_AS` makes the child fail its own allocation; the RSS watchdog `SIGKILL`s it (rc `-9`, no stderr). The watchdog's verdict must be read **before** the generic nonzero-exit branch, or it books as `engine-exit--9` — which is what these rows are. |
| `incidents/multipart-labels.jsonl` | 40 | First-group-only labels collapsed 20 distinct problems onto 3–5 labels: `ipc2026-numeric` held 320 rows under 288 keys, silently breaking the per-instance diff and the `--score-against` join. `instance` is an **int** for single-number filenames and an underscore-joined **string** (`"3_10_50_10"`) otherwise. The type is part of the contract. |
| `incidents/budget-unstamped.jsonl` | 40 | Pre-0.23 rows carry no `budget` stamp, so `classify()` falls back to the registry. The tier-move mechanism depends on the row's own stamp winning wherever it exists; these prove the fallback still works. |
| `conditions/timeline-*.json` | 4 files | **Only 4 of the 76** conditions files on this box carry a per-sample `timeline` — the watcher only started writing one at 0.25 (`PER-INSTANCE-RETRY.md` step 1). The resume gate's entire contention side is untestable without them, and every one is gitignored. |
| `conditions/rollup-only.json` | 1 | No timeline at all — what 72 of 76 real files look like. The gate must **fail closed** on these, not treat them as clean by omission. |
| `conditions/degraded-old-idle-rule-mco-t*.json` | 2 | The 0.24 verdict change, on the boards that forced it. An mco `--threads 4`/`8` board burns 40–80% of this 10-core box *by design*, so the old idle-floor rule read `DEGRADED` in an empty room. These are the only two `DEGRADED` records on the box, and both are mco. The verdict moved onto named-competitor load (`< 25.0`) for exactly this reason. |

## Whole boards are already tracked — don't duplicate them

Two complete 12-board backfill sets are committed and hermetic:
`benchmarks/air-0.19.0/` and `benchmarks/air-0.21.0/` (48 files each — `.jsonl`,
`.md`, `.log`, `.done`), plus the three promoted optimal raws
(`benchmarks/ipc-opt-2008-11.jsonl`, `ipc2014-opt.jsonl`, `ipc2026-opt.jsonl`).

Board-render and round-trip goldens read those directly, the way
`crates/ferroplan/tests/fluent_fold.rs` reads `benchmarks/bench/`:

```rust
const AIR21: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../benchmarks/air-0.21.0/");
```

Copying them in here would double 1.4 MB to no purpose and create a second copy
that can drift from the first.
