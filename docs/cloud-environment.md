# Cloud Environment

This repository ships two cloud-ready capsules. Neither depends on GitHub Actions.

## Development capsule

Open the repository in a Dev Container or GitHub Codespace. The configuration under `.devcontainer/` provides:

- Rust 1.97.1 with `rustfmt` and `clippy`;
- persistent Cargo registry, Git, and target caches;
- Rust Analyzer and LLDB editor support;
- a bootstrap acceptance ladder that writes `.cloud/receipts/bootstrap.json`.

The bootstrap executes:

```sh
cargo fetch --locked
cargo fmt --all --check
cargo check --locked -p ferroplan-cli --bin ferroplan-ppddl
cargo test --locked -p ferroplan ppddl::
```

A successful exact execution emits `ALIVE`. Any failed command emits `BUILD_BROKEN` and records the failed command. CI is never consulted.

## Production runtime capsule

Build the minimal PPDDL runtime image:

```sh
docker build -f deploy/Dockerfile.ppddl -t ferroplan-ppddl:local .
```

Run the admitted future corpus with the hardened Compose service:

```sh
docker compose -f compose.cloud.yml run --rm ppddl > cloud-policy-receipt.json
```

The service:

- runs as a non-root user;
- uses a read-only root filesystem;
- drops all Linux capabilities;
- enables `no-new-privileges`;
- mounts the method corpus read-only;
- performs bounded policy synthesis, independent validation, deterministic seeded simulation, and receipt emission.

## Direct runtime invocation

```sh
docker run --rm \
  --read-only \
  --cap-drop ALL \
  --security-opt no-new-privileges:true \
  -v "$PWD/examples/daily_agent_methods:/work/methods:ro" \
  ferroplan-ppddl:local \
  --domain /work/methods/future/domain.ppddl \
  --problem /work/methods/future/problem.ppddl \
  --horizon 64 \
  --episodes 1000 \
  --seed 42
```

## Standing rules

- Configuration present but unexecuted: `PARTIAL_ALIVE`.
- Toolchain, build, tests, policy synthesis, validation, and replay all pass against the exact tree: `ALIVE`.
- Source compiles unsuccessfully: `BUILD_BROKEN`.
- Required cloud/container transport unavailable: `BLOCKED`.
- Unsupported PPDDL surface or platform capability: `UNSUPPORTED`.

Queued or successful CI does not alter these local standing rules. The former PPDDL-specific Actions workflow was removed so the cloud capsule is the canonical acceptance path.
