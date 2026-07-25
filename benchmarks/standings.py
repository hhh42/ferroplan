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
  timeout (time == budget), mem-cap (notes), engine-reject/error
  (no time recorded — parse/feature rejects land here), else search.
"""

import json
import os
import re
import sys
import tarfile
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
B = os.path.join(ROOT, "benchmarks")
OUT = os.path.join(B, "ipc-standings.md")
ARCHIVE = os.path.join(B, "IPC5-results.tgz")

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
    notes = r.get("notes") or ""
    if notes == "mem-cap":
        return "mem-cap"
    if notes == "spawn-fail":
        # Runner-side fork failure under memory pressure (environmental;
        # see run_instance's retry note in ipc67.py). Pre-0.16-fix sweeps
        # logged these as engine-reject/error — the 0.16 record names the
        # floor-tile t4/t8 cluster explicitly.
        return "spawn-fail"
    t = r.get("time")
    if t is None:
        return "engine-reject/error"
    if t >= budget * 0.95:
        return "timeout"
    return "search"


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
            lines.append(f"| {label} | not swept | — | — | — |")
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
            lines.append(f"| {label} | not swept | — | — | — |")
            continue
        rows, budget = d
        s, n, fails = coverage_line(rows, budget)
        lines.append(
            f"| {label} | yes | {s}/{n} | coverage + VAL "
            f"(no official per-instance archive vendored) | {fails} |"
        )
    lines += [
        "| seq-opt / tempo-opt | out of scope by design (satisficing "
        "planner) | — | — | — |",
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
            lines.append(f"| {label} | not swept | — | — | — |")
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
                f"| {label} | sweep in flight / not yet run | — | "
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
    lines += [
        "| seq-opt | out of scope by design (satisficing planner) | — | "
        "— | — |",
        "",
    ]

    with open(OUT, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"wrote {OUT}")
    if prop_quality:
        print(f"prop-2006 {prop_quality}")


if __name__ == "__main__":
    main()
