import sys,collections; sys.path.insert(0,'/private/tmp/claude-501/-Users-harold-ferroplan/493e019b-1c41-419a-9f7b-f0f4bd1cdbf1/scratchpad')
from join import rows
for board in ['ipc5-qual-pref','ipc5-complex-pref','ipc5-simple-pref','ipc5-metric-time','ipc5-time','ipc5-constraints','ipc5-prop']:
    print(f"\n=== {board}: rows SGPlan5 solved and we did not")
    c=collections.Counter(); ex={}
    for r in rows(board):
        if r['solved'] or not r['sg'] or r['sg']['n']==0: continue
        keys=[k for k in r.keys() if k not in('ipc','variant','instance','solved','budget','ver','mode','jobs','threads','start_ts','end_ts','engine','sg','metric','length','val','makespan')]
        note='; '.join(r.get('notes') or []) or '-'
        k=(r['variant'],note[:90], r.get('reason') or r.get('status') or r.get('rc'))
        c[k]+=1; ex.setdefault(k,[]).append((r['instance'],r.get('time'),r['sg']['t']))
    for (v,note,why),n in sorted(c.items()):
        e=ex[(v,note,why)]; ts=[x[1] for x in e if x[1] is not None]
        print(f"  {n:3d} {v:42s} why={why} our_t=[{min(ts) if ts else None}..{max(ts) if ts else None}] sg_t_max={max(x[2] or 0 for x in e):.1f}  note={note}")
