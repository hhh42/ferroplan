#!/bin/sh
# Phase 0.4 (docs/roadmap-0.30.md): does a rebuild of the v0.29.0 tag
# reproduce the board's engine, ff 0.29.0 [7fdf185b91e6]? The 0.28 rebuild
# did not reproduce 89cfdc5f06ed, and the reason turned out to be SOURCE (a
# nine-line FF_TCONC-gated tcompress change landed between the bump and the
# tag). Between the 0.29 bump (43847be) and v0.29.0 only Cargo.lock's own
# version lines and a test file moved, so this is the toolchain question
# alone: same rustc (1.99.0, b940084d7 2026-09-28), same profile (thin LTO,
# one codegen unit), a fresh worktree under crucible's prefix, the same
# `cargo build --release -p ferroplan-cli` backfill runs.
set -eu
echo $$ > "$(dirname "$0")/run.pid"
OUT="$(dirname "$0")/result.txt"
WT=/Users/harold/.crucible/worktrees/v0.29.0-repro
cd /Users/harold/ferroplan
git worktree add --detach "$WT" v0.29.0 >/dev/null 2>&1 || true
cd "$WT"
{
  echo "commit  $(git rev-parse HEAD)"
  echo "rustc   $(rustc -vV | sed -n 1p)"
  t0=$(date +%s)
  cargo build --release -p ferroplan-cli 2>&1 | tail -2
  echo "build   $(( $(date +%s) - t0 )) s"
  echo "version $("$WT"/target/release/ff --version)"
  echo "blake3  $(/Users/harold/ferroplan/crucible/target/release/crucible engine --repo "$WT" 2>/dev/null | awk '/^blake3/{print $2}')"
  echo "board   7fdf185b91e62cfe1bab8c3b4a9a05d029a802f2248fafa9293e2b561c7a4625"
} > "$OUT" 2>&1
echo DONE >> "$OUT"
