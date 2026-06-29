#!/usr/bin/env bash
#
# publish.sh — release the two crates.io crates (library first, then the CLI).
#
# Run from your OWN machine after `cargo login <token>` (the sandbox this was
# authored in has no crates.io token and the network blocks crates.io, so it
# can't publish — only you can). Everything here is the RELEASING.md pre-flight
# plus the two `cargo publish` calls, in the required order.
#
# Usage:
#   ./publish.sh            # pre-flight, confirm, then publish + tag
#   ./publish.sh --dry-run  # pre-flight + `cargo publish --dry-run` only, no upload
#   ./publish.sh --yes      # skip the confirmation prompt
#
set -euo pipefail
cd "$(dirname "$0")"

DRY_RUN=0
ASSUME_YES=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    --yes|-y)  ASSUME_YES=1 ;;
    *) echo "unknown flag: $arg" >&2; exit 2 ;;
  esac
done

# Version is the single source of truth in the workspace manifest.
VERSION="$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')"
TAG="v${VERSION}"
echo "==> Releasing ferroplan ${VERSION} (tag ${TAG})"

echo "==> Pre-flight"
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p ferroplan -p ferroplan-cli
cargo test -p ferroplan -p ferroplan-cli
# Build the library tarball and verify it compiles in isolation.
cargo package -p ferroplan

if [[ "$DRY_RUN" == 1 ]]; then
  echo "==> --dry-run: library packages cleanly."
  # The CLI dry-run needs `ferroplan ${VERSION}` on the index; before it is
  # published that fails by design, so only build-check the CLI here.
  cargo build -p ferroplan-cli
  echo "==> Dry run OK. Re-run without --dry-run to publish."
  exit 0
fi

# Refuse to publish a version that is already on the index (idempotent-ish guard).
if cargo search ferroplan 2>/dev/null | grep -qE "^ferroplan = \"${VERSION}\""; then
  echo "!! ferroplan ${VERSION} is already on crates.io — bump the version first." >&2
  exit 1
fi

if [[ "$ASSUME_YES" != 1 ]]; then
  echo
  echo "About to PUBLISH ferroplan ${VERSION} and ferroplan-cli ${VERSION} to crates.io."
  echo "This is irreversible (a version can only be yanked, never deleted/reused)."
  read -r -p "Type the version (${VERSION}) to confirm: " reply
  [[ "$reply" == "$VERSION" ]] || { echo "aborted."; exit 1; }
fi

echo "==> Publishing the library"
cargo publish -p ferroplan

echo "==> Waiting for ferroplan ${VERSION} to appear on the index"
for i in $(seq 1 30); do
  if cargo search ferroplan 2>/dev/null | grep -qE "^ferroplan = \"${VERSION}\""; then
    echo "   library is on the index."
    break
  fi
  sleep 5
  [[ "$i" == 30 ]] && echo "   (still not visible after ~150s; the CLI publish may need a retry)"
done

echo "==> Publishing the CLI"
cargo publish -p ferroplan-cli

# Tag the release — but skip if it already exists on the remote (e.g. you cut the
# GitHub Release from the web UI first, which creates the tag).
if [[ -n "$(git ls-remote --tags origin "$TAG" 2>/dev/null)" ]]; then
  echo "==> Tag ${TAG} already on the remote — leaving it as-is."
else
  echo "==> Tagging ${TAG}"
  git tag -a "$TAG" -m "ferroplan ${VERSION}" 2>/dev/null || true # may exist locally
  git push origin "$TAG"
fi

echo "==> Done. Published ferroplan + ferroplan-cli ${VERSION}, tagged ${TAG}."
echo "    (Pushing the tag / cutting the GitHub Release triggers the pages.yml rebuild.)"
