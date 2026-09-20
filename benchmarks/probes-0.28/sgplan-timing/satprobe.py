"""Probe: what does `ff --satisfice` (hard goals only, no metric optimisation) convert
on the preference-board rows the 60 s board records as UNSOLVED?  threads 1, 60 s wall,
2-wide like the board. VAL-checked. A MEASUREMENT: changes no board."""
import json, os, subprocess, sys, time, tempfile, threading, re
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from join import rows
FF='/Users/harold/ferroplan-0.28/target/release/ff'
VAL='/Users/harold/ferroplan/benchmarks/.val/VAL/build/bin/Validate'
C='/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2006/domains'
WALL=int(os.environ.get('WALL','60')); OUT=sys.argv[1]; boards=sys.argv[2:]
EXTRA=os.environ.get('FFARGS','--satisfice').split()
def val(d,p,steps,temporal):
    with tempfile.NamedTemporaryFile('w',suffix='.plan',delete=False) as f:
        for s in steps:
            act='('+' '.join([s['action']]+s.get('args',[])).lower()+')'
            if temporal:
                line=f"{s['time']}: {act}"
                if s.get('duration') is not None: line+=f" [{s['duration']}]"
                f.write(line+'\n')
            else: f.write(act+'\n')
        path=f.name
    try:
        r=subprocess.run(([VAL,'-t','0.0005'] if temporal else [VAL])+[d,p,path],capture_output=True,text=True,timeout=120)
        m=re.search(r'Final value:\s*([-\d.eE+]+)',r.stdout)
        return (r.returncode==0 and 'Plan valid' in r.stdout), (float(m.group(1)) if m else None)
    except Exception as e: return None,None
    finally: os.unlink(path)
def one(r):
    d=f"{C}/{r['variant']}/domain.pddl"; p=f"{C}/{r['variant']}/instances/instance-{r['instance']}.pddl"
    if not os.path.exists(d):
        dd=f"{C}/{r['variant']}/domains/domain-{r['instance']}.pddl"
        if os.path.exists(dd): d=dd
    env=dict(os.environ,FF_TIME_LIMIT=str(WALL),FF_MEM_BUDGET_GB='8')
    t=time.time(); rec=dict(variant=r['variant'],instance=r['instance'],sg_t=r['sg'] and r['sg']['t'],sg_m=r['sg'] and r['sg']['m'],board_note=r.get('notes'))
    try:
        cp=subprocess.run(['perl','-e','alarm shift; exec @ARGV',str(WALL+10),FF,'-o',d,'-f',p,'--json','--threads','1']+EXTRA,capture_output=True,text=True,env=env)
        rec['time']=round(time.time()-t,2); rec['rc']=cp.returncode
        try: j=json.loads(cp.stdout)
        except Exception: j={}
        rec['solved']=bool(j.get('solved')); rec['mode']=j.get('mode'); rec['notes']=j.get('notes')
        rec['evals']=(j.get('statistics') or {}).get('evaluated_states')
        if rec['solved']:
            steps=j['plan']['steps']; temporal=any('time' in s for s in steps)
            rec['length']=len(steps); rec['val'],rec['val_metric']=val(d,p,steps,temporal)
        elif not j: rec['err']=cp.stderr[-300:]
    except Exception as e: rec['solved']=False; rec['err']=repr(e)
    return rec
todo=[r for b in boards for r in rows(b) if not r['solved']]
print(len(todo),'unsolved rows to probe',flush=True)
with open(OUT,'w') as f, ThreadPoolExecutor(2) as ex:
    for rec in ex.map(one,todo):
        f.write(json.dumps(rec)+'\n'); f.flush()
        print(rec['variant'],rec['instance'],'SOLVED' if rec['solved'] else 'no',rec.get('time'),'val',rec.get('val'),'m',rec.get('val_metric'),'sg',rec['sg_t'],rec['sg_m'],rec.get('notes') or rec.get('err','')[:120],flush=True)
