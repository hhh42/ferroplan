#!/bin/sh
# Launch ferroplan-mcp, delegating resolution to roots.py.
#
# This script used to carry its own resolution chain, which never tried
# target/release or target/debug -- so a built binary sitting on disk was
# ignored in favour of a `cargo run` that could not run when cargo was absent,
# and the failure named neither what it looked for nor what it saw.
#
# `dirname $0` is the one locator correct under both the repository layout and
# the installed-cache layout, which is why it is used here instead of any
# environment variable: those are all unset in a plain shell.
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

# `--format human` is the shell-consumable projection of the resolution payload:
# a bare exec-ready argv. The JSON form carries provenance that a shell cannot
# use but a diagnostic can -- run `roots.py resolve` without --format to see it.
# On failure roots.py writes a structured error to stderr and exits 69.
argv=$(python3 "$here/roots.py" resolve \
  --binary ferroplan-mcp \
  --crate ferroplan-mcp \
  --env-root FERROPLAN_ROOT \
  --format human) || exit 69

# argv is shell-quoted by shlex.join, so re-splitting it here is the intended
# reconstruction of the argument vector.
exec /bin/sh -c "exec $argv"
