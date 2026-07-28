#!/bin/sh
set -eu

if command -v bcinr-mcp >/dev/null 2>&1; then
  exec bcinr-mcp
fi

root=${BCINR_ROOT:-}
if [ -z "$root" ]; then
  ferroplan_root=${FERROPLAN_ROOT:-${CLAUDE_PROJECT_DIR:-}}
  if [ -n "$ferroplan_root" ] && [ -f "$ferroplan_root/../bcinr/Cargo.toml" ]; then
    root=$(cd "$ferroplan_root/../bcinr" && pwd)
  fi
fi

if [ -n "$root" ] && [ -f "$root/crates/bcinr-mcp/Cargo.toml" ]; then
  exec cargo run \
    --locked \
    --quiet \
    --manifest-path "$root/Cargo.toml" \
    -p bcinr-mcp \
    --bin bcinr-mcp \
    --
fi

printf '%s\n' \
  "cannot resolve bcinr-mcp from an installed binary or a locked BCINR checkout" >&2
exit 69
