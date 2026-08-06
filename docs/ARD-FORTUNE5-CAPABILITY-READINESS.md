# Architecture Requirements Document: Fortune 5 Capability Readiness

**Product:** ferroplan  
**Document ID:** FP-ARD-F5-001  
**Status:** Implementation contract  
**Depends on:** FP-PRD-F5-001  
**Target release:** 0.22.x readiness line  
**Last updated:** 2026-08-05

## 1. Architectural objective

This document specifies the architecture required to satisfy the Fortune 5 Capability Readiness PRD. It converts readiness from an informal quality label into a deterministic admission system.

The governing equation is:

```text
Capability declaration
→ bounded implementation
→ executable evidence
→ independent verification
→ exact-source admission report
```

No component may infer production standing from existence, successful compilation, a green test subset, or a human-authored status field.

## 2. Governing invariants

1. **Candidate is not consequence.** A plan, handoff, explanation, generated file, or protocol response is non-authoritative until an external execution-trust lifecycle admits its consequence.
2. **SELECT ≠ CONSTRUCT ≠ DO.** Discovery, manufacture, and actuation remain distinct authority domains.
3. **No ambient authority.** Connectivity, local process access, or tool discovery grants no permission to actuate.
4. **Fail closed.** Missing evidence, invalid validation, worker failure, truncation, and saturation terminate as typed non-success outcomes.
5. **Bound before parse.** Every untrusted input boundary enforces byte and structural limits before expensive allocation or search.
6. **One semantic core.** Rust, CLI, Python, WASM, GUI, and MCP+ adapt one planning contract rather than reimplementing planner semantics.
7. **Independent validation.** Search success and plan validity are separate facts.
8. **Declared determinism.** Determinism is scoped to canonical input, options, seed, worker topology, algorithm, and build identity.
9. **Evidence before admission.** Readiness state is verifier-derived.
10. **Replay before standing.** A deterministic claim requires replay evidence at exact source identity.

## 3. System context

```text
                              ┌──────────────────────────┐
Human purpose / formal input │ Rust API / CLI / Python │
────────────────────────────▶│ WASM / Bevy / MCP+      │
                              └─────────────┬────────────┘
                                            │ bounded request
                                            ▼
                              ┌──────────────────────────┐
                              │ Public contract boundary │
                              │ limits + IDs + envelope  │
                              └─────────────┬────────────┘
                                            │
                                            ▼
                              ┌──────────────────────────┐
                              │ ferroplan semantic core  │
                              │ parse/ground/search      │
                              │ validate/explain/hash    │
                              └─────────────┬────────────┘
                                            │ candidate result
                                            ▼
                              ┌──────────────────────────┐
                              │ Result envelope          │
                              │ evidence + limits        │
                              │ candidate authority      │
                              └─────────────┬────────────┘
                                            │ optional downstream
                                            ▼
                              ┌──────────────────────────┐
                              │ BRCE / POWL / OCEL       │
                              │ Truex receipt + replay   │
                              └──────────────────────────┘
```

ferroplan ends at bounded candidate planning and evidence. External systems own real-world actuation and authoritative receipt closure.

## 4. Capability architecture

### 4.1 Typed capability definitions

A new core module SHALL define capabilities as typed immutable data. The source representation SHALL be the only authoritative capability inventory.

```rust
pub struct CapabilityContract {
    pub id: CapabilityId,
    pub version: SemVer,
    pub owner: OwnerId,
    pub component: ComponentId,
    pub interface: InterfaceKind,
    pub authority: AuthorityClass,
    pub determinism: DeterminismClass,
    pub replay: ReplayClass,
    pub input_schema: SchemaId,
    pub output_schema: SchemaId,
    pub resource_profile: ResourceProfileId,
    pub failure_contract: FailureContractId,
    pub telemetry_contract: TelemetryContractId,
    pub compatibility: CompatibilityClass,
    pub security: SecurityClass,
    pub evidence: Vec<EvidenceId>,
}
```

Readiness state SHALL NOT be writable inside `CapabilityContract`.

### 4.2 Verifier-derived admission

A separate verifier SHALL evaluate contracts and evidence:

```rust
pub fn evaluate_capability(
    contract: &CapabilityContract,
    evidence: &EvidenceIndex,
) -> CapabilityEvaluation;
```

`CapabilityEvaluation` SHALL contain:

- state,
- satisfied gates,
- unsatisfied gates,
- refusal reasons,
- exact source/build identity,
- canonical contract fingerprint,
- evaluator version.

Only the evaluator may emit `ADMITTED`.

### 4.3 Canonical manifest

The manifest SHALL be serialized with:

- stable capability ordering by ID,
- stable field ordering through typed serialization,
- no timestamps inside fingerprinted content,
- explicit schema version,
- UTF-8 normalization policy,
- domain-separated SHA-256 fingerprint,
- length-prefixed variable fields where custom hashing is used.

The fingerprint domain SHALL be distinct from Eve closure IDs, PDDL problem fingerprints, artifacts, and receipts.

## 5. Public request and result contracts

### 5.1 Request context

All service-facing adapters SHALL construct a bounded request context:

```rust
pub struct RequestContext {
    pub request_id: RequestId,
    pub capability_id: CapabilityId,
    pub deadline: Option<Deadline>,
    pub cancellation: CancellationToken,
    pub limits: ResourceLimits,
    pub telemetry: TelemetryPolicy,
}
```

Adapters may generate a request ID when none is supplied. A request ID is correlation metadata, not authority or receipt evidence.

### 5.2 Resource limits

```rust
pub struct ResourceLimits {
    pub max_input_bytes: usize,
    pub max_tokens: usize,
    pub max_nesting_depth: usize,
    pub max_identifiers: usize,
    pub max_identifier_bytes: usize,
    pub max_search_nodes: u64,
    pub max_plan_steps: usize,
    pub max_output_bytes: usize,
    pub max_progress_events_per_second: u32,
    pub max_workers: usize,
    pub timeout: Duration,
}
```

Requirements:

- Safe shared-infrastructure defaults SHALL exist.
- Adapter-specific stricter values are permitted.
- Limits SHALL be validated before work begins.
- Limit termination SHALL carry the exact breached dimension.
- Overflow and lossy conversions SHALL refuse rather than wrap.

### 5.3 Result envelope

```rust
pub struct OperationEnvelope<T> {
    pub schema_version: String,
    pub request_id: RequestId,
    pub capability_id: CapabilityId,
    pub capability_version: String,
    pub build_identity: BuildIdentity,
    pub input_fingerprint: String,
    pub authority: AuthorityClass,
    pub outcome: OutcomeClass,
    pub validation: ValidationStatus,
    pub elapsed_micros: u64,
    pub counters: BoundedCounters,
    pub warnings: Vec<Diagnostic>,
    pub payload: Option<T>,
    pub error: Option<PublicError>,
}
```

The envelope SHALL satisfy:

- exactly one of payload or error for terminal outcomes,
- no success when independent validation fails,
- no raw secrets or entire input echo,
- bounded diagnostics and payload,
- explicit candidate-only authority,
- stable schema version.

## 6. Failure architecture

### 6.1 Internal and public errors

Internal errors may retain rich causal chains. Public errors SHALL use stable codes:

```text
FP_INVALID_REQUEST
FP_UNSUPPORTED
FP_PARSE
FP_MODEL
FP_LIMIT_INPUT
FP_LIMIT_STRUCTURE
FP_LIMIT_SEARCH
FP_LIMIT_PLAN
FP_TIMEOUT
FP_CANCELLED
FP_NO_PLAN
FP_VALIDATION
FP_ADAPTER
FP_DEPENDENCY
FP_INVARIANT
```

Every code SHALL define:

- retryability,
- operator action,
- whether input correction is possible,
- safe public detail fields,
- telemetry severity,
- HTTP-equivalent class for service adapters without requiring HTTP.

### 6.2 Panic containment

- Library code SHALL return typed errors for expected failures.
- Service adapters SHALL contain panics at the request boundary.
- Worker panic SHALL terminate the request as `FP_INVARIANT`.
- Panic payloads SHALL not be emitted to untrusted clients.
- Process-wide abort behavior SHALL be documented for unrecoverable runtime configuration.

### 6.3 Partial-result prohibition

A partial plan may be emitted only through an explicitly typed progress or diagnostic channel. It SHALL NOT populate the terminal success payload.

## 7. Determinism and replay architecture

### 7.1 Canonical input identity

Canonical fingerprints SHALL bind:

- exact domain bytes or canonical model representation,
- exact problem bytes or canonical model representation,
- algorithm and heuristic,
- relevant options,
- seed,
- worker count/topology when relevant,
- capability/schema version.

### 7.2 Replay classes

- `Exact`: same canonical input and declared environment yields identical terminal payload and fingerprint.
- `Outcome`: same canonical input yields the same terminal class and a valid equivalent plan; ordering may differ.
- `NonReplayable`: intentionally nondeterministic; prohibited for production service defaults unless explicitly requested.

### 7.3 Replay corpus

The repository SHALL commit replay fixtures covering:

- solved deterministic problem,
- no-plan problem,
- invalid syntax,
- unsupported feature,
- timeout,
- cancellation,
- node limit,
- plan-length limit,
- parallel outcome equivalence,
- Eve deterministic closure,
- cross-surface parity.

Replay evidence SHALL record exact source and manifest fingerprints.

## 8. Core architecture changes

### 8.1 API layering

The core SHALL expose:

1. Existing ergonomic APIs for compatibility.
2. A production API accepting `RequestContext` and returning `OperationEnvelope`.
3. Internal planning functions isolated from adapter concerns.

Compatibility wrappers SHALL call the production core rather than maintaining a second execution path.

### 8.2 Independent plan validation

The production API SHALL validate a returned plan against the parsed model before setting terminal outcome to solved. A failed validator converts the result to `FP_VALIDATION` or `FP_INVARIANT` depending on cause.

### 8.3 Cancellation and budget checks

Budget checks SHALL occur:

- before parsing,
- after parsing/model normalization,
- during grounding boundaries,
- at bounded search intervals,
- before explanation/serialization,
- before adapter write.

Cancellation frequency SHALL be sufficient to meet the documented shutdown envelope without making benchmark claims independent of workload.

## 9. Adapter architecture

### 9.1 CLI

The CLI SHALL:

- support `--output human|json`,
- emit structured output only on stdout in JSON mode,
- emit diagnostics only on stderr,
- expose `readiness`, `version`, and capability discovery,
- use stable exit codes,
- accept bounded timeout/node/plan/input/worker limits,
- handle SIGINT as cancellation where supported.

Proposed exit classes:

| Code | Meaning |
|---|---|
| 0 | Successful operation or readiness fully admitted |
| 2 | Invalid request or parse/model error |
| 3 | No plan / valid negative outcome |
| 4 | Unsupported capability |
| 5 | Resource limit or timeout |
| 6 | Cancelled |
| 7 | Validation or adapter failure |
| 70 | Internal invariant failure |

### 9.2 Python ABI3

The Python adapter SHALL:

- preserve ABI3 floor,
- map public failure codes to a typed exception hierarchy,
- expose structured dict/JSON envelopes,
- release the GIL during bounded planning where safe,
- reject oversized input before copying into planner structures,
- support timeout and limits,
- avoid process-global mutable configuration.

### 9.3 Browser WASM

The WASM adapter SHALL:

- expose only pure planning/validation/readiness functions,
- enforce browser-specific limits,
- return structured serializable errors,
- avoid ambient filesystem/process/network access,
- remain deterministic under declared single-thread mode,
- support cooperative progress/cancellation where the host API permits.

### 9.4 Bevy GUI

The GUI SHALL be an unprivileged client of the same adapter contract. It SHALL:

- keep planning off the render thread,
- support cancellation,
- display outcome, validation, limits, build identity, and candidate status,
- avoid reporting a successful consequence merely because a plan renders,
- bound user-pasted input and rendered output.

### 9.5 MCP+

The MCP+ crate SHALL be a bounded stdio JSON-RPC server. Architectural rules:

- stdout is protocol-only;
- stderr is redacted structured telemetry;
- maximum frame bytes are checked before deserialization;
- one request maps to one `RequestContext`;
- concurrency is bounded by a semaphore or equivalent;
- deadlines and cancellation propagate to core planning;
- tool discovery grants no execution authority;
- allowed operations are `plan`, `validate`, `explain`, `readiness`, and `version`;
- arbitrary command, filesystem mutation, network access, and subprocess execution are absent;
- responses explicitly identify candidate authority and downstream obligations.

The initial implementation SHALL document its protocol profile as ferroplan MCP+ and SHALL NOT claim conformance to an external MCP protocol revision without separate protocol tests.

### 9.6 Plugin control plane

The plugin SHALL consume capability/readiness information rather than maintain a divergent handwritten inventory. Its verifier SHALL refuse absent, stale, or unverifiable capability claims.

## 10. Eve / Genesis architecture

Eve remains the relational boundary through which human purpose enters the formal world.

```text
HumanPurpose
→ bounded Eve request
→ Genesis asset validation
→ relevant closure declaration
→ HDDL decomposition
→ optional PPDDL policy
→ ggen candidate target
→ MCP+ candidate route
→ downstream obligations
```

Readiness additions:

- byte and field limits,
- exact source fingerprints,
- typed refusals,
- no empty identity-equivalent ambiguity,
- explicit schema/version fields,
- deterministic JSON fixtures,
- no authority escalation,
- closure IDs separate from receipts and readiness fingerprints.

## 11. Observability architecture

### 11.1 Event model

Adapters SHALL emit bounded structured events:

```text
request.accepted
request.refused
parse.completed
model.completed
search.started
search.progress
search.saturated
validation.completed
request.completed
request.cancelled
request.failed
```

Each event includes correlation ID, capability ID/version, build identity, monotonic duration, bounded counters, and outcome. Event payloads exclude raw input by default.

### 11.2 Metrics

Minimum metrics:

- requests by capability and outcome,
- active operations,
- parse/model/search/validation duration,
- expanded/generated states,
- timeout/cancellation/limit counts,
- output truncation count,
- panic containment count,
- readiness state by capability/build.

The core SHALL not require a specific telemetry vendor. It SHALL provide a stable event/sink abstraction.

### 11.3 Data handling

Telemetry modes:

- `Off`
- `MetricsOnly`
- `Redacted`
- `AuthorizedReplay`

`AuthorizedReplay` requires explicit caller configuration and SHALL not be the default.

## 12. Security architecture

### 12.1 Trust boundaries

Untrusted boundaries:

- CLI files/stdin,
- Python strings/objects,
- browser JavaScript values,
- GUI text/file input,
- MCP+ frames,
- plugin payloads,
- PDDL/RDF/SPARQL/HDDL/PPDDL text.

All are data, never authority.

### 12.2 Attack controls

Required controls:

- input byte and structure limits,
- integer overflow refusal,
- bounded diagnostic/output lengths,
- no shell interpolation,
- no command execution in MCP+,
- no filesystem traversal by protocol input,
- no network access by default,
- panic containment at service boundaries,
- dependency and license audit,
- secret-safe logs,
- malformed/fuzz corpus,
- denial-of-service budget tests.

### 12.3 Build and artifact provenance

Release evidence SHALL include:

- source commit,
- dirty-state assertion,
- Rust/toolchain identity,
- lockfile fingerprint,
- capability-manifest fingerprint,
- SBOM,
- dependency audit result,
- license inventory,
- artifact checksums,
- test/admission report.

A checksum proves artifact identity, not functional correctness or execution admission.

## 13. Compatibility architecture

### 13.1 Version dimensions

The product SHALL version independently:

- crate/package release,
- capability manifest schema,
- operation envelope schema,
- MCP+ protocol profile,
- Eve handoff schema,
- plugin schema.

### 13.2 Compatibility fixtures

Committed golden fixtures SHALL validate:

- CLI JSON,
- Python JSON/dict,
- WASM JSON,
- MCP+ JSON-RPC,
- readiness manifest,
- Eve handoff,
- error envelope,
- version discovery.

Golden changes require an explicit compatibility review.

## 14. Release and deployment architecture

### 14.1 Build matrix

The release gate SHALL build and test:

- native workspace including Bevy,
- all targets/features where supported,
- release and debug core tests,
- ignored heavy regression corpus,
- browser planner WASM,
- browser Bevy WASM,
- Python ABI3 wheel and import smoke test,
- MCP+ protocol/integration tests,
- plugin test and verifier matrix,
- mdBook and Rust API docs,
- benchmark compilation and readiness evidence.

### 14.2 Promotion stages

```text
source candidate
→ compiled
→ tested
→ negative-tested
→ replay-verified
→ cross-surface verified
→ security-evidenced
→ capability-admitted
→ release candidate
```

No stage inherits the standing of the next stage.

### 14.3 Rollback

Release artifacts SHALL be immutable. Rollback selects the prior admitted artifact and manifest pair. Operators SHALL not combine a binary from one release with a capability manifest from another.

## 15. Operational recovery

Required runbooks:

- repeated timeout or saturation,
- memory-pressure response,
- corrupt/stale cache refusal,
- invalid or divergent plan result,
- adapter protocol desynchronization,
- Python/WASM compatibility regression,
- MCP+ malformed-frame flood,
- release artifact mismatch,
- readiness regression after dependency update.

Recovery SHALL prefer refusal and rollback over silent degradation.

## 16. Verification strategy

### 16.1 Test classes

Every production surface SHALL have applicable evidence from:

- unit tests,
- integration tests,
- negative fixtures,
- property tests,
- replay tests,
- cross-surface contract tests,
- mutation or adversarial tests,
- malformed-input tests,
- bounded-load tests,
- compatibility/golden tests,
- build/install/import tests.

### 16.2 Capability gate mapping

The verifier SHALL map each capability to exact test/evidence IDs. Filename presence alone is not evidence. Evidence records SHALL include the command, expected class, exact source identity, and result digest.

### 16.3 No self-certification

Generated readiness JSON is a claim until the independent admission command recomputes:

- source/build identity,
- manifest fingerprint,
- required evidence presence,
- schema validity,
- all gate results.

## 17. Architecture decisions

### ADR-F5-001 — Typed manifest over handwritten inventory

**Decision:** Capability identity and contracts live in typed Rust definitions; documentation and JSON are projections.  
**Reason:** Handwritten parallel inventories drift.  
**Consequence:** Every adapter can query the same canonical source.

### ADR-F5-002 — Production wrapper over breaking API replacement

**Decision:** Add bounded production APIs and retain compatibility wrappers.  
**Reason:** Existing users require compatibility while service surfaces require stronger contracts.  
**Consequence:** Wrappers must delegate to the same semantic core and are tested for parity.

### ADR-F5-003 — MCP+ is candidate-only

**Decision:** MCP+ exposes planning tools but no arbitrary execution or ambient authority.  
**Reason:** Tool discovery cannot become actuation authority.  
**Consequence:** Real-world execution remains downstream of BRCE and receipt closure.

### ADR-F5-004 — Verifier computes admission

**Decision:** Capability source definitions cannot set `ADMITTED`.  
**Reason:** Producer-authored acceptance violates separation of duties.  
**Consequence:** Missing evidence produces `PARTIAL` or `BLOCKED` automatically.

### ADR-F5-005 — Redacted observability by default

**Decision:** Raw planning and ontology inputs are excluded from default telemetry.  
**Reason:** Enterprise inputs may contain proprietary or sensitive information.  
**Consequence:** Full replay capture requires explicit authorization.

### ADR-F5-006 — Explicit bounded defaults

**Decision:** Service-facing surfaces refuse unbounded work by default.  
**Reason:** Shared infrastructure cannot rely on caller discipline.  
**Consequence:** Ergonomic internal APIs may retain configurable larger limits, but adapters always construct a bounded context.

### ADR-F5-007 — Exact protocol claims only

**Decision:** The new stdio service is named ferroplan MCP+ until external MCP revision conformance is separately verified.  
**Reason:** Protocol branding without exact-version evidence is an unsupported claim.  
**Consequence:** The interface remains useful and testable without overclaiming compatibility.

## 18. Requirement traceability

| PRD requirement | Architecture realization |
|---|---|
| PRD-FR-001 | Typed capability definitions, canonical manifest, verifier |
| PRD-FR-002 | CLI readiness command |
| PRD-FR-003 | `OperationEnvelope<T>` |
| PRD-FR-004 | Public failure taxonomy |
| PRD-FR-005 | `ResourceLimits` and adapter profiles |
| PRD-FR-006 | Request context cancellation/deadline |
| PRD-FR-007 | Determinism and replay classes |
| PRD-FR-008 | Independent validator gate |
| PRD-FR-009 | Bounded Eve contract |
| PRD-FR-010 | Bounded candidate-only MCP+ server |
| PRD-FR-011 | Golden cross-surface fixtures |
| PRD-FR-012 | Version/build discovery |
| PRD-FR-013 | Replay metadata and telemetry modes |
| PRD-FR-014 | Manifest/documentation truth gate |
| PRD-NFR-001..010 | CI, security, observability, compatibility, and operations rails |

## 19. Definition of architecture complete

The architecture is complete only when:

1. Every in-scope capability has a typed contract.
2. Every public adapter delegates to the bounded production core.
3. Every public failure has a stable code and tested mapping.
4. Limits and cancellation reach active search boundaries.
5. Cross-surface fixtures prove semantic parity.
6. MCP+ exists as a bounded non-actuating capability surface.
7. Security and release evidence are generated at exact source identity.
8. The independent verifier computes all advertised capabilities as `ADMITTED`.

Until those conditions hold, the architecture is specified but the product is not admitted.
