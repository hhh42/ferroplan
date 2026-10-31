#!/usr/bin/env python3
"""The instance-feature rung predictor probe (0.31, Lane A' question 2).

The matrix (Phase 0.5) says the banked ladder solves 662 of 936 cells against
an oracle of 706, and that a single rung with the whole wall, chosen per
domain NAME and judged leave-one-out, reaches 678. A name is memorising the
benchmark. This probe asks whether features read off the PDDL TEXT -- no
grounding, no search, milliseconds a cell -- carry that signal:

  * resolve each matrix cell to its domain/problem files the way the crucible
    corpus walker does (digit groups of the file name are the label);
  * extract ~40 features: requirements, type/predicate/action counts and
    arities, precondition/effect sizes, quantifiers and conditional effects,
    object/init/goal sizes, a log grounding estimate;
  * for every standing rung fit "does this rung solve the cell" (a random
    forest), choose the rung with the highest predicted probability, run it
    with the whole wall, count the cells solved;
  * judge it two ways: leave-one-DOMAIN-out (the honest number: a domain the
    predictor has never seen) and 10-fold over cells (the boards' number:
    known domains, unseen instances).

    python3 benchmarks/rung-predictor.py --engine 6facbec9163c --baseline b02061eded16

Reads only: the database is opened immutably, the corpus is read, nothing
is run. Fits ~250 small forests single-threaded (about a minute).
"""

import argparse
import collections
import importlib.util
import math
import os
import re
import sys

import numpy as np
from sklearn.ensemble import RandomForestClassifier
from sklearn.model_selection import GroupKFold, KFold

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("matrix_read", os.path.join(HERE, "matrix-read.py"))
mr = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mr)

CORPUS = os.environ.get("FERROPLAN_IPC_CORPUS") or os.path.join(HERE, ".ipc-corpus")

# ----------------------------------------------------------------------------
# corpus resolution (mirrors crucible-core/src/corpus.rs)
# ----------------------------------------------------------------------------


def digit_groups(name):
    return re.findall(r"\d+", name)


def label_of(file):
    g = digit_groups(file)
    if not g:
        return None
    return str(int(g[0])) if len(g) == 1 else "_".join(g)


def variant_ipc(db, engine):
    q = """
    SELECT DISTINCT v.name, v.ipc
      FROM run r JOIN board b ON b.id = r.board_id
      JOIN instance i ON i.id = r.instance_id
      JOIN variant v ON v.id = i.variant_id
      JOIN engine e ON e.id = r.engine_id
     WHERE e.blake3 LIKE ? AND b.env LIKE '%FF_RUNG_ONLY%'
    """
    return dict(db.execute(q, (engine + "%",)))


def resolve(ipc, variant, label):
    vdir = os.path.join(CORPUS, ipc, "domains", variant)
    idir = os.path.join(vdir, "instances")
    for f in os.listdir(idir):
        if label_of(f) == label:
            shared = os.path.join(vdir, "domain.pddl")
            dom = shared if os.path.isfile(shared) else os.path.join(vdir, "domains", f"domain-{digit_groups(f)[0]}.pddl")
            return dom, os.path.join(idir, f)
    raise FileNotFoundError(f"{ipc}/{variant}/{label}")


# ----------------------------------------------------------------------------
# a small PDDL reader
# ----------------------------------------------------------------------------

TOKEN = re.compile(r"\(|\)|[^\s()]+")


def parse(path):
    with open(path, encoding="utf-8", errors="replace") as fh:
        text = fh.read()
    text = re.sub(r";[^\n]*", "", text).lower()
    stack = [[]]
    for t in TOKEN.findall(text):
        if t == "(":
            stack.append([])
        elif t == ")":
            l = stack.pop()
            stack[-1].append(l)
        else:
            stack[-1].append(t)
    return stack[0][0] if stack[0] else []


def sections(form):
    """(define (domain x) (:requirements ..) ..) -> {":requirements": [..], ...}; repeated keys collect."""
    out = collections.defaultdict(list)
    for s in form[2:]:
        if isinstance(s, list) and s and isinstance(s[0], str) and s[0].startswith(":"):
            out[s[0]].append(s)
    return out


def typed_list(items):
    """['a', 'b', '-', 't', 'c'] -> [('a','t'),('b','t'),('c','object')]."""
    out, pending = [], []
    i = 0
    while i < len(items):
        x = items[i]
        if x == "-":
            ty = items[i + 1]
            ty = "object" if isinstance(ty, list) else ty  # (either ..) -> object
            out += [(p, ty) for p in pending]
            pending = []
            i += 2
        else:
            pending.append(x)
            i += 1
    out += [(p, "object") for p in pending]
    return out


CONNECTIVES = {"and", "or", "not", "imply", "forall", "exists", "when", "at", "over", "always", "sometime",
               "within", "at-most-once", "sometime-after", "sometime-before", "always-within",
               "hold-during", "hold-after", "preference"}
NUMERIC = {"<", ">", "<=", ">=", "=", "increase", "decrease", "assign", "scale-up", "scale-down", "+", "-", "*", "/"}


def walk(x, acc):
    """Count atoms / numeric atoms / connectives in a formula."""
    if not isinstance(x, list) or not x:
        return
    h = x[0]
    if isinstance(h, list):
        for y in x:
            walk(y, acc)
        return
    if h in CONNECTIVES:
        acc[h] += 1
        for y in x[1:]:
            walk(y, acc)
    elif h in NUMERIC:
        acc["numeric"] += 1
        for y in x[1:]:
            walk(y, acc)
    else:
        acc["atom"] += 1


def kv(lst):
    """[':parameters', [...], ':precondition', [...]] -> dict."""
    d = {}
    for i, x in enumerate(lst):
        if isinstance(x, str) and x.startswith(":") and i + 1 < len(lst):
            d[x] = lst[i + 1]
    return d


def features(dom_path, prob_path):
    f = collections.OrderedDict()
    dom = sections(parse(dom_path))
    prob = sections(parse(prob_path))

    reqs = set(r.lstrip(":") for s in dom[":requirements"] for r in s[1:] if isinstance(r, str))
    for r in ["typing", "adl", "negative-preconditions", "disjunctive-preconditions", "equality",
              "existential-preconditions", "universal-preconditions", "quantified-preconditions",
              "conditional-effects", "derived-predicates", "action-costs", "numeric-fluents", "fluents",
              "durative-actions", "preferences", "constraints"]:
        f["req_" + r] = int(r in reqs)

    # types and their closure
    parent = {}
    for s in dom[":types"]:
        for t, p in typed_list(s[1:]):
            parent[t] = p
    f["n_types"] = len(parent)

    def ancestors(t):
        seen = []
        while t is not None and t not in seen:
            seen.append(t)
            t = parent.get(t)
        if "object" not in seen:
            seen.append("object")
        return seen

    objs = []
    for s in dom[":constants"] + prob[":objects"]:
        objs += typed_list(s[1:])
    f["n_objects"] = len(objs)
    size = collections.Counter()
    for _, t in objs:
        for a in ancestors(t):
            size[a] += 1
    size["object"] = max(size["object"], len(objs))
    f["max_type_size"] = max(size.values()) if objs else 0
    f["n_object_types"] = len({t for _, t in objs})

    preds = [s for sec in dom[":predicates"] for s in sec[1:] if isinstance(s, list)]
    f["n_predicates"] = len(preds)
    ar = [len(typed_list(p[1:])) for p in preds]
    f["max_pred_arity"] = max(ar) if ar else 0
    f["mean_pred_arity"] = sum(ar) / len(ar) if ar else 0
    f["n_functions"] = sum(len([s for s in sec[1:] if isinstance(s, list)]) for sec in dom[":functions"])
    f["n_derived"] = len(dom[":derived"])

    acts = dom[":action"] + dom[":durative-action"]
    f["n_actions"] = len(acts)
    npar, pre, eff, cond, quant, ground = [], [], [], 0, 0, 0.0
    big = 0.0
    for a in acts:
        d = kv(a)
        params = typed_list(d.get(":parameters", []))
        npar.append(len(params))
        c = collections.Counter()
        walk(d.get(":precondition", d.get(":condition", [])), c)
        pre.append(c["atom"] + c["numeric"])
        quant += c["forall"] + c["exists"]
        e = collections.Counter()
        walk(d.get(":effect", []), e)
        eff.append(e["atom"] + e["numeric"])
        cond += e["when"]
        quant += e["forall"]
        g = 1.0
        for _, t in params:
            g *= max(size.get(t, 0), 1)
        ground += g
        big = max(big, g)
    f["max_action_params"] = max(npar) if npar else 0
    f["mean_action_params"] = sum(npar) / len(npar) if npar else 0
    f["mean_pre_atoms"] = sum(pre) / len(pre) if pre else 0
    f["mean_eff_atoms"] = sum(eff) / len(eff) if eff else 0
    f["n_conditional_effects"] = cond
    f["n_quantifiers"] = quant
    f["log_ground_actions"] = math.log10(ground + 1)
    f["log_max_action_ground"] = math.log10(big + 1)

    init = [s for sec in prob[":init"] for s in sec[1:] if isinstance(s, list)]
    f["n_init_atoms"] = sum(1 for s in init if s and s[0] != "=" and s[0] != "at")
    f["n_init_fluents"] = sum(1 for s in init if s and s[0] == "=")
    f["n_init_timed"] = sum(1 for s in init if s and s[0] == "at")
    g = collections.Counter()
    for sec in prob[":goal"]:
        walk(sec[1], g)
    f["n_goal_atoms"] = g["atom"] + g["numeric"]
    f["goal_or"] = g["or"]
    f["goal_quant"] = g["forall"] + g["exists"]
    f["goal_pref"] = g["preference"]
    f["has_metric"] = int(bool(prob[":metric"]))
    f["has_constraints"] = int(bool(prob[":constraints"]))
    f["problem_bytes"] = os.path.getsize(prob_path)
    f["domain_bytes"] = os.path.getsize(dom_path)
    f["goal_per_init"] = f["n_goal_atoms"] / max(f["n_init_atoms"], 1)
    f["objects_per_type"] = f["n_objects"] / max(f["n_object_types"], 1)
    f["log_init_x_actions"] = math.log10((f["n_init_atoms"] + 1) * (ground + 1))
    return f


# ----------------------------------------------------------------------------
# the judge
# ----------------------------------------------------------------------------

FAMILY = re.compile(r"-(sequential|propositional|temporal|net-benefit|metric|preferences|time|numeric|agile|satisficing|optimal|multi-core).*$")


def family(variant):
    return FAMILY.sub("", variant)


def choose(P, prior):
    """P: cells x rungs predicted solve probability; ties broken by the prior order."""
    key = P + 1e-6 * prior[None, :]
    return key.argmax(axis=1)


def score(Y, pick):
    return int(Y[np.arange(len(pick)), pick].sum())


def evaluate(X, Y, groups, prior, splitter, seed=0, trees=200):
    pick = np.zeros(len(X), dtype=int)
    conf = np.zeros(len(X))
    for tr, te in splitter(X, groups):
        P = np.zeros((len(te), Y.shape[1]))
        for j in range(Y.shape[1]):
            y = Y[tr, j]
            if y.min() == y.max():
                P[:, j] = float(y[0])
                continue
            rf = RandomForestClassifier(n_estimators=trees, min_samples_leaf=2, random_state=seed, n_jobs=1)
            rf.fit(X[tr], y)
            P[:, j] = rf.predict_proba(X[te])[:, list(rf.classes_).index(1)]
        pick[te] = choose(P, prior)
        conf[te] = P.max(axis=1)
    return pick, conf


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--db", default=os.path.expanduser("~/.crucible/db/crucible.db"))
    ap.add_argument("--engine", required=True)
    ap.add_argument("--baseline", required=True)
    ap.add_argument("--wall", type=float, default=60.0)
    ap.add_argument("--trees", type=int, default=200)
    ap.add_argument("--dump", help="write the feature table as CSV")
    args = ap.parse_args()

    db = mr.open_db(args.db)
    cell = mr.matrix(db, args.engine)
    S = mr.STANDING
    cells = sorted(c for c in cell if all(r in cell[c] for r in S))
    ladder = mr.banked_ladder(db, args.baseline, mr.BOARDS)
    ipc = variant_ipc(db, args.engine)

    rows, names = [], None
    for v, label in cells:
        dom, prob = resolve(ipc[v], v, label)
        f = features(dom, prob)
        names = names or list(f)
        rows.append([f[k] for k in names])
    X = np.array(rows, dtype=float)
    Y = np.array([[int(cell[c][r] is not None and cell[c][r] <= args.wall) for r in S] for c in cells])
    fam = np.array([family(c[0]) for c in cells])
    prior = np.array([Y[:, j].sum() for j in range(len(S))], dtype=float) / len(cells)

    if args.dump:
        with open(args.dump, "w") as fh:
            fh.write("variant,label,family," + ",".join(names) + "," + ",".join("solved_" + r for r in S) + "\n")
            for (v, l), x, y, g in zip(cells, rows, Y, fam):
                fh.write(f"{v},{l},{g}," + ",".join(f"{a:g}" for a in x) + "," + ",".join(map(str, y)) + "\n")

    print(f"cells {len(cells)}  variants {len({c[0] for c in cells})}  families {len(set(fam))}  features {len(names)}")
    print(f"families: {sorted(set(fam))}\n")
    oracle = int(Y.max(axis=1).sum())
    lad = sum(ladder.get(c, 0) for c in cells)
    print(f"{'oracle (any standing rung)':<52} {oracle}")
    print(f"{'banked ladder ' + args.baseline:<52} {lad}")
    best = int(Y.sum(axis=0).max())
    print(f"{'best constant rung (' + S[int(Y.sum(axis=0).argmax())] + ')':<52} {best}")

    # the name-keyed references (what the predictor must reach without the name)
    def loo_by(keyf):
        n = 0
        bykey = collections.defaultdict(list)
        for i, c in enumerate(cells):
            bykey[keyf(c)].append(i)
        for idx in bykey.values():
            for i in idx:
                others = [o for o in idx if o != i]
                cov = Y[others].sum(axis=0) if others else prior
                n += Y[i, int(cov.argmax())]
        return n

    print(f"{'per-variant best rung, leave-one-cell-out (name)':<52} {loo_by(lambda c: c[0])}")
    print(f"{'per-family best rung, leave-one-cell-out (name)':<52} {loo_by(lambda c: family(c[0]))}")

    print("\nthe predictor on text features, whole wall on the chosen rung:")
    gkf = GroupKFold(n_splits=len(set(fam)))
    pick, conf = evaluate(X, Y, fam, prior, lambda X_, g: gkf.split(X_, groups=g), trees=args.trees)
    lodo = score(Y, pick)
    print(f"{'  leave-one-FAMILY-out (unseen domain)':<52} {lodo}")
    kf = KFold(n_splits=10, shuffle=True, random_state=0)
    pick10, conf10 = evaluate(X, Y, fam, prior, lambda X_, g: kf.split(X_), trees=args.trees)
    k10 = score(Y, pick10)
    print(f"{'  10-fold over cells (known domains, unseen instances)':<52} {k10}")

    # gated: the predictor overrides the ladder only when its best probability clears a bar
    lad_cell = np.array([ladder.get(c, 0) for c in cells])
    for gate in (0.7, 0.8, 0.9, 0.95):
        g1 = int(sum(Y[i, pick[i]] if conf[i] >= gate else lad_cell[i] for i in range(len(cells))))
        g2 = int(sum(Y[i, pick10[i]] if conf10[i] >= gate else lad_cell[i] for i in range(len(cells))))
        n1, n2 = int((conf >= gate).sum()), int((conf10 >= gate).sum())
        print(f"{'  gated at ' + str(gate) + ' (else the ladder): unseen / known':<52} {g1} ({n1} overrides) / {g2} ({n2} overrides)")

    # with the family name one-hot added: how much of the signal is the name
    fams = sorted(set(fam))
    H = np.array([[int(g == f_) for f_ in fams] for g in fam], dtype=float)
    XH = np.hstack([X, H])
    pickH, _ = evaluate(XH, Y, fam, prior, lambda X_, g: kf.split(X_), trees=args.trees)
    print(f"{'  10-fold, text features + family name':<52} {score(Y, pickH)}")

    print("\nwhere the leave-one-family-out predictor's pick lands, by family (cells: ladder / predictor / oracle):")
    for f_ in fams:
        idx = np.where(fam == f_)[0]
        l = sum(ladder.get(cells[i], 0) for i in idx)
        p = int(Y[idx, pick[idx]].sum())
        o = int(Y[idx].max(axis=1).sum())
        mark = "" if p >= l else "   <- worse than the ladder"
        print(f"  {f_:<22} {len(idx):3d}  {l:3d} / {p:3d} / {o:3d}{mark}")

    print("\nfeature importance (one forest per rung on all cells, mean decrease in impurity, top 12):")
    imp = np.zeros(len(names))
    for j in range(len(S)):
        if Y[:, j].min() != Y[:, j].max():
            rf = RandomForestClassifier(n_estimators=args.trees, min_samples_leaf=2, random_state=0, n_jobs=1).fit(X, Y[:, j])
            imp += rf.feature_importances_
    for i in np.argsort(-imp)[:12]:
        print(f"  {names[i]:<26} {imp[i] / len(S):.3f}")


if __name__ == "__main__":
    main()
