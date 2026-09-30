#!/usr/bin/env python3
"""Phase 0.3's fork, read off rows.jsonl: per cell, per arm, solved reps of 3."""
import json, collections
rows = [json.loads(l) for l in open('rows.jsonl') if l.startswith('{')]
arms = []
for r in rows:
    k = (r['arm'], r['hatch'])
    if k not in arms: arms.append(k)
cells = []
for r in rows:
    if r['cell'] not in cells: cells.append(r['cell'])
tab = collections.defaultdict(lambda: collections.defaultdict(list))
for r in rows: tab[r['cell']][(r['arm'], r['hatch'])].append((r['solved'], r['wall']))
print(f"{'cell':46s}" + ''.join(f"{(a if not h else h.split('=')[0].replace('FF_','')):>16s}" for a, h in arms))
for c in cells:
    line = f"{c[:46]:46s}"
    for k in arms:
        reps = tab[c].get(k, [])
        n = sum(1 for s, _ in reps if s is True)
        w = [w for s, w in reps if s is True]
        line += f"{(str(n)+'/'+str(len(reps))+(' @'+format(sum(w)/len(w),'.0f')+'s' if w else '')):>16s}"
    print(line)
print()
for c in cells:
    base = tab[c].get(('0.28.0', ''), []); old = tab[c].get(('0.27.1', ''), [])
    nb = sum(1 for s, _ in base if s is True); no = sum(1 for s, _ in old if s is True)
    restored = [h for (a, h) in arms if a == '0.28.0' and h and sum(1 for s, _ in tab[c].get((a, h), []) if s is True) > nb]
    verdict = ('NOT A LOSS: 0.28.0 solves it solo' if nb >= 2 else
               'BOX: 0.27.1 fails it here too' if no == 0 else
               f'restored by {", ".join(restored)}' if restored else 'REAL: no hatch restores it')
    print(f"  {c[:44]:44s} 0.28.0 {nb}/3  0.27.1 {no}/3  -> {verdict}")
