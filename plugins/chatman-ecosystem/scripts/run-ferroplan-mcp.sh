#!/bin/sh
set -eu

mode=${1:-session}
case "$mode" in
  stateless) binary=ferroplan-mcp ;;
  session) binary=ferroplan-session-mcp ;;
  *)
    printf '%s\n' "unsupported Ferroplan MCP mode: $mode" >&2
    exit 64
    ;;
esac

root=${FERROPLAN_ROOT:-${CLAUDE_PROJECT_DIR:-}}
if [ -n "$root" ] && [ -f "$root/Cargo.toml" ]; then
  exec cargo run \
    --quiet \
    --manifest-path "$root/Cargo.toml" \
    -p ferroplan-mcp \
    --bin "$binary" \
    --
fi

if command -v "$binary" >/dev/null 2>&1; then
  exec "$binary"
fi

printf '%s\n' \
  "cannot resolve $binary; launch Claude Code in the Ferroplan checkout, set FERROPLAN_ROOT, or install the binary" >&2
exit 69
