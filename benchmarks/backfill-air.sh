#!/usr/bin/env bash
# Re-measure an OLD tagged engine on THIS box, so the standings history has a
# comparable predecessor.
#
# The Air re-baseline broke the trend line by design: coverage at a fixed time
# budget is a property of the hardware as much as the engine, so no 0.19 cloud
# number may be compared to a 0.20 Air number. That leaves "improvement"
# unsayable until two releases have been measured on one box. This says it, by
# re-running the old engine here.
#
#   bash benchmarks/backfill-air.sh v0.19.0
#
# Method, and why each part matters:
#   - The tag is built in a git WORKTREE, so the working tree is untouched and
#     the sweep can run against a binary from a different commit than HEAD.
#   - The sweep uses the CURRENT harness (ipc67.py) against the OLD binary via
#     $FERROPLAN_FF. Checking out the old benchmarks/ too would vary the
#     INSTRUMENT as well as the engine, and then the delta means nothing —
#     which is exactly the trap this cycle's val_check fix illustrates: the
#     old runner books VAL-unreadable domains as rejected plans and loses 15
#     instances of coverage that the new one keeps.
#   - Same --jobs 2 --mem-gb 6 and same board list as rebaseline-air.sh, so
#     both sides of the comparison share every condition except the engine.
#
# Resume-aware per board, like every driver here. Expect ~20h.
set -eu
cd "$(dirname "$0")/.."
TAG="${1:?usage: backfill-air.sh <tag, e.g. v0.19.0>}"
VER="${TAG#v}"
WT="../ferroplan-backfill-$VER"
OUT="benchmarks/air-$VER"
JOBS="${JOBS:-2}"
MEMGB="${MEMGB:-6}"

git rev-parse -q --verify "refs/tags/$TAG" >/dev/null \
  || { echo "no such tag: $TAG"; exit 1; }

if [ ! -d "$WT" ]; then
  echo "==> worktree $WT at $TAG"
  git worktree add --detach "$WT" "$TAG"
fi

BIN="$(cd "$WT" && pwd)/target/release/ff"
if [ ! -x "$BIN" ]; then
  echo "==> building $TAG (this is the OLD engine)"
  ( cd "$WT" && cargo build --release -q -p ferroplan-cli )
fi
echo "==> engine: $BIN"
"$BIN" --version 2>/dev/null || true

mkdir -p "$OUT"
export FERROPLAN_FF="$BIN"

# Mode::Optimal arrived in 0.19 (Phase 2). Older engines have no optimal.rs at
# all, so the two proof-rate boards are not "0 coverage" for them — the feature
# does not exist, and recording a zero would be a lie the standings would then
# average. Probe once and skip those boards by name if unsupported.
HAS_OPT=1
if ! "$BIN" --help 2>&1 | grep -qi 'optimal'; then
  if ! git -C "$WT" ls-files --error-unmatch crates/ferroplan/src/optimal.rs >/dev/null 2>&1; then
    HAS_OPT=0
  fi
fi
if [ "$HAS_OPT" = "0" ]; then
  echo "==> $TAG predates Mode::Optimal — skipping the two proof-rate boards"
  echo "    (feature absent, NOT zero coverage)"
  : > "$OUT/OPTIMAL-UNSUPPORTED"
fi

run_board() { # name track timeout extra-args...
  local name="$1" track="$2" tmo="$3"; shift 3
  if [ -f "$OUT/$name.done" ]; then echo "SKIP $name (done)"; return; fi
  case " $* " in *" optimal "*)
    if [ "$HAS_OPT" = "0" ]; then echo "SKIP $name (Mode::Optimal absent in $TAG)"; return; fi ;;
  esac
  echo "RUN  $name ($track ${tmo}s $*) $(date '+%H:%M:%S')"
  python3 benchmarks/ipc67.py --track "$track" --timeout "$tmo" \
    --jobs "$JOBS" --mem-gb "$MEMGB" "$@" \
    --out "$OUT/$name.md" >"$OUT/$name.log" 2>&1
  echo "DONE $name: $(tail -1 "$OUT/$name.md") $(date '+%H:%M:%S')"
  touch "$OUT/$name.done"
}

# EXACTLY rebaseline-air.sh's list and order — the comparison is only as good
# as its sameness.
run_board ipc2023-agile      agile-2023      60
run_board ipc2014-tempo      tempo-sat-2014  30
run_board ipc2018-sat        sat-2018        60
run_board ipc2014-sat        seq-sat-2014    60
run_board ipc2014-agile      seq-agile-2014  60
run_board ipc2014-opt        seq-opt-2014    60 --mode optimal
run_board ipc2026-numeric    numeric-2026    60
run_board ipc2023-numeric    numeric-2023    60
run_board ipc-opt-2008-11    seq-opt         60 --mode optimal
run_board ipc67-temporal     tempo-sat       30
run_board ipc67-results      seq-sat         60
run_board ipc2023-agile-300s agile-2023      300
echo "BACKFILL $TAG ALL DONE $(date '+%Y-%m-%d %H:%M:%S')"
echo "next: python3 scripts/standings-snapshot.py --version $VER \\"
echo "        --measured-at \$(date +%F) --released <original release date> \\"
echo "        --note 'backfilled on m5-air from tag $TAG'"
