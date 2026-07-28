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

if [ -n "$root" ] && [ -f "$root/Cargo.toml" ]; then
  exec cargo run \
    --quiet \
    --manifest-path "$root/Cargo.toml" \
    -p bcinr-mcp \
    --bin bcinr-mcp \
    --
fi

printf '%s\n' \
  "cannot resolve bcinr-mcp; install it, set BCINR_ROOT, or place the bcinr checkout beside Ferroplan" >&2
exit 69
