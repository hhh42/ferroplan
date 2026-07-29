# Gall Checkpoints for the Chatman Ecosystem

Last updated: 2026-07-29 (second same-day session audit, see "Audit log" at
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

**Current standing:** `PARTIAL_ALIVE` (strengthened; was `PARTIAL_ALIVE` on
prompt-level-only evidence)

2026-07-29 first-pass audit findings (superseded evidence, kept for
history):
- None of the 8 agent `.md` files under `plugins/chatman-ecosystem/agents/`
  declared a `tools:` frontmatter field. Confirmed independently by that
  session's own Agent-tool listing, which annotated every one of the 8
  chatman-ecosystem agents with `(Tools: All tools)`. No mechanical denial
  existed at the Claude Code harness level.
- Live test: spawned `rdf-observer` and asked it to edit a throwaway file
  outside the repo. It refused — but by **choosing to honor its own role
  prose**, not because the harness blocked the `Edit` tool call. Had the
  model decided differently, the edit would have succeeded with no
  mechanical backstop.
- Conclusion at the time: role separation was **prompt-level compliance**,
  not **mechanical enforcement**.

2026-07-29 second-pass audit (this session, later same day) — implemented
the named next step:
- Added `tools:`/`disallowedTools:` frontmatter (and `isolation: worktree`
  for `source-manufacturer`) to all 8 agent `.md` files, adopting exactly
  the per-agent tool lists PR #2 had already worked out (the smallest slice
  of that PR's rewrite — not the rest of its body/effort/maxTurns changes).
- `claude plugin validate plugins/chatman-ecosystem` and `claude plugin
  validate .` (marketplace) both still `Validation passed with warnings`
  after the change — the only warning (`version: No version specified`) is
  pre-existing and reproduces identically on unmodified `main` (verified via
  `git stash`), so it is not a regression from this change.
- Live mechanical-refusal test (real nested `claude -p` session, plugin
  installed via `claude plugin marketplace add . && claude plugin install
  chatman-ecosystem@chatman-ecosystem`, `--permission-mode acceptEdits`):
  - `rdf-observer` instructed to `Write` to
    `/tmp/claude-scratch-rdf-observer-test.txt` → verbatim harness error
    `Error: No such tool available: Write. Write exists but is not enabled
    in this context.` File confirmed absent afterward (`ls` → No such file).
  - `cmca-allocator` instructed to run `Bash` (`echo hello > ...`) →
    verbatim harness error `Error: No such tool available: Bash. Bash
    exists but is not enabled in this context.` File confirmed absent.
  - `source-manufacturer` (which *does* have `Write` in its allow-list)
    instructed to `Write` to `/tmp/claude-scratch-manufacturer-test.txt` →
    got a *different* error class entirely: a permission-grant prompt
    (`Claude requested permissions to write to ..., but you haven't granted
    it yet.`), not `No such tool available`. This is exactly the expected
    contrast — the tool is registered for this agent (permission layer
    applies), whereas for `rdf-observer`/`cmca-allocator` the tool doesn't
    exist in their resolved set at all (harness layer applies, upstream of
    permissions). Confirms the allow-list, not just the deny-list, is being
    honored.
- **Gap found, not yet resolved**: the same `source-manufacturer` test
  reported `isolation: worktree` was *not* honored when the subagent was
  launched directly via `Task` in this test harness — "the agent was
  launched without `isolation: \"worktree\"`, so no isolated git worktree
  was created." Whether the frontmatter's `isolation:` field is actually
  wired to real subagent launches (vs. only to the top-level `Agent` tool's
  own `isolation` parameter) is unresolved. This affects Checkpoint 3's
  "Manufacturer runs in a worktree" sub-claim and overlaps Checkpoint 11 —
  do not use this pass's evidence to claim worktree isolation works.
- **Not yet re-tested this pass**: `config-law-architect`,
  `ecosystem-controller`, `ferroplan-planner`, `independent-validator`,
  `receipt-auditor` were given the same `tools:`/`disallowedTools:`
  treatment but were not individually live-refusal-tested (only
  `rdf-observer` and `cmca-allocator` were, plus the allow-list positive
  case on `source-manufacturer`). The frontmatter is byte-identical in
  shape to the two that were tested, so the mechanism should generalize,
  but that is an inference, not a receipt, for the untested five.
- `ecosystem-controller`'s `tools:` line uses the plain `Agent` tool name
  (not PR #2's `Agent(chatman-ecosystem:config-law-architect, ...)`
  restricted-subagent-list syntax) — that finer-grained syntax was not
  independently confirmed as currently supported, so the conservative form
  was used instead. Worth revisiting if per-agent Agent-tool scoping turns
  out to be real syntax.

**Next step**: live-refusal-test the remaining 5 agents
(`config-law-architect`, `ecosystem-controller`, `ferroplan-planner`,
`independent-validator`, `receipt-auditor`) the same way; resolve whether
`isolation: worktree` frontmatter is actually wired to subagent launches
(coordinate with Checkpoint 11); decide whether `ecosystem-controller`
should adopt the `Agent(...)` restricted-subagent-list syntax once its
support is confirmed.

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

**Current standing:** `PARTIAL_ALIVE`

The 8-candidate/10-factor happy path was exercised repeatedly this session
with real allocation receipts bound and admitted. The refusal cases (7/9
candidates, wrong factor count, wrong BCINR revision, tampered allocation
result) were **not** all individually re-verified in the 2026-07-29 pass —
only the receipt-tamper case (see Checkpoint 19) and CMCA's own
parent-index/cycle refusals (see Checkpoint 9) were.

**Next step**: run the four untested refusal cases explicitly and record
output here before upgrading past `PARTIAL_ALIVE`.

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

2026-07-29 second-pass audit (this session, later same day): patched
`benchmarks/get-val.sh` to pass `-DCMAKE_POLICY_VERSION_MINIMUM=3.5` by
default (was previously a manual workaround noted here but not applied to
the script). Verified live: `rm -rf benchmarks/.val && sh
benchmarks/get-val.sh` ran clean end-to-end from scratch on this container
(no prior cache) and produced `built:
/home/user/ferroplan/benchmarks/.val/VAL/build/bin/Validate` with no
manual flag needed. This closes half of the named next step.

**Next step**: add a `FERROPLAN_VAL` env-var check to whatever produces
`validator_result` payloads so VAL's output (when present) is what
actually gets bound into the receipt's `validator_result_digest` — this
part is still open.

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

## 2026-07-29 — second same-day pass (Recommended Release Sequence #2)

Picked up the named next steps from the first pass, in order: added
`tools:`/`disallowedTools:` frontmatter to all 8 agents (Checkpoint 3, item
2 in the Recommended Release Sequence), and patched `get-val.sh`'s cmake
invocation (part of Checkpoint 13, item 4 in the sequence). Did not attempt
item 1 (clean-cache install) or item 3 (worktree manufacture) this pass —
see below for why item 3 was touched only incidentally.

**Checkpoint 3 (Mechanical Agent Authority)** — real work, real evidence,
not a mock:
- Adopted PR #2's already-worked-out per-agent `tools:`/`disallowedTools:`
  lists (not the rest of that PR's rewrite) into the 8 agent `.md` files on
  `main`. Added `isolation: worktree` to `source-manufacturer` since that
  sub-claim belongs to this same checkpoint.
- Verified no regression: `claude plugin validate plugins/chatman-ecosystem`
  and `claude plugin validate .` both `Validation passed with warnings`
  before and after the change (`git stash` comparison) — the one warning
  (`version` unset) is pre-existing and unrelated.
- Installed the plugin for real in this container (`claude plugin
  marketplace add . && claude plugin install
  chatman-ecosystem@chatman-ecosystem`) and ran genuine nested `claude -p`
  sessions (`--permission-mode acceptEdits`, since `--dangerously-skip-permissions`
  is blocked for root in this container) to actually spawn subagents and
  attempt forbidden tool calls:
  - `rdf-observer` tried `Write` → harness error `No such tool available:
    Write`; target file confirmed absent afterward.
  - `cmca-allocator` tried `Bash` → harness error `No such tool available:
    Bash`; target file confirmed absent afterward.
  - `source-manufacturer` (which *is* allow-listed for `Write`) tried the
    same kind of write → got a *different*, permission-layer error
    (`Claude requested permissions to write to ..., but you haven't granted
    it yet.`), confirming the allow-list is real too, not just the deny-list.
- This is a genuine standing change from the first pass's finding ("prompt-
  level compliance... no mechanical backstop") to actual harness-level
  denial, independently confirmed by file-absence, for 2 of 8 agents plus
  one allow-list positive control. Kept the overall label at
  `PARTIAL_ALIVE` rather than `ALIVE` because 5 of 8 agents were not
  individually live-tested and the `isolation: worktree` field was found
  **not** to trigger worktree creation when the subagent was launched
  directly via `Task` in the same test — an honest gap, not glossed over.
  Full detail inline under Checkpoint 3 above.

**Checkpoint 13 (Independent PDDL Validation)** — closed half the named gap:
- Patched `benchmarks/get-val.sh` to pass
  `-DCMAKE_POLICY_VERSION_MINIMUM=3.5` by default.
- Verified live: `rm -rf benchmarks/.val && sh benchmarks/get-val.sh` built
  clean from nothing on this container, no manual flag required, producing
  `benchmarks/.val/VAL/build/bin/Validate`.
- Did not attempt the remaining half (`FERROPLAN_VAL` wiring into
  `validator_result`) this pass — named as the next step again.

**Blocked / deferred, not attempted this pass**: Recommended Release
Sequence item 1 (clean-cache install, Checkpoint 2) still needs an external
harness per the first pass's finding; not re-attempted. Item 3 (worktree
manufacture, Checkpoint 11) was not implemented, though this pass
incidentally surfaced a concrete new fact for it (`isolation: worktree`
frontmatter alone does not appear to trigger real worktree creation on
direct `Task` launch — whoever picks up Checkpoint 11 should verify this
rather than assume the frontmatter field is sufficient).

Concrete artifacts left behind by this pass:
- `plugins/chatman-ecosystem/agents/*.md` — all 8 now carry
  `tools:`/`disallowedTools:` (and `source-manufacturer` additionally
  `isolation: worktree`).
- `benchmarks/get-val.sh` — cmake policy flag now built in.
- This audit entry.

Named next steps, not yet started: live-refusal-test the remaining 5
agents for Checkpoint 3; resolve the `isolation: worktree` discrepancy
(coordinate with Checkpoint 11); wire `FERROPLAN_VAL` into
`validator_result` for Checkpoint 13; attempt a genuine clean-cache install
for Checkpoint 2; resolve PR #2's `CI / test` failure or supersede it.
