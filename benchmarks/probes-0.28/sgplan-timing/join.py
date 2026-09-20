import json, os, re, glob, sys, collections
S=os.path.dirname(os.path.abspath(__file__))  # expects IPC5-results.tgz unpacked at ./ipc5/RESULTS
R=S+'/ipc5/RESULTS/sgplan'
B='/Users/harold/ferroplan/benchmarks'
TRACK={'preferences-qualitative':'QualitativePreferences','preferences-complex':'ComplexPreferences',
       'preferences-simple':'SimplePreferences','metric-preferences-simple':'MetricSimplePreferences',
       'metric-time-constraints':'MetricTimeConstraints','time-constraints':'TimeConstraints',
       'metric-time':'MetricTime','time':'Time','propositional':'Propositional','propositional-strips':'Propositional/Strips'}
DOM={'tpp':'TPP'}
def sg(dom,track,i):
    p=f"{R}/{DOM.get(dom,dom)}/{track}/p{i:02d}.soln"
    if not os.path.exists(p): return None
    t=m=None; n=0
    for l in open(p,errors='replace'):
        if l.startswith('; Time'):
            try:t=float(l.split()[2])
            except:pass
        elif l.startswith('; MetricValue'):
            try:m=float(l.split()[2])
            except:pass
        elif re.match(r'^\s*[\d.]+\s*:\s*\(',l): n+=1
    return dict(t=t,m=m,n=n) if n>0 or t is not None else None
def rows(board):
    out=[]
    for l in open(f"{B}/{board}.jsonl"):
        r=json.loads(l); v=r['variant']; dom,rest=v.split('-',1)
        if rest not in TRACK: continue
        r['sg']=sg(dom,TRACK[rest],r['instance']); out.append(r)
    return out
if __name__=='__main__':
    for board in ['ipc5-qual-pref','ipc5-complex-pref','ipc5-simple-pref','ipc5-metric-time','ipc5-time','ipc5-constraints','ipc5-prop']:
        rs=rows(board)
        print(f"\n=== {board}")
        byv=collections.OrderedDict()
        for r in rs: byv.setdefault(r['variant'],[]).append(r)
        print(f"{'variant':48s} {'n':>3} {'us':>3} {'sg':>3} | sgplan time: {'<1s':>4} {'<10s':>4} {'<60s':>4} {'<600':>4} {'>600':>4} {'med':>7} {'max':>7} | sg-only: n  sg_t<60")
        for v,l in byv.items():
            us=sum(r['solved'] for r in l); sgs=[r for r in l if r['sg'] and r['sg']['n']>0]
            ts=sorted(r['sg']['t'] for r in sgs if r['sg']['t'] is not None)
            b=lambda lo,hi: sum(lo<=t<hi for t in ts)
            med=ts[len(ts)//2] if ts else float('nan'); mx=ts[-1] if ts else float('nan')
            only=[r for r in sgs if not r['solved']]; o60=sum(1 for r in only if r['sg']['t'] is not None and r['sg']['t']<60)
            print(f"{v:48s} {len(l):3d} {us:3d} {len(sgs):3d} |              {b(0,1):4d} {b(1,10):4d} {b(10,60):4d} {b(60,600):4d} {b(600,1e9):4d} {med:7.2f} {mx:7.1f} |         {len(only):3d} {o60:3d}")
