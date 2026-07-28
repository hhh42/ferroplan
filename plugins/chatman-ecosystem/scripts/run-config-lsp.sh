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
    --quiet \
    --manifest-path "$root/Cargo.toml" \
    --bin claude-code-config-lsp \
    --
fi

printf '%s\n' \
  "cannot resolve claude-code-config-lsp; install the binary, configure config_lsp_root, or place the checkout beside Ferroplan" >&2
exit 69
