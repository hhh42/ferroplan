#!/bin/sh
set -eu

binary=ferroplan-mcp

if command -v "$binary" >/dev/null 2>&1; then
  exec "$binary"
fi

root=${FERROPLAN_ROOT:-}
if [ -z "$root" ]; then
  project=${CLAUDE_PROJECT_DIR:-}
  if [ -n "$project" ] && [ -f "$project/crates/ferroplan-mcp/Cargo.toml" ]; then
    root=$(cd "$project" && pwd)
  fi
fi

if [ -n "$root" ] && [ -f "$root/crates/ferroplan-mcp/Cargo.toml" ]; then
  exec cargo run \
    --locked \
    --quiet \
    --manifest-path "$root/Cargo.toml" \
    -p ferroplan-mcp \
    --bin "$binary" \
    --
fi

printf '%s\n' \
  "cannot resolve $binary from an installed binary or a locked Ferroplan checkout" >&2
exit 69
