# Product Requirements Document: Fortune 5 Capability Readiness

**Product:** ferroplan  
**Document ID:** FP-PRD-F5-001  
**Status:** Implementation contract  
**Target release:** 0.22.x readiness line  
**Owner:** ferroplan maintainers  
**Last updated:** 2026-08-05

## 1. Purpose

This document defines the product contract required for every ferroplan capability to be admitted as **Fortune 5 production ready**.

“Fortune 5 production ready” is an engineering admission profile. It is not a claim that ferroplan has already been deployed by, audited by, or certified for a Fortune 5 company. A capability is admitted only when committed evidence demonstrates that it satisfies the requirements in this document.

The product objective is to convert ferroplan from a collection of working interfaces into one coherent, bounded, observable, replayable, secure, and supportable planning product.

## 2. Product thesis

ferroplan is the deterministic planning core through which human purpose becomes formally bounded operational intent:

```text
human purpose
→ Eve
→ RDF Genesis
→ SPARQL CONSTRUCT
→ HDDL
→ PPDDL when uncertainty exists
→ ggen manufacturing contract
→ MCP+ capability surface
→ BRCE-authorized actuation boundary
→ OCEL 2.0 observation obligation
→ Truex conformance, receipt, and replay obligation
```

ferroplan itself plans, validates, explains, fingerprints, and exports bounded candidate consequences. It does not grant ambient execution authority and does not self-issue authoritative execution receipts.

## 3. Product outcomes

The readiness program SHALL produce the following outcomes:

1. Every shipped interface has a stable identity, owner, version, authority class, failure contract, resource envelope, telemetry contract, compatibility policy, and recovery procedure.
2. Every advertised capability exists in the repository and is exercised by executable evidence.
3. Every machine-consumable interface emits deterministic, versioned, structured output.
4. Every untrusted input boundary is bounded before parsing or execution.
5. Every capability can be independently diagnosed, replayed where applicable, and refused without partial authority leakage.
6. Release artifacts are reproducible enough to identify exact source, dependency, capability-manifest, and interface-schema lineage.
7. Documentation and the shipped capability inventory cannot diverge silently.

## 4. Personas

### 4.1 Platform engineer

Embeds ferroplan as a Rust library, service, MCP+ capability, Python package, or WebAssembly component. Requires stable interfaces, deterministic behavior, bounded resource use, and compatibility commitments.

### 4.2 Application engineer

Uses the CLI, Python API, browser API, or Eve handoff to solve, validate, explain, and inspect planning problems. Requires actionable errors and uniform semantics across surfaces.

### 4.3 SRE / operator

Operates ferroplan under load. Requires health/readiness reporting, structured telemetry, saturation signals, time and memory limits, cancellation, incident diagnostics, and recovery procedures.

### 4.4 Security / compliance engineer

Requires explicit trust boundaries, no ambient authority, dependency provenance, secret-safe telemetry, bounded input handling, artifact integrity, and machine-readable evidence.

### 4.5 Solution architect

Requires capability contracts that map human purpose to formal planning and downstream Execution Trust obligations without confusing candidate planning output with admitted consequence.

## 5. Shipped capability inventory

The following capability IDs are in scope. A capability SHALL NOT be advertised as shipped unless it appears in the canonical capability manifest and passes its admission checks.

| Capability ID | Surface | Product responsibility |
|---|---|---|
| `fp.core.solve` | Rust library | Parse, ground, search, and return a bounded planning outcome |
| `fp.core.parallel` | Rust library | Execute bounded parallel search while preserving declared determinism semantics |
| `fp.core.stream` | Rust library | Emit bounded progress and terminal outcomes with cancellation support |
| `fp.core.validate` | Rust library | Validate plans independently from search |
| `fp.core.explain` | Rust library | Produce structured explanation evidence |
| `fp.core.fingerprint` | Rust library | Produce stable, domain-separated problem and result identities |
| `fp.eve.enter` | Rust library | Convert human purpose and Genesis assets into a deterministic, non-authoritative handoff |
| `fp.cli` | Native CLI | Expose planning and readiness contracts with stable exit and output semantics |
| `fp.python` | Python ABI3 | Expose bounded planning through typed Python errors and stable JSON/schema contracts |
| `fp.wasm` | Browser/WASM | Expose bounded browser planning without ambient host authority |
| `fp.bevy` | Native/browser GUI | Provide an operator-facing planning client without changing core semantics |
| `fp.mcpplus` | MCP+ stdio surface | Expose candidate planning capabilities through bounded JSON-RPC without actuation authority |
| `fp.plugin.chatman` | Plugin control plane | Expose governed ecosystem integration and verifier evidence |
| `fp.docs` | mdBook/API docs | Publish version-matched operational and integration contracts |
| `fp.release` | Release pipeline | Produce attributable, integrity-checked artifacts and capability evidence |

At the start of this program, `fp.mcpplus` is **BLOCKED** because the repository documentation advertises an MCP server while no `ferroplan-mcp` crate is present in the source tree. The program SHALL either ship the bounded MCP+ surface defined here or remove the advertisement. Silent inconsistency is prohibited.

## 6. Authority model

### PRD-AUTH-001 — Candidate-only planning

All plans, generated artifacts, Eve handoffs, explanations, and tool responses produced by ferroplan SHALL be classified as candidate outputs unless an external Execution Trust system admits them.

### PRD-AUTH-002 — No ambient actuation

No ferroplan interface SHALL execute arbitrary operating-system commands, mutate external systems, or infer authority from connectivity alone.

### PRD-AUTH-003 — Separation of duties

Planning, validation, execution authorization, observation, conformance, receipt issuance, and replay SHALL remain distinguishable roles. A producer SHALL NOT self-author an authoritative acceptance verdict.

### PRD-AUTH-004 — Downstream obligations

Eve and MCP+ outputs SHALL carry explicit obligations for BRCE actuation, POWL geometry, OCEL observation, receipt/refusal, and replay when the requested consequence crosses an execution boundary.

## 7. Functional requirements

### PRD-FR-001 — Canonical capability manifest

The repository SHALL contain one deterministic, versioned, machine-readable capability manifest generated from typed source definitions. It SHALL include:

- capability ID and semantic version,
- owning crate or subsystem,
- interface type,
- authority class,
- determinism class,
- replay class,
- input and output schema identifiers,
- resource-bound declarations,
- failure-contract identifier,
- observability contract,
- compatibility promise,
- security classification,
- readiness state,
- evidence identifiers.

Duplicate IDs, missing mandatory fields, non-canonical ordering, or evidence-free `ADMITTED` states SHALL fail validation.

### PRD-FR-002 — Readiness command

The CLI SHALL expose a non-mutating readiness command that emits the canonical capability report in human-readable or JSON form. The command SHALL use stable exit codes and SHALL distinguish `ADMITTED`, `PARTIAL`, `BLOCKED`, and `UNSUPPORTED`.

### PRD-FR-003 — Uniform result envelope

Machine interfaces SHALL expose a versioned result envelope containing:

- request ID,
- capability ID,
- capability version,
- outcome class,
- deterministic input fingerprint,
- elapsed monotonic time,
- limit/saturation information,
- warnings,
- typed error details when refused or failed,
- candidate-authority notice.

### PRD-FR-004 — Typed failure taxonomy

All public surfaces SHALL map failures into a stable taxonomy that separates at least:

- invalid request,
- unsupported feature,
- parse failure,
- semantic/model failure,
- resource limit exceeded,
- timeout,
- cancellation,
- no plan found,
- internal invariant violation,
- dependency or adapter failure.

Failure messages SHALL remain actionable without exposing secrets or unbounded input content.

### PRD-FR-005 — Resource envelopes

Every solve-capable public interface SHALL accept or enforce bounded limits for:

- input bytes,
- parse/model size,
- wall-clock duration,
- expanded states or search nodes,
- plan length,
- progress-event rate,
- concurrency/worker count,
- output bytes.

Defaults SHALL be safe for shared infrastructure. Explicit unlimited operation SHALL require an internal API and SHALL NOT be the default for service-facing surfaces.

### PRD-FR-006 — Cancellation and deadlines

Long-running native and service-facing operations SHALL support cooperative cancellation and absolute or relative deadlines. Cancellation SHALL terminate with a typed non-success outcome and SHALL NOT masquerade as `NO_PLAN`.

### PRD-FR-007 — Determinism declaration

Each algorithm and interface SHALL declare whether it is:

- deterministic for identical canonical inputs and options,
- deterministic only under a declared seed/worker configuration,
- intentionally nondeterministic.

A deterministic claim SHALL be verified by replay tests and stable fingerprints.

### PRD-FR-008 — Independent validation

Plans returned as solved SHALL be independently validated before a service-facing surface reports success. The validation result SHALL be included in the result envelope or explicitly marked unavailable.

### PRD-FR-009 — Eve production contract

`fp.eve.enter` SHALL:

- validate non-empty, bounded Genesis and planning assets,
- enforce `Need9 ⇒ Split`,
- emit deterministic domain-separated closure identities,
- preserve exact source lineage,
- refuse malformed or authority-escalating requests,
- never claim that a handoff is a receipt,
- never actuate.

### PRD-FR-010 — MCP+ production contract

`fp.mcpplus` SHALL:

- use bounded stdio JSON-RPC framing,
- implement deterministic initialization and capability discovery,
- expose only planning, validation, explanation, and readiness operations,
- impose request-size, concurrency, and deadline limits,
- return typed protocol errors,
- emit no logs on stdout,
- redact sensitive request content from stderr telemetry,
- carry candidate-only and downstream-authority obligations,
- execute no arbitrary command and make no network call by default.

### PRD-FR-011 — Cross-surface semantic parity

For equivalent canonical input and supported options, Rust, CLI, Python, WASM, and MCP+ SHALL agree on:

- parse acceptance,
- terminal outcome class,
- plan validity,
- input fingerprint,
- candidate-authority classification.

Surface-specific presentation differences are permitted; semantic drift is not.

### PRD-FR-012 — Version and schema discovery

Every machine interface SHALL expose product version, capability-manifest version, result-envelope schema version, and build/source identity.

### PRD-FR-013 — Recovery and replay

Failures and successful candidate outcomes SHALL preserve enough bounded metadata to reproduce the request without storing secret-bearing raw inputs by default. Operators SHALL be able to choose between fingerprint-only, redacted, and explicitly authorized full-input replay modes.

### PRD-FR-014 — Documentation truth gate

README capability tables and mdBook integration pages SHALL be generated from or verified against the canonical manifest. An advertised-but-absent capability SHALL fail CI.

## 8. Non-functional requirements

### PRD-NFR-001 — Memory safety

All first-party Rust crates SHALL forbid unsafe code unless a separately reviewed exception identifies the exact block, rationale, tests, and owner. The default target is zero first-party unsafe code.

### PRD-NFR-002 — Input safety

Untrusted inputs SHALL be length-bounded before allocation-heavy parsing. Parsers SHALL reject excessive nesting, token counts, and identifier lengths using typed limit errors.

### PRD-NFR-003 — Reliability

Public operations SHALL fail closed. A partial plan, truncated response, worker panic, invalid validation result, or missing evidence SHALL never be reported as success.

### PRD-NFR-004 — Availability budgets

The product SHALL publish benchmark-derived operating envelopes rather than universal latency promises. Release evidence SHALL include at least:

- representative small, medium, and stress planning corpora,
- p50/p95/p99 solve and validation latency for the declared runner,
- peak memory or a documented proxy,
- timeout and saturation behavior,
- deterministic replay rate.

### PRD-NFR-005 — Observability

Each request-capable surface SHALL support structured telemetry with:

- request/correlation ID,
- capability ID and version,
- outcome class,
- duration,
- bounded counters,
- limit reason,
- build identity.

Raw PDDL, ontology text, prompts, secrets, and personally identifying data SHALL NOT be logged by default.

### PRD-NFR-006 — Security and supply chain

Release admission SHALL require:

- dependency vulnerability audit,
- license inventory,
- software bill of materials,
- pinned CI actions and tool versions where practicable,
- artifact checksums,
- source/build identity,
- secret scanning,
- no unexpected network access in tests.

### PRD-NFR-007 — Compatibility

Stable public schemas and exit codes SHALL follow semantic versioning. Breaking changes require a major-version or explicitly versioned parallel schema. Deprecated fields SHALL remain parseable for at least one minor release unless a security issue requires immediate removal.

### PRD-NFR-008 — Portability

The core SHALL remain portable across supported native Rust targets. Python SHALL use the documented ABI3 floor. Browser WASM SHALL remain free from ambient filesystem, process, and network authority.

### PRD-NFR-009 — Operational recovery

The release SHALL document rollback, cache invalidation, replay, corrupt-artifact refusal, and incident evidence collection. Recovery procedures SHALL not depend on undocumented maintainer knowledge.

### PRD-NFR-010 — Test isolation

Tests SHALL be deterministic by default and SHALL not depend on live external services. Chaos, mutation, negative, replay, and compatibility fixtures SHALL be committed and independently runnable.

## 9. Production-readiness admission gates

A capability is `ADMITTED` only when all applicable gates pass:

| Gate | Evidence |
|---|---|
| Identity | Unique capability ID, version, owner, manifest fingerprint |
| API | Versioned schema, documented compatibility and deprecation policy |
| Correctness | Unit, integration, negative, property, and independent validation tests |
| Determinism | Replay corpus and fingerprint equality where declared |
| Bounds | Input, time, node, worker, progress, plan, and output limits |
| Failure | Typed taxonomy and fail-closed tests |
| Authority | Candidate-only classification and no ambient actuation |
| Security | Audit, SBOM, license, secret, and untrusted-input checks |
| Observability | Structured, redacted request and terminal telemetry |
| Recovery | Cancellation, timeout, replay, rollback, and corrupt-state refusal |
| Compatibility | Cross-surface parity and schema/exit-code fixtures |
| Operations | Runbook, readiness output, benchmark envelope, support ownership |

A passing compiler or unit-test suite is necessary but insufficient.

## 10. Readiness states

- `UNKNOWN`: no authoritative evaluation exists.
- `DECLARED`: capability contract exists but implementation evidence is incomplete.
- `PARTIAL`: implementation exists and some gates pass.
- `ADMITTED`: every applicable gate has executable evidence at the exact source identity.
- `BLOCKED`: a required dependency, implementation, or evidence class is absent.
- `UNSUPPORTED`: the capability is intentionally outside the current product contract.
- `REFUSED`: evidence proves the capability violates a gate.

`ADMITTED` SHALL be computed by verification logic. Producers SHALL NOT set it directly.

## 11. Release acceptance criteria

The readiness release is accepted only when:

1. The PRD and ARD are committed before implementation changes.
2. The canonical capability manifest validates and fingerprints deterministically.
3. The readiness CLI reports no advertised capability as absent.
4. All shipped surfaces pass cross-surface semantic fixtures.
5. All service-facing operations have bounded defaults and typed terminal outcomes.
6. The MCP+ surface exists or all MCP claims are removed; this program targets implementation.
7. Full native, Bevy, browser WASM, Python, plugin, documentation, and MCP+ validation passes.
8. Security, SBOM, license, provenance, and artifact-integrity evidence is emitted.
9. No open critical/high defect remains in a shipped capability.
10. The final admission report is bound to the exact source commit and capability-manifest fingerprint.

## 12. Explicit non-goals

This program does not:

- certify compliance with a named statutory or industry framework,
- claim production use by a Fortune 5 company,
- make the planner an execution-authority system,
- allow LLM output to become authoritative without external admission,
- turn the GUI into a privileged control plane,
- add arbitrary network or operating-system actuation,
- replace external process observation, conformance, receipt, or replay systems.

## 13. Success metric

The primary metric is not the number of interfaces or tests. It is:

```text
admitted capabilities / advertised capabilities = 1.0
```

with every admission supported by exact-source, executable, independently recomputable evidence.
