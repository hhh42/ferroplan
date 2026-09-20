"""Probe: compress durative actions -> classical, plan with ff (threads 1, 60 s wall),
lay the plan out sequentially, VAL it against the ORIGINAL temporal task. Unsolved board
rows only. A MEASUREMENT: changes no board."""
import json, os, subprocess, sys, time, tempfile, re
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from join import rows
import compress as cz
FF='/Users/harold/ferroplan-0.28/target/release/ff'
VAL='/Users/harold/ferroplan/benchmarks/.val/VAL/build/bin/Validate'
C='/Users/harold/ferroplan/benchmarks/.ipc-corpus/ipc-2006/domains'
WALL=int(os.environ.get('WALL','60')); OUT=sys.argv[1]; boards=sys.argv[2:]
TMP=os.path.join(os.path.dirname(os.path.abspath(__file__)),'tprobe-tmp'); os.makedirs(TMP,exist_ok=True)
def one(r):
    v,i=r['variant'],r['instance']
    d=f"{C}/{v}/domain.pddl"; p=f"{C}/{v}/instances/instance-{i}.pddl"
    if not os.path.exists(d): d=f"{C}/{v}/domains/domain-{i}.pddl"
    rec=dict(variant=v,instance=i,sg_t=r['sg'] and r['sg']['t'],board_note=r.get('notes'))
    try:
        dom=cz.parse(open(d).read()); prob=cz.parse(open(p).read())
        cd,das=cz.compress_domain(dom); cp_=cz.strip_metric(prob)
        cdp=f"{TMP}/{v}-{i}-domain.pddl"; cpp=f"{TMP}/{v}-{i}-problem.pddl"
        open(cdp,'w').write(cz.dump(cd)); open(cpp,'w').write(cz.dump(cp_))
        env=dict(os.environ,FF_TIME_LIMIT=str(WALL),FF_MEM_BUDGET_GB='8')
        t=time.time()
        cp=subprocess.run(['perl','-e','alarm shift; exec @ARGV',str(WALL+10),FF,'-o',cdp,'-f',cpp,'--json','--threads','1'],capture_output=True,text=True,env=env)
        rec['time']=round(time.time()-t,2); rec['rc']=cp.returncode
        try: j=json.loads(cp.stdout)
        except Exception: j={}
        rec['solved_classical']=bool(j.get('solved')); rec['mode']=j.get('mode'); rec['notes']=j.get('notes')
        rec['evals']=(j.get('statistics') or {}).get('evaluated_states')
        if not j: rec['err']=(cp.stderr or cp.stdout)[-300:]
        rec['val']=None
        if rec['solved_classical']:
            steps=[(s['action'].lower(),[a.lower() for a in s.get('args',[])]) for s in j['plan']['steps']]
            rec['length']=len(steps)
            sched=cz.schedule(steps,das,cz.init_fluents(prob))
            rec['makespan']=round(sched[-1][0]+sched[-1][3],3) if sched else 0
            pf=f"{TMP}/{v}-{i}.plan"
            with open(pf,'w') as f:
                for (t0,n,a,dur) in sched: f.write(f"{t0:.4f}: ({' '.join([n]+a)}) [{dur:.4f}]\n")
            vr=subprocess.run([VAL,'-t','0.0005',d,p,pf],capture_output=True,text=True,timeout=300)
            rec['val']=(vr.returncode==0 and 'Plan valid' in vr.stdout)
            m=re.search(r'Final value:\s*([-\d.eE+]+)',vr.stdout); rec['val_metric']=float(m.group(1)) if m else None
            if not rec['val']: rec['val_out']=(vr.stdout+vr.stderr)[-400:]
    except Exception as e:
        rec['err']=repr(e)[:300]
    rec['solved']=bool(rec.get('val'))
    return rec
todo=[r for b in boards for r in rows(b) if not r['solved']]
if os.environ.get('ONLY'): todo=[r for r in todo if re.search(os.environ['ONLY'],f"{r['variant']}:{r['instance']}")]
print(len(todo),'unsolved rows to probe',flush=True)
with open(OUT,'w') as f, ThreadPoolExecutor(2) as ex:
    for rec in ex.map(one,todo):
        f.write(json.dumps(rec)+'\n'); f.flush()
        print(rec['variant'],rec['instance'],'classical' ,rec.get('solved_classical'),rec.get('time'),'VAL',rec.get('val'),'len',rec.get('length'),'mk',rec.get('makespan'),'sg_t',rec['sg_t'],(rec.get('err') or rec.get('val_out') or '')[-200:].replace('\n',' | '),flush=True)
