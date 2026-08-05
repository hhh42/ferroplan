# Eve and the Genesis planning lifecycle

`ferroplan` began with a deterministic-planning claim: an LLM should author and
supervise a formal planner rather than impersonate the planner at runtime. Eve
extends that claim outward to the human boundary.

> **Genesis creates the world. Eve makes the world enterable.**

Eve is not an agent persona and not an actuator. Eve is the relational interface
through which a person expresses purpose against a world that already has formal
identity, process, constraints, uncertainty, manufacturing, and consequence law.

The complete handoff is:

```text
human purpose
    -> Eve
    -> RDF Genesis world
    -> SPARQL CONSTRUCT projection
    -> HDDL hierarchical decomposition
    -> PPDDL policy when uncertainty is explicit
    -> ggen manufacturing request
    -> MCP+ capability handoff
    -> BRCE actuation
    -> OCEL 2.0 observation
    -> Truex Kernel conformance
    -> receipt admission/refusal
    -> replay
```

The core crate exposes this as `Eve::enter`. It performs no tool call and grants
no authority. It validates the relational inputs and emits a deterministic,
`serde`-serializable `EveHandoff` that downstream adapters can execute.

## Why Genesis precedes Eve

The interface cannot invent the world it claims to expose. The ontology,
planning vocabulary, process geometry, constraints, and manufacturing targets
must exist before Eve can ground human purpose against them.

| Layer | Responsibility |
|---|---|
| RDF | Canonical created world |
| SPARQL CONSTRUCT | Smallest relevant lawful world projection |
| HDDL | Hierarchical purpose decomposition |
| PPDDL | Bounded policy under explicit uncertainty |
| ggen | Manufacture the required operational projection |
| MCP+ | Expose a manufactured capability without ambient authority |
| BRCE | Exclusive actuation boundary |
| OCEL 2.0 | Observed object-centric path |
| Truex Kernel | Conformance, refusal, and replay evidence |
| Receipt | Admission or refusal authority |

## Example

```rust
use ferroplan::{
    Activator, CapabilityTarget, Eve, EveRequest, GenesisWorld, HddlSurface,
    HumanPurpose, ManufactureTarget,
};

let handoff = Eve::enter(EveRequest {
    purpose: HumanPurpose {
        statement: "Deploy the service safely".into(),
        desired_consequence: "A replayable admitted deployment".into(),
        actor: Some("operator".into()),
        activators: vec![Activator {
            name: "environment".into(),
            value: "production".into(),
        }],
    },
    genesis: GenesisWorld {
        ontology_rdf: "@prefix fp: <urn:ferroplan:> .".into(),
        construct_query: "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }".into(),
        hddl: HddlSurface {
            domain: "(define (domain deploy) (:task deploy-service))".into(),
            problem: "(define (problem deploy-prod) (:domain deploy))".into(),
            root_task: "(deploy-service production)".into(),
        },
        ppddl: None,
    },
    manufacture: ManufactureTarget {
        name: "deploy-service.part".into(),
        template: "ggen://truex/part".into(),
        artifact_kind: ".part.wasm".into(),
        output: "target/parts/deploy-service.part.wasm".into(),
    },
    capability: CapabilityTarget {
        capability: "deploy-service".into(),
        route: "powl://deploy-service/v1".into(),
        authority_scopes: vec!["production:deploy".into()],
    },
})?;

assert!(!handoff.mcp_plus.ambient_authority);
assert!(handoff.mcp_plus.brce_required);
assert!(handoff.ggen.candidate_only);
assert!(handoff.truex.replay_required);
# Ok::<(), ferroplan::EveError>(())
```

## Need9 means split

Eve accepts no more than eight primary ingress activators. A ninth activator is
not hidden in a larger prompt or silently discarded:

```text
Need9 => Split
```

`Eve::enter` returns `EveError::SplitRequired` with deterministic groups of at
most eight activators. The caller must form smaller lawful closures.

## Authority boundary

An `EveHandoff` is a contract, not a receipt. Its deterministic `closure_id`
identifies identical relational inputs, but does not prove execution. The handoff
explicitly preserves these invariants:

- ggen manufactures a candidate and cannot self-admit it;
- MCP+ carries no ambient authority;
- actuation requires BRCE;
- observed motion requires OCEL 2.0 evidence;
- workflow standing requires POWL conformance;
- completion requires admission or refusal receipt plus replay.

Eve makes the created world usable by a human while leaving authority in the
formal layers that own it.
