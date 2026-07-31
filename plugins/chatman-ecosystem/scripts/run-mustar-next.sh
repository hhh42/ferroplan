#!/bin/sh
# Resolve the JSON-first Speckit-Ralph CLI and ask MuStar for one obligation.
set -eu
if [ "$#" -ne 1 ]; then
  echo 'usage: run-mustar-next.sh TARGET' >&2
  exit 64
fi
target=$1
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
argv=$(python3 "$here/roots.py" resolve \
  --binary sr \
  --crate speckit-ralph \
  --env-root CHATMANGPT_ROOT \
  --marker speckit-ralph/src/mustar.rs \
  --sibling chatmangpt \
  --format human) || exit 69
eval "set -- $argv"
exec "$@" mustar next --target "$target"
