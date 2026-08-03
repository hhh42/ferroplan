# Releasing ferroplan

The workspace publishes **three** crates to crates.io: the library `ferroplan`,
the CLI `ferroplan-cli` (the `ff` binary), and the MCP server `ferroplan-mcp` —
the latter two depend on the library. They must be published **in that order**
(library first).

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

## Refresh the record (standings, history, release notes)

Three scripted steps, in this order. All are idempotent, so re-running is safe.

```sh
# 1. Promote the sweep, regenerate the tables. standings.py writes
#    benchmarks/ipc-standings.md (the reference table), STANDINGS.md (the
#    at-a-glance view) AND patches the README's headline block — one command,
#    so the front page can never disagree with the boards behind it.
bash benchmarks/promote-air.sh     # refuses a partial sweep

# 2. Bank this release's numbers, so the NEXT release can show movement.
python3 scripts/standings-snapshot.py --version X.Y.Z --measured-at YYYY-MM-DD

# 3. Trim the release notes: CHANGELOG.md keeps [Unreleased] + the newest two
#    (older move to CHANGELOG-ARCHIVE.md, verbatim); README keeps the newest
#    two "What's new" blocks.
python3 scripts/release-notes-roll.py
```

Why these are gates and not chores:

- **The front page rots fastest.** The README had accumulated SIXTEEN
  "What's new" blocks — ~308 lines, 45% of the file — so a visitor met a year
  of history before learning whether the planner is any good. Both the
  changelog and the README are now trimmed by script, not by discipline.
- **`measured_at` is not the release date.** Coverage at a fixed time budget
  is a property of the hardware as much as the engine, so every snapshot
  records the BOX it ran on and STANDINGS.md only ever compares snapshots
  sharing one. That is what makes "improvement" an honest word here: a
  cloud→Air jump would be silicon, not progress, and the table refuses to
  draw it. Re-measuring an OLD tag on the CURRENT box is the supported way to
  backfill a comparable history.
- **`publish.sh` reads release notes from both `CHANGELOG.md` and
  `CHANGELOG-ARCHIVE.md`**, so archiving never breaks
  `./publish.sh --release-only <old-version>`.

Add to the pre-flight if you want it enforced:

```sh
python3 scripts/release-notes-roll.py --check   # non-zero if a roll is due
```

## Publish (order matters)

```sh
# 1. the library first — everything else depends on it
cargo publish -p ferroplan

# 2. then the CLI (now that `ferroplan` is on the index)
cargo publish -p ferroplan-cli

# 3. then the MCP server (in the publish set since 0.14.0)
cargo publish -p ferroplan-mcp
```

> A `cargo publish --dry-run` for the CLI or MCP crate BEFORE the library is on
> crates.io fails with `no matching package named 'ferroplan' found` — this is
> expected, not a packaging bug. Verify them with
> `cargo build -p ferroplan-cli -p ferroplan-mcp` instead, and publish only
> after step 1 has landed.

## The Python wheel (staged, published separately)

`crates/ferroplan-py` versions with the workspace (bump its `version` in BOTH
`Cargo.toml` and `pyproject.toml` alongside the workspace bump) but publishes
to **PyPI**, not crates.io, via [maturin](https://www.maturin.rs):

```sh
pip install maturin
maturin build --release -m crates/ferroplan-py/Cargo.toml   # -> target/wheels/*.whl
maturin publish -m crates/ferroplan-py/Cargo.toml           # needs a PyPI token
```

The wheel build is part of the pre-flight from 0.14.0 on; publishing it is a
separate, optional step (the crates.io release does not depend on it).

**On the Air, build the wheel LOCALLY and record what that does not cover.**
`docs/migration-m5.md` proposed downgrading this gate to "the CI wheel
builds", on the grounds that a local `maturin build` produces a macOS-ARM
wheel rather than the manylinux x86 one that ships. By direct decision the
gate stays local, because a local build tests almost everything the gate is
for: the PyO3 bindings compile against the current library, maturin's
packaging works, and the wheel is well-formed. The crate is **abi3-py38**, so
one build covers Python 3.8+ and the Python-version dimension is not in
question either.

What a local build does NOT test is the manylinux x86 cross-build. That is a
real but narrow gap, and the honest form of this gate is therefore: run it
locally, and say in the cut record that the shipped manylinux wheel is
unverified until CI or a zig-cross build runs. A gate that names its own blind
spot beats a gate that is skipped.

Each crate package bundles `README.md` and both `LICENSE-*` files (symlinked into
the crate dirs) so the crates.io page and tarball are complete.

## After publishing

- Push the tag; the `pages.yml` workflow rebuilds the mdBook site.
- The external comparison oracles (Metric-FF, SGPlan6, VAL) and IPC benchmark
  instances are **not** part of any published crate — see
  [`benchmarks/COMPARING.md`](benchmarks/COMPARING.md).
