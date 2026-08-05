# Production PPDDL execution

`ferroplan-ppddl` is the production command-line surface for bounded PPDDL policy synthesis. It uses the same `ferroplan` library APIs as embedded consumers and does not depend on GitHub Actions or any hosted service.

## Build

```sh
cargo build --release -p ferroplan-cli --bin ferroplan-ppddl
```

## Execute the reusable future corpus

```sh
target/release/ferroplan-ppddl \
  --domain examples/daily_agent_methods/future/domain.ppddl \
  --problem examples/daily_agent_methods/future/problem.ppddl \
  --horizon 64 \
  --episodes 1000 \
  --seed 42 \
  > future-policy-receipt.json
```

The command performs, in order:

1. bounded explicit-MDP policy synthesis through `ferroplan::solve_ppddl`;
2. independent structural policy validation through `ferroplan::validate_ppddl_policy`;
3. deterministic seeded simulation through `ferroplan::simulate_ppddl`;
4. BLAKE3 binding of the exact domain and problem inputs;
5. machine-readable receipt emission.

The process exits `0` only when policy synthesis reports `solved: true` and the independent validator reports a valid policy. Refusals caused by state, transition, outcome, policy, value-table, or initial-distribution bounds remain typed `PpddlError` failures and are never converted into success.

## No-CI acceptance ladder

Run locally against the exact checkout:

```sh
cargo fmt --check -- crates/ferroplan-cli/src/bin/ferroplan-ppddl.rs
cargo check -p ferroplan-cli --bin ferroplan-ppddl
cargo test -p ferroplan ppddl::
target/debug/ferroplan-ppddl \
  -o examples/daily_agent_methods/future/domain.ppddl \
  -f examples/daily_agent_methods/future/problem.ppddl \
  --episodes 1000 --seed 42 \
  > /tmp/future-policy-receipt.json
python3 -m json.tool /tmp/future-policy-receipt.json >/dev/null
```

Hosted CI status is neither required nor admitted as evidence for this path.
