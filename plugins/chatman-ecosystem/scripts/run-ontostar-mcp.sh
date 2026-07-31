#!/bin/sh
# Resolve and launch Sean Chatman's OntoStar MCP server over stdio.
set -eu
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
argv=$(python3 "$here/roots.py" resolve \
  --binary open-ontologies \
  --crate open-ontologies \
  --env-root ONTOSTAR_ROOT \
  --marker src/a2a/mod.rs \
  --sibling open-ontologies \
  --format human) || exit 69
eval "set -- $argv"
exec "$@" mcp start --transport stdio
