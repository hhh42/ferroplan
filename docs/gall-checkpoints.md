# Gall Checkpoints for the Chatman Ecosystem

Last updated: 2026-07-29 (session audit, second pass — see "Audit log" at
the end).

Each checkpoint must be a **complete, useful system at its own scale**. A
checkpoint is not passed because source exists. It is passed only when its
stated behavior executes, fails lawfully, and produces replayable evidence.

Standing vocabulary (see `~/.claude/rules/no-overclaiming-rust.md` for the
full discipline this repo runs under): `ALIVE`, `PARTIAL_ALIVE`, `BLOCKED`,
`MOCKED`, `REFUSED`, `UNSUPPORTED`, `UNKNOWN`. A standing may only be
upgraded on exhibited evidence (a command, its output, and what it proves) —
never on source presence alone.

## How to use this file (for any agent picking up work here)

1. Read the "Current standing" line under each checkpoint before touching
   it. Do not re-litigate a standing without new evidence.
2. Pick the next open item from "Recommended Release Sequence" unless a
   specific checkpoint was requested.
3. Do real work: run commands, read actual output, update the standing with
   the exact evidence that justifies it. Follow the no-overclaiming
   discipline — a checkpoint's standing is a claim, and claims need receipts.
4. Append to "Audit log" at the end with a dated entry: what you attempted,
   what you found, what changed. Do not delete prior entries.
5. If you build something (a script, a vendored tool, a fixture), leave it
   in the repo in the appropriate location and reference its path here.
6. Never silently promote a standing to `ALIVE` for a partially-exercised
   surface. `PARTIAL_ALIVE` with a named exact blocking hop is more useful
   and more honest than a false `ALIVE`.

---

## 0. Constitutional Vocabulary

**Working system**

The ecosystem has one stable vocabulary for:

* observation;
* admission;
* allocation;
* planning;
* manufacture;
* validation;
* actuation;
* receipt;
* refusal;
* standing.

Core laws:

```text
A = μ(O*)
zero unreceipted actuation
source presence ≠ execution evidence
candidate plan ≠ validated plan
grant ≠ execution
```

**Falsifier**

Two repositories use the same term for incompatible objects or authority levels.

**Current standing:** `ALIVE`

---

## 1. Phase-Space Kernel

**Working system**

A six-dimensional product state exists:

```text
epistemic
× allocation
× planning
× actuation
× drift
× conformance
```

Transitions are explicit. Invalid combinations are refused. Repository mutation collapses advanced standing.

**Required proof**

* Every state validates.
* Every declared transition executes.
* Every undeclared transition refuses.
* Invariants reject illegal combinations.
* The manufacturer is active only during `actuation=manufacturing`.

**Current standing:** `ALIVE` for source-law and fixture scope. Confirmed live
in the 2026-07-29 audit: the `PostToolUse` hook auto-collapsed the canonical
phase vector back to baseline on a new observation event without any
explicit `phase.py transition` call — "repository mutation collapses
advanced standing" fires mechanically, not just by convention.

---

## 2. Claude Projection Loads

**Working system**

The marketplace and plugin install into a clean Claude Code environment.

Claude Code discovers:

* plugin manifest;
* agents;
* skills;
* hooks;
* monitors;
* MCP server;
* plugin settings;
* user configuration.

**Required proof**

```text
clean plugin cache
→ marketplace add
→ plugin install
→ plugin validate
→ session start
→ no loader errors
```

**Falsifier**

Any declared component is missing, rejected, duplicated, or silently ignored.

**Current standing:** `PARTIAL_ALIVE` (was `UNKNOWN`)

2026-07-29 audit findings:
- `claude plugin validate --strict` passes for both the plugin manifest and
  the marketplace manifest.
- All 8 declared agent files, `.mcp.json`, `.lsp.json`,
  `monitors/monitors.json`, `skills/` resolve on disk — no missing,
  duplicated, or silently-ignored component.
- `claude plugin list` shows `chatman-ecosystem@chatman-ecosystem`
  `✔ enabled` in both project and user scope, no loader errors, in the live
  running session.
- **Open defect found**: the installed marketplace clone
  (`~/.claude/plugins/marketplaces/chatman-ecosystem`) is stale/orphaned
  relative to canonical `origin/main`. It sits at commit `75bb6ee`, which
  `git merge-base --is-ancestor 75bb6ee origin/main` confirms is **not an
  ancestor** of the current `origin/main` (`d047fd9` at audit time). Files
  adopted from PR #2 into `main` (`scripts/effective-phase.py`,
  `scripts/actuation-intent.py`, `scripts/grant-actuation.py`,
  `ontology/authority-graph.ttl`) exist in the source repo but are **absent
  from the plugin cache this session actually runs against.**
- Not exercised: a true clean-cache install
  (`marketplace add → install → validate → session start` from an empty
  cache). That requires spawning a separate Claude Code process/cache,
  outside a single session's tool surface — named as the exact blocking hop,
  not silently skipped.

**Next step**: reproduce the marketplace-clone refresh path (`claude plugin
update chatman-ecosystem` or equivalent) and confirm it pulls `d047fd9` or
later; then re-run this checkpoint from a genuinely clean cache (may require
an external harness, e.g. a throwaway container or a fresh `$HOME`).

---

## 3. Mechanical Agent Authority

**Working system**

Claude Code mechanically enforces role ceilings.

* Controller routes but cannot edit.
* Observer observes but cannot edit.
* Allocator allocates but cannot plan or edit.
* Planner plans but cannot edit.
* Validator validates but cannot repair.
* Auditor audits but cannot publish.
* Manufacturer is the sole source editor.
* Manufacturer runs in a worktree.

**Required proof**

Attempt direct edits from every non-manufacturing agent and observe refusal.

Attempt manufacture outside `actuation=manufacturing` and observe refusal.

**Current standing:** `PARTIAL_ALIVE`

2026-07-29 audit findings:
- None of the 8 agent `.md` files under `plugins/chatman-ecosystem/agents/`
  declare a `tools:` frontmatter field. Confirmed independently by this
  session's own Agent-tool listing, which annotates every one of the 8
  chatman-ecosystem agents with `(Tools: All tools)`. No mechanical denial
  exists at the Claude Code harness level.
- Live test: spawned `rdf-observer` (agent whose prose says "You do not
  edit source, execute plans, or authorize actuation") and asked it to
  edit a throwaway file outside the repo. It refused — but by **choosing to
  honor its own role prose** (it treated the instruction as suspicious
  content and declined), not because the harness blocked the `Edit` tool
  call. Had the model decided differently, the edit would have succeeded
  with no mechanical backstop.
- Conclusion: role separation is currently **prompt-level compliance**, not
  **mechanical enforcement**. The checkpoint's own name ("Mechanical Agent
  Authority") is not yet met by what's in `main`.
- PR #2 (`agent/v26.7.29-claude-projection`, still open/draft, not merged)
  proposes exactly this fix: every agent declaring `tools:` and denying
  `Write`/`Edit`/`NotebookEdit` except `source-manufacturer` (isolated in a
  worktree). See PR #2 status below for why it hasn't landed.

**Next step**: add `tools:` allow/deny lists to each of the 8 agent
frontmatter files (the smallest slice of PR #2's rewrite that would move
this checkpoint's needle), and re-run the same live refusal test — this
time expecting a harness-level tool-permission error, not a model choice.

---

## 4. Bounded Lifecycle Observation

**Working system**

Claude hooks emit observation candidates for:

* startup;
* resume;
* clear;
* compact;
* fork;
* tool success;
* tool failure;
* tool batch completion;
* configuration change;
* worktree creation;
* worktree removal;
* session stop.

Hooks do not directly manufacture semantic truth.

```text
hook event
→ observation candidate
≠ admitted phase transition
```

**Required proof**

Every supported event emits a deterministic candidate with stable identifiers and digests.

**Falsifier**

A hook advances canonical phase state without admission.

**Current standing:** `PARTIAL_ALIVE`

2026-07-29 audit note: repeatedly observed in this session that `PostToolUse`
fires on every Bash/Edit/Write call *regardless of whether the mutation was
inside the tracked repo* (e.g. a `Bash` call writing to `/tmp` still
produced a ledger event). This is defensible (bounded observation, not
scoped filtering) but worth flagging: it means the pending-event count can
include events with zero actual repo diff, which the observation/replan
cycle must (and does) still handle correctly — confirmed via
`session_observe` returning `fact_surprises: []` and
`remaining_plan_valid: true` for such no-diff events.

---

## 5. Effective Phase Projection

**Working system**

Canonical phase state is combined with pending observations.

A pending mutation makes the effective state:

```text
observed
× unallocated
× unplanned
× sealed
× drifted
× unknown
```

even when an older snapshot claims advanced standing.

**Required proof**

1. Advance the canonical state.
2. Emit an unadmitted mutation event.
3. Verify that effective state collapses.
4. Admit the event frontier.
5. Verify that state can advance again only with new evidence.

**Current standing:** `ALIVE` for unit-fixture scope; also exercised live
end-to-end in the 2026-07-29 session (not just fixtures): advanced the
canonical vector to `receipted/stable`, made a real commit, watched the
`PostToolUse` hook auto-collapse the canonical vector to baseline, then
closed the loop again (`session_observe` → `session_think` → CMCA →
`bind_allocation_receipt` → `validate` → `bind_plan_receipt` →
`loop.py admit` → `phase.py transition`) twice in the same session — once
for a real source commit, once for a no-diff `/tmp` Bash observation. Both
reconciliation cycles produced a clean 0-pending ledger and a `stable`
phase vector.

---

## 6. Generated Artifact Ownership

**Working system**

Every generated Claude projection artifact has:

* canonical owner;
* generator identity;
* source digest;
* projection digest;
* regeneration command;
* mutation policy.

The generated guard reads the ownership registry rather than a hard-coded file list.

**Required proof**

* Direct edit of a generated artifact refuses.
* Editing its canonical source permits regeneration.
* Regeneration produces deterministic output.
* Repeated generation is byte-identical.

**Falsifier**

A tracked projection can be hand-edited without changing its admitted source.

**Current standing:** `PARTIAL_ALIVE`

Ownership and refusal law exist. Full ggen generation and receipt binding remain open. Not re-audited in the 2026-07-29 pass.

---

## 7. Combined Ferroplan MCP Authority

**Working system**

One stdio MCP server exposes the complete bounded tool surface:

* parsing;
* solving;
* validation;
* decomposition;
* persistent sessions;
* observation;
* bounded thinking;
* CMCA;
* canonical digests;
* allocation receipts;
* plan receipts;
* receipt verification.

**Required proof**

```text
initialize
→ tools/list
→ resources/list
→ invoke all tools
→ malformed-input refusals
→ clean shutdown
```

**Current standing:** `ALIVE` for compile and test scope.

`cargo check --workspace` and `cargo test --workspace` came back green
multiple times in the 2026-07-29 session (both before and after a real
commit). Every MCP tool actually used this session
(`session_open`/`session_observe`/`session_think`/`session_status`,
`cmca_allocate`, `bind_allocation_receipt`, `bind_plan_receipt`, `validate`,
`verify_receipt`) behaved as documented, including refusing malformed input
(out-of-bounds `parent` index, cyclic `parent` ancestry, tampered receipt).

---

## 8. Top-Level CMCA Allocation

**Working system**

An admitted repository observation produces exactly:

```text
8 candidates × 10 factors
```

CMCA returns bounded shares and binds:

* candidate array;
* factor order;
* allocation output;
* BCINR-CMCA revision;
* observation frontier;
* predecessor receipt.

**Required proof**

* Exactly eight candidates accepted.
* Seven or nine candidates refused.
* Wrong factor count refused.
* Wrong BCINR revision refused.
* Tampered allocation result refused.
* Repeated input produces identical allocation evidence.

The 8-candidate/10-factor happy path was exercised repeatedly this session
with real allocation receipts bound and admitted.

2026-07-29 audit (second pass) closed the four refusal cases the first pass
left untested, each as a real `cargo test` against the compiled
`ferroplan-mcp` binary over stdio (not a fixture or a mock):

- **7 candidates refused** — new test `cmca_allocate_rejects_seven_candidates`
  (`crates/ferroplan-mcp/tests/session_protocol.rs`). `cmca_allocate` returns
  a tool error `"CMCA requires exactly 8 nodes; received 7"`.
- **9 candidates refused** — new test `cmca_allocate_rejects_nine_candidates`.
  Same error shape, `"received 9"`.
- **Wrong factor count refused** — new test
  `cmca_allocate_rejects_wrong_factor_count` (8 candidates, 9 factors each).
  Error: `"candidate `node-0` requires 10 factors; received 9"`.
- **Wrong BCINR revision refused** — already covered by the pre-existing
  `bind_allocation_receipt_rejects_wrong_bcinr_revision` test; reran it this
  pass (`cargo test -p ferroplan-mcp --test admission_protocol`) to confirm
  it's still green, rather than assuming stale coverage still holds.
- **Tampered allocation result refused** — new test
  `verify_rejects_tampered_allocation_payload`
  (`crates/ferroplan-mcp/tests/admission_protocol.rs`). This closes a gap
  the *existing* `verify_rejects_tampered_digest` test's own doc comment
  named as uncovered: that test only flips a character of the `receipt`
  field. The new test instead mutates `payload.allocations` (the actual
  bound CMCA output) in place, leaving `payload_digest`/`receipt` exactly as
  bound, and confirms `verify_receipt` still catches it —
  `payload_digest_valid: false, valid: false` — proving the server
  recomputes the payload digest from real payload bytes rather than
  trusting the envelope's self-reported digest.

Also added `cmca_allocate_is_deterministic_across_processes`
(`session_protocol.rs`): two *separate* `ferroplan-mcp` processes fed the
same 8-candidate/10-factor input produce byte-identical `payload_digest`
and `payload` — closing "Repeated input produces identical allocation
evidence" with cross-process evidence, not just same-call determinism.

All 6 new/reconfirmed tests pass: `cargo test -p ferroplan-mcp --test
session_protocol` (10/10) and `cargo test -p ferroplan-mcp --test
admission_protocol` (16/16). `cargo fmt --check` and `cargo clippy -p
ferroplan-mcp --all-targets --all-features -- -D warnings` are both clean on
this change.

Every required-proof line this checkpoint states now has an exhibited
automated test against the real compiled binary:

| Required proof | Test | Status |
|---|---|---|
| Exactly eight candidates accepted | `cmca_allocate_returns_an_allocation` | pass |
| Seven candidates refused | `cmca_allocate_rejects_seven_candidates` | pass |
| Nine candidates refused | `cmca_allocate_rejects_nine_candidates` | pass |
| Wrong factor count refused | `cmca_allocate_rejects_wrong_factor_count` | pass |
| Wrong BCINR revision refused | `bind_allocation_receipt_rejects_wrong_bcinr_revision` | pass |
| Tampered allocation result refused | `verify_rejects_tampered_allocation_payload` | pass |
| Repeated input → identical evidence | `cmca_allocate_is_deterministic_across_processes` | pass |

**Current standing:** `ALIVE` for the automated-test surface (`cargo test -p
ferroplan-mcp` — 6 new/reconfirmed tests, all green — plus `cargo fmt
--check` and `cargo clippy -p ferroplan-mcp --all-targets --all-features --
-D warnings` clean). Not claimed beyond that surface: this is compiled-binary
protocol-level evidence, not a live end-to-end run through a real Claude
agent session (that rung belongs to Checkpoint 12's ladder, not this
checkpoint's own required proof).

---

## 9. Recursive Multifractal Allocation

**Working system**

Any admitted CMCA node can become the root of another eight-node frontier.

```text
parent allocation
→ selected node
→ local observation
→ eight local candidates
→ local allocation
→ local receipt
→ consequence returned upward
```

Each descent binds the parent allocation receipt. Each return binds the local result.

**Required proof**

* Depth one allocation.
* Depth two allocation.
* Parent receipt mismatch refusal.
* Cyclic ancestry refusal.
* Missing return consequence refusal.
* Deterministic replay at each depth.

**Current standing:** `PARTIAL_ALIVE`

2026-07-29 audit findings:
- `cmca_allocate` accepts per-candidate `parent` indices within a single
  call and builds a real tree: interior (parent) nodes receive `share: 0`
  — all allocation mass cascades to leaf nodes. This is genuine, confirmed
  behavior, not assumed.
- Out-of-bounds parent index refused: `"candidate \`orphan-bad-parent\` has
  invalid parent 99"`.
- Cyclic parent chain refused: `"parent relation contains a cycle through
  0"`.
- **Gap found**: `bind_allocation_receipt`'s only chaining field is a flat
  `previous_receipt` (sequential predecessor). There is no
  parent-allocation-receipt field, no "selected node" field, and no
  "consequence returned upward" field. True cross-call recursive descent —
  what the checkpoint's "Working system" diagram actually describes — is
  **architecturally absent from the MCP tool schema**, not merely
  untested. The in-array tree support (above) is real but is a different,
  narrower thing than what this checkpoint asks for.

**Next step**: decide whether recursive CMCA should be modeled as (a) a new
MCP tool/field for parent-receipt-bound descent, or (b) documented as
out-of-scope and the checkpoint's "Working system" text narrowed to match
what actually exists (single-call tree allocation). Don't leave the
mismatch unresolved.

---

## 10. MFW/POWL Planner Routing

**Working system**

MFW or POWL v2 decides which planner rail may answer a planning request.

Ferroplan is one deterministic implementation, not the planning constitution.

```text
admitted planning request
→ planner selection
→ Ferroplan candidate
→ validation
→ promotion or refusal
```

**Required proof**

* Planner identity and version are bound.
* Routing is deterministic for the same admitted request.
* Unsupported domains produce typed refusal.
* A candidate plan cannot self-promote.

**Current standing:** `UNSUPPORTED`

Direct Ferroplan planning exists. Constitutional planner routing is not yet wired. Not re-audited in the 2026-07-29 pass — standing unchanged.

---

## 11. Isolated Source Manufacture

**Working system**

One admitted plan step executes inside an isolated Git worktree.

The manufacturer may change only:

* the selected plan step;
* tightly coupled generated outputs;
* explicitly admitted dependencies.

**Required proof**

* Worktree is created.
* Exact base commit is recorded.
* Change remains inside admitted scope.
* Main working tree remains untouched.
* Worktree cleanup is deterministic.
* Mutation emits a new observation candidate.
* Advanced standing collapses after manufacture.

**Current standing:** `UNSUPPORTED` (was `UNKNOWN`)

2026-07-29 audit: no worktree-related script, profile, or ontology file
exists anywhere under `plugins/chatman-ecosystem/`. This is not "untested" —
there is no mechanism to test. The closest thing is PR #2's still-unmerged
"Isolate and bound the source manufacturer agent" commit
(`7bb5239ce7922e5c790080ed3ec0c0d9ecaa4771`), which does not exist on
`main`. This session's actual manufacturing step (the `.claude/settings.json`
model pin) was committed directly to the main working tree, not in an
isolated worktree — consistent with "not yet implemented," not a defect in
what was done.

**Next step**: either adopt PR #2's worktree-isolation commit (would need
its own review given it also changes agent tool grants — see Checkpoint 3),
or write a standalone `scripts/manufacture-in-worktree.py` that: creates a
worktree at the current HEAD, records the base commit SHA, applies exactly
the admitted plan step's diff, runs build+test inside the worktree, and
either merges back (fast-forward only) or reports failure without touching
the main tree.

---

## 12. Verification Ladder

**Working system**

Evidence advances through distinct verification rungs:

```text
unit
→ integration
→ end-to-end
→ chaos
→ stress
→ benchmark
→ independent validator
```

Each rung has its own executor and claim ceiling.

**Required proof**

* Lower-rung success cannot imply higher-rung success.
* Failed checks remain failed.
* Unavailable executors produce `UNKNOWN`.
* Independent validation records executable identity and input digests.

**Current standing:** `PARTIAL_ALIVE`

Projection fixtures and MCP tests are green. Full ladder remains incomplete. Not re-audited in the 2026-07-29 pass beyond what Checkpoint 13 (VAL) newly unlocks.

---

## 13. Independent PDDL Validation

**Working system**

A planner-independent validator, such as VAL, checks the exact emitted plan against the exact domain and problem.

Ferroplan replay remains useful but is not independent evidence.

**Required proof**

* Valid plan accepted.
* Invalid plan refused.
* Tampered plan refused.
* Domain or problem digest mismatch refused.
* Validator executable identity is recorded.
* Validator output is bound into the receipt.

**Current standing:** `PARTIAL_ALIVE` (was `UNSUPPORTED`)

2026-07-29 audit: vendored and built real, independently-sourced VAL
(`KCL-Planning/VAL`) via `benchmarks/get-val.sh` into
`benchmarks/.val/VAL/build/bin/Validate` (gitignored, self-contained). The
script's pinned CMakeLists needed `-DCMAKE_POLICY_VERSION_MINIMUM=3.5` to
configure against current cmake — worth patching `get-val.sh` to pass that
flag by default so the next run doesn't hit the same wall.

Ran the built `Validate` binary against this session's actual bound
domain/problem/plan (not a toy fixture):
- Valid plan → `Plan valid`, exit 0.
- Reordered/tampered plan (same actions, wrong order) → `Plan failed to
  execute`, exit 1.
- Truncated plan (goal not reached) → `Goal not satisfied` / `Plan
  invalid`, exit 1.
- Mismatched problem (wrong init state) → `Plan failed to execute`, exit 1.

All four required behaviors hold with genuine engine independence — this is
real, not Ferroplan validating itself.

**Not yet done**: wiring VAL into the release loop, and binding VAL's
output (not Ferroplan's own `validate`) into the `validator_result` field
of a bound receipt envelope. `validator_result_digest` in every receipt
bound so far still reflects `ferroplan.validate`, not VAL.

**Next step**: patch `get-val.sh` with the cmake policy flag; add a
`FERROPLAN_VAL` env-var check to whatever produces `validator_result`
payloads so VAL's output (when present) is what actually gets bound.

---

## 14. Canonical Admission Receipts

**Working system**

Allocation and plan evidence are transformed into canonical BLAKE3 envelopes.

A plan receipt binds:

* admitted observation frontier;
* allocation receipt;
* planner identity;
* domain and problem;
* candidate plan;
* independent validator result;
* predecessor receipt.

**Required proof**

* Canonicalization is deterministic.
* Payload digest recomputes.
* Receipt recomputes.
* Wrong predecessor refuses.
* Reordering refuses or canonicalizes identically.
* Payload-only tampering refuses.
* Chain forks are detected.

**Current standing:** `PARTIAL_ALIVE`

Core MCP receipt tests pass. `verify_receipt` recomputation and tamper
detection reconfirmed live in the 2026-07-29 audit (see Checkpoint 19).
Wrong-predecessor and fork-detection cases not individually re-verified
this pass — carried over from prior standing.

---

## 15. Structured BRCE Intent

**Working system**

A protected command is transformed into an exact `ActuationIntent` containing:

* actor;
* operation;
* target;
* argument digest;
* expected preconditions;
* required receipt;
* authority;
* reversibility;
* requested consequence.

The initial protected call is denied after intent creation.

**Required proof**

* Protected command creates an intent.
* Intent digest is deterministic.
* Original call does not execute.
* Unprotected commands do not create false protected intents.
* Equivalent commands canonicalize consistently.

**Current standing:** `ALIVE` for fixture scope.

2026-07-29 audit: `scripts/actuation-intent.py` and `scripts/grant-actuation.py`
exist in the source repo (adopted from PR #2 per
`docs/notes/pr2-claude-projection-ideas-adopted.md`) but are **absent from
the installed plugin cache** this session actually runs against, and are
**not wired into `hooks.json`**. Standing kept at fixture scope, not
upgraded — existence in source is not execution evidence.

---

## 16. Derived Execution Grant

**Working system**

A separate admission step verifies the intent against:

* current effective phase;
* admitted receipt frontier;
* validator evidence;
* authority graph;
* user authorization;
* scope constraints.

It then creates a short-lived `DerivedExecutionGrant`.

**Required proof**

* Missing receipt refuses.
* Stale phase refuses.
* Pending observations refuse.
* Wrong command digest refuses.
* Expired grant refuses.
* Reused grant refuses.
* Grant cannot change intent scope.

**Current standing:** `PARTIAL_ALIVE`

Grant construction exists (`scripts/grant-actuation.py`, unwired — see
Checkpoint 15). Live Claude execution remains unexercised.

---

## 17. Protected Actuation Execution

**Working system**

The exact protected operation is retried with the exact verified grant.

Examples:

* Git push;
* draft PR creation;
* merge;
* package publication;
* destructive filesystem operation;
* state-changing HTTP call.

**Required proof**

* Exact command succeeds with valid grant.
* Modified command refuses.
* Missing grant refuses.
* Expired grant refuses.
* Scope expansion refuses.
* The executor records actual exit status and effects.

**Current standing:** `UNKNOWN`

Not attempted in the 2026-07-29 pass — no execution pipeline exists to test (depends on Checkpoints 15/16 being wired first).

---

## 18. Execution Attestation

**Working system**

Actual execution produces an `ExecutionAttestation` binding:

* grant;
* executor identity;
* command digest;
* start and completion time;
* exit status;
* stdout/stderr commitments;
* resulting object identifiers;
* resulting repository state.

```text
grant ≠ execution
execution attestation = evidence of consequence
```

**Required proof**

A valid grant with no execution cannot produce an attestation.

A failed execution produces a failure attestation, not success.

**Current standing:** `UNSUPPORTED`

No attestation object type or executor exists yet. Unchanged from prior audit.

---

## 19. Receipt-Chain Replay

**Working system**

The complete chain can be replayed from genesis:

```text
observation
→ admission
→ allocation
→ planning
→ manufacture
→ validation
→ intent
→ grant
→ execution
→ attestation
```

The mutable phase snapshot is treated only as a cache.

**Required proof**

* Replay reconstructs the same state.
* Missing event refuses.
* Reordered event refuses.
* Forked predecessor refuses.
* Tampered payload refuses.
* Snapshot disagreement is detected.
* Rebuilding the cache produces the same phase vector.

**Current standing:** `PARTIAL_ALIVE`

2026-07-29 audit: `verify_receipt` on a real, session-bound plan envelope
returned `valid: true` with both `payload_digest` and `receipt` recomputing
exactly. The same envelope with only the `receipt` field zeroed returned
`payload_digest_valid: true, receipt_valid: false, valid: false` — tamper
detection confirmed on live (not fixture) data. Full cross-system replay
(observation → ... → attestation, the entire chain) still does not exist,
since the intent/grant/execution/attestation legs (15–18) are only
partially wired.

---

## 20. Closed Self-Hosting Loop

**Working system**

Ferroplan uses the Chatman ecosystem to modify Ferroplan itself:

```text
observe Ferroplan
→ allocate frontier
→ plan
→ manufacture in worktree
→ observe drift
→ validate
→ admit
→ audit
→ publish draft PR
→ attest execution
→ replay
```

No role collapses into another.

**Required proof**

One complete repository change traverses the loop without manual phase fabrication or unreceipted protected actuation.

**Current standing:** `PARTIAL_ALIVE`

2026-07-29 audit: this session ran the full observe → allocate → plan →
manufacture → observe-drift → validate → admit loop **twice**, end to end,
for two different repository mutations (a real `.claude/settings.json`
commit, and a no-diff Bash observation), each producing bound, verifiable
receipts and a `stable/receipted` phase vector with a 0-pending ledger.
This is the strongest evidence to date for this checkpoint's core claim.
Still missing to call it complete per the checkpoint's own diagram:
worktree-isolated manufacture (Checkpoint 11), draft-PR publication under a
structured intent/grant (Checkpoints 15–17), and execution attestation
(Checkpoint 18). The loop that exists is real; the loop as specified is not
yet whole.

---

## 21. v26.7.29 Crown

**Working system**

The exact release commit demonstrates the complete lawful Claude projection.

Required crown evidence:

1. Clean marketplace installation
2. Strict plugin validation
3. Agent authority refusals
4. Lifecycle candidate generation
5. Effective-phase collapse
6. Top-level CMCA allocation
7. Recursive CMCA allocation
8. Deterministic candidate plan
9. Isolated worktree manufacture
10. Projection regeneration
11. Independent VAL validation
12. Receipt binding
13. Tamper replay
14. Structured protected intent
15. Derived execution grant
16. Draft PR publication
17. Execution attestation
18. Full-chain replay

**Current standing:** `PARTIAL_ALIVE`

PR #2 (`agent/v26.7.29-claude-projection`) is the only draft attempting this
whole surface at once. As of the 2026-07-29 audit it is still `OPEN`/draft,
0 reviews, head commit `d88488608f41` (55 commits), with mixed CI: the
`Chatman Ecosystem` workflow's `projection-law` and `ferroplan-mcp` jobs
pass, but the plain `CI / test` job is `FAILURE`. Not touched further this
pass — recommend resolving the CI failure and getting the PR reviewable
before treating it as the crown vehicle.

---

# Recommended Release Sequence

The next bounded checkpoints should be completed in this order:

```text
1. Clean Claude installation
2. Live agent-authority refusal tests
3. Worktree manufacture
4. VAL integration
5. Recursive CMCA runtime
6. Full receipt replay
7. Intent/grant protected publication
8. Execution attestation
9. Closed self-hosting loop
10. v26.7.29 crown
```

The decisive rule is:

> **Do not build the crown directly. Make each checkpoint independently useful, independently falsifiable, and reusable by the next checkpoint.**

---

# Audit log

## 2026-07-29 — first full pass

Ran checkpoints 2, 3, 9, 13, 19 to real evidence (commands + output shown
inline above); confirmed existence/non-existence for 11 and 15–18 without
attempting new implementation. Upgraded: 2 (`UNKNOWN` → `PARTIAL_ALIVE`),
13 (`UNSUPPORTED` → `PARTIAL_ALIVE`), 11 (`UNKNOWN` → `UNSUPPORTED`, i.e.
sharpened, not upgraded). Sharpened without changing the label: 3, 9, 19.
Left untouched: 0, 1, 4, 5, 6, 7, 8, 10, 12, 14, 16, 17, 18, 20, 21 (either
re-confirmed from existing evidence or explicitly out of this pass's scope).

Concrete artifacts left behind by this pass:
- `benchmarks/.val/VAL/build/bin/Validate` — real vendored VAL binary
  (gitignored, not committed; rebuild with `sh benchmarks/get-val.sh
  -DCMAKE_POLICY_VERSION_MINIMUM=3.5` if the plain script fails to
  configure).
- This file (`docs/gall-checkpoints.md`), created for the first time —
  previously the checkpoint spec existed only in chat history and was at
  risk of being re-derived inconsistently each session.

Named next steps, not yet started: patch `get-val.sh`'s cmake invocation;
add `tools:` frontmatter to the 8 agents (Checkpoint 3); decide and
implement recursive CMCA's actual schema shape (Checkpoint 9); write a
worktree-manufacture script (Checkpoint 11); resolve PR #2's `CI / test`
failure or supersede it.

## 2026-07-29 — second pass (scheduled routine, evening run)

**Backlog discovered before picking new work.** Before touching anything,
checked `git branch -r` and GitHub: eight same-day sibling sessions had
already run against this file's first-pass audit and opened **eight open,
draft, unreviewed, unmerged pull requests** (#2 crown/v26.7.29,
#3 Checkpoint 2, #4 and #5 and #9 — three competing implementations of
Checkpoint 3's `tools:`/`disallowedTools`/bash-fence mechanism, #6
Checkpoint 13, #7 Checkpoint 11, #8 Checkpoint 9, #10 which itself
diagnosed and documented this exact backlog). None of the eight are
merged, so this file's copy on `main` (what every fresh session reads
first) has been stale relative to all of that work all day. PR #10's own
body already recommended #5 as the strongest Checkpoint 3 candidate but
declined to merge or close #4/#9 unilaterally, calling it "a maintainer
call" — every session that has looked at this same backlog today reached
the same conclusion independently. This pass does the same: it does not
merge, close, or supersede any of #2–#10, and does not open a fourth
competing branch against Checkpoint 3's already-contested 8 agent files.
This is now flagged to the user directly (outside this file) as a
governance problem this routine cannot resolve on its own — resolving it
needs a human to pick a Checkpoint 3 winner and start merging the backlog,
or the doc on `main` will keep drifting further behind reality with every
future run.

**One concrete update to a named open item**: PR #10's test plan left
"PR #2's `test` check run finishing green" as an open checkbox, in
progress at write time. Rechecked this pass via
`mcp__github__pull_request_read` (`get_check_runs`) on PR #2
(`agent/v26.7.29-claude-projection`, head `af865f8`): all three checks —
`projection-law`, `ferroplan-mcp`, and the plain `test` job — now show
`status: completed, conclusion: success`. PR #2 is CI-green end to end for
the first time since it was opened. Still draft, 0 reviews, not merged —
only the CI-red blocker is resolved, not the review/merge step.

**New work: closed all four of Checkpoint 8's named refusal-case gaps.**
No same-day sibling branch had touched Checkpoint 8 ("Top-Level CMCA
Allocation") — genuinely unclaimed territory, not a fifth collision. Added
6 new/reconfirmed `cargo test` cases against the real compiled
`ferroplan-mcp` binary over stdio (not fixtures): `cmca_allocate` refusing
7 candidates, refusing 9 candidates, refusing a wrong (9, not 10) factor
count, `verify_receipt` refusing a payload-content-tampered allocation
result (closing a gap the pre-existing `verify_rejects_tampered_digest`
test's own doc comment named as uncovered), and cross-process determinism
of `cmca_allocate` output. Reran the pre-existing
`bind_allocation_receipt_rejects_wrong_bcinr_revision` test to confirm the
fifth required-proof line is still covered rather than assuming stale
evidence still holds. Full receipts: `cargo test -p ferroplan-mcp --test
session_protocol` → 10/10 pass; `cargo test -p ferroplan-mcp --test
admission_protocol` → 16/16 pass; `cargo fmt --check` clean; `cargo clippy
-p ferroplan-mcp --all-targets --all-features -- -D warnings` clean.
Checkpoint 8 upgraded `PARTIAL_ALIVE` → `ALIVE` (test-surface scope,
explicitly not claimed beyond it — see the checkpoint's own standing text
for the exact boundary).

**Not attempted this pass**: reconciling PR #4/#5/#9 (explicitly deferred,
see above); anything on checkpoints 6, 7, 10, 12, 14, 16–18, 20, 21 (no
new evidence, standings unchanged); `cargo check --workspace` /
`cargo test --workspace` (PR #8's audit already found and reproduced a
pre-existing, unrelated `ferroplan-bevy`/`bevy@0.19.0` rustc-version gate
on this container — not re-verified this pass since it doesn't bear on
anything touched here; scoped `cargo test -p ferroplan-mcp -p ferroplan`
was run instead and is fully green).

Branch: `gall-checkpoints/2026-07-29-cmca-refusal-evidence`. PR opened
against `main` (see PR list) with this same evidence inline.
