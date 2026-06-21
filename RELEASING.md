# Releasing ferroplan

The workspace publishes **two** crates to crates.io: the library `ferroplan` and
the CLI `ferroplan-cli` (the `ff` binary), which depends on the library. They must
be published **in that order**.

## Pre-flight (all must pass)

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo bench --no-run
```

## Bump the version

Both crates share `version` via `[workspace.package]` in the root `Cargo.toml`.
Bump it there, and update `ferroplan-cli`'s dependency pin on `ferroplan`
(`version = "X.Y.Z"`) to match. Commit and tag (`vX.Y.Z`).

## Publish (order matters)

```sh
# 1. the library first — the CLI depends on it
cargo publish -p ferroplan

# 2. then the CLI (now that `ferroplan` is on the index)
cargo publish -p ferroplan-cli
```

> A `cargo publish -p ferroplan-cli --dry-run` BEFORE the library is on crates.io
> fails with `no matching package named 'ferroplan' found` — this is expected, not
> a packaging bug. Verify the CLI with `cargo build -p ferroplan-cli` instead, and
> publish it only after step 1 has landed.

Each crate package bundles `README.md` and both `LICENSE-*` files (symlinked into
the crate dirs) so the crates.io page and tarball are complete.

## After publishing

- Push the tag; the `pages.yml` workflow rebuilds the mdBook site.
- The external comparison oracles (Metric-FF, SGPlan6, VAL) and IPC benchmark
  instances are **not** part of any published crate — see
  [`benchmarks/COMPARING.md`](benchmarks/COMPARING.md).
