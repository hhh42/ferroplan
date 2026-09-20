"""PROBE ONLY. SGPlan5-style action compression: every durative action becomes ONE
instantaneous action (cond = start ∧ over-all ∧ end, effect = start then end, end wins),
the classical engine plans it, and the plan is laid out SEQUENTIALLY (no overlap) with
durations evaluated by replaying the numeric fluents. VAL judges the result against the
ORIGINAL temporal domain/problem. Sound for a sequential layout; makespan is poor by
construction (no scheduling pass) — this prices COVERAGE only."""
import re, sys
def parse(text):
    text = re.sub(r';[^\n]*', '', text)
    toks = re.findall(r'\(|\)|[^\s()]+', text)
    pos = 0
    def rd():
        nonlocal pos
        t = toks[pos]; pos += 1
        if t == '(':
            l = []
            while toks[pos] != ')': l.append(rd())
            pos += 1
            return l
        return t.lower()
    return rd()
def dump(s):
    return s if isinstance(s, str) else '(' + ' '.join(dump(x) for x in s) + ')'
def conj(f):
    if isinstance(f, list) and f and f[0] == 'and': return [y for x in f[1:] for y in conj(x)]
    return [] if f == [] else [f]

def drop_prefs(f):
    """remove every (preference ...) subformula; None if f itself is (or wraps only) a preference"""
    if not isinstance(f, list) or not f: return f
    if f[0] == 'preference': return None
    if f[0] == 'and':
        kids = [k for k in (drop_prefs(x) for x in f[1:]) if k is not None]
        return ['and'] + kids
    if f[0] == 'forall' and len(f) == 3:
        k = drop_prefs(f[2])
        return None if k is None or k == ['and'] else ['forall', f[1], k]
    return f
TIMED = {('at', 'start'): 'start', ('over', 'all'): 'all', ('at', 'end'): 'end'}
def when_of(f):
    """first time specifier found inside f (None if untimed)"""
    if isinstance(f, list):
        if len(f) == 3 and isinstance(f[0], str) and isinstance(f[1], str) and (f[0], f[1]) in TIMED:
            return TIMED[(f[0], f[1])]
        for x in f:
            w = when_of(x)
            if w: return w
    return None
def strip(f):
    if isinstance(f, list):
        if len(f) == 3 and isinstance(f[0], str) and isinstance(f[1], str) and (f[0], f[1]) in TIMED:
            return strip(f[2])
        return [strip(x) for x in f]
    return f
NUM = ('increase', 'decrease', 'assign', 'scale-up', 'scale-down')
def atom(e):  # literal -> (atom, positive)
    if isinstance(e, list) and e and e[0] == 'not': return dump(e[1]), False
    return dump(e), True
def compress_action(da):
    d = {da[i]: da[i + 1] for i in range(2, len(da) - 1, 2)}
    name = da[1]
    conds = [c for c in conj(d.get(':condition', [])) if drop_prefs(strip(c)) is not None and drop_prefs(c) is not None]
    effs = conj(d.get(':effect', []))
    se = [strip(e) for e in effs if when_of(e) in ('start', None)]
    ee = [strip(e) for e in effs if when_of(e) == 'end']
    simple = lambda e: isinstance(e, list) and e and e[0] not in NUM + ('forall', 'when')
    start_adds = {atom(e)[0] for e in se if simple(e) and atom(e)[1]}
    start_dels = {atom(e)[0] for e in se if simple(e) and not atom(e)[1]}
    pre = []
    for c in conds:
        w, x = when_of(c), strip(c)
        if w in ('all', 'end') and isinstance(x, list) and x and x[0] != 'not' and dump(x) in start_adds:
            continue  # self-established: not required beforehand
        if w in ('all', 'end') and isinstance(x, list) and x and x[0] == 'not' and dump(x[1]) in start_dels:
            continue
        pre.append(x)
    end_atoms = {atom(e)[0] for e in ee if simple(e)}
    se = [e for e in se if not (simple(e) and atom(e)[0] in end_atoms)]  # end overrides start
    # merge numeric effects that hit the same fluent twice (start decrease / end increase)
    allE, byf = se + ee, {}
    for e in allE:
        if isinstance(e, list) and e and e[0] in ('increase', 'decrease'):
            byf.setdefault(dump(e[1]), []).append(e)
    out = []
    done = set()
    for e in allE:
        if isinstance(e, list) and e and e[0] in ('increase', 'decrease') and len(byf[dump(e[1])]) > 1:
            k = dump(e[1])
            if k in done: continue
            done.add(k)
            inc = [x[2] for x in byf[k] if x[0] == 'increase']; dec = [x[2] for x in byf[k] if x[0] == 'decrease']
            for a in list(inc):
                if a in dec: inc.remove(a); dec.remove(a)  # exact cancel
            if not inc and not dec: continue
            total = ['+'] + inc if len(inc) > 1 else (inc[0] if inc else '0')
            for x in dec: total = ['-', total, x]
            out.append(['increase', e[1], total])
        else:
            out.append(e)
    return [':action', name, ':parameters', d.get(':parameters', []),
            ':precondition', ['and'] + pre, ':effect', ['and'] + out], d
def compress_domain(dom):
    out, das = [], {}
    for x in dom:
        if isinstance(x, list) and x and x[0] == ':durative-action':
            a, d = compress_action(x); out.append(a); das[x[1]] = d
        elif isinstance(x, list) and x and x[0] == ':requirements':
            out.append([r for r in x if r != ':durative-actions'])
        elif isinstance(x, list) and x and x[0] == ':constraints':
            k = drop_prefs(x[1])
            if k is not None and k != ['and']: out.append([':constraints', k])
        else: out.append(x)
    return out, das
def strip_metric(prob):
    out = []
    for x in prob:
        if isinstance(x, list) and x and x[0] == ':metric': continue
        if isinstance(x, list) and x and x[0] in (':goal', ':constraints'):
            k = drop_prefs(x[1])
            if k is None or k == ['and']:
                if x[0] == ':constraints': continue
                k = ['and']
            out.append([x[0], k]); continue
        out.append(x)
    return out
# ---- numeric replay, to price each step's duration ------------------------------------
def init_fluents(prob):
    fl = {}
    for x in prob:
        if isinstance(x, list) and x and x[0] == ':init':
            for f in x[1:]:
                if isinstance(f, list) and f[0] == '=' and isinstance(f[1], list):
                    fl[dump(f[1])] = float(f[2])
    return fl
def ev(e, b, fl):
    if isinstance(e, str):
        if e in b: return b[e]
        return float(e)
    if e[0] in ('+', '-', '*', '/'):
        v = [ev(x, b, fl) for x in e[1:]]
        if e[0] == '-' and len(v) == 1: return -v[0]
        r = v[0]
        for y in v[1:]:
            r = r + y if e[0] == '+' else r - y if e[0] == '-' else r * y if e[0] == '*' else r / y
        return r
    return fl.get(dump([b.get(t, t) if isinstance(t, str) else t for t in e]), 0.0)
def params(plist):
    names = []
    i = 0
    while i < len(plist):
        if plist[i] == '-': i += 2; continue
        names.append(plist[i]); i += 1
    return names
def apply_num(effs, which, b, fl):
    for e in effs:
        if when_of(e) != which: continue
        x = strip(e)
        if isinstance(x, list) and x and x[0] in NUM:
            k = dump([b.get(t, t) for t in x[1]]); v = ev(x[2], b, fl); cur = fl.get(k, 0.0)
            fl[k] = cur + v if x[0] == 'increase' else cur - v if x[0] == 'decrease' else v if x[0] == 'assign' else cur * v if x[0] == 'scale-up' else cur / v
def schedule(steps, das, fl, gap=0.002):
    """steps: [(name,[args])] -> [(t, name, args, dur)] laid end to end"""
    t, out = gap, []
    for name, args in steps:
        d = das[name]; b = dict(zip(params(d.get(':parameters', [])), args))
        dur = ev(d[':duration'][2], b, fl)
        apply_num(conj(d.get(':effect', [])), 'start', b, fl)
        apply_num(conj(d.get(':effect', [])), 'end', b, fl)
        out.append((t, name, args, dur)); t = round(t + dur + gap, 6)
    return out
if __name__ == '__main__':
    dom = parse(open(sys.argv[1]).read()); cd, das = compress_domain(dom)
    print(dump(cd).replace('(:', '\n(:'))
