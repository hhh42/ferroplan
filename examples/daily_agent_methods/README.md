# Daily agent methods PPDDL corpus

This fixture hardcodes reusable operating patterns extracted from admitted agent work on **2026-07-31 America/Los_Angeles**.

The corpus is deliberately split into three authority surfaces:

- `domain.ppddl` — reusable action schemas, refusals, receipt gates, replay, closure, and probabilistic external edges;
- `problem-2026-07-31.ppddl` — the dated admitted observation snapshot;
- `method-catalog.json` — evidence-bound pattern names, laws, exclusions, and hardcoding contract.

## Law

The encoded control flow is:

```text
observe -> admit -> manufacture -> verify -> receipt -> replay -> standing
```

Agent completion does not promote subsystem standing. Unknown work remains unknown. Generated projections cannot authorize their own actuation. Crown standing requires closure, valid receipts, and clean-room replay.

## Probability boundary

The four `0.25` outcomes on `attempt-external-verification` are **uninformative planning priors** used to exercise the PPDDL rail. They are not telemetry or empirical estimates. Production use must replace them with admitted measurements.

## Verification

The Rust integration test calls Ferroplan's public `parse_ppddl` API against the committed domain and problem. It also supplies an invalid probability-mass domain and requires typed refusal.

```bash
cargo test -p ferroplan --test daily_agent_methods_ppddl
```

Policy synthesis is intentionally not a unit-test requirement for the full dated snapshot: the snapshot is an evidence corpus, not a claim that its crown goal is cheaply reachable under every solver bound.

## Standing

`PARTIAL_ALIVE` until exact-head CI executes the parser/refusal test. A green test receipts syntax and semantic admission only; it does not establish empirical probabilities or crown-level standing for the source repositories named in the snapshot.
