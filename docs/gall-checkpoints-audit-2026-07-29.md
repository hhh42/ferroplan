# Gall Checkpoints Audit — 2026-07-29

Independent, read-only re-verification of every checkpoint in
`docs/gall-checkpoints.md` (0 through 21 prose-style, CE-GALL-22 through
CE-GALL-40 receipt-backed — 40 entries; CE-GALL-27 is not a standalone
checkpoint, see below). This audit does not modify
`docs/gall-checkpoints.md`, any `CE-GALL-NN.json` receipt, or any test
file — it is external verification of the existing record.

## How this was produced

- Full test suites re-run for real: `cd plugins/chatman-ecosystem && python3
  -m pytest tests/ -q` (all green) and `cargo test -p ferroplan-mcp` (all
  green — 48 tests across `protocol.rs`, `session_protocol.rs`,
  `session_goal_advance.rs`, `session_lifecycle_bookends.rs`,
  `merged_server.rs`, `dogfood_chain.rs`, plus crate unit tests).
- Track A (CE-GALL-22–40): every receipt's `standing`/`reason` cross-checked
  against its doc section, every named `positive_witness`/`negative_falsifier`
  test function confirmed to exist at (or within a few lines of) its cited
  location, `test_receipts.py` re-run (166/166 passed, includes schema
  validation + the promotion-law test), and five self-flagged items
  spot-checked individually.
- Track B (checkpoints 0–21): the four checkpoints citing a concrete,
  re-runnable artifact were actually re-run; the rest were classified by
  what their own text does or doesn't give a reader to check.

## Track A — CE-GALL-22 through CE-GALL-40 (receipt-backed)

All 18 checkpoints: **doc prose and receipt JSON agree exactly** on
`standing`/`reason` — no discrepancies found. All named test functions
exist (line numbers have drifted a few lines from later edits in five
cases — 35, 36, 37, 38, 39 — never at the wrong file or missing entirely).
No checkpoint is marked `ALIVE`; every receipt has
`replayed_outside_session: false` and `sealed_at_commit: null`, consistent
with the promotion law (`test_promotion_law_actually_refuses` passes).

| Checkpoint | Standing (reason) | Verdict | Note |
|---|---|---|---|
| CE-GALL-22 | PARTIAL_ALIVE (NO_FALSIFIER) | CONFIRMED | No falsifier by design; doc and receipt agree |
| CE-GALL-23 | PARTIAL_ALIVE (NO_REPLAY) | CONFIRMED | Named tests exist and pass |
| CE-GALL-24 | PARTIAL_ALIVE (NO_REPLAY) | CONFIRMED | " |
| CE-GALL-25 | PARTIAL_ALIVE (NO_REPLAY) | CONFIRMED | " |
| CE-GALL-26 | PARTIAL_ALIVE (NO_FALSIFIER) | CONFIRMED | " |
| CE-GALL-28 | PARTIAL_ALIVE (NO_REPLAY) | CONFIRMED | " |
| CE-GALL-29 | PARTIAL_ALIVE (NO_REPLAY) | CONFIRMED | " |
| CE-GALL-30 | PARTIAL_ALIVE (MOCKED) | CONFIRMED | No positive witness by design (refuted claim) |
| CE-GALL-31 | UNSUPPORTED (DEPENDENCY_MISSING) | CONFIRMED | `verify_chain` genuinely absent repo-wide |
| CE-GALL-32 | PARTIAL_ALIVE (NO_REPLAY) | CONFIRMED | Named test exists and passes |
| CE-GALL-33 | PARTIAL_ALIVE (DEPENDENCY_MISSING) | CONFIRMED | Open defect, no falsifier by design |
| CE-GALL-34 | PARTIAL_ALIVE (NO_REPLAY) | CONFIRMED | Named tests exist and pass |
| CE-GALL-35 | PARTIAL_ALIVE (NO_REPLAY) | CONFIRMED | Line drift only (97/191 doc vs 104/201 actual) |
| CE-GALL-36 | PARTIAL_ALIVE (NO_REPLAY) | CONFIRMED | Line drift only |
| CE-GALL-37 | PARTIAL_ALIVE (NO_REPLAY) | CONFIRMED | Line drift only |
| CE-GALL-38 | PARTIAL_ALIVE (NO_REPLAY) | CONFIRMED | Line drift only |
| CE-GALL-39 | UNSUPPORTED (DEFECT_OPEN) | CONFIRMED | Falsifier present and passes (this is the open-defect demonstration) |
| CE-GALL-40 | PARTIAL_ALIVE (NO_REPLAY) | CONFIRMED | `dogfood_chain.rs` is the sole file, no stale duplicate |

## Track B — Checkpoints 0 through 21 (prose-only)

Four checkpoints cite a concrete, re-runnable artifact — all four re-run
live and pass:

| # | Artifact | Result |
|---|---|---|
| 0 | `pytest tests/test_phase_space.py::test_every_invariant_key_is_understood` | PASS |
| 3 | `pytest tests/test_authority.py::test_single_actuator_policy_is_enforced` | PASS |
| 7 | `cargo check --workspace` + `cargo test --workspace` | PASS (clean, all green) |
| 13 | `benchmarks/get-val.sh` / VAL binary, run against `kiln-pack-domain.pddl`/`kiln-pack-6.pddl` | RECONFIRMED, not stale (exit 0, real output) |

The remaining 18 checkpoints were classified by what their own text
supports checking — not by inventing evidence they don't cite:

| # | Standing | Verdict | Note |
|---|---|---|---|
| 1 | ALIVE (fixture scope) | UNVERIFIABLE FROM RECORD | Narrative only |
| 2 | PARTIAL_ALIVE | UNVERIFIABLE FROM RECORD | Names a defect, not independently re-run |
| 4 | PARTIAL_ALIVE | UNVERIFIABLE FROM RECORD | Narrative only |
| 5 | ALIVE (fixture scope) | UNVERIFIABLE FROM RECORD | Narrative only |
| 6 | PARTIAL_ALIVE | UNVERIFIABLE FROM RECORD | No file/command cited |
| 8 | PARTIAL_ALIVE | UNVERIFIABLE FROM RECORD | Cites a file, not independently re-checked |
| 9 | PARTIAL_ALIVE | UNVERIFIABLE FROM RECORD | See contradiction chain below |
| 10 | UNSUPPORTED | UNVERIFIABLE FROM RECORD | Bare assertion (consistent with its own honesty) |
| 11 | UNSUPPORTED | RECONFIRMED | Absence-of-mechanism claim checked and confirmed absent |
| 12 | PARTIAL_ALIVE | UNVERIFIABLE FROM RECORD | No independent command |
| 14 | PARTIAL_ALIVE | **STALE (uncorrected in place)** | See contradiction chain below |
| 15 | ALIVE (fixture scope) | RECONFIRMED | Cited scripts exist, claim already scoped carefully |
| 16 | PARTIAL_ALIVE | RECONFIRMED | Cited script exists, no overclaim |
| 17 | UNKNOWN | UNVERIFIABLE FROM RECORD | Explicitly "not attempted" — honest by design |
| 18 | UNSUPPORTED | UNVERIFIABLE FROM RECORD | Bare assertion |
| 19 | PARTIAL_ALIVE | **STALE (uncorrected in place)** | See contradiction chain below |
| 20 | PARTIAL_ALIVE | RECONFIRMED | Own text already carries the CE-GALL-30 caveat |
| 21 | PARTIAL_ALIVE | RECONFIRMED | Concrete claims (PR #2, commit, CI job), internally consistent; GitHub state not re-checked live in this pass |

## Checkpoints the audit could **not** independently confirm

`1, 2, 4, 5, 6, 8, 9, 10, 12, 17, 18` — eleven checkpoints whose own text
gives nothing concrete (no file, command, or quoted output) to check
against. This is not evidence they're wrong; it's the honest limit of
auditing prose. Checkpoints 10, 17, 18 already self-report as
`UNSUPPORTED`/`UNKNOWN`, so this matches their own claims. Checkpoints
1, 2, 4, 5, 6, 8, 9, 12 claim `PARTIAL_ALIVE` or `ALIVE (fixture scope)`
on narrative evidence alone — a future audit with access to the original
session transcripts, or a fresh live replay, would be needed to actually
confirm or refute them.

## Contradiction chains

| Chain | Status | Detail |
|---|---|---|
| Checkpoint 13 → CE-GALL-30 → CE-GALL-38 | **Self-correcting** | CE-GALL-30's hand-fabrication finding stands as historical record; CE-GALL-38 explicitly self-describes as a partial mechanical re-witness, not a full resolution. No overclaim resurfaces. |
| Checkpoint 14/19 → CE-GALL-31 → CE-GALL-39 | **NOT self-correcting** | Both Checkpoint 14 and Checkpoint 19's own "Required proof" bullet lists still assert fork detection/refusal unqualified. The correction exists only in a prepended update box, not woven into the original checklist text — a reader who reads only the bullet list, not the update box, is still misled. |
| Checkpoint 9 → CE-GALL-37 | **NOT self-correcting** | Checkpoint 9's body still reads "architecturally absent from the MCP tool schema" as a blanket claim. CE-GALL-37 narrows this (recursive descent exists via `cmca_allocate_recursive`; only `bind_allocation_receipt`'s flat `previous_receipt` chaining is the actual remaining gap). No forward-pointer was added to Checkpoint 9's text. |
| Checkpoint 20 | **Self-correcting** | Its own update box already carries the CE-GALL-30 hand-fabrication caveat in place. |

**Follow-up candidate (not fixed by this audit):** Checkpoints 9, 14, and
19 would benefit from an in-place correction (a one-line strike-through or
forward-pointer in their original bullet text, not just a prepended update
box) so a reader stopping at the checkpoint's own section isn't misled by
claims later checkpoints refute.

## CE-GALL-27 status

Confirmed: **not a standalone checkpoint.** It appears exactly once in the
whole file, as an inline note inside Checkpoint 3's section body (no
standalone `## ... (CE-GALL-27)` header — the header sequence goes
CE-GALL-26 → CE-GALL-28), and no
`plugins/chatman-ecosystem/receipts/CE-GALL-27.json` exists. This is a
revision-event label, not a numbering gap to be treated as missing.

## Non-claims

- This audit does not itself replay anything in a separate session, so it
  cannot promote any checkpoint's standing under the promotion law — it is
  evidence for a future replay, not a replay itself.
- No defect found here was fixed, including the already-known
  `bind_allocation_receipt` / `cmca_allocate_recursive` schema mismatch
  surfaced by CE-GALL-40's own receipt, and the two uncorrected-in-place
  contradiction chains (14/19, 9) above — these are named as follow-up
  candidates only.
- Checkpoints 1, 2, 4, 5, 6, 8, 9, 12's `PARTIAL_ALIVE`/`ALIVE` claims are
  not confirmed or refuted by this audit — they remain exactly as
  verifiable (or not) as they were before this pass; "UNVERIFIABLE FROM
  RECORD" is a statement about the record, not a downgrade of the
  checkpoint's standing.
