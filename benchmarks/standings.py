#!/usr/bin/env python3
"""Regenerate benchmarks/ipc-standings.md — the one-table-per-competition
standings document (0.16 Phase 1 deliverable; scripted per RELEASING.md
discipline: scoreboards defend themselves).

Inputs (all optional — missing files are marked "not swept" / "in flight"):
  - raw per-instance JSONLs from ipc67.py sweeps (untracked working data):
      ipc5-prop / ipc5-time / ipc5-metric-time / ipc5-constraints
      ipc67-default (seq-sat) / ipc67-temporal (tempo-sat) / ipc67-netben
      ipc7-mco-t{2,4,8}
  - benchmarks/IPC5-results.tgz — the vendored official IPC-5 results
    archive (see ATTRIBUTION.md): reference plans with MetricValue headers.

Quality scoring, by track semantics:
  - IPC-5 propositional: plan LENGTH vs the archive field's plan lengths
    (action lines counted per .soln — NrActions headers are often empty).
    IPC-2008-style quality ratio (best/ours) plus W/T/L vs best-of-field.
  - IPC-5 preference tracks: already reference-scored on their own boards
    (ipc5-scoreboard.md, ipc5-qualitative-scoreboard.md) — linked, not
    recomputed here.
  - IPC-5 time / metric-time: MAKESPAN vs the archive field's makespans
    (computed per .soln from the timed steps, max(t + duration) — the
    `; MakeSpan` headers are empty on exactly the planner that dominates
    these tracks, sgplan). Scored only on rows that CARRY a makespan:
    the runner records it since 0.22 (the 0.14-era debt, closed at that
    cut so this cycle's re-baseline could score without a second sweep),
    so a pre-0.22 raw renders coverage-only rather than a guessed number.
  - IPC-5 constraints: coverage-only.
  - IPC-6/7 tracks: coverage (+ VAL) against standing baselines; no
    official per-instance archive is vendored for 2008/2011.

Failure classes per unsolved instance (from the JSONL):
  timeout (elapsed >= 90% of budget — the row's OWN `budget` stamp where
  the raw carries one (ipc67.py records it since 0.23; the tier-move
  mechanism), else this file's SWEEPS registry value — including graceful engine exits
  AT an armed FF_TIME_LIMIT wall; 90 because the refill loop's re-entry
  floor is 10% of wall, so nothing between 90% and the wall can be a
  give-up), mem-cap (notes), engine-reject/error (a named mechanism:
  parse/feature reject, grounding verdict, nonzero exit, or a legacy
  pre-0.20 row with no elapsed recorded), else early-exit (search gave
  up with wall budget left — the class the 0.20 refill loop emptied;
  0.21 closed the residual [90%, 95%) boundary sliver by moving the
  line here to the refill floor).
"""

import json
import os
import re
import sys
import tarfile
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
B = os.path.join(ROOT, "benchmarks")
# `--out FILE` so a regeneration can be inspected before it replaces the
# committed table (a bare run overwrites ipc-standings.md in place, and on a
# box holding only some of the raws that is a destructive act).
OUT = (sys.argv[sys.argv.index("--out") + 1] if "--out" in sys.argv
       else os.path.join(B, "ipc-standings.md"))
ARCHIVE = os.path.join(B, "IPC5-results.tgz")

# WHICH BOX PRODUCED A BOARD. The 0.21 Phase 1 re-baseline re-swept the twelve
# canonical boards on the M5 Air; every other board in this table still carries
# numbers from the 4-core cloud container. Faster silicon inflates coverage at a
# fixed budget, so an Air row and a cloud row MUST NOT be read against each
# other — and a missing raw JSONL means different things for the two: for an
# Air board it means the sweep has not finished, for a cloud board it means the
# board was never re-baselined and its record lives in git history. Rendering
# both as "not swept" would claim we had never measured them at all.
AIR_REBASELINED = {
    "2023 classical", "2014 tempo-sat", "2018 seq-sat", "2014 seq-sat",
    "2014 seq-agile", "2014 seq-opt", "2026 numeric (first board)",
    "2023 numeric", "2023 agile ENTRY (300s)", "2026 numeric-opt",
    # 0.22 Phase 8 re-entries: committed to the Air this cycle, so a
    # missing raw is a sweep in flight, not a cloud-era ghost.
    "propositional", "net-benefit", "constraints",
    # 0.23 Phase 3 re-entries (the sitting): time/metric-time re-baseline
    # plus the four mco boards — the LAST cloud-era ghosts. Committed to
    # the Air this cycle (cut23-sweeps.sh carries all six), so a missing
    # raw is a sweep in flight from here on.
    "time", "metric-time", "seq-mco t2", "seq-mco t4", "seq-mco t8",
    "2014 seq-mco t4",
    # shared-sweep labels, split per competition at render time
    "seq-opt", "tempo-sat", "seq-sat",
    # 0.25 Phase 1 entries: born on the Air — a missing raw means the
    # entries sweep hasn't run yet, never a cloud-era ghost.
    "2014 seq-mco t2", "2014 seq-mco t8", "2018 seq-opt", "2023 seq-sat",
    "2023 seq-opt", "2023 numeric-opt", "2026 numeric-opt FULL",
    "simple-preferences (full corpus)", "qualitative-preferences (full corpus)",
    "complex-preferences (full corpus)",
}
CLOUD_ERA = "cloud-era board, NOT re-baselined — see git history"


def absent(label, pending="sweep in flight / not yet run"):
    """Row text for a board with no usable raw, honest about which case."""
    return pending if label in AIR_REBASELINED else CLOUD_ERA

# sweep jsonl -> (label, competition, budget seconds)
SWEEPS = {
    "ipc5-prop.jsonl": ("propositional", "ipc5", 60),
    "ipc5-time.jsonl": ("time", "ipc5", 30),
    "ipc5-metric-time.jsonl": ("metric-time", "ipc5", 30),
    "ipc5-constraints.jsonl": ("constraints", "ipc5", 60),
    "ipc67-default.jsonl": ("seq-sat", "ipc67", 60),
    # >>> TIER MOVE 30 -> 60 (0.23 Phase 3), DEFERRED TO PROMOTE TIME <<<
    # The two temporal boards (this one and ipc2014-tempo below) sweep at
    # 60 s from cut23-sweeps.sh on — but the COMMITTED raws are still the
    # 30 s tier, carry no per-row budget stamp, and this registry value is
    # what classifies their timeouts. Flipping it early would re-class
    # every 30 s wall-exit as "early-exit" — a lie in the one column the
    # refill loop is refereed by. The mechanism: ipc67.py stamps `budget`
    # into every row since 0.23 and classify() prefers the row's own
    # stamp, so the 60 s raws classify right the moment they land, with
    # this field lagging harmlessly. FLIP BOTH FIELDS TO 60 when
    # promote-air23.sh promotes the 60 s boards (it checks the stamps and
    # reminds you), so the fallback and the budget prose in write_summary
    # ("30 s temporal") stay truthful for pre-stamp archaeology.
    "ipc67-temporal.jsonl": ("tempo-sat", "ipc67", 60),
    "ipc67-netben.jsonl": ("net-benefit", "ipc67", 60),
    # The mco methodology (0.16, re-affirmed for the 0.23 re-entry): 60 s
    # WALL-CLOCK per the competition rule — the track scores wall time on
    # a fixed box, however many cores a planner burns — one instance at a
    # time (--threads N --jobs 1). t8 is oversubscribed by construction
    # on the 4P+6E Air and is recorded as such, not excused.
    "ipc7-mco-t2.jsonl": ("seq-mco t2", "ipc7", 60),
    "ipc7-mco-t4.jsonl": ("seq-mco t4", "ipc7", 60),
    "ipc7-mco-t8.jsonl": ("seq-mco t8", "ipc7", 60),
    # The modern corpora (0.17 frontier cycle).
    "ipc2014-sat.jsonl": ("2014 seq-sat", "modern", 60),
    "ipc2014-agile.jsonl": ("2014 seq-agile", "modern", 60),
    "ipc2014-tempo.jsonl": ("2014 tempo-sat", "modern", 60),
    "ipc2014-mco-t4.jsonl": ("2014 seq-mco t4", "modern", 60),
    "ipc2018-sat.jsonl": ("2018 seq-sat", "modern", 60),
    "ipc2023-agile.jsonl": ("2023 classical", "modern", 60),
    "ipc2023-numeric.jsonl": ("2023 numeric", "modern", 60),
    # 0.20: the IPC-2026 numeric dataset's first board (the track ran at
    # ICAPS Dublin, June 2026; corpus vendored from the public repo).
    "ipc2026-numeric.jsonl": ("2026 numeric (first board)", "modern", 60),
    # The official-budget entry (0.19 cut, locked at scoping): ONE sweep
    # at the competition's 300 s agile budget — an ENTRY, not a baseline.
    "ipc2023-agile-300s.jsonl": ("2023 agile ENTRY (300s)", "modern", 300),
    # The optimal tracks (0.19 Phase 2: Mode::Optimal, A* + h^max —
    # coverage IS proof rate; every solved row carries a certificate).
    "ipc-opt-2008-11.jsonl": ("seq-opt", "optimal", 60),
    "ipc2014-opt.jsonl": ("2014 seq-opt", "optimal", 60),
    # 0.21 Phase 4: the IPC-2026 corpus's three -sat/-opt pairs under
    # Mode::Optimal — the third proof board. Certificates are LENGTH
    # optima: the vendored corpus ships no active :metric anywhere
    # (sailing-wind's is commented out; rainbowttles declares
    # :action-costs with zero total-cost effects).
    "ipc2026-opt.jsonl": ("2026 numeric-opt", "modern", 60),
    # ------------------------------------------------------------------
    # 0.25 Phase 1 — the table grows. New ENTRIES, first swept by
    # entries25-sweeps.sh (a separate sweep from the standing 22 so the
    # like-for-like table keeps its identity; the cut record carries two
    # headlines by design). Every one is an entry, not a movement — no
    # before/after exists until its second cut.
    "ipc2014-mco-t2.jsonl": ("2014 seq-mco t2", "modern", 60),
    "ipc2014-mco-t8.jsonl": ("2014 seq-mco t8", "modern", 60),
    "ipc2018-opt.jsonl": ("2018 seq-opt", "optimal", 60),
    "ipc2023-sat.jsonl": ("2023 seq-sat", "modern", 60),
    "ipc2023-opt.jsonl": ("2023 seq-opt", "optimal", 60),
    # The 2023 numeric corpus under Mode::Optimal — the track whose field
    # receipts (ipc-2023n/results/opt.csv) were vendored with the corpus
    # and never had a board to referee.
    "ipc2023-numeric-opt.jsonl": ("2023 numeric-opt", "modern", 60),
    # The FULL 2026 Overall Optimal constituency (13 domains / 260): the
    # 3-pair ipc2026-opt board above stays as the like-for-like slice.
    "ipc2026-opt-full.jsonl": ("2026 numeric-opt FULL", "modern", 60),
    # The IPC-5 preference tracks at full corpus (the curated 8-instance
    # boards under benchmarks/ipc5-*.md predate these and keep their
    # reference-scored role).
    "ipc5-simple-pref.jsonl": ("simple-preferences (full corpus)", "ipc5", 60),
    "ipc5-qual-pref.jsonl": ("qualitative-preferences (full corpus)", "ipc5", 60),
    # 0.25 Phase 2: the complex-preferences ENTRY — the track ferroplan
    # could never attempt ("last of 3, until the feature ships"). Soft
    # trajectory constraints + goal preferences on temporal domains,
    # scored post-hoc; the metric column carries the PDDL3 preference
    # score. Swept once the entries sweep's driver picks it up.
    "ipc5-complex-pref.jsonl": ("complex-preferences (full corpus)", "ipc5", 60),
}

# our 2006 variant name -> (archive domain dir, archive track dir prefix)
ARCH_DOM = {"tpp": "TPP"}  # everything else is lowercase-identical


def arch_track(variant):
    """Map an ipc-2006 variant name to the archive's track directory."""
    dom, _, rest = variant.partition("-")
    dom = ARCH_DOM.get(dom, dom)
    track = {
        "propositional": "Propositional",
        "propositional-strips": "Propositional/Strips",
        "time": "Time",
        "time-strips": "Time/Strips-Time",
        "metric-time": "MetricTime",
        "metric-time-strips": "MetricTime/Strips-MetricTime",
    }.get(rest)
    return (dom, track) if track else (dom, None)


def load_jsonl(path):
    rows = []
    with open(path) as f:
        for line in f:
            rows.append(json.loads(line))
    return rows


def _load_val_unavailable():
    """Domains VAL cannot INGEST at all (benchmarks/val-unavailable.json).

    VAL has several ways to refuse a domain — "Parser failed",
    "Problem in domain definition!" — and it emits them BEFORE judging any
    plan, so a `val: false` row on such a domain is validation UNAVAILABLE,
    not a rejected plan. The 0.20 runner learned the first signature only,
    which is why data-network-2018 and factory-robot-2026 arrived here as
    false and were dropped from coverage outright: the standings table said
    46/240 and 113/320 where the boards beside it said 53 and 121. Reading
    the map keeps the two artifacts telling the same story about one sweep.
    """
    p = os.path.join(B, "val-unavailable.json")
    if not os.path.exists(p):
        return set()
    return set(json.load(open(p)).get("unavailable", {}))


VAL_UNAVAILABLE = _load_val_unavailable()


def val_unavailable(r):
    return f"{r.get('ipc')}/{r.get('variant')}" in VAL_UNAVAILABLE


def solved(r):
    if not r["solved"]:
        return False
    # A false from a domain VAL cannot read is a harness gap, not a verdict.
    return r.get("val") is not False or val_unavailable(r)


def classify(r, budget):
    # A row measured since 0.23 carries its own `budget` stamp, and it wins
    # over the registry value: the 0.23 temporal tier move means one board
    # name spans raws from two tiers, and the timeout class is denominated
    # in the budget the row actually ran under, not the tier the registry
    # currently declares (see the tier-move comment in SWEEPS).
    budget = r.get("budget") or budget
    if solved(r):
        return "solved"
    if r.get("solved") and r.get("val") is False:
        # The engine reported a plan; VAL rejected it. A first-class
        # signal — either an engine soundness bug or a harness/VAL
        # configuration gap on that corpus — never to be lumped into
        # search losses. The audit record investigates per corpus.
        return "VAL-RED"
    notes = r.get("notes") or ""
    # Solution.notes is a list on engine rows; runner-stamped classes are
    # plain strings. Normalize to one text for the mechanism checks.
    ntext = notes if isinstance(notes, str) else " ".join(str(x) for x in notes)
    if ntext == "mem-cap":
        return "mem-cap"
    if ntext == "spawn-fail":
        # Runner-side fork failure under memory pressure (environmental;
        # see run_instance's retry note in ipc67.py). Pre-0.16-fix sweeps
        # logged these as engine-reject/error — the 0.16 record names the
        # floor-tile t4/t8 cluster explicitly.
        return "spawn-fail"
    t = r.get("time")
    if t is None:
        # Pre-0.20 runner rows only: elapsed was not recorded for
        # unsolved rows, so a graceful engine exit AT the armed wall is
        # indistinguishable from a true reject here. The 0.20 runner
        # records elapsed for every row; this legacy class empties as
        # boards re-sweep (the 0.20 audit showed maintenance-2014's
        # "rejects" were wall-exit timeouts).
        return "engine-reject/error"
    if t >= budget * 0.90:
        # 90, not 95: the refill loop refuses a new round below 10% of an
        # armed wall (search.rs), so a graceful exit in [90%, 100%] is a
        # spent wall by construction. At 95 the ten rows in [90%, 95%)
        # booked as early-exit were a DEFINITIONAL gap between the two
        # lines, not give-ups — the 0.21 decode's boundary-sliver finding.
        return "timeout"
    if ntext.startswith("engine-exit") or "unsolvable" in ntext or "reject" in ntext:
        # A named mechanism: parse/feature reject, grounding verdict, or
        # a nonzero exit without a JSON verdict.
        return "engine-reject/error"
    # Finished early, no plan, no named mechanism: the search gave up
    # with wall budget left (capped ladder, exhaustion). The 0.20 refill
    # loop exists to shrink this class; whatever remains is honest.
    return "early-exit"


def archive_lengths():
    """(domain, track, instance) -> {planner: plan length} from the tgz."""
    if not os.path.exists(ARCHIVE):
        return {}
    out = defaultdict(dict)
    with tarfile.open(ARCHIVE) as t:
        for m in t.getmembers():
            if not m.name.endswith(".soln"):
                continue
            parts = m.name.split("/")  # RESULTS/planner/dom/track.../pNN.soln
            if len(parts) < 5:
                continue
            planner, dom = parts[1], parts[2]
            track = "/".join(parts[3:-1])
            inst = int(re.search(r"p(\d+)\.soln", parts[-1]).group(1))
            body = t.extractfile(m).read().decode(errors="replace")
            n = len(re.findall(r"^\s*[\d.]+\s*:?\s*\(", body, re.M))
            if n:
                out[(dom, track, inst)][planner] = n
    return out


# A timed .soln step: `T: (action) [D]` — sgplan glues the bracket to the
# paren, mips-xxl spaces everything, yochanps lowercases; one regex reads
# all three (the classical `T: (action)` shape leaves group 2 empty).
_SOLN_STEP = re.compile(
    r"^\s*([\d.]+)\s*:\s*\([^)]*\)\s*(?:\[\s*([\d.]+)\s*\])?", re.M)


def archive_makespans():
    """(domain, track, instance) -> {planner: makespan} from the tgz.

    The temporal mirror of archive_lengths (0.23 Phase 3): makespan is
    computed from the timed steps — max(t + duration) — NEVER from the
    `; MakeSpan` header, which is empty on exactly the planner that
    dominates these tracks (sgplan), the same reason the length pass
    counts action lines instead of trusting NrActions. Only Time*/
    MetricTime* members are parsed; the *Constraints track variants land
    in the dict too but no variant of ours ever maps to their keys
    (arch_track), so they are inert, not filtered by guesswork.
    """
    if not os.path.exists(ARCHIVE):
        return {}
    out = defaultdict(dict)
    with tarfile.open(ARCHIVE) as t:
        for m in t.getmembers():
            if not m.name.endswith(".soln"):
                continue
            parts = m.name.split("/")  # RESULTS/planner/dom/track.../pNN.soln
            if len(parts) < 5:
                continue
            planner, dom = parts[1], parts[2]
            track = "/".join(parts[3:-1])
            if "Time" not in track:
                continue
            inst = int(re.search(r"p(\d+)\.soln", parts[-1]).group(1))
            body = t.extractfile(m).read().decode(errors="replace")
            ms = 0.0
            for st in _SOLN_STEP.finditer(body):
                ms = max(ms, float(st.group(1)) +
                         (float(st.group(2)) if st.group(2) else 0.0))
            if ms > 0:
                out[(dom, track, inst)][planner] = ms
    return out


# Makespan W/T/L tie band: one ε slot at the COARSEST granularity on either
# side of the comparison (sgplan's archive plans stagger at 0.01 where ours
# ε-separate at 0.001), so ε bookkeeping can never book a win or a loss —
# the quality ratio itself stays raw division, uncushioned.
MS_TIE = 0.011


def makespan_quality(rows, arch_ms):
    """IPC-2008-style quality on the temporal tracks' currency (0.23
    Phase 3) — the mirror of the propositional length path: best-of-field
    makespan / ours, capped at 1, plus W/T/L. Scores ONLY rows that carry
    a `makespan` (recorded since 0.22), so a cloud-era raw yields None and
    the caller keeps its coverage-only note instead of a guessed column.
    """
    w = t_ = l = 0
    ratios = []
    for r in rows:
        ours = r.get("makespan")
        if not solved(r) or not ours or ours <= 0:
            continue
        dom, track = arch_track(r["variant"])
        field = arch_ms.get((dom, track, r["instance"]), {})
        if not field:
            continue
        best = min(field.values())
        ratios.append(min(best / ours, 1.0))
        if ours < best - MS_TIE:
            w += 1
        elif ours > best + MS_TIE:
            l += 1
        else:
            t_ += 1
    if not ratios:
        return None
    return (
        f"makespan vs best-of-field: {w}W/{t_}T/{l}L, "
        f"mean quality {sum(ratios)/len(ratios):.2f} "
        f"({len(ratios)} scored)"
    )


def coverage_line(rows, budget):
    n = len(rows)
    s = sum(1 for r in rows if solved(r))
    cls = defaultdict(int)
    for r in rows:
        cls[classify(r, budget)] += 1
    fails = ", ".join(
        f"{v} {k}" for k, v in sorted(cls.items()) if k != "solved" and v
    )
    return s, n, fails or "none"


GH_BLOB = "https://github.com/hhh42/ferroplan/blob/main/"
SUMMARY = os.path.join(ROOT, "STANDINGS.md")
HISTORY = os.path.join(B, "standings-history.json")
# Tracks where coverage IS proof rate: a solved row carries an optimality
# certificate, so 45% there is a categorically different claim from 45% on a
# satisficing board and must not be read as "worse".
PROOF_TRACKS = {"seq-opt", "2014 seq-opt", "2026 numeric-opt",
                # 0.25 Phase 1 entries — proof boards from birth.
                "2018 seq-opt", "2023 seq-opt", "2023 numeric-opt",
                "2026 numeric-opt FULL"}


# --------------------------------------------------------------------------
# The vs-field column (0.25 Phase 1): field placement as DATA, not a
# hand-refreshed page. Cohorts come from benchmarks/field-results.json
# (the ipc-rankings.md numbers, promoted to machine-readable with their
# provenance) plus the vendored official IPC-2023 numeric CSVs, parsed
# live. A cell is a rough coverage-rate placement under the standing
# caveats (30x budget gap, hardware confound, coverage != IPC's quality
# formula) — never a claimed result. docs/ipc-rankings.md stays the
# prose companion.
def load_field():
    out = {}
    p = os.path.join(B, "field-results.json")
    if os.path.exists(p):
        out.update(json.load(open(p)).get("cohorts", {}))
    import csv as _csv
    for label, fname in (("2023 numeric", "sat.csv"),
                         ("2023 numeric-opt", "opt.csv")):
        cp = os.path.join(B, ".ipc-corpus", "ipc-2023n", "results", fname)
        if not os.path.exists(cp):
            continue
        with open(cp, encoding="utf-8-sig") as f:
            rows = list(_csv.reader(f))
        names = [n.strip() for n in rows[0][2:] if n.strip()]
        tot = [0] * len(names)
        doms = 0
        total_row = None
        for r in rows[1:]:
            if len(r) < 3:
                continue
            # Domain rows carry a group tag in col 1 (SNP/LNP); the
            # trailing summary rows ("Total", per-group) leave it empty —
            # summing those in triples every count.
            if not r[0].strip():
                if r[1].strip() == "Total":
                    total_row = r
                continue
            doms += 1
            for i in range(len(names)):
                try:
                    tot[i] += int(r[2 + i])
                except (ValueError, IndexError):
                    pass
        if total_row is not None:
            # Prefer the official Total row verbatim over our own sum.
            for i in range(len(names)):
                try:
                    tot[i] = int(total_row[2 + i])
                except (ValueError, IndexError):
                    pass
        of = doms * 20
        out[label] = {
            "entrants": [[n, t, of] for n, t in zip(names, tot)],
            "field_size": len(names),
            "note": "official per-domain CSV (ipc-2023n/results), parsed live",
            "confidence": "high",
        }
    return out


def _ord(n):
    if 10 <= n % 100 <= 20:
        return "th"
    return {1: "st", 2: "nd", 3: "rd"}.get(n % 10, "th")


def _placement(cohort, s, n):
    """Rank our s/n among a cohort's entrants by coverage RATE (the only
    currency that survives mismatched denominators). '~' marks a field
    with unlocated entrants — the rank is a floor on ignorance, and says
    so by being approximate."""
    ents = cohort.get("entrants") or []
    if not ents or not n:
        return None
    ours = s / n
    ahead = sum(1 for _, es, eo in ents if eo and es / eo > ours)
    known = len(ents)
    fs = cohort.get("field_size") or known
    total = fs + 1  # the field plus us, the ipc-rankings.md convention
    approx = "~" if fs > known else ""
    lead = max(ents, key=lambda e: (e[1] / e[2]) if e[2] else 0.0)
    r = ahead + 1
    # A sparse entrant list makes strict-rank optimistic; a cohort that
    # KNOWS more entrants sit ahead than it lists carries a rank_floor
    # (e.g. "7 confirmed entrants span 163-198", "below the field
    # median of 24"), and the cell says ≥ instead of pretending. The
    # floor is CONDITIONAL on its justifying entrant still being ahead
    # (rank_floor_if_behind names it) — a future cut that passes that
    # mark must not inherit a stale pessimism either.
    floor = cohort.get("rank_floor", 1)
    justif = cohort.get("rank_floor_if_behind")
    if justif is not None:
        je = next((e for e in ents if e[0] == justif), None)
        if not (je and je[2] and je[1] / je[2] > ours):
            floor = 1
    if floor > r:
        r, approx = floor, "≥"
    return (f"{approx}{r}{_ord(r)} of {total} by rate "
            f"(leader {lead[0]} {lead[1]}/{lead[2]})")


def field_cell(field, label, rows, s, n):
    cohort = field.get(label)
    if not cohort:
        return "—"
    if "splits" in cohort:
        parts = []
        for ipc, sub in sorted(cohort["splits"].items()):
            rs = [r for r in rows if r.get("ipc") == ipc]
            ss = sum(1 for r in rs if solved(r))
            p = _placement(sub, ss, len(rs))
            if p:
                parts.append(f"{ipc[-4:]}: {p}")
        return " · ".join(parts) if parts else "—"
    p = _placement(cohort, s, n)
    return p or "—"


def _history():
    if not os.path.exists(HISTORY):
        return []
    return json.load(open(HISTORY)).get("snapshots", [])


def _current_version():
    """Workspace version, so the delta column never compares a release to
    ITSELF. The snapshot for the release being cut is banked from the same
    boards this table is generated from, so without this every row reads
    `= (vs X)` — technically true and completely useless."""
    try:
        for line in open(os.path.join(ROOT, "Cargo.toml")):
            if line.strip().startswith("version"):
                return line.split("=", 1)[1].strip().strip('"')
    except OSError:
        pass
    return None


def _bar(pct, width=20):
    filled = int(round(pct / 100 * width))
    return "█" * filled + "░" * (width - filled)


def write_summary(data):
    """STANDINGS.md — the at-a-glance view.

    ipc-standings.md is the reference table: every track, every failure class,
    five columns of prose. Correct, and useless for the question "roughly where
    are we?". This answers that question and links there for the detail.

    The delta column obeys one rule: a snapshot is only compared against a
    predecessor measured on the SAME BOX. Faster silicon inflates coverage at a
    fixed budget, so a cloud->Air "improvement" would be pure hardware. Where
    there is no comparable predecessor the column says so instead of inventing
    a number.
    """
    hist = _history()
    box = os.environ.get("FERROPLAN_BOX", "m5-air")
    cur = _current_version()

    def vkey(v):
        try:
            return tuple(int(x) for x in str(v).split("."))
        except ValueError:
            return (0,)

    # The PREVIOUS RELEASE, by version — not the most recently MEASURED
    # snapshot. A backfilled old tag is measured late (0.19 was re-swept after
    # 0.20 shipped), so picking by measured_at silently compares a release to
    # its grandparent and skips one. On 2018-sat that is the difference between
    # +7 and +17.
    cands = [s for s in hist
             if s.get("measured_on") == box and vkey(s.get("version")) < vkey(cur)]
    prev = max(cands, key=lambda s: vkey(s.get("version"))) if cands else None

    live, cloud, pending = [], [], []
    for label, d in data.items():
        if d is None:
            # Two very different absences, and collapsing them would let a
            # half-promoted sweep read as "we never re-measured this".
            (pending if label in AIR_REBASELINED else cloud).append(label)
            continue
        rows, budget = d
        s, n, _ = coverage_line(rows, budget)
        if not n:
            continue
        live.append((label, s, n, 100.0 * s / n, rows))
    live.sort(key=lambda r: -r[3])
    field = load_field()

    tot_s = sum(r[1] for r in live)
    tot_n = sum(r[2] for r in live)
    proofs = sum(r[1] for r in live if r[0] in PROOF_TRACKS)

    L = ["# Where ferroplan stands", ""]
    if tot_n:
        plural = "board" if len(live) == 1 else "boards"
        L += [f"**{100.0 * tot_s / tot_n:.0f}% coverage across {len(live)} IPC "
              f"{plural}** ({tot_s:,}/{tot_n:,}), measured on `{box}`."]
        if proofs:
            L += ["", f"Of those, **{proofs:,} are certified optima** — on the "
                  "optimal tracks coverage IS proof rate, so a solved row is a "
                  "proof, not a plan."]
    L += ["", "Generated by `python3 benchmarks/standings.py` — do not hand-edit. "
          "Per-track detail, quality scoring and failure classes: "
          "[`benchmarks/ipc-standings.md`](benchmarks/ipc-standings.md).", ""]

    bands = [("Strong", lambda p: p >= 60), ("Middle", lambda p: 25 <= p < 60),
             ("Weak", lambda p: p < 25)]
    for name, pred in bands:
        band_rows = [r for r in live if pred(r[3])]
        if not band_rows:
            continue
        L += [f"## {name}", "",
              "| track | coverage | | vs previous | vs field |",
              "|---|---|---|---|---|"]
        for label, s, n, pct, rows in band_rows:
            mark = " ⚖️" if label in PROOF_TRACKS else ""
            L.append(f"| {label}{mark} | {s}/{n} | `{_bar(pct)}` {pct:.0f}% "
                     f"| {_delta(label, s, n, prev)} "
                     f"| {field_cell(field, label, rows, s, n)} |")
        L += [""]

    if pending:
        L += ["## Awaiting promotion", "",
              "Swept on this box but not yet promoted into the table — a sweep "
              "still in flight, or boards written but not published. Not a "
              "measurement gap.", "",
              "".join(f"`{c}` · " for c in sorted(pending)).rstrip(" ·"), ""]
    if cloud:
        L += ["## Not re-baselined", "",
              "These still carry numbers from the previous machine. They are NOT "
              "comparable to the boards above and are excluded from the headline "
              "total — the old numbers stay in git history.", "",
              "".join(f"`{c}` · " for c in sorted(cloud)).rstrip(" ·"), ""]

    L += ["## How to read this", "",
          "- **coverage** is solved/total at that track's official-ish budget "
          "(60 s satisficing, 60 s temporal, 300 s where marked an entry).",
          "- **⚖️ marks a proof track**: coverage is the share of instances "
          "PROVEN optimal, a far harder bar than finding some plan.",
          "- **vs previous** compares only against a release measured on the same "
          "hardware. A blank means no comparable predecessor exists yet, not zero "
          "movement.",
          "- **vs field** is a rough coverage-RATE placement against that "
          "competition's actual entrants (data: `benchmarks/field-results.json` "
          "+ the vendored official IPC-2023n CSVs), under the standing caveats "
          "— official budgets are ~30× ours, and coverage ≠ IPC's "
          "quality-weighted scoring. `~` marks a field with unlocated "
          "entrants; `—` means no per-entrant field data is held. Prose and "
          "provenance: [`docs/ipc-rankings.md`](docs/ipc-rankings.md).",
          "- A board is only as honest as its conditions; those are recorded per "
          "cycle in `docs/roadmap-0.N.md`.", ""]
    with open(SUMMARY, "w") as f:
        f.write("\n".join(L) + "\n")
    print(f"wrote {SUMMARY}")
    _patch_readme(live, tot_s, tot_n, proofs, box)


README_BEGIN = "<!-- STANDINGS:BEGIN"
README_END = "<!-- STANDINGS:END -->"


def _patch_readme(live, tot_s, tot_n, proofs, box):
    """Rewrite the README's headline block between its markers.

    Hand-maintained numbers on a front page drift the moment a sweep lands, and
    a stale headline is worse than none — so the shop window is generated from
    the same data as the table behind it.
    """
    p = os.path.join(ROOT, "README.md")
    if not os.path.exists(p) or not tot_n:
        return
    text = open(p).read()
    i, j = text.find(README_BEGIN), text.find(README_END)
    if i < 0 or j < 0:
        return
    head_end = text.find("-->", i) + 3
    top = [f"| track | coverage | |", "|---|---|---|"]
    for label, s, n, pct, _rows in live[:5]:
        mark = " ⚖️" if label in PROOF_TRACKS else ""
        top.append(f"| {label}{mark} | {s}/{n} | `{_bar(pct, 16)}` {pct:.0f}% |")
    block = [
        "",
        f"**{100.0 * tot_s / tot_n:.0f}% coverage across {len(live)} IPC "
        f"boards** ({tot_s:,}/{tot_n:,}) on `{box}`"
        + (f", including **{proofs:,} certified optima**." if proofs else "."),
        "",
        *top,
        "",
        # ABSOLUTE urls, always. This block is generated into README.md, which
        # ships as the crate README (`readme = "../../README.md"`), and
        # crates.io/docs.rs resolve relative links against the CRATE dir —
        # so `STANDINGS.md` there resolves to crates/ferroplan/STANDINGS.md
        # and 404s. Every other link in that README is absolute for the same
        # reason.
        f"Best five shown. **[Full standings → `STANDINGS.md`]({GH_BLOB}STANDINGS.md)** · "
        "per-track detail, quality scoring and failure classes in "
        f"[`benchmarks/ipc-standings.md`]({GH_BLOB}benchmarks/ipc-standings.md).",
        "",
    ]
    with open(p, "w") as f:
        f.write(text[:head_end] + "\n".join(block) + text[j:])
    print(f"patched {p} standings block")


def _delta(label, s, n, prev):
    if not prev:
        return "— *baseline*"
    p = prev.get("tracks", {}).get(label)
    if not p or not p.get("total"):
        return "— *new*"
    # Compare shares, not raw counts: a corpus can grow between releases.
    was = 100.0 * p["solved"] / p["total"]
    now = 100.0 * s / n
    d = now - was
    if abs(d) < 0.05:
        return f"= (vs {prev['version']})"
    return f"{'+' if d > 0 else '−'}{abs(d):.1f} pts (vs {prev['version']})"


def main():
    arch = archive_lengths()
    arch_ms = archive_makespans()
    lines = [
        "# IPC standings — the one honest table per competition",
        "",
        "Generated by `python3 benchmarks/standings.py` (do not hand-edit;",
        "regenerate after any sweep). Raw inputs are the per-instance JSONLs",
        "and the vendored official IPC-5 archive — see the module docstring",
        "for scoring semantics and the failure-class definitions.",
        "",
    ]
    # A raw JSONL counts only once its .md scoreboard sibling exists —
    # ipc67.py writes the .md at sweep END, so a lone JSONL is a sweep
    # still in flight and must not masquerade as a completed row. The
    # promoted baselines' scoreboards live under different names.
    MD_FOR = {
        "ipc67-default.jsonl": "ipc67-results.md",
        "ipc67-temporal.jsonl": "ipc67-temporal.md",
    }
    data = {}
    for fname, (label, comp, budget) in SWEEPS.items():
        p = os.path.join(B, fname)
        md = os.path.join(B, MD_FOR.get(fname, fname.replace(".jsonl", ".md")))
        done = os.path.exists(p) and os.path.exists(md)
        data[label] = (load_jsonl(p), budget) if done else None

    # ---------------- IPC-5 ----------------
    lines += ["## IPC-5 (2006)", ""]
    ip5 = [
        ("propositional", "quality vs field"),
        # The fallback text renders only when a raw exists but carries no
        # makespan column (a pre-0.22 runner's rows); a scored raw gets the
        # makespan_quality line instead. The 0.14-era runner debt itself is
        # CLOSED (0.22: 486/486 solved temporal rows carry makespan).
        ("time", "coverage-only (raw predates the 0.22 makespan column)"),
        ("metric-time",
         "coverage-only (raw predates the 0.22 makespan column)"),
        ("constraints", "coverage-only (timed modal ops rejected by name)"),
        # 0.25 Phase 1: the preference tracks at full corpus (the curated
        # 8-instance reference-scored boards keep their own files).
        ("simple-preferences (full corpus)",
         "coverage = hard-goal solves; preference metric in the raw"),
        ("qualitative-preferences (full corpus)",
         "coverage = hard-goal solves; preference metric in the raw"),
        ("complex-preferences (full corpus)",
         "coverage = hard-goal solves; PDDL3 preference metric scored "
         "post-hoc in the raw (0.25 Phase 2 entry)"),
    ]
    lines += [
        "| track | entered | coverage | quality | failure classes |",
        "|---|---|---|---|---|",
    ]
    prop_quality = ""
    for label, qnote in ip5:
        d = data.get(label)
        if d is None:
            lines.append(f"| {label} | {absent(label)} | — | — | — |")
            continue
        rows, budget = d
        s, n, fails = coverage_line(rows, budget)
        q = qnote
        if label == "propositional" and arch:
            w = t_ = l = 0
            ratios = []
            for r in rows:
                if not solved(r) or r.get("length") is None:
                    continue
                dom, track = arch_track(r["variant"])
                field = arch.get((dom, track, r["instance"]), {})
                if not field:
                    continue
                best = min(field.values())
                ours = r["length"]
                ratios.append(min(best / ours, 1.0))
                w += ours < best
                t_ += ours == best
                l += ours > best
            if ratios:
                q = (
                    f"len vs best-of-field: {w}W/{t_}T/{l}L, "
                    f"mean quality {sum(ratios)/len(ratios):.2f} "
                    f"({len(ratios)} scored)"
                )
                prop_quality = q
        # The temporal quality currency (0.23 Phase 3): renders only off a
        # re-baselined raw — makespan_quality returns None on rows without
        # the 0.22 makespan column, so a cloud-era ghost cannot acquire a
        # quality number it never measured.
        if label in ("time", "metric-time") and arch_ms:
            q = makespan_quality(rows, arch_ms) or q
        lines.append(f"| {label} | yes | {s}/{n} | {q} | {fails} |")
    lines += [
        "| simple-preferences | yes | see board | reference-scored — "
        "[`ipc5-scoreboard.md`](ipc5-scoreboard.md) | — |",
        "| qualitative-preferences | yes | see board | reference-scored — "
        "[`ipc5-qualitative-scoreboard.md`](ipc5-qualitative-scoreboard.md)"
        " (24W/4T/10L vs SGPlan5 — ahead of the winner; rovers/storage/tpp"
        " won outright) | — |",
        "| complex-preferences | no (modal operators rejected by name) "
        "| — | — | feature gap, on the deferred list |",
        "",
    ]

    # ---------------- IPC-6 / IPC-7 shared sweeps ----------------
    def split_rows(label, ipc):
        d = data.get(label)
        if d is None:
            return None
        rows, budget = d
        sub = [r for r in rows if r.get("ipc") == ipc]
        return (sub, budget) if sub else None

    lines += ["## IPC-6 (2008)", ""]
    lines += [
        "| track | entered | coverage | quality | failure classes |",
        "|---|---|---|---|---|",
    ]
    for label, key in [("seq-sat", "seq-sat"), ("tempo-sat", "tempo-sat"),
                       ("net-benefit", "net-benefit")]:
        d = split_rows(key, "ipc-2008")
        if d is None:
            lines.append(f"| {label} | {absent(label)} | — | — | — |")
            continue
        rows, budget = d
        s, n, fails = coverage_line(rows, budget)
        lines.append(
            f"| {label} | yes | {s}/{n} | coverage + VAL "
            f"(no official per-instance archive vendored) | {fails} |"
        )
    d = split_rows("seq-opt", "ipc-2008")
    if d is None:
        lines.append(f"| seq-opt | {absent('seq-opt')} | — | — | — |")
    else:
        rows, budget = d
        s_, n, fails = coverage_line(rows, budget)
        lines.append(
            f"| seq-opt | yes (first entry, 0.19 — Mode::Optimal) | {s_}/{n} "
            "| coverage = PROOF RATE (A* + admissible LM-cut, h^max sprint "
            "first; every plan "
            f"certified + VAL) | {fails} |"
        )
    lines += [
        "| tempo-opt | out of scope by design (satisficing temporal "
        "path) | — | — | — |",
        "",
    ]

    lines += ["## IPC-7 (2011)", ""]
    lines += [
        "| track | entered | coverage | quality | failure classes |",
        "|---|---|---|---|---|",
    ]
    for label, key in [("seq-sat", "seq-sat"), ("tempo-sat", "tempo-sat")]:
        d = split_rows(key, "ipc-2011")
        if d is None:
            lines.append(f"| {label} | {absent(label)} | — | — | — |")
            continue
        rows, budget = d
        s, n, fails = coverage_line(rows, budget)
        lines.append(
            f"| {label} | yes | {s}/{n} | coverage + VAL | {fails} |"
        )
    # The mco methodology string renders on absent rows too: the rule is a
    # DECISION (see the SWEEPS comment), and it should be readable before
    # the sweep lands, not only after.
    MCO_Q = ("wall-clock per competition rule (--threads N, one instance "
             "at a time; 4P+6E box — t8 oversubscribed by construction)")
    for label in ("seq-mco t2", "seq-mco t4", "seq-mco t8"):
        d = data.get(label)
        if d is None:
            lines.append(f"| {label} | {absent(label)} | — | {MCO_Q} | — |")
            continue
        rows, budget = d
        s, n, fails = coverage_line(rows, budget)
        lines.append(
            f"| {label} | yes (first entry, 0.16) | {s}/{n} | {MCO_Q} | "
            f"{fails} |"
        )
    d = split_rows("seq-opt", "ipc-2011")
    if d is None:
        lines.append(f"| seq-opt | {absent('seq-opt')} | — | — | — |")
    else:
        rows, budget = d
        s_, n, fails = coverage_line(rows, budget)
        lines.append(
            f"| seq-opt | yes (first entry, 0.19 — Mode::Optimal) | {s_}/{n} "
            "| coverage = PROOF RATE (A* + admissible LM-cut, h^max sprint "
            "first; every plan "
            f"certified + VAL) | {fails} |"
        )
    lines.append("")

    # ---------------- The modern corpora (0.17) ----------------
    corpus = os.path.join(B, ".ipc-corpus")

    def load_bounds():
        """Best-known cost per (year, domain, instance) from the official
        bounds files (2023: dict path->[lo,hi]; 2018: list of [path,cost],
        several entries per instance — the minimum is the best known)."""
        best = {}
        p23 = os.path.join(corpus, "ipc-2023", "bounds.json")
        if os.path.exists(p23):
            # agl/, sat/ and opt/ carry DIFFERENT instance sets under the
            # same domain names (0.25: the sat/opt boards joined), so the
            # key is track-scoped — "2023-agl" etc., never bare "2023".
            for path, (_, hi) in json.load(open(p23)).items():
                m = re.match(r"(agl|sat|opt)/([\w-]+)/p(\d+)\.pddl", path)
                if m and hi is not None:
                    k = (f"2023-{m.group(1)}", m.group(2), int(m.group(3)))
                    best[k] = float(hi)
        p18 = os.path.join(corpus, "ipc-2018", "cost_bounds.json")
        if os.path.exists(p18):
            for path, cost in json.load(open(p18)):
                m = re.match(r"sat/([\w-]+)/p(\d+)\.pddl", path)
                if m and cost is not None:
                    k = ("2018", m.group(1), int(m.group(2)))
                    best[k] = min(best.get(k, float("inf")), float(cost))
        return best

    def bounds_quality(rows, year, suffix):
        best = load_bounds()
        w = t = l = 0
        ratios = []
        for r in rows:
            if not solved(r):
                continue
            dom = r["variant"].removesuffix(suffix)
            ref = best.get((year, dom, r["instance"]))
            ours = r.get("metric") if r.get("metric") is not None else r.get("length")
            if ref is None or ours is None:
                continue
            ratios.append(min(ref / ours, 1.0) if ours else 1.0)
            w += ours < ref
            t += ours == ref
            l += ours > ref
        if not ratios:
            return None
        return (
            f"vs best-known bounds: {w}W/{t}T/{l}L, mean quality "
            f"{sum(ratios)/len(ratios):.2f} ({len(ratios)} scored)"
        )

    lines += ["## The modern corpora (IPC 2014 / 2018 / 2023 — first entered 0.17)", ""]
    lines += [
        "| track | entered | coverage | quality | failure classes |",
        "|---|---|---|---|---|",
    ]
    MODERN_Q = {
        "2018 seq-sat": ("2018", "-sequential-satisficing"),
        "2023 classical": ("2023-agl", "-agile"),
        # 0.25: the real 2023 satisficing track, scored against the same
        # vendored bounds file's sat/ keys. The opt/ board is a PROOF
        # board and takes the proof-rate note below instead.
        "2023 seq-sat": ("2023-sat", "-satisficing"),
    }
    for label in ["2014 seq-sat", "2014 seq-agile", "2014 tempo-sat",
                  "2014 seq-mco t2", "2014 seq-mco t4", "2014 seq-mco t8",
                  "2014 seq-opt", "2018 seq-sat", "2018 seq-opt",
                  "2023 classical", "2023 seq-sat", "2023 seq-opt",
                  "2023 agile ENTRY (300s)",
                  "2023 numeric", "2023 numeric-opt",
                  # 0.20 cut prep added this board to SWEEPS but never to the
                  # render list, so it could never have appeared in the table.
                  "2026 numeric (first board)",
                  "2026 numeric-opt", "2026 numeric-opt FULL"]:
        d = data.get(label)
        if d is None:
            lines.append(f"| {label} | {absent(label)} | — | — | — |")
            continue
        rows, budget = d
        s, n, fails = coverage_line(rows, budget)
        if label in MODERN_Q:
            q = bounds_quality(rows, *MODERN_Q[label]) or "coverage-only"
        elif label == "2023 agile ENTRY (300s)":
            q = ("OFFICIAL 300 s budget — a competition-methodology ENTRY, "
                 "not a baseline")
        elif label == "2026 numeric (first board)":
            q = ("coverage + VAL; the corpus ships -sat/-opt domain PAIRS, all "
                 "swept satisficing-style on this first board")
        elif label == "2026 numeric-opt":
            q = ("coverage = PROOF RATE (Mode::Optimal over the three -opt "
                 "pairs; LENGTH optima — the vendored corpus carries no "
                 "active :metric; every certificate VAL-checked)")
        elif label == "2023 numeric":
            q = ("field CSVs vendored (ipc-2023n/results) — per-domain "
                 "comparison in the audit record")
        elif label == "2014 seq-mco t4":
            q = ("wall-clock per competition rule (--threads 4, one "
                 "instance at a time; 4P+6E box)")
        elif label in ("2014 seq-opt", "2018 seq-opt", "2023 seq-opt"):
            q = ("coverage = PROOF RATE (Mode::Optimal, A* + admissible "
                 "LM-cut, h^max sprint first; every plan certified + VAL)")
        elif label in ("2014 seq-mco t2", "2014 seq-mco t8"):
            q = ("wall-clock per competition rule (one instance at a "
                 "time; 4P+6E box" +
                 (", t8 oversubscribed by construction)" if "t8" in label
                  else ")"))
        elif label == "2023 numeric-opt":
            q = ("coverage = PROOF RATE over the numeric corpus; the "
                 "track's official field CSV (ipc-2023n/results/opt.csv) "
                 "is the vs-field referee")
        elif label == "2026 numeric-opt FULL":
            q = ("coverage = PROOF RATE over the official 13-domain/260 "
                 "Overall Optimal constituency (the 3-pair board above "
                 "is the like-for-like slice)")
        else:
            q = "coverage + VAL"
        NEW_25 = ("2014 seq-mco t2", "2014 seq-mco t8", "2018 seq-opt",
                  "2023 seq-sat", "2023 seq-opt", "2023 numeric-opt",
                  "2026 numeric-opt FULL")
        entered = ("yes (first entry, 0.19)" if label == "2014 seq-opt"
                   else "yes (OFFICIAL-BUDGET entry, 0.19)" if label == "2023 agile ENTRY (300s)"
                   # The 2026 corpus was a blocked rider at 0.20 scoping (the
                   # organisers had not published yet) and is swept here for
                   # the first time — not a 0.17 board.
                   else "yes (FIRST ENTRY, 0.20 — new corpus)"
                   if label == "2026 numeric (first board)"
                   else "yes (FIRST ENTRY, 0.21 — the -opt pairs, ⚖️)"
                   if label == "2026 numeric-opt"
                   # 0.25 Phase 1: the table grows — entries, not movement.
                   else "yes (FIRST ENTRY, 0.25 — the table grows)"
                   if label in NEW_25
                   else "yes (first entry, 0.17)")
        lines.append(f"| {label} | {entered} | {s}/{n} | {q} | {fails} |")
    lines += [
        "",
        "The 2023 classical corpus is swept on its agile instances at the "
        "standard 60 s satisficing budget (the competition's agile budget "
        "is 300 s — these rows are BASELINES, marked as such, not "
        "competition entries).",
        "",
    ]

    with open(OUT, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"wrote {OUT}")
    write_summary(data)
    if prop_quality:
        print(f"prop-2006 {prop_quality}")


if __name__ == "__main__":
    main()
