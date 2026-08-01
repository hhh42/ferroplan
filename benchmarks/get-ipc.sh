#!/bin/sh
# Fetch the full IPC benchmark corpora into benchmarks/.ipc-corpus/:
#   - IPC-2006/2008/2011/2014 via a potassco/pddl-instances sparse checkout
#   - IPC-2018 satisficing (official bitbucket) + cost bounds
#   - IPC-2023 classical (official dataset: domains + reference plans +
#     best-known bounds) and IPC-2023 numeric (official dataset + result CSVs)
# 2018/2023 layouts are NORMALIZED into the potassco shape the runner
# expects (domains/<variant>/{domain.pddl,instances/instance-N.pddl});
# official reference artifacts (plans, bounds, CSVs) are kept beside them.
# The curated regression subset vendored under benchmarks/ipc/ does NOT
# need any of this. Used by benchmarks/ipc67.py for full-corpus runs:
#   sh benchmarks/get-ipc.sh
#   python3 benchmarks/ipc67.py
set -e
cd "$(dirname "$0")"

# Instance-number normalizer: strip the leading non-digit prefix, then strip
# leading zeros but ALWAYS keep at least one digit. The naive
# `sed 's/[^0-9]*0*//'` collapses a 0-indexed `p000.pddl` to the empty string
# (-> `instance-.pddl`), which the runner's `int(re.search(r"\d+", name))`
# then dies on. IPC-2026's gear-car and both sailing-wind variants are
# 0-indexed p000..p19, so this is load-bearing, not hypothetical.
instnum() { basename "$1" .pddl | sed 's/^[^0-9]*//; s/^0*\([0-9]\)/\1/'; }
if [ ! -d .ipc-corpus ]; then
  git clone --depth 1 --filter=blob:none --sparse \
    https://github.com/potassco/pddl-instances .ipc-corpus
fi
(cd .ipc-corpus && git sparse-checkout set \
  ipc-2006/domains ipc-2008/domains ipc-2011/domains ipc-2014/domains)

# --- IPC-2018 satisficing (bitbucket) -> ipc-2018/domains/<dom>-sequential-satisficing
if [ ! -d .ipc-corpus/ipc-2018/domains ]; then
  tmp=.ipc-corpus/.tmp-2018
  git clone -q --depth 1 https://bitbucket.org/ipc2018-classical/domains "$tmp"
  for d in "$tmp"/sat/*/; do
    name=$(basename "$d")
    dest=.ipc-corpus/ipc-2018/domains/"$name"-sequential-satisficing
    mkdir -p "$dest"/instances
    [ -f "$d"domain.pddl ] && cp "$d"domain.pddl "$dest"/
    for p in "$d"p*.pddl; do
      base=$(basename "$p" .pddl)
      n=$(echo "$base" | sed 's/^p0*//')
      cp "$p" "$dest"/instances/instance-"$n".pddl
      # flashfill / organic-synthesis carry per-instance domain files;
      # normalized as instances/domain-N.pddl (the runner prefers them).
      [ -f "$d"domain-"$base".pddl ] && \
        mkdir -p "$dest"/domains && cp "$d"domain-"$base".pddl "$dest"/domains/domain-"$n".pddl
    done
  done
  cp "$tmp"/cost_bounds.json .ipc-corpus/ipc-2018/ 2>/dev/null || true
  rm -rf "$tmp"
fi

# --- IPC-2023 classical (github) -> ipc-2023/domains/<dom>-agile (+ ref plans/bounds)
if [ ! -d .ipc-corpus/ipc-2023/domains ]; then
  tmp=.ipc-corpus/.tmp-2023
  git clone -q --depth 1 https://github.com/ipc2023-classical/ipc2023-dataset "$tmp"
  for d in "$tmp"/agl/*/; do
    name=$(basename "$d")
    dest=.ipc-corpus/ipc-2023/domains/"$name"-agile
    mkdir -p "$dest"/instances "$dest"/reference-plans
    [ -f "$d"domain.pddl ] && cp "$d"domain.pddl "$dest"/
    for p in "$d"p*.pddl; do
      base=$(basename "$p" .pddl)
      n=$(echo "$base" | sed 's/^p0*//')
      cp "$p" "$dest"/instances/instance-"$n".pddl
      # quantum-layout carries per-instance domain_pNN.pddl files
      [ -f "$d"domain_"$base".pddl ] && \
        mkdir -p "$dest"/domains && cp "$d"domain_"$base".pddl "$dest"/domains/domain-"$n".pddl
    done
    cp "$d"p*.plan "$dest"/reference-plans/ 2>/dev/null || true
  done
  cp "$tmp"/bounds.json .ipc-corpus/ipc-2023/ 2>/dev/null || true
  rm -rf "$tmp"
fi

# --- IPC-2023 numeric (github) -> ipc-2023n/domains/<dom>-numeric-satisficing (+ CSVs)
if [ ! -d .ipc-corpus/ipc-2023n/domains ]; then
  tmp=.ipc-corpus/.tmp-2023n
  git clone -q --depth 1 https://github.com/ipc2023-numeric/ipc2023-dataset "$tmp"
  for d in "$tmp"/*/; do
    name=$(basename "$d")
    [ -f "$d"domain.pddl ] || continue
    dest=.ipc-corpus/ipc-2023n/domains/"$name"-numeric-satisficing
    mkdir -p "$dest"/instances
    cp "$d"domain.pddl "$dest"/
    for p in "$d"instances/*.pddl; do
      n=$(instnum "$p")
      cp "$p" "$dest"/instances/instance-"$n".pddl
    done
  done
  git clone -q --depth 1 \
    https://github.com/ipc2023-numeric/ipc2023-numeric.github.io "$tmp"-site
  mkdir -p .ipc-corpus/ipc-2023n/results
  cp "$tmp"-site/sat.csv "$tmp"-site/opt.csv .ipc-corpus/ipc-2023n/results/ 2>/dev/null || true
  rm -rf "$tmp" "$tmp"-site
fi
# --- IPC-2026 numeric (github) -> ipc-2026n/domains/<dom>-numeric-2026
# The track ran at ICAPS Dublin (June 2026); the dataset repo carries
# 20-instance domains incl. -sat/-opt pairs. Vendored under the dataset's
# own names; the first board sweeps everything satisficing-style.
if [ ! -d .ipc-corpus/ipc-2026n/domains ]; then
  tmp=.ipc-corpus/.tmp-2026n
  git clone -q --depth 1 https://github.com/ipc2026-numeric/ipc2026-dataset "$tmp"
  for d in "$tmp"/*/; do
    name=$(basename "$d")
    dompddl="$d"domain.pddl
    # line-exchange-snp ships domain_snp.pddl — take the first domain*.pddl
    [ -f "$dompddl" ] || dompddl=$(ls "$d"domain*.pddl 2>/dev/null | head -1)
    [ -n "$dompddl" ] && [ -f "$dompddl" ] || continue
    dest=.ipc-corpus/ipc-2026n/domains/"$name"-numeric-2026
    mkdir -p "$dest"/instances
    cp "$dompddl" "$dest"/domain.pddl
    for p in "$d"instances/*.pddl; do
      n=$(instnum "$p")
      cp "$p" "$dest"/instances/instance-"$n".pddl
    done
  done
  rm -rf "$tmp"
fi
echo "corpus ready: $(pwd)/.ipc-corpus"
