#!/bin/sh
# Fetch the full IPC-2008/IPC-2011 benchmark sets (potassco/pddl-instances)
# into benchmarks/.ipc-corpus/ via a sparse checkout (~hundreds of MB; the
# curated regression subset vendored under benchmarks/ipc/ does NOT need this).
# Used by benchmarks/ipc67.py for full-corpus scoreboard runs:
#   sh benchmarks/get-ipc.sh
#   python3 benchmarks/ipc67.py
set -e
cd "$(dirname "$0")"
if [ ! -d .ipc-corpus ]; then
  git clone --depth 1 --filter=blob:none --sparse \
    https://github.com/potassco/pddl-instances .ipc-corpus
fi
cd .ipc-corpus
git sparse-checkout set ipc-2008/domains ipc-2011/domains
echo "corpus ready: $(pwd)"
