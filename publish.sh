#!/usr/bin/env bash
#
# publish.sh — release the two crates.io crates (library first, then the CLI),
# then cut the matching GitHub Release (title + body sourced straight from
# CHANGELOG.md — never hand-copied, so it can't drift).
#
# Run from your OWN machine after `cargo login <token>` (the sandbox this was
# authored in has no crates.io token and the network blocks crates.io, so it
# can't publish — only you can). Everything here is the RELEASING.md pre-flight
# plus the two `cargo publish` calls, in the required order, plus the GitHub
# Release (needs the `gh` CLI — https://cli.github.com/, `gh auth login` once).
#
# Usage:
#   ./publish.sh                    # pre-flight, confirm, publish, tag, GitHub Release
#   ./publish.sh --dry-run          # pre-flight + `cargo publish --dry-run` only, no upload
#   ./publish.sh --yes              # skip the confirmation prompt
#   ./publish.sh --release-only     # skip crates.io/tagging entirely — just cut/update the
#                                    # GitHub Release for a version that's ALREADY tagged
#                                    # (the tag must already exist locally or on origin).
#   ./publish.sh --release-only 0.2.2   # ...for a specific (e.g. historical/backfill) version
#
set -euo pipefail
cd "$(dirname "$0")"

DRY_RUN=0
ASSUME_YES=0
RELEASE_ONLY=0
RELEASE_ONLY_VERSION=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1; shift ;;
    --yes|-y)  ASSUME_YES=1; shift ;;
    --release-only)
      RELEASE_ONLY=1; shift
      # Optional positional version right after the flag (e.g. `--release-only 0.2.2`).
      if [[ $# -gt 0 && "$1" != --* ]]; then RELEASE_ONLY_VERSION="$1"; shift; fi
      ;;
    *) echo "unknown flag: $1" >&2; exit 2 ;;
  esac
done

# Version is the single source of truth in the workspace manifest, unless
# --release-only names a specific (e.g. historical) version to backfill.
VERSION="${RELEASE_ONLY_VERSION:-$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')}"
TAG="v${VERSION}"

# Cut (or update) the GitHub Release for $VERSION from CHANGELOG.md's matching
# "## [$VERSION] - DATE — Subtitle" section: title = "ferroplan $VERSION — Subtitle",
# body = everything up to the next "## [" header. Requires the tag to already exist
# (locally or on origin) and the `gh` CLI to be installed + authenticated; otherwise
# prints the exact manual web-UI steps and returns non-fatally (release notes are a
# nice-to-have, never worth failing a crates.io publish over).
create_github_release() {
  local ver="$1" tag="v${1}"
  if ! git rev-parse "$tag" >/dev/null 2>&1; then
    git fetch origin --tags -q 2>/dev/null || true
  fi
  if ! git rev-parse "$tag" >/dev/null 2>&1; then
    echo "!! tag ${tag} not found locally or on origin — tag it first." >&2
    return 1
  fi
  if ! command -v gh >/dev/null 2>&1; then
    echo "==> gh CLI not found — skipping the GitHub Release."
    echo "    Install: https://cli.github.com/, then \`gh auth login\`, then re-run:"
    echo "      ./publish.sh --release-only ${ver}"
    echo "    Or create it by hand: https://github.com/hhh42/ferroplan/releases/new?tag=${tag}"
    return 0
  fi
  local header notes_file title
  header="$(grep -m1 "^## \[${ver}\]" CHANGELOG.md || true)"
  if [[ -z "$header" ]]; then
    echo "!! no CHANGELOG.md section \"## [${ver}]\" found — add one before releasing." >&2
    return 1
  fi
  title="ferroplan ${ver}$(sed -E 's/^## \[[^]]+\] - [0-9-]+//' <<<"$header")"
  notes_file="$(mktemp)"
  trap 'rm -f "$notes_file"' RETURN
  awk -v ver="$ver" '
    $0 ~ "^## \\[" ver "\\]" {flag=1; next}
    /^## \[/ {flag=0}
    flag {print}
  ' CHANGELOG.md | sed -e '/./,$!d' >"$notes_file"  # drop leading blank line(s) only
  if gh release view "$tag" >/dev/null 2>&1; then
    echo "==> Updating the existing GitHub Release for ${tag}"
    gh release edit "$tag" --title "$title" --notes-file "$notes_file"
  else
    echo "==> Creating the GitHub Release for ${tag}"
    gh release create "$tag" --title "$title" --notes-file "$notes_file"
  fi
  echo "    https://github.com/hhh42/ferroplan/releases/tag/${tag}"
}

if [[ "$RELEASE_ONLY" == 1 ]]; then
  echo "==> --release-only: cutting the GitHub Release for ${VERSION} (no crates.io publish)"
  create_github_release "$VERSION"
  exit $?
fi

echo "==> Releasing ferroplan ${VERSION} (tag ${TAG})"

echo "==> Pre-flight (scoped to the published crates — does NOT build ferroplan-bevy)"
cargo fmt --all --check
# Scope clippy to the two crates we publish; a bare `--all-targets` would compile the
# whole workspace, including the Bevy GUI (minutes of build for nothing — the library
# itself has no graphics dependency).
cargo clippy -p ferroplan -p ferroplan-cli --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p ferroplan -p ferroplan-cli
# Skips the `#[ignore]`d IPC-benchmark regression guards (multi-minute solves); those
# are CI-gated on every push. Set RUN_HEAVY=1 to include them here too (release-built).
if [[ "${RUN_HEAVY:-0}" == 1 ]]; then
  cargo test --release -p ferroplan -p ferroplan-cli -- --include-ignored
else
  cargo test -p ferroplan -p ferroplan-cli
fi
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

# If the version is already on the index, don't error out — this run may be
# resuming after crates.io succeeded but the script died/was interrupted before
# tagging or the GitHub Release (that's happened: crates.io publish went through,
# nobody ran the rest). Skip straight to tag + Release instead of re-publishing.
ALREADY_ON_INDEX=0
if cargo search ferroplan 2>/dev/null | grep -qE "^ferroplan = \"${VERSION}\""; then
  ALREADY_ON_INDEX=1
  echo "==> ferroplan ${VERSION} is already on crates.io — skipping straight to tag + GitHub Release."
fi

if [[ "$ALREADY_ON_INDEX" != 1 ]]; then
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
fi

# Tag the release — but skip if it already exists on the remote (e.g. this run is
# resuming, or you cut the GitHub Release from the web UI first, which tags too).
if [[ -n "$(git ls-remote --tags origin "$TAG" 2>/dev/null)" ]]; then
  echo "==> Tag ${TAG} already on the remote — leaving it as-is."
else
  echo "==> Tagging ${TAG}"
  git tag -a "$TAG" -m "ferroplan ${VERSION}" 2>/dev/null || true # may exist locally
  git push origin "$TAG"
fi

create_github_release "$VERSION"

echo "==> Done. Published ferroplan + ferroplan-cli ${VERSION}, tagged ${TAG}, cut the GitHub Release."
echo "    (Pushing the tag triggers the pages.yml rebuild.)"
