# Ferroplan planning runtime

Ferroplan now exposes one typed constitution and one bounded universal runtime
for all eighteen admitted planning families. The runtime produces plans,
policies, decompositions, assignments, or capability bindings. It never
actuates them.

| Planning type | Implementation |
|---|---|
| classical | Unit-cost shortest path |
| cost optimal | Dijkstra over action costs |
| numeric | Numeric-goal state search |
| temporal | Duration-minimizing search with timestamps |
| preferences | Hard-goal search with soft-goal penalties |
| probabilistic | Bounded value iteration |
| FOND | Strong winning-state fixed point |
| conformant | Belief-state breadth-first search |
| contingent | Observation-branching AND-OR policy search |
| hierarchical | Recursive task/method expansion with cycle bounds |
| partial order | Stable DAG topological planning |
| workflow | Topological planning with cycle refusal |
| flow constrained | Queue/WIP admission plus planning |
| resolution adaptive | Hierarchical expansion to primitive closure |
| multi-agent | Capability- and capacity-aware assignment |
| RDF derived | Bounded RDF-to-state-space projection and solve |
| A2A delegated | Primitive decomposition and agent assignment |
| MCP bound | Authorized, verified, receipted tool binding |

The implementation is in `crates/ferroplan/src/planning_runtime.rs`. The public
entrypoint is `solve_planning_type`.

## Hard boundaries

All searches have state, depth, or iteration bounds. Invalid probability mass,
unknown identities, hierarchy cycles, workflow cycles, WIP overflow,
uncovered capabilities, and missing MCP authority/verifier/receipt bindings
produce typed errors.

Planner output has no execution authority. BRCE remains outside this module and
is the exclusive consequential path.

## Validation

The `Planning Runtime Crown` workflow executes:

```bash
cargo fmt -p ferroplan -- --check
cargo check -p ferroplan --all-targets --all-features
cargo clippy -p ferroplan --all-targets --all-features -- -D warnings
cargo test -p ferroplan --test planning_types
cargo test -p ferroplan --test planning_runtime
cargo test -p ferroplan --all-features
```

The runtime remains compatible with the repository MSRV, Rust 1.74.
