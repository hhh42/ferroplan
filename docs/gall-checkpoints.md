# Gall Checkpoints for the Chatman Ecosystem

Last updated: 2026-07-29 (session audit #2, see "Audit log" at the end).

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

**Current standing:** `PARTIAL_ALIVE` (mechanical enforcement now confirmed
for the surface actually tested; not yet exhaustive across all 8 agents)

2026-07-29 first-pass findings (superseded in part, kept for history):
- None of the 8 agent `.md` files under `plugins/chatman-ecosystem/agents/`
  declared a `tools:` frontmatter field. Confirmed by this session's own
  Agent-tool listing, which annotated every one of the 8 chatman-ecosystem
  agents with `(Tools: All tools)`. No mechanical denial existed at the
  Claude Code harness level.
- Live test: spawned `rdf-observer` and asked it to edit a throwaway file
  outside the repo. It refused — but by **choosing to honor its own role
  prose**, not because the harness blocked the `Edit` tool call.
- Conclusion at the time: role separation was prompt-level compliance, not
  mechanical enforcement.

2026-07-29 second-pass findings (same day, follow-on session — closes the
named next step):
- Added a `tools:` allow-list line to all 8 agent frontmatter blocks,
  transcribed directly from the grants already declared (but previously
  unused) in `ontology/authority-graph.ttl`'s `ce:allowsTool` triples —
  e.g. `rdf-observer` → `tools: Read, Glob, Grep, Bash`, `cmca-allocator` →
  `tools: Read, mcp__ferroplan`, `source-manufacturer` → `tools: Read,
  Glob, Grep, Bash, Write, Edit, NotebookEdit`. `claude plugin validate`
  (non-strict) still passes with these fields present; `--strict` fails
  only on a pre-existing, unrelated `plugin.json` missing-`version`
  warning confirmed present before this change too (via `git stash`).
- Installed the plugin from a genuinely clean cache in this container
  (`claude plugin list` reported "No plugins installed" before this run —
  this session's own container had never loaded the plugin). Ran
  `claude plugin marketplace add ./` (local directory source, not the
  GitHub source declared in `.claude/settings.json`) then
  `claude plugin install chatman-ecosystem@chatman-ecosystem --scope
  project`. Both succeeded with no loader errors; `claude plugin list`
  showed it `✔ enabled`. This is real clean-cache evidence, but narrower
  than Checkpoint 2's stale-github-marketplace question — see that
  checkpoint for what remains open there.
- Live refusal re-test, this time forcing an actual tool-call attempt
  (explicit "report your raw tool schema, then attempt the call" prompt,
  run via `claude -p --agent <name> --allowedTools "Edit,Write,Bash,Read"`
  so the CLI permission layer could not be the confound) rather than
  letting the model self-censor on role prose alone:
  - `rdf-observer`: raw schema reported as exactly `Read, Glob, Grep,
    Bash` — no `Write`, `Edit`, `NotebookEdit`, no MCP tools. Matches its
    new frontmatter exactly.
  - `cmca-allocator`: raw schema reported as exactly `Read` — no `Bash`,
    no `Write`/`Edit`. Matches its (more restrictive) frontmatter.
  - `source-manufacturer` (positive control): raw schema reported as
    `Read, Bash, Write, Edit, NotebookEdit` — `Write`/`Edit` genuinely
    present here, proving the restriction on the other two agents is the
    `tools:` field being read per-agent, not a blanket default.
  - No file was actually written in any of these three runs — for
    `rdf-observer`/`cmca-allocator` because `Write`/`Edit` are structurally
    absent from the schema (mechanical, not a choice); for
    `source-manufacturer` because it separately declined citing
    `actuation: sealed` in the current phase vector — that second gate is
    still prompt-level, see below.
- **This closes the first half of this checkpoint's "Required proof"**
  ("Attempt direct edits from every non-manufacturing agent and observe
  refusal") for the two agents actually probed, with genuine harness-level
  evidence (tool absent from schema) rather than a model's own judgment
  call. The other 5 non-manufacturing agents
  (`config-law-architect`, `ecosystem-controller`, `ferroplan-planner`,
  `independent-validator`, `receipt-auditor`) now carry the same kind of
  `tools:` line, transcribed from the same ontology, but were **not**
  individually live-probed this pass — their standing rests on the ttl
  transcription plus the fact that Claude Code's `tools:` enforcement was
  just confirmed to work at all (not on a per-agent re-verification).
- **Second half of "Required proof" — "Attempt manufacture outside
  `actuation=manufacturing` and observe refusal" — is still NOT
  mechanically enforced.** `source-manufacturer`'s schema includes
  `Write`/`Edit` unconditionally; nothing in the harness ties tool
  availability to the six-dimensional phase vector's `actuation` value.
  Today it refuses out-of-phase writes only because its own prompt says
  to check `actuation: sealed` first — the same prompt-level-only gap the
  first pass identified, just narrowed to this one axis instead of both.
- Gap noticed in passing: none of the `mcp__ferroplan` grants resolved to
  actual callable MCP tools in these nested `-p` probe sessions (schema
  showed zero `mcp__*` entries even for `cmca-allocator`, whose
  frontmatter grants `mcp__ferroplan`). Plausible cause: the plugin's
  ferroplan MCP server has 2 unset `userConfig` options
  (`claude plugin install` printed "2 userConfig options not yet set") and
  a non-interactive `-p` session may not wait for a stdio MCP server to
  connect. Not chased further this pass — named here so it isn't
  silently lost; relevant to Checkpoint 7 more than Checkpoint 3.

2026-07-29 third-pass findings (same day, follow-on session — attempted
next-step (a), and found a real problem with the evidence method itself):

- Picked up this checkpoint's named next step (a): live-probed the
  remaining 5 non-manufacturing agents (`config-law-architect`,
  `ecosystem-controller`, `ferroplan-planner`, `independent-validator`,
  `receipt-auditor`) against a freshly reinstalled clean plugin cache
  (`claude plugin uninstall` → `marketplace remove` → `marketplace add
  /home/user/ferroplan` → `plugin install`, confirmed via
  `grep tools: .../agents/*.md` on the resulting cache directory that it
  matched this branch's exact committed frontmatter) using the same
  "list your own tool names" `-p --agent` self-report method the prior
  pass used.
- **Found the self-report method itself is unreliable — a real,
  reproducible negative result, not a mock.** Running the *identical*
  prompt against `ecosystem-controller` three times back-to-back produced
  three mutually-contradictory tool lists: run 1 = `Read, Bash, Agent,
  mcp__ferroplan`; run 2 = `WebFetch, WebSearch, TodoWrite, BashOutput,
  KillShell, SlashCommand, Task, ExitPlanMode, Monitor, SendMessage` plus
  15 `mcp__ferroplan__*` tools (no `Read`, no `Bash` at all); run 3 = yet
  another different list (`TodoWrite, BashOutput, KillShell,
  SlashCommand, ListMcpResources, ReadMcpResource, Monitor, SendMessage,
  Artifact, ExitPlanMode` plus the same 15 MCP tools). Since `Read` is
  known to always be granted and didn't even appear in 2 of 3 runs, this
  is the model producing a plausible-sounding but fabricated answer under
  this framing, not genuine introspection of its real tool schema. This
  directly undercuts the confidence of the *previous pass's* headline
  claim ("harness-level, not model self-report") for `rdf-observer`,
  `cmca-allocator`, and `source-manufacturer` — their specific
  no-`Write`/`Edit` results may still be correct, but the **method** used
  to get them has now been shown untrustworthy in general, so those three
  results should be read as "consistent with the intended restriction,"
  not "confirmed."
- Went looking for a non-self-report signal instead. `--debug-file
  ... --debug api` reproducibly logs `Tool search disabled: ToolSearchTool
  is not available (may have been disallowed via disallowedTools)` for
  every agent checked (`rdf-observer`, `config-law-architect`,
  `source-manufacturer`, `cmca-allocator` — 4 separate runs, same line
  every time). This is genuine harness-emitted evidence that the `tools:`
  allow-list is actively consulted, not prompt convention. **But it does
  not discriminate between agents or tools**: `ToolSearch` isn't in any
  of the 8 agents' allow-lists, so the identical line appears regardless
  of whether a given agent's frontmatter grants `Edit`/`Write` or not —
  confirmed by the fact that `source-manufacturer` (which explicitly
  grants `Write`/`Edit`/`NotebookEdit`) produces the exact same line, and
  neither its debug log nor `rdf-observer`'s contains any mention of
  `Edit`/`Write`/`NotebookEdit` at all, granted or not. This was a
  reasoning error in an earlier attempt this same pass (treating
  `Edit`-absence-from-the-log as agent-specific proof) — corrected here
  rather than left in the record uncaught.
- Actually attempted a real Edit call (not self-report) against
  `config-law-architect` under `--permission-mode acceptEdits` with an
  explicit imperative instruction. Result: it declined and stated
  verbatim "not attempted, no success or error text to quote — the call
  was withheld at the role/phase gate before invocation." This is the
  same prompt-level-choice pattern the very first pass (2026-07-29,
  `rdf-observer`) originally flagged as *not* mechanical enforcement —
  meaning the newly-probed agents still have no demonstrated
  tool-schema-level refusal, only self-policed refusal, for the direct
  attempt method.
- Resolved gap (c) from the prior pass's next steps: repeating the
  self-report prompt against `independent-validator` three times showed
  the full `mcp__ferroplan__*` (or `mcp__plugin__chatman-ecosystem__
  ferroplan__*`, naming varied) tool set present in 2 of 3 runs and
  completely absent in 1 of 3 — a genuine startup race between the stdio
  MCP handshake and the session's first turn, not a permanent
  non-resolution as the prior pass guessed.
- New anomaly surfaced, explicitly left at `UNKNOWN` given the self-report
  reliability problem above: `ecosystem-controller`'s frontmatter grants
  no `mcp__*` entry at all, yet 2 of its 3 self-reported runs listed the
  full `mcp__ferroplan__*` tool set. If real, this would mean the
  controller can directly call allocation/planning/receipt-binding MCP
  tools the authority-graph ontology never grants it — a genuine breach
  of "Controller routes but cannot [do the work itself]." Given the
  method's now-demonstrated unreliability, this cannot be asserted; it
  needs a trustworthy verification method (see next step) before it's
  treated as confirmed or dismissed.

Standing intentionally NOT changed this pass despite the additional work:
`PARTIAL_ALIVE` stays `PARTIAL_ALIVE`. The honest outcome of this pass is
a corrected, more skeptical picture of the evidence quality, not a wider
confirmed surface — asserting "5 more agents mechanically confirmed"
would have been the exact overclaiming this file's discipline exists to
prevent.

**Next step**: build an actual ground-truth check instead of self-report
or debug-log inference — e.g. a `PreToolUse`/`UserPromptSubmit` hook (or a
thin MITM on the `--debug` HTTP layer) that captures the literal `tools`
array Claude Code sends to the Anthropic API for a given `--agent`
session, so presence/absence of `Edit`/`Write`/`mcp__ferroplan__*` per
agent can be read directly off the wire instead of inferred. Until that
exists, re-attempt (b) (tie `source-manufacturer`'s `Write`/`Edit` to
`actuation=manufacturing` rather than prompt only) and chase the
`ecosystem-controller` MCP-tool anomaly with that same ground-truth method
once it exists.

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

## 2026-07-29 — second pass (Recommended Release Sequence item 2)

Picked up the named next step under Checkpoint 3 from the first pass:
"add `tools:` allow/deny lists to each of the 8 agent frontmatter files
... and re-run the same live refusal test ... expecting a harness-level
tool-permission error, not a model choice."

What was done:
1. Added a `tools:` line to all 8 files in
   `plugins/chatman-ecosystem/agents/*.md`, transcribing the grants
   already present (but previously unused by anything) in
   `ontology/authority-graph.ttl`'s `ce:allowsTool` triples. No other
   frontmatter fields (`maxTurns`, `effort`, `disallowedTools`) were added
   — kept to the smallest slice that moves the checkpoint's needle, per
   the file's own instruction.
2. Confirmed with `claude plugin validate plugins/chatman-ecosystem`
   (non-strict) that the plugin still loads cleanly with these fields
   present. `--strict` fails only on a pre-existing `plugin.json`
   missing-`version` warning, confirmed via `git stash`/`git stash pop` to
   predate this change and be unrelated to it.
3. This container's own `claude` CLI reported `claude plugin list` → "No
   plugins installed" going in — a genuinely clean cache, unprompted.
   Used it to also touch Checkpoint 2's still-open "re-run from a clean
   cache" next step: `claude plugin marketplace add ./` (local path, not
   the GitHub source in `.claude/settings.json`) then
   `claude plugin install chatman-ecosystem@chatman-ecosystem --scope
   project` both succeeded, `claude plugin list` showed it enabled, no
   loader errors. Narrower than the GitHub-source scenario Checkpoint 2
   asks about (no staleness possible with a local-directory source by
   construction), so Checkpoint 2's standing was left unchanged, but the
   evidence is recorded there.
4. Re-ran the live refusal test from the first pass, this time forcing an
   actual tool-call attempt via `claude -p --agent <name> --allowedTools
   "Edit,Write,Bash,Read"` and asking the agent to report its raw tool
   schema before/while attempting the write, so a model's own role-prose
   decision could not masquerade as the answer:
   - `rdf-observer`: schema = `Read, Glob, Grep, Bash` exactly. No
     `Write`/`Edit`. Matches frontmatter.
   - `cmca-allocator`: schema = `Read` exactly. No `Bash`, no
     `Write`/`Edit`. Matches frontmatter (its ttl grant is narrower still
     than rdf-observer's).
   - `source-manufacturer` (positive control): schema = `Read, Bash,
     Write, Edit, NotebookEdit`. Confirms the restriction seen on the
     other two agents is `tools:` being honored per-agent by the harness,
     not a blanket default that happens to look restrictive.
   All three runs were driven with `-p` (non-interactive) plus
   `--allowedTools` covering every relevant tool at the CLI permission
   layer, specifically to isolate "is the tool absent from the schema" from
   "did the CLI's own permission prompt block it" — the two are different
   mechanisms and only the first is what this checkpoint is about.

Standing changed: Checkpoint 3, `PARTIAL_ALIVE` → `PARTIAL_ALIVE`
(unchanged label, materially different evidence quality — the first
half of the checkpoint's "Required proof" now has genuine harness-level
confirmation instead of an unverified model choice, for the two agents
actually probed; the "outside actuation=manufacturing" half is still
prompt-level only, and 5 of 8 agents still rest on ttl-transcription
without a live per-agent probe). Not upgraded to `ALIVE` because the proof
is not yet exhaustive across all 8 agents and the second required-proof
clause is unmet.

Artifacts/changes left behind:
- `plugins/chatman-ecosystem/agents/*.md` (all 8) — `tools:` frontmatter
  added, committed on branch `gall-checkpoints/2026-07-29-agent-tool-grants`.
- This file, updated in place (Checkpoint 2 and 3 sections, this entry).

Gap surfaced, not chased this pass: `mcp__ferroplan` tool grants (used by
`cmca-allocator`, `ferroplan-planner`, `independent-validator`,
`receipt-auditor`) did not resolve to any callable `mcp__*` tools in the
`-p` probe sessions — plausibly because the plugin has 2 unset
`userConfig` options and/or a non-interactive session doesn't wait for the
stdio MCP server handshake. Relevant to Checkpoints 3 (the 4 agents that
rely on MCP tools for real work still have an effectively-empty toolset
until this is fixed), 7, and 9 — flagged for whoever picks those up next.

Also noticed, not fixed (out of this pass's scope — no Rust files were
touched): `cargo fmt --check` currently reports a pre-existing diff in
`crates/ferroplan-mcp/tests/admission_protocol.rs` (two multi-line call
formatting reflows), unrelated to this pass's changes. Worth a follow-up
before the next `RELEASING.md` pre-flight, since that checklist requires
`fmt --check` to pass clean.

Named next steps, not yet started: live-probe the remaining 5 agents
(`config-law-architect`, `ecosystem-controller`, `ferroplan-planner`,
`independent-validator`, `receipt-auditor`) the same way; make
`source-manufacturer`'s `Write`/`Edit` availability conditional on
`actuation=manufacturing` rather than prompt-only; fix the `mcp__ferroplan`
non-resolution in `-p` sessions; fix the pre-existing `cargo fmt --check`
drift in `admission_protocol.rs`.

## 2026-07-29 — third pass (Checkpoint 3 continued, this session)

Started by running `git fetch origin 'refs/heads/gall-checkpoints/*:...'`
before doing anything else, since the task instructions require checking
for a same-day branch first. This turned up three branches this session's
own initial (unfetched) `git branch -r` had missed: PR #3 (Checkpoint 2,
plugin.json version fix), PR #4 (Checkpoint 3, `disallowedTools` approach),
and PR #5 (Checkpoint 3, `tools:` allow-list approach — the exact same
idea this session had independently started implementing on a fresh
branch before discovering PR #5 already existed). Discarded the duplicate
local work and checked out PR #5's actual branch
(`gall-checkpoints/2026-07-29-agent-tool-grants`) to continue its named
next step instead of opening a fourth competing PR. Noting this
explicitly: PR #4 and PR #5 both still exist, open, and mutually
conflicting (both rewrite the same 8 frontmatter blocks with different
mechanisms — `disallowedTools` deny-list vs. `tools:` allow-list) — this
pass did not attempt to close or merge either, since that wasn't this
session's call to make unilaterally; flagging it here so the next session
doesn't lose it.

Did the work described inline under Checkpoint 3's "third-pass findings"
above: live-probed the 5 previously-unprobed agents, discovered the
self-report tool-listing method is unreliable (`ecosystem-controller`
gave 3 contradictory answers to an identical prompt across 3 runs),
cross-checked with `--debug api` logs and found that signal doesn't
discriminate per-agent either (correcting a reasoning error made
mid-pass), directly attempted a real Edit call against
`config-law-architect` (declined, prompt-level, not a tool-schema error),
and confirmed the `mcp__ferroplan` resolution gap from the prior pass is
a startup race condition (2/3 runs had it, 1/3 didn't), not permanent
absence.

Standing: Checkpoint 3 stays `PARTIAL_ALIVE`, unchanged. This pass
deliberately did not upgrade anything — its real contribution is negative
evidence (the two evidence methods relied on so far, self-report and
debug-log inference, are both shown unreliable for discriminating
per-agent tool availability), which matters more than a false confirmation
would have. Recorded as a correction, not silently absorbed.

No Rust files were touched this pass, so `cargo fmt --check`/clippy gates
were not run. `cargo check --workspace` was attempted once, unrelated to
this pass's actual change, and failed on a pre-existing, unrelated issue:
`crates/ferroplan-bevy` (an optional workspace member) requires `bevy
0.19.0`, which needs `rustc 1.95.0`; this container has `rustc 1.94.1`.
`cargo check -p ferroplan-mcp -p ferroplan` (the crates this checkpoint
work actually concerns) succeeds cleanly. Flagging the bevy/rustc mismatch
here since it would block a full `cargo check --workspace` pre-flight per
`RELEASING.md`, but it is pre-existing and out of scope for this pass.

Named next steps, not yet started: build a real ground-truth tool-schema
capture (hook-based or wire-level) instead of trusting self-report or
debug-log inference; reconcile/choose between PR #4 and PR #5's competing
approaches to the same 8 files; make `source-manufacturer`'s `Write`/
`Edit` conditional on `actuation=manufacturing`; investigate the
`ecosystem-controller` unexplained `mcp__ferroplan__*` exposure once a
trustworthy method exists; fix the pre-existing `cargo fmt --check` drift
in `admission_protocol.rs`; resolve the `ferroplan-bevy`/rustc 1.95
requirement or document it as a known environment constraint.
