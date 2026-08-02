#!/usr/bin/env python3
"""Bank the current standings into benchmarks/standings-history.json.

Run once per release, after the boards are promoted and standings.py has been
regenerated. The history is what lets STANDINGS.md show movement at all.

    python3 scripts/standings-snapshot.py --version 0.20.0 [--box m5-air] \
        [--released 2026-08-01] [--note "..."]

The `measured_on` (box) field is load-bearing, not decoration. Coverage at a
fixed time budget is a property of the HARDWARE as much as the engine, so a
snapshot is only ever compared against a predecessor from the same box —
STANDINGS.md enforces that. It is also why `measured_at` is separate from
`released`: an old version re-swept on new hardware to backfill the trend is
honestly a today measurement of a July release, and must never be presented as
a July number.
"""
import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)),
                                "..", "benchmarks"))
import standings as S  # noqa: E402

HISTORY = S.HISTORY


def arg(name, default=None):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


def main():
    version = arg("--version")
    if not version:
        sys.exit("--version is required (e.g. --version 0.20.0)")
    box = arg("--box", os.environ.get("FERROPLAN_BOX", "m5-air"))
    measured_at = arg("--measured-at")
    if not measured_at:
        sys.exit("--measured-at is required (YYYY-MM-DD) — the date the BOARDS "
                 "were swept, which is not necessarily the release date")
    released = arg("--released", measured_at)
    note = arg("--note", "")

    # --from-dir reads a sweep that was never promoted — a BACKFILL of an old
    # tag, which must be banked without disturbing the live boards.
    src = arg("--from-dir")
    tracks = {}
    for fname, (label, comp, budget) in S.SWEEPS.items():
        if src:
            board = "ipc67-results" if fname == "ipc67-default.jsonl" else fname[:-6]
            p = os.path.join(src, f"{board}.jsonl")
            if not os.path.exists(p):
                continue
            rows = S.load_jsonl(p)
            s_, n_, _ = S.coverage_line(rows, budget)
            if n_:
                tracks[label] = {"solved": s_, "total": n_}
            continue
        p = os.path.join(S.B, fname)
        md = os.path.join(S.B, {"ipc67-default.jsonl": "ipc67-results.md",
                                "ipc67-temporal.jsonl": "ipc67-temporal.md"}
                          .get(fname, fname.replace(".jsonl", ".md")))
        if not (os.path.exists(p) and os.path.exists(md)):
            continue
        rows = S.load_jsonl(p)
        s, n, _ = S.coverage_line(rows, budget)
        if n:
            tracks[label] = {"solved": s, "total": n}
    if not tracks:
        sys.exit("no promoted boards found — promote before snapshotting")

    doc = {"_comment": (
        "Per-release standings snapshots. `measured_on` is the BOX: coverage at a "
        "fixed time budget depends on hardware, so STANDINGS.md only ever compares "
        "snapshots sharing a box. `measured_at` is when the boards were SWEPT, "
        "which differs from `released` for backfilled runs of old versions on new "
        "hardware. Written by scripts/standings-snapshot.py."),
        "snapshots": []}
    if os.path.exists(HISTORY):
        doc = json.load(open(HISTORY))
        doc.setdefault("snapshots", [])

    snap = {"version": version, "released": released, "measured_on": box,
            "measured_at": measured_at, "note": note, "tracks": tracks}
    doc["snapshots"] = [s for s in doc["snapshots"]
                        if not (s["version"] == version
                                and s.get("measured_on") == box)]
    doc["snapshots"].append(snap)
    doc["snapshots"].sort(key=lambda s: (s["measured_at"], s["version"]))

    with open(HISTORY, "w") as f:
        json.dump(doc, f, indent=1, sort_keys=False)
        f.write("\n")
    tot_s = sum(t["solved"] for t in tracks.values())
    tot_n = sum(t["total"] for t in tracks.values())
    print(f"snapshotted {version} on {box} ({measured_at}): "
          f"{len(tracks)} tracks, {tot_s}/{tot_n}")
    print(f"wrote {HISTORY} ({len(doc['snapshots'])} snapshots)")


if __name__ == "__main__":
    main()
