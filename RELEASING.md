# Releasing ferroplan

The workspace publishes **two** crates to crates.io: the library `ferroplan` and
the CLI `ferroplan-cli` (the `ff` binary), which depends on the library. They must
be published **in that order**.

> **TL;DR:** after `cargo login <token>`, run [`./publish.sh`](publish.sh) from a
> machine with crates.io access — it runs the full pre-flight below, then publishes
> both crates in order and tags `vX.Y.Z`. `./publish.sh --dry-run` does the
> pre-flight + a packaging check without uploading. The steps below are the manual
> equivalent.

## Pre-flight (all must pass)

**Update the toolchain FIRST — always run the pre-flight on the latest
stable Rust:**

```sh
rustup update stable
```

Clippy grows new lints with every release and the pre-flight is
`-D warnings`, so a dev box on an older toolchain will pass locally and
then fail `publish.sh` on the release machine. This has bitten twice
(most recently 1.94 vs 1.97, `collapsible_match`): green on latest
stable is the only green that counts.

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo bench --no-run
```

## Bump the version

The workspace crates share `version` via `[workspace.package]` in the root
`Cargo.toml`. Bump it there, and update BOTH dependency pins on `ferroplan`
(`version = "X.Y.Z"`): `crates/ferroplan-cli/Cargo.toml` and
`crates/ferroplan-mcp/Cargo.toml` — a stale pin fails workspace resolution.
Re-lock the workspace-excluded `crates/ferroplan-py/Cargo.lock` too
(`cargo update -w --manifest-path crates/ferroplan-py/Cargo.toml`). Commit
and tag (`vX.Y.Z`).

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
