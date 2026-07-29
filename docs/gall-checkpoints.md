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

**Current standing:** `PARTIAL_ALIVE` (two independent mechanical
enforcement paths now confirmed live — tool-schema omission and a
`PreToolUse` Bash-write fence keyed off the harness's own `agent_type`
field — for the agents actually probed; still not exhaustive across all
8 agents, and the "outside `actuation=manufacturing`" half of the
required proof is still prompt-level only)

2026-07-29 first-pass findings (superseded in part, kept for history):

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
>
> **2026-07-29 cycle update (CE-GALL-35).** That "single re-run" happened —
> see the fourth-pass findings below — but it answered a narrower question
> than expected. The live re-run targeted the Bash-write gap (a non-editor
> agent writing a file *through* `Bash` despite lacking `Write`/`Edit`), not
> the tool-schema-omission question CE-GALL-27 flagged as unmeasured. Both
> are now covered by separate mechanisms: `tools:` frontmatter (generated,
> tested by `test_authority.py`) for named-tool omission, and
> `bash-write-fence.py` (new `PreToolUse` hook, see below and CE-GALL-35) for
> the Bash-write case specifically. Standing still does not move to `ALIVE`:
> the phase-gate half of this checkpoint's required proof ("outside
> `actuation=manufacturing`") remains untouched by every pass to date.

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

2026-07-29 fourth-pass findings (same day, follow-on session — closes the
Bash-write-fence half of the third pass's named gap, with a genuine
ground-truth signal instead of self-report):

- Found a real ground-truth field the third pass didn't know about yet:
  the `PreToolUse` hook payload includes `"agent_type":
  "chatman-ecosystem:rdf-observer"` (confirmed by dumping the raw JSON
  stdin a hook actually receives, via a temporary debug hook appended to a
  scratch copy of `hooks.json` — not self-report, not a debug-log
  inference, the literal payload Claude Code's harness sends to the hook
  process for that tool call). This sidesteps the third pass's whole
  "self-report is unreliable" problem for this specific question (which
  agent is making this tool call) because it's harness-emitted per-call
  metadata, not something the model is asked to describe about itself.
- Wrote `plugins/chatman-ecosystem/scripts/bash-write-fence.py`, a new
  `PreToolUse` hook (wired into `hooks.json` alongside the existing
  `loop.py hook` Bash matcher) that: reads `agent_type` from the payload,
  looks up that agent's own `tools:` frontmatter line, and denies the
  Bash call if the command looks write-shaped (a broadened pattern that
  also catches bare `>`/`>>` redirection, which `loop.py`'s existing
  `MUTATING_BASH` regex does not, since that regex exists for ledger
  bookkeeping, not enforcement) and the agent's allow-list does not grant
  `Write`/`Edit`. Agents with no `tools:` frontmatter, or with
  `Write`/`Edit` already granted (`source-manufacturer`), are left alone.
- Before writing the real regex, ran it against 20 synthetic commands
  covering the exact false-positive traps this checkpoint's own prior
  passes' commands use constantly (`... 2>&1 | head -50`, `2>/dev/null`)
  — an earlier draft flagged `2>&1` as a write (it isn't; it's a stream
  duplication) and `2>/dev/null` as a write (it's a no-op idiom for
  discarding stderr); both were fixed before this was ever pointed at a
  live agent, specifically to avoid the checkpoint gaining a mechanism
  that's real but wrong (over-fencing legitimate reads would be its own
  kind of falsifier).
- Live re-probe against a genuinely nested `claude --plugin-dir
  <scratch copy of this branch's plugin> --agent rdf-observer
  --permission-mode acceptEdits -p "..."` session (not this session's own
  Agent tool), asking it to run exactly one Bash call
  (`echo hook-probe-N > <scratch-file>`) and stop:
  - **Before** this pass's fence existed (same nested-session method,
    same agent, same kind of command, run earlier this same pass before
    the fix was written): the file **was** created — reproducing the
    third-pass audit's theorized gap as an actual file-system fact, not a
    hypothesis.
  - **After** wiring `bash-write-fence.py` into `hooks.json` and pointing
    `--plugin-dir` at the patched copy: the identical style of command
    from `rdf-observer` produced **no file** (checked directly on disk,
    not via model self-report). Re-ran the same probe against
    `config-law-architect` (a second, distinct non-manufacturing agent
    with its own `tools:` grant) with the same result: no file created.
  - Positive control: the same probe against `source-manufacturer`
    (`tools: ..., Write, Edit, NotebookEdit`) **did** create its file —
    confirming the fence is agent-specific (reading the real per-agent
    `tools:` grant), not a blanket Bash lockout.
  - Not cleanly obtained this pass: a positive confirmation that ordinary
    *read-only* Bash commands (e.g. `git log --oneline -3`) still work
    for a fenced agent post-patch. Two attempts at this hit an unrelated
    noise source already flagged by the third pass — the nested `-p`
    session's own CLI permission layer denies many Bash calls outright
    when no `--allowedTools` is passed, independent of this hook, and the
    model then burns its turn budget retrying phrasings instead of
    reporting a clean result. The four write-vs-no-write comparisons
    above are unaffected by this (they turn on file existence on disk,
    not on model-reported text), but the narrower "reads still work"
    claim is left unconfirmed rather than asserted from noisy data.
- Also discovered and recorded, in passing, a distinct environment
  finding relevant to any future pass attempting this kind of live probe
  in a similarly sandboxed container: overriding `$HOME` to an empty
  scratch directory (to simulate a "genuinely clean" install, the
  technique the second and third passes describe using) breaks this
  container's OAuth-based `claude` authentication entirely — every
  `claude -p` call under a scratch `$HOME` hung until an external timeout
  killed it, with no usable error. The working technique in *this*
  session's container was `--plugin-dir <path>` under the real `$HOME`
  (auth intact) rather than a marketplace install under a swapped
  `$HOME`. This is a narrower clean-cache simulation than a full
  marketplace install (no cache/version staleness is possible by
  construction), so it does not itself speak to Checkpoint 2 — recorded
  here only so the next session doesn't re-lose the ~90-175 second
  per-probe budget this container's plugin hooks (`loop.py`/`phase.py`
  status reads, plus a `cargo run` cold-start for the `ferroplan` MCP
  server) actually require; a 60-90s timeout looks identical to a genuine
  hang but is usually just this container being slower than the prompt
  budget assumed.
- Cargo hygiene while on this branch: `cargo fmt --check` was failing on
  a pre-existing, unrelated diff in
  `crates/ferroplan-mcp/tests/admission_protocol.rs` (flagged by the
  third pass, not fixed then). Ran `cargo fmt` (mechanical, no Rust files
  in this pass's own change set were touched otherwise) — now clean.
  `cargo clippy -p ferroplan-mcp -p ferroplan --all-targets
  --all-features -- -D warnings` and `cargo test -p ferroplan-mcp -p
  ferroplan` both pass after the formatting fix.

Standing changed: Checkpoint 3 stays `PARTIAL_ALIVE` — the first half of
the "Required proof" (edit refusal) now has two independent mechanisms
confirmed live: tool-schema omission (`tools:` allow-list, second pass)
*and* Bash-write fencing keyed off a genuine harness field (this pass),
tested against 2 of 8 agents plus the positive control. Not upgraded to
`ALIVE`: 5 of 8 agents' Bash-write fencing is untested live (rests on the
same code path, not individually re-probed); the checkpoint's second
required-proof clause ("manufacture outside `actuation=manufacturing`")
remains entirely prompt-level, unchanged from every prior pass; and
PR #4's competing `disallowedTools` approach for the same 8 files still
exists as an open, unreconciled alternative (noted by the third pass,
still not resolved by this one — this pass's fence is layered on top of
this branch's `tools:` allow-list, not on PR #4's `disallowedTools`
deny-list, so reconciling the two PRs would need to carry this fence's
`agent_type` + frontmatter-lookup approach over either way).

**Next step**: live-probe the remaining 5 non-manufacturing agents'
Bash-write fencing individually (same method, same fence, just unrun);
confirm read-only Bash still functions for a fenced agent with a cleaner
probe method (one that isn't confounded by the CLI's own unrelated
permission-denial noise in headless `-p` sessions); reconcile PR #4 vs.
PR #5's competing frontmatter approaches (a call for whoever has
authority to close/supersede one, not something a single audit pass
should do unilaterally); and separately, tie `source-manufacturer`'s
`Write`/`Edit` availability to `actuation=manufacturing` rather than
prompt-only, which no pass has yet attempted.

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

`loop.py:372` sets `admitted_event_count = event_count` — a blanket watermark
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
`grep -n 'admitted_event_count.*event_count' scripts/loop.py`, is **`:372`**.

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

---

# Audit log

## 2026-07-29 — parallel-agent iteration (branch `chatman-dx-cycle`)

Three agents worked in parallel on disjoint file sets. Two feature commits
landed: `63a8a70` (Rust) and `1a9ab50` (canonical Bash classification). The
suite went from 251 to 308 tests. This entry is the receipt-and-document pass
over that work.

**Corrections to existing receipts**, recorded because a stale receipt is worse
than a missing one — it is evidence pointing at the wrong line:

- CE-GALL-33 cited `loop.py:388` for the admission TOCTOU. The file has shifted
  and `:388` is now the plan-digest format check; the true line is `:372`. A
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

## 2026-07-29 — fourth pass (scheduled run; Checkpoint 3 continued, PR #5 branch)

This was a scheduled (unattended) run. Started per this file's own
instructions: read the whole file plus the Audit log first, then checked
`git branch -r` — which, as the third pass had already warned, missed the
same-day branches until an explicit
`git fetch origin 'refs/heads/gall-checkpoints/*:...'` was run. That
turned up PR #3, #4, and #5 (none merged, none reflected past PR #2 in
this file before the third pass added them). Continued PR #5's branch
(`gall-checkpoints/2026-07-29-agent-tool-grants`, which already carried
the first three passes' work) rather than opening a fifth branch, per
this file's "prefer finishing a started thread" guidance, and picked the
third pass's most concrete named next step: build the Bash-write fence
the `tools:`/`disallowedTools` frontmatter approaches (both PR #4 and
PR #5) structurally cannot express.

What was done, in order:
1. Confirmed the exact gap first: reproduced `rdf-observer` writing a
   file via Bash despite no `Write`/`Edit` grant, using a genuinely
   nested `claude --plugin-dir <this branch's plugin> --agent
   rdf-observer` session (not this session's own Agent tool) — a real
   file appeared on disk. This took real debugging to get working at
   all in this container (see the environment note below); it is not
   assumed from the prior passes' write-ups.
2. Found the ground-truth field the third pass's "next step" asked for,
   by dumping a hook's raw stdin: `PreToolUse` payloads include
   `agent_type` (e.g. `"chatman-ecosystem:rdf-observer"`), a real
   harness-emitted identifier, not a self-report.
3. Wrote `plugins/chatman-ecosystem/scripts/bash-write-fence.py` and wired
   it into `hooks.json`'s existing `PreToolUse`/`Bash` matcher. Verified
   the write-shaped-command regex against 20 synthetic cases first
   (catching two real false-positive bugs — `2>&1` and `2>/dev/null`
   being misread as writes — before ever pointing it at a live agent).
4. Live-reran the nested-session probe with the fence active:
   `rdf-observer` and `config-law-architect` (2 of the 7 non-manufacturing
   agents) no longer create the probe file; `source-manufacturer`
   (positive control) still does. Full detail, including the exact
   commands, is recorded inline under Checkpoint 3's "fourth-pass
   findings" above — not duplicated here.
5. Fixed the pre-existing `cargo fmt --check` drift in
   `admission_protocol.rs` (flagged by the third pass, not fixed then) by
   running `cargo fmt` — purely mechanical, no behavior change. Verified
   `cargo clippy -p ferroplan-mcp -p ferroplan --all-targets
   --all-features -- -D warnings` and `cargo test -p ferroplan-mcp -p
   ferroplan` both pass clean afterward. Did not attempt
   `cargo check --workspace` (the third pass already root-caused its
   failure to the unrelated `ferroplan-bevy`/rustc 1.95 requirement,
   pre-existing and not touched this pass).

Standing: Checkpoint 3 stays `PARTIAL_ALIVE` (see its own standing line
and fourth-pass section above for the precise reasoning) — a second,
independent mechanical-enforcement path is now confirmed live for 2 of 8
agents, not asserted for all 8, and the phase-gate half of the required
proof remains untouched by any pass so far.

New environment finding, useful to whoever runs the next live-agent probe
in a similarly sandboxed container: swapping `$HOME` to a scratch
directory (the "genuinely clean cache" technique the second and third
passes describe) breaks this container's OAuth auth outright — every
nested `claude -p` call under a scratch `$HOME` hangs until externally
killed, with no error text surfaced. `--plugin-dir <path>` under the
*real* `$HOME` was the technique that actually worked here. Separately,
this plugin's own hooks (`loop.py`/`phase.py` status reads on
`SessionStart`, plus a cold `cargo run` for the `ferroplan` MCP server
the first time it's invoked) make a nested probe genuinely take
60–175 seconds; several early attempts this pass looked like hangs but
were just under-timed (a 60–90s timeout is not enough headroom in this
container). Recorded so the next session doesn't re-lose the same time
figuring this out.

Artifacts left behind by this pass:
- `plugins/chatman-ecosystem/scripts/bash-write-fence.py` — new
  `PreToolUse` hook, real (not a mock), tested live against 2 of 8
  non-manufacturing agents plus the `source-manufacturer` positive
  control.
- `plugins/chatman-ecosystem/hooks/hooks.json` — one new hook entry
  wiring the fence into the existing `Bash` `PreToolUse` matcher.
- `crates/ferroplan-mcp/tests/admission_protocol.rs` — `cargo fmt`
  formatting fix only (no logic change), closing a two-pass-old named
  next step.
- This file, updated in place (Checkpoint 3's standing line and a new
  "fourth-pass findings" section) and this entry.

Not attempted this pass, deliberately out of scope: reconciling PR #4 vs.
PR #5 (flagged again as an open, unresolved conflict — a call for
whoever has authority to close/supersede one, not something a single
audit pass should do unilaterally); live-probing the remaining 5
non-manufacturing agents' Bash-write fencing individually; tying
`source-manufacturer`'s `Write`/`Edit` to `actuation=manufacturing`; the
`ecosystem-controller` MCP-tool anomaly from the third pass (still
needs the ground-truth method, which now exists via `agent_type`, but
wasn't pointed at that specific question this pass); Recommended Release
Sequence items 3 onward (worktree manufacture, VAL integration, etc.) —
this pass stayed on item 2 (Checkpoint 3) since it had a concrete,
unfinished, in-scope next step rather than moving on early.

Branch: `gall-checkpoints/2026-07-29-agent-tool-grants` (continues PR #5,
https://github.com/seanchatmangpt/ferroplan/pull/5). Not pushed as part of
writing this entry — see the commit that follows it for the pushed state.
