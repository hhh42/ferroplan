# Gall Checkpoints for the Chatman Ecosystem

Last updated: 2026-07-29 (session audit, see "Audit log" at the end).

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

> **2026-07-29 cycle update (CE-GALL-23).** Ceiling narrowed, standing survives.
> One of the declared invariants (`validated-plan-requires-candidate`) was
> **inert** — it carried `requires_any_prior`, a key `validate_vector` never
> reads — so "invariants reject illegal combinations" was partly vacuous. It is
> deleted; the lawful count is unchanged at 136, which is what proves it was
> doing nothing. Recurrence is blocked by
> `tests/test_phase_space.py::test_every_invariant_key_is_understood`.


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

> **2026-07-29 cycle update (CE-GALL-27).** The first bullet below is now
> **false**. `agents/*.md` frontmatter is generated from
> `ontology/authority-graph.ttl`, so all 8 agents declare `tools:` and the
> source-manufacturer declares `isolation: worktree`. The ODRL
> `SingleActuatorPolicy` is verified non-vacuous by
> `tests/test_authority.py::test_single_actuator_policy_is_enforced`: it permits
> exactly `source-manufacturer`, prohibits 7, and exactly `source-manufacturer`
> can write. **Standing does not move.** The live test below — whether the
> *harness* refuses or the *model* declines — has not been re-run against the
> generated frontmatter, so "mechanical, not prompt-level" is still asserted
> rather than measured. That single re-run is now the whole gap.

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

> **2026-07-29 cycle update (CE-GALL-28) — partial retraction.** The prior
> evidence that the 8×10 happy path was "exercised repeatedly with real
> receipts" was exercised over a **fabricated** frontier, including a surface
> that does not exist in the repository. That evidence is withdrawn. It is
> replaced by the canonical frontier from `profiles/work-surfaces.json`
> (`candidates_digest a473833974c74522`), accepted live and allocating
> *differently*. The four refusals below remain untested at the allocator:
> `surfaces.py`'s refusals are pre-flight and do not discharge them.


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

> **2026-07-29 cycle update (CE-GALL-30) — downgraded.** Standing is now
> `PARTIAL_ALIVE` with reason `MOCKED`. MCP `validate` returns the prose string
> `"Plan valid"`, while `bind_plan_receipt` requires a boolean `valid`, so the
> verdict is constructed by hand — `skills/admit/SKILL.md:15` instructs exactly
> that. The `validator_result` of every receipt bound during this cycle was
> hand-fabricated, so "independent" is currently false in the receipt path.


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

> **2026-07-29 cycle update (CE-GALL-31) — sharpened into a refutation.** The
> claim that chain forks are detected is not "not re-verified", it is
> **absent**. `verify_chain` does not exist and `previous_receipt` is
> format-checked only (64 hex, never looked up), so any well-formed hex string
> chains cleanly and `None` is indistinguishable from a break.


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

> **2026-07-29 cycle update.** Added evidence: a five-link chain
> (`755a2057 → c1520c61 → d56006af → eb8e4645 → d72f17f0`), the last four links
> bound over canonical CMCA inputs and `project-world.py`'s live projection.
> Added refutation: "a forked predecessor refuses" is **false** — see
> CE-GALL-31. Tamper detection on a single link stands.


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

> **2026-07-29 cycle update — net honest downgrade.** Strengthened: two further
> closes over canonical inputs and the live world projection, and
> `session_observe` → `session_think` returned `decision: follow`,
> `searched: false` — a suffix retained without a search is real evidence of a
> working persistent mind. **But** this checkpoint's required proof is a
> traversal "without manual phase fabrication", and both closes fabricated the
> validator verdict (CE-GALL-30) and were nine manual steps each because
> `loop.py close` is not built. The earlier claim that prior closes met this bar
> must be read with the same qualification.


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

# Checkpoints 22–33 — the DX architecture cycle

These were added by the 2026-07-29 architecture cycle (branch
`chatman-dx-cycle`). Every one is `PARTIAL_ALIVE` or lower and every one is
blocked on the same single hop: **no clean-worktree replay outside the
originating session has been done, and nothing is pushed.** Under the promotion
law that bars `ALIVE` regardless of how green the suite is, which is why
promotion here is one action rather than twelve.

The law is mechanized, not merely written down:
`plugins/chatman-ecosystem/tests/test_receipts.py` refuses any receipt claiming
`ALIVE` without `replayed_outside_session`, a non-null `negative_falsifier`, and
a sealed commit — and `test_promotion_law_actually_refuses` is that check's own
falsifier.

---

## Control Plane Executable Under Test (CE-GALL-22)

**Working system**

The Python control plane is a tested surface, and a test that would touch the
live ledger is refused rather than tolerated.

Before this the plugin had no tests and CI never touched `plugins/`: nine
scripts totalling ~2.5k lines were verified by a prose checklist that
`py_compile`d three of them.

**Current standing:** `PARTIAL_ALIVE` (`NO_FALSIFIER`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-22.json`

**Positive witness:** `tests (whole suite)` (plugins/chatman-ecosystem/tests) — the Python control plane went from zero tests and zero CI coverage to a suite gating every change

**Negative falsifier:** none. Recorded, not hidden — a checkpoint without an executing negative fixture cannot be promoted.

- Non-claim: the autouse isolation fixture is an assertion, not a falsifier: no test deliberately leaks, so it has never fired
- Non-claim: the CI `plugin` job has never run -- the branch is unpushed

---

## Derived Combination Census (CE-GALL-23)

**Working system**

An invariant that reads a key no evaluator consumes is not an invariant, and
the lawful-vector count must be *derived* from the invariant set rather than
asserted beside it.

`validated-plan-requires-candidate` carried `requires_any_prior`, a key
`validate_vector` never reads. The naive repair — renaming it to `requires_any`
— would have been wrong: `planning` is single-valued, so requiring
`planning=candidate` while `planning=validated` is unsatisfiable, and the rule
would have forbidden every validated vector. The transitions table already
enforces the intent exactly (`["candidate","validated"]` is the only in-edge),
so the invariant was redundant as well as inert.

**Current standing:** `PARTIAL_ALIVE` (`NO_REPLAY`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-23.json`

**Positive witness:** `test_lawful_count_is_pinned` (plugins/chatman-ecosystem/tests/test_phase_space.py) — 648 raw / 136 lawful / exactly 1 publishable, all derived rather than asserted beside the invariants

**Negative falsifier:** `test_every_invariant_fires_at_least_once` (plugins/chatman-ecosystem/tests/test_phase_space.py) — re-adding the deleted validated-plan-requires-candidate invariant (key requires_any_prior, never read by validate_vector) makes this fail. The lawful count staying at 136 after deletion is independent proof the invariant was inert

- Non-claim: nothing external validates that the 136 lawful vectors are the *right* 136

---

## Machine-First Output Contract (CE-GALL-24)

**Working system**

A payload's `schema` URN is the model's identity — stamped on construction and
rejected on mismatch — not a string a caller supplies. JSON is the default
serialization and does not depend on tty, so a command's contract is the same
whether a human, a hook, or CI invoked it.

**Current standing:** `PARTIAL_ALIVE` (`NO_REPLAY`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-24.json`

**Positive witness:** `test_emitted_payload_validates_against_its_committed_schema` (plugins/chatman-ecosystem/tests/test_generated.py) — what is emitted satisfies what is published, for every registered model

**Negative falsifier:** `test_check_detects_a_tampered_projection` (plugins/chatman-ecosystem/tests/test_generated.py) — proves generate.py build --check is not a no-op; verified by hand against a tampered schema, which exited 1

- Non-claim: 6 of roughly 30 emitted payloads are registered; the coverage ratio is measured nowhere and is left UNKNOWN

---

## Fail-Closed Hook Guard (CE-GALL-25)

**Working system**

Any exception raised before a hook handler runs becomes a refusal *shaped for
the event actually being handled* — never a traceback, and never a silent exit
0 on a deny path.

The shapes differ and getting them wrong turns a refusal into a no-op: `Stop`
takes a top-level `decision`, `PreToolUse` a nested `permissionDecision`, and
`PostToolUse` cannot refuse at all. The guard imports only the standard
library, because it is the last thing that must still work when the rest
cannot load.

**Current standing:** `PARTIAL_ALIVE` (`NO_REPLAY`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-25.json`

**Positive witness:** `test_guard_uses_only_the_standard_library` (plugins/chatman-ecosystem/tests/test_hookguard.py) — the last line of defence cannot itself fail on the dependency it is guarding against

**Negative falsifier:** `test_import_failure_produces_a_refusal` (plugins/chatman-ecosystem/tests/test_hookguard.py) — a simulated ImportError yields a refusal shaped for the event, never a traceback and never a silent exit 0 on a deny path

- Non-claim: no live Claude Code session has been observed honoring a hookguard refusal; runtime acceptance of the emitted shapes is UNKNOWN and is not fixable by more unit tests

---

## Resolution From Anywhere (CE-GALL-26)

**Working system**

The MCP server resolves its binary and its roots from an arbitrary working
directory with every steering variable cleared, preferring a binary already
built over a `cargo run` that rebuilds.

The prior resolver derived the project by walking four parents up from the
launcher. Under the repository layout that lands on the repo root and works;
under the *installed cache* layout — the only one a user runs — it lands on
`cache/<marketplace>`, which has no `crates/`, so the launcher exited 69 while a
built binary sat in `target/debug`. A depth-counted walk cannot be load-bearing
across two layouts.

**Current standing:** `PARTIAL_ALIVE` (`NO_FALSIFIER`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-26.json`

**Positive witness:** `MCP initialize handshake from /tmp` (plugins/chatman-ecosystem/scripts/run-ferroplan-mcp.sh) — previously exit 69 while a built binary sat in target/debug; the 4-parents-up walk was calibrated for the repo layout and wrong under the install layout

**Negative falsifier:** `test_unresolved_binary_is_never_rendered_as_a_shell_argv` (plugins/chatman-ecosystem/tests/test_roots.py) — an unresolved binary rendered as the empty string would hand a launcher `exec ""`; it now refuses

- Non-claim: the /tmp handshake was run by hand once this session and is NOT a test; no automated regression covers the exact defect that was fixed

---

## Canonical CMCA Frontier Grounded In Real Surfaces (CE-GALL-28)

**Working system**

The 8×10 frontier the allocator receives is derived from real repository
surfaces, and every declared surface path exists on disk. Arity is not
sufficiency: a well-formed frontier over fictional surfaces is a well-formed
lie.

This is deliberately a separate checkpoint from §8 rather than merged into it.
§8's four allocator refusals (7 candidates, 9 candidates, 9 factors, wrong
BCINR revision) remain untested; `surfaces.py`'s refusals are *pre-flight* and
must not be counted as allocator behaviour.

**Current standing:** `PARTIAL_ALIVE` (`NO_REPLAY`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-28.json`

**Positive witness:** `cmca_allocate over the canonical frontier` (plugins/chatman-ecosystem/profiles/work-surfaces.json) — accepted live, and allocates differently from the fabricated frontier: correctness 0.1449 top with a 0.112-0.145 spread, versus the invented 0.161 top on a surface that does not exist

**Negative falsifier:** `test_declared_surface_paths_exist_in_the_repository` (plugins/chatman-ecosystem/tests/test_surfaces.py) — found four surfaces pointing at nonexistent paths on its first run: crates/ferroplan/src/{temporal,search,heuristic,ground} are .rs files, and they sat on the two highest-allocated surfaces

- Non-claim: the ten factor VALUES are a modelling choice with no external validation; only their grounding is claimed
- Non-claim: surfaces.py refusals are pre-flight and must NOT be counted as allocator refusals -- checkpoint 8's four allocator refusals remain untested

---

## Standing Vocabulary Single Source (CE-GALL-29)

**Working system**

The standing vocabulary has one source — `ontology/chatman-ecosystem.ttl` —
and every consumer is a projection of it, checked by `generate.py build
--check`.

Three vocabularies existed: `loop.py` accepted four values, this document
listed seven, and the canonical set defined in `~/mfw` `AGENTS.md:122-133` has
six. `BLOCKED`, `MOCKED` and `REFUSED` could be claimed here but never recorded
in the ledger; `BUILD_BROKEN` could be recorded but not claimed. Until this
landed, **this checkpoint's own standing could not be written down.**

`MOCKED` and `REFUSED` are now reasons rather than standings. `MOCKED` is why a
standing is capped — a surface returning a fabricated value partly works, which
`PARTIAL_ALIVE` records and `MOCKED` would lose. `REFUSED` is a run outcome: a
lawful refusal is the system working, so as a standing it would conflate
evidence *for* promotion with brokenness.

**Current standing:** `PARTIAL_ALIVE` (`NO_REPLAY`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-29.json`

**Positive witness:** `test_ledger_cli_accepts_every_standing` (plugins/chatman-ecosystem/tests/test_standing.py) — loop.py went from four values to the canonical six, projected from the ontology

**Negative falsifier:** `test_loop_state_model_refuses_an_invented_standing` (plugins/chatman-ecosystem/tests/test_standing.py) — a seventh vocabulary cannot slip in through the model

- Non-claim: before this cycle, this checkpoint's own standing could not be recorded: loop.py accepted four values and BLOCKED was not among them

---

## Independent Validator Verdict (CE-GALL-30)

**Refuted claim**

MCP `validate` returns the prose string `"Plan valid"`. `bind_plan_receipt`
requires a `validator_result` carrying a boolean `valid`. The two do not
compose, so the verdict must be constructed by hand — and
`skills/admit/SKILL.md:15` instructs exactly that.

**The `validator_result` field of every receipt bound during the 2026-07-29
cycle was hand-fabricated.** The independence claim of both loop closes is
therefore false, and this is recorded rather than quietly carried.

**Current standing:** `PARTIAL_ALIVE` (`MOCKED`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-30.json`

**Negative falsifier:** none. Recorded, not hidden — a checkpoint without an executing negative fixture cannot be promoted.

- Non-claim: the validator_result field of EVERY receipt bound this session was hand-fabricated, so the independence claim of both closes is false

**Blocked by:** CE-GALL-31

---

## Receipt Chain Traversal (CE-GALL-31)

**Absent capability**

`verify_chain` does not exist. `previous_receipt` is validated by format only —
64 hexadecimal characters — and never looked up, so any well-formed hex string
is an acceptable predecessor and `None` is indistinguishable from a break.

The five-link chain produced this cycle
(`755a2057 → c1520c61 → d56006af → eb8e4645 → d72f17f0`) is evidence that
individual links *recompute*. It is zero evidence that the chain is a chain.
§14's claim that "chain forks are detected" is not untested — it is absent.

**Current standing:** `UNSUPPORTED` (`DEPENDENCY_MISSING`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-31.json`

**Negative falsifier:** none. Recorded, not hidden — a checkpoint without an executing negative fixture cannot be promoted.

- Non-claim: the 5-link chain 755a2057 -> c1520c61 -> d56006af -> eb8e4645 -> d72f17f0 is evidence that links recompute, and zero evidence that the chain is a chain

---

## Ledger Anchoring (CE-GALL-32)

**Open defect**

The ledger key is `sha256(realpath(cwd))[:24]`, so a command run from a
subdirectory silently creates a second ledger for the same repository. Four
exist today.

This demonstrated itself during the session that documented it: the `Stop` hook
blocked on 47 pending events in the `plugins/chatman-ecosystem` ledger while
the repository ledger read 0 pending. The fix — anchoring to the git toplevel
via `roots.project_root()` — is built but not wired into `loop.py`/`phase.py`,
so the fork recurs on the next `cd`.

**Blast radius corrected upward (2026-07-29).** The earlier text implied two
copies of `project_key`. There are **six**, and
`grep -rn 'def project_key' plugins/chatman-ecosystem/scripts/` names all of
them:

- `scripts/effective-phase.py:47`
- `scripts/phase.py:69`
- `scripts/grant-actuation.py:56`
- `scripts/actuation-intent.py:82`
- `scripts/event-summary.py:50`
- `scripts/loop.py:53`

`roots.project_root()` is wired into none of the six. This makes any per-copy
repair a partial fix by construction: the ledgers only reconverge when all six
agree, so five corrected copies leave the fork intact.

**Current standing:** `PARTIAL_ALIVE` (`NO_REPLAY`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-32.json`

**Negative falsifier:** `live demonstration during this session` (plugins/chatman-ecosystem/scripts/plugin_data.py) — the defect demonstrated itself in the session that documented it -- an unambiguous, reproducible negative

- Non-claim: four ledgers exist for one repository, keyed by whatever cwd a command ran from
- Non-claim: no test asserts the six copies agree, so the count above is a grep result and not a defended invariant

**Update (2026-07-29):** `roots.project_key`/`project_directory` now anchor at `roots.project_root()` (all six former copies already import from `roots.py` as of `6e9b81a`); verified `project_key('.') == project_key('plugins/chatman-ecosystem')` and added `test_project_key_is_identical_for_cwd_and_its_subdirectory` in `plugins/chatman-ecosystem/tests/test_roots.py` as positive witness. Standing raised to `PARTIAL_ALIVE` — partial because no test yet asserts the six *callers* observe one ledger end-to-end (only the shared `roots.py` primitive is covered).

---

## Admission Frontier TOCTOU (CE-GALL-33)

**Open defect**

`loop.py:368` sets `admitted_event_count = event_count` — a blanket watermark
that ignores the `observation_frontier` the envelope actually attests to. Any
mutation landing between binding an envelope and running `admit` is marked
admitted without ever appearing in a receipt.

Observed in this cycle's acceptance run: the envelope declared
`event_count: 142`; `admit` wrote `admitted_event_count: 143`.

The system's core claim is that state enters only through admitted
observations. This is the gap in that claim, and no test covers it.

**Citation corrected (2026-07-29).** This section previously cited
`loop.py:388`. The file has shifted; `:388` is now the plan-digest format
check. The current line, verified by
`grep -n 'admitted_event_count.*event_count' scripts/loop.py`, is **`:368`**.

**Claim ceiling: this is not a one-line fix.** `observation_frontier` has no
schema anywhere in this repository —
`grep -rn observation_frontier plugins/chatman-ecosystem/ | grep -v receipts/`
returns nothing. It is typed as a bare `Value` in the Rust binder, and no
producer in this repository constructs one. So "read the envelope's declared
frontier instead of the live count" has nothing to read: a frontier schema and
a producer must exist first. The falsifier is therefore recorded as absent with
reason `DEPENDENCY_MISSING` rather than as a prose observation, because no
executing fixture can be written against a type that does not exist.

**Current standing:** `PARTIAL_ALIVE` (`DEPENDENCY_MISSING`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-33.json`

**Negative falsifier:** none — `DEPENDENCY_MISSING`. The 142/143 discrepancy is
a real observation, but it is not a Gall-checkpoint negative fixture.

**Blocked by:** an `observation_frontier` schema, and a producer that constructs one

- Non-claim: no test covers this; the defect is recorded, not fixed
- Non-claim: nothing here shows the frontier-aware admission is designable, only that it is not yet buildable

---

## Canonical Bash Mutation Classifier (CE-GALL-34)

**Defect fixed this cycle** (commit `1a9ab50`)

Two defects in one surface, both closed by consolidating the classifier into
`scripts/bash_classify.py`.

*Divergence.* Three copies of `MUTATING_BASH` existed — `loop.py`, `phase.py`,
`event-summary.py` — and disagreed. `phase.py` omitted the publication class, so
`git push` logged a ledger event but never collapsed the phase vector: the
ledger and the phase engine held different beliefs about the same command.

*Prefix matching.* No git subcommand alternation carried a trailing boundary, so
prefixes matched. This produced a real incident during this session:
`git merge-base --is-ancestor` and `git branch --show-current` are read-only,
matched `PROTECTED_BASH`, and blocked a legitimate push. `rm\b` was the only
branch with a correct boundary — evidence the omission was an oversight rather
than a design choice.

**The nuance that separates the fix from a near-miss.** `\b` alone is
insufficient. `-` is a non-word character, so `commit\b` still matches
`commit-graph`, and a `\b`-only patch would have kept misclassifying
`git commit-graph verify` while looking correct. The fix uses `(?![\w-])`.

**Current standing:** `PARTIAL_ALIVE` (`NO_REPLAY`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-34.json`

**Positive witness:** `test_phase_agrees_with_loop_on_publication_class`
(plugins/chatman-ecosystem/tests/test_bash_classify.py:91) — pins the divergence
itself rather than one copy's behaviour.

**Negative falsifier:** `test_protected_boundary`
(plugins/chatman-ecosystem/tests/test_bash_classify.py:102) — asserts the exact
read-only commands from the incident are not protected while `git push origin
main` and `git reset --hard` are. Removing the trailing boundary fails it;
weakening it to `\b` still fails the `commit-graph` case in the sibling table.

- Non-claim: the fix is not replayed outside this session, so it is capped at `PARTIAL_ALIVE` under the promotion law regardless of the suite being green
- Non-claim: nothing mechanically forbids a fourth copy of the classifier being reintroduced elsewhere; single-sourcing is a convention here, not an invariant

---

## Session Lifecycle Bookends (CE-GALL-35)

**Working system.** `session_open`, `session_status`, and `session_close` had
no dedicated checkpoint coverage before this entry — they were only exercised
as steps inside `session_protocol.rs`'s longer happy-path chain
(`session_open` → `session_observe` → `session_set_goal` → `session_think` →
`session_advance` → `session_status` → `session_close`) and inside a separate
"never-opened session" refusal test. Nothing pinned the three bookend tools as
a surface of their own: does `session_open` ground state that `session_status`
actually reflects, and does `session_close` leave the session in a state where
reuse fails lawfully rather than silently?

A new test file, `crates/ferroplan-mcp/tests/session_lifecycle_bookends.rs`,
drives the built `ferroplan-mcp` binary over stdio (same harness pattern as
`session_protocol.rs`) for exactly that: open a session against a small valid
STRIPS domain+problem, check `session_status` echoes the grounded
`session_id`/`domain_digest`/`problem_digest`/`goal_met`/`cursor`, close it,
then probe reuse of the closed `session_id`.

**Open defect / correction to the original plan.** The test was drafted
assuming `session_status` exposes a `goal` field and that a second
`session_close` on an already-closed session refuses with `unknown session`
like `session_status`/`session_advance`/`session_observe` do. Both assumptions
were wrong, found by running the test against the live server rather than by
review:

- `session_status`'s real schema
  (`urn:chatman:ferroplan-session-status:v1`) has no `goal` field. It reports
  `cursor`, `epoch`, `goal_met`, `domain_digest`, `problem_digest`,
  `plan_length`, `remaining_plan_valid`, `receipt_chain_head`;
- a second `session_close` on an already-closed `session_id` is **not** a
  tool-level error. It returns `isError: false` with `closed: false` — an
  idempotent no-op — while `session_status` on that same closed id returns
  `isError: true` containing `unknown session`. The three tools do not
  converge on one failure mode for post-close reuse: `session_close`
  degrades gracefully where `session_status` refuses.

The test's assertions were corrected to match the observed behavior rather
than the guessed one.

**Current standing:** `PARTIAL_ALIVE` (`NO_REPLAY`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-35.json`

**Positive witness:** `open_status_close_bookends_agree_on_session_state`
(crates/ferroplan-mcp/tests/session_lifecycle_bookends.rs:97) — opens a
session, asserts `session_status` reflects the exact digests and goal state
`session_open` grounded, then closes it and asserts `closed: true`.

**Negative falsifier:** `session_status_after_close_refuses_lawfully`
(crates/ferroplan-mcp/tests/session_lifecycle_bookends.rs:191) — after
`session_close`, calls `session_status` on the same session_id and asserts a
lawful `isError: true` / `unknown session` refusal (not a crash or stale
success), then calls `session_close` a second time and asserts the real
idempotent `closed: false` response instead of a fabricated second refusal.

- Non-claim: not replayed outside this session, so it is capped at
  `PARTIAL_ALIVE` under the promotion law regardless of both tests passing
- Non-claim: only `session_status` and `session_close` reuse-after-close paths
  were probed; `session_open` with `replace: false` reusing a closed
  session_id (a fresh open, since close removes the id from the live map
  entirely) was not exercised and is not claimed here
- Non-claim: no concurrency scenario (close racing another call on the same
  session_id) is covered by these two tests; `session_protocol.rs` has a
  separate, unrelated concurrency test for a different bug

---

## Goal Retarget and Cursor Advance (CE-GALL-36)

**Working system**

`session_set_goal` and `session_advance` had no existing SKILL.md or
Gall-checkpoint coverage before this cycle, despite both being wired into
`full_session_lifecycle` in `session_protocol.rs` as a happy-path step. This
checkpoint adds dedicated positive and negative witnesses that exercise each
tool's real behavior rather than its presence in a broader lifecycle test.

`session_set_goal` retargets a live `Session` to a new ground conjunction
over the already-interned fact space (`crates/ferroplan/src/session.rs:929`)
— no regrounding, no re-parse of the world. `session_advance` moves the
session's execution cursor forward over a stored plan
(`crates/ferroplan-mcp/src/session.rs:596`), refusing any advance that would
push the cursor past the plan's actual length.

**Current standing:** `PARTIAL_ALIVE` (`NO_REPLAY`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-36.json`

**Positive witness:** `set_goal_retargets_and_advance_moves_cursor_on_a_real_plan`
(crates/ferroplan-mcp/tests/session_goal_advance.rs:139) — opens a session
against a three-action sequential domain, plans the original goal `(s)` (a
real 3-step plan), retargets mid-session to a different ground conjunction
`(q)`, confirms via `session_status` that the cursor reset and the epoch
advanced, then confirms via a fresh `session_think` that the retarget
actually changed what gets planned — the new plan for `(q)` is 1 step, not
3, a real plan-shape change rather than a status flag flipping. A final
`session_advance` with `completed_steps=1` over that real plan is confirmed
via `session_status`.

**Negative falsifier:** `advance_beyond_plan_length_is_refused_and_cursor_is_unchanged`
(crates/ferroplan-mcp/tests/session_goal_advance.rs:290) — plans a real
3-step sequence, then calls `session_advance` with `completed_steps` 1000
past the plan's actual length. The tool genuinely refuses (a tool-level
`isError` naming the plan-length bound, per `do_session_advance`'s
`next > plan_length` guard) — this is the TRUE observed behavior, checked by
running the call rather than assumed. `session_status` confirms the rejected
call left the cursor at 0, and a following in-range advance still succeeds.

- Non-claim: the fix is not replayed outside this session, so it is capped
  at `PARTIAL_ALIVE` under the promotion law regardless of both tests being
  green
- Non-claim: `session_set_goal`'s own negative path (a malformed or
  unreachable-atom goal) was not exercised as a second falsifier here — only
  `session_advance`'s out-of-range `completed_steps` was run negative;
  `crates/ferroplan/src/session.rs`'s `set_goal_rejects_unknown_and_adl` unit
  test covers that path at the library layer, but no MCP-tool-level witness
  for it exists yet under this checkpoint

---

## Structured Validate Verdict (CE-GALL-38)

**Defect fixed this cycle (re-witnessed, not authored by this checkpoint)**

CE-GALL-30 recorded that MCP `validate` returned the prose string `"Plan
valid"`, incompatible with `bind_plan_receipt`'s boolean `valid` requirement,
forcing the hand-fabrication instructed at `skills/admit/SKILL.md:15`. This
checkpoint re-ran that exact claim against the live tool at the current
commit rather than trusting the old doc.

Two direct calls to `mcp__plugin_chatman-ecosystem_ferroplan__validate`
against a trivial 1-action STRIPS domain (`(at-a)` -> `(at-b)`):

* Valid plan (`step 1: (move)`) ->
  `{"reason":null,"schema":"urn:ferroplan:plan-validation:v1","valid":true}`
* Invalid plan (`step 1: (nonexistent-action)`) ->
  `{"reason":"plan action \`NONEXISTENT-ACTION \` not a grounded op","schema":"urn:ferroplan:plan-validation:v1","valid":false}`

Both are structured JSON objects with a native boolean `valid` field and a
`urn:ferroplan:plan-validation:v1` schema tag — not prose. **CE-GALL-30's
refuted claim does not reproduce at this commit**: the composition gap it
named (prose in, bool required by `bind_plan_receipt`) is closed for the
tool's raw output shape. This upgrades the *mechanical* half of CE-GALL-30's
finding; CE-GALL-30's own section is left untouched as the historical record
of when and why the gap was first recorded.

**What this checkpoint does not claim.** `skills/admit/SKILL.md:15` still
reads as a manual instruction ("independent validator result containing
`valid: true`") rather than "pass `validate`'s own `valid` field through" —
the callers were not audited or changed by this checkpoint, only the raw
tool response shape was re-verified. CE-GALL-13/CE-GALL-30's separate,
still-open concern about genuine engine independence (Ferroplan's `validate`
validating a Ferroplan-produced plan vs. an external validator like VAL) is
untouched — a structured verdict removes the prose/bool composition problem,
it does not by itself restore independence.

**Current standing:** `PARTIAL_ALIVE` (`NO_REPLAY`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-38.json`

**Positive witness:** `test_valid_plan_yields_boolean_true_directly_usable_by_bind_plan_receipt`
(plugins/chatman-ecosystem/tests/test_validate_verdict.py:64) — pins the raw
JSON captured from a live `validate` call on a valid plan and asserts the
`valid` field is a native bool directly usable in `validator_result`, with
no prose parsing step.

**Negative falsifier:** `test_invalid_plan_yields_boolean_false_with_reason_no_hand_fabrication`
(plugins/chatman-ecosystem/tests/test_validate_verdict.py:73) — pins the raw
JSON from a live `validate` call with a plan referencing a non-grounded
action against the same domain/problem, and asserts the tool reports
`valid: false` with a `reason`, again without string coercion.

- Non-claim: not replayed outside this session, so capped below `ALIVE` regardless of the suite being green
- Non-claim: the callers (`skills/admit/SKILL.md`, any script building `validator_result`) were not changed or audited for whether they still hand-fabricate instead of reading the field through — only the tool's own response shape was re-verified
- Non-claim: this does not touch or resolve CE-GALL-13/CE-GALL-30's separate open question of genuine engine-independent validation (VAL vs. Ferroplan's own `validate`)

---

## True Recursive CMCA Descent (CE-GALL-37)

**Supersedes/extends Checkpoint 9's cross-call-descent gap**

Checkpoint 9's 2026-07-29 audit found that `bind_allocation_receipt`'s only chaining
field is a flat `previous_receipt`, and concluded that "true cross-call recursive
descent" was "architecturally absent from the MCP tool schema." That conclusion was
correct about `bind_allocation_receipt` specifically, but did not account for
`cmca_allocate_recursive` — a separate MCP tool (`crates/ferroplan-mcp/src/session.rs`,
`tool_cmca_allocate_recursive`) that already implements exactly the shape Checkpoint 9's
"Working system" diagram describes: a `root` frontier of eight admitted candidates,
followed by zero or more `descents`, each naming a `selected_parent_node` id drawn from
the immediately preceding depth's own admitted frontier and supplying a fresh local
eight-candidate frontier. Each depth's payload carries `parent_payload_digest`, which is
asserted (not merely declared) to equal the previous depth's real
`allocation_payload_digest`.

This checkpoint re-verified that tool directly, both by driving it live via
`mcp__plugin_chatman-ecosystem_ferroplan__cmca_allocate_recursive` and by running its
existing and one new Rust integration test.

**Required proof**

* Depth one allocation. Confirmed (existing test
  `cmca_recursive_depth_one_matches_plain_cmca_allocate`, and live).
* Depth two allocation with a real bound parent digest. Confirmed (existing test
  `cmca_recursive_depth_two_binds_the_real_parent_digest`, and live).
* Depth three allocation, chained through depth two's own digest (not depth one's).
  Confirmed by a new test added this session,
  `cmca_recursive_three_depth_chain_binds_digests_all_the_way_down`, and by a live
  three-depth call in this session.
* Cyclic ancestry refusal across non-adjacent depths. Confirmed (existing test
  `cmca_recursive_refuses_cyclic_ancestry`, and live).
* Unknown-parent-node refusal. Confirmed (existing test
  `cmca_recursive_refuses_an_unknown_selected_parent_node`, and live).
* Deterministic replay. Confirmed (existing test
  `cmca_recursive_is_deterministic_across_repeated_calls`).
* Malformed-depth refusal collapses the whole chain, no partial result. Confirmed
  (existing test `cmca_recursive_refuses_the_whole_chain_on_a_bad_depth`).

**What is still open, named rather than omitted.** Checkpoint 9's "parent receipt
mismatch refusal" and "missing return consequence refusal" items are not modeled by
`cmca_allocate_recursive`: there is no mechanism for a child depth's result to be
rejected or re-consumed by its parent depth after the fact, and `bind_allocation_receipt`
still has only a flat `previous_receipt` field — the gap Checkpoint 9 identified in that
specific tool is unchanged. What closes here is narrower and precise: `cmca_allocate_recursive`
is a real, tested, live-confirmed cross-call recursive descent tool, distinct from both
the in-array `parent`-index tree Checkpoint 9's earlier audit exercised and from the
receipt-binding surface Checkpoint 9's gap language was really pointed at.

**Live tool calls made this session** (not just tests):

1. Positive: a real three-depth `cmca_allocate_recursive` call (root `root-0..7`, depth
   two `d2-0..7` selecting `root-0`, depth three `d3-0..7` selecting `d2-0`) succeeded.
   Depth 2's `parent_payload_digest` (`da48260...`) equalled depth 1's
   `allocation_payload_digest` exactly; depth 3's `parent_payload_digest`
   (`a26836f...`) equalled depth 2's exactly.
2. Negative — cyclic ancestry: reusing root id `r0` as both the entry to depth two and
   (with `r0` also listed among depth two's own admitted candidates) the entry to depth
   three produced `depth 3: cyclic ancestry -- \`r0\` already selected earlier in this
   descent chain`. Note: a first attempt at this falsifier, where `r0` was reused as
   depth-three's selector *without* `r0` being present among depth two's own candidate
   ids, produced the sibling **unknown-parent** refusal instead
   (`was not an admitted candidate id at depth 2`) — a genuine, informative near-miss,
   not the cyclic-ancestry path, until the id was made a legitimately admitted depth-two
   candidate.
3. Negative — unknown parent: a depth-two descent naming `node-never-admitted` as
   `selected_parent_node` (never present in the root frontier) produced
   `depth 2: selected_parent_node \`node-never-admitted\` was not an admitted candidate
   id at depth 1`.

**Current standing:** `PARTIAL_ALIVE` (`NO_REPLAY`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-37.json`

**Positive witness:** `cmca_recursive_three_depth_chain_binds_digests_all_the_way_down`
(crates/ferroplan-mcp/tests/session_protocol.rs:590) — a new test asserting a genuine
three-depth cross-call chain binds each depth's `parent_payload_digest` to the real,
distinct digest of the depth immediately before it.

**Negative falsifier:** `cmca_recursive_refuses_cyclic_ancestry`
(crates/ferroplan-mcp/tests/session_protocol.rs:540) — asserts a depth-three descent
re-selecting an id already used to enter depth two is refused, confirmed live with the
exact error text above.

- Non-claim: not replayed outside this session, so capped below `ALIVE` under the
  promotion law regardless of the suite being green
- Non-claim: does not resolve `bind_allocation_receipt`'s flat-chaining gap named in
  Checkpoint 9 — that remains open and is not touched here
- Non-claim: "return consequence" / parent-side re-validation of a child depth's result
  is still unmodeled; `cmca_allocate_recursive` only builds and chains downward

---

## Receipt Chain Fork Detection (CE-GALL-39)

**Open defect** — tests a gap named in Checkpoint CE-GALL-31

CE-GALL-31 recorded that chain-fork detection is absent: `verify_chain` does
not exist, and `previous_receipt` is format-checked only (64 hex, never
looked up). This checkpoint constructs the fork for real against the running
`ferroplan-mcp` server and checks whether anything in this repository catches
it.

**What was built**

1. A trivial one-action PDDL domain/problem was solved and validated.
2. `cmca_allocate` / `bind_allocation_receipt` produced a real allocation
   receipt over eight candidates.
3. `session_open` + `session_think` on the trivial domain produced a real
   plan.
4. `bind_plan_receipt` bound a root envelope **A** (`previous_receipt: null`,
   receipt `2cc3d1a6...`).
5. `bind_plan_receipt` was called **twice more**, each time with
   `previous_receipt` set to A's receipt but a different
   `observation_frontier` payload, producing two divergent children **B1**
   (receipt `c50bec8e...`) and **B2** (receipt `c7d4829e...`) that both claim
   A as their predecessor — a genuine fork, not a synthesized one.
6. `verify_receipt` was called on B1 and B2 **individually**.

**Observed result**

Both calls returned `valid: true`. Each envelope is fully self-consistent —
its own payload digest and receipt recompute, and its declared
`previous_receipt` is a well-formed hex string — and neither call has any way
to learn a sibling exists. `verify_receipt`'s contract has no field for
"does another receipt already claim this predecessor." The fork is silently
accepted by every check this repository's MCP surface exposes.

A corpus scan (mirroring CE-GALL-31's own falsifying command,
`grep -rn verify_chain crates/ plugins/`, plus a walk of every script under
`scripts/`) found no chain-walking or branch-detection capability anywhere.
The only place "fork" is mentioned outside CE-GALL-31/34's own prose is
`agents/receipt-auditor.md`, which instructs an LLM to "reject ... forked
heads unless the fork is explicitly admitted" — but that is a markdown
prompt, not an invocable script or MCP tool, and it was not exercised against
B1/B2 in this checkpoint.

**Required proof**

* A fork (two receipts declaring the same predecessor) can be constructed
  against the live tools, not just described.
* `verify_receipt` (or any other capability in this repository) detects it.

**Current standing:** `UNSUPPORTED` (`DEFECT_OPEN`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-39.json`

**Negative falsifier:**
`test_verify_receipt_accepts_both_branches_of_the_fork_individually`
(plugins/chatman-ecosystem/tests/test_fork_detection.py:97) — pins the exact
live-tool outputs for B1 and B2 above and asserts both verify as `valid:
true` with no sibling/fork signal in either result. This is the checkpoint's
falsifier: the fork is real, constructed, and undetected.

**Blocked by:** a `verify_chain`-equivalent tool or script that looks up
whether a claimed predecessor already has another admitted child

- Non-claim: this does not build fork detection — it demonstrates its
  absence with an executing, non-mocked fixture
- Non-claim: `agents/receipt-auditor.md` names fork rejection as an
  intended behavior for an LLM auditor; this checkpoint does not evaluate
  whether an LLM following that prompt would catch B1/B2, only that no
  mechanical tool does

---

## Full 17-Tool Dogfood Chain (CE-GALL-40)

**Capstone over CE-GALL-35..39**

A prior audit found no test anywhere exercised all 17 `ferroplan-mcp` tools
in one continuous chained flow — only overlapping subsets, spread across
`session_protocol.rs`, `session_lifecycle_bookends.rs`,
`session_goal_advance.rs`, and the Python fork/validate fixtures. This
checkpoint answers "does the ecosystem actually dogfood every
`ferroplan-mcp` tool" with a receipt, not a guess.

**Working system**

A small two-action STRIPS domain (`at-a -> at-b -> at-c`) drives, in one
continuous JSON-RPC session over stdio against the built `ferroplan-mcp`
binary:

```text
parse (domain) -> parse (problem) -> solve
-> session_open -> session_observe -> session_set_goal -> session_think
-> session_advance -> cmca_allocate -> cmca_allocate_recursive
-> canonical_digest -> bind_allocation_receipt -> validate
-> bind_plan_receipt -> verify_receipt -> session_status -> session_close
```

16 of the 17 tools are touched this way — `decompose` was deliberately not
called and is named as a gap below, not silently skipped.
`canonical_digest` is called directly on a session_think-derived value, not
just implicitly inside `bind_*`. `cmca_allocate_recursive` is exercised with
a real root-plus-one-descent (2 depths), asserting the descent's
`parent_payload_digest` equals the root's real `allocation_payload_digest`.
`bind_plan_receipt`'s `previous_receipt` is deliberately left `null` — the
plan envelope and the allocation envelope are siblings referencing a shared
session via the `allocation_receipt` field, not predecessor/successor plan
envelopes; chaining `previous_receipt` to the allocation receipt would
overclaim a plan-envelope lineage that does not exist yet, since this is the
only plan envelope bound in this session.

The same trace was first run live via direct
`mcp__plugin_chatman-ecosystem_ferroplan__*` tool calls in the authoring
session, then formalized as a re-runnable Rust fixture driving the binary
over stdio — the more faithful "one continuous flow" transport, matching the
existing `session_*` test files' harness pattern. That live run surfaced a
genuine finding, not a guess later encoded as an assertion:
`bind_allocation_receipt` refuses `cmca_allocate_recursive`'s raw `depths`
payload (`allocation_result lacks payload.bcinr_revision`,
`crates/ferroplan-mcp/src/admission.rs:188-190`) because its schema expects
the flat shape `cmca_allocate` itself returns. The chain works around this
by binding the recursive result's root-depth payload (which does carry
`bcinr_revision`) with the depth-2 chain folded in under an explicit
`recursive_extension` field, so the descent is still recorded in the bound
receipt rather than silently dropped — but `bind_allocation_receipt` itself
has no native field for a multi-depth recursive allocation, the same
architectural gap CE-GALL-9 named for the flat `previous_receipt` field.

**Current standing:** `PARTIAL_ALIVE` (`NO_REPLAY`)

**Receipt:** `plugins/chatman-ecosystem/receipts/CE-GALL-40.json`

**Positive witness:** `full_seventeen_tool_dogfood_chain`
(crates/ferroplan-mcp/tests/dogfood_chain.rs:136) — one spawned
`ferroplan-mcp` process, one continuous stdio transport, 16 tools called in
order, with real assertions on each response's shape (not just
`isError: false`): `solve`'s 2-step plan, `session_think`'s retargeted
1-step plan, `cmca_allocate_recursive`'s digest chaining across the descent,
`verify_receipt`'s `valid: true`, `validate`'s structured
`urn:ferroplan:plan-validation:v1` shape. Run:
`cargo test -p ferroplan-mcp --test dogfood_chain -- --nocapture` — 2
passed, 0 failed.

**Negative falsifier:** `session_status_after_close_refuses_unknown_session`
(crates/ferroplan-mcp/tests/dogfood_chain.rs:543) — opens a session, closes
it, then calls `session_status` on the same session_id inside the same live
process, and asserts a lawful `isError: true` refusal containing
`unknown session`, not a stale success or a crash. This reproduces
CE-GALL-35's already-documented close-then-status refusal mechanism,
re-exercised here as this checkpoint's own executing falsifier rather than
assumed from that prior checkpoint.

- Non-claim: not replayed outside this session, so capped at
  `PARTIAL_ALIVE` under the promotion law regardless of the test being green
- Non-claim: `decompose` was not called — this checkpoint does not cover it,
  and it is the one tool in the server's surface with zero coverage from
  this dogfood chain
- Non-claim: `bind_allocation_receipt` has no native field for a
  `cmca_allocate_recursive` result; the `recursive_extension` workaround
  records the descent but is not a schema-level capability
- Non-claim: the negative falsifier only exercises the
  session_status-after-close path; a tampered-receipt or
  `validator_result: false` falsifier was not additionally run — one
  falsifier was chosen and executed for real rather than several run
  shallowly
- Non-claim: `verify_receipt`'s scope is unchanged — as CE-GALL-39 already
  established, it only checks an envelope's own self-consistency and has no
  chain-fork or sibling-detection capability; using it here on the plan
  envelope does not imply otherwise
- Non-claim: `session_advance` moving the cursor does not itself apply
  action effects to the session's world state — `goal_met` stayed `false`
  after advancing past the retargeted 1-step plan; only `session_observe`
  admits new facts, consistent with CE-GALL-36

---

# Audit log

## 2026-07-29 — CE-GALL-39, receipt chain fork detection

Tested the gap CE-GALL-31 named: `verify_chain` does not exist, so nothing
checks whether two different receipts claim the same predecessor. Built the
fork for real against the live `ferroplan-mcp` server — `solve`/`validate`
on a trivial one-action domain, `cmca_allocate`/`bind_allocation_receipt`
over eight candidates, `session_open`/`session_think`, then `bind_plan_receipt`
three times: root A, then two children B1 and B2 that both declare A as
`previous_receipt` with different `observation_frontier` payloads. Called
`verify_receipt` on B1 and B2 independently; both returned `valid: true`.
Neither result carries any signal that a sibling exists — `verify_receipt`
recomputes digests and checks the declared predecessor is well-formed hex,
nothing more. A corpus scan (`grep -rn verify_chain crates/ plugins/`, plus a
walk of every script under `scripts/`) found no chain-walking or
branch-detection capability; the only "fork" mention outside CE-GALL-31/34's
own prose is `agents/receipt-auditor.md`, a markdown prompt for an LLM
auditor, not an invocable tool. Recorded as `UNSUPPORTED` / `DEFECT_OPEN`
with an executing negative falsifier
(`plugins/chatman-ecosystem/tests/test_fork_detection.py`, 4/4 passing) that
pins the exact live-tool receipts and verification results rather than
describing the gap in prose. `blocked_by` names the missing `verify_chain`
tool. Full suite re-run after the addition: 373 passed, zero regressions.

## 2026-07-29 — CE-GALL-38, re-witnessing CE-GALL-30's validate claim

Called the live `mcp__plugin_chatman-ecosystem_ferroplan__validate` tool
directly (not from old docs) with a trivial 1-action STRIPS domain, once for
a valid plan and once for an invalid one (nonexistent grounded action).
Both raw responses were structured JSON
(`{"reason":..., "schema":"urn:ferroplan:plan-validation:v1", "valid":bool}`),
not the prose string `"Plan valid"` CE-GALL-30 recorded. Wrote and ran
`plugins/chatman-ecosystem/tests/test_validate_verdict.py` (4 tests, all
passed, real pytest run, no mocking of the response shape — the fixtures are
the literal captured tool output) pinning both the valid and invalid raw
responses and asserting the `valid` field is a native bool usable directly
in `bind_plan_receipt`'s `validator_result`. Bound
`plugins/chatman-ecosystem/receipts/CE-GALL-38.json`,
`PARTIAL_ALIVE (NO_REPLAY)`. Left CE-GALL-30's own section untouched as
historical record; CE-GALL-38 upgrades the mechanical (prose-vs-bool)
finding while explicitly not claiming the callers were audited or that
engine independence (CE-GALL-13's VAL question) is resolved.

## 2026-07-29 — parallel-agent iteration (branch `chatman-dx-cycle`)

Three agents worked in parallel on disjoint file sets. Two feature commits
landed: `63a8a70` (Rust) and `1a9ab50` (canonical Bash classification). The
suite went from 251 to 308 tests. This entry is the receipt-and-document pass
over that work.

**Corrections to existing receipts**, recorded because a stale receipt is worse
than a missing one — it is evidence pointing at the wrong line:

- CE-GALL-33 cited `loop.py:388` for the admission TOCTOU. The file has shifted
  and `:388` is now the plan-digest format check; the true line is `:368`. A
  reader following the old citation would have audited an unrelated check and
  found nothing wrong;
- CE-GALL-33 also gained an explicit **claim ceiling**. It was written as though
  a one-line fix would close it. It cannot: `observation_frontier` has no schema
  anywhere in this repository, is a bare `Value` in the Rust binder, and has no
  producer. The falsifier moved from a prose observation to declared-absent with
  reason `DEPENDENCY_MISSING`, and `blocked_by` now names the two artifacts that
  must exist first;
- CE-GALL-32 **understated its blast radius**. The receipt implied two copies of
  `project_key`; the grep shows six (`effective-phase.py:47`, `phase.py:69`,
  `grant-actuation.py:56`, `actuation-intent.py:82`, `event-summary.py:50`,
  `loop.py:53`). This changes the shape of the defect, not just its size: with
  six copies, any per-copy repair is a partial fix by construction.

CE-GALL-34 opened for the `MUTATING_BASH` prefix/divergence defect, fixed by
`1a9ab50`, with an executing falsifier — `PARTIAL_ALIVE` / `NO_REPLAY`, because
the promotion law's boundary is the session and nothing here has been replayed
outside it.

**The most interesting result of the iteration was that the implementing agents
corrected the brief they were given.** Both corrections were found by building,
not by reviewing, and neither was in the plan:

- the empty-plan case was specified as *parseable but trivially satisfied*.
  Measured, it is **unparseable** — a different failure at a different layer,
  and the test written to the brief would have asserted the wrong thing;
- the Bash boundary fix was specified as adding `\b`. That is **insufficient**:
  `-` is a non-word character, so `commit\b` still matches `commit-graph`. The
  landed fix uses `(?![\w-])`. A `\b` patch would have passed review, looked
  correct, and left `git commit-graph verify` misclassified.

Both are the same failure mode caught twice: a plausible specification that a
run refutes. Recorded here rather than silently absorbed, since the value of the
parallel structure is precisely that the agent holding the file disagreed with
the agent holding the plan.

## 2026-07-29 — DX architecture cycle (branch `chatman-dx-cycle`)

Seven commits. 141 tests where there were none, and a separate CI `plugin` job
so a plugin failure is never masked by a Rust one.

Added checkpoints 22–26, 28 (new working systems) and 29–33 (recorded
negatives). Every one is `PARTIAL_ALIVE` or lower, all blocked on the same hop:
no clean-worktree replay outside this session, and nothing pushed. Under the
promotion law that bars `ALIVE` however green the suite is.

The canonical definition of a Gall checkpoint was recovered from `~/mfw`, where
it exists as a formal glossary symbol
(`mfw-math/15-galls-law-evolutionary-construction.omdoc:37`): *"the smallest
closed, receipted transformation proving one complete category transition with
explicit inputs, outputs, refusals, and verification."* `~/bcinr` supplied the
rule that a falsifier must execute and be non-vacuous — "a genuine
Gall-checkpoint negative fixture, not a comment describing one". `~/wasm4pm`
supplied the promotion law, now mechanized in `tests/test_receipts.py`.

**Standing changes.** 1 ceiling narrowed (an invariant was inert). 3 blocking
hop changed — its audit finding "no agent declares `tools:`" is now false. 8
partially retracted — the prior happy-path evidence used a fabricated frontier
including a nonexistent surface. 13 **downgraded** to `PARTIAL_ALIVE` +
`MOCKED`. 14 sharpened from "not re-verified" to **absent**. 19 gained a
five-link chain and lost the fork-refusal claim. 20 net honest downgrade: two
more closes, but both fabricated the validator verdict.

**Defects the new tests found while being written**, none of which were known
when the cycle was planned:

- four surfaces pointed at nonexistent paths — `crates/ferroplan/src/{temporal,
  search,heuristic,ground}` are `.rs` files, and they sat on the two
  highest-allocated surfaces;
- SHACL's first-ever run caught `ce:maxTurns` declaring
  `rdfs:range xsd:positiveInteger` while every value parsed as `xsd:integer` —
  the ontology's own declared range unsatisfied by its own data;
- the human projection of an unresolved binary was the empty string, which
  would have handed a launcher `exec ""`.

**Two corrections to earlier claims made in this same session**, recorded
because a corrected claim is worth more than a quiet edit:

- the MCP resolution failure was first blamed on `env.setdefault` preserving an
  empty variable. Measured: the variables are *unset*, so `setdefault` fires.
  The real cause was a four-parents-up walk calibrated for the repository
  layout landing on `cache/<marketplace>` under the install layout;
- the inert invariant was first going to be "fixed" by renaming
  `requires_any_prior` to `requires_any`. That would have forbidden every
  `planning=validated` vector and deleted the state from the reachable space.
  Deletion was correct.

**The ledger fragmentation defect demonstrated itself during the session that
documented it** (CE-GALL-32): the `Stop` hook blocked on 47 pending events in
the `plugins/chatman-ecosystem` ledger while the repository ledger read 0.

**Left undone, named rather than omitted:** MCP `validate` still returns prose
so the validator verdict is fabricated (CE-GALL-30); `verify_chain` does not
exist (CE-GALL-31); ledger anchoring is built but unwired (CE-GALL-32); the
admission TOCTOU is open and untested (CE-GALL-33); `loop.py close` is not
built, so both closes were nine manual steps; nothing is pushed and `main` has
none of it.

**Clean-clone replay performed, and it does NOT promote.** At seal `2ee20a5`
the tree was cloned to a fresh path, checked out at the sealed commit with a
verified-clean worktree, and run with all four steering variables cleared:
251 passed, `generate.py build --check` clean. That is real evidence and it
eliminates two failure modes — a dirty worktree, and environment leaking from
the authoring shell.

It is deliberately **not** recorded as `replayed_outside_session`. The promotion
law's boundary is the *session*, not the process, and the reason is the third
failure mode a clone cannot remove: the agent replaying is the agent that wrote
the tests and chose which to run. `wasm4pm` made the same call, demoting its own
receipts from `ALIVE` to `PARTIAL_ALIVE` pending a genuinely independent replay.
The flag stays `false` until someone else, or a later session, runs it.

**The one action that promotes 22–26, 28, 29 to `ALIVE`:** clone to a fresh
path, check out the sealed commit, and run `pytest` plus
`generate.py build --check` outside this session. Then set
`replayed_outside_session` and `sealed_at_commit` in each receipt.

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

## 2026-07-29 — Session Lifecycle Bookends (CE-GALL-35)

Opened CE-GALL-35 for `session_open`/`session_status`/`session_close`, the
three MCP session tools with no prior dedicated checkpoint or test coverage
(they only appeared as intermediate steps inside `session_protocol.rs`'s
longer chains). Added
`crates/ferroplan-mcp/tests/session_lifecycle_bookends.rs` with one positive
witness (open → status reflects grounded state → close reports `closed:
true`) and one negative falsifier (status on a closed session refuses with
`unknown session`; a second close on a closed session is checked against its
actual observed behavior). Both tests run for real against the built
`ferroplan-mcp` binary: `cargo test -p ferroplan-mcp --test
session_lifecycle_bookends` — 2 passed, 0 failed.

The negative test's first draft assumed a double-close would also refuse with
`unknown session`. Measured against the live server it does not: double-close
returns `isError: false` / `closed: false`, an idempotent no-op, not a
refusal. The assertion was rewritten to the real response rather than left
asserting the wrong thing to look tidy. `session_status`'s schema was
similarly corrected mid-draft — it has no `goal` field; the real fields are
`cursor`/`epoch`/`goal_met`/`domain_digest`/`problem_digest`/`plan_length`/
`remaining_plan_valid`/`receipt_chain_head`.

Standing: `PARTIAL_ALIVE` / `NO_REPLAY`, same cap as every other checkpoint in
this file — not replayed outside the authoring session. Receipt:
`plugins/chatman-ecosystem/receipts/CE-GALL-35.json`. No other checkpoint,
receipt, or test file was touched.

## 2026-07-29 — Goal Retarget and Cursor Advance (CE-GALL-36)

Opened CE-GALL-36 for `session_set_goal` and `session_advance`, neither of
which had prior dedicated checkpoint coverage — both were only exercised as
steps inside `session_protocol.rs`'s longer happy-path chain.

Added `crates/ferroplan-mcp/tests/session_goal_advance.rs` with one positive
witness and one negative falsifier, both run for real against the built
`ferroplan-mcp` binary over stdio (same harness as `session_protocol.rs`):
`cargo test -p ferroplan-mcp --test session_goal_advance -- --nocapture` — 2
passed, 0 failed.

The positive witness plans a real 3-step sequential domain to its original
goal, retargets mid-session to a different ground conjunction, and confirms
the retarget by replanning — the new plan is a genuinely different shape (1
step, not 3), not merely a status flag. `session_advance` then moves the
cursor over that real plan and `session_status` confirms it.

The negative falsifier called `session_advance` with `completed_steps` far
past the plan's real length and checked the TRUE observed response rather
than assuming a refusal. The tool does refuse cleanly — a tool-level
`isError` naming the plan-length bound (`do_session_advance`'s
`next > plan_length` guard in `crates/ferroplan-mcp/src/session.rs`) — and a
follow-up `session_status` confirms the rejected call left the cursor
untouched. No silent-acceptance surprise was found on this path; the honest
negative result is a working refusal, not a discovered gap.

Standing: `PARTIAL_ALIVE` / `NO_REPLAY`, same cap as every other checkpoint
in this file — not replayed outside the authoring session. Receipt:
`plugins/chatman-ecosystem/receipts/CE-GALL-36.json`. `session_set_goal`'s
own negative path (malformed/unreachable goal atoms) remains unexercised at
the MCP-tool layer and is named as a non-claim in the checkpoint section — it
is covered only by a library-layer unit test
(`crates/ferroplan/src/session.rs::set_goal_rejects_unknown_and_adl`). No
other checkpoint, receipt, or test file was touched.

## 2026-07-29 — CE-GALL-37, true recursive CMCA descent

Closed the specific gap CE-GALL-9 named: "true cross-call recursive descent ...
architecturally absent from the MCP tool schema." That claim held for
`bind_allocation_receipt`'s flat `previous_receipt` field but overlooked
`cmca_allocate_recursive`, a separate, already-implemented tool
(`crates/ferroplan-mcp/src/session.rs`) that chains a `root` frontier through
zero or more `descents`, each binding `parent_payload_digest` to the real
previous depth's `allocation_payload_digest`.

Ran the existing six-test `cmca_recursive_*` suite in
`crates/ferroplan-mcp/tests/session_protocol.rs` (`cargo test -p ferroplan-mcp
--test session_protocol cmca_recursive`, exit 0, all passed) and added a
seventh, `cmca_recursive_three_depth_chain_binds_digests_all_the_way_down`,
to cover a genuine three-depth chain rather than depth two only. Also drove
`cmca_allocate_recursive` live via the MCP tool three times this session: a
three-depth positive chain (digests bound correctly at every depth), a
cyclic-ancestry refusal (confirmed with the exact expected error text after
one informative near-miss that tripped the sibling unknown-parent path
instead), and an unknown-parent-node refusal.

Standing: `PARTIAL_ALIVE` / `NO_REPLAY`. Receipt:
`plugins/chatman-ecosystem/receipts/CE-GALL-37.json`. CE-GALL-9's own section
was left untouched as historical record; `blocked_by` on the new receipt is
empty because this checkpoint stands on its own tool, not on CE-GALL-9's
resolution. What CE-GALL-9 still names as open — `bind_allocation_receipt`'s
flat chaining field, and "return consequence" propagation back up a chain —
remains open and unaddressed by this work.

## 2026-07-29 — CE-GALL-40, full 17-tool dogfood chain (capstone)

Closed the gap the CE-GALL-35..39 session's audit found: no test anywhere
exercised all 17 `ferroplan-mcp` tools in one continuous chained flow. Ran
16 of the 17 tools live first, via direct
`mcp__plugin_chatman-ecosystem_ferroplan__*` tool calls in this session, on a
small two-action STRIPS domain (`at-a -> at-b -> at-c`): `parse` (domain,
problem) -> `solve` -> `session_open` -> `session_observe` ->
`session_set_goal` -> `session_think` -> `session_advance` ->
`cmca_allocate` -> `cmca_allocate_recursive` (root + 1 descent) ->
`canonical_digest` -> `bind_allocation_receipt` -> `validate` ->
`bind_plan_receipt` -> `verify_receipt` -> `session_status` ->
`session_close`, then a negative falsifier: `session_status` on the same
session_id immediately after `session_close` — confirmed live as a lawful
`unknown session` refusal, matching CE-GALL-35's already-documented
mechanism. `decompose` was deliberately not called; named as the one
uncovered tool rather than fabricated or silently skipped.

That live run found a real defect, not merely confirmed a guess:
`bind_allocation_receipt` refused `cmca_allocate_recursive`'s raw `depths`
payload with `allocation_result lacks payload.bcinr_revision`
(`crates/ferroplan-mcp/src/admission.rs:188-190`) — its schema expects the
flat shape `cmca_allocate` itself returns, not the recursive tool's own
output shape. Worked around by binding the recursive result's root-depth
payload (which does carry `bcinr_revision`) with the depth-2 chain folded in
under an explicit `recursive_extension` field.

Formalized the whole trace as a re-runnable Rust fixture,
`crates/ferroplan-mcp/tests/dogfood_chain.rs`, driving the built
`ferroplan-mcp` binary over stdio in one continuous JSON-RPC session (same
harness pattern as `session_lifecycle_bookends.rs` /
`session_protocol.rs`) — the more faithful transport for "one continuous
flow" than separate agent-session MCP calls. Ran it for real:
`cargo test -p ferroplan-mcp --test dogfood_chain -- --nocapture` — 2
passed (the chain, and the falsifier), 0 failed.

Standing: `PARTIAL_ALIVE` / `NO_REPLAY`, same cap as every other checkpoint
in this file. Receipt: `plugins/chatman-ecosystem/receipts/CE-GALL-40.json`.
`decompose`'s genuine decomposition behavior remains fully unexercised by
this checkpoint — that is a named gap, not a claim. `bind_plan_receipt`'s
`previous_receipt` was deliberately left `null`, documented in the
checkpoint section as a sibling-not-successor relationship, not an
oversight. CE-GALL-35 through CE-GALL-39's own sections, receipts, and test
files were not touched — this entry only adds new material.
