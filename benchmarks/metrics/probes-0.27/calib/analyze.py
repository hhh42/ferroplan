#!/usr/bin/env python3
"""Turn results.jsonl into the Phase 0 tables (docs/roadmap-0.27.md 0.b/0.c),
evaluating the pre-registered thresholds exactly as written there."""
import json, statistics, sys, os, collections
HERE = os.path.dirname(os.path.abspath(__file__))
rows = [json.loads(l) for l in open(sys.argv[1] if len(sys.argv) > 1 else f"{HERE}/results.jsonl")]

def pct(xs, p):
    xs = sorted(xs)
    if not xs: return float("nan")
    k = (len(xs) - 1) * p
    lo, hi = int(k), min(int(k) + 1, len(xs) - 1)
    return xs[lo] + (xs[hi] - xs[lo]) * (k - lo)

out = []
P = out.append

# ---- A: rho_min ----
P("## 0.b — ρ on 60 clean-window instances, solo (the corrected instrument)\n")
P("| bucket | n | median ρ | p10 | p5 | min | solved | load (median) |")
P("|---|---:|---:|---:|---:|---:|---:|---:|")
ge10 = []
for b in ("1-10s", "10-50s", "ge50s"):
    rs = [r for r in rows if r["tag"] == f"A:{b}" and r["rho"] is not None]
    if not rs: continue
    rh = [r["rho"] for r in rs]
    if b != "1-10s": ge10 += rh
    P(f"| {b} | {len(rs)} | {statistics.median(rh):.3f} | {pct(rh,0.10):.3f} | {pct(rh,0.05):.3f} | {min(rh):.3f} | {sum(r['solved'] for r in rs)} | {statistics.median(r['load'] for r in rs):.2f} |")
if ge10:
    p5 = pct(ge10, 0.05)
    import math
    rho_min = max(0.85, math.floor(p5 * 20) / 20)
    P(f"\n**ρ_min (pre-registered rule: p5 of the ≥10 s buckets, rounded down to 0.05, floored at 0.85):** p5 = {p5:.3f} → **ρ_min = {rho_min:.2f}**"
      + ("" if p5 >= 0.85 else "  — p5 is BELOW the 0.85 floor: the referee is not safe on this box as measured; say so."))
    low = [r for r in rows if r["tag"].startswith("A:") and r["rho"] is not None and r["rho"] < 0.85]
    if low:
        P(f"\nRows under 0.85 ({len(low)}): " + "; ".join(f"{r['variant']}/{r['label']} ρ {r['rho']} load {r['load']}" for r in low))

# ---- B: packing ----
P("\n## 0.c — packing calibration: 40 PACK-class instances, solo ×3 vs 4-wide ×3\n")
key = lambda r: (r["board"], r["variant"], r["label"])
solo = collections.defaultdict(list); pack = collections.defaultdict(list)
prho = collections.defaultdict(list); loads = []
for r in rows:
    if r["tag"].startswith("B:solo"): solo[key(r)].append(r["wall"])
    elif r["tag"].startswith("B:pack4"): pack[key(r)].append(r["wall"]); prho[key(r)].append(r["rho"]); loads.append(r["load"])
infl = []; lost = []
for k in solo:
    if k in pack and solo[k] and pack[k]:
        s, p = statistics.median(solo[k]), statistics.median(pack[k])
        infl.append((p / s, k, s, p))
if infl:
    xs = [i for i, *_ in infl]
    med, p95 = statistics.median(xs), pct(xs, 0.95)
    P(f"| instances | median inflation | p95 | max | packed ρ (median) | load during packed (median) |")
    P("|---:|---:|---:|---:|---:|---:|")
    P(f"| {len(infl)} | {med:.3f} | {p95:.3f} | {max(xs):.3f} | {statistics.median(x for v in prho.values() for x in v if x is not None):.3f} | {statistics.median(loads):.2f} |")
    verdict = ("**`pack_width = 4` ships**" if med <= 1.05 and p95 <= 1.15 else
               "**`pack_width = 2` ships**" if med <= 1.15 and p95 <= 1.30 else
               "**packing is a RECORDED NEGATIVE; the referee ships alone**")
    P(f"\nPre-registered: ≤5 %/≤15 % → 4-wide; ≤15 %/≤30 % → 2-wide; else negative. Measured median {100*(med-1):+.1f} %, p95 {100*(p95-1):+.1f} % → {verdict}")
    worst = sorted(infl, reverse=True)[:5]
    P("\nWorst five: " + "; ".join(f"{k[1]}/{k[2]} {s:.1f}s→{p:.1f}s ({i:.2f}×)" for i, k, s, p in worst))
    # solves lost under packing (any packed rep unsolved where solo solved)
    solved_solo = {key(r) for r in rows if r["tag"].startswith("B:solo") and r["solved"]}
    unsolved_pack = {key(r) for r in rows if r["tag"].startswith("B:pack4") and not r["solved"]}
    lost = solved_solo & unsolved_pack
    P(f"\nPacked runs that failed where solo solved: {len(lost)}" + (" — " + ", ".join(f"{k[1]}/{k[2]}" for k in sorted(lost)) if lost else " (none; by construction such a miss is re-queued solo, so this is wasted time, never a lost row)"))

# ---- B2: 2-wide ----
pack2 = collections.defaultdict(list)
for r in rows:
    if r["tag"].startswith("B:pack2"): pack2[key(r)].append(r["wall"])
infl2 = [(statistics.median(pack2[k]) / statistics.median(solo[k]), k) for k in solo if k in pack2 and solo[k] and pack2[k]]
if infl2:
    xs = [i for i, _ in infl2]
    med, p95 = statistics.median(xs), pct(xs, 0.95)
    P(f"\n**2-wide (the Python sweeps' `jobs = 2`), {len(infl2)} instances: median inflation {100*(med-1):+.1f} %, p95 {100*(p95-1):+.1f} %, max {100*(max(xs)-1):+.1f} %** → " + ("`pack_width = 2` would ship by the ≤15/≤30 rule" if med <= 1.15 and p95 <= 1.30 else "2-wide is a recorded negative too"))

# ---- C: timeouts ----
P("\n## 0.c — prior timeouts: 20 instances, solo vs 2-wide\n")
cs = {key(r): r for r in rows if r["tag"] == "C:solo"}
cp = {key(r): r for r in rows if r["tag"] == "C:pack2"}
if cs and cp:
    both = [k for k in cs if k in cp]
    rho_ok = sum(1 for k in both if (cp[k]["rho"] or 0) >= (rho_min if ge10 else 0.9))
    solo_solves = [k for k in both if cs[k]["solved"]]
    packed_missed = [k for k in solo_solves if not cp[k]["solved"]]
    P("| pairs | packed ρ ≥ ρ_min | solo solved | packed solved | solo-solves-what-packed-missed | median packed ρ | median solo ρ |")
    P("|---:|---:|---:|---:|---:|---:|---:|")
    P(f"| {len(both)} | {rho_ok} ({100*rho_ok/len(both):.0f} %) | {len(solo_solves)} | {sum(1 for k in both if cp[k]['solved'])} | {len(packed_missed)} | {statistics.median(cp[k]['rho'] for k in both):.3f} | {statistics.median(cs[k]['rho'] for k in both):.3f} |")
    admit = rho_ok / len(both) >= 0.95 and not packed_missed
    P(f"\nPre-registered: ρ ≥ ρ_min on ≥95 % of packed timeouts AND no solo-solves-what-packed-missed → prior timeouts admitted at width 2. → **{'ADMITTED at width 2' if admit else 'stay SOLO'}**")
    if packed_missed:
        P("Missed under packing: " + ", ".join(f"{k[1]}/{k[2]}" for k in packed_missed))

print("\n".join(out))
