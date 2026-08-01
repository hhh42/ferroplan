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
  - IPC-5 time / metric-time / constraints: coverage-only. The honest
    reason, on the record: the runner does not record MAKESPAN (the
    track's quality currency) — a named runner debt, not an archive gap.
  - IPC-6/7 tracks: coverage (+ VAL) against standing baselines; no
    official per-instance archive is vendored for 2008/2011.

Failure classes per unsolved instance (from the JSONL):
  timeout (elapsed >= 95% of budget — including graceful engine exits
  AT an armed FF_TIME_LIMIT wall), mem-cap (notes), engine-reject/error
  (a named mechanism: parse/feature reject, grounding verdict, nonzero
  exit, or a legacy pre-0.20 row with no elapsed recorded), else
  early-exit (search gave up with wall budget left — the class the
  0.20 refill loop exists to shrink).
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
    "2023 numeric", "2023 agile ENTRY (300s)",
    # shared-sweep labels, split per competition at render time
    "seq-opt", "tempo-sat", "seq-sat",
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
    "ipc67-temporal.jsonl": ("tempo-sat", "ipc67", 30),
    "ipc67-netben.jsonl": ("net-benefit", "ipc67", 60),
    "ipc7-mco-t2.jsonl": ("seq-mco t2", "ipc7", 60),
    "ipc7-mco-t4.jsonl": ("seq-mco t4", "ipc7", 60),
    "ipc7-mco-t8.jsonl": ("seq-mco t8", "ipc7", 60),
    # The modern corpora (0.17 frontier cycle).
    "ipc2014-sat.jsonl": ("2014 seq-sat", "modern", 60),
    "ipc2014-agile.jsonl": ("2014 seq-agile", "modern", 60),
    "ipc2014-tempo.jsonl": ("2014 tempo-sat", "modern", 30),
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


def solved(r):
    return bool(r["solved"]) and (r.get("val") is not False)


def classify(r, budget):
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
    if t >= budget * 0.95:
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


def main():
    arch = archive_lengths()
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
        ("time", "coverage-only (makespan not recorded — runner debt)"),
        ("metric-time", "coverage-only (makespan not recorded — runner debt)"),
        ("constraints", "coverage-only (timed modal ops rejected by name)"),
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
    for label in ("seq-mco t2", "seq-mco t4", "seq-mco t8"):
        d = data.get(label)
        if d is None:
            lines.append(
                f"| {label} | {absent(label)} | — | "
                "wall-clock per competition rule (4-core box; t8 "
                "oversubscribed) | — |"
            )
            continue
        rows, budget = d
        s, n, fails = coverage_line(rows, budget)
        lines.append(
            f"| {label} | yes (first entry, 0.16) | {s}/{n} | wall-clock "
            "per competition rule (4-core box; t8 oversubscribed) | "
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
            for path, (_, hi) in json.load(open(p23)).items():
                m = re.match(r"agl/([\w-]+)/p(\d+)\.pddl", path)
                if m and hi is not None:
                    best[("2023", m.group(1), int(m.group(2)))] = float(hi)
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
        "2023 classical": ("2023", "-agile"),
    }
    for label in ["2014 seq-sat", "2014 seq-agile", "2014 tempo-sat",
                  "2014 seq-mco t4", "2014 seq-opt", "2018 seq-sat",
                  "2023 classical", "2023 agile ENTRY (300s)",
                  "2023 numeric",
                  # 0.20 cut prep added this board to SWEEPS but never to the
                  # render list, so it could never have appeared in the table.
                  "2026 numeric (first board)"]:
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
        elif label == "2023 numeric":
            q = ("field CSVs vendored (ipc-2023n/results) — per-domain "
                 "comparison in the audit record")
        elif label == "2014 seq-mco t4":
            q = "wall-clock per competition rule (4-core box)"
        elif label == "2014 seq-opt":
            q = ("coverage = PROOF RATE (Mode::Optimal, A* + admissible "
                 "LM-cut, h^max sprint first; every plan certified + VAL)")
        else:
            q = "coverage + VAL"
        entered = ("yes (first entry, 0.19)" if label == "2014 seq-opt"
                   else "yes (OFFICIAL-BUDGET entry, 0.19)" if label == "2023 agile ENTRY (300s)"
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
    if prop_quality:
        print(f"prop-2006 {prop_quality}")


if __name__ == "__main__":
    main()
