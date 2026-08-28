#!/usr/bin/env bash
# crucible's own pre-flight.
#
# Deliberately separate from the planner's (RELEASING.md): crucible is excluded
# from the ferroplan workspace so a ratatui/rusqlite dependency tree can never
# gate a planner release. The cost of that isolation is that CI never sees this
# crate, so THIS SCRIPT IS THE GATE. Run it before any crucible change lands.
#
# The order is cheapest-first, so a typo fails in seconds rather than after the
# full-corpus replay.
set -euo pipefail
cd "$(dirname "$0")"
step() { printf '\n== %s ==\n' "$1"; }

step "fmt"
cargo fmt --all -- --check

step "clippy"
cargo clippy --all-targets --all-features -- -D warnings

step "test"
cargo test --all

step "the Platform trait is only honest if the generic path compiles"
# libproc is the only target-gated dependency and every use of it sits behind
# `trait Platform`. If a macOS-only call escapes the trait, this fails -- which
# is the whole point of keeping libproc out of the unconditional dep table.
if rustup target list --installed | grep -q x86_64-unknown-linux-gnu; then
  cargo check --target x86_64-unknown-linux-gnu -p crucible-core -p crucible-publish
else
  echo "   SKIPPED: rustup target add x86_64-unknown-linux-gnu to arm this gate"
fi

step "the rescued incident evidence has not been hand-edited"
# Fixtures are real rows from real sweeps, several of them the only surviving
# copy. --check re-derives them from source and refuses any drift, so nobody can
# quietly edit one into agreement with a test.
python3 tests/fixtures/extract.py --check >/dev/null && echo "   all fixtures match"

step "the manifest still says what the Python registries say"
python3 tools/verify-manifest.py

step "the dashboard renders at the sizes people actually use"
cargo run -q -p crucible -- tui --dump --width 118 --height 30 >/dev/null
cargo run -q -p crucible -- tui --dump --width 60 --height 16 >/dev/null

step "the pure layer still agrees with the oracle, row by row"
# THE gate. Classify, coverage_line and the corpus selector, diffed against
# benchmarks/oracle over every board raw on this box. A port that changes a
# number is not a port.
cargo build --release -p crucible-publish -p crucible -q
python3 ../benchmarks/crucible-differential.py

step "every committed raw round-trips byte for byte"
total=0; files=0
# benchmarks/metrics/ holds reports and probe receipts (a sitting's matrix.jsonl
# carries a "solved" key too), never board raws -- skipped, or a decode
# sitting's receipts would be asked to round-trip as a board.
for f in $(find ../benchmarks -name '*.jsonl' -not -path '*/.ipc-corpus/*' -not -path '*/metrics/*' | sort); do
  head -1 "$f" | grep -q '"solved"' || continue
  n=$(target/release/crucible-replay roundtrip --raw "$f")
  total=$((total + n)); files=$((files + 1))
done
echo "   $total rows across $files raws"

printf '\ncrucible pre-flight: clean\n'
