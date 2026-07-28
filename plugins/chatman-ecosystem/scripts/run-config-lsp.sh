#!/bin/sh
set -eu

if command -v claude-code-config-lsp >/dev/null 2>&1; then
  exec claude-code-config-lsp
fi

root=${CLAUDE_CODE_CONFIG_LSP_ROOT:-}
if [ -z "$root" ]; then
  project=${CLAUDE_PROJECT_DIR:-}
  if [ -n "$project" ] && [ -f "$project/../claude-code-config-lsp/Cargo.toml" ]; then
    root=$(cd "$project/../claude-code-config-lsp" && pwd)
  fi
fi

if [ -n "$root" ] && [ -f "$root/Cargo.toml" ]; then
  exec cargo run \
    --locked \
    --quiet \
    --manifest-path "$root/Cargo.toml" \
    --bin claude-code-config-lsp \
    --
fi

printf '%s\n' \
  "cannot resolve claude-code-config-lsp from an installed binary or a locked checkout" >&2
exit 69
