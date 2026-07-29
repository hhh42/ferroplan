#!/bin/sh
# Launch claude-code-config-lsp, delegating resolution to roots.py.
#
# Unlike ferroplan-mcp this crate lives in a *sibling* checkout rather than the
# ferroplan workspace, so the resolver is given its own marker (the crate's own
# Cargo.toml) and the sibling directory name to look for beside the repository.
# The resolution order, the target/{release,debug} lookup, and the structured
# failure are otherwise identical -- one resolver, two callers.
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

argv=$(python3 "$here/roots.py" resolve \
  --binary claude-code-config-lsp \
  --marker Cargo.toml \
  --sibling claude-code-config-lsp \
  --env-root CLAUDE_CODE_CONFIG_LSP_ROOT \
  --format human) || exit 69

exec /bin/sh -c "exec $argv"
